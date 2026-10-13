//! What the machine says about itself — D152.
//!
//! `sysinfo.*` and `debug.display_hostinfo` answer with facts about the host:
//! four strings from `uname`, six memory figures, and a guess at the
//! virtualization. The reference reads them through `sys_metrics`
//! (`reference/Bund/src/stdlib/functions/sysinfo/mem.rs`, `host.rs`,
//! `virt.rs`), which is AGPL-3.0, and a crate under that licence cannot be
//! linked into a binary offered as Apache-2.0. So the facts are read here.
//!
//! **The same facts, and the same arithmetic on them.** A program sees these
//! figures as numbers and compares them, so which counters are added up, and
//! where a division rounds, is behaviour: each function below says what it
//! reads and how, and where that is odd it is odd on purpose. None of it is
//! `sys_metrics`' code. What was taken from it is which file, which call and
//! which formula, which are facts about the oracle's output.
//!
//! **The only `unsafe` the standard library reaches**, and the reason this is
//! a crate: `bund2-stdlib` forbids it. Each block is one C call on a buffer
//! this crate owns.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

use std::io;

/// The four `uname` strings the words read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uname {
    /// `sysname`: what `sysinfo.system` answers.
    pub system: String,
    /// `nodename`: `sysinfo.hostname`.
    pub hostname: String,
    /// `release`: `sysinfo.kernel_version`.
    pub kernel_version: String,
    /// `version`: `sysinfo.os_version`.
    pub os_version: String,
}

/// Memory, in **MiB**, each figure already divided.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Memory {
    pub total: u64,
    pub free: u64,
    pub used: u64,
    pub shared: u64,
    pub buffers: u64,
    pub cached: u64,
}

/// What the host runs under. The names are what `sysinfo.virtualization`
/// prints, through `Debug`, so they are text a program can compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Virtualization {
    Xen,
    Kvm,
    Qemu,
    Vmware,
    Oracle,
    Bochs,
    Parallels,
    Bhyve,
    OpenVz,
    LxcLibvirt,
    Lxc,
    SystemdNspawn,
    Docker,
    Podman,
    Rkt,
    Wsl,
    Unknown,
}

/// `uname(2)`.
#[cfg(unix)]
pub fn uname() -> io::Result<Uname> {
    let mut raw = std::mem::MaybeUninit::<libc::utsname>::uninit();
    // SAFETY: `uname` writes one `utsname` through the pointer and nothing
    // else; the buffer is exactly that and outlives the call.
    if unsafe { libc::uname(raw.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the call succeeded, so every field was written.
    let raw = unsafe { raw.assume_init() };
    Ok(Uname {
        system: field(&raw.sysname),
        hostname: field(&raw.nodename),
        kernel_version: field(&raw.release),
        os_version: field(&raw.version),
    })
}

/// A host this crate has no reading for.
#[cfg(not(unix))]
pub fn uname() -> io::Result<Uname> {
    Err(unsupported())
}

/// One `utsname` field as text: up to its NUL, or all of it if the kernel
/// left none. Bytes that are not UTF-8 are replaced, not refused.
#[cfg(unix)]
fn field(chars: &[libc::c_char]) -> String {
    let bytes: Vec<u8> = chars
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "this host's memory and identity are not read on this platform",
    )
}

/// Memory on macOS.
///
/// - total: `hw.memsize`, in bytes, divided to MiB.
/// - free: free pages less speculative ones.
/// - used: active pages plus wired ones.
/// - shared, buffers, cached: 0. The oracle reports none of them here, which
///   is why `tests/probes/sysinfo-words.bund` reads three `false` on macOS
///   and has a variant for Linux (D150).
///
/// The page counts are 32-bit and are combined before they are widened, as
/// the oracle's release build does, so a difference that would go below zero
/// wraps instead.
#[cfg(target_os = "macos")]
pub fn memory() -> io::Result<Memory> {
    const MIB: u64 = 1024 * 1024;

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
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: `sysconf` takes a name and returns a number.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let page = u64::try_from(page).map_err(|_| io::Error::last_os_error())?;

    let mut stats = std::mem::MaybeUninit::<libc::vm_statistics64>::zeroed();
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY: the buffer is a `vm_statistics64` and `count` is its length in
    // the units the call counts in; the kernel writes at most that much. The
    // host port is this task's own right, given back below.
    #[allow(deprecated)]
    let (rc, released) = unsafe {
        let host = libc::mach_host_self();
        let rc = libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            stats.as_mut_ptr().cast(),
            &raw mut count,
        );
        (rc, mach_port_deallocate(libc::mach_task_self(), host))
    };
    if rc != 0 || released != 0 {
        return Err(io::Error::other(format!(
            "host_statistics64 answered {rc}, and releasing the host port {released}"
        )));
    }
    // SAFETY: zeroed to begin with, and every bit pattern is a valid value
    // of a struct of integers.
    let stats = unsafe { stats.assume_init() };
    let (free, speculative, active, wired) = (
        stats.free_count,
        stats.speculative_count,
        stats.active_count,
        stats.wire_count,
    );
    Ok(Memory {
        total: total / MIB,
        free: u64::from(free.wrapping_sub(speculative)) * page / MIB,
        used: u64::from(active.wrapping_add(wired)) * page / MIB,
        shared: 0,
        buffers: 0,
        cached: 0,
    })
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    /// `libc` declares the call that makes the host port and not the one
    /// that gives it back.
    fn mach_port_deallocate(
        task: libc::mach_port_t,
        name: libc::mach_port_t,
    ) -> libc::kern_return_t;
}

/// Memory on Linux, from `/proc/meminfo`.
#[cfg(target_os = "linux")]
pub fn memory() -> io::Result<Memory> {
    memory_from_meminfo(&std::fs::read_to_string("/proc/meminfo")?)
}

/// A host this crate has no reading for.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn memory() -> io::Result<Memory> {
    Err(unsupported())
}

/// The six figures out of `/proc/meminfo`'s text.
///
/// Six lines are read, in whatever order the kernel prints them: `MemTotal`,
/// `MemFree`, `Buffers`, `Shmem`, and `Cached` and `SReclaimable`, which are
/// both counted as cached. Each is in kB and is divided to MiB **on its
/// own, before any sum**, so `cached` is the two quotients added and can be
/// one less than the quotient of the sum. `used` is the total less free,
/// buffers and cached, taken from those rounded figures.
///
/// A file without all six is an error, as it is for the oracle. A figure
/// that is not a number is an error too, where the oracle's dependency
/// panics.
#[cfg(any(target_os = "linux", test))]
fn memory_from_meminfo(text: &str) -> io::Result<Memory> {
    let bad = |what: String| io::Error::new(io::ErrorKind::InvalidData, what);
    let mut m = Memory::default();
    let mut matched = 0u8;
    for line in text.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let into = match name {
            "MemTotal" => &mut m.total,
            "MemFree" => &mut m.free,
            "Buffers" => &mut m.buffers,
            "Cached" | "SReclaimable" => &mut m.cached,
            "Shmem" => &mut m.shared,
            _ => continue,
        };
        let figure = rest.split_whitespace().next().unwrap_or("");
        let kb: u64 = figure
            .parse()
            .map_err(|_| bad(format!("/proc/meminfo: `{name}` reads `{figure}`")))?;
        *into += kb / 1024;
        matched += 1;
        if matched == 6 {
            m.used = m
                .total
                .wrapping_sub(m.free.wrapping_add(m.buffers).wrapping_add(m.cached));
            return Ok(m);
        }
    }
    Err(io::Error::other("Couldn't get the memory information"))
}

/// The virtualization. macOS is never asked: the oracle answers `Unknown`
/// there whatever the host is.
#[cfg(not(target_os = "linux"))]
pub fn virtualization() -> Virtualization {
    Virtualization::Unknown
}

/// The virtualization on Linux: four probes in order, the first to name one
/// wins, and none naming one is `Unknown`.
///
/// 1. OpenVZ: `/proc/vz` exists and `/proc/bc` does not.
/// 2. WSL: `/proc/sys/kernel/osrelease` mentions `Microsoft` or `WSL`.
/// 3. A container that systemd recorded, in `/run/systemd/container`.
/// 4. A hypervisor named in the machine's DMI strings.
#[cfg(target_os = "linux")]
pub fn virtualization() -> Virtualization {
    use std::path::Path;
    let read = |p: &str| std::fs::read_to_string(p).ok();
    if Path::new("/proc/vz").exists() && !Path::new("/proc/bc").exists() {
        return Virtualization::OpenVz;
    }
    if let Some(v) = read("/proc/sys/kernel/osrelease").and_then(|r| wsl(&r)) {
        return v;
    }
    if let Some(v) = read("/run/systemd/container").and_then(|c| container(&c)) {
        return v;
    }
    let dmi = [
        "/sys/class/dmi/id/product_name",
        "/sys/class/dmi/id/sys_vendor",
        "/sys/class/dmi/id/board_vendor",
        "/sys/class/dmi/id/bios_vendor",
    ];
    hypervisor(dmi.iter().map(|p| std::fs::read(p).ok()))
}

/// WSL, by the kernel's release string.
#[cfg(any(target_os = "linux", test))]
fn wsl(osrelease: &str) -> Option<Virtualization> {
    (osrelease.contains("Microsoft") || osrelease.contains("WSL")).then_some(Virtualization::Wsl)
}

/// The container systemd recorded, by the file's **first line with its
/// newline kept**.
///
/// That is the oracle's reading, and it means a file that ends its line, as
/// systemd writes it, names nothing: `docker\n` is not `docker`. Only a file
/// with no newline matches. Reproduced, because under systemd in a container
/// the oracle answers `Unknown` and so must this (F206).
#[cfg(any(target_os = "linux", test))]
fn container(file: &str) -> Option<Virtualization> {
    let line = match file.find('\n') {
        Some(at) => &file[..=at],
        None => file,
    };
    Some(match line {
        "lxc" => Virtualization::Lxc,
        "lxc-libvirt" => Virtualization::LxcLibvirt,
        "systemd-nspawn" => Virtualization::SystemdNspawn,
        "docker" => Virtualization::Docker,
        "podman" => Virtualization::Podman,
        "rkt" => Virtualization::Rkt,
        "wsl" => Virtualization::Wsl,
        _ => return None,
    })
}

/// The hypervisor, by the first three bytes of each DMI string in turn.
///
/// **A string that cannot be read ends the search**, and so does one shorter
/// than three bytes: the oracle stops at the first failure and does not go
/// on to the next file. Each item is one file's bytes, or `None` for a file
/// that could not be read.
#[cfg(any(target_os = "linux", test))]
fn hypervisor(files: impl Iterator<Item = Option<Vec<u8>>>) -> Virtualization {
    for bytes in files {
        let Some(head) = bytes.as_deref().and_then(|b| b.get(..3)) else {
            return Virtualization::Unknown;
        };
        match head {
            b"Xen" => return Virtualization::Xen,
            b"KVM" => return Virtualization::Kvm,
            b"QEM" => return Virtualization::Qemu,
            b"VMw" | b"VMW" => return Virtualization::Vmware,
            b"inn" => return Virtualization::Oracle,
            b"Boc" => return Virtualization::Bochs,
            b"Par" => return Virtualization::Parallels,
            b"BHY" => return Virtualization::Bhyve,
            _ => {}
        }
    }
    Virtualization::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn uname_answers_four_strings() {
        let u = uname().expect("uname");
        for (what, s) in [
            ("system", &u.system),
            ("hostname", &u.hostname),
            ("kernel_version", &u.kernel_version),
            ("os_version", &u.os_version),
        ] {
            assert!(!s.is_empty(), "{what} is empty");
            assert!(!s.contains('\0'), "{what} holds a NUL");
        }
        let expected = if cfg!(target_os = "macos") {
            "Darwin"
        } else {
            "Linux"
        };
        if cfg!(any(target_os = "macos", target_os = "linux")) {
            assert_eq!(u.system, expected);
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn memory_is_in_mib_and_adds_up() {
        let m = memory().expect("memory");
        assert!(m.total >= 256, "{} MiB of memory", m.total);
        assert!(m.total < 64 * 1024 * 1024, "{} is not MiB", m.total);
        assert!(m.free <= m.total, "free {} of {}", m.free, m.total);
        assert!(m.used <= m.total, "used {} of {}", m.used, m.total);
        assert!(m.used > 0);
        if cfg!(target_os = "macos") {
            assert_eq!((m.shared, m.buffers, m.cached), (0, 0, 0));
        }
    }

    const MEMINFO: &str = "\
MemTotal:       16000000 kB
MemFree:         4001023 kB
MemAvailable:    9000000 kB
Buffers:          200000 kB
Cached:          3001900 kB
SwapCached:            0 kB
Shmem:            100000 kB
KReclaimable:     500000 kB
Slab:             700000 kB
SReclaimable:     501600 kB
SUnreclaim:       200000 kB
";

    /// Each figure is divided on its own: `Cached` and `SReclaimable` come
    /// to 2,931 and 489 MiB and are added, where their sum divided is 3,421.
    #[test]
    fn meminfo_is_divided_line_by_line() {
        let m = memory_from_meminfo(MEMINFO).expect("six lines");
        assert_eq!(m.total, 15_625);
        assert_eq!(m.free, 3_907);
        assert_eq!(m.buffers, 195);
        assert_eq!(m.shared, 97);
        assert_eq!(m.cached, 2_931 + 489);
        assert_eq!((3_001_900 + 501_600) / 1024, 3_421);
        assert_eq!(m.used, 15_625 - (3_907 + 195 + 3_420));
    }

    #[test]
    fn meminfo_without_all_six_lines_is_an_error() {
        let short: String = MEMINFO
            .lines()
            .filter(|l| !l.starts_with("Shmem"))
            .map(|l| format!("{l}\n"))
            .collect();
        assert!(memory_from_meminfo(&short).is_err());
        let garbled = MEMINFO.replace("16000000", "sixteen");
        let err = memory_from_meminfo(&garbled).expect_err("not a number");
        assert!(err.to_string().contains("MemTotal"), "{err}");
    }

    /// F206: the line is compared with its newline, so the file systemd
    /// writes names nothing and only one without a newline does.
    #[test]
    fn a_container_file_that_ends_its_line_names_nothing() {
        assert_eq!(container("docker\n"), None);
        assert_eq!(container("docker"), Some(Virtualization::Docker));
        assert_eq!(container("lxc-libvirt"), Some(Virtualization::LxcLibvirt));
        assert_eq!(container(""), None);
    }

    #[test]
    fn wsl_is_read_from_the_release_string() {
        assert_eq!(
            wsl("5.15.90.1-microsoft-standard-WSL2\n"),
            Some(Virtualization::Wsl)
        );
        assert_eq!(wsl("4.4.0-19041-Microsoft"), Some(Virtualization::Wsl));
        assert_eq!(wsl("6.8.0-1017-azure\n"), None);
    }

    #[test]
    fn a_hypervisor_is_named_by_three_bytes_and_a_missing_file_ends_the_search() {
        let f = |s: &str| Some(s.as_bytes().to_vec());
        assert_eq!(
            hypervisor([f("Virtual Machine\n"), f("KVM\n")].into_iter()),
            Virtualization::Kvm
        );
        assert_eq!(
            hypervisor([f("VMware Virtual Platform\n")].into_iter()),
            Virtualization::Vmware
        );
        assert_eq!(
            hypervisor([f("Standard PC"), f("QEMU")].into_iter()),
            Virtualization::Qemu
        );
        // The second file would have named one, and is never read.
        assert_eq!(
            hypervisor([None, f("KVM")].into_iter()),
            Virtualization::Unknown
        );
        assert_eq!(
            hypervisor([f("ab"), f("KVM")].into_iter()),
            Virtualization::Unknown
        );
        assert_eq!(
            hypervisor([f("Microsoft Corporation")].into_iter()),
            Virtualization::Unknown
        );
    }

    #[test]
    fn the_names_are_the_ones_a_program_prints() {
        assert_eq!(format!("{:?}", Virtualization::Unknown), "Unknown");
        assert_eq!(format!("{:?}", Virtualization::LxcLibvirt), "LxcLibvirt");
        assert_eq!(
            format!("{:?}", Virtualization::SystemdNspawn),
            "SystemdNspawn"
        );
    }
}
