//! RFC-0015 criteria 2, 3, 5, 6, 10 and 11, for §S7 and §S8.

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{close, err, floats, run, top};

const FIXTURE: &str = include_str!("../fixtures/dist.txt");

/// Each family with parameters it accepts, as a program that pushes it.
const FAMILIES: [&str; 17] = [
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
    assert_eq!(ours, 2 * WORDS.len() + 3);
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
    // Each constructor, its operands all one value.
    let arity = |w: &str| match w {
        "dist.hypergeometric" => 3,
        "dist.exponential" | "dist.chisq" | "dist.t" | "dist.poisson" | "dist.geometric"
        | "dist.bernoulli" | "dist.dirichlet" => 1,
        _ => 2,
    };
    for w in &WORDS[..17] {
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
