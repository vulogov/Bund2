# Release notes

## 0.53.0

The release that adds logic programming. RFC-0013 was drafted, reviewed
eleven times, accepted and built in thirteen milestones, and every reading
its milestones made is filed and ruled (D226–D243). Two classes join the
built-in hierarchy.

**Health.** conform **143/159**, CEILING **143/159**: at the ceiling, with
and without the Cranelift tier. It was 146/159 at 0.52.0; the three goldens
D225 accepted as approved deviations account for the difference, and no
golden regressed. COVERAGE 577/602 in-scope words, IMPLEMENTED 597/602. The
decisions register holds 243 entries, none OPEN.

### Logic programming (RFC-0013)

A knowledge base is a value: a LIST of clauses made into a MAP, asked by
words, with Bund running inside rules through bridges.

    :family [
      [ parent tom bob ] [ parent bob ann ]
      [ [ grandparent X Z ] ← [ parent X Y ] [ parent Y Z ] ]
    ] kb! var
    :family var? [ grandparent tom W ] kb.query

- **Building and editing:** `kb.new`, `kb!`, `kb.consult*`, `kb.assert`,
  `kb.asserta`, `kb.retract`, `kb.merge`, `kb.clauses`, `kb.inferences`,
  with `.` forms for the workbench.
- **Asking:** `kb.query` (every solution), `kb.once`, `kb.holds`,
  `kb.call`, and the `query` conditional, which runs a lambda for each
  solution. `kb.word` makes a predicate a word, with NODATA (`∅`) for an
  argument to find.
- **The engine:** one loop with no Rust recursion, answering one solution
  at a time; unification with the occurs check always on; first-argument
  indexing; `true`, `fail`, `=`, `not`, `or`, `if`, `once`, `call`,
  `findall` and `in`. An inference cap and the memory guard reach inside
  a query.
- **Bridges:** `[ test λ ]` keeps a solution when a Bund lambda answers
  true, `[ is X λ ]` binds a variable to what one answers, and `in` ranges
  over the LIST a lambda answers, so a Bund word such as
  `graph.reachable` can be a generator.
- **Proofs:** `kb.prove` answers each solution with the proof tree that
  found it.
- **Asking the user:** `kb.askable` names predicates whose unproved goals
  go to an `ask` lambda, with replies remembered in `known`.
- **`dif/2`**, a constraint that two terms stay different.
- **Tabling:** `kb.table`, so a left-recursive predicate terminates.
- **Forward chaining:** `kb.saturate` answers the KB with every fact its
  rules derive.
- **CLP(FD):** `in`, `ins`, `fd_dom`, `#=`, `#≠`, `#<`, `#>`, `#<=`, `#>=`,
  `all_different`, `label` and `labeling`, over integer expressions with
  `+`, `-`, `*`, `abs`, `min` and `max`, by bounds propagation.
- **Answers** list the variables an answer leaves aliased to one another,
  under `_aliases` (D238), and constraints still pending, under
  `_residual`.
- **Diagnostics:** a failure inside a bridge is placed at the bridge's
  source line, and its backtrace frame names the goal it was solving
  (D240, D241); the profiler reports calls, retries and inferences per
  predicate (D240); `kb.word` warns when it replaces a word it made at
  another arity (D239).

**Performance** (`docs/measurements/`): naive reverse runs at about 2.9
million inferences a second. A variable that occurs once in a clause head
skips the occurs check (D237), which took naive reverse of 120 elements
from 3.24 s to 1.74 s. A clause is compiled only when a call first tries
it (D242), which took one small query of a 10,000-clause predicate from
1.0 ms to 168 µs.

**Changes a program can see** beyond the new words: a hand-built KB (a MAP
not made by `kb!`, `kb.assert` or `kb.consult*`) holding a malformed clause
has it refused by the first call that tries it, not by its predicate's
first call (D242). A float first argument is not an index key (D243).

### Classes (D225)

- `Integer` and `Float` are constructor words that take an operand.
- `Boolean` sits above `Bool`, and `Collection` above `List`, each with
  core methods.
- Three goldens are accepted as approved deviations as a result.

### Examples

- `bund_examples/logic_demo/` runs RFC-0013's seven worked examples
  unattended: facts and rules, bridges, a graph word as a generator,
  `kb.word`, an expert system, tabling and saturation, and SEND + MORE =
  MONEY.
- `bund_examples/logic_programming/` solves the Zebra Puzzle twice, by
  search and by CLP(FD), with each part written on its own named stack.
- `bund_examples/object_oriented/` shows D225's new classes.

Each has an `expected.txt`, and a test compares a run with it.

### Fixed

- **F231:** `scoped_call` left the named stack current when the memory
  guard refused its frame; it now unwinds and restores the caller's stack.

### Tooling

- The golden runner's timeout is 180 s, up from 60. It detects hangs; one
  golden sleeps about 52 s by design.
- `bund2-bench` gains a `kb` bench, with two measurement-only features,
  `kb-unchecked-bench` and `kb-unpolled-bench`, which no shipped build
  enables (D236).
