//! RFC-0015 criteria 2, 3, 6 and 16, for §S13.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{close, err, fixture, floats, run};

const FIXTURE: &str = include_str!("../fixtures/popgen.txt");

fn pulled(src: &str) -> BundValue {
    run(src)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
        .pull()
        .unwrap_or_else(|| panic!("{src}: empty"))
}

fn map(src: &str) -> BTreeMap<String, Vec<f64>> {
    let v = pulled(src);
    v.as_map()
        .unwrap_or_else(|| panic!("{src}: {v:?}"))
        .iter()
        .map(|(k, v)| (k.clone(), floats(v)))
        .collect()
}

fn one(src: &str) -> f64 {
    floats(&pulled(src))[0]
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
        .filter(|n| n.starts_with("popgen."))
        .count();
    assert_eq!(ours, 2 * WORDS.len());
}

// Criterion 16.

/// The RFC's stated values, and every fixture point.
#[test]
fn hwe_against_the_published_algorithm() {
    let m = map("298 489 213 popgen.hwe");
    close("chisq", &m["chisq"], &[0.221489587647], 5e-12);
    close("p.chisq", &m["p.chisq"], &[0.637907263782], 5e-12);
    close("p.exact", &m["p.exact"], &[0.655663488562], 5e-12);
    // 57 heterozygotes, 14 and 50 homozygotes.
    let m = map("14 57 50 popgen.hwe");
    close("p.exact", &m["p.exact"], &[0.842279756571], 5e-12);
    for (aa, ab, bb) in [
        (298, 489, 213),
        (14, 57, 50),
        (10, 0, 10),
        (0, 7, 0),
        (100, 200, 100),
    ] {
        let m = map(&format!("{aa} {ab} {bb} popgen.hwe"));
        let key = format!("hwe.{aa}.{ab}.{bb}");
        for (k, tol) in [
            ("expected", 1e-14),
            ("chisq", 1e-12),
            ("p.chisq", 1e-12),
            ("p.exact", 1e-12),
        ] {
            close(
                &format!("{key}.{k}"),
                &m[k],
                &fixture(FIXTURE, &format!("{key}.{k}")),
                tol,
            );
        }
    }
    // The order of the homozygotes does not matter to the exact test.
    assert_eq!(
        map("213 489 298 popgen.hwe")["p.exact"],
        map("298 489 213 popgen.hwe")["p.exact"]
    );
}

#[test]
fn freq_and_het_by_hand() {
    // AA Ag gg AA: copies A A A g g g A A, so A 5/8 and g 3/8.
    let g = "[ \"AA\" \"Ag\" \"gg\" \"AA\" ]";
    let f = map(&format!("{g} popgen.freq"));
    assert_eq!(f["A"], [0.625]);
    assert_eq!(f["g"], [0.375]);
    assert_eq!(f.len(), 2);
    // One heterozygote of four; 1 − (25 + 9)/64 = 30/64.
    let h = map(&format!("{g} popgen.het"));
    assert_eq!(h["observed"], [0.25]);
    assert_eq!(h["expected"], [0.46875]);
    // Three alleles, each 2/6: every genotype heterozygous, 1 − 3/9.
    let h = map("[ \"12\" \"23\" \"13\" ] popgen.het");
    assert_eq!(h["observed"], [1.0]);
    close("expected", &h["expected"], &[2.0 / 3.0], 1e-15);
}

#[test]
fn fst_by_hand() {
    // p₁ = 0.3, p₂ = 0.6 from 100 copies each:
    // N̂ = 0.09 − 0.21/99 − 0.24/99 = 47/550, D̂ = 0.12 + 0.42 = 27/50,
    // so 47/297.
    close(
        "fst",
        &[one("100 30 100 60 popgen.fst")],
        &[47.0 / 297.0],
        1e-14,
    );
    // The same frequency in both: N̂ is negative, −2·0.16/49 over 0.32.
    close(
        "fst",
        &[one("50 10 50 10 popgen.fst")],
        &[-2.0 * 0.16 / 49.0 / 0.32],
        1e-14,
    );
}

#[test]
fn ld_by_hand() {
    // 40 10 10 40: pAB 0.4, pA = pB = 0.5, D = 0.15, D_max 0.25, r² 0.0225/0.0625.
    let m = map("40 10 10 40 popgen.ld");
    close("D", &m["D"], &[0.15], 1e-15);
    close("D'", &m["D'"], &[0.6], 1e-15);
    close("r2", &m["r2"], &[0.36], 1e-14);
    // Repulsion: D = −0.15, D' = −0.6.
    let m = map("10 40 40 10 popgen.ld");
    close("D", &m["D"], &[-0.15], 1e-15);
    close("D'", &m["D'"], &[-0.6], 1e-15);
    // 30 10 20 40: pA 0.4, pB 0.5, D = 0.3 − 0.2 = 0.1, D_max =
    // min(0.4·0.5, 0.6·0.5) = 0.2, r² = 0.01/(0.4·0.6·0.5·0.5) = 1/6.
    let m = map("30 10 20 40 popgen.ld");
    close("D", &m["D"], &[0.1], 1e-14);
    close("D'", &m["D'"], &[0.5], 1e-14);
    close("r2", &m["r2"], &[1.0 / 6.0], 1e-14);
}

/// Six sequences of ten sites. Site 9 holds a gap and site 10 an N, so both
/// are skipped. Three sites segregate:
///
/// - site 3, G G C C G G: 2 × 4 = 8 pairs differ;
/// - site 5, A A A A T A: 5 pairs;
/// - site 8, T T T A T T: 5 pairs.
///
/// So S = 3 and π = 18/C(6,2) = 18/15 = 6/5. With n = 6, a₁ = 137/60 and
/// a₂ = 5269/3600, so θ_W = 180/137. Tajima's b₁ = 7/15, b₂ = 1/3,
/// c₁ = 7/15 − 60/137 = 59/2055, c₂ = 1/3 − 80/137 + 5269/18769 =
/// 1696/56307, e₁ = c₁/a₁ = 236/18769 and e₂ = c₂/(a₁² + a₂) =
/// 1017600/225584611, so D = (6/5 − 180/137)/√(3e₁ + 6e₂) =
/// (−78/685)/√(14615052/225584611).
const SIX: &str = "[ \"ACGTACGTAC\" \"ACGTACGTAC\" \"ACCTACGTAN\" \
                   \"ACCTACGAAC\" \"ACGTTCGTAC\" \"ACGTACGT-C\" ]";

#[test]
fn diversity_on_six_sequences() {
    close("pi", &[one(&format!("{SIX} popgen.pi"))], &[1.2], 1e-15);
    close(
        "theta.w",
        &[one(&format!("{SIX} popgen.theta.w"))],
        &[180.0 / 137.0],
        1e-15,
    );
    let want = (-78.0 / 685.0) / (14_615_052.0f64 / 225_584_611.0).sqrt();
    close(
        "tajima",
        &[one(&format!("{SIX} popgen.tajima"))],
        &[want],
        1e-13,
    );
    // Lower case and U read the same.
    let lower = SIX.to_lowercase().replace('t', "u");
    close("pi", &[one(&format!("{lower} popgen.pi"))], &[1.2], 1e-15);
}

/// Final frequencies of `reps` trajectories from one seeded generator.
fn finals(seed: i64, reps: usize, call: &str) -> Vec<f64> {
    let src = format!("{seed} rng.seed {}", format!("{call} ").repeat(reps));
    let mut i = run(&src).unwrap_or_else(|e| panic!("{e}"));
    (0..reps)
        .map(|_| *floats(&i.pull().unwrap()).last().unwrap())
        .collect()
}

#[test]
fn wright_fisher_is_unbiased_over_1000_replicates() {
    // Drift does not move the expected frequency: E[p_t] = p.
    let xs = finals(2026, 1000, "100 0.3 50 popgen.wf");
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let sd = (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    let se = sd / n.sqrt();
    assert!(se > 0.0);
    assert!((mean - 0.3).abs() <= 3.0 * se, "mean {mean}, se {se}");
}

#[test]
fn a_trajectory_is_generations_plus_one_draws_of_2n() {
    let path = floats(&pulled("5 rng.seed 10 0.5 30 popgen.wf"));
    assert_eq!(path.len(), 31);
    assert_eq!(path[0], 0.5);
    for p in &path {
        assert!((p * 20.0 - (p * 20.0).round()).abs() < 1e-9, "{p}");
    }
    // Seeded alike, alike.
    assert_eq!(path, floats(&pulled("5 rng.seed 10 0.5 30 popgen.wf")));
    // No generations: just p.
    assert_eq!(floats(&pulled("10 0.25 0 popgen.wf")), [0.25]);
}

#[test]
fn neutral_selection_is_drift_under_the_same_seed() {
    assert_eq!(
        floats(&pulled("9 rng.seed 100 0.3 0.0 0.5 80 popgen.wf.selection")),
        floats(&pulled("9 rng.seed 100 0.3 80 popgen.wf"))
    );
    // Strong selection for A from 0.1 fixes it in a large population.
    let path = floats(&pulled(
        "9 rng.seed 1000 0.1 0.5 0.5 200 popgen.wf.selection",
    ));
    assert!(*path.last().unwrap() > 0.99, "{path:?}");
}

// Criterion 3.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "-1 2 3 popgen.hwe".to_string(),
            "POPGEN.HWE: E101 nAA is outside popgen.hwe's domain: -1".to_string(),
        ),
        (
            "0 0 0 popgen.hwe".into(),
            "POPGEN.HWE: E101 counts is outside popgen.hwe's domain: no individuals".into(),
        ),
        (
            "10 0 0 popgen.hwe".into(),
            "POPGEN.HWE: E101 counts is outside popgen.hwe's domain: a monomorphic sample, whose χ² is undefined"
                .into(),
        ),
        (
            "100000000 1 0 popgen.hwe".into(),
            "POPGEN.HWE: E111 a sample of 100000001 individuals exceeds its cap of 100000000"
                .into(),
        ),
        (
            "[ \"AA\" \"A\" ] popgen.freq".into(),
            "POPGEN.FREQ: E101 genotypes is outside popgen.freq's domain: \"A\"".into(),
        ),
        (
            "10 0 10 0 popgen.fst".into(),
            "POPGEN.FST: E101 counts is outside popgen.fst's domain: both populations fixed for one allele, so D̂ = 0"
                .into(),
        ),
        (
            "10 11 10 0 popgen.fst".into(),
            "POPGEN.FST: E101 k1 is outside popgen.fst's domain: 11".into(),
        ),
        (
            "1 0 10 5 popgen.fst".into(),
            "POPGEN.FST: E101 n1 is outside popgen.fst's domain: 1".into(),
        ),
        (
            "10 10 0 0 popgen.ld".into(),
            "POPGEN.LD: E101 counts is outside popgen.ld's domain: a monomorphic locus, whose D' and r² are undefined"
                .into(),
        ),
        (
            "[ \"ACGT\" \"ACG\" ] popgen.pi".into(),
            "POPGEN.PI: E102 lengths do not agree: 4 and 3".into(),
        ),
        (
            "[ \"ACGT\" \"ACXT\" ] popgen.pi".into(),
            "POPGEN.PI: E109 X at 3 is not in the iupac alphabet".into(),
        ),
        (
            "[ \"ACGT\" \"ACGA\" \"ACGT\" ] popgen.tajima".into(),
            "POPGEN.TAJIMA: E101 alignment is outside popgen.tajima's domain: 3 sequences, where 4 are needed"
                .into(),
        ),
        (
            "[ \"ACGT\" \"ACGT\" \"ACGT\" \"ACGT\" ] popgen.tajima".into(),
            "POPGEN.TAJIMA: E101 alignment is outside popgen.tajima's domain: no segregating site"
                .into(),
        ),
        (
            "1000000000001 0.5 1 popgen.wf".into(),
            "POPGEN.WF: E111 N = 1000000000001 exceeds its cap of 1000000000000".into(),
        ),
        (
            "10 0.5 10000001 popgen.wf".into(),
            "POPGEN.WF: E111 10000001 generations exceeds its cap of 10000000".into(),
        ),
        (
            "10 1.5 10 popgen.wf".into(),
            "POPGEN.WF: E101 p is outside popgen.wf's domain: 1.5".into(),
        ),
        (
            "10 0.5 -1.5 0.5 10 popgen.wf.selection".into(),
            "POPGEN.WF.SELECTION: E101 s is outside popgen.wf.selection's domain: -1.5".into(),
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

// No abort, whatever the operand.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "-1",
        "0",
        "1",
        "2",
        "0.5",
        "\"AA\"",
        "list",
        "[ \"AA\" ]",
        "[ \"\" \"\" \"\" \"\" ]",
        "[ \"A-\" \"AC\" \"GN\" \"TT\" ]",
        "[ 1 2 ]",
    ];
    for w in WORDS {
        for o in operands {
            for k in 1..=5 {
                let src = format!("{} {w}", vec![o; k].join(" "));
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
        ("298 489", "213", "popgen.hwe"),
        ("100 30 100", "60", "popgen.fst"),
        ("40 10 10", "40", "popgen.ld"),
        ("", SIX, "popgen.pi"),
    ] {
        let plain = pulled(&format!("{below} {top_} {w}"));
        let mut i = run(&format!("7 {below} {top_} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        let got = i.pull_workbench().unwrap();
        assert_eq!(format!("{got:?}"), format!("{plain:?}"), "{w}.");
    }
}

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":popgen.pi { 42 } register list popgen.pi").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
