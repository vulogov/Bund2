//! Goldens that belong to one host — **D150**.
//!
//! A golden is what the oracle left on the machine it was captured on, and
//! for a few programs that is not what the oracle leaves on another: the last
//! digit of a sine is the platform's maths library, and whether the host
//! reports its memory is the host. No setting in the child's environment
//! reaches either, so `PINNED_HOST` cannot close them.
//!
//! Such a golden has a **variant**: the oracle's capture on the other host,
//! kept at `tests/golden/hosts/<host>/<name>` and read there in place of the
//! shared one. Every other golden is shared, and a host with no variant for a
//! golden reads the shared one, so the denominator is the same everywhere.
//!
//! **A variant is recorded, not found.** `tests/golden/HOSTS.txt` lists each
//! with the reference that approved it, and the two are checked against each
//! other on every load: a file with no row would change what a host is held
//! to without anyone having said so, and a row with no file holds it to
//! nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where the variants live, under `tests/golden/`.
const DIR: &str = "hosts";

/// The host this process runs on, as a variant names it.
pub(crate) fn host() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// A variant's path under `tests/golden/`.
pub(crate) fn variant_name(host: &str, name: &str) -> String {
    format!("{DIR}/{host}/{name}")
}

/// The host and the shared golden a path under `tests/golden/` is a variant
/// of, or `None` for a shared golden.
pub(crate) fn split_variant(path: &str) -> Option<(&str, &str)> {
    path.strip_prefix(DIR)?.strip_prefix('/')?.split_once('/')
}

/// Every recorded variant: (host, golden) to the reference that approved it.
pub(crate) type Rows = BTreeMap<(String, String), String>;

fn list(repo: &Path) -> PathBuf {
    repo.join("tests/golden/HOSTS.txt")
}

/// `<host>\t<golden>\t<reason>` a line. A missing file is no variants.
fn parse(text: &str) -> Rows {
    let mut out = Rows::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split('\t');
        if let (Some(h), Some(g), Some(r)) = (parts.next(), parts.next(), parts.next()) {
            out.insert(
                (h.trim().to_string(), g.trim().to_string()),
                r.trim().to_string(),
            );
        }
    }
    out
}

/// What is wrong between the list and the files, or nothing. `files` are
/// paths under `tests/golden/`.
fn disagreement(rows: &Rows, files: &[String]) -> Option<String> {
    let on_disk: std::collections::BTreeSet<(String, String)> = files
        .iter()
        .filter_map(|f| split_variant(f))
        .map(|(h, g)| (h.to_string(), g.to_string()))
        .collect();
    if let Some((h, g)) = on_disk.iter().find(|k| !rows.contains_key(*k)) {
        return Some(format!(
            "tests/golden/{} is not recorded in tests/golden/HOSTS.txt. A variant \
             changes what {h} is held to, so it is written by \
             `cargo xtask golden --variant {g} --reason <ref>` and by nothing else",
            variant_name(h, g)
        ));
    }
    if let Some((h, g)) = rows.keys().find(|k| !on_disk.contains(*k)) {
        return Some(format!(
            "tests/golden/HOSTS.txt records a variant of {g} for {h}, and \
             tests/golden/{} does not exist",
            variant_name(h, g)
        ));
    }
    None
}

/// The recorded variants, checked against the files.
pub(crate) fn load(repo: &Path) -> Result<Rows, String> {
    let rows = parse(&std::fs::read_to_string(list(repo)).unwrap_or_default());
    let root = repo.join("tests/golden");
    let mut files = Vec::new();
    super::collect_goldens(&root, &root.join(DIR), &mut files);
    match disagreement(&rows, &files) {
        Some(what) => Err(what),
        None => Ok(rows),
    }
}

/// Write the list back, sorted, with its own explanation at the top.
pub(crate) fn save(repo: &Path, rows: &Rows) -> Result<(), String> {
    let mut text = String::from(
        "# Goldens that belong to one host. Written by `cargo xtask golden --variant`.\n\
         #\n\
         # A golden the oracle does not reproduce on another host has a variant\n\
         # there: the oracle's own capture on that host, kept under\n\
         # tests/golden/hosts/<host>/ and read on that host in place of the shared\n\
         # golden. Every golden not listed is shared by every host. D150.\n\
         #\n\
         # <host>\\t<golden>\\t<reason>\n\n",
    );
    for ((host, golden), reason) in rows {
        text.push_str(&format!("{host}\t{golden}\t{reason}\n"));
    }
    std::fs::write(list(repo), text).map_err(|e| format!("writing HOSTS.txt: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(of: &[(&str, &str)]) -> Rows {
        of.iter()
            .map(|(h, g)| ((h.to_string(), g.to_string()), "D150".to_string()))
            .collect()
    }

    #[test]
    fn a_variant_path_splits_back_into_its_host_and_golden() {
        let v = variant_name("linux-x86_64", "probes/x.golden");
        assert_eq!(v, "hosts/linux-x86_64/probes/x.golden");
        assert_eq!(split_variant(&v), Some(("linux-x86_64", "probes/x.golden")));
        assert_eq!(split_variant("probes/x.golden"), None);
        assert_eq!(split_variant("hostsfile/a/b.golden"), None);
    }

    #[test]
    fn the_list_is_read_past_its_comments() {
        let r = parse("# a comment\n\nlinux-x86_64\tprobes/x.golden\tD150\n");
        assert_eq!(r, rows(&[("linux-x86_64", "probes/x.golden")]));
    }

    /// **Both directions.** A file nobody recorded and a record with no file
    /// are each refused, and the two agreeing is not.
    #[test]
    fn the_list_and_the_files_must_agree() {
        let recorded = rows(&[("linux-x86_64", "probes/x.golden")]);
        let file = vec![
            "probes/x.golden".to_string(),
            "hosts/linux-x86_64/probes/x.golden".to_string(),
        ];
        assert_eq!(disagreement(&recorded, &file), None);
        let unrecorded = disagreement(&Rows::new(), &file).expect("a file with no row");
        assert!(unrecorded.contains("is not recorded"), "{unrecorded}");
        let missing =
            disagreement(&recorded, &["probes/x.golden".to_string()]).expect("a row with no file");
        assert!(missing.contains("does not exist"), "{missing}");
    }
}
