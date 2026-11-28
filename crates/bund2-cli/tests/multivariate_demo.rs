//! RFC-0020 criterion 10: `bund_examples/multivariate/` reduces, clusters,
//! tests, classifies and estimates a density on one data set, and prints
//! `expected.txt`.
//!
//! The one draw follows an `rng.seed`, so the output is fixed. Standard
//! input is closed, as the Makefile runs it. It is not a golden: the
//! oracle has none of these words (D21).
//!
//! The data set is R 4.6.2's `iris`, which `lda.R` writes into the
//! committed `lda_r.txt` (RFC-0020, "R is not a dependency"). The second
//! test holds the demo's CSV text to that fixture, so the example's input
//! is the one R wrote and no test runs R.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn demo_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bund_examples/multivariate")
}

#[test]
fn the_multivariate_demo_prints_its_expected_output() {
    let dir = demo_dir();
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .arg("demo.bund")
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .expect("bund2 runs");
    let expected = std::fs::read_to_string(dir.join("expected.txt")).expect("expected.txt");
    assert!(
        out.status.success(),
        "the demo failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), expected);
    assert!(
        out.stderr.is_empty(),
        "the demo wrote to stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The bracketed list under `label` in `lda_r.txt`.
fn fixture(text: &str, label: &str) -> Vec<f64> {
    let line = text
        .lines()
        .find(|l| l.split_once(' ').map(|(k, _)| k) == Some(label))
        .unwrap_or_else(|| panic!("lda_r.txt has no {label}"));
    let (_, body) = line.split_once(' ').unwrap();
    body.trim_matches(|c| c == '[' || c == ']')
        .split_whitespace()
        .map(|v| v.parse::<f64>().unwrap())
        .collect()
}

#[test]
fn the_demo_reads_the_iris_r_wrote() {
    let demo = std::fs::read_to_string(demo_dir().join("demo.bund")).expect("demo.bund");
    let header = "\"sepal.length,sepal.width,petal.length,petal.width,species";
    let start = demo.find(header).expect("the CSV header") + header.len();
    let rows: Vec<&str> = demo[start..]
        .lines()
        .skip(1)
        .take_while(|l| !l.starts_with('"'))
        .collect();
    assert_eq!(rows.len(), 150);

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../bund2-stdlib/src/biomath/fixtures/lda_r.txt");
    let text = std::fs::read_to_string(path).expect("lda_r.txt");
    assert!(text.starts_with("# R 4.6.2 "), "lda_r.txt's R version");
    let x = fixture(&text, "iris");
    let groups = fixture(&text, "iris.groups");
    assert_eq!((x.len(), groups.len()), (600, 150));

    let species = ["setosa", "versicolor", "virginica"];
    for (i, row) in rows.iter().enumerate() {
        let cells: Vec<&str> = row.split(',').collect();
        assert_eq!(cells.len(), 5, "row {}", i + 1);
        for j in 0..4 {
            let v: f64 = cells[j].parse().unwrap();
            // Equal as FLOATs: R's 17 digits and the CSV's one parse alike.
            assert_eq!(v, x[4 * i + j], "row {} column {}", i + 1, j + 1);
        }
        let k = species
            .iter()
            .position(|s| *s == cells[4])
            .expect("a species")
            + 1;
        assert_eq!(k as f64, groups[i], "row {}'s species", i + 1);
    }
}
