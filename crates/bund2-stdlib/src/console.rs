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

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect { consumes, produces }
}

/// How a value prints, as distinct from how it renders.
///
/// `println` prints the *contents* — `Hello World!`, not
/// `Value { id: … data: String("Hello World!") … }`. The `Debug` rendering is
/// `debug.display_stack`'s business, and conflating the two is why the two
/// functions are separate here.
fn display(v: &BundValue) -> String {
    if let Some(s) = v.as_str() {
        return s;
    }
    if let Some(i) = v.as_int() {
        return i.to_string();
    }
    match v {
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
/// A non-empty stack renders as a table sized to its widest row. That is not
/// implemented here: getting it byte-exact needs the reference's table
/// library and its padding rules, and a near-miss would fail a golden while
/// looking right. Reporting the gap beats guessing at it.
fn draw_box(rows: &[String]) -> String {
    if rows.is_empty() {
        return "╭╮\n╰╯".to_string();
    }
    let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) + 2;
    let mut out = String::new();
    out.push('╭');
    out.push_str(&"─".repeat(width));
    out.push_str("╮\n");
    for r in rows {
        let pad = width - 2 - r.chars().count();
        out.push_str(&format!("│ {r}{} │\n", " ".repeat(pad)));
    }
    out.push('╰');
    out.push_str(&"─".repeat(width));
    out.push('╯');
    out
}

fn display_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut rows = Vec::new();
    let mut held = Vec::new();
    while let Some(v) = vm.pull() {
        rows.push(v.render(false));
        held.push(v);
    }
    // Displaying is not consuming: put them back in the order they were in.
    for v in held.into_iter().rev() {
        vm.push(v);
    }
    println!("{}", draw_box(&rows));
    Ok(())
}

fn display_workbench(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut rows = Vec::new();
    let mut held = Vec::new();
    while let Some(v) = vm.pull_workbench() {
        rows.push(v.render(false));
        held.push(v);
    }
    for v in held.into_iter().rev() {
        vm.push_workbench(v);
    }
    println!("{}", draw_box(&rows));
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
        assert_eq!(draw_box(&[]), "╭╮\n╰╯");
    }

    /// `println` prints contents, not the `Debug` rendering. Conflating the
    /// two would put `Value { id: … }` where `Hello World!` belongs.
    #[test]
    fn println_prints_contents_not_the_rendering() {
        assert_eq!(display(&BundValue::str("Hello World!")), "Hello World!");
        assert_eq!(display(&BundValue::Int(42)), "42");
    }
}
