# logic_demo

The seven worked examples of RFC-0013, logic programming, one after
another: facts and a rule asked three ways, bridges into Bund, a graph word
as a generator, a predicate made into a word, an expert system, tabling and
saturation, and SEND + MORE = MONEY with CLP(FD).

    make            # run demo.bund
    make check      # run it and compare the output with expected.txt

or, without `make`, from this directory:

    bund2 demo.bund

It reads nothing from the terminal. Example 5 asks its questions through
an `ask` lambda that calls `input`, but all five replies are seeded in
`known`, so the lambda never runs.

## The output

`expected.txt` is what a run prints, and each line of it is the answer the
example's comment in the RFC gives. A test in `crates/bund2-cli/tests`,
`logic_demo.rs`, runs the demo with nothing on standard input and compares
the two. It is not a golden: the reference has none of the `kb.` words, so
there is no oracle to capture one from.

## A knowledge base

A KB is a LIST of clauses made into a MAP by `kb!`:

    :family [
      [ parent tom bob ] [ parent bob ann ]                    // facts
      [ [ grandparent X Z ] ← [ parent X Y ] [ parent Y Z ] ]  // a rule
    ] kb! var

- **A capitalised name is a variable**, any other is an atom, which
  answers as a STRING.
- **A rule is a LIST** whose first item is its head, then `←`, then its
  goals.

## The examples

| | What it shows | Words |
|---|---|---|
| 1 | Every solution, whether there is one, and a lambda run for each | `kb.query`, `kb.holds`, `query` |
| 2 | Bund inside a rule: a test that keeps a solution, and a value bound from a lambda | `test`, `is`, `kb.once` |
| 3 | A Bund word as a generator: a variable bound to each node a graph reaches | `in`, `graph.reachable` |
| 4 | A predicate made into a word, NODATA (`∅`) standing for an unbound argument | `kb.word` |
| 5 | An expert system that asks what no clause proves, with remembered replies | `kb.askable`, `query`'s `ask` and `known` |
| 6 | A left-recursive predicate, tabled; then every fact the rules derive | `kb.table`, `kb.saturate`, `kb.clauses` |
| 7 | A puzzle solved by constraint propagation and labeling | `ins`, `all_different`, `#=`, `#≠`, `label` |

Example 7 also asks for every solution, and there is one.
