//! Refuse to run under a compiler the repository did not ask for.
//!
//! # Why this exists — F80
//!
//! `rust-toolchain.toml` pinned **1.90.0** while the workspace's pinned
//! Cranelift required **1.95.0**, and neither fact surfaced. The pin was wrong
//! *and* it was not in force, because `RUSTUP_TOOLCHAIN` was set in the
//! environment and silently outranks the file. What eventually surfaced was a
//! dependency refusing to build — a message about `cranelift-assembler-x64`,
//! naming neither the pin nor the override.
//!
//! Everything else the repository measures had been running under whichever
//! compiler the environment happened to select: conformance, the benchmarks
//! whose nanoseconds RFC-0001 and RFC-0005 are argued from, all of it. Those
//! numbers were not wrong, but nothing established which compiler produced
//! them.
//!
//! # Why a guard here rather than a build script
//!
//! A shell driver was the alternative. It would duplicate what
//! `rust-toolchain.toml` already does, need maintaining beside it, and **still
//! lose to `RUSTUP_TOOLCHAIN`** unless it unset the variable — so it would
//! solve the general problem by re-implementing rustup and the specific
//! problem by accident.
//!
//! This runs on every `cargo xtask` invocation, which is the entry point the
//! project already uses for every measurement it trusts. A driver only helps
//! the person who remembers to type it.
//!
//! # What it does not do
//!
//! It does not install anything, edit the pin, or set the variable. A tool
//! that silently fixes its own environment makes the next mismatch harder to
//! see, not easier.

use std::path::Path;
use std::process::Command;

/// Set to any non-empty value to run anyway. Deliberately awkward to type, and
/// deliberately present: a guard with no exit becomes a reason to delete the
/// guard.
const OVERRIDE: &str = "BUND2_ALLOW_TOOLCHAIN_MISMATCH";

/// The `channel` from `rust-toolchain.toml`, if it names a concrete version.
///
/// `stable`, `beta`, `nightly` and dated nightlies return `None`: they do not
/// denote one version, so there is nothing to compare and refusing would be
/// noise.
fn pinned(repo: &Path) -> Option<String> {
    let body = std::fs::read_to_string(repo.join("rust-toolchain.toml")).ok()?;
    let line = body
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("channel"))?;
    let value = line.split('=').nth(1)?.trim().trim_matches('"').to_string();
    value
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .map(|_| value)
}

/// The running compiler's version, as `rustc --version` reports it.
fn active() -> Option<String> {
    let out = Command::new("rustc").arg("--version").output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    // "rustc 1.95.0 (59807616e 2026-04-14)" -> "1.95.0"
    text.split_whitespace().nth(1).map(str::to_string)
}

/// `Err` with an explanation if the running compiler is not the pinned one.
pub fn check(repo: &Path) -> Result<(), String> {
    if std::env::var(OVERRIDE).is_ok_and(|v| !v.is_empty()) {
        return Ok(());
    }
    let (Some(want), Some(have)) = (pinned(repo), active()) else {
        // No concrete pin, or no `rustc` on PATH. Neither is this guard's
        // business to decide.
        return Ok(());
    };
    if want == have {
        return Ok(());
    }

    let overridden = std::env::var("RUSTUP_TOOLCHAIN").ok().filter(|v| !v.is_empty());
    let mut msg = format!(
        "toolchain mismatch: rust-toolchain.toml pins {want}, but rustc is {have}.\n\n\
         Every number this tool prints — conformance, coverage, the benchmark\n\
         medians the RFCs argue from — would be produced by a compiler the\n\
         repository did not ask for, and nothing downstream would record which."
    );
    if let Some(v) = overridden {
        msg.push_str(&format!(
            "\n\nRUSTUP_TOOLCHAIN={v} is set in the environment, and it overrides\n\
             rust-toolchain.toml. That is the likely cause. Unset it:\n\n    \
             unset RUSTUP_TOOLCHAIN"
        ));
    } else {
        msg.push_str(&format!(
            "\n\nInstall the pinned toolchain:\n\n    rustup toolchain install {want}"
        ));
    }
    msg.push_str(&format!(
        "\n\nTo run anyway, knowing the numbers are not attributable:\n\n    \
         {OVERRIDE}=1 cargo xtask <command>\n\nSee F80."
    ));
    Err(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A concrete version is compared; a channel name is not.
    #[test]
    fn only_a_concrete_version_is_a_pin() {
        let dir = std::env::temp_dir().join("bund2-xtask-toolchain-test");
        let _ = std::fs::create_dir_all(&dir);
        for (body, want) in [
            ("[toolchain]\nchannel = \"1.95.0\"\n", Some("1.95.0")),
            ("[toolchain]\nchannel = \"stable\"\n", None),
            ("[toolchain]\nchannel = \"nightly-2026-01-01\"\n", None),
        ] {
            std::fs::write(dir.join("rust-toolchain.toml"), body).expect("write");
            assert_eq!(pinned(&dir).as_deref(), want, "for {body:?}");
        }
        // No file at all is not a mismatch.
        let empty = dir.join("empty");
        let _ = std::fs::create_dir_all(&empty);
        assert_eq!(pinned(&empty), None);
        assert!(check(&empty).is_ok(), "no pin means nothing to enforce");
    }

    /// The version is parsed out of `rustc --version`, not matched whole.
    #[test]
    fn the_active_version_is_the_second_field() {
        let v = active().expect("rustc on PATH");
        assert!(
            v.chars().next().is_some_and(|c| c.is_ascii_digit()),
            "parsed {v:?} from `rustc --version`"
        );
    }
}
