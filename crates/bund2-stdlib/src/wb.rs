//! The workbench half of a word — what a `.` suffix actually means.
//!
//! # There is no single `.` contract, and this module refuses to invent one
//!
//! D24 states the `.` contract as "primary operand from the workbench,
//! secondary from the main stack, result to the workbench". That is *one* of
//! at least three shapes the reference uses, and a word written from the
//! description rather than from its own source lands on the wrong one:
//!
//! | shape | operand 1 | operand 2 | result | example |
//! |---|---|---|---|---|
//! | **D24's** | workbench | stack | workbench | `string.distance.` (`.../string/distance.rs:38-39,90-91`) |
//! | **mixed-in, stack-out** | workbench | stack | **stack** | `string.prefix.` (`.../string/prefix_suffix.rs:32-39,52`) |
//! | **all-workbench** | workbench | **workbench** | workbench | `head.` (`reference/rust_multistackvm/src/stdlib/values/value_carcdr.rs:64-67,80-86`) |
//!
//! F73 records the first two; the third turned up when the suffix variants of
//! `car`/`cdr`/`head`/`tail`/`at` were read. `cargo xtask coverage` calls these
//! "one mechanical paired test per base", and that is exactly the assumption
//! that produces a wrong word.
//!
//! So [`Side`] carries only "which side is this call", and every family names
//! its own operand sources and its own destination at the registration site.
//! Nothing here defaults.

use bund2_api::{Error, Vm};
use bund2_value::BundValue;

/// Which of a word's two forms is running — the reference's `StackOps`
/// (`reference/rust_multistackvm/src/stdlib/values/value_carcdr.rs:14-23`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Stack,
    Bench,
}

impl Side {
    /// Pull from whichever side this call is on.
    pub(crate) fn pull(self, vm: &mut dyn Vm) -> Option<BundValue> {
        match self {
            Side::Stack => vm.pull(),
            Side::Bench => vm.pull_workbench(),
        }
    }

    /// Push to whichever side this call is on.
    pub(crate) fn push(self, vm: &mut dyn Vm, v: BundValue) {
        match self {
            Side::Stack => vm.push(v),
            Side::Bench => vm.push_workbench(v),
        }
    }

    /// How deep this side is.
    pub(crate) fn depth(self, vm: &dyn Vm) -> usize {
        match self {
            Side::Stack => vm.depth(),
            Side::Bench => vm.workbench_depth(),
        }
    }

    /// The `.` that goes on the end of a word's error prefix, and nothing for
    /// the plain form. The reference spells the prefix out per registration
    /// (`value_carcdr.rs:150-171`); this composes it instead, which is the same
    /// string with one place to be wrong rather than thirty.
    pub(crate) fn dot(self) -> &'static str {
        match self {
            Side::Stack => "",
            Side::Bench => ".",
        }
    }
}

/// Guard one side's depth, with the reference's message.
///
/// **The message says "Stack" even for the workbench form.** That is not a slip
/// here: `stdlib_carcdr_base` writes `"Stack is too shallow for inline {}()"`
/// in *both* arms (`value_carcdr.rs:16,21`), and a golden captures the text.
pub(crate) fn shallow(vm: &dyn Vm, side: Side, n: usize, prefix: &str) -> Result<(), Error> {
    if side.depth(vm) < n {
        return Err(Error(format!("Stack is too shallow for inline {prefix}()")));
    }
    Ok(())
}

/// Pull, or report the absence the reference reports.
pub(crate) fn operand(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<BundValue, Error> {
    side.pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returned: NO DATA has been obtained")))
}

/// The workbench form of a one-operand word that the reference gives none —
/// D18, as D24 bounds it and D111 applies it.
///
/// **One operand from the workbench, and everything the word answers back to
/// the workbench.** This is D24's shape with no second operand to place, which
/// is why it can be written once: the three shapes this module's opening
/// table lists differ only in where a *second* operand comes from.
///
/// The base word runs unchanged, on the stack, over the operand moved there
/// for it. So it answers, refuses and words its refusals exactly as the plain
/// form does — a `math.sqrt.` that could disagree with `math.sqrt` would be a
/// second implementation to keep in step.
///
/// - **A word that leaves its operand in place leaves it on the workbench.**
///   `len` answers beside its operand; so does `len.`.
/// - **A refusal leaves the stack as it was.** Whatever the base left above
///   where it started goes back to the workbench, so a refused operand is on
///   the side it came from if it survives at all.
/// - **The stack beneath is not touched**, which is what `eff(0, 0)` says.
///
/// `name` is the dotted name, for the one message this adds.
pub(crate) fn bench_form(
    vm: &mut dyn Vm,
    base: bund2_api::NativeFn,
    name: &str,
) -> Result<(), Error> {
    let Some(operand) = vm.pull_workbench() else {
        return Err(Error(format!(
            "Workbench is too shallow for inline {}",
            name.to_uppercase()
        )));
    };
    let floor = vm.depth();
    vm.push(operand);
    let outcome = base(vm);
    // What the word left above where it started, oldest first, so the
    // workbench ends with the same value on top that the stack had.
    let extra = vm.depth().saturating_sub(floor);
    let mut left = Vec::with_capacity(extra);
    for _ in 0..extra {
        match vm.pull() {
            Some(v) => left.push(v),
            None => break,
        }
    }
    for v in left.into_iter().rev() {
        vm.push_workbench(v);
    }
    outcome
}

/// Register `name.` as [`bench_form`] over the function `name` is bound to.
macro_rules! bench {
    ($r:expr, $name:literal, $f:expr) => {
        $r.register_native(
            concat!($name, "."),
            |vm| $crate::wb::bench_form(vm, $f, concat!($name, ".")),
            bund2_api::StackEffect::fixed(0, 0),
            bund2_api::WordKind::Sync,
        );
    };
}
pub(crate) use bench;

#[cfg(test)]
mod bench_form_tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;

    /// Every form D111 added. Kept here by hand, so that removing one from a
    /// registration is a failing test and not a quiet loss.
    const ADDED: [&str; 58] = [
        "?alias", "?class", "?effect", "?lambda", "?object", "?stdlib", "?word", "alias=",
        "compile", "context", "csv", "curry", "generator.sample", "graph.allpath",
        "graph.transitiveclosure", "is", "json", "json.from_value", "json.to_value", "lambda!",
        "lambda=", "len", "math.abs", "math.acos", "math.asin", "math.atan", "math.cbrt",
        "math.ceil", "math.cos", "math.cosecant", "math.cosh", "math.exp", "math.factorial",
        "math.floor", "math.fract", "math.ln", "math.log10", "math.round", "math.signum",
        "math.sin", "math.sinh", "math.sqrt", "math.tan", "math.tanh", "not", "ptr",
        "resolve.class", "seq", "string.camel", "string.lower", "string.snake", "string.title",
        "string.upper", "time.timestamp", "type", "type.of", "unwrap", "var?",
    ];

    /// Operands of enough kinds that each word both answers and refuses.
    const OPERANDS: [&str; 8] = [
        "0.5",
        "3",
        "\"hello big world\"",
        "\"[1,2]\"",
        "[ 1 2 3 ]",
        ":len",
        "true",
        "{ 1 }",
    ];

    /// What a program left: its refusal, or the stack and then the workbench,
    /// each bottom first.
    fn outcome(src: &str) -> Result<(Vec<String>, Vec<String>), String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        // A time displays with the moment it was first looked at, which two
        // runs do not share. Everything after `stamp:` up to the comma goes.
        let shown = |v: &bund2_value::BundValue| {
            let text = v.display();
            match text.split_once("stamp: ") {
                Some((head, tail)) => {
                    let rest = tail.split_once(',').map_or("", |(_, r)| r);
                    format!("{head}stamp{rest}")
                }
                None => text,
            }
        };
        let stack = i.snapshot().iter().map(shown).collect();
        let mut bench = Vec::new();
        while let Some(v) = i.pull_workbench() {
            bench.push(shown(&v));
        }
        bench.reverse();
        Ok((stack, bench))
    }

    /// **The workbench form answers what the plain form answers, on the other
    /// side, and refuses in the same words.** `7` sits beneath throughout: the
    /// plain form leaves it under its answer and the workbench form leaves it
    /// alone.
    #[test]
    fn each_added_form_is_its_plain_word_on_the_workbench() {
        let mut answered = 0;
        for word in ADDED {
            // A word whose answer is drawn by chance is compared only on
            // whether it answers.
            let by_chance = word == "generator.sample";
            for operand in OPERANDS {
                let plain = outcome(&format!("7 {operand} {word}"));
                let bench = outcome(&format!("7 {operand} . {word}."));
                match (plain, bench) {
                    (Ok((stack, wb)), Ok((bstack, bwb))) => {
                        answered += 1;
                        assert!(wb.is_empty(), "{operand} {word}: plain touched the workbench");
                        assert_eq!(bstack, ["7"], "{operand} {word}.: the stack beneath moved");
                        if !by_chance {
                            assert_eq!(&stack[1..], &bwb[..], "{operand} {word}.");
                        }
                    }
                    // The report names the word that was applied, which is
                    // the one place the two differ; what the word said is
                    // after it.
                    (Err(a), Err(b)) => {
                        let said = |e: &str| {
                            e.rsplit_once(" returned error: ")
                                .map_or(e.to_string(), |(_, m)| m.to_string())
                        };
                        assert_eq!(said(&a), said(&b), "{operand} {word}.");
                    }
                    (a, b) => panic!("{operand} {word}: plain {a:?}, workbench {b:?}"),
                }
            }
        }
        // The palette is doing its job only if words answer as well as refuse.
        assert!(answered > 120, "only {answered} programs answered");
    }

    /// A refusal puts back on the workbench what the plain word would have
    /// left on the stack, and adds nothing to the stack.
    #[test]
    fn a_refusal_leaves_the_stack_as_it_was() {
        for word in ADDED {
            for operand in OPERANDS {
                let mut plain = Interp::new();
                crate::register_all(&mut plain.registry);
                let src = format!("7 {operand} {word}");
                let stream = bund2_syntax::compile(&src).expect("compiles");
                if plain.eval(&stream).is_ok() {
                    continue;
                }
                let survived = plain.depth() - 1;

                let mut bench = Interp::new();
                crate::register_all(&mut bench.registry);
                let src = format!("7 {operand} . {word}.");
                let stream = bund2_syntax::compile(&src).expect("compiles");
                assert!(bench.eval(&stream).is_err(), "{src}");
                assert_eq!(bench.depth(), 1, "{src}: the stack grew");
                assert_eq!(bench.workbench_depth(), survived, "{src}");
            }
        }
    }

    /// The one message the form adds.
    #[test]
    fn an_empty_workbench_is_reported_under_the_dotted_name() {
        for word in ADDED {
            let e = outcome(&format!("7 {word}.")).expect_err("refused");
            assert!(
                e.ends_with(&format!(
                    "Workbench is too shallow for inline {}.",
                    word.to_uppercase()
                )),
                "{word}.: {e}"
            );
        }
    }
}
