# RFC-0012: Graph algorithms

- Status: **Draft** (2026-10-18), revised after its second adversarial
  review (`docs/rfc/reviews/RFC-0012-review-2026-10-18-2.md`, "Not ready
  for Accepted") and awaiting the next. The first review
  (`RFC-0012-review-2026-10-18.md`) found Bund2's `graph.path`
  Title-casing its operands (F218, fixed). The second found a wrong
  `graph.maxflow` answer, a misstated Bellman–Ford rule, weights that are
  not finite, and a `graph.path` crash in the reference; the owner ruled
  on all four in **D193** ("1A 2A 3A 5A"). Every other finding of both is
  answered in place. *(First written as Proposed, which skipped review;
  back to Draft the same day.)*
- Depends on: RFC-0002 (natives and their declared effects), RFC-0004
  (the effects `bund2 check` reads, and the residual stack), RFC-0005
  (which natives Tier 1 promotes)
- Decisions consumed: D192 (and its notes of 2026-10-18), D193, D14,
  D18, D24, D111 (and its note of 2026-10-07), D37, D38, D39, D48, D49,
  D55; the dispositions of F17, F218 and F219
- Registers searched by subject ("graph", the nineteen word names,
  `graph.rs`, `make_graph`, `fast_paths`, `calc_path`, `algos`, "Title",
  "NaN"): D14, D18, D24, D37, D38, D39, D48, D49, D55, D111, D192, D193,
  F15, F17, F101 (with its 2026-10-18 note on `fast_paths`' skipped
  edges), F218, F219, open-questions' subsystem table (`bund/graph`) and
  PROMOTABLE.txt. ERRATA has no graph entry.
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
- The names in `nodes` and at both ends of every edge are trimmed and
  Title-cased as the graph is read (`make_graph.rs:9-11`). **A node
  operand is not**: `graph.path` casts its start and end and looks them up
  as given (`getpath.rs:53-60`, `:65-71`), as `graph.paths` does its start
  (`dijkstra.rs:52-58`).
- Both builders take an edge as a LIST (or PAIR, which `cast_list` admits,
  `reference/rust_dynamic/src/cast.rs:57-59`) of at least two elements;
  fewer is refused (`make_graph.rs:68-69`, `:134-135`). Exactly three take
  the third as the weight, cast to FLOAT, and a failed cast is refused
  (`:59-63`, `:125-129`); two, or four and more, weigh 100 (`:66-67`,
  `:131-133`; F101). An edge naming an unknown node is refused
  (`:114-116`).
- There are **two readers**, and they disagree:

  | | `graph.path` (`make_fast_graph`, then `fast_paths`) | `graph.paths`, `.allpath`, `.transitiveclosure` (`make_graph`, then `algos`) |
  |---|---|---|
  | weight | cast `as usize`: 1.9 reads as 1 (`make_graph.rs:64`) | `f64` as given (`:130`), NaN and ±inf included |
  | self-loop | **skipped** (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/fast_paths-1.0.0/src/input_graph.rs:190-196`) | kept |
  | weight below 1 after the cast | **skipped**: the edge does not exist (`:197-203`) | kept |
  | repeated edge | of the edges kept, the **lowest** weight: `freeze` sorts by from, to, weight and keeps the first of each pair (`:136-161`) | the **last** given, which `algos`' `add_edge` removes and pushes to the end (`algos/…/4c08437/src/cs/graph.rs:108-113`) |

  So `[ :A :B 0.5 ] [ :A :B 5.0 ]` keeps 5.0 under `graph.path`: the 0.5
  edge was dropped before the lowest was chosen (F101's 2026-10-18 note).
  The `algos` revision is the one the oracle links
  (`reference/Bund/Cargo.lock:86-88`, `#4c08437…`).
- **`graph.path` panics on a node no kept edge reaches.** `fast_paths`
  sizes its graph by the largest kept endpoint (`input_graph.rs:204`) and
  `calc_path` asserts both nodes are inside it
  (`fast_paths-1.0.0/src/path_calculator.rs:77-82`), so an isolated last
  node, a graph with no edges, or one whose edges are all self-loops or
  under weight 1 crashes the oracle (F219).
- **A weight that is not finite reaches the `algos` words.** Programs make
  one in both engines (`-1.0 math.sqrt` is NaN; `1.0e200 1.0e200 *` is
  inf), and `graph.paths` answers a row `path=NaN` over a NaN edge, in the
  oracle and in Bund2.
- `graph.paths` iterates the `HashMap` Dijkstra answers to build its rows
  (`dijkstra.rs:60-80`), so their order is the map's; `graph.allpath` and
  `graph.transitiveclosure` do the same over Johnson's and Warshall's.
  `algos`' Dijkstra refuses a negative weight it meets
  (`algos/…/src/cs/graph/dijkstra.rs:105-109`).
- The family is **library**, not core, under D14's partition
  (`docs/core-words.md:16`), so it is outside the 100% preservation that
  core words carry.
- Bund2 reproduces all seven words (`crates/bund2-stdlib/src/graph.rs`),
  `graph.path`'s operand lookup since F218, and with F219's crash answered
  `found=false`.

## Design

### S1. The words

The graph is on top; a node operand beneath it.

| word | stack | answers |
|---|---|---|
| `graph.bfs` | `<start> <graph>` | LIST of names, breadth first |
| `graph.dfs` | `<start> <graph>` | LIST of names, depth-first preorder |
| `graph.reachable` | `<start> <graph>` | LIST of names reachable, `start` always included, node order |
| `graph.neighbors` | `<node> <graph>` | LIST of names one edge out, `node` itself when it has a self-loop |
| `graph.degree` | `<node> <graph>` | MAP `in`, `out`: INTEGERs, a repeated edge counted once, a self-loop once each way |
| `graph.toposort` | `<graph>` | LIST of every name, each before its successors |
| `graph.cycle` | `<graph>` | LIST of names around one cycle, or `[ ]` |
| `graph.components` | `<graph>` | LIST of LISTs, edges read both ways |
| `graph.scc` | `<graph>` | LIST of LISTs, each mutually reachable |
| `graph.mst` | `<graph>` | MAP `edges` (`[ from to weight ]`), `weight`; edges read both ways, self-loops never chosen (`graph_algorithms.rs`, `mst`), negative weights taken; a forest when the graph is not connected |
| `graph.bellmanford` | `<start> <graph>` | `graph.paths`' rows (`dijkstra.rs:69-76`): `node`, `path` or NODATA |
| `graph.maxflow` | `<sink> <source> <graph>` | MAP `flow`, `edges` (`[ from to share ]`, every edge whose share is above 0.0); every capacity above 0.0 counts, however small beside the largest (D193 §1); a sink the source cannot reach answers `flow` 0.0 |

Over the empty graph, `graph.toposort`, `graph.components` and
`graph.scc` answer `[ ]`, `graph.mst` answers `edges=[ ]` and
`weight=0.0`, and every word with a node operand refuses it as unknown.

**The `.` forms.** Every word also has a `.` form (D18, as D24 bounds it
and D111 applies it; extending it to these twelve is D192's build
choice). It runs the plain word through `bench_form`
(`crates/bund2-stdlib/src/wb.rs`): the graph is **taken from the
workbench**, the node operands from the stack as before, and **the answer
goes to the workbench**. `<graph> . :fetch graph.bfs. take` brings it
back. Its declared effect takes the node operands from the stack and
leaves nothing there.

### S2. Reading a graph

As `graph.paths` reads it (D192 §3 and its note): the rules under
"Current behaviour", directed, a weight kept as written, a repeated edge
taking the last weight given — `make_graph`'s reader, not `graph.path`'s,
because it keeps a fractional or negative weight, which `graph.mst` and
`graph.bellmanford` need. Two differences from `graph.paths`:

- **A weight that is not finite is refused** (D193 §3): no answer over a
  NaN or an infinite weight means anything.
- **A re-given edge keeps the place where it first appeared** (where
  `algos` moves it to the end), so "the order the graph lists its edges"
  below means first appearance.

**A node operand is Title-cased**, as the graph's own names are, so a
program finds a node by the spelling it gave in `nodes`. No reference
word does this (F218); it is D192's choice. It splits the family —
`graph.paths` refuses `:a` where `graph.bellmanford` takes it — and,
because Title case splits words, it also finds a node by spellings the
program never wrote: `:ab`, `:AB` and `"Ab"` all name the node listed as
`:AB`, and `:my_node` names the one listed as `"My Node"`.

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
- `GRAPH.BELLMANFORD: A reaches a negative cycle through B`: `B` is the
  smallest-numbered node of the negative cycle the search finds by walking
  back from the last edge it relaxed. With two reachable negative cycles,
  which one that is depends on the order of the edges, so `B` need not be
  the smallest node on any cycle; for one program it is the same on every
  run (D193 §2);
- `GRAPH.MAXFLOW: the source and the sink are one node: A`, checked before
  any capacity;
- `GRAPH.MAXFLOW: a capacity must not be below zero: A -> B weighs -1`,
  checked for **every** edge before the flow starts, self-loops and
  unreachable edges included.

The **shape** of the graph is checked first, in the reference's words, by
the `graph_operand` the seven share: `MISSED TYPE`, `unknown data type`,
`type casting returns: …`, `Nodes list must be a list`, `Edges list must
be a list`. Then the reader, naming the word and, where there is one, the
type by name:

- `a node name must be a STRING, not Integer`;
- `an edge must be a LIST, not Integer`;
- `an edge is too small`;
- `an edge's node must be a STRING`;
- `an edge names unknown nodes: A and Z`;
- `an edge's weight must be a FLOAT, not Integer`;
- `an edge's weight must be finite: A -> B weighs NaN` (D193 §3).

(`read` also refuses `nodes` or `edges` that is not a LIST, but
`graph_operand` refuses those first, so no word reaches it.) Then the
node operands, in order: a missing one `NO DATA #n`, the graph being #1;
one that is not a STRING, `the start node must be a STRING, not Integer`;
one not in the graph, `unknown start node: Z`. The role is `start`,
`source`, `sink`, or none for `graph.neighbors` and `graph.degree`. So a
malformed graph is reported before a bad node operand.

A `.` form checks both sides before it takes anything:
`Workbench is too shallow for inline GRAPH.BFS.` with no graph there, and
`Stack is too shallow for inline GRAPH.MAXFLOW.` with too few node
operands. Neither moves a value (`wb.rs`, `bench_form`).

**What a refusal leaves** (RFC-0004's residual stack,
`docs/rfc/RFC-0004-stack-effects.md:491-496`): each word pulls the graph,
then its node operands in turn, and refuses at the first that fails. The
values already pulled are gone, and everything beneath is untouched: `7
:E :Z <graph> graph.maxflow` refuses `:Z` and leaves `7 E`, and a graph
refused before its node is pulled leaves the node. A `.` form moves what
its word left above the stack beneath to the workbench, so `7 <graph> .
:E :Z graph.maxflow.` leaves `7` on the stack and `E` on the workbench.

### S5. Implementation

Bund2's own (D192 §2), in `crates/bund2-stdlib/src/graph_algorithms.rs`.
Each word reads the graph once, `operand` → `read`, and runs its
algorithm over that one `Net`. Every loop is bounded by the graph it was
handed (D39); the depth-first searches keep their own stack.

Nothing panics, and nothing answers a plausible value where an invariant
breaks (D37): a node index out of range when a name is asked for,
`toposort` stalling with no cycle to name, Bellman–Ford's walk back
reaching a node no edge relaxed, and `graph.maxflow` handed a capacity the
reader refuses are each `Error::internal`.

`graph.maxflow` is Edmonds–Karp. Its bound, at most nodes × edges
augmenting rounds (Edmonds and Karp, 1972), rests on two facts about the
residual graph: an augmentation removes only arcs on the shortest path it
used, and adds only the reverses of those arcs. Both hold in floating
point. An arc is present while its residual is above 0.0 (D193 §1). The
bottleneck `b` is the least residual on the path, so the arc that has it
ends at `cap - b`, exactly 0.0, and leaves; every other arc on the path
keeps a residual of at least 0.0, since `b ≤ cap`; every reverse arc gains
`b > 0`; no arc off the path changes. The loop stops at the bound and
would report passing it as an internal error, not hang. With capacities
that are not whole numbers, a share in `edges` can be of rounding size.

**Every word reads the graph afresh on each call**, as the reference's
words do: the MAP is the graph, and nothing is cached beside it. Each
call costs time in proportion to nodes plus edges before the algorithm
starts.

### S6. The tiers

The twelve, and their forms, are natives like any other, and nothing in
Tier 0 or the profiler treats them specially. Tier 1 promotes only the
natives RFC-0005's criterion 28 palette brings to `Ok` (§S5, D48, D55),
and syncs its state before any other, "as an embedder's native is"
(`tests/golden/PROMOTABLE.txt:1-6`). The palette cannot build a graph, so
all 24 of these names, like `graph.path`, `graph.paths`, `graph.allpath`
and `graph.transitiveclosure` and their forms, are `not reached` there and
not promotable. `graph` and `graph.`, which only check a MAP, are
promotable (`PROMOTABLE.txt:139-140`). Syncing costs speed and never
meaning.

## Which points are rulings

Accepting this RFC accepts the following. The owner ruled on these
(D192, D193): the twelve words and nothing more; Bund2's own
implementation; the graph read as `graph.paths` reads it, directed; node
order everywhere, and F17 fixed by it; values for answers and errors for
impossible questions; no tolerance in `graph.maxflow`; the Bellman–Ford
rule as built; weights that are not finite refused; F218 fixed; and
`graph.path` answering `found=false` where the reference crashes (F219).

These are D192's build choices, which D192 says can be overruled, and
which acceptance would ratify:

- a node operand Title-cased (S2);
- a re-given edge keeping its first place with its last weight (S2);
- a `.` form for each of the twelve (S1);
- which node each cycle error names (S4);
- `graph.cycle` answering `[ ]` rather than failing; `graph.reachable`
  including its start; `graph.degree` counting a repeated edge once;
  `graph.mst` as a forest; Edmonds–Karp rather than Dinic; and refusals
  in the word's own name, not `MAKE_GRAPH:`.

## Preservation analysis

- **The seven reference words**: preserved exactly, with three stated
  exceptions. F17's order is **deliberately changed** from unspecified to
  node order — an added guarantee, D192 §4. `graph.path`'s crash on a node
  no kept edge reaches is **deliberately changed** to `found=false` (F219,
  D193 §5). `graph.path`'s operands are looked up as given again (F218, a
  Bund2 defect fixed, not a deviation). No golden moves.
- **The twelve new words**: no reference behaviour exists. `conform`,
  `IMPLEMENTED` and `COVERAGE` move by zero. Nothing in `wb.rs`'s `ADDED`
  list pins their `.` forms; `each_form_holds_the_graph_on_the_workbench`
  names all twelve. What a program moving between old and new words sees
  differently:
  - **the reader**: against `graph.path`, a fractional weight is not
    truncated, an edge under 1 and a self-loop exist, a negative weight
    is taken, and a repeated edge takes its last weight (S2, "Current
    behaviour");
  - **non-finite weights**: `graph.paths` answers `path=NaN`, and the new
    words refuse the graph (D193 §3);
  - **operand casing**: the new words Title-case a node operand and the
    seven do not (S2);
  - **negative weights**: `graph.paths` refuses one Dijkstra meets, and
    `graph.bellmanford` takes it, so a swap from one to the other works
    one way;
  - **edge order**: a re-given edge keeps its first place (S2);
  - **the `.` forms** answer to the workbench and refuse a missing operand
    as `Stack is too shallow …`, where the plain forms say `NO DATA #n`.

## Alternatives considered

D192 records each with its reason: `algos`' own modules (`HashMap` order,
unchecked indexing), `petgraph` (a new dependency and a third
representation), integer weights (one graph meaning two things), a
`directed` key (`graph` would print it: the probe golden pins `{ edges=[]
:: nodes=[] :: type=graph }`, `tests/golden/probes/graph-algorithms.golden:9`),
NODATA for an impossible question (no reason given, easily passed along),
and `graph.path`'s `found` MAP for every answer. D192's note adds
`graph.path`'s reader for the new words, and looking a node operand up as
given. D193 records the rest: a tolerance in `graph.maxflow` (wrong
answers when capacities differ in scale), computing the smallest node on
every reachable negative cycle (one more search per node), specifying
NaN and ±inf answers instead of refusing them, and an ordinary error for
F219 instead of `found=false`.

## Acceptance criteria

1. **Effects.** Each of the twelve leaves exactly one answer above a `7`
   and the `7` beneath it; each `.` form leaves one answer on the
   workbench and the stack at `7`:
   `each_form_holds_the_graph_on_the_workbench`, all twelve.
2. **The reader is `graph.paths`' reader**, but for weights that are not
   finite: it refuses what `build_algos` refuses, and otherwise numbers the
   nodes and keeps the edges and weights as `build_algos` does —
   `the_reader_agrees_with_build_algos`, 2,000 graphs with names
   Title-casing merges, edges of one to four elements and PAIRs,
   fractional, negative and INTEGER weights, repeated edges, unknown nodes,
   and now and then a node, an end or an edge of the wrong type. That it
   refuses a weight that is not finite: criterion 6.
3. **Each algorithm's answer, as a set or a value**, agrees with an
   independent one over seeded random graphs:
   `reachable_is_the_transitive_closure_row`,
   `scc_is_mutual_reachability`, `components_join_what_any_edge_joins`,
   `toposort_and_cycle_agree`, `bellman_ford_agrees_with_dijkstra`,
   `bellman_ford_agrees_with_floyd_warshall` (also: the node named is
   reachable from the start and the smallest of a negative cycle through
   it, the rule of S4), `mst_is_minimal` (at least 100 of its 200 graphs
   checked exhaustively), `max_flow_is_the_min_cut` (every source and
   sink) and `max_flow_is_the_min_cut_across_magnitudes` (whole capacities
   over thirteen orders of magnitude, flow equal to the cut exactly).
4. **S3's order**: `traversal_orders_match_their_definitions`,
   `answers_follow_the_nodes_list`, `ties_and_edges_follow_the_graph_order`,
   and the MST order check in `mst_is_minimal`.
5. **F17**: `the_algos_words_answer_in_node_order` in `graph.rs` —
   `graph.paths`, `graph.allpath` and `graph.transitiveclosure` over
   `nodes` `[ :C :A :B ]`, twenty times in one process.
6. **S1's answers and S4's refusals**, through the interpreter:
   `traversals_answer_in_node_order`, `a_node_operand_is_title_cased`,
   `structure_words_answer_lists_of_lists`,
   `a_cycle_has_no_topological_order`, `weighted_words_answer_maps`,
   `a_repeated_edge_takes_its_last_weight`,
   `the_workbench_forms_hold_the_graph`, `each_refusal_section_s4_names`,
   `every_node_operand_is_refused_naming_the_word`,
   `every_reader_refusal_names_the_word_and_the_type`,
   `a_malformed_graph_is_refused_in_the_reference_words`,
   `a_weight_that_is_not_finite_is_refused` (NaN, inf and −inf made in
   Bund, all twelve words),
   `a_small_capacity_beside_a_huge_one_still_carries_flow`, and
   `ties_and_edges_follow_the_graph_order` (an unreachable sink).
7. **What a refusal leaves**: `a_refusal_leaves_what_s4_says` — plain and
   `.` forms, a refused graph leaving its node, and both `.` form
   pre-checks.
8. **F218 and F219**: `graph_path_looks_its_nodes_up_as_given`;
   `graph_path_answers_no_path_for_a_node_no_kept_edge_reaches` (the five
   graphs that crash the oracle); and
   `graph_path_drops_an_edge_under_one_before_the_lowest_is_kept`.
9. **Depth**: a 200,000-node chain read by `read`, open and closed into a
   cycle, through `dfs`, `scc`, `find_cycle` and `toposort` on a thread
   whose stack is set to 2 MiB (`a_deep_graph_does_not_overflow_the_stack`).
10. `cargo xtask conform` unchanged at 136/148, ceiling 136/148; clippy
    clean; the workspace's tests pass.

## Open questions

- **Operand casing across the family** (S2). D192 keeps Title-casing for
  the new words; the owner can reverse it, which would make every graph
  word look its operands up as given.
- **Caching a read graph** (S5): not proposed; it would need a derived
  form that a `set` on the MAP invalidates.
