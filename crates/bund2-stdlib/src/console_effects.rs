//! **The spinners and the timed effects of `bund/console`** — D217's
//! seventeen words, the rest of the subsystem D214 began.
//!
//! - `console.spinner` and its five endings drive **one spinner per
//!   process**, as the reference's `lazy_static` `SPINNER` is
//!   (`reference/Bund/src/stdlib/functions/console/spinner.rs:14-146`).
//! - `spinner.text` and its colours print a coloured line, after an empty one
//!   while a spinner runs (`:148-171,250-280`).
//! - `console.typewriter`, `console.text.matrix` and `console.text.rainbow`
//!   draw frame by frame, 50 ms apart (`console/terminal.rs:27-37`,
//!   `spinner.rs:209-248`).
//!
//! **Reproduced, not called.** The reference draws the spinner with `spinoff`
//! 0.8.0 and the effects with `rusty-termcolor` 0.1.62. `spinoff` `expect`s
//! every write and the join of its thread, and the effects `unwrap` every
//! flush, so either panics when standard output has gone (D37). Each is
//! written out here frame for frame from the pinned source, with its writes
//! made fallible. The spinner thread drops a failed write, since nothing is
//! left to report it to; the words' own writes answer an `Error`.
//!
//! The end marks are `colored` 2.2.0's, the version `spinoff` resolves in the
//! reference's lock file, so their escapes are the reference's, and so is
//! the decision to write none when standard output is not a terminal.
//!
//! No golden holds the spinner, whose frames depend on timing, nor
//! `console.text.matrix` and `spinner.text.randomcolor`, which draw at
//! random.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use colored::Colorize;
use rand::Rng;
use rusty_termcolor::colors::{self, Color, RESET};

use crate::console_text::{cast_message, colored, message};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// Write and flush standard output, answering the failure.
fn emit(s: &str) -> Result<(), Error> {
    let mut out = std::io::stdout().lock();
    out.write_all(s.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|e| Error(format!("writing to standard output: {e}")))
}

/// Write and flush standard output from the spinner's thread, dropping the
/// failure: the thread has nowhere to report it.
fn emit_quietly(s: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(s.as_bytes()).and_then(|()| out.flush());
}

/// `spinoff`'s `Dots` (`spinoff-0.8.0/src/spinners.rs`, `"dots"`).
const DOTS: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const DOTS_INTERVAL: Duration = Duration::from_millis(80);

/// `delete_last_line`: back to the start of the line, blank what was drawn,
/// and back again (`spinoff-0.8.0/src/utils.rs`).
fn blanking(len: usize) -> String {
    format!("\r{}\r", " ".repeat(len))
}

/// A running spinner: `Spinner::new_with_stream`'s thread and its flag.
struct Spinner {
    spinning: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Spinner {
    /// Start drawing `msg` beside the frames, from the first, until stopped
    /// (`spinoff-0.8.0/src/lib.rs`, `new_with_stream`). No colour is given,
    /// so a frame is written plain.
    fn start(msg: String) -> Self {
        let spinning = Arc::new(AtomicBool::new(true));
        let flag = Arc::clone(&spinning);
        let thread = std::thread::Builder::new()
            .name("bund2-spinner".into())
            .spawn(move || {
                let mut last = 0;
                for frame in DOTS.iter().cycle() {
                    if !flag.load(Ordering::Relaxed) {
                        break;
                    }
                    let drawn = format!("{frame} {msg}");
                    emit_quietly(&format!("{}{drawn}", blanking(last)));
                    last = drawn.len();
                    std::thread::sleep(DOTS_INTERVAL);
                }
                emit_quietly(&blanking(last));
            })
            .ok();
        Spinner { spinning, thread }
    }

    /// `stop_spinner_thread`: signal, then wait for the thread to blank its
    /// line. A thread that cannot be joined has already gone.
    fn stop(&mut self) {
        self.spinning.store(false, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The reference's `SPINNER`: at most one, shared by every VM in the process,
/// since the terminal is.
static SPINNER: Mutex<Option<Spinner>> = Mutex::new(None);

/// The spinner slot. A poisoned lock means a word panicked while holding it,
/// which Bund2 does not do; the slot is still usable, so it is taken.
fn slot() -> std::sync::MutexGuard<'static, Option<Spinner>> {
    SPINNER.lock().unwrap_or_else(|e| e.into_inner())
}

/// `console.spinner` — start the spinner, or change its message by
/// restarting it (`spinner.rs:21-40`; `update_text`).
fn spinner_start(vm: &mut dyn Vm) -> Result<(), Error> {
    let msg = cast_message(vm, "SPINNER")?;
    let mut s = slot();
    if let Some(sp) = s.as_mut() {
        sp.stop();
    }
    *s = Some(Spinner::start(msg));
    Ok(())
}

/// How a spinner ends, and what replaces it (`spinner.rs:42-146`).
#[derive(Clone, Copy)]
enum End {
    /// `stop_with_message`: the message alone.
    Stop,
    /// `success`, `fail`: a bold mark in a colour, and no spinner after.
    Mark(&'static str, colored::Color),
    /// `warn`, `info`: a bold mark, then a new spinner saying `continue`.
    MarkContinue(&'static str, colored::Color),
}

fn spinner_end(vm: &mut dyn Vm, end: End) -> Result<(), Error> {
    let msg = cast_message(vm, "SPINNER")?;
    let mut s = slot();
    let Some(mut sp) = s.take() else {
        return Ok(());
    };
    sp.stop();
    let line = match end {
        End::Stop => msg,
        End::Mark(m, c) | End::MarkContinue(m, c) => {
            format!("{} {msg}", m.color(c).bold())
        }
    };
    let wrote = emit(&format!("{line}\n"));
    if let End::MarkContinue(..) = end {
        *s = Some(Spinner::start("continue".into()));
    }
    wrote
}

/// `spinner.text` and its colours: the message as `conv(STRING)` gives it,
/// coloured, after an empty line if a spinner runs (`spinner.rs:148-171`).
fn spinner_text(vm: &mut dyn Vm, color: &Color) -> Result<(), Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error("CONSOLE::SPINNER: NO DATA #1".into()))?;
    if let Some(why) = v.conv_refusal() {
        return Err(Error(format!(
            "CONSOLE::SPINNER error converting message: {why}"
        )));
    }
    let spinning = slot().is_some();
    let lead = if spinning { "\n" } else { "" };
    emit(&format!("{lead}{}\n", colored(&v.display(), color)))
}

fn spinner_text_random(vm: &mut dyn Vm) -> Result<(), Error> {
    spinner_text(vm, &colors::random_pleasing_color())
}

/// The effects' pause (`EffectSettings::default`, `delay: 50`).
const EFFECT_DELAY: Duration = Duration::from_millis(50);
/// The effects' passes (`EffectSettings::default`, `iterations: 3`).
const EFFECT_PASSES: usize = 3;

/// `console.typewriter` — one character at a time, uncoloured, with no
/// newline after (`terminal.rs:27-37`; `effects.rs`, `typewriter` with no
/// colour).
fn typewriter(vm: &mut dyn Vm) -> Result<(), Error> {
    let msg = cast_message(vm, "TYPEWRITER")?;
    for c in msg.chars() {
        emit(&c.to_string())?;
        std::thread::sleep(EFFECT_DELAY);
    }
    Ok(())
}

/// The symbols `matrix_effect` draws undecoded characters from.
const MATRIX_SYMBOLS: &str = "!@#$%^&*()_+-=[]{}|;:,.<>?";

/// `console.text.matrix` — three passes decoding the message left to right
/// in green over random symbols, a reset and a newline, then the word's own
/// reset with no newline (`spinner.rs:209-228`; `effects.rs`,
/// `matrix_effect`).
fn matrix(vm: &mut dyn Vm) -> Result<(), Error> {
    let msg = message(vm)?;
    let chars: Vec<char> = msg.chars().collect();
    let symbols: Vec<char> = MATRIX_SYMBOLS.chars().collect();
    let mut rng = rand::thread_rng();
    for _ in 0..EFFECT_PASSES {
        for i in 0..chars.len() {
            let line: String = chars
                .iter()
                .enumerate()
                .map(|(j, &c)| {
                    if j <= i {
                        c
                    } else {
                        symbols[rng.gen_range(0..symbols.len())]
                    }
                })
                .collect();
            emit(&format!("\r{}{line}", colors::GREEN))?;
            std::thread::sleep(EFFECT_DELAY);
        }
    }
    emit(&format!("{RESET}\n{RESET}"))
}

/// `rainbow_text`'s seven colours (`effects.rs`, `rainbow_text`).
const RAINBOW: [Color; 7] = [
    Color::new(255, 0, 0),
    Color::new(255, 127, 0),
    Color::new(255, 255, 0),
    Color::new(0, 255, 0),
    Color::new(0, 0, 255),
    Color::new(75, 0, 130),
    Color::new(143, 0, 255),
];

/// One frame of the rainbow: character `j` in colour `(i + j) mod 7`, then a
/// reset, from the start of the line.
fn rainbow_frame(text: &str, i: usize) -> String {
    let mut s = String::from("\r");
    for (j, c) in text.chars().enumerate() {
        s.push_str(&format!("{}{c}", RAINBOW[(i + j) % RAINBOW.len()]));
    }
    s.push_str(RESET);
    s
}

/// `console.text.rainbow` — three passes of seven frames, a newline, then the
/// word's own reset with no newline (`spinner.rs:230-248`; `effects.rs`,
/// `rainbow_text`).
fn rainbow(vm: &mut dyn Vm) -> Result<(), Error> {
    let msg = message(vm)?;
    for _ in 0..EFFECT_PASSES {
        for i in 0..RAINBOW.len() {
            emit(&rainbow_frame(&msg, i))?;
            std::thread::sleep(EFFECT_DELAY);
        }
    }
    emit(&format!("\n{RESET}"))
}

pub fn register(r: &mut Registry) {
    use colored::Color as C;
    r.register_native("console.spinner", spinner_start, eff(1, 0), WordKind::Sync);
    // The marks and colours `spinoff` writes (`lib.rs`, `success`, `fail`,
    // `warn`, `info`).
    macro_rules! end {
        ($name:literal, $end:expr) => {
            r.register_native($name, |vm| spinner_end(vm, $end), eff(1, 0), WordKind::Sync);
        };
    }
    end!("console.spinner.stop", End::Stop);
    end!("console.spinner.success", End::Mark("✓", C::Green));
    end!("console.spinner.fail", End::Mark("✗", C::Red));
    end!("console.spinner.warning", End::MarkContinue("⚠", C::Yellow));
    end!("console.spinner.info", End::MarkContinue("ℹ", C::Blue));
    macro_rules! text {
        ($name:literal, $color:path) => {
            r.register_native(
                $name,
                |vm| spinner_text(vm, &$color),
                eff(1, 0),
                WordKind::Sync,
            );
        };
    }
    text!("spinner.text", colors::WHITE);
    text!("spinner.text.red", colors::RED);
    text!("spinner.text.green", colors::GREEN);
    text!("spinner.text.blue", colors::BLUE);
    text!("spinner.text.magenta", colors::MAGENTA);
    text!("spinner.text.cyan", colors::CYAN);
    text!("spinner.text.yellow", colors::YELLOW);
    r.register_native(
        "spinner.text.randomcolor",
        spinner_text_random,
        eff(1, 0),
        WordKind::Sync,
    );
    r.register_native("console.typewriter", typewriter, eff(1, 0), WordKind::Sync);
    r.register_native("console.text.matrix", matrix, eff(1, 0), WordKind::Sync);
    r.register_native("console.text.rainbow", rainbow, eff(1, 0), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A rainbow frame** starts at the line's start, colours character `j`
    /// with colour `(i + j) mod 7`, and resets — `rainbow_text`'s frame.
    #[test]
    fn a_rainbow_frame_shifts_its_colours_by_one() {
        assert_eq!(
            rainbow_frame("ab", 0),
            "\r\x1B[38;2;255;0;0ma\x1B[38;2;255;127;0mb\x1B[0m"
        );
        assert_eq!(
            rainbow_frame("ab", 6),
            "\r\x1B[38;2;143;0;255ma\x1B[38;2;255;0;0mb\x1B[0m"
        );
        assert_eq!(rainbow_frame("", 3), "\r\x1B[0m");
    }

    /// **`delete_last_line`**: a carriage return, a blank as long as what was
    /// drawn in bytes, and another return.
    #[test]
    fn blanking_covers_what_was_drawn() {
        assert_eq!(blanking(0), "\r\r");
        assert_eq!(blanking(3), "\r   \r");
    }

    /// **A spinner starts, restarts and stops** without a panic, and the slot
    /// is empty after `success`; an ending with no spinner is a no-op that
    /// still consumes its message. The one test that touches the process's
    /// spinner, so no other test races it.
    #[test]
    fn the_spinner_starts_restarts_and_ends() {
        let mut i = bund2_interp::Interp::new();
        crate::register_all(&mut i.registry);
        let mut run = |src: &str| {
            let ir = bund2_syntax::compile(src).map_err(|e| format!("{e:?}"))?;
            i.eval(&ir).map_err(|e| e.0)
        };
        run("\"one\" console.spinner \"two\" console.spinner").expect("starts");
        assert!(slot().is_some());
        run("\"done\" console.spinner.success").expect("ends");
        assert!(slot().is_none());
        run("\"nothing\" console.spinner.fail").expect("no spinner, no-op");
        assert!(slot().is_none());
        run("\"warn\" console.spinner \"careful\" console.spinner.warning").expect("warns");
        assert!(slot().is_some(), "warning starts a `continue` spinner");
        run("\"bye\" console.spinner.stop").expect("stops");
        assert!(slot().is_none());
        let e = run("1 console.spinner").expect_err("an integer");
        assert!(
            e.ends_with("CONSOLE::SPINNER error casting message: This Dynamic type is not string"),
            "{e}"
        );
    }
}
