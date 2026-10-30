//! The `kb.` natives (RFC-0013 §S9).
//!
//! The building words (M2): `kb.new`, `kb!`, `kb.consult*`, `kb.assert`,
//! `kb.asserta`, `kb.retract`, `kb.merge`, `kb.clauses` and `kb.inferences`,
//! with the `.` forms of the six that take a KB. Each reads and writes the KB
//! through `store.rs`, so what is here is operands and texts.
//!
//! The asking words (M3): `kb.query`, `kb.once` and `kb.holds`, each a
//! `solve.rs` activation, and [`VmBridge`], the one implementation of
//! `Bridge`. They are opaque: a bridge re-enters evaluation
//! (`promotable.rs`).
//!
//! - **E00 is the house text, unprefixed** (`singles.rs`, `var`). Every
//!   other refusal begins `<W>: `, the word's name in capitals (§S12).
//! - **A `.` form takes the KB from the workbench and leaves the answer
//!   there** (D111's container family), through `bench_form`, so it refuses
//!   as its plain word does.

use bund2_api::{Error, Leaves, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, INTEGER, LIST, MAP, NODATA};

use super::solve::{Bridge, Solver, Stop};
use super::store::{Kb, Refused};

fn refused(word: &str, r: &Refused) -> Error {
    Error(format!("{}: {}", word.to_uppercase(), r.text()))
}

/// E00, and the `n` operands it guards, `#1` first.
fn operands(vm: &mut dyn Vm, word: &str, n: usize) -> Result<Vec<BundValue>, Error> {
    if vm.depth() < n {
        return Err(Error(format!("Stack is too shallow for inline {word}")));
    }
    (0..n)
        .map(|_| {
            vm.pull().ok_or_else(|| {
                Error::internal(format!(
                    "{word}: the stack was deep enough and gave nothing"
                ))
            })
        })
        .collect()
}

/// E00, and the one operand it guards.
fn operand(vm: &mut dyn Vm, word: &str) -> Result<BundValue, Error> {
    operands(vm, word, 1)?
        .pop()
        .ok_or_else(|| Error::internal(format!("{word}: one operand was guarded and none pulled")))
}

fn answer(vm: &mut dyn Vm, word: &str, kb: Result<Kb, Refused>) -> Result<(), Error> {
    let kb = kb.map_err(|r| refused(word, &r))?;
    vm.push(kb.into_value());
    Ok(())
}

/// `kb.new` — a KB with no clauses.
fn kb_new(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(Kb::new().into_value());
    Ok(())
}

/// `[ clause… ] kb!` — a KB of the LIST's clauses, in order.
fn kb_bang(vm: &mut dyn Vm) -> Result<(), Error> {
    let list = operand(vm, "kb!")?;
    let kb = match list.as_list().filter(|_| list.dt() == LIST) {
        Some(items) => Kb::build(items),
        None => Err(Refused::NotClauses),
    };
    answer(vm, "kb!", kb)
}

/// `c₁ … cₙ kb.consult*` — a KB of the clauses down to the first NODATA,
/// which it consumes, or to the bottom of the stack, oldest first (D12,
/// D182). Each turn takes a value off the stack, so the scan ends.
fn kb_consult(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut items = Vec::new();
    while let Some(v) = vm.pull() {
        if v.dt() == NODATA {
            break;
        }
        items.push(v);
    }
    items.reverse();
    answer(vm, "kb.consult*", Kb::build(&items))
}

/// The two operands of a word that takes `kb x`: `x` is `#1`, the KB `#2`.
fn kb_and(vm: &mut dyn Vm, word: &str) -> Result<(Kb, BundValue), Error> {
    let mut ops = operands(vm, word, 2)?.into_iter();
    let (Some(x), Some(kb)) = (ops.next(), ops.next()) else {
        return Err(Error::internal(format!(
            "{word}: two operands were guarded and not two pulled"
        )));
    };
    let kb = Kb::read(&kb, 2).map_err(|r| refused(word, &r))?;
    Ok((kb, x))
}

/// `kb clause kb.assert` — the clause after its predicate's others.
fn kb_assert(vm: &mut dyn Vm) -> Result<(), Error> {
    let (kb, c) = kb_and(vm, "kb.assert")?;
    answer(vm, "kb.assert", kb.assert(&c, false))
}

/// `kb clause kb.asserta` — the clause before its predicate's others.
fn kb_asserta(vm: &mut dyn Vm) -> Result<(), Error> {
    let (kb, c) = kb_and(vm, "kb.asserta")?;
    answer(vm, "kb.asserta", kb.assert(&c, true))
}

/// `kb clause kb.retract` — without the first clause that unifies with the
/// operand, renamed; the KB unchanged when none does.
fn kb_retract(vm: &mut dyn Vm) -> Result<(), Error> {
    let (kb, c) = kb_and(vm, "kb.retract")?;
    answer(vm, "kb.retract", kb.retract(&c))
}

/// `kb₁ kb₂ kb.merge` — kb₂'s clauses after kb₁'s, per predicate.
fn kb_merge(vm: &mut dyn Vm) -> Result<(), Error> {
    let (first, second) = kb_and(vm, "kb.merge")?;
    let second = Kb::read(&second, 1).map_err(|r| refused("kb.merge", &r))?;
    answer(vm, "kb.merge", first.merge(&second))
}

/// `kb kb.clauses` — every clause, in `preds` key order, then clause order.
fn kb_clauses(vm: &mut dyn Vm) -> Result<(), Error> {
    let kb = operand(vm, "kb.clauses")?;
    let all = Kb::read(&kb, 1)
        .and_then(|kb| kb.clauses())
        .map_err(|r| refused("kb.clauses", &r))?;
    vm.push(BundValue::list(all));
    Ok(())
}

/// `kb n kb.inferences` — the KB with its inference cap; ≤ 0 means none.
fn kb_inferences(vm: &mut dyn Vm) -> Result<(), Error> {
    let (kb, n) = kb_and(vm, "kb.inferences")?;
    let Some(n) = n.as_int().filter(|_| n.dt() == INTEGER) else {
        return Err(Error(format!(
            "KB.INFERENCES: #1 parameter must be INT, found {}",
            n.type_name()
        )));
    };
    vm.push(kb.with_inferences(n).into_value());
    Ok(())
}

/// The `Vm` as the engine sees it.
pub struct VmBridge<'a> {
    vm: &'a mut dyn Vm,
    /// The asking word, in capitals, for W1's prefix.
    word: String,
}

impl<'a> VmBridge<'a> {
    /// The bridge for one call of an asking word, `word` in capitals.
    pub fn new(vm: &'a mut dyn Vm, word: &str) -> VmBridge<'a> {
        VmBridge {
            vm,
            word: word.to_string(),
        }
    }
}

impl Bridge for VmBridge<'_> {
    /// W1: a `Warning`, so the query goes on (D36, D39).
    fn warn(&mut self, reason: String) {
        self.vm.report(bund2_api::diag::Diagnostic::warning(format!(
            "{}: {reason}",
            self.word
        )));
    }

    fn run(&mut self, body: Vec<BundValue>) -> Result<Option<BundValue>, Error> {
        super::bridge::run(self.vm, body)
    }
}

/// How many answers an asking word wants, and what it answers with.
#[derive(Clone, Copy)]
enum Asking {
    /// `kb.query`: a LIST of every answer.
    All,
    /// `kb.once`: the first answer, or NODATA.
    Once,
    /// `kb.holds`: whether there is an answer.
    Holds,
}

/// `kb goal kb.query` and its two siblings.
fn ask(vm: &mut dyn Vm, word: &str, asking: Asking) -> Result<(), Error> {
    let up = word.to_uppercase();
    let stop = |s: Stop| match s {
        Stop::Code(text) => Error(format!("{up}: {text}")),
        Stop::Raw(e) => e,
    };
    let (kb, goal) = kb_and(vm, word)?;
    if goal.dt() == NODATA {
        return Err(Error(format!("{up}: the query has no goal")));
    }
    let mut solver = Solver::new(&kb, &goal).map_err(stop)?;
    let mut bridge = VmBridge {
        vm,
        word: up.clone(),
    };
    let answer = match asking {
        Asking::All => {
            let mut all = Vec::new();
            while let Some(a) = solver.next(&mut bridge).map_err(stop)? {
                all.push(a);
            }
            BundValue::list(all)
        }
        Asking::Once => solver
            .next(&mut bridge)
            .map_err(stop)?
            .unwrap_or_else(BundValue::nodata),
        Asking::Holds => BundValue::boolean(solver.next(&mut bridge).map_err(stop)?.is_some()),
    };
    bridge.vm.push(answer);
    Ok(())
}

/// The functor and arity of `kb.call` and `kb.word`, `#2` and `#1`: a STRING
/// (or PTR, as `:name` lowers) and a non-negative INT, or E09.
fn functor_arity(word: &str, functor: &BundValue, n: &BundValue) -> Result<(String, usize), Error> {
    let name = functor
        .as_str()
        .filter(|_| matches!(functor.dt(), bund2_value::STRING | bund2_value::PTR));
    let arity = n
        .as_int()
        .filter(|_| n.dt() == INTEGER)
        .and_then(|n| usize::try_from(n).ok());
    match (name, arity) {
        (Some(f), Some(n)) => Ok((f, n)),
        _ => Err(Error(format!(
            "{}: the functor must be a STRING and the arity a non-negative INT",
            word.to_uppercase()
        ))),
    }
}

/// `a₁ … aₙ kb functor n kb.call` — every answer of `functor(a₁, …, aₙ)`,
/// each the arguments as instantiated, a LIST (L5). NODATA among them is an
/// unbound argument.
fn kb_call(vm: &mut dyn Vm) -> Result<(), Error> {
    let up = "KB.CALL";
    let stop = |s: Stop| match s {
        Stop::Code(text) => Error(format!("{up}: {text}")),
        Stop::Raw(e) => e,
    };
    let mut ops = operands(vm, "kb.call", 3)?.into_iter();
    let (Some(n), Some(functor), Some(kb)) = (ops.next(), ops.next(), ops.next()) else {
        return Err(Error::internal(
            "kb.call: three operands were guarded and not three pulled",
        ));
    };
    let (functor, n) = functor_arity("kb.call", &functor, &n)?;
    let kb = Kb::read(&kb, 3).map_err(|r| refused("kb.call", &r))?;
    let mut args = operands(vm, "kb.call", n)?;
    args.reverse();
    let mut solver = Solver::for_call(&kb, &functor, &args).map_err(stop)?;
    let mut bridge = VmBridge {
        vm,
        word: up.to_string(),
    };
    let mut all = Vec::new();
    while let Some(a) = solver.next(&mut bridge).map_err(stop)? {
        all.push(a);
    }
    bridge.vm.push(BundValue::list(all));
    Ok(())
}

/// `var functor n kb.word` — register `functor` as a word whose body is
/// `"var" var? "functor" n kb.call` (§S10). The KB is read when the word
/// runs. It behaves as `register` does, and warns as it does on binding
/// `endcontext` (F62).
fn kb_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut ops = operands(vm, "kb.word", 3)?.into_iter();
    let (Some(n), Some(functor), Some(var)) = (ops.next(), ops.next(), ops.next()) else {
        return Err(Error::internal(
            "kb.word: three operands were guarded and not three pulled",
        ));
    };
    let (functor, n) = functor_arity("kb.word", &functor, &n)?;
    let Some(var) = var
        .as_str()
        .filter(|_| matches!(var.dt(), bund2_value::STRING | bund2_value::PTR))
    else {
        return Err(Error(format!(
            "KB.WORD: #3 parameter must be STRING, found {}",
            var.type_name()
        )));
    };
    if super::store::reserved(&functor, n).is_some() {
        return Err(refused(
            "kb.word",
            &Refused::Reserved(format!("{functor}/{n}")),
        ));
    }
    let body = BundValue::lambda(vec![
        BundValue::str(var),
        BundValue::call("var?"),
        BundValue::str(functor.as_str()),
        BundValue::int(i64::try_from(n).unwrap_or(i64::MAX)),
        BundValue::call("kb.call"),
    ]);
    if functor == "endcontext" {
        vm.report(bund2_api::diag::Diagnostic::warning(
            crate::values::ENDCONTEXT_SHADOWED,
        ));
    }
    vm.register_lambda(&functor, body);
    Ok(())
}

fn kb_query(vm: &mut dyn Vm) -> Result<(), Error> {
    ask(vm, "kb.query", Asking::All)
}

fn kb_once(vm: &mut dyn Vm) -> Result<(), Error> {
    ask(vm, "kb.once", Asking::Once)
}

fn kb_holds(vm: &mut dyn Vm) -> Result<(), Error> {
    ask(vm, "kb.holds", Asking::Holds)
}

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

pub fn register(r: &mut Registry) {
    let map = |c| eff(c, 1).leaving(Leaves::Tag(MAP));
    r.register_native("kb.new", kb_new, map(0), WordKind::Sync);
    r.register_native("kb!", kb_bang, map(1), WordKind::Sync);
    // Down to a NODATA or the bottom, so no pair says it (D12).
    r.register_native(
        "kb.consult*",
        kb_consult,
        StackEffect::opaque(0),
        WordKind::Sync,
    );
    // The KB is beneath the other operand; a `.` form holds the KB, D111's
    // container family, so its slot is 1.
    r.register_native("kb.assert", kb_assert, eff(2, 1), WordKind::Sync);
    crate::wb::bench!(r, "kb.assert", kb_assert, 2, 1);
    r.register_native("kb.asserta", kb_asserta, eff(2, 1), WordKind::Sync);
    crate::wb::bench!(r, "kb.asserta", kb_asserta, 2, 1);
    r.register_native("kb.retract", kb_retract, eff(2, 1), WordKind::Sync);
    crate::wb::bench!(r, "kb.retract", kb_retract, 2, 1);
    r.register_native("kb.merge", kb_merge, eff(2, 1), WordKind::Sync);
    crate::wb::bench!(r, "kb.merge", kb_merge, 2, 1);
    r.register_native(
        "kb.clauses",
        kb_clauses,
        eff(1, 1).leaving(Leaves::Tag(LIST)),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "kb.clauses", kb_clauses);
    r.register_native("kb.inferences", kb_inferences, eff(2, 1), WordKind::Sync);
    crate::wb::bench!(r, "kb.inferences", kb_inferences, 2, 1);
    // Opaque: a bridge may re-enter evaluation, and no `.` form (D111).
    for (name, f) in [
        ("kb.query", kb_query as bund2_api::NativeFn),
        ("kb.once", kb_once),
        ("kb.holds", kb_holds),
    ] {
        r.register_native(name, f, StackEffect::opaque(2), WordKind::Sync);
    }
    // Its depth is the arity it is handed.
    r.register_native("kb.call", kb_call, StackEffect::opaque(3), WordKind::Sync);
    // `register`'s shape: three operands in, a word bound, nothing left.
    r.register_native("kb.word", kb_word, eff(3, 0), WordKind::Sync);
}

/// **Criterion 1a**: each building word's answer, each `.` form's, and each
/// code the building words raise, by its text.
#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    use crate::kb::store::Kb;

    /// Push `before`, then run `src`.
    fn run_on(before: Vec<BundValue>, src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        for v in before {
            i.push(v);
        }
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| {
            e.0.rsplit_once(" returned error: ")
                .map_or(e.0.clone(), |(_, m)| m.to_string())
        })?;
        Ok(i)
    }

    fn run(src: &str) -> Interp {
        run_on(Vec::new(), src).unwrap_or_else(|e| panic!("{src} failed: {e}"))
    }

    fn top(src: &str) -> BundValue {
        run(src).peek().unwrap_or_else(|| panic!("{src}: empty"))
    }

    fn err_on(before: Vec<BundValue>, src: &str) -> String {
        match run_on(before, src) {
            Ok(_) => panic!("{src} was expected to fail"),
            Err(e) => e,
        }
    }

    fn err(src: &str) -> String {
        err_on(Vec::new(), src)
    }

    /// The clauses of the KB on top of `src`'s stack, as one LIST's display.
    fn clauses(src: &str) -> String {
        let kb = Kb::read(&top(src), 1).unwrap_or_else(|r| panic!("{src}: {}", r.text()));
        BundValue::list(kb.clauses().unwrap()).display()
    }

    /// How `src`, a LIST literal, displays: what `clauses` should equal.
    fn shown(src: &str) -> String {
        top(src).display()
    }

    const FAMILY: &str = "[ [ parent tom bob ] [ parent bob ann ] \
                          [ [ grandparent X Z ] ← [ parent X Y ] [ parent Y Z ] ] ]";

    #[test]
    fn kb_new_has_no_clauses() {
        let kb = top("kb.new");
        let m = kb.as_map().unwrap();
        assert_eq!(
            m.get("type").and_then(BundValue::as_str).as_deref(),
            Some("kb")
        );
        assert_eq!(clauses("kb.new"), BundValue::list(Vec::new()).display());
    }

    /// `kb!` keys each clause by its head and drops a rule's `←`;
    /// `kb.clauses` answers in `preds` key order, then clause order.
    #[test]
    fn kb_bang_builds_and_clauses_reads_back() {
        assert_eq!(
            clauses(&format!("{FAMILY} kb!")),
            shown(
                "[ [ [ grandparent X Z ] [ parent X Y ] [ parent Y Z ] ] \
                   [ parent tom bob ] [ parent bob ann ] ]"
            )
        );
        let kb = top(&format!("{FAMILY} kb!"));
        let preds = kb.as_map().unwrap().get("preds").unwrap().as_map().unwrap();
        assert_eq!(
            preds.keys().collect::<Vec<_>>(),
            ["grandparent/2", "parent/2"]
        );
        // `<-` is the other spelling; `[ ready ← … ]` is a fact of arity 2.
        let kb = top("[ [ [ ready ] <- [ p ] ] [ go ← now ] ] kb!");
        let preds = kb.as_map().unwrap().get("preds").unwrap().as_map().unwrap();
        assert_eq!(preds.keys().collect::<Vec<_>>(), ["go/2", "ready/0"]);
        // `p/10` before `p/2`: `preds` is a BTreeMap.
        assert_eq!(
            clauses("[ [ p a b ] [ p a b c d e f g h i j ] ] kb!"),
            shown("[ [ p a b c d e f g h i j ] [ p a b ] ]")
        );
        // Atoms are heads whatever they were written as.
        let kb = top("[ ready :go ] kb!");
        let preds = kb.as_map().unwrap().get("preds").unwrap().as_map().unwrap();
        assert_eq!(preds.keys().collect::<Vec<_>>(), ["go/0", "ready/0"]);
        let stack = run(&format!("{FAMILY} kb! kb.clauses")).snapshot();
        assert_eq!(stack.len(), 1);
    }

    /// Down to a NODATA, which it consumes, oldest first; or to the bottom.
    #[test]
    fn kb_consult_takes_down_to_nodata_or_the_bottom() {
        let i = run("7 ∅ [ p a ] [ p b ] kb.consult*");
        let stack = i.snapshot();
        assert_eq!(stack.len(), 2, "{stack:?}");
        assert_eq!(stack[0].display(), "7");
        assert_eq!(
            clauses("7 ∅ [ p a ] [ p b ] kb.consult*"),
            shown("[ [ p a ] [ p b ] ]")
        );
        assert_eq!(run("[ p a ] [ p b ] kb.consult*").snapshot().len(), 1);
        assert_eq!(
            clauses("kb.consult*"),
            BundValue::list(Vec::new()).display()
        );
    }

    #[test]
    fn assert_appends_and_asserta_prepends() {
        assert_eq!(
            clauses("[ [ p a ] ] kb! [ p b ] kb.assert"),
            shown("[ [ p a ] [ p b ] ]")
        );
        assert_eq!(
            clauses("[ [ p a ] ] kb! [ p b ] kb.asserta"),
            shown("[ [ p b ] [ p a ] ]")
        );
        assert_eq!(
            clauses("kb.new [ [ q X ] ← [ p X ] ] kb.assert"),
            shown("[ [ [ q X ] [ p X ] ] ]")
        );
    }

    /// The first clause whose renamed head and body unify; none, unchanged.
    #[test]
    fn retract_removes_the_first_clause_that_unifies() {
        let kb = "[ [ p a ] [ p b ] [ p a ] [ [ q X ] ← [ p X ] ] ] kb!";
        assert_eq!(
            clauses(&format!("{kb} [ p a ] kb.retract")),
            shown("[ [ p b ] [ p a ] [ [ q X ] [ p X ] ] ]")
        );
        assert_eq!(
            clauses(&format!("{kb} [ p Y ] kb.retract")),
            shown("[ [ p b ] [ p a ] [ [ q X ] [ p X ] ] ]")
        );
        // A rule matches by head and body; a variable renames.
        assert_eq!(
            clauses(&format!("{kb} [ [ q Z ] ← [ p Z ] ] kb.retract")),
            shown("[ [ p a ] [ p b ] [ p a ] ]")
        );
        // The body must unify too: `q(a)` the fact is not `q(X) :- p(X)`.
        assert_eq!(clauses(&format!("{kb} [ q a ] kb.retract")), clauses(kb));
        assert_eq!(clauses(&format!("{kb} [ p c ] kb.retract")), clauses(kb));
        // Retracting the last clause of a predicate removes its key.
        let kb = top("[ [ p a ] ] kb! [ p a ] kb.retract");
        let preds = kb.as_map().unwrap().get("preds").unwrap().as_map().unwrap();
        assert!(preds.is_empty(), "{preds:?}");
    }

    #[test]
    fn merge_puts_the_second_after_the_first_per_predicate() {
        let src = "[ [ p a ] [ q a ] ] kb! 5 kb.inferences \
                   [ [ p b ] [ r b ] ] kb! 9 kb.inferences kb.merge";
        assert_eq!(clauses(src), shown("[ [ p a ] [ p b ] [ q a ] [ r b ] ]"));
        let kb = top(src);
        assert_eq!(
            kb.as_map()
                .unwrap()
                .get("inferences")
                .and_then(BundValue::as_int),
            Some(5)
        );
        let kb =
            top("kb.new :tabled [ \"p/1\" ] set kb.new :tabled [ \"p/1\" \"q/1\" ] set kb.merge");
        let tabled = kb.as_map().unwrap().get("tabled").unwrap().display();
        assert_eq!(tabled, shown("[ \"p/1\" \"q/1\" ]"));
    }

    #[test]
    fn inferences_sets_the_cap() {
        let kb = top("kb.new 1000 kb.inferences");
        assert_eq!(
            kb.as_map()
                .unwrap()
                .get("inferences")
                .and_then(BundValue::as_int),
            Some(1000)
        );
        let kb = top("kb.new -1 kb.inferences");
        assert_eq!(
            kb.as_map()
                .unwrap()
                .get("inferences")
                .and_then(BundValue::as_int),
            Some(-1)
        );
    }

    /// Every form: the KB from the workbench, the answer back there, and the
    /// same answer as the plain word.
    #[test]
    fn each_form_holds_the_kb_on_the_workbench() {
        const KB: &str = "[ [ p a ] [ q a ] ] kb!";
        let forms: [(&str, &str); 6] = [
            ("kb.assert", "[ p b ]"),
            ("kb.asserta", "[ p b ]"),
            ("kb.retract", "[ p a ]"),
            ("kb.merge", "[ [ p c ] ] kb!"),
            ("kb.clauses", ""),
            ("kb.inferences", "10"),
        ];
        for (word, rest) in forms {
            let plain = run(&format!("7 {KB} {rest} {word}"));
            let mut bench = run(&format!("7 {KB} . {rest} {word}."));
            let stack = plain.snapshot();
            assert_eq!(stack.len(), 2, "{word}: {stack:?}");
            assert_eq!(
                bench
                    .snapshot()
                    .iter()
                    .map(BundValue::display)
                    .collect::<Vec<_>>(),
                ["7"],
                "{word}."
            );
            let answer = bench.pull_workbench().unwrap();
            assert_eq!(bench.workbench_depth(), 0, "{word}.");
            assert_eq!(answer.display(), stack[1].display(), "{word}.");
        }
    }

    // The codes, by text (§S12).

    #[test]
    fn e00_is_the_house_text_unprefixed() {
        for (word, n) in [
            ("kb!", 0),
            ("kb.assert", 1),
            ("kb.asserta", 1),
            ("kb.retract", 1),
            ("kb.merge", 1),
            ("kb.clauses", 0),
            ("kb.inferences", 1),
        ] {
            let pad = "kb.new ".repeat(n);
            assert_eq!(
                err(&format!("{pad}{word}")),
                format!("Stack is too shallow for inline {word}"),
                "{word}"
            );
        }
    }

    #[test]
    fn e01_names_the_operand() {
        assert_eq!(
            err("dict [ p a ] kb.assert"),
            "KB.ASSERT: #2 parameter must be a knowledge base"
        );
        assert_eq!(
            err("dict :type :graph set [ p a ] kb.retract"),
            "KB.RETRACT: #2 parameter must be a knowledge base"
        );
        assert_eq!(
            err("kb.new dict kb.merge"),
            "KB.MERGE: #1 parameter must be a knowledge base"
        );
        assert_eq!(
            err("dict kb.new kb.merge"),
            "KB.MERGE: #2 parameter must be a knowledge base"
        );
        assert_eq!(
            err("[ 1 ] kb.clauses"),
            "KB.CLAUSES: #1 parameter must be a knowledge base"
        );
        assert_eq!(
            err("42 5 kb.inferences"),
            "KB.INFERENCES: #2 parameter must be a knowledge base"
        );
    }

    #[test]
    fn e02_names_the_key_read() {
        assert_eq!(
            err("kb.new :preds 42 set [ p a ] kb.assert"),
            "KB.ASSERT: malformed knowledge base: preds is not a MAP"
        );
        assert_eq!(
            err("kb.new :preds 42 set kb.clauses"),
            "KB.CLAUSES: malformed knowledge base: preds is not a MAP"
        );
        let mut preds = std::collections::BTreeMap::new();
        preds.insert("p/1".to_string(), BundValue::int(3));
        let kb = Kb::new().into_value().set("preds", BundValue::map(preds));
        assert_eq!(
            err_on(vec![kb.clone()], "[ p a ] kb.assert"),
            "KB.ASSERT: malformed knowledge base: preds p/1 is not a LIST"
        );
        // Only what is read is checked: `q/1` is not `p/1`.
        assert!(run_on(vec![kb], "[ q a ] kb.assert").is_ok());
        assert_eq!(
            err("kb.new :tabled 3 set kb.new kb.merge"),
            "KB.MERGE: malformed knowledge base: tabled is not a LIST"
        );
        assert_eq!(
            err("kb.new kb.new :askable [ 3 ] set kb.merge"),
            "KB.MERGE: malformed knowledge base: askable is not a LIST of STRING"
        );
    }

    #[test]
    fn e03_refuses_a_reserved_head() {
        assert_eq!(
            err("[ [ = a b ] ] kb!"),
            "KB!: =/2 is reserved and cannot be defined"
        );
        assert_eq!(
            err("kb.new [ [ not x y z ] ← [ p ] ] kb.assert"),
            "KB.ASSERT: not/3 is reserved and cannot be defined"
        );
        // Every phase's pairs are reserved now; `true` the BOOL is `true/0`.
        assert_eq!(
            err("[ [ dif a b ] ] kb!"),
            "KB!: dif/2 is reserved and cannot be defined"
        );
        assert_eq!(
            err("kb.new [ label X ] kb.asserta"),
            "KB.ASSERTA: label/1 is reserved and cannot be defined"
        );
        assert_eq!(
            err("[ true ] kb!"),
            "KB!: true/0 is reserved and cannot be defined"
        );
        // `in/1` is not reserved, and `kb.retract` does not refuse one.
        assert!(run_on(Vec::new(), "[ [ in a ] ] kb!").is_ok());
        assert!(run_on(Vec::new(), "kb.new [ = a b ] kb.retract").is_ok());
    }

    #[test]
    fn e04_e05_e26_refuse_a_value_that_is_not_a_term() {
        let bad_var = BundValue::list(vec![BundValue::call("p"), BundValue::call("X|Y")]);
        assert_eq!(
            err_on(vec![BundValue::list(vec![bad_var.clone()])], "kb!"),
            "KB!: X|Y is not a variable name: it contains '|'"
        );
        assert_eq!(
            err_on(vec![bad_var], "kb.new swap kb.retract"),
            "KB.RETRACT: X|Y is not a variable name: it contains '|'"
        );
        let e = err("[ [ [ p [ :a | ] ] ] ] kb!");
        assert!(
            e.starts_with("KB!: '|' must stand between a head and a tail in "),
            "{e}"
        );
        let ctx = BundValue::list(vec![
            BundValue::list(vec![BundValue::call("p")]),
            BundValue::context(),
        ]);
        let e = err_on(vec![ctx], "kb.new swap kb.assert");
        assert!(
            e.starts_with("KB.ASSERT: a CONTEXT cannot be a term: "),
            "{e}"
        );
    }

    #[test]
    fn e06_refuses_a_head_that_is_not_an_atom_or_a_compound() {
        assert_eq!(
            err("[ 42 ] kb!"),
            "KB!: a clause head must be an atom or a compound: 42"
        );
        let e = err("kb.new [ :tom :bob ] kb.assert");
        assert!(
            e.starts_with("KB.ASSERT: a clause head must be an atom or a compound: "),
            "{e}"
        );
        let e = err("kb.new [ [ X ] ← [ p ] ] kb.retract");
        assert!(
            e.starts_with("KB.RETRACT: a clause head must be an atom or a compound: "),
            "{e}"
        );
        assert!(err("[ nil ] kb!").starts_with("KB!: a clause head must be"));
        assert!(err("∅ [ [ 1 2 ] ← [ p ] ] kb.consult*").starts_with("KB.CONSULT*: a clause head"));
    }

    #[test]
    fn e07_refuses_kb_bang_without_a_list() {
        assert_eq!(err("42 kb!"), "KB!: #1 parameter must be a list of clauses");
    }

    #[test]
    fn e08_refuses_a_cap_that_is_not_an_int() {
        assert_eq!(
            err("kb.new \"x\" kb.inferences"),
            "KB.INFERENCES: #1 parameter must be INT, found String"
        );
        assert_eq!(
            err("kb.new 1.5 kb.inferences"),
            "KB.INFERENCES: #1 parameter must be INT, found Float"
        );
    }

    #[test]
    fn e59_refuses_a_merge_both_tabled_and_askable() {
        assert_eq!(
            err("kb.new :tabled [ \"p/1\" ] set kb.new :askable [ \"p/1\" ] set kb.merge"),
            "KB.MERGE: p/1 cannot be both tabled and askable"
        );
    }
}
