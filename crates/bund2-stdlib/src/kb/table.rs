//! Tabling: the tables, their variants, and the completion stack (RFC-0013
//! §S18, M10).
//!
//! **Linear tabling with SCC completion.** `solve.rs` owns the search and
//! its `Leader` and `Consumer` choicepoints; what is here is the activation
//! store's part, which no backtrack undoes:
//!
//! - **the tables**, one per variant of a tabled call, each holding its
//!   answers (compared as variants, with attributes) and its status,
//!   *evaluating* or *complete*. A variant with no table is *new*;
//! - **the completion stack**: an entry is pushed when a variant first
//!   leads and popped only when its SCC completes. Each entry records its
//!   SCC root, the oldest entry of its SCC; the choicepoint height of its
//!   `Leader`; and the innermost `not`, running `if` or `findall` it was
//!   called under, for E54.
//!
//! **The join rule.** A call of an evaluating variant V merges every entry
//! from **V's SCC root** to the top into that root's SCC ([`Tables::merge`]).
//! Starting from V's own entry instead would leave the entries between the
//! root and V in an SCC of their own, completed before the tables they read.
//!
//! **A variant** is the term with its variables numbered in order of first
//! occurrence, and the suspended `dif`s on them (§S17) written after it, so
//! two calls that differ only in the names of their variables share a
//! table, and two that differ in a constraint do not.
//!
//! Nothing here uses the `Vm`.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use bund2_value::BundValue;

use super::term::{Arena, Cell, Term};

/// A table's status. A variant with no table yet is *new*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Evaluating,
    Complete,
}

/// The innermost `not`, running `if` or `findall` a call was made under:
/// its choicepoint's height, and the construct as E54 names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Neg {
    pub height: usize,
    pub construct: &'static str,
}

/// An answer, or a variant's call: a term of the template store and the
/// suspended `dif`s on its variables, sharing them.
#[derive(Debug, Clone)]
pub struct Instance {
    pub term: Term,
    pub difs: Vec<(Term, Term)>,
}

struct Table {
    /// `functor/arity`, as E54 names it.
    name: String,
    /// The variant's call, for a later pass to run again.
    call: Instance,
    /// The predicate's clauses, when `preds` has any.
    pred: Option<usize>,
    answers: Vec<Instance>,
    seen: HashSet<String>,
    status: Status,
    /// Its completion-stack entry, while it is evaluating.
    entry: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    table: usize,
    root: usize,
    leader: usize,
    neg: Option<Neg>,
}

/// E54: a merge reached below a `not`, `if` or `findall`.
#[derive(Debug, PartialEq, Eq)]
pub struct Unstratified {
    pub construct: &'static str,
    pub name: String,
}

/// The tables of one activation, and the completion stack.
#[derive(Default)]
pub struct Tables {
    tables: Vec<Table>,
    by_variant: HashMap<String, usize>,
    stack: Vec<Entry>,
    /// Answers added in the activation: a pass that leaves it unchanged
    /// added nothing.
    pub added: u64,
    /// The opaque constants met, by key equality: a variant names one by
    /// its place here.
    opaques: Vec<BundValue>,
}

impl Tables {
    /// The table of a variant, if it has one.
    pub fn find(&self, variant: &str) -> Option<usize> {
        self.by_variant.get(variant).copied()
    }

    pub fn status(&self, t: usize) -> Option<Status> {
        self.tables.get(t).map(|t| t.status)
    }

    pub fn pred(&self, t: usize) -> Option<usize> {
        self.tables.get(t).and_then(|t| t.pred)
    }

    pub fn call(&self, t: usize) -> Option<&Instance> {
        self.tables.get(t).map(|t| &t.call)
    }

    pub fn answer(&self, t: usize, i: usize) -> Option<&Instance> {
        self.tables.get(t).and_then(|t| t.answers.get(i))
    }

    pub fn len(&self, t: usize) -> usize {
        self.tables.get(t).map_or(0, |t| t.answers.len())
    }

    /// A new variant leads (step 1): its table is made, *evaluating*, and
    /// its entry pushed, its own SCC's root. `leader` is the height its
    /// `Leader` choicepoint is pushed at.
    pub fn lead(
        &mut self,
        variant: String,
        name: String,
        call: Instance,
        pred: Option<usize>,
        leader: usize,
        neg: Option<Neg>,
    ) -> usize {
        let t = self.tables.len();
        let entry = self.stack.len();
        self.tables.push(Table {
            name,
            call,
            pred,
            answers: Vec::new(),
            seen: HashSet::new(),
            status: Status::Evaluating,
            entry,
        });
        self.by_variant.insert(variant, t);
        self.stack.push(Entry {
            table: t,
            root: entry,
            leader,
            neg,
        });
        t
    }

    /// A call of evaluating `v`, made under `neg` (step 2): every entry
    /// from V's SCC root to the top joins that root's SCC. E54 when the
    /// root's `Leader` lies below the consumer's `neg`, or below the `neg`
    /// any joining entry was called under.
    pub fn merge(&mut self, v: usize, neg: Option<Neg>) -> Result<(), Unstratified> {
        let Some(entry) = self.tables.get(v).map(|t| t.entry) else {
            return Ok(());
        };
        let Some(root) = self.stack.get(entry).map(|e| e.root) else {
            return Ok(());
        };
        let Some(leader) = self.stack.get(root).map(|e| e.leader) else {
            return Ok(());
        };
        let below = |n: Option<Neg>| n.filter(|n| n.height > leader);
        if let Some(n) = below(neg) {
            return Err(self.unstratified(n, v));
        }
        for e in self.stack.get(root..).unwrap_or(&[]) {
            if let Some(n) = below(e.neg) {
                return Err(self.unstratified(n, e.table));
            }
        }
        for e in self.stack.get_mut(root..).unwrap_or(&mut []) {
            e.root = root;
        }
        Ok(())
    }

    fn unstratified(&self, n: Neg, t: usize) -> Unstratified {
        Unstratified {
            construct: n.construct,
            name: self
                .tables
                .get(t)
                .map_or_else(String::new, |t| t.name.clone()),
        }
    }

    /// Table `t`'s completion-stack entry, if it is evaluating and that
    /// entry is its SCC's root.
    pub fn root_entry(&self, t: usize) -> Option<usize> {
        let table = self.tables.get(t)?;
        let e = self.stack.get(table.entry)?;
        (table.status == Status::Evaluating && e.root == table.entry).then_some(table.entry)
    }

    /// How many entries the completion stack holds.
    pub fn height(&self) -> usize {
        self.stack.len()
    }

    /// The table of the entry at `i`.
    pub fn table_at(&self, i: usize) -> Option<usize> {
        self.stack.get(i).map(|e| e.table)
    }

    /// Mark every member of the SCC rooted at `root` complete, and pop
    /// them (step 3).
    pub fn complete(&mut self, root: usize) {
        for e in self.stack.drain(root..) {
            if let Some(t) = self.tables.get_mut(e.table) {
                t.status = Status::Complete;
            }
        }
    }

    /// Whether table `t` holds an answer whose variant is `variant`.
    pub fn has(&self, t: usize, variant: &str) -> bool {
        self.tables.get(t).is_some_and(|t| t.seen.contains(variant))
    }

    /// Add `answer`, whose variant is `variant`, to table `t`, unless a
    /// variant of it is there already (`$answer`).
    pub fn add(&mut self, t: usize, variant: String, answer: Instance) {
        if let Some(table) = self.tables.get_mut(t)
            && table.seen.insert(variant)
        {
            table.answers.push(answer);
            self.added += 1;
        }
    }

    /// A term's variant, and the `dif`s on its variables after it, each
    /// pair of sides numbered on from the term's own variables.
    pub fn variant(&mut self, arena: &Arena, t: Term, difs: &[(Term, Term)]) -> String {
        let mut out = String::new();
        let mut vars = HashMap::new();
        self.write(arena, t, &mut vars, &mut out);
        for (a, b) in difs {
            out.push_str("|dif ");
            self.write(arena, *a, &mut vars, &mut out);
            self.write(arena, *b, &mut vars, &mut out);
        }
        out
    }

    /// A term written out in prefix order, each item tagged by its kind, so
    /// two terms write alike exactly when they are variants. A worklist, as
    /// every walk here is.
    fn write(&mut self, arena: &Arena, t: Term, vars: &mut HashMap<Term, usize>, out: &mut String) {
        let mut todo = vec![t];
        while let Some(t) = todo.pop() {
            let t = arena.deref(t);
            match arena.cell(t) {
                Cell::Var(_) => {
                    let n = vars.len();
                    let n = *vars.entry(t).or_insert(n);
                    let _ = write!(out, "V{n} ");
                }
                Cell::Atom(a) => {
                    let text = arena.atoms.text(a);
                    let _ = write!(out, "a{}:{text} ", text.len());
                }
                Cell::Int(i) => {
                    let _ = write!(out, "i{i} ");
                }
                // Key equality: `-0.0` is `0.0`, and every NaN is one.
                Cell::Float(x) => {
                    let x = if x == 0.0 {
                        0.0
                    } else if x.is_nan() {
                        f64::NAN
                    } else {
                        x
                    };
                    let _ = write!(out, "f{} ", x.to_bits());
                }
                Cell::Nil => out.push_str("n "),
                Cell::Cons(head, tail) => {
                    out.push_str("c ");
                    todo.extend([tail, head]);
                }
                Cell::Struct {
                    functor,
                    start,
                    arity,
                } => {
                    let text = arena.atoms.text(functor);
                    let _ = write!(out, "s{}:{text}/{arity} ", text.len());
                    todo.extend(arena.args(start, arity).iter().rev());
                }
                Cell::Opaque(i) => {
                    let v = arena
                        .opaque_value(i)
                        .cloned()
                        .unwrap_or_else(BundValue::nodata);
                    let n = match self.opaques.iter().position(|o| *o == v) {
                        Some(n) => n,
                        None => {
                            self.opaques.push(v);
                            self.opaques.len() - 1
                        }
                    };
                    let _ = write!(out, "o{n} ");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::term::Scope;

    /// **Criterion 20, the join rule itself.** The completion stack driven
    /// through the second KB's call sequence: leaders `path(a, _)`,
    /// `path(b, _)`, `path(c, _)` and `path(e, _)`; `path(c, _)` consumes
    /// `path(a, _)` and `path(e, _)` consumes `path(b, _)`. The four end
    /// under the root `path(a, _)`: the second merge reached from
    /// `path(b, _)`'s SCC root, not from its own entry. `path(g, _)` and
    /// `path(h, _)` complete as SCCs of their own and are popped first.
    #[test]
    fn the_merge_reaches_from_the_scc_root() {
        let mut arena = Arena::default();
        let mut tables = Tables::default();
        let variant = |arena: &mut Arena, tables: &mut Tables, node: &str| {
            let mut scope = Scope::default();
            let goal = BundValue::list(vec![
                BundValue::call("path"),
                BundValue::str(node),
                BundValue::call("Y"),
            ]);
            let t = arena.to_template(&goal, &mut scope).unwrap();
            let v = tables.variant(arena, t, &[]);
            (
                v,
                Instance {
                    term: t,
                    difs: Vec::new(),
                },
            )
        };
        let mut height = 0;
        let mut lead = |arena: &mut Arena, tables: &mut Tables, node: &str| {
            let (v, call) = variant(arena, tables, node);
            assert_eq!(tables.find(&v), None, "path({node}, _) is new");
            height += 1;
            tables.lead(v, "path/2".into(), call, None, height, None)
        };
        let a = lead(&mut arena, &mut tables, "a");
        let b = lead(&mut arena, &mut tables, "b");
        let c = lead(&mut arena, &mut tables, "c");
        // `path(c, _)` consumes `path(a, _)`: a, b and c are one SCC.
        tables.merge(a, None).unwrap();
        assert_eq!(tables.root_entry(c), None, "path(c, _) fails back");
        let e = lead(&mut arena, &mut tables, "e");
        // `path(e, _)` consumes `path(b, _)`, whose SCC root is a's entry.
        tables.merge(b, None).unwrap();
        assert_eq!(tables.root_entry(e), None, "path(e, _) fails back");
        // Then, through `path(a, _)`'s clause for `edge a g`, g and h.
        let g = lead(&mut arena, &mut tables, "g");
        let h = lead(&mut arena, &mut tables, "h");
        for t in [h, g] {
            let root = tables.root_entry(t).unwrap();
            tables.complete(root);
            assert_eq!(tables.status(t), Some(Status::Complete));
        }
        assert_eq!(tables.height(), 4);
        for (i, t) in [a, b, c, e].into_iter().enumerate() {
            assert_eq!(tables.table_at(i), Some(t));
            assert_eq!(tables.stack[i].root, 0, "entry {i} is under path(a, _)");
        }
        assert_eq!(tables.root_entry(a), Some(0));
        tables.complete(0);
        assert_eq!(tables.height(), 0);
        for t in [a, b, c, e] {
            assert_eq!(tables.status(t), Some(Status::Complete));
        }
    }

    /// Calls that differ only in their variables' names are one variant;
    /// a repeated variable, or a constraint, makes another.
    #[test]
    fn variants_number_variables_by_first_occurrence() {
        let mut arena = Arena::default();
        let mut tables = Tables::default();
        let mut key = |src: Vec<BundValue>| {
            let t = arena
                .to_term(&BundValue::list(src), &mut Scope::default())
                .unwrap();
            tables.variant(&arena, t, &[])
        };
        let c = BundValue::call;
        let x = key(vec![c("p"), c("X"), c("Y")]);
        assert_eq!(x, key(vec![c("p"), c("A"), c("B")]));
        assert_ne!(x, key(vec![c("p"), c("A"), c("A")]));
        assert_ne!(x, key(vec![c("p"), c("A"), BundValue::str("b")]));
        assert_eq!(
            key(vec![c("p"), BundValue::float(-0.0)]),
            key(vec![c("p"), BundValue::float(0.0)])
        );
        assert_ne!(
            key(vec![c("p"), BundValue::int(1)]),
            key(vec![c("p"), BundValue::float(1.0)])
        );
    }

    /// E54 when the root's `Leader` lies below a consumer's construct, or
    /// below the construct a joining entry was called under.
    #[test]
    fn a_merge_below_a_construct_is_unstratified() {
        let mut arena = Arena::default();
        let mut tables = Tables::default();
        let t = arena.atom("p");
        let call = || Instance {
            term: t,
            difs: Vec::new(),
        };
        let not = |h| {
            Some(Neg {
                height: h,
                construct: "not",
            })
        };
        let p = tables.lead("p".into(), "p/0".into(), call(), None, 0, None);
        assert_eq!(
            tables.merge(p, not(1)),
            Err(Unstratified {
                construct: "not",
                name: "p/0".into()
            })
        );
        // q led under the `not` at height 1; it joins p's SCC later.
        tables.lead("q".into(), "q/0".into(), call(), None, 2, not(1));
        assert_eq!(
            tables.merge(p, None),
            Err(Unstratified {
                construct: "not",
                name: "q/0".into()
            })
        );
        // A construct below the root's `Leader` holds the whole SCC.
        let mut tables = Tables::default();
        let p = tables.lead("p".into(), "p/0".into(), call(), None, 3, not(1));
        tables.lead("q".into(), "q/0".into(), call(), None, 4, not(2));
        assert_eq!(tables.merge(p, not(2)), Ok(()));
    }

    // **Criterion 20**, through the words.

    use bund2_api::Vm as _;
    use bund2_interp::Interp;

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

    /// `var`'s value in each answer of `goal` over `kb`, in answer order.
    fn values(kb: &str, goal: &str, var: &str) -> Vec<String> {
        let list = top(&format!("{kb} {goal} kb.query"));
        list.as_list()
            .unwrap_or_else(|| panic!("{goal}: not a LIST"))
            .iter()
            .map(|a| {
                a.as_map()
                    .and_then(|m| m.get(var))
                    .map_or_else(|| "?".into(), BundValue::display)
            })
            .collect()
    }

    fn count(kb: &str, goal: &str) -> usize {
        top(&format!("{kb} {goal} kb.query"))
            .as_list()
            .map_or(0, <[BundValue]>::len)
    }

    fn atoms(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|x| BundValue::str(*x).display()).collect()
    }

    /// `path/2` tabled, right-recursive, as criterion 20's KBs give it.
    fn right(edges: &str) -> String {
        format!(
            "[ {edges} [ [ path X Y ] ← [ edge X Y ] ] \
             [ [ path X Y ] ← [ edge X Z ] [ path Z Y ] ] ] kb! :path 2 kb.table"
        )
    }

    const EXAMPLE_6: &str = "[ [ edge a b ] [ edge b c ] [ edge c a ] \
        [ [ path X Y ] ← [ path X Z ] [ edge Z Y ] ] \
        [ [ path X Y ] ← [ edge X Y ] ] ] kb! :path 2 kb.table";

    /// Worked example 6: left recursion over a cycle terminates, and the
    /// answers come in table order.
    #[test]
    fn example_6_answers_in_table_order() {
        assert_eq!(
            values(EXAMPLE_6, "[ path a Y ]", "Y"),
            atoms(&["b", "c", "a"])
        );
        // Every pair of the cycle, each once.
        assert_eq!(count(EXAMPLE_6, "[ path X Y ]"), 9);
        // A ground call holds once.
        assert_eq!(count(EXAMPLE_6, "[ path a a ]"), 1);
        assert_eq!(count(EXAMPLE_6, "[ path a d ]"), 0);
    }

    /// **Criterion 20.** A left-recursive `path` over a 1,000-node cycle
    /// terminates, and its answers equal `graph.reachable`'s nodes.
    #[test]
    fn a_thousand_node_cycle_equals_graph_reachable() {
        const N: usize = 1000;
        // Names Title case leaves as they are (`graph.rs` Title-cases every
        // name, and splits at a digit): `N` and three lowercase letters.
        let name = |i: usize| {
            let l = |k: usize| char::from(b'a' + u8::try_from(k % 26).unwrap_or(0));
            format!("N{}{}{}", l(i / 676), l(i / 26), l(i))
        };
        let mut facts = String::new();
        let mut edges = String::new();
        let mut nodes = String::new();
        for i in 0..N {
            let (a, b) = (name(i), name((i + 1) % N));
            facts.push_str(&format!("[ edge \"{a}\" \"{b}\" ] "));
            edges.push_str(&format!("[ :{a} :{b} 1.0 ] "));
            nodes.push_str(&format!(":{a} "));
        }
        let kb = format!(
            "[ {facts} [ [ path X Y ] ← [ path X Z ] [ edge Z Y ] ] \
             [ [ path X Y ] ← [ edge X Y ] ] ] kb! :path 2 kb.table"
        );
        let start = name(0);
        let mut tabled = values(&kb, &format!("[ path \"{start}\" Y ]"), "Y");
        let reach = top(&format!(
            ":{start} [ {edges} ] [ {nodes} ] graph! graph.reachable"
        ));
        let mut reach: Vec<String> = reach
            .as_list()
            .unwrap()
            .iter()
            .map(|v| BundValue::str(v.as_str().unwrap()).display())
            .collect();
        assert_eq!(tabled.len(), N);
        tabled.sort();
        reach.sort();
        assert_eq!(tabled, reach);
    }

    /// **Criterion 20, a member completed before its SCC.** `path(b, _)` is
    /// first called under `path(a, _)` and consumes it while it holds only
    /// `b` and `c`; completed on its own, its table would lack `d`.
    #[test]
    fn a_member_is_completed_with_its_scc() {
        let kb = right("[ edge a b ] [ edge b a ] [ edge a c ] [ edge c d ]");
        let ys = values(&kb, "[ [ path a X ] [ path b Y ] ]", "Y");
        assert!(ys.contains(&atoms(&["d"])[0]), "{ys:?}");
        let mut b = values(&kb, "[ path b Y ]", "Y");
        b.sort();
        assert_eq!(b, atoms(&["a", "b", "c", "d"]));
    }

    /// **Criterion 20, a variant that joins the SCC late.** `path(e, _)`
    /// consumes `path(b, _)`, which is in `path(a, _)`'s SCC but is not its
    /// root; `h` enters `path(a, _)`'s table only after `path(e, _)` has
    /// failed back, through `path(g, _)`.
    #[test]
    fn a_late_variant_merges_from_the_root() {
        let kb = right(
            "[ edge a b ] [ edge b c ] [ edge c a ] [ edge c e ] [ edge e b ] \
             [ edge a g ] [ edge g h ]",
        );
        let ys = values(&kb, "[ [ path a X ] [ path e Y ] ]", "Y");
        assert!(ys.contains(&atoms(&["h"])[0]), "{ys:?}");
        let mut e = values(&kb, "[ path e Y ]", "Y");
        e.sort();
        assert_eq!(e, atoms(&["a", "b", "c", "e", "g", "h"]));
    }

    /// **Criterion 20.** `once` never cuts a table short: the table is
    /// complete before its first answer is returned.
    #[test]
    fn once_does_not_cut_a_table_short() {
        let n = count(EXAMPLE_6, "[ path a Z ]");
        assert_eq!(n, 3);
        assert_eq!(
            count(EXAMPLE_6, "[ [ once [ path a Y ] ] [ path a Z ] ]"),
            n
        );
        let kb = right("[ edge a b ] [ edge b a ] [ edge a c ] [ edge c d ]");
        let n = count(&kb, "[ path a Z ]");
        assert_eq!(count(&kb, "[ [ once [ path a Y ] ] [ path a Z ] ]"), n);
        assert_eq!(count(&kb, "[ [ not [ path a z ] ] [ path a Z ] ]"), n);
    }

    /// **Criterion 20, E54.** Across a `Not`, a running `Else` and a
    /// `Collect`.
    #[test]
    fn e54_is_a_merge_across_negation_or_aggregation() {
        const TABLED: &str = ":p 0 kb.table :q 0 kb.table";
        let q = "[ [ q ] ← [ p ] ]";
        for (p, construct) in [
            ("[ [ p ] ← [ not [ q ] ] ]", "not"),
            ("[ [ p ] ← [ if [ [ q ] ] [ true ] [ fail ] ] ]", "if"),
            ("[ [ p ] ← [ findall X [ [ q ] ] L ] ]", "findall"),
        ] {
            assert_eq!(
                err(&format!("[ {p} {q} ] kb! {TABLED} [ p ] kb.query")),
                format!(
                    "KB.QUERY: {construct} over the incomplete table of p/0: \
                     the program is not stratified"
                ),
                "{p}"
            );
        }
        // Over a complete table, each is fine.
        let kb =
            format!("[ [ q ] [ [ p ] ← [ not [ q ] ] ] [ [ r ] ← [ q ] [ p ] ] ] kb! {TABLED}");
        assert_eq!(count(&kb, "[ p ]"), 0);
        assert_eq!(count(&kb, "[ r ]"), 0);
    }

    /// **Criterion 6c.** A predicate named only in `tabled` is defined: it
    /// has no answers, and is not E12.
    #[test]
    fn a_predicate_only_tabled_is_defined() {
        assert_eq!(count("kb.new :p 1 kb.table", "[ p X ]"), 0);
        assert!(
            err("kb.new [ p X ] kb.query").contains("unknown predicate p/1"),
            "untabled, it is E12"
        );
    }

    /// A table answer is proved by `[ goal ← :tabled ]` (§S15).
    #[test]
    fn a_table_answer_is_proved_as_tabled() {
        let list = top(&format!("{EXAMPLE_6} [ path a b ] kb.prove"));
        let a = list.as_list().unwrap();
        assert_eq!(a.len(), 1);
        let proof = a[0].as_map().unwrap()["proof"].as_list().unwrap().to_vec();
        assert_eq!(proof.len(), 3, "[ goal ← :tabled ]");
        assert_eq!(proof[2].display(), BundValue::str("tabled").display());
    }

    /// A variant carries its attributes: a call under a `dif` is a table of
    /// its own, and one without it reads the whole.
    #[test]
    fn a_variant_carries_its_dif() {
        assert_eq!(
            values(EXAMPLE_6, "[ [ dif Y b ] [ path a Y ] ]", "Y"),
            atoms(&["c", "a"])
        );
        assert_eq!(
            values(
                EXAMPLE_6,
                "[ [ dif Y b ] [ path a Y ] [ path a Z ] [ = Y Z ] ]",
                "Z"
            ),
            atoms(&["c", "a"])
        );
    }
}
