//! Console output, and the debug display the goldens capture.
//!
//! **D15 scopes this**: only basic console output is in scope — `print`,
//! `println`, `nl`, `space` and their workbench forms
//! (`reference/rust_multistackvm/src/stdlib/print.rs:63-68`). No spinners, no
//! animations, no colour.
//!
//! `debug.display_stack` is here because the golden capture epilogue calls it,
//! so nothing can be conformed without it.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{ContentArrangement, Table};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect { consumes, produces }
}

/// How a value prints, as distinct from how it renders.
///
/// `println` prints the *contents* — `Hello World!`, not
/// `Value { id: … data: String("Hello World!") … }`. The `Debug` rendering is
/// `debug.display_stack`'s business, and conflating the two is why the two
/// functions are separate here.
/// `push` boxes scalars, so this looks through the box before matching. A
/// `Bool` that reached a stack arrives as `Heap { payload: Scalar(Bool) }`,
/// and matching the outer value alone sent it to the `Debug` rendering — the
/// oracle prints `false` where that printed `Value { id: … }`.
fn display(v: &BundValue) -> String {
    if let Some(s) = v.as_str() {
        return s;
    }
    if let Some(i) = v.as_int() {
        return i.to_string();
    }
    match v.unboxed() {
        BundValue::Float(f) => format!("{f}"),
        BundValue::Bool(b) => b.to_string(),
        BundValue::Nodata | BundValue::None => String::new(),
        other => other.render(false),
    }
}

fn println_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is too shallow for inline PRINTLN".into()));
    };
    println!("{}", display(&v));
    Ok(())
}

fn print_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is too shallow for inline PRINT".into()));
    };
    print!("{}", display(&v));
    Ok(())
}

fn nl(_vm: &mut dyn Vm) -> Result<(), Error> {
    println!();
    Ok(())
}

fn space(_vm: &mut dyn Vm) -> Result<(), Error> {
    print!(" ");
    Ok(())
}

/// The box the reference draws around a stack dump.
///
/// An **empty** stack renders as two lines, `╭╮` then `╰╯` — a box of zero
/// width. That is what the capture epilogue leaves behind for every program
/// that ends with an empty stack and an empty workbench, and it is the whole
/// output of `helloworld.golden` beyond the greeting.
///
/// A **non-empty** stack renders as a one-column table, one row per value,
/// bottom of the stack first.
///
/// This draws it with `comfy_table` under the same three settings the
/// reference applies —
/// `reference/Bund/src/stdlib/functions/debug_fun/debug_display_stack.rs:14-27`
/// loads `UTF8_FULL`, applies `UTF8_ROUND_CORNERS`, and sets
/// `ContentArrangement::Dynamic`. Using the same library is not laziness, it
/// is the only way to be byte-exact: the goldens capture the box down to the
/// dashed row separator `├╌╌┤` that the rounded-corners modifier substitutes
/// for `UTF8_FULL`'s solid one, and a hand-rolled near-miss fails a golden
/// while looking right.
///
/// `Dynamic` sizes to the terminal, so it wraps under one and does not when
/// output is a pipe. The goldens were captured through a pipe — the widest is
/// a single unwrapped 190-column row — and `conform` compares captured output,
/// so both sides see the same absence of a terminal.
pub(crate) fn draw_box_rows(rows: &[String]) -> String {
    if rows.is_empty() {
        // Zero columns, so no preset applies and comfy_table prints nothing at
        // all. The reference still emits a degenerate two-line box, which is
        // most of what the capture epilogue leaves behind.
        return "╭╮\n╰╯".to_string();
    }
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    for r in rows {
        table.add_row(vec![r.clone()]);
    }
    table.to_string()
}

/// Displaying is not consuming, so this reads the stack rather than draining
/// and refilling it. Draining reversed the row order and re-ran `push`, which
/// rewrites the stack tag on every value it touches.
fn display_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    let rows: Vec<String> = vm.snapshot().iter().map(|v| v.render(false)).collect();
    println!("{}", draw_box_rows(&rows));
    Ok(())
}

fn display_workbench(vm: &mut dyn Vm) -> Result<(), Error> {
    let rows: Vec<String> = vm
        .snapshot_workbench()
        .iter()
        .map(|v| v.render(false))
        .collect();
    println!("{}", draw_box_rows(&rows));
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.register_native("println", println_word, eff(1, 0), WordKind::Sync);
    r.register_native("print", print_word, eff(1, 0), WordKind::Sync);
    r.register_native("nl", nl, eff(0, 0), WordKind::Sync);
    r.register_native("space", space, eff(0, 0), WordKind::Sync);
    r.register_native(
        "debug.display_stack",
        display_stack,
        eff(0, 0),
        WordKind::Sync,
    );
    r.register_native(
        "debug.display_workbench",
        display_workbench,
        eff(0, 0),
        WordKind::Sync,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two lines every golden ends with, twice over, when a program
    /// leaves nothing behind.
    #[test]
    fn an_empty_stack_is_a_zero_width_box() {
        assert_eq!(draw_box_rows(&[]), "╭╮\n╰╯");
    }

    /// The box, byte-for-byte, against bytes lifted out of a golden.
    ///
    /// These are `tests/golden/tests/string_concatenation.golden:8-12` with
    /// the two long renderings replaced by short stand-ins — what is under
    /// test is the frame, not what goes in the cells. The dashed `├╌╌┤` row
    /// separator is the tell that `UTF8_ROUND_CORNERS` was applied: plain
    /// `UTF8_FULL` draws it solid.
    #[test]
    fn a_non_empty_stack_is_a_rounded_table() {
        assert_eq!(
            draw_box_rows(&["alpha".to_string(), "bb".to_string()]),
            concat!(
                "╭───────╮\n",
                "│ alpha │\n",
                "├╌╌╌╌╌╌╌┤\n",
                "│ bb    │\n",
                "╰───────╯",
            )
        );
    }

    /// One row per value, in stack order, and the box is as wide as the
    /// widest. A single row gets no separator at all.
    #[test]
    fn one_row_has_no_separator() {
        assert_eq!(
            draw_box_rows(&["x".to_string()]),
            "╭───╮\n│ x │\n╰───╯"
        );
    }

    /// `println` prints contents, not the `Debug` rendering. Conflating the
    /// two would put `Value { id: … }` where `Hello World!` belongs.
    #[test]
    fn println_prints_contents_not_the_rendering() {
        assert_eq!(display(&BundValue::str("Hello World!")), "Hello World!");
        assert_eq!(display(&BundValue::Int(42)), "42");
    }
}
