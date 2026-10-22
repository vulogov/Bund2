# RFC-0012: Graph algorithms

- Status: **Draft** (2026-10-18), revised after each of six adversarial
  reviews, every one "Not ready for Accepted", and awaiting the next. The
  first review
  (`RFC-0012-review-2026-10-18.md`) found Bund2's `graph.path`
  Title-casing its operands (F218, fixed). The second (`RFC-0012-review-2026-10-18-2.md`) found a wrong
  `graph.maxflow` answer, a misstated Bellman–Ford rule, weights that are
  not finite, and a `graph.path` crash in the reference; the owner ruled
  on all four in **D193** ("1A 2A 3A 5A"). Every other finding of both is
  answered in place. *(First written as Proposed, which skipped review;
  back to Draft the same day.)* The third review
  (`RFC-0012-review-2026-10-20.md`, "Not ready for Accepted") found the
  family core, not library; `graph.allpath`'s answer turning on the node
  listed last (F220); a one-node path F219's fix refused; and sums that
  overflow, one hiding a negative cycle. The owner ruled on the last three
  in **D198**, **D199** and **D200**; the rest is answered in place, and
  the review's answers are listed at the end. The fourth review
  (`RFC-0012-review-2026-10-21.md`, "Not ready for Accepted") found
  `graph.bellmanford` and `graph.mst` refusing sums that leave the answer
  finite, `graph.mst` answering `-0.0` over the empty graph, and F220
  stated more broadly than it holds. The owner ruled on the first two in
  **D202** (A: refuse only an answer that cannot be represented); the rest
  is answered in place, and listed at the end. The fifth review
  (`RFC-0012-review-2026-10-21-2.md`, "Not ready for Accepted") found
  `graph.allpath` refusing graphs with no cycle (F220's third face, put to
  the owner as **D203**, resolved A: preserved under D200); D202's build
  keeping an exception the
  ruling did not grant, and untested by any cross-check (both answered in
  code); and the `.` forms' workbench half undeclared, which D195 bears on.
  The rest is answered in place, and listed at the end. The sixth review
  (`RFC-0012-review-2026-10-22.md`) found F219's `found=false` approved on
  a premise the reference contradicts, since `fast_paths` never answers
  `found=false` (**D206**, A: the reference's own no-path error), and
  `graph.bellmanford`'s cycle test decided by rounding in both directions
  (**D207**, C: the relaxation runs exactly). The rest is answered in
  place, and listed at the end.
- Depends on: RFC-0002 (natives and their declared effects), RFC-0004
  (the effects `bund2 check` reads, and the residual stack), RFC-0005
  (which natives Tier 1 promotes)
- Decisions consumed: D192 (and its notes of 2026-10-18), D193, D198,
  D199, D200, D202, D203, D206, D207, D14, D18, D24, D111 (and its note of 2026-10-07), D37, D38,
  D39, D48, D49, D55, D195; the dispositions of F17, F218, F219 and F220.
- Registers searched by subject ("graph", the nineteen word names,
  `graph.rs`, `make_graph`, `fast_paths`, `calc_path`, `algos`, "Title",
  "NaN"): D14, D18, D24, D37, D38, D39, D48, D49, D55, D111, D192, D193,
  F15, F17, F101 (with its 2026-10-18 note on `fast_paths`' skipped
  edges), F218, F219, open-questions' subsystem table (`bund/graph`) and
  PROMOTABLE.txt. ERRATA has no graph entry. *Since the third review, by
  "overflow", "allpath", "johnson", "workbench" and "bench" as well:* D195,
  D198, D199, D200, D202, D203, F220 and its notes. *Since the sixth, by
  "rounding", "tolerance", "found=false", "can not calculate" and "cycle":*
  D193 again, D206, D207.
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
  `:131-133`; F101). An edge naming an unknown node is refused by both
  (`make_fast_graph`'s `:48-50`, `make_graph`'s `:114-116`).
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
  oracle and in Bund2: `make_graph` takes any FLOAT as a weight
  (`make_graph.rs:126-130`), and `algos`' Dijkstra refuses only one below
  zero (`algos/…/src/cs/graph/dijkstra.rs:105-109`), which NaN is not.
- `graph.paths` iterates the `HashMap` Dijkstra answers to build its rows
  (`dijkstra.rs:60-80`), so their order is the map's; `graph.allpath` and
  `graph.transitiveclosure` do the same over Johnson's and Warshall's.
  `algos`' Dijkstra refuses a negative weight it meets
  (`algos/…/src/cs/graph/dijkstra.rs:105-109`).
- **`graph.allpath`'s answer turns on the last distinct node listed**
  (F220). `algos`' Johnson takes the largest vertex as its extra source `q`
  (`algos/…/src/cs/graph/johnson.rs:41-44`), and nodes are numbered at
  their first appearance, after Title-casing (`make_graph.rs:92-97`), so
  `q` is the last *distinct* name: listed `[ :C :A :B :C ]`, it is `B`.
  `add_edge` replaces `q`'s own out-edges with its zero-weight ones
  (`johnson.rs:45-47`, `graph.rs:108-113`), and the potentials `h` come
  from Bellman–Ford over that graph (`johnson.rs:50-53`). An edge `q → v`
  is then reweighed as `w + h_q - h_v` with `h_q` = 0 (`johnson.rs:61-65`):
  below zero when `w < h_v`, which a negative `w` need not be, since `h_v`
  can itself be negative. Where it is below zero, Dijkstra refuses it
  (`dijkstra.rs:105-109`), and Johnson drops without a word the rows of
  every source whose search meets that edge (`johnson.rs:71-74`).
  `[ [ :A :B 1.0 ] [ :B :C 1.0 ] [ :C :A -0.5 ] ] [ :A :B :C ] graph!
  graph.allpath` answers `[]` in both engines, and all nine rows with the
  nodes listed `[ :C :A :B ]` or `[ :C :A :B :C ]`. Over `[ :A :B :C :D ]`,
  `[ [ :D :A -0.5 ] [ :A :B 1.0 ] ]` drops only D's rows. `[ [ :B :A -2.0 ]
  [ :C :A -1.0 ] ]` over `[ :A :B :C ]` answers in full, `C`'s negative
  edge notwithstanding. A negative cycle the potentials' Bellman–Ford finds
  is refused, `GRAPH.ALLPATH: returned: negative cycle detected in graph`
  (`johnson.rs:50-53`, `bellman_ford.rs:91-101`); only one through `q`'s
  replaced out-edges goes unseen. **And a graph with no cycle can be
  refused as one.** That Bellman–Ford runs over the graph in which `q` has
  a zero-weight edge to every node (`johnson.rs:38-47`), so a negative path
  into `q` closes a negative cycle there. `[ [ :A :C -1.0 ] ] [ :A :B :C ]
  graph! graph.allpath` answers `GRAPH.ALLPATH: returned: negative cycle
  detected in graph`, and four rows when listed `[ :C :A :B ]`. On a DAG
  this is the commonest way to meet F220. All of these were run in both
  engines. D200's ruling covers this face (**D203**, resolved A).
- The family is **core** under D14's partition: `bund/graph` is listed
  under `## core` (`docs/core-words.md:9`, the line at `:16`), not under
  `## library` (`:43`). So all seven words carry D14's 100% preservation,
  and each change below to one of them is a deviation in a core word.
  *(Until the third review this said "library", citing the same line; F17's
  note of 2026-10-18 had the same misreading.)*
- Bund2 reproduces all seven words (`crates/bund2-stdlib/src/graph.rs`),
  `graph.path`'s operand lookup since F218, and with F219's crash answered
  by the reference's own no-path error, `GRAPH.PATH: can not calculate
  path` (`getpath.rs:93`; D206) — or, from a node to itself, the path of
  that one node (D198). *(Until D206 F219's case answered `found=false`,
  which the reference never answers: `fast_paths` answers `None` for no
  path (`fast_paths-1.0.0/src/path_calculator.rs:198-199`) and every `Some`
  it answers is found (`:201-208`, `src/shortest_path.rs:84-86`), so
  `getpath.rs:76-77` cannot run.)*

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
| `graph.mst` | `<graph>` | MAP `edges` (`[ from to weight ]`), `weight`; edges read both ways, self-loops never chosen (`graph_algorithms.rs`, `mst`), negative weights taken; a forest when the graph is not connected; a total that overflows is refused (S4) |
| `graph.bellmanford` | `<start> <graph>` | `graph.paths`' rows (`dijkstra.rs:69-76`): `node`, `path` or NODATA, never an infinite distance (S4); each distance the nearest `f64` to its path's exact sum (S5, D207) |
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
leaves nothing there, and takes the graph from the workbench and leaves the
answer there: `(1, 1)` on the workbench's axis (RFC-0004 §S1, D195),
declared from how the form is built, since there is nothing to measure. So
`bund2 check` reports `:A graph.bfs.` run with no graph on the workbench.

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

**A node operand is trimmed and Title-cased** (`node_name`, `graph.rs`),
as the graph's own names are, so a
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
  the graph lists its edges — equal by `==`, so `-0.0` and `0.0` are
  equal and keep that order — and `graph.maxflow` its `edges` in that
  order;
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
  the smallest node of all the reachable cycles; for one program it is the
  same on every run (D193 §2). A cycle is negative exactly when its
  weights sum below zero, however far from the start it lies: the test is
  made with no rounding (D207, S5);
- `GRAPH.BELLMANFORD: the distance to C overflows`, naming the first node
  in node order whose distance cannot be represented; `GRAPH.MST: the total weight overflows`;
  `GRAPH.MAXFLOW: the flow overflows: S -> T` (D199). Every weight is
  finite, so a sum that is not overflowed, and **only an answer that
  overflows is refused** (D202). A sum that overflows on the way to a
  finite distance, or a running total whose total is representable, is
  answered. A negative cycle whose sums would run past `-1.0e308` is found
  and reported as the cycle it is. How: S5.
- `GRAPH.MAXFLOW: the source and the sink are one node: A`, checked before
  any capacity;
- `GRAPH.MAXFLOW: a capacity must not be below zero: A -> B weighs -1`,
  checked for **every** edge before the flow starts, self-loops and
  unreachable edges included.

The **shape** of the graph is checked first, in the reference's words, by
the `graph_operand` that `graph.paths`, `graph.allpath` and
`graph.transitiveclosure` use (`graph.rs`) and the twelve reach through
`operand`: `MISSED TYPE`, `unknown data type`,
`type casting returns: …`, `Nodes list must be a list`, `Edges list must
be a list`. Of the other four reference words, `graph.path` makes the same
checks inline (`getpath.rs:14-43`); `graph` and `graph.` do not check
`type` at all, taking a missing one as `"graph"` (`mod.rs:38-41`); and
`graph!` checks nothing (`mod.rs:68-93`). A graph operand that is not a
MAP is its own `type`, `nodes` and `edges` (`get_or_self`, in
`crates/bund2-value`). So `"graph"` passes the type check and answers
`Nodes list must be a list`, another STRING answers `unknown data type`,
any other value `type casting returns: …`, and `MISSED TYPE` is a MAP
without `type`. Run under `bund2` on 2026-10-22; the sixth review ran
`graph.paths` in both engines and found the same messages. Then the
reader, naming the word
and, where there is one, the type by name:

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

**What a refusal leaves** (the residual RFC-0004 calls "what F18 calls
the observable part", `docs/rfc/RFC-0004-stack-effects.md:544-546`): each
word pulls the graph,
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
reader refuses are each `Error::internal`. A sum that overflows is not an
invariant broken but the program's numbers, so it is an ordinary error
(S4, D199). In `graph.maxflow` only the total can overflow: each reverse
arc holds at most its forward edge's capacity, which is finite. That is
exact arithmetic's invariant, `r + a = w`; in `f64` the subtraction and
the addition each round, so it rests on that drift never reaching the gap
between a capacity and `f64::MAX`. It is not proved here. The sixth
review's model of `max_flow` over 20,000 graphs with fractional capacities
found no reverse arc above its capacity. And the total only grows, by a positive bottleneck each round, so a running total
that overflows is an answer that does (D202).

**`graph.bellmanford` relaxes exactly** (D207). Every finite `f64` is a
whole number of 2^-1074 units below 2^2098, so `Exact` holds a distance as
that number in two's complement over 2,176 bits, and no sum of up to 2^76
weights rounds or wraps; a graph that could need more relaxations is an
`Error::internal`, and none that fits in memory can. Every comparison is
then exact, so a cycle is reported exactly when one is negative, and each
distance is rounded once, at the end, to the nearest `f64`, ties to even.
Over whole weights that is the sum `f64` forms; over fractional ones it
can differ in the last bit from a left-to-right `f64` sum, toward the
correctly rounded one (`0.1`, `0.2`, `0.3` answer `0.6`, not
`0.6000000000000001`). It also does what D202's `Wide` fallback did: only a
distance whose rounded value does not fit an `f64` is refused (D202), and
the `-1.0e308` cycle is a cycle. *(From D202 to D207 the relaxation ran in
`f64`, falling back to `Wide`, an `f64` significand with its own exponent,
when a sum overflowed. `Wide` widened the range and not the precision, so a
zero-weight cycle 1000.0 from the start was reported and a −2⁻⁵² one 1e20
away was missed: the sixth review's finding 2.)* **What it costs**:
measured on macOS arm64, release, the relaxation of a 2,000-node chain
listed backwards, which needs every pass, takes about 100 ms against about
7 ms in `f64`; on random graphs of 400 nodes and 4,000 edges the
difference is within noise. `the_exact_run_scales_with_the_weights`,
`exact_sums_round_once_to_the_nearest_f64` and
`bellman_ford_agrees_with_an_exact_floyd_warshall_over_fractional_weights`
check it. `graph.mst` sums lightest first
from the first weight, so a forest of `-0.0` edges totals `-0.0` and an
empty one 0.0. Only if that sum overflows does it sum again, in an order
whose running sum never exceeds the largest weight in magnitude. That
fallback's total is in a different order, so its last bits are its own
(`bellman_ford`, `Exact`, `mst_total`).

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
(`tests/golden/PROMOTABLE.txt:1-6`). The palette (`KINDS` in
`crates/bund2-stdlib/src/lib.rs`) cannot build a graph: its `dict` has no
`type`, which every word that checks one refuses. So
all 24 of these names, like `graph.path`, `graph.paths`, `graph.allpath`
and `graph.transitiveclosure` and their forms, are `not reached` there and
not promotable. `graph` and `graph.`, which only check a MAP, are
promotable (`PROMOTABLE.txt:139-140`). Syncing costs speed and never
meaning.

## Which points are rulings

Accepting this RFC accepts the following. The owner ruled on these
(D192, D193, D198, D199, D200, D202, D203, D206, D207): the twelve words
and nothing more;
Bund2's own implementation; the graph read as `graph.paths` reads it,
directed; node order everywhere, and F17 fixed by it; values for answers
and errors for impossible questions; no tolerance in `graph.maxflow`; the
Bellman–Ford rule as built, its relaxation exact (D207); weights that are
not finite refused, and sums that overflow refused (D199) where the answer
overflows (D202); F218 fixed; `graph.path` answering the reference's own
no-path error where the reference crashes (F219, D206), and the path of
one node from a node to itself (D198); and `graph.allpath`'s dependence on
the last node preserved (F220, D200), its refusal of a graph with no cycle
included (D203).

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

- **The seven reference words** are core (D14), so each exception below
  is a deviation in a guaranteed word, approved by the ruling named.
  Preserved exactly, with three stated exceptions. F17's order is
  **deliberately changed** from unspecified to node order — an added
  guarantee, D192 §4, which deviates from nothing because the reference's
  order is unspecified. `graph.path`'s crash on a node no kept edge
  reaches is **deliberately changed** to `GRAPH.PATH: can not calculate
  path`, the reference's own answer for "no path" inside `fast_paths`' size
  (F219, D193 §5, D206), so "no path" has one shape; or to the path of one
  node when the start is the end (D198), which is what `fast_paths` answers
  for a node inside its size. `graph.path`'s operands
  are looked up as given again (F218, a Bund2 defect fixed, not a
  deviation). No golden moves.
- **Preserved, and a defect** (F220, D200): `graph.allpath` answers no
  rows from a source whose search meets an out-edge of the last distinct
  node that Johnson's reweighting leaves below zero ("Current behaviour"
  states the condition) — so with negative weights its answer depends on
  how `nodes` is ordered. A negative cycle is mostly refused; one through
  that node's replaced out-edges goes unseen. And a graph with **no**
  cycle is refused as "negative cycle detected" when a negative path runs
  into that node (F220's third face; D200 covers it, **D203** resolved
  A). Bund2 runs the same `algos` revision and keeps all of this;
  `graph.bellmanford` is the word for negative weights.
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
  - **negative weights**: `graph.paths` refuses one Dijkstra meets,
    `graph.allpath` drops rows or refuses a graph with no cycle as F220
    says, and `graph.bellmanford` takes it and refuses only a negative
    cycle the start reaches, decided exactly (D207), so a swap to it works
    one way: from an error or missing rows to an answer. Its distances are
    correctly rounded sums, so over fractional weights one can differ in
    the last bit from `graph.paths`' left-to-right sum;
  - **overflow**: `graph.paths` and `graph.allpath`, the two that sum in
    `f64`, answer `inf` where a sum overflows, and the new words refuse it
    (D199), and only there (D202): a graph whose distances are finite is
    answered, by `graph.bellmanford` as by `graph.paths`, whatever its
    paths' sums. `graph.path` reads each weight `as usize`, and over
    `[ [ :A :B 1.0e300 ] [ :B :C 1.0e300 ] ]` from A to C answers `can not
    calculate path` in both engines; the other four do not sum;
  - **edge order**: a re-given edge keeps its first place (S2);
  - **the `.` forms** answer to the workbench and refuse a missing operand
    as `Stack is too shallow …`, where the plain forms say `NO DATA #n`.
    Every other refusal of a `.` form is in the plain word's name:
    `7 dict . graph.bfs.` answers `GRAPH.BFS: MISSED TYPE`
    (`each_form_holds_the_graph_on_the_workbench`); only the two
    pre-checks name the `.` form.

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
F219 instead of `found=false`, which D206 later chose, as the reference's
own error, once it was shown that the reference never answers
`found=false`. D207 records stating the rounding, and confirming a
reported cycle exactly, against the exact relaxation.

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
   independent one over seeded random graphs. The generator draws **whole
   weights**, whose sums never round, so these say nothing about rounding;
   `bellman_ford_agrees_with_an_exact_floyd_warshall_over_fractional_weights`
   does, for Bellman–Ford (600 graphs of weights `k × 2^p` against
   Floyd–Warshall in `i128`, distances equal to the bit, a cycle reported
   exactly when one is negative, near-zero cycles included). The rest:
   `reachable_is_the_transitive_closure_row`,
   `scc_is_mutual_reachability`, `components_join_what_any_edge_joins`,
   `toposort_and_cycle_agree`, `bellman_ford_agrees_with_dijkstra`,
   `bellman_ford_agrees_with_floyd_warshall` (also: the node named is
   reachable from the start and is the least node of some negative cycle,
   among the nodes not below it — consistent with S4's rule, and weaker,
   since it does not check that the cycle is the one the walk back found),
   `mst_is_minimal` (at least 100 of its 200 graphs
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
   `a_small_capacity_beside_a_huge_one_still_carries_flow`,
   `ties_and_edges_follow_the_graph_order` (an unreachable sink), and
   `the_empty_graph_answers_as_s1_says` (S1's empty-graph paragraph, all
   five claims, `graph.mst`'s `0.0` with its sign).
7. **What a refusal leaves**: `a_refusal_leaves_what_s4_says` — plain and
   `.` forms, a refused graph leaving its node, and both `.` form
   pre-checks.
8. **F218 and F219**: `graph_path_looks_its_nodes_up_as_given`;
   `graph_path_answers_no_path_for_a_node_no_kept_edge_reaches` (the five
   graphs that crash the oracle, and the in-size graph beside the first,
   all answering `can not calculate path`, D206);
   `graph_path_from_a_node_to_itself_is_found_wherever_it_is_listed`
   (D198); and
   `graph_path_drops_an_edge_under_one_before_the_lowest_is_kept`.
9. **Depth**: a 200,000-node chain read by `read`, and the same chain
   closed into a cycle by adding the closing edge to the `Net` directly,
   through `dfs`, `scc`, `find_cycle` and `toposort` called directly, on
   a thread whose stack is set to 2 MiB
   (`a_deep_graph_does_not_overflow_the_stack`).
10. `cargo xtask conform` moves by zero across this RFC's commits — the
    figure itself moves with unrelated probes, and is not the criterion;
    clippy clean; the workspace's tests pass.
11. **Overflow, F220 and `-0.0`**: `a_sum_that_overflows_is_refused`
    (the three words; the negative cycle at `-1.0e308`, with and without
    an unrelated node, reported as a cycle; a distance below the range),
    `only_an_answer_that_cannot_be_represented_is_refused` and
    `mst_total_keeps_its_partial_sums_in_range`,
    `the_exact_run_scales_with_the_weights` and
    `exact_sums_round_once_to_the_nearest_f64` (D202, D207),
    `a_cycle_is_negative_exactly_when_its_weights_sum_below_zero` (D207),
    `allpath_with_a_negative_edge_out_of_the_last_node_answers_nothing`,
    `allpath_refuses_an_acyclic_graph_with_a_negative_path_into_the_last_node`
    (F220's third face), and
    `mst_keeps_equal_weights_in_listed_order_negative_zero_included`. The
    `.` forms' workbench half: `the_workbench_is_counted` in `check.rs`.

## Open questions

- **Operand casing across the family** (S2). D192 keeps Title-casing for
  the new words; the owner can reverse it, which would make every graph
  word look its operands up as given.
- **Caching a read graph** (S5): not proposed; it would need a derived
  form that a `set` on the MAP invalidates.

## The third review, answered (2026-10-20)

| Finding | Answer |
|---|---|
| 1. The family is core | Corrected under "Current behaviour" and in the preservation analysis; a dated note on F17 |
| 2. `graph.allpath` and the last node | F220; preserved by the owner's ruling (D200), stated under "Current behaviour" and in the preservation analysis; test in criterion 11 |
| 3. A node to itself | D198: the path of one node, weight 0.0, wherever the node is listed; criterion 8 |
| 4. Overflow | D199: the three summing words refuse, naming what overflowed; S4, S5, criterion 11 |
| 5. `-0.0` in `graph.mst` | Sorted by `partial_cmp`, so equal weights keep the graph's order; S3, criterion 11 |
| 6. Criterion 10 | Restated as a move of zero |
| 7. Smaller points | S4's wording on the cycle node; `make_fast_graph`'s citation; criteria 3 and 9 describe what they check; the NaN claim cited |

## The fourth review, answered (2026-10-21)

| Finding | Answer |
|---|---|
| 1. `graph.bellmanford` refuses finite distances | **D202**, A: a scaled second run when a sum overflows; only a distance that cannot be represented is refused, by its node; S4, S5, criterion 11 |
| 2. `graph.mst` refuses a representable total | **D202**, A: re-summed in an order that keeps the running sum in range; S5, criterion 11 |
| 3. `graph.mst` answers `-0.0` over the empty graph | A Bund2 bug against S1: the total is a fold from 0.0. `the_empty_graph_answers_as_s1_says` covers the paragraph's five claims and the sign; criterion 6 |
| 4. F220 stated too broadly | "Current behaviour" states Johnson's condition as `johnson.rs` computes it, the last *distinct* node, and the negative cycles it refuses; the preservation analysis narrowed; a dated note on F220. D200 stands |
| 5. "the `graph_operand` the seven share" | Corrected to the three that use it, with what the other four check |
| 6. RFC-0004's drifted citation | Cites F18's residual instead |
| 7. Smaller points | The Dijkstra citation is `:105-109` both times; "the seven answer `inf`" is the two that sum in `f64`, with `graph.path`'s case; a negative cycle near `-1.0e308` is now reported as a cycle (D202); S2 says trimmed as well as Title-cased; criterion 10 without figures; D192's note corrected in the register |
| Preservation: the `.` forms' refusals | Listed: every refusal but the two pre-checks is in the plain word's name |

## The fifth review, answered (2026-10-21)

| Finding | Answer |
|---|---|
| 1. `graph.allpath` refuses graphs with no cycle | "Current behaviour" and the preservation analysis state it; F220 note; test `allpath_refuses_an_acyclic_graph_with_a_negative_path_into_the_last_node` in both node orders; **D203** resolved A: D200 covers it, preserved |
| 2. D202's build kept an exception | Removed: the fallback runs in `Wide`, an exponent beside the `f64`, so any weight is taken; the test that pinned the old message now asserts the answer, for a tiny weight on a path, unreachable and on a self-loop, and the `-1.0e308` cycle with one; D202 note |
| 3. No cross-check of the fallback | `the_wide_run_scales_with_the_weights`: 1,200 seeded graphs scaled by `2^1020` with a subnormal self-loop, each answer the plain one scaled to the bit; and `wide_sums_round_as_unbounded_f64_would`; criterion 11 |
| 4. The `.` forms' workbench half | Declared `(1, 1)` from how they are built (`BUILT` in `bench.rs`), so `check` finds `:A graph.bfs.`; S1 cites D195; D195 note |
| 5. A `-0.0` forest totals 0.0 | `mst_total` folds from the first weight, so it totals `-0.0` again and the empty forest 0.0; tested; S5 |
| 6. Smaller points | The residual cited to RFC-0004 `:544-546`; the palette cited by `KINDS`; why `graph.maxflow`'s overflow is the answer's; the `graph.mst` fallback's last bits; the registers searched brought up to date; the stale doc comment went with the code it described |

## The sixth review, answered (2026-10-22)

| Finding | Answer |
|---|---|
| 1. F219's `found=false` rests on a false premise | **D206**, A: F219's case answers `GRAPH.PATH: can not calculate path`, the reference's own answer for "no path", so it has one shape; "Current behaviour", the rulings and the preservation analysis restated; criterion 8's test pairs it with the in-size case; F219 note |
| 2. `graph.bellmanford`'s cycle test depends on rounding | **D207**, C: the relaxation runs in `Exact`, with no rounding, and each distance is rounded once; it replaces `Wide`; S4, S5, criterion 11; the cost measured in S5 |
| 3. The cross-checks draw whole weights | Criterion 3 says so; `bellman_ford_agrees_with_an_exact_floyd_warshall_over_fractional_weights` checks fractional weights against `i128`; `exact_sums_round_once_to_the_nearest_f64` checks the rounding |
| 4. Smaller points | The Status line restated; the rulings list names D202, D203, D206 and D207; S4 names the first overflowing node in node order, and what a graph operand that is not a MAP is refused as |
| Assumed: reverse arcs within capacity | Stated in S5 as resting on rounding drift, unproved, with the review's model |
| Assumed: no word named like the 24 | A program's own definition replaces a native, so `:graph.bfs { "mine" } register` makes `graph.bfs` answer `mine`, as it would for any native; run under `bund2` on this date |
