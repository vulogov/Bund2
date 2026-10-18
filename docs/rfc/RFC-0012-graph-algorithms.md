# RFC-0012: Graph algorithms

- Status: **Draft** (2026-10-18), awaiting its first adversarial review.
  The design rests on the repository owner's rulings in **D192** ("1A 2A
  3A 4A 5A"), and it was built the same day; each criterion below names
  the test that reads it. *(First written as Proposed, which skipped the
  review every earlier RFC had before that status — RFC-0011's went Draft,
  review, Proposed. Asked "is RFC-0012 ready?", the answer was no: one
  statement of current behaviour was wrong, several claims had no test,
  and two subjects in the registers went unmentioned. Each is answered
  in place, and the status went back to Draft the same day.)*
- Depends on: RFC-0002 (natives and their declared effects), RFC-0004
  (the effects `bund2 check` reads)
- Decisions consumed: D192, D38, D37, D39, D111 (and its note of
  2026-10-07), D14; F17's disposition
- Registers searched by subject ("graph", the word names, `graph.rs`,
  `make_graph.rs`, `algos`): D14 (library scope), D38 (the `algos` pin),
  D111 and its 2026-10-07 note (the workbench forms), F15 and F17 (order),
  open-questions' subsystem table (`bund/graph`, five corpus programs),
  and PROMOTABLE.txt (the tier's palette)
- Reference SHA: `reference/Bund` 21b40b0
- Supersedes: nothing

## Summary

Twelve graph words of Bund2's own — traversal, structure, and three
weighted questions — over the graph MAP the reference's seven graph words
already read, and a fixed order for the answers of three of those seven
(F17).

## Motivation

The reference answers three graph questions: one shortest path
(`graph.path`), the distances from one node (`graph.paths`), every pair's
distance (`graph.allpath`) and which pairs connect
(`graph.transitiveclosure`) (`reference/Bund/src/stdlib/functions/graph/mod.rs:113-120`).
A program that wants a dependency order, the strongly connected parts of
a graph, a spanning tree or a flow writes the algorithm in Bund, over a
MAP of LISTs. And three of the four answers come back in an order that
changes from run to run (F17), so a program cannot print them reproducibly.

## Current behaviour

- A graph is a MAP with `type = "graph"`, `nodes` and `edges`, built by
  `graph!` from two LISTs (`mod.rs:68-93`). Nothing is cached; every word
  re-reads it.
- Node names are Title-cased before anything looks them up
  (`make_graph.rs:9-11`).
- An edge is a LIST of at least two elements; fewer is refused, `relation
  is too small` (`make_graph.rs:135`). Exactly three take the third as the
  weight, cast to FLOAT, and a cast that fails is refused; two, or four and
  more, weigh 100.0 and anything past the second is ignored
  (`make_graph.rs:125-133`). An edge naming an unknown node is an error
  (`:113-116`).
- `graph.paths` iterates the `HashMap` `algos`' Dijkstra answers to build
  its rows (`dijkstra.rs:60-80`), so their order is the map's.
- The family is **library**, not core, under D14's partition
  (`docs/core-words.md:16`), so it is outside the 100% preservation that
  core words carry. This RFC changes nothing there: the seven words stay
  as the reference has them but for F17's order, and the twelve are new.
- Bund2 reproduces all seven words (`crates/bund2-stdlib/src/graph.rs`),
  and a graph's edges are kept by `algos`' `Graph`, whose `add_edge`
  replaces an earlier edge between the same pair
  (`~/.cargo/git/checkouts/algos-9d1538761d16fda1/4c08437/src/cs/graph.rs:108-121`).

## Design

### S1. The words

The graph is on top; a node operand beneath it. Every word has a `.` form
holding the graph on the workbench (D111's container family).

| word | stack | answers |
|---|---|---|
| `graph.bfs` | `<start> <graph>` | LIST of names, breadth first |
| `graph.dfs` | `<start> <graph>` | LIST of names, depth-first preorder |
| `graph.reachable` | `<start> <graph>` | LIST of names reachable, `start` included, node order |
| `graph.neighbors` | `<node> <graph>` | LIST of names one edge out |
| `graph.degree` | `<node> <graph>` | MAP `in`, `out` |
| `graph.toposort` | `<graph>` | LIST of every name, each before its successors |
| `graph.cycle` | `<graph>` | LIST of names around one cycle, or `[ ]` |
| `graph.components` | `<graph>` | LIST of LISTs, edges read both ways |
| `graph.scc` | `<graph>` | LIST of LISTs, each mutually reachable |
| `graph.mst` | `<graph>` | MAP `edges` (`[ from to weight ]`, lightest first), `weight` |
| `graph.bellmanford` | `<start> <graph>` | `graph.paths`' rows: `node`, `path` or NODATA |
| `graph.maxflow` | `<sink> <source> <graph>` | MAP `flow`, `edges` (`[ from to share ]`) |

### S2. Reading a graph

As the reference's words read it (D192 §3): the rules under "Current
behaviour", directed, a repeated edge replacing the earlier one as
`algos` does. A node operand is Title-cased. Refusals name the word.

### S3. Order

Every answer is in the order of the `nodes` list (D192 §4): successors
are taken in node order, ties go to the smaller-numbered node, groups are
ordered by their first member, and `graph.mst` takes equal weights in the
order the graph lists them. F17's three words sort their rows the same
way.

### S4. Failures

An impossible question is an error a program catches with `?try`
(D192 §5): `GRAPH.TOPOSORT: the graph has a cycle through A`;
`GRAPH.BELLMANFORD: A reaches a negative cycle through B`;
`GRAPH.MAXFLOW: the source and the sink are one node: A`; a capacity
below zero or not finite. An unknown or non-STRING node operand is refused
naming the word.

### S5. Implementation

Bund2's own, in `crates/bund2-stdlib/src/graph_algorithms.rs`, over one
read of the graph (D192 §2). Every loop is bounded by the graph it was
handed (D39); the depth-first searches keep their own stack;
`graph.maxflow` is Edmonds–Karp, whose rounds are bounded by nodes ×
edges, and passing that bound would be an internal error, not a hang.
Nothing panics (D37).

**Every word reads the graph afresh on each call**, as the reference's
words do (`graph.rs`'s module comment): the MAP is the graph, and nothing
is cached beside it. Each call costs time in proportion to nodes plus
edges before the algorithm starts, so a program that asks many questions
of one large graph pays that each time. Caching would need a place to keep
a derived form that a `set` on the MAP invalidates; that is not proposed.

### S6. The tiers

The twelve, and their forms, are natives like any other, and nothing in
Tier 0 or the profiler treats them specially: a profiled run counts each
call. Tier 1 promotes only the natives criterion 28's palette certifies
(RFC-0005 §S5, D48, D55), and the palette cannot build a graph, so these
are listed `not reached` in `tests/golden/PROMOTABLE.txt`, as the
reference's graph words are. A compiled body therefore syncs its state
before each call to one: that costs speed and never meaning, and the
answer is the interpreter's.

## Preservation analysis

- The seven reference words: preserved exactly, except F17's order,
  which is **deliberately changed** from unspecified to node order — an
  added guarantee, D192 §4. No golden moves.
- The twelve new words: no reference behaviour exists. `conform`,
  `IMPLEMENTED` and `COVERAGE` move by zero.

## Alternatives considered

D192 records each with its reason: `algos`' own modules (`HashMap` order,
unchecked indexing), `petgraph` (a new dependency and a third
representation), integer weights (one graph meaning two things), a
`directed` key (`graph` would print it), NODATA for an impossible question
(no reason given, easily passed along), and `graph.path`'s `found` MAP
for every answer.

## Acceptance criteria

1. The twelve words and their `.` forms are registered with fixed
   effects — `bund2 words`; the honesty test
   `no_native_takes_a_stack_name_in_an_operand_the_palette_leaves_at_seven`
   lists `graph.maxflow`'s third operand.
2. A graph reads the same to every word:
   `the_reader_agrees_with_build_algos` (300 graphs).
3. Each algorithm agrees with an independent answer over seeded random
   graphs: `reachable_is_the_transitive_closure_row`,
   `scc_is_mutual_reachability`, `components_join_what_any_edge_joins`,
   `toposort_and_cycle_agree`, `bellman_ford_agrees_with_dijkstra`,
   `bellman_ford_agrees_with_floyd_warshall`, `mst_is_minimal`,
   `max_flow_is_the_min_cut`.
4. Answers and refusals as §S1 and §S4 say, through the interpreter: the
   unit tests in `graph_algorithms.rs`, among them
   `each_refusal_section_s4_names` (a missing operand, an integer weight,
   an unknown node, a short edge, a negative and an infinite capacity, and
   a four-element edge weighing 100.0).
5. Every `.` form holds the graph on the workbench, answers what its
   plain word answers there, and refuses a non-graph in the same words:
   `each_form_holds_the_graph_on_the_workbench`, all twelve.
6. A graph too deep for a recursive search is answered: a 200,000-node
   chain, open and closed into a cycle, through `dfs`, `scc`,
   `find_cycle` and `toposort` on a test thread's 2 MiB stack
   (`a_deep_graph_does_not_overflow_the_stack`).
7. F17: `graph.paths.` agrees row for row with `graph.paths`
   (`each_form_of_several_operands_takes_the_operand_d111_names`).
8. `cargo xtask conform` unchanged at 136/148, ceiling 136/148; clippy
   clean; the workspace's tests pass.
