# Defect register

Defects in the existing implementation, found during analysis. Each needs a
`disposition`: PRESERVE (Bund2 reproduces the bug) or FIX (Bund2 corrects it,
and the affected golden is regenerated with a reference to this entry).

Fixing a behavioural defect is a deviation from 100% preservation and needs an
explicit decision. Leaving `disposition` empty blocks any work item that would
touch the area.

---

## F1 — `unregister` registered twice
The class variant shadows the lambda variant, so lambda unregistration is
unreachable by name. The class one is presumably meant to be `unregister.class`.
- `reference/rust_multistackvm/src/stdlib/lambdas/registry.rs`
- Behavioural. Disposition:

## F2 — `if.false.in_workbench` uses the wrong stack
`stdlib_logic_if_false_in_workbench` passes `StackOps::FromStack`, not
`FromWorkBench`.
- `reference/rust_multistackvm/src/stdlib/logic/if_fun.rs`
- Behavioural. Disposition:

## F3 — `stdlib_math_op_inline` checks the wrong stack
The `FromWorkBench` arm checks `current_stack_len()` before separately checking
`workbench.len()`.
- `reference/rust_multistackvm/src/stdlib/math/math_op.rs`
- Behavioural. Disposition:

## F4 — redundant clone in `push_to_workbench`
Clones an owned value, pushes the clone, drops the original.
- `reference/rust_multistack/src/ts_workbench.rs`
- Performance only. Disposition: FIX

## F5 — `_inline` suffix rebuilt three times per call
`is_inline` formats it once; `get_inline` formats it again for `contains_key`
and a third time for `get`.
- `reference/rust_multistackvm/src/multistackvm_inline.rs`
- Performance only. Disposition: FIX

## F6 — alias resolved twice per CALL
Once in `apply`, again in `i()`.
- `reference/rust_multistackvm/src/multistackvm_apply.rs`, `multistackvm_inline.rs`
- Performance only. Disposition: FIX

## F7 — instrumentation in the dispatch path
`time_graph::instrument` on `apply`, `i`, `i_direct`, `call`, `lambda_eval`,
`stdlib_execute_base_inline`, `stdlib_logic_if_base`, `stdlib_logic_times`.
Must be removed or feature-gated before any baseline measurement.
- Performance only. Disposition: FIX

## F8 — unbounded inter-crate version pins
`">=0.*.*"` between the five library crates: a `Value` layout change propagates
silently.
- Resolved by the monorepo. Disposition: FIX (structural)

## F9 — the parser has a side channel
The `ctx` rule mutates the caller's `state` vector rather than returning a
subtree, which makes `( ... )` unanalysable.
- `reference/bund_language_parser/src/vm/ctx.rs`
- Structural. Disposition: FIX (scoped block node, RFC-0003)

## F10 — debugger history written to the working directory
- `reference/Bund/src/stdlib/functions/debug_fun/`
- Cosmetic. Disposition: FIX

## F11 — inverted guard in `register_method_value_init`
`if ! value.type_of() == OBJECT` parses as `(!value.type_of()) == OBJECT`; the
guard never fires as intended.
- `reference/Bund/src/stdlib/functions/oop/value_class.rs`
- Behavioural. Disposition:

## F12 — `Ord::cmp` disagrees with `PartialOrd::partial_cmp` for floats
`partial_cmp` handles `Val::F64` (`reference/rust_dynamic/src/ord.rs:19-21`),
but `cmp` has no `Val::F64` arm: two FLOATs fall through to
`self.id.cmp(&other.id)` (`reference/rust_dynamic/src/ord.rs:199`), ordering
by random nanoid. This violates the std requirement that `Ord::cmp` agree with
`PartialOrd::partial_cmp`.

Currently latent. The only corpus sort path is
`algos::sort::quicksort::sort::<Value>`
(`reference/Bund/src/stdlib/functions/values/sort_lists.rs:36`); that
implementation compares exclusively with the `>` operator, which dispatches
through `PartialOrd`, never `Ord::cmp`. So
`reference/Bund/tests/testing_sorting_numbers_in_list.bund:7` sorts 15 floats
correctly and its assertion holds. The defect becomes reachable the moment
anything calls `.cmp()`, `min`/`max`, `slice::sort`, or puts a `Value` in a
`BTreeMap`/`BTreeSet`.

Consequence for Bund2: `PartialOrd`, not `Ord`, is the authority on observable
ordering. Reimplementing `cmp` "correctly" for floats is a behaviour change on
a path the reference has, even though no golden covers it.
- `reference/rust_dynamic/src/ord.rs:167-204`
- Behavioural, latent. Disposition: **FIX** — Bund2 implements `Ord::cmp` with
  a proper `Val::F64` arm, consistent with `partial_cmp`
  (`reference/rust_dynamic/src/ord.rs:19-21`). No golden regenerates: the
  defective arm is unreachable through `sort`, so no captured output depends
  on it.

## F13 — `dup` deep-copies through a bincode round-trip
`Value::dup` serialises the value to bytes and deserialises it back, then
regenerates the id (`reference/rust_dynamic/src/dup.rs:7-13`). `Value` derives
`Clone` (`reference/rust_dynamic/src/value.rs:15`) and every field is deeply
cloneable, so a structural clone would do.

This is not a cold path. The chain is `dup` (alias of `dup_one`,
`reference/rust_multistackvm/src/stdlib/create_aliases.rs:18`) ->
`stdlib_dup_one_in_current_inline`
(`reference/rust_multistack/src/stdlib/dup.rs:31`) -> `dup_in_current_stack`
(`reference/rust_multistack/src/ts_stack_op.rs:30`) -> `val.dup()`
(`reference/rust_multistack/src/ts_stack_op.rs:34`). `dup` is **55
invocations across 38 of 132 programs** — one of the ten most-used words in
the corpus. `?move`/`?.` reach the same code at `ts_stack_op.rs:72,84`.

Two consequences beyond speed:

1. **It interacts with D1/D2.** If a lazily-generated id or stamp must be
   materialised in order to serialise, then every `dup` materialises both, and
   the laziness those decisions bought is lost on one of the hottest words in
   the language. See Q8.
2. **For JSON values it may not be identity.** `to_binary` special-cases
   `dt == JSON` by converting to a string and re-wrapping
   (`reference/rust_dynamic/src/bincode.rs:9-28`), and `from_binary` re-parses
   it through `serde_json`
   (`reference/rust_dynamic/src/bincode.rs:54-69`). A round-trip through JSON
   text is not guaranteed to preserve key order or numeric spelling, so `dup`
   on a JSON value is behavioural, not merely slow.

- `reference/rust_dynamic/src/dup.rs:7-13`
- Performance for most types; behavioural for JSON. Disposition: **FIX** —
  `dup` becomes a structural clone plus fresh identity. `Value` derives `Clone`
  (`reference/rust_dynamic/src/value.rs:15`) and every field deep-clones, so
  this is the same operation without the round-trip. Required by D20: leaving
  the round-trip in place would force lazy `id`/`stamp` materialisation on one
  of the ten most-used words and make D1 and D2 decorative.

  Deviation to state in the implementing work item: for JSON values the
  reference round-trips through `serde_json` text
  (`reference/rust_dynamic/src/bincode.rs:9-28,54-69`), which need not
  preserve key order or numeric spelling; a structural clone preserves the
  value exactly. No golden covers this — `reference/Bund/examples/json_answer.bund`
  is the only corpus program using JSON words, and it never dups.

## F14 — error text embeds a value's `id` and `stamp`, so it cannot reproduce
`reference/Bund/src/stdlib/helpers/eval.rs:33` formats the failing value with
`{:?}`: `bail!("Attempt to evaluate value {:?} returned error: {}", &word, err)`.
`Value`'s derived `Debug` (`reference/rust_dynamic/src/value.rs:15-18`) prints
`id` and `stamp`, so every such message carries a fresh nanoid and a wall-clock
millisecond reading. `reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:78`
does the same. The end-of-run stack dump
(`reference/Bund/src/stdlib/helpers/print_error.rs:126,155`) prints values the
same way.

Found empirically, not by reading: running the oracle twice over the hermetic
suite and diffing. It is invisible to static analysis because every word
involved is pure or stdout — the non-determinism enters through `Debug`, not
through any word's effect.

**This is the single largest source of unreproducible output in the corpus**:
of 77 suite programs, 18 differ between runs and 14 of those differ only in
these embedded ids and stamps.

Interacts with D1/D2: under lazy identity, `Debug` would materialise both
fields purely to print them.
- `reference/Bund/src/stdlib/helpers/eval.rs:33`
- Behavioural. Disposition: **FIX** — Bund2's error text carries the failing
  value's *data*, not its `Debug`. The end-of-run stack dump does the same.

  This deviates from nothing observable. The id and stamp differ on every run
  by construction, so there is no behaviour there to preserve — only the rest
  of the message is a contract. Removing them makes the message *more*
  reproducible, not less faithful.

  Consequence for golden capture: the oracle will keep emitting them, so
  capture must normalise `id: "..."` and `stamp: N` before recording, and
  compare normalised. Verified empirically — normalising those two fields (and
  F15's member order) makes **15 of the 18 unstable programs reproduce**.

## F15 — dictionary and tag iteration order is unspecified
Printing a dict emits members in `HashMap` iteration order, which differs
between runs. `reference/Bund/examples/configuration_create.bund` prints
`{ type=simple ::  N=100 ::  X=0.0 ::  Step=0.1 :: }` on one run and
`{ X=0.0 ::  N=100 ::  type=simple ::  Step=0.1 :: }` on the next. `Value.tags`
is a `HashMap<String, String>` (`reference/rust_dynamic/src/value.rs:24`).

The same instability reaches the graph algorithms, whose result lines reorder
between runs — `reference/Bund/examples/graph_algorithms/simple_graph_dijkstra.bund`,
`simple_graph_allshortpath.bund`, `simple_graph_transitiveclosure.bund` — and
the exception payload in
`reference/Bund/examples/code_snippets/application_conditional_demos.bund`.

Bund2 cannot be conformant against a golden that reorders. Fixing this means
choosing a deterministic map for the value representation, which is RFC-0001
territory and interacts with D13.
- `reference/rust_dynamic/src/value.rs:24` and the dict implementation
- Behavioural. Disposition: **FIX** — Bund2 uses a deterministic map for dict
  members and tags, so printing is stable.

  As with F14, this deviates from nothing: `HashMap` specifies no iteration
  order, so the reference has no order to preserve. Choosing one *adds* a
  guarantee. Which order — insertion or sorted — is a value-representation
  question for RFC-0001, and it interacts with D13; this entry fixes only that
  the order must be deterministic.

  Consequence for golden capture: the oracle remains unordered, so comparison
  must normalise member order. A golden therefore pins dict *content*, not
  dict order — which is the most that can honestly be claimed against a
  reference that does not define one.

  Does not cover the graph algorithms — see F17, a separate cause.

  **Second surface, found while implementing `cargo xtask golden`.** The
  unordered map reaches output through Rust's `Debug` as well as through the
  Bund display form: an OBJECT prints as `Map({".super": Value { .. }, ..})`,
  members in `HashMap` order. Every OOP program in the suite differed between
  runs on this alone — `create_object.bund`, `value_demo.bund`,
  `class_display_demo.bund` and nine more — even after the display-form
  members were sorted. Same defect, same fix; it simply has two rendering
  paths, and a normaliser that handles only one recovers none of these
  programs.

## F16 — `!` on a CLASS always fails
`execute`'s CLASS arm (`reference/rust_multistackvm/src/stdlib/execute.rs:86-88`)
delegates to `execute_class`, which pushes the **class value** and calls
`stdlib_object_inline`
(`reference/rust_multistackvm/src/stdlib/bund_execute/execute_class.rs:8-11`).
That function immediately `cast_string()`s its operand to obtain a class *name*
(`reference/rust_multistackvm/src/stdlib/bund_object.rs:179`), so a CLASS value
always fails with `OBJECT returns error: This Dynamic type is not string`.

The arm is unreachable in any useful sense: it cannot succeed for the only type
that routes to it. Confirmed against the oracle by
`tests/probes/execute-arm-class.bund`. No corpus program executes a class, so
no golden covers it.
- `reference/rust_multistackvm/src/stdlib/bund_execute/execute_class.rs:8-11`
- Behavioural. Disposition: **FIX** — `<class> !` creates an object of that
  class. Proposed by the repository owner. This is what `execute_class`
  evidently intends: it already routes to the object machinery, and only the
  operand type is wrong.

  Nothing can depend on the present behaviour, because the arm always errors
  and no corpus program executes a class. So this adds working behaviour
  rather than changing any.

  One constraint the implementation must respect, and it is not obvious: **a
  CLASS value does not know its own name.** `stdlib_class_inline`
  (`reference/rust_multistackvm/src/stdlib/artefacts.rs:69-73`) creates a bare
  class with only `.super` set, and `register` obtains the name by pulling it
  from *beneath* the class on the stack
  (`reference/rust_multistackvm/src/stdlib/classes/registry.rs:9-20`) — which
  is why the idiom is `:Name class ... register`. Meanwhile the existing
  instantiation path resolves a class *name* through the class registry
  (`reference/rust_multistackvm/src/stdlib/bund_object.rs:176`), and `.super`
  holds parent class *names*
  (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:95`), so parent
  construction needs the registry regardless.

  Settled by D23 (answering Q12): **both provenances work**. `!` builds the
  object from the CLASS value it is given — never by a registry lookup of the
  class itself — so a class constructed dynamically on the stack and never
  registered instantiates just as one resolved back from the registry does.
  Parents still resolve by name through the registry. One residual is carried
  as Q13: an object made from an anonymous class has no `.class_name`.

  `tests/probes/execute-arm-class.bund` currently pins the failure and becomes
  the proof of the fix.

## F17 — graph algorithm results are returned in `HashMap` order
`reference/Bund/examples/graph_algorithms/simple_graph_dijkstra.bund`,
`simple_graph_allshortpath.bund` and `simple_graph_transitiveclosure.bund`
print their result lines in a different order on every run.

This is **not** F15. The unordered container is not Bund's dict — it is the
return type of the dependency: `algos::cs::graph::dijkstra::shortest_paths`
returns `HashMap<V, Option<W>>`
(`~/.cargo/git/checkouts/algos-9d1538761d16fda1/4c08437/src/cs/graph/dijkstra.rs:71`,
reached from `reference/Bund/src/stdlib/functions/graph/dijkstra.rs:8`). Bund
iterates that map to emit one line per node, so whole *lines* reorder rather
than members within a line.

Confirmed empirically: normalising F14's id/stamp and F15's member order
recovers 15 of the 18 unreproducible programs. These three are the remainder.

Fixing it means Bund2's graph words impose an order on results the dependency
does not. That is the same "adds a guarantee" argument as F15, but it lands in
`bund/graph`, which D14 has not ruled on — `graph!` is used by 5 corpus
programs and the subsystem is otherwise library-shaped.
- `reference/Bund/src/stdlib/functions/graph/dijkstra.rs:8` and siblings
- Behavioural. Disposition:

## F18 — depth guards under-declare arity for 14 words
A word normally opens with `if vm.stack.current_stack_len() < N { bail!("Stack
is too shallow ...") }`. For 14 words that `N` is smaller than what the word
actually pulls, so the guard passes and the *second* pull fails with a
different message.

`pair` is the clearest: it guards `< 1`
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:7`) and then pulls twice,
so `1 pair` reports `NO DATA #2` rather than "Stack is too shallow for inline
pair()". Confirmed against the oracle.

The full set, from `cargo xtask arity` — each declares 1 and consumes 2:
`at`, `complex`, `head`, `pair`, `string.distance`,
`string.distance.dameraulevenshtein`, `string.distance.hamming`,
`string.distance.jarowinkler`, `string.distance.levenshtein`,
`string.distance.sift3`, `string.regex`, `string.regex.matches`,
`string.wildcard`, `tail`
(`reference/rust_multistackvm/src/stdlib/values/value_carcdr.rs:199,203`,
`reference/rust_multistackvm/src/stdlib/artefacts.rs:143,144`,
`reference/Bund/src/stdlib/functions/string/distance.rs:139-147`,
`reference/Bund/src/stdlib/functions/string/regex.rs:99`,
`reference/Bund/src/stdlib/functions/string/regex_matches.rs:104`,
`reference/Bund/src/stdlib/functions/string/wildmatch.rs:89`).

Two consequences:

1. **The error message differs from every other arity failure**, so a program
   catching or printing it sees `NO DATA #2` where it would elsewhere see
   "Stack is too shallow". That is observable behaviour, which is why this is
   a defect and not a tidy-up.
2. **The declared guard cannot be trusted as an arity source.** RFC-0004 must
   take the probed column of `docs/arity.md`, not the declared one, wherever
   the two disagree.

No corpus program hits any of these paths, so no golden covers them.
- see the citations above
- Behavioural. Disposition:

## F19 — the stack layer's `functions` table is dead code
`rust_multistack` keeps a fourth name-keyed table alongside the two inline
tables: `functions: HashMap<String, AppFn>`
(`reference/rust_multistack/src/ts.rs:21`), filled by `register_function`
(`reference/rust_multistack/src/ts_functions.rs:6`) from **27 call sites**
across `reference/rust_multistack/src/stdlib/`.

Nothing reaches it. The map is read only by `get_function`
(`reference/rust_multistack/src/ts_functions.rs:25`), which is called only by
`TS::f` (`reference/rust_multistack/src/ts_functions.rs:36`), and `TS::f` is
called from nowhere in any of the six crates. `i_direct` consults the two
*inline* tables only
(`reference/rust_multistackvm/src/multistackvm_inline.rs:42,52`), so no name
registered here is reachable as a word, from Bund source or from Rust.

This resolves the contradiction RFC-0000 recorded but did not settle — three
dispatch tiers against four tables. The fourth is not an embedding API and not
a second dispatch path; it is 29 registrations of an unused parallel table,
most of them duplicating a name already registered as an inline word in the
same file (`reference/rust_multistack/src/stdlib/drop.rs:70,71` registers
`drop` both ways).

Consequence for RFC-0002: **bund2 does not need a fourth `WordEntry` variant.**
The slot table absorbs three tiers, not four. Porting the table because it
exists would add a name space the language does not have.
- `reference/rust_multistack/src/ts_functions.rs:6,25,36`
- Dead code. Disposition:

## F20 — the `swap` alias shadows a different inline word
`swap` is registered twice, in two namespaces. `reference/rust_multistack/src/stdlib/swap.rs:98`
registers it as an inline word backed by `stdlib_swap_in_current_inline`;
`reference/rust_multistackvm/src/stdlib/create_aliases.rs:19` registers it as
an alias of `swap_one`, backed by `stdlib_swap_one_in_current_inline`
(`reference/rust_multistack/src/stdlib/swap.rs:100`).

Alias resolution runs before the inline tables
(`reference/rust_multistackvm/src/multistackvm_apply.rs:39`), so the alias
wins and the inline `swap` is unreachable by name. Verified against the
oracle: `1 2 3 swap` and `1 2 3 swap_one` both leave `1 3 2`.

This is the same shape as F1, where the class `unregister` shadows the lambda
one — and it is why the "617 distinct registered names" figure is not merely a
deduplication detail. The two `swap` registrations are different functions with
different arity, not two spellings of one.

- `reference/rust_multistack/src/stdlib/swap.rs:98,100`,
  `reference/rust_multistackvm/src/stdlib/create_aliases.rs:19`
- Behavioural. Disposition:

## F21 — the oracle is not built from the pinned submodules
`reference/Bund/Cargo.toml:13,14,23,24,43` are **registry** dependencies and
there is no `[patch.crates-io]`, so building the oracle links published crates
rather than the sibling submodules. `reference/Bund/Cargo.lock` resolves
`rust_dynamic` 0.49.0, `bundcore` 0.7.0, `bund_language_parser` 0.14.0,
`rust_multistack` 0.33.0 and `rust_multistackvm` 0.38.0 — while three of those
submodules declare newer versions: 0.50.0, 0.8.0 and 0.15.0.

So every golden was produced by registry source, and every `path:line` in
every RFC points at submodule source. They agree today — `cargo xtask cite`
compares the `src/` trees byte for byte and all five match — but nothing made
them agree, and the guard RFC-0000 proposed for this class (an empty
`git status` inside each submodule) is blind to it by construction, because
the submodules are not build inputs.

This is F8 — the reference's unbounded inter-crate pins — reproduced inside
this project's own methodology.

Mitigated rather than fixed: `cargo xtask cite` now verifies provenance on
every run and in CI, hard-failing on any byte divergence and reporting the
version skew as an advisory. The fix proper is a `[patch.crates-io]` section,
which cannot be written because `reference/` is read-only; Bund2's own
workspace must avoid the shape entirely.
- `reference/Bund/Cargo.toml:13,14,23,24,43`, `reference/Bund/Cargo.lock`
- Methodological. Disposition:

## F22 — `<-` and `←` alias a word that dispatch cannot reach
`stacks_left` is registered only through `register_function`
(`reference/rust_multistack/src/stdlib/rotate.rs:93`), the dead table of F19,
and never through `register_inline`. Two aliases point at it:
`reference/rust_multistackvm/src/stdlib/create_aliases.rs:22,23`.

Since `i_direct` consults only the inline tables, both aliases are dead.
Confirmed against the oracle: `1 2 <-` fails with
`i(stacks_left) for stack returned: Inline stacks_left not registered`, while
the mirrored `stacks_right` — which *is* registered inline
(`reference/rust_multistack/src/stdlib/rotate.rs`) — works.

So the language documents a left-rotation word it cannot execute, and the
asymmetry with `stacks_right` suggests the registration call was simply
written against the wrong function. Consequence for F19: dropping the dead
table also drops `<-` and `←` unless `stacks_left` is re-registered inline,
which is a fix rather than a removal.
- `reference/rust_multistack/src/stdlib/rotate.rs:93`,
  `reference/rust_multistackvm/src/stdlib/create_aliases.rs:22,23`
- Behavioural. Disposition:

## F23 — `rotate_stack_right` rotates left
`stdlib_stack_right_inline` (`reference/rust_multistack/src/stdlib/rotate.rs:83-89`)
ends by calling `stdlib_stack_left` (`:88`), and it is what
`rotate_stack_right` is registered to (`:102`). So the word rotates the
current stack left.

Two corrections to the first draft of this entry, both material. The affected
word is `rotate_stack_right`, not `stacks_right`: line 88 sits in
`stdlib_stack_right_inline`, while `stacks_right` is registered to
`stdlib_stacks_right_inline` (`:17-19`, `:94`), which is correct. And the
claim that the pair was "broken in both directions" was false — verified
against the oracle, `1 2 ->` gives `2`. `stacks_right` works.

What is true is narrower and still worth recording: `rotate_stack_right` is
also registered twice, as a function (`:101`) and inline (`:102`), which is
another instance of the F24 pattern.

No corpus program uses `rotate_stack_right`, `rotate_stack_left`, `->` or
`<-`, so no golden covers it.
- `reference/rust_multistack/src/stdlib/rotate.rs:83-89,101,102`
- Behavioural. Disposition:

## F24 — four names exist only in the dead table, and `push` is registered twice
Comparing every `register_function` name against every `register_inline` name
in `reference/rust_multistack/src` leaves six that are function-only: `dup`,
`dup_in`, `from_workbench`, `push`, `push_to`, `stacks_left`.

Two of those are reachable by another route. `dup` resolves through the alias
to `dup_one` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:18`),
and `push` is registered inline by the Bund crate
(`reference/Bund/src/stdlib/functions/values/push.rs:74`) — so the stack
layer's `push` registration is a third instance of the F1/F20 pattern: the
same name bound twice to different functions, one of them unreachable.

That leaves **four genuinely dead words**: `dup_in`, `from_workbench`,
`push_to`, `stacks_left`. F22 covers `stacks_left` and the two aliases into
it; the other three have no aliases and no corpus uses.

Whether Bund2 revives or omits them is a preservation deviation either way,
so it is a decision rather than a disposition: **D29**.
- `reference/rust_multistack/src/stdlib/dup.rs`, `workbench.rs`, `push.rs`,
  `rotate.rs`; `reference/Bund/src/stdlib/functions/values/push.rs:74`
- Behavioural. Disposition:

## F25 — a dead dispatch cluster holds a second, divergent resolution order
`apply_in`, `call_in` and `lambda_eval_in`
(`reference/rust_multistackvm/src/multistackvm_apply_in.rs`,
`multistackvm_call.rs:12`, `multistackvm_lambda_eval_in.rs`) form a
self-referential cluster: each is called only by the others, and nothing
outside calls any of them.

It matters because it is not a copy of the live path, though the first draft
of this entry got the difference wrong. There is **no inversion**: `apply` and
`apply_in` test `if self.autoadd` in the same position and the same polarity
in all three arms (`multistackvm_apply.rs:19,72,89` against
`multistackvm_apply_in.rs:15,45,62`).

The two real divergences in the CALL arm are these. `apply_in` has **no
`$`-prefix arm** — it goes from the `autoadd` test straight to alias
resolution (`multistackvm_apply_in.rs:15-20`), where `apply` checks for the
sigil first (`multistackvm_apply.rs:33`). And under `autoadd` the two do
different things: `apply` pulls the top value and appends the name to it
(`:20-27`), while `apply_in` pushes the name onto the named stack whole
(`multistackvm_apply_in.rs:16`).

The live path is unaffected — `!` reaches `vm.call`, which uses `apply`
(`reference/rust_multistackvm/src/stdlib/execute.rs:32`). The first draft
tried to demonstrate that with `1 2 "$drop" ptr !` and demonstrated nothing:
`drop` has no alias, so both the `$` and plain spellings converge on the same
inline word. See F26 for what a discriminating pair looks like, and for why
the `$` arm is not the bypass its comment claims.

Consequence for RFC-0002: the dispatch contract has one resolution order to
specify, not two, and porting this cluster would import a second.
- `reference/rust_multistackvm/src/multistackvm_apply_in.rs`,
  `multistackvm_lambda_eval_in.rs`, `multistackvm_call.rs:12`
- Dead code. Disposition:

## F26 — `$name` does not bypass alias resolution
The comment above the `$` arm says the prefix forces an internal call
"without lambda check or alias resolution"
(`reference/rust_multistackvm/src/multistackvm_apply.rs:29-32`). Half of that
is true. `call_internal_word` strips the `$` and calls `i()`
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`),
and `VM::i` resolves aliases before dispatching
(`reference/rust_multistackvm/src/multistackvm_inline.rs:71-72`). So `$` skips
the **lambda** lookup and goes straight through the **alias** table.

Confirmed against the oracle with a discriminating pair — one that turns on a
name which *is* an alias:

- `1 "$dup" ptr !` leaves two values. `dup` is an alias of `dup_one`
  (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:18`) and is not
  an inline word in its own right (F24), so it could only have been found
  *through* the alias table.
- `:Hi { … } register "Hi" ptr !` prints `HI`; the same with `"$Hi"` fails
  with "not registered" — the lambda check really is skipped.

A pair built on `drop`, which has no alias, proves nothing: both spellings
converge on the same inline word. An earlier version of F25 used exactly that
pair and drew a conclusion from it.

This has been observed before and never recorded, which is why it keeps
resurfacing: the first review reported it, and
`docs/research/00-jit-feasibility.md:119` and `:556` still assert the bypass —
`:556` proposing a distinct IR opcode on the strength of it. ERRATA entry
added.

Consequence for RFC-0002: `$` is not an escape from the name tables. It
selects *which* tables, and the alias table is not one it skips — so a
distinct opcode premised on full bypass would be wrong.
- `reference/rust_multistackvm/src/multistackvm_apply.rs:29-32`,
  `multistackvm_call_internal_word.rs:7-8`, `multistackvm_inline.rs:71-72`
- Behavioural, and a wrong source comment. Disposition:

## F27 — FIFO stacks are documented but unreachable, and `peek` disagrees with `pull`
`Introduction.typ:16` of the Library Guide states that BUND "offers you an
ability to creae a stack with FIFO policy". The machinery exists:
`Stack::fifo` (`reference/rust_multistack/src/stack.rs:27`) sets
`policy = false` (`:30`), and `TS::add_named_fifo`
(`reference/rust_multistack/src/ts_add.rs:20-27`) builds one.

`add_named_fifo` has **no caller**. Not in `rust_multistackvm`, not in the
Bund runtime, not in `rust_multistack` itself. `policy` is set false in
exactly one place — `stack.rs:30`, inside `Stack::fifo` — so every stack in a
running Bund is LIFO and both FIFO branches are dead code. No word creates a
FIFO stack.

Hidden behind that is a second defect that cannot currently be observed.
`push` honours the policy (`stack_push.rs:9,11`): LIFO pushes the back, FIFO
pushes the front. `pull` always pops the back (`stack_pull.rs:8`), with the
policy branch commented out at `:9-13` — and that is **correct**, because
pushing at the opposite end is what makes it FIFO; the commented-out version
would have popped the front and turned FIFO back into LIFO. The error is in
`peek`, which does branch (`stack_peek.rs:9,11`): on a FIFO stack it returns
`front_mut`, the newest value, while the next `pull` removes the oldest. So
`peek` and `pull` would disagree about what is on top.

Latent, not live: with no way to build a FIFO stack, no program can observe
it. It becomes live the moment Bund2 exposes the FIFO policy the guide
advertises.

- Found by: reading the Library Guide (Q17)
- Affects: whether Bund2 implements FIFO stacks at all
- Disposition: preserve the observable behaviour — every stack LIFO — and do
  not implement the FIFO policy without a decision. Exposing it would add a
  language feature the reference does not have, which is a deviation even
  though the guide describes it. If it is ever exposed, `peek` must follow
  `pull`, not `push`.

## F28 — `push_to_stack` checks the wrong stack's length against the cap
`TS::push` reads the current stack's length and the current stack's capacity,
which is consistent: `stack_name` comes from `current_stack_name()`
(`reference/rust_multistack/src/ts_push.rs:14`), and both
`current_stack_len()` (`:20`) and `stack_capacity(stack_name)` (`:21`) are
about that same stack.

`TS::push_to_stack` is not. It takes `cap` for the **named** stack
(`reference/rust_multistack/src/ts_push.rs:48`) but `stack_len` from the
**current** one (`:47`), then evicts when `stack_len >= cap` (`:54`). So
pushing to a capped stack `B` while `A` is current drops B's oldest element
based on A's depth — evicting when B is empty if A is deep, and never
evicting when A is shallow no matter how full B is.

`stack_len(name)` exists and is the function this wants
(`reference/rust_multistack/src/ts_len.rs:19-28`); `push_to_stack` calls
`current_stack_len` instead.

Reachable but untested: capacities are set only by
`ensure_stack_with_capacity`
(`reference/rust_multistack/src/stdlib/ensure_stack.rs:96`, implementation at
`:32-59`), a registered inline word with **zero corpus uses**, so no golden
covers it.

- Found by: reading `rust_multistack` for RFC-0001
- Disposition: Bund2 fixes it — use the named stack's length. A golden cannot
  disagree, because none exercises it. Record the divergence in RFC-0001.

## F29 — `valuemap` is write-only: nothing can read a `Val::ValueMap` back
`valuemap` is a registered inline word
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:147`) with the alias
`match` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:45`), and
`set` has a real `VALUEMAP` branch that inserts through `set_vmap`
(`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:17-19`).

No read path exists.

- `Value::get` dispatches on `dt` and its arm lists
  `MAP | INFO | CONFIG | ASSOCIATION | CURRY | MESSAGE | CONDITIONAL | OBJECT | CLASS`
  (`reference/rust_dynamic/src/get.rs:7`). `VALUEMAP` is absent, so a
  valuemap falls to the catch-all at `:18` and `get` returns **`self.clone()`**
  — the whole map — rather than the value under the key, and rather than an
  error.
- `Value::has_key` has the same arm without `VALUEMAP`
  (`reference/rust_dynamic/src/has_key.rs:7`) and its catch-all returns
  `make_false()` (`:19-20`), so `?key` on a valuemap always answers false.

Confirmed against the oracle. `valuemap "k" 42 set "k" get` leaves the map
itself on the stack, not `42` — `tests/probes/valuemap-hash-eq.bund`.

The failure is silent, which is what makes it worth recording: a program that
uses a valuemap gets a plausible-looking value back and no diagnostic.

- Found by: reading `rust_dynamic` for RFC-0001
- Disposition: undecided. Bund2 either implements the read path or does not
  implement `valuemap`; both deviate. See F30 — implementing the read path
  naively does not work either. Blocked pending a decision alongside D29.

## F30 — `Hash` and `PartialEq` disagree, so `Val::ValueMap` cannot key on content
`impl Hash for Value` hashes **only the id**:
`self.id.hash(hasher)` and nothing else (`reference/rust_dynamic/src/hash.rs:6`).

`impl PartialEq for Value` compares **content** for exactly four of the twenty
`Val` arms — `I64` (`reference/rust_dynamic/src/eq.rs:10`), `F64` (`:21`),
`String` (`:32`) and `Time` (`:40`). A mismatched scalar pair falls back to
`self.id == other.id` (`:15`, `:26`, `:34`, `:42`), and **every other payload
kind** reaches the catch-all at `:45`, which returns `self.id == other.id`
(`:53`).

An earlier version of this entry said the fallback fires "only when the types
differ", which is wrong and is where RFC-0001's first draft inherited the same
error. For sixteen of twenty kinds — `Bool`, `List`, `Map`, `Lambda`,
`ValueMap`, `Json` among them — equality *is* identity. D1 recorded this
correctly from the start.

Two structurally identical strings are therefore `==` but hash to different
buckets, since every construction mints a fresh id
(`reference/rust_dynamic/src/create.rs:10` and every sibling constructor).
That breaks the `Hash`/`Eq` contract, and `Val::ValueMap` is a
`HashMap<Value, Value>` (`reference/rust_dynamic/src/types.rs:79`) — a map
keyed by exactly the type whose contract is broken.

Latent today only because of F29: `get` never reaches the lookup, so nothing
can observe the miss. It becomes live the moment a read path is added, which
is why F29 cannot be fixed by adding `VALUEMAP` to `get.rs:7` alone.

This is a hard constraint on **D1**. If identity is minted lazily, then
hashing a value forces it to materialise, and two equal values must reach the
*same* id for a content-keyed map to work — which is the opposite of what a
per-construction nanoid gives. Either the value hashes by content (changing
observable `ValueMap` behaviour) or `ValueMap` keys by identity and equal-
looking keys stay distinct (preserving it).

- Found by: reading `rust_dynamic` for RFC-0001
- Disposition: RFC-0001 must state which. Recorded here; not decided.

## F31 — `resolve` cannot find any stack-layer word
`TS::register_inline` stores handlers under a **suffixed** key: it inserts
`format!("{}_inline", &name)` (`reference/rust_multistack/src/ts_inline.rs:8`),
so registering `dup_one` writes the key `dup_one_inline`.
`TS::get_inline` reads with the same suffix
(`reference/rust_multistack/src/ts_inline.rs:33,34`).

`TS::is_inline` does **not**. It tests `contains_key(&name)` with the bare
name (`reference/rust_multistack/src/ts_inline.rs:25`), which no key ever
matches, so it returns false for every word in the stack layer.

The one caller that matters is the `resolve` word
(`reference/rust_multistackvm/src/stdlib/lambdas/resolve.rs:70`). It tries
lambda, then `vm.is_inline` for the VM layer, then `vm.stack.is_inline` for
the stack layer (`:17`, `:19`, `:21`), and bails otherwise (`:23`). Since the
third test can never succeed, **`resolve` fails for every stack-layer word**.

Confirmed against the oracle. `"println" resolve` leaves a PTR (`dt: 7`);
`"dup_one" resolve` returns `RESOLVE: function dup_one not found`.

31 words are affected — every `register_inline` in `rust_multistack`,
including `drop`, `swap`, `take`, `move`, `dup_one`, `dup_many`, `clear`,
`fold` and the rotations.

Note `VM::is_inline` is correct: it adds the suffix
(`reference/rust_multistackvm/src/multistackvm_inline.rs:25`), matching the
suffix `VM::register_inline` writes (`:8`). Only the stack layer's copy of the
pattern dropped it, which is why the defect is invisible for VM-layer words.

- Found by: reading the dispatch chain for RFC-0002
- Affects: `resolve`, and any future caller of `TS::is_inline`
- Disposition: Bund2 fixes it. Under RFC-0002's interned single slot table the
  bug is not expressible — there is no suffix and no second table to disagree
  with. No golden covers `resolve`, so conformance cannot move; record the
  divergence in RFC-0002.


## F32 — `unregister` is registered twice, so no lambda can be unregistered
`init_stdlib` in the lambda registry binds the same name twice in consecutive
statements:

```rust reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:88
    let _ = vm.register_inline("register".to_string(), stdlib_lambda_register);
    let _ = vm.register_inline("unregister".to_string(), stdlib_lambda_unregister);
    let _ = vm.register_inline("unregister".to_string(), stdlib_class_unregister);
```

Registration is last-write-wins — `register_inline` unregisters before
inserting (`reference/rust_multistackvm/src/multistackvm_inline.rs:6-9`) — so
`unregister` resolves to `stdlib_class_unregister` and
`stdlib_lambda_unregister` is unreachable. **There is no way to unregister a
lambda from Bund.**

Confirmed against the oracle. Registering a lambda named `println`, calling
`:println unregister`, and calling `println` again still runs the lambda.

- Found by: reading the registration mechanism for RFC-0002
- Affects: `unregister` for lambdas; `stdlib_lambda_unregister` is dead code
- Disposition: Bund2 fixes it — the two need distinct names, or one word that
  dispatches on what the name is bound to. No corpus program calls
  `unregister`, so no golden covers it. This is also why RFC-0002's registry
  builder must **not** silently dedupe duplicate registrations: replaying them
  in order is what reproduces the reference, and deduping would change which
  handler wins. Record the divergence in RFC-0002.
