//! RFC-0015 criteria 2, 3, 6, 13 and 14, for §S10.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::{CODES, WORDS};
use crate::biomath::testkit::{err, floats, run};

const GC_PRT: &str = include_str!("../fixtures/gc.prt");

fn pulled(src: &str) -> BundValue {
    run(src)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
        .pull()
        .unwrap_or_else(|| panic!("{src}: empty"))
}

fn string(src: &str) -> String {
    let v = pulled(src);
    v.as_str()
        .unwrap_or_else(|| panic!("{src}: not a STRING: {v:?}"))
}

fn int(v: &BundValue) -> i64 {
    v.as_int()
        .unwrap_or_else(|| panic!("not an INTEGER: {v:?}"))
}

fn boolean(src: &str) -> bool {
    match pulled(src).unboxed() {
        BundValue::Bool(b, _) => *b,
        v => panic!("{src}: not a BOOL: {v:?}"),
    }
}

/// A MAP of INTEGERs.
fn counts(src: &str) -> BTreeMap<String, i64> {
    pulled(src)
        .as_map()
        .unwrap_or_else(|| panic!("{src}: not a MAP"))
        .iter()
        .map(|(k, v)| (k.clone(), int(v)))
        .collect()
}

fn want(pairs: &[(&str, i64)]) -> BTreeMap<String, i64> {
    pairs.iter().map(|(k, n)| (k.to_string(), *n)).collect()
}

/// A LIST of MAPs.
fn maps(src: &str) -> Vec<BTreeMap<String, BundValue>> {
    let v = pulled(src);
    v.as_list()
        .unwrap_or_else(|| panic!("{src}: not a LIST"))
        .iter()
        .map(|m| m.as_map().unwrap_or_else(|| panic!("{m:?}")).clone())
        .collect()
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
        .filter(|n| n.starts_with("bio."))
        .count();
    assert_eq!(ours, 2 * WORDS.len());
}

// Criterion 13.

/// Worked example 2.
const PRIMER: &str = "ATGGCCATTGTAATGGGCCGCTGA";

#[test]
fn revcomp_of_every_iupac_code_and_of_the_primer() {
    // A↔T, C↔G, R↔Y, K↔M, B↔V, D↔H; S, W and N are their own.
    assert_eq!(
        string("\"ACGTRYSWKMBDHVN\" bio.complement"),
        "TGCAYRSWMKVHDBN"
    );
    assert_eq!(string("\"ACGTRYSWKMBDHVN\" bio.revcomp"), "NBDHVKMWSRYACGT");
    assert_eq!(string("\"acgtryswkmbdhvn\" bio.revcomp"), "nbdhvkmwsryacgt");
    assert_eq!(
        string(&format!("\"{PRIMER}\" bio.revcomp")),
        "TCAGCGGCCCATTACAATGGCCAT"
    );
}

#[test]
fn the_primer_translates_under_table_1() {
    assert_eq!(string(&format!("\"{PRIMER}\" 1 bio.translate")), "MAIVMGR*");
    // RNA reads as DNA, case is folded, an ambiguous codon is X, and a
    // trailing partial codon is dropped.
    assert_eq!(string("\"augNNNuaaGC\" 1 bio.translate"), "MX*");
    // Table 2 reads AGA as a stop and TGA as W.
    assert_eq!(string("\"AGATGA\" 2 bio.translate"), "*W");
}

#[test]
fn valid_against_each_alphabet() {
    for (s, alphabet, ok) in [
        ("ACGT", "dna", true),
        ("acgt", "dna", true),
        ("ACGU", "dna", false),
        ("ACGU", "rna", true),
        ("ACGT", "rna", false),
        ("MKVW*", "protein", true),
        ("MKVB", "protein", false),
        ("ACGTRYSWKMBDHVN", "iupac", true),
        ("ACGTU", "iupac", false),
        ("AC-GT", "iupac", false),
        ("", "dna", true),
    ] {
        assert_eq!(
            boolean(&format!("\"{s}\" \"{alphabet}\" bio.valid?")),
            ok,
            "{s} {alphabet}"
        );
    }
}

#[test]
fn complement_and_transcription_keep_case() {
    assert_eq!(string("\"AcGtN\" bio.complement"), "TgCaN");
    assert_eq!(string("\"ATGcat\" bio.transcribe"), "AUGcau");
    assert_eq!(string("\"AUGcau\" bio.backtranscribe"), "ATGcat");
    // Any other character is kept.
    assert_eq!(string("\"AT-x\" bio.transcribe"), "AU-x");
}

#[test]
fn gc_over_unambiguous_bases() {
    // G G C C A A T are unambiguous, N is not: 4 of 7.
    assert_eq!(floats(&pulled("\"GGCCAATN\" bio.gc")), [4.0 / 7.0]);
    // U reads as T: G C of A U G C, 2 of 4.
    assert_eq!(floats(&pulled("\"augc\" bio.gc")), [0.5]);
}

#[test]
fn kmers_and_count() {
    // ACGTACG: AC CG GT TA AC CG.
    let two = want(&[("AC", 2), ("CG", 2), ("GT", 1), ("TA", 1)]);
    assert_eq!(counts("\"ACGTACG\" 2 bio.kmers"), two);
    assert_eq!(counts("\"acgTACg\" 2 bio.kmers"), two);
    assert_eq!(counts("\"ACG\" 4 bio.kmers"), want(&[]));
    assert_eq!(
        counts("\"AACGTa\" bio.count"),
        want(&[("A", 3), ("C", 1), ("G", 1), ("T", 1)])
    );
}

#[test]
fn hamming_as_the_string_word() {
    // GGACG against GGTCA: positions 3 and 5 differ.
    assert_eq!(int(&pulled("\"GGACG\" \"GGTCA\" bio.hamming")), 2);
    assert_eq!(
        int(&pulled("\"GGACG\" \"GGTCA\" string.distance.hamming")),
        2
    );
    assert_eq!(int(&pulled("\"acgt\" \"ACGT\" bio.hamming")), 4);
}

#[test]
fn consensus_by_column() {
    // Columns: A A A (gap) → A; C G C C → C; G G C G → G; T T A (gap) → T.
    assert_eq!(
        string("[ \"ACGT\" \"AGGT\" \"ACCA\" \"-CG-\" ] bio.consensus"),
        "ACGT"
    );
    // A tie goes to the earlier symbol: A over C, then A over C again.
    assert_eq!(string("[ \"AC\" \"CA\" ] bio.consensus"), "AA");
    // A column of gaps is a gap; case is folded.
    assert_eq!(string("[ \"a-\" \"c-\" \"A-\" ] bio.consensus"), "A-");
}

/// 120 bases, built of C's and four ORFs placed by hand:
///
/// | bases | holds | strand |
/// |---|---|---|
/// | 10..22 | ATG AAA TTT TAA, M K F | + |
/// | 35..47 | CTA CCC GGG CAT, whose reverse complement is ATG CCC GGG TAG, M P G | − |
/// | 59..68 | ATG GCC TGA, M A | + |
/// | 84..93 | TCA AAA CAT, ATG TTT TGA reversed, M F | − |
///
/// The C's between are P on the forward strand and G on the reverse, and
/// neither strand holds another ATG with a stop after it in frame.
fn orf_fixture() -> String {
    let c = |n| "C".repeat(n);
    let s = format!(
        "{}ATGAAATTTTAA{}CTACCCGGGCAT{}ATGGCCTGA{}TCAAAACAT{}",
        c(10),
        c(13),
        c(12),
        c(16),
        c(27)
    );
    assert_eq!(s.len(), 120);
    s
}

fn orfs(s: &str, min: i64) -> Vec<(i64, i64, String, String)> {
    maps(&format!("\"{s}\" {min} bio.orfs"))
        .into_iter()
        .map(|m| {
            let text = |k: &str| m[k].as_str().unwrap_or_default();
            (
                int(&m["start"]),
                int(&m["end"]),
                text("strand"),
                text("protein"),
            )
        })
        .collect()
}

#[test]
fn orfs_on_both_strands() {
    let s = orf_fixture();
    let o = |a, e, strand: &str, p: &str| (a, e, strand.to_string(), p.to_string());
    assert_eq!(
        orfs(&s, 1),
        [
            o(10, 22, "+", "MKF"),
            o(35, 47, "-", "MPG"),
            o(59, 68, "+", "MA"),
            o(84, 93, "-", "MF"),
        ]
    );
    // At least 3 codons: the two of two codons drop out.
    assert_eq!(orfs(&s, 3), [o(10, 22, "+", "MKF"), o(35, 47, "-", "MPG")]);
    // Lower case reads the same.
    assert_eq!(orfs(&s.to_lowercase(), 1).len(), 4);
}

/// Five sites, six columns:
///
/// ```text
/// ACGTAC  ACGTTC  ACGAAC  TCGTAG  ACCTAC
/// ```
///
/// Column counts: A4 T1 | C5 | G4 C1 | T4 A1 | A4 T1 | C4 G1. With 0.5
/// added to each count of 5 sites, a frequency is (c + 0.5)/7, and against
/// 0.25 the score is log₂(4(c + 0.5)/7): c = 0 gives log₂(2/7), 1 log₂(6/7),
/// 3 exactly 1, 4 log₂(18/7), 5 log₂(22/7).
const SITES: &str = "[ \"ACGTAC\" \"ACGTTC\" \"ACGAAC\" \"TCGTAG\" \"ACCTAC\" ]";

fn l(c: f64) -> f64 {
    (4.0 * (c + 0.5) / 7.0).log2()
}

#[test]
fn pwm_by_hand() {
    let m = pulled(&format!("{SITES} bio.pwm"));
    let rows: Vec<Vec<f64>> = m
        .as_matrix()
        .unwrap_or_else(|| panic!("{m:?}"))
        .iter()
        .map(|r| r.iter().flat_map(floats).collect())
        .collect();
    let want = [
        // A
        [4.0, 0.0, 0.0, 1.0, 4.0, 0.0],
        // C
        [0.0, 5.0, 1.0, 0.0, 0.0, 4.0],
        // G
        [0.0, 0.0, 4.0, 0.0, 0.0, 1.0],
        // T
        [1.0, 0.0, 0.0, 4.0, 1.0, 0.0],
    ];
    for (r, counts) in want.iter().enumerate() {
        for (j, &c) in counts.iter().enumerate() {
            assert!((rows[r][j] - l(c)).abs() <= 1e-15, "[{r}][{j}]: {rows:?}");
        }
    }
    assert_eq!(l(3.0), 1.0);
}

#[test]
fn pwm_scan_by_hand() {
    // TTACGTACGG's five windows, each column's count of the window's base:
    // 0 TTACGT 1 0 0 0 0 0; 1 TACGTA 1 0 1 0 1 0; 2 ACGTAC 4 5 4 4 4 4;
    // 3 CGTACG 0 0 0 1 0 1; 4 GTACGG 0 0 0 0 0 1.
    let scores = [
        l(1.0) + 5.0 * l(0.0),
        3.0 * l(1.0) + 3.0 * l(0.0),
        5.0 * l(4.0) + l(5.0),
        2.0 * l(1.0) + 4.0 * l(0.0),
        l(1.0) + 5.0 * l(0.0),
    ];
    let scan = |threshold: &str| -> Vec<(i64, f64)> {
        maps(&format!(
            "\"TTACGTACGG\" {SITES} bio.pwm {threshold} bio.pwm.scan"
        ))
        .into_iter()
        .map(|m| (int(&m["pos"]), floats(&m["score"])[0]))
        .collect()
    };
    let all = scan("-100.0");
    assert_eq!(all.len(), 5);
    for ((pos, got), want) in all.iter().zip(scores) {
        assert!((got - want).abs() <= 1e-13, "{pos}: {got} against {want}");
    }
    let above = scan("0.0");
    assert_eq!(above.len(), 1);
    assert_eq!(above[0].0, 2);
    // A window holding an ambiguous base is not scored.
    let n = maps(&format!("\"ACGTACN\" {SITES} bio.pwm -100.0 bio.pwm.scan"));
    assert_eq!(n.len(), 1);
}

// Criterion 14.

/// Every table in the committed `gc.prt`: its 64 codons, spelled from the
/// file's own Base1–Base3 lines, translate to its `ncbieaa` row.
#[test]
fn every_table_in_gc_prt() {
    let body = &GC_PRT[GC_PRT.find("Genetic-code-table").unwrap()..];
    let field = |block: &str, key: &str| -> String {
        let at = block.find(key).unwrap_or_else(|| panic!("{key}: {block}"));
        let rest = block[at + key.len()..].trim_start();
        match rest.strip_prefix('"') {
            Some(q) => q[..q.find('"').unwrap()].to_string(),
            None => rest.split_whitespace().next().unwrap().to_string(),
        }
    };
    let mut ids = Vec::new();
    for block in body.split('}').filter(|b| b.contains(" id ")) {
        let id: i64 = field(block, " id ").trim_end_matches(',').parse().unwrap();
        let aa = field(block, "ncbieaa");
        let base = |k| field(block, &format!("-- Base{k}"));
        let (b1, b2, b3) = (base(1), base(2), base(3));
        let codons: String = (0..64)
            .flat_map(|i| [&b1, &b2, &b3].map(|b| char::from(b.as_bytes()[i])))
            .collect();
        assert_eq!(codons.len(), 192, "table {id}");
        assert_eq!(
            string(&format!("\"{codons}\" {id} bio.translate")),
            aa,
            "table {id}"
        );
        ids.push(id);
    }
    let ours: Vec<i64> = CODES.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, ours);
    assert!(GC_PRT.contains("Version 4.6"));
}

// Criterion 3.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "\"ACGT\" \"dnA\" bio.valid?".to_string(),
            "BIO.VALID?: E101 alphabet is outside bio.valid?'s domain: \"dnA\"".to_string(),
        ),
        (
            "\"ACGT\" 7 bio.translate".into(),
            "BIO.TRANSLATE: E101 table is outside bio.translate's domain: 7".into(),
        ),
        (
            "\"ACGT\" 0 bio.kmers".into(),
            "BIO.KMERS: E101 k is outside bio.kmers's domain: 0".into(),
        ),
        (
            "\"ACGT\" 33 bio.kmers".into(),
            "BIO.KMERS: E101 k is outside bio.kmers's domain: 33".into(),
        ),
        (
            "\"NNRY\" bio.gc".into(),
            "BIO.GC: E101 s is outside bio.gc's domain: no unambiguous base".into(),
        ),
        (
            "42 bio.revcomp".into(),
            "BIO.REVCOMP: E101 s is outside bio.revcomp's domain: 42".into(),
        ),
        (
            "\"ACGT\" -1 bio.orfs".into(),
            "BIO.ORFS: E101 min is outside bio.orfs's domain: -1".into(),
        ),
        (
            "list bio.consensus".into(),
            "BIO.CONSENSUS: E101 sequences is outside bio.consensus's domain: list/0".into(),
        ),
        (
            "\"AC\" \"ACG\" bio.hamming".into(),
            "BIO.HAMMING: E102 lengths do not agree: 2 and 3".into(),
        ),
        (
            "[ \"ACGT\" \"ACG\" ] bio.consensus".into(),
            "BIO.CONSENSUS: E102 lengths do not agree: 4 and 3".into(),
        ),
        (
            "\"ACGU\" bio.revcomp".into(),
            "BIO.REVCOMP: E109 U at 4 is not in the iupac alphabet".into(),
        ),
        (
            "\"ACGX\" bio.gc".into(),
            "BIO.GC: E109 X at 4 is not in the iupac alphabet".into(),
        ),
        (
            "[ \"ACGT\" \"ACNT\" ] bio.pwm".into(),
            "BIO.PWM: E109 N at 3 is not in the dna alphabet".into(),
        ),
        (
            "\"ACGT\" [ [ 1 2 ] [ 3 4 ] ] matrix 0 bio.pwm.scan".into(),
            "BIO.PWM.SCAN: E101 pwm is outside bio.pwm.scan's domain: a MATRIX of 2 rows, where A C G T are 4"
                .into(),
        ),
        (
            "\"ACGT\" [ [ 1 ] [ 1 ] [ 1 ] [ \"x\" ] ] matrix 0 bio.pwm.scan".into(),
            "BIO.PWM.SCAN: E103 pwm is not a MATRIX of numbers: \"x\"".into(),
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

// Criterion 5's spirit: no abort, whatever the operand.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "-1",
        "0",
        "1",
        "100",
        "\"\"",
        "\"ACGT\"",
        "\"é\"",
        "list",
        "[ \"\" ]",
        "[ \"AC\" \"GT\" ]",
        "[ 1 2 ]",
        "[ [ 1 ] ] matrix",
        "[ [ 1 ] [ 1 ] [ 1 ] [ 1 ] ] matrix",
    ];
    for w in WORDS {
        for o in operands {
            for src in [
                format!("{o} {w}"),
                format!("{o} {o} {w}"),
                format!("{o} {o} {o} {w}"),
            ] {
                if let Err(e) = run(&src) {
                    assert!(
                        e.contains(": E1") || e.contains("too shallow"),
                        "{src}: {e}"
                    );
                }
            }
        }
    }
}

// The `.` forms.

#[test]
fn each_dot_form_answers_on_the_workbench() {
    for (below, top_, w) in [
        ("", "\"ACGTT\"", "bio.revcomp"),
        (&format!("\"{PRIMER}\""), "1", "bio.translate"),
        ("\"ACGTACG\"", "2", "bio.kmers"),
        ("\"GGACG\"", "\"GGTCA\"", "bio.hamming"),
        ("\"ACGT\"", "\"dna\"", "bio.valid?"),
    ] {
        let plain = pulled(&format!("{below} {top_} {w}"));
        let mut i = run(&format!("7 {below} {top_} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        let got = i.pull_workbench().unwrap();
        assert_eq!(format!("{got:?}"), format!("{plain:?}"), "{w}.");
    }
    // Three operands: the threshold from the workbench.
    let plain = pulled(&format!("\"TTACGTACGG\" {SITES} bio.pwm 0.0 bio.pwm.scan"));
    let mut i = run(&format!(
        "\"TTACGTACGG\" {SITES} bio.pwm 0.0 . bio.pwm.scan."
    ))
    .unwrap();
    assert_eq!(i.snapshot().len(), 0);
    let got = i.pull_workbench().unwrap();
    assert_eq!(format!("{got:?}"), format!("{plain:?}"));
}

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":bio.revcomp { 42 } register \"A\" bio.revcomp").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
