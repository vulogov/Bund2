# logic_programming

The [Zebra Puzzle](https://en.wikipedia.org/wiki/Zebra_Puzzle), solved
twice with Bund2's logic programming (RFC-0013): once by searching over a
list of houses, as a Prolog program would, and once by constraint
propagation with CLP(FD). Each part of the program is written in its own
namespace, a named stack.

    make            # run zebra.bund
    make check      # run it and compare the output with expected.txt

or, without `make`, from this directory:

    bund2 zebra.bund

## The puzzle

Five houses stand in a row, each a different colour, and in each lives a man
of a different nation, with a different pet, drink and brand of cigarette.
Fifteen clues, listed at the top of `zebra.bund`, fix all of it. Who drinks
water, and who owns the zebra?

    the japanese owns the zebra, the norwegian drinks water

## Namespaces: named stacks

`@name` switches to the stack called `name`, creating it on first use, and
the program runs there until the next `@name`. Each part has its own:

| Stack | What it holds |
|---|---|
| `@lists` | the list predicates the search needs: `member`, `left_right`, `next_to`, `nth` |
| `@search` | the puzzle as a search, merged with `@lists` |
| `@constraints` | the puzzle as constraints over house numbers |
| `@answers` | each method's answer, moved there to be compared |
| `@main` | where the output is printed |

A knowledge base is written as clauses pushed onto its own stack, and
`kb.consult*` turns everything on that stack into one KB, so a stack is the
namespace a group of clauses is written in. `kb.merge` then joins two of
them: `@search` uses `@lists`'s predicates without redefining them. A value
reaches another namespace with `:name move`, which moves the top of the
current stack onto the stack called `name`; that is how both answers reach
`@answers`, and how their comparison comes back to `@main`.

## Two ways to solve it

**By search** (`@search`). The street is a list of five houses, each
`[ h Nation Colour Pet Drink Smoke ]`, with a `_` for whatever is not yet
known. Every clue is a goal about that list:

    [ member [ h englishman red _ _ _ ] Hs ]                       // clue 2
    [ left_right [ h _ ivory _ _ _ ] [ h _ green _ _ _ ] Hs ]      // clue 6
    [ next_to [ h norwegian _ _ _ _ ] [ h _ blue _ _ _ ] Hs ]      // clue 15

Unification fills a house in as each clue is met, and backtracking undoes
it when a later clue fails.

**By constraints** (`@constraints`). Here each of the 25 names is the
number, 1 to 5, of the house it belongs to. Each group of five is `ins` 1
to 5 and `all_different`, and each clue is an equation between house
numbers:

    [ #= English Red ]                           // clue 2
    [ #= Green [ + Ivory 1 ] ]                   // clue 6
    [ #= [ abs [ - Norwegian Blue ] ] 1 ]        // clue 15

Propagation narrows the domains as each clue is posted, and `label` searches
what is left.

Both find exactly one solution, and the program checks that they agree.

## What it shows

| Feature | Where |
|---|---|
| Facts, rules, lists with `[ H \| T ]`, anonymous `_` | `@lists`, `@search` |
| A KB built from a stack, and two KBs merged | `kb.consult*`, `kb.merge` |
| Every solution, the first one, and a lambda run for each | `kb.query`, `kb.once`, `query` |
| A bridge: a Bund lambda inside a rule | `is` in `nth` |
| A predicate made into a word, with `∅` for an unknown | `kb.word` on `owns_zebra` |
| Constraints and labeling | `ins`, `all_different`, `#=`, `abs`, `label` |
| Named stacks as namespaces | `@name`, `move` |

## The output

`expected.txt` is what a run prints. A test in `crates/bund2-cli/tests`,
`logic_programming.rs`, runs the example with nothing on standard input and
compares the two. It is not a golden: the reference has none of the `kb.`
words, so there is no oracle to capture one from.
