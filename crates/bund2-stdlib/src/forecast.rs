//! **`bund/forecast`** — D215's twenty-two words and the two aliases that name
//! one of them.
//!
//! D28 deferred the family for its crates; D215 brings it into the default
//! build. Every answer is the crate the reference calls, at the version its
//! lock file resolves: `augurs` 0.9.0 for MSTL, the MAD and DBSCAN outlier
//! detectors, DTW clustering and the periodogram; `markov-chain` 0.1.1 over
//! `decorum`'s `R64`; `distimate` 0.2.1's PERT; `rstats` 2.1.15 and `statrs`
//! for the summary statistics; and `anomaly_detection` 0.3.1 and `breakout`
//! 0.3.0 for `forecast.estimate!`'s two lists.
//!
//! **Each word reads its series through `series::get_data`**, the reader the
//! statistics words share (`reference/Bund/src/stdlib/functions/statistics/get_data.rs`),
//! so a LIST, a METRICS and a drained stack are the three sources here too.
//!
//! **`forecast.markov` is random.** `markov-chain` picks each step with the
//! thread's RNG, so its mean varies from run to run and no golden can hold
//! it; the PERT words are analytic and the rest deterministic.
//!
//! **Where the reference panics, Bund2 refuses** (D37). `forecast.estimate!`
//! `unwrap`s the geometric and harmonic means and the median spread
//! (`forecast/estimation.rs:74-77`), so a series holding a value that is not
//! normal aborts the oracle; each is an ordinary error here, in the crate's
//! own words under the word's prefix. The outlier words slice their series by
//! the detector's intervals (`math/anomalies.rs:51,90`); an interval out of
//! range would panic there and is refused here.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;
use std::collections::BTreeMap;

use crate::series::{Mode, get_data};
use crate::wb::Side;

/// `cast_float`: a FLOAT and nothing else, with the reference's sentence,
/// which names the tag (`reference/rust_dynamic/src/cast.rs:9-16`).
fn cast_float(v: &BundValue) -> Result<f64, String> {
    match v.unboxed() {
        BundValue::Float(f, _) => Ok(*f),
        other => Err(format!("This Dynamic type is not float: {}", other.dt())),
    }
}

/// `cast_int`: an INTEGER and nothing else (`cast.rs:17-24`).
fn cast_int(v: &BundValue) -> Result<i64, String> {
    v.as_int()
        .ok_or_else(|| "This Dynamic type is not integer".to_string())
}

fn floats(v: &[f64]) -> BundValue {
    BundValue::list(v.iter().map(|x| BundValue::float(*x)).collect())
}

/// `forecast.markov` — train a chain of order 16 on the series, generate up
/// to 16 steps, and answer their arithmetic mean (`forecast/markov.rs:13-42`).
fn markov(vm: &mut dyn Vm, side: Side, mode: Mode, prefix: &str) -> Result<(), Error> {
    use rstats::Stats;
    let source = get_data(vm, side, mode, prefix)
        .map_err(|e| Error(format!("{prefix} returned: {}", e.0)))?;
    let dst: Vec<decorum::R64> = source.into_iter().map(decorum::R64::from).collect();
    let mut chain = markov_chain::Chain::<decorum::R64>::new(16);
    chain.train(dst);
    let res = chain.generate_limit(16);
    if res.is_empty() {
        return Err(Error(format!(
            "{prefix} forecasting does not returned a prognisis"
        )));
    }
    let mean = res
        .amean()
        .map_err(|e| Error(format!("{prefix} forecasting can not compute mean: {e}")))?;
    side.push(vm, BundValue::float(mean));
    Ok(())
}

/// `forecast.mstl` — fit MSTL with periods `[2]` over a non-seasonal AutoETS
/// trend, and predict `n` steps at the interval level given
/// (`forecast/mstl.rs:15-71`). The series is read from `side`; `n` and the
/// level are pulled from the **stack** in either form, and the MAP is pushed
/// to the stack, as the reference does (`:21,28,66`).
fn mstl(vm: &mut dyn Vm, side: Side, mode: Mode, prefix: &str) -> Result<(), Error> {
    use augurs::prelude::*;
    let source = get_data(vm, side, mode, prefix)
        .map_err(|e| Error(format!("{prefix} NO DATA #2: {}", e.0)))?;
    let n = match vm.pull() {
        Some(v) => {
            cast_int(&v).map_err(|e| Error(format!("{prefix} sensitivity casting error: {e}")))?
        }
        None => return Err(Error(format!("{prefix} NO DATA #2"))),
    };
    let interval = match vm.pull() {
        Some(v) => {
            cast_float(&v).map_err(|e| Error(format!("{prefix} interval casting error: {e}")))?
        }
        None => return Err(Error(format!("{prefix} NO DATA #3"))),
    };
    let ets = augurs::ets::AutoETS::non_seasonal().into_trend_model();
    let model = augurs::mstl::MSTLModel::new(vec![2], ets);
    let fit = model
        .fit(&source)
        .map_err(|e| Error(format!("{prefix} fitting returns: {e}")))?;
    let forecasts = fit
        .predict(n as usize, interval)
        .map_err(|e| Error(format!("{prefix} forecasting returns: {e}")))?;
    let mut m = BTreeMap::new();
    m.insert("forecast".to_string(), floats(&forecasts.point));
    let (lower, upper) = match &forecasts.intervals {
        Some(iv) => {
            m.insert("interval".to_string(), BundValue::float(iv.level));
            (floats(&iv.lower), floats(&iv.upper))
        }
        None => (floats(&[]), floats(&[])),
    };
    m.insert("lower".to_string(), lower);
    m.insert("upper".to_string(), upper);
    vm.push(BundValue::map(m));
    Ok(())
}

#[derive(Clone, Copy)]
enum Estimate {
    Expected,
    Uncertainty,
    Analysis,
}

/// `forecast.estimate`, `.uncertainty` and `!` — a PERT distribution from the
/// series' minimum, arithmetic mean and maximum at the shape given
/// (`forecast/estimation.rs:23-130`). The series is consumed and the shape
/// pulled from the same side.
fn estimate(vm: &mut dyn Vm, side: Side, what: Estimate, prefix: &str) -> Result<(), Error> {
    use distimate::EstimationDistribution;
    use rstats::Stats;
    use statrs::statistics::Statistics;
    let source = get_data(vm, side, Mode::Consume, prefix)
        .map_err(|e| Error(format!("{prefix} returned: {}", e.0)))?;
    let shape = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} can not obtain shape")))?;
    let shape =
        cast_float(&shape).map_err(|e| Error(format!("{prefix} casting shape returns: {e}")))?;
    // The folds start from NaN, which `f64::min` and `max` skip (`:38-39`).
    let min = source.iter().cloned().fold(f64::NAN, f64::min);
    let max = source.iter().cloned().fold(f64::NAN, f64::max);
    let mean = source.as_slice().amean().map_err(|e| {
        Error(format!(
            "{prefix}: for min={min}, max={max}, AMEAN error: {e}"
        ))
    })?;
    let pert = distimate::Pert::new_with_shape(min, mean, max, shape).map_err(|e| {
        Error(format!(
            "{prefix}: for min={min}, max={max}, mean={mean} PERT error: {e}"
        ))
    })?;
    let answer = match what {
        Estimate::Expected => BundValue::float(pert.expected_value()),
        Estimate::Uncertainty => BundValue::float(pert.uncertainty()),
        Estimate::Analysis => {
            // Where the reference `unwrap`s (`:74-77`), a refusal in the
            // crate's words: it aborts on a value that is not normal.
            let unwrapped = |r: Result<f64, rstats::RE>, what: &str| {
                r.map_err(|e| Error(format!("{prefix} {what}: {e}")))
            };
            let s = source.as_slice();
            let mut m = BTreeMap::new();
            let mut set = |k: &str, v: BundValue| {
                m.insert(k.to_string(), v);
            };
            set("sample.size", BundValue::int(source.len() as i64));
            set("expected_value", BundValue::float(pert.expected_value()));
            set("uncertainty", BundValue::float(pert.uncertainty()));
            set("likely", BundValue::float(pert.most_likely_estimate()));
            set("optimistic", BundValue::float(pert.optimistic_estimate()));
            set("pessimistic", BundValue::float(pert.pessimistic_estimate()));
            set("alpha", BundValue::float(pert.alpha()));
            set("beta", BundValue::float(pert.beta()));
            set("kurtosis", BundValue::float(pert.kurtosis()));
            set("min", BundValue::float(min));
            set("max", BundValue::float(max));
            set("mean.arithmetic", BundValue::float(mean));
            set(
                "mean.geometric",
                BundValue::float(unwrapped(s.gmean(), "geometric mean")?),
            );
            set(
                "mean.harmonic",
                BundValue::float(unwrapped(s.hmean(), "harmonic mean")?),
            );
            let medmad = s
                .medmad()
                .map_err(|e| Error(format!("{prefix} median: {e}")))?;
            set("median", BundValue::float(medmad.centre));
            set("stddev", BundValue::float(medmad.spread));
            set("variance", BundValue::float(source.clone().variance()));
            set("mean", BundValue::float(source.clone().mean()));
            set("entropy", BundValue::float(source.clone().entropy()));
            let pair = |(a, b): (f64, f64)| {
                BundValue::list(vec![BundValue::float(a), BundValue::float(b)])
            };
            set("confidence1", pair(pert.confidence_interval(0.1)));
            set("confidence2", pair(pert.confidence_interval(0.5)));
            set("confidence3", pair(pert.confidence_interval(0.99)));
            if let Ok(a) = pert.calculate_within_interval(&source, 0.2, 0.8) {
                set("assurance", BundValue::float(a));
            }
            set(
                "not_likely_to_overrun",
                BundValue::float(pert.risk_of_overrun(0.2)),
            );
            set("can_overrun", BundValue::float(pert.risk_of_overrun(0.5)));
            set("will_overrun", BundValue::float(pert.risk_of_overrun(0.8)));
            set("anomalies", floats(&anomalies(&source, source.len() / 4)?));
            set("breakouts", floats(&breakouts(&source, source.len() / 4)?));
            BundValue::map(m)
        }
    };
    side.push(vm, answer);
    Ok(())
}

/// `math::anomalies::detect_anomalies` — the series as `f32`, its anomalies'
/// indices from `anomaly_detection`, answered as the `f32` values widened back
/// (`reference/Bund/src/stdlib/functions/math/anomalies.rs:6-21`).
fn anomalies(source: &[f64], period: usize) -> Result<Vec<f64>, Error> {
    let src: Vec<f32> = source.iter().map(|x| *x as f32).collect();
    let fit = anomaly_detection::AnomalyDetector::fit(&src, period)
        .map_err(|e| Error(format!("ANOMALY DETECTION returns error: {e}")))?;
    Ok(fit
        .anomalies()
        .iter()
        .filter_map(|ix| src.get(*ix).map(|v| f64::from(*v)))
        .collect())
}

/// `math::breakout::detect_breakouts` — `breakout`'s multi detector at a
/// minimum size of `n`, degree 2, beta 0.008 (`math/breakout.rs:4-20`).
fn breakouts(source: &[f64], n: usize) -> Result<Vec<f64>, Error> {
    let ix = breakout::multi()
        .min_size(n)
        .degree(2)
        .beta(0.008)
        .percent(None)
        .fit(source)
        .map_err(|e| Error(format!("BREAKOUT DETECTION returns error: {e}")))?;
    Ok(ix.iter().filter_map(|i| source.get(*i).copied()).collect())
}

/// The operand the two-series words pull first, from `side`
/// (`forecast/outliers.rs:13-23`): a FLOAT, refused as `sensitivity` in all
/// three, clustering included.
fn sensitivity(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<f64, Error> {
    match side.pull(vm) {
        Some(v) => {
            cast_float(&v).map_err(|e| Error(format!("{prefix} sensitivity casting error: {e}")))
        }
        None => Err(Error(format!("{prefix} NO DATA #1"))),
    }
}

/// Two series, read in turn from `side` and consumed (`outliers.rs:24-31`).
fn two_series(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<[Vec<f64>; 2], Error> {
    let mut read = || {
        get_data(vm, side, Mode::Consume, prefix)
            .map_err(|e| Error(format!("{prefix} NO DATA #2: {}", e.0)))
    };
    let a = read()?;
    let b = read()?;
    Ok([a, b])
}

/// The outliers MAP both detectors answer (`math/anomalies.rs:40-59`): key
/// `"0"` and `"1"` for the two series, each a LIST of the intervals' values;
/// an interval with no end is skipped.
fn outlier_map(
    series: &[Vec<f64>; 2],
    results: Vec<augurs::outlier::Series>,
) -> Result<BundValue, Error> {
    let mut m = BTreeMap::new();
    for (s, r) in results.into_iter().enumerate() {
        let mut row = Vec::new();
        for iv in r.outlier_intervals.intervals {
            let Some(end) = iv.end else { continue };
            let values = series
                .get(s)
                .and_then(|v| v.get(iv.start..end))
                .ok_or_else(|| {
                    Error::internal(format!(
                        "an outlier interval {}..{end} lies outside series {s}",
                        iv.start
                    ))
                })?;
            row.push(floats(values));
        }
        m.insert(s.to_string(), BundValue::list(row));
    }
    Ok(BundValue::map(m))
}

/// `outlier.detect` — augurs' MAD detector at the sensitivity given
/// (`forecast/outliers.rs:12-38`, `math/anomalies.rs:23-60`). The MAP is
/// pushed to the stack in either form.
fn outliers_mad(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    use augurs::outlier::OutlierDetector;
    let sens = sensitivity(vm, side, prefix)?;
    let series = two_series(vm, side, prefix)?;
    let slices: &[&[f64]] = &[&series[0], &series[1]];
    let detector = augurs::outlier::MADDetector::with_sensitivity(sens).map_err(|e| {
        Error(format!(
            "{prefix} return error: DETECT_OUTLIERS: detector create returns: {e}"
        ))
    })?;
    let processed = detector.preprocess(slices).map_err(|e| {
        Error(format!(
            "{prefix} return error: DETECT_OUTLIERS: detector processor returns: {e}"
        ))
    })?;
    let out = detector.detect(&processed).map_err(|e| {
        Error(format!(
            "{prefix} return error: DETECT_OUTLIERS: detector detector returns: {e}"
        ))
    })?;
    vm.push(outlier_map(&series, out.series_results)?);
    Ok(())
}

/// `outlier.detect.dbscan` — augurs' DBSCAN detector, otherwise as
/// `outlier.detect` (`forecast/outliers_dbscan.rs:12-38`, `anomalies.rs:62-99`).
fn outliers_dbscan(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    use augurs::outlier::OutlierDetector;
    let sens = sensitivity(vm, side, prefix)?;
    let series = two_series(vm, side, prefix)?;
    let slices: &[&[f64]] = &[&series[0], &series[1]];
    let detector = augurs::outlier::DbscanDetector::with_sensitivity(sens).map_err(|e| {
        Error(format!(
            "{prefix} return error: DETECT_OUTLIERS: detector create returns: {e}"
        ))
    })?;
    let processed = detector.preprocess(slices).map_err(|e| {
        Error(format!(
            "{prefix} return error: DETECT_OUTLIERS: detector processor returns: {e}"
        ))
    })?;
    let out = detector.detect(&processed).map_err(|e| {
        Error(format!(
            "{prefix} return error: DETECT_OUTLIERS: detector detector returns: {e}"
        ))
    })?;
    vm.push(outlier_map(&series, out.series_results)?);
    Ok(())
}

/// `clusters.detect` — the two series' DTW distance matrix, clustered by
/// DBSCAN at the epsilon given with a minimum cluster of 2: a LIST with an
/// INTEGER cluster per series, or `"NOISE"` (`forecast/clustering.rs:12-38`,
/// `math/clusters.rs:9-29`).
fn clusters(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    let eps = sensitivity(vm, side, prefix)?;
    let series = two_series(vm, side, prefix)?;
    let slices: &[&[f64]] = &[&series[0], &series[1]];
    let dm = augurs::dtw::Dtw::euclidean().distance_matrix(slices);
    let found = augurs::clustering::DbscanClusterer::new(eps, 2).fit(&dm);
    let out = found
        .into_iter()
        .map(|c| {
            if c.is_cluster() {
                BundValue::int(i64::from(c.as_i32()))
            } else {
                BundValue::str("NOISE")
            }
        })
        .collect();
    vm.push(BundValue::list(out));
    Ok(())
}

/// `periodic.detect` — the periodogram's periods, and the series cut into
/// rows by them in turn, repeating, until it runs out; with no period found,
/// the series itself as one flat LIST (`forecast/periodic_detector.rs:13-51`).
/// The answer goes to the stack in either form.
fn periodic(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    use augurs::seasons::Detector;
    let source = get_data(vm, side, Mode::Consume, prefix)
        .map_err(|e| Error(format!("{prefix} NO DATA #2: {}", e.0)))?;
    let periods = augurs::seasons::PeriodogramDetector::default().detect(&source);
    if periods.is_empty() {
        vm.push(floats(&source));
        return Ok(());
    }
    let mut rows = Vec::new();
    let mut c = 0usize;
    // A period of zero never advances `c`; the reference would loop for ever
    // there. The periodogram answers periods of two and more, so this is a
    // broken invariant, not a program's input.
    if periods.contains(&0) {
        return Err(Error::internal(
            "the periodogram answered a period of zero".to_string(),
        ));
    }
    'outer: loop {
        for p in &periods {
            if c >= source.len() {
                break 'outer;
            }
            let end = (c + *p as usize).min(source.len());
            rows.push(floats(&source[c..end]));
            c += *p as usize;
        }
    }
    vm.push(BundValue::list(rows));
    Ok(())
}

pub fn register(r: &mut Registry) {
    // Every form reads a series of any length, so each is opaque on the side
    // it reads, as the statistics words are (D175).
    let stack = StackEffect::opaque(1);
    let bench = StackEffect::opaque(0);
    macro_rules! four {
        ($base:literal, $f:ident, $prefix:literal) => {{
            fn s(vm: &mut dyn Vm) -> Result<(), Error> {
                $f(vm, Side::Stack, Mode::Consume, $prefix)
            }
            fn sk(vm: &mut dyn Vm) -> Result<(), Error> {
                $f(vm, Side::Stack, Mode::Keep, concat!($prefix, ","))
            }
            fn w(vm: &mut dyn Vm) -> Result<(), Error> {
                $f(vm, Side::Bench, Mode::Consume, concat!($prefix, "."))
            }
            fn wk(vm: &mut dyn Vm) -> Result<(), Error> {
                $f(vm, Side::Bench, Mode::Keep, concat!($prefix, ".,"))
            }
            r.register_native($base, s, stack, WordKind::Sync);
            r.register_native(concat!($base, ","), sk, stack, WordKind::Sync);
            r.register_native(concat!($base, "."), w, bench, WordKind::Sync);
            r.register_native(concat!($base, ".,"), wk, bench, WordKind::Sync);
        }};
    }
    four!("forecast.markov", markov, "FORECAST.MARKOV");
    four!("forecast.mstl", mstl, "FORECAST.MSTL");

    macro_rules! two {
        ($base:literal, $call:expr, $prefix:literal) => {{
            r.register_native(
                $base,
                |vm| $call(vm, Side::Stack, $prefix),
                stack,
                WordKind::Sync,
            );
            r.register_native(
                concat!($base, "."),
                |vm| $call(vm, Side::Bench, concat!($prefix, ".")),
                bench,
                WordKind::Sync,
            );
        }};
    }
    two!(
        "forecast.estimate",
        |vm, side, p| estimate(vm, side, Estimate::Expected, p),
        "FORECAST.ESTIMATE"
    );
    two!(
        "forecast.estimate.uncertainty",
        |vm, side, p| estimate(vm, side, Estimate::Uncertainty, p),
        // Not the word's name (`estimation.rs:143,147`).
        "FORECAST.UNCERTAINTY"
    );
    two!(
        "forecast.estimate!",
        |vm, side, p| estimate(vm, side, Estimate::Analysis, p),
        "FORECAST.ESTIMATE!"
    );
    // Both outlier words report as `OUTLIER.DETECT` (`outliers.rs:42,46`,
    // `outliers_dbscan.rs:42,46`).
    two!("outlier.detect", outliers_mad, "OUTLIER.DETECT");
    two!("outlier.detect.dbscan", outliers_dbscan, "OUTLIER.DETECT");
    two!("clusters.detect", clusters, "CLUSTERS.DETECT");
    two!("periodic.detect", periodic, "PERIODIC.DETECT");

    // `sample.analysis` is an alias of `forecast.estimate!`
    // (`reference/Bund/src/stdlib/functions/create_aliases.rs:38-39`).
    r.register_alias("sample.analysis", "forecast.estimate!");
    r.register_alias("sample.analysis.", "forecast.estimate!.");
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    fn run(src: &str) -> Result<Vec<BundValue>, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let ir = bund2_syntax::compile(src).map_err(|e| format!("{e:?}"))?;
        i.eval(&ir).map_err(|e| e.0)?;
        Ok(i.snapshot())
    }

    fn float(v: Option<&BundValue>) -> Option<f64> {
        match v?.unboxed() {
            BundValue::Float(f, _) => Some(*f),
            _ => None,
        }
    }

    /// **`forecast.markov` answers the mean of what the chain generates**,
    /// which is drawn from the series, so it lies within the series' range;
    /// an empty series generates nothing and is refused in the reference's
    /// words. The chain picks at random, which is why no golden holds it:
    /// over sixty runs of this series the oracle answered five different
    /// means and refused twice, and Bund2 answered the same five and refused
    /// four times. So each run here may answer or refuse, and most answer.
    #[test]
    fn markov_answers_within_the_series() {
        let mut answered = 0;
        for _ in 0..40 {
            match run("[ 1.0 2.0 3.0 2.0 1.0 4.0 ] forecast.markov") {
                Ok(out) => {
                    let m = float(out.last()).expect("a FLOAT");
                    assert!((1.0..=4.0).contains(&m), "{m}");
                    answered += 1;
                }
                Err(e) => assert!(e.ends_with("does not returned a prognisis"), "{e}"),
            }
        }
        assert!(answered >= 20, "{answered} of 40 answered");
        let out = run("[ 5.0 ] forecast.markov").expect("runs");
        assert_eq!(float(out.last()), Some(5.0));
        let e = run("list forecast.markov").expect_err("empty");
        assert!(
            e.ends_with("FORECAST.MARKOV forecasting does not returned a prognisis"),
            "{e}"
        );
    }

    /// **Where the reference aborts, Bund2 refuses** (F228): the oracle
    /// `unwrap`s the geometric mean, so a series holding a zero panics there.
    /// The other refusals are the reference's own texts, run against it.
    #[test]
    fn estimate_refuses_where_the_reference_panics() {
        let e = run("4.0 [ 1.0 0.0 3.0 ] forecast.estimate!").expect_err("a zero");
        assert!(
            e.ends_with(
                "FORECAST.ESTIMATE! geometric mean: Arithmetic error: gmean: attempt to take ln of zero"
            ),
            "{e}"
        );
        let e = run("4.0 [ 2.0 2.0 2.0 ] forecast.estimate").expect_err("flat");
        assert!(
            e.ends_with(
                "FORECAST.ESTIMATE: for min=2, max=2, mean=2 PERT error: Invalid parameters: min (2) must be less than max (2)."
            ),
            "{e}"
        );
        let e = run("4.0 list forecast.estimate.uncertainty").expect_err("empty");
        assert!(
            e.ends_with(
                "FORECAST.UNCERTAINTY: for min=NaN, max=NaN, AMEAN error: Missing or insufficient data: amean: empty self vec"
            ),
            "{e}"
        );
        let e = run("\"x\" [ 1.0 2.0 ] forecast.estimate").expect_err("a STRING shape");
        assert!(
            e.ends_with(
                "FORECAST.ESTIMATE casting shape returns: This Dynamic type is not float: 4"
            ),
            "{e}"
        );
    }

    /// **`forecast.mstl,` restores its LIST to the stack and then pulls it as
    /// `n`**, so on the stack the keep form always refuses, in both engines.
    #[test]
    fn mstl_keep_on_the_stack_pulls_its_own_list() {
        let e = run("0.95 3 [ 1.0 2.0 1.5 2.5 2.0 3.0 2.5 3.5 ] forecast.mstl,")
            .expect_err("the list is n");
        assert!(
            e.ends_with(
                "FORECAST.MSTL, sensitivity casting error: This Dynamic type is not integer"
            ),
            "{e}"
        );
    }

    /// **The outlier words need two series**, and report as `OUTLIER.DETECT`
    /// whichever detector ran.
    #[test]
    fn the_outlier_words_read_two_series() {
        let e = run("[ 1.0 2.0 ] 0.5 outlier.detect.dbscan").expect_err("one series");
        assert!(
            e.ends_with(
                "OUTLIER.DETECT NO DATA #2: Stack is too shallow for inline OUTLIER.DETECT"
            ),
            "{e}"
        );
    }
}
