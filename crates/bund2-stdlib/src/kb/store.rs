//! Reading, validating and writing a knowledge base's MAP (RFC-0013 §S2).
//!
//! **A KB is a value.** Every edit answers a new MAP, so a query holds the KB
//! it was handed (D5, D35). The clauses are kept as the values they were
//! written as, keyed by `"functor/arity"` under `preds`; no index is stored,
//! so an edit made with `set` cannot mislead the engine.
//!
//! **A word checks only what it reads.** A known key of the wrong kind is
//! E02 when it is read, and unknown keys are carried along untouched.
//!
//! **Which LIST is a rule.** A clause is a rule when it is a LIST whose first
//! item is itself a LIST: the head, written `[ [ p X ] ← … ]` or, for arity
//! 0, `[ [ ready ] ← … ]`. Anything else is a fact, read as §S1 reads a
//! term, so `[ ready ← … ]` is the fact `ready(←, …)`.
//!
//! Nothing here uses the `Vm`.

use std::collections::BTreeMap;

use bund2_value::{BundValue, CALL, LIST, MAP};

use super::term::{Arena, Cell, Refusal, SHOWN, Scope, Term};
use super::unify::{Trail, unify};

/// Why a KB, or a clause for one, was refused. The text is what follows
/// `<W>: ` (§S12); the word adds the prefix.
#[derive(Debug, Clone, PartialEq)]
pub enum Refused {
    /// E01: the `k`th operand is not a KB.
    NotKb(usize),
    /// E01 for the `query` conditional: its `kb` slot is missing or is not a
    /// KB. A slot has no number, so it is named as E08 names one.
    NoKbSlot,
    /// E02: a known key, named as shown, is not of its kind.
    Malformed(String, &'static str),
    /// E03: a clause would define a reserved `functor/arity`.
    Reserved(String),
    /// E04, E05, E26: a value is not a term.
    Term(Refusal),
    /// E06: a clause's head is not an atom or a compound.
    BadHead(BundValue),
    /// E07: `kb!`'s operand is not a LIST.
    NotClauses,
    /// E59: a predicate is named both `tabled` and `askable`.
    Both(String),
}

impl Refused {
    pub fn text(&self) -> String {
        match self {
            Refused::NotKb(k) => format!("#{k} parameter must be a knowledge base"),
            Refused::NoKbSlot => "kb must be a knowledge base".into(),
            Refused::Malformed(key, kind) => {
                format!("malformed knowledge base: {key} is not a {kind}")
            }
            Refused::Reserved(key) => format!("{key} is reserved and cannot be defined"),
            Refused::Term(r) => r.text(),
            Refused::BadHead(head) => format!(
                "a clause head must be an atom or a compound: {}",
                head.summary(SHOWN)
            ),
            Refused::NotClauses => "#1 parameter must be a list of clauses".into(),
            Refused::Both(key) => format!("{key} cannot be both tabled and askable"),
        }
    }
}

impl From<Refusal> for Refused {
    fn from(r: Refusal) -> Self {
        Refused::Term(r)
    }
}

/// Every phase's reserved pairs (§S3), with the phase each belongs to. An
/// arity of `None` reserves every arity.
pub const RESERVED: &[(&str, Option<usize>, u8)] = &[
    ("true", Some(0), 1),
    ("fail", Some(0), 1),
    ("=", Some(2), 1),
    ("not", None, 1),
    ("or", None, 1),
    ("if", Some(3), 1),
    ("once", None, 1),
    ("call", None, 1),
    ("findall", Some(3), 1),
    ("is", Some(2), 1),
    ("test", Some(1), 1),
    ("in", Some(2), 1),
    ("dif", Some(2), 2),
    ("#=", Some(2), 3),
    ("#≠", Some(2), 3),
    ("#<", Some(2), 3),
    ("#>", Some(2), 3),
    ("#<=", Some(2), 3),
    ("#>=", Some(2), 3),
    ("in", Some(3), 3),
    ("ins", Some(3), 3),
    ("label", Some(1), 3),
    ("labeling", Some(2), 3),
    ("all_different", Some(1), 3),
    ("fd_dom", Some(2), 3),
];

/// The phase that reserves `functor/arity`, if one does.
pub fn reserved(functor: &str, arity: usize) -> Option<u8> {
    RESERVED
        .iter()
        .find(|(f, n, _)| *f == functor && n.is_none_or(|n| n == arity))
        .map(|&(_, _, phase)| phase)
}

/// A clause split into its head and its body goals, and the value it is
/// stored as: the rule without its `←`.
struct Parts<'a> {
    head: &'a BundValue,
    goals: &'a [BundValue],
    stored: BundValue,
}

fn is_arrow(v: &BundValue) -> bool {
    v.dt() == CALL && matches!(v.str_ref(), Some("←" | "<-"))
}

/// A stored clause's head and body goals, for the solver to compile.
pub(crate) fn split(v: &BundValue) -> (&BundValue, &[BundValue]) {
    let p = parts(v);
    (p.head, p.goals)
}

fn parts(v: &BundValue) -> Parts<'_> {
    if let Some(items @ [head, rest @ ..]) = v.as_list()
        && head.dt() == LIST
    {
        return match rest {
            [arrow, goals @ ..] if is_arrow(arrow) => Parts {
                head,
                goals,
                stored: BundValue::list(std::iter::once(head).chain(goals).cloned().collect()),
            },
            _ => Parts {
                head,
                goals: rest,
                stored: BundValue::list(items.to_vec()),
            },
        };
    }
    Parts {
        head: v,
        goals: &[],
        stored: v.clone(),
    }
}

/// A clause read into `arena` as `'$clause'(Head, [Goal…])`, with its
/// predicate's key. Its variables are its own: `scope` is fresh per clause,
/// which is how a stored clause is renamed.
fn read(arena: &mut Arena, p: &Parts<'_>) -> Result<(String, Term), Refused> {
    let mut scope = Scope::default();
    let head = arena.to_term(p.head, &mut scope)?;
    let key = match arena.cell(arena.deref(head)) {
        Cell::Atom(a) => format!("{}/0", arena.atoms.text(a)),
        Cell::Struct { functor, arity, .. } => {
            format!("{}/{arity}", arena.atoms.text(functor))
        }
        _ => return Err(Refused::BadHead(p.head.clone())),
    };
    let goals = p
        .goals
        .iter()
        .map(|g| arena.to_term(g, &mut scope))
        .collect::<Result<Vec<_>, _>>()?;
    let mut body = arena.nil();
    for g in goals.into_iter().rev() {
        body = arena.cons(g, body);
    }
    let t = arena.compound("$clause", &[head, body]);
    Ok((key, t))
}

/// A clause, validated: its predicate's key and the value to store.
/// Refuses E03–E06 and E26.
pub fn clause(v: &BundValue) -> Result<(String, BundValue), Refused> {
    let p = parts(v);
    let (key, _) = read(&mut Arena::default(), &p)?;
    if let Some((f, n)) = key.rsplit_once('/')
        && reserved(f, n.parse().unwrap_or(usize::MAX)).is_some()
    {
        return Err(Refused::Reserved(key));
    }
    Ok((key, p.stored))
}

/// A KB read off an operand: its MAP, checked only for `type`.
#[derive(Debug, Clone)]
pub struct Kb {
    map: BTreeMap<String, BundValue>,
}

impl Kb {
    /// A KB with no clauses.
    pub fn new() -> Kb {
        let mut map = BTreeMap::new();
        map.insert("type".into(), BundValue::str("kb"));
        map.insert("preds".into(), BundValue::map(BTreeMap::new()));
        Kb { map }
    }

    /// `v` as a KB, the `k`th operand: a MAP whose `type` is `"kb"` (E01).
    pub fn read(v: &BundValue, k: usize) -> Result<Kb, Refused> {
        let map = v
            .as_map()
            .filter(|_| v.dt() == MAP)
            .filter(|m| m.get("type").and_then(BundValue::as_str).as_deref() == Some("kb"))
            .ok_or(Refused::NotKb(k))?;
        Ok(Kb { map: map.clone() })
    }

    /// The KB built from `clauses`, in order (`kb!`, `kb.consult*`): O(n).
    pub fn build(clauses: &[BundValue]) -> Result<Kb, Refused> {
        let mut preds: BTreeMap<String, Vec<BundValue>> = BTreeMap::new();
        for c in clauses {
            let (key, stored) = clause(c)?;
            preds.entry(key).or_default().push(stored);
        }
        let mut kb = Kb::new();
        kb.set_preds(
            preds
                .into_iter()
                .map(|(k, v)| (k, BundValue::list(v)))
                .collect(),
        );
        Ok(kb)
    }

    pub fn into_value(self) -> BundValue {
        BundValue::map(self.map)
    }

    /// `preds`; missing means no clauses (E02 if not a MAP).
    pub fn preds(&self) -> Result<BTreeMap<String, BundValue>, Refused> {
        match self.map.get("preds") {
            None => Ok(BTreeMap::new()),
            Some(v) if v.dt() == MAP => Ok(v.as_map().cloned().unwrap_or_default()),
            Some(_) => Err(Refused::Malformed("preds".into(), "MAP")),
        }
    }

    fn set_preds(&mut self, preds: BTreeMap<String, BundValue>) {
        self.map.insert("preds".into(), BundValue::map(preds));
    }

    /// One predicate's clauses; missing means none (E02 if not a LIST).
    pub fn clauses_of(
        preds: &BTreeMap<String, BundValue>,
        key: &str,
    ) -> Result<Vec<BundValue>, Refused> {
        match preds.get(key) {
            None => Ok(Vec::new()),
            Some(v) => v
                .as_list()
                .filter(|_| v.dt() == LIST)
                .map(<[BundValue]>::to_vec)
                .ok_or_else(|| Refused::Malformed(format!("preds {key}"), "LIST")),
        }
    }

    /// Every clause, in `preds` key order and then clause order.
    pub fn clauses(&self) -> Result<Vec<BundValue>, Refused> {
        let preds = self.preds()?;
        let mut all = Vec::new();
        for key in preds.keys() {
            all.extend(Kb::clauses_of(&preds, key)?);
        }
        Ok(all)
    }

    /// `tabled` or `askable`: a LIST of `"f/n"` STRINGs; missing means none.
    pub fn names(&self, key: &str) -> Result<Vec<String>, Refused> {
        let Some(v) = self.map.get(key) else {
            return Ok(Vec::new());
        };
        let items = v
            .as_list()
            .filter(|_| v.dt() == LIST)
            .ok_or_else(|| Refused::Malformed(key.into(), "LIST"))?;
        items
            .iter()
            .map(|i| {
                i.as_str()
                    .ok_or_else(|| Refused::Malformed(key.into(), "LIST of STRING"))
            })
            .collect()
    }

    /// `clause` added to its predicate: after its clauses, or before them
    /// when `first` (`kb.asserta`).
    pub fn assert(mut self, clause_value: &BundValue, first: bool) -> Result<Kb, Refused> {
        let (key, stored) = clause(clause_value)?;
        let mut preds = self.preds()?;
        let mut list = Kb::clauses_of(&preds, &key)?;
        if first {
            list.insert(0, stored);
        } else {
            list.push(stored);
        }
        preds.insert(key, BundValue::list(list));
        self.set_preds(preds);
        Ok(self)
    }

    /// The KB without the first clause whose renamed head and body unify
    /// with `pattern`; unchanged when none does. Refuses E04–E06 and E26.
    ///
    /// A stored clause that does not read as one — a LIST edited in with
    /// `set` — is passed over: it can match nothing.
    pub fn retract(mut self, pattern: &BundValue) -> Result<Kb, Refused> {
        let mut arena = Arena::default();
        let (key, want) = read(&mut arena, &parts(pattern))?;
        let mut preds = self.preds()?;
        let mut list = Kb::clauses_of(&preds, &key)?;
        let mut trail = Trail::default();
        let mut found = None;
        for (i, c) in list.iter().enumerate() {
            let mark = arena.mark();
            let bound = trail.mark();
            let hit = read(&mut arena, &parts(c))
                .is_ok_and(|(k, t)| k == key && unify(&mut arena, &mut trail, want, t));
            trail.undo_to(&mut arena, bound);
            arena.reset(mark);
            if hit {
                found = Some(i);
                break;
            }
        }
        let Some(i) = found else {
            return Ok(self);
        };
        list.remove(i);
        if list.is_empty() {
            preds.remove(&key);
        } else {
            preds.insert(key, BundValue::list(list));
        }
        self.set_preds(preds);
        Ok(self)
    }

    /// `other`'s clauses after this KB's, per predicate; `tabled` and
    /// `askable` unioned; this KB's `inferences` and other keys kept.
    pub fn merge(mut self, other: &Kb) -> Result<Kb, Refused> {
        let mut preds = self.preds()?;
        let theirs = other.preds()?;
        for key in theirs.keys() {
            let mut list = Kb::clauses_of(&preds, key)?;
            list.extend(Kb::clauses_of(&theirs, key)?);
            preds.insert(key.clone(), BundValue::list(list));
        }
        let mut lists = Vec::new();
        for key in ["tabled", "askable"] {
            let mut names = self.names(key)?;
            for n in other.names(key)? {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
            lists.push(names);
        }
        if let [tabled, askable] = &lists[..]
            && let Some(both) = tabled.iter().find(|n| askable.contains(n))
        {
            return Err(Refused::Both(both.clone()));
        }
        for (key, names) in ["tabled", "askable"].into_iter().zip(lists) {
            if self.map.contains_key(key) || other.map.contains_key(key) {
                let list = names.into_iter().map(BundValue::str).collect();
                self.map.insert(key.into(), BundValue::list(list));
            }
        }
        self.set_preds(preds);
        Ok(self)
    }

    /// The inference cap, if one is set: `inferences` above 0 (§S7). E02
    /// if it is not an INT.
    pub fn inferences(&self) -> Result<Option<u64>, Refused> {
        match self.map.get("inferences") {
            None => Ok(None),
            Some(v) if v.dt() == bund2_value::INTEGER => Ok(v
                .as_int()
                .and_then(|n| u64::try_from(n).ok())
                .filter(|n| *n > 0)),
            Some(_) => Err(Refused::Malformed("inferences".into(), "INT")),
        }
    }

    /// The KB with its inference cap set; ≤ 0 means none (§S7).
    pub fn with_inferences(mut self, n: i64) -> Kb {
        self.map.insert("inferences".into(), BundValue::int(n));
        self
    }
}

impl Default for Kb {
    fn default() -> Self {
        Kb::new()
    }
}
