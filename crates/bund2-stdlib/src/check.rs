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

use bund2_api::{MethodEffect, Registry, StackEffect};
use bund2_value::{
    BOOL, BundValue, CALL, CLASS, CONDITIONAL, CONTEXT, FLOAT, INTEGER, LAMBDA, LIST, MAP, OBJECT,
    PTR, STRING,
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

    /// A word with a fixed pair: take, then leave that many unknowns.
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
        stack.push_unknown(eff.produces);
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
                // `@name` and a CONTEXT both change which stack the walk is
                // about, so the stack stops meaning anything.
                CONTEXT => return self.abandon("a stack switch".into()),
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
        if level >= MAX_INFER_DEPTH {
            return StackEffect::opaque(0);
        }
        let saved_report = std::mem::take(&mut self.report);
        let saved_why = std::mem::take(&mut self.why);
        let saved_mode = self.mode;
        self.mode = Mode::Infer;
        let mut st = Stack::default();
        let went = self.walk(body, &mut st, Some(0), None, level);
        self.report = saved_report;
        self.why = saved_why;
        self.mode = saved_mode;
        if !went {
            return StackEffect::opaque(0);
        }
        let consumes = st.dip.clamp(0, i64::from(u8::MAX)) as u8;
        let produces = st.items.len().min(usize::from(u8::MAX)) as u8;
        StackEffect::fixed(consumes, produces)
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
                            _ => stack.push(Abs::Unknown),
                        }
                    }
                    None => stack.push_unknown(1),
                }
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
        assert!(eff_of("{ drop } loop").opaque, "`loop` runs a body");
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
