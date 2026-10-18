# graph_demo

Every graph word, over a few small graphs: build tasks, roads between
towns, ferries between islands and water mains. That is the reference's
seven — `graph!`, `graph`, `graph.`, `graph.path`, `graph.paths`,
`graph.allpath` and `graph.transitiveclosure` — the twelve Bund2 adds
(D192, RFC-0012), every word's `.` form, and `io.graph`, which draws a
chart. It ends by keeping a graph in a file and reading it back.

    make            # run demo.bund
    make twice      # run it twice and compare the output

or, without `make`, from this directory:

    bund2 demo.bund

It writes one file, `roads.graph`, in the directory it runs from;
`make clean` removes it.

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

From the reference:

| word | stack | answers |
|---|---|---|
| `graph!` | `<edges> <nodes>` | the graph MAP; never fails, leaving what is not a list where it was |
| `graph` | `<map>` | the MAP as a graph: `type`, `nodes` and `edges` filled in, and checked |
| `graph.` | | the same, on the workbench |
| `graph.path` | `<end> <start> <graph>` | `{ found path weight }`: one shortest path |
| `graph.paths` | `<start> <graph>` | the distance to every node, by Dijkstra |
| `graph.allpath` | `<graph>` | every pair's distance, by Johnson |
| `graph.transitiveclosure` | `<graph>` | every pair a path connects |

`graph.path` and `graph.paths` look their nodes up exactly as given, so
they want `"Ashby"`, not `:ashby`.

Bund2's own:

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

Each word that asks a question has a `.` form that takes the graph from
the workbench and leaves the answer there; a node stays on the stack:
`<graph> . :fetch graph.bfs. take` brings the answer back.

And one more that is not a graph word: `io.graph` takes a list of FLOATs
and answers a text chart of them; `io.graph.` takes the list from the
workbench.

## What the demo shows

**Making a graph.** `graph!` from two lists; `graph` on a bare dict and
on one built by hand with `set`; `graph.` on the workbench; `graph!` on
something that is not a list, which it leaves on the stack; and `graph`
refusing a MAP whose `nodes` is not a list.

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

Then the reference's `graph.path` finds the shortest way from Ashby to
Elm (19 km, by Cliff and Dale) and refuses `:ashby`; and over one-way
ferries, `graph.allpath` gives every pair's distance and
`graph.transitiveclosure` every pair a trip connects.

**Water mains.** 14 units a second reach the town: the two mains into it
carry 4 and 10, and nothing more can get through them.

    { edges=[ … ] ::  flow=14.0 :: }

**The workbench.** Every question again, through its `.` form, with the
same answers.

**A graph in a file.** A graph is an ordinary MAP, so it travels as any
value does. `encode.base64` turns it into Bund's binary form and that
into one line of text, `file.write` keeps the line in `roads.graph`,
`io.textfile … car` reads it back, `decode.base64` restores the MAP, and
`graph` checks it before it is asked anything. `==` cannot compare two
MAPs, so the demo compares what they answer. Last, `io.graph` charts the
distance from Ashby to each town, taken from `graph.bellmanford`'s rows
with `{ :path get } map`:

     19.00 ┤   ╭
     13.00 ┤  ╭╯
      9.00 ┤ ╭╯
      7.00 ┤╭╯
      0.00 ┼╯

(The chart above is shortened; the real one has a row for every unit.)

## Writing Bund like this

- `graph!` takes the edges first and the nodes second: the nodes list is
  on top.
- `:name <value> var` keeps a graph to ask several questions of, and
  `:name var?` reads it back; a bare name does not.
