//! The engine: one loop over heap structures (RFC-0013 §S4).
//!
//! **Nothing here recurses in Rust.** The goal list is a chain of nodes in a
//! `Vec`, the choicepoints are a `Vec`, and every walk over a term is
//! `term.rs`'s or `unify.rs`'s, each with its own stack (D39). A bridge (S5)
//! re-enters Bund through [`Bridge`], and that recursion is bounded by
//! `stack_ok` as any nested evaluation is.
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
//! - the predicate table and its first-argument indexes;
//! - the suspended `dif`s and their attributes (§S17), taken back to a
//!   choicepoint's mark as the trail is.
//!
//! Nothing here uses the `Vm`.

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use bund2_api::Error;
use bund2_value::{BundValue, LAMBDA};

use super::dif::{DifMark, Difs};
use super::proof::Proofs;
use super::store::{self, Kb, Refused};
use super::term::{Arena, Atom, Cell, Mark, Refusal, SHOWN, Scope, Term};
use super::unify::{Trail, unify, unify_waking};

/// The inferences after which one activation warns, once (W1, D39).
pub const WARN_AT: u64 = 10_000_000;

/// Whether each inference polls the memory guard: always, except under the
/// measurement-only `kb-unpolled-bench` (criterion 16), which is unsound.
pub const POLLED: bool = !cfg!(feature = "kb-unpolled-bench");

/// What the engine asks of the `Vm`, and nothing more. `words.rs` holds the
/// one implementation.
pub trait Bridge {
    /// Report W1, a `Warning`: the run goes on (D36).
    fn warn(&mut self, reason: String);
    /// Run a bridge's λ, already substituted, on a scratch stack, and answer
    /// what it left on top (§S5 step 3, `bridge.rs: run`).
    fn run(&mut self, body: Vec<BundValue>) -> Result<Option<BundValue>, Error>;
    /// Run the `ask` lambda on `goal`, read back, and answer what it left on
    /// top (§S16 step 4, `bridge.rs: ask`). `None` when the query has no
    /// `ask` lambda (E50), as no asking word but `query` has.
    fn ask(&mut self, goal: BundValue) -> Option<Result<Option<BundValue>, Error>> {
        let _ = goal;
        None
    }
    /// Write the replies so far through to the `known` variable, when the
    /// query names one (§S16). Each is a `[ goal reply ]` pair.
    fn remember(&mut self, replies: &[BundValue]) {
        let _ = replies;
    }
}

/// The three bridges of §S5's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bridging {
    Is,
    Test,
    In,
}

impl Bridging {
    fn name(self) -> &'static str {
        match self {
            Bridging::Is => "is",
            Bridging::Test => "test",
            Bridging::In => "in",
        }
    }
}

/// A clause instance's variables by name: what a bridge's λ is substituted
/// from (§S5 step 1).
type Names = Rc<[(String, Term)]>;

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

/// One goal of the goal list. `Cut`, `Collect` and `Asked` are §S4's internal
/// goals: a kind of their own, not terms, so a predicate named `$cut` is
/// ordinary.
#[derive(Debug, Clone, Copy)]
enum Goal {
    Call(Term),
    /// Truncate the choicepoint stack to this height.
    Cut(usize),
    /// Copy the template of the `Collect` choicepoint at this height.
    Collect(usize),
    /// Mark the `Ask` choicepoint at this height answered (§S16).
    Asked(usize),
    Fail,
}

/// A goal, the rest of the list after it, and the clause instance it belongs
/// to (an index into the solver's scopes; 0 is the query's).
///
/// `proof` is the goal's proof slot (§S15): a fresh variable the goal's
/// justification is bound to, trailed, so backtracking undoes it. `None`
/// when proofs are not kept, and for the internal goals.
#[derive(Debug, Clone, Copy)]
struct Node {
    goal: Goal,
    next: Option<usize>,
    scope: usize,
    proof: Option<Term>,
}

/// A goal to put on the goal list, and its proof slot.
type Slotted = (Goal, Option<Term>);

/// What a choicepoint holds besides its marks, and what it resumes (§S4).
enum Kind {
    /// `asked` is the height of the `Ask` beneath, for an askable call.
    Clauses {
        call: Term,
        pred: usize,
        cands: Rc<[usize]>,
        next: usize,
        cont: Option<usize>,
        proof: Option<Term>,
        asked: Option<usize>,
    },
    /// `or`'s branches after the first; `next` 0 is the second branch.
    Branches {
        goal: Term,
        branches: Rc<[Term]>,
        next: usize,
        cont: Option<usize>,
        proof: Option<Term>,
    },
    Items {
        var: Term,
        items: Rc<[Term]>,
        next: usize,
        cont: Option<usize>,
    },
    /// `not`'s goal had no solution: resume.
    Not {
        goal: Term,
        cont: Option<usize>,
        proof: Option<Term>,
    },
    /// `if`'s condition had no solution: run the else branch.
    Else {
        goal: Term,
        otherwise: Term,
        cont: Option<usize>,
        proof: Option<Term>,
    },
    /// `findall`'s goal is exhausted: unify the copies with the list.
    /// `inner` is the goal's own proof slot, and `proofs` its copies, one
    /// beside each result, when proofs are kept.
    Collect {
        goal: Term,
        template: Term,
        list: Term,
        results: Vec<Term>,
        inner: Option<Term>,
        proofs: Vec<Term>,
        cont: Option<usize>,
        proof: Option<Term>,
    },
    /// An askable call: ask, unless one of its clauses answered it (§S16).
    /// `answered` is set by `$asked`, and is not trailed, so it survives
    /// backtracking into the later clauses.
    Ask {
        call: Term,
        answered: bool,
        cont: Option<usize>,
        proof: Option<Term>,
    },
}

struct Choicepoint {
    trail: usize,
    mark: Mark,
    difs: DifMark,
    goals: usize,
    /// How many clause-instance scopes there were, and which was current.
    scopes: usize,
    scope: usize,
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
    /// Its variables by name, kept only when its body holds a LAMBDA: an
    /// instance needs them only for a bridge to substitute from.
    names: Option<Names>,
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
    /// Named in `askable`, with its clauses if `preds` has any (§S16).
    Askable(Option<usize>),
    /// A later phase's: reserved by it, or named in `tabled`.
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
    dif: Atom,
}

/// One activation: a KB, a goal, and the search between them.
pub struct Solver {
    arena: Arena,
    trail: Trail,
    goals: Vec<Node>,
    cont: Option<usize>,
    cps: Vec<Choicepoint>,
    source: BTreeMap<String, BundValue>,
    tabled: Vec<String>,
    askable: Vec<String>,
    known: HashMap<(Atom, usize), Known>,
    preds: Vec<Pred>,
    control: Control,
    cap: Option<u64>,
    count: u64,
    warned: bool,
    names: Vec<(String, Term)>,
    /// `kb.call`'s arguments, whose instances are its answers.
    args: Option<Vec<Term>>,
    /// Each clause instance's variables that a bridge may need, by name; the
    /// query's first. Cut back on backtrack with the choicepoint's count.
    scopes: Vec<Names>,
    scope: usize,
    vars: HashMap<Term, Term>,
    /// Set when proofs are kept (`kb.prove`, `query`'s `prove`).
    proofs: Option<Proofs>,
    /// The remembered replies (§S16), each ground goal in the template
    /// store, which no backtrack cuts; and the same pairs as values, what
    /// `known` is written with.
    replies: Vec<(Term, bool)>,
    told: Vec<BundValue>,
    /// The `dif`s posted, and their attributes (§S17).
    difs: Difs,
    started: bool,
    done: bool,
}

impl Solver {
    /// An activation of `goal` over `kb`. Reads `preds`, `tabled`,
    /// `askable` and `inferences` (E02, and E59 for a predicate both tabled
    /// and askable), and the goal (E04, E05, E26).
    pub fn new(kb: &Kb, goal: &BundValue) -> Result<Solver, Stop> {
        Solver::build(kb, |arena, scope| Ok((arena.to_term(goal, scope)?, None)))
    }

    /// An activation of `functor(args…)` over `kb`, whose answers are each
    /// the arguments as instantiated, a LIST (`kb.call`, L5).
    pub fn for_call(kb: &Kb, functor: &str, args: &[BundValue]) -> Result<Solver, Stop> {
        Solver::build(kb, |arena, scope| {
            let ts = args
                .iter()
                .map(|a| arena.to_term(a, scope))
                .collect::<Result<Vec<_>, _>>()?;
            let f = arena.atoms.intern(functor);
            Ok((arena.structure(f, &ts), Some(ts)))
        })
    }

    fn build(
        kb: &Kb,
        goal: impl FnOnce(&mut Arena, &mut Scope) -> Result<(Term, Option<Vec<Term>>), Refusal>,
    ) -> Result<Solver, Stop> {
        let source = kb.preds()?;
        let tabled = kb.names("tabled")?;
        let askable = kb.names("askable")?;
        if let Some(both) = tabled.iter().find(|n| askable.contains(n)) {
            return Err(Refused::Both(both.clone()).into());
        }
        let cap = kb.inferences()?;
        let mut arena = Arena::default();
        let mut scope = Scope::default();
        let (t, args) = goal(&mut arena, &mut scope).map_err(Refused::from)?;
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
            dif: a("dif"),
        };
        Ok(Solver {
            arena,
            trail: Trail::default(),
            goals: vec![Node {
                goal: Goal::Call(t),
                next: None,
                scope: 0,
                proof: None,
            }],
            cont: Some(0),
            cps: Vec::new(),
            source,
            tabled,
            askable,
            known: HashMap::new(),
            preds: Vec::new(),
            control,
            cap,
            count: 0,
            warned: false,
            names: scope.names().to_vec(),
            args,
            scopes: vec![scope.names().into()],
            scope: 0,
            vars: HashMap::new(),
            proofs: None,
            replies: Vec::new(),
            told: Vec::new(),
            difs: Difs::default(),
            started: false,
            done: false,
        })
    }

    /// Keep proofs (§S15): each answer becomes `{ answer, proof }`. Called
    /// before the first [`Solver::next`].
    pub fn proving(mut self) -> Solver {
        let proofs = Proofs::new(&mut self.arena);
        if let Some(n) = self.goals.first_mut() {
            n.proof = Some(proofs.root);
        }
        self.proofs = Some(proofs);
        self
    }

    /// Start from the replies `var` holds, `held` (§S16's `known`): a LIST
    /// of `[ goal reply ]` pairs, each goal ground and each reply `true` or
    /// `false`, read as terms. A variable that does not exist yet, `None`,
    /// starts empty. Anything else is E58. Called before the first
    /// [`Solver::next`].
    pub fn remembering(mut self, var: &str, held: Option<&BundValue>) -> Result<Solver, Stop> {
        let Some(held) = held else {
            return Ok(self);
        };
        let refused = || {
            Stop::Code(format!(
                "known: {var} must hold a LIST of ground [ goal reply ] pairs, found {}",
                held.summary(SHOWN)
            ))
        };
        let pairs = held
            .as_list()
            .filter(|_| held.dt() == bund2_value::LIST)
            .ok_or_else(refused)?;
        for pair in pairs {
            let Some([goal, reply]) = pair.as_list().filter(|_| pair.dt() == bund2_value::LIST)
            else {
                return Err(refused());
            };
            let mark = self.arena.mark();
            let read = self.arena.to_term(goal, &mut Scope::default());
            let ground = read.is_ok_and(|g| self.arena.is_ground(g));
            let reply = self.reply(reply);
            self.arena.reset(mark);
            let (true, Some(reply)) = (ground, reply) else {
                return Err(refused());
            };
            let g = self
                .arena
                .to_template(goal, &mut Scope::default())
                .map_err(Refused::from)?;
            self.replies.push((g, reply));
            self.told.push(pair.clone());
        }
        Ok(self)
    }

    /// A reply read as a term (§S16): `true` or `false`, so a BOOL, the
    /// CALLs and the STRINGs all count. Anything else is `None`.
    fn reply(&mut self, v: &BundValue) -> Option<bool> {
        let mark = self.arena.mark();
        let t = self.arena.to_term(v, &mut Scope::default()).ok();
        let b = t.and_then(|t| match self.arena.cell(self.arena.deref(t)) {
            Cell::Atom(a) => match self.arena.atoms.text(a) {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            },
            _ => None,
        });
        self.arena.reset(mark);
        b
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

    fn answer(&mut self) -> BundValue {
        if let Some(args) = &self.args {
            return BundValue::list(args.iter().map(|a| self.arena.to_value(*a)).collect());
        }
        let mut map: Vec<(String, BundValue)> = self
            .names
            .iter()
            .filter(|(n, _)| !n.starts_with('_'))
            .map(|(n, t)| (n.clone(), self.arena.to_value(*t)))
            .collect();
        let residual = self.residual();
        if !residual.is_empty() {
            map.push(("_residual".to_string(), BundValue::list(residual)));
        }
        let answer = BundValue::map(map.into_iter().collect());
        match self.proofs {
            None => answer,
            Some(p) => BundValue::map(
                [
                    ("answer".to_string(), answer),
                    ("proof".to_string(), self.arena.to_value(p.root)),
                ]
                .into_iter()
                .collect(),
            ),
        }
    }

    /// The suspended `dif`s, each as the goal `[ dif A B ]` that re-posted
    /// restores it (§S8, L15). A query variable reads as a PTR of its name,
    /// any other as a PTR `_G`*n*.
    fn residual(&mut self) -> Vec<BundValue> {
        let pending = self.difs.residual(&mut self.arena, &mut self.trail);
        let named = |v: Term| {
            let name = self
                .names
                .iter()
                .find(|(_, t)| self.arena.deref(*t) == v)
                .map_or_else(|| format!("_G{}", v.id()), |(n, _)| n.clone());
            BundValue::ptr(name)
        };
        pending
            .into_iter()
            .map(|(a, b)| {
                BundValue::list(vec![
                    BundValue::call("dif"),
                    self.arena.to_value_with(a, &named),
                    self.arena.to_value_with(b, &named),
                ])
            })
            .collect()
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
            self.scope = node.scope;
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
                Goal::Asked(h) => {
                    match self.cps.get_mut(h).map(|c| &mut c.kind) {
                        Some(Kind::Ask { answered, .. }) => *answered = true,
                        _ => {
                            return Err(Stop::Raw(Error::internal(
                                "kb: an $asked goal outlived its Ask choicepoint",
                            )));
                        }
                    }
                    true
                }
                Goal::Call(t) => {
                    self.infer(bridge)?;
                    self.call(t, node.proof, bridge)?
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
        if POLLED && let Some(trip) = bund2_api::memguard::take() {
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

    /// Unify, and run the wake-ups of what that bound (§S4, §S17). Every
    /// unification the search makes goes through here; a trial's does not.
    fn unify(&mut self, a: Term, b: Term) -> bool {
        unify_waking(&mut self.arena, &mut self.trail, &mut self.difs, a, b)
    }

    fn push(&mut self, goal: Goal, proof: Option<Term>, next: Option<usize>) -> Option<usize> {
        self.goals.push(Node {
            goal,
            next,
            scope: self.scope,
            proof,
        });
        Some(self.goals.len() - 1)
    }

    /// `goals` in order, ahead of `rest`.
    fn prepend(&mut self, goals: &[Slotted], rest: Option<usize>) -> Option<usize> {
        let mut cont = rest;
        for (g, proof) in goals.iter().rev() {
            cont = self.push(*g, *proof, cont);
        }
        cont
    }

    /// A fresh proof slot when proofs are kept and the goal has one to fill.
    fn slot(&mut self, parent: Option<Term>) -> Option<Term> {
        parent.map(|_| self.arena.var())
    }

    /// `goals` as calls, each with a fresh slot when `parent` has one.
    fn slotted(&mut self, goals: &[Term], parent: Option<Term>) -> Vec<Slotted> {
        goals
            .iter()
            .map(|g| (Goal::Call(*g), self.slot(parent)))
            .collect()
    }

    /// Bind `slot` to `[ goal ← parts… ]` (§S15), when there is a slot.
    fn justify(&mut self, slot: Option<Term>, goal: Term, parts: &[Term]) {
        let (Some(slot), Some(p)) = (slot, self.proofs) else {
            return;
        };
        let node = p.node(&mut self.arena, goal, parts);
        self.trail.bind_slot(&mut self.arena, slot, node);
    }

    /// The marker atom `pick` names, when proofs are kept.
    fn marker(&self, pick: fn(&Proofs) -> Term) -> Option<Term> {
        self.proofs.as_ref().map(pick)
    }

    /// The parts' slots, as terms, for `justify`.
    fn parts(goals: &[Slotted]) -> Vec<Term> {
        goals.iter().filter_map(|(_, p)| *p).collect()
    }

    fn push_cp(&mut self, kind: Kind) {
        self.cps.push(Choicepoint {
            trail: self.trail.mark(),
            mark: self.arena.mark(),
            difs: self.difs.mark(),
            goals: self.goals.len(),
            scopes: self.scopes.len(),
            scope: self.scope,
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
            self.difs.undo_to(cp.difs);
            self.arena.reset(cp.mark);
            self.goals.truncate(cp.goals);
            self.scopes.truncate(cp.scopes);
            self.scope = cp.scope;
            match &mut cp.kind {
                Kind::Clauses {
                    call,
                    pred,
                    cands,
                    next,
                    cont,
                    proof,
                    asked,
                } => {
                    let (call, pred, cont, proof, asked) = (*call, *pred, *cont, *proof, *asked);
                    let ci = cands.get(*next).copied();
                    *next += 1;
                    if *next < cands.len() {
                        self.cps.push(cp);
                    }
                    let Some(ci) = ci else { continue };
                    self.infer(bridge)?;
                    if self.try_clause(call, pred, ci, cont, proof, asked) {
                        return Ok(true);
                    }
                }
                Kind::Ask {
                    call,
                    answered,
                    cont,
                    proof,
                } => {
                    if *answered {
                        continue;
                    }
                    let (call, cont, proof) = (*call, *cont, *proof);
                    self.infer(bridge)?;
                    if self.ask(call, proof, bridge)? {
                        self.cont = cont;
                        return Ok(true);
                    }
                }
                Kind::Branches {
                    goal,
                    branches,
                    next,
                    cont,
                    proof,
                } => {
                    let (goal, b, cont, proof) =
                        (*goal, branches.get(*next).copied(), *cont, *proof);
                    // The first branch is 1, as `goal i get` finds it.
                    let i = *next + 2;
                    *next += 1;
                    if *next < branches.len() {
                        self.cps.push(cp);
                    }
                    let Some(b) = b else { continue };
                    self.cont = self.branch(goal, b, i, proof, cont);
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
                    if self.unify(var, item) {
                        self.cont = cont;
                        return Ok(true);
                    }
                }
                Kind::Not { goal, cont, proof } => {
                    let (goal, proof) = (*goal, *proof);
                    let no = self.marker(|p| p.no_proof);
                    self.justify(proof, goal, &[no].into_iter().flatten().collect::<Vec<_>>());
                    self.cont = *cont;
                    return Ok(true);
                }
                Kind::Else {
                    goal,
                    otherwise,
                    cont,
                    proof,
                } => {
                    let (goal, otherwise, cont, proof) = (*goal, *otherwise, *cont, *proof);
                    let p_else = self.slot(proof);
                    let parts: Vec<Term> = [self.marker(|p| p.else_), p_else]
                        .into_iter()
                        .flatten()
                        .collect();
                    self.justify(proof, goal, &parts);
                    self.cont = self.push(Goal::Call(otherwise), p_else, cont);
                    return Ok(true);
                }
                Kind::Collect {
                    goal,
                    list,
                    results,
                    proofs,
                    cont,
                    proof,
                    ..
                } => {
                    let (goal, list, cont, proof) = (*goal, *list, *cont, *proof);
                    let mut copies = Vec::with_capacity(results.len());
                    let mut proved = Vec::with_capacity(proofs.len());
                    for (i, r) in results.iter().enumerate() {
                        self.vars.clear();
                        copies.push(self.arena.copy(*r, &mut self.vars, false));
                        if let Some(p) = proofs.get(i) {
                            proved.push(self.arena.copy(*p, &mut self.vars, false));
                        }
                    }
                    let mut l = self.arena.nil();
                    for c in copies.into_iter().rev() {
                        l = self.arena.cons(c, l);
                    }
                    if self.unify(list, l) {
                        let parts: Vec<Term> = self
                            .marker(|p| p.findall)
                            .into_iter()
                            .chain(proved)
                            .collect();
                        self.justify(proof, goal, &parts);
                        self.cont = cont;
                        return Ok(true);
                    }
                }
            }
        }
    }

    /// `$collect`: copy the template, as it now stands, past backtracking;
    /// and the goal's proof beside it, when proofs are kept, sharing the
    /// copy's variables.
    fn collect(&mut self, h: usize) -> Result<(), Stop> {
        let (template, inner) = match self.cps.get(h).map(|c| &c.kind) {
            Some(Kind::Collect {
                template, inner, ..
            }) => (*template, *inner),
            _ => {
                return Err(Stop::Raw(Error::internal(
                    "kb: a $collect goal outlived its findall choicepoint",
                )));
            }
        };
        self.vars.clear();
        let copy = self.arena.copy(template, &mut self.vars, true);
        let proof = inner.map(|p| self.arena.copy(p, &mut self.vars, true));
        if let Some(Choicepoint {
            kind: Kind::Collect {
                results, proofs, ..
            },
            ..
        }) = self.cps.get_mut(h)
        {
            results.push(copy);
            proofs.extend(proof);
        }
        Ok(())
    }

    /// Run `or`'s branch `i` of `goal` (§S15: `[ goal ← :branch i p ]`).
    fn branch(
        &mut self,
        goal: Term,
        b: Term,
        i: usize,
        proof: Option<Term>,
        cont: Option<usize>,
    ) -> Option<usize> {
        let pb = self.slot(proof);
        if proof.is_some() {
            let n = self.arena.int(i64::try_from(i).unwrap_or(i64::MAX));
            let parts: Vec<Term> = [self.marker(|p| p.branch), Some(n), pb]
                .into_iter()
                .flatten()
                .collect();
            self.justify(proof, goal, &parts);
        }
        self.push(Goal::Call(b), pb, cont)
    }

    /// Run one goal, binding `proof`, its slot, as §S15 justifies it.
    /// `false` means it failed.
    fn call(
        &mut self,
        t: Term,
        proof: Option<Term>,
        bridge: &mut dyn Bridge,
    ) -> Result<bool, Stop> {
        let d = self.arena.deref(t);
        let rest = self.cont;
        let (f, args) = match self.arena.cell(d) {
            Cell::Var(_) => return Err(self.unbound(d, d)),
            Cell::Cons(..) => {
                let (items, end) = self.arena.spine(d);
                return match self.arena.cell(end) {
                    Cell::Nil => {
                        // A LIST of goals is proved as a rule body is.
                        let goals = self.slotted(&items, proof);
                        self.justify(proof, d, &Self::parts(&goals));
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
        let builtin: Vec<Term> = self.marker(|p| p.builtin).into_iter().collect();
        match (f, args.as_slice()) {
            (f, []) if f == c.true_ => {
                self.justify(proof, d, &builtin);
                Ok(true)
            }
            (f, []) if f == c.fail => Ok(false),
            (f, [a, b]) if f == c.eq => {
                let ok = self.unify(*a, *b);
                if ok {
                    self.justify(proof, d, &builtin);
                }
                Ok(ok)
            }
            (f, [a, b]) if f == c.dif => {
                let ok = self.difs.post(&mut self.arena, &mut self.trail, *a, *b);
                if ok {
                    self.justify(proof, d, &builtin);
                }
                Ok(ok)
            }
            (f, gs) if f == c.not => {
                let h = self.cps.len();
                self.push_cp(Kind::Not {
                    goal: d,
                    cont: rest,
                    proof,
                });
                // G's own proofs are never read: `not` keeps none.
                let mut goals = self.slotted(gs, None);
                goals.extend([(Goal::Cut(h), None), (Goal::Fail, None)]);
                self.cont = self.prepend(&goals, None);
                Ok(true)
            }
            (f, gs) if f == c.once => {
                let h = self.cps.len();
                let mut goals = self.slotted(gs, proof);
                self.justify(proof, d, &Self::parts(&goals));
                goals.push((Goal::Cut(h), None));
                self.cont = self.prepend(&goals, rest);
                Ok(true)
            }
            (f, branches) if f == c.or => {
                let Some((first, others)) = branches.split_first() else {
                    return Ok(false);
                };
                if !others.is_empty() {
                    self.push_cp(Kind::Branches {
                        goal: d,
                        branches: others.into(),
                        next: 0,
                        cont: rest,
                        proof,
                    });
                }
                self.cont = self.branch(d, *first, 1, proof, rest);
                Ok(true)
            }
            (f, [cond, then, otherwise]) if f == c.if_ => {
                let h = self.cps.len();
                self.push_cp(Kind::Else {
                    goal: d,
                    otherwise: *otherwise,
                    cont: rest,
                    proof,
                });
                let (pc, pt) = (self.slot(proof), self.slot(proof));
                let parts: Vec<Term> = [self.marker(|p| p.then_), pc, pt]
                    .into_iter()
                    .flatten()
                    .collect();
                self.justify(proof, d, &parts);
                self.cont = self.prepend(
                    &[
                        (Goal::Call(*cond), pc),
                        (Goal::Cut(h), None),
                        (Goal::Call(*then), pt),
                    ],
                    rest,
                );
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
                // `call` is transparent: the called goal fills its slot.
                self.cont = self.push(Goal::Call(goal), proof, rest);
                Ok(true)
            }
            (f, [template, goal, list]) if f == c.findall => {
                let h = self.cps.len();
                let inner = self.slot(proof);
                self.push_cp(Kind::Collect {
                    goal: d,
                    template: *template,
                    list: *list,
                    results: Vec::new(),
                    inner,
                    proofs: Vec::new(),
                    cont: rest,
                    proof,
                });
                self.cont = self.prepend(
                    &[
                        (Goal::Call(*goal), inner),
                        (Goal::Collect(h), None),
                        (Goal::Fail, None),
                    ],
                    None,
                );
                Ok(true)
            }
            (f, [x, l]) if f == c.in_ => self.member(*x, *l, d, proof, bridge),
            (f, [x, lambda]) if f == c.is => {
                self.bridge(Bridging::Is, Some(*x), *lambda, d, proof, bridge)
            }
            (f, [lambda]) if f == c.test => {
                self.bridge(Bridging::Test, None, *lambda, d, proof, bridge)
            }
            (f, _) => {
                let n = args.len();
                match self.lookup(f, n)? {
                    Known::User(p) => Ok(self.resolve(d, p, &args, proof, None)),
                    Known::Askable(p) => {
                        // `Ask` first, then `Clauses` above it: the question
                        // comes only once every clause has been tried.
                        let h = self.cps.len();
                        self.push_cp(Kind::Ask {
                            call: d,
                            answered: false,
                            cont: rest,
                            proof,
                        });
                        Ok(p.is_some_and(|p| self.resolve(d, p, &args, proof, Some(h))))
                    }
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

    /// `[ in X L ]` over a LIST; over a λ, a bridge.
    fn member(
        &mut self,
        x: Term,
        l: Term,
        goal: Term,
        proof: Option<Term>,
        bridge: &mut dyn Bridge,
    ) -> Result<bool, Stop> {
        let l = self.arena.deref(l);
        match self.arena.cell(l) {
            Cell::Opaque(i) if self.arena.opaque_value(i).is_some_and(|v| v.dt() == LAMBDA) => {
                return self.bridge(Bridging::In, Some(x), l, goal, proof, bridge);
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
        // Justified before `Items` is pushed: every item has the same proof,
        // so a backtrack to the next item keeps it.
        let builtin: Vec<Term> = self.marker(|p| p.builtin).into_iter().collect();
        self.justify(proof, goal, &builtin);
        Ok(self.each(x, &items))
    }

    /// X takes each of `items` on backtracking: `Items` holds the rest.
    fn each(&mut self, x: Term, items: &[Term]) -> bool {
        let Some((first, others)) = items.split_first() else {
            return false;
        };
        if !others.is_empty() {
            self.push_cp(Kind::Items {
                var: x,
                items: others.into(),
                next: 0,
                cont: self.cont,
            });
        }
        self.unify(x, *first)
    }

    /// A bridge (§S5): substitute the clause instance's variables into λ,
    /// run it, and read what it left as `how` says.
    fn bridge(
        &mut self,
        how: Bridging,
        x: Option<Term>,
        lambda: Term,
        goal: Term,
        proof: Option<Term>,
        bridge: &mut dyn Bridge,
    ) -> Result<bool, Stop> {
        let lambda = self.arena.deref(lambda);
        let body = match self.arena.cell(lambda) {
            Cell::Var(_) => return Err(self.unbound(lambda, goal)),
            Cell::Opaque(i) => match self.arena.opaque_value(i) {
                Some(v) if v.dt() == LAMBDA => v.as_lambda().unwrap_or(&[]).to_vec(),
                _ => return Err(self.not_callable(lambda)),
            },
            _ => return Err(self.not_callable(lambda)),
        };
        let scope = self
            .scopes
            .get(self.scope)
            .cloned()
            .unwrap_or_else(|| Rc::from([]));
        let arena = &self.arena;
        let body = super::bridge::substitute(&body, &|name| {
            scope.iter().find(|(n, _)| n == name).map(|(_, v)| {
                let v = arena.deref(*v);
                match arena.cell(v) {
                    Cell::Var(_) => None,
                    _ => Some(arena.to_value(v)),
                }
            })
        })
        .map_err(|var| {
            Stop::Code(format!(
                "instantiation error: {var} is unbound in the lambda of {}",
                self.show(goal)
            ))
        })?;
        // λ′, the substituted λ, which re-run reproduces the binding.
        let shown = proof.map(|_| BundValue::lambda(body.clone()));
        let left = bridge.run(body).map_err(|e| {
            Stop::Code(format!(
                "bridge lambda in {} returns: {}",
                self.show(goal),
                e.0
            ))
        })?;
        let Some(left) = left else {
            return Err(Stop::Code(format!(
                "{}: the lambda left nothing in {}",
                how.name(),
                self.show(goal)
            )));
        };
        if let Some(l) = shown {
            // Justified before the outcome binds anything, so `in`'s items
            // share it, as they do over a LIST.
            let parts: Vec<Term> = [self.marker(|p| p.bridge), Some(self.arena.opaque(l))]
                .into_iter()
                .flatten()
                .collect();
            self.justify(proof, goal, &parts);
        }
        match how {
            Bridging::Test => match left.unboxed() {
                BundValue::Bool(b, _) => Ok(*b),
                _ => Err(Stop::Code(format!(
                    "test: expected BOOL, found {} in {}",
                    left.type_name(),
                    self.show(goal)
                ))),
            },
            Bridging::Is => {
                let t = self
                    .arena
                    .to_term(&left, &mut Scope::default())
                    .map_err(Refused::from)?;
                Ok(x.is_some_and(|x| self.unify(x, t)))
            }
            Bridging::In => {
                let Some(items) = left.as_list().filter(|_| left.dt() == bund2_value::LIST) else {
                    return Err(Stop::Code(format!(
                        "in: expected a list, found {} in {}",
                        left.summary(SHOWN),
                        self.show(goal)
                    )));
                };
                let mut scope = Scope::default();
                let terms = items
                    .iter()
                    .map(|v| self.arena.to_term(v, &mut scope))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(Refused::from)?;
                Ok(x.is_some_and(|x| self.each(x, &terms)))
            }
        }
    }

    /// No clause answered an askable call: ask (§S16). The goal must be
    /// ground (E51). A remembered reply that unifies with it is used; with
    /// none, the `ask` lambda runs (E50 when there is none, E52, E53, E57),
    /// and its reply is remembered and written through. `true` succeeds,
    /// once; `false` fails.
    fn ask(
        &mut self,
        call: Term,
        proof: Option<Term>,
        bridge: &mut dyn Bridge,
    ) -> Result<bool, Stop> {
        if !self.arena.is_ground(call) {
            return Err(Stop::Code(format!(
                "ask: {} must be ground",
                self.show(call)
            )));
        }
        let mut remembered = None;
        for (g, reply) in &self.replies {
            let bound = self.trail.mark();
            let hit = unify(&mut self.arena, &mut self.trail, call, *g);
            self.trail.undo_to(&mut self.arena, bound);
            if hit {
                remembered = Some(*reply);
                break;
            }
        }
        let (yes, how) = match remembered {
            Some(yes) => (yes, self.marker(|p| p.known)),
            None => {
                let goal = self.arena.to_value(call);
                let Some(ran) = bridge.ask(goal.clone()) else {
                    return Err(Stop::Code(format!(
                        "{} is askable, and this query has no ask lambda",
                        self.key_of(call)
                    )));
                };
                let left = ran.map_err(|e| Stop::Code(format!("ask lambda returns: {}", e.0)))?;
                let Some(left) = left else {
                    return Err(Stop::Code(format!(
                        "ask: the lambda left nothing for {}",
                        self.show(call)
                    )));
                };
                let Some(yes) = self.reply(&left) else {
                    return Err(Stop::Code(format!(
                        "ask: expected true or false, found {} for {}",
                        left.summary(SHOWN),
                        self.show(call)
                    )));
                };
                self.vars.clear();
                let g = self.arena.copy(call, &mut self.vars, true);
                self.replies.push((g, yes));
                self.told
                    .push(BundValue::list(vec![goal, BundValue::boolean(yes)]));
                bridge.remember(&self.told);
                (yes, self.marker(|p| p.asked))
            }
        };
        if yes {
            let parts: Vec<Term> = [how, self.marker(|p| p.yes)]
                .into_iter()
                .flatten()
                .collect();
            self.justify(proof, call, &parts);
        }
        Ok(yes)
    }

    /// A callable term's `functor/arity`, as E50 names it.
    fn key_of(&self, t: Term) -> String {
        match self.arena.cell(self.arena.deref(t)) {
            Cell::Atom(a) => format!("{}/0", self.arena.atoms.text(a)),
            Cell::Struct { functor, arity, .. } => {
                format!("{}/{arity}", self.arena.atoms.text(functor))
            }
            _ => self.show(t),
        }
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
            _ if self.tabled.contains(&key) => Known::Unbuilt(2),
            _ if self.askable.contains(&key) => Known::Askable(if self.source.contains_key(&key) {
                Some(self.compile(&key)?)
            } else {
                None
            }),
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
            let names = holds_lambda(goals_v).then(|| scope.names().into());
            clauses.push(Clause {
                head,
                body: body.into(),
                key,
                names,
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
    /// two or more remain, and try the first. `asked` is the height of an
    /// askable call's `Ask`, which each clause body marks when it succeeds.
    fn resolve(
        &mut self,
        call: Term,
        p: usize,
        args: &[Term],
        proof: Option<Term>,
        asked: Option<usize>,
    ) -> bool {
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
                proof,
                asked,
            });
        }
        self.try_clause(call, p, first, rest, proof, asked)
    }

    /// Rename clause `ci` of `p`, unify its head with `call`, and put its
    /// body ahead of `rest`, followed by `$asked(h)` for an askable call.
    fn try_clause(
        &mut self,
        call: Term,
        p: usize,
        ci: usize,
        rest: Option<usize>,
        proof: Option<Term>,
        asked: Option<usize>,
    ) -> bool {
        let Some(clause) = self.preds.get(p).and_then(|pr| pr.clauses.get(ci)) else {
            return false;
        };
        let (head, body, names) = (clause.head, clause.body.clone(), clause.names.clone());
        self.vars.clear();
        let h = self.arena.copy(head, &mut self.vars, false);
        if !self.unify(call, h) {
            return false;
        }
        let mut goals: Vec<Slotted> = body
            .iter()
            .map(|g| {
                let g = self.arena.copy(*g, &mut self.vars, false);
                (Goal::Call(g), proof.map(|_| self.arena.var()))
            })
            .collect();
        // `[ goal ← :fact ]`, or `[ goal ← p₁ … pₖ ]` for a rule (§S15).
        let parts = match self.marker(|p| p.fact) {
            Some(fact) if goals.is_empty() => vec![fact],
            _ => Self::parts(&goals),
        };
        self.justify(proof, call, &parts);
        goals.extend(asked.map(|h| (Goal::Asked(h), None)));
        if let Some(names) = names {
            // This instance's variables, for a bridge in its body.
            let instance: Names = names
                .iter()
                .map(|(n, v)| (n.clone(), self.vars.get(v).copied().unwrap_or(*v)))
                .collect();
            self.scopes.push(instance);
            self.scope = self.scopes.len() - 1;
        }
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

    /// A variable's name: the query's, else the current clause instance's,
    /// else `_G`*n*.
    fn var_name(&self, v: Term) -> String {
        let instance = self.scopes.get(self.scope).map_or(&[][..], |s| &s[..]);
        self.names
            .iter()
            .chain(instance)
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
}

/// Whether any of `values` holds a LAMBDA, at any depth: a clause whose body
/// does may run a bridge, and keeps its variables' names for it.
fn holds_lambda(values: &[BundValue]) -> bool {
    let mut todo: Vec<&BundValue> = values.iter().collect();
    while let Some(v) = todo.pop() {
        if v.dt() == LAMBDA {
            return true;
        }
        if let Some(items) = v.as_list() {
            todo.extend(items);
        }
    }
    false
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
            "KB.HOLDS: malformed knowledge base: inferences is not an INT"
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
            err("kb.new [ label X ] kb.query"),
            "KB.QUERY: label/1 belongs to Phase 3, which is not built"
        );
        // `dif/2`, Phase 2's one reserved pair, is M9's (`dif.rs`'s tests).
        // A predicate named in `tabled` (the implementation plan), whether
        // or not it has clauses, until M10. `askable` is M8's (`query.rs`'s
        // tests).
        assert_eq!(
            err("[ [ p 1 ] ] kb! :tabled [ \"p/1\" ] set [ p X ] kb.query"),
            "KB.QUERY: p/1 belongs to Phase 2, which is not built"
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

    /// **Criterion 16's switches are off in a shipped build.** The occurs
    /// check (L7) and the guard poll (S4) are compiled out only by the
    /// measurement-only features `kb-unchecked-bench` and
    /// `kb-unpolled-bench`, which nothing enables but `bund2-bench`.
    #[test]
    fn the_shipped_engine_checks_and_polls() {
        // Through `black_box`: a test of what the build compiled in, which
        // clippy would otherwise read as an assertion on a constant.
        let checked = std::hint::black_box(super::super::unify::CHECKED);
        let polled = std::hint::black_box(super::POLLED);
        assert!(checked, "the occurs check is compiled out");
        assert!(polled, "the guard poll is compiled out");
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
            fn run(&mut self, _: Vec<BundValue>) -> Result<Option<BundValue>, bund2_api::Error> {
                Ok(None)
            }
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
