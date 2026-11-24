//! RFC-0015 criteria 2, 3, 5, 6, 10 and 11, for §S7 and §S8; RFC-0019
//! criteria 1, 2 and 14 for §S2's noncentral families.

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{close, err, floats, run, top};

const FIXTURE: &str = include_str!("../fixtures/dist.txt");

/// Each family with parameters it accepts, as a program that pushes it.
const FAMILIES: [&str; 20] = [
    "1.0 2.0 dist.normal",
    "0.5 0.75 dist.lognormal",
    "-1.0 3.0 dist.uniform",
    "1.5 dist.exponential",
    "2.5 1.5 dist.gamma",
    "2.0 3.5 dist.beta",
    "4.5 dist.chisq",
    "5.5 dist.t",
    "4.0 7.5 dist.f",
    "20 0.3 dist.binomial",
    "4.2 dist.poisson",
    "3.5 0.4 dist.negbinomial",
    "50 12 10 dist.hypergeometric",
    "0.2 dist.geometric",
    "0.3 dist.bernoulli",
    "6 0.2 list + 0.3 swap + 0.5 swap + dist.multinomial",
    "2.0 list + 3.0 swap + 4.0 swap + dist.dirichlet",
    // RFC-0019 §S2.
    "5.5 1.5 dist.nct",
    "4.5 2.5 dist.ncchisq",
    "4.0 7.5 3.0 dist.ncf",
];

const CONTINUOUS: usize = 9;

fn one(src: &str) -> f64 {
    let v = top(src);
    assert_eq!(v.len(), 1, "{src}: {v:?}");
    v[0]
}

// Criterion 2.

#[test]
fn every_word_is_registered_with_its_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS.iter().chain(&["rng.seed"]) {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
        assert!(i.registry.effect_of(&format!("{w}.")).is_some(), "{w}.");
    }
    // A producer has no `.` form (§S3, D24).
    assert!(i.registry.effect_of("rng.uniform").is_some());
    assert!(i.registry.effect_of("rng.uniform.").is_none());
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("dist.") || n.starts_with("rng."))
        // RFC-0016 §S7's sampling words, which `sample`'s own test holds.
        .filter(|n| !crate::biomath::sample::WORDS.contains(&n.as_str()))
        .count();
    // `rng.seed.`, `rng.uniform`, and RFC-0019 §S7's `rng.derive`, a
    // producer with no `.` form.
    assert_eq!(ours, 2 * WORDS.len() + 4);
}

// Criterion 10.

#[test]
fn a_constructor_pushes_its_map() {
    let mut i = run("1.0 2.0 dist.normal").unwrap();
    let v = i.pull().unwrap();
    assert_eq!(v.display(), "{ dist=normal ::  mean=1.0 ::  sd=2.0 :: }");
    // One built by hand is the same distribution.
    let by_hand = "0.5 dict \"dist\" \"normal\" set \"mean\" 1.0 set \"sd\" 2.0 set dist.pdf";
    assert_eq!(one(by_hand), one("0.5 1.0 2.0 dist.normal dist.pdf"));
}

/// Every program in the committed fixture, against mpmath's answer: a
/// density to 1e-12 (B11's closed form); a tail to 1e-12 where it is a closed
/// form and 1e-8 where it is a series or continued fraction; a quantile to
/// 1e-8, the polish being iterative, and a discrete one exactly.
#[test]
fn each_family_against_the_fixture() {
    let closed_tails = [
        "dist.normal",
        "dist.lognormal",
        "dist.uniform",
        "dist.exponential",
        "dist.geometric",
        "dist.bernoulli",
    ];
    let mut seen = 0;
    for line in FIXTURE.lines().filter(|l| l.contains(" dist.")) {
        let (program, want) = line.rsplit_once(' ').unwrap();
        let want: f64 = want.parse().unwrap();
        let family = program.split(' ').rev().nth(1).unwrap();
        let tol = if program.ends_with("dist.pdf") {
            1e-12
        } else if program.ends_with("dist.quantile") {
            1e-8
        } else if closed_tails.contains(&family) {
            1e-12
        } else {
            1e-8
        };
        close(program, &[one(program)], &[want], tol);
        seen += 1;
    }
    assert_eq!(seen, 177);
}

const STUDENT: &str = include_str!("../fixtures/student.txt");

/// The t at a large df and near p = ½, where its cdf answered ½ (at 1e30)
/// and its quantile failed to polish (from 2e6, and near ½): student.py's
/// mpmath values, independent of the incomplete beta. Density and tails to
/// 1e-12; a quantile to 1e-9, or within 1e-15 absolutely below 1e-6, since
/// near ½ every x within ulp(p)/pdf ≈ 3e-16 of the quantile is the same p.
#[test]
fn the_t_at_a_large_df_and_near_one_half() {
    let mut seen = 0;
    for line in STUDENT.lines() {
        let (program, want) = line.rsplit_once(' ').unwrap();
        let want: f64 = want.parse().unwrap();
        let got = one(program);
        if program.ends_with("dist.quantile") && want.abs() < 1e-6 {
            assert!(
                (got - want).abs() <= 1e-15,
                "{program}: {got:e} against {want:e}"
            );
        } else {
            let tol = if program.ends_with("dist.quantile") {
                1e-9
            } else {
                1e-12
            };
            close(program, &[got], &[want], tol);
        }
        seen += 1;
    }
    assert_eq!(seen, 102);
}

#[test]
fn the_cdf_of_the_quantile_is_p() {
    for d in &FAMILIES[..CONTINUOUS] {
        for p in [0.001, 0.5, 0.999] {
            let got = one(&format!("{p} {d} dist.quantile {d} dist.cdf"));
            assert!((got - p).abs() <= 1e-10, "{d} at {p}: {got}");
        }
    }
}

#[test]
fn the_mappings_to_statrs_are_rs() {
    // dgeom counts failures: P(X = 0) = p, where statrs's trials start at 1.
    close(
        "geom",
        &[one("0 0.2 dist.geometric dist.pdf")],
        &[0.2],
        1e-15,
    );
    // dhyper(x, m = K, n = N - K, k = n): 2 of 10 drawn from 50 holding 12,
    // C(12, 2)·C(38, 8) / C(50, 10).
    let want = 66.0 * 48_903_492.0 / 10_272_278_170.0;
    close(
        "hyper",
        &[one("2 50 12 10 dist.hypergeometric dist.pdf")],
        &[want],
        1e-12,
    );
}

#[test]
fn the_moments_against_their_closed_forms() {
    let cases: [(&str, f64, f64); 15] = [
        ("1.0 2.0 dist.normal", 1.0, 4.0),
        (
            "0.5 0.75 dist.lognormal",
            (0.5f64 + 0.28125).exp(),
            0.5625f64.exp_m1() * (1.0f64 + 0.5625).exp(),
        ),
        ("-1.0 3.0 dist.uniform", 1.0, 16.0 / 12.0),
        ("1.5 dist.exponential", 1.0 / 1.5, 1.0 / 2.25),
        ("2.5 1.5 dist.gamma", 2.5 / 1.5, 2.5 / 2.25),
        ("2.0 3.5 dist.beta", 2.0 / 5.5, 7.0 / (5.5 * 5.5 * 6.5)),
        ("4.5 dist.chisq", 4.5, 9.0),
        ("5.5 dist.t", 0.0, 5.5 / 3.5),
        (
            "4.0 7.5 dist.f",
            7.5 / 5.5,
            2.0 * 56.25 * 9.5 / (4.0 * 5.5 * 5.5 * 3.5),
        ),
        ("20 0.3 dist.binomial", 6.0, 4.2),
        ("4.2 dist.poisson", 4.2, 4.2),
        (
            "3.5 0.4 dist.negbinomial",
            3.5 * 0.6 / 0.4,
            3.5 * 0.6 / 0.16,
        ),
        (
            "50 12 10 dist.hypergeometric",
            2.4,
            10.0 * 0.24 * 0.76 * 40.0 / 49.0,
        ),
        ("0.2 dist.geometric", 4.0, 20.0),
        ("0.3 dist.bernoulli", 0.3, 0.21),
    ];
    for (d, mean, var) in cases {
        close(d, &[one(&format!("{d} dist.mean"))], &[mean], 1e-12);
        close(d, &[one(&format!("{d} dist.var"))], &[var], 1e-12);
    }
}

#[test]
fn the_multivariate_densities() {
    let want = |key: &str| -> Vec<f64> {
        let line = FIXTURE.lines().find(|l| l.starts_with(key)).unwrap();
        vec![line.rsplit(' ').next().unwrap().parse().unwrap()]
    };
    let m = FAMILIES[15];
    let d = FAMILIES[16];
    close(
        "dmultinom",
        &[one(&format!("1 list + 2 swap + 3 swap + {m} dist.pdf"))],
        &want("multinomial 6 [0.2 0.3 0.5] at [1 2 3]"),
        1e-12,
    );
    close(
        "dirichlet",
        &[one(&format!(
            "0.2 list + 0.3 swap + 0.5 swap + {d} dist.pdf"
        ))],
        &want("dirichlet [2.0 3.0 4.0] at [0.2 0.3 0.5]"),
        1e-12,
    );
    // A count that does not sum to the size has no mass.
    assert_eq!(
        one(&format!("1 list + 1 swap + 1 swap + {m} dist.pdf")),
        0.0
    );
    assert_eq!(
        err(&format!("1 list + 2 swap + {m} dist.pdf")),
        "DIST.PDF: E102 lengths do not agree: 2 and 3"
    );
    for w in ["dist.cdf", "dist.sf", "dist.quantile"] {
        let op = w.trim_start_matches("dist.");
        assert_eq!(
            err(&format!("0.5 {d} {w}")),
            format!("{}: E105 dirichlet has no {op}", w.to_uppercase())
        );
    }
    assert_eq!(
        err(&format!("{m} dist.mean")),
        "DIST.MEAN: E105 multinomial has no mean"
    );
}

// Criterion 3, for the codes §S7 and §S8 raise.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "1.0 -2.0 dist.normal",
            "DIST.NORMAL: E104 invalid normal parameters: \
             Standard deviation is NaN, zero or less than zero",
        ),
        (
            "3.0 -1.0 dist.uniform",
            "DIST.UNIFORM: E104 invalid uniform parameters: \
             Maximum is not greater than the minimum",
        ),
        (
            "10 20 5 dist.hypergeometric",
            "DIST.HYPERGEOMETRIC: E104 invalid hypergeometric parameters: successes > population",
        ),
        (
            "3.0 0.0 dist.negbinomial",
            "DIST.NEGBINOMIAL: E104 invalid negbinomial parameters: p is 0, outside R's (0, 1]",
        ),
        (
            "0.5 dict \"dist\" \"normal\" set \"mean\" 1.0 set dist.pdf",
            "DIST.PDF: E104 invalid normal parameters: sd is missing",
        ),
        (
            "0.5 dict \"dist\" \"normal\" set \"mean\" 1.0 set \"sd\" \"x\" set dist.pdf",
            "DIST.PDF: E104 invalid normal parameters: sd is not a finite number",
        ),
        (
            "2.5 20 dist.binomial",
            "DIST.BINOMIAL: E101 size is outside dist.binomial's domain: 2.5",
        ),
        (
            "1.1 1.0 2.0 dist.normal dist.quantile",
            "DIST.QUANTILE: E101 p is outside dist.quantile's domain: 1.1",
        ),
        (
            "float.NaN 1.0 2.0 dist.normal dist.cdf",
            "DIST.CDF: E101 x is outside dist.cdf's domain: NaN",
        ),
        (
            "-1.0 1.5 dist.exponential dist.lpdf",
            "DIST.LPDF: E101 x is outside dist.lpdf's domain: -1.0",
        ),
        (
            "0.5 7 dist.pdf",
            "DIST.PDF: E101 d is outside dist.pdf's domain: 7",
        ),
        (
            "2.0 dist.t dist.var",
            "DIST.VAR: E105 t with df = 2 has no variance",
        ),
        (
            "4.0 4.0 dist.f dist.var",
            "DIST.VAR: E105 f with df2 = 4 has no variance",
        ),
        (
            "0.5 1000000000000000.0 1000000000000000.0 dist.beta dist.cdf",
            "DIST.CDF: E110 the incomplete beta's continued fraction did not converge \
             within 100000 iterations",
        ),
        (
            "1.0 2.0 dist.normal 100000001 dist.sample*",
            "DIST.SAMPLE*: E111 100000001 draws exceeds its cap of 100000000",
        ),
        (
            "10000000000000000.0 dist.poisson dist.sample",
            "DIST.SAMPLE: E111 a Poisson draw's mean of 10000000000000000 exceeds its cap \
             of 9007199254740992",
        ),
        (
            "0 0.5 1.0 dist.gamma dist.pdf",
            "DIST.PDF: E114 the answer overflows a FLOAT: the gamma density at 0",
        ),
        (
            "1 1.0 2.0 dist.normal dist.quantile",
            "DIST.QUANTILE: E114 the answer overflows a FLOAT: the normal quantile at 1",
        ),
        (
            "1000.0 2.0 dist.lognormal dist.mean",
            "DIST.MEAN: E114 the answer overflows a FLOAT: the lognormal mean",
        ),
        (
            "1.5 rng.seed",
            "RNG.SEED: E101 n is outside rng.seed's domain: 1.5",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

// Criterion 5: no abort, whatever the operand.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "float.-Inf",
        "-0.1",
        "1.1",
        "0",
        "1",
        "0.0",
        "1.0",
        "-1",
        "0.5",
        "10.0 308.0 math.power",
        "\"x\"",
        "list",
    ];
    let answer = |src: &str| match run(src) {
        Ok(mut i) => {
            let v = i.pull().unwrap();
            assert!(floats(&v).iter().all(|y| y.is_finite()), "{src}: {v:?}");
        }
        Err(e) => assert!(
            e.contains(": E1") || e.contains("too shallow"),
            "{src}: {e}"
        ),
    };
    for d in FAMILIES {
        for o in operands {
            for w in [
                "dist.pdf",
                "dist.lpdf",
                "dist.cdf",
                "dist.sf",
                "dist.quantile",
            ] {
                answer(&format!("{o} {d} {w}"));
            }
        }
        for w in ["dist.mean", "dist.var", "dist.sample"] {
            answer(&format!("{d} {w}"));
        }
    }
    // RFC-0019 §S3's family, outside FAMILIES since it draws nothing: each
    // word over the palette, the quantile over a few, its polish being
    // a double integral per step.
    for o in operands {
        for w in ["dist.pdf", "dist.lpdf", "dist.cdf", "dist.sf"] {
            answer(&format!("{o} {TUKEY} {w}"));
        }
    }
    for o in ["float.NaN", "-0.1", "1.1", "0", "1", "0.5", "\"x\""] {
        answer(&format!("{o} {TUKEY} dist.quantile"));
    }
    for w in ["dist.mean", "dist.var", "dist.sample"] {
        answer(&format!("{TUKEY} {w}"));
    }
    // Each constructor, its operands all one value.
    let arity = |w: &str| match w {
        "dist.hypergeometric" | "dist.ncf" => 3,
        "dist.exponential" | "dist.chisq" | "dist.t" | "dist.poisson" | "dist.geometric"
        | "dist.bernoulli" | "dist.dirichlet" => 1,
        _ => 2,
    };
    for w in &WORDS[..21] {
        for o in operands {
            answer(&format!(
                "{} {w} 0.5 swap dist.cdf",
                vec![o; arity(w)].join(" ")
            ));
        }
    }
}

// The `.` forms take the plain word's top operand from the workbench.

#[test]
fn each_dot_form_answers_on_the_workbench() {
    for (below, top_, w) in [
        ("0.5", "1.0 2.0 dist.normal", "dist.pdf"),
        ("0.25", "2.5 1.5 dist.gamma", "dist.quantile"),
        ("1.0", "2.0", "dist.normal"),
        ("", "4.2 dist.poisson", "dist.var"),
    ] {
        let plain = run(&format!("{below} {top_} {w}")).unwrap().pull().unwrap();
        let mut i = run(&format!("7 {below} {top_} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        assert_eq!(
            i.pull_workbench().unwrap().display(),
            plain.display(),
            "{w}."
        );
    }
}

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":dist.normal { 42 } register 1.0 2.0 dist.normal").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}

// Criterion 11.

fn draws(src: &str) -> Vec<f64> {
    top(src)
}

#[test]
fn the_same_seed_gives_the_same_list() {
    for d in FAMILIES {
        let src = format!("7 rng.seed {d} 20 dist.sample* 7 rng.seed {d} 20 dist.sample*");
        let mut i = run(&src).unwrap();
        let (b, a) = (i.pull().unwrap(), i.pull().unwrap());
        assert_eq!(a.display(), b.display(), "{d}");
    }
    assert_ne!(
        draws("7 rng.seed 1.0 2.0 dist.normal 5 dist.sample*"),
        draws("8 rng.seed 1.0 2.0 dist.normal 5 dist.sample*")
    );
}

#[test]
fn the_other_sources_do_not_touch_the_stream() {
    let alone = draws("11 rng.seed 1.0 2.0 dist.normal 6 dist.sample*");
    let mixed = "11 rng.seed \
                 1.0 2.0 dist.normal 3 dist.sample* \
                 math.random.int drop \
                 \"g\" dict \"type\" \"normal\" set generator \
                 \"g\" generator.sample drop \
                 math.random.int drop \
                 1.0 2.0 dist.normal 3 dist.sample*";
    let mut i = run(mixed).unwrap();
    let second = floats(&i.pull().unwrap());
    let mut got = floats(&i.pull().unwrap());
    got.extend(second);
    assert_eq!(got, alone);
}

#[test]
fn two_interps_on_one_thread_do_not_share_draws() {
    let mut a = run("5 rng.seed").unwrap();
    let mut b = run("5 rng.seed").unwrap();
    let src = bund2_syntax::compile("rng.uniform rng.uniform").unwrap();
    a.eval(&src).unwrap();
    // Drawing from b between a's draws leaves a's stream as it was.
    let mut c = run("5 rng.seed rng.uniform").unwrap();
    b.eval(&src).unwrap();
    c.eval(&bund2_syntax::compile("rng.uniform").unwrap())
        .unwrap();
    let (a2, a1) = (a.pull().unwrap(), a.pull().unwrap());
    let (c2, c1) = (c.pull().unwrap(), c.pull().unwrap());
    assert_eq!((a1.display(), a2.display()), (c1.display(), c2.display()));
}

#[test]
fn a_binomial_draw_at_a_billion_is_quick() {
    let mut i = run("1000000000 0.3 dist.binomial").unwrap();
    let d = i.pull().unwrap();
    i.push(d);
    let src = bund2_syntax::compile("dist.sample").unwrap();
    let start = std::time::Instant::now();
    i.eval(&src).unwrap();
    let took = start.elapsed();
    assert!(took.as_micros() < 1_000, "{took:?}");
    let x = floats(&i.pull().unwrap())[0];
    assert!((x - 3e8).abs() < 1e5, "{x}");
}

/// The samplers draw from the families the densities describe: each
/// family's mean over 20 000 seeded draws is within five standard errors of
/// its `dist.mean`. Catches a sampler on the wrong parameterisation, the
/// geometric's trials for failures, say.
#[test]
fn each_sampler_draws_its_family() {
    for d in &FAMILIES[..15] {
        let xs = draws(&format!("3 rng.seed {d} 20000 dist.sample*"));
        let n = xs.len() as f64;
        let mean = xs.iter().sum::<f64>() / n;
        let (mu, var) = (
            one(&format!("{d} dist.mean")),
            one(&format!("{d} dist.var")),
        );
        assert!(
            (mean - mu).abs() <= 5.0 * (var / n).sqrt(),
            "{d}: {mean} against {mu}"
        );
    }
    let m = top(&format!("{} dist.sample", FAMILIES[15]));
    assert_eq!(m.iter().sum::<f64>(), 6.0, "{m:?}");
    let y = top(&format!("{} dist.sample", FAMILIES[16]));
    assert!((y.iter().sum::<f64>() - 1.0).abs() < 1e-12, "{y:?}");
}

/// The stack after `src`, top last, as f64s.
fn stack(src: &str) -> Vec<f64> {
    run(src)
        .unwrap()
        .snapshot()
        .iter()
        .flat_map(floats)
        .collect()
}

#[test]
fn rng_uniform_draws_from_the_unit_interval() {
    let xs = stack("9 rng.seed rng.uniform rng.uniform rng.uniform");
    assert_eq!(xs.len(), 3);
    assert!(xs.iter().all(|x| (0.0..1.0).contains(x)), "{xs:?}");
    assert_eq!(xs, stack("9 rng.seed rng.uniform rng.uniform rng.uniform"));
    // `rng.seed.` takes n from the workbench.
    assert_eq!(
        stack("9 rng.seed rng.uniform"),
        stack("9 . rng.seed. rng.uniform")
    );
}

// RFC-0019 §S2: the noncentral t, χ² and F.

const NONCENTRAL: &str = include_str!("../fixtures/noncentral.txt");

/// RFC-0019's tolerance: |a − b| ≤ 10⁻⁹|b| + 10⁻¹².
fn within(what: &str, got: f64, want: f64) {
    assert!(
        (got - want).abs() <= 1e-9 * want.abs() + 1e-12,
        "{what}: {got:e} against {want:e}"
    );
}

/// Criterion 2: pdf, cdf, sf, quantile, mean and variance against mpmath's
/// closed forms and quadratures, over a grid with ncp 0, ncp 100 and a
/// negative ncp for the t.
#[test]
fn each_noncentral_family_against_the_fixture() {
    let mut seen = 0;
    for line in NONCENTRAL.lines() {
        let (program, want) = line.rsplit_once(' ').unwrap();
        within(program, one(program), want.parse().unwrap());
        seen += 1;
    }
    assert_eq!(seen, 11 * 14);
}

#[test]
fn a_noncentral_quantile_inverts_its_cdf() {
    for d in &FAMILIES[17..] {
        for p in [1e-6, 0.001, 0.3, 0.5, 0.999, 1.0 - 1e-6] {
            let got = one(&format!("{p} {d} dist.quantile {d} dist.cdf"));
            assert!((got - p).abs() <= 1e-12, "{d} at {p}: {got}");
        }
    }
}

/// At ncp = 0 each family is its central one, within the tolerance and not
/// to the bit (§S2).
#[test]
fn at_ncp_zero_each_is_its_central_family() {
    for (nc, central, xs) in [
        (
            "7.0 0.0 dist.nct",
            "7.0 dist.t",
            [-3.0, -0.5, 0.0, 1.5, 6.0],
        ),
        (
            "3.5 0.0 dist.ncchisq",
            "3.5 dist.chisq",
            [0.1, 1.0, 3.0, 9.0, 30.0],
        ),
        (
            "4.0 9.0 0.0 dist.ncf",
            "4.0 9.0 dist.f",
            [0.05, 0.5, 1.0, 3.0, 20.0],
        ),
    ] {
        for x in xs {
            for w in ["dist.pdf", "dist.cdf", "dist.sf"] {
                let what = format!("{x} {nc} {w}");
                within(&what, one(&what), one(&format!("{x} {central} {w}")));
            }
        }
        for p in [0.01, 0.5, 0.99] {
            let what = format!("{p} {nc} dist.quantile");
            within(
                &what,
                one(&what),
                one(&format!("{p} {central} dist.quantile")),
            );
        }
        for w in ["dist.mean", "dist.var"] {
            let what = format!("{nc} {w}");
            within(&what, one(&what), one(&format!("{central} {w}")));
        }
    }
}

/// The closed-form moments at parameters the fixture does not hold.
#[test]
fn the_noncentral_moments_are_their_closed_forms() {
    // nct, df 6, ncp 2: E[T] = δ√(ν/2)·Γ((ν−1)/2)/Γ(ν/2) = 2√3·Γ(5/2)/Γ(3),
    // Γ(5/2) = 3√π/4, so 3√(3π)/4; Var = ν(1 + δ²)/(ν − 2) − E[T]².
    let mean = 3.0 * (3.0 * std::f64::consts::PI).sqrt() / 4.0;
    within("nct mean", one("6.0 2.0 dist.nct dist.mean"), mean);
    within(
        "nct var",
        one("6.0 2.0 dist.nct dist.var"),
        7.5 - mean * mean,
    );
    within("ncchisq mean", one("3.0 4.0 dist.ncchisq dist.mean"), 7.0);
    within("ncchisq var", one("3.0 4.0 dist.ncchisq dist.var"), 22.0);
    // ncf, 3, 10, 2: 10·5/(3·8); 2·(10/3)²·(25 + 7·8)/(64·6).
    within(
        "ncf mean",
        one("3.0 10.0 2.0 dist.ncf dist.mean"),
        50.0 / 24.0,
    );
    within(
        "ncf var",
        one("3.0 10.0 2.0 dist.ncf dist.var"),
        2.0 * (100.0 / 9.0) * 81.0 / 384.0,
    );
}

/// 10⁵ seeded draws have a sample mean within four standard errors.
#[test]
fn each_noncentral_sampler_draws_its_family() {
    for d in [
        "5.5 1.5 dist.nct",
        "10.0 -2.0 dist.nct",
        "4.5 2.5 dist.ncchisq",
        "3.0 100.0 dist.ncchisq",
        "4.0 7.5 3.0 dist.ncf",
        "5.0 12.0 100.0 dist.ncf",
    ] {
        let xs = draws(&format!("19 rng.seed {d} 100000 dist.sample*"));
        let n = xs.len() as f64;
        let mean = xs.iter().sum::<f64>() / n;
        let (mu, var) = (
            one(&format!("{d} dist.mean")),
            one(&format!("{d} dist.var")),
        );
        assert!(
            (mean - mu).abs() <= 4.0 * (var / n).sqrt(),
            "{d}: {mean} against {mu}"
        );
    }
}

#[test]
fn each_noncentral_refusal_is_its_template() {
    for (src, want) in [
        (
            "0.0 1.0 dist.nct",
            "DIST.NCT: E104 invalid nct parameters: df is 0, not above 0",
        ),
        (
            "3.0 -1.0 dist.ncchisq",
            "DIST.NCCHISQ: E104 invalid ncchisq parameters: ncp is -1, below 0",
        ),
        (
            "3.0 -2.0 1.0 dist.ncf",
            "DIST.NCF: E104 invalid ncf parameters: df2 is -2, not above 0",
        ),
        (
            "0.5 dict \"dist\" \"ncf\" set \"df1\" 3.0 set \"df2\" 5.0 set dist.cdf",
            "DIST.CDF: E104 invalid ncf parameters: ncp is missing",
        ),
        (
            "2.0 1.0 dist.nct dist.var",
            "DIST.VAR: E105 nct with df = 2 has no variance",
        ),
        (
            "1.0 1.0 dist.nct dist.mean",
            "DIST.MEAN: E105 nct with df = 1 has no mean",
        ),
        (
            "3.0 4.0 1.0 dist.ncf dist.var",
            "DIST.VAR: E105 ncf with df2 = 4 has no variance",
        ),
        (
            "5.0 3.0 10000000000000.0 dist.ncchisq dist.cdf",
            "DIST.CDF: E111 a noncentral series' terms exceeds its cap of 1000000",
        ),
        (
            "1.0 5.0 10000000.0 dist.nct dist.pdf",
            "DIST.PDF: E111 a noncentral series' terms exceeds its cap of 1000000",
        ),
        (
            "float.NaN 3.0 dist.nct",
            "DIST.NCT: E101 df is outside dist.nct's domain: NaN",
        ),
        (
            "1 5.0 1.0 dist.nct dist.quantile",
            "DIST.QUANTILE: E114 the answer overflows a FLOAT: the nct quantile at 1",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

/// A t MAP with an `ncp` key is still the central t (§Preservation): only
/// the new families read `ncp`.
#[test]
fn a_central_map_ignores_an_ncp_key() {
    let by_hand = "1.5 dict \"dist\" \"t\" set \"df\" 5.0 set \"ncp\" 2.0 set dist.cdf";
    assert_eq!(one(by_hand), one("1.5 5.0 dist.t dist.cdf"));
    let nc = one("1.5 5.0 2.0 dist.nct dist.cdf");
    assert!((one(by_hand) - nc).abs() > 0.1);
}

/// A large ncp sums from the mode, so its first term does not underflow:
/// the t at ncp 100, where R's AS 243 fails, answers the fixture's value
/// above; here, its two tails add to 1 far into either side.
#[test]
fn a_large_ncp_keeps_both_tails() {
    for x in [40.0, 80.0, 100.0, 140.0, 400.0] {
        let lo = one(&format!("{x} 8.0 100.0 dist.nct dist.cdf"));
        let hi = one(&format!("{x} 8.0 100.0 dist.nct dist.sf"));
        // Each tail is summed on its own, so they agree to the incomplete
        // beta's accuracy at a near 5 000, not to the bit.
        assert!((lo + hi - 1.0).abs() <= 1e-10, "{x}: {lo} + {hi}");
        assert!(lo > 0.0 && hi > 0.0, "{x}: {lo}, {hi}");
    }
}

// RFC-0019 criterion 3: the studentized range.

const TUKEY: &str = "3 10.0 dist.tukey";
const TUKEY_FIXTURE: &str = include_str!("../fixtures/tukey.txt");

/// The cdf and sf against mpmath's 50-digit double integral, k from 2 to
/// 1000, df from 2 to 1000, and each tail's far end.
#[test]
fn the_studentized_range_against_the_fixture() {
    let mut seen = 0;
    for line in TUKEY_FIXTURE.lines() {
        let (program, want) = line.rsplit_once(' ').unwrap();
        within(program, one(program), want.parse().unwrap());
        seen += 1;
    }
    assert_eq!(seen, 9 * 2);
}

#[test]
fn a_tukey_quantile_inverts_its_cdf() {
    for d in ["2 2.0 dist.tukey", TUKEY, "20 60.0 dist.tukey"] {
        for p in [0.001, 0.5, 0.95, 0.999] {
            let got = one(&format!("{p} {d} dist.quantile {d} dist.cdf"));
            assert!((got - p).abs() <= 1e-12, "{d} at {p}: {got}");
        }
    }
    assert_eq!(one(&format!("0.0 {TUKEY} dist.quantile")), 0.0);
    assert_eq!(one(&format!("0.0 {TUKEY} dist.cdf")), 0.0);
    assert_eq!(one(&format!("-1.0 {TUKEY} dist.sf")), 1.0);
}

#[test]
fn a_tukey_map_reads_as_its_constructor() {
    let m = run(TUKEY).unwrap().pull().unwrap();
    assert_eq!(
        m.display(),
        run("dict \"dist\" \"tukey\" set \"nmeans\" 3 set \"df\" 10.0 set")
            .unwrap()
            .pull()
            .unwrap()
            .display()
    );
}

#[test]
fn each_tukey_refusal_is_its_template() {
    for (src, want) in [
        (
            format!("1.0 {TUKEY} dist.pdf"),
            "DIST.PDF: E105 tukey has no pdf",
        ),
        (
            format!("1.0 {TUKEY} dist.lpdf"),
            "DIST.LPDF: E105 tukey has no lpdf",
        ),
        (
            format!("{TUKEY} dist.sample"),
            "DIST.SAMPLE: E105 tukey has no sample",
        ),
        (
            format!("{TUKEY} 0 dist.sample*"),
            "DIST.SAMPLE*: E105 tukey has no sample",
        ),
        (
            format!("{TUKEY} dist.mean"),
            "DIST.MEAN: E105 tukey has no mean",
        ),
        (
            format!("{TUKEY} dist.var"),
            "DIST.VAR: E105 tukey has no variance",
        ),
        (
            "3 1.5 dist.tukey".to_string(),
            "DIST.TUKEY: E104 invalid tukey parameters: df is 1.5, below 2",
        ),
        (
            "1 10.0 dist.tukey".to_string(),
            "DIST.TUKEY: E104 invalid tukey parameters: nmeans is 1, below 2",
        ),
        (
            "10001 10.0 dist.tukey".to_string(),
            "DIST.TUKEY: E104 invalid tukey parameters: nmeans is 10001, above 10000",
        ),
        (
            "3.0 10.0 dist.tukey".to_string(),
            "DIST.TUKEY: E101 nmeans is outside dist.tukey's domain: 3.0",
        ),
        (
            "1.0 dict \"dist\" \"tukey\" set \"nmeans\" 3 set \"df\" 1.0 set dist.cdf".to_string(),
            "DIST.CDF: E104 invalid tukey parameters: df is 1, below 2",
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}
