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
use bund2_value::{BundValue, CFLOAT, FLOAT, LIST, MATRIX, Metric, STRING, TEXTBUFFER};

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

/// How deep a MATRIX of matrices is followed before an element is left alone.
///
/// The reference recurses per level with no bound and would exhaust its stack
/// (`reference/rust_dynamic/src/math.rs:51`). Past this depth an element's
/// operation is an error, which the matrix arm already answers by returning
/// its left operand untouched — the reference's own answer to any element
/// that will not combine (`:53-55`). D39.
const MAX_MATRIX_DEPTH: usize = 32;

/// `Value::numeric_op` (`reference/rust_dynamic/src/math.rs:127-409`).
///
/// `x` is the **top** of the stack.
///
/// **The reference dispatches on `x`'s payload first and its tag second**, and
/// this follows it arm for arm, in its order. That order is the behaviour: a
/// BOOL has a payload no arm names and a tag the second match does not name
/// either, so `true 1 +` is refused by the *last* arm, with the `X` sentence
/// and the tag, while `1 true +` is refused by the integer's arm with the `Y`
/// sentence and no tag.
///
/// An earlier version matched the numeric pairs, tried strings, and answered
/// everything else `Incompartible Y argument for the math operations`. A
/// 22-by-22 matrix of operand kinds against the oracle found that wrong for
/// 362 of 484 pairs under `+` alone — F168.
pub(crate) fn numeric_op(op: Op, x: &BundValue, y: &BundValue) -> Result<BundValue, Error> {
    numeric_op_at(op, x, y, 0)
}

fn numeric_op_at(op: Op, x: &BundValue, y: &BundValue, depth: usize) -> Result<BundValue, Error> {
    use BundValue::{Float, Int};

    // **METRICS: `Add` shifts a sample in, and nothing else does anything**
    // (`:129-138`, `:16-23`). The operand is whatever converts to FLOAT — a
    // list by its length, a bool as 0 or 1 — and the buffer keeps its size:
    // `push` appends and drops the oldest
    // (`reference/rust_dynamic/src/push.rs:118-126`).
    if let Some(samples) = x.as_metrics() {
        let sample = crate::convert::conv_value(y, FLOAT)
            .ok()
            .and_then(|f| match f.unboxed() {
                Float(f, _) => Some(*f),
                _ => None,
            })
            .ok_or_else(|| {
                Error("Incompartible Y argument for the metrics math operations".into())
            })?;
        if op != Op::Add {
            return Ok(x.clone());
        }
        let mut next = samples.to_vec();
        next.push(Metric::new(sample));
        next.remove(0);
        return Ok(BundValue::metrics(next));
    }

    // **JSON merges with JSON under `Add`** and is returned untouched by the
    // other three (`:139-146`, `:25-34`).
    if let Some(mut a) = x.as_json() {
        let Some(b) = y.as_json() else {
            return Err(Error(
                "Incompartible X and Y argument for the JSON math operations".into(),
            ));
        };
        if op == Op::Add {
            json_merge(&mut a, &b);
        }
        return Ok(BundValue::json(a));
    }

    let ys = y.as_str();
    match x.unboxed() {
        // A float on top (`:147-176`). Its refusal says **`X`**, where the
        // integer's below says `Y`; neither names a tag.
        Float(a, _) => {
            return match y.unboxed() {
                Float(b, _) => {
                    if op == Op::Div && *b == 0.0 {
                        return Err(Error("Float-point division to 0.0".into()));
                    }
                    Ok(BundValue::float(op.float(*a, *b)))
                }
                // The *divisor's* zero test uses the divisor's own kind.
                Int(b, _) => {
                    if op == Op::Div && *b == 0 {
                        return Err(Error("Integer division to 0.0".into()));
                    }
                    Ok(BundValue::float(op.float(*a, *b as f64)))
                }
                // The string leads and the float is **truncated to an
                // integer** first (`:171-173`): `"s" 2.5 +` is `"s2"`.
                _ => match &ys {
                    Some(b) => Ok(BundValue::str(string_op_int(op, b, *a as i64)?)),
                    None => Err(Error(
                        "Incompartible X argument for the math operations".into(),
                    )),
                },
            };
        }
        // An integer on top (`:177-206`).
        Int(a, _) => {
            return match y.unboxed() {
                Float(b, _) => {
                    if op == Op::Div && *b == 0.0 {
                        return Err(Error("Float-point division to 0.0".into()));
                    }
                    Ok(BundValue::float(op.float(*a as f64, *b)))
                }
                Int(b, _) => {
                    if op == Op::Div && *b == 0 {
                        return Err(Error("Integer division to 0.0".into()));
                    }
                    Ok(BundValue::int(op.int(*a, *b)))
                }
                // The operands swap places (`:200-202`), so the string leads.
                _ => match &ys {
                    Some(b) => Ok(BundValue::str(string_op_int(op, b, *a)?)),
                    None => Err(Error(
                        "Incompartible Y argument for the math operations".into(),
                    )),
                },
            };
        }
        _ => {}
    }

    // **A string on top takes anything that converts to one** (`:236-293`).
    // A string, an integer and a float each have an arm; everything else is
    // `conv(STRING)` and concatenated, so `[ 9 ] "s" +` is `"s[ 9 :: ]"`. What
    // will not convert is refused with `conv`'s own sentence after this arm's.
    //
    // The arm is chosen by *payload*, so a PTR and a CALL take it too, and the
    // answer is a plain STRING for all of them — except a TEXTBUFFER, which
    // stays one.
    if let Some(a) = x.as_str() {
        let text = match (&ys, y.unboxed()) {
            (Some(b), _) => string_op(op, &a, b),
            (None, Int(b, _)) => string_op_int(op, &a, *b)?,
            (None, Float(b, _)) => string_op_float(op, &a, *b)?,
            _ => match crate::convert::conv_value(y, STRING) {
                Ok(b) => string_op(op, &a, &b.as_str().unwrap_or_default()),
                Err(e) => {
                    return Err(Error(format!(
                        "Incompartible Y argument for the string operations: {}",
                        e.0
                    )));
                }
            },
        };
        return Ok(if x.dt() == TEXTBUFFER {
            BundValue::textbuffer(text)
        } else {
            BundValue::str(text)
        });
    }

    // From here the reference matches the **tag** (`:294-408`).
    match x.dt() {
        // A LIST on top is append, and only under `Add` (`:296-325`). Two
        // lists concatenate; a list and anything else pushes the anything-else
        // as one element.
        LIST => {
            if op != Op::Add {
                return Err(Error("Incompartible operation for the list".into()));
            }
            let mut items = x.as_list().unwrap_or_default().to_vec();
            match (y.dt() == LIST, y.as_list()) {
                (true, Some(tail)) => items.extend(tail.iter().cloned()),
                _ => items.push(y.clone()),
            }
            Ok(BundValue::list(items))
        }
        // A MATRIX combines with a MATRIX cell by cell, and takes a LIST as a
        // new row under `Add` (`:327-352`). Note which sentence the list arm's
        // refusal uses: `for the list`, though the operand on top is a matrix.
        MATRIX => {
            let rows = x.as_matrix().unwrap_or_default();
            match y.dt() {
                MATRIX => Ok(BundValue::matrix(matrix_op(
                    op,
                    rows,
                    y.as_matrix().unwrap_or_default(),
                    depth,
                ))),
                LIST => {
                    if op != Op::Add {
                        return Err(Error("Incompartible operation for the list".into()));
                    }
                    let mut next = rows.to_vec();
                    next.push(y.as_list().unwrap_or_default().to_vec());
                    Ok(BundValue::matrix(next))
                }
                _ => Err(Error("Incompartible operation for the matrix".into())),
            }
        }
        // Complex with complex, and nothing else (`:361-368`).
        CFLOAT => match (complex_parts(x), y.dt() == CFLOAT, complex_parts(y)) {
            (Some(a), true, Some(b)) => {
                let (re, im) = complex_op(op, a, b);
                Ok(BundValue::complex_float(re, im))
            }
            _ => Err(Error(
                "Incompartible Y argument for the math operations".into(),
            )),
        },
        // **Everything else on top** — a BOOL, a LAMBDA, a PAIR, a MAP, a
        // CLASS, an OBJECT, a VALUEMAP, a TIME, NODATA — is appended to a LIST
        // underneath under `Add`, and otherwise refused by tag (`:369-405`).
        dt => {
            if y.dt() != LIST {
                return Err(Error(format!(
                    "Incompartible X argument for the math operations: {dt}"
                )));
            }
            if op != Op::Add {
                return Err(Error("Incompartible operation for the list".into()));
            }
            let mut items = y.as_list().unwrap_or_default().to_vec();
            items.push(x.clone());
            Ok(BundValue::list(items))
        }
    }
}

/// `numeric_op_matrix` (`reference/rust_dynamic/src/math.rs:36-65`).
///
/// **Any mismatch returns the left matrix unchanged, silently**: a different
/// number of rows, a row of a different length, or a pair of cells that will
/// not combine. No error is raised for any of them.
fn matrix_op(
    op: Op,
    x: &[Vec<BundValue>],
    y: &[Vec<BundValue>],
    depth: usize,
) -> Vec<Vec<BundValue>> {
    if x.len() != y.len() || depth >= MAX_MATRIX_DEPTH {
        return x.to_vec();
    }
    let mut res = Vec::with_capacity(x.len());
    for (xr, yr) in x.iter().zip(y) {
        if xr.len() != yr.len() {
            return x.to_vec();
        }
        let mut row = Vec::with_capacity(xr.len());
        for (a, b) in xr.iter().zip(yr) {
            match numeric_op_at(op, a, b, depth + 1) {
                Ok(v) => row.push(v),
                Err(_) => return x.to_vec(),
            }
        }
        res.push(row);
    }
    res
}

/// The two floats a CFLOAT holds.
pub(crate) fn complex_parts(v: &BundValue) -> Option<(f64, f64)> {
    match v.as_list()? {
        [re, im] => match (re.unboxed(), im.unboxed()) {
            (BundValue::Float(re, _), BundValue::Float(im, _)) => Some((*re, *im)),
            _ => None,
        },
        _ => None,
    }
}

/// `numeric_op_cpx_float_cpx_float` (`:94-101`), which is `num::Complex`'s
/// four operators written out.
fn complex_op(op: Op, (a, b): (f64, f64), (c, d): (f64, f64)) -> (f64, f64) {
    match op {
        Op::Add => (a + c, b + d),
        Op::Sub => (a - c, b - d),
        Op::Mul => (a * c - b * d, a * d + b * c),
        Op::Div => {
            let n = c * c + d * d;
            ((a * c + b * d) / n, (b * c - a * d) / n)
        }
    }
}

/// `json_value_merge::Merge::merge`, version 2.0.1, which is what
/// `numeric_op_json_json` calls (`:25-34`).
///
/// Objects merge key by key, arrays concatenate, an object merged into an
/// array is appended to it, and any other pair is replaced by the right side.
/// The crate recurses per level of object; this keeps the pending pairs on the
/// heap instead (D39).
fn json_merge(a: &mut serde_json::Value, b: &serde_json::Value) {
    use serde_json::Value;
    let mut work: Vec<(&mut Value, &Value)> = vec![(a, b)];
    while let Some((a, b)) = work.pop() {
        match (a, b) {
            (Value::Object(a), Value::Object(b)) => {
                // Each key is a distinct slot, so the borrows do not overlap;
                // collecting them through `iter_mut` is what lets the
                // compiler see that.
                for (k, _) in b {
                    a.entry(k.clone()).or_insert(Value::Null);
                }
                for (k, slot) in a.iter_mut() {
                    if let Some(v) = b.get(k) {
                        work.push((slot, v));
                    }
                }
            }
            (Value::Array(a), Value::Array(b)) => a.extend(b.iter().cloned()),
            (Value::Array(a), Value::Object(_)) => a.push(b.clone()),
            (a, b) => *a = b.clone(),
        }
    }
}

/// `str::repeat`, refusing a product that cannot be held.
///
/// The reference calls `repeat` bare (`reference/rust_dynamic/src/math.rs:112,120`)
/// and **aborts** on `capacity overflow` for a count too large. Bund2 let that
/// panic inside the native, where it was caught and reported as an *internal*
/// error. It is not one: the program asked for it. D37, and the treatment D98
/// gave `io.textfile`.
///
/// A *negative* count aborts the reference the same way, because `as usize`
/// turns it into an enormous one. Bund2 has always taken it as zero and
/// answered the empty string; that is left as it was.
fn repeat(x: &str, count: usize) -> Result<String, Error> {
    let refuse = || {
        Error(format!(
            "a string of {} bytes cannot be repeated {count} times",
            x.len()
        ))
    };
    let total = x.len().checked_mul(count).ok_or_else(refuse)?;
    let mut out = String::new();
    out.try_reserve_exact(total).map_err(|_| refuse())?;
    for _ in 0..count {
        out.push_str(x);
    }
    Ok(out)
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
fn string_op_int(op: Op, x: &str, y: i64) -> Result<String, Error> {
    Ok(match op {
        Op::Mul => repeat(x, y.max(0) as usize)?,
        Op::Add => format!("{x}{y}"),
        _ => x.to_string(),
    })
}

/// `string_op_string_float` (`reference/rust_dynamic/src/math.rs:118-124`).
/// `Mul` repeats by the float **cast to a count**, which truncates and takes a
/// negative or a NaN as zero; `Add` appends the float as `{}` prints it, so
/// `2.0` appends `2`.
fn string_op_float(op: Op, x: &str, y: f64) -> Result<String, Error> {
    Ok(match op {
        Op::Mul => repeat(x, y as usize)?,
        Op::Add => format!("{x}{y}"),
        _ => x.to_string(),
    })
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
    // The depth, not a snapshot: a snapshot reads the whole workbench to
    // answer a question about its length, and D55's audit counts that as
    // reading beyond the operands.
    if vm.depth() < 1 || vm.workbench_depth() < 1 {
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
            crate::wb::bench!(r, $name, |vm| float_op(vm, |x| x.$m()));
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

    /// F168: which arm refuses, and in which words. Each row measured on the
    /// oracle; the left column is read with the **last** term on top.
    ///
    /// The sentences differ by arm, and that is the point of the test: `X`
    /// with a tag from the last arm, `X` without one from the float's, `Y`
    /// from the integer's, and each container's own.
    #[test]
    fn each_arm_refuses_in_its_own_words() {
        for (src, want) in [
            // The last arm: no payload arm and no tag arm. The tag is named.
            ("1 true +", "ADD returns error: Incompartible X argument for the math operations: 1"),
            ("\"s\" { 7 } +", "ADD returns error: Incompartible X argument for the math operations: 17"),
            ("1 dict +", "ADD returns error: Incompartible X argument for the math operations: 11"),
            ("1 nodata -", "SUB returns error: Incompartible X argument for the math operations: 97"),
            // ...which under a list is an append, and only under `+`.
            ("[ 9 ] true -", "SUB returns error: Incompartible operation for the list"),
            // A float on top says `X`; an integer on top says `Y`. No tag.
            ("true 2.5 +", "ADD returns error: Incompartible X argument for the math operations"),
            ("true 2 +", "ADD returns error: Incompartible Y argument for the math operations"),
            // A string on top: what will not convert is refused in `conv`'s words.
            (
                "1 2 pair \"s\" +",
                "ADD returns error: Incompartible Y argument for the string operations: Can not convert Value from 10",
            ),
            (
                "class \"s\" +",
                "ADD returns error: Incompartible Y argument for the string operations: Source value is not MAP but 31 and not suitable for conversion",
            ),
            (
                "[ [ 1 ] ] matrix \"s\" +",
                "ADD returns error: Incompartible Y argument for the string operations: Can not convert list to 4",
            ),
            // The containers.
            ("1 [ [ 1 ] ] matrix +", "ADD returns error: Incompartible operation for the matrix"),
            ("[ 1 ] [ [ 1 ] ] matrix -", "SUB returns error: Incompartible operation for the list"),
            ("1 1.0 2.0 complex +", "ADD returns error: Incompartible Y argument for the math operations"),
            ("\"s\" metrics +", "ADD returns error: Incompartible Y argument for the metrics math operations"),
            (
                "1 '{\"a\":1}' json +",
                "ADD returns error: Incompartible X and Y argument for the JSON math operations",
            ),
        ] {
            let e = err_of(src);
            assert!(e.ends_with(want), "{src}: {e}");
        }
    }

    /// The arms whose answer cannot be printed, and so cannot sit in the probe.
    #[test]
    fn complex_and_metrics_arithmetic() {
        // `complex` takes the real part from the top, so these are 2+1i and
        // -1.5+3i, and their product is -6 + 4.5i. The multiplication is
        // written out rather than taken from a crate, so it is checked against
        // one worked by hand -- and the oracle agrees.
        let i = run_src("3.0 -1.5 complex 1.0 2.0 complex *").expect("runs");
        let v = i.peek().expect("a value");
        assert_eq!(complex_parts(&v), Some((-6.0, 4.5)));

        // `+` shifts one sample in and keeps the buffer's 128; the other
        // three return it untouched.
        let i = run_src("7 metrics +").expect("runs");
        let m = i.peek().and_then(|v| v.as_metrics().map(<[Metric]>::to_vec)).expect("metrics");
        assert_eq!(m.len(), 128);
        assert_eq!(m.last().map(|s| s.data), Some(7.0));
        let i = run_src("7 metrics *").expect("runs");
        let m = i.peek().and_then(|v| v.as_metrics().map(<[Metric]>::to_vec)).expect("metrics");
        assert_eq!(m.last().map(|s| s.data), Some(0.0));
    }

    /// A repeat too large to hold is the program's error, not Bund2's.
    ///
    /// The reference aborts here; Bund2 panicked inside the native and called
    /// it an internal error.
    #[test]
    fn an_impossible_repeat_is_reported_as_the_programs() {
        let e = err_of("\"ab\" 4611686018427387904 *");
        assert!(
            e.ends_with("MUL returns error: a string of 2 bytes cannot be repeated 4611686018427387904 times"),
            "{e}"
        );
        assert!(!e.contains("internal error"), "{e}");
        // A negative count has always been taken as zero.
        assert_eq!(top_str("\"ab\" -3 *").as_deref(), Some(""));
    }

    /// The merge is `json_value_merge`'s, written as a worklist.
    #[test]
    fn json_merge_follows_the_crate_it_replaces() {
        let j = |s: &str| serde_json::from_str::<serde_json::Value>(s).expect("json");
        for (a, b, want) in [
            (r#"["a","b"]"#, r#"["b","c"]"#, r#"["a","b","b","c"]"#),
            (
                r#"{"value1":"a","value2":"b"}"#,
                r#"{"value1":"a","value2":"c","value3":"d"}"#,
                r#"{"value1":"a","value2":"c","value3":"d"}"#,
            ),
            ("[]", r#"{"field1":"value1"}"#, r#"[{"field1":"value1"}]"#),
            (r#"{"field1":"value1"}"#, r#"["value2","value3"]"#, r#"["value2","value3"]"#),
            (r#"{"a":{"b":{"c":1}}}"#, r#"{"a":{"b":{"d":2}}}"#, r#"{"a":{"b":{"c":1,"d":2}}}"#),
        ] {
            let mut got = j(a);
            json_merge(&mut got, &j(b));
            assert_eq!(got, j(want), "{a} <- {b}");
        }
    }
}
