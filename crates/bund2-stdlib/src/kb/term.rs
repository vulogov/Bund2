//! Terms: the arena they live in, the atoms' interner, and conversion from a
//! Bund value and back (RFC-0013 §S1).
//!
//! **A term is an index into the arena**, never a Rust tree, so nothing here
//! recurses on a term's depth. Both conversions keep their own work stack, as
//! `render` does since F117: a value the program built can be as deep as it
//! likes, and the depth costs heap, not Rust stack (D39, criterion 3).
//!
//! **The arena lives for one activation** and is reset to a mark on
//! backtrack (§S4). A binding is written into the variable's cell; `unify.rs`
//! trails it, so the trail is unwound before the arena is cut back. The
//! interner is not reset: an atom outlives the choicepoint that first named it.
//!
//! Nothing here uses the `Vm`.

use std::collections::HashMap;

use bund2_value::{BundValue, CONTEXT, LIST, NODATA};

/// How wide a value is shown in a refusal (D36).
pub(crate) const SHOWN: usize = 48;

/// A term: one cell of the arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Term(usize);

/// An interned atom's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Atom(usize);

/// What one arena cell holds.
#[derive(Debug, Clone, Copy)]
pub enum Cell {
    /// A variable, and what it is bound to. Its age is its index: a lower
    /// index is older.
    Var(Option<Term>),
    Atom(Atom),
    Int(i64),
    /// Compared by key equality (§S1, L2): every NaN is one float, and
    /// `-0.0` is `0.0`.
    Float(f64),
    /// The empty list, which is not an atom (L4).
    Nil,
    Cons(Term, Term),
    /// A compound: its functor, and its arguments at
    /// `args[start..start + arity]`. Never of arity 0, which is the atom.
    Struct {
        functor: Atom,
        start: usize,
        arity: usize,
    },
    /// Any other value, at `opaques[i]`, compared by key equality (L13).
    Opaque(usize),
}

/// Where the arena stood, to cut it back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    cells: usize,
    args: usize,
    opaques: usize,
}

/// Atom texts, each held once.
#[derive(Debug, Default)]
pub struct Interner {
    ids: HashMap<String, Atom>,
    texts: Vec<String>,
}

impl Interner {
    pub fn intern(&mut self, text: &str) -> Atom {
        if let Some(a) = self.ids.get(text) {
            return *a;
        }
        let a = Atom(self.texts.len());
        self.texts.push(text.to_string());
        self.ids.insert(text.to_string(), a);
        a
    }

    pub fn text(&self, a: Atom) -> &str {
        self.texts.get(a.0).map_or("", String::as_str)
    }
}

/// The cells of one activation.
#[derive(Debug, Default)]
pub struct Arena {
    cells: Vec<Cell>,
    args: Vec<Term>,
    opaques: Vec<BundValue>,
    pub atoms: Interner,
}

/// Why a value is not a term. The text is what follows `<W>: ` (§S12); the
/// word that converted the value adds the prefix.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    /// E04: a variable's name holds `|`.
    BarInVariable(String),
    /// E05: a `|` that does not stand between a head and one tail.
    MisplacedBar(BundValue),
    /// E26: a CONTEXT would switch stacks if placed in a body.
    Context(BundValue),
}

impl Refusal {
    pub fn text(&self) -> String {
        match self {
            Refusal::BarInVariable(name) => {
                format!("{name} is not a variable name: it contains '|'")
            }
            Refusal::MisplacedBar(list) => format!(
                "'|' must stand between a head and a tail in {}",
                list.summary(SHOWN)
            ),
            Refusal::Context(v) => format!("a CONTEXT cannot be a term: {}", v.summary(SHOWN)),
        }
    }
}

/// The named variables of one clause or goal, in the order they first appear.
/// `_` and NODATA are fresh at each occurrence and never named here.
#[derive(Debug, Default)]
pub struct Scope {
    names: Vec<(String, Term)>,
}

impl Scope {
    pub fn names(&self) -> &[(String, Term)] {
        &self.names
    }

    fn get(&self, name: &str) -> Option<Term> {
        self.names.iter().find(|(n, _)| n == name).map(|(_, t)| *t)
    }
}

impl Arena {
    pub fn mark(&self) -> Mark {
        Mark {
            cells: self.cells.len(),
            args: self.args.len(),
            opaques: self.opaques.len(),
        }
    }

    /// Cut the arena back to `m`. The trail must have been unwound to the
    /// same point first, or an older variable is left bound to a cell that
    /// no longer exists.
    pub fn reset(&mut self, m: Mark) {
        self.cells.truncate(m.cells);
        self.args.truncate(m.args);
        self.opaques.truncate(m.opaques);
    }

    /// How many cells are live: what criterion 5 records as growth.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cell(&self, t: Term) -> Cell {
        self.cells.get(t.0).copied().unwrap_or(Cell::Nil)
    }

    fn push(&mut self, c: Cell) -> Term {
        self.cells.push(c);
        Term(self.cells.len() - 1)
    }

    pub fn var(&mut self) -> Term {
        self.push(Cell::Var(None))
    }

    pub fn atom(&mut self, text: &str) -> Term {
        let a = self.atoms.intern(text);
        self.push(Cell::Atom(a))
    }

    pub fn int(&mut self, i: i64) -> Term {
        self.push(Cell::Int(i))
    }

    pub fn float(&mut self, f: f64) -> Term {
        self.push(Cell::Float(f))
    }

    pub fn nil(&mut self) -> Term {
        self.push(Cell::Nil)
    }

    pub fn cons(&mut self, head: Term, tail: Term) -> Term {
        self.push(Cell::Cons(head, tail))
    }

    /// A compound, or the atom itself when `args` is empty (L4).
    pub fn compound(&mut self, functor: &str, args: &[Term]) -> Term {
        let functor = self.atoms.intern(functor);
        self.structure(functor, args)
    }

    fn structure(&mut self, functor: Atom, args: &[Term]) -> Term {
        if args.is_empty() {
            return self.push(Cell::Atom(functor));
        }
        let start = self.args.len();
        self.args.extend_from_slice(args);
        self.push(Cell::Struct {
            functor,
            start,
            arity: args.len(),
        })
    }

    pub fn opaque(&mut self, v: BundValue) -> Term {
        self.opaques.push(v);
        let i = self.opaques.len() - 1;
        self.push(Cell::Opaque(i))
    }

    /// A compound's arguments. Empty for anything else.
    pub fn args(&self, start: usize, arity: usize) -> &[Term] {
        self.args.get(start..start + arity).unwrap_or(&[])
    }

    pub fn opaque_value(&self, i: usize) -> Option<&BundValue> {
        self.opaques.get(i)
    }

    /// Bind an unbound variable. `unify.rs` trails it; nothing else binds.
    pub(crate) fn bind(&mut self, var: Term, to: Term) {
        if let Some(c) = self.cells.get_mut(var.0) {
            *c = Cell::Var(Some(to));
        }
    }

    pub(crate) fn unbind(&mut self, var: Term) {
        if let Some(c) = self.cells.get_mut(var.0) {
            *c = Cell::Var(None);
        }
    }

    /// Follow bindings to a term that is not a bound variable. The chain is
    /// acyclic, since the occurs check is always on (L7) and a variable is
    /// bound only while unbound, so this ends.
    pub fn deref(&self, mut t: Term) -> Term {
        while let Cell::Var(Some(next)) = self.cell(t) {
            t = next;
        }
        t
    }

    /// Read a value as a term (§S1). Named variables are found in, or added
    /// to, `scope`.
    pub fn to_term(&mut self, v: &BundValue, scope: &mut Scope) -> Result<Term, Refusal> {
        enum Job<'a> {
            Visit(&'a BundValue),
            Struct(Atom, usize),
            /// A list of `n` items, the last of them its tail when `tail`.
            List(usize, bool),
        }
        let mut jobs = vec![Job::Visit(v)];
        let mut done: Vec<Term> = Vec::new();
        while let Some(job) = jobs.pop() {
            match job {
                Job::Visit(v) => match self.read_one(v, scope)? {
                    Read::Term(t) => done.push(t),
                    Read::Struct(functor, args) => {
                        jobs.push(Job::Struct(functor, args.len()));
                        jobs.extend(args.iter().rev().map(Job::Visit));
                    }
                    Read::List(items, tail) => {
                        let n = items.len() + usize::from(tail.is_some());
                        jobs.push(Job::List(n, tail.is_some()));
                        jobs.extend(tail.map(Job::Visit));
                        jobs.extend(items.iter().rev().map(Job::Visit));
                    }
                },
                Job::Struct(functor, n) => {
                    let args = done.split_off(done.len().saturating_sub(n));
                    let t = self.structure(functor, &args);
                    done.push(t);
                }
                Job::List(n, has_tail) => {
                    let mut items = done.split_off(done.len().saturating_sub(n));
                    let popped = if has_tail { items.pop() } else { None };
                    let mut tail = popped.unwrap_or_else(|| self.nil());
                    for head in items.into_iter().rev() {
                        tail = self.cons(head, tail);
                    }
                    done.push(tail);
                }
            }
        }
        // One job in, one term out: every arm above leaves exactly one.
        Ok(done.pop().unwrap_or_else(|| self.nil()))
    }

    /// One level of §S1's table.
    fn read_one<'a>(&mut self, v: &'a BundValue, scope: &mut Scope) -> Result<Read<'a>, Refusal> {
        match v.unboxed() {
            BundValue::Int(i, _) => return Ok(Read::Term(self.int(*i))),
            BundValue::Float(f, _) => return Ok(Read::Term(self.float(*f))),
            BundValue::Bool(b, _) => {
                return Ok(Read::Term(self.atom(if *b { "true" } else { "false" })));
            }
            BundValue::Nodata(_) => return Ok(Read::Term(self.var())),
            _ => {}
        }
        let dt = v.dt();
        if dt == NODATA {
            return Ok(Read::Term(self.var()));
        }
        if dt == CONTEXT {
            return Err(Refusal::Context(v.clone()));
        }
        if dt == bund2_value::CALL {
            let name = v.str_ref().unwrap_or_default();
            return self.read_call(name, scope).map(Read::Term);
        }
        if dt == bund2_value::STRING || dt == bund2_value::PTR {
            let text = v.str_ref().unwrap_or_default();
            return Ok(Read::Term(self.atom(text)));
        }
        if dt != LIST {
            return Ok(Read::Term(self.opaque(v.clone())));
        }
        let items = v.as_list().unwrap_or(&[]);
        let Some(first) = items.first() else {
            return Ok(Read::Term(self.nil()));
        };
        if let Some(functor) = atom_call(first) {
            let functor = self.atoms.intern(functor);
            return Ok(Read::Struct(functor, items.get(1..).unwrap_or(&[])));
        }
        let bars: Vec<usize> = items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| is_bar(item).then_some(i))
            .collect();
        match bars.as_slice() {
            [] => Ok(Read::List(items, None)),
            // The one place a `|` splits: neither first nor last, with one
            // tail after it.
            [i] if *i > 0 && *i + 2 == items.len() => Ok(Read::List(
                items.get(..*i).unwrap_or(&[]),
                items.get(*i + 1),
            )),
            _ => Err(Refusal::MisplacedBar(v.clone())),
        }
    }

    fn read_call(&mut self, name: &str, scope: &mut Scope) -> Result<Term, Refusal> {
        if name == "nil" {
            return Ok(self.nil());
        }
        if !is_variable_name(name) {
            return Ok(self.atom(name));
        }
        if name.contains('|') {
            return Err(Refusal::BarInVariable(name.to_string()));
        }
        if name == "_" {
            return Ok(self.var());
        }
        if let Some(t) = scope.get(name) {
            return Ok(t);
        }
        let t = self.var();
        scope.names.push((name.to_string(), t));
        Ok(t)
    }

    /// Read a term back as a value (§S1's second table).
    pub fn to_value(&self, t: Term) -> BundValue {
        enum Job {
            Visit(Term),
            Struct(Atom, usize),
            /// `n` items, then what the tail reads as.
            List(usize, Tail),
        }
        enum Tail {
            Proper,
            Partial,
            /// Bound to a non-list, which was visited after the items.
            Improper,
        }
        let mut jobs = vec![Job::Visit(t)];
        let mut done: Vec<BundValue> = Vec::new();
        while let Some(job) = jobs.pop() {
            match job {
                Job::Visit(t) => {
                    let t = self.deref(t);
                    match self.cell(t) {
                        Cell::Var(_) => done.push(BundValue::nodata()),
                        Cell::Atom(a) => done.push(atom_value(self.atoms.text(a))),
                        Cell::Int(i) => done.push(BundValue::int(i)),
                        Cell::Float(f) => done.push(BundValue::float(f)),
                        Cell::Nil => done.push(BundValue::list(Vec::new())),
                        Cell::Opaque(i) => done.push(
                            self.opaque_value(i)
                                .cloned()
                                .unwrap_or_else(BundValue::nodata),
                        ),
                        Cell::Struct {
                            functor,
                            start,
                            arity,
                        } => {
                            jobs.push(Job::Struct(functor, arity));
                            jobs.extend(
                                self.args(start, arity).iter().rev().map(|a| Job::Visit(*a)),
                            );
                        }
                        Cell::Cons(..) => {
                            let (items, end) = self.spine(t);
                            let tail = match self.cell(end) {
                                Cell::Nil => Tail::Proper,
                                Cell::Var(_) => Tail::Partial,
                                _ => Tail::Improper,
                            };
                            let improper = matches!(tail, Tail::Improper);
                            jobs.push(Job::List(items.len(), tail));
                            if improper {
                                jobs.push(Job::Visit(end));
                            }
                            jobs.extend(items.iter().rev().map(|h| Job::Visit(*h)));
                        }
                    }
                }
                Job::Struct(functor, n) => {
                    let args = done.split_off(done.len().saturating_sub(n));
                    let mut items = Vec::with_capacity(n + 1);
                    items.push(BundValue::call(self.atoms.text(functor)));
                    items.extend(args);
                    done.push(BundValue::list(items));
                }
                Job::List(n, tail) => {
                    let extra = usize::from(matches!(tail, Tail::Improper));
                    let mut items = done.split_off(done.len().saturating_sub(n + extra));
                    match tail {
                        Tail::Proper => {}
                        Tail::Partial => {
                            items.push(BundValue::call("|"));
                            items.push(BundValue::nodata());
                        }
                        Tail::Improper => {
                            let end = items.pop().unwrap_or_else(BundValue::nodata);
                            items.push(BundValue::call("|"));
                            items.push(end);
                        }
                    }
                    done.push(BundValue::list(items));
                }
            }
        }
        done.pop().unwrap_or_else(BundValue::nodata)
    }

    /// A list's items, and the dereferenced term its spine ends in.
    pub fn spine(&self, list: Term) -> (Vec<Term>, Term) {
        let mut items = Vec::new();
        let mut at = self.deref(list);
        while let Cell::Cons(head, tail) = self.cell(at) {
            items.push(head);
            at = self.deref(tail);
        }
        (items, at)
    }
}

/// What one level of a value reads as, before its parts are read.
enum Read<'a> {
    Term(Term),
    Struct(Atom, &'a [BundValue]),
    /// A list's items, and the one tail after a splitting `|`.
    List(&'a [BundValue], Option<&'a BundValue>),
}

/// **L1:** a CALL starting uppercase or `_` is a variable.
fn is_variable_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|c| c == '_' || c.is_uppercase())
}

fn is_bar(v: &BundValue) -> bool {
    v.dt() == bund2_value::CALL && v.str_ref() == Some("|")
}

/// The functor a LIST's first item makes it a compound with: an atom CALL,
/// which is neither a variable nor `nil` (§S1).
fn atom_call(v: &BundValue) -> Option<&str> {
    if v.dt() != bund2_value::CALL {
        return None;
    }
    let name = v.str_ref()?;
    (name != "nil" && !is_variable_name(name)).then_some(name)
}

/// An atom reads back as a STRING, and `true` and `false` as BOOL (§S1).
fn atom_value(text: &str) -> BundValue {
    match text {
        "true" => BundValue::boolean(true),
        "false" => BundValue::boolean(false),
        _ => BundValue::str(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> BundValue {
        BundValue::call(s)
    }
    fn s(x: &str) -> BundValue {
        BundValue::str(x)
    }
    fn l(items: Vec<BundValue>) -> BundValue {
        BundValue::list(items)
    }

    fn read(v: &BundValue) -> (Arena, Term, Scope) {
        let mut arena = Arena::default();
        let mut scope = Scope::default();
        let t = arena.to_term(v, &mut scope).expect("a term");
        (arena, t, scope)
    }

    fn kind(arena: &Arena, t: Term) -> &'static str {
        match arena.cell(arena.deref(t)) {
            Cell::Var(_) => "var",
            Cell::Atom(_) => "atom",
            Cell::Int(_) => "int",
            Cell::Float(_) => "float",
            Cell::Nil => "nil",
            Cell::Cons(..) => "cons",
            Cell::Struct { .. } => "struct",
            Cell::Opaque(_) => "opaque",
        }
    }

    /// §S1's first table, row by row.
    #[test]
    fn each_kind_of_value_reads_as_its_term() {
        let rows: Vec<(BundValue, &str)> = vec![
            (c("X"), "var"),
            (c("_"), "var"),
            (c("_Hidden"), "var"),
            (c("nil"), "nil"),
            (c("tom"), "atom"),
            (BundValue::ptr("tom"), "atom"),
            (s("Tom"), "atom"),
            (BundValue::boolean(true), "atom"),
            (BundValue::int(3), "int"),
            (BundValue::float(2.5), "float"),
            (l(vec![c("parent"), c("tom"), c("X")]), "struct"),
            (l(vec![c("ready")]), "atom"),
            (l(vec![s("tom"), s("bob")]), "cons"),
            (l(vec![]), "nil"),
            (BundValue::nodata(), "var"),
            (BundValue::map(Default::default()), "opaque"),
            (BundValue::none(), "opaque"),
        ];
        for (v, want) in rows {
            let (arena, t, _) = read(&v);
            assert_eq!(kind(&arena, t), want, "{}", v.summary(48));
        }
    }

    /// An atom compares by text, whatever kind of value spelt it (L2), and a
    /// compound with no arguments is its atom (L4).
    #[test]
    fn atoms_are_one_by_text() {
        let mut arena = Arena::default();
        let mut scope = Scope::default();
        let spellings = [
            c("true"),
            s("true"),
            BundValue::ptr("true"),
            BundValue::boolean(true),
            l(vec![c("true")]),
        ];
        let atoms: Vec<Cell> = spellings
            .iter()
            .map(|v| {
                let t = arena.to_term(v, &mut scope).unwrap();
                arena.cell(t)
            })
            .collect();
        for a in &atoms {
            let (Cell::Atom(x), Cell::Atom(y)) = (a, &atoms[0]) else {
                panic!("not an atom: {a:?}");
            };
            assert_eq!(x, y);
        }
    }

    /// The traps §S1 lists, each read as it says.
    #[test]
    fn the_traps_read_as_section_s1_says() {
        // `[ tom bob ]` is tom(bob).
        let (arena, t, _) = read(&l(vec![c("tom"), c("bob")]));
        let Cell::Struct { functor, arity, .. } = arena.cell(t) else {
            panic!("not a compound")
        };
        assert_eq!((arena.atoms.text(functor), arity), ("tom", 1));

        // `[ tom | T ]` is tom('|', T): `|` splits only a list.
        let (arena, t, _) = read(&l(vec![c("tom"), c("|"), c("T")]));
        let Cell::Struct { start, arity, .. } = arena.cell(t) else {
            panic!("not a compound")
        };
        assert_eq!(arity, 2);
        let Cell::Atom(bar) = arena.cell(arena.args(start, arity)[0]) else {
            panic!("'|' is not an atom here")
        };
        assert_eq!(arena.atoms.text(bar), "|");

        // `[ | a ]` is '|'(a): its first item is an atom CALL.
        let (arena, t, _) = read(&l(vec![c("|"), s("a")]));
        let Cell::Struct { functor, arity, .. } = arena.cell(t) else {
            panic!("not a compound")
        };
        assert_eq!((arena.atoms.text(functor), arity), ("|", 1));

        // `[ nil ]` is a one-item list holding the empty list.
        let (arena, t, _) = read(&l(vec![c("nil")]));
        let (items, end) = arena.spine(t);
        assert_eq!(items.len(), 1);
        assert_eq!(kind(&arena, items[0]), "nil");
        assert_eq!(kind(&arena, end), "nil");

        // `[ nil X ]` is a two-item list.
        let (arena, t, _) = read(&l(vec![c("nil"), c("X")]));
        assert_eq!(arena.spine(t).0.len(), 2);

        // `[ ready [ p X ] ]` is ready(p(X)).
        let (arena, t, _) = read(&l(vec![c("ready"), l(vec![c("p"), c("X")])]));
        let Cell::Struct { functor, arity, .. } = arena.cell(t) else {
            panic!("not a compound")
        };
        assert_eq!((arena.atoms.text(functor), arity), ("ready", 1));

        // A class name begins uppercase, so it is a variable.
        let (arena, t, _) = read(&c("Integer"));
        assert_eq!(kind(&arena, t), "var");
    }

    /// A named variable is one variable wherever it recurs in a scope; `_` and
    /// NODATA are fresh at each occurrence.
    #[test]
    fn variables_are_named_once_and_anonymous_ones_are_fresh() {
        let v = l(vec![
            c("f"),
            c("X"),
            c("X"),
            c("_"),
            c("_"),
            BundValue::nodata(),
            BundValue::nodata(),
        ]);
        let (arena, t, scope) = read(&v);
        let Cell::Struct { start, arity, .. } = arena.cell(t) else {
            panic!("not a compound")
        };
        let a = arena.args(start, arity);
        assert_eq!(a[0], a[1]);
        assert_ne!(a[2], a[3]);
        assert_ne!(a[4], a[5]);
        assert_eq!(scope.names().len(), 1);
        assert_eq!(scope.names()[0].0, "X");
    }

    /// E04, E05 and E26, with their texts after `<W>: ` (§S12).
    #[test]
    fn what_is_not_a_term_is_refused() {
        let refused = |v: BundValue| {
            let mut arena = Arena::default();
            arena
                .to_term(&v, &mut Scope::default())
                .expect_err("refused")
        };
        let e = refused(c("X|Y"));
        assert_eq!(e, Refusal::BarInVariable("X|Y".into()));
        assert_eq!(e.text(), "X|Y is not a variable name: it contains '|'");

        // A `|` last, twice, or with two items after it. (One first makes
        // the LIST a compound, since `|` is an atom CALL: see the traps.)
        for v in [
            l(vec![s("a"), c("|")]),
            l(vec![s("a"), c("|"), s("b"), c("|"), s("c")]),
            l(vec![s("a"), c("|"), s("b"), s("c")]),
        ] {
            let e = refused(v);
            assert!(matches!(e, Refusal::MisplacedBar(_)), "{e:?}");
            assert!(
                e.text()
                    .starts_with("'|' must stand between a head and a tail in "),
                "{}",
                e.text()
            );
        }

        let e = refused(l(vec![s("a"), BundValue::context()]));
        assert!(matches!(e, Refusal::Context(_)), "{e:?}");
        assert!(
            e.text().starts_with("a CONTEXT cannot be a term: "),
            "{}",
            e.text()
        );
    }

    /// §S1's second table.
    #[test]
    fn terms_read_back_as_section_s1_says() {
        let back = |v: BundValue| {
            let (arena, t, _) = read(&v);
            arena.to_value(t)
        };
        let v = back(c("tom"));
        assert_eq!((v.dt(), v.str_ref()), (bund2_value::STRING, Some("tom")));
        assert_eq!(back(s("true")), BundValue::boolean(true));
        assert_eq!(back(c("false")), BundValue::boolean(false));
        assert_eq!(back(BundValue::int(7)), BundValue::int(7));
        assert_eq!(back(BundValue::float(0.5)), BundValue::float(0.5));
        assert_eq!(back(c("X")).dt(), NODATA);
        assert_eq!(back(c("nil")).as_list().map(<[_]>::len), Some(0));

        // A compound: a LIST with its functor as a CALL.
        let v = back(l(vec![c("parent"), c("tom"), c("X")]));
        let items = v.as_list().unwrap();
        assert_eq!(items[0].dt(), bund2_value::CALL);
        assert_eq!(items[0].str_ref(), Some("parent"));
        assert_eq!(items[1].str_ref(), Some("tom"));
        assert_eq!(items[2].dt(), NODATA);

        // A partial list: `|` then NODATA.
        let v = back(l(vec![s("a"), c("|"), c("T")]));
        let items = v.as_list().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[1].str_ref(), Some("|"));
        assert_eq!(items[1].dt(), bund2_value::CALL);
        assert_eq!(items[2].dt(), NODATA);

        // An improper list: `|` then the tail's value.
        let v = back(l(vec![s("a"), c("|"), s("b")]));
        let items = v.as_list().unwrap();
        assert_eq!(items[1].str_ref(), Some("|"));
        assert_eq!(items[2].str_ref(), Some("b"));

        // A list whose tail is a list reads back flat.
        let v = back(l(vec![s("a"), c("|"), l(vec![s("b"), s("c")])]));
        assert_eq!(v.as_list().map(<[_]>::len), Some(3));

        // An opaque value is the same value: identity, for a MAP (L13).
        let m = BundValue::map(Default::default());
        assert_eq!(back(m.clone()), m);
        assert_ne!(back(m), BundValue::map(Default::default()));
    }

    /// Backtracking cuts the arena back to a mark.
    #[test]
    fn the_arena_resets_to_a_mark() {
        let mut arena = Arena::default();
        arena.atom("kept");
        let m = arena.mark();
        let mut scope = Scope::default();
        arena
            .to_term(
                &l(vec![c("f"), c("X"), BundValue::map(Default::default())]),
                &mut scope,
            )
            .unwrap();
        assert!(arena.len() > 1);
        arena.reset(m);
        assert_eq!(arena.len(), 1);
        assert_eq!(arena.mark(), m);
    }

    /// **Criterion 3, conversion.** A term 100,000 deep, built in Rust since
    /// the parser stops at 1,024 (F116), reads as a term and back in a debug
    /// build: neither walk recurses on the depth.
    #[test]
    fn a_term_100000_deep_converts_both_ways() {
        const DEPTH: usize = 100_000;
        // f(f(…f(X)…)), and [[…[a]…]].
        let mut compound = c("X");
        let mut list = s("a");
        for _ in 0..DEPTH {
            compound = l(vec![c("f"), compound]);
            list = l(vec![list]);
        }
        for v in [compound, list] {
            let (arena, t, _) = read(&v);
            let back = arena.to_value(t);
            let mut depth = 0;
            let mut at = &back;
            while let Some(items) = at.as_list() {
                depth += 1;
                at = items.last().unwrap();
            }
            assert_eq!(depth, DEPTH);
        }
    }
}
