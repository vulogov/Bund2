//! Graph algorithms of Bund2's own (D192, RFC-0012): traversal, structure,
//! and three weighted questions, over the graph MAP the reference's words
//! read (`graph.rs`).
//!
//! **None of these is the reference's.** Its registry has seven graph words
//! (`reference/Bund/src/stdlib/functions/graph/mod.rs:113-120`), and Bund2
//! has all seven in `graph.rs`. These twelve are additions, as `debug.profile`
//! is, so `conform`, `IMPLEMENTED` and `COVERAGE` move by zero, and what
//! checks them is the cross-checks at the foot of this file rather than a
//! golden.
//!
//! Three rules hold for every word here, and each was the owner's ruling:
//!
//! - **A graph reads as it does to `graph.paths`** (D192 §3): node names
//!   Title-cased, a weight a FLOAT and 100.0 when absent, an edge naming an
//!   unknown node an error, every edge directed. A second edge between the
//!   same two nodes, in the same direction, replaces the first, as `algos`'
//!   `Graph::add_edge` does. Unlike `graph.paths`, a weight that is not
//!   finite is refused (D193). `graph.path` reads differently (F101's note).
//!   A node operand is Title-cased, which no reference word does (F218):
//!   D192's choice, so a node is found the way `nodes` spelled it.
//! - **Every answer is in the order of the `nodes` list** (§4), so it prints
//!   the same on every run.
//! - **An impossible question is an error** (§5) a program can catch with
//!   `?try`, and it names a node where there is one to name.
//!
//! Each algorithm is written here, over [`Net`], rather than taken from
//! `algos` (§2): the modules at the pinned revision answer in `HashMap`
//! order, and several index without a check. Every loop is bounded by the
//! graph it was handed (D39), and the depth-first searches keep their own
//! stack, so a deep graph cannot overflow Rust's.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;

use crate::graph::{graph_operand, node_name};

/// An edge as the algorithms hand it back: from, to, and a weight or share.
type Edge = (usize, usize, f64);

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// A graph read once, for the algorithms below.
///
/// Nodes are numbered in the order of the `nodes` list, duplicates skipped,
/// as `build_algos` numbers them. `edges` holds one edge per ordered pair,
/// in the order each pair was first seen, with the weight it was last
/// given. `out` is each node's successors, ascending and distinct.
#[derive(Debug)]
pub(crate) struct Net {
    pub(crate) names: Vec<String>,
    pub(crate) ix: HashMap<String, usize>,
    pub(crate) edges: Vec<(usize, usize, f64)>,
    pub(crate) out: Vec<Vec<usize>>,
}

impl Net {
    fn len(&self) -> usize {
        self.names.len()
    }

    fn name_list(&self, nodes: &[usize]) -> Result<BundValue, Error> {
        let names = nodes
            .iter()
            .map(|n| self.name(*n).map(BundValue::str))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(BundValue::list(names))
    }

    /// A node's name. The algorithms answer only indices they were handed,
    /// so one past the end is a defect here, not a node to leave out.
    fn name(&self, n: usize) -> Result<String, Error> {
        self.names.get(n).cloned().ok_or_else(|| {
            Error::internal(format!(
                "a graph algorithm answered node {n} of a graph of {}",
                self.names.len()
            ))
        })
    }
}

/// Read `nodes` and `edges` into a [`Net`], with `build_algos`' rules and
/// the word's own name in each refusal.
pub(crate) fn read(nodes: &BundValue, edges: &BundValue, word: &str) -> Result<Net, Error> {
    let mut names: Vec<String> = Vec::new();
    let mut ix: HashMap<String, usize> = HashMap::new();
    let node_list = nodes
        .as_list()
        .ok_or_else(|| Error(format!("{word}: the nodes must be a LIST")))?;
    for n in node_list {
        let name = n.as_str().map(|s| node_name(&s)).ok_or_else(|| {
            Error(format!(
                "{word}: a node name must be a STRING, not {}",
                n.type_name()
            ))
        })?;
        if !ix.contains_key(&name) {
            ix.insert(name.clone(), names.len());
            names.push(name);
        }
    }
    let edge_list = edges
        .as_list()
        .ok_or_else(|| Error(format!("{word}: the edges must be a LIST")))?;
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    let mut at: HashMap<(usize, usize), usize> = HashMap::new();
    for e in edge_list {
        let rel = e.as_list().ok_or_else(|| {
            Error(format!(
                "{word}: an edge must be a LIST, not {}",
                e.type_name()
            ))
        })?;
        if rel.len() < 2 {
            return Err(Error(format!("{word}: an edge is too small")));
        }
        let end = |k: usize| -> Result<String, Error> {
            rel.get(k)
                .and_then(BundValue::as_str)
                .map(|s| node_name(&s))
                .ok_or_else(|| Error(format!("{word}: an edge's node must be a STRING")))
        };
        let (from, to) = (end(0)?, end(1)?);
        let (Some(&f), Some(&t)) = (ix.get(&from), ix.get(&to)) else {
            return Err(Error(format!(
                "{word}: an edge names unknown nodes: {from} and {to}"
            )));
        };
        // Exactly three elements carry a weight; two or four and more weigh
        // 100.0 (`reference/Bund/src/stdlib/functions/graph/make_graph.rs:125-133`).
        let w = match rel.get(2) {
            Some(w) if rel.len() == 3 => match *w.unboxed() {
                BundValue::Float(f, _) => f,
                _ => {
                    return Err(Error(format!(
                        "{word}: an edge's weight must be a FLOAT, not {}",
                        w.type_name()
                    )));
                }
            },
            _ => 100.0,
        };
        // Not `graph.paths`' rule: `make_graph` takes any FLOAT, so NaN and
        // ±inf, which `math.sqrt` and an overflowing `*` make, reach its
        // Dijkstra. Here no answer over such a weight means anything (D193).
        if !w.is_finite() {
            return Err(Error(format!(
                "{word}: an edge's weight must be finite: {from} -> {to} weighs {w}"
            )));
        }
        match at.get(&(f, t)) {
            Some(&k) => {
                if let Some(slot) = edges.get_mut(k) {
                    slot.2 = w;
                }
            }
            None => {
                at.insert((f, t), edges.len());
                edges.push((f, t, w));
            }
        }
    }
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for &(f, t, _) in &edges {
        if let Some(o) = out.get_mut(f) {
            o.push(t);
        }
    }
    for o in &mut out {
        o.sort_unstable();
        o.dedup();
    }
    Ok(Net {
        names,
        ix,
        edges,
        out,
    })
}

/// Pull the graph from the top of the stack and read it.
fn operand(vm: &mut dyn Vm, word: &str) -> Result<Net, Error> {
    let (nodes, edges) = graph_operand(vm, word)?;
    read(&nodes, &edges, word)
}

/// Pull a node's name, the `n`th operand, and find it in `net`. `role` is
/// how a refusal names it: "start", or empty for plain "node".
fn node_operand(
    vm: &mut dyn Vm,
    net: &Net,
    word: &str,
    role: &str,
    n: usize,
) -> Result<usize, Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{word}: NO DATA #{n}")))?;
    let role = if role.is_empty() {
        String::new()
    } else {
        format!("{role} ")
    };
    let name = v.as_str().map(|s| node_name(&s)).ok_or_else(|| {
        Error(format!(
            "{word}: the {role}node must be a STRING, not {}",
            v.type_name()
        ))
    })?;
    net.ix
        .get(&name)
        .copied()
        .ok_or_else(|| Error(format!("{word}: unknown {role}node: {name}")))
}

/// Breadth-first order from `s`, successors taken in node order.
pub(crate) fn bfs(net: &Net, s: usize) -> Vec<usize> {
    let mut seen = vec![false; net.len()];
    let mut order = Vec::new();
    let mut queue = VecDeque::from([s]);
    if let Some(m) = seen.get_mut(s) {
        *m = true;
    }
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &v in net.out.get(u).map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(m) = seen.get_mut(v)
                && !*m
            {
                *m = true;
                queue.push_back(v);
            }
        }
    }
    order
}

/// Depth-first preorder from `s`, successors taken in node order. The stack
/// is this function's: each step advances one successor or pops one node,
/// so it ends within edges plus nodes steps.
pub(crate) fn dfs(net: &Net, s: usize) -> Vec<usize> {
    let mut seen = vec![false; net.len()];
    let mut order = vec![s];
    if let Some(m) = seen.get_mut(s) {
        *m = true;
    }
    let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
    while let Some(top) = stack.last_mut() {
        let (u, i) = *top;
        match net.out.get(u).and_then(|o| o.get(i)) {
            Some(&v) => {
                top.1 += 1;
                if let Some(m) = seen.get_mut(v)
                    && !*m
                {
                    *m = true;
                    order.push(v);
                    stack.push((v, 0));
                }
            }
            None => {
                stack.pop();
            }
        }
    }
    order
}

/// Every node reachable from `s`, `s` itself included, in node order — the
/// row of `graph.transitiveclosure` that starts at `s`, which is reflexive.
pub(crate) fn reachable(net: &Net, s: usize) -> Vec<usize> {
    let mut found = bfs(net, s);
    found.sort_unstable();
    found
}

/// One directed cycle, as the nodes along it from where it was entered, or
/// `None` for an acyclic graph. A self-loop is a cycle of one.
///
/// Searched depth-first from each unvisited node in node order; the first
/// edge back to a node still on the search's stack closes the cycle.
pub(crate) fn find_cycle(net: &Net) -> Option<Vec<usize>> {
    let n = net.len();
    // `None`: not visited. `Some(Some(k))`: on the stack at position k.
    // `Some(None)`: finished.
    let mut state: Vec<Option<Option<usize>>> = vec![None; n];
    for root in 0..n {
        if state.get(root).is_some_and(Option::is_some) {
            continue;
        }
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        if let Some(s) = state.get_mut(root) {
            *s = Some(Some(0));
        }
        while let Some(top) = stack.last_mut() {
            let (u, i) = *top;
            match net.out.get(u).and_then(|o| o.get(i)) {
                Some(&v) => {
                    top.1 += 1;
                    match state.get(v).copied().flatten() {
                        None => {
                            let depth = stack.len();
                            if let Some(s) = state.get_mut(v) {
                                *s = Some(Some(depth));
                            }
                            stack.push((v, 0));
                        }
                        Some(Some(k)) => {
                            return Some(stack.get(k..)?.iter().map(|f| f.0).collect());
                        }
                        Some(None) => {}
                    }
                }
                None => {
                    if let Some(s) = state.get_mut(u) {
                        *s = Some(None);
                    }
                    stack.pop();
                }
            }
        }
    }
    None
}

/// A topological order, the smallest-numbered ready node first, or the node
/// a cycle was found through: the first node of the cycle [`find_cycle`]
/// answers. The outer `Err` is a defect: a sort that stalled with no cycle.
pub(crate) fn toposort(net: &Net) -> Result<Result<Vec<usize>, usize>, Error> {
    let n = net.len();
    let mut indegree = vec![0usize; n];
    for o in &net.out {
        for &v in o {
            if let Some(d) = indegree.get_mut(v) {
                *d += 1;
            }
        }
    }
    let mut ready: BinaryHeap<Reverse<usize>> = indegree
        .iter()
        .enumerate()
        .filter(|(_, d)| **d == 0)
        .map(|(u, _)| Reverse(u))
        .collect();
    let mut order = Vec::with_capacity(n);
    while let Some(Reverse(u)) = ready.pop() {
        order.push(u);
        for &v in net.out.get(u).map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(d) = indegree.get_mut(v) {
                *d -= 1;
                if *d == 0 {
                    ready.push(Reverse(v));
                }
            }
        }
    }
    if order.len() == n {
        return Ok(Ok(order));
    }
    // Some node was never ready, so a cycle exists and the search finds it.
    match find_cycle(net).and_then(|c| c.first().copied()) {
        Some(through) => Ok(Err(through)),
        None => Err(Error::internal(format!(
            "graph.toposort ordered {} of {n} nodes, and no cycle holds the rest",
            order.len()
        ))),
    }
}

/// Group nodes by `label`: each group's members in node order, the groups
/// ordered by their first member.
fn groups(label: &[usize]) -> Vec<Vec<usize>> {
    let mut slot: HashMap<usize, usize> = HashMap::new();
    let mut out: Vec<Vec<usize>> = Vec::new();
    for (u, l) in label.iter().enumerate() {
        let k = *slot.entry(*l).or_insert_with(|| {
            out.push(Vec::new());
            out.len() - 1
        });
        if let Some(g) = out.get_mut(k) {
            g.push(u);
        }
    }
    out
}

/// Union-find root of `u`, halving the path as it goes.
fn root(parent: &mut [usize], mut u: usize) -> usize {
    while let Some(&p) = parent.get(u) {
        if p == u {
            break;
        }
        let gp = parent.get(p).copied().unwrap_or(p);
        if let Some(slot) = parent.get_mut(u) {
            *slot = gp;
        }
        u = gp;
    }
    u
}

/// The weakly connected components: every edge read both ways.
pub(crate) fn components(net: &Net) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..net.len()).collect();
    for &(f, t, _) in &net.edges {
        let (a, b) = (root(&mut parent, f), root(&mut parent, t));
        if a != b
            && let Some(slot) = parent.get_mut(a.max(b))
        {
            *slot = a.min(b);
        }
    }
    let label: Vec<usize> = (0..net.len()).map(|u| root(&mut parent, u)).collect();
    groups(&label)
}

/// The strongly connected components, by Tarjan's algorithm with the
/// recursion kept on a stack of this function's own.
pub(crate) fn scc(net: &Net) -> Vec<Vec<usize>> {
    let n = net.len();
    let mut index: Vec<Option<usize>> = vec![None; n];
    let mut low = vec![0usize; n];
    let mut on = vec![false; n];
    let mut comp = vec![0usize; n];
    let mut st: Vec<usize> = Vec::new();
    let (mut next, mut ncomp) = (0usize, 0usize);
    let mut enter = |v: usize,
                     index: &mut Vec<Option<usize>>,
                     low: &mut Vec<usize>,
                     on: &mut Vec<bool>,
                     st: &mut Vec<usize>| {
        if let Some(s) = index.get_mut(v) {
            *s = Some(next);
        }
        if let Some(s) = low.get_mut(v) {
            *s = next;
        }
        if let Some(s) = on.get_mut(v) {
            *s = true;
        }
        st.push(v);
        next += 1;
    };
    for r in 0..n {
        if index.get(r).copied().flatten().is_some() {
            continue;
        }
        enter(r, &mut index, &mut low, &mut on, &mut st);
        let mut call: Vec<(usize, usize)> = vec![(r, 0)];
        while let Some(frame) = call.last_mut() {
            let u = frame.0;
            match net.out.get(u).and_then(|o| o.get(frame.1)) {
                Some(&v) => {
                    frame.1 += 1;
                    match index.get(v).copied().flatten() {
                        None => {
                            enter(v, &mut index, &mut low, &mut on, &mut st);
                            call.push((v, 0));
                        }
                        Some(iv) => {
                            if on.get(v).copied().unwrap_or(false)
                                && let Some(l) = low.get_mut(u)
                            {
                                *l = (*l).min(iv);
                            }
                        }
                    }
                }
                None => {
                    call.pop();
                    let lu = low.get(u).copied().unwrap_or(0);
                    if let Some(&(p, _)) = call.last()
                        && let Some(l) = low.get_mut(p)
                    {
                        *l = (*l).min(lu);
                    }
                    if index.get(u).copied().flatten() == Some(lu) {
                        while let Some(w) = st.pop() {
                            if let Some(s) = on.get_mut(w) {
                                *s = false;
                            }
                            if let Some(s) = comp.get_mut(w) {
                                *s = ncomp;
                            }
                            if w == u {
                                break;
                            }
                        }
                        ncomp += 1;
                    }
                }
            }
        }
    }
    groups(&comp)
}

/// A minimum spanning forest by Kruskal, every edge read both ways and
/// self-loops left out. Edges of equal weight are taken in the order the
/// graph lists them, so the forest chosen is the same on every run.
pub(crate) fn mst(net: &Net) -> Vec<(usize, usize, f64)> {
    let mut order: Vec<&(usize, usize, f64)> =
        net.edges.iter().filter(|(f, t, _)| f != t).collect();
    order.sort_by(|a, b| a.2.total_cmp(&b.2));
    let mut parent: Vec<usize> = (0..net.len()).collect();
    let mut chosen = Vec::new();
    for &&(f, t, w) in &order {
        let (a, b) = (root(&mut parent, f), root(&mut parent, t));
        if a != b {
            if let Some(slot) = parent.get_mut(a.max(b)) {
                *slot = a.min(b);
            }
            chosen.push((f, t, w));
        }
    }
    chosen
}

/// Shortest distances from `s` by Bellman–Ford, which takes negative
/// weights. The inner `Err` names a node on a negative cycle reachable from
/// `s`: the smallest-numbered node of the one cycle the walk back finds,
/// which depends on the order of the edges when two cycles are reachable
/// (D193). The outer `Err` is a defect in the walk.
pub(crate) fn bellman_ford(net: &Net, s: usize) -> Result<Result<Vec<Option<f64>>, usize>, Error> {
    let n = net.len();
    let mut dist: Vec<Option<f64>> = vec![None; n];
    let mut pred: Vec<Option<usize>> = vec![None; n];
    if let Some(d) = dist.get_mut(s) {
        *d = Some(0.0);
    }
    let relax = |dist: &mut Vec<Option<f64>>, pred: &mut Vec<Option<usize>>| -> Option<usize> {
        let mut last = None;
        for &(f, t, w) in &net.edges {
            let Some(df) = dist.get(f).copied().flatten() else {
                continue;
            };
            let better = match dist.get(t).copied().flatten() {
                None => true,
                Some(dt) => df + w < dt,
            };
            if better {
                if let Some(d) = dist.get_mut(t) {
                    *d = Some(df + w);
                }
                if let Some(p) = pred.get_mut(t) {
                    *p = Some(f);
                }
                last = Some(t);
            }
        }
        last
    };
    for _ in 1..n.max(1) {
        if relax(&mut dist, &mut pred).is_none() {
            return Ok(Ok(dist));
        }
    }
    let Some(mut v) = relax(&mut dist, &mut pred) else {
        return Ok(Ok(dist));
    };
    // Every node the walk visits was relaxed, so it has a predecessor.
    let back = |u: usize| -> Result<usize, Error> {
        pred.get(u).copied().flatten().ok_or_else(|| {
            Error::internal(format!(
                "graph.bellmanford walked back to node {u}, which no edge relaxed"
            ))
        })
    };
    // Walking back as many steps as there are nodes lands on the cycle.
    for _ in 0..n {
        v = back(v)?;
    }
    // Report that cycle's smallest-numbered node, so the message is the same
    // whichever of its edges the last pass relaxed.
    let mut least = v;
    let mut u = back(v)?;
    let mut steps = 0;
    while u != v {
        steps += 1;
        if steps > n {
            return Err(Error::internal(
                "graph.bellmanford walked a cycle longer than the graph",
            ));
        }
        least = least.min(u);
        u = back(u)?;
    }
    Ok(Err(least))
}

/// The maximum flow from `s` to `t`, each edge's weight its capacity, by
/// Edmonds–Karp: augment along a shortest path in the residual graph until
/// none is left. Answers the flow and each listed edge's share of it.
///
/// `Err` is the message. A capacity must not be below zero; the reader has
/// already refused one that is not finite. Shortest augmenting paths need at
/// most nodes × edges rounds; the loop stops there, and passing it would be
/// a defect here, said as one. The bound needs no tolerance: the bottleneck
/// arc ends at `cap - b`, exactly 0.0, and every reverse arc gains `b > 0`.
pub(crate) fn max_flow(
    net: &Net,
    s: usize,
    t: usize,
    word: &str,
) -> Result<(f64, Vec<Edge>), Error> {
    struct Arc {
        to: usize,
        cap: f64,
        rev: usize,
    }
    let n = net.len();
    let mut arcs: Vec<Vec<Arc>> = (0..n).map(|_| Vec::new()).collect();
    let mut listed: Vec<(usize, usize, usize, f64)> = Vec::new();
    for &(f, to, w) in &net.edges {
        if !w.is_finite() {
            return Err(Error::internal(format!(
                "graph.maxflow was handed a capacity of {w}, which the reader refuses"
            )));
        }
        if w < 0.0 {
            return Err(Error(format!(
                "{word}: a capacity must not be below zero: {} -> {} weighs {w}",
                net.name(f)?,
                net.name(to)?
            )));
        }
        if f == to {
            continue;
        }
        let (kf, kt) = (
            arcs.get(f).map_or(0, Vec::len),
            arcs.get(to).map_or(0, Vec::len),
        );
        if let Some(a) = arcs.get_mut(f) {
            a.push(Arc {
                to,
                cap: w,
                rev: kt,
            });
        }
        if let Some(a) = arcs.get_mut(to) {
            a.push(Arc {
                to: f,
                cap: 0.0,
                rev: kf,
            });
        }
        listed.push((f, to, kf, w));
    }
    // An arc is present while any capacity is left on it: no tolerance, so
    // a small capacity beside a huge one still carries flow (D193).
    let bound = n.saturating_mul(net.edges.len()).saturating_add(1);
    let mut flow = 0.0;
    let mut rounds = 0usize;
    loop {
        let mut via: Vec<Option<(usize, usize)>> = vec![None; n];
        let mut queue = VecDeque::from([s]);
        let mut reached = false;
        while let Some(u) = queue.pop_front() {
            if u == t {
                reached = true;
                break;
            }
            for (k, a) in arcs
                .get(u)
                .map(Vec::as_slice)
                .unwrap_or(&[])
                .iter()
                .enumerate()
            {
                if a.cap > 0.0 && a.to != s && via.get(a.to).is_some_and(Option::is_none) {
                    if let Some(slot) = via.get_mut(a.to) {
                        *slot = Some((u, k));
                    }
                    queue.push_back(a.to);
                }
            }
        }
        if !reached {
            break;
        }
        rounds += 1;
        if rounds > bound {
            return Err(Error::internal(
                "graph.maxflow augmented more often than nodes × edges, which \
                 shortest augmenting paths cannot need",
            ));
        }
        // The path, from the sink back to the source.
        let mut path: Vec<(usize, usize)> = Vec::new();
        let mut v = t;
        while v != s {
            let Some((u, k)) = via.get(v).copied().flatten() else {
                return Err(Error::internal(
                    "graph.maxflow reached the sink by a path it cannot retrace",
                ));
            };
            path.push((u, k));
            v = u;
            if path.len() > n {
                return Err(Error::internal(
                    "graph.maxflow retraced a path longer than the graph",
                ));
            }
        }
        let b = path
            .iter()
            .filter_map(|&(u, k)| arcs.get(u).and_then(|a| a.get(k)).map(|a| a.cap))
            .fold(f64::INFINITY, f64::min);
        for &(u, k) in &path {
            let Some((to, rev)) = arcs.get_mut(u).and_then(|a| a.get_mut(k)).map(|a| {
                a.cap -= b;
                (a.to, a.rev)
            }) else {
                continue;
            };
            if let Some(r) = arcs.get_mut(to).and_then(|a| a.get_mut(rev)) {
                r.cap += b;
            }
        }
        flow += b;
    }
    let shares = listed
        .into_iter()
        .filter_map(|(f, to, k, w)| {
            let left = arcs.get(f).and_then(|a| a.get(k)).map_or(w, |a| a.cap);
            let used = w - left;
            (used > 0.0).then_some((f, to, used))
        })
        .collect();
    Ok((flow, shares))
}

/// `<start> <graph> graph.bfs` — the nodes reached from `start`, breadth
/// first.
fn graph_bfs(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.BFS")?;
    let s = node_operand(vm, &net, "GRAPH.BFS", "start", 2)?;
    vm.push(net.name_list(&bfs(&net, s))?);
    Ok(())
}

/// `<start> <graph> graph.dfs` — the nodes reached from `start`, depth
/// first, each where the search first entered it.
fn graph_dfs(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.DFS")?;
    let s = node_operand(vm, &net, "GRAPH.DFS", "start", 2)?;
    vm.push(net.name_list(&dfs(&net, s))?);
    Ok(())
}

/// `<start> <graph> graph.reachable` — every node some path from `start`
/// reaches, `start` included, in node order.
fn graph_reachable(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.REACHABLE")?;
    let s = node_operand(vm, &net, "GRAPH.REACHABLE", "start", 2)?;
    vm.push(net.name_list(&reachable(&net, s))?);
    Ok(())
}

/// `<node> <graph> graph.neighbors` — the nodes one edge out of `node`.
fn graph_neighbors(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.NEIGHBORS")?;
    let u = node_operand(vm, &net, "GRAPH.NEIGHBORS", "", 2)?;
    let out = net.out.get(u).cloned().unwrap_or_default();
    vm.push(net.name_list(&out)?);
    Ok(())
}

/// `<node> <graph> graph.degree` — a MAP of the edges into `node` (`in`)
/// and out of it (`out`). A self-loop counts once each way.
fn graph_degree(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.DEGREE")?;
    let u = node_operand(vm, &net, "GRAPH.DEGREE", "", 2)?;
    let into = net.edges.iter().filter(|e| e.1 == u).count();
    let out = net.edges.iter().filter(|e| e.0 == u).count();
    vm.push(
        BundValue::map(Default::default())
            .set(
                "in",
                BundValue::int(i64::try_from(into).unwrap_or(i64::MAX)),
            )
            .set(
                "out",
                BundValue::int(i64::try_from(out).unwrap_or(i64::MAX)),
            ),
    );
    Ok(())
}

/// `<graph> graph.toposort` — every node, each before the nodes its edges
/// lead to. A cycle has no such order and is an error naming a node on it.
fn graph_toposort(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.TOPOSORT")?;
    match toposort(&net)? {
        Ok(order) => {
            vm.push(net.name_list(&order)?);
            Ok(())
        }
        Err(u) => Err(Error(format!(
            "GRAPH.TOPOSORT: the graph has a cycle through {}",
            net.name(u)?
        ))),
    }
}

/// `<graph> graph.cycle` — one directed cycle, `[ A B C ]` for A→B→C→A, or
/// an empty LIST when there is none.
fn graph_cycle(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.CYCLE")?;
    let cycle = find_cycle(&net).unwrap_or_default();
    vm.push(net.name_list(&cycle)?);
    Ok(())
}

fn push_groups(vm: &mut dyn Vm, net: &Net, groups: &[Vec<usize>]) -> Result<(), Error> {
    let lists = groups
        .iter()
        .map(|g| net.name_list(g))
        .collect::<Result<Vec<_>, _>>()?;
    vm.push(BundValue::list(lists));
    Ok(())
}

/// `<graph> graph.components` — the nodes in groups joined by edges in
/// either direction: a LIST of LISTs.
fn graph_components(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.COMPONENTS")?;
    push_groups(vm, &net, &components(&net))
}

/// `<graph> graph.scc` — the nodes in groups where each reaches every
/// other: a LIST of LISTs.
fn graph_scc(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.SCC")?;
    push_groups(vm, &net, &scc(&net))
}

fn edge_rows(net: &Net, edges: &[(usize, usize, f64)]) -> Result<BundValue, Error> {
    let rows = edges
        .iter()
        .map(|&(f, t, w)| {
            Ok(BundValue::list(vec![
                BundValue::str(net.name(f)?),
                BundValue::str(net.name(t)?),
                BundValue::float(w),
            ]))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(BundValue::list(rows))
}

/// `<graph> graph.mst` — a minimum spanning forest, every edge read both
/// ways: a MAP of `edges`, each `[ from to weight ]` as the graph lists it,
/// lightest first, and their total `weight`.
fn graph_mst(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.MST")?;
    let chosen = mst(&net);
    let total: f64 = chosen.iter().map(|e| e.2).sum();
    vm.push(
        BundValue::map(Default::default())
            .set("edges", edge_rows(&net, &chosen)?)
            .set("weight", BundValue::float(total)),
    );
    Ok(())
}

/// `<start> <graph> graph.bellmanford` — shortest distances from `start`
/// where weights may be negative. The rows are `graph.paths`' — `node`, and
/// `path`, the distance or NODATA — in node order. A negative cycle `start`
/// reaches makes some distance unbounded, and is an error naming a node on
/// it.
fn graph_bellmanford(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.BELLMANFORD")?;
    let s = node_operand(vm, &net, "GRAPH.BELLMANFORD", "start", 2)?;
    let dist = match bellman_ford(&net, s)? {
        Ok(dist) => dist,
        Err(u) => {
            return Err(Error(format!(
                "GRAPH.BELLMANFORD: {} reaches a negative cycle through {}",
                net.name(s)?,
                net.name(u)?
            )));
        }
    };
    let rows = dist
        .iter()
        .enumerate()
        .map(|(u, d)| {
            Ok(BundValue::map(Default::default())
                .set("node", BundValue::str(net.name(u)?))
                .set("path", d.map_or_else(BundValue::nodata, BundValue::float)))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    vm.push(BundValue::list(rows));
    Ok(())
}

/// `<sink> <source> <graph> graph.maxflow` — the most that can flow from
/// `source` to `sink`, each edge's weight its capacity: a MAP of `flow` and
/// `edges`, each `[ from to share ]` for an edge that carries some, in the
/// order the graph lists them. Operands are in `graph.path`'s order.
fn graph_maxflow(vm: &mut dyn Vm) -> Result<(), Error> {
    let net = operand(vm, "GRAPH.MAXFLOW")?;
    let s = node_operand(vm, &net, "GRAPH.MAXFLOW", "source", 2)?;
    let t = node_operand(vm, &net, "GRAPH.MAXFLOW", "sink", 3)?;
    if s == t {
        return Err(Error(format!(
            "GRAPH.MAXFLOW: the source and the sink are one node: {}",
            net.name(s)?
        )));
    }
    let (flow, shares) = max_flow(&net, s, t, "GRAPH.MAXFLOW")?;
    vm.push(
        BundValue::map(Default::default())
            .set("flow", BundValue::float(flow))
            .set("edges", edge_rows(&net, &shares)?),
    );
    Ok(())
}

pub fn register(r: &mut Registry) {
    // The graph is on top, as for `graph.paths`; a workbench form holds the
    // graph, D111's container family.
    r.register_native("graph.bfs", graph_bfs, eff(2, 1), WordKind::Sync);
    crate::wb::bench!(r, "graph.bfs", graph_bfs, 2, 0);
    r.register_native("graph.dfs", graph_dfs, eff(2, 1), WordKind::Sync);
    crate::wb::bench!(r, "graph.dfs", graph_dfs, 2, 0);
    r.register_native(
        "graph.reachable",
        graph_reachable,
        eff(2, 1),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "graph.reachable", graph_reachable, 2, 0);
    r.register_native(
        "graph.neighbors",
        graph_neighbors,
        eff(2, 1),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "graph.neighbors", graph_neighbors, 2, 0);
    r.register_native("graph.degree", graph_degree, eff(2, 1), WordKind::Sync);
    crate::wb::bench!(r, "graph.degree", graph_degree, 2, 0);
    r.register_native("graph.toposort", graph_toposort, eff(1, 1), WordKind::Sync);
    crate::wb::bench!(r, "graph.toposort", graph_toposort);
    r.register_native("graph.cycle", graph_cycle, eff(1, 1), WordKind::Sync);
    crate::wb::bench!(r, "graph.cycle", graph_cycle);
    r.register_native(
        "graph.components",
        graph_components,
        eff(1, 1),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "graph.components", graph_components);
    r.register_native("graph.scc", graph_scc, eff(1, 1), WordKind::Sync);
    crate::wb::bench!(r, "graph.scc", graph_scc);
    r.register_native("graph.mst", graph_mst, eff(1, 1), WordKind::Sync);
    crate::wb::bench!(r, "graph.mst", graph_mst);
    r.register_native(
        "graph.bellmanford",
        graph_bellmanford,
        eff(2, 1),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "graph.bellmanford", graph_bellmanford, 2, 0);
    r.register_native("graph.maxflow", graph_maxflow, eff(3, 1), WordKind::Sync);
    crate::wb::bench!(r, "graph.maxflow", graph_maxflow, 3, 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::Interp;

    fn run_src(src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    fn top(src: &str) -> String {
        match run_src(src) {
            Ok(i) => i.peek().map(|v| v.display()).unwrap_or_default(),
            Err(e) => panic!("{src} failed: {e}"),
        }
    }

    /// How a LIST of names displays: `l(&["A", "B"])`.
    fn l(names: &[&str]) -> String {
        list_of(names).display()
    }

    fn list_of(names: &[&str]) -> BundValue {
        BundValue::list(names.iter().map(|s| BundValue::str(*s)).collect())
    }

    /// How a LIST of LISTs of names displays.
    fn ll(groups: &[&[&str]]) -> String {
        BundValue::list(groups.iter().map(|g| list_of(g)).collect()).display()
    }

    fn err_of(src: &str) -> String {
        match run_src(src) {
            Ok(_) => panic!("{src} was expected to fail"),
            Err(e) => e,
        }
    }

    /// A→B, A→C, B→D, C→D, D→E, and F alone.
    const DAG: &str = "[ [ :A :B 1.0 ] [ :A :C 4.0 ] [ :B :D 2.0 ] [ :C :D 1.0 ] [ :D :E 3.0 ] ] \
                       [ :A :B :C :D :E :F ] graph!";

    #[test]
    fn traversals_answer_in_node_order() {
        assert_eq!(
            top(&format!(":A {DAG} graph.bfs")),
            l(&["A", "B", "C", "D", "E"])
        );
        assert_eq!(
            top(&format!(":A {DAG} graph.dfs")),
            l(&["A", "B", "D", "E", "C"])
        );
        assert_eq!(
            top(&format!(":C {DAG} graph.reachable")),
            l(&["C", "D", "E"])
        );
        assert_eq!(top(&format!(":F {DAG} graph.reachable")), l(&["F"]));
        assert_eq!(top(&format!(":A {DAG} graph.neighbors")), l(&["B", "C"]));
        let d = top(&format!(":D {DAG} graph.degree"));
        assert!(d.contains("in=2") && d.contains("out=1"), "{d}");
    }

    /// A node operand is Title-cased as the graph's names are, unlike
    /// `graph.paths`' start node.
    #[test]
    fn a_node_operand_is_title_cased() {
        assert_eq!(
            top(&format!(":a {DAG} graph.bfs")),
            l(&["A", "B", "C", "D", "E"])
        );
        let e = err_of(&format!(":Z {DAG} graph.bfs"));
        assert!(e.contains("GRAPH.BFS: unknown start node: Z"), "{e}");
        let e = err_of(&format!(":Z {DAG} graph.degree"));
        assert!(e.contains("GRAPH.DEGREE: unknown node: Z"), "{e}");
        let e = err_of(&format!("1 {DAG} graph.dfs"));
        assert!(
            e.contains("GRAPH.DFS: the start node must be a STRING"),
            "{e}"
        );
    }

    #[test]
    fn structure_words_answer_lists_of_lists() {
        assert_eq!(
            top(&format!("{DAG} graph.toposort")),
            l(&["A", "B", "C", "D", "E", "F"])
        );
        assert_eq!(
            top(&format!("{DAG} graph.components")),
            ll(&[&["A", "B", "C", "D", "E"], &["F"]])
        );
        let cyc = "[ [ :A :B ] [ :B :C ] [ :C :A ] [ :C :D ] ] [ :D :A :B :C ] graph!";
        assert_eq!(
            top(&format!("{cyc} graph.scc")),
            ll(&[&["D"], &["A", "B", "C"]])
        );
        assert_eq!(top(&format!("{cyc} graph.cycle")), l(&["A", "B", "C"]));
        assert_eq!(top(&format!("{DAG} graph.cycle")), l(&[]));
        assert_eq!(
            top("[ [ :A :A ] ] [ :A ] graph! graph.cycle"),
            l(&["A"]),
            "a self-loop is a cycle of one"
        );
    }

    #[test]
    fn a_cycle_has_no_topological_order() {
        let e = err_of("[ [ :A :B ] [ :B :A ] ] [ :A :B ] graph! graph.toposort");
        assert!(
            e.contains("GRAPH.TOPOSORT: the graph has a cycle through A"),
            "{e}"
        );
        let caught = top(
            "?try :try { [ [ :A :A ] ] [ :A ] graph! graph.toposort } set \
             :except { \"caught\" } set !",
        );
        assert_eq!(caught, "caught");
    }

    #[test]
    fn weighted_words_answer_maps() {
        let m = top(&format!("{DAG} graph.mst"));
        assert!(m.contains("weight=7.0"), "{m}");
        let edge = |f: &str, t: &str, w: f64| {
            BundValue::list(vec![
                BundValue::str(f),
                BundValue::str(t),
                BundValue::float(w),
            ])
        };
        let want = BundValue::list(vec![
            edge("A", "B", 1.0),
            edge("C", "D", 1.0),
            edge("B", "D", 2.0),
            edge("D", "E", 3.0),
        ]);
        assert!(m.contains(&format!("edges={}", want.display())), "{m}");
        let b = top(&format!(":A {DAG} graph.bellmanford"));
        assert!(b.contains("node=A ::  path=0.0"), "{b}");
        let neg = "[ [ :A :B 4.0 ] [ :A :C 1.0 ] [ :B :C -3.0 ] ] [ :A :B :C ] graph!";
        let b = top(&format!(":A {neg} graph.bellmanford"));
        assert!(b.contains("node=C ::  path=1.0"), "{b}");
        let e = err_of(
            ":A [ [ :A :B 1.0 ] [ :B :C -2.0 ] [ :C :B 1.0 ] ] [ :A :B :C ] graph! graph.bellmanford",
        );
        assert!(
            e.contains("GRAPH.BELLMANFORD: A reaches a negative cycle through B"),
            "{e}"
        );
        let f = top(&format!(":E :A {DAG} graph.maxflow"));
        // The cut {A→B, C→D} carries 1 + 1.
        assert!(f.contains("flow=2.0"), "{f}");
        let e = err_of(&format!(":A :A {DAG} graph.maxflow"));
        assert!(e.contains("the source and the sink are one node: A"), "{e}");
    }

    /// A second edge between the same pair replaces the first, as `algos`'
    /// `add_edge` does for `graph.paths`.
    #[test]
    fn a_repeated_edge_takes_its_last_weight() {
        let b = top(":A [ [ :A :B 1.0 ] [ :A :B 5.0 ] ] [ :A :B ] graph! graph.bellmanford");
        assert!(b.contains("path=5.0"), "{b}");
        let p = top(":A [ [ :A :B 1.0 ] [ :A :B 5.0 ] ] [ :A :B ] graph! graph.paths");
        assert!(p.contains("path=5.0"), "{p}");
    }

    #[test]
    fn the_workbench_forms_hold_the_graph() {
        let mut i = match run_src(&format!("7 {DAG} . :A graph.bfs.")) {
            Ok(i) => i,
            Err(e) => panic!("{e}"),
        };
        assert_eq!(i.depth(), 1);
        assert_eq!(
            i.pull_workbench().map(|v| v.display()).unwrap_or_default(),
            l(&["A", "B", "C", "D", "E"])
        );
    }

    /// The stack beneath, then the workbench, or the refusal's own words.
    fn outcome(src: &str) -> Result<(Vec<String>, Vec<String>), String> {
        let mut i = run_src(src).map_err(|e| {
            e.rsplit_once(" returned error: ")
                .map_or(e.clone(), |(_, m)| m.to_string())
        })?;
        let stack = i.snapshot().iter().map(BundValue::display).collect();
        let mut bench = Vec::new();
        while let Some(v) = i.pull_workbench() {
            bench.push(v.display());
        }
        bench.reverse();
        Ok((stack, bench))
    }

    /// Every word, with the operands beneath its graph, bottom first.
    const FORMS: [(&str, &[&str]); 12] = [
        ("graph.bfs", &[":A"]),
        ("graph.dfs", &[":A"]),
        ("graph.reachable", &[":C"]),
        ("graph.neighbors", &[":A"]),
        ("graph.degree", &[":D"]),
        ("graph.bellmanford", &[":A"]),
        ("graph.maxflow", &[":E", ":A"]),
        ("graph.toposort", &[]),
        ("graph.cycle", &[]),
        ("graph.components", &[]),
        ("graph.scc", &[]),
        ("graph.mst", &[]),
    ];

    /// **Each `.` form holds the graph on the workbench and answers what its
    /// plain word answers, there**, leaving the stack beneath alone; and
    /// with a value that is not a graph, both refuse in the same words.
    /// Swapping the source and sink of `graph.maxflow.` would answer a
    /// different flow, so the slot is checked as well as the answer.
    #[test]
    fn each_form_holds_the_graph_on_the_workbench() {
        for (word, rest) in FORMS {
            let rest = rest.join(" ");
            let plain = outcome(&format!("7 {rest} {DAG} {word}"));
            let bench = outcome(&format!("7 {DAG} . {rest} {word}."));
            let (Ok((stack, wb)), Ok((bstack, bwb))) = (&plain, &bench) else {
                panic!("{word}: plain {plain:?}, workbench {bench:?}");
            };
            assert!(wb.is_empty(), "{word}");
            assert_eq!(stack.len(), 2, "{word}: one answer above the 7, {stack:?}");
            assert_eq!(stack[0], "7", "{word}: the stack beneath moved");
            assert_eq!(
                bwb.len(),
                1,
                "{word}.: one answer on the workbench, {bwb:?}"
            );
            assert_eq!(bstack, &["7"], "{word}.: the stack beneath moved");
            assert_eq!(&stack[1..], &bwb[..], "{word}.");

            let plain = outcome(&format!("7 {rest} dict {word}"));
            let bench = outcome(&format!("7 dict . {rest} {word}."));
            match (plain, bench) {
                (Err(a), Err(b)) => {
                    assert_eq!(a, b, "{word}");
                    assert!(
                        a.contains(&format!("{}: MISSED TYPE", word.to_uppercase())),
                        "{a}"
                    );
                }
                (a, b) => panic!("{word} on a dict: plain {a:?}, workbench {b:?}"),
            }
        }
    }

    #[test]
    fn each_refusal_section_s4_names() {
        let e = err_of(&format!("{DAG} graph.bfs"));
        assert!(e.contains("GRAPH.BFS: NO DATA #2"), "{e}");
        let e = err_of(&format!(":A {DAG} graph.maxflow"));
        assert!(e.contains("GRAPH.MAXFLOW: NO DATA #3"), "{e}");
        let e = err_of("[ [ :A :B 2 ] ] [ :A :B ] graph! graph.toposort");
        assert!(
            e.contains("GRAPH.TOPOSORT: an edge's weight must be a FLOAT, not Integer"),
            "{e}"
        );
        let e = err_of("[ [ :A :Z ] ] [ :A ] graph! graph.scc");
        assert!(
            e.contains("GRAPH.SCC: an edge names unknown nodes: A and Z"),
            "{e}"
        );
        let e = err_of("[ [ :A ] ] [ :A ] graph! graph.cycle");
        assert!(e.contains("GRAPH.CYCLE: an edge is too small"), "{e}");
        let e = err_of(":B :A [ [ :A :B -1.0 ] ] [ :A :B ] graph! graph.maxflow");
        assert!(
            e.contains("GRAPH.MAXFLOW: a capacity must not be below zero: A -> B weighs -1"),
            "{e}"
        );
        // Four elements weigh 100.0, as two do (`make_graph.rs:125-133`).
        let d = top(":B :A [ [ :A :B 1.0 :x ] ] [ :A :B ] graph! graph.maxflow");
        assert!(d.contains("flow=100.0"), "{d}");
        // The reader refuses a capacity that is not finite, so one that reached
        // the algorithm would be a defect, and is reported as one.
        let mut n = net(&(
            list_of(&["A", "B"]),
            BundValue::list(vec![BundValue::list(vec![
                BundValue::str("A"),
                BundValue::str("B"),
                BundValue::float(1.0),
            ])]),
        ));
        n.edges[0].2 = f64::INFINITY;
        let e = max_flow(&n, 0, 1, "TEST").map(|f| f.0);
        assert!(matches!(&e, Err(m) if m.is_internal()), "{e:?}");
    }

    /// **A graph too deep for a recursive search is answered.** A chain of
    /// 200,000 nodes, closed into one cycle: a search that recursed once per
    /// node would overflow a test thread's 2 MiB stack long before the end.
    #[test]
    fn a_deep_graph_does_not_overflow_the_stack() {
        // The stack is fixed here, not left to RUST_MIN_STACK.
        let run = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(deep_graph)
            .unwrap_or_else(|e| panic!("{e}"));
        if let Err(e) = run.join() {
            std::panic::resume_unwind(e);
        }
    }

    fn deep_graph() {
        const N: usize = 200_000;
        // Read from the MAP's two lists, as a word reads it.
        let chain = read(
            &BundValue::list((0..N).map(|k| BundValue::str(format!("N{k}"))).collect()),
            &BundValue::list(
                (1..N)
                    .map(|k| {
                        BundValue::list(vec![
                            BundValue::str(format!("N{}", k - 1)),
                            BundValue::str(format!("N{k}")),
                            BundValue::float(1.0),
                        ])
                    })
                    .collect(),
            ),
            "TEST",
        )
        .unwrap_or_else(|e| panic!("{}", e.0));
        let open = chain;
        assert_eq!(dfs(&open, 0).len(), N);
        assert_eq!(scc(&open).len(), N);
        assert_eq!(find_cycle(&open), None);
        assert_eq!(topo(&open).map(|o| o.len()), Ok(N));
        let mut closed = open;
        closed.edges.push((N - 1, 0, 1.0));
        closed.out[N - 1].push(0);
        assert_eq!(scc(&closed).len(), 1);
        assert_eq!(find_cycle(&closed).map(|c| c.len()), Some(N));
        assert_eq!(topo(&closed), Err(0));
    }

    /// **Every answer follows the `nodes` list, not the alphabet** (§S3).
    /// The nodes are listed in reverse, so an answer sorted by name, or in
    /// a map's order, fails.
    #[test]
    fn answers_follow_the_nodes_list() {
        let g = "[ [ :D :A 1.0 ] [ :D :B 1.0 ] [ :C :B 1.0 ] ] [ :D :C :B :A ] graph!";
        assert_eq!(top(&format!(":D {g} graph.neighbors")), l(&["B", "A"]));
        assert_eq!(top(&format!(":D {g} graph.bfs")), l(&["D", "B", "A"]));
        assert_eq!(top(&format!(":D {g} graph.dfs")), l(&["D", "B", "A"]));
        assert_eq!(top(&format!(":D {g} graph.reachable")), l(&["D", "B", "A"]));
        // Of the nodes free to go next, the earlier-listed goes first.
        assert_eq!(
            top(&format!("{g} graph.toposort")),
            l(&["D", "C", "B", "A"])
        );
        let rows = top(&format!(":D {g} graph.bellmanford"));
        let at: Vec<usize> = ["node=D", "node=C", "node=B", "node=A"]
            .iter()
            .map(|n| rows.find(n).unwrap_or_else(|| panic!("{n} in {rows}")))
            .collect();
        assert!(at.windows(2).all(|w| w[0] < w[1]), "{rows}");
        // C is not reached from D.
        assert!(rows.contains("NODATA"), "{rows}");

        // Groups by their first member, members in node order.
        let h = "[ [ :D :A ] [ :A :D ] [ :C :B ] ] [ :D :C :B :A ] graph!";
        assert_eq!(
            top(&format!("{h} graph.components")),
            ll(&[&["D", "A"], &["C", "B"]])
        );
        assert_eq!(
            top(&format!("{h} graph.scc")),
            ll(&[&["D", "A"], &["C"], &["B"]])
        );
        // The first cycle a search in node order closes.
        let c = "[ [ :B :A ] [ :A :B ] [ :C :D ] [ :D :C ] ] [ :D :C :B :A ] graph!";
        assert_eq!(top(&format!("{c} graph.cycle")), l(&["D", "C"]));
    }

    fn edges_of(rows: &[(&str, &str, f64)]) -> String {
        BundValue::list(
            rows.iter()
                .map(|(f, t, w)| {
                    BundValue::list(vec![
                        BundValue::str(*f),
                        BundValue::str(*t),
                        BundValue::float(*w),
                    ])
                })
                .collect(),
        )
        .display()
    }

    /// `graph.mst` takes equal weights in the order the graph lists its
    /// edges, a re-given edge keeping its **first** place with its last
    /// weight (`algos` would move it to the end); `graph.maxflow`'s `edges`
    /// follow that order too, not the order the flow found them; and
    /// `graph.mst` reads an edge both ways.
    #[test]
    fn ties_and_edges_follow_the_graph_order() {
        let m = top("[ [ :C :A 1.0 ] [ :A :B 1.0 ] [ :B :C 1.0 ] ] [ :A :B :C ] graph! graph.mst");
        assert!(
            m.contains(&format!(
                "edges={}",
                edges_of(&[("C", "A", 1.0), ("A", "B", 1.0)])
            )),
            "{m}"
        );
        let r = top(
            "[ [ :A :B 5.0 ] [ :C :A 1.0 ] [ :B :C 1.0 ] [ :A :B 1.0 ] ] [ :A :B :C ] graph! graph.mst",
        );
        assert!(
            r.contains(&format!(
                "edges={}",
                edges_of(&[("A", "B", 1.0), ("C", "A", 1.0)])
            )),
            "{r}"
        );
        // B -> A only, yet it joins A and B.
        let b = top("[ [ :B :A 2.0 ] ] [ :A :B ] graph! graph.mst");
        assert!(b.contains("weight=2.0"), "{b}");
        let f = top(
            ":T :S [ [ :M :T 1.0 ] [ :S :M 1.0 ] [ :S :X 9.0 ] ] [ :S :T :M :X ] graph! graph.maxflow",
        );
        assert!(
            f.contains(&format!(
                "edges={}",
                edges_of(&[("M", "T", 1.0), ("S", "M", 1.0)])
            )),
            "{f}"
        );
        // An unreachable sink is a flow of nothing, not an error.
        let z = top(":T :S [ [ :T :S 1.0 ] ] [ :S :T ] graph! graph.maxflow");
        assert!(z.contains("flow=0.0"), "{z}");
    }

    /// **The node operand of every word that takes one** is refused naming
    /// the word, its role and its number, and a malformed graph is refused
    /// before a node operand is looked at.
    #[test]
    fn every_node_operand_is_refused_naming_the_word() {
        let cases: [(&str, &str, &str); 7] = [
            ("graph.bfs", "", "start "),
            ("graph.dfs", "", "start "),
            ("graph.reachable", "", "start "),
            ("graph.neighbors", "", ""),
            ("graph.degree", "", ""),
            ("graph.bellmanford", "", "start "),
            ("graph.maxflow", ":E ", "source "),
        ];
        for (word, below, role) in cases {
            let w = word.to_uppercase();
            let e = err_of(&format!("{below}:Z {DAG} {word}"));
            assert!(e.contains(&format!("{w}: unknown {role}node: Z")), "{e}");
            let e = err_of(&format!("{below}1 {DAG} {word}"));
            assert!(
                e.contains(&format!("{w}: the {role}node must be a STRING, not")),
                "{e}"
            );
            let e = err_of(&format!("{below}:Z [ [ :A ] ] [ :A ] graph! {word}"));
            assert!(e.contains(&format!("{w}: an edge is too small")), "{e}");
        }
        let e = err_of(&format!(":Z :A {DAG} graph.maxflow"));
        assert!(e.contains("GRAPH.MAXFLOW: unknown sink node: Z"), "{e}");
        let e = err_of(&format!("1 :A {DAG} graph.maxflow"));
        assert!(
            e.contains("GRAPH.MAXFLOW: the sink node must be a STRING"),
            "{e}"
        );
        let e = err_of(&format!(":a :A {DAG} graph.maxflow"));
        assert!(
            e.contains("GRAPH.MAXFLOW: the source and the sink are one node: A"),
            "{e}"
        );
    }

    /// The graph-shape refusals come from the reference-worded
    /// `graph_operand`, for each of the twelve.
    #[test]
    fn a_malformed_graph_is_refused_in_the_reference_words() {
        for (word, rest) in FORMS {
            let rest = rest.join(" ");
            let w = word.to_uppercase();
            for (graph, says) in [
                ("dict :type :tree set", "unknown data type"),
                (
                    "dict :type 1 set",
                    "type casting returns: This Dynamic type is not string",
                ),
                (
                    "dict :type :graph set :nodes 1 set",
                    "Nodes list must be a list",
                ),
                (
                    "dict :type :graph set :edges 1 set",
                    "Edges list must be a list",
                ),
            ] {
                let e = err_of(&format!("{rest} {graph} {word}"));
                assert!(
                    e.contains(&format!("{w}: {says}")),
                    "{word} on {graph}: {e}"
                );
            }
        }
    }

    /// **A weight that is not finite is refused by every word** (D193),
    /// made as a program makes one: `math.sqrt` of a negative is NaN and an
    /// overflowing `*` is ±inf.
    #[test]
    fn a_weight_that_is_not_finite_is_refused() {
        for (make, shows) in [
            ("-1.0 math.sqrt", "NaN"),
            ("1.0e200 1.0e200 *", "inf"),
            ("1.0e200 -1.0e200 *", "-inf"),
        ] {
            let g = format!("{make} [ :A :B ] ++ list push [ :A :B ] graph!");
            for (word, rest) in FORMS {
                // Nodes of this two-node graph, as many as the word takes.
                let rest = [":B", ":A"][2 - rest.len()..].join(" ");
                let e = err_of(&format!("{rest} {g} {word}"));
                assert!(
                    e.contains(&format!(
                        "{}: an edge's weight must be finite: A -> B weighs {shows}",
                        word.to_uppercase()
                    )),
                    "{word} {make}: {e}"
                );
            }
        }
    }

    /// **`graph.maxflow` counts every capacity above zero**, however small
    /// beside the largest: the flow here is 1e13 + 5, exact in an f64.
    #[test]
    fn a_small_capacity_beside_a_huge_one_still_carries_flow() {
        let f = top(
            ":T :S [ [ :S :T 10000000000000.0 ] [ :S :M 5.0 ] [ :M :T 5.0 ] ] \
             [ :S :M :T ] graph! graph.maxflow",
        );
        assert!(f.contains("flow=10000000000005.0"), "{f}");
        assert!(
            f.contains(&format!(
                "edges={}",
                edges_of(&[
                    ("S", "T", 10000000000000.0),
                    ("S", "M", 5.0),
                    ("M", "T", 5.0)
                ])
            )),
            "{f}"
        );
    }

    /// The reader's refusals S4 lists, each naming a type by its name.
    #[test]
    fn every_reader_refusal_names_the_word_and_the_type() {
        let e = err_of("list [ 1 ] graph! graph.scc");
        assert!(
            e.contains("GRAPH.SCC: a node name must be a STRING, not Integer"),
            "{e}"
        );
        let e = err_of("[ 1 ] [ :A ] graph! graph.scc");
        assert!(
            e.contains("GRAPH.SCC: an edge must be a LIST, not Integer"),
            "{e}"
        );
        let e = err_of("[ [ 1 :A ] ] [ :A ] graph! graph.scc");
        assert!(
            e.contains("GRAPH.SCC: an edge's node must be a STRING"),
            "{e}"
        );
        let e = err_of(&format!("1 {DAG} graph.bfs"));
        assert!(
            e.contains("GRAPH.BFS: the start node must be a STRING, not Integer"),
            "{e}"
        );
        // One node as both ends is refused before any capacity is looked at.
        let e = err_of(":A :a [ [ :A :B -1.0 ] ] [ :A :B ] graph! graph.maxflow");
        assert!(
            e.contains("GRAPH.MAXFLOW: the source and the sink are one node: A"),
            "{e}"
        );
    }

    /// Run `src` to its end or its first refusal: the stack, the workbench,
    /// and the refusal if there was one.
    fn residue(src: &str) -> (Vec<String>, Vec<String>, Option<String>) {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).unwrap_or_else(|e| panic!("{}", e.render(src)));
        let err = i.eval(&stream).err().map(|e| e.0);
        let stack = i.snapshot().iter().map(BundValue::display).collect();
        let mut bench = Vec::new();
        while let Some(v) = i.pull_workbench() {
            bench.push(v.display());
        }
        bench.reverse();
        (stack, bench, err)
    }

    /// **What a refusal leaves** (§S4, RFC-0004's residual): the values a
    /// word pulled before it refused are gone, the rest are where they were,
    /// and a `.` form moves what its word left to the workbench.
    #[test]
    fn a_refusal_leaves_what_s4_says() {
        let cases: [(&str, &[&str], &[&str], &str); 7] = [
            (":Z DAG graph.bfs", &["7"], &[], "unknown start node: Z"),
            (
                ":E :Z DAG graph.maxflow",
                &["7", "E"],
                &[],
                "unknown source node: Z",
            ),
            // A malformed graph is refused before its node is pulled.
            (
                ":A [ [ :A ] ] [ :A ] graph! graph.bfs",
                &["7", "A"],
                &[],
                "an edge is too small",
            ),
            (
                "[ [ :A :B 2 ] ] [ :A :B ] graph! graph.toposort",
                &["7"],
                &[],
                "must be a FLOAT",
            ),
            ("DAG . :Z graph.bfs.", &["7"], &[], "unknown start node: Z"),
            (
                "DAG . :E :Z graph.maxflow.",
                &["7"],
                &["E"],
                "unknown source node: Z",
            ),
            // A `.` form checks both sides before it takes anything.
            (
                "DAG . graph.maxflow.",
                &["7"],
                &["{DAG}"],
                "Stack is too shallow for inline GRAPH.MAXFLOW.",
            ),
        ];
        let dag = top(DAG);
        for (src, stack, bench, says) in cases {
            let src = format!("7 {}", src.replace("DAG", DAG));
            let (st, wb, err) = residue(&src);
            let err = err.unwrap_or_else(|| panic!("{src} did not refuse"));
            assert!(err.contains(says), "{src}: {err}");
            assert_eq!(st, stack, "{src}: the stack");
            let bench: Vec<String> = bench
                .iter()
                .map(|b| {
                    if *b == "{DAG}" {
                        dag.clone()
                    } else {
                        b.to_string()
                    }
                })
                .collect();
            assert_eq!(wb, bench, "{src}: the workbench");
        }
        let (_, _, err) = residue(":A graph.bfs.");
        assert!(
            err.is_some_and(|e| e.contains("Workbench is too shallow for inline GRAPH.BFS.")),
            "an empty workbench"
        );
    }

    // ---- Cross-checks over seeded random graphs ----------------------------

    /// xorshift64, so the graphs are the same on every run.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n.max(1)
        }
    }

    const LETTERS: [&str; 7] = ["A", "B", "C", "D", "E", "F", "G"];

    /// A random graph of up to seven nodes, integral weights from `lo` up.
    fn random(rng: &mut Rng, lo: i64) -> (BundValue, BundValue) {
        let n = 1 + rng.below(7) as usize;
        let m = rng.below(13) as usize;
        let nodes: Vec<BundValue> = LETTERS[..n].iter().map(|s| BundValue::str(*s)).collect();
        let edges = (0..m)
            .map(|_| {
                let f = LETTERS[rng.below(n as u64) as usize];
                let t = LETTERS[rng.below(n as u64) as usize];
                let w = lo + rng.below(9) as i64;
                BundValue::list(vec![
                    BundValue::str(f),
                    BundValue::str(t),
                    BundValue::float(w as f64),
                ])
            })
            .collect();
        (BundValue::list(nodes), BundValue::list(edges))
    }

    fn graphs(seed: u64, count: usize, lo: i64) -> Vec<(BundValue, BundValue)> {
        let mut rng = Rng(seed);
        (0..count).map(|_| random(&mut rng, lo)).collect()
    }

    fn topo(n: &Net) -> Result<Vec<usize>, usize> {
        toposort(n).unwrap_or_else(|e| panic!("{}", e.0))
    }

    fn bf(n: &Net, s: usize) -> Result<Vec<Option<f64>>, usize> {
        bellman_ford(n, s).unwrap_or_else(|e| panic!("{}", e.0))
    }

    fn net(g: &(BundValue, BundValue)) -> Net {
        read(&g.0, &g.1, "TEST").unwrap_or_else(|e| panic!("{}", e.0))
    }

    /// Names that Title-casing merges or changes, so the node list holds
    /// duplicates once it is read.
    const NAMES: [&str; 8] = ["a", "A", "b", "my node", "my_node", "My Node", "x y", "X_Y"];

    /// A graph of anything the reader must take or refuse: names from
    /// `NAMES`, now and then an edge naming one the node list lacks; edges
    /// of one to four elements, and now and then a PAIR; weights
    /// fractional, negative, or now and then an INTEGER; edges given more
    /// than once; and, rarely, a node, an end or an edge of the wrong type.
    fn wide(rng: &mut Rng) -> (BundValue, BundValue) {
        let pick = |rng: &mut Rng| NAMES[rng.below(NAMES.len() as u64) as usize];
        let given: Vec<&str> = (0..1 + rng.below(7)).map(|_| pick(rng)).collect();
        // An edge's end is spelled as one of the given names, so that a
        // refusal is the exception: one in sixty is a name nothing gives.
        let end = |rng: &mut Rng| match rng.below(60) {
            0 => "zz",
            _ => given[rng.below(given.len() as u64) as usize],
        };
        let edges = (0..rng.below(13))
            .map(|_| {
                let ends = |rng: &mut Rng| match rng.below(80) {
                    0 => BundValue::int(1),
                    _ => BundValue::str(end(rng)),
                };
                if rng.below(80) == 0 {
                    return BundValue::int(2);
                }
                if rng.below(20) == 0 {
                    return BundValue::pair(ends(rng), ends(rng));
                }
                let mut e = vec![ends(rng), ends(rng)];
                match rng.below(60) {
                    0 => e.truncate(1),
                    1 => e.push(BundValue::int(3)),
                    2..=9 => {}
                    10..=15 => e.extend([BundValue::float(2.0), BundValue::str("x")]),
                    _ => e.push(BundValue::float((rng.below(41) as f64 - 20.0) / 4.0)),
                }
                BundValue::list(e)
            })
            .collect();
        let mut nodes: Vec<BundValue> = given.iter().map(|s| BundValue::str(*s)).collect();
        if rng.below(80) == 0 {
            nodes.push(BundValue::int(3));
        }
        (BundValue::list(nodes), BundValue::list(edges))
    }

    /// **The reader is `graph.paths`' reader**: it refuses what
    /// `build_algos` refuses, and otherwise numbers the nodes and keeps
    /// the edges and their weights as `build_algos` does, the last of a
    /// repeated edge winning. (`graph.path` reads differently — weights cut
    /// to whole numbers, the lowest of a repeated edge kept — and is not
    /// the model; §S2.)
    #[test]
    fn the_reader_agrees_with_build_algos() {
        let mut rng = Rng(1);
        let (mut taken, mut refused) = (0, 0);
        for _ in 0..2000 {
            let g = wide(&mut rng);
            let ours = read(&g.0, &g.1, "TEST");
            let theirs = crate::graph::build_algos(&g.0, &g.1);
            match (ours, theirs) {
                (Ok(n), Ok((ix, _, a))) => {
                    taken += 1;
                    assert_eq!(n.ix, ix);
                    assert_eq!(a.edges().count(), n.edges.len());
                    for &(f, t, w) in &n.edges {
                        assert_eq!(a.edge_weight(&f, &t), Some(w));
                    }
                }
                (Err(_), Err(_)) => refused += 1,
                (o, t) => panic!(
                    "{} {}: ours {:?}, build_algos {:?}",
                    g.0.display(),
                    g.1.display(),
                    o.map(|_| ()).map_err(|e| e.0),
                    t.map(|_| ()).map_err(|e| e.0)
                ),
            }
        }
        assert!(
            taken > 500 && refused > 100,
            "{taken} read, {refused} refused"
        );
    }

    #[test]
    fn reachable_is_the_transitive_closure_row() {
        for g in graphs(2, 300, 1) {
            let n = net(&g);
            let (_, _, a) =
                crate::graph::build_algos(&g.0, &g.1).unwrap_or_else(|e| panic!("{}", e.0));
            let closure = algos::cs::graph::warshall::transitive_closure(&a).unwrap_or_default();
            for s in 0..n.len() {
                let mut want: Vec<usize> = (0..n.len())
                    .filter(|t| closure.get(&(s, *t)).copied().unwrap_or(false))
                    .collect();
                want.sort_unstable();
                assert_eq!(reachable(&n, s), want, "{:?} from {s}", n.edges);
                let mut b = bfs(&n, s);
                let mut d = dfs(&n, s);
                b.sort_unstable();
                d.sort_unstable();
                assert_eq!(b, want);
                assert_eq!(d, want);
            }
        }
    }

    /// **Breadth first and depth-first preorder, as orders**: against a
    /// level-by-level search and a recursive one written out here, both
    /// taking successors in node order.
    #[test]
    fn traversal_orders_match_their_definitions() {
        fn preorder(n: &Net, u: usize, seen: &mut Vec<bool>, out: &mut Vec<usize>) {
            seen[u] = true;
            out.push(u);
            for &v in &n.out[u] {
                if !seen[v] {
                    preorder(n, v, seen, out);
                }
            }
        }
        for g in graphs(10, 300, 1) {
            let n = net(&g);
            for s in 0..n.len() {
                let mut seen = vec![false; n.len()];
                let mut want = Vec::new();
                preorder(&n, s, &mut seen, &mut want);
                assert_eq!(dfs(&n, s), want, "dfs {:?} from {s}", n.edges);

                let mut seen = vec![false; n.len()];
                seen[s] = true;
                let (mut want, mut level) = (vec![s], vec![s]);
                while !level.is_empty() {
                    let mut next = Vec::new();
                    for &u in &level {
                        for &v in &n.out[u] {
                            if !seen[v] {
                                seen[v] = true;
                                next.push(v);
                            }
                        }
                    }
                    want.extend(&next);
                    level = next;
                }
                assert_eq!(bfs(&n, s), want, "bfs {:?} from {s}", n.edges);
            }
        }
    }

    /// Two nodes share a strongly connected component exactly when each
    /// reaches the other; the components partition the nodes.
    #[test]
    fn scc_is_mutual_reachability() {
        for g in graphs(3, 300, 1) {
            let n = net(&g);
            let groups = scc(&n);
            let mut of = vec![usize::MAX; n.len()];
            for (k, grp) in groups.iter().enumerate() {
                for &u in grp {
                    assert_eq!(of[u], usize::MAX, "{u} in two components");
                    of[u] = k;
                }
            }
            let reach: Vec<Vec<usize>> = (0..n.len()).map(|s| reachable(&n, s)).collect();
            for u in 0..n.len() {
                for v in 0..n.len() {
                    let mutual = reach[u].contains(&v) && reach[v].contains(&u);
                    assert_eq!(of[u] == of[v], mutual, "{:?}: {u} {v}", n.edges);
                }
            }
        }
    }

    #[test]
    fn components_join_what_any_edge_joins() {
        for g in graphs(4, 300, 1) {
            let n = net(&g);
            let groups = components(&n);
            let mut of = vec![usize::MAX; n.len()];
            for (k, grp) in groups.iter().enumerate() {
                for &u in grp {
                    of[u] = k;
                }
            }
            assert!(of.iter().all(|k| *k != usize::MAX));
            for &(f, t, _) in &n.edges {
                assert_eq!(of[f], of[t]);
            }
            // Fewest groups: every group is connected through its edges.
            for grp in &groups {
                let mut seen = vec![grp[0]];
                let mut i = 0;
                while i < seen.len() {
                    let u = seen[i];
                    for &(f, t, _) in &n.edges {
                        for (a, b) in [(f, t), (t, f)] {
                            if a == u && !seen.contains(&b) {
                                seen.push(b);
                            }
                        }
                    }
                    i += 1;
                }
                assert_eq!(seen.len(), grp.len());
            }
        }
    }

    /// A topological order respects every edge, and exists exactly when no
    /// cycle is found; a cycle found is a cycle.
    #[test]
    fn toposort_and_cycle_agree() {
        for g in graphs(5, 400, 1) {
            let n = net(&g);
            match (topo(&n), find_cycle(&n)) {
                (Ok(order), None) => {
                    let mut at = vec![0; n.len()];
                    for (k, u) in order.iter().enumerate() {
                        at[*u] = k;
                    }
                    assert_eq!(order.len(), n.len());
                    for &(f, t, _) in &n.edges {
                        assert!(at[f] < at[t], "{:?}", n.edges);
                    }
                }
                (Err(through), Some(c)) => {
                    assert!(c.contains(&through));
                    for k in 0..c.len() {
                        let (u, v) = (c[k], c[(k + 1) % c.len()]);
                        assert!(
                            n.out[u].contains(&v),
                            "{c:?} is not a cycle of {:?}",
                            n.edges
                        );
                    }
                }
                (t, c) => panic!("{:?}: toposort {t:?}, cycle {c:?}", n.edges),
            }
        }
    }

    /// With no negative weight, Bellman–Ford answers what `graph.paths`'
    /// Dijkstra does.
    #[test]
    fn bellman_ford_agrees_with_dijkstra() {
        for g in graphs(6, 300, 1) {
            let n = net(&g);
            let (_, _, a) =
                crate::graph::build_algos(&g.0, &g.1).unwrap_or_else(|e| panic!("{}", e.0));
            for s in 0..n.len() {
                let want = algos::cs::graph::dijkstra::shortest_paths(&a, &s).unwrap_or_default();
                let got = bf(&n, s).unwrap_or_else(|u| panic!("cycle through {u}"));
                for (t, d) in got.iter().enumerate() {
                    assert_eq!(
                        *d,
                        want.get(&t).copied().flatten(),
                        "{:?} {s}->{t}",
                        n.edges
                    );
                }
            }
        }
    }

    /// With negative weights, against Floyd–Warshall written out here: equal
    /// distances when no negative cycle is reachable, and a reported node
    /// that is on one when it is.
    #[test]
    fn bellman_ford_agrees_with_floyd_warshall() {
        for g in graphs(7, 400, -3) {
            let n = net(&g);
            let k = n.len();
            let mut d = vec![vec![f64::INFINITY; k]; k];
            for (u, row) in d.iter_mut().enumerate() {
                row[u] = 0.0;
            }
            for &(f, t, w) in &n.edges {
                d[f][t] = d[f][t].min(w);
            }
            for m in 0..k {
                for u in 0..k {
                    for v in 0..k {
                        if d[u][m] + d[m][v] < d[u][v] {
                            d[u][v] = d[u][m] + d[m][v];
                        }
                    }
                }
            }
            for s in 0..k {
                let negative = (0..k).any(|v| d[s][v].is_finite() && d[v][v] < 0.0);
                match bf(&n, s) {
                    Ok(got) => {
                        assert!(!negative, "{:?} from {s}: missed a cycle", n.edges);
                        for (t, x) in got.iter().enumerate() {
                            let want = d[s][t].is_finite().then_some(d[s][t]);
                            assert_eq!(*x, want, "{:?} {s}->{t}", n.edges);
                        }
                    }
                    Err(u) => {
                        assert!(negative, "{:?} from {s}: no cycle", n.edges);
                        assert!(d[u][u] < 0.0, "{u} is not on a negative cycle");
                        assert!(d[s][u].is_finite(), "{s} does not reach {u}");
                        // The smallest node on its cycle: a negative cycle
                        // through `u` exists among nodes numbered `u` and up.
                        let mut e = vec![vec![f64::INFINITY; k]; k];
                        for &(f, t, w) in &n.edges {
                            if f >= u && t >= u {
                                e[f][t] = e[f][t].min(w);
                            }
                        }
                        for m in u..k {
                            for a in u..k {
                                for b in u..k {
                                    if e[a][m] + e[m][b] < e[a][b] {
                                        e[a][b] = e[a][m] + e[m][b];
                                    }
                                }
                            }
                        }
                        assert!(e[u][u] < 0.0, "{:?}: {u} is not the least", n.edges);
                    }
                }
            }
        }
    }

    /// The forest's weight is the least of every forest with as many edges,
    /// found by trying every subset of up to eight edges.
    #[test]
    fn mst_is_minimal() {
        let mut tried = 0;
        for g in graphs(8, 200, -2) {
            let n = net(&g);
            let e: Vec<(usize, usize, f64)> =
                n.edges.iter().copied().filter(|(f, t, _)| f != t).collect();
            let chosen = mst(&n);
            // Lightest first, equal weights in the graph's order.
            let place = |x: &(usize, usize, f64)| n.edges.iter().position(|y| y == x);
            for w in chosen.windows(2) {
                assert!(
                    w[0].2 < w[1].2 || (w[0].2 == w[1].2 && place(&w[0]) < place(&w[1])),
                    "{:?}: {chosen:?}",
                    n.edges
                );
            }
            if e.len() > 8 {
                continue;
            }
            tried += 1;
            let size = n.len() - components(&n).len();
            assert_eq!(chosen.len(), size);
            let got: f64 = chosen.iter().map(|x| x.2).sum();
            let mut best = f64::INFINITY;
            for mask in 0u32..(1 << e.len()) {
                if mask.count_ones() as usize != size {
                    continue;
                }
                let mut parent: Vec<usize> = (0..n.len()).collect();
                let mut ok = true;
                let mut w = 0.0;
                for (i, &(f, t, x)) in e.iter().enumerate() {
                    if mask & (1 << i) != 0 {
                        let (a, b) = (root(&mut parent, f), root(&mut parent, t));
                        if a == b {
                            ok = false;
                            break;
                        }
                        parent[a.max(b)] = a.min(b);
                        w += x;
                    }
                }
                if ok {
                    best = best.min(w);
                }
            }
            assert_eq!(got, best, "{:?}", n.edges);
        }
        assert!(tried >= 100, "only {tried} of 200 graphs were small enough");
    }

    /// Capacities across thirteen orders of magnitude, every one a whole
    /// number so that every sum is exact in an f64: the flow equals the least
    /// cut exactly, for every source and sink. A tolerance relative to the
    /// largest capacity would lose the small ones.
    #[test]
    fn max_flow_is_the_min_cut_across_magnitudes() {
        let mut rng = Rng(11);
        for _ in 0..300 {
            let n = 2 + rng.below(6) as usize;
            let nodes: Vec<BundValue> = LETTERS[..n].iter().map(|s| BundValue::str(*s)).collect();
            let edges: Vec<BundValue> = (0..rng.below(13))
                .map(|_| {
                    let w = (1 + rng.below(9)) as f64 * 10f64.powi(rng.below(14) as i32);
                    BundValue::list(vec![
                        BundValue::str(LETTERS[rng.below(n as u64) as usize]),
                        BundValue::str(LETTERS[rng.below(n as u64) as usize]),
                        BundValue::float(w),
                    ])
                })
                .collect();
            let g = net(&(BundValue::list(nodes), BundValue::list(edges)));
            for s in 0..n {
                for t in (0..n).filter(|t| *t != s) {
                    let (flow, _) =
                        max_flow(&g, s, t, "TEST").unwrap_or_else(|e| panic!("{}", e.0));
                    let mut cut = f64::INFINITY;
                    for mask in 0u32..(1 << n) {
                        if mask & (1 << s) == 0 || mask & (1 << t) != 0 {
                            continue;
                        }
                        let c: f64 = g
                            .edges
                            .iter()
                            .filter(|(f, to, _)| mask & (1 << f) != 0 && mask & (1 << to) == 0)
                            .map(|e| e.2)
                            .sum();
                        cut = cut.min(c);
                    }
                    assert_eq!(flow, cut, "{:?} {s}->{t}", g.edges);
                }
            }
        }
    }

    /// For every source and sink, the flow equals the least cut, found by
    /// trying every set of nodes holding the source and not the sink; and each edge's share is within
    /// its capacity and the flow is conserved at every other node.
    #[test]
    fn max_flow_is_the_min_cut() {
        for g in graphs(9, 300, 0) {
            let n = net(&g);
            let k = n.len();
            if k < 2 {
                continue;
            }
            for (s, t) in (0..k).flat_map(|s| (0..k).map(move |t| (s, t))) {
                if s == t {
                    continue;
                }
                let (flow, shares) =
                    max_flow(&n, s, t, "TEST").unwrap_or_else(|e| panic!("{}", e.0));
                let mut cut = f64::INFINITY;
                for mask in 0u32..(1 << k) {
                    if mask & (1 << s) == 0 || mask & (1 << t) != 0 {
                        continue;
                    }
                    let c: f64 = n
                        .edges
                        .iter()
                        .filter(|(f, to, _)| mask & (1 << f) != 0 && mask & (1 << to) == 0)
                        .map(|e| e.2)
                        .sum();
                    cut = cut.min(c);
                }
                assert!(
                    (flow - cut).abs() < 1e-9,
                    "{:?}: flow {flow}, cut {cut}",
                    n.edges
                );
                let mut net_out = vec![0.0f64; k];
                for &(f, to, x) in &shares {
                    let cap = n.edges.iter().find(|e| e.0 == f && e.1 == to).map(|e| e.2);
                    assert!(cap.is_some_and(|c| x <= c + 1e-9));
                    net_out[f] += x;
                    net_out[to] -= x;
                }
                for (u, b) in net_out.iter().enumerate() {
                    if u != s && u != t {
                        assert!(b.abs() < 1e-9, "{:?}: not conserved at {u}", n.edges);
                    }
                }
            }
        }
    }
}
