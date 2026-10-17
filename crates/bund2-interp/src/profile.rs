//! **The record a profiled run keeps — RFC-0011 §S2 and §S4.**
//!
//! One node per word as the program names it — a registered native, a
//! method native, a conditional runner, or a word the program registered in
//! Bund — and one edge per caller-callee pair. The interpreter opens and
//! closes nodes through three hooks and keeps this record's mirror stack in
//! step with its frames by depth (§S2); nothing here formats, and the two
//! renderers live in the CLI.
//!
//! **The arithmetic (§S2).** Every activation of a node records one span. A
//! node's inclusive time is the sum of the spans of its activations that are
//! not nested inside another activation of the same node; its exclusive time
//! is the sum over all activations of the span less the spans of its direct
//! children. Exclusive times therefore sum to the root's span. An edge
//! carries a call count and the sum of the callee's spans along it; a self
//! edge carries the count alone.
//!
//! **A body no word names has no node** (RFC-0008 §D2): it is an anonymous
//! entry on the mirror, so what it calls is charged to the nearest named
//! node above it. The one exception is the root, node 0, the program itself.

use std::collections::{HashMap, HashSet};

/// "No node yet" in the symbol table.
const NONE: u32 = u32::MAX;
use std::time::{Duration, Instant};

use bund2_api::{Symbol, WordKind};

/// What kind of word a node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Node 0: the program's top-level body, which no word names.
    Program,
    /// A native registered under a name, called through `Interp::invoke`.
    Native,
    /// A method native, called through `Vm::call_method`.
    Method,
    /// A conditional runner, called through `Vm::run_conditional`.
    Conditional,
    /// A word a program or a loaded file registered in Bund.
    Word,
}

/// How a node is identified: by the symbol dispatch resolved, or by the
/// string name a method or conditional is registered under.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Sym(Symbol, Kind),
    Name(String, Kind),
}

/// One word's figures.
#[derive(Debug, Clone)]
pub struct Node {
    pub key: Key,
    pub kind: Kind,
    /// Declared at registration for a native; `None` for the other kinds.
    pub word_kind: Option<WordKind>,
    /// Other spellings the program used for this word — an alias, `!` for
    /// `execute` — so a report can say both names (§S1).
    pub spelled: Vec<Symbol>,
    pub calls: u64,
    pub inclusive: Duration,
    pub exclusive: Duration,
    pub min: Duration,
    pub max: Duration,
    /// The deepest nesting of this node inside itself: 1 for a word that
    /// never recursed.
    pub max_depth: u32,
    active: u32,
    /// The edges out of this node: `(callee, via, edge)`. A short list
    /// scanned linearly, which is cheaper than hashing a three-tuple for the
    /// handful of callees a word has.
    out: Vec<(usize, Option<usize>, Edge)>,
}

impl Node {
    fn new(key: Key, kind: Kind) -> Self {
        Self {
            key,
            kind,
            word_kind: None,
            spelled: Vec::new(),
            calls: 0,
            inclusive: Duration::ZERO,
            exclusive: Duration::ZERO,
            min: Duration::MAX,
            max: Duration::ZERO,
            max_depth: 0,
            active: 0,
            out: Vec::new(),
        }
    }
}

/// One caller-callee pair, by the road it took.
///
/// The key an edge is kept under is `(caller, callee, via)`: `via` is the
/// native that filed the body the call was made from as a tail request —
/// `!` for `{ … } !`, `if` for `{ … } if` — so the callee shows as the
/// caller's child, marked, rather than as the native's, whose span closed
/// before the body ran (§S1). `None` when the call was made from the
/// caller's own body.
#[derive(Debug, Clone, Copy, Default)]
pub struct Edge {
    pub calls: u64,
    /// The callee's spans along this edge; zero on a self edge.
    pub time: Duration,
}

/// An open activation on the mirror stack.
struct Open {
    /// `None` for a body no word names.
    node: Option<usize>,
    /// The interpreter's frame depth this entry belongs to: a word's frame
    /// sits at this depth, a leaf was opened while the frames were this deep.
    depth: usize,
    start: Instant,
    /// The spans of the direct children already closed under this entry.
    children: Duration,
    /// Another activation of the same node was open when this one started.
    nested: bool,
    /// A native, closed by its caller rather than by a frame pop.
    leaf: bool,
    /// For an anonymous body: the native that filed it as a tail request.
    via: Option<usize>,
    /// For a named node: the caller and the index into its `out` list of the
    /// edge this activation was entered along, so the span is added to the
    /// same edge on the way out.
    edge: Option<(usize, usize)>,
}

/// The record.
pub struct Profile {
    nodes: Vec<Node>,
    /// Node ids by symbol index, one slot per kind a symbol can be — a
    /// native and a word — so the hot path indexes rather than hashes.
    by_sym: Vec<[u32; 2]>,
    /// Node ids by string name, for methods and conditional runners.
    by_name: HashMap<(String, Kind), usize>,
    open: Vec<Open>,
    started: Instant,
    /// Set by [`Profile::finish`]: the run's wall time from the record's
    /// creation.
    pub wall: Option<Duration>,
    /// The spans of the top-level nodes, which the root's exclusive time is
    /// the wall less.
    root_children: Duration,
    /// How many times the clock was read.
    pub readings: u64,
    /// What one reading costs on this host, measured by whoever made the
    /// record and printed beside every table.
    pub reading_cost: Duration,
    /// A debugger console was attached during the run, so the tier was off
    /// for two reasons and a stop's wall time lies in whichever node was open.
    pub console_attached: bool,
    /// Names registered more than once while the record was kept.
    pub redefined: HashSet<String>,
    /// The native that filed the tail request whose body is about to be
    /// pushed, if one did.
    filed_by: Option<usize>,
}

impl Profile {
    pub fn new(reading_cost: Duration) -> Self {
        let root = Node::new(Key::Name("<program>".into(), Kind::Program), Kind::Program);
        Self {
            nodes: vec![root],
            by_sym: Vec::new(),
            by_name: HashMap::new(),
            open: Vec::new(),
            started: Instant::now(),
            wall: None,
            root_children: Duration::ZERO,
            readings: 1,
            reading_cost,
            console_attached: false,
            redefined: HashSet::new(),
            filed_by: None,
        }
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Every edge as `(caller, callee, via, edge)`.
    pub fn edges(&self) -> impl Iterator<Item = (usize, usize, Option<usize>, &Edge)> {
        self.nodes
            .iter()
            .enumerate()
            .flat_map(|(a, n)| n.out.iter().map(move |(b, v, e)| (a, *b, *v, e)))
    }

    /// The node for a symbol of one kind, made on first sight.
    fn sym_node(&mut self, sym: Symbol, kind: Kind) -> usize {
        let slot = usize::from(kind == Kind::Word);
        let i = sym.index();
        if i >= self.by_sym.len() {
            self.by_sym.resize(i + 1, [NONE; 2]);
        }
        let id = self.by_sym[i][slot];
        if id != NONE {
            return id as usize;
        }
        let id = self.nodes.len();
        self.nodes.push(Node::new(Key::Sym(sym, kind), kind));
        self.by_sym[i][slot] = id as u32;
        id
    }

    /// The node for a string-named native of one kind, made on first sight.
    fn name_node(&mut self, name: &str, kind: Kind) -> usize {
        if let Some(&id) = self.by_name.get(&(name.to_string(), kind)) {
            return id;
        }
        let id = self.nodes.len();
        self.nodes
            .push(Node::new(Key::Name(name.to_string(), kind), kind));
        self.by_name.insert((name.to_string(), kind), id);
        id
    }

    /// The index in `caller`'s `out` list of the edge to `callee` via `via`,
    /// made on first sight.
    fn edge_index(&mut self, caller: usize, callee: usize, via: Option<usize>) -> usize {
        let out = &mut self.nodes[caller].out;
        if let Some(i) = out.iter().position(|(b, v, _)| *b == callee && *v == via) {
            return i;
        }
        out.push((callee, via, Edge::default()));
        out.len() - 1
    }

    /// The nearest named node open above the top, and the native that filed
    /// the innermost anonymous body between them, if one did.
    fn parent(&self) -> (usize, Option<usize>) {
        let mut via = None;
        for o in self.open.iter().rev() {
            if let Some(id) = o.node {
                return (id, via);
            }
            via = via.or(o.via);
        }
        (0, via)
    }

    /// The top open leaf's node: the native running now, if one is.
    fn running_leaf(&self) -> Option<usize> {
        self.open.last().filter(|o| o.leaf).and_then(|o| o.node)
    }

    fn enter(&mut self, node: Option<usize>, depth: usize, leaf: bool, via: Option<usize>) {
        let start = Instant::now();
        self.readings += 1;
        let mut nested = false;
        let mut edge = None;
        if let Some(id) = node {
            let (parent, via) = self.parent();
            let i = self.edge_index(parent, id, via);
            self.nodes[parent].out[i].2.calls += 1;
            edge = Some((parent, i));
            let n = &mut self.nodes[id];
            nested = n.active > 0;
            n.active += 1;
            n.calls += 1;
            n.max_depth = n.max_depth.max(n.active);
        }
        self.open.push(Open {
            node,
            depth,
            start,
            children: Duration::ZERO,
            nested,
            leaf,
            via,
            edge,
        });
    }

    fn leave(&mut self) {
        let Some(o) = self.open.pop() else {
            return;
        };
        let span = o.start.elapsed();
        self.readings += 1;
        let Some(id) = o.node else {
            // An anonymous body: its children were charged to the named node
            // above it as they closed, and its own span is nobody's.
            return;
        };
        {
            let n = &mut self.nodes[id];
            n.active = n.active.saturating_sub(1);
            n.exclusive += span.saturating_sub(o.children);
            if !o.nested {
                n.inclusive += span;
            }
            n.min = n.min.min(span);
            n.max = n.max.max(span);
        }
        match self.open.iter_mut().rev().find(|x| x.node.is_some()) {
            Some(p) => p.children += span,
            None => self.root_children += span,
        }
        if let Some((parent, i)) = o.edge
            && parent != id
        {
            self.nodes[parent].out[i].2.time += span;
        }
    }

    /// Hook 1: a registered native is about to run, `depth` frames deep.
    pub fn open_native(&mut self, sym: Symbol, kind: WordKind, depth: usize) {
        let id = self.sym_node(sym, Kind::Native);
        self.nodes[id].word_kind = Some(kind);
        self.enter(Some(id), depth, true, None);
    }

    /// Hook 2: a method native is about to run.
    pub fn open_method(&mut self, name: &str, depth: usize) {
        let id = self.name_node(name, Kind::Method);
        self.enter(Some(id), depth, true, None);
    }

    /// Hook 2: a conditional runner is about to run.
    pub fn open_conditional(&mut self, ty: &str, depth: usize) {
        let id = self.name_node(ty, Kind::Conditional);
        self.enter(Some(id), depth, true, None);
    }

    /// The native, method or runner opened last has returned.
    ///
    /// Everything the leaf opened beneath itself has been closed by the frame
    /// pops inside it; if a path missed one, closing down to the leaf is what
    /// keeps the mirror from drifting.
    pub fn close_leaf(&mut self) {
        while let Some(top) = self.open.last() {
            let was_leaf = top.leaf;
            self.leave();
            if was_leaf {
                return;
            }
        }
    }

    /// Hook 3: a frame was pushed, so the frames are now `depth` deep. A
    /// named frame opens its word's node; an unnamed one opens an anonymous
    /// entry that remembers which native filed it, if one did.
    pub fn open_frame(&mut self, who: Option<Symbol>, depth: usize) {
        let via = self.filed_by.take();
        match who {
            Some(w) => {
                let id = self.sym_node(w, Kind::Word);
                self.enter(Some(id), depth, false, None);
            }
            None => self.enter(None, depth, false, via),
        }
    }

    /// Hook 3: frames were popped and are now `depth` deep. Every entry that
    /// belonged to a deeper frame is closed — by depth, not by one, so an
    /// unwind that popped forty frames closes forty entries.
    pub fn close_to(&mut self, depth: usize) {
        while self.open.last().is_some_and(|o| o.depth > depth) {
            self.leave();
        }
    }

    /// A native filed a lambda as a tail request: the next unnamed frame is
    /// its body, and what that body calls is charged to the enclosing word
    /// `via` that native — `!`, `if`, whichever runs a body in tail position.
    pub fn tail_filed(&mut self) {
        self.filed_by = self.running_leaf();
    }

    /// The program spelled `spelled` and dispatch resolved it to `target`.
    pub fn spelled(&mut self, target: Symbol, kind: Kind, spelled: Symbol) {
        let id = self.sym_node(target, kind);
        let n = &mut self.nodes[id];
        if !n.spelled.contains(&spelled) {
            n.spelled.push(spelled);
        }
    }

    /// `name` was registered while already bound.
    pub fn redefined(&mut self, name: &str) {
        self.redefined.insert(name.to_string());
    }

    /// The run is over: close whatever is still open, and fix the root.
    pub fn finish(&mut self) {
        while !self.open.is_empty() {
            self.leave();
        }
        let wall = self.started.elapsed();
        self.readings += 1;
        self.wall = Some(wall);
        let root = &mut self.nodes[0];
        root.calls = 1;
        root.inclusive = wall;
        root.exclusive = wall.saturating_sub(self.root_children);
        root.min = wall;
        root.max = wall;
        root.max_depth = 1;
    }

    /// Open entries, for a test that checks the mirror did not drift.
    pub fn open_entries(&self) -> usize {
        self.open.len()
    }
}
