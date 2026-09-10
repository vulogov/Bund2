//! `sysinfo.*` — facts about the host and the build.
//!
//! Only `sysinfo.version` so far, because it is the one the corpus reaches:
//! `if_word.bund` asks `?word` about `version`, which is an alias for it
//! (`reference/Bund/src/stdlib/functions/create_aliases.rs:32`). The rest of
//! the family reads hostname, CPU and memory — machine-specific, so no golden
//! can capture them and nothing yet needs them.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;

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

pub fn register(r: &mut Registry) {
    r.register_native("sysinfo.version", version, eff(0, 1), WordKind::Sync);
    r.register_alias("version", "sysinfo.version");
}
