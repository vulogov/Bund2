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
use bund2_value::BundValue;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect { consumes, produces }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Op {
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
fn numeric_op(op: Op, x: &BundValue, y: &BundValue) -> Result<BundValue, Error> {
    use BundValue::{Float, Int};
    let (xa, ya) = (x.unboxed(), y.unboxed());
    let xs = x.as_str();
    let ys = y.as_str();

    match (xa, ya) {
        (Int(a), Int(b)) => {
            if op == Op::Div && *b == 0 {
                return Err(Error("Integer division to 0.0".into()));
            }
            Ok(Int(op.int(*a, *b)))
        }
        (Float(a), Float(b)) => {
            if op == Op::Div && *b == 0.0 {
                return Err(Error("Float-point division to 0.0".into()));
            }
            Ok(Float(op.float(*a, *b)))
        }
        // Mixed kinds promote to float, and the *divisor's* zero test uses the
        // divisor's own kind (`math.rs:160-170,178-188`).
        (Float(a), Int(b)) => {
            if op == Op::Div && *b == 0 {
                return Err(Error("Integer division to 0.0".into()));
            }
            Ok(Float(op.float(*a, *b as f64)))
        }
        (Int(a), Float(b)) => {
            if op == Op::Div && *b == 0.0 {
                return Err(Error("Float-point division to 0.0".into()));
            }
            Ok(Float(op.float(*a as f64, *b)))
        }
        _ => {
            // String arms. An `I64` left operand with a string right operand
            // reaches `string_op_string_int(op, s_y, i_x)` — note the operands
            // swap places (`math.rs:200-202`), so the *string* leads.
            match (xs, ys, xa, ya) {
                (Some(a), Some(b), _, _) => Ok(BundValue::str(string_op(op, &a, &b))),
                (_, Some(b), Int(a), _) => Ok(BundValue::str(string_op_int(op, &b, *a))),
                (Some(a), _, _, Int(b)) => Ok(BundValue::str(string_op_int(op, &a, *b))),
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

fn add(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Add, vm)
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

pub fn register(r: &mut Registry) {
    r.register_native("+", add, eff(2, 1), WordKind::Sync);
    r.register_native("-", sub, eff(2, 1), WordKind::Sync);
    r.register_native("*", mul, eff(2, 1), WordKind::Sync);
    r.register_native("/", div, eff(2, 1), WordKind::Sync);
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

    /// Arithmetic does not average `q`; the result carries the default.
    #[test]
    fn arithmetic_leaves_q_at_the_default() {
        let i = run_src("1 2 +").expect("runs");
        assert_eq!(i.peek().map(|v| v.q()), Some(100.0));
    }

    #[test]
    fn a_shallow_stack_is_an_error_that_names_the_word() {
        let e = err_of("1 +");
        assert!(e.contains("Stack is too shallow for inline ADD()"), "{e}");
    }
}
