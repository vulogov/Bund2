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
    StackEffect::fixed(consumes, produces)
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
    v.display()
}

/// The reference's refusal, for a value that has no string form.
///
/// `print` and `println` go through `conv(STRING)`
/// (`reference/rust_multistackvm/src/stdlib/print.rs:16`) and report what it
/// says, so a PAIR — which no conversion arm admits — stops the program rather
/// than printing something invented.
fn as_text(v: &BundValue, prefix: &str) -> Result<String, Error> {
    if let Some(why) = v.conv_refusal() {
        return Err(Error(format!("{prefix} returns: {why}")));
    }
    Ok(display(v))
}

/// `print` / `println` and their `.` siblings
/// (`reference/rust_multistackvm/src/stdlib/print.rs:6-32`).
///
/// **The `.` form guards the wrong stack — F77.** `stdlib_print_inline_base`
/// tests `current_stack_len() < 1` *before* the `StackOps` match (`:7-9`), so
/// the workbench form checks the depth of a stack it is not going to read and
/// then pulls from the workbench. A program with a full workbench and an empty
/// current stack gets "Stack is too shallow"; one with a full stack and an
/// empty workbench passes the guard and gets "PRINT. returns: NO DATA".
/// Preserved, because both messages are observable.
fn print_base(vm: &mut dyn Vm, nl: bool, side: crate::wb::Side) -> Result<(), Error> {
    let prefix = &format!("{}{}", if nl { "PRINTLN" } else { "PRINT" }, side.dot());
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let Some(v) = side.pull(vm) else {
        return Err(Error(format!("{prefix} returns: NO DATA")));
    };
    let text = as_text(&v, prefix)?;
    if nl {
        bund2_api::outln!("{text}")?;
    } else {
        bund2_api::out!("{text}")?;
    }
    Ok(())
}

fn nl(_vm: &mut dyn Vm) -> Result<(), Error> {
    bund2_api::outln!()?;
    Ok(())
}

fn space(_vm: &mut dyn Vm) -> Result<(), Error> {
    bund2_api::out!(" ")?;
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
    bund2_api::outln!("{}", draw_box_rows(&rows))?;
    Ok(())
}

fn display_workbench(vm: &mut dyn Vm) -> Result<(), Error> {
    let rows: Vec<String> = vm
        .snapshot_workbench()
        .iter()
        .map(|v| v.render(false))
        .collect();
    bund2_api::outln!("{}", draw_box_rows(&rows))?;
    Ok(())
}

pub fn register(r: &mut Registry) {
    use crate::wb::Side;
    r.register_native("println", |vm| print_base(vm, true, Side::Stack), eff(1, 0), WordKind::Sync);
    r.register_native("print", |vm| print_base(vm, false, Side::Stack), eff(1, 0), WordKind::Sync);
    // F77: the guard still reads the current stack, so the `.` forms do
    // require one value there — declared, because `check` would otherwise
    // miss an underflow the reference really reports. But they take nothing
    // from it: the value printed comes off the workbench. `consumes` is a
    // floor, so a floor of one and a net of zero is `1 -> 1`. It said `1 -> 0`,
    // a net of minus one on a stack the word never touches (F92).
    r.register_native("println.", |vm| print_base(vm, true, Side::Bench), eff(1, 1), WordKind::Sync);
    r.register_native("print.", |vm| print_base(vm, false, Side::Bench), eff(1, 1), WordKind::Sync);
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
    // **`eff(1, 1)`, not `eff(1, 0)`.** It peeks: the floor is one value and
    // the net is zero, which is the shape F92 corrected for `println.`.
    r.register_native("debug.dump", debug_dump, eff(1, 1), WordKind::Sync);
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
        assert_eq!(display(&BundValue::int(42)), "42");
    }
}

/// **`debug.dump` — RFC-0008 criterion 3.**
///
/// A hexdump of a value's bytes, to stdout, through the reference's own
/// `hexdump` crate. It **peeks**, so the value stays where it was: the dump is
/// an observation and not a consumption.
///
/// **Five cases, and which one applies is decided by the tag.** The four
/// scalars dump their machine representation — eight bytes for an INT, eight
/// for a FLOAT, one for a BOOL, and a STRING's own bytes — and everything else
/// dumps the wire encoding. `bytes_of` in the reference is native-endian, which
/// `to_ne_bytes` is.
///
/// **Only the four scalar cases can be goldened.** The fallback encodes the
/// value's identity and stamp into the bytes, so the oracle's own two runs of
/// `[ 1 2 ] debug.dump` differ from each other — F14's class reached through a
/// hexdump rather than through a `Debug` line. Criterion 3 names only the four,
/// and this is why.
fn debug_dump(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = vm.peek().ok_or_else(|| Error("DUMP: NO DATA #1".into()))?;
    for line in hexdump::hexdump_iter(&dump_bytes(&v)?) {
        bund2_api::outln!("{line}")?;
    }
    Ok(())
}

/// Which bytes a value dumps. Separate from the printing so the widths can be
/// asserted without a golden.
fn dump_bytes(v: &BundValue) -> Result<Vec<u8>, Error> {
    // **The tag decides, not the payload.** A PTR and a CALL both carry a
    // string and neither is a STRING, so `dt` is what the reference switches
    // on and what this switches on.
    let bytes: Vec<u8> = match v.dt() {
        bund2_value::INTEGER => v
            .as_int()
            .ok_or_else(|| {
                Error::internal("a value tagged INTEGER whose payload is not an integer")
            })?
            .to_ne_bytes()
            .to_vec(),
        bund2_value::FLOAT => match v.unboxed() {
            BundValue::Float(f, _) => f.to_ne_bytes().to_vec(),
            _ => {
                return Err(Error::internal(
                    "a value tagged FLOAT whose payload is not a float",
                ))
            }
        },
        bund2_value::BOOL => vec![u8::from(match v.unboxed() {
            BundValue::Bool(b, _) => *b,
            _ => {
                return Err(Error::internal(
                    "a value tagged BOOL whose payload is not a bool",
                ))
            }
        })],
        bund2_value::STRING => v
            .as_str()
            .ok_or_else(|| Error::internal("a value tagged STRING whose payload is not a string"))?
            .into_bytes(),
        // **The reference's five casting failures are unreachable in both
        // implementations**, because each arm has already read the tag that
        // guarantees the cast. Writing their text would mean inventing the
        // `{err}` half of a message no run can produce, so the four arms above
        // name the broken invariant instead — D37's third way out, for a case
        // that is a Bund2 defect if it ever happens rather than a program
        // error. Criterion 3 asked for all seven texts to match; five of them
        // are vacuous and this is where that was found.
        _ => bund2_value::wire::to_binary(v)
            .map_err(|e| Error(format!("DUMP: error converting to binary: {e}")))?,
    };
    Ok(bytes)
}

#[cfg(test)]
mod dump_tests {
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

    /// **It peeks.** The dump is an observation, so the value is still on the
    /// stack afterwards — which is why the effect is `1 -> 1` and not `1 -> 0`.
    #[test]
    fn debug_dump_leaves_its_operand_where_it_found_it() {
        let out = run("42 debug.dump").expect("runs");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].as_int(), Some(42));
    }

    /// The reference's own text, for the one error a program can actually
    /// reach. The other six are unreachable behind the tag each arm reads.
    #[test]
    fn an_empty_stack_is_the_references_own_message() {
        let e = run("debug.dump").expect_err("nothing to dump");
        assert!(e.ends_with("returned error: DUMP: NO DATA #1"), "{e}");
    }

    /// **The four scalar widths**, read off the rendered summary line — eight
    /// bytes, eight, one, and the string's own length. The summary is the last
    /// line of a dump and carries the total, so it is the cheapest assertion
    /// about width that does not duplicate the golden.
    #[test]
    fn the_four_scalar_tags_dump_the_widths_the_criterion_names() {
        for (src, bytes) in [
            ("7 debug.dump", 8usize),
            ("2.5 debug.dump", 8),
            ("true debug.dump", 1),
            ("\"hello\" debug.dump", 5),
            ("\"\" debug.dump", 0),
        ] {
            let out = run(src).expect(src);
            let got = super::dump_bytes(&out[0]).expect(src);
            assert_eq!(got.len(), bytes, "{src}");
        }
    }
}
