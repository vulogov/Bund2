//! The graph family: `graph!`, `graph` and `graph.`, which build or check the
//! MAP, and `graph.path`, `graph.paths`, `graph.allpath` and
//! `graph.transitiveclosure`, which run algorithms over it.
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
//! johnson and warshall, each from the crate and revision the reference links
//! (D38). Those three build their answer by iterating a `HashMap`, so **the
//! order of the rows changes from run to run**, in the reference and here alike.
//! A golden can capture a count or a single row, but not the list as printed.

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
    for n in node_list {
        // A node that is not a string is an error (`:23-26`), not skipped.
        let name = n.as_str().map(|s| node_name(&s)).ok_or_else(|| {
            Error("MAKE_FAST_GRAPH: error casting node name: This Dynamic type is not string".into())
        })?;
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
            .ok_or_else(|| {
                Error(format!(
                    "MAKE_FAST_GRAPH: error casting relation: This is not a LIST/PAIR value but {}",
                    e.dt()
                ))
            })?
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
        // Only an edge of exactly three takes its weight (`:59`); four or more
        // fall back to 100, the extras ignored.
        let w = match rel.get(2) {
            Some(w) if rel.len() == 3 => match *w.unboxed() {
                BundValue::Float(f, _) => f as usize,
                _ => {
                    return Err(Error(format!(
                        "MAKE_FAST_GRAPH: Error casting weight: This Dynamic type is not float: {}",
                        w.dt()
                    )))
                }
            },
            _ => 100,
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

/// `graph` and `graph.` — take a MAP as a graph and fill in its defaults
/// (`reference/Bund/src/stdlib/functions/graph/mod.rs:14-65,95-103`).
///
/// `type` defaults to `"graph"`, and `nodes` and `edges` to empty lists; both
/// must then be LISTs. Unlike `graph!`, which builds from two lists and never
/// fails, this validates a value it is handed. The `.` form takes and answers
/// on the workbench.
fn graph_word(vm: &mut dyn Vm) -> Result<(), Error> {
    graph_base(vm, crate::wb::Side::Stack, "GRAPH")
}

fn graph_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    graph_base(vm, crate::wb::Side::Bench, "GRAPH.")
}

fn graph_base(vm: &mut dyn Vm, side: crate::wb::Side, prefix: &str) -> Result<(), Error> {
    match side {
        crate::wb::Side::Stack if vm.depth() < 1 => {
            return Err(Error(format!("Stack is too shallow for inline {prefix}")));
        }
        crate::wb::Side::Bench if vm.workbench_depth() < 1 => {
            return Err(Error(format!("Workbench is too shallow for inline {prefix}")));
        }
        _ => {}
    }
    let g = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix}: NO DATA #1")))?;
    let ty = g.get("type").unwrap_or_else(|| BundValue::str("graph"));
    let nodes = g.get("nodes").unwrap_or_else(|| BundValue::list(Vec::new()));
    let edges = g.get("edges").unwrap_or_else(|| BundValue::list(Vec::new()));
    if nodes.dt() != LIST {
        return Err(Error(format!("{prefix}: Nodes list must be a list")));
    }
    if edges.dt() != LIST {
        return Err(Error(format!("{prefix}: Edges list must be a list")));
    }
    side.push(vm, g.set("type", ty).set("nodes", nodes).set("edges", edges));
    Ok(())
}

/// A node index, its inverse, and the graph `algos` runs on.
type AlgosBuilt = (
    HashMap<String, usize>,
    HashMap<usize, String>,
    algos::cs::graph::Graph<usize, f64>,
);

/// Index the nodes and load the edges into a directed `algos` graph
/// (`reference/Bund/src/stdlib/functions/graph/make_graph.rs:78-141`).
///
/// The same rules as `build`, under the prefix `MAKE_GRAPH`, except that a
/// weight stays an `f64` (default 100.0) instead of being truncated
/// (`:125-133`). Every node is also added as a vertex (`:95`), so a node with
/// no edges still appears in the answers.
fn build_algos(nodes: &BundValue, edges: &BundValue) -> Result<AlgosBuilt, Error> {
    let mut ix: HashMap<String, usize> = HashMap::new();
    let mut names: HashMap<usize, String> = HashMap::new();
    let mut g: algos::cs::graph::Graph<usize, f64> = algos::cs::graph::Graph::new();
    let node_list = nodes
        .as_list()
        .ok_or_else(|| Error("MAKE_GRAPH: error casting nodes list".into()))?
        .to_vec();
    for n in node_list {
        let name = n.as_str().map(|s| node_name(&s)).ok_or_else(|| {
            Error("MAKE_GRAPH: error casting node name: This Dynamic type is not string".into())
        })?;
        if !ix.contains_key(&name) {
            let c = ix.len();
            ix.insert(name.clone(), c);
            names.insert(c, name);
            g.add_vertex(c);
        }
    }
    let edge_list = edges
        .as_list()
        .ok_or_else(|| Error("MAKE_GRAPH: error casting edges list".into()))?
        .to_vec();
    for e in edge_list {
        let rel = e
            .as_list()
            .ok_or_else(|| {
                Error(format!(
                    "MAKE_GRAPH: error casting relation: This is not a LIST/PAIR value but {}",
                    e.dt()
                ))
            })?
            .to_vec();
        if rel.len() < 2 {
            return Err(Error("MAKE_GRAPH: relation is too small".into()));
        }
        let from = rel
            .first()
            .and_then(BundValue::as_str)
            .map(|s| node_name(&s))
            .ok_or_else(|| {
                Error("NAME_GRAPH: error casting from node name: This Dynamic type is not string".into())
            })?;
        let to = rel
            .get(1)
            .and_then(BundValue::as_str)
            .map(|s| node_name(&s))
            .ok_or_else(|| {
                Error("MAKE_GRAPH: error casting from node name: This Dynamic type is not string".into())
            })?;
        let (Some(&from_ix), Some(&to_ix)) = (ix.get(&from), ix.get(&to)) else {
            return Err(Error(format!("MAKE_GRAPH: {from} and {to} unknown nodes")));
        };
        let w = match rel.get(2) {
            Some(w) if rel.len() == 3 => match *w.unboxed() {
                BundValue::Float(f, _) => f,
                _ => {
                    return Err(Error(format!(
                        "MAKE_GRAPH: Error casting weight: This Dynamic type is not float: {}",
                        w.dt()
                    )))
                }
            },
            _ => 100.0,
        };
        g.add_edge(from_ix, to_ix, w);
    }
    Ok((ix, names, g))
}

/// The prologue the three `algos` words share: pull the graph, check its
/// `type`, and default and check `nodes` and `edges`
/// (`dijkstra.rs`, `allshortpath.rs` and `transitiveclosure.rs` each repeat it).
fn graph_operand(vm: &mut dyn Vm, prefix: &str) -> Result<(BundValue, BundValue), Error> {
    let g = vm
        .pull()
        .ok_or_else(|| Error(format!("{prefix}: NO DATA #1")))?;
    let ty = g
        .get("type")
        .ok_or_else(|| Error(format!("{prefix}: MISSED TYPE")))?;
    let Some(ty) = ty.as_str() else {
        return Err(Error(format!(
            "{prefix}: type casting returns: This Dynamic type is not string"
        )));
    };
    if ty != "graph" {
        return Err(Error(format!("{prefix}: unknown data type")));
    }
    let nodes = g.get("nodes").unwrap_or_else(|| BundValue::list(Vec::new()));
    let edges = g.get("edges").unwrap_or_else(|| BundValue::list(Vec::new()));
    if nodes.dt() != LIST {
        return Err(Error(format!("{prefix}: Nodes list must be a list")));
    }
    if edges.dt() != LIST {
        return Err(Error(format!("{prefix}: Edges list must be a list")));
    }
    Ok((nodes, edges))
}

/// `graph.paths` — shortest distances from one node to every other, by
/// `algos`' Dijkstra (`reference/Bund/src/stdlib/functions/graph/dijkstra.rs`).
///
/// The graph is on top and the start node beneath it. **The start node is not
/// Title-cased**, unlike every node name in the graph, so `:a` finds nothing
/// where `"A"` finds `A`. Each row is `node` and `path`, the distance or NODATA.
/// The rows come in `HashMap` order, which changes from run to run, as the
/// reference's do.
fn graph_paths(vm: &mut dyn Vm) -> Result<(), Error> {
    let (nodes, edges) = graph_operand(vm, "GRAPH.PATHS")?;
    let (ix, names, g) = build_algos(&nodes, &edges)?;
    let start_val = vm
        .pull()
        .ok_or_else(|| Error("GRAPH.PATHS: NO DATA #2".into()))?;
    let Some(start) = start_val.as_str() else {
        return Err(Error(
            "GRAPH.PATHS: casting start node name returns: This Dynamic type is not string".into(),
        ));
    };
    let Some(start_ix) = ix.get(&start) else {
        return Err(Error(format!("GRAPH.PATHS: unknown start node: {start}")));
    };
    let found = algos::cs::graph::dijkstra::shortest_paths(&g, start_ix)
        .map_err(|e| Error(format!("GRAPH.PATHS: returned: {e}")))?;
    let mut rows: Vec<BundValue> = Vec::new();
    for (k, v) in found {
        let name = names
            .get(&k)
            .ok_or_else(|| Error(format!("GRAPH.PATHS: unknown node {k}")))?;
        let path = v.map_or_else(BundValue::nodata, BundValue::float);
        rows.push(
            BundValue::map(Default::default())
                .set("node", BundValue::str(name.clone()))
                .set("path", path),
        );
    }
    vm.push(BundValue::list(rows));
    Ok(())
}

/// `graph.allpath` — every pair's shortest distance, by `algos`' Johnson
/// (`reference/Bund/src/stdlib/functions/graph/allshortpath.rs`).
///
/// A pair with no path is left out. Each row is `weight`, `from`, `to` and
/// `itself`. The unknown-node errors say `GRAPH.TRANSITIVECLOSURE`, as the
/// reference's copy of that file does.
fn graph_allpath(vm: &mut dyn Vm) -> Result<(), Error> {
    let (nodes, edges) = graph_operand(vm, "GRAPH.ALLPATH")?;
    let (_, names, g) = build_algos(&nodes, &edges)?;
    let found = algos::cs::graph::johnson::all_pairs_shortest_paths(&g)
        .map_err(|e| Error(format!("GRAPH.ALLPATH: returned: {e}")))?;
    let mut rows: Vec<BundValue> = Vec::new();
    for ((from_ix, to_ix), w) in found {
        let Some(w) = w else { continue };
        let from = names
            .get(&from_ix)
            .ok_or_else(|| Error(format!("GRAPH.TRANSITIVECLOSURE: unknown node {from_ix}")))?;
        let to = names
            .get(&to_ix)
            .ok_or_else(|| Error(format!("GRAPH.TRANSITIVECLOSURE: unknown node {to_ix}")))?;
        rows.push(
            BundValue::map(Default::default())
                .set("weight", BundValue::float(w))
                .set("from", BundValue::str(from.clone()))
                .set("to", BundValue::str(to.clone()))
                .set("itself", BundValue::boolean(from_ix == to_ix)),
        );
    }
    vm.push(BundValue::list(rows));
    Ok(())
}

/// `graph.transitiveclosure` — every pair connected by some path, by `algos`'
/// Warshall (`reference/Bund/src/stdlib/functions/graph/transitiveclosure.rs`).
/// Each row is `from` and `to`.
fn graph_transitiveclosure(vm: &mut dyn Vm) -> Result<(), Error> {
    let (nodes, edges) = graph_operand(vm, "GRAPH.TRANSITIVECLOSURE")?;
    let (_, names, g) = build_algos(&nodes, &edges)?;
    let found = algos::cs::graph::warshall::transitive_closure(&g)
        .map_err(|e| Error(format!("GRAPH.TRANSITIVECLOSURE: returned: {e}")))?;
    let mut rows: Vec<BundValue> = Vec::new();
    for ((from_ix, to_ix), connected) in found {
        if !connected {
            continue;
        }
        let from = names
            .get(&from_ix)
            .ok_or_else(|| Error(format!("GRAPH.TRANSITIVECLOSURE: unknown node {from_ix}")))?;
        let to = names
            .get(&to_ix)
            .ok_or_else(|| Error(format!("GRAPH.TRANSITIVECLOSURE: unknown node {to_ix}")))?;
        rows.push(
            BundValue::map(Default::default())
                .set("from", BundValue::str(from.clone()))
                .set("to", BundValue::str(to.clone())),
        );
    }
    vm.push(BundValue::list(rows));
    Ok(())
}

pub fn register(r: &mut Registry) {
    // `reference/Bund/src/stdlib/functions/graph/mod.rs:113-114`; the `.`
    // form takes and answers on the workbench.
    r.register_native("graph", graph_word, eff(1, 1), WordKind::Sync);
    r.register_native("graph.", graph_wb, eff(0, 0), WordKind::Sync);
    r.register_native("graph.paths", graph_paths, eff(2, 1), WordKind::Sync);
    r.register_native("graph.allpath", graph_allpath, eff(1, 1), WordKind::Sync);
    r.register_native(
        "graph.transitiveclosure",
        graph_transitiveclosure,
        eff(1, 1),
        WordKind::Sync,
    );
    // Opaque: it pushes back what is not a LIST, so it consumes 0 or 2
    // depending on the tags it is handed (`mod.rs:68-93`).
    r.register_native("graph!", graph_bang, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("graph.path", graph_path, eff(3, 1), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;

    fn run_src(src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    fn err_of(src: &str) -> String {
        match run_src(src) {
            Ok(_) => panic!("{src} was expected to fail"),
            Err(e) => e,
        }
    }

    fn top_text(src: &str) -> String {
        match run_src(src) {
            Ok(i) => i.peek().map(|v| v.display()).unwrap_or_default(),
            Err(e) => panic!("{src} failed: {e}"),
        }
    }

    /// Every node name in the graph is Title-cased, but `graph.paths` looks
    /// its start node up as given (`dijkstra.rs:52-58`).
    #[test]
    fn the_start_node_is_not_title_cased() {
        let e = err_of(":a list [ :A ] graph! graph.paths");
        assert!(e.contains("GRAPH.PATHS: unknown start node: a"), "{e}");
    }

    #[test]
    fn a_node_that_is_not_a_string_is_an_error() {
        let e = err_of("list [ 1 ] graph! graph.allpath");
        assert!(e.contains("MAKE_GRAPH: error casting node name"), "{e}");
        let f = err_of(":A list [ 1 ] graph! graph.path");
        assert!(f.contains("MAKE_FAST_GRAPH: error casting node name"), "{f}");
    }

    #[test]
    fn an_edge_to_an_unknown_node_is_an_error() {
        let e = err_of("[ [ :A :Z ] ] [ :A ] graph! graph.transitiveclosure");
        assert!(e.contains("MAKE_GRAPH: A and Z unknown nodes"), "{e}");
    }

    #[test]
    fn an_algorithm_wants_a_typed_graph() {
        let e = err_of(":A dict graph.paths");
        assert!(e.contains("GRAPH.PATHS: MISSED TYPE"), "{e}");
    }

    /// A third element is the weight only when there are exactly three
    /// (`make_graph.rs:125`); a fourth sends the edge back to 100.0.
    #[test]
    fn only_a_three_element_edge_carries_its_weight() {
        let three = top_text(":A [ [ :A :B 2.0 ] ] [ :A :B ] graph! graph.paths");
        assert!(three.contains("path=2.0"), "{three}");
        let four = top_text(":A [ [ :A :B 2.0 :x ] ] [ :A :B ] graph! graph.paths");
        assert!(four.contains("path=100.0"), "{four}");
        let e = err_of(":A [ [ :A :B 2 ] ] [ :A :B ] graph! graph.paths");
        assert!(e.contains("MAKE_GRAPH: Error casting weight"), "{e}");
    }
}
