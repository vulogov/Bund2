//! **The text and colour half of `bund/console`** — D214's fourteen words.
//!
//! D15 deferred the whole subsystem; D214 brings back the words that print a
//! line once and return: `console.text` in white, six colours and a random
//! one (`reference/Bund/src/stdlib/functions/console/spinner.rs:173-188,282-312`),
//! the three two-colour fades `console.text.ok`, `.warning` and `.error`
//! (`:190-207,314-324`), and `console.box`, `console.clear` and
//! `console.title` (`console/terminal.rs:10-50`). The spinners and the three
//! effects that sleep between characters stay out under D15.
//!
//! **The escapes are `rusty-termcolor` 0.1.62's**, the reference's pinned
//! version: its colour constants, `fade_color`, `box_text` and
//! `random_pleasing_color` are called as they are. Its *printing* functions
//! are not, because they write with `println!`, which panics when standard
//! output has gone (D37); each one's format is reproduced here and written
//! through `outln!` instead, citing the function it stands for.
//!
//! A golden strips ANSI, so it pins the text these print and not the colour;
//! the unit tests below pin the escape sequences.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;
use rusty_termcolor::colors::{self, Color, RESET};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// The message, as `stdlib_console_text_base` reads it (`spinner.rs:173-183`):
/// `conv(STRING)` and then `cast_string`, so a number prints as its text and
/// a value `conv` refuses stops the word with `conv`'s own sentence.
pub(crate) fn message(vm: &mut dyn Vm) -> Result<String, Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error("CONSOLE: NO DATA #1".into()))?;
    if let Some(why) = v.conv_refusal() {
        return Err(Error(format!("CONSOLE error converting message: {why}")));
    }
    Ok(v.display())
}

/// The coloured line `rusty_termcolor::println_colored` prints:
/// `{color}{text}{RESET}` and a newline (`formatting.rs`, `println_colored`).
pub(crate) fn colored(text: &str, color: &Color) -> String {
    format!("{color}{text}{RESET}")
}

/// `console.text` and its colours (`spinner.rs:173-188`).
fn text(vm: &mut dyn Vm, color: &Color) -> Result<(), Error> {
    let msg = message(vm)?;
    bund2_api::outln!("{}", colored(&msg, color))
}

/// `console.text.randomcolor` — the crate's own random colour
/// (`spinner.rs:310-312`), so the colour varies from run to run and the text
/// does not.
fn text_random(vm: &mut dyn Vm) -> Result<(), Error> {
    text(vm, &colors::random_pleasing_color())
}

/// The fade `stdlib_console_text_fade_base` prints (`spinner.rs:190-207`):
/// `print_fade` over `fade_color(from, to, msg.len())`, then `RESET` and a
/// newline. `print_fade` colours character `i` with
/// `colors[(i * colors.len()) / chars.len()]` and ends with `RESET`
/// (`formatting.rs`, `print_fade`). The gradient has as many steps as the
/// message has **bytes**, and is indexed by **characters**, as the reference
/// computes it; a one-byte message's single step is `t = 0/0`, which the
/// crate's `as u8` turns into black.
fn faded(text: &str, from: &Color, to: &Color) -> String {
    let steps = colors::fade_color(from, to, text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut s = String::new();
    for (i, c) in chars.iter().enumerate() {
        let at = (i * steps.len()) / chars.len();
        if let Some(color) = steps.get(at) {
            s.push_str(&format!("{color}{c}"));
        }
    }
    s.push_str(RESET);
    s.push_str(RESET);
    s
}

fn fade(vm: &mut dyn Vm, from: &Color, to: &Color) -> Result<(), Error> {
    let msg = message(vm)?;
    bund2_api::outln!("{}", faded(&msg, from, to))
}

/// The terminal words' operand (`terminal.rs:16-22`): `cast_string` with no
/// `conv` first, so only a string-carrying value passes.
pub(crate) fn cast_message(vm: &mut dyn Vm, word: &str) -> Result<String, Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("CONSOLE::{word}: NO DATA #1")))?;
    v.as_str().ok_or_else(|| {
        Error(format!(
            "CONSOLE::{word} error casting message: This Dynamic type is not string"
        ))
    })
}

/// `console.box` — push the message boxed by `box_text` (`terminal.rs:39-50`).
fn boxed(vm: &mut dyn Vm) -> Result<(), Error> {
    let msg = cast_message(vm, "BOX")?;
    vm.push(BundValue::str(rusty_termcolor::formatting::box_text(&msg)));
    Ok(())
}

/// `console.title` — the title escape `set_title` prints, with no newline
/// (`terminal.rs:15-25`; `system.rs`, `set_title`).
fn title(vm: &mut dyn Vm) -> Result<(), Error> {
    let msg = cast_message(vm, "TITLE")?;
    bund2_api::out!("\x1B]0;{msg}\x07")
}

/// `console.clear` — the escape `clear_screen` prints: clear the screen and
/// home the cursor (`terminal.rs:10-13`; `system.rs`, `clear_screen`).
fn clear(_vm: &mut dyn Vm) -> Result<(), Error> {
    bund2_api::out!("\x1B[2J\x1B[1;1H")
}

pub fn register(r: &mut Registry) {
    macro_rules! colour {
        ($name:literal, $color:path) => {
            r.register_native($name, |vm| text(vm, &$color), eff(1, 0), WordKind::Sync);
        };
    }
    colour!("console.text", colors::WHITE);
    colour!("console.text.red", colors::RED);
    colour!("console.text.green", colors::GREEN);
    colour!("console.text.blue", colors::BLUE);
    colour!("console.text.magenta", colors::MAGENTA);
    colour!("console.text.cyan", colors::CYAN);
    colour!("console.text.yellow", colors::YELLOW);
    r.register_native(
        "console.text.randomcolor",
        text_random,
        eff(1, 0),
        WordKind::Sync,
    );
    // The fades' colour pairs (`spinner.rs:314-324`).
    r.register_native(
        "console.text.ok",
        |vm| fade(vm, &colors::WHITE, &colors::GREEN),
        eff(1, 0),
        WordKind::Sync,
    );
    r.register_native(
        "console.text.warning",
        |vm| fade(vm, &colors::GREEN, &colors::YELLOW),
        eff(1, 0),
        WordKind::Sync,
    );
    r.register_native(
        "console.text.error",
        |vm| fade(vm, &colors::YELLOW, &colors::RED),
        eff(1, 0),
        WordKind::Sync,
    );
    r.register_native("console.box", boxed, eff(1, 1), WordKind::Sync);
    r.register_native("console.title", title, eff(1, 0), WordKind::Sync);
    r.register_native("console.clear", clear, eff(0, 0), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The escapes are the crate's**: a 24-bit foreground, the text, and
    /// the reset.
    #[test]
    fn a_colour_is_a_24_bit_foreground_then_a_reset() {
        assert_eq!(colored("hi", &colors::RED), "\x1B[38;2;255;0;0mhi\x1B[0m");
        assert_eq!(colored("", &colors::WHITE), "\x1B[38;2;255;255;255m\x1B[0m");
    }

    /// **A fade steps per byte and indexes per character**, and ends with
    /// two resets, `print_fade`'s and the word's own. A one-byte message's
    /// one step is black (`t = 0/0`).
    #[test]
    fn a_fade_runs_from_one_colour_to_the_other() {
        assert_eq!(
            faded("ab", &colors::WHITE, &colors::GREEN),
            "\x1B[38;2;255;255;255ma\x1B[38;2;0;255;0mb\x1B[0m\x1B[0m"
        );
        assert_eq!(
            faded("a", &colors::WHITE, &colors::GREEN),
            "\x1B[38;2;0;0;0ma\x1B[0m\x1B[0m"
        );
        assert_eq!(faded("", &colors::WHITE, &colors::GREEN), "\x1B[0m\x1B[0m");
        // Two characters, three bytes: three steps, and the second
        // character takes step `(1 * 3) / 2`, the middle one.
        let s = faded("éa", &colors::YELLOW, &colors::RED);
        assert!(s.starts_with("\x1B[38;2;255;255;0mé"), "{s:?}");
    }

    fn run(src: &str) -> Result<Vec<BundValue>, String> {
        use bund2_api::Vm as _;
        let mut i = bund2_interp::Interp::new();
        crate::register_all(&mut i.registry);
        let ir = bund2_syntax::compile(src).map_err(|e| format!("{e:?}"))?;
        i.eval(&ir).map_err(|e| e.0)?;
        Ok(i.snapshot())
    }

    /// **`console.box` pushes the box**, and the terminal words cast without
    /// converting, so a number is refused where `console.text` prints it.
    #[test]
    fn the_box_is_pushed_and_the_terminal_words_cast_strictly() {
        let out = run("\"ab\" console.box").expect("runs");
        assert_eq!(
            out.last().and_then(|v| v.as_str()).as_deref(),
            Some("╔════╗\n║ ab ║\n╚════╝")
        );
        let e = run("1 console.box").expect_err("an integer");
        assert!(
            e.ends_with("CONSOLE::BOX error casting message: This Dynamic type is not string"),
            "{e}"
        );
        let e = run("console.title").expect_err("nothing");
        assert!(e.ends_with("CONSOLE::TITLE: NO DATA #1"), "{e}");
        let e = run("console.text").expect_err("nothing");
        assert!(e.ends_with("CONSOLE: NO DATA #1"), "{e}");
        run("42 console.text.red").expect("a number is converted");
    }
}
