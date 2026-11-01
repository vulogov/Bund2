//! Forward chaining: `kb.saturate` (RFC-0013 §S19).
//!
//! **What it accepts.** Each rule is read once, in `preds` key order and
//! then clause order, and the first that cannot be saturated is E55:
//!
//! - `or`, `if` or `once` anywhere in its body;
//! - a goal whose functor is not written: `call`'s first argument, or a goal,
//!   that is a variable;
//! - a head variable that no positive goal names. A positive goal is a
//!   predicate's call, `call`'s literal one included, `=`, or `findall`'s
//!   result. A bridge is not one, nor `dif`, nor anything under `not` or in
//!   `findall`'s goal.
//!
//! The rules' dependencies are then stratified. A predicate named under
//! `not` or in `findall`'s goal is a negative dependency, and a predicate
//! that depends negatively on itself is E56.
//!
//! **How it runs: semi-naive, stratum by stratum.** Each round is one
//! `solve.rs` activation over a KB of facts alone, so every built-in and
//! bridge is the solver's own. Each rule is a clause of one internal
//! predicate, `join`, whose answers are the facts the rule derives. Round 1
//! joins each rule in full. A later round joins each rule once per positive
//! goal over a predicate of the stratum: that goal reads the facts new in the
//! previous round, the goals before it the facts known before them, and the
//! goals after it all. A fact that is a variant of a known one is dropped.
//!
//! **The internal names cannot collide.** `join` and the views of a
//! predicate's old and new facts carry a prefix that no predicate the KB
//! names begins with.
//!
//! Nothing here uses the `Vm`.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use bund2_api::Error;
use bund2_value::{BundValue, CALL, LIST};

use super::solve::{Bridge, Solver, Stop};
use super::store::{self, Kb, Refused};
use super::table::Tables;
use super::term::{Arena, Cell, SHOWN, Scope, Term};

/// E55's three reasons, after the variable's.
const CONTROL: &str = "or, if and once are not allowed";
const LITERAL: &str = "call needs a literal functor";

/// One body goal, as a round's join clause writes it.
enum Lit {
    /// A positive call of the predicate keyed `"f/n"`: what a round reads
    /// through a view.
    Pred(String, BundValue),
    Other(BundValue),
}

/// A rule, read and accepted.
struct Rule {
    key: String,
    head: BundValue,
    body: Vec<Lit>,
    /// Each predicate its body names, and whether under `not` or `findall`.
    deps: Vec<(String, bool)>,
}

/// The rule as written, with its `←`, for E55's text.
fn shown(head: &BundValue, goals: &[BundValue]) -> String {
    let mut items = vec![head.clone(), BundValue::call("←")];
    items.extend_from_slice(goals);
    BundValue::list(items).summary(SHOWN)
}

/// `t`'s unbound variables, by first occurrence.
fn vars(arena: &Arena, t: Term) -> Vec<Term> {
    let mut out = Vec::new();
    let mut todo = vec![t];
    while let Some(t) = todo.pop() {
        let t = arena.deref(t);
        match arena.cell(t) {
            Cell::Var(_) if !out.contains(&t) => out.push(t),
            Cell::Struct { start, arity, .. } => {
                todo.extend(arena.args(start, arity).iter().rev());
            }
            Cell::Cons(head, tail) => todo.extend([tail, head]),
            _ => {}
        }
    }
    out
}

/// A stored rule read, checked (E55), and written back with `call`s of a
/// literal functor and LISTs of goals flattened into the body.
fn rule(key: &str, stored: &BundValue) -> Result<Rule, Stop> {
    let (head_v, goals_v) = store::split(stored);
    let mut arena = Arena::default();
    let mut scope = Scope::default();
    let head = arena.to_term(head_v, &mut scope).map_err(Refused::from)?;
    let goals = goals_v
        .iter()
        .map(|g| arena.to_term(g, &mut scope))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Refused::from)?;
    let refuse = |reason: &str| {
        Stop::Code(format!(
            "saturate: {} cannot be saturated: {reason}",
            shown(head_v, goals_v)
        ))
    };
    // Each goal, and whether it is under `not` or `findall`: what is under
    // one is walked for its dependencies and is not a body goal of its own.
    let mut body: Vec<(Option<String>, Term)> = Vec::new();
    let mut deps = Vec::new();
    let mut bound = HashSet::new();
    let mut todo: Vec<(Term, bool)> = goals.iter().rev().map(|g| (*g, false)).collect();
    while let Some((g, neg)) = todo.pop() {
        let g = arena.deref(g);
        let (functor, args) = match arena.cell(g) {
            Cell::Var(_) => return Err(refuse(LITERAL)),
            Cell::Atom(a) => (a, Vec::new()),
            Cell::Struct {
                functor,
                start,
                arity,
            } => (functor, arena.args(start, arity).to_vec()),
            Cell::Cons(..) => {
                let (items, end) = arena.spine(g);
                if matches!(arena.cell(end), Cell::Nil) {
                    todo.extend(items.into_iter().rev().map(|i| (i, neg)));
                } else if !neg {
                    body.push((None, g));
                }
                continue;
            }
            _ => {
                if !neg {
                    body.push((None, g));
                }
                continue;
            }
        };
        let f = arena.atoms.text(functor).to_string();
        match (f.as_str(), args.as_slice()) {
            ("or", _) | ("if", [_, _, _]) | ("once", _) => return Err(refuse(CONTROL)),
            ("call", [target, extra @ ..]) => {
                let target = arena.deref(*target);
                match arena.cell(target) {
                    Cell::Var(_) => return Err(refuse(LITERAL)),
                    Cell::Atom(a) => {
                        let t = arena.structure(a, extra);
                        todo.push((t, neg));
                    }
                    Cell::Struct {
                        functor,
                        start,
                        arity,
                    } => {
                        let mut all = arena.args(start, arity).to_vec();
                        all.extend_from_slice(extra);
                        let t = arena.structure(functor, &all);
                        todo.push((t, neg));
                    }
                    Cell::Cons(..) | Cell::Nil if extra.is_empty() => todo.push((target, neg)),
                    // Not callable: the solver's E11.
                    _ if !neg => body.push((None, g)),
                    _ => {}
                }
            }
            ("not", [_, ..]) => {
                todo.extend(args.iter().rev().map(|a| (*a, true)));
                if !neg {
                    body.push((None, g));
                }
            }
            ("findall", [_, goal, result]) => {
                todo.push((*goal, true));
                if !neg {
                    bound.extend(vars(&arena, *result));
                    body.push((None, g));
                }
            }
            ("=", [_, _]) if !neg => {
                bound.extend(vars(&arena, g));
                body.push((None, g));
            }
            _ if store::reserved(&f, args.len()).is_some() => {
                if !neg {
                    body.push((None, g));
                }
            }
            _ => {
                let key = format!("{f}/{}", args.len());
                deps.push((key.clone(), neg));
                if !neg {
                    bound.extend(vars(&arena, g));
                    body.push((Some(key), g));
                }
            }
        }
    }
    for v in vars(&arena, head) {
        if !bound.contains(&v) {
            let name = scope
                .names()
                .iter()
                .find(|(_, t)| *t == v)
                .map_or("_", |(n, _)| n.as_str());
            return Err(refuse(&format!("{name} occurs only in the head")));
        }
    }
    // Written back with each variable named: as written, or `_`*k* for one
    // that had no name, `_` or NODATA, avoiding the names written.
    let written: HashMap<Term, String> =
        scope.names().iter().map(|(n, t)| (*t, n.clone())).collect();
    let fresh = RefCell::new((HashMap::<Term, String>::new(), 0usize));
    let named = |v: Term| {
        if let Some(n) = written.get(&v) {
            return BundValue::call(n.as_str());
        }
        let mut fresh = fresh.borrow_mut();
        if let Some(n) = fresh.0.get(&v) {
            return BundValue::call(n.as_str());
        }
        let name = loop {
            let name = format!("_{}", fresh.1);
            fresh.1 += 1;
            if !written.values().any(|w| *w == name) {
                break name;
            }
        };
        fresh.0.insert(v, name.clone());
        BundValue::call(name)
    };
    let body = body
        .into_iter()
        .map(|(key, g)| {
            let v = arena.to_value_with(g, &named);
            match key {
                Some(k) => Lit::Pred(k, v),
                None => Lit::Other(v),
            }
        })
        .collect();
    Ok(Rule {
        key: key.to_string(),
        head: arena.to_value_with(head, &named),
        body,
        deps,
    })
}

/// Each rule head's stratum: the least numbering in which a predicate's
/// stratum is at least each positive dependency's, and above each negative
/// one's. E56 names the first predicate, in key order, that depends
/// negatively on itself.
fn stratify(rules: &[Rule]) -> Result<BTreeMap<String, usize>, Stop> {
    let mut edges: BTreeMap<&str, Vec<(&str, bool)>> = BTreeMap::new();
    let mut nodes: BTreeSet<&str> = BTreeSet::new();
    for r in rules {
        nodes.insert(&r.key);
        let out = edges.entry(&r.key).or_default();
        for (k, neg) in &r.deps {
            nodes.insert(k);
            out.push((k, *neg));
        }
    }
    // What each predicate reaches, itself included.
    let reach: HashMap<&str, HashSet<&str>> = nodes
        .iter()
        .map(|&p| {
            let mut seen = HashSet::from([p]);
            let mut todo = vec![p];
            while let Some(u) = todo.pop() {
                for (v, _) in edges.get(u).map_or(&[][..], Vec::as_slice) {
                    if seen.insert(*v) {
                        todo.push(*v);
                    }
                }
            }
            (p, seen)
        })
        .collect();
    let reaches = |a: &str, b: &str| reach.get(a).is_some_and(|s| s.contains(b));
    for &p in edges.keys() {
        let cyclic = edges
            .iter()
            .any(|(u, out)| reaches(p, u) && out.iter().any(|(v, neg)| *neg && reaches(v, p)));
        if cyclic {
            return Err(Stop::Code(format!(
                "saturate: {p} depends negatively on itself"
            )));
        }
    }
    let mut stratum: BTreeMap<String, usize> = nodes.iter().map(|n| (n.to_string(), 0)).collect();
    // With no negative cycle, the longest chain of dependencies settles
    // within one pass per predicate, as Bellman-Ford's does.
    for _ in 0..=nodes.len() {
        let mut changed = false;
        for (u, out) in &edges {
            for (v, neg) in out {
                let need = stratum.get(*v).copied().unwrap_or(0) + usize::from(*neg);
                if let Some(s) = stratum.get_mut(*u)
                    && *s < need
                {
                    *s = need;
                    changed = true;
                }
            }
        }
        if !changed {
            return Ok(stratum);
        }
    }
    Err(Stop::Raw(Error::internal(
        "kb.saturate: the strata did not settle, though no predicate depends \
         negatively on itself",
    )))
}

/// The facts known, by their variant (§S18's key), so a derived variant of
/// one is dropped.
#[derive(Default)]
struct Seen {
    arena: Arena,
    tables: Tables,
    keys: HashSet<String>,
}

impl Seen {
    /// Whether `fact` is new; it is known from now on.
    fn insert(&mut self, fact: &BundValue) -> Result<bool, Stop> {
        let mark = self.arena.mark();
        let t = self
            .arena
            .to_term(fact, &mut Scope::default())
            .map_err(Refused::from)?;
        let key = self.tables.variant(&self.arena, t, &[]);
        self.arena.reset(mark);
        Ok(self.keys.insert(key))
    }
}

/// The internal predicates' functors.
struct Names {
    prefix: String,
}

impl Names {
    /// A prefix no key in `keys` begins with.
    fn new<'a>(keys: impl Iterator<Item = &'a String> + Clone) -> Names {
        let mut prefix = "$sat".to_string();
        while keys.clone().any(|k| k.starts_with(&prefix)) {
            prefix.push('$');
        }
        Names { prefix }
    }

    fn join(&self) -> String {
        format!("{}join", self.prefix)
    }

    /// The functor of `key`'s old facts, or its new ones.
    fn view(&self, key: &str, new: bool) -> String {
        let f = key.rsplit_once('/').map_or(key, |(f, _)| f);
        let which = if new { "new" } else { "old" };
        format!("{}{which}:{f}", self.prefix)
    }
}

/// `goal` or a fact with its functor replaced by `functor`.
fn renamed(v: &BundValue, functor: &str) -> BundValue {
    match v.as_list().filter(|_| v.dt() == LIST) {
        Some([first, args @ ..]) if first.dt() == CALL => {
            let mut items = vec![BundValue::call(functor)];
            items.extend_from_slice(args);
            BundValue::list(items)
        }
        _ => BundValue::call(functor),
    }
}

/// The key a view's functor makes of `key`.
fn view_key(names: &Names, key: &str, new: bool) -> String {
    let n = key.rsplit_once('/').map_or("0", |(_, n)| n);
    format!("{}/{n}", names.view(key, new))
}

/// `kb` with every fact its rules derive appended to its predicate: by
/// round, then by rule, then in join order (§S19).
pub fn saturate(kb: &Kb, bridge: &mut dyn Bridge) -> Result<Kb, Stop> {
    let preds = kb.preds()?;
    let cap = kb.inferences()?;
    let mut clauses: BTreeMap<String, Vec<BundValue>> = BTreeMap::new();
    let mut facts: BTreeMap<String, Vec<BundValue>> = BTreeMap::new();
    let mut rules = Vec::new();
    for key in preds.keys() {
        let list = Kb::clauses_of(&preds, key)?;
        let held = facts.entry(key.clone()).or_default();
        for c in &list {
            if store::split(c).1.is_empty() {
                held.push(c.clone());
            } else {
                rules.push(rule(key, c)?);
            }
        }
        clauses.insert(key.clone(), list);
    }
    // A predicate a body names and nothing defines has no facts.
    for r in &rules {
        for (k, _) in &r.deps {
            facts.entry(k.clone()).or_default();
        }
    }
    let strata = stratify(&rules)?;
    let mut seen = Seen::default();
    for fact in facts.values().flatten() {
        seen.insert(fact)?;
    }
    let names = Names::new(facts.keys());
    let join = names.join();
    let mut derived: BTreeMap<String, Vec<BundValue>> = BTreeMap::new();
    let mut spent = 0;
    let top = strata.values().copied().max().unwrap_or(0);
    for s in 0..=top {
        let members: Vec<&Rule> = rules
            .iter()
            .filter(|r| strata.get(&r.key) == Some(&s))
            .collect();
        let heads: BTreeSet<&str> = members.iter().map(|r| r.key.as_str()).collect();
        // Where each head's facts stood when the previous round began:
        // before it, the old facts; from it, the new. `None` in round 1.
        let mut before: Option<BTreeMap<&str, usize>> = None;
        while !members.is_empty() {
            let now: BTreeMap<&str, usize> = heads
                .iter()
                .map(|k| (*k, facts.get(*k).map_or(0, Vec::len)))
                .collect();
            let mut joins = Vec::new();
            for r in &members {
                let head = BundValue::list(vec![
                    BundValue::call(join.as_str()),
                    BundValue::str(r.key.as_str()),
                    r.head.clone(),
                ]);
                let clause = |goals: Vec<BundValue>| {
                    BundValue::list(std::iter::once(head.clone()).chain(goals).collect())
                };
                let Some(before) = &before else {
                    let goals = r
                        .body
                        .iter()
                        .map(|l| match l {
                            Lit::Pred(_, v) | Lit::Other(v) => v.clone(),
                        })
                        .collect();
                    joins.push(clause(goals));
                    continue;
                };
                let grew = |k: &str| before.get(k) < now.get(k);
                for (i, l) in r.body.iter().enumerate() {
                    let Lit::Pred(k, _) = l else { continue };
                    if !heads.contains(k.as_str()) || !grew(k) {
                        continue;
                    }
                    let goals = r
                        .body
                        .iter()
                        .enumerate()
                        .map(|(j, l)| match l {
                            Lit::Pred(k, v) if heads.contains(k.as_str()) && j <= i => {
                                renamed(v, &names.view(k, j == i))
                            }
                            Lit::Pred(_, v) | Lit::Other(v) => v.clone(),
                        })
                        .collect();
                    joins.push(clause(goals));
                }
            }
            if joins.is_empty() {
                break;
            }
            let mut round: BTreeMap<String, Vec<BundValue>> =
                facts.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            if let Some(before) = &before {
                for k in &heads {
                    let all = facts.get(*k).map_or(&[][..], Vec::as_slice);
                    let at = before.get(k).copied().unwrap_or(0).min(all.len());
                    let (old, new) = all.split_at(at);
                    for (part, is_new) in [(old, false), (new, true)] {
                        let f = names.view(k, is_new);
                        round.insert(
                            view_key(&names, k, is_new),
                            part.iter().map(|v| renamed(v, &f)).collect(),
                        );
                    }
                }
            }
            round.insert(format!("{join}/2"), joins);
            let mut round_kb = Kb::new().with_preds(round);
            if let Some(cap) = cap {
                round_kb = round_kb.with_inferences(i64::try_from(cap).unwrap_or(i64::MAX));
            }
            let mut solver = Solver::for_call(
                &round_kb,
                &join,
                &[BundValue::nodata(), BundValue::nodata()],
            )?
            .spent(spent)
            .naming();
            let mut found = Vec::new();
            while let Some(answer) = solver.next(bridge)? {
                if let Some([key, fact]) = answer.as_list()
                    && let Some(key) = key.as_str()
                {
                    found.push((key, fact.clone()));
                }
            }
            spent = solver.inferences();
            let mut added = false;
            for (key, fact) in found {
                if seen.insert(&fact)? {
                    facts.entry(key.clone()).or_default().push(fact.clone());
                    derived.entry(key).or_default().push(fact);
                    added = true;
                }
            }
            if !added {
                break;
            }
            before = Some(now);
        }
    }
    for (key, more) in derived {
        clauses.entry(key).or_default().extend(more);
    }
    Ok(kb.clone().with_preds(clauses))
}

/// **Criterion 21**: saturation (§S19).
#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    use super::Names;
    use crate::kb::store::{Kb, split};
    use crate::kb::term::{Arena, Scope};

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

    /// A value as its term reads back, so a fact written with CALLs and one
    /// derived, with STRINGs, show alike.
    fn canon(v: &BundValue) -> String {
        let mut arena = Arena::default();
        let t = arena.to_term(v, &mut Scope::default()).unwrap();
        arena.to_value(t).display()
    }

    /// The facts of `key` in the KB `src` leaves, in clause order.
    fn facts(src: &str, key: &str) -> Vec<String> {
        let kb = Kb::read(&top(src), 1).unwrap();
        let preds = kb.preds().unwrap();
        Kb::clauses_of(&preds, key)
            .unwrap()
            .iter()
            .filter(|c| split(c).1.is_empty())
            .map(canon)
            .collect()
    }

    /// `[ f a b … ]`, as `canon` shows it.
    fn fact(items: &[&str]) -> String {
        let mut v = vec![BundValue::call(items[0])];
        v.extend(items[1..].iter().map(|x| BundValue::str(*x)));
        canon(&BundValue::list(v))
    }

    const EXAMPLE_6: &str = "[ [ edge a b ] [ edge b c ] [ edge c a ] \
                             [ [ path X Y ] ← [ path X Z ] [ edge Z Y ] ] \
                             [ [ path X Y ] ← [ edge X Y ] ] ] kb!";

    /// Example 6: 3 edges, 2 rules and 9 path facts.
    #[test]
    fn example_6_saturates_to_nine_paths() {
        let all = top(&format!("{EXAMPLE_6} kb.saturate kb.clauses"));
        assert_eq!(all.as_list().map(<[BundValue]>::len), Some(14));
        let mut paths = facts(&format!("{EXAMPLE_6} kb.saturate"), "path/2");
        paths.sort();
        let mut want = Vec::new();
        for x in ["a", "b", "c"] {
            for y in ["a", "b", "c"] {
                want.push(fact(&["path", x, y]));
            }
        }
        want.sort();
        assert_eq!(paths, want);
    }

    /// A graph of `n` nodes `n0…`: a cycle, and a chord from each third
    /// node, as edges.
    fn graph(n: usize) -> String {
        let mut edges = String::new();
        for i in 0..n {
            edges.push_str(&format!("[ edge n{i} n{} ] ", (i + 1) % n));
            if i % 3 == 0 {
                edges.push_str(&format!("[ edge n{i} n{} ] ", (i + 5) % n));
            }
        }
        edges.push_str(&format!("[ edge x0 n0 ] [ edge n{} y0 ] ", n / 2));
        edges
    }

    /// **Criterion 21:** the saturated `path` facts for a source equal the
    /// tabled answers for that source, for left-, right- and doubly
    /// recursive `path`.
    #[test]
    fn saturated_paths_equal_tabled_answers() {
        let rules = [
            "[ [ path X Y ] ← [ path X Z ] [ edge Z Y ] ] [ [ path X Y ] ← [ edge X Y ] ]",
            "[ [ path X Y ] ← [ edge X Y ] ] [ [ path X Y ] ← [ edge X Z ] [ path Z Y ] ]",
            "[ [ path X Y ] ← [ edge X Y ] ] [ [ path X Y ] ← [ path X Z ] [ path Z Y ] ]",
        ];
        for edges in ["[ edge a b ] [ edge b c ] [ edge c a ] ", &graph(10)] {
            for r in rules {
                let kb = format!("[ {edges} {r} ] kb!");
                let saturated = facts(&format!("{kb} kb.saturate"), "path/2");
                let mut sources: Vec<String> = Vec::new();
                for e in edges.split("] ").filter(|e| !e.is_empty()) {
                    let s = e.split_whitespace().nth(2).unwrap().to_string();
                    if !sources.contains(&s) {
                        sources.push(s);
                    }
                }
                for s in sources {
                    let tabled = top(&format!("{kb} :path 2 kb.table [ path {s} Y ] kb.query"));
                    let mut want: Vec<String> = tabled
                        .as_list()
                        .unwrap()
                        .iter()
                        .map(|a| {
                            let y = a.as_map().unwrap().get("Y").unwrap().as_str().unwrap();
                            fact(&["path", &s, &y])
                        })
                        .collect();
                    want.sort();
                    let prefix = format!("{} ", fact(&["path", &s]).trim_end_matches(" ]"));
                    let mut got: Vec<String> = saturated
                        .iter()
                        .filter(|f| f.starts_with(&prefix))
                        .cloned()
                        .collect();
                    got.sort();
                    assert_eq!(got, want, "path {s} for {r}");
                }
            }
        }
    }

    /// Derived facts are appended by round, then by rule, then in join
    /// order.
    #[test]
    fn facts_come_by_round_then_rule_then_join_order() {
        let kb = "[ [ edge a b ] [ edge b c ] [ edge c d ] \
                  [ [ path X Y ] ← [ path X Z ] [ edge Z Y ] ] \
                  [ [ path X Y ] ← [ edge X Y ] ] ] kb!";
        let want: Vec<String> = [
            ["path", "a", "b"],
            ["path", "b", "c"],
            ["path", "c", "d"],
            ["path", "a", "c"],
            ["path", "b", "d"],
            ["path", "a", "d"],
        ]
        .iter()
        .map(|f| fact(f))
        .collect();
        assert_eq!(facts(&format!("{kb} kb.saturate"), "path/2"), want);
    }

    /// Stratified `not` and `findall` read the strata below them complete;
    /// a `test` bridge runs once per binding; `call` of a literal functor is
    /// the goal it makes.
    #[test]
    fn negation_aggregation_and_bridges_saturate() {
        let kb = "[ [ node a ] [ node b ] [ node c ] [ edge a b ] [ edge b c ] \
                  [ [ path X Y ] ← [ edge X Y ] ] \
                  [ [ path X Y ] ← [ call path X Z ] [ edge Z Y ] ] \
                  [ [ cut X Y ] ← [ node X ] [ node Y ] [ not [ path X Y ] ] ] \
                  [ [ reach X L ] ← [ node X ] [ findall Y [ path X Y ] L ] ] \
                  [ [ num 3 ] ] [ [ num 7 ] ] [ [ num 9 ] ] \
                  [ [ big N ] ← [ num N ] [ test { N 5 > } ] ] ] kb!";
        let src = format!("{kb} kb.saturate");
        let cut = facts(&src, "cut/2");
        assert_eq!(cut.len(), 9 - 3, "{cut:?}");
        assert!(!cut.contains(&fact(&["cut", "a", "c"])));
        assert!(cut.contains(&fact(&["cut", "c", "a"])));
        let reach = facts(&src, "reach/2");
        assert_eq!(reach.len(), 3, "{reach:?}");
        let reached = |x: &str, ys: &[&str]| {
            let ys = ys.iter().map(|y| BundValue::str(*y)).collect();
            canon(&BundValue::list(vec![
                BundValue::call("reach"),
                BundValue::str(x),
                BundValue::list(ys),
            ]))
        };
        let want = [
            reached("a", &["b", "c"]),
            reached("b", &["c"]),
            reached("c", &[]),
        ];
        assert_eq!(reach, want);
        let big = facts(&src, "big/1");
        let queried = top(&format!("{kb} [ big N ] kb.query"));
        assert_eq!(big.len(), queried.as_list().unwrap().len(), "{big:?}");
        assert!(!big.is_empty());
    }

    /// Saturating twice gives the same value, a non-ground fact's variables
    /// kept apart or shared as derived.
    #[test]
    fn saturating_twice_gives_the_same_value() {
        let kbs = [
            EXAMPLE_6.to_string(),
            format!(
                "[ {} [ [ path X Y ] ← [ edge X Y ] ] \
                     [ [ path X Y ] ← [ path X Z ] [ path Z Y ] ] ] kb!",
                graph(7)
            ),
            "[ [ same X X ] [ any _ ] [ [ pair X Y ] ← [ same X Y ] ] \
               [ [ both X Y ] ← [ any X ] [ any Y ] ] \
               [ [ lone X ] ← [ any X ] [ not [ same X b ] ] ] ] kb!"
                .to_string(),
        ];
        for kb in kbs {
            let once = top(&format!("{kb} kb.saturate"));
            let twice = top(&format!("{kb} kb.saturate kb.saturate"));
            assert_eq!(twice.display(), once.display(), "{kb}");
        }
        let src = "[ [ same X X ] [ any _ ] [ [ pair X Y ] ← [ same X Y ] ] \
                   [ [ both X Y ] ← [ any X ] [ any Y ] ] ] kb! kb.saturate";
        // Each derived fact's variables are CALLs `_G`k, by first occurrence.
        let args = |key| {
            let kb = Kb::read(&top(src), 1).unwrap();
            let preds = kb.preds().unwrap();
            let last = Kb::clauses_of(&preds, key).unwrap().pop().unwrap();
            last.as_list().unwrap()[1..]
                .iter()
                .map(|a| (a.dt() == bund2_value::CALL, a.as_str().unwrap()))
                .collect::<Vec<_>>()
        };
        let var = |n: &str| (true, n.to_string());
        assert_eq!(args("pair/2"), [var("_G0"), var("_G0")]);
        assert_eq!(args("both/2"), [var("_G0"), var("_G1")]);
    }

    /// The input's clauses, rules among them, are kept in place, and its
    /// other keys with them.
    #[test]
    fn the_input_is_kept() {
        let kb = top(&format!("{EXAMPLE_6} :path 2 kb.table kb.saturate"));
        let m = kb.as_map().unwrap();
        assert!(m.contains_key("tabled"));
        let kb = Kb::read(&kb, 1).unwrap();
        let preds = kb.preds().unwrap();
        let path = Kb::clauses_of(&preds, "path/2").unwrap();
        assert!(!split(&path[0]).1.is_empty() && !split(&path[1]).1.is_empty());
        assert_eq!(path.len(), 2 + 9);
    }

    /// **Criterion 21:** each of E55's reasons.
    #[test]
    fn e55_names_the_clause_and_its_reason() {
        let e = |rules: &str| err(&format!("[ [ q a ] {rules} ] kb! kb.saturate"));
        // The clause as written, `summary`'s form, as every goal in a
        // refusal is (D36).
        let at = |rule: &str, reason: &str| {
            format!(
                "KB.SATURATE: saturate: {} cannot be saturated: {reason}",
                top(rule).summary(crate::kb::term::SHOWN)
            )
        };
        assert_eq!(
            e("[ [ p X Y ] ← [ q X ] ]"),
            at("[ [ p X Y ] ← [ q X ] ]", "Y occurs only in the head")
        );
        // A bridge, `dif` and `not` bind nothing for the head.
        for body in [
            "[ q X ] [ is N { 1 } ]",
            "[ q X ] [ dif N X ]",
            "[ q X ] [ not [ q N ] ]",
        ] {
            assert!(
                e(&format!("[ [ p X N ] ← {body} ]")).ends_with("N occurs only in the head"),
                "{body}"
            );
        }
        assert!(e("[ [ p _ ] ← [ q a ] ]").ends_with("_ occurs only in the head"));
        for body in [
            "[ or [ q X ] [ q X ] ]",
            "[ if [ q X ] [ true ] [ fail ] ]",
            "[ once [ q X ] ]",
            "[ q X ] [ not [ once [ q X ] ] ]",
        ] {
            assert!(
                e(&format!("[ [ p X ] ← {body} ]")).ends_with(": or, if and once are not allowed"),
                "{body}"
            );
        }
        for body in ["[ q X ] [ call G X ]", "[ q X ] G", "[ q X ] [ not G ]"] {
            assert!(
                e(&format!("[ [ p X ] ← {body} ]")).ends_with(": call needs a literal functor"),
                "{body}"
            );
        }
        // `call` of a literal functor, and `findall`'s result, are positive.
        assert_eq!(
            facts(
                "[ [ q a ] [ [ p X ] ← [ call q X ] ] ] kb! kb.saturate",
                "p/1"
            )
            .len(),
            1
        );
        assert_eq!(
            facts(
                "[ [ q a ] [ [ p L ] ← [ findall X [ q X ] L ] ] ] kb! kb.saturate",
                "p/1"
            )
            .len(),
            1
        );
    }

    /// **Criterion 21:** E56, directly, through `findall`, and through
    /// another predicate.
    #[test]
    fn e56_is_a_negative_cycle() {
        let e = |rules: &str| err(&format!("[ [ q a ] {rules} ] kb! kb.saturate"));
        assert_eq!(
            e("[ [ p X ] ← [ q X ] [ not [ p X ] ] ]"),
            "KB.SATURATE: saturate: p/1 depends negatively on itself"
        );
        assert_eq!(
            e("[ [ p L ] ← [ findall X [ p X ] L ] ]"),
            "KB.SATURATE: saturate: p/1 depends negatively on itself"
        );
        assert_eq!(
            e("[ [ p X ] ← [ q X ] [ not [ r X ] ] ] [ [ r X ] ← [ p X ] ]"),
            "KB.SATURATE: saturate: p/1 depends negatively on itself"
        );
    }

    /// E00, E01, E02 and E30, which the asking words share.
    #[test]
    fn the_shared_refusals() {
        assert_eq!(
            err("kb.saturate"),
            "Stack is too shallow for inline kb.saturate"
        );
        assert_eq!(
            err("1 kb.saturate"),
            "KB.SATURATE: #1 parameter must be a knowledge base"
        );
        assert_eq!(
            err("kb.new :preds 1 set kb.saturate"),
            "KB.SATURATE: malformed knowledge base: preds is not a MAP"
        );
        assert_eq!(
            err(&format!("{EXAMPLE_6} 5 kb.inferences kb.saturate")),
            "KB.SATURATE: inference cap of 5 reached"
        );
        // The cap spans the rounds: example 6 needs more than one's worth.
        let all = top(&format!("{EXAMPLE_6} 1000 kb.inferences kb.saturate"));
        assert!(all.as_map().is_some());
    }

    /// The internal names begin with a prefix no predicate begins with.
    #[test]
    fn internal_names_avoid_the_kb() {
        let keys = [
            "$sat/1".to_string(),
            "$sat$join/2".to_string(),
            "p/1".to_string(),
        ];
        let names = Names::new(keys.iter());
        assert_eq!(names.join(), "$sat$$join");
        assert_eq!(names.view("p/1", true), "$sat$$new:p");
    }

    #[test]
    fn check_knows_kb_saturate() {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let check =
            |src: &str| crate::check::check(&bund2_syntax::compile(src).unwrap(), &i.registry, 0);
        let r = check("kb.new kb.saturate kb.clauses");
        assert!(r.findings.is_empty() && r.abandoned == 0, "{r:?}");
        let r = check("kb.saturate");
        assert_eq!(r.findings.len(), 1, "{r:?}");
        assert_eq!(r.findings[0].word, "kb.saturate");
    }
}
