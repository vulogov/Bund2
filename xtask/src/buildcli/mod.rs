//! Build the `bund2` binary a measurement is about, and say which one it is.
//!
//! # Why this is one function and not five
//!
//! Five subcommands measure the binary — `conform`, `bench`, `effects`,
//! `corpus`/`coverage` and `depth` — and each grew its own copy of "run cargo
//! build, then join a path". The copies drifted in the way copies do: only
//! `depth` learned `--features`, and it learned it because RFC-0005 needed one
//! criterion to run.
//!
//! The cost of that drift was a wrong claim, not an inconvenience. RFC-0005
//! recorded "criterion 2 has run: 73/86 with the feature on, 73/86 with it
//! off". It had not. `conform` rejects `--features`, and its builder ran
//! `cargo build -p bund2-cli` **unconditionally**, so a binary built with the
//! feature beforehand was rebuilt without it and then measured. Both numbers
//! came from the same non-jit binary, and nothing in the output said so.
//!
//! That is F80's shape for the third time: a measurement whose subject is
//! whatever happened to be in `target/`. Fixing it per-subcommand is what
//! produced the drift.

use std::path::{Path, PathBuf};

/// Pull `--features <list>` out of an argument vector, leaving the rest.
///
/// Every subcommand that measures the binary accepts it, so the parsing lives
/// here too rather than being written out five times with five spellings.
pub fn take_features(args: &[String]) -> (String, Vec<String>) {
    let mut features = String::new();
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--features" {
            if let Some(v) = it.next() {
                features = v.clone();
            }
        } else {
            rest.push(a.clone());
        }
    }
    (features, rest)
}

/// Pull `--jit-threshold <n>` out of an argument vector, leaving the rest.
///
/// **RFC-0005 criterion 2's third run needs it** — `cargo xtask conform
/// --features jit --jit-threshold 1`, so every body compiles on its first
/// evaluation. Parsed here beside `--features` for the same reason: every
/// subcommand that measures the binary should spell it one way.
///
/// A value that is not a number is refused rather than ignored. This is a
/// command line, which *can* report — unlike `BUND2_JIT_THRESHOLD`, which the
/// runtime reads in a constructor and must ignore when malformed.
pub fn take_jit_threshold(args: &[String]) -> Result<(Option<u32>, Vec<String>), String> {
    let mut threshold = None;
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--jit-threshold" {
            let v = it
                .next()
                .ok_or("--jit-threshold needs a number, as in `--jit-threshold 1`")?;
            threshold = Some(
                v.parse::<u32>()
                    .map_err(|_| format!("--jit-threshold takes a number, not `{v}`"))?,
            );
        } else {
            rest.push(a.clone());
        }
    }
    Ok((threshold, rest))
}

/// Build `bund2-cli` and return the binary, or say why not.
///
/// **Builds, never finds.** A path that happens to exist is not evidence about
/// the code being measured.
pub fn bund2(repo: &Path, release: bool, features: &str) -> Result<PathBuf, String> {
    let mut cmd =
        std::process::Command::new(std::env::var("CARGO").as_deref().unwrap_or("cargo"));
    cmd.args(["build", "-q", "-p", "bund2-cli"]);
    if release {
        cmd.arg("--release");
    }
    if !features.is_empty() {
        cmd.args(["--features", features]);
    }
    let status = cmd
        .current_dir(repo)
        .status()
        .map_err(|e| format!("building bund2-cli: {e}"))?;
    if !status.success() {
        return Err(format!(
            "bund2-cli failed to build{}; nothing was measured",
            if features.is_empty() {
                String::new()
            } else {
                format!(" with --features {features}")
            }
        ));
    }
    let path = repo.join(if release {
        "target/release/bund2"
    } else {
        "target/debug/bund2"
    });
    if !path.is_file() {
        return Err(format!(
            "no bund2 at {} after a successful build",
            path.strip_prefix(repo).unwrap_or(&path).display()
        ));
    }
    Ok(path)
}

/// One line naming the binary a report is about, for the report to print.
///
/// A number without this is not attributable, which is the whole lesson of
/// F80 and of the `conform --features` defect above.
pub fn provenance(release: bool, features: &str) -> String {
    provenance_with(release, features, None)
}

/// [`provenance`], naming §S7's threshold when a run overrode it.
///
/// The same number at threshold 64 and at threshold 1 means two different
/// things — at 64 most corpus programs compile nothing — so a report that
/// quotes one without the other is not attributable (F125).
pub fn provenance_with(release: bool, features: &str, threshold: Option<u32>) -> String {
    format!(
        "bund2-cli, {} profile, features: {}{}",
        if release { "release" } else { "dev" },
        if features.is_empty() {
            "(none)"
        } else {
            features
        },
        match threshold {
            Some(n) => format!(", jit-threshold: {n}"),
            None => String::new(),
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_are_taken_out_of_the_arguments() {
        let args: Vec<String> = ["--accept", "x", "--features", "jit", "--reason", "y"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (f, rest) = take_features(&args);
        assert_eq!(f, "jit");
        assert_eq!(rest, vec!["--accept", "x", "--reason", "y"]);
    }

    /// A missing value must not swallow the next argument as a feature list.
    #[test]
    fn a_bare_features_flag_yields_nothing() {
        let (f, rest) = take_features(&["--features".to_string()]);
        assert_eq!(f, "");
        assert!(rest.is_empty());
    }

    #[test]
    fn provenance_names_the_profile_and_the_features() {
        assert_eq!(
            provenance(true, "jit"),
            "bund2-cli, release profile, features: jit"
        );
        assert_eq!(
            provenance(false, ""),
            "bund2-cli, dev profile, features: (none)"
        );
    }
}
