//! `smysl.prove` (§S7): a KB's proofs, written into a library as units.
//!
//! **It solves first and writes after.** Every answer and its proof is
//! found holding no lock, through the engine `kb.prove` runs (RFC-0013
//! §S15), so a bridge that writes to the same library neither deadlocks
//! nor meets SM07. Only then are §S9's locks taken, and every node judged
//! and written against the library as re-opened under them.
//!
//! - **A fact node** stands for its units when its shape is one of §S7's
//!   first seven and the library bears it out; a `rank` fact stands for
//!   nothing; any other fact is a **program fact**, a `claim` of the
//!   status `opts.facts` names.
//! - **A rule node**, and each node that has a rule's shape, is a `claim`
//!   (a `finding` at the root) whose grounds are what its premises became,
//!   and whose attestation names them as parents (R15).
//! - **Its status** keeps rule M by construction: `speculative` with no
//!   grounds, else the lower of `derived` and the weakest declared status
//!   among them, lowered to `inferred` when a capping node lies under it.
//! - **Its text** is §S7.1's rendering, normalised to NFC: the gist fitted
//!   to every profile of the library's views, the detail in full.
//!
//! D246 adds the LAMBDA row to §S7.1 (R26), answers NODATA for a root
//! that becomes no single unit (R27), and refuses `opts.facts
//! "unfounded"` with SM03 (R28).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, CALL, INTEGER, LAMBDA, LIST, MAP, NODATA, STRING};
use smysl_core::{GranularityProfile, Status, Uid};
use smysl_graph::{EffectiveStatus, RetractionPolicy, Store, effective_status, labels_of};
use unicode_normalization::UnicodeNormalization;

use crate::kb::solve::{Solver, Stop};
use crate::kb::store::Kb;
use crate::kb::words::VmBridge;

use super::form::Batch;
use super::handle::Handle;
use super::{Code, Refusal, map, sm03, sm05};

/// The word, for its texts.
const W: &str = "smysl.prove";

/// The suffix of a gist that was cut (§S7).
const CUT: &str = "…";

/// How many characters of a uid a gist keeps (§S7).
const SHORT_UID: usize = 12;

// --- the options ---------------------------------------------------------------

/// `opts`: the status a program fact is written at, and its source.
pub struct Opts {
    facts: Status,
    source: Option<BundValue>,
}

impl Opts {
    /// `opts`, operand `#k`: a MAP of `facts` and `source`, both optional.
    ///
    /// - `facts` is `"speculative"` (the default) or `"cited"`, which needs
    ///   `source`. `derived` and `inferred` need grounds, and `measured`
    ///   and `unfounded` are never authored: SM03 (D246 R28). Any other
    ///   value is SM05.
    /// - `source` is a source MAP (§S2), checked when a program fact is
    ///   written with it.
    pub fn of(v: &BundValue, k: usize) -> Result<Opts, Refusal> {
        let m = super::research::spec_of(v, k, &["facts", "source"])?;
        let facts = match m.get("facts") {
            None => Status::Speculative,
            Some(f) => {
                let s = f.str_ref().filter(|_| f.dt() == STRING).ok_or_else(|| {
                    sm03(format!(
                        "spec facts must be STRING, found {}",
                        f.type_name()
                    ))
                })?;
                match Status::parse(s) {
                    Some(st @ (Status::Speculative | Status::Cited)) => st,
                    Some(st @ (Status::Derived | Status::Inferred)) => {
                        return Err(sm03(format!(
                            "status {} needs grounds, which a program fact has none of",
                            st.as_str()
                        )));
                    }
                    Some(st) => {
                        return Err(sm03(format!("status {} cannot be authored", st.as_str())));
                    }
                    None => return Err(sm05("status", s)),
                }
            }
        };
        let source = match m.get("source") {
            None => None,
            Some(s) if s.dt() == MAP => Some(s.clone()),
            Some(s) => {
                return Err(sm03(format!(
                    "spec source must be MAP, found {}",
                    s.type_name()
                )));
            }
        };
        if facts == Status::Cited && source.is_none() {
            return Err(sm03("spec lacks source, which facts cited needs"));
        }
        Ok(Opts { facts, source })
    }
}

// --- solving -------------------------------------------------------------------

/// Every answer's proof, found holding no lock, as `kb.prove` finds them.
///
/// `kb` is operand `#3`. Refusals are `kb.prove`'s codes with this word's
/// prefix; E00 and E40 keep their house texts (§S7). With no `ask` slot,
/// an askable goal is E50.
pub fn solve(vm: &mut dyn Vm, kb: &BundValue, goal: &BundValue) -> Result<Vec<BundValue>, Error> {
    let up = W.to_uppercase();
    let stop = |s: Stop| match s {
        Stop::Code(text) => Error(format!("{up}: {text}")),
        Stop::Raw(e) => e,
    };
    let kb = Kb::read(kb, 3).map_err(|r| Error(format!("{up}: {}", r.text())))?;
    if goal.dt() == NODATA {
        return Err(Error(format!("{up}: the query has no goal")));
    }
    let mut solver = Solver::new(&kb, goal).map_err(stop)?.proving();
    let mut bridge = VmBridge::new(vm, &up);
    let mut proofs = Vec::new();
    while let Some(a) = solver.next(&mut bridge).map_err(stop)? {
        let proof = a
            .as_map()
            .filter(|_| a.dt() == MAP)
            .and_then(|m| m.get("proof"))
            .cloned()
            .ok_or_else(|| {
                Error::internal(format!(
                    "{W}: a proving solver answered {} and not {{ answer, proof }}",
                    a.type_name()
                ))
            })?;
        proofs.push(proof);
    }
    Ok(proofs)
}

// --- rendering (§S7.1) ---------------------------------------------------------

/// An atom's text: bare when it matches `[a-z][A-Za-z0-9_.]*`, otherwise in
/// double quotes, escaped so the rendering is one line.
fn atom(s: &str) -> String {
    let mut chars = s.chars();
    let bare = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    if bare {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A FLOAT: the shortest decimal that reads back as the same `f64`, with
/// `.0` if integral; `nan`, `inf` and `-inf` for the rest.
fn float(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf" } else { "-inf" }.into();
    }
    let s = format!("{f}");
    if s.contains('.') { s } else { format!("{s}.0") }
}

/// One value that is not a LIST or a LAMBDA, or SM03.
fn scalar(v: &BundValue, short: bool) -> Result<String, Refusal> {
    match v.unboxed() {
        BundValue::Bool(b, _) => return Ok(b.to_string()),
        BundValue::Int(i, _) => return Ok(i.to_string()),
        BundValue::Float(f, _) => return Ok(float(*f)),
        BundValue::Nodata(_) => return Ok("_".into()),
        _ => {}
    }
    match (v.dt(), v.str_ref()) {
        (CALL, Some(name)) => Ok(name.to_string()),
        (STRING, Some(s)) if short && Uid::parse(s).is_ok() => {
            Ok(atom(&s.chars().take(SHORT_UID).collect::<String>()))
        }
        (STRING, Some(s)) => Ok(atom(s)),
        _ => Err(sm03(format!(
            "the goal holds a {}, which has no stable text",
            v.type_name()
        ))),
    }
}

/// §S7.1's rendering of `v`, with every uid cut to its first 12
/// characters when `short`. A LIST is `[ … ]` and a LAMBDA `{ … }` (D246
/// R26); a `|` CALL is its name, so a partial list reads `[ a b | _ ]`.
/// Iterative, so a deep term cannot exhaust the native stack.
pub fn render(v: &BundValue, short: bool) -> Result<String, Refusal> {
    enum Step<'a> {
        Value(&'a BundValue),
        Text(&'static str),
    }
    let mut out = String::new();
    let mut todo = vec![Step::Value(v)];
    while let Some(step) = todo.pop() {
        let v = match step {
            Step::Text(t) => {
                out.push_str(t);
                continue;
            }
            Step::Value(v) => v,
        };
        let (items, open, close, empty) = match v.dt() {
            LIST => (v.as_list(), "[ ", " ]", "[ ]"),
            LAMBDA => (v.as_lambda(), "{ ", " }", "{ }"),
            _ => {
                out.push_str(&scalar(v, short)?);
                continue;
            }
        };
        let items = items.unwrap_or(&[]);
        if items.is_empty() {
            out.push_str(empty);
            continue;
        }
        out.push_str(open);
        todo.push(Step::Text(close));
        for (i, item) in items.iter().enumerate().rev() {
            todo.push(Step::Value(item));
            if i > 0 {
                todo.push(Step::Text(" "));
            }
        }
    }
    Ok(out)
}

fn nfc(s: &str) -> String {
    s.nfc().collect()
}

// --- fitting a gist ------------------------------------------------------------

/// The profiles a gist must fit (§S7): every distinct profile among the
/// library's views whose estimator this build has, or the `default`
/// preset when there is no view. `true` when views exist and none could be
/// evaluated, so the preset stands in for them and a warning says so.
pub fn profiles(store: &Store) -> (Vec<GranularityProfile>, bool) {
    let mut all: Vec<GranularityProfile> = Vec::new();
    for v in store.views() {
        if !all.contains(&v.granularity) {
            all.push(v.granularity.clone());
        }
    }
    if all.is_empty() {
        return (vec![GranularityProfile::standard()], false);
    }
    let known: Vec<GranularityProfile> =
        all.into_iter().filter(|p| p.tokens("").is_some()).collect();
    if known.is_empty() {
        (vec![GranularityProfile::standard()], true)
    } else {
        (known, false)
    }
}

/// `text` as a gist every profile admits: whole if it fits each, else cut
/// at the shortest prefix `fit_gist_with` gives, followed by `…`
/// (`smysl-core/src/types/view.rs:204-212`). A profile that leaves room
/// for no character is SM12.
pub fn fit(text: &str, profiles: &[GranularityProfile]) -> Result<String, Failed> {
    let fits = profiles
        .iter()
        .all(|p| p.tokens(text).is_some_and(|t| p.gist_within_bound(t)));
    if fits {
        return Ok(text.to_string());
    }
    let mut cut = text.len();
    for p in profiles {
        let Some(end) = p.fit_gist_with(text, CUT) else {
            continue;
        };
        if end == 0 {
            return Err(Refusal::new(
                Code::Sm12,
                format!("the library's view allows no gist of {} tokens", p.l0_max),
            )
            .into());
        }
        cut = cut.min(end);
    }
    let head = text.get(..cut).ok_or_else(|| {
        Error::internal(format!(
            "{W}: fit_gist_with answered byte {cut}, which is not a character boundary; \
             SMYSL backs off to one (smysl-core/src/types/estimate.rs:199-202)"
        ))
    })?;
    Ok(format!("{head}{CUT}"))
}

// --- what a fact stands for ----------------------------------------------------

/// The units a fact node stands for, when the library bears it out (§S7):
/// `Some` of its units, none for a `rank` fact; `None` for a program fact.
pub fn stands_for(store: &Store, eff: &EffectiveStatus, goal: &BundValue) -> Option<Vec<Uid>> {
    let items = goal.as_list().filter(|_| goal.dt() == LIST)?;
    let (f, args) = items.split_first()?;
    let name = f.str_ref().filter(|_| f.dt() == CALL)?;
    let text = |i: usize| args.get(i).filter(|a| a.dt() == STRING)?.str_ref();
    let uid = |i: usize| Uid::parse(text(i)?).ok();
    let held = |i: usize| uid(i).filter(|u| store.get(u).is_some());
    let core = |u: &Uid| store.get(u).map(|x| &x.core);
    let one = |u: Uid, ok: bool| ok.then(|| vec![u]);
    match (name, args.len()) {
        ("unit", 3) => {
            let u = held(0)?;
            let c = core(&u)?;
            one(
                u,
                c.schema.to_string() == text(1)? && c.status.as_str() == text(2)?,
            )
        }
        ("gist", 2) => {
            let u = held(0)?;
            one(u, core(&u)?.gist == text(1)?)
        }
        ("grounds", 2) => {
            let u = held(0)?;
            one(u, core(&u)?.grounds.contains(&uid(1)?))
        }
        ("deps", 2) => {
            let u = held(0)?;
            one(u, core(&u)?.deps.contains(&uid(1)?))
        }
        ("label", 2) => {
            let u = held(1)?;
            let l = text(0)?;
            one(u, labels_of(store, &u).iter().any(|b| b.as_str() == l))
        }
        ("effective", 2) => {
            let u = held(0)?;
            let now = eff.get(&u)?;
            one(u, now != core(&u)?.status && now.as_str() == text(1)?)
        }
        ("rel", 3) => {
            let (kind, from, to) = (text(0)?, held(1)?, held(2)?);
            let live = store.relations().any(|r| {
                r.kind.as_str() == kind && r.from == from && r.to == to && !store.is_withdrawn(r)
            });
            let units: BTreeSet<Uid> = [from, to].into_iter().collect();
            live.then(|| units.into_iter().collect())
        }
        ("rank", 2) => {
            let st = Status::parse(text(0)?)?;
            let n = args.get(1).filter(|a| a.dt() == INTEGER)?.as_int()?;
            (n == i64::from(st.as_u8())).then(Vec::new)
        }
        _ => None,
    }
}

// --- proof nodes ---------------------------------------------------------------

/// A proof node, by RFC-0013 §S15's shape `[ goal ← parts… ]`.
enum Node<'a> {
    /// `:fact`.
    Fact(&'a BundValue),
    /// `:builtin` and `:bridge`; `:no-proof` and `:tabled`, which cap.
    Nothing { cap: bool },
    /// `:branch i p`: what `p` became.
    Through(&'a BundValue),
    /// A rule, `once`, a LIST of goals, `:then`, `:else` (capping) and
    /// `:findall`: a conclusion from its premises.
    Rule {
        goal: &'a BundValue,
        premises: &'a [BundValue],
        cap: bool,
    },
}

fn broken(what: &str) -> Error {
    Error::internal(format!(
        "{W}: a proof node {what}; the engine builds every node as [ goal ← parts… ] \
         (RFC-0013 §S15)"
    ))
}

fn node(p: &BundValue) -> Result<Node<'_>, Error> {
    let items = p
        .as_list()
        .filter(|_| p.dt() == LIST)
        .ok_or_else(|| broken(&format!("is a {}, not a LIST", p.type_name())))?;
    let [goal, arrow, parts @ ..] = items else {
        return Err(broken("has fewer than two items"));
    };
    if arrow.dt() != CALL || arrow.str_ref() != Some("←") {
        return Err(broken("has no ← after its goal"));
    }
    let marker = parts
        .first()
        .filter(|m| m.dt() == STRING)
        .and_then(|m| m.str_ref());
    let rule = |premises, cap| Node::Rule {
        goal,
        premises,
        cap,
    };
    let rest = parts.get(1..).unwrap_or(&[]);
    Ok(match marker {
        None => rule(parts, false),
        Some("fact") => Node::Fact(goal),
        Some("builtin" | "bridge") => Node::Nothing { cap: false },
        Some("no-proof" | "tabled") => Node::Nothing { cap: true },
        Some("branch") => Node::Through(
            parts
                .get(2)
                .ok_or_else(|| broken("marks a branch with no proof"))?,
        ),
        Some("then" | "findall") => rule(rest, false),
        Some("else") => rule(rest, true),
        Some(m @ ("asked" | "known")) => {
            return Err(Error::internal(format!(
                "{W}: a proof holds a :{m} node, which only an ask slot gives, and {W} has none"
            )));
        }
        Some(m) => return Err(broken(&format!("has the unknown marker {m}"))),
    })
}

/// A node's goal, as a premise line names it.
fn goal_of(p: &BundValue) -> Result<&BundValue, Error> {
    match node(p)? {
        Node::Fact(g) | Node::Rule { goal: g, .. } => Ok(g),
        Node::Nothing { .. } | Node::Through(_) => p
            .as_list()
            .and_then(<[BundValue]>::first)
            .ok_or_else(|| broken("has no goal")),
    }
}

/// `<functor>/<n>` of a goal: a compound's functor and arity, an atom's
/// name and 0, and `,/<n>` for a LIST of goals, which is proved as a
/// conjunction is.
fn functor(goal: &BundValue) -> String {
    match goal.as_list().filter(|_| goal.dt() == LIST) {
        Some([f, args @ ..]) if f.dt() == CALL => {
            format!("{}/{}", f.str_ref().unwrap_or(""), args.len())
        }
        Some(items) => format!(",/{}", items.len()),
        None => format!("{}/0", goal.str_ref().unwrap_or("")),
    }
}

/// What a node became: the units it grounds a conclusion on, and whether
/// a capping node lies under it.
struct Became {
    units: Vec<Uid>,
    cap: bool,
}

/// A refusal or a defect, from inside the write.
pub enum Failed {
    Refused(Refusal),
    Internal(Error),
}

impl From<Refusal> for Failed {
    fn from(r: Refusal) -> Failed {
        Failed::Refused(r)
    }
}

impl From<Error> for Failed {
    fn from(e: Error) -> Failed {
        Failed::Internal(e)
    }
}

/// One call's export, against the library as re-opened under the locks.
struct Export<'s, 'o> {
    store: &'s Store,
    eff: EffectiveStatus,
    batch: Batch<'s>,
    profiles: Vec<GranularityProfile>,
    opts: &'o Opts,
    /// The declared status of each unit this call decided.
    declared: HashMap<Uid, Status>,
}

impl Export<'_, '_> {
    fn status_of(&self, u: &Uid) -> Result<Status, Error> {
        self.declared
            .get(u)
            .copied()
            .or_else(|| self.store.get(u).map(|x| x.core.status))
            .ok_or_else(|| {
                Error::internal(format!(
                    "{W}: ground {} is neither held nor written; a ground is a held unit \
                     or one this call wrote",
                    u.canonical()
                ))
            })
    }

    /// Write one unit for `goal`, its premises' goals in its detail.
    #[allow(clippy::too_many_arguments)]
    fn write(
        &mut self,
        schema: &str,
        status: Status,
        goal: &BundValue,
        body: String,
        premises: &[BundValue],
        grounds: BTreeSet<Uid>,
        source: Option<&BundValue>,
    ) -> Result<Uid, Failed> {
        let gist = fit(&nfc(&render(goal, true)?), &self.profiles)?;
        let mut detail = nfc(&render(goal, false)?);
        for p in premises {
            detail.push('\n');
            detail.push_str(&nfc(&render(goal_of(p)?, false)?));
        }
        let mut m = BTreeMap::new();
        m.insert("schema".to_string(), BundValue::str(schema));
        m.insert("gist".to_string(), BundValue::str(gist));
        m.insert("body".to_string(), BundValue::str(body));
        m.insert("detail".to_string(), BundValue::str(detail));
        m.insert("status".to_string(), BundValue::str(status.as_str()));
        if !grounds.is_empty() {
            m.insert(
                "grounds".to_string(),
                BundValue::list(grounds.iter().map(map::uid).collect()),
            );
        }
        if let Some(s) = source {
            m.insert("source".to_string(), s.clone());
        }
        let uid = self.batch.proved(&m, grounds)?;
        self.declared.insert(uid, status);
        Ok(uid)
    }

    /// A fact node: its units, or a program fact's `claim`.
    fn fact(&mut self, goal: &BundValue) -> Result<Became, Failed> {
        if let Some(units) = stands_for(self.store, &self.eff, goal) {
            return Ok(Became { units, cap: false });
        }
        let (facts, source) = (self.opts.facts, self.opts.source.clone());
        let uid = self.write(
            "claim",
            facts,
            goal,
            "program fact".into(),
            &[],
            BTreeSet::new(),
            source.as_ref(),
        )?;
        Ok(Became {
            units: vec![uid],
            cap: false,
        })
    }

    /// A conclusion from what its premises became.
    fn conclude(
        &mut self,
        goal: &BundValue,
        premises: &[BundValue],
        kids: Vec<Became>,
        cap: bool,
        root: bool,
    ) -> Result<Became, Failed> {
        let cap = cap || kids.iter().any(|k| k.cap);
        let grounds: BTreeSet<Uid> = kids.into_iter().flat_map(|k| k.units).collect();
        let mut status = Status::Speculative;
        if !grounds.is_empty() {
            status = Status::Derived;
            for g in &grounds {
                status = status.min(self.status_of(g)?);
            }
            if cap {
                status = status.min(Status::Inferred);
            }
        }
        let body = format!(
            "proved by {} from {} premises",
            functor(goal),
            premises.len()
        );
        let schema = if root { "finding" } else { "claim" };
        let uid = self.write(schema, status, goal, body, premises, grounds, None)?;
        Ok(Became {
            units: vec![uid],
            cap,
        })
    }

    /// One answer's proof, post-order with an explicit stack, so a deep
    /// proof cannot exhaust the native one: its root's uid, or NODATA when
    /// the root became no unit or more than one (D246 R27).
    fn answer(&mut self, proof: &BundValue) -> Result<BundValue, Failed> {
        enum Task<'a> {
            Visit(&'a BundValue, bool),
            Conclude {
                goal: &'a BundValue,
                premises: &'a [BundValue],
                cap: bool,
                root: bool,
            },
        }
        let mut todo = vec![Task::Visit(proof, true)];
        let mut done: Vec<Became> = Vec::new();
        while let Some(task) = todo.pop() {
            match task {
                Task::Visit(p, root) => match node(p)? {
                    Node::Fact(goal) => done.push(self.fact(goal)?),
                    Node::Nothing { cap } => done.push(Became {
                        units: Vec::new(),
                        cap,
                    }),
                    Node::Through(q) => todo.push(Task::Visit(q, root)),
                    Node::Rule {
                        goal,
                        premises,
                        cap,
                    } => {
                        todo.push(Task::Conclude {
                            goal,
                            premises,
                            cap,
                            root,
                        });
                        todo.extend(premises.iter().rev().map(|q| Task::Visit(q, false)));
                    }
                },
                Task::Conclude {
                    goal,
                    premises,
                    cap,
                    root,
                } => {
                    let at = done.len().checked_sub(premises.len()).ok_or_else(|| {
                        Error::internal(format!(
                            "{W}: a conclusion found fewer results than its premises"
                        ))
                    })?;
                    let kids = done.split_off(at);
                    done.push(self.conclude(goal, premises, kids, cap, root)?);
                }
            }
        }
        match done.as_slice() {
            [Became { units, .. }] => Ok(match units.as_slice() {
                [u] => map::uid(u),
                _ => BundValue::nodata(),
            }),
            _ => Err(
                Error::internal(format!("{W}: a proof left {} results, not one", done.len()))
                    .into(),
            ),
        }
    }
}

/// What the write answers: each answer's root uid, and whether the
/// library's bound could not be evaluated.
pub struct Written {
    pub roots: Vec<BundValue>,
    pub unevaluable: bool,
}

/// Write `proofs` into the library at `h`'s root, as one guarded write.
pub fn write(h: &Handle, opts: &Opts, proofs: &[BundValue]) -> Result<Written, Failed> {
    let done = super::write::write(W, &h.root, |w| {
        let decided = {
            let store = w.library.store();
            let (profiles, unevaluable) = profiles(store);
            let mut x = Export {
                store,
                eff: effective_status(store, RetractionPolicy::Strict),
                batch: Batch::new(store, &h.agent),
                profiles,
                opts,
                declared: HashMap::new(),
            };
            let roots: Result<Vec<BundValue>, Failed> =
                proofs.iter().map(|p| x.answer(p)).collect();
            roots.map(|roots| (roots, x.batch.finish().0, unevaluable))
        };
        match decided {
            Ok((roots, records, unevaluable)) => {
                w.append(&records)?;
                Ok(Ok(Written { roots, unevaluable }))
            }
            Err(Failed::Refused(r)) => Err(r),
            Err(Failed::Internal(e)) => Ok(Err(e)),
        }
    });
    match done {
        Ok(Ok(w)) => Ok(w),
        Ok(Err(e)) => Err(Failed::Internal(e)),
        Err(r) => Err(Failed::Refused(r)),
    }
}

/// The warning when no profile of the library's views could be evaluated.
pub const UNEVALUABLE: &str = "the library's views name only token estimators this build \
     does not have, so their gist bound could not be evaluated; gists are cut by the \
     default preset";
