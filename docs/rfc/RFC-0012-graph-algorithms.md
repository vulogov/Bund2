# RFC-0012: Graph algorithms

- Status: **Draft** (2026-10-18), revised after its first adversarial
  review (`docs/rfc/reviews/RFC-0012-review-2026-10-18.md`, "Not ready for
  Accepted") and awaiting the next. The design rests on the repository
  owner's rulings in **D192** ("1A 2A 3A 4A 5A"), and it was built the
  same day; each criterion below names the test that reads it. *(First
  written as Proposed, which skipped the review every earlier RFC had
  before that status. Asked "is RFC-0012 ready?", the answer was no, and
  the status went back to Draft the same day. The review then found that
  Bund2's `graph.path` Title-cased its operands where the reference does
  not; that is F218, fixed on the owner's ruling, 1A. Every other finding
  is answered in place.)*
- Depends on: RFC-0002 (natives and their declared effects), RFC-0004
  (the effects `bund2 check` reads), RFC-0005 (which natives Tier 1
  promotes)
- Decisions consumed: D192 (and its two notes of 2026-10-18), D14, D38,
  D37, D39, D111 (and its note of 2026-10-07); F17's and F218's
  dispositions
- Registers searched by subject ("graph", the nineteen word
  names, `graph.rs`, `make_graph`, `fast_paths`, `algos`, "Title"): D14
  (library scope), D37, D38 (the `algos` pin), D39, D111 and its
  2026-10-07 note (the workbench forms), D192, F15 and F17 (order), F101
  (`graph.path`'s node and edge reading, and the four-element rule), F218
  (`graph.path`'s operands), open-questions' subsystem table
  (`bund/graph`), and PROMOTABLE.txt (the tier's palette). ERRATA has no
  graph entry.
- Reference SHA: `reference/Bund` 21b40b0
- Supersedes: nothing

## Summary

Twelve graph words of Bund2's own — traversal, structure, and three
weighted questions — over the graph MAP the reference's seven graph words
already read, and a fixed order for the answers of three of those seven
(F17).

## Motivation

The reference answers four questions of a graph: one shortest path
(`graph.path`, `reference/Bund/src/stdlib/functions/graph/getpath.rs:106`),
the distances from one node (`graph.paths`, `dijkstra.rs:95`), every
pair's distance (`graph.allpath`, `allshortpath.rs:95`) and which pairs
connect (`graph.transitiveclosure`, `transitiveclosure.rs:84`). The other
three words, `graph`, `graph.` and `graph!`, build or check the MAP
(`mod.rs:113-115`). A program that wants a dependency order, the strongly
connected parts of a graph, a spanning tree or a flow writes the algorithm
in Bund, over a MAP of LISTs. And three of the four answers come back in
an order that changes from run to run, because `algos` hands each back as
a `HashMap` (`~/.cargo/git/checkouts/algos-9d1538761d16fda1/4c08437/src/cs/graph/dijkstra.rs:71`,
`johnson.rs:25`, `warshall.rs:40`; F17), so a program cannot print them
reproducibly.

## Current behaviour

- A graph is a MAP with `type = "graph"`, `nodes` and `edges`, built by
  `graph!` from two LISTs (`mod.rs:68-93`). Nothing is cached: each
  algorithm word calls its builder on every call (`getpath.rs:44`,
  `dijkstra.rs:44`, `allshortpath.rs:44`, `transitiveclosure.rs:44`).
- The names in `nodes` and at both ends of every edge are Title-cased as
  the graph is read (`make_graph.rs:9-11`). **A node operand is not**:
  `graph.path` casts its start and end and looks them up as given
  (`getpath.rs:53-60`, `:65-71`), as `graph.paths` does its start
  (`dijkstra.rs:52-58`). So `:a` finds no node, even one the graph listed
  as `:a`.
- There are **two readers**, and they disagree:

  | | `graph.path` (`make_fast_graph`) | `graph.paths`, `.allpath`, `.transitiveclosure` (`make_graph`) |
  |---|---|---|
  | weight | cast `as usize`: 1.9 reads as 1, −1.0 as 0 (`make_graph.rs:64`) | `f64` as given (`:130`) |
  | repeated edge | the **lowest** weight kept: `freeze` sorts by from, to, weight and keeps the first of each pair (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/fast_paths-1.0.0/src/input_graph.rs:136-161`) | the **last** given, which `algos`' `add_edge` removes and pushes to the end of the node's edges (`algos/…/4c08437/src/cs/graph.rs:108-113`) |

  The `algos` revision is the one the oracle links
  (`reference/Bund/Cargo.lock:86-88`, `#4c08437…`).
- Both readers take an edge as a LIST of at least two elements; fewer is
  refused, `relation is too small` (`make_graph.rs:134-135`). Exactly
  three take the third as the weight, cast to FLOAT, and a cast that fails
  is refused (`:125-129`); two, or four and more, weigh 100.0 and anything
  past the second is ignored (`:131-133`; F101). An edge naming an unknown
  node is refused (`:113-116`).
- `graph.paths` iterates the `HashMap` Dijkstra answers to build its rows
  (`dijkstra.rs:60-80`), so their order is the map's; `graph.allpath` and
  `graph.transitiveclosure` do the same over Johnson's and Warshall's.
  `algos`' Dijkstra refuses a negative weight it meets
  (`algos/…/src/cs/graph/dijkstra.rs:104-108`).
- The family is **library**, not core, under D14's partition
  (`docs/core-words.md:16`), so it is outside the 100% preservation that
  core words carry.
- Bund2 reproduces all seven words (`crates/bund2-stdlib/src/graph.rs`),
  `graph.path`'s operand lookup since F218.

## Design

### S1. The words

The graph is on top; a node operand beneath it. Every word also has a
`.` form holding the graph on the workbench (slot 0). D111 rules the
container family; extending it to these twelve is D192's build choice.

| word | stack | answers |
|---|---|---|
| `graph.bfs` | `<start> <graph>` | LIST of names, breadth first |
| `graph.dfs` | `<start> <graph>` | LIST of names, depth-first preorder |
| `graph.reachable` | `<start> <graph>` | LIST of names reachable, `start` always included, node order |
| `graph.neighbors` | `<node> <graph>` | LIST of names one edge out |
| `graph.degree` | `<node> <graph>` | MAP `in`, `out`: INTEGERs, a repeated edge counted once, a self-loop once each way |
| `graph.toposort` | `<graph>` | LIST of every name, each before its successors |
| `graph.cycle` | `<graph>` | LIST of names around one cycle, or `[ ]` |
| `graph.components` | `<graph>` | LIST of LISTs, edges read both ways |
| `graph.scc` | `<graph>` | LIST of LISTs, each mutually reachable |
| `graph.mst` | `<graph>` | MAP `edges` (`[ from to weight ]`), `weight`; edges read both ways, self-loops never chosen, negative weights taken; a forest when the graph is not connected |
| `graph.bellmanford` | `<start> <graph>` | `graph.paths`' rows (`dijkstra.rs:69-76`): `node`, `path` or NODATA |
| `graph.maxflow` | `<sink> <source> <graph>` | MAP `flow`, `edges` (`[ from to share ]`, only edges that carry flow); a sink the source cannot reach answers `flow` 0.0 |

### S2. Reading a graph

As `graph.paths` reads it (D192 §3 and its note): the rules under
"Current behaviour", directed, a weight kept as written, a repeated edge
taking the last weight given. That is `make_graph`'s reader, not
`graph.path`'s, chosen because it keeps a fractional or negative weight,
which `graph.mst` and `graph.bellmanford` need. A re-given edge keeps the
place where it **first** appeared (where `algos` moves it to the end), so
"the order the graph lists its edges" below means first appearance.

**A node operand is Title-cased**, as the graph's own names are, so a
program names a node the way it wrote it in `nodes`. No reference word
does this (F218); it is D192's choice, and it splits the family:
`graph.paths` refuses `:a` where `graph.bellmanford` takes it.

### S3. Order

Every answer follows the order of the `nodes` list (D192 §4), not the
alphabet:

- successors are taken in node order, so `graph.bfs`, `graph.dfs` and
  `graph.neighbors` are fixed by it;
- `graph.reachable` answers in node order;
- `graph.toposort` takes, of the nodes free to go next, the earliest
  listed;
- `graph.components` and `graph.scc` list each group's members in node
  order, and the groups by their first member;
- `graph.cycle` answers the first cycle a depth-first search in node order
  closes, starting where the search entered it;
- `graph.mst` answers its edges lightest first, equal weights in the order
  the graph lists its edges, and `graph.maxflow` its `edges` in that order;
- F17's three words sort their rows the same way: `graph.paths` by node,
  `graph.allpath` and `graph.transitiveclosure` by `from` and then `to`.

### S4. Failures

An impossible question is an error a program catches with `?try`
(D192 §5):

- `GRAPH.TOPOSORT: the graph has a cycle through A`, naming the first node
  of the cycle `graph.cycle` would answer;
- `GRAPH.BELLMANFORD: A reaches a negative cycle through B`, `B` the
  smallest-numbered node on a negative cycle reachable from `A`;
- `GRAPH.MAXFLOW: the source and the sink are one node: A`;
- `GRAPH.MAXFLOW: a capacity must be a number not below zero: A -> B weighs
  -1`, checked for **every** edge before the flow starts, self-loops and
  unreachable edges included, and for a capacity that is not finite.

The operands are refused naming the word: a missing one `NO DATA #n`,
the graph being #1; a node that is not a STRING, `the start node must be
a STRING`; one not in the graph, `unknown start node: Z` (the role is
`start`, `source`, `sink`, or none for `graph.neighbors` and
`graph.degree`). An edge the reader refuses names the word too (`an edge
is too small`, `an edge's weight must be a FLOAT`, `an edge names unknown
nodes`). The **shape** of the graph is checked first, in the reference's
words, by the `graph_operand` the seven share: `MISSED TYPE`, `unknown
data type`, `type casting returns: …`, `Nodes list must be a list`,
`Edges list must be a list`. So a malformed graph is reported before a
bad node operand.

**What a refusal leaves**: each word pulls the graph, then its node
operands in turn, and refuses at the first that fails, so the values
already pulled are gone and anything beneath is untouched. That is the
reference words' pull-then-check shape; RFC-0004 treats the residual stack
as observable, and this is it.

### S5. Implementation

Bund2's own (D192 §2), in `crates/bund2-stdlib/src/graph_algorithms.rs`.
Each word reads the graph once, `operand` → `read`, and runs its
algorithm over that one `Net`. Every loop is bounded by the graph it was
handed (D39); the depth-first searches keep their own stack. Nothing
panics (D37).

`graph.maxflow` is Edmonds–Karp. Its bound, at most nodes × edges
augmenting rounds (Edmonds and Karp, 1972), rests on two facts about the
residual graph: an augmentation removes only arcs on the shortest path it
used, and adds only the reverses of those arcs. Both hold in floating
point here. The bottleneck `b` is the least residual on the path, so the
arc that has it ends at `cap - b`, exactly 0.0; an arc counts as present
only above a tolerance (1e-12 of the largest capacity, at least 1e-12),
and `b` is above it, so a reverse arc gains a present residual. No arc
off the path changes. The loop stops at the bound and would report
passing it as an internal error, not hang.

**Every word reads the graph afresh on each call**, as the reference's
words do: the MAP is the graph, and nothing is cached beside it. Each
call costs time in proportion to nodes plus edges before the algorithm
starts. Caching would need a place to keep a derived form that a `set` on
the MAP invalidates; that is not proposed.

### S6. The tiers

The twelve, and their forms, are natives like any other, and nothing in
Tier 0 or the profiler treats them specially. Tier 1 promotes only the
natives RFC-0005's criterion 28 palette certifies (§S5, D48, D55); the
palette cannot build a graph, so all 24 names are `not reached` in
`tests/golden/PROMOTABLE.txt`, as the reference's graph words are. None
is promotable: a compiled body syncs its state before each call to one,
which costs speed and never meaning.

## Preservation analysis

- **The seven reference words**: preserved exactly, except F17's order,
  which is **deliberately changed** from unspecified to node order — an
  added guarantee, D192 §4. `graph.path`'s operands are looked up as given
  again (F218, a Bund2 defect fixed, not a deviation). No golden moves.
- **The twelve new words**: no reference behaviour exists. `conform`,
  `IMPLEMENTED` and `COVERAGE` move by zero. What a program moving between
  old and new words sees differently:
  - **the reader**: the new words read as `graph.paths` does, so against
    `graph.path` a fractional weight, a negative weight and a repeated
    edge read differently (S2);
  - **operand casing**: the new words Title-case a node operand and the
    seven do not, so `graph.bellmanford` takes the `:a` that
    `graph.paths` refuses (S2, D192's note);
  - **negative weights**: `graph.paths` refuses one Dijkstra meets, and
    `graph.bellmanford` takes it, so a swap from one to the other works
    one way;
  - **edge order**: a re-given edge keeps its first place (S2).

## Alternatives considered

D192 records each with its reason: `algos`' own modules (`HashMap` order,
unchecked indexing), `petgraph` (a new dependency and a third
representation), integer weights (one graph meaning two things), a
`directed` key (`graph` would print it: the probe golden pins `{ edges=[]
:: nodes=[] :: type=graph }`, `tests/golden/probes/graph-algorithms.golden:9`),
NODATA for an impossible question (no reason given, easily passed along),
and `graph.path`'s `found` MAP for every answer. D192's note records two
more: `graph.path`'s reader for the new words (it would cut weights to
whole numbers and refuse nothing negative), and looking a node operand up
as given (a lower-case or multi-word name could never be found).

## Acceptance criteria

1. **Effects.** Each of the twelve leaves exactly one answer above the
   stack beneath it, and each `.` form leaves the stack beneath alone:
   `each_form_holds_the_graph_on_the_workbench`, all twelve, over a `7`.
   (`bund2 words` shows only that the 24 names are registered; the honesty
   test's `UNVARIED` entry for `graph.maxflow` is an exemption, not a
   check.)
2. **The reader is `graph.paths`' reader**: it refuses what `build_algos`
   refuses, and otherwise numbers the nodes and keeps the edges and
   weights as `build_algos` does — `the_reader_agrees_with_build_algos`,
   2,000 graphs with names Title-casing merges, edges of one to four
   elements, fractional, negative and INTEGER weights, repeated edges and
   unknown nodes.
3. **Each algorithm's answer, as a set or a value**, agrees with an
   independent one over seeded random graphs:
   `reachable_is_the_transitive_closure_row`,
   `scc_is_mutual_reachability`, `components_join_what_any_edge_joins`,
   `toposort_and_cycle_agree`, `bellman_ford_agrees_with_dijkstra`,
   `bellman_ford_agrees_with_floyd_warshall` (also: the node named is
   reachable from the start and the smallest on its negative cycle),
   `mst_is_minimal` (at least 100 of its 200 graphs checked exhaustively),
   `max_flow_is_the_min_cut` (every source and sink).
4. **S3's order**: `traversal_orders_match_their_definitions` (BFS and
   DFS against a level-by-level and a recursive search, over random
   graphs); `answers_follow_the_nodes_list` (every word that answers names,
   over a `nodes` list in reverse); `ties_and_edges_follow_the_graph_order`
   (`graph.mst`'s ties and a re-given edge's place, `graph.maxflow`'s
   `edges`); the MST order check in `mst_is_minimal`.
5. **F17**: `the_algos_words_answer_in_node_order` in `graph.rs` —
   `graph.paths`, `graph.allpath` and `graph.transitiveclosure` over
   `nodes` `[ :C :A :B ]`, twenty times in one process, answering in that
   order, the pair words by `from` and then `to`.
6. **S1's answers and S4's refusals**, through the interpreter:
   `traversals_answer_in_node_order`, `a_node_operand_is_title_cased`,
   `structure_words_answer_lists_of_lists`,
   `a_cycle_has_no_topological_order`, `weighted_words_answer_maps`,
   `a_repeated_edge_takes_its_last_weight`,
   `the_workbench_forms_hold_the_graph`, `each_refusal_section_s4_names`,
   `every_node_operand_is_refused_naming_the_word` (every word with a node
   operand, every role, unknown and non-STRING, and a bad edge before a bad
   operand), `a_malformed_graph_is_refused_in_the_reference_words` (the
   shape refusals, all twelve),
   `a_capacity_that_is_not_finite_is_refused_through_the_word` (±∞ and
   NaN), and `ties_and_edges_follow_the_graph_order` (an unreachable sink).
7. **F218**: `graph_path_looks_its_nodes_up_as_given` — `:c :a` is
   `unknown start node: a`, as the oracle answers.
8. **Depth**: a 200,000-node chain, open and closed into a cycle, through
   `dfs`, `scc`, `find_cycle` and `toposort` on a test thread's 2 MiB
   stack (`a_deep_graph_does_not_overflow_the_stack`).
9. `cargo xtask conform` unchanged at 136/148, ceiling 136/148; clippy
   clean; the workspace's tests pass.

## Open questions

- **Weights that are not finite.** Only `graph.maxflow` refuses one. The
  others take it as IEEE arithmetic does: `graph.mst` orders it by
  `total_cmp`, and `graph.bellmanford` stores NaN for a node first reached
  over a NaN edge. Bund's division refuses a zero divisor in both engines,
  so the plain way to make one is closed; whether another word makes one
  was not checked. Unspecified until a program is shown to make one.
- **Operand casing across the family** (S2). D192 keeps Title-casing for
  the new words; the owner can reverse it, which would make every graph
  word look its operands up as given.
- **Caching a read graph** (S5): not proposed; would need a derived form a
  `set` invalidates.
