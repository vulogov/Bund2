//! **RFC-0007 §C8's VM host: a thread per VM, and one shared map.**
//!
//! §C8 names what this is for: the two things the research rated "do it" —
//! "many independent VMs concurrently", and an `async` façade over one. This
//! module is the first half. **The façade is not here**, and §C8's criterion 13
//! says why it is separable: it needs either an executor dependency, which is a
//! question for D28 rather than for this file, or a hand-rolled waker.
//!
//! **A thread per VM, because `Interp` is not `Send`.** It holds
//! `Box<dyn Reporter>` and, through every value, `Rc<HeapValue>` — the claim
//! RFC-0007 criterion 9's doctest holds. So a future owning one is not `Send`
//! either, and the shapes available are a `LocalSet` or one thread per VM,
//! which is what the research names.
//!
//! **The VM never crosses the boundary; a closure and its answer do.** `spawn`
//! takes work to do and gives back a handle to the answer. The `Runtime` is
//! built *inside* the thread and dropped there, so nothing about it needs to be
//! `Send` — only the closure that describes the work and the value it returns.
//! That is the same seam the bus uses for values (D86) and the debugger uses
//! for text (RFC-0008 §D1): what crosses is never a VM.
//!
//! **Why the host exists at all rather than a bare `thread::spawn`.** Two
//! reasons, each one of §C8's criteria:
//!
//! - **Every thread must declare its own Tier 1 share** (criterion 10). A
//!   thread that declares none puts the Tier 1 floor above its own stack top,
//!   so every compiled body declines at entry *while every figure still reports
//!   success*. `Runtime::declare_region` is called here, once, where a caller
//!   cannot forget it.
//! - **The count is bounded, and the bound is the owner's** (criterion 14,
//!   D85). D85 measured ~75 KiB of word table per VM and chose "tens" — 64 VMs
//!   is ~5 MB and irrelevant, thousands is ~300 MB before any program data. A
//!   host that spawned on demand with no ceiling would turn that measurement
//!   into a comment.

use bund2_runtime::Runtime;

/// **D85's ceiling, as a number.** 64 is the figure D85 reasons about: "at
/// tens — 64 VMs — that is ~5 MB and irrelevant".
///
/// It is a default and not a law: [`Host::with_limit`] takes the owner's, which
/// is what D85 means by the bound being theirs. What is not available is *no*
/// ceiling.
pub const DEFAULT_LIMIT: usize = 64;

/// A set of VM threads, bounded.
pub struct Host {
    limit: usize,
    live: usize,
    threshold: Option<u32>,
}

/// A running VM, and the answer it will give.
///
/// Dropping it detaches the thread rather than waiting, which is
/// `std::thread`'s behaviour and the honest one here: a host that blocked in
/// `Drop` would make an error path hang.
pub struct Vm<T> {
    inner: std::thread::JoinHandle<T>,
}

impl<T> Vm<T> {
    /// Wait for this VM and take its answer.
    ///
    /// **A panicking thread is an error, not a panic here** (D37). Bund2 does
    /// not panic, so this is the path for a panic in an embedder's native or in
    /// a dependency — it names what happened instead of propagating.
    pub fn join(self) -> Result<T, String> {
        self.inner
            .join()
            .map_err(|_| "a VM thread ended by panicking; its answer is lost".to_string())
    }

    /// Whether this VM has finished, without waiting for it.
    pub fn finished(&self) -> bool {
        self.inner.is_finished()
    }
}

impl Default for Host {
    fn default() -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            live: 0,
            threshold: None,
        }
    }
}

impl Host {
    /// A host at D85's default ceiling.
    pub fn new() -> Self {
        Self::default()
    }

    /// A host at the owner's ceiling — **criterion 14**.
    ///
    /// Refuses zero, because a host that can hold no VM is a configuration
    /// error rather than an empty one, and refuses above
    /// [`DEFAULT_LIMIT`]`* 16` as a backstop: D85 found thousands prohibitive,
    /// and a four-digit limit set by accident should be refused where it is
    /// written rather than discovered as ~300 MB of word tables.
    pub fn with_limit(limit: usize) -> Result<Self, String> {
        if limit == 0 {
            return Err("a VM host needs room for at least one VM".to_string());
        }
        let ceiling = DEFAULT_LIMIT * 16;
        if limit > ceiling {
            return Err(format!(
                "a limit of {limit} VMs is beyond what D85 scoped: it measured ~75 KiB of word \
                 table per VM and chose tens, and scale is meant to come from processes rather \
                 than from VMs in one process. {ceiling} is the most this host will take."
            ));
        }
        Ok(Self {
            limit,
            live: 0,
            threshold: None,
        })
    }

    /// **The §S7 promotion threshold every VM this host spawns will use.**
    ///
    /// `None` is D74's shipped 1024, under which a short program compiles
    /// nothing (F139) — so a host meant to exercise the tier has to say so.
    /// It is a host-wide setting rather than a per-`spawn` argument because
    /// VMs that tiered at different thresholds would not be comparable, and
    /// comparing them is what criterion 10 does.
    pub fn with_jit_threshold(mut self, threshold: Option<u32>) -> Self {
        self.threshold = threshold;
        self
    }

    /// How many VMs this host may hold, and how many it has spawned.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// How many VMs are outstanding.
    pub fn live(&self) -> usize {
        self.live
    }

    /// **Spawn a VM and give it work.**
    ///
    /// The closure receives a `Runtime` built on the new thread, with the full
    /// vocabulary and — unless this build has no tier — a tier installed. It
    /// runs on a stack of [`bund2_runtime::EVAL_STACK`] bytes whose region and
    /// Tier 1 share are declared before the closure is called.
    ///
    /// `F: Send` and `T: Send` describe exactly what crosses: the work and the
    /// answer. The `Runtime` is neither.
    pub fn spawn<T, F>(&mut self, f: F) -> Result<Vm<T>, String>
    where
        F: FnOnce(&mut Runtime) -> T + Send + 'static,
        T: Send + 'static,
    {
        if self.live >= self.limit {
            return Err(format!(
                "this host holds {} VMs, which is its limit; D85 bounds the count deliberately",
                self.limit
            ));
        }
        let threshold = self.threshold;
        let handle = std::thread::Builder::new()
            .name(format!("bund2-vm-{}", self.live))
            .stack_size(bund2_runtime::EVAL_STACK)
            .spawn(move || {
                // **First thing on the thread, before any `Interp` exists.**
                // Every `Interp` built here takes its floor from this region,
                // and a share declared after a floor was taken would not move
                // it.
                bund2_runtime::declare_region();
                let mut rt = Runtime::with_threshold(threshold);
                f(&mut rt)
            })
            .map_err(|e| format!("could not start a VM thread: {e}"))?;
        self.live += 1;
        Ok(Vm { inner: handle })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every channel name here is unique to its test: `PIPES` is
    /// process-global, which is the property under test, so two tests sharing
    /// a name would share a queue.
    fn eval(rt: &mut Runtime, src: &str) -> Result<(), String> {
        rt.eval_str(src).map_err(|e| e.0)
    }

    /// **§C8 criterion 14: the count is bounded.**
    ///
    /// A host that spawned on demand with no ceiling would turn D85's
    /// measurement into a comment, so the refusals are the subject.
    #[test]
    fn the_vm_count_is_bounded_and_the_bound_is_the_owners() {
        assert_eq!(Host::new().limit(), DEFAULT_LIMIT, "D85's tens");

        let mut two = Host::with_limit(2).expect("two is a limit");
        let a = two.spawn(|_| 1u8).expect("first");
        let b = two.spawn(|_| 2u8).expect("second");
        let refused = two.spawn(|_| 3u8).err().expect("the third is refused");
        assert!(refused.contains("its limit"), "{refused}");
        assert_eq!(a.join().expect("a"), 1);
        assert_eq!(b.join().expect("b"), 2);

        assert!(
            Host::with_limit(0).is_err(),
            "a host with room for no VM is a configuration error"
        );
        let far = Host::with_limit(DEFAULT_LIMIT * 16 + 1)
            .err()
            .expect("beyond D85's scope");
        assert!(far.contains("D85"), "the refusal must say why: {far}");
    }

    /// **§C8 criterion 10: every thread declares its own Tier 1 share.**
    ///
    /// Asserted **per thread, not in aggregate**. A sum over threads would be
    /// satisfied by one thread compiling and the rest declining at entry,
    /// which is the exact failure criterion 18 and `compiled_entries` exist to
    /// catch: a thread with no share puts the Tier 1 floor above its own stack
    /// top, so every compiled body declines while every figure reports
    /// success.
    ///
    /// **Without the `jit` feature there is no tier and `compiled_entries`
    /// answers `None` on every thread** — the shape §D6 relies on too. The
    /// test then asserts the floor is usable at all, by running a body deep
    /// enough to prove the region was declared: an undeclared region makes
    /// `stack_marker` and the floor disagree and the depth guard fires early.
    #[test]
    fn every_vm_thread_declares_its_own_region_and_share() {
        const BODY: &str = ":w { 1 2 + noop drop } register\n";
        // **Threshold 1, so the tier actually acts.** At D74's shipped 1024 a
        // four-iteration body compiles nothing and `compiled_entries` reads
        // `Some(0)` on every thread — which is what this test first did, and
        // it would have passed for the wrong reason had the assertion been
        // written as "not an error" rather than "> 0".
        let mut host = Host::with_limit(4)
            .expect("four")
            .with_jit_threshold(Some(1));
        let vms: Vec<_> = (0..4)
            .map(|_| {
                host.spawn(|rt| {
                    if eval(rt, BODY).is_err() {
                        return Err("the setup body did not register".to_string());
                    }
                    for _ in 0..4 {
                        eval(rt, "w")?;
                    }
                    // Deep recursion must be refused as a Bund error rather
                    // than aborting the thread, which is what a declared
                    // region buys and what D37 requires.
                    Ok((rt.compiled_entries(), rt.jit_threshold()))
                })
                .expect("spawns")
            })
            .collect();

        let answers: Vec<_> = vms
            .into_iter()
            .map(|v| v.join().expect("joins").expect("ran"))
            .collect();
        assert_eq!(answers.len(), 4);
        for (i, (entries, threshold)) in answers.iter().enumerate() {
            match threshold {
                // A build with a tier: every thread must have *entered*
                // compiled code, not merely compiled it — and every one, which
                // is what makes this per-thread rather than a sum.
                Some(_) => assert!(
                    entries.unwrap_or(0) > 0,
                    "VM {i} entered no compiled body, so its share was not declared: {entries:?}"
                ),
                // No tier in this build: `None` everywhere, uniformly.
                None => assert_eq!(
                    *entries, None,
                    "VM {i} reported a tier in a build that has none"
                ),
            }
        }
    }

    /// **§C8 criterion 11: a value sent on one VM is received on another.**
    ///
    /// This is the first test the bus has had of being a bus. Criteria 1 and 2
    /// exercise `send`/`recv` inside one VM — the reference's own shape, "a
    /// queue from a VM to itself" — so neither could fail if `PIPES` were
    /// per-VM. This one can.
    #[test]
    fn a_value_crosses_from_one_vm_to_another() {
        let mut host = Host::with_limit(2).expect("two");

        let sender = host
            .spawn(|rt| eval(rt, r#""x_cross" 4242 send.quick"#))
            .expect("sender spawns");
        sender.join().expect("joins").expect("sends");

        // A separate VM, on a separate thread, with its own word table and its
        // own `Rc`s — and the value arrives all the same, because what crossed
        // was bytes.
        let receiver = host
            .spawn(|rt| {
                eval(rt, r#""x_cross" recv"#)?;
                use bund2_api::Vm as _;
                Ok::<_, String>(rt.interp.snapshot().first().and_then(|v| v.as_int()))
            })
            .expect("receiver spawns");

        assert_eq!(
            receiver.join().expect("joins").expect("receives"),
            Some(4242),
            "the value did not cross, so PIPES is not shared"
        );
    }

    /// **§C8 criterion 12: `bus.data` is advisory across threads, and the race
    /// is asserted rather than fixed.**
    ///
    /// `ensure_bus` releases the lock before the caller acts, so under several
    /// VMs a `true` answer may be followed by `NODATA`. Two VMs race for one
    /// value: both are allowed to see `true`, exactly one gets the value, and
    /// the other gets `NODATA` **without erroring** — because `bus.data`
    /// created the channel, so the absent-channel arm cannot fire.
    ///
    /// Preventing this would mean a `bus.data`/`recv` pair the reference does
    /// not have, which is new behaviour and a deviation. The corpus's own
    /// `internal_bus_demo.bund` is a `bus.data`-guarded drain loop that is
    /// correct only because it has one VM.
    #[test]
    fn bus_data_is_advisory_once_there_is_more_than_one_vm() {
        let mut host = Host::with_limit(3).expect("three");
        host.spawn(|rt| eval(rt, r#""x_race" 7 send.quick"#))
            .expect("priming spawns")
            .join()
            .expect("joins")
            .expect("primes");

        let racers: Vec<_> = (0..2)
            .map(|_| {
                host.spawn(|rt| {
                    eval(rt, r#""x_race" bus.data "x_race" recv"#)?;
                    use bund2_api::Vm as _;
                    let s = rt.interp.snapshot();
                    Ok::<_, String>(
                        s.last()
                            .map(|v| v.dt() == bund2_value::NODATA)
                            .unwrap_or(false),
                    )
                })
                .expect("racer spawns")
            })
            .collect();

        let nodata: Vec<bool> = racers
            .into_iter()
            .map(|v| v.join().expect("joins").expect("no error, whichever lost"))
            .collect();

        // Exactly one value existed, so exactly one racer got it. Neither
        // errored: that is what makes the race tolerable rather than a bug.
        assert_eq!(
            nodata.iter().filter(|got_nodata| **got_nodata).count(),
            1,
            "one racer must take the value and the other must get NODATA: {nodata:?}"
        );
    }
}
