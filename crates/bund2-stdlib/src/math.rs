//! Arithmetic — `+`, `-`, `*`, `/`.
//!
//! **The operands are reversed**, as they are for the comparisons: the top of
//! the stack is the *left* side. `math_op` pulls `value1` first and computes
//! `numeric_op(op, value1, value2)`
//! (`reference/rust_multistackvm/src/stdlib/math/math_op.rs:7-20`,
//! `reference/rust_multistackvm/src/stdlib/math/add.rs:6-8`). So `3 5 -` asks
//! for `5 - 3` and answers `2`, and `10 2 /` answers `0`. Both confirmed
//! against the oracle.
//!
//! Strings participate. `"a" "b" +` is `"ba"` — the top operand leads —  and
//! `"ab" 3 *` repeats. A non-`Add` operation on two strings **silently returns
//! the left operand** rather than failing (`reference/rust_dynamic/src/math.rs:106`),
//! which is F64 and is preserved here.
//!
//! `q` is **not** averaged by arithmetic. `numeric_op` builds a fresh value and
//! `calc_q` (`reference/rust_dynamic/src/q.rs:4`) is never called from
//! `math.rs`, so a result carries the default 100.0. Confirmed by probe:
//! `1 2 + debug.display_stack` shows `q: 100.0`.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl Op {
    /// The prefix the reference's errors carry — `"ADD"`, `"SUB"`, and so on
    /// (`reference/rust_multistackvm/src/stdlib/math/add.rs:7`).
    fn prefix(self) -> &'static str {
        match self {
            Op::Add => "ADD",
            Op::Sub => "SUB",
            Op::Mul => "MUL",
            Op::Div => "DIV",
        }
    }

    fn int(self, x: i64, y: i64) -> i64 {
        match self {
            Op::Add => x.wrapping_add(y),
            Op::Sub => x.wrapping_sub(y),
            Op::Mul => x.wrapping_mul(y),
            Op::Div => x.wrapping_div(y),
        }
    }

    fn float(self, x: f64, y: f64) -> f64 {
        match self {
            Op::Add => x + y,
            Op::Sub => x - y,
            Op::Mul => x * y,
            Op::Div => x / y,
        }
    }
}

/// `Value::numeric_op` for the arms Bund2 can construct
/// (`reference/rust_dynamic/src/math.rs:127-235`).
///
/// `x` is the **top** of the stack.
pub(crate) fn numeric_op(op: Op, x: &BundValue, y: &BundValue) -> Result<BundValue, Error> {
    use BundValue::{Float, Int};
    let (xa, ya) = (x.unboxed(), y.unboxed());
    let xs = x.as_str();
    let ys = y.as_str();

    // **A LIST left operand is append, and only under `Add`**
    // (`reference/rust_dynamic/src/math.rs:296-324`). Two lists concatenate
    // (`:299-308`); a list and anything else pushes the anything-else as one
    // element (`:315-320`). Every other operation on a list is
    // `Incompartible operation for the list` (`:322`).
    //
    // This is checked before the numeric arms because it dispatches on the
    // **tag**, where those dispatch on the payload — a LIST has no scalar arm
    // to match, so reaching the numeric `match` at all means falling through
    // to the string fallback and reporting the wrong error. That is what
    // `dynamic_demo_4.bund` hit: `list . … +.` appends a name to a list on the
    // workbench, and it reported
    // `Incompartible Y argument for the math operations`.
    if x.dt() == LIST {
        if op != Op::Add {
            return Err(Error("Incompartible operation for the list".into()));
        }
        let mut items = x.as_list().unwrap_or_default().to_vec();
        match (y.dt() == LIST, y.as_list()) {
            (true, Some(tail)) => items.extend(tail.iter().cloned()),
            _ => items.push(y.clone()),
        }
        return Ok(BundValue::list(items));
    }

    match (xa, ya) {
        (Int(a, _), Int(b, _)) => {
            if op == Op::Div && *b == 0 {
                return Err(Error("Integer division to 0.0".into()));
            }
            Ok(BundValue::int(op.int(*a, *b)))
        }
        (Float(a, _), Float(b, _)) => {
            if op == Op::Div && *b == 0.0 {
                return Err(Error("Float-point division to 0.0".into()));
            }
            Ok(BundValue::float(op.float(*a, *b)))
        }
        // Mixed kinds promote to float, and the *divisor's* zero test uses the
        // divisor's own kind (`math.rs:160-170,178-188`).
        (Float(a, _), Int(b, _)) => {
            if op == Op::Div && *b == 0 {
                return Err(Error("Integer division to 0.0".into()));
            }
            Ok(BundValue::float(op.float(*a, *b as f64)))
        }
        (Int(a, _), Float(b, _)) => {
            if op == Op::Div && *b == 0.0 {
                return Err(Error("Float-point division to 0.0".into()));
            }
            Ok(BundValue::float(op.float(*a as f64, *b)))
        }
        _ => {
            // String arms. An `I64` left operand with a string right operand
            // reaches `string_op_string_int(op, s_y, i_x)` — note the operands
            // swap places (`math.rs:200-202`), so the *string* leads.
            match (xs, ys, xa, ya) {
                (Some(a), Some(b), _, _) => Ok(BundValue::str(string_op(op, &a, &b))),
                (_, Some(b), Int(a, _), _) => Ok(BundValue::str(string_op_int(op, &b, *a))),
                (Some(a), _, _, Int(b, _)) => Ok(BundValue::str(string_op_int(op, &a, *b))),
                _ => Err(Error("Incompartible Y argument for the math operations".into())),
            }
        }
    }
}

/// `string_op_string_string` (`reference/rust_dynamic/src/math.rs:103-108`).
///
/// **F64: only `Add` does anything.** Every other operation returns `x`
/// unchanged, silently — `"a" "b" -` is `"b"` on the oracle, not an error.
fn string_op(op: Op, x: &str, y: &str) -> String {
    match op {
        Op::Add => format!("{x}{y}"),
        _ => x.to_string(),
    }
}

/// `string_op_string_int` (`reference/rust_dynamic/src/math.rs:110-116`).
/// `Mul` repeats, `Add` appends the number's text, and everything else is the
/// same silent pass-through.
fn string_op_int(op: Op, x: &str, y: i64) -> String {
    match op {
        Op::Mul => x.repeat(y.max(0) as usize),
        Op::Add => format!("{x}{y}"),
        _ => x.to_string(),
    }
}

fn run(op: Op, vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(format!(
            "Stack is too shallow for inline {}()",
            op.prefix()
        )));
    }
    let x = crate::pull::operand(vm, op.prefix(), 1)?;
    let y = crate::pull::operand(vm, op.prefix(), 2)?;
    match numeric_op(op, &x, &y) {
        Ok(v) => {
            vm.push(v);
            Ok(())
        }
        Err(e) => Err(Error(format!("{} returns error: {}", op.prefix(), e.0))),
    }
}

/// The `.` form — **D24's contract, and it crosses two stacks.**
///
/// Operand #1 comes off the **workbench**, operand #2 off the **main stack**
/// with no branch at all, and the result goes back to the **workbench**
/// (`reference/rust_multistackvm/src/stdlib/math/math_op.rs:96,101,107`). So
/// `+.` is not "`+` on the workbench": it is `-1` workbench, `-1` stack, `+1`
/// workbench, which is why RFC-0004 gives a stack effect two axes.
///
/// The guard is both stacks — main at 1 *and* workbench at 1 (`:86-91`) — and
/// both report the same "Stack is too shallow" text even though one of them is
/// about the workbench.
fn run_workbench(op: Op, vm: &mut dyn Vm) -> Result<(), Error> {
    let prefix = format!("{}.", op.prefix());
    if vm.depth() < 1 || vm.snapshot_workbench().is_empty() {
        return Err(Error(format!("Stack is too shallow for inline {prefix}()")));
    }
    let x = vm
        .pull_workbench()
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #1")))?;
    let y = crate::pull::operand(vm, &prefix, 2)?;
    match numeric_op(op, &x, &y) {
        Ok(v) => {
            vm.push_workbench(v);
            Ok(())
        }
        Err(e) => Err(Error(format!("{prefix} returns error: {}", e.0))),
    }
}

fn add(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Add, vm)
}

/// The `*` fold family — `*+`, `*-`, `**`, `*/`, their `.` forms, and `Σ` and
/// `Σ.` for `*+` (D12). One function serves all eight
/// (`reference/rust_multistackvm/src/stdlib/math/math_op.rs:22-76`).
///
/// It pulls the accumulator from its side — the stack, or the workbench for the
/// `.` form — and one operand from the stack, applies the binary operation as
/// `+` would, and pushes the result back to the accumulator's side (`:36-58`).
/// It stops when the stack runs out, and at a NODATA, which it consumes
/// (`:43-50`); either way the accumulator goes back where it came from. Each
/// turn takes a value off the stack, so the loop always ends.
///
/// The prefix is `*` and the operation's, `*ADD` and `*ADD.`
/// (`reference/rust_multistackvm/src/stdlib/math/add.rs`), and both guards say
/// "Stack is too shallow", the workbench one included (`:24-34`).
fn fold(op: Op, vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    let prefix = match side {
        crate::wb::Side::Stack => format!("*{}", op.prefix()),
        crate::wb::Side::Bench => format!("*{}.", op.prefix()),
    };
    let short = match side {
        crate::wb::Side::Stack => vm.depth() < 2,
        crate::wb::Side::Bench => vm.depth() < 1 || vm.workbench_depth() < 1,
    };
    if short {
        return Err(Error(format!("Stack is too shallow for inline {prefix}()")));
    }
    loop {
        let Some(x) = side.pull(vm) else {
            return Err(Error(format!("{prefix} can not get X")));
        };
        let Some(y) = vm.pull() else {
            side.push(vm, x);
            return Ok(());
        };
        if y.dt() == bund2_value::NODATA {
            side.push(vm, x);
            return Ok(());
        }
        match numeric_op(op, &x, &y) {
            Ok(v) => side.push(vm, v),
            Err(e) => return Err(Error(format!("{prefix} returns error: {}", e.0))),
        }
    }
}
fn add_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    run_workbench(Op::Add, vm)
}
fn sub_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    run_workbench(Op::Sub, vm)
}
fn mul_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    run_workbench(Op::Mul, vm)
}
fn div_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    run_workbench(Op::Div, vm)
}
fn sub(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Sub, vm)
}
fn mul(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Mul, vm)
}
fn div(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Div, vm)
}

/// The `math.*` float family — seventeen words, one body
/// (`reference/rust_multistackvm/src/stdlib/math/float_math.rs:93-165,170-188`).
///
/// Each is a plain `f64` method on the pulled operand, so the interesting part
/// is the gate rather than the arithmetic: the operand goes through
/// `cast_float`, which accepts **only** a `Val::F64`
/// (`reference/rust_dynamic/src/cast.rs:9-16`). An integer is refused —
/// `2 math.sqrt` is an error, `2.0 math.sqrt` is not — and the error names the
/// tag it got. This is a stricter gate than the arithmetic words use, which
/// coerce across int and float, and it is preserved as written.
///
/// The guard's message says `float_op` and not the word's own name (`:95`),
/// so every one of the seventeen reports the same text. That is F40's shape
/// and it is reproduced rather than improved.
fn float_op(vm: &mut dyn Vm, f: fn(f64) -> f64) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline float_op".into()));
    }
    let v = crate::pull::operand(vm, "FLOAT_OP", 1)?;
    let BundValue::Float(x, _) = *v.unboxed() else {
        // `cast_float`'s text, inside the base's wrapper (`:156`). Confirmed
        // against the oracle: `2 math.sqrt` reports
        // `FLOAT_OP returns error: This Dynamic type is not float: 2`.
        return Err(Error(format!(
            "FLOAT_OP returns error: This Dynamic type is not float: {}",
            v.dt()
        )));
    };
    vm.push(BundValue::float(f(x)));
    Ok(())
}

/// A series of floats for a statistics-style word, as the reference's
/// `get_data` takes one in `Consume` mode from the stack
/// (`reference/Bund/src/stdlib/functions/statistics/get_data.rs:184-222`).
///
/// What it takes depends on the **top** value, peeked (`:198`):
///
/// - a LIST is pulled and each element converted to FLOAT (`:117-169`);
/// - a METRICS is pulled and its data read (`:68-104`);
/// - a NODATA is `END OF DATA` (`:212`);
/// - anything else starts a **pull of the whole stack**, down to its end or
///   to a NODATA, converting each value (`:8-42`). So `1 2 3` handed to a
///   word that wants two series is swallowed entirely by the first.
fn data_series(vm: &mut dyn Vm, prefix: &str) -> Result<Vec<f64>, Error> {
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let Some(top) = vm.peek() else {
        return Err(Error(format!("{prefix} returns NO DATA")));
    };
    let as_float = |v: &BundValue| -> Result<f64, Error> {
        let f = crate::convert::conv_value(v, bund2_value::FLOAT)?;
        match *f.unboxed() {
            BundValue::Float(x, _) => Ok(x),
            _ => Err(Error::internal("a conversion to FLOAT answered another kind")),
        }
    };
    let mut res: Vec<f64> = Vec::new();
    match top.dt() {
        LIST => {
            let l = vm
                .pull()
                .ok_or_else(|| Error(format!("{prefix} returns NO DATA")))?;
            let items = l
                .as_list()
                .ok_or_else(|| Error(format!("{prefix} did not find a list type on the stack")))?;
            for v in items {
                res.push(
                    as_float(v)
                        .map_err(|e| Error(format!("{prefix} error FLOAT conversion: {}", e.0)))?,
                );
            }
        }
        bund2_value::METRICS => {
            let m = vm
                .pull()
                .ok_or_else(|| Error(format!("{prefix} returns NO DATA")))?;
            let metrics = m
                .as_metrics()
                .ok_or_else(|| Error(format!("{prefix} did not find a metric type on the stack")))?;
            res.extend(metrics.iter().map(|x| x.data));
        }
        bund2_value::NODATA => return Err(Error(format!("{prefix} END OF DATA"))),
        _ => {
            while let Some(v) = vm.pull() {
                if v.dt() == bund2_value::NODATA {
                    break;
                }
                res.push(as_float(&v).map_err(|e| {
                    Error(format!("{prefix} returns during conversion: {}", e.0))
                })?);
            }
        }
    }
    Ok(res)
}

/// `math.interpolation` — linear interpolation of `xp` over the points
/// `x`, `y` (`reference/Bund/src/stdlib/functions/math/interp.rs:12-38`),
/// by the reference's own `interp` crate.
///
/// X is the top series and Y the next (`:16,22`), then `xp` beneath them,
/// which must already be a FLOAT (`:29`). So the source reads
/// `<xp> <y> <x> math.interpolation`.
fn math_interpolation(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for MATH.INTERPOLATION".into()));
    }
    let x = data_series(vm, "MATH.INTERPOLATION")
        .map_err(|e| Error(format!("MATH.INTERPOLATION getting X returns: {}", e.0)))?;
    let y = data_series(vm, "MATH.INTERPOLATION")
        .map_err(|e| Error(format!("MATH.INTERPOLATION getting Y returns: {}", e.0)))?;
    let xp_val = vm
        .pull()
        .ok_or_else(|| Error("MATH.INTERPOLATION: NO DATA #3".into()))?;
    let BundValue::Float(xp, _) = *xp_val.unboxed() else {
        return Err(Error(format!(
            "MATH.INTERPOLATION error casting XP: This Dynamic type is not float: {}",
            xp_val.dt()
        )));
    };
    vm.push(BundValue::float(interp::interp(
        &x,
        &y,
        xp,
        &interp::InterpMode::default(),
    )));
    Ok(())
}

pub fn register(r: &mut Registry) {
    // The seventeen, in the reference's registration order (`:171-187`).
    macro_rules! float_word {
        ($name:literal, $m:ident) => {
            r.register_native(
                $name,
                |vm| float_op(vm, |x| x.$m()),
                eff(1, 1),
                WordKind::Sync,
            );
        };
    }
    float_word!("math.floor", floor);
    float_word!("math.abs", abs);
    float_word!("math.signum", signum);
    float_word!("math.cbrt", cbrt);
    float_word!("math.ceil", ceil);
    float_word!("math.round", round);
    float_word!("math.fract", fract);
    float_word!("math.sqrt", sqrt);
    float_word!("math.sin", sin);
    float_word!("math.cos", cos);
    float_word!("math.tan", tan);
    float_word!("math.asin", asin);
    float_word!("math.acos", acos);
    float_word!("math.atan", atan);
    float_word!("math.sinh", sinh);
    float_word!("math.cosh", cosh);
    float_word!("math.tanh", tanh);
    // The five `float.*` constants
    // (`reference/rust_multistackvm/src/stdlib/math/float.rs:31-35`). Each
    // pushes a FLOAT, and prints as Rust's `f64` does: `NaN`, `inf`, `-inf`.
    macro_rules! float_const {
        ($name:literal, $v:expr) => {
            r.register_native(
                $name,
                |vm| {
                    vm.push(BundValue::float($v));
                    Ok(())
                },
                eff(0, 1),
                WordKind::Sync,
            );
        };
    }
    float_const!("float.NaN", f64::NAN);
    float_const!("float.+Inf", f64::INFINITY);
    float_const!("float.-Inf", f64::NEG_INFINITY);
    float_const!("float.Pi", std::f64::consts::PI);
    float_const!("float.E", std::f64::consts::E);
    // `reference/rust_multistackvm/src/stdlib/create_aliases.rs:30-31`.
    r.register_alias("π", "float.Pi");
    r.register_alias("Pi", "float.Pi");
    // The folds (D12): opaque, because each consumes the stack down to its end
    // or a NODATA. Registered at `reference/rust_multistackvm/src/stdlib/math/`
    // `add.rs:25-26`, `sub.rs:25-26`, `mul.rs:25-26` and `div.rs:25-26`.
    r.register_native("*+", |vm| fold(Op::Add, vm, crate::wb::Side::Stack), StackEffect::opaque(2), WordKind::Sync);
    r.register_native("*-", |vm| fold(Op::Sub, vm, crate::wb::Side::Stack), StackEffect::opaque(2), WordKind::Sync);
    r.register_native("**", |vm| fold(Op::Mul, vm, crate::wb::Side::Stack), StackEffect::opaque(2), WordKind::Sync);
    r.register_native("*/", |vm| fold(Op::Div, vm, crate::wb::Side::Stack), StackEffect::opaque(2), WordKind::Sync);
    r.register_native("*+.",|vm| fold(Op::Add, vm, crate::wb::Side::Bench), StackEffect::opaque(1), WordKind::Sync);
    r.register_native("*-.", |vm| fold(Op::Sub, vm, crate::wb::Side::Bench), StackEffect::opaque(1), WordKind::Sync);
    r.register_native("**.", |vm| fold(Op::Mul, vm, crate::wb::Side::Bench), StackEffect::opaque(1), WordKind::Sync);
    r.register_native("*/.", |vm| fold(Op::Div, vm, crate::wb::Side::Bench), StackEffect::opaque(1), WordKind::Sync);
    // `reference/rust_multistackvm/src/stdlib/create_aliases.rs:38-39`.
    r.register_alias("Σ", "*+");
    r.register_alias("Σ.", "*+.");
    r.register_native("+", add, eff(2, 1), WordKind::Sync);
    r.register_native("-", sub, eff(2, 1), WordKind::Sync);
    r.register_native("*", mul, eff(2, 1), WordKind::Sync);
    r.register_native("/", div, eff(2, 1), WordKind::Sync);
    // `eff` cannot yet say "one from each stack" — RFC-0004 §S1 is what widens
    // it. Until then these declare the main-stack half, which is the half the
    // current shape can express, and the doc comment carries the rest.
    r.register_native("+.", add_wb, eff(1, 0), WordKind::Sync);
    r.register_native("-.", sub_wb, eff(1, 0), WordKind::Sync);
    r.register_native("*.", mul_wb, eff(1, 0), WordKind::Sync);
    r.register_native("/.", div_wb, eff(1, 0), WordKind::Sync);
    // `reference/Bund/src/stdlib/functions/math/interp.rs:48`. Opaque: when X
    // is not a list, the first series swallows the stack (`data_series`).
    r.register_native(
        "math.interpolation",
        math_interpolation,
        StackEffect::opaque(2),
        WordKind::Sync,
    );
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

    fn top_int(src: &str) -> Option<i64> {
        run_src(src).ok()?.peek()?.as_int()
    }

    fn top_str(src: &str) -> Option<String> {
        run_src(src).ok()?.peek()?.as_str()
    }

    /// `Interp` is not `Debug`, so `expect_err` is unavailable; this keeps the
    /// error and discards the VM.
    fn err_of(src: &str) -> String {
        match run_src(src) {
            Ok(_) => panic!("{src} was expected to fail"),
            Err(e) => e,
        }
    }

    /// The top of the stack is the left operand — `3 5 -` is `5 - 3`. All four
    /// confirmed against the oracle.
    #[test]
    fn the_top_of_the_stack_is_the_left_operand() {
        assert_eq!(top_int("3 5 -"), Some(2));
        assert_eq!(top_int("3 5 /"), Some(1));
        assert_eq!(top_int("10 2 /"), Some(0));
        assert_eq!(top_int("7 2 /"), Some(0));
    }

    #[test]
    fn mixed_kinds_promote_to_float() {
        let i = run_src("3 2.0 +").expect("runs");
        assert_eq!(i.peek().map(|v| v.dt()), Some(bund2_value::FLOAT));
        let j = run_src("2.0 3 -").expect("runs");
        assert_eq!(j.peek().map(|v| v.dt()), Some(bund2_value::FLOAT));
    }

    /// Strings concatenate top-first, and repeat under `*`.
    #[test]
    fn strings_participate() {
        assert_eq!(top_str("\"a\" \"b\" +").as_deref(), Some("ba"));
        assert_eq!(top_str("\"ab\" 3 *").as_deref(), Some("ababab"));
    }

    /// **F64, preserved.** A non-`Add` on two strings silently yields the left
    /// operand instead of failing.
    #[test]
    fn a_non_add_on_two_strings_is_a_silent_pass_through() {
        assert_eq!(top_str("\"a\" \"b\" -").as_deref(), Some("b"));
    }

    /// Dividing *by* zero means the zero is the second operand, which is the
    /// one written **first** — `0 1 /` is `1 / 0`. An earlier version of this
    /// test wrote `1 0 /`, which is `0 / 1` and answers 0; the reversal this
    /// module documents caught its own test.
    #[test]
    fn division_by_zero_names_the_operand_kind() {
        assert_eq!(top_int("1 0 /"), Some(0), "1 0 / is 0 / 1");
        let e = err_of("0 1 /");
        assert!(e.contains("Integer division to 0.0"), "{e}");
        let f = err_of("0.0 1.0 /");
        assert!(f.contains("Float-point division to 0.0"), "{f}");
    }

    /// **Arithmetic does not average `q`** — D32, as amended on Q35's answer.
    ///
    /// The operand at `q` 0.0 is what lets this fail. The earlier version added
    /// `1 2 +`, both at 100.0, where an average and a fresh default are the same
    /// number — so it passed under either rule and decided nothing. No Bund
    /// program can build a numeric value at another `q`, which is why this is
    /// constructed in Rust rather than parsed.
    #[test]
    fn arithmetic_leaves_q_at_the_default() {
        use bund2_api::Vm as _;
        let mut i = bund2_interp::Interp::new();
        crate::register_all(&mut i.registry);
        i.push(bund2_value::BundValue::int(2).with_q(0.0));
        i.push(bund2_value::BundValue::int(1));
        let stream = bund2_syntax::compile("+").expect("compiles");
        i.eval(&stream).expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(3));
        assert_eq!(
            i.peek().map(|v| v.q()),
            Some(100.0),
            "an average of 0.0 and 100.0 would read 50.0"
        );
    }

    #[test]
    fn a_shallow_stack_is_an_error_that_names_the_word() {
        let e = err_of("1 +");
        assert!(e.contains("Stack is too shallow for inline ADD()"), "{e}");
    }

    /// `xp` is `cast_float`ed, not converted (`interp.rs:29`), so an INTEGER
    /// is refused even though the lists beside it convert.
    #[test]
    fn interpolation_wants_a_float_xp() {
        let e = err_of("2 [ 10 20 ] [ 1 2 ] math.interpolation");
        assert!(e.contains("MATH.INTERPOLATION error casting XP"), "{e}");
    }
}
