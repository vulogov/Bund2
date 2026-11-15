//! RFC-0015 criteria 2, 3, 6 and 15, for §S12.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::{BLOSUM_SYMBOLS, BLOSUM62, WORDS};
use crate::biomath::testkit::{err, floats, run};

const FIXTURE: &str = include_str!("../fixtures/align.txt");
const NCBI_BLOSUM62: &str = include_str!("../fixtures/BLOSUM62");

fn pulled(src: &str) -> BundValue {
    run(src)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
        .pull()
        .unwrap_or_else(|| panic!("{src}: empty"))
}

/// The `opts` MAP, built as a program builds one.
fn opts(mode: &str, matrix: &str, open: &str, extend: &str) -> String {
    format!(
        "dict \"mode\" \"{mode}\" set \"matrix\" {matrix} set \
         \"gap.open\" {open} set \"gap.extend\" {extend} set"
    )
}

struct Got {
    score: f64,
    a: String,
    b: String,
    start: Vec<f64>,
    end: Vec<f64>,
    identity: f64,
}

fn align(a: &str, b: &str, o: &str) -> Got {
    let src = format!("\"{a}\" \"{b}\" {o} bio.align");
    let v = pulled(&src);
    let m: BTreeMap<String, BundValue> =
        v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone();
    let s = |k: &str| m[k].as_str().unwrap_or_default();
    Got {
        score: floats(&m["score"])[0],
        a: s("a"),
        b: s("b"),
        start: floats(&m["start"]),
        end: floats(&m["end"]),
        identity: floats(&m["identity"])[0],
    }
}

fn blosum(x: char, y: char) -> f64 {
    let i = |c: char| BLOSUM_SYMBOLS.find(c).unwrap();
    f64::from(BLOSUM62[i(x)][i(y)])
}

fn dna(x: char, y: char) -> f64 {
    if x == y { 5.0 } else { -4.0 }
}

/// An alignment's score, recomputed from its gapped strings: each pair by
/// the matrix, each run of gaps as open + (L − 1)·extend, and in semiglobal
/// mode a run at either end of the alignment free.
fn rescore(a: &str, b: &str, s: fn(char, char) -> f64, open: f64, ext: f64, semi: bool) -> f64 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    assert_eq!(a.len(), b.len());
    let mut total = 0.0;
    for row in [(&a, &b), (&b, &a)] {
        let (g, _) = row;
        let mut k = 0;
        while k < g.len() {
            if g[k] == '-' {
                let from = k;
                while k < g.len() && g[k] == '-' {
                    k += 1;
                }
                let free = semi && (from == 0 || k == g.len());
                if !free {
                    total -= open + (k - from - 1) as f64 * ext;
                }
            } else {
                k += 1;
            }
        }
    }
    for (x, y) in a.iter().zip(&b) {
        assert!(!(*x == '-' && *y == '-'), "a column of two gaps");
        if *x != '-' && *y != '-' {
            total += s(*x, *y);
        }
    }
    total
}

// Criterion 2.

#[test]
fn every_word_is_registered_with_its_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
        assert!(i.registry.effect_of(&format!("{w}.")).is_some(), "{w}.");
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("bio.align"))
        .count();
    assert_eq!(ours, 2 * WORDS.len());
}

// Criterion 15.

/// Every line of the fixture: the score Biopython answers, from both words,
/// and an alignment that is the sequences, gapped, and scores what it says.
#[test]
fn every_fixture_score_and_a_consistent_alignment() {
    let mut n = 0;
    for line in FIXTURE.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let [mode, matrix, open, extend, a, b, want] = f[..] else {
            panic!("{line}");
        };
        let want: f64 = want.parse().unwrap();
        let o = opts(mode, &format!("\"{matrix}\""), open, extend);
        let got = align(a, b, &o);
        assert_eq!(got.score, want, "{line}");
        let only = floats(&pulled(&format!("\"{a}\" \"{b}\" {o} bio.align.score")));
        assert_eq!(only, [want], "{line}: bio.align.score");
        // The gapped strings are the sequences, or their aligned stretch.
        let (sa, ea) = (got.start[0] as usize, got.end[0] as usize);
        let (sb, eb) = (got.start[1] as usize, got.end[1] as usize);
        assert_eq!(got.a.replace('-', ""), a[sa..ea], "{line}");
        assert_eq!(got.b.replace('-', ""), b[sb..eb], "{line}");
        if mode != "local" {
            assert_eq!((sa, ea, sb, eb), (0, a.len(), 0, b.len()), "{line}");
        }
        let s = if matrix == "dna" { dna } else { blosum };
        let (open, extend) = (open.parse().unwrap(), extend.parse().unwrap());
        assert_eq!(
            rescore(&got.a, &got.b, s, open, extend, mode == "semiglobal"),
            want,
            "{line}: {} / {}",
            got.a,
            got.b
        );
        n += 1;
    }
    assert_eq!(n, 48);
}

#[test]
fn semiglobal_and_local_by_hand() {
    // ACGTACGT sits inside TTTTACGTACGTAAAA at 4: 8 matches of 5, and the
    // four gaps at each end are free. 8 identical columns of 16.
    let g = align(
        "TTTTACGTACGTAAAA",
        "ACGTACGT",
        &opts("semiglobal", "\"dna\"", "10", "0.5"),
    );
    assert_eq!(g.score, 40.0);
    assert_eq!(g.a, "TTTTACGTACGTAAAA");
    assert_eq!(g.b, "----ACGTACGT----");
    assert_eq!(g.identity, 0.5);
    // The same with the longer sequence second.
    let g = align(
        "ACGT",
        "TTACGTGG",
        &opts("semiglobal", "\"dna\"", "10", "0.5"),
    );
    assert_eq!(
        (g.score, g.a.as_str(), g.b.as_str()),
        (20.0, "--ACGT--", "TTACGTGG")
    );
    // Charged, the same ends cost 10 + 0.5 each run of 2: 20 − 2·10.5.
    let g = align("ACGT", "TTACGTGG", &opts("global", "\"dna\"", "10", "0.5"));
    assert_eq!(g.score, -1.0);
    // Local: CCC against CCC, 3 × 9 under BLOSUM62, at 3..6 and 0..3.
    let g = align(
        "WWWCCCWWW",
        "CCC",
        &opts("local", "\"blosum62\"", "10", "0.5"),
    );
    assert_eq!((g.score, g.a.as_str(), g.b.as_str()), (27.0, "CCC", "CCC"));
    assert_eq!((g.start, g.end), (vec![3.0, 0.0], vec![6.0, 3.0]));
    assert_eq!(g.identity, 1.0);
    // Local with nothing scoring above 0: the empty alignment.
    let g = align("AAAA", "TTTT", &opts("local", "\"dna\"", "10", "0.5"));
    assert_eq!((g.score, g.a.as_str(), g.identity), (0.0, "", 0.0));
    // Lower case reads as upper, and is answered as given.
    let g = align("acgt", "ACGT", &opts("global", "\"dna\"", "10", "0.5"));
    assert_eq!((g.score, g.a.as_str(), g.identity), (20.0, "acgt", 1.0));
}

#[test]
fn a_programs_own_matrix() {
    // A and C, +1 for a match and −1 for a mismatch; CA is AC's score.
    let m = "dict \"AA\" 1 set \"CC\" 1 set \"AC\" -1 set";
    let o = opts("global", m, "2", "1");
    // ACCA against ACA: one gap of 1, three matches, 3 − 2.
    let g = align("ACCA", "ACA", &o);
    assert_eq!(g.score, 1.0);
    let s = |x: char, y: char| if x == y { 1.0 } else { -1.0 };
    assert_eq!(rescore(&g.a, &g.b, s, 2.0, 1.0, false), 1.0);
    assert_eq!(
        floats(&pulled(&format!("\"ACCA\" \"ACA\" {o} bio.align.score"))),
        [1.0]
    );
}

#[test]
fn blosum62_is_ncbis() {
    let mut lines = NCBI_BLOSUM62.lines().filter(|l| !l.starts_with('#'));
    let header: String = lines.next().unwrap().split_whitespace().collect();
    assert_eq!(header, BLOSUM_SYMBOLS);
    let mut rows = 0;
    for (r, line) in lines.enumerate() {
        let f: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(f[0], &BLOSUM_SYMBOLS[r..=r]);
        let cells: Vec<i8> = f[1..].iter().map(|x| x.parse().unwrap()).collect();
        assert_eq!(cells, BLOSUM62[r], "row {}", f[0]);
        rows += 1;
    }
    assert_eq!(rows, 24);
}

// Criterion 3, with criterion 15's cap.

#[test]
fn each_refusal_is_its_template() {
    let dna = opts("global", "\"dna\"", "10", "0.5");
    for (src, want) in [
        (
            format!(
                "\"AC\" \"AC\" {} bio.align",
                opts("glocal", "\"dna\"", "10", "0.5")
            ),
            "BIO.ALIGN: E101 mode is outside bio.align's domain: \"glocal\"".to_string(),
        ),
        (
            "\"AC\" \"AC\" dict \"mode\" \"global\" set bio.align".into(),
            "BIO.ALIGN: E101 opts is outside bio.align's domain: no gap.open".into(),
        ),
        (
            format!(
                "\"AC\" \"AC\" {} bio.align",
                opts("global", "\"dna\"", "10", "0")
            ),
            "BIO.ALIGN: E101 gap.extend is outside bio.align's domain: 0".into(),
        ),
        (
            format!(
                "\"AC\" \"AC\" {} bio.align",
                opts("global", "\"pam250\"", "10", "1")
            ),
            "BIO.ALIGN: E101 matrix is outside bio.align's domain: \"pam250\"".into(),
        ),
        (
            format!(
                "\"AG\" \"AC\" {} bio.align.score",
                opts("global", "dict \"AA\" 1 set \"AC\" 0 set", "10", "1")
            ),
            "BIO.ALIGN.SCORE: E101 matrix is outside bio.align.score's domain: no score for GA"
                .into(),
        ),
        (
            format!(
                "\"AC\" \"AC\" {} bio.align",
                opts("global", "dict \"AC\" 1 set \"CA\" 2 set", "10", "1")
            ),
            "BIO.ALIGN: E101 matrix is outside bio.align's domain: AC is 1 and CA is 2".into(),
        ),
        (
            "42 \"AC\" dict bio.align".into(),
            "BIO.ALIGN: E101 a is outside bio.align's domain: 42".into(),
        ),
        (
            format!("\"ACGU\" \"ACGT\" {dna} bio.align"),
            "BIO.ALIGN: E109 U at 4 is not in the dna alphabet".into(),
        ),
        (
            format!(
                "\"ACJ\" \"AC\" {} bio.align",
                opts("local", "\"blosum62\"", "10", "1")
            ),
            "BIO.ALIGN: E109 J at 3 is not in the blosum62 alphabet".into(),
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

/// 10⁴ against 10⁴ + 1 is past the cap of 10⁸ cells, and is refused before
/// the matrix, or anything sized by n·m, is touched.
#[test]
fn the_cap_refuses_before_allocating() {
    let a = "A".repeat(10_000);
    let b = "A".repeat(10_001);
    for w in WORDS {
        let src = format!(
            "\"{a}\" \"{b}\" {} {w}",
            opts("global", "\"dna\"", "10", "0.5")
        );
        assert_eq!(
            err(&src),
            format!(
                "{}: E111 an alignment of 10000·10001 cells exceeds its cap of 100000000",
                w.to_uppercase()
            )
        );
    }
}

// No abort, whatever the operand.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "-1",
        "0",
        "\"\"",
        "\"ACGT\"",
        "\"é\"",
        "list",
        "dict",
        "[ [ 1 ] ] matrix",
    ];
    let good = [
        opts("global", "\"dna\"", "10", "0.5"),
        opts("local", "\"blosum62\"", "1", "1"),
        opts("semiglobal", "dict \"AA\" 1 set", "1", "1"),
    ];
    for w in WORDS {
        for x in operands {
            for y in operands {
                for o in good.iter().map(String::as_str).chain(operands) {
                    if let Err(e) = run(&format!("{x} {y} {o} {w}")) {
                        assert!(
                            e.contains(": E1") || e.contains("too shallow"),
                            "{x} {y} {o} {w}: {e}"
                        );
                    }
                }
            }
        }
    }
    // Empty sequences align.
    let g = align("", "", &good[0]);
    assert_eq!((g.score, g.a.as_str()), (0.0, ""));
    let g = align("", "ACG", &good[0]);
    assert_eq!((g.score, g.a.as_str(), g.b.as_str()), (-11.0, "---", "ACG"));
}

// The `.` forms.

#[test]
fn each_dot_form_answers_on_the_workbench() {
    let o = opts("local", "\"blosum62\"", "10", "0.5");
    for w in WORDS {
        let plain = pulled(&format!("\"HEAGAWGHEE\" \"PAWHEAE\" {o} {w}"));
        let mut i = run(&format!("7 \"HEAGAWGHEE\" \"PAWHEAE\" {o} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        let got = i.pull_workbench().unwrap();
        assert_eq!(format!("{got:?}"), format!("{plain:?}"), "{w}.");
    }
}

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":bio.align { 42 } register \"A\" \"A\" dict bio.align").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
