//! `graph!` and `graph.path` — the two the corpus reaches.
//!
//! A graph is **not a tag**. It is an ordinary MAP with `type = "graph"`,
//! `nodes` and `edges` (`reference/Bund/src/stdlib/functions/graph/mod.rs:87-91`),
//! so every graph word begins by re-reading those three keys and re-deriving
//! its own representation. Nothing is cached between calls, and a program can
//! build one with `dict` and `set` without going near `graph!`.
//!
//! **`graph!` needs no library at all** — it is three `set`s. The algorithms
//! do: `graph.path` runs `fast_paths`, while `graph.paths`,
//! `graph.allpath` and `graph.transitiveclosure` run `algos`' dijkstra,
//! johnson and warshall. Only the first is implemented, because only the first
//! is reached by a golden — which is also why D38's `algos` pin is not what
//! unblocks these two programs.

use std::collections::HashMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST};
use convert_case::{Case, Casing};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// Node names are **Title-cased before anything looks them up**
/// (`reference/Bund/src/stdlib/functions/graph/make_graph.rs:9-11`).
///
/// So `:A` stays `A`, but `:my_node` becomes `My Node` and `:AB` becomes `Ab`.
/// The renaming applies to the node list and to both ends of every edge, and
/// it is what the returned path holds — a program gets Title-cased names back
/// whatever it put in.
fn node_name(s: &str) -> String {
    s.trim().to_case(Case::Title)
}

/// `graph!` — build the graph MAP from two lists already on the stack
/// (`reference/Bund/src/stdlib/functions/graph/mod.rs:68-93`).
///
/// **It never fails.** A pulled value that is not a LIST is *pushed back* and
/// an empty list used in its place (`:71-75`), and an empty stack yields an
/// empty list too (`:76`). So `graph!` on rubbish leaves the rubbish where it
/// was and answers an empty graph, which is why the word takes `!` — it is the
/// forgiving constructor beside `graph`, which validates and reports.
///
/// Nodes are pulled **first**, edges second, so the source reads
/// `<edges> <nodes> graph!`.
fn graph_bang(vm: &mut dyn Vm) -> Result<(), Error> {
    let nodes = match vm.pull() {
        Some(v) if v.dt() == LIST => v,
        Some(v) => {
            vm.push(v);
            BundValue::list(Vec::new())
        }
        None => BundValue::list(Vec::new()),
    };
    let edges = match vm.pull() {
        Some(v) if v.dt() == LIST => v,
        Some(v) => {
            vm.push(v);
            BundValue::list(Vec::new())
        }
        None => BundValue::list(Vec::new()),
    };
    let res = BundValue::map(Default::default())
        .set("type", BundValue::str("graph"))
        .set("nodes", nodes)
        .set("edges", edges);
    vm.push(res);
    Ok(())
}

/// A node index, its inverse, and the loaded graph — what `build` hands back.
type Built = (
    HashMap<String, usize>,
    HashMap<usize, String>,
    fast_paths::InputGraph,
);

/// Index the nodes and load the edges into a `fast_paths::InputGraph`
/// (`make_graph.rs:13-74`).
///
/// Three details that decide the answer:
///
/// - A node is indexed on **first sight** and duplicates are skipped
///   (`:27-31`), so the index is the order of the `nodes` list.
/// - An edge of two elements gets weight **100**; three elements take the
///   third, cast to float and then **truncated to `usize`** (`:59-66`). So a
///   weight of `1.0` is 1 and `1.9` is also 1 — the fractional part is
///   discarded, and a weight below 1.0 becomes 0.
/// - An edge naming an unknown node is an error, not a silently added node
///   (`:47-49`).
fn build(nodes: &BundValue, edges: &BundValue) -> Result<Built, Error> {
    let mut ix: HashMap<String, usize> = HashMap::new();
    let mut names: HashMap<usize, String> = HashMap::new();
    let mut g = fast_paths::InputGraph::new();

    let node_list = nodes
        .as_list()
        .ok_or_else(|| Error("MAKE_FAST_GRAPH: error casting nodes list".into()))?
        .to_vec();
    for (n, name) in node_list.iter().filter_map(|n| n.as_str().map(|s| (n, s))) {
        let _ = n;
        let name = node_name(&name);
        if !ix.contains_key(&name) {
            let c = ix.len();
            ix.insert(name.clone(), c);
            names.insert(c, name);
        }
    }

    let edge_list = edges
        .as_list()
        .ok_or_else(|| Error("MAKE_FAST_GRAPH: error casting edges list".into()))?
        .to_vec();
    for e in edge_list {
        let rel = e
            .as_list()
            .ok_or_else(|| Error("MAKE_FAST_GRAPH: error casting relation".into()))?
            .to_vec();
        if rel.len() < 2 {
            return Err(Error("MAKE_FAST_GRAPH: relation is too small".into()));
        }
        let from = rel
            .first()
            .and_then(BundValue::as_str)
            .map(|s| node_name(&s))
            .ok_or_else(|| Error("MAKE_FAST_GRAPH: error casting from node name".into()))?;
        let to = rel
            .get(1)
            .and_then(BundValue::as_str)
            .map(|s| node_name(&s))
            .ok_or_else(|| Error("NAME_GRAPH: error casting from node name".into()))?;
        let (Some(&from_ix), Some(&to_ix)) = (ix.get(&from), ix.get(&to)) else {
            return Err(Error(format!(
                "MAKE_FAST_GRAPH: {from} and {to} unknown nodes"
            )));
        };
        let w = match rel.get(2) {
            Some(w) => match *w.unboxed() {
                BundValue::Float(f, _) => f as usize,
                _ => return Err(Error("MAKE_FAST_GRAPH: Error casting weight".into())),
            },
            None => 100,
        };
        g.add_edge(from_ix, to_ix, w);
    }
    g.freeze();
    Ok((ix, names, g))
}

/// `graph.path` — shortest path between two nodes
/// (`reference/Bund/src/stdlib/functions/graph/getpath.rs:13-93`).
///
/// Operand order is the graph, then the **start**, then the **end** (`:47`,
/// `:60`), all pulled — so the source reads `<end> <start> … graph.path`, and
/// the golden's `:C :A [edges] [nodes] graph!` puts `C` deepest as the end.
///
/// The answer is a MAP: `found` alone when there is no path (`:76`), or
/// `path`, `weight` and `found` when there is (`:87-89`). `weight` is pushed
/// as a **FLOAT** even though `fast_paths` computes in `usize` (`:88`), which
/// is why the golden prints `Weight: 3.0` and not `3`.
fn graph_path(vm: &mut dyn Vm) -> Result<(), Error> {
    let g = vm.pull().ok_or_else(|| Error("GRAPH.PATH: NO DATA #1".into()))?;
    let ty = g
        .get("type")
        .ok_or_else(|| Error("GRAPH.PATH: MISSED TYPE".into()))?;
    match ty.as_str().as_deref() {
        Some("graph") => {}
        _ => return Err(Error("GRAPH.PATH: unknown data type".into())),
    }
    let nodes = g.get("nodes").unwrap_or_else(|| BundValue::list(Vec::new()));
    let edges = g.get("edges").unwrap_or_else(|| BundValue::list(Vec::new()));
    if nodes.dt() != LIST {
        return Err(Error("GRAPH.PATH: Nodes list must be a list".into()));
    }
    if edges.dt() != LIST {
        return Err(Error("GRAPH.PATH: Edges list must be a list".into()));
    }
    let (ix, names, input) = build(&nodes, &edges)?;
    let prepared = fast_paths::prepare(&input);

    let start_val = vm.pull().ok_or_else(|| Error("GRAPH.PATH: NO DATA #2".into()))?;
    let start = start_val
        .as_str()
        .map(|s| node_name(&s))
        .ok_or_else(|| Error("GRAPH.PATH: casting start node name returns".into()))?;
    let Some(&start_ix) = ix.get(&start) else {
        return Err(Error(format!("GRAPH.PATH: unknown start node: {start}")));
    };
    let end_val = vm.pull().ok_or_else(|| Error("GRAPH.PATH: NO DATA #3".into()))?;
    let end = end_val
        .as_str()
        .map(|s| node_name(&s))
        .ok_or_else(|| Error("GRAPH.PATH: casting start node name returns".into()))?;
    let Some(&end_ix) = ix.get(&end) else {
        return Err(Error(format!("GRAPH.PATH: unknown end node: {start}")));
    };

    let Some(found) = fast_paths::calc_path(&prepared, start_ix, end_ix) else {
        return Err(Error("GRAPH.PATH: can not calculate path".into()));
    };
    let mut res = BundValue::map(Default::default());
    if !found.is_found() {
        res = res.set("found", BundValue::boolean(false));
    } else {
        let mut p: Vec<BundValue> = Vec::new();
        for n in found.get_nodes() {
            let name = names
                .get(n)
                .ok_or_else(|| Error(format!("GRAPH.PATH: unknown node {n}")))?;
            p.push(BundValue::str(name.clone()));
        }
        res = res
            .set("path", BundValue::list(p))
            .set("weight", BundValue::float(found.get_weight() as f64))
            .set("found", BundValue::boolean(true));
    }
    vm.push(res);
    Ok(())
}

pub fn register(r: &mut Registry) {
    // Opaque: it pushes back what is not a LIST, so it consumes 0 or 2
    // depending on the tags it is handed (`mod.rs:68-93`).
    r.register_native("graph!", graph_bang, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("graph.path", graph_path, eff(3, 1), WordKind::Sync);
}
