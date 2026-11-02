//! CLP(FD): finite-domain constraints over integers (RFC-0013 §S20, §S21,
//! M12) **[L10]**.
//!
//! **A domain is an attribute.** [`Fd`] holds each constrained variable's
//! domain and the propagators posted over it, and logs every change, so
//! [`Fd::undo_to`] takes both back to a choicepoint's [`FdMark`] as the trail
//! is unwound. A variable with no entry may be any integer.
//!
//! **Propagation runs from a queue.** A domain that narrows queues the
//! propagators that watch its variable, and `solve.rs` runs the queue to a
//! fixpoint after each unification, one inference per run (§S21). A domain
//! that empties fails the unification; one that becomes a single value binds
//! its variable, and that binding wakes what it wakes.
//!
//! - [`dom`]: domains, and their bounds' arithmetic;
//! - [`post`]: reading the constraint goals, and expressions into linear
//!   forms.
//!
//! Nothing here uses the `Vm`.

pub mod dom;
pub mod post;

use std::collections::HashMap;

use dom::{Acc, Bound, Dom, INF, SUP, div_ceil, div_floor, mul};

use super::term::{Arena, Cell, Term};
use super::unify::{Trail, unify};

/// How a linear form compares with zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rel {
    Eq,
    Le,
    Ne,
}

/// A propagator. Its terms are read through `deref` each run, so one bound
/// since it was posted reads as its value.
#[derive(Debug, Clone)]
pub enum Prop {
    /// `Σ aᵢxᵢ + k  rel  0`, each `aᵢ` within `i64`.
    Lin {
        terms: Vec<(Bound, Term)>,
        k: Bound,
        rel: Rel,
    },
    /// `z = x × y`.
    Times(Term, Term, Term),
    /// `z = |x|`.
    Abs(Term, Term),
    /// `z = min(x, y)`.
    Min(Term, Term, Term),
    /// `z = max(x, y)`.
    Max(Term, Term, Term),
    /// Pairwise different, by value elimination.
    Distinct(Vec<Term>),
}

/// Where the store stood when a choicepoint was pushed.
#[derive(Debug, Clone, Copy)]
pub struct FdMark {
    doms: usize,
    props: usize,
    watch: usize,
}

/// The domains and propagators of an activation.
#[derive(Debug, Default)]
pub struct Fd {
    doms: HashMap<Term, Dom>,
    /// Each domain change: the variable, and what it held before.
    dlog: Vec<(Term, Option<Dom>)>,
    props: Vec<Prop>,
    /// A variable's propagators, and each attachment's variable, newest
    /// last, so it can be undone.
    watch: HashMap<Term, Vec<usize>>,
    wlog: Vec<Term>,
    /// The propagators to run; empty between unifications.
    queue: Vec<usize>,
}

impl Fd {
    /// Whether nothing is constrained: the one test a query with no
    /// constraint pays.
    pub fn idle(&self) -> bool {
        self.doms.is_empty() && self.props.is_empty()
    }

    pub fn mark(&self) -> FdMark {
        FdMark {
            doms: self.dlog.len(),
            props: self.props.len(),
            watch: self.wlog.len(),
        }
    }

    /// Undo every change since `m`, and drop what was queued.
    pub fn undo_to(&mut self, m: FdMark) {
        while self.dlog.len() > m.doms {
            let Some((v, old)) = self.dlog.pop() else {
                break;
            };
            match old {
                Some(d) => self.doms.insert(v, d),
                None => self.doms.remove(&v),
            };
        }
        while self.wlog.len() > m.watch {
            let Some(v) = self.wlog.pop() else { break };
            if let Some(ids) = self.watch.get_mut(&v) {
                ids.pop();
                if ids.is_empty() {
                    self.watch.remove(&v);
                }
            }
        }
        self.props.truncate(m.props);
        self.queue.clear();
    }

    /// The next propagator to run.
    pub fn queued(&mut self) -> Option<usize> {
        self.queue.pop()
    }

    /// Drop what was queued: a unification failed.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    fn set(&mut self, v: Term, d: Dom) {
        let old = self.doms.insert(v, d);
        self.dlog.push((v, old));
    }

    fn enqueue(&mut self, v: Term) {
        if let Some(ids) = self.watch.get(&v) {
            for id in ids {
                if !self.queue.contains(id) {
                    self.queue.push(*id);
                }
            }
        }
    }

    /// `t`'s domain: an INT's own value, a variable's entry or every integer,
    /// and `None` for anything else.
    pub fn dom(&self, arena: &Arena, t: Term) -> Option<Dom> {
        let t = arena.deref(t);
        match arena.cell(t) {
            Cell::Int(n) => Some(Dom::interval(n.into(), n.into())),
            Cell::Var(_) => Some(self.doms.get(&t).cloned().unwrap_or_else(Dom::full)),
            _ => None,
        }
    }

    /// The domain of a variable that has one: what `_residual` reports.
    pub fn held(&self, v: Term) -> Option<&Dom> {
        self.doms.get(&v).filter(|d| !d.is_full())
    }

    /// Make `v`, a fresh or unconstrained variable, constrained with every
    /// integer, so binding it to anything else fails.
    pub fn constrain(&mut self, arena: &Arena, v: Term) {
        let v = arena.deref(v);
        if matches!(arena.cell(v), Cell::Var(_)) && !self.doms.contains_key(&v) {
            self.set(v, Dom::full());
        }
    }

    /// Narrow `t` to `d`: `false` when that empties it, or when `t` is not
    /// an integer. A variable left with one value is bound to it.
    pub fn narrow(&mut self, arena: &mut Arena, trail: &mut Trail, t: Term, d: &Dom) -> bool {
        let t = arena.deref(t);
        let cur = match arena.cell(t) {
            Cell::Int(n) => return d.contains(n.into()),
            Cell::Var(_) => self.doms.get(&t).cloned().unwrap_or_else(Dom::full),
            _ => return false,
        };
        let new = cur.intersect(d);
        if new == cur {
            if !self.doms.contains_key(&t) {
                self.set(t, new);
            }
            return true;
        }
        if new.is_empty() {
            return false;
        }
        let single = new.single();
        self.set(t, new);
        self.enqueue(t);
        if let Some(n) = single {
            let value = arena.int(n);
            return unify(arena, trail, t, value);
        }
        true
    }

    /// Post `p`: watched by each of its variables, which become
    /// constrained, and queued to run.
    pub fn post(&mut self, arena: &Arena, p: Prop) {
        let id = self.props.len();
        let mut vars: Vec<Term> = match &p {
            Prop::Lin { terms, .. } => terms.iter().map(|(_, t)| *t).collect(),
            Prop::Times(x, y, z) | Prop::Min(x, y, z) | Prop::Max(x, y, z) => vec![*x, *y, *z],
            Prop::Abs(x, z) => vec![*x, *z],
            Prop::Distinct(ts) => ts.clone(),
        };
        vars = vars.into_iter().map(|t| arena.deref(t)).collect();
        vars.dedup();
        self.props.push(p);
        for v in vars {
            if !matches!(arena.cell(v), Cell::Var(_)) {
                continue;
            }
            self.constrain(arena, v);
            let ids = self.watch.entry(v).or_default();
            if !ids.contains(&id) {
                ids.push(id);
                self.wlog.push(v);
            }
        }
        self.queue.push(id);
    }

    /// `v` was just bound: check what it was bound to against its domain,
    /// and queue its propagators. Bound to a variable, its domain and its
    /// propagators move there (§S21).
    pub fn bound(&mut self, arena: &mut Arena, trail: &mut Trail, v: Term) -> bool {
        let Some(d) = self.doms.get(&v).cloned() else {
            return true;
        };
        let to = arena.deref(v);
        match arena.cell(to) {
            Cell::Int(n) => {
                if !d.contains(n.into()) {
                    return false;
                }
                self.enqueue(v);
                true
            }
            Cell::Var(_) => {
                let ids = self.watch.get(&v).cloned().unwrap_or_default();
                for id in ids {
                    let theirs = self.watch.entry(to).or_default();
                    if !theirs.contains(&id) {
                        theirs.push(id);
                        self.wlog.push(to);
                    }
                }
                self.enqueue(to);
                self.narrow(arena, trail, to, &d)
            }
            _ => false,
        }
    }

    /// Run propagator `id`: `false` when it empties a domain.
    pub fn run(&mut self, arena: &mut Arena, trail: &mut Trail, id: usize) -> bool {
        let Some(p) = self.props.get(id).cloned() else {
            return true;
        };
        match p {
            Prop::Lin { terms, k, rel } => match rel {
                Rel::Le => self.le(arena, trail, &terms, k),
                Rel::Eq => {
                    let negated: Vec<(Bound, Term)> = terms.iter().map(|(a, t)| (-a, *t)).collect();
                    self.le(arena, trail, &terms, k) && self.le(arena, trail, &negated, -k)
                }
                Rel::Ne => self.ne(arena, trail, &terms, k),
            },
            Prop::Times(x, y, z) => self.times(arena, trail, x, y, z),
            Prop::Abs(x, z) => self.abs(arena, trail, x, z),
            Prop::Min(x, y, z) => self.extreme(arena, trail, x, y, z, false),
            Prop::Max(x, y, z) => self.extreme(arena, trail, x, y, z, true),
            Prop::Distinct(ts) => self.distinct(arena, trail, &ts),
        }
    }

    fn bounds(&self, arena: &Arena, t: Term) -> (Bound, Bound) {
        self.dom(arena, t)
            .map_or((INF, SUP), |d| (d.min(), d.max()))
    }

    /// `Σ aᵢxᵢ + k ≤ 0`, by bounds: each term is at most what the others'
    /// least values leave it.
    fn le(
        &mut self,
        arena: &mut Arena,
        trail: &mut Trail,
        terms: &[(Bound, Term)],
        k: Bound,
    ) -> bool {
        let mins: Vec<Bound> = terms
            .iter()
            .map(|(a, t)| {
                let (lo, hi) = self.bounds(arena, *t);
                if *a > 0 { mul(*a, lo) } else { mul(*a, hi) }
            })
            .collect();
        let unbounded = mins.iter().filter(|m| **m == INF).count();
        let mut acc = Acc::default();
        acc.add(k);
        for m in mins.iter().filter(|m| **m != INF) {
            acc.add(*m);
        }
        if unbounded == 0 && acc.value() > 0 {
            return false;
        }
        for (i, (a, t)) in terms.iter().enumerate() {
            let mine = mins.get(i).copied().unwrap_or(INF);
            let others = match (unbounded, mine == INF) {
                (0, _) => {
                    let mut o = acc;
                    o.sub(mine);
                    o.value()
                }
                (1, true) => acc.value(),
                _ => continue,
            };
            // aᵢxᵢ ≤ −others.
            let room = dom::neg(others);
            let d = if *a > 0 {
                Dom::interval(INF, div_floor(room, *a))
            } else {
                Dom::interval(div_ceil(room, *a), SUP)
            };
            if !self.narrow(arena, trail, *t, &d) {
                return false;
            }
        }
        true
    }

    /// `Σ aᵢxᵢ + k ≠ 0`: checked once every term is fixed, and the one value
    /// removed once all but one are.
    fn ne(
        &mut self,
        arena: &mut Arena,
        trail: &mut Trail,
        terms: &[(Bound, Term)],
        k: Bound,
    ) -> bool {
        let mut acc = Acc::default();
        acc.add(k);
        let mut open = None;
        for (a, t) in terms {
            match self.dom(arena, *t).and_then(|d| d.single()) {
                Some(v) => acc.add(*a * Bound::from(v)),
                None if open.is_none() => open = Some((*a, *t)),
                None => return true,
            }
        }
        let rest = acc.value();
        let Some((a, t)) = open else {
            return rest != 0;
        };
        if rest == INF || rest == SUP || rest % a != 0 {
            return true;
        }
        let v = -rest / a;
        match self.dom(arena, t) {
            Some(d) => {
                let d = d.remove(v);
                self.narrow(arena, trail, t, &d)
            }
            None => false,
        }
    }

    fn times(&mut self, arena: &mut Arena, trail: &mut Trail, x: Term, y: Term, z: Term) -> bool {
        let (xl, xh) = self.bounds(arena, x);
        let (yl, yh) = self.bounds(arena, y);
        let corners = [mul(xl, yl), mul(xl, yh), mul(xh, yl), mul(xh, yh)];
        let lo = corners.iter().copied().min().unwrap_or(INF);
        let hi = corners.iter().copied().max().unwrap_or(SUP);
        if !self.narrow(arena, trail, z, &Dom::interval(lo, hi)) {
            return false;
        }
        // A fixed, non-zero factor divides the product's bounds.
        for (a, b) in [(x, y), (y, x)] {
            let fixed = self.dom(arena, b).and_then(|d| d.single());
            let Some(v) = fixed.filter(|v| *v != 0).map(Bound::from) else {
                continue;
            };
            let (zl, zh) = self.bounds(arena, z);
            let d = if v > 0 {
                Dom::interval(div_ceil(zl, v), div_floor(zh, v))
            } else {
                Dom::interval(div_ceil(zh, v), div_floor(zl, v))
            };
            if !self.narrow(arena, trail, a, &d) {
                return false;
            }
        }
        true
    }

    fn abs(&mut self, arena: &mut Arena, trail: &mut Trail, x: Term, z: Term) -> bool {
        let (xl, xh) = self.bounds(arena, x);
        let d = if xl >= 0 {
            Dom::interval(xl, xh)
        } else if xh <= 0 {
            Dom::interval(dom::neg(xh), dom::neg(xl))
        } else {
            Dom::interval(0, dom::neg(xl).max(xh))
        };
        if !self.narrow(arena, trail, z, &d) {
            return false;
        }
        let (zl, zh) = self.bounds(arena, z);
        let (xl, xh) = self.bounds(arena, x);
        let d = if xl >= 0 {
            Dom::interval(zl, zh)
        } else if xh <= 0 {
            Dom::interval(dom::neg(zh), dom::neg(zl))
        } else {
            Dom::interval(dom::neg(zh), zh)
        };
        self.narrow(arena, trail, x, &d)
    }

    /// `z = max(x, y)` when `max`, else `z = min(x, y)`.
    fn extreme(
        &mut self,
        arena: &mut Arena,
        trail: &mut Trail,
        x: Term,
        y: Term,
        z: Term,
        max: bool,
    ) -> bool {
        let (xl, xh) = self.bounds(arena, x);
        let (yl, yh) = self.bounds(arena, y);
        let d = if max {
            Dom::interval(xl.max(yl), xh.max(yh))
        } else {
            Dom::interval(xl.min(yl), xh.min(yh))
        };
        if !self.narrow(arena, trail, z, &d) {
            return false;
        }
        // Neither argument passes z's far bound: below it for max, above it
        // for min.
        let (zl, zh) = self.bounds(arena, z);
        let d = if max {
            Dom::interval(INF, zh)
        } else {
            Dom::interval(zl, SUP)
        };
        self.narrow(arena, trail, x, &d) && self.narrow(arena, trail, y, &d)
    }

    fn distinct(&mut self, arena: &mut Arena, trail: &mut Trail, ts: &[Term]) -> bool {
        for (i, t) in ts.iter().enumerate() {
            let Some(v) = self.dom(arena, *t).and_then(|d| d.single()) else {
                continue;
            };
            for (j, u) in ts.iter().enumerate() {
                if i == j {
                    continue;
                }
                let Some(d) = self.dom(arena, *u) else {
                    return false;
                };
                let d = d.remove(v.into());
                if !self.narrow(arena, trail, *u, &d) {
                    return false;
                }
            }
        }
        true
    }
}

/// **Criteria 22–24**: domains, puzzles and edges (§S20, §S21).
#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::{BundValue, CALL, LIST, MAP, NODATA, PTR, STRING};

    fn run(src: &str) -> Result<BundValue, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| {
            e.0.rsplit_once(" returned error: ")
                .map_or(e.0.clone(), |(_, m)| m.to_string())
        })?;
        i.peek().ok_or_else(|| format!("{src}: empty"))
    }

    fn top(src: &str) -> BundValue {
        run(src).unwrap_or_else(|e| panic!("{src} failed: {e}"))
    }

    fn err(src: &str) -> String {
        match run(src) {
            Ok(v) => panic!("{src} was expected to fail, and left {}", v.display()),
            Err(e) => e,
        }
    }

    /// A value as the tests write it: a PTR as `*name`, a STRING as `:text`,
    /// NODATA as `_`, a LIST in brackets and a MAP in braces.
    fn shape(v: &BundValue) -> String {
        match v.dt() {
            LIST => {
                let items: Vec<String> = v.as_list().unwrap_or(&[]).iter().map(shape).collect();
                format!("[ {} ]", items.join(" "))
            }
            MAP => {
                let kv: Vec<String> = v
                    .as_map()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| format!("{k}={}", shape(v)))
                    .collect();
                format!("{{{}}}", kv.join(", "))
            }
            CALL => v.as_str().unwrap(),
            PTR => format!("*{}", v.as_str().unwrap()),
            STRING => format!(":{}", v.as_str().unwrap()),
            NODATA => "_".into(),
            _ => v.display(),
        }
    }

    /// Every answer of `goal` over an empty KB, shaped.
    fn answers(goal: &str) -> Vec<String> {
        answers_in("kb.new", goal)
    }

    fn answers_in(kb: &str, goal: &str) -> Vec<String> {
        let list = top(&format!("{kb} {goal} kb.query"));
        list.as_list()
            .unwrap_or_else(|| panic!("{goal}: not a LIST"))
            .iter()
            .map(shape)
            .collect()
    }

    /// **Criterion 22.** A domain fails without labeling, narrows into
    /// `_residual` as `in`, and with a hole as `fd_dom`, which re-posted
    /// restores it exactly.
    #[test]
    fn domains_narrow_and_answer_as_residuals() {
        assert!(answers("[ [ in X 0 2 ] [ #= X 3 ] ]").is_empty());
        assert_eq!(
            answers("[ [ in X 0 9 ] [ #> X 7 ] ]"),
            ["{X=_, _residual=[ [ in *X 8 9 ] ]}"]
        );
        let holed = "{X=_, _residual=[ [ fd_dom *X [ [ 0 4 ] [ 6 9 ] ] ] ]}";
        assert_eq!(answers("[ [ in X 0 9 ] [ #≠ X 5 ] ]"), [holed]);
        assert_eq!(answers("[ fd_dom X [ [ 0 4 ] [ 6 9 ] ] ]"), [holed]);
        assert_eq!(
            answers("[ [ in X inf 3 ] [ #>= X -2 ] [ in Y 0 sup ] ]"),
            ["{X=_, Y=_, _residual=[ [ in *X -2 3 ] [ in *Y 0 :sup ] ] }".replace(" ] }", " ]}")]
        );
        // One value left binds the variable.
        assert_eq!(answers("[ [ in X 0 9 ] [ #> X 8 ] ]"), ["{X=9}"]);
    }

    /// Binding a constrained variable: an INT is checked against the
    /// domain, anything else fails, and two such variables intersect.
    #[test]
    fn binding_a_constrained_variable() {
        assert_eq!(answers("[ [ in X 0 9 ] [ = X 4 ] ]"), ["{X=4}"]);
        assert!(answers("[ [ in X 0 9 ] [ = X 12 ] ]").is_empty());
        assert!(answers("[ [ in X 0 9 ] [ = X a ] ]").is_empty());
        assert_eq!(
            answers("[ [ in X 0 5 ] [ in Y 3 9 ] [ = X Y ] ]"),
            ["{X=_, Y=_, _aliases=[ [ = *X *Y ] ], _residual=[ [ in *X 3 5 ] ]}"]
        );
        // The propagators wake: X + 1 = Y, then Y bound.
        assert_eq!(answers("[ [ #= [ + X 1 ] Y ] [ = Y 7 ] ]"), ["{X=6, Y=7}"]);
        // Left to right: [ #< X Y ] is X < Y.
        assert_eq!(
            answers("[ [ ins [ X Y ] 0 3 ] [ #< X Y ] [ = X 2 ] ]"),
            ["{X=2, Y=3}"]
        );
    }

    /// The propagators: linear sums, `*`, `abs`, `min`, `max`, and
    /// `all_different`'s value elimination.
    #[test]
    fn propagators_narrow_by_bounds() {
        let residual = |goal: &str| answers(goal).join(" ");
        assert!(residual("[ [ ins [ X Y ] 0 10 ] [ #= [ + X Y ] 3 ] ]").contains("[ in *X 0 3 ]"));
        assert!(residual("[ [ ins [ X Y ] 2 5 ] [ #= Z [ * X Y ] ] ]").contains("[ in *Z 4 25 ]"));
        assert!(residual("[ [ in X -7 3 ] [ #= Z [ abs X ] ] ]").contains("[ in *Z 0 7 ]"));
        assert!(
            residual("[ [ in X 1 9 ] [ in Y 4 6 ] [ #= Z [ min X Y ] ] ]")
                .contains("[ in *Z 1 6 ]")
        );
        assert!(
            residual("[ [ in X 1 9 ] [ in Y 4 6 ] [ #= Z [ max X Y ] ] ]")
                .contains("[ in *Z 4 9 ]")
        );
        assert_eq!(
            answers("[ [ ins [ X Y Z ] 1 3 ] [ all_different [ X Y Z ] ] [ = X 1 ] [ = Y 2 ] ]"),
            ["{X=1, Y=2, Z=3}"]
        );
        assert!(answers("[ [ ins [ X Y ] 1 1 ] [ all_different [ X Y ] ] ]").is_empty());
        // A product fixed by its factors' values: 6 = X * 3.
        assert_eq!(answers("[ [ #= 6 [ * X 3 ] ] ]"), ["{X=2}"]);
        assert!(answers("[ [ #= 7 [ * X 3 ] ] ]").is_empty());
    }

    /// Labeling: leftmost first and smallest value first; `:down` and
    /// `:ff`; every solution on backtracking.
    #[test]
    fn labeling_orders_its_search() {
        assert_eq!(
            answers("[ [ ins [ X Y ] 1 2 ] [ label [ X Y ] ] ]"),
            ["{X=1, Y=1}", "{X=1, Y=2}", "{X=2, Y=1}", "{X=2, Y=2}"]
        );
        assert_eq!(
            answers("[ [ ins [ X Y ] 1 2 ] [ labeling [ :down ] [ X Y ] ] ]"),
            ["{X=2, Y=2}", "{X=2, Y=1}", "{X=1, Y=2}", "{X=1, Y=1}"]
        );
        // First fail: Y has two values to X's three, so Y goes first.
        assert_eq!(
            answers("[ [ in X 1 3 ] [ in Y 1 2 ] [ labeling [ :ff ] [ X Y ] ] ]")[..3],
            ["{X=1, Y=1}", "{X=2, Y=1}", "{X=3, Y=1}"]
        );
        assert_eq!(
            answers("[ [ ins [ X Y ] 1 3 ] [ #< X Y ] [ label [ X Y ] ] ]"),
            ["{X=1, Y=2}", "{X=1, Y=3}", "{X=2, Y=3}"]
        );
    }

    /// A constrained but unbound variable is unbound to a bridge (E20).
    #[test]
    fn a_bridge_sees_a_constrained_variable_unbound() {
        let e = err("kb.new [ [ in X 0 9 ] [ test { X 1 > } ] ] kb.query");
        assert!(
            e.starts_with("KB.QUERY: instantiation error: X is unbound in the lambda"),
            "{e}"
        );
    }

    const SEND: &str = ":puzzle [
          [ [ solve [ S E N D M O R Y ] ] ←
              [ ins [ S E N D M O R Y ] 0 9 ]
              [ all_different [ S E N D M O R Y ] ]
              [ #≠ S 0 ] [ #≠ M 0 ]
              [ #= [ + [ + [ + [ * 1000 S ] [ * 100 E ] ] [ * 10 N ] ] D ]
                   [ - [ + [ + [ + [ * 10000 M ] [ * 1000 O ] ] [ * 100 N ] ] [ + [ * 10 E ] Y ] ]
                       [ + [ + [ + [ * 1000 M ] [ * 100 O ] ] [ * 10 R ] ] E ] ] ]
              [ label [ S E N D M O R Y ] ] ]
        ] kb! var";

    /// **Criterion 23**: SEND + MORE = MONEY answers its one solution, as
    /// worked example 7 writes it.
    #[test]
    fn send_more_money() {
        let once = top(&format!("{SEND} :puzzle var? [ solve L ] kb.once"));
        assert_eq!(shape(&once), "{L=[ 9 5 6 7 1 0 8 2 ]}");
        let all = top(&format!("{SEND} :puzzle var? [ solve L ] kb.query"));
        assert_eq!(all.as_list().map(<[BundValue]>::len), Some(1));
    }

    const QUEENS: &str = "[
          [ [ queens Qs ] ←
              [ = Qs [ A B C D E F G H ] ] [ ins Qs 1 8 ] [ safe Qs ]
              [ labeling [ :ff ] Qs ] ]
          [ safe nil ]
          [ [ safe [ Q | Qs ] ] ← [ apart Q Qs 1 ] [ safe Qs ] ]
          [ apart _ nil _ ]
          [ [ apart Q [ R | Rs ] K ] ←
              [ #≠ Q R ] [ #≠ [ abs [ - Q R ] ] K ] [ #= K1 [ + K 1 ] ]
              [ apart Q Rs K1 ] ]
        ] kb!";

    /// **Criterion 23**: 8-queens with `labeling [ :ff ]` answers 92.
    #[test]
    fn eight_queens() {
        let all = top(&format!("{QUEENS} [ queens Qs ] kb.query"));
        assert_eq!(all.as_list().map(<[BundValue]>::len), Some(92));
    }

    /// **Criterion 24**: the edges of `i64`.
    #[test]
    fn bounds_at_the_edges_of_i64() {
        let (min, max) = (i64::MIN, i64::MAX);
        // X + 1 at the top widens Y's upper bound to sup.
        assert_eq!(
            answers(&format!("[ [ in X {min} {max} ] [ #= Y [ + X 1 ] ] ]")),
            [format!(
                "{{X=_, Y=_, _residual=[ [ in *X {min} {max} ] [ in *Y {} :sup ] ]}}",
                min + 1
            )]
        );
        // A sum of four full-range products propagates without overflow:
        // each product, and the sum, may be any integer.
        let got = answers(&format!(
            "[ [ ins [ A B C D E F G H ] {min} {max} ] \
               [ #= S [ + [ + [ * A B ] [ * C D ] ] [ + [ * E F ] [ * G H ] ] ] ] ]"
        ));
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(
            got[0].contains(&format!("[ in *H {min} {max} ]")),
            "{got:?}"
        );
        assert!(!got[0].contains("*S"), "{got:?}");
        // A product beyond i64 has no INT: binding its factors fails.
        assert!(
            answers(&format!(
                "[ [ ins [ A B ] {min} {max} ] [ #= S [ * A B ] ] [ = A {max} ] [ = B {max} ] ]"
            ))
            .is_empty()
        );
        // Terms beyond i128 between them cancel exactly: max·A + max·B −
        // max·C − max·D + 5, all four at max.
        assert_eq!(
            answers(&format!(
                "[ [ ins [ A B C D ] {min} {max} ] \
                   [ #= S [ + [ - [ + [ * {max} A ] [ * {max} B ] ] \
                                  [ + [ * {max} C ] [ * {max} D ] ] ] 5 ] ] \
                   [ = A {max} ] [ = B {max} ] [ = C {max} ] [ = D {max} ] ]"
            )),
            [format!("{{A={max}, B={max}, C={max}, D={max}, S=5}}")]
        );
        // A lower bound above i64::MAX empties the domain.
        assert!(answers(&format!("[ [ #> X {max} ] ]")).is_empty());
        assert!(answers(&format!("[ [ in X 1 9 ] [ #= Y [ + X {max} ] ] ]")).is_empty());
        assert!(answers(&format!("[ [ #< X {min} ] ]")).is_empty());
        // The capped ping-pong ends in E30.
        assert_eq!(
            err("kb.new 1000000 kb.inferences \
                 [ [ ins [ X Y ] 0 1000000000000 ] [ #> X Y ] [ #> Y X ] ] kb.query"),
            "KB.QUERY: inference cap of 1000000 reached"
        );
    }

    /// **Criterion 24**: E60 to E66.
    #[test]
    fn e60_to_e66() {
        let e = |goal: &str| err(&format!("kb.new {goal} kb.query"));
        assert!(
            e("[ #= X a ]")
                .starts_with("KB.QUERY: type error: integer expression expected, found \"a\" in "),
            "{}",
            e("[ #= X a ]")
        );
        assert!(e("[ #= X [ / X 2 ] ]").contains("integer expression expected"));
        assert!(e("[ #= X 1.5 ]").contains("integer expression expected"));
        let big = i64::MAX;
        assert!(
            e(&format!("[ #= Y [ * {big} [ * 4 X ] ] ]"))
                .contains("cannot be represented as an INT in"),
            "{}",
            e(&format!("[ #= Y [ * {big} [ * 4 X ] ] ]"))
        );
        assert_eq!(
            e("[ label [ X ] ]"),
            "KB.QUERY: label: X has an infinite domain"
        );
        assert_eq!(
            e("[ [ in X 0 sup ] [ label [ X ] ] ]"),
            "KB.QUERY: label: X has an infinite domain"
        );
        for goal in [
            "[ label 5 ]",
            "[ ins a 0 1 ]",
            "[ all_different 7 ]",
            "[ label [ a ] ]",
        ] {
            assert!(
                e(goal).contains(": expected a LIST of variables, found "),
                "{goal}: {}",
                e(goal)
            );
        }
        assert_eq!(
            e("[ in X a 5 ]"),
            "KB.QUERY: in: bounds must be INT, inf or sup, found \"a\""
        );
        assert!(e("[ fd_dom X [ [ 1 ] ] ]").contains("in: bounds must be INT, inf or sup"));
        assert_eq!(
            e("[ [ in X 0 1 ] [ labeling [ :up ] [ X ] ] ]"),
            "KB.QUERY: labeling: unknown option \"up\""
        );
        assert_eq!(
            e("[ in X 5 1 ]"),
            "KB.QUERY: in: the lower bound 5 is above the upper bound 1"
        );
        assert_eq!(
            e("[ in X sup inf ]"),
            "KB.QUERY: in: the lower bound \"sup\" is above the upper bound \"inf\""
        );
    }
}
