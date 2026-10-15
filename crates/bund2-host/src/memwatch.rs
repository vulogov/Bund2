//! The OOM guard: stop a program before it takes the host down — D171.
//!
//! **Why a guard at all.** A Bund program that grows without bound — a word
//! that calls itself forever, a loop that keeps pushing — does not fail on
//! macOS. The kernel compresses, then swaps, then starves its own daemons,
//! and on 2026-10-15 one such program (`:f { f } register f`, at 18 GB) took
//! the whole machine down with a watchdog panic, "LOW swap space", rather
//! than being killed. Linux usually kills the process instead, but inside a
//! container the kill is the container's limit and comes with no
//! explanation either.
//!
//! **What this module does.** A thread samples the host's memory every
//! [`INTERVAL`] and, when the danger is real *and* this process is the cause,
//! calls the trip function it was given. It does nothing else: it does not
//! exit and does not touch the interpreter. The binary wires the trip to a
//! flag the interpreter reads at its next call, where it becomes an ordinary
//! run-time error with the diagnostic, backtrace and stack every other error
//! has (D171). The design follows the guard in the owner's `plakat`
//! (`src/memwatch.rs` there), which exits instead.
//!
//! **The signal is the kernel's own pressure level on macOS**, not a free-RAM
//! threshold. Free pages read near zero on a healthy Mac, because the kernel
//! keeps reclaimable pages around; a free-RAM guard there cries wolf. Where
//! the kernel gives no level (Linux, or a Mac whose `sysctl` does not
//! answer) the fallback differs: on Linux, available memory against the
//! floor, read from the cgroup when the process runs under a limit, which is
//! the figure that predicts the kill; on macOS, nothing, since an unanswered
//! `sysctl` carries no signal.
//!
//! **Attribution.** Stopping the program only helps if the program is the
//! cause, so a trip also needs the process to hold at least
//! [`CULPRIT_SHARE`] of the memory, or available memory to have fallen by
//! [`FREE_DROP`] since the guard armed. Another application driving the host
//! to pressure does not stop a small Bund program.
//!
//! **The floor is capped at a tenth of the memory**, so a container smaller
//! than [`DEFAULT_FLOOR`] is not read as always in danger.
//!
//! **Two triggers.** *Acute*: critical, and both available memory and free
//! swap below the floor — the next allocation may be the one that hangs the
//! host, so the first such sample trips. *Sustained*: critical for
//! [`Policy::sustained`] consecutive samples, since the kernel can reclaim
//! or swap through a short spike.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// How often the host is sampled.
///
/// **25 ms, not `plakat`'s 100.** A Bund program that recurses without end
/// grows at about 3 GB a second in a release build (measured 2026-10-15 in a
/// 512 MB container: 6, 153, 301, 450 MB at 50 ms steps), so a 100 ms sample
/// lets 300 MB through between looks — more than a small container has
/// left. A sample is a few system calls, or three small files on Linux.
pub const INTERVAL: Duration = Duration::from_millis(25);
/// The floor, in bytes, below which available memory is dangerous.
pub const DEFAULT_FLOOR: u64 = 1_500_000_000;
/// Consecutive critical samples before a sustained trip: about 1.5 s.
pub const DEFAULT_SUSTAINED: u32 = 60;
/// How many samples of the current fall in available memory are counted
/// against it: the guard trips while the program can still be stopped,
/// not after the sample that would have shown it too late.
pub const LOOKAHEAD: u64 = 2;
/// A process holding this share of the memory is the cause.
pub const CULPRIT_SHARE: f64 = 0.25;
/// Or available memory fell this far, in bytes, since the guard armed.
pub const FREE_DROP: u64 = 4_000_000_000;

/// The kernel's memory-pressure level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressure {
    Normal,
    Warning,
    Critical,
    /// No level is available: not macOS, or the `sysctl` did not answer.
    Unknown,
}

/// What the guard reads each sample. Sizes are bytes; `None` is a figure the
/// host did not give.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub pressure: Pressure,
    /// All the memory the process can have: physical RAM, or the cgroup's
    /// limit when that is lower.
    pub total: Option<u64>,
    /// Memory a new allocation can have without swapping.
    pub available: Option<u64>,
    pub swap_free: Option<u64>,
    /// This process's footprint.
    pub resident: Option<u64>,
}

/// Why the guard tripped, with the figures it tripped on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trip {
    pub resident: u64,
    pub available: u64,
    pub swap_free: u64,
    /// The first sample over the floor tripped, rather than a sustained run.
    pub acute: bool,
}

/// The guard's tuning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Bytes. `0` disables the guard.
    pub floor: u64,
    /// Consecutive critical samples before a sustained trip; at least 2.
    pub sustained: u32,
    /// Whether this host's pressure level is the signal. True on macOS,
    /// where an `Unknown` level is no signal at all and available memory is
    /// never consulted for criticality.
    pub kernel_level: bool,
}

impl Policy {
    /// The defaults, adjusted by `BUND2_OOM_GUARD_GB` (the floor; `0`
    /// disables) and `BUND2_OOM_GUARD_SUSTAINED`. A value that does not parse
    /// leaves the default: a typo in an environment variable should not
    /// switch a safety off.
    pub fn from_env() -> Policy {
        let floor = std::env::var("BUND2_OOM_GUARD_GB")
            .ok()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|gb| gb.is_finite() && *gb >= 0.0)
            .map_or(DEFAULT_FLOOR, |gb| (gb * 1e9) as u64);
        let sustained = std::env::var("BUND2_OOM_GUARD_SUSTAINED")
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            .map_or(DEFAULT_SUSTAINED, |n| n.max(2));
        Policy {
            floor,
            sustained,
            kernel_level: cfg!(target_os = "macos"),
        }
    }
}

/// The decision, apart from the thread and the host, so it can be tested.
#[derive(Debug, Clone)]
pub struct Watch {
    policy: Policy,
    /// Available memory when the guard armed.
    baseline: Option<u64>,
    /// Available memory at the previous sample, for the rate of fall.
    last: Option<u64>,
    breaches: u32,
}

impl Watch {
    pub fn new(policy: Policy, first: &Sample) -> Watch {
        Watch {
            policy,
            baseline: first.available,
            last: first.available,
            breaches: 0,
        }
    }

    /// Feed one sample; `Some` when the guard trips. A trip resets the count,
    /// so a program that catches the error and goes on growing trips again
    /// only after another full window, or at once if the danger is acute.
    pub fn step(&mut self, s: &Sample) -> Option<Trip> {
        if self.policy.floor == 0 {
            return None;
        }
        let floor = self.floor(s);
        // **Projected, not only read.** The fall since the last sample,
        // counted [`LOOKAHEAD`] times against what is left: a program falling
        // fast is judged by where it will be when the guard next looks.
        let fall = match (self.last, s.available) {
            (Some(prev), Some(now)) => prev.saturating_sub(now),
            _ => 0,
        };
        self.last = s.available;
        let projected = s
            .available
            .map(|a| a.saturating_sub(fall.saturating_mul(LOOKAHEAD)));
        let critical = match s.pressure {
            Pressure::Critical => true,
            Pressure::Unknown if !self.policy.kernel_level => projected.is_some_and(|a| a < floor),
            _ => false,
        };
        if !critical || !self.culprit(s) {
            self.breaches = 0;
            return None;
        }
        self.breaches = self.breaches.saturating_add(1);
        let available = s.available.unwrap_or(0);
        let swap_free = s.swap_free.unwrap_or(0);
        let acute = projected.unwrap_or(0) < floor && swap_free < floor;
        if acute || self.breaches >= self.policy.sustained {
            self.breaches = 0;
            return Some(Trip {
                resident: s.resident.unwrap_or(0),
                available,
                swap_free,
                acute,
            });
        }
        None
    }

    /// The floor for this sample: the policy's, or a tenth of the memory
    /// when that is smaller. Without the cap a container smaller than the
    /// floor reads as always below it, and the guard would stop any program
    /// that reached [`CULPRIT_SHARE`] of its limit, however much room was
    /// left.
    fn floor(&self, s: &Sample) -> u64 {
        match s.total {
            Some(t) if t > 0 => self.policy.floor.min(t / 10),
            _ => self.policy.floor,
        }
    }

    /// Whether stopping this process would relieve the pressure.
    fn culprit(&self, s: &Sample) -> bool {
        let big_share = match (s.resident, s.total) {
            (Some(r), Some(t)) if t > 0 => r as f64 >= CULPRIT_SHARE * t as f64,
            _ => false,
        };
        let consumed = match (self.baseline, s.available) {
            (Some(b), Some(a)) => b.saturating_sub(a) >= FREE_DROP,
            _ => false,
        };
        big_share || consumed
    }
}

/// A running guard. Dropping it stops the thread at once: the thread parks
/// between samples and the drop unparks it, so ending a program never waits
/// out an interval.
pub struct Guard {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Guard {
    /// Start sampling under `policy`, calling `on_trip` on each trip. Inert
    /// when the floor is `0`, or when the thread cannot be started: a guard
    /// that cannot run must not stop the program it was meant to protect.
    pub fn start<F>(policy: Policy, on_trip: F) -> Guard
    where
        F: Fn(Trip) + Send + 'static,
    {
        Guard::start_with(policy, sample, on_trip)
    }

    /// [`Guard::start`] with the sampler given, for tests.
    pub fn start_with<S, F>(policy: Policy, sampler: S, on_trip: F) -> Guard
    where
        S: Fn() -> Sample + Send + 'static,
        F: Fn(Trip) + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        if policy.floor == 0 {
            return Guard { stop, handle: None };
        }
        let flag = Arc::clone(&stop);
        let handle = thread::Builder::new()
            .name("bund2-oom-guard".into())
            .spawn(move || {
                let mut watch = Watch::new(policy, &sampler());
                while !flag.load(Ordering::Acquire) {
                    if let Some(trip) = watch.step(&sampler()) {
                        on_trip(trip);
                    }
                    thread::park_timeout(INTERVAL);
                }
            })
            .ok();
        Guard { stop, handle }
    }

    /// Whether a thread is sampling.
    pub fn is_armed(&self) -> bool {
        self.handle.is_some()
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            h.thread().unpark();
            // A guard thread that panicked has nothing to report here.
            let _ = h.join();
        }
    }
}

/// One reading of this host.
pub fn sample() -> Sample {
    Sample {
        pressure: pressure(),
        total: total(),
        available: available(),
        swap_free: swap_free(),
        resident: resident(),
    }
}

/// `kern.memorystatus_vm_pressure_level`: 1 normal, 2 warning, 4 critical.
#[cfg(target_os = "macos")]
pub fn pressure() -> Pressure {
    let mut level: libc::c_int = 0;
    let mut len = std::mem::size_of::<libc::c_int>();
    // SAFETY: the level is a C int; the buffer is one, `len` says so, and
    // the name is NUL-terminated.
    let rc = unsafe {
        libc::sysctlbyname(
            c"kern.memorystatus_vm_pressure_level".as_ptr(),
            (&raw mut level).cast(),
            &raw mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return Pressure::Unknown;
    }
    match level {
        l if l >= 4 => Pressure::Critical,
        2 | 3 => Pressure::Warning,
        1 => Pressure::Normal,
        _ => Pressure::Unknown,
    }
}

/// No kernel level off macOS; the policy falls back to available memory.
#[cfg(not(target_os = "macos"))]
pub fn pressure() -> Pressure {
    Pressure::Unknown
}

#[cfg(target_os = "macos")]
fn total() -> Option<u64> {
    let mut total: u64 = 0;
    let mut len = std::mem::size_of::<u64>();
    // SAFETY: `hw.memsize` is a 64-bit integer; the buffer is one, `len`
    // says so, and the name is NUL-terminated.
    let rc = unsafe {
        libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&raw mut total).cast(),
            &raw mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (rc == 0).then_some(total)
}

/// Memory a new allocation can have without swapping, in bytes: free plus
/// reclaimable pages — inactive, speculative, purgeable and file-backed —
/// which is what Activity Monitor calls available. `None` when the host
/// does not say. Public for F210: a word about to allocate a result it can
/// size in advance asks this first.
#[cfg(target_os = "macos")]
pub fn available() -> Option<u64> {
    // SAFETY: `sysconf` takes a name and returns a number.
    let page = u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).ok()?;
    let mut stats = std::mem::MaybeUninit::<libc::vm_statistics64>::zeroed();
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY: as in `crate::memory`: the buffer is a `vm_statistics64`,
    // `count` is its length in the call's units, and the host port is given
    // back below.
    #[allow(deprecated)]
    let (rc, released) = unsafe {
        let host = libc::mach_host_self();
        let rc = libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            stats.as_mut_ptr().cast(),
            &raw mut count,
        );
        (
            rc,
            crate::mach_port_deallocate(libc::mach_task_self(), host),
        )
    };
    if rc != 0 || released != 0 {
        return None;
    }
    // SAFETY: zeroed to begin with, and every bit pattern is a valid value
    // of a struct of integers.
    let s = unsafe { stats.assume_init() };
    let pages = u64::from(s.free_count)
        + u64::from(s.inactive_count)
        + u64::from(s.speculative_count)
        + u64::from(s.purgeable_count)
        + u64::from(s.external_page_count);
    Some(pages.saturating_mul(page))
}

/// `vm.swapusage`'s available figure.
#[cfg(target_os = "macos")]
fn swap_free() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::xsw_usage>::zeroed();
    let mut len = std::mem::size_of::<libc::xsw_usage>();
    // SAFETY: `vm.swapusage` is an `xsw_usage`; the buffer is one and `len`
    // says so.
    let rc = unsafe {
        libc::sysctlbyname(
            c"vm.swapusage".as_ptr(),
            usage.as_mut_ptr().cast(),
            &raw mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return None;
    }
    // SAFETY: zeroed, then written by the kernel; every bit pattern of a
    // struct of integers is a valid value.
    Some(unsafe { usage.assume_init() }.xsu_avail)
}

/// The physical footprint, which is what the kernel charges the process,
/// compressed pages included.
#[cfg(target_os = "macos")]
fn resident() -> Option<u64> {
    let mut info = std::mem::MaybeUninit::<libc::rusage_info_v2>::zeroed();
    // SAFETY: the flavour is V2 and the buffer is a `rusage_info_v2`; the
    // pid is this process's own.
    let rc = unsafe {
        libc::proc_pid_rusage(
            libc::getpid(),
            libc::RUSAGE_INFO_V2,
            info.as_mut_ptr().cast(),
        )
    };
    if rc != 0 {
        return None;
    }
    // SAFETY: zeroed, then written by the kernel; a struct of integers.
    Some(unsafe { info.assume_init() }.ri_phys_footprint)
}

#[cfg(target_os = "linux")]
fn total() -> Option<u64> {
    let ram = meminfo_field(&std::fs::read_to_string("/proc/meminfo").ok()?, "MemTotal");
    match (ram, cgroup_limit()) {
        (Some(r), Some((max, _))) => Some(r.min(max)),
        (r, c) => r.or(c.map(|(max, _)| max)),
    }
}

/// `MemAvailable`, or the room left under the cgroup's limit when that is
/// smaller: inside a container the limit is what kills. See the macOS
/// reading for the contract.
#[cfg(target_os = "linux")]
pub fn available() -> Option<u64> {
    let ram = meminfo_field(
        &std::fs::read_to_string("/proc/meminfo").ok()?,
        "MemAvailable",
    );
    let room = cgroup_limit().map(|(max, cur)| max.saturating_sub(cur));
    match (ram, room) {
        (Some(r), Some(c)) => Some(r.min(c)),
        (r, c) => r.or(c),
    }
}

#[cfg(target_os = "linux")]
fn swap_free() -> Option<u64> {
    meminfo_field(&std::fs::read_to_string("/proc/meminfo").ok()?, "SwapFree")
}

/// `/proc/self/statm`'s resident pages.
#[cfg(target_os = "linux")]
fn resident() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    // SAFETY: `sysconf` takes a name and returns a number.
    let page = u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).ok()?;
    Some(pages.saturating_mul(page))
}

/// The cgroup v2 limit and usage, `(memory.max, memory.current)`, when the
/// process runs under a limit. `None` when there is no limit (`max`) or no
/// cgroup v2.
#[cfg(target_os = "linux")]
fn cgroup_limit() -> Option<(u64, u64)> {
    let own = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let path = own.lines().find_map(|l| l.strip_prefix("0::"))?.trim();
    let dir = std::path::Path::new("/sys/fs/cgroup").join(path.trim_start_matches('/'));
    let max = std::fs::read_to_string(dir.join("memory.max")).ok()?;
    let max: u64 = max.trim().parse().ok()?;
    let cur: u64 = std::fs::read_to_string(dir.join("memory.current"))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some((max, cur))
}

/// One `/proc/meminfo` figure, in bytes.
#[cfg(any(target_os = "linux", test))]
fn meminfo_field(text: &str, name: &str) -> Option<u64> {
    text.lines().find_map(|l| {
        let (n, rest) = l.split_once(':')?;
        if n != name {
            return None;
        }
        let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
        Some(kb.saturating_mul(1024))
    })
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn total() -> Option<u64> {
    None
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn available() -> Option<u64> {
    None
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn swap_free() -> Option<u64> {
    None
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn resident() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    const GB: u64 = 1_000_000_000;

    fn policy(kernel_level: bool) -> Policy {
        Policy {
            floor: DEFAULT_FLOOR,
            sustained: 3,
            kernel_level,
        }
    }

    fn at(pressure: Pressure, available: u64, swap_free: u64, resident: u64) -> Sample {
        Sample {
            pressure,
            total: Some(24 * GB),
            available: Some(available),
            swap_free: Some(swap_free),
            resident: Some(resident),
        }
    }

    #[test]
    fn critical_pressure_from_a_big_process_trips_after_the_window() {
        let mut w = Watch::new(policy(true), &at(Pressure::Normal, 12 * GB, 8 * GB, 0));
        let s = at(Pressure::Critical, 3 * GB, 4 * GB, 18 * GB);
        assert_eq!(w.step(&s), None);
        assert_eq!(w.step(&s), None);
        let trip = w.step(&s).unwrap();
        assert!(!trip.acute);
        assert_eq!(trip.resident, 18 * GB);
        // The count starts again after a trip.
        assert_eq!(w.step(&s), None);
    }

    #[test]
    fn acute_danger_trips_on_the_first_sample() {
        let mut w = Watch::new(policy(true), &at(Pressure::Normal, 12 * GB, 8 * GB, 0));
        let trip = w
            .step(&at(Pressure::Critical, GB / 2, GB / 2, 18 * GB))
            .unwrap();
        assert!(trip.acute);
    }

    #[test]
    fn a_small_process_is_not_stopped_for_pressure_it_did_not_cause() {
        let mut w = Watch::new(policy(true), &at(Pressure::Normal, 3 * GB, GB, 0));
        let s = at(Pressure::Critical, GB / 2, GB / 2, GB);
        for _ in 0..10 {
            assert_eq!(w.step(&s), None);
        }
    }

    #[test]
    fn a_fall_in_available_memory_since_arming_attributes_the_pressure() {
        let mut w = Watch::new(policy(true), &at(Pressure::Normal, 12 * GB, 8 * GB, 0));
        // Only 2 GB resident, but 11 GB of available memory went since arming.
        assert!(w.step(&at(Pressure::Critical, GB, GB, 2 * GB)).is_some());
    }

    #[test]
    fn a_break_in_the_pressure_resets_the_window() {
        let mut w = Watch::new(policy(true), &at(Pressure::Normal, 12 * GB, 8 * GB, 0));
        let hot = at(Pressure::Critical, 3 * GB, 4 * GB, 18 * GB);
        let calm = at(Pressure::Warning, 3 * GB, 4 * GB, 18 * GB);
        assert_eq!(w.step(&hot), None);
        assert_eq!(w.step(&hot), None);
        assert_eq!(w.step(&calm), None);
        assert_eq!(w.step(&hot), None);
        assert_eq!(w.step(&hot), None);
        assert!(w.step(&hot).is_some());
    }

    #[test]
    fn without_a_kernel_level_available_memory_is_the_signal_off_macos_only() {
        let s = at(Pressure::Unknown, GB, GB, 18 * GB);
        let first = at(Pressure::Unknown, 12 * GB, 8 * GB, 0);
        // Linux: below the floor is critical.
        assert!(Watch::new(policy(false), &first).step(&s).is_some());
        // macOS: an unanswered sysctl is no signal.
        let mut mac = Watch::new(policy(true), &first);
        for _ in 0..10 {
            assert_eq!(mac.step(&s), None);
        }
    }

    #[test]
    fn a_container_smaller_than_the_floor_trips_near_its_limit_not_before() {
        const MB: u64 = 1_000_000;
        let small = |available: u64, resident: u64| Sample {
            pressure: Pressure::Unknown,
            total: Some(512 * MB),
            available: Some(available),
            swap_free: Some(0),
            resident: Some(resident),
        };
        let mut w = Watch::new(policy(false), &small(300 * MB, 200 * MB));
        // A quarter of the limit, with 300 MB of room: not danger.
        for _ in 0..10 {
            assert_eq!(w.step(&small(300 * MB, 200 * MB)), None);
        }
        // Under a tenth of the limit left: acute.
        assert!(w.step(&small(40 * MB, 460 * MB)).is_some());
    }

    #[test]
    fn a_fast_fall_trips_before_the_floor_is_reached() {
        const MB: u64 = 1_000_000;
        let small = |available: u64, resident: u64| Sample {
            pressure: Pressure::Unknown,
            total: Some(512 * MB),
            available: Some(available),
            swap_free: Some(0),
            resident: Some(resident),
        };
        let mut w = Watch::new(policy(false), &small(500 * MB, 2 * MB));
        // 75 MB a sample against a 51 MB floor: two samples ahead is
        // 150 MB, so the guard trips once 201 MB or less is left, where the
        // next sample would already be past the floor.
        for left in [430, 355, 280, 205] {
            assert_eq!(w.step(&small(left * MB, (500 - left) * MB)), None, "{left}");
        }
        assert!(w.step(&small(130 * MB, 370 * MB)).is_some());
        // The same 130 MB, reached slowly, is room.
        let mut slow = Watch::new(policy(false), &small(135 * MB, 2 * MB));
        for _ in 0..10 {
            assert_eq!(slow.step(&small(130 * MB, 370 * MB)), None);
        }
    }

    #[test]
    fn a_zero_floor_disables_the_guard_and_starts_no_thread() {
        let off = Policy {
            floor: 0,
            ..policy(true)
        };
        let mut w = Watch::new(off, &at(Pressure::Normal, 12 * GB, 8 * GB, 0));
        assert_eq!(w.step(&at(Pressure::Critical, 0, 0, 24 * GB)), None);
        let g = Guard::start_with(off, || at(Pressure::Critical, 0, 0, 24 * GB), |_| {});
        assert!(!g.is_armed());
    }

    #[test]
    fn the_thread_reports_a_trip_and_stops_promptly_when_dropped() {
        let (tx, rx) = mpsc::channel();
        let g = Guard::start_with(
            policy(true),
            || at(Pressure::Critical, 0, 0, 20 * GB),
            move |t| {
                let _ = tx.send(t);
            },
        );
        assert!(g.is_armed());
        let trip = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(trip.acute);
        // The drop unparks the thread, so it returns without waiting out an
        // interval; the bound is loose so a loaded machine does not fail it.
        let t0 = std::time::Instant::now();
        drop(g);
        assert!(
            t0.elapsed() < Duration::from_secs(1),
            "drop waited {:?}",
            t0.elapsed()
        );
    }

    #[test]
    fn meminfo_fields_are_read_in_bytes() {
        let text = "MemTotal:       16384 kB\nMemAvailable:    2048 kB\nSwapFree: 0 kB\n";
        assert_eq!(meminfo_field(text, "MemAvailable"), Some(2048 * 1024));
        assert_eq!(meminfo_field(text, "SwapFree"), Some(0));
        assert_eq!(meminfo_field(text, "Cached"), None);
    }

    #[test]
    fn this_host_answers_every_reading() {
        let s = sample();
        if cfg!(any(target_os = "macos", target_os = "linux")) {
            assert!(s.total.is_some_and(|t| t > 0), "{s:?}");
            assert!(s.available.is_some(), "{s:?}");
            assert!(s.resident.is_some_and(|r| r > 0), "{s:?}");
        }
        if cfg!(target_os = "macos") {
            assert_ne!(s.pressure, Pressure::Unknown, "{s:?}");
        }
    }
}
