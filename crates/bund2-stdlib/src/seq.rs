//! Sequence generation and counted iteration — `seq` and `times`.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LAMBDA};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect { consumes, produces }
}

/// `conf_get` — a key from a config dict, or a default
/// (`reference/Bund/src/stdlib/helpers/conf.rs:5-22`).
///
/// A missing key, and a receiver that is not a dict at all, both give the
/// default. The reference logs the second case and carries on; there is no
/// error path.
fn conf_get(conf: &BundValue, key: &str) -> Option<BundValue> {
    conf.get(key)
}

fn conf_f64(conf: &BundValue, key: &str, default: f64) -> f64 {
    match conf_get(conf, key) {
        Some(v) => match v.unboxed() {
            BundValue::Float(f) => *f,
            BundValue::Int(i) => *i as f64,
            _ => default,
        },
        None => default,
    }
}

fn conf_i64(conf: &BundValue, key: &str, default: i64) -> i64 {
    match conf_get(conf, key) {
        Some(v) => match v.unboxed() {
            BundValue::Int(i) => *i,
            BundValue::Float(f) => *f as i64,
            _ => default,
        },
        None => default,
    }
}

/// `seq` — a LIST of floats described by a config dict
/// (`reference/Bund/src/stdlib/functions/math/seq.rs:53-70`).
///
/// Four keys, each with a default the reference hardcodes: `type`
/// (`"seq.ascending"`), `X` (`0.0`), `Step` (`1.0`) and `N` (`128`). The
/// elements are **always floats**, whatever the config holds — `seq_ascending`
/// and its siblings push `Value::from_float` unconditionally (`:23-25`).
///
/// Confirmed against the oracle:
///
/// ```text
/// config :N 5 set :X 1.0 set :Step 2.0 set seq   ->  [1.0, 3.0, 5.0, 7.0, 9.0]
/// config :N 3 set :type "seq.descending" set …   ->  [10.0, 7.5, 5.0]
/// config :N 3 set :type "single" set :X 7.0 set  ->  [7.0, 7.0, 7.0]
/// config :N 3 set                                ->  [0.0, 1.0, 2.0]
/// ```
fn seq(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(conf) = vm.pull() else {
        return Err(Error("SEQ_OP returns: NO DATA #1".into()));
    };
    let kind = conf_get(&conf, "type")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| "seq.ascending".to_string());
    let x = conf_f64(&conf, "X", 0.0);
    let step = conf_f64(&conf, "Step", 1.0);
    let n = conf_i64(&conf, "N", 128).max(0) as usize;

    let items: Vec<BundValue> = match kind.as_str() {
        "seq.ascending" => (0..n)
            .map(|i| BundValue::Float(x + step * i as f64))
            .collect(),
        "seq.descending" => (0..n)
            .map(|i| BundValue::Float(x - step * i as f64))
            .collect(),
        "single" => (0..n).map(|_| BundValue::Float(x)).collect(),
        other => return Err(Error(format!("Unknown SEQ type: {other}"))),
    };
    vm.push(BundValue::list(items));
    Ok(())
}

/// `times` — run a lambda `n` times, **pushing the counter each round**
/// (`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:7-45`).
///
/// The counter runs `0..n`, so the body sees `0` first and `n-1` last, and a
/// body that does not consume it leaves `n` values behind. A non-positive `n`
/// runs the body zero times.
///
/// The lambda is on top — pushed last — so `n { … } times` reads in order.
fn times(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline times".into()));
    }
    let lambda_val = crate::pull::operand(vm, "TIMES", 1)?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error("TIMES: #1 parameter must be lambda".into()));
    }
    let n_val = crate::pull::operand(vm, "TIMES", 2)?;
    let n = n_val
        .as_int()
        .ok_or_else(|| Error("TIMES returns error: operand is not an integer".into()))?;
    let body = lambda_val
        .as_lambda()
        .ok_or_else(|| Error::internal("a value tagged LAMBDA carried no body"))?
        .to_vec();
    for v in 0..n {
        vm.push(BundValue::Int(v));
        vm.eval_body(&body)
            .map_err(|e| Error(format!("TIMES: lambda execution returns error: {}", e.0)))?;
    }
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.register_native("seq", seq, eff(1, 1), WordKind::Sync);
    r.register_native("times", times, eff(2, 0), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::Interp;

    fn run_src(src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    fn floats(src: &str) -> Vec<f64> {
        let i = run_src(src).expect("runs");
        let v = i.peek().expect("a value");
        v.as_list()
            .expect("a list")
            .iter()
            .filter_map(|e| match e.unboxed() {
                BundValue::Float(f) => Some(*f),
                _ => None,
            })
            .collect()
    }

    /// All four confirmed against the oracle.
    #[test]
    fn seq_generates_what_the_oracle_generates() {
        assert_eq!(
            floats("config :N 5 set :X 1.0 set :Step 2.0 set seq"),
            vec![1.0, 3.0, 5.0, 7.0, 9.0]
        );
        assert_eq!(
            floats("config :N 3 set :type \"seq.descending\" set :X 10.0 set :Step 2.5 set seq"),
            vec![10.0, 7.5, 5.0]
        );
        assert_eq!(
            floats("config :N 3 set :type \"single\" set :X 7.0 set seq"),
            vec![7.0, 7.0, 7.0]
        );
    }

    /// The defaults are the reference's hardcoded ones: X 0.0, Step 1.0,
    /// type ascending. (N defaults to 128 and is not exercised here.)
    #[test]
    fn seq_defaults_every_key() {
        assert_eq!(floats("config :N 3 set seq"), vec![0.0, 1.0, 2.0]);
    }

    /// Elements are floats even when the config holds integers.
    #[test]
    fn seq_always_yields_floats() {
        let i = run_src("config :N 2 set :X 1 set :Step 1 set seq").expect("runs");
        let v = i.peek().expect("a value");
        for e in v.as_list().expect("a list") {
            assert_eq!(e.dt(), bund2_value::FLOAT, "{:?}", e.summary(40));
        }
    }

    #[test]
    fn an_unknown_seq_type_is_named() {
        let e = match run_src("config :type \"nope\" set seq") {
            Ok(_) => panic!("expected a failure"),
            Err(e) => e,
        };
        assert!(e.contains("Unknown SEQ type: nope"), "{e}");
    }

    /// The counter is pushed each round, so a body that ignores it leaves `n`
    /// values behind.
    /// The body is `0 drop`, not `{ }`: an empty lambda does not parse (F51),
    /// which this test discovered by trying.
    #[test]
    fn times_pushes_the_counter() {
        let i = run_src("3 { 0 drop } times").expect("runs");
        assert_eq!(i.depth(), 3);
        assert_eq!(i.snapshot()[0].as_int(), Some(0), "counts from zero");
        assert_eq!(i.snapshot()[2].as_int(), Some(2), "and stops at n-1");
    }

    #[test]
    fn times_runs_the_body() {
        let i = run_src("3 { drop 1 } times").expect("runs");
        assert_eq!(i.depth(), 3);
        assert!(i.snapshot().iter().all(|v| v.as_int() == Some(1)));
    }

    #[test]
    fn times_with_a_non_positive_count_runs_nothing() {
        assert_eq!(run_src("0 { 9 } times").expect("runs").depth(), 0);
        assert_eq!(run_src("-2 { 9 } times").expect("runs").depth(), 0);
    }

    #[test]
    fn times_requires_a_lambda() {
        let e = match run_src("3 4 times") {
            Ok(_) => panic!("expected a failure"),
            Err(e) => e,
        };
        assert!(e.contains("TIMES: #1 parameter must be lambda"), "{e}");
    }
}
