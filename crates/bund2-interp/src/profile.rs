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
//!
//! **Steps (D188).** Beside the clock, every node carries a count of the
//! values the interpreter dispatched while it was the innermost named node
//! open: one increment per step of `run_to`, no reading of any clock. The
//! count follows the same arithmetic as the time — a span's steps are the
//! steps between its opening and its closing, exclusive less its direct
//! children's — so exclusive steps sum to the root's, and two runs of one
//! program on any two hosts count the same. A native dispatches nothing of
//! its own, so its steps are zero; its cost in steps is the one step that
//! named it, charged to the word whose body did.

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
    /// The activations that were not nested inside another activation of
    /// this node: the ones whose spans `inclusive`, `min` and `max` are over.
    /// Equal to `calls` for a word that never recursed.
    pub outer: u64,
    pub inclusive: Duration,
    pub exclusive: Duration,
    /// The shortest and longest of one *outermost* span. A nested
    /// activation's span lies inside an outer one and says nothing on its
    /// own, so it is left out, as it is left out of `inclusive`.
    pub min: Duration,
    pub max: Duration,
    /// The deepest nesting of this node inside itself: 1 for a word that
    /// never recursed.
    pub max_depth: u32,
    /// Values dispatched while this node was the innermost named node open —
    /// its exclusive steps (D188). Exact, and the same on every host.
    pub steps: u64,
    /// Values dispatched inside this node's outermost activations, its own
    /// and its callees' — its inclusive steps.
    pub steps_inclusive: u64,
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
            outer: 0,
            inclusive: Duration::ZERO,
            exclusive: Duration::ZERO,
            min: Duration::MAX,
            max: Duration::ZERO,
            max_depth: 0,
            steps: 0,
            steps_inclusive: 0,
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
    /// The steps inside the callee's spans along this edge; zero on a self
    /// edge, as the time is.
    pub steps: u64,
}

/// An open activation on the mirror stack.
struct Open {
    /// `None` for a body no word names.
    node: Option<usize>,
    /// The interpreter's frame depth this entry belongs to: a word's frame
    /// sits at this depth, a leaf was opened while the frames were this deep.
    depth: usize,
    start: Instant,
    /// The record's step count when this entry opened.
    start_steps: u64,
    /// The spans of the direct children already closed under this entry.
    children: Duration,
    /// The steps inside those children.
    children_steps: u64,
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
    /// The steps inside those spans.
    root_children_steps: u64,
    /// Every value the interpreter dispatched while the record was kept
    /// (D188). One increment per step, read by no clock.
    steps: u64,
    /// How many times the clock was read.
    pub readings: u64,
    /// What one reading costs on this host, measured by whoever made the
    /// record and printed beside every table.
    pub reading_cost: Duration,
    /// The smallest difference two readings of the clock can show on this
    /// host, measured with `reading_cost`. A span shorter than this reads as
    /// zero or as one tick; a per-call figure near it is the clock's, not
    /// the word's.
    pub resolution: Duration,
    /// The node that was innermost when the run's last failure was raised,
    /// with the failure's text: the native that answered the error, or the
    /// word whose body named something that could not be dispatched. `None`
    /// when nothing failed.
    failed_in: Option<(usize, String)>,
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
    /// A fresh record. `reading_cost` and `resolution` are what whoever
    /// makes the record measured of the clock on this host; the record
    /// prints them and never measures them itself.
    pub fn new(reading_cost: Duration, resolution: Duration) -> Self {
        let root = Node::new(Key::Name("<program>".into(), Kind::Program), Kind::Program);
        Self {
            nodes: vec![root],
            by_sym: Vec::new(),
            by_name: HashMap::new(),
            open: Vec::new(),
            started: Instant::now(),
            wall: None,
            root_children: Duration::ZERO,
            root_children_steps: 0,
            steps: 0,
            readings: 1,
            reading_cost,
            resolution,
            console_attached: false,
            redefined: HashSet::new(),
            filed_by: None,
            failed_in: None,
        }
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Every value dispatched while the record was kept.
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// The node that was innermost when the last failure was raised.
    pub fn failing(&self) -> Option<usize> {
        self.failed_in.as_ref().map(|(id, _)| *id)
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
            start_steps: self.steps,
            children: Duration::ZERO,
            children_steps: 0,
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
        let span_steps = self.steps.saturating_sub(o.start_steps);
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
            n.steps += span_steps.saturating_sub(o.children_steps);
            // An outermost activation: its span is the node's own time, and
            // the one a minimum or maximum can be read from. A nested one
            // lies inside it and is counted once, in `calls` and `max_depth`.
            if !o.nested {
                n.outer += 1;
                n.inclusive += span;
                n.steps_inclusive += span_steps;
                n.min = n.min.min(span);
                n.max = n.max.max(span);
            }
        }
        match self.open.iter_mut().rev().find(|x| x.node.is_some()) {
            Some(p) => {
                p.children += span;
                p.children_steps += span_steps;
            }
            None => {
                self.root_children += span;
                self.root_children_steps += span_steps;
            }
        }
        if let Some((parent, i)) = o.edge
            && parent != id
        {
            let e = &mut self.nodes[parent].out[i].2;
            e.time += span;
            e.steps += span_steps;
        }
    }

    /// The interpreter is about to dispatch one value (D188). One increment
    /// on the on path; the off path never reaches here.
    #[inline]
    pub fn step(&mut self) {
        self.steps += 1;
    }

    /// A failure with this text was raised with the mirror as it is: the
    /// innermost named node open is where. One failure is seen at every
    /// level it passes on its way out, wrapped by the natives it passes
    /// through, so a mark whose text contains the one kept is the same
    /// failure further out and the inner mark stands — the rule D173's
    /// traceback uses. A different text is a later failure, and the last
    /// one raised is the one that ended the run.
    pub fn mark_failure(&mut self, reason: &str) {
        match &self.failed_in {
            Some((_, kept)) if reason.contains(kept.as_str()) => {}
            _ => self.failed_in = Some((self.parent().0, reason.to_string())),
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
        root.outer = 1;
        root.inclusive = wall;
        root.exclusive = wall.saturating_sub(self.root_children);
        root.min = wall;
        root.max = wall;
        root.max_depth = 1;
        root.steps_inclusive = self.steps;
        root.steps = self.steps.saturating_sub(self.root_children_steps);
    }

    /// Open entries, for a test that checks the mirror did not drift.
    pub fn open_entries(&self) -> usize {
        self.open.len()
    }
}
