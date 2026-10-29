//! The engine: one loop over heap structures (RFC-0013 §S4).
//!
//! **Nothing here recurses in Rust.** The goal list is a chain of nodes in a
//! `Vec`, the choicepoints are a `Vec`, and every walk over a term is
//! `term.rs`'s or `unify.rs`'s, each with its own stack (D39). A bridge (S5)
//! re-enters Bund through [`Bridge`]; that is M4's.
//!
//! **Answers one at a time.** [`Solver::next`] runs to the next answer and
//! returns, its state kept in the solver, so a caller can stop after any
//! answer without unwinding anything (L6, D84).
//!
//! **What an activation holds** (§S4's stores):
//!
//! - the arena's main store, cut back to a choicepoint's mark on backtrack;
//! - the arena's template store, never cut: each predicate's clauses,
//!   compiled on its first call, and `findall`'s copies;
//! - the trail, unwound to a choicepoint's mark;
//! - the goal nodes, cut back to a choicepoint's count. A node popped above
//!   the newest choicepoint's count is reclaimed at once: nothing can return
//!   to it, so a deterministic recursion such as `[ loop ] ← [ loop ]` runs in
//!   constant goal space;
//! - the choicepoints;
//! - the predicate table and its first-argument indexes.
//!
//! Nothing here uses the `Vm`.

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use bund2_api::Error;
use bund2_value::{BundValue, LAMBDA};

use super::store::{self, Kb, Refused};
use super::term::{Arena, Atom, Cell, Mark, SHOWN, Scope, Term};
use super::unify::{Trail, unify};

/// The inferences after which one activation warns, once (W1, D39).
pub const WARN_AT: u64 = 10_000_000;

/// What the engine asks of the `Vm`, and nothing more. `words.rs` holds the
/// one implementation.
pub trait Bridge {
    /// Report W1, a `Warning`: the run goes on (D36).
    fn warn(&mut self, reason: String);
}

/// Why a query stopped short of its answers.
#[derive(Debug)]
pub enum Stop {
    /// A refusal; the text is what follows `<W>: `.
    Code(String),
    /// An error that carries its own text: the guard's (E40), unprefixed,
    /// or an internal one.
    Raw(Error),
}

impl From<Refused> for Stop {
    fn from(r: Refused) -> Self {
        Stop::Code(r.text())
    }
}

/// One goal of the goal list. `Cut` and `Collect` are §S4's internal goals:
/// a kind of their own, not terms, so a predicate named `$cut` is ordinary.
#[derive(Debug, Clone, Copy)]
enum Goal {
    Call(Term),
    /// Truncate the choicepoint stack to this height.
    Cut(usize),
    /// Copy the template of the `Collect` choicepoint at this height.
    Collect(usize),
    Fail,
}

/// A goal and the rest of the list after it.
#[derive(Debug, Clone, Copy)]
struct Node {
    goal: Goal,
    next: Option<usize>,
}

/// What a choicepoint holds besides its marks, and what it resumes (§S4).
enum Kind {
    Clauses {
        call: Term,
        pred: usize,
        cands: Rc<[usize]>,
        next: usize,
        cont: Option<usize>,
    },
    Branches {
        branches: Rc<[Term]>,
        next: usize,
        cont: Option<usize>,
    },
    Items {
        var: Term,
        items: Rc<[Term]>,
        next: usize,
        cont: Option<usize>,
    },
    /// `not`'s goal had no solution: resume.
    Not { cont: Option<usize> },
    /// `if`'s condition had no solution: run the else branch.
    Else {
        otherwise: Term,
        cont: Option<usize>,
    },
    /// `findall`'s goal is exhausted: unify the copies with the list.
    Collect {
        template: Term,
        list: Term,
        results: Vec<Term>,
        cont: Option<usize>,
    },
}

struct Choicepoint {
    trail: usize,
    mark: Mark,
    goals: usize,
    kind: Kind,
}

/// A first argument's key, for the index (§S6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Atom(Atom),
    Int(i64),
    Struct(Atom, usize),
    Nil,
    Cons,
}

/// A clause compiled into the template store.
struct Clause {
    head: Term,
    body: Rc<[Term]>,
    /// Its first argument's key; `None` matches every call.
    key: Option<Key>,
}

struct Pred {
    clauses: Vec<Clause>,
    all: Rc<[usize]>,
    index: HashMap<Key, Rc<[usize]>>,
}

/// What a `functor/arity` that is not a control construct names.
#[derive(Debug, Clone, Copy)]
enum Known {
    User(usize),
    /// A later phase's: reserved by it, or named in `tabled` or `askable`.
    Unbuilt(u8),
    Unknown,
}

/// The control constructs' functors, interned once.
#[derive(Clone, Copy)]
struct Control {
    true_: Atom,
    fail: Atom,
    eq: Atom,
    not: Atom,
    or: Atom,
    if_: Atom,
    once: Atom,
    call: Atom,
    findall: Atom,
    is: Atom,
    test: Atom,
    in_: Atom,
}

/// One activation: a KB, a goal, and the search between them.
pub struct Solver {
    arena: Arena,
    trail: Trail,
    goals: Vec<Node>,
    cont: Option<usize>,
    cps: Vec<Choicepoint>,
    source: BTreeMap<String, BundValue>,
    deferred: Vec<String>,
    known: HashMap<(Atom, usize), Known>,
    preds: Vec<Pred>,
    control: Control,
    cap: Option<u64>,
    count: u64,
    warned: bool,
    names: Vec<(String, Term)>,
    vars: HashMap<Term, Term>,
    started: bool,
    done: bool,
}

impl Solver {
    /// An activation of `goal` over `kb`. Reads `preds`, `tabled`,
    /// `askable` and `inferences` (E02), and the goal (E04, E05, E26).
    pub fn new(kb: &Kb, goal: &BundValue) -> Result<Solver, Stop> {
        let source = kb.preds()?;
        let mut deferred = kb.names("tabled")?;
        deferred.extend(kb.names("askable")?);
        let cap = kb.inferences()?;
        let mut arena = Arena::default();
        let mut scope = Scope::default();
        let t = arena.to_term(goal, &mut scope).map_err(Refused::from)?;
        let mut a = |s: &str| arena.atoms.intern(s);
        let control = Control {
            true_: a("true"),
            fail: a("fail"),
            eq: a("="),
            not: a("not"),
            or: a("or"),
            if_: a("if"),
            once: a("once"),
            call: a("call"),
            findall: a("findall"),
            is: a("is"),
            test: a("test"),
            in_: a("in"),
        };
        Ok(Solver {
            arena,
            trail: Trail::default(),
            goals: vec![Node {
                goal: Goal::Call(t),
                next: None,
            }],
            cont: Some(0),
            cps: Vec::new(),
            source,
            deferred,
            known: HashMap::new(),
            preds: Vec::new(),
            control,
            cap,
            count: 0,
            warned: false,
            names: scope.names().to_vec(),
            vars: HashMap::new(),
            started: false,
            done: false,
        })
    }

    /// Inferences so far.
    pub fn inferences(&self) -> u64 {
        self.count
    }

    /// The arena's main store and template store, in cells: what criterion 5
    /// records.
    pub fn arena_cells(&self) -> (usize, usize) {
        (self.arena.len(), self.arena.template_len())
    }

    /// The next answer, as a MAP from each named variable of the goal to its
    /// value (§S8); `None` when there are no more.
    pub fn next(&mut self, bridge: &mut dyn Bridge) -> Result<Option<BundValue>, Stop> {
        if self.done {
            return Ok(None);
        }
        if self.started && !self.backtrack(bridge)? {
            self.done = true;
            return Ok(None);
        }
        self.started = true;
        if self.run(bridge)? {
            Ok(Some(self.answer()))
        } else {
            self.done = true;
            Ok(None)
        }
    }

    fn answer(&self) -> BundValue {
        let map = self
            .names
            .iter()
            .filter(|(n, _)| !n.starts_with('_'))
            .map(|(n, t)| (n.clone(), self.arena.to_value(*t)))
            .collect();
        BundValue::map(map)
    }

    /// Run until the goal list is empty (an answer, `true`) or nothing is
    /// left to try (`false`).
    fn run(&mut self, bridge: &mut dyn Bridge) -> Result<bool, Stop> {
        loop {
            let Some(at) = self.cont else {
                return Ok(true);
            };
            let node = *self.goals.get(at).ok_or_else(|| {
                Stop::Raw(Error::internal(
                    "kb: a goal list pointed past the goal nodes",
                ))
            })?;
            let floor = self.cps.last().map_or(0, |c| c.goals);
            if at + 1 == self.goals.len() && at >= floor {
                self.goals.pop();
            }
            self.cont = node.next;
            let ok = match node.goal {
                Goal::Cut(h) => {
                    self.cps.truncate(h);
                    true
                }
                Goal::Fail => false,
                Goal::Collect(h) => {
                    self.collect(h)?;
                    true
                }
                Goal::Call(t) => {
                    self.infer(bridge)?;
                    self.call(t)?
                }
            };
            if !ok && !self.backtrack(bridge)? {
                return Ok(false);
            }
        }
    }

    /// Count one inference: poll the guard (E40), hold the cap (E30), and
    /// warn once past [`WARN_AT`] (W1).
    fn infer(&mut self, bridge: &mut dyn Bridge) -> Result<(), Stop> {
        if let Some(trip) = bund2_api::memguard::take() {
            return Err(Stop::Raw(trip.error()));
        }
        self.count += 1;
        if let Some(cap) = self.cap
            && self.count > cap
        {
            return Err(Stop::Code(format!("inference cap of {cap} reached")));
        }
        if !self.warned && self.count >= WARN_AT {
            self.warned = true;
            bridge.warn(format!(
                "{} inferences and not finished; set kb.inferences to bound this query",
                self.count
            ));
        }
        Ok(())
    }

    fn push(&mut self, goal: Goal, next: Option<usize>) -> Option<usize> {
        self.goals.push(Node { goal, next });
        Some(self.goals.len() - 1)
    }

    /// `goals` in order, ahead of `rest`.
    fn prepend(&mut self, goals: &[Goal], rest: Option<usize>) -> Option<usize> {
        let mut cont = rest;
        for g in goals.iter().rev() {
            cont = self.push(*g, cont);
        }
        cont
    }

    fn push_cp(&mut self, kind: Kind) {
        self.cps.push(Choicepoint {
            trail: self.trail.mark(),
            mark: self.arena.mark(),
            goals: self.goals.len(),
            kind,
        });
    }

    /// Back to the newest choicepoint and on with its alternative. `false`
    /// when none is left.
    fn backtrack(&mut self, bridge: &mut dyn Bridge) -> Result<bool, Stop> {
        loop {
            let Some(mut cp) = self.cps.pop() else {
                return Ok(false);
            };
            self.trail.undo_to(&mut self.arena, cp.trail);
            self.arena.reset(cp.mark);
            self.goals.truncate(cp.goals);
            match &mut cp.kind {
                Kind::Clauses {
                    call,
                    pred,
                    cands,
                    next,
                    cont,
                } => {
                    let (call, pred, cont) = (*call, *pred, *cont);
                    let ci = cands.get(*next).copied();
                    *next += 1;
                    if *next < cands.len() {
                        self.cps.push(cp);
                    }
                    let Some(ci) = ci else { continue };
                    self.infer(bridge)?;
                    if self.try_clause(call, pred, ci, cont) {
                        return Ok(true);
                    }
                }
                Kind::Branches {
                    branches,
                    next,
                    cont,
                } => {
                    let (b, cont) = (branches.get(*next).copied(), *cont);
                    *next += 1;
                    if *next < branches.len() {
                        self.cps.push(cp);
                    }
                    let Some(b) = b else { continue };
                    self.cont = self.push(Goal::Call(b), cont);
                    return Ok(true);
                }
                Kind::Items {
                    var,
                    items,
                    next,
                    cont,
                } => {
                    let (var, item, cont) = (*var, items.get(*next).copied(), *cont);
                    *next += 1;
                    if *next < items.len() {
                        self.cps.push(cp);
                    }
                    let Some(item) = item else { continue };
                    self.infer(bridge)?;
                    if unify(&mut self.arena, &mut self.trail, var, item) {
                        self.cont = cont;
                        return Ok(true);
                    }
                }
                Kind::Not { cont } => {
                    self.cont = *cont;
                    return Ok(true);
                }
                Kind::Else { otherwise, cont } => {
                    self.cont = self.push(Goal::Call(*otherwise), *cont);
                    return Ok(true);
                }
                Kind::Collect {
                    list,
                    results,
                    cont,
                    ..
                } => {
                    let (list, cont) = (*list, *cont);
                    let mut copies = Vec::with_capacity(results.len());
                    for r in results.iter() {
                        self.vars.clear();
                        copies.push(self.arena.copy(*r, &mut self.vars, false));
                    }
                    let mut l = self.arena.nil();
                    for c in copies.into_iter().rev() {
                        l = self.arena.cons(c, l);
                    }
                    if unify(&mut self.arena, &mut self.trail, list, l) {
                        self.cont = cont;
                        return Ok(true);
                    }
                }
            }
        }
    }

    /// `$collect`: copy the template, as it now stands, past backtracking.
    fn collect(&mut self, h: usize) -> Result<(), Stop> {
        let template = match self.cps.get(h).map(|c| &c.kind) {
            Some(Kind::Collect { template, .. }) => *template,
            _ => {
                return Err(Stop::Raw(Error::internal(
                    "kb: a $collect goal outlived its findall choicepoint",
                )));
            }
        };
        self.vars.clear();
        let copy = self.arena.copy(template, &mut self.vars, true);
        if let Some(Choicepoint {
            kind: Kind::Collect { results, .. },
            ..
        }) = self.cps.get_mut(h)
        {
            results.push(copy);
        }
        Ok(())
    }

    /// Run one goal. `false` means it failed.
    fn call(&mut self, t: Term) -> Result<bool, Stop> {
        let d = self.arena.deref(t);
        let rest = self.cont;
        let (f, args) = match self.arena.cell(d) {
            Cell::Var(_) => return Err(self.unbound(d, d)),
            Cell::Cons(..) => {
                let (items, end) = self.arena.spine(d);
                return match self.arena.cell(end) {
                    Cell::Nil => {
                        let goals: Vec<Goal> = items.into_iter().map(Goal::Call).collect();
                        self.cont = self.prepend(&goals, rest);
                        Ok(true)
                    }
                    Cell::Var(_) => Err(self.unbound(end, d)),
                    _ => Err(self.not_callable(d)),
                };
            }
            Cell::Atom(a) => (a, Vec::new()),
            Cell::Struct {
                functor,
                start,
                arity,
            } => (functor, self.arena.args(start, arity).to_vec()),
            Cell::Nil | Cell::Int(_) | Cell::Float(_) | Cell::Opaque(_) => {
                return Err(self.not_callable(d));
            }
        };
        let c = self.control;
        let calls = |g: &[Term]| g.iter().copied().map(Goal::Call).collect::<Vec<_>>();
        match (f, args.as_slice()) {
            (f, []) if f == c.true_ => Ok(true),
            (f, []) if f == c.fail => Ok(false),
            (f, [a, b]) if f == c.eq => Ok(unify(&mut self.arena, &mut self.trail, *a, *b)),
            (f, gs) if f == c.not => {
                let h = self.cps.len();
                self.push_cp(Kind::Not { cont: rest });
                let mut goals = calls(gs);
                goals.extend([Goal::Cut(h), Goal::Fail]);
                self.cont = self.prepend(&goals, None);
                Ok(true)
            }
            (f, gs) if f == c.once => {
                let h = self.cps.len();
                let mut goals = calls(gs);
                goals.push(Goal::Cut(h));
                self.cont = self.prepend(&goals, rest);
                Ok(true)
            }
            (f, branches) if f == c.or => {
                let Some((first, others)) = branches.split_first() else {
                    return Ok(false);
                };
                if !others.is_empty() {
                    self.push_cp(Kind::Branches {
                        branches: others.into(),
                        next: 0,
                        cont: rest,
                    });
                }
                self.cont = self.push(Goal::Call(*first), rest);
                Ok(true)
            }
            (f, [cond, then, otherwise]) if f == c.if_ => {
                let h = self.cps.len();
                self.push_cp(Kind::Else {
                    otherwise: *otherwise,
                    cont: rest,
                });
                self.cont =
                    self.prepend(&[Goal::Call(*cond), Goal::Cut(h), Goal::Call(*then)], rest);
                Ok(true)
            }
            (f, [g, extra @ ..]) if f == c.call => {
                let g = self.arena.deref(*g);
                let goal = match (self.arena.cell(g), extra) {
                    (Cell::Var(_), _) => return Err(self.unbound(g, d)),
                    (_, []) => g,
                    (Cell::Atom(a), _) => self.arena.structure(a, extra),
                    (
                        Cell::Struct {
                            functor,
                            start,
                            arity,
                        },
                        _,
                    ) => {
                        let mut all = self.arena.args(start, arity).to_vec();
                        all.extend_from_slice(extra);
                        self.arena.structure(functor, &all)
                    }
                    _ => return Err(self.not_callable(g)),
                };
                self.cont = self.push(Goal::Call(goal), rest);
                Ok(true)
            }
            (f, [template, goal, list]) if f == c.findall => {
                let h = self.cps.len();
                self.push_cp(Kind::Collect {
                    template: *template,
                    list: *list,
                    results: Vec::new(),
                    cont: rest,
                });
                self.cont = self.prepend(&[Goal::Call(*goal), Goal::Collect(h), Goal::Fail], None);
                Ok(true)
            }
            (f, [x, l]) if f == c.in_ => self.member(*x, *l, d),
            (f, [_, _]) if f == c.is => Err(self.no_bridge(d)),
            (f, [_]) if f == c.test => Err(self.no_bridge(d)),
            (f, _) => {
                let n = args.len();
                match self.lookup(f, n)? {
                    Known::User(p) => Ok(self.resolve(d, p, &args)),
                    Known::Unbuilt(phase) => Err(Stop::Code(format!(
                        "{}/{n} belongs to Phase {phase}, which is not built",
                        self.arena.atoms.text(f)
                    ))),
                    Known::Unknown => Err(Stop::Code(format!(
                        "existence error: unknown predicate {}/{n} in {}",
                        self.arena.atoms.text(f),
                        self.show(d)
                    ))),
                }
            }
        }
    }

    /// `[ in X L ]` over a LIST. Over a λ it is a bridge (M4).
    fn member(&mut self, x: Term, l: Term, goal: Term) -> Result<bool, Stop> {
        let l = self.arena.deref(l);
        match self.arena.cell(l) {
            Cell::Opaque(i) if self.arena.opaque_value(i).is_some_and(|v| v.dt() == LAMBDA) => {
                return Err(self.no_bridge(goal));
            }
            Cell::Var(_) => return Err(self.unbound(l, goal)),
            Cell::Nil | Cell::Cons(..) => {}
            _ => return Err(self.not_a_list(l, goal)),
        }
        let (items, end) = self.arena.spine(l);
        match self.arena.cell(end) {
            Cell::Nil => {}
            Cell::Var(_) => return Err(self.unbound(end, goal)),
            _ => return Err(self.not_a_list(l, goal)),
        }
        let Some((first, others)) = items.split_first() else {
            return Ok(false);
        };
        if !others.is_empty() {
            self.push_cp(Kind::Items {
                var: x,
                items: others.into(),
                next: 0,
                cont: self.cont,
            });
        }
        Ok(unify(&mut self.arena, &mut self.trail, x, *first))
    }

    /// What `functor/arity` names, decided once per activation.
    fn lookup(&mut self, f: Atom, n: usize) -> Result<Known, Stop> {
        if let Some(k) = self.known.get(&(f, n)) {
            return Ok(*k);
        }
        let text = self.arena.atoms.text(f).to_string();
        let key = format!("{text}/{n}");
        let k = match store::reserved(&text, n) {
            Some(phase) if phase > 1 => Known::Unbuilt(phase),
            _ if self.deferred.contains(&key) => Known::Unbuilt(2),
            _ if self.source.contains_key(&key) => Known::User(self.compile(&key)?),
            _ => Known::Unknown,
        };
        self.known.insert((f, n), k);
        Ok(k)
    }

    /// Compile a predicate's clauses into the template store, on its first
    /// call (§S4's renaming, §S6's index).
    fn compile(&mut self, key: &str) -> Result<usize, Stop> {
        let values = Kb::clauses_of(&self.source, key)?;
        let mut clauses = Vec::with_capacity(values.len());
        for v in &values {
            let (head_v, goals_v) = store::split(v);
            let mut scope = Scope::default();
            let head = self
                .arena
                .to_template(head_v, &mut scope)
                .map_err(Refused::from)?;
            let first = match self.arena.cell(head) {
                Cell::Atom(_) => None,
                Cell::Struct { start, arity, .. } => self.arena.args(start, arity).first().copied(),
                _ => return Err(Refused::BadHead(head_v.clone()).into()),
            };
            let key = first.and_then(|a| self.key(a));
            let body = goals_v
                .iter()
                .map(|g| self.arena.to_template(g, &mut scope))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Refused::from)?;
            clauses.push(Clause {
                head,
                body: body.into(),
                key,
            });
        }
        let all: Rc<[usize]> = (0..clauses.len()).collect();
        self.preds.push(Pred {
            clauses,
            all,
            index: HashMap::new(),
        });
        Ok(self.preds.len() - 1)
    }

    /// A first argument's index key. `None` for an unbound variable, an
    /// opaque constant or a float, which match every clause.
    fn key(&self, t: Term) -> Option<Key> {
        let t = self.arena.deref(t);
        match self.arena.cell(t) {
            Cell::Atom(a) => Some(Key::Atom(a)),
            Cell::Int(i) => Some(Key::Int(i)),
            Cell::Struct { functor, arity, .. } => Some(Key::Struct(functor, arity)),
            Cell::Nil => Some(Key::Nil),
            Cell::Cons(..) => Some(Key::Cons),
            Cell::Var(_) | Cell::Float(_) | Cell::Opaque(_) => None,
        }
    }

    /// A user predicate's call: compute the candidates, push `Clauses` when
    /// two or more remain, and try the first.
    fn resolve(&mut self, call: Term, p: usize, args: &[Term]) -> bool {
        let key = args.first().and_then(|a| self.key(*a));
        let Some(pred) = self.preds.get_mut(p) else {
            return false;
        };
        let cands = match key {
            None => pred.all.clone(),
            Some(k) => {
                let clauses = &pred.clauses;
                pred.index
                    .entry(k)
                    .or_insert_with(|| {
                        clauses
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| c.key.is_none_or(|ck| ck == k))
                            .map(|(i, _)| i)
                            .collect()
                    })
                    .clone()
            }
        };
        let Some(&first) = cands.first() else {
            return false;
        };
        let rest = self.cont;
        if cands.len() > 1 {
            self.push_cp(Kind::Clauses {
                call,
                pred: p,
                cands,
                next: 1,
                cont: rest,
            });
        }
        self.try_clause(call, p, first, rest)
    }

    /// Rename clause `ci` of `p`, unify its head with `call`, and put its
    /// body ahead of `rest`.
    fn try_clause(&mut self, call: Term, p: usize, ci: usize, rest: Option<usize>) -> bool {
        let Some(clause) = self.preds.get(p).and_then(|pr| pr.clauses.get(ci)) else {
            return false;
        };
        let (head, body) = (clause.head, clause.body.clone());
        self.vars.clear();
        let h = self.arena.copy(head, &mut self.vars, false);
        if !unify(&mut self.arena, &mut self.trail, call, h) {
            return false;
        }
        let goals: Vec<Goal> = body
            .iter()
            .map(|g| Goal::Call(self.arena.copy(*g, &mut self.vars, false)))
            .collect();
        self.cont = self.prepend(&goals, rest);
        true
    }

    // The refusals' texts (§S12).

    /// A term as a refusal shows it: a query variable by its name, any other
    /// by `_G`*n* (§S8), and `summary` bounding the whole (D36).
    fn show(&self, t: Term) -> String {
        self.arena
            .to_value_with(t, &|v| BundValue::ptr(self.var_name(v)))
            .summary(SHOWN)
    }

    fn var_name(&self, v: Term) -> String {
        self.names
            .iter()
            .find(|(_, t)| self.arena.deref(*t) == v)
            .map_or_else(|| format!("_G{}", v.id()), |(n, _)| n.clone())
    }

    /// E10.
    fn unbound(&self, var: Term, goal: Term) -> Stop {
        Stop::Code(format!(
            "instantiation error: {} is unbound in {}",
            self.var_name(var),
            self.show(goal)
        ))
    }

    /// E11.
    fn not_callable(&self, t: Term) -> Stop {
        Stop::Code(format!("type error: {} is not callable", self.show(t)))
    }

    /// E23.
    fn not_a_list(&self, l: Term, goal: Term) -> Stop {
        Stop::Code(format!(
            "in: expected a list, found {} in {}",
            self.show(l),
            self.show(goal)
        ))
    }

    /// A bridge goal, refused until M4 builds `bridge.rs`.
    fn no_bridge(&self, goal: Term) -> Stop {
        Stop::Code(format!(
            "bridges are not built yet (RFC-0013 M4): {}",
            self.show(goal)
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use bund2_api::Vm as _;
    use bund2_api::diag::{Diagnostic, Reporter, Severity};
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    use crate::kb::store::Kb;

    type Said = Rc<RefCell<Vec<(Severity, String)>>>;

    /// Keeps every diagnostic, as `logging.rs`'s tests do.
    struct Kept(Said);

    impl Reporter for Kept {
        fn report(&mut self, d: &Diagnostic) {
            self.0.borrow_mut().push((d.severity, d.reason.clone()));
        }
    }

    fn interp() -> (Interp, Said) {
        let said: Said = Rc::default();
        let mut i = Interp::new();
        i.reporter = Box::new(Kept(Rc::clone(&said)));
        crate::register_all(&mut i.registry);
        (i, said)
    }

    /// Push `before`, then run `src`; the failure's own reason.
    fn run_on(i: &mut Interp, before: Vec<BundValue>, src: &str) -> Result<(), String> {
        for v in before {
            i.push(v);
        }
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| {
            e.0.rsplit_once(" returned error: ")
                .map_or(e.0.clone(), |(_, m)| m.to_string())
        })
    }

    fn top_on(before: Vec<BundValue>, src: &str) -> BundValue {
        let (mut i, _) = interp();
        run_on(&mut i, before, src).unwrap_or_else(|e| panic!("{src} failed: {e}"));
        i.peek().unwrap_or_else(|| panic!("{src}: empty"))
    }

    fn top(src: &str) -> BundValue {
        top_on(Vec::new(), src)
    }

    fn err_on(before: Vec<BundValue>, src: &str) -> String {
        let (mut i, _) = interp();
        match run_on(&mut i, before, src) {
            Ok(()) => panic!("{src} was expected to fail"),
            Err(e) => e,
        }
    }

    fn err(src: &str) -> String {
        err_on(Vec::new(), src)
    }

    /// `kb.query`'s answers, each as `name=value` pairs in MAP order.
    fn answers_on(before: Vec<BundValue>, src: &str) -> Vec<String> {
        let list = top_on(before, src);
        list.as_list()
            .unwrap_or_else(|| panic!("{src}: not a LIST: {}", list.display()))
            .iter()
            .map(|a| {
                a.as_map()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| format!("{k}={}", v.display()))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect()
    }

    fn answers(src: &str) -> Vec<String> {
        answers_on(Vec::new(), src)
    }

    const FAMILY: &str = "[ [ parent tom bob ] [ parent bob ann ] [ parent bob pat ] \
                          [ [ grandparent X Z ] ← [ parent X Y ] [ parent Y Z ] ] ] kb!";

    #[test]
    fn the_first_worked_example_answers() {
        assert_eq!(
            answers(&format!("{FAMILY} [ grandparent tom W ] kb.query")),
            ["W=ann", "W=pat"]
        );
        assert_eq!(
            top(&format!("{FAMILY} [ grandparent tom :ann ] kb.holds")).display(),
            BundValue::boolean(true).display()
        );
        assert_eq!(
            top(&format!("{FAMILY} [ grandparent :ann W ] kb.holds")).display(),
            BundValue::boolean(false).display()
        );
        let once = top(&format!("{FAMILY} [ parent bob C ] kb.once"));
        assert_eq!(
            once.as_map().unwrap().get("C").unwrap().display(),
            "bob".replace("bob", "ann")
        );
        assert_eq!(
            top(&format!("{FAMILY} [ parent ann C ] kb.once")).dt(),
            bund2_value::NODATA
        );
    }

    /// §S8: `_`-names are left out; a ground goal that holds answers one
    /// empty MAP; an unbound answer is NODATA.
    #[test]
    fn answers_are_shaped_as_section_s8_says() {
        assert_eq!(
            answers(&format!("{FAMILY} [ parent _P C ] kb.query")),
            ["C=bob", "C=ann", "C=pat"]
        );
        assert_eq!(
            answers(&format!("{FAMILY} [ parent tom bob ] kb.query")),
            [""]
        );
        assert!(answers(&format!("{FAMILY} [ parent tom ann ] kb.query")).is_empty());
        let a = answers("[ [ p _ ] ] kb! [ p X ] kb.query");
        assert_eq!(a, [format!("X={}", BundValue::nodata().display())]);
    }

    /// A query that is a LIST of goals is their conjunction, and each
    /// control construct of §S3 does what its row says.
    #[test]
    fn control_constructs_do_what_section_s3_says() {
        let kb = "[ [ n 1 ] [ n 2 ] [ n 3 ] [ [ small X ] ← [ n X ] [ not [ = X 3 ] ] ] ] kb!";
        assert_eq!(
            answers(&format!("{kb} [ small X ] kb.query")),
            ["X=1", "X=2"]
        );
        assert_eq!(
            answers(&format!("{kb} [ [ n X ] [ = X 2 ] ] kb.query")),
            ["X=2"]
        );
        assert_eq!(answers(&format!("{kb} [ once [ n X ] ] kb.query")), ["X=1"]);
        assert_eq!(
            answers(&format!("{kb} [ or [ = X a ] [ n X ] ] kb.query")),
            ["X=a", "X=1", "X=2", "X=3"]
        );
        assert_eq!(
            answers(&format!(
                "{kb} [ if [ n X ] [ = Y yes ] [ = Y no ] ] kb.query"
            )),
            ["X=1,Y=yes"]
        );
        assert_eq!(
            answers(&format!(
                "{kb} [ if [ n 9 ] [ = Y yes ] [ = Y no ] ] kb.query"
            )),
            ["Y=no"]
        );
        assert_eq!(
            answers(&format!("{kb} [ findall X [ n X ] L ] kb.query")),
            [format!(
                "L={},X={}",
                top("[ 1 2 3 ]").display(),
                BundValue::nodata().display()
            )]
        );
        assert_eq!(
            answers(&format!("{kb} [ findall X [ n 9 ] L ] kb.query")),
            [format!(
                "L={},X={}",
                BundValue::list(Vec::new()).display(),
                BundValue::nodata().display()
            )]
        );
        assert_eq!(
            answers(&format!("{kb} [ call n X ] kb.query")),
            ["X=1", "X=2", "X=3"]
        );
        assert_eq!(answers(&format!("{kb} [ call [ n ] 2 ] kb.query")), [""]);
        assert_eq!(
            answers(&format!("{kb} [ in X [ :a :b ] ] kb.query")),
            ["X=a", "X=b"]
        );
        assert!(answers(&format!("{kb} [ fail ] kb.query")).is_empty());
        assert_eq!(
            answers(&format!("{kb} [ [ true ] [ n 1 ] ] kb.query")),
            [""]
        );
        // `findall` keeps sharing within one copy: `X = Y` in each.
        let a = answers("kb.new [ findall [ p X X ] [ = X _ ] L ] kb.query");
        assert_eq!(a.len(), 1);
    }

    /// Recursion, backtracking into a recursive predicate, and a list.
    #[test]
    fn recursion_and_lists() {
        let kb = "[ [ app nil L L ] [ [ app [ H | T ] L [ H | R ] ] ← [ app T L R ] ] ] kb!";
        assert_eq!(
            answers(&format!("{kb} [ app [ 1 2 ] [ 3 ] X ] kb.query")),
            [format!("X={}", top("[ 1 2 3 ]").display())]
        );
        assert_eq!(
            answers(&format!("{kb} [ app X Y [ 1 2 ] ] kb.query")).len(),
            3
        );
    }

    /// **Criterion 2b:** `ready` and `[ ready ]` are one term, as goals and
    /// as heads.
    #[test]
    fn ready_and_a_one_item_list_of_it_unify_as_goals_and_heads() {
        let ready = BundValue::call("ready");
        let goals = [
            ready.clone(),
            BundValue::list(vec![ready.clone()]),
            BundValue::str("ready"),
        ];
        for kb in [
            "[ ready ] kb!",
            "[ [ ready ] ] kb!",
            "[ [ [ ready ] ] ] kb!",
        ] {
            for goal in &goals {
                let src = format!("{kb} swap kb.holds");
                assert_eq!(
                    top_on(vec![goal.clone()], &src).display(),
                    BundValue::boolean(true).display(),
                    "{src}"
                );
            }
        }
        // A rule whose head is written `[ ready ]`.
        let src = "[ [ go ] [ [ ready ] ← go ] ] kb! [ ready ] kb.holds";
        assert_eq!(top(src).display(), BundValue::boolean(true).display());
    }

    // **Criterion 1b**, the codes the asking words raise by their text.

    #[test]
    fn e00_is_unprefixed() {
        for w in ["kb.query", "kb.once", "kb.holds"] {
            assert_eq!(
                err(&format!("kb.new {w}")),
                format!("Stack is too shallow for inline {w}")
            );
        }
    }

    #[test]
    fn e01_and_e02_read_the_kb() {
        assert_eq!(
            err("dict [ p ] kb.query"),
            "KB.QUERY: #2 parameter must be a knowledge base"
        );
        assert_eq!(
            err("kb.new :preds 1 set [ p ] kb.once"),
            "KB.ONCE: malformed knowledge base: preds is not a MAP"
        );
        assert_eq!(
            err("kb.new :inferences :x set [ p ] kb.holds"),
            "KB.HOLDS: malformed knowledge base: inferences is not a INT"
        );
    }

    #[test]
    fn e04_e05_e26_refuse_the_goal() {
        let bad = BundValue::list(vec![BundValue::call("p"), BundValue::call("X|Y")]);
        assert_eq!(
            err_on(vec![Kb::new().into_value(), bad], "kb.query"),
            "KB.QUERY: X|Y is not a variable name: it contains '|'"
        );
        assert!(err("kb.new [ p [ :a | ] ] kb.query").starts_with("KB.QUERY: '|' must stand"));
        let ctx = BundValue::list(vec![BundValue::call("p"), BundValue::context()]);
        assert!(
            err_on(vec![Kb::new().into_value(), ctx], "kb.query")
                .starts_with("KB.QUERY: a CONTEXT cannot be a term: ")
        );
    }

    #[test]
    fn e10_names_the_unbound_variable_and_the_goal() {
        let e = err("kb.new [ call G ] kb.query");
        assert!(
            e.starts_with("KB.QUERY: instantiation error: G is unbound in "),
            "{e}"
        );
        let e = err("kb.new [ [ = X 1 ] Y ] kb.holds");
        assert!(
            e.starts_with("KB.HOLDS: instantiation error: Y is unbound in "),
            "{e}"
        );
        let e = err("kb.new [ in X L ] kb.query");
        assert!(e.contains("instantiation error: L is unbound"), "{e}");
        // A clause variable has no name the program wrote: `_G`n.
        let e = err("[ [ [ p ] ← Q ] ] kb! [ p ] kb.query");
        assert!(e.contains("instantiation error: _G"), "{e}");
    }

    #[test]
    fn e11_refuses_what_is_not_callable() {
        assert_eq!(
            err("kb.new 42 kb.query"),
            "KB.QUERY: type error: 42 is not callable"
        );
        assert!(err("kb.new [ call 1.5 ] kb.query").starts_with("KB.QUERY: type error: "));
        // `nil` as a goal (S3).
        assert!(
            err_on(
                vec![Kb::new().into_value(), BundValue::call("nil")],
                "kb.once"
            )
            .starts_with("KB.ONCE: type error: ")
        );
        let e = err_on(
            vec![Kb::new().into_value(), BundValue::map(Default::default())],
            "kb.query",
        );
        assert!(e.starts_with("KB.QUERY: type error: "), "{e}");
    }

    /// **Criterion 6a** (L8).
    #[test]
    fn e12_names_an_undefined_predicate() {
        assert_eq!(
            err(&format!("{FAMILY} [ sibling tom X ] kb.query")),
            format!(
                "KB.QUERY: existence error: unknown predicate sibling/2 in {}",
                BundValue::list(vec![
                    BundValue::call("sibling"),
                    BundValue::str("tom"),
                    BundValue::ptr("X")
                ])
                .summary(super::SHOWN)
            )
        );
        // `[ ready [ p X ] ]` is `ready/1`, not a conjunction (S1's traps).
        assert!(err("[ ready ] kb! [ ready [ p X ] ] kb.query").contains("ready/1"));
        // A wrong arity of a control construct is a user predicate.
        assert!(err("kb.new [ if a b ] kb.query").contains("unknown predicate if/2"));
    }

    #[test]
    fn e13_is_a_query_with_no_goal() {
        assert_eq!(
            err_on(
                vec![Kb::new().into_value(), BundValue::nodata()],
                "kb.query"
            ),
            "KB.QUERY: the query has no goal"
        );
    }

    #[test]
    fn e14_names_a_later_phase() {
        assert_eq!(
            err("kb.new [ dif a b ] kb.query"),
            "KB.QUERY: dif/2 belongs to Phase 2, which is not built"
        );
        assert_eq!(
            err("kb.new [ label X ] kb.query"),
            "KB.QUERY: label/1 belongs to Phase 3, which is not built"
        );
        // A predicate named in `tabled` or `askable` (the implementation
        // plan), whether or not it has clauses.
        assert_eq!(
            err("[ [ p 1 ] ] kb! :tabled [ \"p/1\" ] set [ p X ] kb.query"),
            "KB.QUERY: p/1 belongs to Phase 2, which is not built"
        );
        assert_eq!(
            err_on(
                vec![BundValue::call("q")],
                "kb.new :askable [ \"q/0\" ] set swap kb.holds"
            ),
            "KB.HOLDS: q/0 belongs to Phase 2, which is not built"
        );
    }

    #[test]
    fn e30_is_the_cap_and_try_catches_it() {
        const LOOP: &str = "[ [ [ loop ] ← [ loop ] ] ] kb!";
        assert_eq!(
            err(&format!("{LOOP} 1000 kb.inferences [ loop ] kb.query")),
            "KB.QUERY: inference cap of 1000 reached"
        );
        // Within the cap, the answers.
        assert_eq!(
            answers(&format!(
                "{FAMILY} 1000 kb.inferences [ parent bob C ] kb.query"
            )),
            ["C=ann", "C=pat"]
        );
        let caught = top(&format!(
            "?try :try {{ {LOOP} 50 kb.inferences [ loop ] kb.holds }} set \
             :except {{ \"caught\" }} set !"
        ));
        assert_eq!(caught.as_str().as_deref(), Some("caught"));
    }

    /// **Criterion 5.** `[ loop ]` over `[ [ loop ] ← [ loop ] ]`, capped at
    /// 20,000,000: one W1 at 10,000,000, then E30, which `?try` catches. The
    /// arena does not grow: the clause is ground, so renaming shares it, and
    /// the goal node is reclaimed with no choicepoint above it.
    #[test]
    fn the_cap_warns_once_then_stops_and_the_arena_stays_flat() {
        let (mut i, said) = interp();
        let src = "?try :try { [ [ [ loop ] ← [ loop ] ] ] kb! 20000000 kb.inferences \
                   [ loop ] kb.holds } set :except { \"caught\" } set !";
        run_on(&mut i, Vec::new(), src).unwrap();
        assert_eq!(i.peek().and_then(|v| v.as_str()).as_deref(), Some("caught"));
        let said = said.borrow();
        let warnings: Vec<_> = said
            .iter()
            .filter(|(s, _)| *s == Severity::Warning)
            .collect();
        assert_eq!(
            warnings,
            [&(
                Severity::Warning,
                "KB.HOLDS: 10000000 inferences and not finished; set kb.inferences to bound \
                 this query"
                    .to_string()
            )]
        );

        // The arena, recorded: run the activation directly.
        let kb = Kb::build(&[top("[ [ [ loop ] ← [ loop ] ] ]").as_list().unwrap()[0].clone()])
            .unwrap()
            .with_inferences(1_000_000);
        let mut s = super::Solver::new(&kb, &BundValue::call("loop")).unwrap();
        struct Quiet;
        impl super::Bridge for Quiet {
            fn warn(&mut self, _: String) {}
        }
        assert!(s.next(&mut Quiet).is_err());
        assert_eq!(s.inferences(), 1_000_001);
        let (main, tpl) = s.arena_cells();
        assert!(main <= 1, "the main store grew to {main} cells");
        assert!(tpl <= 4, "the template store holds {tpl} cells");
        assert!(s.goals.len() <= 1, "{} goal nodes", s.goals.len());
    }

    /// **Criterion 11**, the first half: `kb.clauses` → `kb!`, and
    /// `save.model` → `load.model`, give the same answers.
    #[test]
    fn a_kb_round_trips_through_clauses_and_the_world() {
        let want = answers(&format!("{FAMILY} [ grandparent X Y ] kb.query"));
        assert_eq!(
            answers(&format!(
                "{FAMILY} kb.clauses kb! [ grandparent X Y ] kb.query"
            )),
            want
        );
        let world = format!("bund2-kb-test-{}", std::process::id());
        let file = format!("{world}.world");
        let _ = std::fs::remove_file(&file);
        let (mut i, _) = interp();
        let r = run_on(
            &mut i,
            Vec::new(),
            &format!(
                "\"family\" {FAMILY} \"{world}\" save.model \"family\" \"{world}\" load.model \
                 [ grandparent X Y ] kb.query"
            ),
        );
        let _ = std::fs::remove_file(&file);
        r.unwrap();
        let got: Vec<String> = i
            .peek()
            .unwrap()
            .as_list()
            .unwrap()
            .iter()
            .map(|a| {
                a.as_map()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| format!("{k}={}", v.display()))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect();
        assert_eq!(got, want);
    }

    /// A LIST `levels` deep counting the leaf, as `to_binary` counts: a
    /// compound `f` around `f` around … around the atom `a`.
    fn deep(levels: usize) -> BundValue {
        let mut v = BundValue::call("a");
        for _ in 1..levels {
            v = BundValue::list(vec![BundValue::call("f"), v]);
        }
        v
    }

    /// **Criterion 11**, the depths of §S2's table: a fact's term 253 levels
    /// crosses and 254 does not; a rule's goal 252 crosses and 253 does not.
    #[test]
    fn the_wire_bounds_a_clause_as_section_s2_says() {
        let crosses = |kb: BundValue| {
            let (mut i, _) = interp();
            run_on(&mut i, vec![kb], "\"t_kb_deep\" swap send.quick").is_ok()
        };
        let fact =
            |n: usize| Kb::build(&[BundValue::list(vec![BundValue::call("p"), deep(n - 1)])]);
        // The fact's term is `p(…)`: one level for `p`'s LIST, and the
        // argument's.
        assert!(crosses(fact(253).unwrap().into_value()));
        assert!(!crosses(fact(254).unwrap().into_value()));
        let rule = |n: usize| {
            Kb::build(&[BundValue::list(vec![
                BundValue::list(vec![BundValue::call("p")]),
                BundValue::list(vec![BundValue::call("q"), deep(n - 1)]),
            ])])
        };
        assert!(crosses(rule(252).unwrap().into_value()));
        assert!(!crosses(rule(253).unwrap().into_value()));
    }

    /// **Criterion 11**, the last: an opaque constant sent over the bus no
    /// longer unifies with its original (L13, D88).
    #[test]
    fn an_opaque_constant_over_the_bus_is_a_new_one() {
        let thing = BundValue::map(Default::default());
        let kb = Kb::build(&[BundValue::list(vec![BundValue::call("has"), thing.clone()])])
            .unwrap()
            .into_value();
        let goal = BundValue::list(vec![BundValue::call("has"), thing]);
        let (mut i, _) = interp();
        run_on(&mut i, vec![kb.clone(), goal.clone()], "kb.holds").unwrap();
        let here = i.pull().unwrap();
        assert_eq!(here.display(), BundValue::boolean(true).display());
        run_on(
            &mut i,
            vec![BundValue::str("t_kb_opaque"), kb],
            "send.quick \"t_kb_opaque\" recv",
        )
        .unwrap();
        let sent = i.pull().unwrap();
        run_on(&mut i, vec![sent, goal], "kb.holds").unwrap();
        assert_eq!(
            i.peek().unwrap().display(),
            BundValue::boolean(false).display()
        );
    }

    /// **Criterion 15.** NODATA in an asserted clause is a fresh variable, and
    /// an answer with an unbound part, asserted again, answers as it did.
    #[test]
    fn nodata_is_a_fresh_variable() {
        let clause = BundValue::list(vec![
            BundValue::call("q"),
            BundValue::nodata(),
            BundValue::nodata(),
        ]);
        let kb = Kb::new().assert(&clause, false).unwrap().into_value();
        assert_eq!(
            top_on(vec![kb.clone()], "[ q a b ] kb.holds").display(),
            BundValue::boolean(true).display()
        );

        let kb = Kb::build(&[top("[ [ p [ f _ ] ] ]").as_list().unwrap()[0].clone()])
            .unwrap()
            .into_value();
        let first = answers_on(vec![kb.clone()], "[ p X ] kb.query");
        let answer = top_on(vec![kb], "[ p X ] kb.once");
        let x = answer.as_map().unwrap().get("X").unwrap().clone();
        let again = Kb::new()
            .assert(&BundValue::list(vec![BundValue::call("p"), x]), false)
            .unwrap()
            .into_value();
        assert_eq!(answers_on(vec![again.clone()], "[ p X ] kb.query"), first);
        assert_eq!(
            top_on(vec![again], "[ p [ f zzz ] ] kb.holds").display(),
            BundValue::boolean(true).display()
        );
    }

    /// The logical update view: a query holds the KB value it was handed.
    #[test]
    fn a_query_sees_the_kb_it_was_handed() {
        assert_eq!(
            answers(&format!(
                ":k {FAMILY} var :k var? [ parent tom X ] :k var? [ parent tom zed ] kb.assert \
                 drop kb.query"
            )),
            ["X=bob"]
        );
    }
}
