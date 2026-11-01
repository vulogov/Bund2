//! Reading CLP(FD) goals (RFC-0013 §S20): bounds, LISTs, options, and
//! expressions into linear forms.
//!
//! **An expression is a linear form**, `Σ aᵢxᵢ + k`, wherever it can be:
//! `+`, `-` and a product with a constant side are folded into it. A product
//! of two variables, `abs`, `min` and `max` each become a fresh variable and
//! a propagator of their own. A coefficient beyond `i64`, or a constant
//! beyond `i128`, cannot be produced, and is E61 (D199, D202).
//!
//! The walk keeps its own stack, as every walk over a term does (D39).
//!
//! Nothing here uses the `Vm`.

use super::dom::{Bound, Dom, INF, SUP};
use super::{Fd, Prop, Rel};
use crate::kb::term::{Arena, Cell, Term};

/// Why a constraint goal was refused; `solve.rs` writes the text (§S12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bad {
    /// E60: not an integer expression.
    NotExpr(Term),
    /// E61: a value that cannot be represented.
    TooBig(Term),
    /// E63: not a LIST of variables.
    NotList(Term),
    /// E64: not INT, `inf` or `sup`.
    Bound(Term),
    /// E65: not a labeling option.
    Option(Term),
    /// E66: the lower bound above the upper.
    Above(Term, Term),
}

/// `Σ aᵢxᵢ + k`.
#[derive(Debug, Clone, Default)]
pub struct Lin {
    pub terms: Vec<(Bound, Term)>,
    pub k: Bound,
}

const MIN: Bound = i64::MIN as i128;
const MAX: Bound = i64::MAX as i128;

impl Lin {
    fn constant(k: Bound) -> Lin {
        Lin {
            terms: Vec::new(),
            k,
        }
    }

    fn var(t: Term) -> Lin {
        Lin {
            terms: vec![(1, t)],
            k: 0,
        }
    }

    /// `self + other`; `at` is what E61 names.
    fn plus(mut self, other: Lin, at: Term) -> Result<Lin, Bad> {
        self.terms.extend(other.terms);
        self.k = self.k.checked_add(other.k).ok_or(Bad::TooBig(at))?;
        Ok(self)
    }

    /// `c × self`, each coefficient within `i64`.
    fn scaled(mut self, c: Bound, at: Term) -> Result<Lin, Bad> {
        for (a, _) in &mut self.terms {
            *a = a
                .checked_mul(c)
                .filter(|a| (MIN..=MAX).contains(a))
                .ok_or(Bad::TooBig(at))?;
        }
        self.k = self.k.checked_mul(c).ok_or(Bad::TooBig(at))?;
        Ok(self)
    }
}

/// An INT, `inf` or `sup` (E64).
pub fn bound(arena: &Arena, t: Term) -> Result<Bound, Bad> {
    let t = arena.deref(t);
    match arena.cell(t) {
        Cell::Int(n) => Ok(n.into()),
        Cell::Atom(a) if arena.atoms.text(a) == "inf" => Ok(INF),
        Cell::Atom(a) if arena.atoms.text(a) == "sup" => Ok(SUP),
        _ => Err(Bad::Bound(t)),
    }
}

/// `[lo, hi]` from two bounds (E64, E66).
pub fn interval(arena: &Arena, lo: Term, hi: Term) -> Result<Dom, Bad> {
    let (l, h) = (bound(arena, lo)?, bound(arena, hi)?);
    if l > h {
        return Err(Bad::Above(arena.deref(lo), arena.deref(hi)));
    }
    Ok(Dom::interval(l, h))
}

/// A proper LIST's items (E63).
pub fn items(arena: &Arena, l: Term) -> Result<Vec<Term>, Bad> {
    let l = arena.deref(l);
    if !matches!(arena.cell(l), Cell::Nil | Cell::Cons(..)) {
        return Err(Bad::NotList(l));
    }
    let (items, end) = arena.spine(l);
    match arena.cell(end) {
        Cell::Nil => Ok(items),
        _ => Err(Bad::NotList(l)),
    }
}

/// A LIST of variables and INTs, as `label`, `labeling` and
/// `all_different` take (E63, naming the item that is neither).
pub fn variables(arena: &Arena, l: Term) -> Result<Vec<Term>, Bad> {
    let items = items(arena, l)?;
    for i in &items {
        let i = arena.deref(*i);
        if !matches!(arena.cell(i), Cell::Var(_) | Cell::Int(_)) {
            return Err(Bad::NotList(i));
        }
    }
    Ok(items)
}

/// `fd_dom`'s LIST of `[ lo hi ]` pairs, as one domain: a non-list is
/// E63, a pair that is not two bounds E64, a pair upside down E66.
pub fn intervals(arena: &Arena, l: Term) -> Result<Dom, Bad> {
    let mut parts = Vec::new();
    for pair in items(arena, l)? {
        let pair = arena.deref(pair);
        match items(arena, pair).as_deref() {
            Ok([lo, hi]) => parts.extend_from_slice(interval(arena, *lo, *hi)?.intervals()),
            _ => return Err(Bad::Bound(pair)),
        }
    }
    Ok(Dom::union(parts))
}

/// `labeling`'s options: first-fail and downward (E65).
pub fn options(arena: &Arena, l: Term) -> Result<(bool, bool), Bad> {
    let (mut ff, mut down) = (false, false);
    for o in items(arena, l)? {
        let o = arena.deref(o);
        match arena.cell(o) {
            Cell::Atom(a) if arena.atoms.text(a) == "ff" => ff = true,
            Cell::Atom(a) if arena.atoms.text(a) == "down" => down = true,
            _ => return Err(Bad::Option(o)),
        }
    }
    Ok((ff, down))
}

#[derive(Clone, Copy)]
enum Op {
    Add,
    Sub,
    Mul,
    Neg,
    Abs,
    Min,
    Max,
}

/// A fresh variable for `lin`, unless `lin` is one variable already.
fn var_of(arena: &mut Arena, fd: &mut Fd, mut lin: Lin) -> Term {
    if let ([(1, v)], 0) = (lin.terms.as_slice(), lin.k) {
        return *v;
    }
    let z = arena.var();
    fd.constrain(arena, z);
    lin.terms.push((-1, z));
    fd.post(
        arena,
        Prop::Lin {
            terms: lin.terms,
            k: lin.k,
            rel: Rel::Eq,
        },
    );
    z
}

/// A variable for a propagator's result.
fn fresh(arena: &mut Arena, fd: &mut Fd) -> Term {
    let z = arena.var();
    fd.constrain(arena, z);
    z
}

/// An expression as a linear form, posting what is not linear (E60, E61).
pub fn linear(arena: &mut Arena, fd: &mut Fd, t: Term) -> Result<Lin, Bad> {
    enum Job {
        Visit(Term),
        Apply(Op, Term),
    }
    let mut jobs = vec![Job::Visit(t)];
    let mut done: Vec<Lin> = Vec::new();
    while let Some(job) = jobs.pop() {
        match job {
            Job::Visit(t) => {
                let t = arena.deref(t);
                match arena.cell(t) {
                    Cell::Int(n) => done.push(Lin::constant(n.into())),
                    Cell::Var(_) => done.push(Lin::var(t)),
                    Cell::Struct {
                        functor,
                        start,
                        arity,
                    } => {
                        let op = match (arena.atoms.text(functor), arity) {
                            ("+", 2) => Op::Add,
                            ("-", 2) => Op::Sub,
                            ("*", 2) => Op::Mul,
                            ("-", 1) => Op::Neg,
                            ("abs", 1) => Op::Abs,
                            ("min", 2) => Op::Min,
                            ("max", 2) => Op::Max,
                            _ => return Err(Bad::NotExpr(t)),
                        };
                        jobs.push(Job::Apply(op, t));
                        jobs.extend(
                            arena
                                .args(start, arity)
                                .iter()
                                .rev()
                                .map(|a| Job::Visit(*a)),
                        );
                    }
                    _ => return Err(Bad::NotExpr(t)),
                }
            }
            Job::Apply(op, at) => {
                let unary = matches!(op, Op::Neg | Op::Abs);
                let b = done.pop().unwrap_or_default();
                let a = if unary {
                    Lin::default()
                } else {
                    done.pop().unwrap_or_default()
                };
                let r = match op {
                    Op::Add => a.plus(b, at)?,
                    Op::Sub => a.plus(b.scaled(-1, at)?, at)?,
                    Op::Neg => b.scaled(-1, at)?,
                    Op::Mul if a.terms.is_empty() => b.scaled(a.k, at)?,
                    Op::Mul if b.terms.is_empty() => a.scaled(b.k, at)?,
                    Op::Mul => {
                        let (x, y) = (var_of(arena, fd, a), var_of(arena, fd, b));
                        let z = fresh(arena, fd);
                        fd.post(arena, Prop::Times(x, y, z));
                        Lin::var(z)
                    }
                    Op::Abs if b.terms.is_empty() => {
                        Lin::constant(b.k.checked_abs().ok_or(Bad::TooBig(at))?)
                    }
                    Op::Abs => {
                        let x = var_of(arena, fd, b);
                        let z = fresh(arena, fd);
                        fd.post(arena, Prop::Abs(x, z));
                        Lin::var(z)
                    }
                    Op::Min | Op::Max if a.terms.is_empty() && b.terms.is_empty() => {
                        Lin::constant(if matches!(op, Op::Min) {
                            a.k.min(b.k)
                        } else {
                            a.k.max(b.k)
                        })
                    }
                    Op::Min | Op::Max => {
                        let (x, y) = (var_of(arena, fd, a), var_of(arena, fd, b));
                        let z = fresh(arena, fd);
                        fd.post(
                            arena,
                            if matches!(op, Op::Min) {
                                Prop::Min(x, y, z)
                            } else {
                                Prop::Max(x, y, z)
                            },
                        );
                        Lin::var(z)
                    }
                };
                done.push(r);
            }
        }
    }
    Ok(done.pop().unwrap_or_default())
}

/// Post `[ <rel> a b ]`, read left to right (§S20). `false` when a relation
/// of constants does not hold; otherwise its propagator is queued.
pub fn relation(
    arena: &mut Arena,
    fd: &mut Fd,
    name: &str,
    a: Term,
    b: Term,
    goal: Term,
) -> Result<bool, Bad> {
    let la = linear(arena, fd, a)?;
    let lb = linear(arena, fd, b)?;
    let less = |x: Lin, y: Lin, strict: bool| -> Result<Lin, Bad> {
        let mut d = x.plus(y.scaled(-1, goal)?, goal)?;
        if strict {
            d.k = d.k.checked_add(1).ok_or(Bad::TooBig(goal))?;
        }
        Ok(d)
    };
    let (lin, rel) = match name {
        "#=" => (less(la, lb, false)?, Rel::Eq),
        "#≠" => (less(la, lb, false)?, Rel::Ne),
        "#<=" => (less(la, lb, false)?, Rel::Le),
        "#<" => (less(la, lb, true)?, Rel::Le),
        "#>=" => (less(lb, la, false)?, Rel::Le),
        _ => (less(lb, la, true)?, Rel::Le),
    };
    // Each variable once, an INT folded into the constant.
    let mut terms: Vec<(Bound, Term)> = Vec::new();
    let mut k = lin.k;
    for (c, t) in lin.terms {
        let t = arena.deref(t);
        match arena.cell(t) {
            Cell::Int(n) => {
                k = c
                    .checked_mul(n.into())
                    .and_then(|p| k.checked_add(p))
                    .ok_or(Bad::TooBig(goal))?;
            }
            _ => match terms.iter_mut().find(|(_, u)| *u == t) {
                Some((a, _)) => {
                    *a = a
                        .checked_add(c)
                        .filter(|a| (MIN..=MAX).contains(a))
                        .ok_or(Bad::TooBig(goal))?;
                }
                None => terms.push((c, t)),
            },
        }
    }
    terms.retain(|(c, _)| *c != 0);
    if terms.is_empty() {
        return Ok(match rel {
            Rel::Eq => k == 0,
            Rel::Le => k <= 0,
            Rel::Ne => k != 0,
        });
    }
    fd.post(arena, Prop::Lin { terms, k, rel });
    Ok(true)
}
