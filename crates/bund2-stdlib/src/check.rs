//! `bund2 check` — static stack-underflow detection. **RFC-0004 §S4.**
//!
//! Walks a lowered program with an abstract stack and reports each site that
//! would take more than it can have. It reports; it never changes what
//! `script` does, and a program it warns about still runs and still fails the
//! way it fails today.
//!
//! # Why the answer is a warning
//!
//! D36's ladder: nothing has stopped, so a finding is a `Warning` — one line,
//! no table, no stack dump. And RFC-0004's alternatives section rejects making
//! `check` fail a build, because `!` is `Opaque` and one of the five most-used
//! words in the corpus: a false-positive rate above zero is certain, and a
//! checker that blocks would be turned off. A warning that is right is worth
//! more than an error that is disabled.
//!
//! # What it tracks, and where it stops
//!
//! One abstract stack per program point: a slot per value, holding what the
//! walk knows about it — its tag, a literal name, a literal body, a class or
//! an object of one — or nothing ([`Abs`]). A literal pushes itself; a call
//! with a declared [`StackEffect`] takes and leaves that many; a block pushes
//! a LAMBDA rather than running it, and the body travels with the slot.
//!
//! Four things end tracking, and they are §S6's barriers rather than
//! shortcuts:
//!
//! - an **`Opaque`** word whose operand the walk cannot see — `apply`,
//!   `lambda*`, `graph!`, and `!` on a slot it knows nothing about;
//! - a **stack switch**, `@name`, because the stack being tracked is no longer
//!   the stack in play;
//! - an **unknown word**, which may be a lambda registered at run time;
//! - a **`CONTEXT`**, which switches for the same reason `@name` does.
//!
//! At each of those the analysis abandons what it knows and does not start
//! again, which is why the count of abandonments is reported alongside the
//! findings (criterion 5). A run that says "no problems" without saying it
//! skipped every `!` is misleading.
//!
//! # One barrier the program itself lowers — D178
//!
//! An `Opaque` word is one whose arity depends on what it is handed. When
//! what it is handed is a **literal in the stream right before the call**,
//! the arity is written in the program, and reading it is not a guess. The
//! one such rule so far is `format`: it pulls its template and then one value
//! per distinct key in it
//! (`reference/rust_multistackvm/src/stdlib/string/format.rs:13-28`), so
//! `"a={A} b={B}" format` is `3 -> 1` wherever it appears, and the same `leon`
//! parser the word runs gives the count. `format` was the barrier the corpus
//! hit most — 134 sites, nearly all with a literal template — and a tracked
//! region now runs through them. A computed template, or one `leon` refuses,
//! leaves `format` as opaque as before.
//!
//! # Tags, and the arms of `!` they open — D179
//!
//! `!` dispatches on the tag of the value it pulls
//! (`reference/rust_multistackvm/src/stdlib/execute.rs:26-98`). When the walk
//! knows that tag it takes the arm the run would take:
//!
//! - a **LAMBDA** whose body is a literal is walked in place, on the same
//!   stack (`:93-94`, `lambda_eval`);
//! - a **PTR, STRING or CALL** whose text is a literal is the word of that
//!   name (`:27-30`, `vm.call`);
//! - a **CLASS** is constructed, the way Bund2's D23 arm does it;
//! - an **OBJECT** pulls a method name from beneath it, pushes the object
//!   back and dispatches (`bund_execute/execute_object.rs:8-20`): the slot is
//!   searched on the class and then its parents, depth first
//!   (`multistackvm_object.rs:6-24`); a LAMBDA slot is walked in place, a PTR
//!   slot is a method native whose [`MethodEffect`] is declared where it is
//!   registered, and any other slot is pushed (`:55-79`).
//!
//! Objects come from `object`, whose class name is a literal in nearly every
//! corpus use, and from `<class> !`. Construction is followed as the run
//! performs it (`bund_object.rs:27-113` and `:115-188`): every parent in
//! `.super` is built first and its `.init` run with the parent object on
//! top, which is pulled again only if it is still there (`:83-94`); then the
//! object itself is pushed, its own `.init` run, and the object left on top
//! whatever the constructor did (`:132-174`). A class the program defines is
//! read as it is built — `class`, then `set` with a literal key on the CLASS,
//! then `register` — so its methods and its `.init` are bodies the walk can
//! enter. The receiver's class survives a method call because each method
//! native says what it does to its receiver, which a plain pair cannot.
//!
//! Nothing is guessed. A computed class name, a computed method name, a slot
//! the class does not have, a parent neither the program nor Bund2 defines,
//! a LIST or CONDITIONAL under `!`, and `.format`'s template-driven arity are
//! each an abandonment with its own reason.

use std::collections::BTreeMap;
use std::rc::Rc;

use bund2_api::{Leaves, MethodEffect, Registry, StackEffect};
use bund2_value::{
    BOOL, BundValue, CALL, CLASS, CONDITIONAL, CONTEXT, FLOAT, INTEGER, LAMBDA, LIST, MAP, NODATA,
    OBJECT, PTR, STRING,
};

/// One site that would take more than the stack can give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Index into the lowered stream, so a caller can map it to a span. For a
    /// finding inside a body the walk entered — a lambda `!` ran, a method, a
    /// constructor — this is the site that entered it, and `via` says so.
    pub at: usize,
    pub word: String,
    /// What the word needs.
    pub needs: u8,
    /// What the analysis could prove was there.
    pub have: i64,
    /// The body the finding is inside, when it is not at `at` itself — D179.
    pub via: Option<String>,
}

/// What a walk learned.
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// Sites the analysis could reason about.
    pub analysed: usize,
    /// Sites where it gave up: an `Opaque` word, a stack switch, or a name it
    /// does not know. **Reported, never hidden** — criterion 5.
    pub abandoned: usize,
    /// Why it gave up, most common first. For the report's prose.
    pub reasons: Vec<(String, usize)>,
}

/// Bind the words a program registers with a **literal** body, before walking.
///
/// **Q23, answered.** `check` runs on a fresh registry, so without this it
/// abandons at every word the program defines — which is most of what a real
/// program calls. The corpus idiom is `:Name { … } register`, three adjacent
/// values in the lowered stream, and binding those is reading the program
/// rather than guessing about it.
///
/// It deliberately handles **only** the literal form. `<computed> <lambda>
/// register` is D16's open world and stays unknown; so does a body built at
/// run time. Recognising a shape that is actually there is not the same as
/// assuming one that might be.
fn prebind(stream: &[BundValue], registry: &Registry) -> Registry {
    let mut r = registry.clone();
    for w in stream.windows(3) {
        let (name, body, call) = (&w[0], &w[1], &w[2]);
        if call.dt() != CALL || call.as_str().as_deref() != Some("register") {
            continue;
        }
        if body.dt() != LAMBDA {
            continue;
        }
        let (Some(n), Some(_)) = (name.as_str(), body.as_lambda()) else {
            continue;
        };
        if name.dt() == STRING {
            r.register_lambda(&n, body.clone());
        }
    }
    r
}

/// The effect of a site whose operand is a **literal in the stream** — D178.
///
/// `prev` is the value lowered immediately before the call, which is what the
/// word will find on top when it runs, because a literal does nothing but
/// push itself. `format`'s template is pulled first and then one value per
/// distinct key (`reference/rust_multistackvm/src/stdlib/string/format.rs:13-28`;
/// a repeated key is filled once, `:25-27`), so a STRING there fixes the pair.
///
/// Nothing is said about a template that is computed, or one `leon` does not
/// parse: the run refuses the second with `FORMAT error parsing template`, and
/// that is the run's report to make, not this one's. Only `format` — `format.`
/// takes its template from the workbench, where no literal in this stream
/// puts it.
fn literal_operand_effect(prev: Option<&BundValue>, name: &str) -> Option<StackEffect> {
    if name != "format" {
        return None;
    }
    let prev = prev?;
    if prev.dt() != STRING {
        return None;
    }
    let tpl = prev.as_str()?;
    let template = leon::Template::parse(tpl.as_str()).ok()?;
    let keys: std::collections::BTreeSet<&str> = template.keys().copied().collect();
    let consumes = (1 + keys.len()).min(usize::from(u8::MAX)) as u8;
    Some(StackEffect::fixed(consumes, 1))
}

/// How deep the walk follows bodies into bodies before it gives up.
///
/// A self-recursive word would otherwise be inferred forever. Bund2 does not
/// hang (D39), so this is a bound rather than a guard: past it the answer is
/// `Opaque`, which is the honest reading of "I could not work it out".
const MAX_INFER_DEPTH: u8 = 8;

/// How many generations of `.super` construction follows. The deepest
/// built-in chain is five (`Bool` → `Value` → `Object` → `Printable` →
/// `Display`); a program's hierarchy is not expected to be deeper than this,
/// and one that is becomes an abandonment rather than a stack overflow.
const MAX_ANCESTRY: u8 = 32;

/// What the walk knows about one stack slot — D179.
#[derive(Clone, Debug)]
enum Abs {
    /// Nothing: a value a word produced, or one that arrived from beneath.
    Unknown,
    /// Its tag and nothing more.
    Tagged(u16),
    /// A PTR, STRING or CALL literal: a name the run will see as text.
    Name(u16, Rc<str>),
    /// A literal `{ … }`: a body the walk can enter.
    Lambda(Rc<[BundValue]>),
    /// A literal `[ … ]`, kept for `.super [ :A :B ]`.
    List(Rc<[BundValue]>),
    /// A CLASS being built, or one resolved from the registry.
    Class(Rc<ClassModel>),
    /// An OBJECT, and the class it was constructed from.
    Object(Rc<ClassModel>),
    /// A CONDITIONAL — its `type` as the model's name, its slots as `set`
    /// filled them — D181.
    Cond(Rc<ClassModel>),
}

impl Abs {
    fn of_literal(v: &BundValue) -> Abs {
        match v.dt() {
            t @ (PTR | STRING | CALL) => match v.as_str() {
                Some(s) => Abs::Name(t, Rc::from(s.as_str())),
                None => Abs::Tagged(t),
            },
            LAMBDA => match v.as_lambda() {
                Some(b) => Abs::Lambda(Rc::from(b)),
                None => Abs::Tagged(LAMBDA),
            },
            LIST => match v.as_list() {
                Some(l) => Abs::List(Rc::from(l)),
                None => Abs::Tagged(LIST),
            },
            t => Abs::Tagged(t),
        }
    }

    /// For a reason: what `!` found on top.
    fn describe(&self) -> String {
        match self {
            Abs::Unknown => "a value whose tag is not known".into(),
            Abs::Tagged(t) => format!("a {}", tag_name(*t)),
            Abs::Name(t, n) => format!("`{n}`, a {}", tag_name(*t)),
            Abs::Lambda(_) => "a lambda".into(),
            Abs::List(_) => "a LIST".into(),
            Abs::Class(m) => format!("the class {}", m.display_name()),
            Abs::Object(m) => format!("an object of {}", m.display_name()),
            Abs::Cond(m) => format!("a CONDITIONAL of type {}", m.display_name()),
        }
    }
}

/// A tag's name, for a reason. The common tags; anything else is named by
/// its number, which is what `debug.display_stack` would show too.
fn tag_name(t: u16) -> String {
    match t {
        BOOL => "BOOL".into(),
        INTEGER => "INTEGER".into(),
        FLOAT => "FLOAT".into(),
        STRING => "STRING".into(),
        CALL => "CALL".into(),
        PTR => "PTR".into(),
        LIST => "LIST".into(),
        MAP => "MAP".into(),
        LAMBDA => "LAMBDA".into(),
        CONDITIONAL => "CONDITIONAL".into(),
        CLASS => "CLASS".into(),
        OBJECT => "OBJECT".into(),
        other => format!("value of tag {other}"),
    }
}

/// What the walk knows about a class: its name, once `register` or the
/// registry has given it one, and its slots as the program or Bund2 wrote
/// them — a LAMBDA body, a PTR to a method native, a `.super` list, or a plain
/// value.
#[derive(Debug, Default)]
struct ClassModel {
    name: Option<Rc<str>>,
    slots: BTreeMap<String, Abs>,
}

impl ClassModel {
    /// A registered class, read from its MAP. Every built-in sets
    /// `.class_name`; a program's class is named by `register`.
    fn of_value(name: &str, v: &BundValue) -> Self {
        let mut slots = BTreeMap::new();
        if let Some(m) = v.as_map() {
            for (k, val) in m {
                slots.insert(k.clone(), Abs::of_literal(val));
            }
        }
        Self {
            name: Some(Rc::from(name)),
            slots,
        }
    }

    /// A CONDITIONAL of this `type`, with no slot filled yet.
    fn kind(ty: &str) -> Self {
        Self {
            name: Some(Rc::from(ty)),
            slots: BTreeMap::new(),
        }
    }

    fn bare() -> Self {
        let mut slots = BTreeMap::new();
        slots.insert(".super".to_string(), Abs::List(Rc::from(Vec::new())));
        Self { name: None, slots }
    }

    fn with(&self, key: &str, value: Abs) -> Self {
        let mut slots = self.slots.clone();
        slots.insert(key.to_string(), value);
        Self {
            name: self.name.clone(),
            slots,
        }
    }

    fn named(&self, name: &str) -> Self {
        Self {
            name: Some(Rc::from(name)),
            slots: self.slots.clone(),
        }
    }

    fn display_name(&self) -> String {
        match &self.name {
            Some(n) => format!("`{n}`"),
            None => "an unnamed class".into(),
        }
    }

    /// The parent names in `.super`, in order
    /// (`reference/rust_multistackvm/src/stdlib/bund_object.rs:38-44`).
    fn supers(&self) -> Vec<Rc<str>> {
        match self.slots.get(".super") {
            Some(Abs::List(items)) => items
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| Rc::from(s.as_str()))
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// The same class: the one `Rc`, or two models that carry the same name —
/// construction compares `.class_name` strings (`bund_object.rs:14-21`).
fn same_class(a: &Rc<ClassModel>, b: &Rc<ClassModel>) -> bool {
    Rc::ptr_eq(a, b) || matches!((&a.name, &b.name), (Some(x), Some(y)) if x == y)
}

/// The same knowledge about a slot, for the merge a repeated body needs.
fn same(a: &Abs, b: &Abs) -> bool {
    match (a, b) {
        (Abs::Unknown, Abs::Unknown) => true,
        (Abs::Tagged(x), Abs::Tagged(y)) => x == y,
        (Abs::Name(x, m), Abs::Name(y, n)) => x == y && m == n,
        (Abs::Lambda(x), Abs::Lambda(y)) => Rc::ptr_eq(x, y),
        (Abs::List(x), Abs::List(y)) => Rc::ptr_eq(x, y),
        (Abs::Class(x), Abs::Class(y))
        | (Abs::Object(x), Abs::Object(y))
        | (Abs::Cond(x), Abs::Cond(y)) => same_class(x, y),
        _ => false,
    }
}

/// Why a walk is running: over a program, where taking from an empty stack
/// is a finding, or over a body whose effect is wanted, where it is the
/// body's consumption (RFC-0004 §S3's dip).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Program,
    Infer,
}

/// The abstract stack. `dip` counts what was taken from beneath its bottom —
/// meaningful in `Infer` mode, always zero in `Program` mode, where an empty
/// stack is an empty stack.
#[derive(Default)]
struct Stack {
    items: Vec<Abs>,
    dip: i64,
}

impl Stack {
    fn pop(&mut self) -> Abs {
        self.items.pop().unwrap_or_else(|| {
            self.dip += 1;
            Abs::Unknown
        })
    }

    fn push(&mut self, a: Abs) {
        self.items.push(a);
    }

    fn top(&self) -> Option<&Abs> {
        self.items.last()
    }

    fn push_unknown(&mut self, n: u8) {
        for _ in 0..n {
            self.items.push(Abs::Unknown);
        }
    }

    /// The same depth, nothing known about any slot: a rotation.
    fn forget_tags(&mut self) {
        for a in &mut self.items {
            *a = Abs::Unknown;
        }
    }

    fn copy(&self) -> Stack {
        Stack {
            items: self.items.clone(),
            dip: self.dip,
        }
    }

    /// How much deeper this stack is than `base`, counting what it took
    /// from beneath `base`'s bottom as taken.
    fn net_against(&self, base: &Stack) -> i64 {
        let dipped = (self.dip - base.dip).max(0);
        self.items.len() as i64 - dipped - base.items.len() as i64
    }

    /// Replace this stack by `once`, a run of the same depth over it,
    /// keeping a slot only where the two agree — D180.
    fn keep_what_agrees(&mut self, once: &Stack) {
        let dipped = (once.dip - self.dip).max(0) as usize;
        let mut merged = Vec::with_capacity(once.items.len());
        for (j, after) in once.items.iter().enumerate() {
            let kept = j
                .checked_sub(dipped)
                .and_then(|i| self.items.get(i))
                .filter(|orig| same(orig, after))
                .cloned();
            merged.push(kept.unwrap_or(Abs::Unknown));
        }
        self.items = merged;
        self.dip = once.dip;
    }

    /// Two branches joined: the same depth, and a slot kept only where both
    /// agree — D181. `None` when their depths differ.
    fn join(a: Stack, b: &Stack) -> Option<Stack> {
        if a.items.len() != b.items.len() || a.dip != b.dip {
            return None;
        }
        let items = a
            .items
            .into_iter()
            .zip(b.items.iter())
            .map(|(x, y)| if same(&x, y) { x } else { Abs::Unknown })
            .collect();
        Some(Stack { items, dip: a.dip })
    }
}

/// A CONDITIONAL's slot, as its handler would read it.
enum Slot {
    Lambda(Rc<[BundValue]>),
    /// Missing, or a literal that is not a lambda: an empty body.
    Absent,
    /// A value the walk cannot see into, which may be a lambda at run time.
    Unseen,
}

/// What dispatch found for a method name on a class.
enum Located {
    Lambda(Rc<[BundValue]>),
    Native(Rc<str>),
    Value(Abs),
    Missing,
    /// A parent the walk could not resolve, so the search is inconclusive.
    Unresolved(Rc<str>),
}

struct Walker<'a> {
    registry: &'a Registry,
    /// Words `register` bound while walking, shadowing the registry as F93
    /// says a lambda shadows a native.
    lambdas: BTreeMap<String, Rc<[BundValue]>>,
    /// Classes the program registered, by name. Consulted before the
    /// registry, because `register_class` overrides at run time too.
    classes: BTreeMap<String, Rc<ClassModel>>,
    /// Built-in classes, read from the registry once each.
    builtin: BTreeMap<String, Rc<ClassModel>>,
    report: Report,
    why: BTreeMap<String, usize>,
    mode: Mode,
    /// The named stacks that are not current, each known or forgotten — D182.
    /// A stack never switched to is absent, and is created empty when it is
    /// (`reference/rust_multistack/src/ts_ensure.rs:10-15`).
    stacks: BTreeMap<String, Option<Stack>>,
    /// The name of the stack the walk is on. The program starts on `main`.
    current: String,
    /// The stack to return to at each `endcontext`, pushed at every switch
    /// as `VM::to_stack` pushes it (`reference/rust_multistackvm/src/multistackvm_to_stack.rs:5-19`).
    nesting: Vec<String>,
    /// False while a body runs on a copy of the stack, where a switch would
    /// move the copy under another name.
    may_switch: bool,
    /// `Some` while the `try` slot of a `?try` is walked — D183: what the
    /// `raise`s reached in it leave for `except`. `None` outside one.
    raised: Option<Raised>,
    /// A `raise` inside `try` has ended the path the walk is on: every
    /// enclosing body stops, up to the `?try` that reads this.
    halted: bool,
}

/// What the `raise`s reached so far inside a `try` leave on the stack.
enum Raised {
    /// None reached yet.
    Nowhere,
    /// The stack every `raise` reached agrees on, slot by slot.
    At(Stack),
    /// Two left different depths, so `except` runs at a depth the walk
    /// cannot name.
    Disagree,
}

/// The outcome of one site: the walk goes on, or it has abandoned.
type Went = bool;

impl<'a> Walker<'a> {
    fn new(registry: &'a Registry, mode: Mode) -> Self {
        Self {
            registry,
            lambdas: BTreeMap::new(),
            classes: BTreeMap::new(),
            builtin: BTreeMap::new(),
            report: Report::default(),
            why: BTreeMap::new(),
            mode,
            stacks: BTreeMap::new(),
            current: "main".to_string(),
            nesting: Vec::new(),
            may_switch: true,
            raised: None,
            halted: false,
        }
    }

    /// Switch the walk to the stack called `name`, saving the current one —
    /// `VM::to_stack` through `TS::ensure_stack`, which creates a missing
    /// stack empty (`reference/rust_multistack/src/ts_ensure.rs:10-15`).
    fn switch_to(&mut self, name: &str, stack: &mut Stack) -> Went {
        if self.mode != Mode::Program {
            return self.abandon("a stack switch".into());
        }
        if !self.may_switch {
            return self.abandon("a stack switch inside a body run some number of times".into());
        }
        self.report.analysed += 1;
        if name == self.current {
            return true;
        }
        let old = std::mem::take(stack);
        self.stacks.insert(self.current.clone(), Some(old));
        match self.stacks.remove(name) {
            Some(Some(s)) => *stack = s,
            Some(None) => {
                return self.abandon(format!(
                    "a switch to `{name}`, a stack another word may have changed"
                ));
            }
            None => *stack = Stack::default(),
        }
        self.current = name.to_string();
        true
    }

    /// A word that reaches a stack by name leaves the walk's picture of every
    /// other stack stale, so they are forgotten and a later switch to one is
    /// an abandonment rather than a claim.
    fn forget_other_stacks(&mut self) {
        for v in self.stacks.values_mut() {
            *v = None;
        }
    }

    fn finish(mut self) -> Report {
        let mut reasons: Vec<(String, usize)> = std::mem::take(&mut self.why).into_iter().collect();
        reasons.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        self.report.reasons = reasons;
        self.report
    }

    fn abandon(&mut self, why: String) -> Went {
        self.report.abandoned += 1;
        *self.why.entry(why).or_default() += 1;
        false
    }

    /// Take `n` values for `word`, top first. In `Program` mode, fewer than
    /// `n` present is a finding, after which the stack is emptied and the
    /// caller leaves what the word would leave, so one underflow does not
    /// cascade into a report per term.
    fn take(
        &mut self,
        stack: &mut Stack,
        n: u8,
        word: &str,
        at: usize,
        via: Option<&str>,
    ) -> Option<Vec<Abs>> {
        if self.mode == Mode::Program && stack.items.len() < usize::from(n) {
            self.report.findings.push(Finding {
                at,
                word: word.to_string(),
                needs: n,
                have: stack.items.len() as i64,
                via: via.map(str::to_string),
            });
            stack.items.clear();
            return None;
        }
        Some((0..n).map(|_| stack.pop()).collect())
    }

    /// A word with a fixed pair: take, then leave that many unknowns — the
    /// top one tagged when the registration site says what it is (D181).
    fn apply_fixed(
        &mut self,
        stack: &mut Stack,
        word: &str,
        eff: StackEffect,
        at: usize,
        via: Option<&str>,
    ) {
        self.report.analysed += 1;
        let _ = self.take(stack, eff.consumes, word, at, via);
        if eff.produces == 0 {
            return;
        }
        stack.push_unknown(eff.produces - 1);
        stack.push(match eff.leaves {
            Leaves::Unknown => Abs::Unknown,
            Leaves::Tag(t) => Abs::Tagged(t),
            Leaves::Conditional(ty) => Abs::Cond(Rc::new(ClassModel::kind(ty))),
        });
    }

    /// Walk one stream on one stack. `site` is the top-level index a nested
    /// walk's findings are charged to; `via` names the body.
    fn walk(
        &mut self,
        stream: &[BundValue],
        stack: &mut Stack,
        site: Option<usize>,
        via: Option<&str>,
        level: u8,
    ) -> Went {
        if level >= MAX_INFER_DEPTH {
            return self.abandon("a body nested deeper than the analysis follows".into());
        }
        for (i, v) in stream.iter().enumerate() {
            let at = site.unwrap_or(i);
            match v.dt() {
                // `@name` and `( … )` switch stacks — D182: the current one
                // is saved under its name, the target is restored or created
                // empty, and the name to return to is pushed
                // (`reference/rust_multistackvm/src/multistackvm_apply.rs:69-87`).
                CONTEXT => {
                    let Some(name) = v.as_str() else {
                        return self.abandon("a stack switch with no name".into());
                    };
                    if self.mode == Mode::Program && self.may_switch {
                        self.nesting.push(self.current.clone());
                    }
                    if !self.switch_to(&name, stack) {
                        return false;
                    }
                }
                CALL => {
                    let Some(name) = v.as_str() else {
                        return self.abandon("a call with no name".into());
                    };
                    let prev = i.checked_sub(1).and_then(|p| stream.get(p));
                    if !self.call(&name, prev, stack, at, via, level) {
                        return false;
                    }
                }
                // Everything else is a value the stream pushes: a literal, a
                // string, a PTR, a lowered `{ … }` block or `[ … ]` list.
                _ => stack.push(Abs::of_literal(v)),
            }
            if self.halted {
                return true;
            }
        }
        true
    }

    /// Is `name` the native of that name, rather than something the program
    /// bound over it? The structural arms below read the native's meaning
    /// into the walk, which is wrong for a word the program redefined.
    fn is_native(&self, name: &str) -> bool {
        !self.lambdas.contains_key(name) && self.registry.effect_of(name).is_some()
    }

    /// One call, by name.
    fn call(
        &mut self,
        name: &str,
        prev: Option<&BundValue>,
        stack: &mut Stack,
        at: usize,
        via: Option<&str>,
        level: u8,
    ) -> Went {
        if let Some(eff) = literal_operand_effect(prev, name) {
            self.apply_fixed(stack, name, eff, at, via);
            return true;
        }
        if self.is_native(name)
            && let Some(went) = self.structural(name, stack, at, via, level)
        {
            return went;
        }
        match self.effect_of_name(name, level) {
            Some(e) if e.opaque => {
                self.abandon(format!("`{name}`, whose effect is not a fixed pair"))
            }
            Some(e) => {
                self.apply_fixed(stack, name, e, at, via);
                true
            }
            // An unknown name may be a lambda registered at run time — D16's
            // open world — so this is "cannot tell", not "wrong". **Name it.**
            // "a word with no declared effect" is true and useless: the reader
            // cannot tell an unimplemented word from a lambda the program
            // registered, and those want opposite responses — implement the
            // first, infer the second.
            None => self.abandon(format!("`{name}`, which declares no effect")),
        }
    }

    /// The effect of a name: **declared** for a native, **inferred** for a
    /// Bund word. RFC-0004 §S3.
    ///
    /// A native's is written at its registration site. A lambda's is composed
    /// from the effects of the words in its body, which is what §S3 means by
    /// inference and what makes `check` able to reason past a program's own
    /// vocabulary instead of stopping at it.
    fn effect_of_name(&mut self, name: &str, level: u8) -> Option<StackEffect> {
        if let Some(b) = self.lambdas.get(name).cloned() {
            return Some(self.infer_in(&b, level + 1));
        }
        if let Some(e) = self.registry.effect_of(name) {
            return Some(e);
        }
        let body = self.registry.lambda_body(name)?;
        Some(self.infer_in(&body, level + 1))
    }

    /// Compose a body's terms into one effect. **RFC-0004 §S3.**
    ///
    /// The walk is run on an empty stack relative to an unknown entry depth:
    /// what it takes from beneath is what the body **needs**, and what it
    /// leaves is what the body produces. `{ + }` needs 2 and leaves 1, and
    /// `{ 1 2 + }` needs nothing and leaves 1. **Composition saturates**: a
    /// body the walk abandons inside has no pair, because every depth after
    /// the abandonment is unknown too.
    ///
    /// The body's own abandonments and findings are not the caller's: they
    /// are kept apart, and the caller sees one `Opaque` for the whole body.
    fn infer_in(&mut self, body: &[BundValue], level: u8) -> StackEffect {
        self.infer_in_why(body, level).0
    }

    /// [`Walker::infer_in`], and when the body is opaque, why: the reason
    /// the walk gave up *in this body*, the one most often given when there
    /// were several. A nested body's reasons stay its own, as they do in
    /// `infer_in`, so the reason names the term of this body that blocked —
    /// for a word that calls itself, the call to itself.
    fn infer_in_why(&mut self, body: &[BundValue], level: u8) -> (StackEffect, Option<String>) {
        if level >= MAX_INFER_DEPTH {
            return (
                StackEffect::opaque(0),
                Some("a body nested deeper than the analysis follows".to_string()),
            );
        }
        let saved_report = std::mem::take(&mut self.report);
        let saved_why = std::mem::take(&mut self.why);
        let saved_mode = self.mode;
        // A body inferred for its pair is not on a `try`'s path: a `raise`
        // in it is the pair `raise` declares, not a failure to place.
        let saved_raised = self.raised.take();
        self.mode = Mode::Infer;
        let mut st = Stack::default();
        let went = self.walk(body, &mut st, Some(0), None, level);
        let why = std::mem::replace(&mut self.why, saved_why)
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
            .map(|(reason, _)| reason);
        self.report = saved_report;
        self.mode = saved_mode;
        self.raised = saved_raised;
        if !went {
            return (StackEffect::opaque(0), why);
        }
        let consumes = st.dip.clamp(0, i64::from(u8::MAX)) as u8;
        let produces = st.items.len().min(usize::from(u8::MAX)) as u8;
        (StackEffect::fixed(consumes, produces), None)
    }

    /// The words whose meaning the walk reads, beyond a pair: the three that
    /// move values and keep their tags, and the ones that make, register and
    /// run classes, objects and bodies. `None` for any other word.
    fn structural(
        &mut self,
        name: &str,
        stack: &mut Stack,
        at: usize,
        via: Option<&str>,
        level: u8,
    ) -> Option<Went> {
        let went = match name {
            "dup" => {
                self.report.analysed += 1;
                match self.take(stack, 1, name, at, via) {
                    Some(mut t) => {
                        let v = t.remove(0);
                        stack.push(v.clone());
                        stack.push(v);
                    }
                    None => stack.push_unknown(2),
                }
                true
            }
            "swap" => {
                self.report.analysed += 1;
                match self.take(stack, 2, name, at, via) {
                    Some(mut t) => {
                        let (a, b) = (t.remove(0), t.remove(0));
                        stack.push(a);
                        stack.push(b);
                    }
                    None => stack.push_unknown(2),
                }
                true
            }
            "drop" => {
                self.report.analysed += 1;
                let _ = self.take(stack, 1, name, at, via);
                true
            }
            // `ptr` turns a STRING into a PTR of the same text, which is how
            // the corpus spells `"name" ptr !`.
            "ptr" => {
                self.report.analysed += 1;
                match self.take(stack, 1, name, at, via) {
                    Some(mut t) => match t.remove(0) {
                        Abs::Name(_, s) => stack.push(Abs::Name(PTR, s)),
                        _ => stack.push(Abs::Tagged(PTR)),
                    },
                    None => stack.push_unknown(1),
                }
                true
            }
            // A bare CLASS with an empty `.super`; the name stays beneath it
            // for `register` (`reference/rust_multistackvm/src/stdlib/artefacts.rs:69-73`).
            "class" => {
                self.report.analysed += 1;
                stack.push(Abs::Class(Rc::new(ClassModel::bare())));
                true
            }
            // `<name> <LAMBDA|CLASS> register`
            // (`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:9-23`).
            "register" => {
                self.report.analysed += 1;
                if let Some(mut t) = self.take(stack, 2, name, at, via) {
                    let (body, who) = (t.remove(0), t.remove(0));
                    match (body, who) {
                        (Abs::Lambda(b), Abs::Name(_, n)) => {
                            self.lambdas.insert(n.to_string(), b);
                        }
                        (Abs::Class(m), Abs::Name(_, n)) => {
                            self.classes.insert(n.to_string(), Rc::new(m.named(&n)));
                        }
                        _ => {}
                    }
                }
                true
            }
            // `<container> <key> <value> set`. On a CLASS with a literal key
            // the slot is recorded, which is how a program's class is read.
            "set" | "," => {
                self.report.analysed += 1;
                match self.take(stack, 3, name, at, via) {
                    Some(mut t) => {
                        let (value, key, container) = (t.remove(0), t.remove(0), t.remove(0));
                        match (container, key) {
                            (Abs::Class(m), Abs::Name(_, k)) => {
                                stack.push(Abs::Class(Rc::new(m.with(&k, value))));
                            }
                            (Abs::Cond(m), Abs::Name(_, k)) => {
                                stack.push(Abs::Cond(Rc::new(m.with(&k, value))));
                            }
                            _ => stack.push(Abs::Unknown),
                        }
                    }
                    None => stack.push_unknown(1),
                }
                true
            }
            // `n { … } times` pushes the counter before each run
            // (`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:18-20`);
            // `list { … } loop` pushes the element (`loop_fun.rs:21-23`); `cond
            // { … } if` and its three siblings run the body once or not at all
            // (`if_fun.rs:28-66`). Each is the body run a number of times the
            // walk does not know — D180.
            "times" => self.repeat(
                name,
                Some(Abs::Tagged(INTEGER)),
                "the count",
                stack,
                at,
                via,
                level,
            ),
            "loop" => self.repeat(name, Some(Abs::Unknown), "the list", stack, at, via, level),
            "if" | "?true" | "if.false" | "?false" => {
                self.repeat(name, None, "the condition", stack, at, via, level)
            }
            // `ifthenelse_fun.rs:28-71`: the `then` lambda on top, the `else`
            // beneath it, the condition beneath both; one of the two runs.
            // `notifthenelse` runs the other one, which the walk need not
            // tell apart. Both are walked and joined — D184.
            "ifthenelse" | "?true*" | "notifthenelse" | "?false*" => {
                self.report.analysed += 1;
                let Some(mut t) = self.take(stack, 3, name, at, via) else {
                    return Some(self.abandon(format!("`{name}` with nothing beneath it to run")));
                };
                let (Abs::Lambda(then_b), Abs::Lambda(else_b)) = (t.remove(0), t.remove(0)) else {
                    return Some(
                        self.abandon(format!("`{name}` with a branch the walk cannot see")),
                    );
                };
                self.join_branches(name, &then_b, &else_b, stack, at, level)
            }
            // `while_fun.rs:5-47`: the lambda on top, then a condition pulled
            // before every run, so the body leaves the next one; `for_fun.rs:5-50`:
            // the lambda on top, the body runs first and leaves the condition
            // pulled after it. Either way one run nets zero against the
            // stack it found, or the depth after depends on the runs — D184.
            "while" | "for" | "for." => {
                self.report.analysed += 1;
                let Some(mut t) = self.take(stack, 1, name, at, via) else {
                    return Some(self.abandon(format!("`{name}` with nothing beneath it to run")));
                };
                let Abs::Lambda(b) = t.remove(0) else {
                    return Some(self.abandon(format!("`{name}` with a body the walk cannot see")));
                };
                self.run_until(name, &b, stack, at, via, level)
            }
            // `loop_fun.rs:51-101`: the lambda on top, then the body runs
            // while the stack is not empty and its top is not a NODATA, which
            // is pulled. `*loop.` reads the stack as well (`:115-117`, F78).
            "*loop" | "*loop." => {
                if self.mode != Mode::Program {
                    return Some(self.abandon(format!(
                        "`{name}`, which runs to the end of a stack whose depth is not known"
                    )));
                }
                self.report.analysed += 1;
                let Some(mut t) = self.take(stack, 1, name, at, via) else {
                    return Some(self.abandon(format!("`{name}` with nothing beneath it to run")));
                };
                let Abs::Lambda(b) = t.remove(0) else {
                    return Some(self.abandon(format!("`{name}` with a body the walk cannot see")));
                };
                self.run_over_stack(name, &b, stack, at, level)
            }
            // `curry` takes its name first (`conditional_curry.rs`, through
            // `named`) and `!` on it registers a word — D181.
            "curry" => {
                self.report.analysed += 1;
                match self.take(stack, 1, name, at, via) {
                    Some(mut t) => {
                        let mut m = ClassModel::kind("curry");
                        m.slots.insert("name".into(), t.remove(0));
                        stack.push(Abs::Cond(Rc::new(m)));
                    }
                    None => stack.push_unknown(1),
                }
                true
            }
            // The stack family — D182. `clear` and `fold` take the whole
            // current stack, which the abstract stack knows; `endcontext`
            // returns to the stack a switch left; `to_stack` and `to_current`
            // switch by a literal name; the rotations and the words that
            // reach another stack by name are followed as far as a pair goes
            // and the rest of the picture is forgotten.
            // `raise.rs:6-16`: pull the message — none is not an underflow,
            // `:14` — and fail. Inside a `try` the path ends here and
            // `except` runs on what is left; the `?try` reads it — D183.
            "raise" => {
                self.report.analysed += 1;
                if self.mode == Mode::Program && stack.items.is_empty() {
                    // "No message for raise": nothing pulled.
                } else {
                    let _ = self.take(stack, 1, name, at, via);
                }
                if self.raised.is_some() {
                    self.record_raise(stack);
                    self.halted = true;
                }
                true
            }
            "clear" => {
                if self.mode != Mode::Program {
                    return Some(self.abandon("`clear`, whose effect is not a fixed pair".into()));
                }
                self.report.analysed += 1;
                stack.items.clear();
                true
            }
            // `fold` pulls until a NODATA, which it consumes, or the bottom
            // (`reference/rust_multistack/src/ts_list.rs:8-24`). Followed
            // when every slot it would pull is a known tag: down to the first
            // NODATA from the top, or the whole stack. A slot the walk does
            // not know may be a NODATA, and then the depth after is not known.
            "fold" => {
                if self.mode != Mode::Program {
                    return Some(self.abandon("`fold`, whose effect is not a fixed pair".into()));
                }
                let mut keep = 0;
                for (i, a) in stack.items.iter().enumerate().rev() {
                    match a {
                        Abs::Tagged(NODATA) => {
                            keep = i;
                            break;
                        }
                        Abs::Unknown => {
                            return Some(self.abandon(
                                "`fold`, which stops at a NODATA the walk cannot rule out".into(),
                            ));
                        }
                        _ => {}
                    }
                }
                self.report.analysed += 1;
                stack.items.truncate(keep);
                stack.push(Abs::Tagged(LIST));
                true
            }
            "endcontext" => self.endcontext(stack),
            "to_stack" | "to_current" => {
                let Some(mut t) = self.take(stack, 1, name, at, via) else {
                    return Some(self.abandon(format!("`{name}` with no stack name")));
                };
                match t.remove(0) {
                    Abs::Name(_, n) => self.switch_to(&n, stack),
                    _ => self.abandon(format!("`{name}` to a computed stack name")),
                }
            }
            "rotate_current_left" | "rotate_current_right" | "<--" | "-->" => {
                self.report.analysed += 1;
                stack.forget_tags();
                true
            }
            "stacks_right" | "stacks_left" | "->" | "→" | "<-" | "←" | "drop_stack" => {
                self.abandon(format!("`{name}`, which changes the current stack"))
            }
            "rotate_stack_left" | "rotate_stack_right" | "return_to" | "move_from" | "swap_in" => {
                let eff = self.registry.effect_of(name)?;
                self.apply_fixed(stack, name, eff, at, via);
                self.forget_other_stacks();
                true
            }
            "object" => self.object_word(stack, at, via, level),
            // `True` and `False` are `true "Bool" object` and its twin
            // (`reference/Bund/src/stdlib/functions/oop/bool_class.rs`).
            "True" | "False" => {
                stack.push(Abs::Tagged(BOOL));
                stack.push(Abs::Name(STRING, Rc::from("Bool")));
                self.object_word(stack, at, via, level)
            }
            "!" | "execute" => self.execute(stack, at, via, level),
            _ => return None,
        };
        Some(went)
    }

    /// A body run some number of times the walk cannot know — D180. `times`,
    /// `loop` and the `if` family all pull the body from the top and what
    /// governs the runs from beneath it, then run the body with `each`
    /// pushed first, zero or more times.
    ///
    /// The depth after is known only when one run leaves the stack as deep
    /// as it found it, because then every count of runs leaves it there. The
    /// body is walked once on a copy; a run that nets anything else is an
    /// abandonment that says how much, and a run that nets zero is kept with
    /// every slot it may have rewritten forgotten — the zero-run and the
    /// many-run stacks agree on depth and on nothing else. A finding inside
    /// the body is a finding: a body that cannot run once is wrong whatever
    /// the count, and RFC-0004 §S4 made findings warnings for exactly the
    /// case of a body that never runs.
    #[allow(clippy::too_many_arguments)]
    fn repeat(
        &mut self,
        name: &str,
        each: Option<Abs>,
        governs: &str,
        stack: &mut Stack,
        at: usize,
        via: Option<&str>,
        level: u8,
    ) -> Went {
        self.report.analysed += 1;
        let Some(mut t) = self.take(stack, 2, name, at, via) else {
            return self.abandon(format!("`{name}` with nothing beneath it to run"));
        };
        let Abs::Lambda(b) = t.remove(0) else {
            return self.abandon(format!("`{name}` with a body the walk cannot see"));
        };
        self.run_repeatedly(name, each, governs, &b, stack, at, level)
    }

    /// `body` run with `each` pushed first, zero or more times, on `stack`:
    /// kept when one run nets zero, abandoned with the amount otherwise.
    #[allow(clippy::too_many_arguments)]
    fn run_repeatedly(
        &mut self,
        name: &str,
        each: Option<Abs>,
        governs: &str,
        body: &[BundValue],
        stack: &mut Stack,
        at: usize,
        level: u8,
    ) -> Went {
        let mut once = stack.copy();
        if let Some(e) = each {
            once.push(e);
        }
        let inside = format!("the body `{name}` runs");
        let outer = std::mem::replace(&mut self.may_switch, false);
        let went = self.walk(body, &mut once, Some(at), Some(&inside), level + 1);
        self.may_switch = outer;
        if !went {
            return false;
        }
        // The run reached a `raise`: that run ends in `except`, which has
        // the stack it left, and the path that goes on past `{name}` is the
        // one where the body does not run — D183.
        if std::mem::take(&mut self.halted) {
            return true;
        }
        let net = once.net_against(stack);
        if net != 0 {
            return self.abandon(format!(
                "`{name}` with a body that leaves {net:+} value(s) per run, so the depth after depends on {governs}"
            ));
        }
        stack.keep_what_agrees(&once);
        true
    }

    /// Two bodies of which one runs, each walked on a copy with the same
    /// stack and joined where they agree — `?ifthenelse` (D181), `?true*`
    /// (D184). A branch that ends in a `raise` leaves no path to join (D183).
    fn join_branches(
        &mut self,
        word: &str,
        then_b: &[BundValue],
        else_b: &[BundValue],
        stack: &mut Stack,
        at: usize,
        level: u8,
    ) -> Went {
        let mut then_s = stack.copy();
        let mut else_s = stack.copy();
        let outer = std::mem::replace(&mut self.may_switch, false);
        let inside = format!("the `then` of `{word}`");
        let went_then = self.walk(then_b, &mut then_s, Some(at), Some(&inside), level + 1);
        let then_raised = std::mem::take(&mut self.halted);
        let inside = format!("the `else` of `{word}`");
        let went_else = self.walk(else_b, &mut else_s, Some(at), Some(&inside), level + 1);
        let else_raised = std::mem::take(&mut self.halted);
        self.may_switch = outer;
        if !(went_then && went_else) {
            return false;
        }
        match (then_raised, else_raised) {
            (true, true) => {
                self.halted = true;
                return true;
            }
            (true, false) => {
                *stack = else_s;
                return true;
            }
            (false, true) => {
                *stack = then_s;
                return true;
            }
            (false, false) => {}
        }
        match Stack::join(then_s, &else_s) {
            Some(j) => {
                *stack = j;
                true
            }
            None => self.abandon(format!(
                "`{word}` whose `then` and `else` leave different depths, so the depth after depends on the condition"
            )),
        }
    }

    /// `while` and `for`: a body run until a condition it leaves is false.
    /// `while` pulls the condition before each run and `for` after, so one
    /// run is "pull, body" or "body, pull" against the stack the loop found,
    /// with the first condition on top for `while`; it nets zero or the
    /// depth after depends on how many runs there are. `while` ends by
    /// pulling the condition it found false; `for` runs at least once, so a
    /// `raise` on the run ends its path (D183).
    fn run_until(
        &mut self,
        name: &str,
        body: &[BundValue],
        stack: &mut Stack,
        at: usize,
        via: Option<&str>,
        level: u8,
    ) -> Went {
        let before = name == "while";
        let mut once = stack.copy();
        let inside = format!("the body `{name}` runs");
        let outer = std::mem::replace(&mut self.may_switch, false);
        let went = if before {
            if self.take(&mut once, 1, name, at, via).is_none() {
                self.may_switch = outer;
                return self.abandon(format!("`{name}` with no condition beneath its body"));
            }
            self.walk(body, &mut once, Some(at), Some(&inside), level + 1)
        } else {
            let went = self.walk(body, &mut once, Some(at), Some(&inside), level + 1);
            if went && !self.halted && self.take(&mut once, 1, name, at, Some(&inside)).is_none() {
                self.may_switch = outer;
                return self.abandon(format!("`{name}` whose body leaves no condition"));
            }
            went
        };
        self.may_switch = outer;
        if !went {
            return false;
        }
        if std::mem::take(&mut self.halted) {
            if before {
                // The condition the loop found may be false: that path takes
                // it and runs nothing.
                let _ = self.take(stack, 1, name, at, via);
            } else {
                self.halted = true;
            }
            return true;
        }
        let net = once.net_against(stack);
        if net != 0 {
            return self.abandon(format!(
                "`{name}` with a run that leaves {net:+} value(s) beyond the condition it takes, so the depth after depends on how many runs there are"
            ));
        }
        stack.keep_what_agrees(&once);
        if before {
            let _ = self.take(stack, 1, name, at, via);
        }
        true
    }

    /// `*loop`: the body run on the stack itself until it is empty or a
    /// NODATA is on top, which is pulled. In `Program` mode the depth is
    /// exact, so the runs are walked one by one on the stack — a run that
    /// takes beneath it is a finding, charged to the body as any run's is —
    /// and a run that does not shorten the stack never ends.
    fn run_over_stack(
        &mut self,
        name: &str,
        body: &[BundValue],
        stack: &mut Stack,
        at: usize,
        level: u8,
    ) -> Went {
        let inside = format!("the body `{name}` runs");
        let outer = std::mem::replace(&mut self.may_switch, false);
        let mut went = true;
        loop {
            match stack.items.last() {
                None => break,
                Some(Abs::Tagged(NODATA)) => {
                    stack.pop();
                    break;
                }
                Some(Abs::Unknown) => {
                    went = self.abandon(format!(
                        "`{name}`, which stops at a NODATA the walk cannot rule out"
                    ));
                    break;
                }
                Some(_) => {}
            }
            let depth = stack.items.len();
            went = self.walk(body, stack, Some(at), Some(&inside), level + 1);
            if !went || self.halted {
                break;
            }
            if stack.items.len() >= depth {
                went = self.abandon(format!(
                    "`{name}` with a run that does not shorten the stack, so it never reaches the end"
                ));
                break;
            }
        }
        self.may_switch = outer;
        went
    }

    /// A `raise` inside `try` left `stack`: the first fixes what `except`
    /// sees, a later one is joined with it as two branches are.
    fn record_raise(&mut self, stack: &Stack) {
        self.raised = Some(match self.raised.take() {
            Some(Raised::At(prev)) => match Stack::join(prev, stack) {
                Some(j) => Raised::At(j),
                None => Raised::Disagree,
            },
            Some(Raised::Disagree) => Raised::Disagree,
            Some(Raised::Nowhere) | None => Raised::At(stack.copy()),
        });
    }

    /// `?try` — `conditional_tryexcept.rs:30-50`: run `try`; when it fails,
    /// push an ERROR conditional carrying `associated` and the message, then
    /// run `except` and `recovery`. Followed where the walk can **place** the
    /// failure — D183. The walk assumes every word it passes returns, as it
    /// does everywhere; the one word that never does is `raise`
    /// (`raise.rs:16`), so a `raise` the walk reaches ends the path it is on
    /// with the stack `except` will see. A `raise` reached on the `try`'s
    /// own path leaves no success path; one reached inside a body `if` or
    /// `times` runs leaves the path where that body does not run, and the
    /// two are joined as `?ifthenelse` joins its branches. A `try` with no
    /// `raise` may still fail at any word, and the walk cannot say where,
    /// so it is abandoned rather than followed on the assumption.
    fn run_tryexcept(
        &mut self,
        m: &Rc<ClassModel>,
        stack: &mut Stack,
        at: usize,
        level: u8,
    ) -> Went {
        let body = match Self::slot(m, "try") {
            Slot::Lambda(b) => b,
            // `:14-17`: no `try` is an empty lambda, which cannot fail.
            Slot::Absent => {
                self.report.analysed += 1;
                return true;
            }
            Slot::Unseen => {
                return self
                    .abandon("the `try` slot of `?try`, which is not a literal lambda".into());
            }
        };
        self.report.analysed += 1;
        let mut ok = stack.copy();
        let outer = std::mem::replace(&mut self.may_switch, false);
        let saved = self.raised.replace(Raised::Nowhere);
        let went = self.walk(
            &body,
            &mut ok,
            Some(at),
            Some("the `try` of `?try`"),
            level + 1,
        );
        let raised = std::mem::replace(&mut self.raised, saved);
        let try_ended = std::mem::take(&mut self.halted);
        if !went {
            self.may_switch = outer;
            return false;
        }
        let mut failed = match raised {
            Some(Raised::At(f)) => f,
            Some(Raised::Disagree) => {
                self.may_switch = outer;
                return self.abandon(
                    "`?try` whose `raise`s leave different depths, so the depth after depends on which one runs"
                        .into(),
                );
            }
            Some(Raised::Nowhere) | None => {
                self.may_switch = outer;
                return self.abandon(
                    "`?try` whose `try` reaches no `raise`, so the walk cannot place the failure `except` handles"
                        .into(),
                );
            }
        };
        // `:34-39`: the ERROR conditional, with the `?try`'s own `associated`.
        let mut error = ClassModel::kind("error");
        if let Some(a) = m.slots.get("associated") {
            error.slots.insert("associated".into(), a.clone());
        }
        error.slots.insert("context".into(), Abs::Tagged(STRING));
        failed.push(Abs::Cond(Rc::new(error)));
        // `:40-47`: `except`, then `recovery`, each on what the other left. A
        // `raise` in either is not this `?try`'s to catch (`:42`, `:46`): the
        // walk is back under the enclosing one, if any.
        let went = self.slot_walk(m, "except", "?try", &mut failed, at, level)
            && self.slot_walk(m, "recovery", "?try", &mut failed, at, level);
        self.may_switch = outer;
        if !went {
            return false;
        }
        if try_ended {
            *stack = failed;
            return true;
        }
        match Stack::join(ok, &failed) {
            Some(j) => {
                *stack = j;
                true
            }
            None => self.abandon(
                "`?try` whose `try` and `except` leave different depths, so the depth after depends on whether `try` failed"
                    .into(),
            ),
        }
    }

    /// The body a slot runs: the lambda, or nothing.
    fn body_of(slot: Slot) -> Rc<[BundValue]> {
        match slot {
            Slot::Lambda(b) => b,
            Slot::Absent | Slot::Unseen => Rc::from(Vec::new()),
        }
    }

    /// One slot of a CONDITIONAL, as its handler reads it: a LAMBDA is run and
    /// anything else is an empty body (`slot_body`, after
    /// `conditional_ifthenelse.rs:15-26`).
    fn slot(m: &ClassModel, key: &str) -> Slot {
        match m.slots.get(key) {
            None => Slot::Absent,
            Some(Abs::Lambda(b)) => Slot::Lambda(b.clone()),
            Some(Abs::Unknown) | Some(Abs::Tagged(LAMBDA)) => Slot::Unseen,
            Some(_) => Slot::Absent,
        }
    }

    /// Walk a slot's body in place, or nothing when the slot is empty.
    fn slot_walk(
        &mut self,
        m: &ClassModel,
        key: &str,
        word: &str,
        stack: &mut Stack,
        at: usize,
        level: u8,
    ) -> Went {
        match Self::slot(m, key) {
            Slot::Lambda(b) => {
                self.report.analysed += 1;
                let inside = format!("the `{key}` of `{word}`");
                self.walk(&b, stack, Some(at), Some(&inside), level + 1)
            }
            Slot::Absent => true,
            Slot::Unseen => self.abandon(format!(
                "the `{key}` slot of `{word}`, which is not a literal lambda"
            )),
        }
    }

    /// `!` on a CONDITIONAL: `execute_conditionals.rs:9-25` reads `type` and
    /// calls its handler. Followed where the handler runs literal slots:
    /// `through` runs `run`; `ifthenelse` runs `if`, takes the condition and
    /// runs one branch (`conditional_ifthenelse.rs:14-50`), so both branches
    /// are walked and kept only where they agree; `error` runs
    /// `associated`; `csv` and `sqlite` push a row and run `lambda` per row
    /// (`run_csv`, `run_sqlite`), which is D180's rule, or once over a
    /// column as a LIST; `curry` registers a word built from `data`, `lambda`
    /// and `!` (`run_curry`); `tryexcept` runs `try` and, where a `raise`
    /// places the failure, `except` and `recovery` on what it left
    /// (`run_tryexcept`, D183).
    fn run_conditional(
        &mut self,
        m: &Rc<ClassModel>,
        stack: &mut Stack,
        at: usize,
        via: Option<&str>,
        level: u8,
    ) -> Went {
        let kind = m.name.as_deref().unwrap_or("").to_string();
        match kind.as_str() {
            "through" => self.slot_walk(m, "run", "?", stack, at, level),
            "error" => self.slot_walk(m, "associated", "?error", stack, at, level),
            "ifthenelse" => {
                if !self.slot_walk(m, "if", "?ifthenelse", stack, at, level) {
                    return false;
                }
                self.report.analysed += 1;
                if self.take(stack, 1, "?ifthenelse", at, via).is_none() {
                    return self.abandon("`?ifthenelse` whose `if` left no condition".into());
                }
                let (then_b, else_b) = match (Self::slot(m, "then"), Self::slot(m, "else")) {
                    (Slot::Unseen, _) | (_, Slot::Unseen) => {
                        return self.abandon(
                            "a `then` or `else` slot of `?ifthenelse`, which is not a literal lambda".into(),
                        );
                    }
                    (t, e) => (Self::body_of(t), Self::body_of(e)),
                };
                self.report.analysed += 2;
                self.join_branches("?ifthenelse", &then_b, &else_b, stack, at, level)
            }
            "tryexcept" => self.run_tryexcept(m, stack, at, level),
            "csv" | "sqlite" => {
                let body = match Self::slot(m, "lambda") {
                    Slot::Lambda(b) => b,
                    Slot::Absent => Rc::from(Vec::new()),
                    Slot::Unseen => {
                        return self.abandon(format!(
                            "the `lambda` slot of `{kind}`, which is not a literal lambda"
                        ));
                    }
                };
                self.report.analysed += 1;
                if kind == "csv" && m.slots.contains_key("column") {
                    stack.push(Abs::Tagged(LIST));
                    let inside = "the `lambda` of `csv`".to_string();
                    return self.walk(&body, stack, Some(at), Some(&inside), level + 1);
                }
                let each = if kind == "csv" {
                    Abs::Tagged(LIST)
                } else {
                    Abs::Tagged(MAP)
                };
                self.run_repeatedly(&kind, Some(each), "the rows", &body, stack, at, level)
            }
            "curry" => {
                let (Some(Abs::Name(_, n)), Slot::Lambda(b)) =
                    (m.slots.get("name"), Self::slot(m, "lambda"))
                else {
                    return self.abandon("`curry` whose name or lambda is not a literal".into());
                };
                let data: Vec<BundValue> = match m.slots.get("data") {
                    None => Vec::new(),
                    Some(Abs::List(items)) => items.iter().rev().cloned().collect(),
                    Some(_) => {
                        return self.abandon("`curry` whose `data` is not a literal list".into());
                    }
                };
                self.report.analysed += 1;
                let mut body = data;
                body.push(BundValue::lambda(b.to_vec()));
                body.push(BundValue::call("!"));
                self.lambdas.insert(n.to_string(), Rc::from(body));
                true
            }
            "context" => self.abandon("a stack switch".into()),
            "fmt" => self.abandon("`fmt`, whose message pulls what its template names".into()),
            other => self.abandon(format!(
                "`!` on a CONDITIONAL of type `{other}`, which the walk does not follow"
            )),
        }
    }

    /// `endcontext` — the top to the workbench if there is one, the current
    /// stack dropped, and the walk back on the stack the switch left
    /// (`reference/rust_multistackvm/src/stdlib/ctx.rs:5-27`, with Bund2
    /// keeping the stack to return to in the context record).
    fn endcontext(&mut self, stack: &mut Stack) -> Went {
        if self.mode != Mode::Program || !self.may_switch {
            return self.abandon("a stack switch".into());
        }
        let Some(prev) = self.nesting.pop() else {
            return self.abandon("`endcontext` with no context open".into());
        };
        self.report.analysed += 1;
        if !stack.items.is_empty() {
            stack.pop();
        }
        let dropped = std::mem::take(&mut self.current);
        self.stacks.remove(&dropped);
        *stack = match self.stacks.remove(&prev) {
            Some(Some(s)) => s,
            Some(None) => {
                return self.abandon(format!(
                    "a return to `{prev}`, a stack another word may have changed"
                ));
            }
            None => Stack::default(),
        };
        self.current = prev;
        true
    }

    /// `object` — construct from a registered class, by name
    /// (`reference/rust_multistackvm/src/stdlib/bund_object.rs:115-188`).
    fn object_word(&mut self, stack: &mut Stack, at: usize, via: Option<&str>, level: u8) -> Went {
        let Some(mut t) = self.take(stack, 1, "object", at, via) else {
            return self.abandon("`object` of a class name that is not there".into());
        };
        match t.remove(0) {
            Abs::Name(_, n) => match self.class_model(&n) {
                Some(m) => self.construct(&m, stack, at, level),
                None => self.abandon(format!(
                    "`object` of `{n}`, a class neither this program nor Bund2 defines"
                )),
            },
            _ => self.abandon("`object` of a computed class name".into()),
        }
    }

    /// A class by name: the program's first, then the registry's.
    fn class_model(&mut self, name: &str) -> Option<Rc<ClassModel>> {
        if let Some(m) = self.classes.get(name) {
            return Some(m.clone());
        }
        if let Some(m) = self.builtin.get(name) {
            return Some(m.clone());
        }
        let v = self.registry.class(name)?;
        let m = Rc::new(ClassModel::of_value(name, &v));
        self.builtin.insert(name.to_string(), m.clone());
        Some(m)
    }

    /// Construction, as `stdlib_object_inline` performs it
    /// (`bund_object.rs:123-174`): parents first, then the object pushed, its
    /// `.init` run, the object pulled if it is still on top, and pushed.
    fn construct(&mut self, m: &Rc<ClassModel>, stack: &mut Stack, at: usize, level: u8) -> Went {
        self.report.analysed += 1;
        if !self.construct_parents(m, stack, at, level, 0) {
            return false;
        }
        stack.push(Abs::Object(m.clone()));
        if !self.run_init(m, stack, at, level) {
            return false;
        }
        if matches!(stack.top(), Some(Abs::Object(o)) if same_class(o, m)) {
            stack.pop();
        }
        stack.push(Abs::Object(m.clone()));
        true
    }

    /// `make_bund_object`'s parent loop (`bund_object.rs:42-104`): each
    /// parent is built, pushed, its `.init` run, and pulled again only if an
    /// object of its class is still on top (`:83-94`).
    fn construct_parents(
        &mut self,
        m: &Rc<ClassModel>,
        stack: &mut Stack,
        at: usize,
        level: u8,
        generation: u8,
    ) -> Went {
        if generation >= MAX_ANCESTRY {
            return self.abandon(format!(
                "an ancestry of {} deeper than {MAX_ANCESTRY} generations",
                m.display_name()
            ));
        }
        for p in m.supers() {
            let Some(pm) = self.class_model(&p) else {
                return self.abandon(format!(
                    "a parent `{p}` of {}, a class neither this program nor Bund2 defines",
                    m.display_name()
                ));
            };
            if !self.construct_parents(&pm, stack, at, level, generation + 1) {
                return false;
            }
            stack.push(Abs::Object(pm.clone()));
            if !self.run_init(&pm, stack, at, level) {
                return false;
            }
            if matches!(stack.top(), Some(Abs::Object(o)) if same_class(o, &pm)) {
                stack.pop();
            }
        }
        true
    }

    /// An object's own `.init`, with the object on top: a LAMBDA is walked, a
    /// PTR is a method native, and a PTR naming no registered method is
    /// skipped in silence, as the run skips it (`bund_object.rs:137-162`).
    fn run_init(&mut self, m: &Rc<ClassModel>, stack: &mut Stack, at: usize, level: u8) -> Went {
        match m.slots.get(".init").cloned() {
            Some(Abs::Lambda(b)) => {
                let inside = format!("the constructor of {}", m.display_name());
                self.walk(&b, stack, Some(at), Some(&inside), level + 1)
            }
            Some(Abs::Name(PTR, mname)) => match self.registry.method_effect(&mname) {
                Some(e) => {
                    let inside = format!("the constructor of {}", m.display_name());
                    self.apply_method(e, m, &mname, stack, at, Some(&inside))
                }
                None => true,
            },
            _ => true,
        }
    }

    /// A method native's declared effect, applied with the receiver on top.
    fn apply_method(
        &mut self,
        e: MethodEffect,
        m: &Rc<ClassModel>,
        mname: &str,
        stack: &mut Stack,
        at: usize,
        via: Option<&str>,
    ) -> Went {
        match e {
            MethodEffect::Peek { produces } => {
                self.report.analysed += 1;
                if self.take(stack, 1, mname, at, via).is_some() {
                    stack.push(Abs::Object(m.clone()));
                }
                stack.push_unknown(produces);
                true
            }
            MethodEffect::Rebuild { below } => {
                self.report.analysed += 1;
                let _ = self.take(stack, below.saturating_add(1), mname, at, via);
                stack.push(Abs::Object(m.clone()));
                true
            }
            MethodEffect::Consume { below, produces } => {
                self.report.analysed += 1;
                let _ = self.take(stack, below.saturating_add(1), mname, at, via);
                stack.push_unknown(produces);
                true
            }
            MethodEffect::Opaque => self.abandon(format!(
                "`{mname}` of {}, whose effect is not a fixed pair",
                m.display_name()
            )),
        }
    }

    /// `locate_value_in_object` (`multistackvm_object.rs:6-24`): the class's
    /// own slot, then each parent's, depth first, first match wins.
    fn locate(&mut self, m: &Rc<ClassModel>, key: &str, generation: u8) -> Located {
        if let Some(a) = m.slots.get(key) {
            return match a {
                Abs::Lambda(b) => Located::Lambda(b.clone()),
                Abs::Name(PTR, n) => Located::Native(n.clone()),
                other => Located::Value(other.clone()),
            };
        }
        if generation >= MAX_ANCESTRY {
            return Located::Missing;
        }
        for p in m.supers() {
            let Some(pm) = self.class_model(&p) else {
                return Located::Unresolved(p);
            };
            match self.locate(&pm, key, generation + 1) {
                Located::Missing => continue,
                found => return found,
            }
        }
        Located::Missing
    }

    /// `!` — `execute`, by the tag of what it pulls
    /// (`reference/rust_multistackvm/src/stdlib/execute.rs:24-98`).
    fn execute(&mut self, stack: &mut Stack, at: usize, via: Option<&str>, level: u8) -> Went {
        let Some(mut t) = self.take(stack, 1, "!", at, via) else {
            return self.abandon("`!` on a value whose tag is not known".into());
        };
        match t.remove(0) {
            // `:93-94`: the body, on this stack.
            Abs::Lambda(b) => {
                self.report.analysed += 1;
                self.walk(&b, stack, Some(at), Some("the lambda `!` runs"), level + 1)
            }
            // `:27-30`: the word of that name.
            Abs::Name(_, n) => self.call(&n, None, stack, at, via, level),
            // D23: Bund2 constructs from the CLASS value.
            Abs::Class(m) => self.construct(&m, stack, at, level),
            // `execute_conditionals.rs:9-25`: by the `type` slot — D181.
            Abs::Cond(m) => self.run_conditional(&m, stack, at, via, level),
            // `execute_object.rs:8-20`: pull the name, push the object back,
            // dispatch.
            Abs::Object(m) => {
                let Some(mut k) = self.take(stack, 1, "!", at, via) else {
                    return self.abandon(format!(
                        "`!` on an object of {}, with no method name beneath it",
                        m.display_name()
                    ));
                };
                let Abs::Name(_, key) = k.remove(0) else {
                    return self.abandon(format!(
                        "`!` on an object of {}, whose method name is computed",
                        m.display_name()
                    ));
                };
                stack.push(Abs::Object(m.clone()));
                self.dispatch(&m, &key, stack, at, level)
            }
            other => self.abandon(format!("`!` on {}", other.describe())),
        }
    }

    /// `m(name)` with the receiver on top (`multistackvm_object.rs:45-86`).
    fn dispatch(
        &mut self,
        m: &Rc<ClassModel>,
        key: &str,
        stack: &mut Stack,
        at: usize,
        level: u8,
    ) -> Went {
        match self.locate(m, key, 0) {
            Located::Lambda(b) => {
                self.report.analysed += 1;
                let inside = format!("method `{key}` of {}", m.display_name());
                self.walk(&b, stack, Some(at), Some(&inside), level + 1)
            }
            Located::Native(mname) => match self.registry.method_effect(&mname) {
                Some(e) => {
                    let inside = format!("method `{key}` of {}", m.display_name());
                    self.apply_method(e, m, &mname, stack, at, Some(&inside))
                }
                None => self.abandon(format!(
                    "method `{key}` of {}, bound to `{mname}`, which is not registered",
                    m.display_name()
                )),
            },
            // `:80-82`: a slot that is neither is pushed.
            Located::Value(a) => {
                self.report.analysed += 1;
                stack.push(a);
                true
            }
            Located::Missing => self.abandon(format!(
                "`!` on an object of {}, which has no slot `{key}`",
                m.display_name()
            )),
            Located::Unresolved(p) => self.abandon(format!(
                "a parent `{p}` of {}, a class neither this program nor Bund2 defines",
                m.display_name()
            )),
        }
    }
}

/// Walk a lowered stream and report what it would underflow on.
///
/// `depth_in` is what the stack holds when the stream starts — 0 for a whole
/// program.
pub fn check(stream: &[BundValue], registry: &Registry, depth_in: i64) -> Report {
    let registry = prebind(stream, registry);
    let mut w = Walker::new(&registry, Mode::Program);
    let mut stack = Stack::default();
    stack.push_unknown(depth_in.clamp(0, i64::from(u8::MAX)) as u8);
    let _ = w.walk(stream, &mut stack, None, None, 0);
    w.finish()
}

/// Compose a body's terms into one effect. **RFC-0004 §S3.** See
/// `Walker::infer_in`, which this runs on a fresh walk.
pub fn infer(body: &[BundValue], registry: &Registry, depth: u8) -> StackEffect {
    let mut w = Walker::new(registry, Mode::Infer);
    w.infer_in(body, depth)
}

/// [`infer`], and for an opaque body the reason the walk gave up in it, in
/// the words `bund2 check` reports — RFC-0011's effect column prints it, so
/// `opaque` says what to look at. `None` beside a fixed pair, and beside an
/// opaque one only if the walk stopped without saying why.
pub fn infer_explained(
    body: &[BundValue],
    registry: &Registry,
    depth: u8,
) -> (StackEffect, Option<String>) {
    let mut w = Walker::new(registry, Mode::Infer);
    w.infer_in_why(body, depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::Interp;

    fn reg() -> bund2_api::Registry {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i.registry
    }

    fn eff_of(src: &str) -> StackEffect {
        let r = reg();
        let body = bund2_syntax::compile(src).expect("compiles");
        // `compile` appends EXIT; inference ignores it as a pushed value, so
        // trim it rather than let it inflate `produces`.
        let body: Vec<_> = body
            .into_iter()
            .take_while(|v| v.dt() != bund2_value::EXIT)
            .collect();
        infer(&body, &r, 0)
    }

    /// `infer_explained` over the body of `word` after `src` registered it.
    fn explained(src: &str, word: &str) -> (StackEffect, Option<String>) {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).expect("compiles");
        i.eval(&stream).expect("runs");
        let body = i.registry.lambda_body(word).expect("registered");
        infer_explained(&body, &i.registry, 0)
    }

    /// **RFC-0011's effect column says why a word is opaque.** A fixed pair
    /// has no reason; a word that calls itself is opaque because of the
    /// call to itself, and the reason names it; a word that calls something
    /// with no declared effect names that.
    #[test]
    fn an_opaque_body_says_why() {
        let (e, why) = explained(":inc { 1 + } register", "inc");
        assert_eq!((e.consumes, e.produces, e.opaque), (1, 1, false));
        assert_eq!(why, None);

        let (e, why) = explained(
            ":fib { dup 1 < { dup 1 swap - fib swap 2 swap - fib + } if } register",
            "fib",
        );
        assert!(e.opaque);
        let why = why.expect("a reason");
        assert!(why.contains("`fib`"), "{why}");

        let (e, why) = explained(":w { 1 nosuchword } register", "w");
        assert!(e.opaque);
        assert_eq!(
            why.as_deref(),
            Some("`nosuchword`, which declares no effect")
        );

        // The same answer `infer` gives.
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let body = bund2_syntax::compile("1 +").expect("compiles");
        let body: Vec<_> = body
            .into_iter()
            .take_while(|v| v.dt() != bund2_value::EXIT)
            .collect();
        let (e, _) = infer_explained(&body, &i.registry, 0);
        let f = infer(&body, &i.registry, 0);
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (f.consumes, f.produces, f.opaque)
        );
    }

    fn check_src(src: &str) -> Report {
        let r = reg();
        let stream = bund2_syntax::compile(src).expect("compiles");
        check(&stream, &r, 0)
    }

    fn clean(rep: &Report) -> bool {
        rep.findings.is_empty() && rep.abandoned == 0
    }

    /// **RFC-0004 §S3.** `consumes` is how far below its entry depth the body
    /// reaches; `produces` is that plus the net.
    #[test]
    fn composition_counts_the_dip_not_the_terms() {
        let e = eff_of("+");
        assert_eq!((e.consumes, e.produces, e.opaque), (2, 1, false));

        let e = eff_of("1");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));

        let e = eff_of("drop");
        assert_eq!((e.consumes, e.produces, e.opaque), (1, 0, false));

        // Needs nothing: it supplies its own operands.
        let e = eff_of("1 2 +");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));

        // Two additions in a row need **three**, not four: the first leaves a
        // value the second consumes. Counting terms instead of tracking the
        // dip would say four.
        let e = eff_of("+ +");
        assert_eq!((e.consumes, e.produces, e.opaque), (3, 1, false));
    }

    /// **D178.** A literal template fixes `format`'s arity: one value per
    /// distinct key, plus the template. Without the literal it is as opaque
    /// as before, and so is a template `leon` refuses.
    #[test]
    fn a_literal_template_fixes_formats_arity() {
        let e = eff_of("\"a={A} b={B} a={A}\" format");
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (2, 1, false),
            "two keys, one repeated"
        );

        let e = eff_of("\"no keys\" format");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));

        assert!(eff_of("format").opaque, "a computed template says nothing");
        assert!(
            eff_of("\"{unclosed\" format").opaque,
            "a template leon refuses is left to the run"
        );
    }

    /// **D178.** The rule holds in a whole-program walk: the site is analysed,
    /// an underflow is a finding naming what `format` needs, and nothing is
    /// abandoned on the way.
    #[test]
    fn check_tracks_through_a_literal_template() {
        let rep = check_src("1 \"x={A} y={B}\" format println");
        assert_eq!(rep.abandoned, 0, "{:?}", rep.reasons);
        assert_eq!(rep.findings.len(), 1, "{:?}", rep.findings);
        assert_eq!(
            (
                rep.findings[0].word.as_str(),
                rep.findings[0].needs,
                rep.findings[0].have
            ),
            ("format", 3, 2)
        );

        let rep = check_src("1 2 \"x={A} y={B}\" format println");
        assert!(clean(&rep), "{rep:?}");
    }

    /// Composition saturates: one unknown term makes the whole unknown,
    /// because every depth after it is unknown too.
    #[test]
    fn opaque_saturates() {
        assert!(eff_of("!").opaque, "`!` on a value from beneath has no tag");
        assert!(
            eff_of("1 2 + !").opaque,
            "`!` on an INTEGER is the fallback arm"
        );
        assert!(
            eff_of("{ dup } loop").opaque,
            "`loop` whose body nets +1 per run has no pair — D180"
        );
        assert!(
            eff_of("nosuchwordanywhere").opaque,
            "an unknown name may be bound at run time — D16"
        );
    }

    /// **D179.** `!` on a literal lambda is the body, on the same stack, and
    /// `!` on a literal name is the word. A finding inside the body is charged
    /// to the `!` and says which body it is in.
    #[test]
    fn bang_on_a_literal_lambda_or_name_is_followed() {
        let e = eff_of("{ + } !");
        assert_eq!((e.consumes, e.produces, e.opaque), (2, 1, false));

        let e = eff_of(":drop !");
        assert_eq!((e.consumes, e.produces, e.opaque), (1, 0, false));

        let e = eff_of("\"drop\" ptr !");
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (1, 0, false),
            "`ptr` keeps the text"
        );

        let rep = check_src("1 { + } !");
        assert_eq!(rep.abandoned, 0, "{:?}", rep.reasons);
        assert_eq!(rep.findings.len(), 1, "{:?}", rep.findings);
        let f = &rep.findings[0];
        assert_eq!((f.word.as_str(), f.needs, f.have, f.at), ("+", 2, 1, 2));
        assert_eq!(f.via.as_deref(), Some("the lambda `!` runs"));

        // A redefined `!` is the program's word, not the native's arms.
        let rep = check_src(":! { drop } register 1 2 !");
        assert!(clean(&rep), "{rep:?}");
    }

    /// **D179.** Construction of a built-in class runs every constructor in
    /// the chain: `Bool`'s `.value_init` takes the value beneath the object,
    /// so `"Bool" object` on an empty stack is a finding inside that
    /// constructor, and `true "Bool" object` is clean. `True` is the same
    /// construction with its operands supplied.
    #[test]
    fn object_follows_the_constructors_of_a_built_in_class() {
        let rep = check_src("\"Bool\" object");
        assert_eq!(rep.abandoned, 0, "{:?}", rep.reasons);
        assert_eq!(rep.findings.len(), 1, "{:?}", rep.findings);
        let f = &rep.findings[0];
        assert_eq!((f.word.as_str(), f.needs, f.have), (".value_init", 2, 1));
        assert_eq!(f.via.as_deref(), Some("the constructor of `Value`"));

        assert!(clean(&check_src("true \"Bool\" object println")));
        assert!(clean(&check_src("True println")));

        let e = eff_of(":Value object");
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (1, 1, false),
            "the name and the value, one object"
        );

        // The run says so too: an unregistered class is a run-time error, so
        // the walk claims nothing about it.
        let rep = check_src("\"NoSuchClass\" object");
        assert_eq!(rep.abandoned, 1, "{rep:?}");
    }

    /// **D179.** A method call on a known object: the receiver survives a
    /// `Peek` or `Rebuild` method, is consumed by a `Consume` one, and
    /// `.format`'s template-driven arity is an abandonment with its name.
    #[test]
    fn a_method_call_on_a_known_object_is_followed() {
        // `value_demo.bund`: `:str 42 :Value object !` leaves the object and
        // its string; `format println` consumes the string.
        assert!(clean(&check_src(
            ":str 42 :Value object ! \"Object is storing a value: {A}\" format println"
        )));

        // `intervals_demo.bund`'s shape: `overlap` needs the probe beneath the
        // name, consumes the object and answers a label.
        let e = eff_of(":overlap [ [ 1.0 5.0 ] ] \"Intervals\" object !");
        assert_eq!((e.consumes, e.produces, e.opaque), (1, 1, false));
        assert!(clean(&check_src(
            "7.0 :overlap [ [ 1.0 5.0 ] ] \"Intervals\" object ! println"
        )));
        let rep = check_src(":overlap [ [ 1.0 5.0 ] ] \"Intervals\" object ! println");
        assert_eq!(rep.findings.len(), 1, "{rep:?}");
        assert_eq!(rep.findings[0].word, ".intervals_overlap");

        // `:push List !` — `List` is `[ ] "List" object`, a word; `push`
        // rebuilds the receiver.
        assert!(clean(&check_src(
            "42 :push [ 1 ] \"List\" object ! println"
        )));

        // `.id` peeks: the object and the id, then `swap drop` keeps the id —
        // the corpus's `:.id :Probed object ! swap drop`, with the base
        // chain declared as the probe declares it.
        assert!(clean(&check_src(
            ":P class :.super [ :Object ] set register :.id :P object ! swap drop println"
        )));

        let rep = check_src(":format 42 :Value object !");
        assert_eq!(rep.abandoned, 1, "{rep:?}");
        assert!(
            rep.reasons[0].0.contains(".format"),
            "the reason names the method: {:?}",
            rep.reasons
        );

        let rep = check_src(":nosuchslot 42 :Value object !");
        assert_eq!(rep.abandoned, 1, "{rep:?}");
        assert!(rep.reasons[0].0.contains("no slot"), "{:?}", rep.reasons);
    }

    /// **D180.** A body `times`, `loop` or `if` runs is walked once; the
    /// depth after is known when one run nets zero, and said to depend on
    /// the count or the condition when it does not. A finding inside the
    /// body is reported with the body named.
    #[test]
    fn a_repeated_body_is_followed_when_one_run_nets_zero() {
        let e = eff_of("5 { drop } times");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 0, false));

        // The count and the accumulator beneath are taken, the sum is left.
        let e = eff_of("{ + } times");
        assert_eq!((e.consumes, e.produces, e.opaque), (2, 1, false));

        assert!(clean(&check_src("0 10 { + } times println")));
        assert!(clean(&check_src("[ 1 2 3 ] { println } loop")));
        assert!(clean(&check_src("true { \"yes\" println } if")));
        assert!(clean(&check_src(
            "42 :Value object ?object { \"built\" println } if println"
        )));

        // The counter is pushed and never taken, and `dup` adds one more.
        let rep = check_src("3 { dup } times");
        assert_eq!(rep.abandoned, 1, "{rep:?}");
        assert!(
            rep.reasons[0].0.contains("+2") && rep.reasons[0].0.contains("the count"),
            "{:?}",
            rep.reasons
        );

        let rep = check_src("true { 1 } if");
        assert!(
            rep.reasons[0].0.contains("the condition"),
            "{:?}",
            rep.reasons
        );

        let rep = check_src("10 { + } times println");
        assert_eq!(rep.findings.len(), 1, "{rep:?}");
        assert_eq!(
            rep.findings[0].via.as_deref(),
            Some("the body `times` runs")
        );

        let rep = check_src("3 :nobody times");
        assert!(rep.reasons[0].0.contains("cannot see"), "{:?}", rep.reasons);
    }

    /// **D181.** A constructor's declared `Leaves` makes the CONDITIONAL a
    /// slot model, `set` fills it, and `!` runs the handler the type names.
    #[test]
    fn a_conditional_is_followed_by_its_declared_type() {
        // `through`: `run` on the same stack.
        let e = eff_of("? :run { 1 2 + } set !");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));
        let rep = check_src("? :run { + } set !");
        assert_eq!(rep.findings.len(), 1, "{rep:?}");
        assert_eq!(rep.findings[0].via.as_deref(), Some("the `run` of `?`"));

        // `ifthenelse`: both branches walked, kept where they agree.
        let e = eff_of("?ifthenelse :if { true } set :then { 1 } set :else { 2 } set !");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));
        let rep = check_src("?ifthenelse :if { true } set :then { 1 } set :else { 1 2 } set !");
        assert!(
            rep.reasons[0].0.contains("different depths"),
            "{:?}",
            rep.reasons
        );

        // `tryexcept` with no `raise` to place the failure is not followed,
        // and says why.
        let rep = check_src("?try :try { 1 } set :except { drop } set !");
        assert_eq!(rep.abandoned, 1, "{rep:?}");
        assert!(
            rep.reasons[0].0.contains("reaches no `raise`"),
            "{:?}",
            rep.reasons
        );

        // `csv` and `sqlite`: a row per run, D180's rule; a column once.
        assert!(clean(&check_src("\"f.csv\" csv :lambda { println } set !")));
        assert!(clean(&check_src(
            "\"f.csv\" csv :column \"x\" set :lambda { println } set !"
        )));
        assert!(clean(&check_src(
            "\"f.db\" sqlite :sql \"select 1\" set :lambda { println } set !"
        )));
        // The row is pushed and never taken, and `dup` adds one more.
        let rep = check_src("\"f.csv\" csv :lambda { dup } set !");
        assert!(rep.reasons[0].0.contains("+2"), "{:?}", rep.reasons);

        // A slot filled with something the walk cannot see is said so.
        let rep = check_src("? :run 1 2 + set !");
        assert!(
            rep.reasons[0].0.contains("not a literal lambda"),
            "{:?}",
            rep.reasons
        );

        // `curry`: the word it registers has the effect of its body.
        let e = eff_of(":f curry :lambda { + } set :data [ 1 ] set ! f");
        assert_eq!((e.consumes, e.produces, e.opaque), (1, 1, false));
    }

    /// **D182.** One abstract stack per name: `@name` saves the current one
    /// and restores or creates the target, `( … )` returns through
    /// `endcontext`, `clear` and `fold` take the whole current stack, and a
    /// word that reaches another stack by name makes a later switch to it an
    /// abandonment rather than a claim.
    /// **D183.** A `raise` the walk reaches inside `try` ends that path with
    /// the stack `except` sees: what `try` left, then the ERROR conditional.
    #[test]
    fn a_try_is_followed_where_a_raise_places_the_failure() {
        // `raise.rs:14`: no message is not an underflow.
        assert!(clean(&check_src("raise")));

        // The `try` path ends at `raise`: `except` drops the error and leaves
        // one value, the only path there is.
        let e = eff_of("?try :try { \"boom\" raise } set :except { drop 1 } set !");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));

        // `recovery` runs on what `except` left, and `!` on the error runs
        // the `?try`'s `associated`.
        let e = eff_of(
            "?try :try { \"boom\" raise } set :except { drop 1 2 } set :recovery { drop } set !",
        );
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));
        let e =
            eff_of("?try :try { \"boom\" raise } set :except { ! } set :associated { 1 } set !");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));

        // A `raise` inside a body `if` runs leaves two paths — the body not
        // run, and `except` on what the body left — joined where they agree.
        let e = eff_of("?try :try { 1 true { \"boom\" raise } if } set :except { drop } set !");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));
        let rep =
            check_src("?try :try { 1 true { \"boom\" raise } if } set :except { drop drop } set !");
        assert_eq!(rep.abandoned, 1, "{rep:?}");
        assert!(
            rep.reasons[0].0.contains("whether `try` failed"),
            "{:?}",
            rep.reasons
        );

        // Two `raise`s at different depths: `except` runs at a depth the walk
        // cannot name.
        let rep = check_src(
            "?try :try { true { \"a\" raise } if 1 \"b\" raise } set :except { drop } set !",
        );
        assert_eq!(rep.abandoned, 1, "{rep:?}");
        assert!(
            rep.reasons[0]
                .0
                .contains("different depths, so the depth after depends on which"),
            "{:?}",
            rep.reasons
        );

        // The corpus's `testing_tryexcept.bund`: the message and `true` are
        // what the golden shows.
        assert!(clean(&check_src(
            "\"Testing\" ?try :try { \"I raise\" raise } set :except { drop true } set ! println println"
        )));
    }

    /// **D184.** `?true*` is the inline `ifthenelse`: both branches walked
    /// and joined. `while` and `for` are followed when one run nets zero
    /// beyond the condition it takes; `*loop` runs on the stack itself.
    #[test]
    fn the_inline_branches_and_the_conditional_loops_are_followed() {
        // `?true*`: the `then` lambda on top, `else` beneath, condition
        // beneath both.
        let e = eff_of("true { 1 } { 2 } ?true*");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));
        // The condition beneath both branches, then what the branch takes.
        let e = eff_of("{ drop } { drop } ?false*");
        assert_eq!((e.consumes, e.produces, e.opaque), (2, 0, false));
        assert!(clean(&check_src(
            "42 42 != { true } { false } ifthenelse println"
        )));
        let rep = check_src("true { 1 } { 1 2 } ?true*");
        assert!(
            rep.reasons[0]
                .0
                .contains("`?true*` whose `then` and `else`"),
            "{:?}",
            rep.reasons
        );
        // The lambda on top is `then` (`ifthenelse_fun.rs:28-37`).
        let rep = check_src("true { 1 } { + } ?true*");
        assert_eq!(rep.findings.len(), 1, "{rep:?}");
        assert_eq!(
            rep.findings[0].via.as_deref(),
            Some("the `then` of `?true*`")
        );

        // `while`: the condition is pulled before each run and the body
        // leaves the next; the loop ends by pulling the one it found false.
        assert!(clean(&check_src("0 true { 1 + dup 10 < } while println")));
        let e = eff_of("{ 1 + dup 10 < } while");
        assert_eq!((e.consumes, e.produces, e.opaque), (2, 1, false));
        let rep = check_src("0 true { 1 + dup dup 10 < } while");
        assert!(
            rep.reasons[0]
                .0
                .contains("`while` with a run that leaves +1"),
            "{:?}",
            rep.reasons
        );

        // `for`: the body runs first and leaves the condition pulled after.
        assert!(clean(&check_src("0 { 1 + dup 10 < } for println")));
        let e = eff_of("{ dup 10 < } for");
        assert_eq!((e.consumes, e.produces, e.opaque), (1, 1, false));
        let rep = check_src("0 { 1 + } for");
        assert!(
            rep.reasons[0].0.contains("`for` with a run that leaves -1"),
            "{:?}",
            rep.reasons
        );

        // `*loop`: the stack is walked to its end, or to a NODATA it pulls.
        let rep = check_src("1 2 3 { println } *loop");
        assert!(clean(&rep), "{rep:?}");
        assert!(clean(&check_src("5 nodata 1 2 { println } *loop println")));
        let rep = check_src("5 nodata 1 2 { println } *loop println println");
        assert_eq!(rep.findings.len(), 1, "{rep:?}");
        // A run that takes beneath the stack is a finding charged to the body:
        // three values, two taken per run, the second run one short.
        let rep = check_src("1 2 3 { drop drop } *loop");
        assert_eq!(rep.findings.len(), 1, "{rep:?}");
        assert_eq!(
            rep.findings[0].via.as_deref(),
            Some("the body `*loop` runs")
        );
        // What a native leaves is not known not to be a NODATA.
        let rep = check_src("1 2 3 { + } *loop");
        assert!(
            rep.reasons[0].0.contains("cannot rule out"),
            "{:?}",
            rep.reasons
        );
        let rep = check_src("1 2 3 { dup } *loop");
        assert!(
            rep.reasons[0].0.contains("never reaches the end"),
            "{:?}",
            rep.reasons
        );
        let e = eff_of("{ drop } *loop");
        assert!(e.opaque);
    }

    #[test]
    fn a_stack_switch_is_followed_per_name() {
        assert!(clean(&check_src(
            "@t 1 2 @main 3 @t + println @main println"
        )));

        // A fresh stack is empty, in both engines.
        let rep = check_src("1 2 @t +");
        assert_eq!(rep.findings.len(), 1, "{rep:?}");
        assert_eq!(
            (rep.findings[0].word.as_str(), rep.findings[0].have),
            ("+", 0)
        );

        // `( … )` carries its top out to the workbench; `take` brings it back.
        assert!(clean(&check_src("( 1 2 + ) take println")));
        let rep = check_src("1 2 ( + ) take println");
        assert_eq!(
            rep.findings.len(),
            1,
            "a scratch context starts empty: {rep:?}"
        );
        let rep = check_src("1 endcontext");
        assert!(
            rep.reasons[0].0.contains("no context open"),
            "{:?}",
            rep.reasons
        );

        assert!(clean(&check_src(
            ":t to_stack 1 :main to_stack 2 :t to_stack println"
        )));
        assert!(clean(&check_src("1 2 3 clear 4 println")));
        let rep = check_src("1 2 clear +");
        assert_eq!(rep.findings.len(), 1, "{rep:?}");
        assert!(clean(&check_src("1 2 3 fold println")));
        // `fold` stops at a NODATA and consumes it; what lies beneath stays.
        assert!(clean(&check_src(
            "1 2 nodata 3 4 fold println println println"
        )));
        let rep = check_src("1 2 + 3 fold println");
        assert!(rep.reasons[0].0.contains("NODATA"), "{:?}", rep.reasons);

        let rep = check_src("@t 1 stacks_right");
        assert!(
            rep.reasons[0].0.contains("changes the current stack"),
            "{:?}",
            rep.reasons
        );
        let rep = check_src("@t 1 2 @main :t rotate_stack_left @t");
        assert!(
            rep.reasons[0].0.contains("another word may have changed"),
            "{:?}",
            rep.reasons
        );

        // A body's effect is about one stack: a switch inside it has no pair.
        assert!(eff_of("@t 1").opaque);
        let rep = check_src("3 { @t drop } times");
        assert!(
            rep.reasons[0].0.contains("inside a body"),
            "{:?}",
            rep.reasons
        );
    }

    /// **D179.** A class the program defines is read as it is built, parents
    /// included, and a lambda method or constructor is walked in place.
    /// `class_hierarhy_call_method.bund` and `class_constructors_demo.bund`,
    /// as they stand in the corpus.
    #[test]
    fn a_class_the_program_defines_is_modelled() {
        let hierarchy = ":A class\n  :hello    \"Hello world!\" set\n  :greet    {\n    :hello swap ! \"A.hello returns: {A}\" format println\n    :pi swap ! \"And PI is {A}\" format println\n  } set\nregister\n:B class\n  :pi     3.14    set\n  :.super [ :A ]  set\nregister\n\"Object B.greet\" println\n:greet\n  :B object !\n";
        let rep = check_src(hierarchy);
        assert!(clean(&rep), "{rep:?}");
        assert!(rep.analysed >= 10, "{rep:?}");

        let constructors = ":A class\n  :.super [ :B :E ]  set\n  :.init { :.class_name get \"Constructor of {A}\" format println } set register\n:B class\n  :.super [ :C :D ]  set\n  :.init { :.class_name get \"Constructor of {A}\" format println } set register\n:C class\n  :.init { :.class_name get \"Constructor of {A}\" format println } set register\n:D class\n  :.init { :.class_name get \"Constructor of {A}\" format println } set register\n:E class\n  :.init { :.class_name get \"Constructor of {A}\" format println } set register\n:A object\n";
        let rep = check_src(constructors);
        assert!(clean(&rep), "{rep:?}");

        // A constructor that takes more than it has is a finding inside it.
        let rep = check_src(":A class :.init { drop drop } set register :A object");
        assert_eq!(rep.findings.len(), 1, "{rep:?}");
        assert_eq!(
            rep.findings[0].via.as_deref(),
            Some("the constructor of `A`")
        );

        // A parent nobody defines is an abandonment naming the parent.
        let rep = check_src(":A class :.super [ :Nobody ] set register :A object");
        assert_eq!(rep.abandoned, 1, "{rep:?}");
        assert!(rep.reasons[0].0.contains("`Nobody`"), "{:?}", rep.reasons);

        // `<class> !` constructs from the value — D23.
        assert!(clean(&check_src(":A class register :A class ! println")));
    }
}
