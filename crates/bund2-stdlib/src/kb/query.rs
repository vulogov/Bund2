//! The `query` conditional (RFC-0013 §S11, L12).
//!
//! `query` pushes a CONDITIONAL of type `query`; `!` reaches [`run_query`]
//! through `run_conditional`, as `sqlite`'s runner is reached. The runner
//! solves to the next answer, pushes it, and runs `lambda` with
//! `eval_lambda`, then solves on (L6). Its state stays in this Rust frame
//! (D84), so a `lambda` that never returns early costs no Rust depth across
//! answers (criterion 4).
//!
//! **`lambda` runs on the current stack, and nothing restores it.** It is the
//! program's own code, as `sqlite`'s is, unlike a bridge's λ.
//!
//! `run_query` calls `eval_lambda`, so it is named in `REENTERING`.

use bund2_api::{Error, Leaves, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BOOL, BundValue, INTEGER, LAMBDA, NODATA, PTR, STRING};

use super::solve::{Solver, Stop};
use super::store::{Kb, Refused};
use super::words::VmBridge;

const UP: &str = "QUERY";

fn code(text: impl std::fmt::Display) -> Error {
    Error(format!("{UP}: {text}"))
}

/// `query` — a CONDITIONAL to fill with `set` and run with `!`.
fn query_word(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(crate::conditional::new_conditional("query"));
    Ok(())
}

/// A count slot: missing, or an INT ≥ 0 (E08).
fn count(c: &BundValue, slot: &str) -> Result<Option<usize>, Error> {
    let Some(v) = c.get(slot) else {
        return Ok(None);
    };
    if v.dt() != INTEGER {
        return Err(code(format!(
            "{slot} must be a non-negative INT, found {}",
            v.type_name()
        )));
    }
    v.as_int()
        .and_then(|n| usize::try_from(n).ok())
        .map(Some)
        .ok_or_else(|| {
            code(format!(
                "{slot} must be a non-negative INT, found {}",
                v.summary(super::term::SHOWN)
            ))
        })
}

/// Run a `query` conditional: each answer pushed, then `lambda` run on it.
///
/// Refuses a slot of the wrong kind, or a negative `limit` or `skip_first`
/// (E08), a missing `kb` (E01) or `goal` (E13), an error in `lambda` (E25),
/// and otherwise what the asking words refuse. `prove true` is E14 until
/// Phase 2 builds S15 (D227); `ask` and `known` matter only to an askable
/// predicate, which is E14 until Phase 2 is built.
pub fn run_query(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    let kb = match c.get("kb") {
        Some(v) => Kb::read(&v, 0).map_err(|_| code(Refused::NoKbSlot.text()))?,
        None => return Err(code(Refused::NoKbSlot.text())),
    };
    let goal = match c.get("goal") {
        Some(g) if g.dt() != NODATA => g,
        _ => return Err(code("the query has no goal")),
    };
    let lambda = match c.get("lambda") {
        None => BundValue::lambda(Vec::new()),
        Some(l) if l.dt() == LAMBDA => l,
        Some(l) => {
            return Err(code(format!(
                "lambda must be LAMBDA, found {}",
                l.type_name()
            )));
        }
    };
    // Phase 2's asking slots: unread until an askable predicate can be
    // called, but a wrong kind is E08 now, as S11 says of every slot.
    if let Some(a) = c.get("ask").filter(|a| a.dt() != LAMBDA) {
        return Err(code(format!("ask must be LAMBDA, found {}", a.type_name())));
    }
    if let Some(k) = c.get("known").filter(|k| !matches!(k.dt(), STRING | PTR)) {
        return Err(code(format!(
            "known must be STRING, found {}",
            k.type_name()
        )));
    }
    let limit = count(&c, "limit")?;
    let skip_first = count(&c, "skip_first")?.unwrap_or(0);
    match c.get("prove") {
        None => {}
        Some(p) if p.dt() == BOOL => {
            if matches!(p.unboxed(), BundValue::Bool(true, _)) {
                return Err(code("prove belongs to Phase 2, which is not built"));
            }
        }
        Some(p) => {
            return Err(code(format!("prove must be BOOL, found {}", p.type_name())));
        }
    }
    if limit == Some(0) {
        return Ok(());
    }
    let stop = |s: Stop| match s {
        Stop::Code(text) => code(text),
        Stop::Raw(e) => e,
    };
    let mut solver = Solver::new(&kb, &goal).map_err(stop)?;
    let mut seen = 0usize;
    let mut run = 0usize;
    loop {
        let answer = {
            let mut bridge = VmBridge::new(vm, UP);
            solver.next(&mut bridge).map_err(stop)?
        };
        let Some(answer) = answer else {
            return Ok(());
        };
        seen += 1;
        if seen <= skip_first {
            continue;
        }
        vm.push(answer);
        vm.eval_lambda(&lambda)
            .map_err(|e| code(format!("lambda returns: {}", e.0)))?;
        run += 1;
        if limit.is_some_and(|l| run >= l) {
            return Ok(());
        }
    }
}

pub fn register(r: &mut Registry) {
    // D181: it leaves a CONDITIONAL of the type its runner is filed under.
    r.register_native(
        "query",
        query_word,
        StackEffect::fixed(0, 1).leaving(Leaves::Conditional("query")),
        WordKind::Sync,
    );
    r.register_conditional("query", run_query);
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    fn interp() -> Interp {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i
    }

    fn run_in(i: &mut Interp, src: &str) -> Result<(), String> {
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| {
            e.0.rsplit_once(" returned error: ")
                .map_or(e.0.clone(), |(_, m)| m.to_string())
        })
    }

    fn stack(src: &str) -> Vec<String> {
        let mut i = interp();
        run_in(&mut i, src).unwrap_or_else(|e| panic!("{src} failed: {e}"));
        i.snapshot().iter().map(BundValue::display).collect()
    }

    fn top(src: &str) -> BundValue {
        let mut i = interp();
        run_in(&mut i, src).unwrap_or_else(|e| panic!("{src} failed: {e}"));
        i.peek().unwrap_or_else(|| panic!("{src}: empty"))
    }

    fn err(src: &str) -> String {
        match run_in(&mut interp(), src) {
            Ok(()) => panic!("{src} was expected to fail"),
            Err(e) => e,
        }
    }

    /// How a LIST of LISTs of names displays.
    fn rows(rows: &[&[&str]]) -> String {
        BundValue::list(
            rows.iter()
                .map(|r| BundValue::list(r.iter().map(|s| BundValue::str(*s)).collect()))
                .collect(),
        )
        .display()
    }

    const FAMILY: &str = ":family [ [ parent tom bob ] [ parent bob ann ] [ parent bob pat ] \
                          [ [ grandparent X Z ] ← [ parent X Y ] [ parent Y Z ] ] ] kb! var";

    /// The first worked example's `query`: `lambda` runs once per answer,
    /// with the answer pushed.
    #[test]
    fn query_runs_its_lambda_once_per_answer() {
        let got = stack(&format!(
            "{FAMILY} query :kb :family var? set :goal [ grandparent G :pat ] set \
             :lambda {{ :G get }} set !"
        ));
        assert_eq!(got, ["tom"]);
        let got = stack(&format!(
            "{FAMILY} query :kb :family var? set :goal [ parent P C ] set \
             :lambda {{ :C get }} set !"
        ));
        assert_eq!(got, ["bob", "ann", "pat"]);
    }

    /// **Criterion 1d**: `query`'s slots.
    #[test]
    fn query_reads_its_slots() {
        let q = |slots: &str| {
            format!(
                "{FAMILY} query :kb :family var? set :goal [ parent P C ] set \
                 :lambda {{ :C get }} set {slots} !"
            )
        };
        assert_eq!(stack(&q(":limit 2 set")), ["bob", "ann"]);
        assert_eq!(stack(&q(":skip_first 1 set")), ["ann", "pat"]);
        assert_eq!(stack(&q(":skip_first 1 set :limit 1 set")), ["ann"]);
        assert!(stack(&q(":limit 0 set")).is_empty(), "0 asks for none");
        assert!(stack(&q(":skip_first 9 set")).is_empty());
        // No lambda: each answer is pushed and left.
        assert_eq!(
            stack(&format!(
                "{FAMILY} query :kb :family var? set :goal [ parent tom C ] set !"
            ))
            .len(),
            1
        );
        assert!(stack(&q(":prove false set")).len() == 3);
    }

    #[test]
    fn query_refuses_a_slot_of_the_wrong_kind() {
        assert_eq!(
            err("query :goal [ p ] set !"),
            "QUERY: kb must be a knowledge base"
        );
        assert_eq!(
            err("query :kb dict set :goal [ p ] set !"),
            "QUERY: kb must be a knowledge base"
        );
        assert_eq!(
            err("query :kb kb.new set !"),
            "QUERY: the query has no goal"
        );
        assert_eq!(
            err("query :kb kb.new set :goal [ p ] set :lambda 1 set !"),
            "QUERY: lambda must be LAMBDA, found Integer"
        );
        assert_eq!(
            err("query :kb kb.new set :goal [ p ] set :limit -1 set !"),
            "QUERY: limit must be a non-negative INT, found -1"
        );
        assert_eq!(
            err("query :kb kb.new set :goal [ p ] set :skip_first \"x\" set !"),
            "QUERY: skip_first must be a non-negative INT, found String"
        );
        assert_eq!(
            err("query :kb kb.new set :goal [ p ] set :prove 1 set !"),
            "QUERY: prove must be BOOL, found Integer"
        );
        assert_eq!(
            err("query :kb kb.new set :goal [ p ] set :prove true set !"),
            "QUERY: prove belongs to Phase 2, which is not built"
        );
        assert_eq!(
            err("query :kb kb.new set :goal [ p ] set :ask 1 set !"),
            "QUERY: ask must be LAMBDA, found Integer"
        );
        assert_eq!(
            err("query :kb kb.new set :goal [ p ] set :known 1 set !"),
            "QUERY: known must be STRING, found Integer"
        );
        // Of the right kind, both are accepted and unread until Phase 2.
        assert_eq!(
            stack(&format!(
                "{FAMILY} query :kb :family var? set :goal [ parent P C ] set \
                 :lambda {{ :C get }} set :ask {{ true }} set :known :replies set !"
            )),
            ["bob", "ann", "pat"]
        );
        // The asking words' codes, prefixed `QUERY: `.
        assert!(
            err("query :kb kb.new set :goal [ p ] set !")
                .starts_with("QUERY: existence error: unknown predicate p/0")
        );
    }

    /// **Criterion 1d**: E25, and `?try` catches it.
    #[test]
    fn e25_is_an_error_in_lambda() {
        // The whole text: `err` would cut it at the inner error's own
        // "returned error: ".
        let src = format!(
            "{FAMILY} query :kb :family var? set :goal [ parent P C ] set \
             :lambda {{ no_such_word_here }} set !"
        );
        let stream = bund2_syntax::compile(&src).unwrap();
        let e = interp().eval(&stream).unwrap_err().0;
        assert!(e.contains("QUERY: lambda returns: "), "{e}");
        assert!(e.contains("no_such_word_here"), "{e}");
        let caught = top(&format!(
            "?try :try {{ {FAMILY} query :kb :family var? set :goal [ parent P C ] set \
             :lambda {{ no_such_word_here }} set ! }} set :except {{ \"caught\" }} set !"
        ));
        assert_eq!(caught.as_str().as_deref(), Some("caught"));
    }

    /// `kb.call`: each answer the instantiated arguments (L5), as the RFC's
    /// own example has it.
    #[test]
    fn kb_call_answers_the_instantiated_arguments() {
        assert_eq!(
            top(&format!(
                "{FAMILY} :tom ∅ :family var? :grandparent 2 kb.call"
            ))
            .display(),
            rows(&[&["tom", "ann"], &["tom", "pat"]])
        );
        // Arity 0: one answer, the empty argument LIST.
        assert_eq!(
            top("[ ready ] kb! :ready 0 kb.call").display(),
            BundValue::list(vec![BundValue::list(Vec::new())]).display()
        );
    }

    /// **Criterion 1d**: E09, for `kb.call` and `kb.word`.
    #[test]
    fn e09_refuses_a_functor_or_arity_of_the_wrong_kind() {
        for src in [
            "kb.new 1 2 kb.call",
            "kb.new :p -1 kb.call",
            "kb.new :p :x kb.call",
        ] {
            assert_eq!(
                err(src),
                "KB.CALL: the functor must be a STRING and the arity a non-negative INT",
                "{src}"
            );
        }
        assert_eq!(
            err(":family 1 2 kb.word"),
            "KB.WORD: the functor must be a STRING and the arity a non-negative INT"
        );
        assert_eq!(
            err("kb.new :p kb.call"),
            "Stack is too shallow for inline kb.call"
        );
        assert_eq!(
            err(":a :b kb.word"),
            "Stack is too shallow for inline kb.word"
        );
        // Too few arguments for the arity.
        assert_eq!(
            err("kb.new :p 2 kb.call"),
            "Stack is too shallow for inline kb.call"
        );
        assert_eq!(
            err("dict :p 0 kb.call"),
            "KB.CALL: #3 parameter must be a knowledge base"
        );
    }

    #[test]
    fn kb_word_refuses_a_reserved_name_and_a_var_that_is_not_a_name() {
        assert_eq!(
            err(":family :findall 3 kb.word"),
            "KB.WORD: findall/3 is reserved and cannot be defined"
        );
        assert_eq!(
            err("1 :g 2 kb.word"),
            "KB.WORD: #3 parameter must be STRING, found Integer"
        );
    }

    /// **Criterion 10.** `:tom ∅ grandparent` answers a LIST containing
    /// `[ "tom" "ann" ]`, as `kb.call` does; the word follows a rebinding of
    /// `family`; `∅` and `|` both mark an unbound argument.
    #[test]
    fn kb_word_calls_its_predicate_and_reads_the_kb_at_call_time() {
        let both = rows(&[&["tom", "ann"], &["tom", "pat"]]);
        let src = format!("{FAMILY} :family :grandparent 2 kb.word");
        assert_eq!(top(&format!("{src} :tom ∅ grandparent")).display(), both);
        assert_eq!(top(&format!("{src} :tom | grandparent")).display(), both);
        assert_eq!(
            top(&format!(
                "{src} :family [ [ parent tom zed ] [ parent zed kim ] \
                 [ [ grandparent X Z ] ← [ parent X Y ] [ parent Y Z ] ] ] kb! var \
                 :tom ∅ grandparent"
            ))
            .display(),
            rows(&[&["tom", "kim"]])
        );
        // The fourth worked example.
        let ops = ":ops [ [ depends web api ] [ depends api db ] [ depends api cache ] \
                   [ down cache ] [ [ needs S D ] ← [ depends S D ] ] \
                   [ [ needs S D ] ← [ depends S M ] [ needs M D ] ] \
                   [ [ impaired S D ] ← [ needs S D ] [ down D ] ] ] kb! var \
                   :ops :impaired 2 kb.word :web ∅ impaired";
        assert_eq!(top(ops).display(), rows(&[&["web", "cache"]]));
    }

    /// The interned-name table's size, measured by interning a fresh probe.
    fn names_now() -> usize {
        let sym = bund2_value::intern_stack_name(&bund2_value::fresh_name());
        format!("{sym:?}")
            .trim_start_matches("StackSym(")
            .trim_end_matches(')')
            .parse()
            .unwrap()
    }

    /// **Criterion 8b.** 100,000 calls of a `kb.word` word whose predicate
    /// runs a bridge grow the name table by at most the nesting depth, 1.
    #[test]
    fn a_kb_word_called_many_times_does_not_grow_the_name_table() {
        let mut i = interp();
        run_in(
            &mut i,
            ":k [ [ [ ok X ] ← [ test { true } ] ] ] kb! var :k :ok 1 kb.word",
        )
        .unwrap();
        let before = names_now();
        run_in(&mut i, "100000 { drop :a ok drop } times").unwrap();
        let grew = names_now() - before - 1;
        assert!(grew <= 1, "the name table grew by {grew}");
    }

    fn check(src: &str) -> crate::check::Report {
        let i = interp();
        let stream = bund2_syntax::compile(src).unwrap();
        crate::check::check(&stream, &i.registry, 0)
    }

    /// **Criterion 13.** `bund2 check` reports underflow for `kb.query` with
    /// one operand, follows a `query` whose `lambda` consumes its MAP, and
    /// follows a call to a `kb.word` word made from literals.
    #[test]
    fn check_follows_the_family() {
        let r = check("[ p ] kb.query");
        assert_eq!(r.findings.len(), 1, "{r:?}");
        assert_eq!(r.findings[0].word, "kb.query");

        let r = check("query :kb kb.new set :goal [ p ] set :lambda { drop } set ! 1 2 +");
        assert!(r.findings.is_empty(), "{r:?}");
        assert_eq!(r.abandoned, 0, "{r:?}");
        let r = check("query :kb kb.new set :goal [ p ] set :lambda { drop drop } set !");
        assert_eq!(r.findings.len(), 1, "the second drop: {r:?}");

        let r = check(":family :grandparent 2 kb.word :tom ∅ grandparent drop");
        assert!(r.findings.is_empty(), "{r:?}");
        assert_eq!(r.abandoned, 0, "{r:?}");
        // One argument short: the finding is placed at the call site.
        let r = check(":family :grandparent 2 kb.word :tom grandparent");
        assert_eq!(r.findings.len(), 1, "{r:?}");
        assert_eq!(r.findings[0].word, "grandparent");

        // The asking words' arms, and `kb.call` with a literal arity.
        let r = check("kb.new [ p ] kb.holds kb.new [ p ] kb.once kb.new [ p ] kb.query");
        assert!(r.findings.is_empty() && r.abandoned == 0, "{r:?}");
        let r = check(":a :b kb.new :p 2 kb.call drop");
        assert!(r.findings.is_empty() && r.abandoned == 0, "{r:?}");
    }
}
