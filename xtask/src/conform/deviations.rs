//! The deviation register — **F48**.
//!
//! `conform` compares captured bytes and counts equality. A golden that Bund2
//! is *approved* to disagree with therefore fails forever, indistinguishable in
//! the report from a regression — and the baseline ratchet then treats
//! implementing an approved deviation as a drop, so the tool reports a
//! regression for doing what a decision instructed.
//!
//! CLAUDE.md names three dispositions for a failing golden, and the third is
//! "a deviation already approved in the work item". This is the mechanism that
//! disposition never had.
//!
//! # What a row records
//!
//! A golden name, the **approving reference** — an F-number or a decision —
//! and a **hash of Bund2's expected output**. The hash is the point: a bare
//! exclusion stops checking, while a hash keeps checking against the right
//! thing, so an *unintended* change to a deviating golden still fails.
//!
//! # What it is not
//!
//! Not a way to make a golden pass by regenerating it. `tests/golden/` is the
//! oracle's record and stays the oracle's record; a deviation says "Bund2
//! answers differently here, on purpose, and here is the answer it must keep
//! giving". Approved deviations are counted and reported **separately** and
//! never folded into the ratio.

use std::collections::BTreeMap;
use std::path::Path;

/// One approved deviation.
#[derive(Debug, Clone)]
pub struct Deviation {
    /// The golden this concerns, as `conform` names it.
    pub golden: String,
    /// The F-number or decision that approved it.
    pub reason: String,
    /// A hash of the output Bund2 is expected to produce instead.
    pub expected: u64,
}

/// A cheap, stable content hash.
///
/// Stable across runs and machines is all that is needed — this identifies an
/// output, it does not defend against anyone.
pub fn hash(s: &str) -> u64 {
    // FNV-1a, written out so the value cannot drift with a std change.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn path(repo: &Path) -> std::path::PathBuf {
    repo.join("tests/golden/DEVIATIONS.txt")
}

/// Read the register. A missing file is an empty register, not an error.
pub fn load(repo: &Path) -> BTreeMap<String, Deviation> {
    let mut out = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(path(repo)) else {
        return out;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // `<golden>\t<reason>\t<hash>`
        let mut parts = line.split('\t');
        let (Some(g), Some(r), Some(h)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let Ok(expected) = h.trim().parse::<u64>() else {
            continue;
        };
        out.insert(
            g.trim().to_string(),
            Deviation {
                golden: g.trim().to_string(),
                reason: r.trim().to_string(),
                expected,
            },
        );
    }
    out
}

/// Write the register back, sorted, with its own explanation at the top.
pub fn save(repo: &Path, rows: &BTreeMap<String, Deviation>) -> Result<(), String> {
    let mut text = String::new();
    text.push_str(
        "# Approved deviations. Written by `cargo xtask conform --accept-deviation`.\n\
         #\n\
         # A golden Bund2 is approved to disagree with, the decision or defect that\n\
         # approved it, and a hash of the output Bund2 must keep producing. The hash\n\
         # is why this is not an exclusion: an unintended change to a deviating\n\
         # golden still fails.\n\
         #\n\
         # These are reported separately and never folded into the conformance\n\
         # ratio. `tests/golden/` remains the oracle's record; nothing here edits it.\n\
         #\n\
         # <golden>\\t<reason>\\t<hash of bund2's expected output>\n\n",
    );
    for d in rows.values() {
        text.push_str(&format!("{}\t{}\t{}\n", d.golden, d.reason, d.expected));
    }
    std::fs::write(path(repo), text).map_err(|e| format!("writing DEVIATIONS.txt: {e}"))
}

/// How a golden's outcome relates to the register.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Not a deviation; judge it normally.
    NotDeviating,
    /// Approved, and Bund2 produced what it was recorded as producing.
    Approved,
    /// Approved, but Bund2's output has **changed** since it was recorded.
    /// A regression inside a deviation, which an exclusion would have hidden.
    Drifted,
}

pub fn judge(rows: &BTreeMap<String, Deviation>, golden: &str, got: &str) -> Verdict {
    match rows.get(golden) {
        None => Verdict::NotDeviating,
        Some(d) if d.expected == hash(got) => Verdict::Approved,
        Some(_) => Verdict::Drifted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hash_is_stable_and_discriminating() {
        assert_eq!(hash("abc"), hash("abc"));
        assert_ne!(hash("abc"), hash("abd"));
        assert_ne!(hash(""), hash("a"));
    }

    fn one(reason: &str, out: &str) -> BTreeMap<String, Deviation> {
        let mut m = BTreeMap::new();
        m.insert(
            "probes/x.golden".to_string(),
            Deviation {
                golden: "probes/x.golden".into(),
                reason: reason.into(),
                expected: hash(out),
            },
        );
        m
    }

    #[test]
    fn an_unlisted_golden_is_judged_normally() {
        let rows = one("F33", "expected");
        assert_eq!(
            judge(&rows, "probes/other.golden", "anything"),
            Verdict::NotDeviating
        );
    }

    #[test]
    fn a_listed_golden_producing_its_recorded_output_is_approved() {
        let rows = one("F33", "expected");
        assert_eq!(judge(&rows, "probes/x.golden", "expected"), Verdict::Approved);
    }

    /// **The reason this stores a hash rather than an exclusion.** A deviating
    /// golden whose output changes is a regression inside a deviation, and an
    /// exclusion would have hidden it.
    #[test]
    fn a_listed_golden_whose_output_changed_has_drifted() {
        let rows = one("F33", "expected");
        assert_eq!(
            judge(&rows, "probes/x.golden", "something else"),
            Verdict::Drifted
        );
    }

    #[test]
    fn a_missing_register_is_empty_not_an_error() {
        let empty = load(Path::new("/nonexistent-repo-path"));
        assert!(empty.is_empty());
    }
}
