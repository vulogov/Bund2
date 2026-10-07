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

/// The workbench form of a word that the reference gives none — D18, as D24
/// bounds it and D111 applies it.
///
/// **One operand from the workbench, the others from the stack, and
/// everything the word answers back to the workbench.** That is D24's shape.
/// `consumes` is how many operands the plain word takes and `slot` is where
/// among them the workbench's goes, counted from the top: 0 is the operand
/// the plain word has on top.
///
/// **Which operand is the workbench's is D111's ruling, by family.** A word
/// that works on a container takes the container — `set.` builds a dict up on
/// the workbench as the reference's `push.` builds a list — and any other
/// word takes the operand the plain form has on top. The slot is written at
/// each registration, from that word's own operand order.
///
/// The base word runs unchanged, on the stack, over the operand put there for
/// it. So it answers, refuses and words its refusals exactly as the plain
/// form does — a `math.sqrt.` that could disagree with `math.sqrt` would be a
/// second implementation to keep in step.
///
/// - **A word that leaves an operand in place leaves it on the workbench.**
///   `len` answers beside its operand; so does `len.`.
/// - **A refusal moves nothing new onto the stack.** Whatever the base left
///   above where its operands began goes to the workbench.
/// - **The stack beneath the operands is not touched**, and the form's
///   declared effect says how many it takes from there: all but one.
///
/// `name` is the dotted name, for the two messages this adds.
pub(crate) fn bench_form(
    vm: &mut dyn Vm,
    base: bund2_api::NativeFn,
    name: &str,
    consumes: usize,
    slot: usize,
) -> Result<(), Error> {
    if vm.workbench_depth() < 1 {
        return Err(Error(format!(
            "Workbench is too shallow for inline {}",
            name.to_uppercase()
        )));
    }
    let others = consumes.saturating_sub(1);
    if vm.depth() < others {
        return Err(Error(format!(
            "Stack is too shallow for inline {}",
            name.to_uppercase()
        )));
    }
    let floor = vm.depth() - others;
    let Some(operand) = vm.pull_workbench() else {
        return Err(Error::internal(
            "the workbench was not empty and gave nothing",
        ));
    };
    // Lift what sits above the slot, put the operand in, and put it back.
    let mut above = Vec::with_capacity(slot);
    for _ in 0..slot.min(others) {
        match vm.pull() {
            Some(v) => above.push(v),
            None => break,
        }
    }
    vm.push(operand);
    for v in above.into_iter().rev() {
        vm.push(v);
    }
    let outcome = base(vm);
    // What the word left above where its operands began, oldest first, so the
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
/// Two arguments more for a word of several operands: how many, and the slot.
macro_rules! bench {
    ($r:expr, $name:literal, $f:expr) => {
        $crate::wb::bench!($r, $name, $f, 1, 0);
    };
    ($r:expr, $name:literal, $f:expr, $consumes:literal, $slot:literal) => {
        $r.register_native(
            concat!($name, "."),
            |vm| $crate::wb::bench_form(vm, $f, concat!($name, "."), $consumes, $slot),
            // What it takes from the *stack*: every operand but the
            // workbench's. Its answer goes to the workbench, so it leaves
            // nothing. An earlier version declared `(0, 0)` for every form,
            // and the effect audit refused ten of them.
            bund2_api::StackEffect::fixed($consumes - 1, 0),
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

    /// D111's words of two and three operands: the name, the operands as the
    /// plain word wants them bottom to top, and which of them — counted from
    /// the top — the workbench form takes from the workbench.
    const SEVERAL: [(&str, &[&str], usize); 32] = [
        ("==", &["3", "3"], 0),
        ("!=", &["3", "4"], 0),
        ("≠", &["3", "4"], 0),
        ("<", &["3", "4"], 0),
        ("<=", &["3", "4"], 0),
        ("⩽", &["3", "4"], 0),
        (">", &["3", "4"], 0),
        (">=", &["3", "4"], 0),
        ("⩾", &["3", "4"], 0),
        ("and", &["true", "false"], 0),
        ("or", &["true", "false"], 0),
        ("pair", &["1", "2"], 0),
        ("complex", &["1.0", "2.0"], 0),
        ("math.nroot", &["27.0", "3.0"], 0),
        ("math.power", &["2.0", "10.0"], 0),
        ("math.perimeter", &["2.0", "3.0"], 0),
        ("seq.asc", &["4", "1.0", "0.0"], 0),
        ("seq.desc", &["4", "1.0", "9.0"], 0),
        ("set", &["dict", "\"a\"", "1"], 2),
        ("∈", &["dict", "\"a\"", "1"], 2),
        ("get", &["dict \"a\" 1 set", "\"a\""], 1),
        ("?key", &["dict \"a\" 1 set", "\"a\""], 1),
        ("concat_with_space", &["\"t\" convert.to_textbuffer", "\"x\""], 1),
        ("sp", &["\"t\" convert.to_textbuffer", "\"x\""], 1),
        ("tag", &["5", "\"k\"", "\"v\""], 2),
        ("attribute", &["5", "\"a\""], 1),
        ("?type", &["5", "\"Integer\""], 1),
        ("json.path", &["\"$.a\"", "'{\"a\":1}' json"], 0),
        ("graph.paths", &[":A", "[ [ :A :B 1.0 ] [ :B :C 2.0 ] ] [ :A :B :C ] graph!"], 0),
        ("graph.path", &[":C", ":A", "[ [ :A :B 1.0 ] [ :B :C 2.0 ] ] [ :A :B :C ] graph!"], 0),
        ("wrap", &["9", "\"x\" \"List\" object"], 0),
        ("?is", &["\"x\" \"List\" object", "\"List\""], 1),
    ];

    /// The plain program, and the same with one operand on the workbench.
    fn both(word: &str, operands: &[&str], slot: usize) -> (String, String) {
        let at = operands.len() - 1 - slot;
        let plain = format!("7 {} {word}", operands.join(" "));
        let rest: Vec<&str> = operands
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != at)
            .map(|(_, o)| *o)
            .collect();
        let bench = format!("7 {} . {} {word}.", operands[at], rest.join(" "));
        (plain, bench)
    }

    /// **Each form of several operands answers what its plain word answers,
    /// with the named operand on the workbench and the answer left there.**
    /// Every row has to answer: a row that refused on both sides would pass
    /// while proving nothing about which operand went where.
    #[test]
    fn each_form_of_several_operands_takes_the_operand_d111_names() {
        for (word, operands, slot) in SEVERAL {
            let (plain, bench) = both(word, operands, slot);
            let (stack, wb) = outcome(&plain).unwrap_or_else(|e| panic!("{plain}: {e}"));
            let (bstack, bwb) = outcome(&bench).unwrap_or_else(|e| panic!("{bench}: {e}"));
            assert!(wb.is_empty(), "{plain}");
            assert_eq!(bstack, ["7"], "{bench}: the stack beneath moved");
            // `graph.paths` answers its rows in an order that changes.
            if word != "graph.paths" {
                assert_eq!(&stack[1..], &bwb[..], "{bench}");
            } else {
                assert_eq!(stack.len() - 1, bwb.len(), "{bench}");
            }
        }
    }

    /// With the operand in any *other* slot the word is given the wrong
    /// thing, so for the container words the two must not agree. This is what
    /// says the slot in the registration is the one in the table.
    #[test]
    fn a_container_word_takes_its_container_and_nothing_else() {
        for (word, operands, slot) in SEVERAL {
            if slot == 0 {
                continue;
            }
            let (plain, _) = both(word, operands, slot);
            let (_, wrong) = both(word, operands, 0);
            let right = outcome(&plain).map(|(s, _)| s[1..].to_vec());
            let other = outcome(&wrong).map(|(_, w)| w);
            assert_ne!(right, other, "{wrong} answered as if the key were the container");
        }
    }

    /// `set.` builds on the workbench, which is the point of the ruling.
    #[test]
    fn a_dict_is_built_up_on_the_workbench() {
        let (stack, wb) = outcome("dict . \"a\" 1 set. \"b\" 2 set. \"a\" get.").expect("runs");
        assert!(stack.is_empty(), "{stack:?}");
        assert_eq!(wb, ["1"]);
        let (_, wb) = outcome("\"\" convert.to_textbuffer . \"a\" sp. \"b\" sp.").expect("runs");
        assert_eq!(wb, ["a b"]);
    }

    /// The second message the form adds: operands the stack was to supply.
    #[test]
    fn a_stack_without_the_other_operands_is_reported() {
        let e = outcome("dict . \"a\" set.").expect_err("refused");
        assert!(e.ends_with("Stack is too shallow for inline SET."), "{e}");
        let e = outcome("3 . <.").expect_err("refused");
        assert!(e.ends_with("Stack is too shallow for inline <."), "{e}");
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
