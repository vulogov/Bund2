# graph_demo

The twelve graph words Bund2 adds to the reference's seven, over three
small graphs: build tasks, roads between towns, and water mains. The words
are Bund2's own (D192, RFC-0012); the reference has `graph!`, `graph`,
`graph.path`, `graph.paths`, `graph.allpath` and
`graph.transitiveclosure`, and Bund2 has those too.

    make            # run demo.bund
    make twice      # run it twice and compare the output

or, without `make`, from this directory:

    bund2 demo.bund

## A graph

A graph is a MAP of `nodes` and `edges`, which `graph!` builds from two
lists, the edges first:

    [ [ :fetch :configure ] [ :configure :compile ] ]   // edges: from, to, weight
    [ :fetch :configure :compile ]                      // nodes
    graph!

- **Edges are directed.** A two-way road is two edges.
- **An edge weighs 100.0 unless it says otherwise**, and a weight must be
  a FLOAT: `[ :a :b 7.0 ]`, not `[ :a :b 7 ]`.
- **Names are Title-cased**, so `:fetch` comes back as `Fetch` and
  `:water_main` as `Water Main`. A node handed to one of the new words is
  Title-cased too; the reference's `graph.path` and `graph.paths` look it
  up as given, so the demo passes `graph.paths` `"Ashby"`.
- **Every answer is in the order of the `nodes` list**, so the output is
  the same on every run. The reference's own `graph.paths`, `graph.allpath`
  and `graph.transitiveclosure` answer in an order that changes (F17);
  Bund2's are sorted the same way as the new words.

## The words

The graph goes on top; a node, when the word takes one, beneath it.

| word | stack | answers |
|---|---|---|
| `graph.toposort` | `<graph>` | every node, each before the nodes its edges lead to |
| `graph.reachable` | `<node> <graph>` | every node a path from `node` reaches, `node` included |
| `graph.bfs` | `<node> <graph>` | the nodes reached from `node`, breadth first |
| `graph.dfs` | `<node> <graph>` | the same, depth first |
| `graph.neighbors` | `<node> <graph>` | the nodes one edge out of `node` |
| `graph.degree` | `<node> <graph>` | `{ in out }`: how many edges come in and go out |
| `graph.components` | `<graph>` | the nodes in groups joined by edges either way |
| `graph.scc` | `<graph>` | the nodes in groups where each reaches every other |
| `graph.cycle` | `<graph>` | one cycle, `[ A B C ]` for A→B→C→A, or `[]` |
| `graph.mst` | `<graph>` | `{ edges weight }`: the lightest edges joining everything, every edge read both ways |
| `graph.bellmanford` | `<node> <graph>` | the distance to every node, as `graph.paths` answers it, negative weights allowed |
| `graph.maxflow` | `<sink> <source> <graph>` | `{ flow edges }`: the most that can flow, each weight a capacity, and each edge's share |

Each has a `.` form that holds the graph on the workbench:
`<graph> . :fetch graph.bfs.` leaves the answer there.

## What the demo shows

**Build tasks.** An order to run them in, what a change to `compile`
reaches, the two searches from `fetch`, and the groups: `docs` depends on
nothing and stands alone.

    an order to run them in:   [ Fetch ::  Configure ::  Compile ::  Link ::  Test ::  Package ::  Docs :: ]
    groups of related tasks:   [ [ Fetch ::  Configure ::  Compile ::  Link ::  Test ::  Package :: ] ::  [ Docs :: ] :: ]

**A mistake: package before configure.** That closes a loop.
`graph.cycle` finds it, `graph.scc` shows which tasks wait on each other,
and `graph.toposort`, which has no order to give, fails — an error the
program catches with `?try`:

    a cycle:                   [ Configure ::  Compile ::  Link ::  Package :: ]
    no order: Lambda content evaluation returned error: GRAPH.TOPOSORT: the graph has a cycle through Configure

**Roads.** The lightest set of roads joining all five towns, 20 km of
them, and the distances from Ashby by the reference's Dijkstra and by
Bellman–Ford, which agree. A road with a rebate is a negative weight,
which only Bellman–Ford takes; two roads that pay in a loop would pay
without end, and that is an error naming a town on the loop:

    a loop that pays: Lambda content evaluation returned error: GRAPH.BELLMANFORD: Ashby reaches a negative cycle through Brook

**Water mains.** 14 units a second reach the town: the two mains into it
carry 4 and 10, and nothing more can get through them.

    { edges=[ … ] ::  flow=14.0 :: }

## Writing Bund like this

- `graph!` takes the edges first and the nodes second: the nodes list is
  on top.
- `:name <value> var` keeps a graph to ask several questions of, and
  `:name var?` reads it back; a bare name does not.
