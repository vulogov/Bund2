//! `sysinfo.*` — facts about the host and the build.
//!
//! **Every answer here but `sysinfo.version` names the machine**, so no golden
//! can hold one. The probe `tests/probes/sysinfo-words.bund` reduces each to a
//! claim that is both stable and portable instead — a type, or a comparison
//! against zero — which is the same discipline `fs.cwd` and `filename` get.
//! The six memory values are *volatile* on top of that, and the workbench
//! forms answer on the workbench, where the capture records whatever is left;
//! `convert.to_bool.` is what turns one of those into a stable boolean,
//! because no word moves a scalar from the workbench to the stack.
//!
//! F154: `sysinfo.mem.used` answers **free** memory, while
//! `sysinfo.mem.used.` answers used. One of the twelve registrations in the
//! reference passes the wrong operation, and the two forms of that one word
//! disagree by whatever the host's figures differ by.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;

use crate::wb::Side;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `sysinfo.version` — the interpreter's own version
/// (`reference/Bund/src/stdlib/functions/sysinfo/host.rs:9-12`).
///
/// **A deviation with no golden to record it against.** The reference pushes
/// `env!("CARGO_PKG_VERSION")` of the `bund` crate; this pushes Bund2's. The
/// strings necessarily differ, and reporting Bund's version from Bund2 would
/// be a lie rather than a preservation.
///
/// It is safe because nothing captures the value: `if_word.bund` is the only
/// corpus program that mentions `version`, and it only asks `?word` whether
/// the name resolves — never runs it. If a golden ever prints it, that golden
/// is unreproducible for the same reason F66's are, and the register is where
/// that would go.
fn version(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::str(env!("CARGO_PKG_VERSION")));
    Ok(())
}

/// The host's virtualization, as `sys_metrics` names it
/// (`reference/Bund/src/stdlib/functions/sysinfo/virt.rs:9-11`).
pub(crate) fn virtualization() -> String {
    format!("{:?}", sys_metrics::virt::get_virt_info())
}

/// `sysinfo.virtualization` — push the virtualization's name (`virt.rs:13-16`).
fn virtualization_word(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::str(virtualization()));
    Ok(())
}

/// `sysinfo.virtualization?` — whether any virtualization was detected
/// (`virt.rs:17-27`).
fn is_virtualized(vm: &mut dyn Vm) -> Result<(), Error> {
    let known = !matches!(
        sys_metrics::virt::get_virt_info(),
        sys_metrics::virt::Virtualization::Unknown
    );
    vm.push(BundValue::boolean(known));
    Ok(())
}

/// The four host strings (`reference/Bund/src/stdlib/functions/sysinfo/host.rs`).
///
/// **The error names the word in lower case**, not an upper-case prefix: the
/// reference writes `sysinfo.hostname returns: {}` rather than the
/// `SYSINFO.HOSTNAME` that the memory words use. Reproduced, including the
/// inconsistency between the two files.
fn host_string(
    vm: &mut dyn Vm,
    word: &str,
    read: impl Fn() -> Result<String, String>,
) -> Result<(), Error> {
    let s = read().map_err(|e| Error(format!("{word} returns: {e}")))?;
    vm.push(BundValue::str(s));
    Ok(())
}

/// Which field of `get_memory` a call reads
/// (`reference/Bund/src/stdlib/functions/sysinfo/mem.rs`, `MemOperation`).
#[derive(Clone, Copy)]
enum MemOp {
    Total,
    Free,
    Used,
    Shared,
    Buffers,
    Cached,
}

/// The twelve memory words (`mem.rs`, `bund_mem_base`).
///
/// **`sys_metrics` reports MiB, so the reference's `(res*1024)*1024` is the
/// correct conversion to bytes** -- checked, because it reads like a defect
/// and is not: `sysinfo.mem.total` answered exactly `sysctl -n hw.memsize` on
/// the measuring machine. F154 says so too, so neither place can be "fixed"
/// alone.
///
/// Neither form takes an operand; the side decides only where the answer
/// goes.
fn mem(vm: &mut dyn Vm, side: Side, op: MemOp, prefix: &str) -> Result<(), Error> {
    let m = sys_metrics::memory::get_memory()
        .map_err(|e| Error(format!("{prefix} returns: {e}")))?;
    let mib = match op {
        MemOp::Total => m.total,
        MemOp::Free => m.free,
        MemOp::Used => m.used,
        MemOp::Shared => m.shared,
        MemOp::Buffers => m.buffers,
        MemOp::Cached => m.cached,
    };
    // `as i64` after the two multiplications, exactly where the reference
    // puts the cast.
    side.push(vm, BundValue::int(((mib * 1024) * 1024) as i64));
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.register_native("sysinfo.version", version, eff(0, 1), WordKind::Sync);
    // `virt.rs:37-38`.
    r.register_native(
        "sysinfo.virtualization",
        virtualization_word,
        eff(0, 1),
        WordKind::Sync,
    );
    r.register_native("sysinfo.virtualization?", is_virtualized, eff(0, 1), WordKind::Sync);

    // `host.rs:66-70`. The lower-case word name is the error text, not a
    // prefix -- see `host_string`.
    r.register_native(
        "sysinfo.hostname",
        |vm| {
            host_string(vm, "sysinfo.hostname", || {
                sys_metrics::host::get_hostname().map_err(|e| format!("{e}"))
            })
        },
        eff(0, 1),
        WordKind::Sync,
    );
    r.register_native(
        "sysinfo.kernel_version",
        |vm| {
            host_string(vm, "sysinfo.kernel_version", || {
                sys_metrics::host::get_kernel_version().map_err(|e| format!("{e}"))
            })
        },
        eff(0, 1),
        WordKind::Sync,
    );
    r.register_native(
        "sysinfo.os_version",
        |vm| {
            host_string(vm, "sysinfo.os_version", || {
                sys_metrics::host::get_os_version().map_err(|e| format!("{e}"))
            })
        },
        eff(0, 1),
        WordKind::Sync,
    );
    r.register_native(
        "sysinfo.system",
        |vm| {
            host_string(vm, "sysinfo.system", || {
                sys_metrics::host::get_host_info()
                    .map(|h| h.system)
                    .map_err(|e| format!("{e}"))
            })
        },
        eff(0, 1),
        WordKind::Sync,
    );

    // `mem.rs:91-103`. Twelve registrations, and F154 is the one that passes
    // the wrong operation: `sysinfo.mem.used` reads `free`. Reproduced.
    macro_rules! mem_pair {
        ($base:literal, $prefix:literal, $stack_op:expr, $bench_op:expr) => {
            r.register_native(
                $base,
                |vm| mem(vm, Side::Stack, $stack_op, $prefix),
                eff(0, 1),
                WordKind::Sync,
            );
            r.register_native(
                concat!($base, "."),
                |vm| mem(vm, Side::Bench, $bench_op, concat!($prefix, ".")),
                eff(0, 0),
                WordKind::Sync,
            );
        };
        ($base:literal, $prefix:literal, $op:expr) => {
            mem_pair!($base, $prefix, $op, $op);
        };
    }
    mem_pair!("sysinfo.mem.total", "SYSINFO.MEM.TOTAL", MemOp::Total);
    mem_pair!("sysinfo.mem.free", "SYSINFO.MEM.FREE", MemOp::Free);
    // F154: the stack form reads `Free`, the workbench form reads `Used`.
    mem_pair!(
        "sysinfo.mem.used",
        "SYSINFO.MEM.USED",
        MemOp::Free,
        MemOp::Used
    );
    mem_pair!("sysinfo.mem.shared", "SYSINFO.MEM.SHARED", MemOp::Shared);
    mem_pair!("sysinfo.mem.buffers", "SYSINFO.MEM.BUFFERS", MemOp::Buffers);
    mem_pair!("sysinfo.mem.cached", "SYSINFO.MEM.CACHED", MemOp::Cached);

    r.register_alias("version", "sysinfo.version");
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm;
    use bund2_interp::Interp;

    fn interp() -> Interp {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i
    }

    fn run(i: &mut Interp, src: &str) -> Result<(), String> {
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)
    }

    fn int_of(i: &mut Interp) -> i64 {
        i.pull().and_then(|v| v.as_int()).expect("an integer")
    }

    /// **F154.** `sysinfo.mem.used` reads `free` and `sysinfo.mem.used.` reads
    /// `used`, because one of the reference's twelve registrations passes the
    /// wrong operation.
    ///
    /// The assertion is an *inequality between words*, never a number: the
    /// figures are volatile and machine-specific, which is also why no golden
    /// can witness this. A host whose used and free memory happened to be
    /// equal to the megabyte would make the first assertion vacuous rather
    /// than wrong, so the second one carries the claim.
    #[test]
    fn used_answers_free_and_used_dot_answers_used() {
        let mut i = interp();
        run(&mut i, "sysinfo.mem.free").expect("free");
        let free = int_of(&mut i);
        run(&mut i, "sysinfo.mem.used").expect("used");
        let used_stack = int_of(&mut i);
        run(&mut i, "sysinfo.mem.used.").expect("used.");
        let used_bench = i
            .pull_workbench()
            .and_then(|v| v.as_int())
            .expect("an integer on the workbench");

        // Both read `free`, so they agree to within whatever was allocated
        // between the two calls -- one megabyte is the resolution of the
        // underlying figure.
        let megabyte = 1024 * 1024;
        assert!(
            (free - used_stack).abs() <= 8 * megabyte,
            "`used` should track `free`: free {free}, used {used_stack}"
        );
        // ...and the workbench form reads something else entirely.
        assert_ne!(
            used_bench, used_stack,
            "the two forms of `used` must disagree; that is the defect"
        );
    }

    /// `sys_metrics` reports MiB and the reference multiplies by 1024 twice,
    /// so the answer is bytes. A total that came back in MiB would be about a
    /// million times smaller, which is what this bound catches.
    #[test]
    fn a_total_is_a_byte_count_and_a_multiple_of_a_megabyte() {
        let mut i = interp();
        run(&mut i, "sysinfo.mem.total").expect("total");
        let total = int_of(&mut i);
        assert!(
            total >= 256 * 1024 * 1024,
            "a byte count, not MiB: {total}"
        );
        assert_eq!(total % (1024 * 1024), 0, "MiB scaled up, so exact: {total}");
    }

    /// The four host strings answer something, and the memory words need no
    /// operand -- neither form reads one, the side decides only where the
    /// answer goes.
    #[test]
    fn the_host_facts_answer_and_the_workbench_forms_touch_no_stack() {
        let mut i = interp();
        for w in [
            "sysinfo.hostname",
            "sysinfo.kernel_version",
            "sysinfo.os_version",
            "sysinfo.system",
        ] {
            run(&mut i, w).unwrap_or_else(|e| panic!("{w}: {e}"));
            let s = i.pull().and_then(|v| v.as_str()).expect("a string");
            assert!(!s.is_empty(), "{w} answered an empty string");
        }
        let mut i = interp();
        run(&mut i, "sysinfo.mem.total.").expect("total.");
        assert_eq!(i.depth(), 0, "the stack is untouched");
        assert_eq!(i.workbench_depth(), 1, "the answer is on the workbench");
    }
}
