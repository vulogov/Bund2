//! Unification, the occurs check, and the trail (RFC-0013 §S4).
//!
//! **Iterative.** Unification works over a worklist of term pairs and the
//! occurs check over a worklist of subterms, so neither recurses on a term's
//! depth (D39, criterion 3).
//!
//! **The occurs check always runs (L7).** Binding a variable to a term that
//! contains it fails, so no unification builds a cycle and every walk over a
//! term ends.
//!
//! Attributes and their wake-ups are Phase 2's (`dif`, S17) and Phase 3's
//! (domains, S21) **[L14]**; nothing in Phase 1 carries one.
//!
//! Nothing here uses the `Vm`.

use bund2_value::BundValue;

use super::term::{Arena, Cell, Term};

/// The variables bound since the activation began, newest last.
#[derive(Debug, Default)]
pub struct Trail {
    bound: Vec<Term>,
}

impl Trail {
    pub fn mark(&self) -> usize {
        self.bound.len()
    }

    /// Unbind every variable bound since `mark`.
    pub fn undo_to(&mut self, arena: &mut Arena, mark: usize) {
        while self.bound.len() > mark {
            if let Some(v) = self.bound.pop() {
                arena.unbind(v);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.bound.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bound.is_empty()
    }

    fn bind(&mut self, arena: &mut Arena, var: Term, to: Term) {
        arena.bind(var, to);
        self.bound.push(var);
    }
}

/// Unify `a` and `b`, with the occurs check.
///
/// **On failure nothing stays bound**: the bindings this call made are undone
/// before it answers, so a caller that fails here owes no unwinding of its
/// own for them.
pub fn unify(arena: &mut Arena, trail: &mut Trail, a: Term, b: Term) -> bool {
    let mark = trail.mark();
    let ok = unify_pairs(arena, trail, a, b);
    if !ok {
        trail.undo_to(arena, mark);
    }
    ok
}

fn unify_pairs(arena: &mut Arena, trail: &mut Trail, a: Term, b: Term) -> bool {
    let mut pairs = vec![(a, b)];
    while let Some((a, b)) = pairs.pop() {
        let (a, b) = (arena.deref(a), arena.deref(b));
        if a == b {
            continue;
        }
        match (arena.cell(a), arena.cell(b)) {
            // **The younger is bound to the older**, so a binding never
            // points from an older cell to one a backtrack would cut away.
            (Cell::Var(_), Cell::Var(_)) => {
                let (young, old) = if a > b { (a, b) } else { (b, a) };
                trail.bind(arena, young, old);
            }
            (Cell::Var(_), _) => {
                if occurs(arena, a, b) {
                    return false;
                }
                trail.bind(arena, a, b);
            }
            (_, Cell::Var(_)) => {
                if occurs(arena, b, a) {
                    return false;
                }
                trail.bind(arena, b, a);
            }
            (Cell::Atom(x), Cell::Atom(y)) => {
                if x != y {
                    return false;
                }
            }
            (Cell::Int(x), Cell::Int(y)) => {
                if x != y {
                    return false;
                }
            }
            // Key equality, which is `BundValue`'s own (§S1, L2).
            (Cell::Float(x), Cell::Float(y)) => {
                if BundValue::float(x) != BundValue::float(y) {
                    return false;
                }
            }
            (Cell::Nil, Cell::Nil) => {}
            (Cell::Cons(h1, t1), Cell::Cons(h2, t2)) => {
                pairs.push((t1, t2));
                pairs.push((h1, h2));
            }
            (
                Cell::Struct {
                    functor: f1,
                    start: s1,
                    arity: n1,
                },
                Cell::Struct {
                    functor: f2,
                    start: s2,
                    arity: n2,
                },
            ) => {
                if f1 != f2 || n1 != n2 {
                    return false;
                }
                let left = arena.args(s1, n1);
                let right = arena.args(s2, n2);
                pairs.extend(left.iter().copied().zip(right.iter().copied()).rev());
            }
            // Key equality: identity for a MAP, a LAMBDA or an OBJECT (L13).
            (Cell::Opaque(x), Cell::Opaque(y)) => {
                if arena.opaque_value(x) != arena.opaque_value(y) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

/// Whether `var` occurs in `t`.
pub fn occurs(arena: &Arena, var: Term, t: Term) -> bool {
    let mut work = vec![t];
    while let Some(t) = work.pop() {
        let t = arena.deref(t);
        if t == var {
            return true;
        }
        match arena.cell(t) {
            Cell::Cons(h, tail) => {
                work.push(tail);
                work.push(h);
            }
            Cell::Struct { start, arity, .. } => {
                work.extend_from_slice(arena.args(start, arity));
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::term::Scope;

    fn c(s: &str) -> BundValue {
        BundValue::call(s)
    }
    fn s(x: &str) -> BundValue {
        BundValue::str(x)
    }
    fn l(items: Vec<BundValue>) -> BundValue {
        BundValue::list(items)
    }
    fn f(args: Vec<BundValue>) -> BundValue {
        let mut items = vec![c("f")];
        items.extend(args);
        l(items)
    }

    /// A term written out, its variables numbered by first appearance, so
    /// two runs compare however their cells were numbered.
    fn show(arena: &Arena, t: Term, vars: &mut Vec<Term>) -> String {
        let t = arena.deref(t);
        match arena.cell(t) {
            Cell::Var(_) => {
                let n = vars.iter().position(|v| *v == t).unwrap_or_else(|| {
                    vars.push(t);
                    vars.len() - 1
                });
                format!("_{n}")
            }
            Cell::Atom(a) => arena.atoms.text(a).to_string(),
            Cell::Int(i) => i.to_string(),
            // As a key: `-0.0` is `0.0`.
            Cell::Float(x) => format!("{:?}", if x == 0.0 { 0.0 } else { x }),
            Cell::Nil => "[]".into(),
            Cell::Cons(h, tail) => {
                format!("[{}|{}]", show(arena, h, vars), show(arena, tail, vars))
            }
            Cell::Struct {
                functor,
                start,
                arity,
            } => {
                let args: Vec<String> = arena
                    .args(start, arity)
                    .iter()
                    .map(|a| show(arena, *a, vars))
                    .collect();
                format!("{}({})", arena.atoms.text(functor), args.join(","))
            }
            // By identity, which is what an opaque value compares by.
            Cell::Opaque(i) => format!("#{:?}", arena.opaque_value(i).map(|v| v.identity().0)),
        }
    }

    /// Every variable reachable from `t` once bindings are followed. In an
    /// idempotent substitution none of them is bound.
    fn reachable_vars(arena: &Arena, t: Term) -> Vec<Term> {
        let mut out = Vec::new();
        let mut work = vec![t];
        while let Some(t) = work.pop() {
            let t = arena.deref(t);
            match arena.cell(t) {
                Cell::Var(_) => out.push(t),
                Cell::Cons(h, tail) => work.extend([h, tail]),
                Cell::Struct { start, arity, .. } => {
                    work.extend_from_slice(arena.args(start, arity));
                }
                _ => {}
            }
        }
        out
    }

    struct Outcome {
        unified: bool,
        /// Both sides written out after unifying, when it succeeded.
        shown: Option<(String, String)>,
    }

    /// Read `a` and then `b` in one scope, so they share their variables,
    /// and unify them in the order `swap` says.
    fn run(a: &BundValue, b: &BundValue, swap: bool) -> Outcome {
        let mut arena = Arena::default();
        let mut trail = Trail::default();
        let mut scope = Scope::default();
        let ta = arena.to_term(a, &mut scope).unwrap();
        let tb = arena.to_term(b, &mut scope).unwrap();
        let before = (show(&arena, ta, &mut vec![]), show(&arena, tb, &mut vec![]));
        let unified = if swap {
            unify(&mut arena, &mut trail, tb, ta)
        } else {
            unify(&mut arena, &mut trail, ta, tb)
        };
        if !unified {
            // Nothing stays bound after a failure.
            assert_eq!(trail.len(), 0);
            let after = (show(&arena, ta, &mut vec![]), show(&arena, tb, &mut vec![]));
            assert_eq!(before, after);
            return Outcome {
                unified,
                shown: None,
            };
        }
        // Idempotent: what a variable is bound to holds no bound variable.
        for (name, v) in scope.names() {
            for r in reachable_vars(&arena, *v) {
                assert!(
                    matches!(arena.cell(r), Cell::Var(None)),
                    "{name} reaches a bound variable"
                );
            }
        }
        let mut vars = Vec::new();
        let sa = show(&arena, ta, &mut vars);
        let sb = show(&arena, tb, &mut vars);
        Outcome {
            unified,
            shown: Some((sa, sb)),
        }
    }

    fn check(a: &BundValue, b: &BundValue, want: bool) {
        let ab = run(a, b, false);
        let ba = run(a, b, true);
        let what = format!("{} = {}", a.summary(60), b.summary(60));
        assert_eq!(ab.unified, want, "{what}");
        // Symmetric: the same answer, and the same instance, either way.
        assert_eq!(ba.unified, want, "{what}, swapped");
        assert_eq!(ab.shown, ba.shown, "{what}");
        // A unifier makes the two sides one term.
        if let Some((x, y)) = ab.shown {
            assert_eq!(x, y, "{what}");
        }
    }

    /// **Criterion 2a.** Over an enumerated table of term pairs, `unify` is
    /// symmetric and its substitution idempotent, and `[ = X [ f X ] ]` fails.
    /// Each §S1 term kind meets each other; then shared and repeated
    /// variables, nested compounds and partial lists.
    #[test]
    fn unify_is_symmetric_and_idempotent_over_the_table() {
        let map = BundValue::map(Default::default());
        let kinds = [
            c("a"),
            BundValue::int(1),
            BundValue::float(1.0),
            c("nil"),
            l(vec![s("a"), s("b")]),
            l(vec![s("a"), c("|"), c("b")]),
            f(vec![BundValue::int(1)]),
            map.clone(),
            BundValue::map(Default::default()),
        ];
        // Each kind against each other: equal only to itself. INT 1 and FLOAT
        // 1.0 differ (L2); two MAPs alike in content differ (L13).
        for (i, x) in kinds.iter().enumerate() {
            for (j, y) in kinds.iter().enumerate() {
                check(x, y, i == j);
            }
            // A variable takes any of them.
            check(&c("X"), x, true);
        }

        let rows: Vec<(BundValue, BundValue, bool)> = vec![
            // Atoms by text, whatever spelt them (L2).
            (c("tom"), s("tom"), true),
            (s("true"), BundValue::boolean(true), true),
            (c("tom"), c("bob"), false),
            // Floats by key equality.
            (BundValue::float(f64::NAN), BundValue::float(f64::NAN), true),
            (BundValue::float(-0.0), BundValue::float(0.0), true),
            (
                f(vec![BundValue::int(1)]),
                f(vec![BundValue::float(1.0)]),
                false,
            ),
            // Functor and arity.
            (f(vec![c("a")]), f(vec![c("a"), c("b")]), false),
            (f(vec![c("a")]), l(vec![c("g"), c("a")]), false),
            // A compound of no arguments is its atom (L4).
            (l(vec![c("ready")]), c("ready"), true),
            // Shared variables.
            (f(vec![c("X"), c("b")]), f(vec![c("a"), c("Y")]), true),
            (f(vec![c("X"), c("Y")]), f(vec![c("Y"), c("X")]), true),
            (f(vec![c("X"), c("Y")]), f(vec![c("Y"), c("a")]), true),
            // Repeated variables.
            (f(vec![c("X"), c("X")]), f(vec![c("a"), c("b")]), false),
            (f(vec![c("X"), c("X")]), f(vec![c("Y"), c("a")]), true),
            (f(vec![c("_"), c("_")]), f(vec![c("a"), c("b")]), true),
            // Nested compounds.
            (
                l(vec![
                    c("g"),
                    f(vec![c("X")]),
                    l(vec![c("h"), c("Y"), l(vec![c("X")])]),
                ]),
                l(vec![
                    c("g"),
                    f(vec![c("a")]),
                    l(vec![c("h"), c("b"), c("Z")]),
                ]),
                true,
            ),
            (
                l(vec![c("g"), f(vec![c("X")]), c("X")]),
                l(vec![c("g"), f(vec![c("a")]), c("b")]),
                false,
            ),
            // Partial lists.
            (
                l(vec![s("a"), c("|"), c("T")]),
                l(vec![s("a"), s("b"), s("c")]),
                true,
            ),
            (l(vec![c("X"), c("|"), c("T")]), c("nil"), false),
            (
                l(vec![s("a"), c("|"), c("T")]),
                l(vec![s("a"), c("|"), c("U")]),
                true,
            ),
            (
                l(vec![s("a"), c("|"), s("b")]),
                l(vec![s("a"), c("|"), c("T")]),
                true,
            ),
            (
                l(vec![c("X"), c("Y"), c("|"), c("T")]),
                l(vec![s("a")]),
                false,
            ),
            // The occurs check (L7): `[ = X [ f X ] ]` fails, as does a list
            // that holds its own tail.
            (c("X"), f(vec![c("X")]), false),
            (c("X"), l(vec![s("a"), c("|"), c("X")]), false),
            (
                f(vec![c("X"), c("Y")]),
                f(vec![c("Y"), f(vec![c("X")])]),
                false,
            ),
        ];
        for (a, b, want) in &rows {
            check(a, b, *want);
        }
    }

    /// **L7, as `=` sees it:** after `[ = X [ f X ] ]` fails, X is unbound.
    #[test]
    fn the_occurs_check_leaves_nothing_bound() {
        let mut arena = Arena::default();
        let mut trail = Trail::default();
        let mut scope = Scope::default();
        let x = arena.to_term(&c("X"), &mut scope).unwrap();
        let fx = arena.to_term(&f(vec![c("X")]), &mut scope).unwrap();
        assert!(occurs(&arena, x, fx));
        assert!(!unify(&mut arena, &mut trail, x, fx));
        assert!(matches!(arena.cell(x), Cell::Var(None)));
        assert!(trail.is_empty());
    }

    /// The younger variable is bound to the older, and the trail undoes it.
    #[test]
    fn the_younger_variable_is_bound_and_the_trail_unbinds_it() {
        let mut arena = Arena::default();
        let mut trail = Trail::default();
        let old = arena.var();
        let young = arena.var();
        let mark = trail.mark();
        assert!(unify(&mut arena, &mut trail, old, young));
        assert!(matches!(arena.cell(old), Cell::Var(None)));
        assert!(matches!(arena.cell(young), Cell::Var(Some(t)) if t == old));
        trail.undo_to(&mut arena, mark);
        assert!(matches!(arena.cell(young), Cell::Var(None)));
    }

    /// **Criterion 3, unification.** Two terms 100,000 deep unify, and the
    /// occurs check walks one, in a debug build: neither recurses on depth.
    #[test]
    fn terms_100000_deep_unify_and_pass_the_occurs_check() {
        const DEPTH: usize = 100_000;
        let deep = |leaf: BundValue| {
            let mut v = leaf;
            for _ in 0..DEPTH {
                v = f(vec![v]);
            }
            v
        };
        let deep_list = |leaf: BundValue| {
            let mut v = leaf;
            for _ in 0..DEPTH {
                v = l(vec![s("a"), c("|"), v]);
            }
            v
        };
        let mut arena = Arena::default();
        let mut trail = Trail::default();
        let mut scope = Scope::default();

        // f(…f(X)…) = f(…f(a)…) binds X to a.
        let with_x = arena.to_term(&deep(c("X")), &mut scope).unwrap();
        let with_a = arena.to_term(&deep(c("a")), &mut scope).unwrap();
        assert!(unify(&mut arena, &mut trail, with_x, with_a));
        let x = scope.names()[0].1;
        assert_eq!(show(&arena, x, &mut vec![]), "a");

        // Y = f(…f(Y)…) fails, the check having walked all of it.
        let y = arena.to_term(&c("Y"), &mut scope).unwrap();
        let with_y = arena.to_term(&deep(c("Y")), &mut scope).unwrap();
        assert!(occurs(&arena, y, with_y));
        assert!(!unify(&mut arena, &mut trail, y, with_y));

        // Z = f(…f(b)…) passes it, and binds Z.
        let z = arena.to_term(&c("Z"), &mut scope).unwrap();
        let with_b = arena.to_term(&deep(c("b")), &mut scope).unwrap();
        assert!(unify(&mut arena, &mut trail, z, with_b));

        // A list 100,000 long, with a variable tail, against a closed one.
        let open = arena.to_term(&deep_list(c("T")), &mut scope).unwrap();
        let closed = arena.to_term(&deep_list(c("nil")), &mut scope).unwrap();
        assert!(unify(&mut arena, &mut trail, open, closed));
        let mut zz = arena.to_term(&c("W"), &mut scope).unwrap();
        let tail_holding_w = arena.to_term(&deep_list(c("W")), &mut scope).unwrap();
        assert!(!unify(&mut arena, &mut trail, zz, tail_holding_w));
        zz = arena.deref(zz);
        assert!(matches!(arena.cell(zz), Cell::Var(None)));
    }
}
