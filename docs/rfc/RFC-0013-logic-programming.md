# RFC-0013: Logic programming — knowledge bases, unification and resolution

- Status: **Draft — revision 7 (2026-10-28).**
  - Revision 5 carried every correction from the pre-RFC review and three
    adversarial reviews (`reviews/RFC-0013-review-2026-10-28.md`, `-2.md`
    and `-3.md`, each "Not ready for Accepted").
  - Revision 6 answers the fourth review, the first on file
    (`reviews/RFC-0013-review-2026-10-28-4.md`: 1 blocking, 12 should-fix,
    8 notes). Revision 7 answers the fifth (`-5.md`: no blocking finding,
    4 should-fix, 3 notes).
  - Appendix A maps each finding to the place that answers it.
  - **D226 (L1–L15) is RESOLVED** (2026-10-28, "all as recommended"):
    every ruling as this text is written, L11 deferred. The fifth review
    found nothing else blocking. What remains is the owner's acceptance.
  - **F231**, a Bund2 defect found along the way, was filed apart and is
    fixed (`6e900f8`).
  - *Landed 2026-10-28 on the owner's "land it as a draft with D226".*
    The draft was written in a session whose files never reached this
    repository, so it arrived as text. Three changes were made on landing,
    and only these:
    - Its rulings were filed as D225, which D225 (`Boolean` and
      `Collection`) had taken by then; they are **D226** throughout.
    - Its citations were checked on landing, against `reference/` at the
      pinned SHA and against Bund2 at `0c145de`. A `path:line` into
      `crates/` became a backticked symbol, as `cargo xtask cite` requires
      (Q30), and a line that did not say what the text claims is corrected
      where it stands, with a dated note.
    - **Missing, and so unverifiable:** the three reviews named above, the
      pre-RFC review, the research document `06-logic-programming.md`
      (which the header says this RFC supersedes in part) and the ERRATA
      entry for it. None is in the repository. Appendix A is the draft's own account
      of those reviews and is kept as written.
- Depends on:
  - RFC-0001: values, `set` rebuilding a payload, key equality and
    identity.
  - RFC-0002: natives, effects, `register_lambda` and `register_var`.
  - RFC-0003: list literals lowered as data, and `scoped_call`.
  - RFC-0004: effects, `opaque`, and `bund2 check`.
  - RFC-0005: which natives Tier 1 crosses.
  - RFC-0012: a family of Bund2's own over a MAP; the graph family the
    examples use.
- Bears on:
  - RFC-0010: class names, which are Title-cased (S14).
  - RFC-0006: bundles.
  - RFC-0007: the bus.
  - RFC-0008: the debugger.
  - RFC-0011: the profiler.
- Decisions consumed:
  - D5, D12, D13, D14, D18, D20, D22, D24, D28, D32, D34, D35, D36, D37,
    D38;
  - D39 and its notes of 2026-09-11 and 2026-09-12;
  - D41, D47, D48, D55, D71, D72, D73, D79, D84, D87, D88, D111, D171,
    D181, D182, D191, D192, D195, D199, D202, D211, D212.
- Dispositions consumed: F57, F62, F63, F110, F116, F118, F140, F158, F218.
- **Rulings: D226, L1–L15**, entered OPEN in
  `docs/registers/decisions.md` and **RESOLVED** by the owner on
  2026-10-28, all as recommended.
  - The design is written against each recommended option, so no passage
    changes.
  - Every dependent passage is marked **[L*n*]**.
- Registers searched by subject:
  - These found no entry: "prolog", "unif", "backtrack", "datalog", "logic
    var", "expert system", "inference engine", "rete".
  - These found the decisions and dispositions listed above: "opaque",
    "re-enter", "palette", "conditional", "sqlite", "csv", "workbench",
    "cache", "payload", "nodata", "fold", "var", "register", "scoped",
    "recursion", "identity", "intern", "stack name", "switch", "memguard",
    "OOM", "container", "BUILT", "bus", "send", "decode", "stamp",
    "bundle", "profile", "breakpoint", "Title", "casing", "summary",
    "stdin", "test build", "overflow", "wire depth", "noeval", "cycle",
    "save.stacks".
  - ERRATA has no logic entry. The research document one would correct
    was never committed (below), so none is owed.
- Reference SHA: `reference/Bund` 21b40b0. Bund2 read at `0c145de` on
  landing, and at `2ebe0c4` for revision 6.
- Supersedes: nothing in this repository. The draft named the research
  document `06-logic-programming.md` §4.2–§4.5, §7 and §8 (L3, L3a, L5),
  per an ERRATA entry; neither was ever committed, so there is nothing to
  supersede.

---

## Summary

- **A knowledge base is a MAP of `type = "kb"`**, as a graph is a MAP of
  `type = "graph"`. It holds clauses written as Bund list literals, grouped
  by predicate.
- **A family of `kb.` words** builds knowledge bases, edits them and asks
  them questions. The questions are answered by unification and SLD
  resolution, on an engine that keeps all its search state on the heap.
- **One conditional type, `query`,** runs a lambda once per answer, as
  `sqlite` runs one per row.
- **Rules call Bund** through three bridge goals, each of which runs a
  lambda on a scratch stack.
- **Bund calls rules** through the words `kb.word` registers.
- **The work comes in three phases:**
  1. the pure core, the bridges and the words;
  2. the expert-system layer: proof trees, questions to a user with their
     replies remembered, `dif`, tabling and forward chaining;
  3. CLP(FD).
- **Nothing in Bund's syntax, value types or wire format changes.** Every
  word is Bund2's own, so `conform`, `COVERAGE` and `IMPLEMENTED` move by
  zero (D72).

---

## Motivation

Bund cannot state a rule and ask what follows from it.

- It has no logic variable, no unification, no backtracking and no clause
  store.
- The `?`-family branches on one value.
- The graph family answers fixed questions about one relation (RFC-0012).

So three kinds of question are written today as loops over MAPs:

- reachability under a recursive rule;
- a diagnosis from symptoms;
- a schedule under constraints.

The language already holds most of what logic programming needs:

| Need | Bund already has | Where |
|---|---|---|
| Terms | A list of names is a quoted term: it is lowered as data and pushed unevaluated | `lower_into` in `crates/bund2-syntax/src/lib.rs`; `apply_step` in `crates/bund2-interp/src/lib.rs` |
| Variable names | A name may begin uppercase or with `_` | `reference/bund_language_parser/bund.pest:28,36`; `is_element` in `crates/bund2-syntax/src/lib.rs` |
| Control as data | CONDITIONAL values | the module header of `crates/bund2-stdlib/src/conditional.rs` |
| Words from data | `register` | `Vm::register_lambda` in `crates/bund2-api/src/lib.rs` |

Triska makes the case for the rest:

- **Rules as data, read by an interpreter.** "Separating the knowledge base
  from the way it is interpreted allowed us to add features while leaving
  the inference rules unchanged"
  (https://www.metalevel.at/prolog/expertsystems). Phase 2 is that.
- **A pure, monotonic core.** "Adding a constraint can at most *reduce* the
  set of solutions" (https://www.metalevel.at/prolog/purity). So Phase 1
  has no cut, no assert inside a goal, and no `var/1`.

Both quotations were read through a fetch tool that summarises pages
(Q44).

---

## Current behaviour

This is the contract the design must keep. Every claim was read for this
RFC or for one of its reviews.

### Syntax

- **List literals.** A list literal is `"[" ~ term+ ~ "]"`
  (`reference/bund_language_parser/bund.pest:32`).
  - There is no empty list literal, and no empty block (D72).
  - Bund2 lowers the items in place into a LIST (the `Term::List` arm of
    `lower_into`, `crates/bund2-syntax/src/lib.rs`).
- **What each token lowers to**, by the arms of `lower_into`:
  - A name lowers to a CALL (`Term::Name`).
  - An atom lowers to a STRING (`colon_or_atom`; `Term::Str`; F63).
  - `@name` lowers to a CONTEXT (`Term::Stack`).
  - A `( … )` block lowers in place: CONTEXT, then its items, then
    `endcontext` (`Term::Ctx`, D34).
- **Name characters.**
  - `is_element` admits `?`, `#`, `=`, `|` and `_`.
  - `$` is a name character, through `is_symbol`.
  - `is_symbol` admits U+2000–U+2BFF.
  - A name ends at whitespace or at end of input (the parser's `name`).
- **What the first review's probe showed.**
  - Inside a list literal these are names: `|`, `←`, `<-`, `∅`, `_`, `#=`,
    `≠`, `nil`, `_X`.
  - `[ a b]` is a parse error.
  - `[ X| T ]` lexes `X|` as one name.
  - `[H|T]` is a parse error.

### Evaluation

- **Only CALL and CONTEXT act in a body.**
  - `apply_step` dispatches CALL and CONTEXT, and pushes every other value
    unevaluated (`crates/bund2-interp/src/lib.rs`).
  - A CONTEXT switches stacks (its `bund2_value::CONTEXT` arm).
- **Pushed heap values are tagged.** A heap value pushed to a stack is
  tagged with the stack's name (D41, `with_tag` in
  `crates/bund2-interp/src/stacks.rs`). `with_tag` keeps identity.
- **Aliases.**
  - `|` and `∅` alias `nodata` (`register_alias` in
    `crates/bund2-stdlib/src/values.rs`).
  - `<-` and `←` alias `stacks_left` (`register_alias` in
    `crates/bund2-stdlib/src/stack.rs`).
- **Booleans.** `true` and `false` are words, not literals (the module
  header of `crates/bund2-stdlib/src/logic.rs`). Inside a list literal they
  are CALLs.
- **Comparisons take the top of the stack as the left operand.** "`3 5 <`
  therefore asks whether `5 < 3`" (the same header).
- **`len`** is `eff(1, 2)` (its `register_native` in
  `crates/bund2-stdlib/src/singles.rs`).
- **The house underflow text** is unprefixed:
  `Stack is too shallow for inline var` (`singles.rs`).

### Conditionals

- **Dispatch.**
  - A constructor declares its type through `Leaves::Conditional` (D181,
    `register` in `crates/bund2-stdlib/src/conditional.rs`).
  - `!` reaches `Vm::run_conditional` (`crates/bund2-interp/src/lib.rs`),
    which runs the registered `ConditionalFn` (`register_conditional` in
    `crates/bund2-api/src/lib.rs`).
- **`sqlite`** pushes a MAP per row and runs `lambda` (`run_sqlite` in
  `crates/bund2-stdlib/src/data.rs`).
  - A missing `lambda` is empty.
  - A negative `skip_first` skips every row (its doc comment).
- **`?try`** reifies a failure as an `error` conditional, with the text in
  `context` (`run_tryexcept` in `conditional.rs`).
- **`raise`** fails with its operand (`raise`).
- **`bund2 check`** follows `csv` and `sqlite` with `run_repeatedly` (the
  `"csv" | "sqlite"` arm in `crates/bund2-stdlib/src/check.rs`), and
  abandons any other type (the `other` arm beneath it).

### Re-entering evaluation

- **A word that runs a body is opaque.** Such a word is
  `StackEffect::opaque`, as `map` is (`map_word`'s registration in
  `crates/bund2-stdlib/src/seq.rs`). "A **non-opaque** native never
  re-enters evaluation" is load-bearing (the comment above
  `REPORTS_MID_BODY` in `crates/bund2-stdlib/src/promotable.rs`).
- **Two source scans check this.** Each scan of `bund2-stdlib` reports the
  file and the enclosing function of every match. It asserts equality with
  a hand-kept list (`honesty_tests` in `crates/bund2-stdlib/src/lib.rs`):
  - `REENTERING` covers `eval_lambda`, `apply` and `scoped_call`
    (`every_reentering_function_is_named`).
  - `SWITCHES` covers stack switches and drops (D73, F140;
    `every_native_that_changes_the_current_stack_is_named`, whose patterns
    include `.drop_stack(`).
- **`bund2 check`** has arms for `map` (`check.rs`) and for `register`,
  which records the body it binds.

### `scoped_call` and stack names

- **The ordinary path.**
  - `scoped_call` switches to the named stack, creating it
    (`crates/bund2-interp/src/lib.rs`; `to_stack` and `add_as_current` in
    `stacks.rs`).
  - It then pushes a frame carrying `ExitAction::ToStack(prev)`. That frame
    restores the stack on success, and on an error in the body.
- **F231, fixed.** D171's guard could fail `push_frame_once` before that
  frame existed (its `memguard::take` poll), which left the named stack
  current. `context` shares the path (`run_context` in `conditional.rs`).
  Since `6e900f8`, `scoped_call` unwinds to its floor when `push_frame`
  fails and switches back by hand if the caller's stack is still not
  current; `a_trip_refusing_a_scoped_call_leaves_the_stack_where_it_was`
  (`crates/bund2-runtime/tests/oom_guard.rs`) holds it.
- **Reading a stack that is not current.** `Vm::pull_from` pulls from a
  stack by name, and answers nothing for a stack that does not exist
  (`pull_from` in `crates/bund2-interp/src/lib.rs`). `drop_stack` goes to
  `drop_named`, which changes nothing for a name that is not there.
- **Dropping a stack.** A stack that is not current is removed with no
  epoch bump and no change to the ring's order (`drop_named` in
  `stacks.rs`).
- **The tier, positions and breakpoints.**
  - A `scoped_call` body is never offered to the tier (its comment on the
    exit action). Bodies run inside it through `eval_lambda` may be
    (`push_frame_once`'s `exit.is_none()` test).
  - Source positions are found by payload key (`call_site`).
  - Breakpoints match by word name (`breakpoint_hit`).
- **Stack names last as long as the thread.**
  - `intern_stack_name` keeps a thread-local `Vec` that only grows and is
    searched linearly (`crates/bund2-value/src/lib.rs`).
  - An interned name returns its existing index.
  - `fresh_name` mints a new id on every call.
- **A program may create a stack under any name.** `load.stacks` restores
  stacks by name (`load_stacks` in `crates/bund2-stdlib/src/world.rs`;
  `write_stacks` saves them).

### Words, variables, values

- **Registering words and variables.**
  - `register_lambda` fires `watch redefine` (D191) and records the
    definition site (`crates/bund2-interp/src/lib.rs`).
  - `register` warns on binding `endcontext` (F62) and is in
    `REPORTS_MID_BODY` (`promotable.rs`).
  - `Vm::register_var` binds a variable (`crates/bund2-api/src/lib.rs`).
- **Key equality** (`impl PartialEq for BundValue`,
  `crates/bund2-value/src/lib.rs`):
  - `1` equals `1.0` (`int_eq_float`).
  - NaN equals itself, and −0.0 equals 0.0 (the module header, "Two
    equalities").
  - TIME compares by content (`Payload::Time`).
  - Other heap values, apart from strings and complex numbers, compare by
    **identity** (`a.identity() == b.identity()`).
- **Identity is not preserved by copying.**
  - `dup` mints a fresh identity (D13, D20).
  - A value decoded by `wire::from_binary` (`crates/bund2-value/src/wire.rs`)
    keeps its stamp and gets a fresh identity (D88).
- **Wire depth.** `to_binary` refuses a value deeper than `MAX_WIRE_DEPTH`,
  256, counting the leaf (F118; `depth_of` in `wire.rs`).
- **`summary` is bounded:** four items, two levels and a width
  (`summarise` in `crates/bund2-value/src/lib.rs`, D36).
- **Threads and state.** `BundValue` is not `Send`: `enum BundValue` holds
  an `Rc`. `bund2-api` has no per-VM extension slot.

### The graph family, the bus, the world file, input and the memory guard

- **Graph node names** are Title-cased when a graph is read (`node_name`
  in `crates/bund2-stdlib/src/graph.rs`). A node operand is looked up as
  given (D211, F218).
- **The bus.** `send` reads `<channel> <object>` and wire-encodes the object
  (`send_base` and `push` in `crates/bund2-stdlib/src/bus.rs`). The bus is
  process-wide (D87).
- **The world file.**
  - `save.model` reads `<name> <value> <file>` (`save_model` in
    `crates/bund2-stdlib/src/world.rs`).
  - `load.model` pushes every model in the file (F110, `load_model`).
- **`input`** pushes nothing on Ctrl-D (`Read::End` in `input`,
  `crates/bund2-stdlib/src/terminal.rs`). The test build reads no terminal
  (F158).
- **The memory guard.**
  - It is polled "where a body starts running".
  - A trip is consumed by one `take` (the module header and `take` in
    `crates/bund2-api/src/memguard.rs`).
  - Its tests run in their own binary (the header of
    `crates/bund2-runtime/tests/oom_guard.rs`).

### Rules of the house

- **D37:** Bund2 does not panic.
- **D39:**
  - an internal loop is bounded;
  - a program's loop is warned at 10,000,000 iterations and never stopped;
  - no Rust recursion is driven by a value a program controls.
- **D84:** no native becomes a state machine.
- **D111:**
  - A container's word takes the container from the workbench.
  - "nothing has asked for one of the 48 opaque words" — a `.` form of an
    opaque word (D111).
- **D199, D202:** a sum that overflows is refused only where the answer
  does.

---

## Design — Phase 1

### S1. Terms

**Reading a value as a term:**

| Value | Term |
|---|---|
| CALL starting uppercase or `_`, without `\|` | **variable** **[L1]**. `_` alone is anonymous: each occurrence is fresh. |
| CALL starting uppercase or `_`, containing `\|` | refused (E04) |
| CALL `nil` | **the empty list** **[L4]**. It is not an atom. |
| CALL, PTR or STRING otherwise | **atom**, compared by text **[L2]** |
| BOOL | the atom `true` or `false` |
| INT | **integer** |
| FLOAT | **float**. Floats compare by key equality. `1` and `1.0` do not unify, departing from key equality's `int_eq_float` **[L2]**. |
| LIST whose first item is an atom CALL | **compound**: the first item is the functor, the rest are the arguments. **With no arguments it is the atom itself**: `[ ready ]` and `ready` are one term **[L4]**. |
| LIST otherwise | **list**. A single `\|` item, neither first nor last, splits head from tail. Any other `\|` is E05. |
| NODATA | a fresh variable, one per occurrence |
| CONTEXT | refused (E26): it would switch stacks if placed in a body |
| any other value | **opaque constant**, compared by key equality. For MAP, LAMBDA and OBJECT that is identity **[L13]**. A LAMBDA is also a bridge's body. |

**The traps in this notation:**

- `[ parent tom X ]` is `parent(tom, X)`.
- `[ :tom :bob ]` is a list.
- `[ tom bob ]` is `tom(bob)` **[L4]**.
- `[ tom | T ]` is the compound `tom('|', T)`, because `|` splits only a
  list.
- `[ nil ]` is a one-item list holding the empty list, because `nil` is not
  an atom **[L4]**.
- `[ nil X ]` is a two-item list.
- `[ parent ∅ X ]` is `parent('∅', X)`. Inside a list literal `∅` and `|`
  are CALLs, so they are atoms (or, for `|`, a split). NODATA is a fresh
  variable only when a value built at run time holds it, as at a
  `kb.word` call site (S10).
- `[ ready [ p X ] ]` is the compound `ready(p(X))`, not the conjunction
  of `ready` and `[ p X ]`: S1 decides by the first item. A conjunction
  that starts with a zero-arity goal is written `[ [ ready ] [ p X ] ]`.
  The same holds wherever S3 writes `[ G… ]`. Unless `ready/1` is defined,
  the mistake is E12 **[L8]**, so it does not pass silently.
- `[ in X [ a b c ] ]` ranges over the compound `a(b, c)`, which is not a
  list (E23). The atoms are written `[ :a :b :c ]`.
- `Integer`, `Boolean`, `Value` and every other class name (RFC-0010,
  D225) begin uppercase, so in a term they are variables. A KB about
  classes writes them as STRINGs, `:Integer`.

**Reading a term back as a value:**

| Term | Value |
|---|---|
| atom | STRING (`true` and `false` read back as BOOL) |
| integer, float | INT, FLOAT |
| compound | LIST, with the functor as a CALL |
| list | LIST. A partial list's tail reads as a `\|` CALL followed by NODATA. An improper list's tail, bound to a non-list, reads as a `\|` CALL followed by that tail's value. |
| empty list | an empty LIST |
| opaque | the same value |
| unbound variable | NODATA |

- **`true` and `false` read back as BOOL, whatever they were written as.**
  Atoms compare by text **[L2]**, so the STRING `"true"`, the CALL `true`
  and the BOOL `true` are one atom. A fact stored with the STRING answers
  a BOOL.
- **Nothing read back runs in a body.** Atoms read back as STRING (F63). A
  compound's LIST is pushed whole. CONTEXT is refused on the way in.
- **NODATA reads back as a fresh variable**, so an answer can be handed back
  to `kb.assert`.
- **Aliasing is lost.** Two unbound variables bound to each other read back
  as two NODATAs (Q45).
- **Every term is acyclic.** The occurs check is always on **[L7]**, so no
  unification builds a cycle, and every walk over a term ends.

### S2. A knowledge base is a value **[L3]**

A KB is a MAP with these keys:

| Key | Holds | Missing |
|---|---|---|
| `type` | `"kb"` | not a KB (E01) |
| `preds` | MAP from `"functor/arity"` to that predicate's clause LIST | no clauses |
| `inferences` | INT cap; ≤ 0 means none | none |
| `tabled` | LIST of `"f/n"` (Phase 2 **[L14]**) | none |
| `askable` | LIST of `"f/n"` (Phase 2 **[L14]**) | none |

- **Reading a KB.** A word checks only what it reads. A known key of the
  wrong kind is E02, and unknown keys are ignored.
- **No index is stored.** An edit with `set` therefore cannot mislead the
  engine.
- **What is defined.** A predicate is *defined* when `preds` has its key, or
  when `tabled` or `askable` names it. Naming it in both `tabled` and
  `askable` is E59.

**Clauses.**

- A **fact** is a term.
- A **rule** is a LIST whose first item is the head term and whose
  remaining items are body goals.
  - One `←` or `<-` after the head is dropped when the rule is added.
  - A rule head of arity 0 is written `[ [ ready ] ← … ]`. Written as
    `[ ready ← … ]`, it is the fact `ready(←, …)`.
- A reserved head is E03. A head that is a variable, number, list or
  opaque value is E06.

**Why a KB is a value:**

1. **A store could not hold opaque constants.** They are `BundValue`s,
   which are not `Send`, so a process-wide store cannot hold them. A per-VM
   store needs a slot `bund2-api` lacks.
2. **Value semantics give Prolog's logical update view.** A query holds the
   KB it was handed, and `kb.assert` makes a new value (D5, D35).
3. **Bund works on it unchanged.** `var` names it, `.` holds it on the
   workbench, `save.model` stores it, and `send` ships it.

**Across the wire.** Clauses survive the wire. Opaque constants get fresh
identities (D88), so they no longer unify with their originals **[L13]**.

**Cost.**

- `kb.assert` rebuilds the predicate's LIST and the `preds` MAP. Adding *n*
  clauses one by one costs O(*n*²) `Rc` copies. `kb!` and `kb.consult*`
  cost O(*n*).
- A compiled-form cache keyed on the payload could never go stale (D5,
  D35). None is proposed (Q46).

**Wire depth.** Depth is counted as `to_binary` counts it, leaf included.
`save.model` and `send` refuse anything deeper (F118).

| What | Starts at level | Deepest allowed |
|---|---|---|
| a clause (under the KB, `preds` and its LIST) | 4 | — |
| a fact's term | 4 | 253 levels |
| a rule's head and each of its goals | 5 | 252 levels |

### S3. Goals and control

A goal is a term. A query or a body is one goal, or a LIST of goals taken
as a conjunction.

**Reserved `functor/arity` pairs.** A head using one is E03. A goal using
one whose phase is not yet built is E14. Every phase's pairs are reserved
now.

| Phase | Reserved |
|---|---|
| 1 | `true/0` `fail/0` `=/2` `not/N` `or/N` `if/3` `once/N` `call/N` `findall/3` `is/2` `test/1` `in/2` |
| 2 | `dif/2` |
| 3 | `#=/2` `#≠/2` `#</2` `#>/2` `#<=/2` `#>=/2` `in/3` `ins/3` `label/1` `labeling/2` `all_different/1` `fd_dom/2` |

**Phase 1 control:**

| Goal | Meaning |
|---|---|
| `true`, `fail` | succeed once; fail |
| `[ = A B ]` | unify, with the occurs check **[L7]** |
| `[ not G… ]` | succeed once, binding nothing, if G… has no solution |
| `[ or [ G… ] [ G… ] … ]` | each branch in turn, on backtracking |
| `[ if [ C… ] [ T… ] [ E… ] ]` | commit to C's first solution and run T; if C has none, run E |
| `[ once G… ]` | G…'s first solution |
| `[ call G A… ]` | G with A… appended. G is an atom, a compound, or (with no A) a LIST of goals. |
| `[ findall T [ G… ] L ]` | L unifies with the LIST of copies of T, one per solution of G… |
| `[ is X λ ]` `[ test λ ]` `[ in X λ ]` `[ in X L ]` | bridges (S5) |

**What a goal may not be:**

- an unbound goal: E10;
- a number, an opaque constant, or the empty list (`nil`): E11;
- a predicate that is neither defined nor reserved: E12 **[L8]**.

**A goal's variables** are those of the clause instance it belongs to. A
goal handed to a solving word belongs to an imaginary headless clause.

### S4. Operational semantics

The engine is one loop over heap structures. Nothing recurses in Rust,
except a bridge (S5) and an `ask` (S16), which re-enter Bund.

#### Stores and their lifetimes

| Store | Holds | Lifetime | Undone on backtrack |
|---|---|---|---|
| arena | term cells, goal-list cells, proof nodes | activation | reset to each choicepoint's mark |
| trail | variable bindings, attribute changes | activation | unwound to each mark |
| choicepoint stack | the kinds below | activation | popped. A choicepoint's own fields are **not** trailed. |
| wake-up queue | pending attribute wake-ups | one unification | emptied |
| side store | `Collect`'s results and their proofs, as owned trees | the `Collect` | dropped with it |
| activation store | compiled templates and indexes; tables and the completion stack (S18); remembered replies (S16) | activation | **not** undone |

**Internal goals.** `$cut(h)`, `$collect`, `$asked(h)` and `$answer` are a
goal kind of their own, not terms. A user predicate named `$cut` is
therefore ordinary.

**Arena growth.** The arena is reset only on backtrack. A deterministic run
of *n* inferences therefore holds O(*n*) cells until it backtracks or ends.

#### Choicepoint kinds

**Every choicepoint** records the trail's mark and the arena's mark, and
backtracking to it unwinds to both. The table names what each kind holds
besides, and the goal list it resumes with is in its "On backtrack"
column.

| Kind | Holds, beyond the above | On backtrack |
|---|---|---|
| `Clauses` | the call, its candidates, a cursor | try the next candidate. Taking the last pops this choicepoint first. |
| `Branches` | the remaining `or` branches | resume the next branch |
| `Items` | `in`'s LIST, a cursor | bind X to the next item |
| `Not` | the continuation | G had no solution: resume |
| `Else` | E and the continuation | C had no solution: resume E |
| `Collect` | the template and L, plus the side-store results and proofs | G is exhausted: rebuild the results in the arena, unify with L, resume |
| `Ask` | an askable call; whether a clause answered it (Phase 2) | ask if no clause answered (S16); otherwise fail |
| `Leader` | a variant, its clauses, its pass (Phase 2) | next clause, next pass, or completion (S18) |
| `Consumer` | the call, the continuation, the table, a cursor (Phase 2) | the next table answer, if any |
| `Label` | a variable, its next value, its order (Phase 3) | bind the next value |

#### The loop

Pop the first goal, and act on it:

- **A user predicate:** compute its candidates (S6). If two or more
  remain, push `Clauses`. Rename the first candidate, unify the head, and
  prepend the body.
- **`=`:** unify.
- **`or`:** push `Branches`, then run the first branch.
- **`once G…`:** with `h` the current height, continue with
  `G…, $cut(h), rest`. `$cut(h)` truncates the choicepoint stack to `h`.
- **`if C T E`:** push `Else(E, rest)`, with `h` the height below it.
  Continue with `C…, $cut(h), T…, rest`.
- **`not G…`:** push `Not(rest)`, with `h` the height below it. Continue
  with `G…, $cut(h), fail`.
- **`findall T G L`:** push `Collect`. Continue with `G…, $collect, fail`.
  `$collect` copies T, and its proof when proofs are kept, into the side
  store.
- **`call`:** build the goal and continue.
- **A bridge:** see S5.
- **`true`:** continue.
- **The empty goal list:** an answer.

**On failure**, pop the top choicepoint, unwind the trail to its mark,
reset the arena to its mark, and take its alternative. When no choicepoint
is left, the query is exhausted.

#### Unification

- **Iterative.** It works over a worklist of cell pairs. It dereferences
  both sides and binds a variable to the other side.
- **Variable to variable.** The younger variable is bound to the older.
  Bindings are trailed.
- **Structure.** Compounds unify by functor, by arity, and then argument by
  argument. Lists unify through `Nil` and `Cons`.
- **The occurs check [L7].** It always runs: binding a variable to a term
  that contains it fails. The check is a second worklist.
- **Attributes** (Phase 2's `dif`, Phase 3's domains).
  - Binding a variable that has attributes queues a wake-up. Nothing runs
    yet.
  - On a variable–variable binding, the bound variable's attributes merge
    onto the survivor, trailed.
  - When the pair worklist is empty, the wake-up queue runs to a fixpoint.
    A failure there fails the unification.
  - A trial unification (S17) runs no wake-ups.

#### Other mechanics

- **Renaming.** A clause is compiled once per activation into a template in
  the activation store. Renaming copies the template into the arena.
- **Inferences and the memory guard.** One inference is any of these: a
  resolution attempt, a built-in goal, a bridge, an `ask`, a tabling step,
  a saturation join attempt, a propagator run, or a labeling step.
  - The loop polls `bund2_api::memguard::take` at every inference. A trip
    is E40.
  - The poll is needed because a query that grows the arena without
    starting a body would never meet the interpreter's own poll.

### S5. Bridges

| Goal | λ leaves on top | Then |
|---|---|---|
| `[ is X λ ]` | one value | X unifies with it, read as a term. Nothing there is E21. |
| `[ test λ ]` | `true` or `false` | succeed or fail. Another kind is E22; nothing is E21. |
| `[ in X λ ]` | a LIST | X takes each item on backtracking. Another kind is E23. |
| `[ in X L ]` | — | L must be a proper list. If it is unbound or partial, that is E10; any other non-list, a compound or an improper list among them, is E23. |

**Running λ:**

1. **Substitute.** Each CALL in λ that names a variable of the goal's
   clause instance is replaced by that variable's value, read back as S1
   says.
   - The walk is a worklist. It enters nested lambdas, and the items of a
     `( … )` block (D34).
   - It does **not** enter list literals. A list inside λ is quoted data,
     often an inner goal. So `{ … [ parent X Y ] kb.query … }` leaves the
     inner `Y` to the inner query.
   - A CALL that names no variable stays a call. `{ X 18 <= }` substitutes
     only `X`.
2. **Refuse.**
   - A variable named in λ that is unbound is E20.
   - A bound compound with unbound parts is substituted, with NODATA in
     those parts.
3. **Run.**
   1. **Take the scratch name** for this depth (S6). If a stack of that
      name already exists, the program made it. The bridge then mints a
      new name for that depth and uses it.
   2. **Run the body.** `scoped_call` creates the stack and runs the body
      on it.
   3. **Read** the top of the scratch stack, with `Vm::pull_from`.
   4. **Restore.** If the scratch stack is still current, switch back to
      the stack the bridge came from. Since F231's fix, `scoped_call`
      never returns that way; the check guards against a `Vm` that breaks
      the contract, and criterion 7 holds it with one.
   5. **Drop** the scratch stack. It is not current, so the ring is
      untouched (`drop_named` in `crates/bund2-interp/src/stacks.rs`).
      If λ already dropped it, the drop changes nothing and bumps
      nothing.
4. **Read** the result as the table above says.
5. **Errors.** An error in λ is E24, naming the goal, and `?try` catches
   it.
   - **A guard trip** is taken by whichever poll meets it first. The
     engine's own poll (S4) raises E40, the guard's text alone. The
     interpreter's poll inside λ fails λ, so the trip surfaces as E24
     wrapping the guard's text; inside `ask` it is E57. All three carry
     the guard's text, which is what criterion 1b tests.
   - **Nested bridges nest prefixes.** Each level adds one E24 prefix.
     `summary` bounds the values in the text, but not the prefix chain,
     which grows with the nesting depth that `stack_ok` bounds. Bund's errors are text (`pub struct Error(pub String)` in
   `crates/bund2-api/src/lib.rs`), so
   `catch/3` is not offered.

**`in` over λ.** λ runs each time the goal is called, and its LIST is held
in `Items`. A new call after an earlier goal backtracks runs λ again, with
its side effects.

**What a bridge guarantees:** the current stack is the same before and
after it, and the scratch stack exists only during it.

**What it leaves to λ.**

- What λ does to other stacks, the ring or the workbench is λ's own doing.
- A λ that drops the scratch stack runs its remaining items on the next
  stack in the ring, and the read then finds nothing (E21).

**Other consequences.**

- **What λ can see.** While it runs, the scratch stack is in the ring,
  so `stack_names` and `save.stacks` name it, under its id-shaped name
  (`fresh_name`, 21 characters). It is also the current stack, so
  `debug.display_stack` shows its values, though not its name
  (`display_stack` in `console.rs` renders `vm.snapshot()`). A program that saves
  it and later restores it under that name is S5 step 3.1's collision,
  which is why the step exists.
- **Tags.** A heap value λ leaves keeps the scratch stack's name as its tag
  in an answer (D41). Identity is unaffected.
- **Rust depth.** A bridge costs one native frame. A λ that runs another
  solving word nests, and `stack_ok` refuses deep nesting (its check at
  the head of `scoped_call`, `crates/bund2-interp/src/lib.rs`).
- **Source positions.** The substituted body has none (Q49), so E24 names
  the goal.

### S6. The engine's organisation

The engine lives in `crates/bund2-stdlib/src/kb/`. The existing
`src/logic.rs` holds the comparisons.

| File | Holds | Uses `Vm` |
|---|---|---|
| `mod.rs` | `register` | registration |
| `term.rs` | arena, interner, conversion | no |
| `unify.rs` | unification, occurs check, trail, wake-ups | no |
| `store.rs` | reading, validating and writing KB MAPs | no |
| `solve.rs` | the loop, choicepoints, control, indexing, caps | through `Bridge` |
| `bridge.rs` | `run` (S5), `ask` (S16), the scratch-name pool | **yes** |
| `words.rs` | the natives, and `VmBridge`, the one implementation of `Bridge` | yes |
| `query.rs` | `run_query`, the `query` runner | yes |
| `table.rs`, `chain.rs`, `proof.rs`, `dif.rs` | Phase 2 **[L14]** | no |
| `fd/` | Phase 3 **[L14]** | no |

**The audits.** `REENTERING`, `SWITCHES`, D47's id gate, the palette and
`bund2 check` read only `bund2-stdlib`, so the family lives there.

- `src/kb/` is the crate's first subdirectory. The `REENTERING`,
  `SWITCHES` and `REPORTS_MID_BODY` scans, and
  `nothing_shipped_follows_a_test_module`, walk subdirectories already.
- `no_native_reports_at_error_severity` (D36) reads `src/` alone, with
  one `read_dir`. M1 makes it walk subdirectories as the others do, before
  any file lands in `kb/` (criterion 12a).

- `kb/bridge.rs: run` and `kb/bridge.rs: ask` hold every call of
  `scoped_call`, `to_stack` and `drop_stack`. Both are named in `SWITCHES`
  and in `REENTERING`.
- `kb/query.rs: run_query` calls `eval_lambda`. It is named in
  `REENTERING`.

**The scratch-name pool.**

- **One name per depth.** The pool is thread-local and holds one name per
  activation depth. An activation at depth *d* uses entry *d*, minted with
  `fresh_name()` when *d* is first reached.
- **No sharing.** Activations nest strictly, so no two live activations
  share a depth. A guard decrements the depth on every exit path (D37).
- **Bounded growth.** The interned-name table grows by the greatest nesting
  depth a thread reaches, plus one name for each collision with a stack the
  program made (S5 step 3). It does not grow with the number of bridges or
  activations.
- **Never in the ring.** The stack exists only during a bridge, so it is
  never in the ring while `query`'s `lambda` runs.

**Indexing.**

- **The key.** A call's dereferenced first argument gives the key: an
  atom's text, an integer, a float's key bits, a compound's functor and
  arity, `Nil`, or `Cons`.
- **Candidates.** These are the clauses with that key, merged in clause
  order with those whose first argument is a variable or opaque.
- **No key.** An unbound or opaque first argument, or arity 0, takes every
  clause.
- **When it is built.** The index is built on a predicate's first call in
  an activation.

**Build [L9].** The family is in the default build. It adds no dependency,
so D28 does not apply.

### S7. Termination and caps

| Cap | Where | Meaning | Reached |
|---|---|---|---|
| `limit` | `query` slot | answers wanted | a normal end |
| `inferences` | KB key, set by `kb.inferences` | work allowed | E30, which `?try` catches |
| none | — | the program's loop | at 10,000,000 inferences in one activation, one `Warning` (W1), then it goes on (D36, D39) |

### S8. Answers **[L5]**

- **Shape.** An answer is a MAP from each named variable of the goal to its
  value, read back as S1 says.
  - Names beginning with `_` are left out.
  - A ground goal that holds answers with one empty MAP.
  - Answers come in search order. Keys are in MAP order.
- **Residuals [L15].** In Phases 2 and 3, a variable left with a suspended
  `dif` or with a domain answers NODATA, and the answer gains `_residual`.
  That is a LIST of goals which, re-posted, restore exactly what remains:
  - `[ dif A B ]`;
  - `[ in X 1 9 ]`, for one interval;
  - `[ fd_dom X [ [ 1 3 ] [ 5 9 ] ] ]`, for a domain with holes.
- **Variables inside `_residual`.** A query variable reads as a PTR of its
  name. Any other variable reads as a PTR `_G`*n*.

### S9. The words

Effects are given for the main stack, then the workbench (RFC-0004 §S1,
D195). An opaque count is the least the word consumes. Codes are from S12.

#### Building and editing (fixed effects)

| Word | Operands → result | Effect | Refuses |
|---|---|---|---|
| `kb.new` | → kb | `eff(0,1)`, MAP | — |
| `kb!` | list → kb | `eff(1,1)`, MAP | E00, E07; per clause E03–E06, E26 |
| `kb.consult*` | *c₁…cₙ* → kb | `opaque(0)` | per clause E03–E06, E26 |
| `kb.assert`, `kb.asserta` | kb clause → kb | `eff(2,1)` | E00, E01, E02, E03–E06, E26 |
| `kb.retract` | kb clause → kb | `eff(2,1)` | E00, E01, E02, E04–E06, E26 |
| `kb.merge` | kb₁ kb₂ → kb | `eff(2,1)` | E00, E01, E02, E59 |
| `kb.clauses` | kb → list | `eff(1,1)`, LIST | E00, E01, E02 |
| `kb.inferences` | kb int → kb | `eff(2,1)` | E00, E01, E08 |
| `kb.table`, `kb.askable` | kb functor *n* → kb | `eff(3,1)` | E00, E01, E03, E09, E59 (Phase 2 **[L14]**) |

- **`kb.consult*`** scans down from the top of the stack to the first
  NODATA, which it consumes, or to the bottom of the stack (D12,
  D182). It then adds the items above that point oldest first, so clause
  order is source order.
- **`kb.assert`** appends to its predicate. **`kb.asserta`** prepends.
- **`kb.retract`** removes the first clause whose renamed head and body
  unify with the operand. If none does, the KB is unchanged.
- **`kb.merge`** puts kb₂'s clauses after kb₁'s, per predicate. It unions
  `tabled` and `askable`, and keeps kb₁'s `inferences`.
- **`kb.clauses`** gives every clause in `preds` key order, then clause
  order. `preds` is a `BTreeMap`, so `p/10` comes before `p/2`.

#### Asking (opaque)

| Word | Operands → result | Effect | `check` records |
|---|---|---|---|
| `kb.query` | kb goal → list of MAP | `opaque(2)` | 2 → LIST |
| `kb.once` | kb goal → MAP or NODATA | `opaque(2)` | 2 → 1 |
| `kb.holds` | kb goal → BOOL | `opaque(2)` | 2 → BOOL |
| `kb.call` | *a₁…aₙ* kb functor *n* → list of LIST | `opaque(3)` | *n*+3 → LIST if *n* is literal; otherwise abandons |
| `kb.prove` | kb goal → list of MAP | `opaque(2)` | 2 → LIST (Phase 2 **[L14]**) |
| `kb.saturate` | kb → kb | `opaque(1)` | 1 → MAP (Phase 2 **[L14]**) |

`kb.consult*` has no `check` arm. It is abandoned, as any `opaque(0)` fold
is.

**Refusals of the asking words:**

| Word | Phase 1 | Phase 2 adds | Phase 3 adds |
|---|---|---|---|
| `kb.query`, `kb.once`, `kb.holds`, `kb.prove` | E00, E01, E02, E04, E05, E10–E14, E20–E24, E26, E30, E40 | E50, E51, E54 | E60–E66 |
| `kb.call` | as above, and E09 | as above | as above |
| `kb.saturate` | E00, E01, E02, E20–E24, E30, E40 | E55, E56 | — |

- **`kb.call`** answers each answer as the instantiated argument LIST
  **[L5]**. `:tom ∅ :family var? :grandparent 2 kb.call` gives
  `[ [ "tom" "ann" ] … ]`.
- **`kb.saturate`** answers the input KB's clauses, plus the derived facts.
- **Why opaque.** These words may run bridges or `ask`, and a word that may
  re-enter evaluation must be opaque (`promotable.rs`).
- **Why `check` still knows their effect.** Bridges and `ask` run on the
  scratch stack and restore the current one. Each word's effect on the
  current stack is therefore fixed, and `bund2 check` gets an arm for each.

#### Making words, and the conditional

| Word | Operands → result | Effect | Refuses |
|---|---|---|---|
| `kb.word` | var functor *n* → | `eff(3,0)` | E00, E03, E08 (var not a STRING), E09 |
| `query` | → c | `eff(0,1)`, `Conditional("query")` **[L12]** | — (its runner refuses, S11) |

#### `.` forms (D18, D24, D111)

- **Which words have one:** `kb.assert.`, `kb.asserta.`, `kb.retract.`,
  `kb.merge.` (which holds kb₁ on the workbench), `kb.clauses.`,
  `kb.inferences.`, `kb.table.` and `kb.askable.`.
- **What a form does:** it takes the KB from the workbench and leaves the
  answer there.
- **How a form is declared:**
  - it is registered with `bench!` for the stack half (the `bench` macro
    in `crates/bund2-stdlib/src/wb.rs`);
  - it is listed in `BUILT` for the workbench half, `(1,1)`
    (`crates/bund2-stdlib/src/bench.rs`).
- **What has none.** Opaque words have no `.` form (D111). No word has a
  `,` form (D22).

### S10. `kb.word`

`:family :grandparent 2 kb.word` registers the word `grandparent` through
`register_lambda`, with this body:

```
"family" var? "grandparent" 2 kb.call
```

- **The KB is read at call time.** It is whatever `family` holds when the
  word runs.
- **An unbound argument is NODATA**, written `∅` or `|`. At a call site, `_`
  would be an unknown word.
- **One word per functor name.** `p/1` and `p/2` both register `p`, and the
  second replaces the first (Q47).
- **It behaves like `register`:**
  - it fires `watch redefine`;
  - it records its definition site;
  - it shares F62's warning and is in `REPORTS_MID_BODY`;
  - when the operands are literals, `bund2 check` records its body, as the
    `register` arm does (`check.rs`).

### S11. The `query` conditional **[L12]**

| Slot | Kind | Meaning | Missing |
|---|---|---|---|
| `kb` | KB | the knowledge base | E01 |
| `goal` | term | the goal | E13 |
| `lambda` | LAMBDA | run per answer, with the answer pushed | empty |
| `limit` | INT ≥ 0 | stop after this many answers. `0` asks for none, so the runner returns at once. Unlike `inferences`, where ≤ 0 means no cap, a missing slot is the only way to ask for no limit. | none |
| `skip_first` | INT ≥ 0 | skip answers first | 0 |
| `ask` | LAMBDA | Phase 2 (S16) | asking is E50 |
| `known` | STRING | Phase 2 (S16): a variable to persist replies in | replies kept for the run only |
| `prove` | BOOL | Phase 2 (S15): push `{ answer, proof }` | `false` |

**The runner, `kb/query.rs: run_query`, refuses:**

- a slot of the wrong kind, or a negative `limit` or `skip_first`: E08.
  `sqlite`'s behaviour on a negative `skip_first`, skipping every row, is
  not copied;
- an error in `lambda`: E25;
- otherwise, the asking words' codes, plus Phase 2's E52, E53, E57 and E58.

**Answers one at a time [L6].** The runner solves to the next answer, runs
`lambda` with `eval_lambda`, then solves on. Its state stays in its Rust
frame (D84).

**`lambda` runs on the current stack, and nothing restores it.** Unlike a
bridge's λ, `lambda` is the program's own code, as `sqlite`'s is: a
`lambda` that switches stacks leaves the next answer to be pushed on the
stack it switched to.

**`bund2 check`.** `query` joins the `"csv" | "sqlite"` arm
(`check.rs`), with `Abs::Tagged(MAP)` per answer. `ask` runs on the
scratch stack, so it does not change the walk.

### S12. Refusals and warnings

- Every text except E00 begins `<W>: `, where `<W>` is the word's name in
  capitals.
- E00 is the house text, unprefixed (`singles.rs`, `var`).
- Values are shown with `summary` (D36).

#### Phase 1

| Code | Text after `<W>: ` |
|---|---|
| E00 | (unprefixed) `Stack is too shallow for inline <w>` |
| E01 | `#<k> parameter must be a knowledge base` |
| E02 | `malformed knowledge base: <key> is not a <kind>` |
| E03 | `<f>/<n> is reserved and cannot be defined` |
| E04 | `<name> is not a variable name: it contains '\|'` |
| E05 | `'\|' must stand between a head and a tail in <list>` |
| E06 | `a clause head must be an atom or a compound: <head>` |
| E07 | `#1 parameter must be a list of clauses` |
| E08 | `<slot or operand> must be <kind>, found <type>` |
| E09 | `the functor must be a STRING and the arity a non-negative INT` |
| E10 | `instantiation error: <what> is unbound in <goal>` |
| E11 | `type error: <value> is not callable` |
| E12 | `existence error: unknown predicate <f>/<n> in <goal>` **[L8]** |
| E13 | `the query has no goal` |
| E14 | `<f>/<n> belongs to Phase <p>, which is not built` |
| E20 | `instantiation error: <Var> is unbound in the lambda of <goal>` |
| E21 | `<bridge>: the lambda left nothing in <goal>` |
| E22 | `test: expected BOOL, found <type> in <goal>` |
| E23 | `in: expected a list, found <value> in <goal>` |
| E24 | `bridge lambda in <goal> returns: <error>` |
| E25 | `lambda returns: <error>` |
| E26 | `a CONTEXT cannot be a term: <value>` |
| E30 | `inference cap of <n> reached` |
| E40 | the memory guard's own text (`Trip::error`) |
| W1 | `<n> inferences and not finished; set kb.inferences to bound this query` (`Warning`) |

#### Phase 2

| Code | Text after `<W>: ` |
|---|---|
| E50 | `<f>/<n> is askable, and this query has no ask lambda` |
| E51 | `ask: <goal> must be ground` |
| E52 | `ask: the lambda left nothing for <goal>` |
| E53 | `ask: expected true or false, found <value> for <goal>` |
| E54 | `<construct> over the incomplete table of <f>/<n>: the program is not stratified` (`<construct>` is `not`, `if` or `findall`) |
| E55 | `saturate: <clause> cannot be saturated: <reason>` |
| E56 | `saturate: <f>/<n> depends negatively on itself` |
| E57 | `ask lambda returns: <error>` |
| E58 | `known: <var> must hold a LIST of ground [ goal reply ] pairs, found <value>` |
| E59 | `<f>/<n> cannot be both tabled and askable` |

E55's `<reason>` is one of:

- `<Var> occurs only in the head`;
- `or, if and once are not allowed`;
- `call needs a literal functor`.

#### Phase 3

| Code | Text after `<W>: ` |
|---|---|
| E60 | `type error: integer expression expected, found <value> in <goal>` |
| E61 | `<value> cannot be represented as an INT in <goal>` |
| E62 | `label: <Var> has an infinite domain` |
| E63 | `<goal>: expected a LIST of variables, found <value>` |
| E64 | `in: bounds must be INT, inf or sup, found <value>` |
| E65 | `labeling: unknown option <value>` |
| E66 | `in: the lower bound <lo> is above the upper bound <hi>` |

### S13. The tiers and the audits

- **Tier 1 never crosses an opaque word:** `kb.consult*`, `kb.query`,
  `kb.once`, `kb.holds`, `kb.call`, `kb.prove`, `kb.saturate`.
- **The palette reaches four words:**
  - `kb.new`;
  - `kb!`;
  - `kb.word`, which `REPORTS_MID_BODY` excludes;
  - `query`, as it reaches `?try` (`tests/golden/PROMOTABLE.txt:57`).
- **It does not reach the fixed words that read a KB.** The palette's
  `dict` has no `type` (RFC-0012 §S6).
- **Epoch bumps.** Each bridge bumps the stack epoch twice: once in
  `add_as_current`, which `to_stack` reaches for a new name and returns
  through without bumping again, and once when the exit action's
  `to_stack(prev)` brings the caller's stack back to the front. The drop
  bumps nothing, and the ring is in its old order
  (`crates/bund2-interp/src/stacks.rs`). The bridging words are never crossed, so
  this costs only a guard check in a compiled caller.
- **Bridge bodies are never offered to the tier.** A body run inside a
  bridge is new every run, so it stays cold.

### S14. Interactions

- **Bundles (RFC-0006).**
  - The words are natives, so a bundle carries them.
  - `--noeval` (D79) does not touch them: a bridge runs a lambda, not
    source text.
- **The bus (RFC-0007, D87).**
  - KBs, clauses and answers cross the bus as any MAP or LIST does.
  - Opaque constants arrive with fresh identities (D88) **[L13]**.
  - Bus channels are not stacks, so a channel cannot collide with a pooled
    scratch name.
- **The debugger (RFC-0008).** Breakpoints fire inside bridges. A stop
  shows the bridge's frame, without a source position (Q48, Q49).
- **The profiler (RFC-0011).**
  - It profiles the solving words, the `query` runner (through
    `run_conditional`'s hook), and bridge bodies.
  - It does not report per-predicate time (Q48).
- **The world file.** `save.model` stores a KB. `load.model` pushes every
  model in the file (F110), with fresh identities (D88).
- **Classes (RFC-0010, D225).** Class names are Title-cased words, so in
  a term they are variables (S1's traps); a KB names a class by STRING.
  An OBJECT in a term is an opaque constant, compared by identity
  **[L13]**.
- **Certainty factors [L11]** are not offered. `q` stays reserved (D32).

---

## Design — Phase 2: the expert-system layer **[L14]**

### S15. Proof trees

`kb.prove`, or the `query` slot `prove`, gives one MAP per answer, holding
`answer` and `proof`.

**A proof** is a LIST of three parts:

1. the goal, read back as the answer instantiated it;
2. the CALL `←`;
3. the justification.

Its second item is always `←`, so a proof is told from a goal by shape.

| Justified by | Proof |
|---|---|
| a fact | `[ goal ← :fact ]` |
| a rule | `[ goal ← p₁ … pₖ ]` |
| `call G` | the called goal's own proof (`call` is transparent) |
| `or` | `[ goal ← :branch i p ]` |
| `once G` | `[ goal ← p ]`, the proof of the kept solution |
| `if C T E` | `[ goal ← :then pC pT ]` or `[ goal ← :else pE ]` |
| `findall` | `[ goal ← :findall p₁ … pₙ ]`, each copied to the side store at its `$collect` |
| `not` | `[ goal ← :no-proof ]` |
| `=` | `[ goal ← :builtin ]` |
| a bridge | `[ goal ← :bridge λ′ ]`, where λ′ is the substituted λ |
| asked / remembered | `[ goal ← :asked :yes ]` / `[ goal ← :known :yes ]` |
| a table answer | `[ goal ← :tabled ]`. Table answers keep no proof. |

**Storage.** Proof nodes live in the arena, or in the side store under
`Collect`. They are kept only when proofs are asked for.

### S16. Questions to the user, and remembered replies

`kb.askable` names a predicate in `askable`, which defines it (S2).

**Asking.**

- **Setup.** A call on an askable predicate pushes `Ask(call)`, then the
  `Clauses` choicepoint above it.
- **Marking.** Each clause body is followed by `$asked(h)`, which marks the
  `Ask` at height `h` as *answered*.
  - That mark is a choicepoint field, which is not trailed, so it survives
    backtracking into later clauses.
  - No `$cut` inside a clause body reaches below `h`.
- **When backtracking reaches `Ask`:**
  - If a clause answered, `Ask` fails.
  - Otherwise:
    1. **The goal must be ground** (E51).
    2. **A remembered reply** that matches the goal is used.
    3. **With no `ask` slot,** the goal is E50.
    4. **Otherwise `ask` runs**, through `kb/bridge.rs: ask`, exactly as a
       bridge runs λ: on the scratch stack, with the goal read back and
       pushed, and the current stack restored afterwards.
       - Leaving nothing is E52.
       - A reply that is not `true` or `false` is E53. The reply is read as
         a term, so a BOOL and the CALLs `true` and `false` all count.
       - An error is E57.
    5. **The reply is recorded.** A `true` reply succeeds once; a `false`
       reply fails.

**Remembered replies.**

- **Within a run, always.** Replies are kept in the activation store for
  the run. A second rule, or a re-run, that needs the same question does
  not ask again.
- **Matching.** The store is a LIST of `[ goal reply ]` pairs, each goal
  ground. A ground goal matches a pair by unification. That is exact at any
  depth and any width; `summary` is never used.
- **Across runs, with `known`.** `known` names a variable that holds the
  same LIST.
  - A variable that does not exist yet starts empty.
  - Anything other than a LIST of ground pairs with replies `true` or
    `false` is E58.
  - The variable is written through on every reply, with `register_var`.
    So a reply survives a later error, and a nested `query` naming the
    same variable sees it.
- **Seeding.** `:answers [ [ [ has fur ] true ] ] var` answers `[ has fur ]`
  with `true`, so that question is never asked.

### S17. `dif/2`

- **Meaning.** `[ dif A B ]` holds iff A and B are different terms.
- **Checking.** The engine runs a trial unification of A and B, then undoes
  it. The trial is structural only and runs no wake-ups.
  - If A and B cannot unify, `dif` holds and is discharged.
  - If they unify without binding anything, `dif` fails.
  - Otherwise `dif` suspends, as an attribute, on the variables the trial
    would have bound.
- **Waking.** A wake-up re-runs the check after the unification that bound
  one of those variables has finished. A half-built compound is never
  judged.
- **Variable–variable binding.** The attribute moves to the surviving
  variable, so `[ dif X a ] [ = X Y ] [ = Y a ]` fails.
- **Domains do not discharge a `dif`.** For X ∈ 1..3, `dif(X, 5)` stays
  suspended until X is bound. This is weaker than it could be, but sound.
- **In answers.** A suspended `dif` reaches the answer as `_residual`
  **[L15]**.
- **Purity.** `[ not [ = A B ] ]` remains, but it is sound on ground terms
  only. `dif` is the monotonic one (https://www.metalevel.at/prolog/purity).

### S18. Tabling

`kb.table` names a predicate in `tabled`.

**Variants.** A tabled call is keyed by its *variant*: the call with its
variables renumbered in order of first occurrence, attributes included. A
variant's table lives in the activation store and holds:

- its answers, compared as variants, with attributes;
- a status: *new*, *evaluating* or *complete*.

**The completion stack.** It lives in the activation store and is not
undone on backtrack.

- A variant is pushed when it first leads.
- It is popped only when its SCC completes.
- Each entry records its **SCC root**, the oldest entry of its SCC.

**Linear tabling, SCC completion and local scheduling:**

1. **A call of a *new* variant leads.**
   - It pushes `Leader` and a completion-stack entry, and marks the table
     *evaluating*.
   - It runs the clauses, each followed by `$answer`. `$answer` adds the
     answer to the table, unless it is a variant of one already there, and
     then fails.
   - Nothing is returned to the caller yet.
2. **A call of an *evaluating* variant V consumes.**
   - It pushes `Consumer`, which holds the call, the continuation, the
     table and a cursor, and it returns V's answers so far.
   - Every completion-stack entry from **V's SCC root** to the top merges
     into one SCC under that root.
3. **When a leader's clauses are exhausted:**
   - **If its entry is an SCC root,** it re-runs every member's clauses
     until a whole pass adds no answer to any table in the SCC. It then
     marks them all *complete*, pops them, and returns its answers through
     a `Consumer` over its now-complete table.
   - **Otherwise** it fails back. Its entry stays on the completion stack
     until the root completes.
4. **A call of a *complete* variant** reads its table through `Consumer`.

**Negation and aggregation (E54).**

- Each `Leader` and `Consumer` records the height of the innermost of
  these, as they stood when it was pushed:
  - a `Not`;
  - an `Else` whose condition is running;
  - a `Collect`.
- A merge whose SCC root lies below that height is E54, naming the
  construct.
- So negation, if-then-else and aggregation over an incomplete table are
  refused, whether the table was evaluating already or joined the SCC
  later. The well-founded semantics is not offered.

**Consequences.**

- Left and right recursion over finite data terminate.
- Every table is final when read. Answers are returned only after
  completion, so `once`, `if` and `not` never cut a table short.
- A predicate that is both tabled and askable is E59.
- A bridge in a tabled predicate runs again on each pass.
- Tables last one activation.
- A predicate with unbounded answers still runs forever, and S7 applies.

### S19. Forward chaining

`kb.saturate` answers the input KB's clauses, plus every fact its rules
derive, run to a fixpoint.

**What it accepts.**

- **Range restriction (E55).** Each head variable occurs in a positive body
  goal that is not a bridge.
- **Stratification (E56).** `not G` and `findall`'s goal make their
  predicates negative dependencies. A predicate that depends negatively on
  itself is E56.
- **Body goals (E55).** `call` must have a literal functor. `or`, `if` and
  `once` are refused. Bridges run once per binding.

**How it runs: semi-naive, stratum by stratum.**

1. Round 1 joins every rule in full.
2. Each later round joins each rule with at least one goal over the facts
   new in the previous round.
3. A derived fact that is a variant of a known fact is dropped.
4. A stratum ends when a round adds nothing.

**Order.** Derived facts are appended per predicate: by round, then by
rule, then in join order. So the same KB always saturates to the same
value.

---

## Design — Phase 3: CLP(FD) **[L10] [L14]**

### S20. Constraints

| Goal | Meaning |
|---|---|
| `[ in X Lo Hi ]` | X ∈ [Lo, Hi]. Lo and Hi are INT, `inf` or `sup`; anything else is E64. Lo above Hi is E66. |
| `[ ins [ X … ] Lo Hi ]` | `in` for each item. A non-list is E63. |
| `[ fd_dom X [ [ lo hi ] … ] ]` | X's domain is exactly that union of intervals. It is S8's residual form. |
| `[ #= A B ]` `[ #≠ A B ]` `[ #< A B ]` `[ #> A B ]` `[ #<= A B ]` `[ #>= A B ]` | relations, read **left to right**: `[ #> X 7 ]` is X > 7, unlike Bund's `X 7 >` |
| `[ all_different [ X … ] ]` | pairwise `#≠`, with value elimination. A non-list is E63. |
| `[ label [ X … ] ]` | leftmost variable first, smallest value first, through `Label`. A non-list is E63. |
| `[ labeling [ opts ] [ X … ] ]` | `:ff` picks the smallest domain first; `:down` tries the largest value first. Any other option is E65. |

**Expressions** are an INT, a variable, `[ + A B ]`, `[ - A B ]`,
`[ * A B ]`, `[ - A ]`, `[ abs A ]`, `[ min A B ]` or `[ max A B ]`.
Anything else is E60.

### S21. Domains and propagation

**Domains.**

- A domain is a sorted list of disjoint closed intervals over `i64`, held
  as an attribute.
- Changes are trailed.
- An empty domain fails. A singleton domain binds its variable.

**Propagators.**

- Bounds propagation for linear relations, and for `*`, `abs`, `min` and
  `max`.
- Value removal for `#≠`, once one side is fixed.
- `all_different` removes each fixed value from the other variables.
- Propagators run from the wake-up queue to a fixpoint.
- **Each propagator run is one inference**, so W1, E30 and the memory guard
  apply inside propagation.

**Bound arithmetic** is done in checked `i128`, with ±∞ sentinels:

| Operation | Result |
|---|---|
| ±∞ + finite | ±∞ |
| ∞ × positive, −∞ × negative | ∞ |
| ∞ × negative, −∞ × positive | −∞ |
| ±∞ × 0 | 0 |
| an `i128` overflow | saturates to the sentinel of its sign |

**A bound outside `i64`:**

| Bound | Result |
|---|---|
| lower bound above `i64::MAX` | the domain empties, and the constraint fails |
| upper bound below `i64::MIN` | the domain empties, and the constraint fails |
| lower bound below `i64::MIN` | widens to `inf` |
| upper bound above `i64::MAX` | widens to `sup` |

E61 is raised only where a value must be produced and cannot be (D199,
D202).

**Binding a constrained variable.**

- An INT is checked against the domain, and wakes the propagators.
- A non-integer fails.
- Two constrained variables intersect their domains.

**Limits.**

- Labeling an unbounded variable is E62.
- A constrained but unbound variable is unbound to a bridge (E20).
- In an answer it is NODATA, plus `_residual` **[L15]**.
- Reification and other global constraints are left to an amendment.

---

## Worked examples

None of these has been run (Q44). The third review traced each by hand
against the stdlib's operand orders:

- `var` takes `<name> <value>`;
- `set` takes `<receiver> <key> <value>`;
- `get` takes `<container> <key>`, consuming both;
- `format` takes the values, then the template;
- `graph!` takes the edges, then the nodes;
- `graph.reachable` takes `<start> <graph>`, and its answer includes the
  start.

The fourth review re-read each of these against Bund2, along with `input`
(it pulls a prompt and trims the line) and the existence of `var?` and
`list`. All hold. What Q44 leaves open is the run itself: the `kb.` words
do not exist yet.

### 1. Facts, a rule, three ways to ask

```
:family [
  [ parent tom bob ] [ parent bob ann ] [ parent bob pat ]
  [ [ grandparent X Z ] ← [ parent X Y ] [ parent Y Z ] ]
] kb! var

:family var? [ grandparent tom W ] kb.query      // W = "ann", then W = "pat"
:family var? [ grandparent tom :ann ] kb.holds   // true

query
  :kb     :family var? set
  :goal   [ grandparent G :pat ] set
  :lambda { :G get "{0} is a grandparent of pat" format println } set
!
```

### 2. Bridges

```
:people [
  [ age ann 34 ] [ age bob 15 ] [ age pat 71 ]
  [ [ adult P ]      ← [ age P A ] [ test { A 18 <= } ] ]          // 18 <= A
  [ [ older P Q ]    ← [ age P A ] [ age Q B ] [ test { A B < } ] ] // B < A
  [ [ next_age P N ] ← [ age P A ] [ is N { A 1 + } ] ]
] kb! var

:people var? [ adult P ] kb.query          // P = "ann", then "pat"
:people var? [ next_age bob N ] kb.once    // { N: 16 }
```

### 3. A graph word as a generator

```
:roads [ [ :a :b 1.0 ] [ :b :c 1.0 ] ] [ :a :b :c :d ] graph! var
:trips [
  [ [ reaches A B ] ← [ in B { A :roads var? graph.reachable } ] ]
] kb! var
:trips var? [ reaches "A" B ] kb.query     // B = "A", "B", "C"
```

Node names are Title-cased when the graph is read, and a start node is
looked up as given (D211). So the start is written as the STRING `"A"`; a
capitalised CALL would be a variable.

### 4. Diagnosis, and a predicate as a word

```
:ops [
  [ depends web api ] [ depends api db ] [ depends api cache ]
  [ down cache ]
  [ [ needs S D ] ← [ depends S D ] ]
  [ [ needs S D ] ← [ depends S M ] [ needs M D ] ]
  [ [ impaired S D ] ← [ needs S D ] [ down D ] ]
] kb! var

:ops :impaired 2 kb.word
:web ∅ impaired          // [ [ "web" "cache" ] ]
```

### 5. An expert system (Phase 2)

```
:zoo [
  [ [ animal dog ]  ← [ has fur ] [ says woof ] ]
  [ [ animal cat ]  ← [ has fur ] [ says meow ] ]
  [ [ animal duck ] ← [ has feathers ] [ says quack ] ]
] kb! :has 1 kb.askable :says 1 kb.askable var

:answers list var
query
  :kb     :zoo var? set
  :goal   [ animal A ] set
  :known  :answers set
  :ask    { "{0}? (y/n) " format input "y" == } set
  :lambda { :A get "it is a {0}" format println } set
!
```

- **At most five questions are asked:** has fur, says woof, says meow, has
  feathers, says quack.
  - `[ has fur ]` is asked once, though two rules need it.
  - Each reply is written through to `answers`.
- **The prompt** is the goal LIST's `display`, then `? (y/n) `.
- **Ctrl-D.** `input` pushes nothing, so `==` underflows inside `ask`,
  which is E57.
- **Unattended (F158).** Seed all five replies, so that nothing is read
  from a terminal:

  ```
  :answers [ [ [ has fur ] true ] [ [ says woof ] false ] [ [ says meow ] true ]
             [ [ has feathers ] false ] [ [ says quack ] false ] ] var
  ```

  The answer is then `A = "cat"`.

### 6. Tabling and saturation (Phase 2)

```
:net [
  [ edge a b ] [ edge b c ] [ edge c a ]
  [ [ path X Y ] ← [ path X Z ] [ edge Z Y ] ]
  [ [ path X Y ] ← [ edge X Y ] ]
] kb! :path 2 kb.table var
:net var? [ path a Y ] kb.query          // Y = "b", "c", "a", in table order
:net var? kb.saturate kb.clauses         // 3 edges, 2 rules, 9 path facts
```

### 7. SEND + MORE = MONEY (Phase 3)

```
:puzzle [
  [ [ solve [ S E N D M O R Y ] ] ←
      [ ins [ S E N D M O R Y ] 0 9 ]
      [ all_different [ S E N D M O R Y ] ]
      [ #≠ S 0 ] [ #≠ M 0 ]
      [ #= [ + [ + [ + [ * 1000 S ] [ * 100 E ] ] [ * 10 N ] ] D ]
           [ - [ + [ + [ + [ * 10000 M ] [ * 1000 O ] ] [ * 100 N ] ] [ + [ * 10 E ] Y ] ]
               [ + [ + [ + [ * 1000 M ] [ * 100 O ] ] [ * 10 R ] ] E ] ] ]
      [ label [ S E N D M O R Y ] ] ]
] kb! var
:puzzle var? [ solve L ] kb.once         // { L: [ 9 5 6 7 1 0 8 2 ] }
```

---

## Implementation plan

Each milestone ends with its criteria passing and `cargo xtask conform`
unchanged. Until Phase 2 is built, a call on a predicate named in `tabled`
or `askable` is E14.

| Milestone | Builds | Criteria |
|---|---|---|
| M1 | `term.rs`, `unify.rs`; D36's scan walks subdirectories (S6) | 2a, 3, 12a's scan |
| M2 | `store.rs`; the building words, their `.` forms, `BUILT` | 1a |
| M3 | `solve.rs`; `kb.query`, `kb.once`, `kb.holds`; caps; the guard poll | 1b, 2b, 5, 6a, 11, 15 |
| M4 | `bridge.rs` | 1c, 6b, 7, 8a, 8c, 9, 14 |
| M5 | `kb.call`, `kb.word`, `query`, the `check` arms, the audit lists, `PROMOTABLE.txt` | 1d, 4, 8b, 10, 12a, 13 |
| M6 | measurements | 16 |
| M7 | proofs **[L14]** | 17 |
| M8 | asking **[L14]** | 6c, 12b, 18 |
| M9 | `dif` **[L14]** | 19 |
| M10 | tabling **[L14]** | 20 |
| M11 | saturation **[L14]** | 21 |
| M12 | CLP(FD) **[L10] [L14]** | 12c, 22–24 |
| M13 | `bund_examples/logic_demo/` | 25 |

---

## Which points are rulings

Accepting this RFC accepts the following.

- **D226's L1–L15**, as ruled.
- **Phase 1 (S1–S14):**
  - the words of S9, and no others;
  - a KB as a MAP value, with no index and no cache;
  - the occurs check always on **[L7]**;
  - the bridge as specified: substitution, the depth-indexed name pool, a
    stack per bridge, collision handling, the restore, and its guarantees;
  - NODATA as the unbound marker;
  - `kb.call`'s answer shape;
  - the two caps;
  - the solving words opaque, with `check` arms;
  - the `.` forms of S9, a build choice that acceptance ratifies;
  - the reserved set;
  - the texts of S12.
- **Phases 2 and 3 (S15–S21)**, as specified or as direction only, per
  **[L14]**.
- **The family's standing.** It is Bund2's own, outside D14's in-scope set
  (D72), and not in `docs/core-words.md`.
- **Certainty factors** are deferred **[L11]**.

---

## Preservation analysis

- **No existing word changes.** Every name is new, and `bund2 words` lists
  them. `PROMOTABLE.txt` regenerates. `conform`, `COVERAGE` and
  `IMPLEMENTED` move by zero.
- **No syntax changes.** Every term is a list literal the grammar already
  accepts. The notation's meanings exist only inside the values `kb.` words
  read.
- **No value type, `dt` or wire variant is added.**
- **F231 is fixed** (`6e900f8`), apart from this RFC. `context` now comes
  back to the caller's stack when the guard refuses its frame. It is
  Bund2's own path; no golden reaches it.
- **Test-pinned lists change size.** `REENTERING`, `SWITCHES` and
  `REPORTS_MID_BODY` gain entries and `BUILT` grows (criterion 12), and
  one scan starts walking subdirectories (S6).
- **New names can collide with a program's own words.** A program that
  `register`s `query` or a `kb.` name now redefines a native, and `watch
  redefine` fires. No corpus program
  does: `query` appears only in a comment and a string of
  `tests/probes/sqlite-conditional.bund`, and no reference word carries
  either name.
- **λ can see its scratch stack** (S5, "What λ can see").

---

## Alternatives considered

| Alternative | Why not |
|---|---|
| A store held by the engine | S2: opaque constants are not `Send`; there is no per-VM slot |
| An index stored in the value | It goes stale under `set`, and gains nothing |
| A KB as a named stack | It must be re-read on every query |
| A new `dt` | The wire format is the reference's (the header of `crates/bund2-value/src/wire.rs`) |
| A fresh scratch stack per bridge, or one kept for a whole activation | The name table grows without bound, and the stack sits in the ring |
| Substituting into list literals | Variable capture |
| `.` forms for opaque words | D111 |
| The occurs check off, or optional (L7 B, C) | A cyclic term makes unification, read-back, copying and variant checks loop inside one inference, out of reach of W1, E30 and the guard. Cycle-safe walks would need visited sets everywhere, and a refusal for cyclic read-back. |
| Completing each variant's table on its own | Wrong answers |
| A leader stack of live choicepoints | It loses SCC members that have handed back |
| SLG with suspension (XSB) | Linear tabling needs only S4's loop |
| The well-founded semantics | Refused (E54) |
| Rete | Semi-naive evaluation suits one saturation of a value |
| `known` keyed by `summary` | `summary` is lossy |
| An `or` residual for domains with holes | Re-posting it splits the answers |
| A Prolog text reader, cut, `catch/3` in goals, `?x` variables | S3, S5, L1 B |
| A separate crate | Outside the audits |

---

## Acceptance criteria

Each criterion is a named test in `crates/bund2-stdlib`, unless placed
otherwise, or a measurement in `docs/measurements/`.

### Phase 1

1. **Answers and refusals.** Each S9 word, each `.` form and `query` has a
   test for its answer, and each Phase 1 code has a test by its text:
   - **1a** — the building words (M2).
   - **1b** — the asking words: E10–E14, E26, E30, and E00 unprefixed
     (M3). E40 is tested in `crates/bund2-runtime/tests/oom_guard.rs`,
     because the guard's flag is process-wide and that binary runs one
     test at a time: a trip met by the engine's poll is E40, and one met
     inside a bridge's λ is E24, each carrying the guard's text (S5).
   - **1c** — E20–E24, and `test` on `true`, `false`, a non-BOOL and
     nothing (M4).
   - **1d** — E09, E25, and `query`'s slots (M5).
2. **Unification.**
   - **2a** — over an enumerated table of term pairs, `unify` is
     symmetric and its substitution idempotent, and `[ = X [ f X ] ]`
     fails (M1). The table covers each S1 term kind against each other,
     shared and repeated variables, nested compounds and partial lists. It
     is a table rather than a property test, so no dependency is added.
   - **2b** — `ready` and `[ ready ]` unify, as goals and as heads (M3).
3. **No recursion on depth.** Terms 100,000 deep, built in Rust (F116),
   convert, unify and pass the occurs check in a debug build.
4. **No Rust depth across answers.** `[ nat _ ]` over `[ nat z ]` and
   `[ [ nat [ s N ] ] ← [ nat N ] ]`, with `limit` 1,000,000, answers
   1,000,000 empty MAPs. The Rust stack depth, sampled from `lambda` at the
   100,000th and the 1,000,000th answer, is equal. The sample is taken by
   a native the test registers, which pushes the address of one of its
   own locals as an INT; equal addresses mean equal depth. Both samples
   fall after `lambda` is hot, so under `--features jit` both come from
   the same compiled caller: an earlier sample would come through the
   interpreter's frames and differ for that reason alone.
5. **The inference cap.** `[ loop ]` over `[ [ loop ] ← [ loop ] ]`, with
   `inferences` 20,000,000, emits W1 once, then E30, which `?try` catches.
   The arena growth is recorded.
6. **Errors.**
   - **6a** — E12 for an undefined predicate **[L8]** (M3).
   - **6b** — E20 names the variable and the goal (M4).
   - **6c** — a predicate defined only by `askable` or `tabled` is not E12
     (M8, M10).
7. **Bridges leave no trace.**
   - Three bridges are run: one that succeeds, one that fails, and one whose
     λ runs an inner query whose bridge fails. After each, the current
     stack's name is unchanged and `stack_names()` holds no scratch stack.
   - The restore (S5 step 3.4): `kb/bridge.rs: run` is run against a `Vm`
     wrapper whose `scoped_call` returns `Err` with the scratch stack
     current, as F231 did before its fix. The wrapper delegates every
     other method to an `Interp`, and lives in `bund2-stdlib`'s tests,
     which have `bund2-interp` as a dev-dependency.
   - A stack made by `load.stacks` under a pooled name is left intact, and
     the bridge reads its own result.
8. **The name table.** It is measured by interning a fresh probe name
   before and after, and comparing indices. Each case grows the table by at
   most the greatest nesting depth:
   - **8a** — 100,000 sequential `kb.holds` activations, each with a bridge,
     in a `times` loop (M4);
   - **8b** — 100,000 calls of a `kb.word` word whose predicate runs a
     bridge (M5);
   - **8c** — one query whose 100,000 bridges each run an inner `kb.query`
     (M4).
9. **The logical update view.** A bridge that runs `kb.assert` on the KB
   being solved does not change the answers.
10. **`kb.word`.**
    - `:tom ∅ grandparent` answers a LIST containing `[ "tom" "ann" ]`, as
      `kb.call` does.
    - The word follows a rebinding of `family`.
    - `∅` and `|` both mark an unbound argument.
11. **Round trips.**
    - `kb.clauses` → `kb!`, and `save.model` → `load.model`, give the same
      answers.
    - A fact term 254 deep, and a rule goal 253 deep, are refused.
    - An opaque constant sent over the bus no longer unifies with its
      original **[L13]**.
12. **Audit lists.**
    - **12a** — `SWITCHES` gains `kb/bridge.rs: run`. `REENTERING` gains
      `kb/bridge.rs: run` and `kb/query.rs: run_query`.
      `REPORTS_MID_BODY` gains `kb.word`. `BUILT` gains Phase 1's forms.
      `PROMOTABLE.txt`'s diff names only S13's words.
      `no_native_reports_at_error_severity` walks subdirectories, and
      asserts that it read at least one file under `kb/`, as
      `nothing_shipped_follows_a_test_module` counts the files it reads
      (M1).
    - **12b** — `kb/bridge.rs: ask` is added to both lists, and Phase 2's
      forms to `BUILT`.
    - **12c** — Phase 3 changes none of these lists.
13. **`bund2 check`.**
    - It reports underflow for `kb.query` with one operand.
    - It follows a `query` whose `lambda` consumes its MAP.
    - It follows a call to a `kb.word` word made from literals.
14. **Substitution.** An inner list literal's `Y` is not substituted. A
    `( … )` block inside λ is.
15. **NODATA.**
    - NODATA in an asserted clause is a fresh variable.
    - A re-asserted answer with an unbound part answers as the original did.
16. **Measured, not gated:**
    - naive reverse of 30 elements, ×10,000;
    - a 10,000-fact join;
    - 10,000 one-by-one asserts against `kb!`;
    - `[ nat _ ]` to 1,000,000;
    - criterion 5's arena;
    - the occurs check's share of naive reverse;
    - the per-inference guard poll's share of naive reverse.

### Phase 2 **[L14]**

17. **Proofs.** `kb.prove` gives proofs, as S15 describes, for examples 1,
    2 and 4, for a rule using `or`, and for a rule using `findall`.
    Re-running each `:bridge` λ′ reproduces its binding.
18. **Asking.**
    - Example 5, with a scripted `ask`, asks at most five questions.
    - A second `query` with the same `known` asks none.
    - Goals that differ only in a fifth argument, or only at depth 3, are
      each asked.
    - With the five seeds, example 5 asks nothing and answers `"cat"`.
    - E50–E53, E57, E58 (including a non-ground seed) and E59 are tested.
19. **`dif`.**
    - `(G, S=T)` and `(S=T, G)` agree, for `G = [ dif X a ]`.
    - `[ dif X a ] [ = X Y ] [ = Y a ]` fails.
    - The residual appears in `_residual`.
20. **Tabling.**
    - A left-recursive `path` over a 1,000-node cycle terminates, and
      equals `graph.reachable` (names in Title case).
    - The KBs below are written out here because the reviews that first
      gave them are not in the repository; they test the failures those
      reviews named. Each tables `path/2`, with
      `[ [ path X Y ] ← [ edge X Y ] ]` and
      `[ [ path X Y ] ← [ edge X Z ] [ path Z Y ] ]`.
    - **A member completed before its SCC.** Over
      `[ edge a b ] [ edge b a ] [ edge a c ] [ edge c d ]`, the query
      `[ [ path a X ] [ path b Y ] ]` answers Y = `d` among its answers.
      `path(b, _)` is first called under `path(a, _)` and consumes it while
      it holds only `b` and `c`; completed on its own, its table would lack
      `d`.
    - **A variant that joins the SCC late.** Over
      `[ edge a b ] [ edge b c ] [ edge c a ] [ edge c e ] [ edge e b ]
      [ edge a g ] [ edge g h ]`, the query `[ [ path a X ] [ path e Y ] ]`
      answers Y = `h` among its answers. `path(e, _)` consumes `path(b, _)`,
      which is in `path(a, _)`'s SCC but is not its root, so the merge must
      reach down to that root (S18 step 2). `h` enters `path(a, _)`'s table
      only after `path(e, _)` has failed back, through `path(g, _)`.
    - `[ [ once [ path a Y ] ] [ path a Z ] ]` answers every Z.
    - **E54.** With `p/0` and `q/0` tabled, `[ [ p ] ← [ not [ q ] ] ]` and
      `[ [ q ] ← [ p ] ]` make `[ p ]` E54: `q` merges into `p`'s SCC across
      the `Not`. Keeping `[ [ q ] ← [ p ] ]` and replacing `p`'s clause,
      `[ [ p ] ← [ if [ [ q ] ] [ true ] [ fail ] ] ]` is E54 across a
      running `Else`, and `[ [ p ] ← [ findall X [ [ q ] ] L ] ]` across a
      `Collect`.
    - **The join rule itself.** A unit test in `kb/table.rs` drives the
      completion stack through the second KB's call sequence (leaders
      `path(a, _)`, `path(b, _)`, `path(c, _)`, `path(e, _)`; consumers
      `path(a, _)` from `path(c, _)` and `path(b, _)` from `path(e, _)`)
      and asserts that every entry ends under the root `path(a, _)`. The
      KB tests show the answers are right; this one shows the merge
      reached from V's SCC root and not from V's own entry.
21. **Saturation.**
    - The saturated `path` facts for a source equal the tabled answers for
      that source.
    - Saturating twice gives the same value.
    - Each E55 reason, and E56, is tested.

### Phase 3 **[L10] [L14]**

22. **Domains.**
    - `[ in X 0 2 ] [ #= X 3 ]` fails without labeling.
    - `[ in X 0 9 ] [ #> X 7 ]` leaves `[ in X 8 9 ]` in `_residual`.
    - A domain with a hole leaves `fd_dom`. Re-posting it restores the
      domain exactly.
23. **Puzzles.**
    - SEND+MORE=MONEY answers its one solution.
    - 8-queens with `labeling [ :ff ]` answers 92.
24. **Edges.**
    - Bounds at `i64::MIN` and `i64::MAX`, and a sum of four full-range
      products, propagate without overflow.
    - A lower bound above `i64::MAX` fails.
    - `[ ins [ X Y ] 0 1000000000000 ] [ #> X Y ] [ #> Y X ]`, with
      `inferences` 1,000,000, ends in E30.
    - E60–E66 are tested.

### Examples

25. **The examples run unattended.** `bund_examples/logic_demo/` runs every
    worked example without a terminal (example 5 with its five seeds).
    Its output is checked in as `bund_examples/logic_demo/expected.txt`,
    written from each example's comments, and a test in
    `crates/bund2-cli/tests` runs the demo and compares it with that file.
    It is not a golden: the oracle has none of these words.

---

## Rulings requested — D226

*Ruled 2026-10-28, "all as recommended": each row's **Recommended**
column is the ruling (D226, RESOLVED).*

| | Question | Options | Recommended |
|---|---|---|---|
| **L1** | What a variable is | **A** a CALL starting uppercase or `_`, without `\|`. **B** `?x`. **C** a PTR. | **A** |
| **L2** | How constants compare | **A** atoms by text; INT ≠ FLOAT; floats by key equality. **B** by `dt`. | **A** |
| **L3** | Where a KB lives | **A** a MAP value. **B** a store held by the engine. | **A** |
| **L4** | Lists and atoms | **A** `nil` is the empty list and not an atom; a LIST is a compound only when its first item is an atom CALL; a compound with no arguments is its atom. **B** an explicit `[ list … ]` for data lists. | **A** |
| **L5** | The shape of an answer | **A** a MAP per answer; LISTs from `kb.call`. **B** the instantiated goal. | **A** |
| **L6** | Answers one at a time | **A** `query`'s `lambda`. **B** cursor values. | **A** (D84) |
| **L7** | The occurs check | **A** always on. **B** off (ISO). **C** per KB. B and C need cycle-safe walks throughout, and a refusal for cyclic read-back. | **A** |
| **L8** | An undefined predicate | **A** an existence error. **B** failure. | **A** |
| **L9** | The default build | **A** in it. **B** a feature, off by default (D28). | **A** |
| **L10** | CLP(FD) | **A** Bund2's own. **B** a pinned crate (D38). **C** out of scope. | **A** |
| **L11** | Certainty through `q` | **A** yes. **B** an explicit argument. **C** defer. | **C** |
| **L12** | The conditional's name | **A** `query`. **B** `kb.run`. **C** `?query`. | **A** |
| **L13** | Opaque constants | **A** key equality, which is identity (D13, D20, D88). **B** structural. | **A** |
| **L14** | What acceptance covers | **A** every phase, built in order, each gated by its criteria. **B** Phase 1 only. | **A** |
| **L15** | Residuals in answers | **A** `_residual`. **B** omitted. | **A** |

---

## Open questions

These are entered in `docs/registers/open-questions.md`.

- **Q44 [UNGROUNDED].** The quotations were read through a summarising
  fetch, and no example has been executed.
- **Q45.** Answers lose the aliasing between unbound variables.
- **Q46.** A compiled-form cache keyed on the KB's payload.
- **Q47.** Whether `kb.word` should put the arity in the word's name.
- **Q48.** Whether the debugger should name the goal being solved, and
  whether the profiler should report time per predicate.
- **Q49.** Whether a bridge body should inherit λ's source positions.

---

## Appendix A: review history

*Note, 2026-10-28, on landing:* none of the review files this appendix
names is in the repository, nor is the pre-RFC research document. The
tables below are the draft's own account and could not be checked.

| Revision | What changed |
|---|---|
| Pre-RFC | the research document `06-logic-programming.md` checked against source: nine corrections, in ERRATA |
| 1 | the first draft |
| 2 | answers the first review (4 blocking, 14 should-fix, 12 notes); F231 filed; D226 entered |
| 3 | specifies Phases 2 and 3 in full |
| 4 | answers the second review (3 blocking, 18 should-fix, 16 notes) |
| 5 | answers the third review (2 blocking, 10 should-fix, 14 notes); consolidates |
| 5, landed | D226 renumbered; citations converted to symbols; missing files noted |
| 6 | answers the fourth review (1 blocking, 12 should-fix, 8 notes); F231 fixed apart |
| 7 | answers the fifth review (no blocking, 4 should-fix, 3 notes) |

**Where each finding is answered.**

**The pre-RFC's nine corrections** are in S2, S9, S6, S10, S8, S1/S5,
S5/S6, S11 and criterion 11.

**The first review (`-2026-10-28.md`):**

| Finding | Answered by |
|---|---|
| B1 | `kb.call`'s LISTs (S9) |
| B2 | the two caps (S7) |
| B3 | superseded by the second review's B1 (S6) |
| B4 | F231, and the restore (S5) |
| S1 | the guarantees (S5) |
| S2 | `SWITCHES` (S6) |
| S3 | the registers consumed (header) |
| S4 | `BUILT` (S9) |
| S5 | `test` (S5) |
| S6 | substitution (S5) |
| S7 | NODATA (S1) |
| S8 | the examples |
| S9 | criterion 4 |
| S10 | `kb/` (S6) |
| S11 | the markers and the registers |
| S12 | no index (S2) |
| S13 | the reserved set (S3) |
| S14 | the citations |
| N1–N12 | in place |

**The second review (`-2.md`):**

| Finding | Answered by |
|---|---|
| B1 | the name pool (S6) |
| B2 | SCC completion (S18), refined by the third review's B2 |
| B3 | pairs matched by unification (S16) |
| S1, S3 | a stack per bridge (S5) |
| S2 | criterion 7's wrapper |
| S4 | the stores table (S4) |
| S5 | wake-ups (S4) |
| S6 | `Ask` (S16) |
| S7 | proofs (S15) |
| S8 | stratification (S19) |
| S9 | the texts (S12) |
| S10 | superseded by `fd_dom` (S8) |
| S11 | the zero-arity rule (S1) |
| S12 | E26 |
| S13 | examples 3 and 5 |
| S14 | the plan |
| S15 | the markers |
| S16 | the citations |
| S17 | the `i128` sentinels (S21) |
| S18 | the choicepoint kinds (S4) |
| N1–N16 | in place; N16 completed by the third review's N4 |

**The third review (`-3.md`):**

| Finding | Answered by |
|---|---|
| B1 | the occurs check always on (L7 A, S1, S4): cyclic terms cannot arise |
| B2 | the completion stack and its join rule; E54 widened to merges across `Not`, a running `Else` or `Collect` (S18); criterion 20 |
| S1 | collision with a program's stack (S5 step 3; criterion 7) |
| S2 | `nil` is not an atom; the zero-arity rule is marked [L4]; D226's L4 row is updated |
| S3 | replies are read as terms; seeds must be ground (E58); E50 is raised only at its own step; five questions; a seeded example |
| S4 | `findall` proofs go in the side store; `Consumer`'s fields (S4) |
| S5 | `fd_dom` (S8, S20) |
| S6 | E24 names the goal |
| S7 | audit entries named as the scans name them (S6, criterion 12) |
| S8 | criteria reassigned to milestones (the plan) |
| S9 | refusal cells per word (S9, S11); E59; E08 for `kb.word` |
| S10 | each propagator run is an inference (S4, S21); criterion 24 |
| N1 | a λ that drops the scratch stack (S5) |
| N2 | choicepoint fields are not trailed; `$asked(h)` is addressed by height |
| N3 | E59 |
| N4 | answers compared with their attributes (S18) |
| N5 | `$answer`; the wake-up queue and the completion stack in the stores table |
| N6 | the trial runs no wake-ups (S17) |
| N7 | the ±∞ table (S21) |
| N8 | [L4] |
| N9 | [L14] marked throughout |
| N10 | `kb.consult*` is abandoned by `check` |
| N11 | `kb.saturate` keeps the input's clauses |
| N12 | `VmBridge` (S6) |
| N13 | criterion 8's probe |
| N14 | example 6 quoted |

**The fourth review (`-4.md`), the first on file:**

| Finding | Answered by |
|---|---|
| B1 | criterion 20's KBs written out, with what each tests |
| S1 | F231 marked fixed (status, Current behaviour, S5 step 3.4, Preservation, criterion 7) |
| S2 | D36's scan walks subdirectories (S6; M1; criterion 12a) |
| S3 | the conjunction trap, with its spelling and E12 (S1) |
| S4 | criterion 4's probe native |
| S5 | E40's test in `oom_guard.rs` (criterion 1b) |
| S6 | criterion 2a's enumerated table; no dependency |
| S7 | criterion 25's `expected.txt` and its test |
| S8 | `REPORTS_MID_BODY` in criterion 12a |
| S9 | every choicepoint's continuation and marks (S4) |
| S10 | which poll meets a trip, and its code (S5) |
| S11 | improper read-back (S1); a non-list `in` is E23 (S5, S12); `nil` as a goal is E11 (S3) |
| S12 | the header's SHAs and the ERRATA line |
| N1 | the `∅` trap (S1) |
| N2 | RFC-0010 under "Bears on"; the class trap (S1); classes (S14) |
| N3 | `true` and `false` read back (S1) |
| N4 | the epoch bumps (S13) |
| N5 | a drop of a dropped stack (S5 step 3.5) |
| N6 | `kb.consult*` (S9) |
| N7 | `limit` 0 (S11) |
| N8 | the worked examples' preamble |
| Preservation | F231's fix; test-pinned lists; name collisions; λ seeing its stack |
| Assumptions | `lambda`'s stack (S11); nested prefixes (S5); `fresh_name` (S5); the poll's cost (criterion 16) |

**The fifth review (`-5.md`):**

| Finding | Answered by |
|---|---|
| S1 | criterion 4 samples at the 100,000th and 1,000,000th answers, both after `lambda` is hot |
| S2 | the choicepoint preamble names only the marks; resumption is in "On backtrack" (S4) |
| S3 | criterion 12a asserts the scan read a file under `kb/` |
| S4 | criterion 20's `if` and `findall` programs |
| N1 | `debug.display_stack` shows values, not the name (S5) |
| N2 | criterion 20's unit test on the join rule |
| N3 | Q44's register row narrowed |
| Assumption | criterion 7's wrapper delegates to an `Interp`, in `bund2-stdlib`'s tests |
