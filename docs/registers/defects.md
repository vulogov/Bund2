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
- Behavioural. Disposition: **FIX**, and see **F32**, which records the same
  defect independently: `unregister` is bound twice in consecutive statements
  (`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:89,90`) and the
  class variant wins, so no lambda can be unregistered. Confirmed against the
  oracle there. Bund2 gives the two words distinct names. This entry found it
  first from the source; F32 found it again while reading the registration
  mechanism for RFC-0002 and confirmed it by probe.

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

## F72 — `endcontext`'s guard is dead, so closing a context that was never opened destroys a stack

`endcontext` opens by refusing to run when no context is open:

```rust reference/rust_multistackvm/src/stdlib/ctx.rs:6
    if vm.stacks_stack.len() < 1 {
```

That condition is never true. `stacks_stack` is **seeded with one entry** at
construction — `ss.push_back("main".to_string())`
(`reference/rust_multistackvm/src/multistackvm.rs:38`) — and `pop_stacks`
refuses to take the last one, returning `peek_stacks()` instead of popping
when the length is not greater than 1
(`reference/rust_multistackvm/src/multistackvm_stacks_stack.rs:11-15`).
`clear_stacks` also keeps one (`:30-40`). The deque therefore never falls below
one element, and the guard is unreachable — the same shape as F11 and F69,
where a test that was written never fires.

**What runs instead is destructive.** With the guard passed, `endcontext` moves
the current stack's top to the **workbench** (`ctx.rs:9-17`) and then calls
`vm.stack.drop_stack()` (`:18`), which removes the current stack outright.

Confirmed against the oracle. `1 2 endcontext` prints nothing, and afterwards:
the stack is **empty and unnamed** — `current` answers a fresh nanoid, because
dropping the last named stack leaves the reference to invent one — the value
`1` is gone with the stack it lived on, and `2` is sitting on the workbench.

So a program that closes a context it never opened does not get an error. It
loses a stack.

The context stack itself is real and works: applying a `CONTEXT` value calls
`VM::to_stack` (`multistackvm_apply.rs:69-78`), which switches stacks *and*
records the switch (`multistackvm_to_stack.rs:8`). It is only the emptiness
test that cannot fire.

No corpus program calls `endcontext`.

- Found by: probing the words no program exercises — Bund2 refused
  `( 1 2 ) endcontext` and the reference did not, which is Q22's last open item
- Behavioural. Disposition: **FIX — Bund2 keeps the guard the author wrote.**
  `endcontext` with no context open reports `Context is empty`, which is
  `ctx.rs:7`'s own message, rather than dropping a stack.

  Bund2 also returns to the stack the context was opened *from*, which it
  recorded, instead of leaving the choice to whatever the deque holds next.
  That is not a second deviation so much as the first one's consequence: a
  guard that fires means the pop is only ever reached with a context to pop.

  **No golden covers it and none could.** The reference's post-state contains a
  stack named by a fresh nanoid, so `cargo xtask golden` would refuse the
  capture as unreproducible — the same reason `drop_stack` cannot be probed.
  `tests/probes/remaining-vocabulary.bund` names both exclusions.

## F71 — `stacks_left` is registered only as a function, so the word is unreachable

Every other word in `rotate.rs` is registered twice, as a function and inline.
Two are not:

```rust reference/rust_multistack/src/stdlib/rotate.rs:93
    let _ = ts.register_function("stacks_left".to_string(), stdlib_stacks_left);
```

`stacks_left` gets only the **function** form, and `stacks_right` only the
**inline** form (`:94`). The inline handler `stdlib_stacks_left_inline` exists
at `:9-11` and nothing registers it, so it is dead code — the same shape as F1
and F32, where a registration mistake leaves a working handler unreachable.

A bare `stacks_left` therefore fails, while the identical `stacks_right`
succeeds. **The two aliases go down with it**: `<-` and `←` both point at
`stacks_left`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:22-23`), so three
spellings of "move left around the stack ring" are unusable and one spelling of
"move right" works.

Confirmed against the oracle: `stacks_left`, `<-` and `←` each report an
error; `stacks_right` prints the new current stack.

No corpus program calls any of them.

- Found by: writing a probe for the navigation words, after `cargo xtask
  coverage` listed them among the words no program exercises
- Behavioural. Disposition: **FIX — Bund2 registers both, as every neighbour in
  that file does.** F1's precedent: a handler the author wrote and the
  registration hid is a defect, not a contract. Reproducing the unreachability
  would mean deleting a working word and three of its spellings to match a
  missing line.

  No golden covers it, so there is nothing to record as a deviation. It is
  pinned by `tests/probes/stack-navigation.bund`, which exercises the
  right-hand spellings the oracle can run and states why the left-hand ones are
  absent.

## F70 — `move` to a stack that does not exist yet never terminates

`move` drains the current stack into a named one
(`reference/rust_multistack/src/stdlib/stack_move.rs:24-30` calling
`move_from_current`), and the drain is a loop that pulls from *current* and
pushes to *name_to*:

```rust reference/rust_multistack/src/ts_move.rs:17
    pub fn move_from_current(&mut self, name_to: String) -> &mut TS {
```

The loop is correct only while `current` and `name_to` are different stacks.
They stop being different on the first push, because three innocuous facts
compose:

- `push_to_stack` calls `ensure_stack` before pushing
  (`reference/rust_multistack/src/ts_push.rs:46`);
- `ensure_stack` creates the stack through `add_named_stack`, which appends it
  with `self.stacks.push_back(...)`
  (`reference/rust_multistack/src/ts_add.rs:14`);
- **the current stack is the back of that deque** —
  `current_stack_name` is `self.ensure().stacks.back().cloned()`
  (`reference/rust_multistack/src/ts_current.rs:7`).

So pushing to a stack that does not exist yet **makes it current**. The next
`pull()` takes back the value just pushed, pushes it again, and the loop never
reaches an empty stack.

Confirmed against the oracle, and the mechanism confirmed by its own
workaround:

- `1 2 3 :box move` — **hangs**. Killed at 8 seconds; no output, no error.
- `:box ensure_stack @main 1 2 3 :box move` — exits 0 and prints normally,
  because `ensure_stack` inside the push then finds `box` already present and
  does not re-append it.

No corpus program calls `move`, which is presumably why this has never been
hit.

- Found by: investigating Q21 — Bund2's `move` family diverged from the
  reference, and establishing which side was right meant running the
  reference's version
- Behavioural, and **worse than an error**: a hang cannot be reported, caught
  by `?try`, or distinguished from a long computation. It takes the session
  with it exactly as a panic would, which is D37's argument arriving through a
  loop rather than an abort.
- Disposition: **FIX — Bund2 drains a snapshot.** Bund2 takes the source
  stack's contents once and then pushes, so the destination becoming current
  mid-drain cannot feed the loop. `1 2 3 :box move` terminates and moves three
  values. This is a deviation with no golden to record it against, because the
  reference produces no output to capture — it produces nothing at all.
- Follow-up 2026-09-03: **the shape is now checked, and one more instance was
  found in Bund2.**

  Every drain in `crates/` was audited. All five collect their values before
  pushing any, so none can feed itself. `cargo xtask lint` now enforces that: a
  `while let Some(..) = vm.pull…` loop whose body pushes is reported. Verified
  by reintroducing the hang in `move` and watching the lint fail.

  The instance found was `Stacks::to_stack`'s rotation
  (`crates/bund2-interp/src/lib.rs`), which spun `while current_name() != name`.
  It terminated — a membership test ten lines above guarantees the name is
  present — but that is the same kind of reasoning this defect punishes, so it
  is now bounded by the deque's length. The reference guards its own rotation
  by counting a full circle and failing (`ts_to_current.rs:24-26`); bounding is
  the same guarantee without the error.

  **What is deliberately not fixed: a program that loops forever.**
  `true { } while` is Turing-completeness, not a defect, and the reference
  behaves the same way. `cargo xtask depth` bounds the three axes it can with a
  60-second wall clock. The rule this defect establishes is narrower and
  checkable: *a native's own loop must be bounded by data it has already
  taken.*

## F69 — the `dt` guard in `cast_json_to_value` never fires, and the fallback does not cover the same case

```rust reference/rust_dynamic/src/cast_json_to_value.rs:6
        if ! self.dt == JSON {
```

`!` binds tighter than `==`, so this parses as `(!self.dt) == JSON`: a bitwise
NOT of a `u16` compared against 24. For a genuine JSON value `!24u16` is 65511,
so the guard never fires — and for every other tag it does not fire either.
**Exactly F11's shape**, at a different site.

Unusually for this class, it is redundant twice over, and the reachability is
worth stating because it changes the disposition:

- **Every caller already establishes the tag.** Five construct
  `Value::json(...)` on the line before (`json.rs:27`, `wrap_json.rs:54,79`,
  `conv.rs:681`, `map.rs:14`) and the one Bund word that reaches it guards with
  `is_type(JSON)` first
  (`reference/rust_multistackvm/src/stdlib/json/conversion.rs:37`).
- **The payload match catches the same case anyway.** Its final arm returns
  `This Dynamic type is not JSON: {dt}` (`:99`), which is what the guard was
  written to say.

**But the two tests are not the same test, and that is the actual defect.** The
guard reads the **tag**; the fallback reads the **payload arm**. A value whose
`dt` is not JSON while its payload is still `Val::Json` passes both and gets
converted, where a working guard would refuse it. That pair is constructible in
principle — the reference assigns `dt` directly elsewhere, as
`make_bund_object` does with `res.dt = OBJECT`
(`reference/rust_multistackvm/src/stdlib/bund_object.rs:35`) — which is the
tag/payload split RFC-0001 makes an explicit axis.

Nothing reachable from Bund produces such a value today, so no golden covers
it and the defect is latent rather than observable.

- Found by: implementing `json.to_value` and reading the callee, after the
  caller had already been read
- Behavioural, latent. Disposition: **Bund2 writes the guard as intended.**
  `crates/bund2-stdlib/src/json.rs` checks `dt() != JSON` and then matches the
  payload through `as_json`, so a mismatched pair is refused by the first test
  and a JSON-tagged value with a foreign payload is an `Error::internal` by the
  second. There is no observable deviation, because no reachable value has a
  mismatched pair — this is the case where writing the correct guard costs
  nothing and reproducing the broken one would buy nothing.

## F12 — `Ord::cmp` disagrees with `PartialOrd::partial_cmp` for floats
`lt` handles `Val::F64` (`reference/rust_dynamic/src/ord.rs:19-21`) — an
earlier version of this entry attributed those lines to `partial_cmp`, which
is `:6-8` and delegates to `cmp`, so it cannot disagree with `cmp` at all. The
disagreement is between `cmp` and the four individually overridden
comparisons, `lt`, `le`, `gt` and `ge`
(`reference/rust_dynamic/src/ord.rs:9,48,87,126`), none of which reads an id.
Those four are the reachable path;
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
- Behavioural. Disposition: **FIX — declare the probed arity, and accept the
  changed error text.** `StackEffect` (RFC-0002) carries the arity the word
  actually consumes, so the guard fires before the first pull and these
  fourteen report "Stack is too shallow for inline <word>()" where the
  reference reports `NO DATA #2`.

  Option A — preserve the guard, declare 1 for a word that consumes 2 — was
  rejected because the declared guard **is already not the contract**: this
  entry's own second consequence says RFC-0004 must take the probed column
  wherever the two disagree. A static arity that lies does not stay cosmetic.
  RFC-0004 infers effects from it and RFC-0005 orders JIT guards by it, so
  preserving the wrong number propagates the defect into two later RFCs in
  order to keep an error string on a path that fails either way.

  **What changes is larger than an earlier version of this disposition said,
  and it is not only the message.**

  *The residual stack changes.* `pair` guards `< 1`, pulls `x`
  (`reference/rust_multistackvm/src/stdlib/artefacts.rs:10`), then fails on
  the second pull (`:19`) — so `1 pair` errors today with an **empty** stack,
  where a guard at 2 fails before pulling and leaves the value. That is
  observable, because the error path prints the stack:
  `print_error` calls `debug_display_stack`
  (`reference/Bund/src/stdlib/helpers/print_error.rs:126-131`), reached from
  `reference/Bund/src/stdlib/helpers/run_snippet.rs:88`, and
  `tests/golden/probes/execute-arm-not-executable.golden` captures that block.

  *The replacement message is not uniform.* It is `"Stack is too shallow for
  inline pair()"` for `pair`, but `complex` bails with the **same** string
  (`reference/rust_multistackvm/src/stdlib/artefacts.rs:27`) — an unrecorded
  copy-paste, now F40 — and the ten string words use
  `"Stack is too shallow for inline {}"` with a prefix and no parentheses
  (`reference/Bund/src/stdlib/functions/string/distance.rs:25`,
  `reference/Bund/src/stdlib/functions/string/regex.rs:15`). So "the message
  becomes the too-shallow one" is three different messages.

  None of the fourteen is reached by a corpus program, so no golden covers
  them and `conform` cannot move. But the deviation to record in RFC-0002 is
  *residual stack and message*, not message alone.

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
- Dead code. Disposition: **OMIT the table.** Bund2 has no fourth namespace:
  the slot table absorbs three tiers, not four, which is this entry's own
  consequence for RFC-0002. Porting a table because it exists would add a name
  space the language does not have.

  The names split cleanly, and neither half needs a new decision:

  - Registered **both** ways — `drop`, `dup` and the rest of the duplicating
    pairs — are unaffected. The inline registration is what makes them
    callable, and it is preserved.
  - Registered **only** here — `dup_in`, `from_workbench`, `push_to`,
    `stacks_left` — are exactly the dead words, and **D29 has already ruled**:
    `stacks_left` is revived, the other three are omitted.

  So omitting the table costs no reachable behaviour, and the one name that
  needed reviving is revived by decision rather than by porting the table that
  hid it.

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
(`reference/rust_multistackvm/src/stdlib/execute.rs:30`). The first draft
tried to demonstrate that with `1 2 "$drop" ptr !` and demonstrated nothing:
`drop` has no alias, so both the `$` and plain spellings converge on the same
inline word. See F26 for what a discriminating pair looks like, and for why
the `$` arm is not the bypass its comment claims.

Consequence for RFC-0002: the dispatch contract has one resolution order to
specify, not two, and porting this cluster would import a second.
- `reference/rust_multistackvm/src/multistackvm_apply_in.rs`,
  `multistackvm_lambda_eval_in.rs`, `multistackvm_call.rs:12`
- Dead code. Disposition: **OMIT the cluster**, in three parts, because
  "dead code, ignore it" is not sufficient for a path that encodes a different
  contract.

  1. **Not ported.** Bund2 specifies one resolution order, because the
     reference has one that runs: `VM::call` -> `apply`
     (`reference/rust_multistackvm/src/multistackvm_call.rs:8`), which is what
     `execute` reaches
     (`reference/rust_multistackvm/src/stdlib/execute.rs:30`). `call_in` ->
     `apply_in` -> `lambda_eval_in` -> `apply_in` is closed, and nothing
     outside the three calls any of them.

  2. **The divergences are not preserved, and cannot be observed.** No
     `$`-prefix arm, and an `autoadd` that pushes the name whole rather than
     appending it to the value beneath. This is a deviation of the safest
     available kind — nothing can call the code — but it is a deviation, which
     is why this entry cannot simply wave the cluster away. `autoadd` has
     **three live branches**, in `apply`; `apply_in`'s three are unreachable.

  3. **The forward constraint, which is the part with teeth.** When a later
     RFC needs per-stack dispatch — RFC-0007's actor model is the likely one —
     it is built on the single resolution order with the stack as a
     **parameter**, not by reviving a second dispatcher. The reference's own
     attempt at a second one drifted into disagreeing with the live path about
     `$`. That is what a parallel code path costs, and this entry is the
     evidence for the rule.

  Porting it as a second `WordEntry` path buys nothing: it cannot be called,
  and it would import the `$` disagreement into a design whose point is that
  one name reaches one slot by one order.

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
- Behavioural, and a wrong source comment. Disposition: **PRESERVE the
  behaviour; the comment is wrong, not the code.** `$name` skips the lambda
  check and does **not** skip alias resolution, because `call_internal_word`
  strips the sigil and calls `self.i`
  (`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`),
  and `i` resolves aliases
  (`reference/rust_multistackvm/src/multistackvm_inline.rs:71`). The source
  comment at `reference/rust_multistackvm/src/multistackvm_apply.rs:30-31`
  claims both are skipped; only the first is.

  Bund2 reproduces it exactly, including the surprise, because it is the only
  way to reach a native that a lambda has shadowed and programs can depend on
  that — confirmed by probe, `println` runs a lambda while `$println` runs the
  native.

  **One consequence RFC-0002 must carry**: because `$name` enters at `i` it
  resolves *one* alias link where a plain name resolves two, so the two
  spellings diverge on a chain two deep. Oracle: with `a2 -> b2 -> println`,
  `a2` succeeds and `$a2` fails with `Inline b2 not registered`. RFC-0002
  resolves to a fixed point, which is a deviation at two links rather than
  three.

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

No read path exists, and the cause is in **two layers**, not one.

At the **word** layer, `stdlib_value_get` casts the key to a string
(`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:54`) *before*
pulling the container (`:60`), so it can never observe that the container is a
valuemap. `set` pulls all three operands first and branches on the container's
type (`:16-19`), passing the key through as a `Value`. `?key` has the same
stringification (`:97`). **No change confined to `rust_dynamic` can fix this** —
it is a word-level defect, which is why D30's fix is a word change.

At the **value** layer:

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
- Disposition: **fixed, per D30.** Bund2's `get` word pulls both operands,
  branches on the container's type, and looks up by the key `Value` for a
  valuemap — the same shape `set` already has
  (`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:16-19`). D30
  also settles F30, without which the branch alone would still miss.
  `?key` is not covered by the decision; carried as Q19.

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
- Disposition: **fixed, per D30 — hash by content, mirroring equality.**
  Content-compared kinds hash their content; identity-compared kinds hash
  their identity. That satisfies the `Hash`/`Eq` contract and makes a
  scalar-keyed lookup succeed. Composite keys stay identity-keyed, because
  `eq` for a list is identity — a stated limit, not an oversight.

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
- **Duplicate of F1**, which recorded the same defect from the source before
  this one confirmed it by probe. Both are kept, per the append-only rule;
  F1 carries the disposition and this entry carries the oracle evidence.
- Affects: `unregister` for lambdas; `stdlib_lambda_unregister` is dead code
- Disposition: Bund2 fixes it — the two need distinct names, or one word that
  dispatches on what the name is bound to. No corpus program calls
  `unregister`, so no golden covers it. This is also why RFC-0002's registry
  builder must **not** silently dedupe duplicate registrations: replaying them
  in order is what reproduces the reference, and deduping would change which
  handler wins. Record the divergence in RFC-0002.
- Update 2026-09-02: **"no corpus program calls `unregister`" is wrong, and a
  golden does cover it.**
  `reference/Bund/examples/bund_dynamic_demos/resolving_lambda.bund:36` calls
  `:HelloWorld unregister`, and its next four lines are written specifically to
  observe the result — the source comment reads "After unregister, function
  ?lambda must return FALSE, but by calling ```not``` we are making it TRUE".

  The golden shows the program's own expectation failing. The oracle prints no
  confirmation line, because `?lambda` still answers `true` after the
  `unregister` that this defect says never happens. Confirmed directly:
  registering `HW`, calling `:HW unregister`, then `:HW ?lambda` prints `true`
  on the oracle and `false` on Bund2.

  So the disposition stands and its consequence is now recorded:
  `resolving_lambda.golden` is an **approved deviation** under this F-number.
  Bund2 prints the confirmation line the program was written to print, and the
  oracle does not.

  Found by implementing `not`, which is the word that made the divergence
  visible — before it, the golden failed on an unregistered word and the
  disagreement was hidden behind that.

## F33 — `PartialEq` is asymmetric across int/float, and `impl Eq` asserts otherwise
Comparing an integer to a float truncates; comparing a float to an integer
widens:

- `Val::I64` against `Val::F64` — `*i_val_self == *f_val_other as i64`
  (`reference/rust_dynamic/src/eq.rs:13`)
- `Val::F64` against `Val::I64` — `*f_val_self == *i_val_other as f64`
  (`reference/rust_dynamic/src/eq.rs:24`)

So `42 == 42.5` truncates `42.5` to `42` and answers **true**, while
`42.5 == 42` widens `42` to `42.0` and answers **false**. Confirmed against
the oracle: `42 42.5 ==` prints `false` and `42.5 42 ==` prints `true` — the
operands reach `stdlib_logic_compare` in stack order, so the printed pair is
the reverse of the written one, and the asymmetry is visible either way.

`impl Eq for Value` (`reference/rust_dynamic/src/eq.rs:59-62`) is an empty
impl asserting the reflexive-symmetric-transitive contract that `eq.rs:13`
and `:24` break.

This is reachable: the `==` word accepts `INTEGER | FLOAT | CINTEGER | CFLOAT
| TIME` on both sides (`reference/rust_multistackvm/src/stdlib/logic/logic_compare_fun.rs:17-20`),
so the mixed pair is exactly a case it forwards to `PartialEq`.

- Found by: RFC-0001 review 2
- Affects: **D30 directly.** A content hash must decide whether `42` and
  `42.5` share a bucket, and no bucket assignment can be consistent with an
  asymmetric equality. Whichever direction Bund2 picks is a deviation.
- Disposition: **fixed, per D30's amendment — exact numeric comparison.** An
  integer and a float are equal when they denote the same mathematical value.
  Neither truncation nor widening would do: both are non-transitive, the first
  at `42/42.5/42.9` and the second above 2^53, and both are pinned in
  `tests/golden/probes/eq-asymmetry.golden`. Exactness is the only reading of
  "bidirectional" that yields a valid equivalence relation, which is what a
  content hash requires. Q20 closed.

## F34 — mutating a container resets its header, discarding `attr`, `curr` and `tags`
`set` on a map rebuilds through `Value::from_dict` and then restores only the
tag (`reference/rust_dynamic/src/set.rs:21-23`). `from_dict` is a constructor,
so it writes `attr: Vec::new()`, `curr: -1` and `tags: HashMap::new()`
(`reference/rust_dynamic/src/create_map.rs:38-40`). `from_list`
(`reference/rust_dynamic/src/create_list.rs:19-27`) and `from_valuemap`
(`reference/rust_dynamic/src/create_map.rs:43`) do the same.

So mutation does not preserve the header — it **resets** it. Confirmed against
the oracle: `dict 99 attribute "b" 2 set` renders `attr: []`, while the
control `1 2 attribute` renders `attr` populated.

- Found by: RFC-0001 review 2
- Affects: **RFC-0001's value semantics.** `Rc::make_mut` *copies* the header,
  so clone-on-write preserves `attr`/`curr`/`tags` where the reference
  discards them. That is a divergence on every container mutation, and it was
  absent from the preservation table.
- Disposition: Bund2 reproduces the reset — a container mutation clears
  `attr`, `curr` and `tags` and re-applies the stack tag, matching the
  constructors. Record in RFC-0001.

## F35 — `push` on a `RESULT` silently yields a `LIST`
`Value::push` handles `LIST | RESULT` in one arm
(`reference/rust_dynamic/src/push.rs:37`) and returns `Value::from_list(data)`
(`:48`), which sets `dt: LIST` (`reference/rust_dynamic/src/create_list.rs:23`).

Pushing to a `RESULT` therefore converts it to a `LIST`. The `dt` is the whole
distinction between the two — they share the `Val::List` payload — so the
value silently changes type.

- Found by: RFC-0001 review 2
- Disposition: **fix.** Bund2 restores the `dt` after a push, the way `set`
  already does for maps (`reference/rust_dynamic/src/set.rs:22` restores
  `raw_value.dt = self.dt`), so a `RESULT` stays a `RESULT`.
  The asymmetry between `set` restoring the tag and `push` not is the defect.
  No corpus program pushes to a `RESULT`, so no golden covers it.

## F36 — four `dt` constants are readable but have no writer
`ASSOCIATION` (`reference/rust_dynamic/src/types.rs:51`) appears in eight
reader arms — `get` (`reference/rust_dynamic/src/get.rs:7`), `has_key`
(`reference/rust_dynamic/src/has_key.rs:7,26`), `set`
(`reference/rust_dynamic/src/set.rs:14,74`), `reduce`
(`reference/rust_dynamic/src/reduce.rs:19`) and `conv`
(`reference/rust_dynamic/src/conv.rs:518,595,729`).

No constructor writes it. Nothing in `rust_dynamic` assigns `dt: ASSOCIATION`
or `dt = ASSOCIATION`, so every one of those arms is unreachable.

**It is not alone.** Scanning all 42 `dt` constants for a write — either the
`dt:` field initialiser or a post-construction `dt = ` assignment, the second
of which is how `PAIR` and `MESSAGE` are set
(`reference/rust_dynamic/src/create.rs:166,186`) — four have neither:

    LITERAL   LARGE_FLOAT   ASSOCIATION   TOKEN

`TOKEN` pairs with `Val::Token`, which has no constructor either — F38.

- Found by: RFC-0001 review 2; extended to all four by review 3
- Disposition: Bund2 omits all four unless a writer is found. A `dt` constant
  with no values carries no behaviour. Record the omission in RFC-0001's tag
  table, and note that it makes the tag count 38 live constants of 42
  declared.

## F37 — `stdlib/classes/registry.rs` is source that is never compiled
`reference/rust_multistackvm/src/stdlib/classes/registry.rs` defines
`stdlib_lambda_register` and `stdlib_lambda_unregister` and registers both as
words (`:61-62`). Nothing declares the module: `stdlib/mod.rs` lists 20-odd
`pub mod` entries and `classes` is not among them
(`reference/rust_multistackvm/src/stdlib/mod.rs:3-27`), and the only `classes`
in the crate root is `multistackvm_classes`
(`reference/rust_multistackvm/src/lib.rs:9`), a different file. So the file is
never compiled and its registrations never run.

**No count is affected.** Its two names, `register` and `unregister`, are both
also registered by `stdlib/lambdas/registry.rs:88-90`, which does compile, so
the file contributes no unique name and the 617 total is the same with or
without it.

What it does affect is **attribution**. `cargo xtask corpus` reports the
implementing site as "last wins" over its own path-ordered scan, and the real
order is the explicit call sequence in
`reference/rust_multistackvm/src/stdlib/mod.rs:29-51` — a different order,
which happens to agree here because both live registrations for `unregister`
are in one file and source order settles them (F32). The agreement is
accidental, not constructed.

- Found by: RFC-0002 review 2
- Disposition: no tool change. Registration is last-write-wins, so the outcome
  is determined by order rather than by which files exist, and no observed
  attribution is currently wrong. Recorded so that a future disagreement
  between path order and init order is diagnosed rather than rediscovered.
  Bund2 does not carry the file.

## F38 — `Val::Token` has no constructor, so one payload arm is unreachable
`Val::Token(String)` is declared (`reference/rust_dynamic/src/types.rs:69`)
and appears nowhere else in `rust_dynamic`: no constructor writes it, no
conversion produces it, and `TOKEN` — the matching `dt` constant
(`reference/rust_dynamic/src/types.rs:56`) — is never assigned either.

So nineteen of the twenty payload arms can be produced by a running Bund and
one cannot.

- Found by: RFC-0001 review 3
- Affects: RFC-0001's criterion 7, which asked for a byte-identical wire
  format "for one value of each of the 20 payload kinds, captured from the
  oracle". No oracle run can produce a `Token`, so the criterion was
  unsatisfiable as written. It now asks for nineteen.
- Disposition: Bund2 omits the arm unless a writer is found. An arm with no
  constructor carries no behaviour.


## F39 — a caller search that stops at four crates finds phantom dead code
`Value::exit` was reported as having zero callers by RFC-0001's fourth review
and repeated by the RFC, on the strength of a search across
`reference/rust_dynamic/src`, `reference/rust_multistackvm/src`,
`reference/rust_multistack/src` and `reference/Bund/src`.

It has one, in the fifth crate: the parser's `EOI` handler
(`reference/bund_language_parser/src/vm/eoi.rs:8`). So **every parsed program
ends with an `EXIT` value**, three evaluation loops break on it
(`reference/Bund/src/stdlib/helpers/eval.rs:16`,
`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:35`,
`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:60`), and the
arm is one of the most-executed in the language rather than dead.

Corrected by the repository owner.

This is not a defect in the reference. It is a defect in **method**, and it is
recorded because the same four-crate habit produced F19, F25, F36 and F38 —
all genuine, but all established by the same kind of search. `Value::operator`
and `Value::embedding` were re-checked across all six crates and do have zero
callers; `Val::Token` likewise.

- Found by: the repository owner, correcting RFC-0001 review 4
- Affects: any "no callers" claim in the registers
- Disposition: **method, not behaviour.** A reachability claim must name the
  crates it searched, and the search must cover all six —
  `Bund`, `bundcore`, `bund_language_parser`, `rust_dynamic`,
  `rust_multistack`, `rust_multistackvm`. `cargo xtask corpus` scans three by
  design, because it is looking for *word registrations* and those live in
  three; a claim about a *constructor* has no such excuse.


## F40 — guard messages name the wrong word
`stdlib_complex_inline` guards the stack and bails with
`"Stack is too shallow for inline pair()"`
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:27`) — the message
belongs to `stdlib_pair_inline` directly above it
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:8`), and was copied with
the guard.

So a program that under-feeds `complex` is told that `pair` failed. Both are
in F18's fourteen, so both already report the wrong *kind* of error; this one
additionally reports the wrong *word*.

**It is not the only one.** `stdlib_object_value_wrap` — the word `wrap`
(`reference/Bund/src/stdlib/functions/oop/value_class.rs:168`) — guards the
stack and bails with `"Stack is too shallow for inline UNWRAP"`
(`:85`). Confirmed against the oracle: `1 wrap` reports UNWRAP.

So at least two guards name a neighbour rather than themselves, both copied
along with the guard they sit under.

- Found by: RFC-0002 review 4, while checking what F18's fix replaces;
  extended while enumerating constructible payload arms for RFC-0001
- Disposition: **FIX.** The message names the word that failed. It is covered
  by F18's disposition — that fix rewrites these guards anyway — and is
  recorded separately because it is a distinct defect that would survive a
  fix addressing only the arity.

## F41 — a scripted edit whose target does not match silently does nothing
RFC-0001's fourth revision reported that `Payload` had been defined, that
`curr` had been changed from a `Cell` to a plain field, and that F15's
`BTreeMap` choice and F36's omissions had been recorded. **None of it landed.**
The edit was a Python `str.replace` whose `old` text did not match the file —
it named the identity field `id` where the document says `identity`, and used
different column alignment — so the replace returned the string unchanged, the
script exited 0, and the pass was reported as complete.

Two preservation rows and the review history then asserted work that was not
in the file, and both read as internally consistent, so `cargo xtask lint`
could not see it: the rows and the history agreed with each other and only
disagreed with the code block.

This is the same class as the duplicated `## Design` section in RFC-0002 —
a scripted edit that did not do what the script said — and the third instance
after that one and F18's disposition landing on F1.

- Found by: RFC-0001 review 5
- Disposition: **method, not behaviour.** Every scripted replacement asserts
  its target matched before writing. A `replace` without an assert is the
  failure mode, because it cannot fail loudly.

  `cargo xtask lint` gains the check that would have caught this specific
  shape: a type named in a fenced `rust` block but never introduced there.
  `Payload` was used at `Rc<Payload>` and defined nowhere; `NativeFn` in
  RFC-0002 was the same defect, found by that RFC's fourth review.

## F42 — `set` on a `LIST` discards the container and the `dt`
`Value::set` on a list or result returns `Value::from_list(vec![value])`
(`reference/rust_dynamic/src/set.rs:9`) — a **new one-element list holding
only the value being set**. The existing elements are dropped, and so is the
`dt`, so a `RESULT` becomes a `LIST`.

The map arm two lines below restores the tag — `raw_value.dt = self.dt`
(`reference/rust_dynamic/src/set.rs:22`) — so the asymmetry is within one
function.

This is F35's defect in the sibling word: F35 records `push` on a `RESULT`
yielding a `LIST`, and `set` does the same and additionally discards the
container. Neither is reached by a corpus program.

- Found by: RFC-0001 review 5
- Disposition: **FIX**, with F35. `set` on a list sets an element and
  preserves the `dt`, as the map arm already preserves it. No golden covers
  it, so `conform` cannot move.

## F43 — the golden capture deadlocks on any program over the pipe buffer
`run_once` in `xtask/src/golden/mod.rs` spawned the oracle with
`Stdio::piped()` on both streams and then waited for exit **before** reading
either. A child that fills the OS pipe buffer — 64 KiB here — blocks on
`write` while the parent blocks on `try_wait`, and neither moves.

The symptom is the misleading part: the parent's own 60-second timeout fires
and the program is reported **"timed out"**, which reads as *the oracle hung*
rather than *we never emptied the pipe*. It cost this session an hour of
diagnosing a non-existent non-determinism, first in `time.now` and then in the
nanosecond stamps `metrics` renders — both plausible, both wrong.

Found by `tests/probes/dt-reachable.bund`, which produces **69,747 bytes**
against a 65,536-byte buffer.

**Impact on the recorded numbers: none, and it was close.** Of the 18 programs
`tests/golden/UNSTABLE.txt` lists as unreproducible, 12 produce well under
64 KiB and were genuinely non-deterministic — F14, F15 and F17 as recorded.
The other 6 are the `bund/image` family at **1.2 MB** each, which would have
deadlocked; they are out of scope under D28 for an unrelated reason, so their
exclusion is over-determined and no conformance figure moves. The next
large-output in-scope program would have been silently mis-attributed.

- Found by: enumerating reachable `dt` values for RFC-0001
- Disposition: **FIXED.** Both pipes are drained on their own threads while
  the child runs. This is a defect in Bund2's tooling rather than in the
  reference, recorded here beside F39 and F41 because the register is where
  method defects live and because the failure mode — a wrong diagnosis that
  looks like a finding — is the same one those two record.

## F44 — measuring an unfiltered program wrote into the pinned submodule
While sizing the 18 unreproducible programs for F43, every one was run
directly against the oracle — including `image_blur_and_save.bund` and
`image_upscale_and_save.bund`, which write their output **into
`reference/Bund/examples/image/`**. Two tracked PNG/JPG files were rewritten,
and `git status` inside the submodule stopped being empty.

Restored with `git checkout --`, and no citation was affected, since the files
are images and nothing cites them.

The lesson is the one the hermetic filter already encodes and the measurement
bypassed: `cargo xtask golden` runs only programs that passed the effect
audit, and running the oracle by hand skips that gate. Both files gave **0
bytes of stdout** in the size measurement, which is what a filesystem writer
looks like from outside — the output went somewhere else.

- Found by: `git status` on the submodule, after the F43 measurement
- Disposition: **method.** An ad-hoc oracle run over a program not in
  `tests/golden/HERMETIC.txt` must be treated as an effectful action, and the
  submodule checked afterwards. CLAUDE.md already requires the submodule stay
  clean; this records how it stopped being so, since the failure was silent —
  the programs succeeded and nothing reported anything.

## F45 — the bincode wire format is not byte-deterministic for maps
`Val::Map` is a `HashMap<String, Value>` and `Val::ValueMap` a
`HashMap<Value, Value>` (`reference/rust_dynamic/src/types.rs:78,79`). bincode
serialises a map by iterating it, and Rust's `HashMap` uses a per-process
random seed, so **two runs of the reference serialising the same map emit
different bytes**.

Confirmed directly: a `HashMap` of eight string keys iterated in two runs of
one binary gives `["b","e","d","g","c","a","h","f"]` and then
`["a","e","c","f","d","h","b","g"]`.

This is **F15's defect in a second surface**. F15 records `HashMap` ordering
in the `Debug` rendering, which cost 15 of the 18 unreproducible programs;
this is the same non-determinism reaching the serialised form, where no
normaliser can reach it.

Two consequences:

- **RFC-0001's criterion D4 is unachievable as written** for any value
  containing a map. "Byte-identical to the reference" presumes the reference
  has *one* answer to compare against, and for maps it does not. D4 now scopes
  itself to the map-free arms and states why.
- **Byte-comparing two world files is meaningless.** D27 and RFC-0002's
  criterion 7 concern a lambda surviving a save/reload; that round trip is
  well-defined, but a byte comparison of the stored blob is not.

- Found by: writing the wire format down for `bund2-value`
- Disposition: **PRESERVE the format, not the bytes.** Bund2 emits the same
  encoding — same field order, same variant indices — and a map's entry order
  is whatever its container yields, exactly as the reference's is. What Bund2
  guarantees, and what a criterion can check, is that the reference can
  *decode* what Bund2 writes and Bund2 can decode what the reference writes.
  Byte-equality is checkable only for map-free values, and D4 says so.

## F46 — a lazy stamp is observable through the width of the box drawn round it

`debug.display_stack` draws its rows with `comfy_table`, which sizes a column
to the widest content in it
(`reference/Bund/src/stdlib/functions/debug_fun/debug_display_stack.rs:14-27`).
The golden capture then normalises the *text* of a stamp to `<stamp>`, but the
box was already drawn: the border length is fixed at capture time by the stamp
the reference actually printed.

D2 makes Bund2's stamp lazy, taken at first need. Nothing before this needed
one, so a value reached the box unmaterialised and rendered `stamp: 0.0` —
three characters where the reference prints `1787954810882.0`, fifteen. The
rows compared equal after normalisation and the borders did not:

    oracle  ╭──────…──────╮   150 columns
    bund2   ╭──────…──╮       138 columns

Twelve columns, on every non-empty box, in a value whose normalised text was
already identical. Identity escapes this only by accident: `format_id(0)` is a
21-character placeholder, exactly a nanoid's width, so its laziness cannot be
seen.

- Found by: diffing `debug.display_stack` against the oracle on a
  three-value stack, after the unit test on the frame alone passed
- Disposition: **Bund2 bug, fixed.** `render` materialises the stamp. This is
  D2's own rule — first *need*, and printing is a need — and it inherits D7's
  accepted consequence that stamps order by observation, not construction. A
  scalar has nowhere to keep the sample and `render` cannot promote through
  `&self`, so it samples without caching.

The general form is worth more than the instance: **laziness that changes the
output is not laziness.** Anything deferred must render at the width the
reference renders it, because a golden captures a layout and not only a value.
A unit test on the frame cannot catch this — only the oracle can, which is
what the oracle is for.

## F47 — every ordering comparison answers true when the operand kinds differ

`PartialOrd for Value` overrides `lt`, `le`, `gt` and `ge` individually, and
each override matches the receiver's payload arm against the operand's. When
they differ, every one of them falls to `_ => return true`
(`reference/rust_dynamic/src/ord.rs:16,24,55,63,94,102,133,141`).

So an integer against a float answers **true to all four operators at once**,
including the two that cannot both hold. Confirmed against the oracle:

    1 2.0 <     true
    1 2.0 >     true
    1 2.0 <=    true
    1 2.0 >=    true

The sane implementation sitting next to it never runs. `partial_cmp` delegates
to `Ord::cmp` (`:6-8`), and `cmp` compares `I64` against `I64` properly
(`:170-177`) — but Rust's `<` calls `PartialOrd::lt`, and the override shadows
it. The comparison words reach the broken path
(`reference/rust_multistackvm/src/stdlib/logic/logic_compare_fun.rs:29-39`).

Note that `cmp` is no better across kinds: it falls back to comparing **ids**
(`:175`), so an int against a float would order by minting order. Neither path
gives a usable answer; only the overrides are reachable.

- Found by: reading `ord.rs` while implementing the comparison words, then
  probing all four operators against the oracle
- Disposition: **PRESERVED, pending D33.** Bund2 reproduces it exactly. It is
  a defect, but fixing it is a deviation, and an unplanned deviation is a
  decision. D30 settled equality across int/float and did not reach ordering.
  D33 carries the question; until it resolves, the reference's answers stand
  and `crates/bund2-stdlib/src/logic.rs` pins all four in a test.

**Dated note, 2026-09-11 — D33 resolved, and this is now a corrected
deviation.** The repository owner took D33's option 2: an integer and a float
order by the mathematical values they denote, so exactly one of `<`, `==`, `>`
holds, and NaN orders against nothing. `numeric_ord` mirrors `numeric_eq`
(`crates/bund2-stdlib/src/logic.rs`), with `exact_int_float_ord` handling the
two casts neither of which is safe alone — `i as f64` is lossy above 2^53 and
`f as i64` saturates. `an_int_and_a_float_order_by_their_mathematical_value`
replaces the test that pinned the reference's four answers. Conformance does
not move: no corpus program orders across kinds (measured — three files use an
ordering operator at all, and every one compares int to int). The disagreement
with the oracle is a deviation with no golden to record it against, which is
F48's gap, the same one D30's two deviations already sit in.

## F48 — `conform` cannot express a deviation the owner already approved

D30 mandates two deviations from the reference and names the goldens each one
breaks: `eq-asymmetry` (F33) and `valuemap-hash-eq` (F29). Both are approved.
Neither can be recorded as approved.

`cargo xtask conform` compares captured bytes and counts equality
(`xtask/src/conform/mod.rs`). A golden Bund2 deliberately disagrees with
fails, permanently, and sits in the failure list looking exactly like a
regression.

CLAUDE.md prescribes `cargo xtask golden --accept <name> --reason <ref>` for
the original-implementation-bug disposition, but that is the wrong instrument
here. `--accept` re-runs the **oracle** and writes what it produced
(`xtask/src/golden/mod.rs`). The oracle has not changed, so the bytes
are identical, the golden is reported unchanged, and Bund2 still fails it.
`--accept` handles a changed *capture*; it has nothing to say about a changed
*Bund2*.

Two consequences:

- **The conformance number understates Bund2 and will keep doing so.** Every
  approved deviation is a permanent subtraction, and the count cannot
  distinguish one from a real regression — which is the one thing the number
  exists to do.
- **The baseline ratchet enforces the wrong direction.** Implementing an
  approved deviation *lowers* the count, so `conform` reports a regression for
  doing what a decision instructed.

- Found by: implementing `==` under D30 and looking for where to record that
  `tests/golden/probes/eq-asymmetry.golden` is now expected to disagree
- Disposition: **FIXED.** Decided by the repository owner. `tests/golden/DEVIATIONS.txt`
  records, per golden, the approving reference and a **hash of Bund2's expected
  output**; `cargo xtask conform --accept-deviation <golden> --reason <ref>`
  writes a row, and `--reason` is mandatory because a deviation without the
  decision that approved it is indistinguishable from a regression someone gave
  up on.

  Three properties were chosen deliberately.

  **A hash, not an exclusion.** An exclusion stops checking; a hash keeps
  checking against the right thing, so an *unintended* change to a deviating
  golden is reported as a **drift** — a regression inside a deviation, which an
  exclusion would have hidden.

  **The denominator does not move.** A deviating golden is still a captured
  golden, so it stays in the total and is reported apart:
  `CONFORMANCE 21/69 (+2 approved deviation(s))`. An earlier version of this
  fix removed them from the denominator, which quietly contradicted the report's
  own "denominator is every captured golden" and made the ratio look better by
  shrinking what it was measured against.

  **`tests/golden/` is untouched.** Nothing regenerates a golden from Bund2's
  output; the oracle's record stays the oracle's record. The register sits
  beside it.

  Recorded so far: `eq-asymmetry` under F33 and `valuemap-hash-eq` under F29 —
  D30's two, which had been failing correctly and indistinguishably since the
  decision was taken. F66's four are not yet recorded, because D36's error
  presentation is still settling and recording a hash of output that is about
  to change would only produce a drift.

## F49 — the grammar accepts digit separators the token handler cannot convert

`digits` admits `_` between digits — `digits = @{ (ASCII_DIGIT | ("_" ~
ASCII_DIGIT))+ }` (`reference/bund_language_parser/bund.pest:49`) — and both
`integer` and `float` are built from it (`:22-23`). The handler then parses the
raw token text with `lexical_core`, which rejects `_`
(`reference/bund_language_parser/src/vm/integer.rs:8-14`, and the same shape in
`float.rs:8-14`).

So `1_000` parses and then fails conversion. Against the oracle it is a hard
error, not a fallback to a name:

    Error parsing token: Error converting INT to VALUE: lexical parse error:
    'invalid digit found' at index 1

- Found by: reading the grammar for RFC-0003, then probing the oracle
- Disposition: **PRESERVE the observable behaviour** — `1_000` is an error in
  this language and a program relying on it cannot exist. Bund2's parser
  should reject it at the same point rather than silently accepting a literal
  the reference refuses. Worth stating in RFC-0003 because the obvious reading
  of the grammar alone says the opposite.

## F50 — a leading-zero integer silently parses as several values

`int = @{ "0" | (ASCII_NONZERO_DIGIT ~ digits?) }`
(`reference/bund_language_parser/bund.pest:48`) cannot match a run beginning
with `0` followed by more digits. The grammar does not fail — it matches `0`,
then matches again, so the literal decomposes.

`007` puts **three** integers on the stack: `I64(0)`, `I64(0)`, `I64(7)`.
Confirmed against the oracle with `debug.display_stack`.

This is worse than F49, which at least errors. A zero-padded number — a date
field, an ID, an octal-looking constant — is accepted and means something
entirely different, with nothing printed to say so.

- Found by: reading the grammar for RFC-0003, then probing the oracle
- Disposition: **PRESERVE.** It follows from the grammar and any program in
  the corpus that contains a zero-padded literal already means the decomposed
  form. Changing it would change those programs. RFC-0003 states it, and
  `bund2 check` (RFC-0004) is the right place to warn.

## F51 — `{}` and `[]` are parse errors

`lambda = { "{" ~ term+ ~ "}" }` and `list = { "[" ~ term+ ~ "]" }`
(`reference/bund_language_parser/bund.pest:31-32`) both require **at least one**
term. There is no empty form.

Confirmed: `{} 1 println` fails at `1:2`, pointing at the `}`.

An empty lambda is constructible at runtime — `conditional_run` defaults a
missing slot to `Value::lambda()`
(`reference/Bund/src/stdlib/functions/conditional/conditional_ifthenelse.rs:17,21,25`)
— so the value exists and only the *literal* is unspellable.

- Found by: reading the grammar for RFC-0003, then probing the oracle
- Disposition: **PRESERVE**, and state it in RFC-0003. A parser that accepts
  `{}` would accept programs the reference rejects, which is the direction
  that silently breaks the oracle relationship.

## F52 — the interpreter loop exists twice, verbatim

**Correction (2026-08-30): there are three copies, and the third is not
identical.**

`Bund::eval` (`reference/bundcore/src/bundcore_eval.rs:7-45`) and
`bund_compile_and_eval` (`reference/Bund/src/stdlib/helpers/eval.rs`) are the
same loop: append `\n`, `bund_parse`, then match each word's `dt` — `NONE`
continue, `EXIT` break, `ERROR` bail, everything else `apply` — down to the
identical error strings.

The debugger holds a **third**
(`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:52-95`): same
append, same `bund_parse`, same four arms, same two error strings — but it
prints each word before applying it (`:74`) and then runs a readline loop after
every word (`:81-95`).

That third copy matters twice over. It falsifies "the two copies are currently
identical", which was this entry's reason for calling the collapse a non-
deviation; and it is the clearest argument for a materialised frame stack in the
whole reference, because a per-word step loop is exactly what one gives for
free. The debugger exists as a duplicated interpreter *because* the interpreter
has no steppable state.

They live in different crates, so a fix to one drifts from the others silently.
The top-level path uses the first (`bc.eval`, via
`reference/Bund/src/stdlib/helpers/run_snippet.rs`), `bund.eval` uses the
second, and `--debugger` uses the third.

- Found by: reading both eval paths for RFC-0003
- Disposition: **Bund2 has one loop, parameterised.** The first two are
  identical, so collapsing them preserves what both do. The third is not: it
  prints and it steps. Collapsing all three means the single evaluator takes a
  per-word observer — nothing for the two silent callers, print-and-step for the
  debugger — which reproduces all three behaviours without three loops. Calling
  this a pure non-deviation, as an earlier version of this entry did, was only
  true while the third copy was unnoticed.

## F53 — `execute.` guards the main stack and then pulls the workbench

`stdlib_execute_from_workbench_inline` tests `vm.stack.current_stack_len() < 1`
(`reference/rust_multistackvm/src/stdlib/execute.rs:117`) — the **main** stack
— before delegating with `StackOps::FromWorkBench`, which pulls from the
**workbench** (`:22`).

So a value waiting on the workbench cannot be executed while the main stack
happens to be empty. Confirmed against the oracle: with a PTR on the workbench
and nothing on the stack, `execute.` returns

    Stack is too shallow for inline execute()

The base function's own guard is correct (`:14-18`), which is why this is only
reachable through the outer wrapper. `bund.eval.` gets the same shape right,
testing `vm.stack.workbench.len()`
(`reference/Bund/src/stdlib/functions/bund/bund_eval.rs:17-21`), so the
convention is clear and this is the outlier.

- Found by: reading `execute.rs` for RFC-0003, then probing the oracle
- Disposition: **Bund2 checks the stack it pulls from — together with F59.**
  An original-implementation bug. It is *not* widening-only, as an earlier
  version of this entry implied: correcting the guard exposes the LIST and MAP
  arms, which have no coherent workbench behaviour. F59 records that and carries
  the joint disposition — the recursion passes `StackOps::FromStack`, so
  `execute.` means "receiver from the workbench, then proceed as `execute`".
  Neither half lands without the other. No corpus program uses `execute.` or
  `!.`, so no golden is at risk.

## F54 — a second, orphaned copy of `execute_object`

`reference/rust_multistackvm/src/stdlib/execute_types/execute_object.rs`
exists but is not declared in `execute_types/mod.rs`, which lists only
`conditional_through` and `execute_conditionals` (`mod.rs:31-33` in a 38-line
file). The reachable copy is
`reference/rust_multistackvm/src/stdlib/bund_execute/execute_object.rs`,
declared at `bund_execute/mod.rs:3` and called from `execute.rs:91`.

**Correction (2026-08-30): it is not a copy of `execute_object`.** The file
declares `pub fn execute_object` (`:8`) whose body is byte-identical to
`execute_conditionals` — it reads the value's `type` slot, looks it up in `CF`,
and calls the handler. So the name says object dispatch and the code does
conditional dispatch.

That is a worse trap than a stale duplicate. A `path:line` citation into this
file resolves, and returns confidently wrong code: a reader grounding a claim
about how OBJECT is executed would be reading the conditional dispatcher. The
live object executor is
`reference/rust_multistackvm/src/stdlib/bund_execute/execute_object.rs`, which
pulls a method name and calls `vm.m` — nothing to do with `CF`.

- Found by: resolving `execute`'s OBJECT arm for RFC-0003; the name/content
  mismatch found on 2026-08-30 while closing F59's scope
- Disposition: **Nothing to port, and nothing to cite.** Recorded so a later
  reader does not ground a claim in it. `cargo xtask cite` cannot catch this —
  the path and line exist, and only the content is wrong for its name.

## F55 — `input*`'s lambda type check is disabled by operator precedence

`if ! lambda_value.type_of() == LAMBDA`
(`reference/Bund/src/stdlib/functions/io/input.rs:91`) does not negate the
comparison. `!` binds to `type_of()`, which returns `u16`, so this is a
**bitwise complement**: `(!dt) == 17`. For that to hold `dt` would have to be
`65518`, which is not a type tag — the guard can never fire, and its message
`INPUT*: #1 must be a LAMBDA` is unreachable.

The effect is only a worse error. A non-lambda passes the guard and reaches
`lambda_eval`, which rejects it with `This is not a lambda`
(`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:28`) — so the
operation still fails, naming the wrong layer.

- Found by: reading `input*` to ground D3's REPL evidence
- Disposition: **Bund2 checks the type it means to check.** The reachable
  behaviour changes only in the text of an error on an already-failing path.
  If a golden pins that text it needs `--accept` under this F-number, and F48
  applies.

## F56 — the `$` alias is unreachable, because `$` is also the sigil

`$` is registered as an alias for `take`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:36`). It can never
fire.

`apply` tests the first character of a call name and routes any `$`-prefixed
name to `call_internal_word` (`reference/rust_multistackvm/src/multistackvm_apply.rs:33-35`)
**before** alias resolution at `:39`. `call_internal_word` strips the sigil
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`), so
a bare `$` becomes the empty name. Confirmed against the oracle — `1 2 $`
returns:

    i() for stack returned: Inline  not registered

Note the doubled space: the name is empty. The two operands are untouched, so
nothing was taken.

This is the same ordering F26 examined from the other side. F26 established
that `$` does *not* bypass alias resolution for `$name`, because
`call_internal_word` calls `i()` and `i()` resolves aliases. The bare `$` is
the case where the sigil consumes the whole name and there is nothing left to
resolve.

- Found by: reading the alias table for RFC-0003, then probing the oracle
- Disposition: **PRESERVE the unreachability, do not port the alias.** Bund2
  registers the other 41 of this table's 42 and not this one, and RFC-0003 says why
  (count corrected from "45" on 2026-08-29: the file holds 42 `register_alias` calls)
  — an alias whose name is a prefix sigil is shadowed by the sigil. Porting it
  would create a word the reference does not have.

## F57 — a failing `context` lambda leaves the VM on the wrong stack

`conditional_ctx::conditional_run` saves the current stack name, switches with
`to_stack(cond_name)`, runs the `pre`, `run` and `post` lambdas, and only then
restores (`reference/Bund/src/stdlib/functions/conditional/conditional_ctx.rs:60-74`).

Each of the three evaluations `bail!`s on error, and every one of those returns
before the restoring `to_stack(prev_stack_name)` at `:74` (corrected from `:71`,
2026-08-29). There is no unwind
protection, so an error inside a context leaves the interpreter on the
context's stack.

At top level this is masked — the error ends the program. It is observable
wherever the error is caught, which `?try` does by design
(`reference/Bund/src/stdlib/functions/conditional/conditional_tryexcept.rs:32-49`):
the `except` lambda then runs against a stack the program did not choose.

- Found by: reading `conditional_ctx` for RFC-0003
- Disposition: **Bund2 restores on both paths.** An original-implementation
  bug. RFC-0003's flat frame loop makes this structural rather than a fix — a
  context is a frame with an exit action, and unwinding runs it. The
  reference's shape cannot express that because the restore is a statement
  after three early returns.

## F58 — `( … )` inside a block hoists out of it

`ctx::process_token` pushes `Value::context()` and each inner term into the
`state` vector and returns a `CALL "endcontext"`
(`reference/bund_language_parser/src/vm/ctx.rs:8-20`). `state` is the vector
`bund_parse` is building (`reference/bund_language_parser/src/lib.rs:16,21-23`).

`lambda::process_token` and `list::process_token` pass that same vector down
while collecting their own terms into a *local* `res`
(`reference/bund_language_parser/src/vm/lambda.rs:11`, `list.rs:11`). So a
`( … )` nested inside `{ … }` or `[ … ]` writes its marker and its inner terms
to the **top-level stream**, and only the `endcontext` call lands in the block.

The block is silently reordered and the top-level stream silently gains
elements. Confirmed against the oracle:

    :F { 7 9 } register        ok
    :F { ( 7 ) 9 } register    REGISTER expecting lambda name to be string

The second fails because the CONTEXT marker and `7` were emitted before the
atom `F`, so `register` finds the wrong values beneath it.

This is F9 with a behavioural half. F9 recorded the parser side channel as a
*representation* problem; nesting makes it an observable one.

- Found by: RFC-0003's first review, then confirmed against the oracle
- Disposition: **DEVIATION, approved — D34 resolves to lower in place.** `( … )`
  becomes a scope in the block that lexically contains it, so the bracket is
  balanced wherever it appears. Top-level `( … )` is unchanged and no golden
  moves. See D34 for the grounding and the measured consequence of the hoist.

## F59 — fixing F53 makes `execute.`'s LIST and MAP arms reachable, and they are wrong for the workbench

F53's fix — guard the stack that is pulled from — is not "widening only", which
is how RFC-0003's first draft classified it.

With the `:117` guard corrected, `execute.` on a LIST reaches
`reference/rust_multistackvm/src/stdlib/execute.rs:40-48`, which pushes each
element onto the **main** stack (`:41`) and then recurses with `op` still
`FromWorkBench` (`:42`) — so the recursion pulls from the workbench, not from
what it just pushed. The MAP arm pulls its key from the **main** stack (`:56`)
whatever `op` says.

Today these paths are unreachable through `execute.` whenever the main stack is
empty, because the wrong guard rejects the call first. The F53 fix removes that
accidental shield and exposes two arms that never had a correct workbench
implementation.

### What the arms actually do, measured

Both are reachable today whenever the main stack is non-empty, and both are
already incoherent. With `dup` and a `LIST(111, 222)` on the workbench and `7`
on the main stack, `execute.` leaves main as `7, 111, 111, 222` and then fails
with `Stack is too shallow for inline EXECUTE.()`. The list's elements were
pushed onto main **as data and never executed**, while the workbench's `dup` was
executed in their place, once per element, until the workbench ran dry. The MAP
arm behaves the same way: it resolves the key's value onto main, then looks for
the next thing on the workbench.

### The convention that settles it

The `,`-suffix family already fixes what `op` means. In `get,`/`set,` the
workbench variant pulls the **receiver** from the workbench
(`reference/Bund/src/stdlib/functions/values/getsetinplace.rs:42-45`) but takes
the **key** from the main stack unconditionally (`:54`), returns the receiver
per `op` (`:66-69`), and pushes the **result** to the main stack unconditionally
(`:70`).

So: `op` selects where the receiver lives; operands and results live on the main
stack. Under that rule `execute.`'s MAP arm taking its key from main
(`reference/rust_multistackvm/src/stdlib/execute.rs:56`) is correct, and pushing
the resolved value to main (`:66`) is correct. The defect is only that the
recursion re-passes `op` (`:42`, `:67`) when the thing to execute is by then on
the main stack.

- Found by: RFC-0003's first review; behaviour then probed against the oracle
- Disposition: **Fixed with F53, as one change. The recursion passes
  `StackOps::FromStack`.** Decided by the repository owner.

  `execute.` therefore means: take the receiver from the workbench, then proceed
  exactly as `execute`. That is the sibling convention rather than a new rule,
  and it is the only reading under which F53's guard fix is coherent — which is
  why the two are not separable.

  Rejected: threading the workbench through the push sites as well, which is
  self-consistent but contradicts `get,`/`set,` and would fork the meaning of
  the `.` suffix; and rejecting LIST and MAP outright, which is also a deviation
  since the call mutates the main stack before failing today.

  **Risk is measured at zero.** No corpus program uses `execute.` or its alias
  `!.` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:6`), so no
  golden captures any behaviour of any arm.

  **Scope closed (2026-08-30): two arms, not four.** `execute_class` and
  `execute_object` were read. Neither has the push/pull split, because neither
  consults `op` at all — both take it as `_op`.

  - `execute_class` pushes the value onto the main stack and delegates to
    `stdlib_object_inline`
    (`reference/rust_multistackvm/src/stdlib/bund_execute/execute_class.rs:8-10`).
    No pull, no recursion.
  - `execute_object` guards the main stack, pulls a **method name** from it,
    pushes the value onto it, and dispatches with `vm.m`
    (`reference/rust_multistackvm/src/stdlib/bund_execute/execute_object.rs:8-20`).
    No recursion.

  **Both already behave exactly as this disposition prescribes.** The receiver
  is in hand by the time they are called, and every operand and result they
  touch is on the main stack — which is "op selects the receiver; everything
  else is main", arrived at independently. That is confirmation of the rule
  rather than an exception to it, and it leaves LIST and MAP as the only two
  arms that ever split pushes from pulls.

  Worth noting the contrast with F53: `execute_object` guards
  `current_stack_len() < 1` — the same shape as F53's bug — and here it is
  **correct**, because the method name it needs genuinely comes from the main
  stack. The guard is only wrong when it does not match what is pulled.


## F60 — `endcontext`'s "Context is empty" guard can never fire, and the failure is silent

`stdlib_endcontext` guards with `if vm.stacks_stack.len() < 1 { bail!("Context
is empty") }` (`reference/rust_multistackvm/src/stdlib/ctx.rs:6-8`). That
condition is unreachable in both directions:

- `stacks_stack` is initialised holding `"main"`
  (`reference/rust_multistackvm/src/multistackvm.rs:38-39`), so it starts at
  length 1, not 0.
- `pop_stacks` refuses to go below one — it pops only when `len() > 1` and
  otherwise peeks without popping
  (`reference/rust_multistackvm/src/multistackvm_stacks_stack.rs:10-16`).

So the length is never less than 1 and the guard is dead code, in the same
class as F55's precedence bug and F56's shadowed alias.

What happens instead: `endcontext` with no context open moves the current
stack's top value to the workbench, calls `drop_stack()` on the **current**
stack, and discards `pop_stacks`'s result. Confirmed against the oracle —
`111 222 333` on `main`, then a bare `endcontext`:

    before                   111, 222, 333
    after a bare endcontext  main is empty; 333 is on the workbench

`111` and `222` are gone. No error, no diagnostic.

The root cause is that the invariant the guard wants is not represented.
`stacks_stack` conflates the initial stack with stacks opened by `(`, so
"is a context open?" cannot be asked of it.

- Found by: grounding D34, then probing the oracle
- Disposition: **Bund2 implements the guard the reference wrote and could not
  fire.** Context depth is tracked separately from the base stack — under
  RFC-0003's frame loop a context is a frame with an exit action, so the depth
  is the count of context frames — and `endcontext` fails with the reference's
  own message, `Context is empty`, when that count is zero.

  This is an original-implementation bug and a **narrowing**: a program that
  today silently destroys a stack now gets an error. No golden can capture the
  current behaviour as intended, since it produces no output; if one pins the
  destroyed state it needs `--accept` under this F-number, and F48 applies.

  D34 fixes the *parse* half — a lowered `( … )` is balanced by construction.
  F60 is what remains: `endcontext` is a registered inline
  (`reference/rust_multistackvm/src/stdlib/ctx.rs:31`) and so callable by hand,
  balanced or not.


## F61 — a comment marker abutting a word is swallowed into the word

`element` admits `/` (`reference/bund_language_parser/bund.pest:36`), and `name`
is `element ~ nelement*` (`:28`). `COMMENT` is applied by pest *between* tokens
(`:54`), so it cannot interrupt one.

Therefore `1 2 +// add` lexes `+//` as a **single name**, not as `+` followed by
a comment. Confirmed against the oracle:

    i(+//) for stack returned: Inline +// not registered

and `1` and `2` are still on the stack, unadded.

This is the fourth trap in the same family as F49, F50 and F51: a form that
reads one way and lexes another. It is the least visible of them, because the
error names a word the programmer never wrote and the source looks like an
ordinary trailing comment.

- Found by: RFC-0003's second review, then confirmed against the oracle
- Disposition: **PRESERVE, and state it.** It follows from `element` including
  `/`, which is also what makes `/` and `*` ordinary word characters — the
  language has words spelled `*+` and `*/`. Requiring whitespace before `//`
  would be a narrowing with no decision behind it. RFC-0003 states it and
  `bund2 check` (RFC-0004) is where a warning belongs.

## F62 — `endcontext` is shadowable, so a context can be opened and never closed

`apply` consults the lambda table before the inline table
(`reference/rust_multistackvm/src/multistackvm_apply.rs:46,59`), so a lambda
registered under a stdlib name wins. `endcontext` is an ordinary registered
inline (`reference/rust_multistackvm/src/stdlib/ctx.rs:31`) and has no
protection.

Since `( … )` lowers to a CONTEXT marker plus a `CALL "endcontext"`
(`reference/bund_language_parser/src/vm/ctx.rs:8-20`), shadowing that name
redirects the closing half of every parenthesis in the program. Confirmed
against the oracle:

    :endcontext { "HIJACKED" println } register
    ( 7 )

prints `HIJACKED` and continues. The context was opened — the stack switched —
and never closed. Execution proceeds on the scratch stack with no diagnostic.

This is not introduced by D34: the reference emits the same call, so the same
shadowing works today. D34 makes it uniform rather than positional, which is
why it is worth recording now.

- Found by: RFC-0003's second review, then confirmed against the oracle
- Disposition: **OPEN.** Three shapes are available and none is obviously right:
  lower `( … )` to an opcode the word table cannot intercept; keep the call but
  refuse to register a lambda over the name; or preserve the hazard and document
  it. The first is cleanest for D34 but removes a hook that metaprogramming
  might legitimately want, and this language's whole posture is that the word
  table is open (D16). Not decided here.

## F63 — `:atom` cannot name a word that `name` can spell

`atom` is built from `aelement`, which admits only
`ASCII_ALPHANUMERIC | LETTER | "." | "_"`
(`reference/bund_language_parser/bund.pest:38,26`). `name` is built from
`element`, which additionally admits `-` and sixteen other punctuation
characters (`:36,28`).

So the two character classes disagree, and the atom's is strictly smaller.
`foo-bar` is a legal word name; `:foo-bar` is a **parse error**. Confirmed
against the oracle:

    :my-word { 1 } register    parse error
    foo-bar                    Inline foo-bar not registered

The second reaches dispatch, which proves the name is well-formed.

This matters because `:name { … } register` is *the* documented idiom for
defining a word (`reference/Bund/examples/helloworld_lambda.bund`), and
`register` takes the name as a string
(`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:17-22`). A whole
class of spellable word names — every kebab-case one, and anything using the
other sixteen characters — cannot be written with the standard idiom. They
remain reachable through a quoted string, `"my-word" { 1 } register`, so the
capability exists and only the shorthand is missing.

### The intended semantics, from the repository owner

`:<X>` is an **atom**, and an atom is **interchangeable with a string** — its
content is `X`, and `X` is any run of characters without whitespace. The parser
already agrees at the value level: `atom::process_token` returns
`Value::from_string` (`reference/bund_language_parser/src/vm/atom.rs:7-10`), so
`:foo` and `"foo"` produce the same STRING and nothing downstream can tell them
apart. The atom is a surface form, not a distinct type.

Given that, `aelement` is simply wrong. It admits
`ASCII_ALPHANUMERIC | LETTER | "." | "_"` where the syntax it implements should
admit every non-whitespace character, so the grammar narrows the construct to a
fraction of what it means. `:my-word` failing is not a deliberate restriction on
atoms; it is the grammar failing to express them.

An earlier version of this entry speculated the narrow class "may be
deliberate", and dispositioned PRESERVE on that guess. The guess was wrong.

- Found by: RFC-0003's second review round, then confirmed against the oracle;
  intended semantics supplied by the repository owner
- Disposition: **Bund2 widens `aelement` to any run of non-whitespace
  characters.** An original-implementation bug, in the same family as F49 and
  F61 — a grammar rule that does not describe the construct it names.

  This is a **widening**: `:my-word` and every other spaceless atom start
  parsing, and no program the reference accepts changes meaning, because the
  atoms it accepts today are a strict subset. The trailing-whitespace
  requirement is unchanged, so `:foo}` still fails exactly as `println}` does
  (S1), and atom termination stays consistent with name termination.

  No golden is at risk: a program using `:my-word` cannot exist in the corpus,
  since it would not parse.

## F65 — `conditional_try` is defined and never registered

`stdlib_conditional_try_inline` builds a CONDITIONAL tagged `type: "try"`
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:127-131`). Nothing
registers it: `artefacts.rs`'s `init_stdlib` binds fourteen words (`:134-147`)
and this is not among them, and the name appears nowhere else in any of the six
crates.

The tag it would have produced is also orphaned. `?try` builds
`type: "tryexcept"`
(`reference/Bund/src/stdlib/functions/conditional/conditional_tryexcept.rs:7-11`)
and the conditional table binds `tryexcept`
(`reference/Bund/src/stdlib/functions/conditional/mod.rs:21`); no handler is
bound to `try`. So even if the word were reachable, executing what it pushes
would fail with `EXECUTE:CONDITIONAL conditionals handler does not exist: try`.

Third of its kind, after F55's guard that cannot fire and F56's alias the sigil
shadows.

- Found by: building the conditional table for RFC-0003's S7
- Disposition: **Not ported.** Bund2 binds the eight types that have handlers
  plus `through`, and no word that produces an unhandled tag.

## F64 — a non-`Add` arithmetic operation on two strings silently returns an operand

`string_op_string_string` implements `Add` and falls through to `_ => x` for
everything else (`reference/rust_dynamic/src/math.rs:103-108`), and
`string_op_string_int` does the same (`:110-116`).

So `"a" "b" -` is `"b"` — the left operand, unchanged — rather than an error.
Confirmed against the oracle. `*` and `/` behave the same way on two strings.

That is worse than the type errors either side of it: `numeric_op` rejects an
incompatible operand pair explicitly (`:203`), so a string against a list fails
loudly while a string against a string fails silently and looks like it worked.

- Found by: implementing arithmetic for RFC-0003's word vocabulary, then
  probing the oracle
- Disposition: **PRESERVE, and pin it.** A program relying on it is
  indistinguishable from one with a typo, but changing the answer changes what
  those programs do. `crates/bund2-stdlib/src/math.rs` reproduces it with a
  test that names this F-number, and `bund2 check` (RFC-0004) is where a
  warning belongs.

## F66 — an uncaught error prints the reference's own build paths, so those goldens cannot reproduce

An uncaught error does not terminate the reference with a message. It prints a
two-row `comfy_table` report — an `Error` row and a **`Location`** row — then
`[BUND]  Content of the stack` and the stack and workbench boxes, and exits
**0**.

The `Location` row holds a Rust source path from the machine that built the
oracle:

    │ Location ┆ /Users/gandalf/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/
                 rust_multistackvm-0.38.0/src/stdlib/math/math_op.rs:111:29 │

`easy_error`'s `bail!` also appends a location to the message itself, so a
*caught* error's text carries one too — `tryexcept_demo.golden` records
`… Because I can (src/stdlib/functions/conditional/raise.rs:16:5)` inside a
string the program printed with `format`.

Four goldens are affected: `tryexcept_demo`, `tryexcept_demo_divide_to_0`,
`execute-arm-class` and `execute-arm-not-executable`.

Two separate problems, and only one is about paths:

- **The captured text is machine-specific.** It names a `~/.cargo/registry`
  directory, a crates.io index hash and a crate version. No other machine
  reproduces it, and it is F21 made visible: the path proves the oracle links
  the registry rather than the pinned submodule.
- **The error *presentation* is unimplemented.** Bund2 prints the bare message
  to stderr and exits 1; the reference prints a table and exits 0. That part is
  reproducible and is simply not built, and it is worth separating from the
  paths, because every golden whose program errors depends on it.

- Found by: measuring the remaining conformance failures after the conditional
  registry landed
- Disposition: **Build the presentation; the paths are a deviation.** Bund2
  reproduces the table, the `[BUND]` banner and the exit code, so a program that
  errors produces the reference's shape. The `Location` row cannot carry the
  reference's path and will carry Bund2's own; the `bail!` suffix inside a
  message likewise. Those four goldens then differ only in the path, which is an
  approved deviation under this F-number — and F48 applies, since `conform`
  still has no way to record one.
- Update 2026-09-01: **recorded.** F48's register exists, and three of the four
  are now in `tests/golden/DEVIATIONS.txt` under `F66`. Two corrections to the
  paragraph above, both found by doing it:

  - **It is four goldens, but only three are F66.** `execute-arm-class` is
    recorded under `F16/D23/D25` instead. The reference errors on `<class> !`
    and Bund2 constructs, which is RFC-0009's deliberate deviation; it happens
    to be a golden of an error report, which is what put it on F66's list.
  - **They differ by more than the path.** D36 replaced the Rust location with
    a Bund one and added `Source` and `Stack` rows, so the table shape differs
    too, and D36's own consequences say so. The deviation approved here is
    D36's presentation as a whole, not the path alone.

  Two Bund2 bugs were fixed first, so the recorded hashes pin the reference's
  frame rather than a broken copy of it: the `[BUND]` banner used one space
  where the reference's coloured path emits two
  (`reference/Bund/src/stdlib/helpers/print_error.rs:133,153`), and
  `TextReporter` printed a workbench box after the stack box where the
  reference prints only the stack (`:126-131`).

## F68 — `convert.to_bool` panics the process on an unrecognised string

`stdlib_convert_base` pulls a value and calls `value.conv(BOOL)`
(`reference/rust_multistackvm/src/stdlib/convert/internal.rs:25`). For a STRING
source that reaches `value_string_conversion`'s `BOOL` arm, which is

```rust reference/rust_dynamic/src/conv.rs:207
        BOOL => {
```

and calls `rustils::parse::boolean::string_to_bool` (`:208-210`). That function
has **no error arm** in `conv`'s eyes — it returns a bare `bool`, not a
`Result` — because it panics instead.

Confirmed against the oracle. `"maybe" convert.to_bool` prints a backtrace,
`The application panicked (crashed).  Invalid String: maybe`, names
`rustils-0.1.23/src/parse/boolean.rs:402`, and exits **101**.

That exit code is the tell. Every other Bund failure exits **0** and prints a
`comfy_table` report (F66); this one does not reach `print_error` at all,
because the process is already gone. So a program's stacks, word table and
session state are lost, and the trace names Rust frames and no Bund word —
which is D37's argument, arriving from the reference rather than from Bund2.

The recognised set is narrow. Confirmed accepted: `true`, `TRUE`, `yes`, `1`,
`no`, `0`. Confirmed fatal: `maybe`. `rustils` is pinned only as `0.1.*`
(`reference/rust_dynamic/Cargo.toml:22`), so the exact set is not fixed by the
submodule either.

No corpus program converts a string to a bool, so no golden covers it.

- Found by: differential-testing Bund2's `convert.*` table against the oracle
  while implementing it — 26 conversions agree, and the 27th crashed the oracle
- Behavioural. Disposition: **FIX — Bund2 returns `false`.** D37 forbids Bund2
  from reproducing a panic, and there is no faithful alternative: the reference
  has no error path here to copy, only an abort. `false` is chosen over an
  error because `string_to_bool` is *total* for every input the oracle
  survives, and a word that returns a bool for `"no"` and an error for `"maybe"`
  would invent a failure mode the reference does not have.

  This is a deviation with no golden to record it against, so it is pinned by
  test in `crates/bund2-stdlib/src/convert.rs` instead, with the accepted set
  above as the cases that must keep agreeing.

## F67 — a missing parent class is reported under the child's name

`make_bund_object` walks `.super`, and when a named parent is not registered it
fails with `OBJECT class {} not registered`
(`reference/rust_multistackvm/src/stdlib/bund_object.rs:99`). The name it
interpolates is `name` — the class **being constructed** — while the parent
that is actually missing is `class_name`, bound in the same loop and used
correctly one line above (`:98`).

So constructing `B`, whose `.super` names an unregistered `A`, reports
`OBJECT class B not registered` — naming a class that *is* registered and
saying nothing about `A`. The message sends the reader to the wrong end of the
hierarchy.

- Found by: RFC-0009's first review
- Disposition: **Bund2 names the parent.** An original-implementation bug, and
  a message-only change on a path that already fails. If a golden pins the
  text it needs `--accept-deviation` under this F-number; none does today,
  because no corpus program constructs a class with an unregistered parent.

## F73 — the `.` suffix does not agree with itself about where the answer goes

A `.`-suffixed word is meant to be its plain sibling with the first operand
taken from the workbench. In `bund/string` the *input* half of that is
consistent — operand 1 comes off the workbench and operand 2 off the current
stack, which is why the guard checks both depths
(`reference/Bund/src/stdlib/functions/string/prefix_suffix.rs:23-30,32-39`) —
but the *output* half splits, family by family, with no rule behind it:

| pushes the answer to the **stack** | pushes it to the **workbench** |
|---|---|
| `string.prefix` / `.suffix` (`prefix_suffix.rs:52`) | `string.distance*` (`distance.rs:90-91`) |
| `string.regex` (`regex.rs:47`) | `string.expressionmatch` (`textexpr_match.rs:66-67`) |
| `string.regex.matches` (`regex_matches.rs:57`) | `string.fuzzymatch` (`fuzzy_match.rs:67-70`) |
| `string.regex.split` (`regex_split.rs:50`) | `string.deunicode` (`unicode.rs:39-42`) |
| `string.wildcard` (`wildmatch.rs:44,46`) | `string.wrap.english` (`textwrap.rs:76-79`) |
| `string.grok` (`grok.rs:64`) | |
| `string.tokenize*` (`tokenize.rs:73`) | |

The left column is written `vm.stack.push(res)` unconditionally; the right is
written `match op { FromStack => push, FromWorkBench => push_to_workbench }`.
Both spellings appear in files a few hundred lines apart, and `string.regex.`
and `string.distance.` — neighbours by name and identical in shape — are on
opposite sides.

The consequence for a program is that `take` after a `.` word is right half the
time. A user who learns the idiom from `string.distance.` and applies it to
`string.regex.` pulls whatever was underneath instead.

Confirmed against the oracle for both columns.

- Found by: implementing the group and reading each file's push rather than
  the first one's
- Behavioural. Disposition: **PRESERVE.** Every one of these is a word's
  observable contract, and a program written against the reference depends on
  it. Bund2 reproduces the table exactly; `crates/bund2-stdlib/src/library_string.rs`
  carries it as the module's opening documentation so the next reader does not
  infer the rule that is not there.

## F74 — `string.tokenize.unique` and `.stemmed` answer in a different order every run

Both build their result by inserting tokens into a `HashSet` and then iterating
it (`reference/Bund/src/stdlib/functions/string/tokenize.rs:49-65`). Rust seeds
`HashSet`'s hasher per process, so the LIST that reaches the stack is in a
different order on every invocation.

Five runs of the oracle on `"the cat sat on the cat with a mat"`:

```
[ sat ::  with ::  on ::  mat ::  a ::  cat ::  the :: ]
[ with ::  cat ::  on ::  mat ::  sat ::  the ::  a :: ]
[ a ::  sat ::  mat ::  with ::  the ::  on ::  cat :: ]
[ on ::  mat ::  a ::  sat ::  the ::  with ::  cat :: ]
[ the ::  sat ::  mat ::  with ::  cat ::  on ::  a :: ]
```

`.stemmed` behaves the same way. Sorting does not rescue it: `sort` is not
alphabetical, so tokens that compare equal stay in hash order.

- Found by: running the oracle repeatedly before writing a probe, because the
  source read `HashSet`
- Behavioural. Disposition: **PRESERVE the property, not an order.** Bund2 uses
  a `HashSet` too, so it reproduces "unordered and deduplicated" — which is all
  there is to reproduce, since no particular order is the reference's answer
  either. **No golden may capture these words**, and the probe that exercises
  `string.tokenize.unique` says so in its header. `.stemmed` is unimplemented
  for an unrelated reason: `rnltk` pulls `nalgebra` in behind it.

## F75 — `string.expressionmatch` cannot compile any expression

The word builds its matcher with `srch::Expression::new`
(`reference/Bund/src/stdlib/functions/string/textexpr_match.rs:61`) and bails
when that fails (`:63`). It always fails. Every atom in the crate's own
specification was tried against the oracle:

```
numeric        => STRING.EXPRESSIONMATCH returned error when creates matcher
alpha          => STRING.EXPRESSIONMATCH returned error when creates matcher
alphanumeric   => STRING.EXPRESSIONMATCH returned error when creates matcher
length 5       => STRING.EXPRESSIONMATCH returned error when creates matcher
equals hello   => STRING.EXPRESSIONMATCH returned error when creates matcher
```

`contains "a"` and `starts "h"` fail earlier still, in Bund's own tokenizer.
So the word has two forms, an entry in the word table, an arity in
`docs/arity.md`, and no input for which it returns a value.

`srch` is pinned at `0.0.1` (`reference/Bund/Cargo.toml:126`), which is the
only published version.

- Found by: trying to write a probe for `string.expressionmatch.` and failing
  to find an expression that works, then checking the crate's specification
- Behavioural. Disposition: **PRESERVE.** Bund2 calls the same crate the same
  way and fails on the same inputs, so the two agree. There is nothing to fix
  without changing what the word means, which is a language decision and not
  this register's to take. Recorded so that a future reader does not spend the
  same hour concluding the implementation is broken.

## F76 — a MAP's key order is unreproducible, so any printed dict differs between runs

`rust_dynamic`'s dict iterates a hash map, so the order `println` renders is
seeded per process. This is not specific to any word — three runs of
`dict :a 1 set :b 2 set :c 3 set println` on the oracle:

```
{ a=1 ::  b=2 ::  c=3 :: }
{ b=2 ::  a=1 ::  c=3 :: }
{ b=2 ::  a=1 ::  c=3 :: }
```

`string.grok` shows it too, and there the map is built by the word rather than
the program: five runs gave three different orders for one input.

**No golden is exposed today** — none captures a MAP with more than one key —
but the constraint is permanent: a golden that prints a multi-key MAP cannot be
stable, whatever Bund2 does.

- Found by: `string.grok`'s contents matching the oracle exactly while its key
  order did not, which turned out to be true of every dict
- Behavioural. Disposition: **DEVIATE, and the deviation is an improvement.**
  Bund2's MAP order is stable across runs. Reproducing the instability would
  mean deliberately randomising output, which is not preservation of anything a
  program can depend on — there is no order to preserve. A program that needs a
  particular order must sort, on either implementation. Stated here rather than
  in `DEVIATIONS.txt` because no golden differs.

## F77 — `print.` and `println.` guard the stack and then read the workbench

`stdlib_print_inline_base` tests the depth of the **current stack** before it
looks at which side it was asked for:

```rust reference/rust_multistackvm/src/stdlib/print.rs:7
    if vm.stack.current_stack_len() < 1 {
```

The `StackOps` match that decides where the value comes from is the *next*
statement (`:10-13`). So for `print.` and `println.` the guard is about a stack
the word will not read, and the workbench — which it will — is never checked.

Two observable consequences, both confirmed against the oracle:

- a full workbench with an empty current stack is refused with
  `Stack is too shallow for inline PRINT.`, though the value it was asked to
  print is right there;
- a full current stack with an empty workbench passes the guard and fails one
  line later with `PRINT. returns: NO DATA`, having consumed nothing.

The same shape as F18's fourteen and F72's dead test: a guard that was written
and does not guard the thing. Here it is not dead, just aimed at the wrong
stack.

- Found by: implementing the `.` siblings and reading the guard rather than
  assuming it mirrored `sort.`, which is three files away and does check the
  side it reads (`.../values/sort_lists.rs:19-20`)
- Behavioural. Disposition: **PRESERVE.** Both messages are observable and a
  program can depend on either. Bund2 declares `print.` as `1 -> 0` for the
  same reason — the stack cell really is required — so `bund2 check` reports the
  underflow the reference reports rather than the one it ought to.

## F78 — two `.` words pass `FromStack`, so they ignore the workbench their name promises

`if_fun.rs` has four entry points into one base function. Three pass the
`StackOps` their name implies; the fourth does not:

```rust reference/rust_multistackvm/src/stdlib/logic/if_fun.rs:103
pub fn stdlib_logic_if_false_in_workbench(vm: &mut VM) -> Result<&mut VM, Error> {
```

Its body passes `StackOps::FromStack` (`:104`), where
`stdlib_logic_if_in_workbench` two functions above passes
`StackOps::FromWorkBench` (`:95-96`). Only the error prefix was changed to
`"?FALSE."`. So `if.false.in_workbench` — and its alias `?false.`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:16`) — is `?false`
with a different message: it reads its condition from the current stack, guards
`current_stack_len() < 2`, and never touches the workbench.

Confirmed against the oracle. `false return { … } ?false.` reports
`Stack is too shallow for inline ?FALSE.` from `if_fun.rs:16` — the *FromStack*
guard — with the condition sitting on the workbench where the name said to put
it. `false { … } ?false.` runs.

**The same slip appears once more**, in the loop family:

```rust reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:115
pub fn stdlib_logic_loop_over_workbench(vm: &mut VM) -> Result<&mut VM, Error> {
    stdlib_logic_loop_over_stack_base(vm, StackOps::FromStack, "*LOOP.".to_string())
```

against `stdlib_logic_loop_over_stack` at `:111-113`, which passes the same
thing. `*loop` and `*loop.` are therefore the same word twice. Neither is
implemented in Bund2 yet, so only the `?false.` half is live.

The other six `.` words in this family are correct: `if.in_workbench`,
`ifthenelse.`, `notifthenelse.`, `loop.`, `map.`, `times.` and `while.` all read
the side they name.

- Found by: implementing the nine logic `.` variants and diffing each against
  the oracle — `?false.` was the one case of six that disagreed, and the
  disagreement pointed at the reference rather than at Bund2
- Behavioural. Disposition: **PRESERVE.** `crates/bund2-stdlib/src/control.rs`
  registers `if.false.in_workbench` with `Side::Stack` and `opaque(2)`, and says
  in the doc comment that the `Side::Stack` is the bug being kept. A program
  that calls `?false.` today is a program that puts its condition on the stack,
  because nothing else works.

**Note, 2026-09-11: `*loop.` and `for.` are the same shape.** `*loop.` is the
second of the two this entry's title counts, and was implemented on this date:
`stdlib_logic_loop_over_workbench` passes `StackOps::FromStack`
(`reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:115-117`), so its
lambda comes off the stack. `for.` is a third. Its function reads both the
lambda and the condition with `vm.stack.pull()` and never consults the
workbench (`reference/rust_multistackvm/src/stdlib/logic/for_fun.rs:51-97`), so
it is `for` with other prefixes. Both are preserved
(`crates/bund2-stdlib/src/seq.rs`, `loop_over_wb`;
`crates/bund2-stdlib/src/control.rs`, `for_wb`), and
`tests/probes/stack-loops.bund` records them.

## F79 — `cargo xtask cite` could not see a citation into `crates/`, so five rotted unnoticed

**This is a defect in Bund2's own tooling, not in the reference.** It is
recorded here because it is the mechanism by which grounded claims silently
stop being grounded, and CLAUDE.md makes grounding a requirement rather than a
courtesy.

`citations_in` matched one prefix:

```rust xtask/src/cite/mod.rs:137
    let starts_at = |i: usize| {
```

— and before this fix that closure tested `reference/` alone. A citation into
`crates/` was read past, whatever it said.

The consequence is worse than a gap, because `crates/` decays *faster* than
`reference/`. `reference/` is pinned by SHA and cannot move; `crates/` is
edited every session. **D41 moved `with_tag` by roughly two hundred lines and
invalidated five citations on the day they were written**, while `cargo xtask
cite` reported **zero defects over 1782 citations**:

| citation | pointed at | should have been |
|---|---|---|
| RFC-0005 §S1 | `crates/bund2-interp/src/lib.rs` — a doc comment | `:147` |
| RFC-0005 §S1 | `crates/bund2-value/src/lib.rs` — inside `mod tests` | `:2018` |
| D35 | `crates/bund2-value/src/lib.rs` — a doc comment | `:338` |
| D35 | `:159` — `q: 100.0` | `:210` |
| D35 | `:480` — a blank doc line | `:1114` |

Two of those are in **D35, which is RESOLVED and load-bearing** for RFC-0003
§S3 and RFC-0005 §S3. Two more had been propagated into
`docs/registers/open-questions.md` and a bench doc comment.

An earlier session widened `ROOTS` so the walk *read* Rust files, and wrote a
doc comment implying their citations were now checked. They were scanned and
not extracted — the fix was half of one, and the comment overstated it.

- Found by: an adversarial review of RFC-0005 that opened the citations by hand
- Tooling. Disposition: **FIXED, in three parts.**
  1. `CITE_PREFIXES` now lists `reference/`, `crates/`, `xtask/`, `docs/`,
     `tests/golden/` and `tests/probes/`. A bare `tests/` is deliberately
     absent: in this repository it is ambiguous, meaning `reference/Bund/tests`
     by convention, and listing it produced 14 false findings.
  2. Citations in `docs/research/` are advisory, never fatal. Those documents
     are immutable and superseded through `ERRATA.md`, so a tool that fails the
     build on one demands an edit the project forbids — and the reasoning trail
     is allowed to describe a plan that was not adopted.
  3. The five citations above are corrected, with a note in D35 recording that
     only the numbers moved and the claims were re-verified.

**What was still not checked, and now is — Q30, resolved.** Existence and range
are weak: a line that exists and says something else passes. The advisory
"quoted token near the cited line" check *did* flag all five — `` `with_tag`
occurs in the file but not within 3 lines`` — and stayed advisory, so the run
stayed green.

The fix was not to harden that check. It was to notice the asymmetry underneath
it: **`reference/` is pinned by SHA and cannot move; `crates/` and `xtask/` are
edited every session**, so a line number into them decays on every edit above
it. The window is not months. Two of these citations were repaired for a review
and went stale the same afternoon, when a 21-line comment was added above
`with_tag` while fixing an unrelated clippy warning.

So `cite` now **refuses** a `path:line` citation into `crates/` or `xtask/`
outside a fenced block, and checks instead that a backticked symbol on the line
appears in the file. That check asks "is it there", not "is it near line N", so
it has no false-positive mode and is a hard failure. **84 citations were
converted across 28 files.** The advisory list is now `reference/`-only: 95
entries, all against a pinned tree.

A fenced exact-match block may still carry `path:line` for live code, because
that check compares the quoted body against the file and fails loudly when the
line moves — which is the property Q30 wanted and the proximity heuristic could
not give.

## F80 — the pinned Cranelift cannot build under the pinned toolchain

**Tooling, in Bund2's own workspace.** Recorded because it makes every
`jit`-gated acceptance criterion in RFC-0005 unrunnable, including the one the
whole milestone rests on.

`Cargo.toml` pins Cranelift exactly, on §3.2e's instruction:

```toml Cargo.toml:50
cranelift-codegen  = "=0.135.0"
```

and `rust-toolchain.toml` pins the compiler:

```toml rust-toolchain.toml:2
channel = "1.90.0"
```

`cranelift-codegen@0.135.0` and six sibling crates declare
`rust-version = 1.95.0`. So `--features jit` fails before it compiles a line of
Bund2:

```
error: rustc 1.94.1 is not supported by the following packages:
  cranelift-assembler-x64@0.135.0 requires rustc 1.95.0
```

— on `bund2-jit`, on `bund2-runtime`, and on `bund2-bench`. Nothing gated on
the feature can be built, run or measured today.

**What this costs RFC-0005.** Criterion 2 is "conformance moves by exactly
zero — `cargo xtask conform` reads the same N/M with the `jit` feature on and
off". That cannot be run. Criteria 4, 5, 6, 7, 9, 10, 11 and 12 are equally
gated. The RFC is not wrong about them; they are simply undecidable until this
is resolved, which is a fact its status should carry rather than one an
implementer discovers.

This is exactly the churn §3.2e predicted — *"`cranelift-jit` describes itself
as 'extremely experimental'. The API has moved across recent versions. Pin
exact versions; budget for periodic migration work"*. The pin did its job; the
budgeted migration has not been spent.

- Found by: adding a `jit` feature to `bund2-bench` so RFC-0005's criterion 7
  could run, and discovering the feature cannot build in any crate
- Tooling. Disposition: **FIXED — the toolchain pin raised to 1.95.0**, chosen
  by the repository owner from the three options (raise the toolchain; pin
  Cranelift back to a release supporting 1.90, which may not exist at a monthly
  cadence; or split the build matrix). Raising is the only one that does not
  make Tier 1 a different build from Tier 0.

  Verified on 1.95.0: the whole workspace builds, **276 tests pass**, clippy
  reports no errors, and conformance is unchanged at 73/86. `--features jit`
  and `--features aot` now build in `bund2-jit`, `bund2-runtime`, `bund2-cli`
  and `bund2-bench`, none of which was possible before.

  **RFC-0005 criterion 2 ran for the first time**: `cargo xtask conform` reads
  **73/86 with the feature on and 73/86 with it off**. It passes, and it is
  *vacuous today* — the feature gates no code yet, so equality is trivial. It
  stops being vacuous the day a tier exists, which is the point of having wired
  it now rather than then.

  One environmental note that is not a repository defect: `RUSTUP_TOOLCHAIN` in
  this shell overrides `rust-toolchain.toml`, which is why the 1.90.0 pin was
  not in force and the discrepancy surfaced as a Cranelift error rather than a
  toolchain one.

**And that half is now guarded.** `xtask/src/toolchain/mod.rs` compares the
running `rustc` against the pin on every `cargo xtask` invocation and refuses
on a mismatch, naming `RUSTUP_TOOLCHAIN` when it is the cause:

```
xtask: toolchain mismatch: rust-toolchain.toml pins 1.95.0, but rustc is 1.94.1.
...
RUSTUP_TOOLCHAIN=1.94.1-aarch64-apple-darwin is set in the environment, and it
overrides rust-toolchain.toml.
```

A shell build-driver was considered and rejected: it would duplicate
`rust-toolchain.toml`, need maintaining beside it, and **still lose to the same
environment variable** unless it unset it — solving the general problem by
re-implementing rustup and the specific one by accident. The guard sits at the
entry point every number the project quotes already passes through, which a
driver does not.

The escape is `BUND2_ALLOW_TOOLCHAIN_MISMATCH=1`, deliberately awkward and
deliberately present: a guard with no exit becomes a reason to delete the
guard. A concrete `channel` is compared; `stable`, `beta` and dated nightlies
are not, because they do not denote one version.

## F81 — `#` discards the result of the `unwrap` it depends on

`stdlib_object_execute_base` runs `unwrap` and then `!`, and throws away the
outcome of both:

```rust reference/Bund/src/stdlib/functions/oop/object_execute.rs:35-38
    vm.stack.push(obj_val.clone());
    let _ = vm.apply(Value::call("unwrap".to_string(), Vec::new()));
    vm.stack.push(lambda_val.clone());
    let _ = vm.apply(Value::call("!".to_string(), Vec::new()));
```

The word has already checked that operand #2 is an OBJECT (`:28-32`), so the
common failure is excluded. What is not excluded is an OBJECT carrying no
`.data` anywhere in its `.super` tree: `unwrap` bails
(`value_class.rs:120-123`), pushes nothing, and `#` continues — the lambda then
runs against whatever the stack happens to hold, and `#` reports success.

`#` also returns `Ok(vm)` unconditionally, so a failing lambda is invisible too.

**Status:** REPRODUCED. Bund2 spells the same two discards in
`crates/bund2-stdlib/src/oop.rs`'s `object_execute` rather than propagating,
because the alternative changes what a program observes. Reachable only for an
object with no `.data`; no golden reaches it.

## F82 — `wrap`'s guard and two of its errors name `UNWRAP`

Three of the five messages in `stdlib_object_value_wrap` name the other word:

```rust reference/Bund/src/stdlib/functions/oop/value_class.rs:84-93
    if vm.stack.current_stack_len() < 2 {
        bail!("Stack is too shallow for inline UNWRAP");
    }
    let obj_val = match vm.stack.pull() {
        Some(obj_val) => if obj_val.type_of() == OBJECT {
            obj_val
        } else {
            bail!("UNWRAP NO OBJECT IN #1");
        },
        None => bail!("UNWRAP NO DATA IN #1"),
```

Only the two later ones say `WRAP` (`:97`, `:102`). A `?try` handler that
matches on the text cannot tell which word failed, and the depth in the first
message — 2 — is `wrap`'s, not `unwrap`'s, so the sentence is internally
inconsistent as well as misattributed.

**Status:** REPRODUCED, in `crates/bund2-stdlib/src/oop.rs`'s `wrap_word`. The
text is observable through `?try`.

## F83 — three tags are routed to a conversion that refuses them by name

`Value::conv` sends CLASS and OBJECT to `value_map_conversion`:

```rust reference/rust_dynamic/src/conv.rs:729-736
                MAP | INFO | CONFIG | ASSOCIATION | MESSAGE | CONDITIONAL | CLASS | OBJECT => match &self.data {
                    Val::Map(m_val) => value_map_conversion(t, self.dt, m_val),
                    _ => Err(format!("Can not convert MAP Value from {:?}", &self.dt).into()),
                },
                VALUEMAP => match &self.data {
                    Val::ValueMap(m_val) => true_value_map_conversion(t, self.dt, m_val),
                    _ => Err(format!("Can not convert VALUEMAP Value from {:?}", &self.dt).into()),
                },
```

and both destinations then reject the tag they were sent, because neither guard
lists it:

```rust reference/rust_dynamic/src/conv.rs:595
    if ot != MAP && ot != ASSOCIATION && ot != INFO && ot != CONFIG && ot != MESSAGE && ot != CONDITIONAL {
```

(`true_value_map_conversion`'s guard at `:518` is the same list, and likewise
omits VALUEMAP.) So `valuemap convert.to_int`, `class convert.to_int` and an
OBJECT equivalent all fail with `Source value is not MAP but 30 and not
suitable for conversion` — a complaint about the dispatcher's own routing,
naming a type the program never mentioned.

The dispatcher and the guards disagree about which tags `value_map_conversion`
handles; one of the two lists is wrong, and the arms are unreachable either way.

**Status:** REPRODUCED in `crates/bund2-stdlib/src/convert.rs`'s `conv_value`,
which returns the same sentence for the same three tags. Observable through
`?try`.

## F84 — Bund2's Tier 0 `autoadd` is not the reference's

**A Bund2 defect, dormant.** Recorded because RFC-0005 criterion 18 needs an
oracle for `autoadd`, and Tier 0 is not one.

The reference consults `autoadd` in three places. A CALL is appended *into*
the value beneath it, leaving one value:

```rust reference/rust_multistackvm/src/multistackvm_apply.rs:19-22
                            if self.autoadd {
                                match self.stack.pull() {
                                    Some(mut val) => {
                                        self.stack.push(val.push(value));
```

A CONTEXT value is pushed instead of switching stacks (`:72-73`), and **every
other value, literals included**, is appended to the value beneath (`:89-97`).

Bund2 differs on all three:

- `Interp::dispatch`'s `autoadd` arm pulls the value beneath, pushes it back,
  and pushes the CALL **as a separate value** — two values where the
  reference leaves one (`crates/bund2-interp/src/lib.rs`, `dispatch`).
- Literals and CONTEXT values are not collected at all. `Interp::apply`'s doc
  comment says so: "`autoadd` is not implemented, so the branches at `:19` and
  `:89` are absent."
- The unit test `autoadd_appends_the_name_instead_of_running_it` asserts the
  two-value shape ("the value is left beneath"), so it pins the divergence
  rather than catching it.

**Not observable today**: nothing in Bund2 binds `:` or `;`, which are what set
and clear the flag (`reference/rust_multistackvm/src/stdlib/autoadd.rs:28-29`).

**Status:** OPEN. To be fixed when `:` and `;` are bound — the collecting
append needs `Value::push`'s semantics for every kind of receiver — and the
test rewritten then to assert the reference's shape. Found by RFC-0005's sixth
review (S1).

## F85 — recursion through a loop word aborts Tier 0 on the machine stack

**A Bund2 defect under D37, and one the reference shares.**

`:f { 1 { f } times } register` followed by `f` recurses through `times`. In
Bund2, each level nests Rust frames. `times_base`
(`crates/bund2-stdlib/src/seq.rs`) calls `Vm::eval_lambda`, which pushes a
frame and runs `run_to` from inside the native (`Interp::eval_lambda`,
`crates/bund2-interp/src/lib.rs`), and the body's call to `f` reaches `times`
again from there. So this recursion is bounded by the machine stack, not by the
heap that RFC-0003's frame loop gives direct recursion. RFC-0003's criterion 2
exercises direct recursion only, which is why it did not catch this.

Both implementations die the same way — exit 134, `thread 'main' has overflowed
its stack` / `fatal runtime error: stack overflow, aborting` — measured
2026-09-10 on release builds, on this machine's 8 MiB main-thread stack
(`ulimit -s`: 8176 KiB).

**Why this is a defect and not a match.** D37 forbids an abort however
faithfully it reproduces the reference. In the reference it is an oracle
defect; in Bund2 it is a D37 gap. The shape is not special to `times`: every
native that runs a body synchronously — `loop`, `map`, `while`, the
conditionals, `?try`, the method paths — spends a Rust frame per level the same
way.

**Status:** FIXED in Tier 0, 2026-09-10 — RFC-0005 §S8's Tier 0 floor.
`bund2` runs evaluation on a thread it spawns with an 8 MiB stack plus a
256 KiB reserve (`EVAL_STACK`, `crates/bund2-cli/src/main.rs`), and declares
that stack's top and size. `Vm::eval_lambda`, `Vm::apply` and
`Vm::scoped_call` compare the address of a local against the floor and, below
it, refuse with `Error::stack_exhausted` (`crates/bund2-interp/src/lib.rs`).
Every body-error wrapper passes that error through unchanged
(`Error::context`, `crates/bund2-api/src/lib.rs`), so it is reported once
rather than re-wrapped at every level on the way out.

The program above now reports `machine stack exhausted: …` instead of
aborting. `cargo xtask depth`'s new `loop` axis — 100,000 levels through
`times` — reports a Bund-level error where it used to abort. The reference
still aborts, so this is now a divergence in Bund2's favour, required by D37.
No golden captures either side: the abort's text carries a thread id, which a
capture cannot reproduce.

## F86 — `cite` measured a range citation from its last line

**A Bund2 tooling defect**, found by RFC-0005's seventh review (S8).

`citations_in` (`xtask/src/cite/mod.rs`) read `:19-27` as the two numbers 19
and 27, and corroboration then checked each number on its own against a window
of three lines either side. A quoted token on line 19 was therefore reported
as "not within 3 lines" of 27. RFC-0005's `autoadd`, cited as
`multistackvm_apply.rs:19-27` with `autoadd` on line 19, was one such case. A
comma list had the same fault: every number had to have the token nearby.

**Status:** FIXED 2026-09-10. A range now arrives expanded to every line it
names, up to `RANGE_CAP` (200 lines, past which only its two ends are kept).
Corroboration takes one window per citation: a token near **any** line the
citation names corroborates it, which is the rule the `crates/` check already
followed ("at least one, not all"). A range past the end of its file is
reported once, not once per line. Advisories fell from 116 at the seventh
review to 51, with no document changed. The test is
`a_range_is_expanded_unless_it_is_wider_than_the_cap`.

**What it gives up:** a comma list with one stale number is no longer flagged
as long as another number in the same list is corroborated. That was already
true of `crates/` citations, and a check that flagged every list with any
unrelated number was producing noise, not findings.

## F87 — `execute.` declares a fixed effect and runs arbitrary code

**A Bund2 defect in a declared effect**, RFC-0004's annotation, found by
RFC-0005's eighth review (B2).

`execute` is registered `StackEffect::opaque(1)`, and `execute.` was
registered `eff(1, 0)`. Both reach `execute_value`
(`crates/bund2-stdlib/src/values.rs`). It runs a name through `vm.apply`, a
lambda through `vm.tail_lambda`, and re-enters itself for a LIST or a MAP. So
`execute.` ran arbitrary code while declaring that it consumed one value and
produced none, and `:!. ?effect` printed `opaque=false`. The `consumes=1` was
wrong on its own terms too: `execute_from_workbench` pulls its receiver from
the workbench, and `execute_from_the_workbench_works_on_an_empty_stack`
asserts that the main stack may be empty.

**Why it matters.** `bund2 check` tracked stack depth straight across `!.`,
and RFC-0005 §S5 promotes across any call with a fixed effect, so values held
in registers would have been invisible to whatever `!.` ran. The probes call
it (`tests/probes/remaining-vocabulary.bund:52` and `:55`).

**Status:** FIXED 2026-09-10. `execute.` is `StackEffect::opaque(0)`: opaque,
and consuming nothing from the main stack. `no_fixed_effect_native_runs_a_body`
(`crates/bund2-stdlib/src/lib.rs`) runs every native with a fixed effect
against a stack and a workbench of lambdas, and asserts through `entry_log`
that none starts a body. Before the fix it named `execute.` and nothing else,
which agrees with the review's hand audit of the sixteen functions that
re-enter evaluation. Conformance is 79/86, ceiling 79/86, before and after.

**Note, 2026-09-10 (F91).** "Agrees with the review's hand audit" is true of
the audit and not of the code. `run_init` is one of the sixteen functions, and
the audit checked the words registered on each. `run_init` is reached one call
up, from `object`, which declared `eff(1, 1)`. The lambda-only test could not
reach it either, because `object` needs a registered class name. RFC-0005's
ninth review found it (B2), and criterion 24 is now an audit over the corpus.

## F88 — the corpus lexer takes SYMBOL's ASCII members only, so coverage never saw `∅`, `∈` or `→`

**A Bund2 tooling defect**, found while listing the words Bund2 implements and
no golden runs.

The oracle's grammar admits any Unicode `SYMBOL` in a name: `element` is
`LETTER | SYMBOL | …` (`reference/bund_language_parser/bund.pest:36`), and
pest's `SYMBOL` is the categories Sm, Sc, Sk and So. `xtask`'s corpus lexer
took only SYMBOL's ASCII members (`is_element`, `xtask/src/corpus/lex.rs`), so
`∅`, `∈` and `→`, which the reference registers as aliases
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:25,41,42`), were
reported as "LEX ANOMALY … matches no grammar rule" and dropped from the
tokens. Three probes already ran them, and coverage counted none of the three.

**Status:** FIXED 2026-09-10. `is_unicode_symbol` admits the Unicode blocks
whose every assigned character is a symbol: arrows, the mathematical and
supplemental mathematical operators, the supplemental arrows and the currency
signs, plus Latin-1's `¬ ± × ÷`. Rust's standard library has no
general-category test, so a symbol outside those blocks is still reported as
an anomaly. That errs towards reporting, never towards hiding a character the
oracle rejects.

## F89 — a stack Bund2 creates does not become current, where the reference's does

**A Bund2 defect**, found by the probe `tests/probes/stack-words-by-name.bund`.

The reference creates a stack through `add_named_stack`, which appends it to
the back of its deque (`reference/rust_multistack/src/ts_add.rs:14`), and its
current stack *is* the back
(`reference/rust_multistack/src/ts_current.rs:7`). So `ensure_stack`, and its
alias `stack`, make a new stack current, and so does every push to a named
stack that does not exist yet, since `push_to_stack` creates through
`ensure_stack` (`reference/rust_multistack/src/ts_push.rs:46`). F70 recorded
this mechanism as the cause of `move`'s hang, but decided only the hang.
Bund2's `ensure_stack` appended the name at the far end of its ring, so the
current stack never changed: `"t" stack current` printed `main` where the
oracle prints `t`.

**Status:** FIXED 2026-09-10. Bund2's ring keeps the current stack at the
front and the others after it in the reference's cyclic order, so the
reference's append is two moves: the old current goes from the front to the
back, and the new name goes on the front (`Stacks::add_as_current`,
`crates/bund2-interp/src/lib.rs`). `Stacks::to_stack` now creates through the
same helper. A first attempt put the new name on the front alone. That made
the right stack current but got the ring wrong, and the captured golden
`tests/probes/stack-navigation.bund` failed at once, because `stacks_right`
reached a different stack than the oracle's. That is how the ring-order half
was found. The unit test `move_sends_a_value_to_a_named_stack` had asserted
that the current stack stayed put, which pinned the divergence. It now asserts
that the destination becomes current. Conformance 79/86, ceiling 79/86.

## F90 — Bund2 tags every value pushed to the workbench, where the reference tags none

**A Bund2 defect**, found by the probe
`tests/probes/workbench-string-variants.bund`.

The reference pushes to the workbench with no `set_tag`
(`reference/rust_multistack/src/ts_workbench.rs:25-28`). A value moved there
from a stack keeps the tag that stack gave it, and a value made on the spot has
none. `string.expressionmatch.`'s answer and `convert.to_textbuffer.`'s result
render `tags: {}` in the oracle. Bund2's `push_workbench` tagged every value
with the current stack's name, so both rendered `tags: {"stack": "main"}`.

**Status:** FIXED 2026-09-10. `push_workbench` pushes the value as it arrives
(`crates/bund2-interp/src/lib.rs`). A value from a stack still carries that
stack's tag, because pulling never removes it. Conformance 79/86, ceiling
79/86, before and after.

## F91 — `object` and `display` declare fixed effects and run code

**A Bund2 defect in two declared effects**, RFC-0004's annotation, found by
RFC-0005's ninth review (B2). The same class as F87.

`object` was registered `eff(1, 1)`. `object_word` looks up the class, and
`make_object` and `push_and_init` then call `run_init` for the class and every
parent. `run_init` evaluates `.init`: a LAMBDA through `vm.eval_lambda`, or a
PTR through `vm.method` followed by a direct call to the method native
(`crates/bund2-stdlib/src/oop.rs`). So `object` ran arbitrary code, and could
consume below its operand. `:B class :.init { swap drop } set register` then
`1 2 :B object` leaves `1` and the object: the `2` is gone. The corpus does
this. `class_constructors_demo.bund:31` runs five `.init` lambdas.

`display` was registered `eff(1, 0)`. A `fmt` CONDITIONAL is rendered by its
runner, reached through `vm.conditional("fmt")`, which pulls a value per
placeholder. An OBJECT is dispatched as `:display <obj> !`, which runs its
`.display` method (`display`, `crates/bund2-stdlib/src/singles.rs`).

**Why it matters.** RFC-0005 §S5 promotes across any call with a fixed effect,
so values held in registers would have been invisible to the code these words
run, and `bund2 check` tracked depth straight across them. F87's test passes
only lambdas, and `object` needs a registered class name, so it could not see
either word.

**Status:** FIXED 2026-09-10. Both are `StackEffect::opaque(1)`. RFC-0005
criterion 24 is now `every_fixed_effect_native_keeps_its_effect_over_the_corpus`
(`crates/bund2-stdlib/src/lib.rs`). It runs every program `conform` runs, in
process, with `Interp::effect_audit` on (`crates/bund2-interp/src/lib.rs`). A
native with a fixed effect may not start a body, file a tail request or
dispatch a word, and must move the current stack by what it declares. Before
the fix it named `object` for `format`, `get` and `println` dispatched and a
body started, in `class_constructors_demo.bund`. `display` was named for its
depth, and its OBJECT arm is the same shape as `object`'s. Conformance 82/89,
ceiling 82/89, before and after, on a working tree carrying three uncommitted
probes (79/86 without them).

## F92 — ten declared effects miscount the current stack

**A Bund2 defect in declared effects**, RFC-0004's annotation, found by the
first run of RFC-0005 criterion 24's corpus audit (F91).

`StackEffect` is one pair, and RFC-0004 §S1 says what it counts: the main
stack, with `consumes` a floor rather than a net (`dup_one` is `1 -> 2`). The
audit compares each fixed-effect native's pair with the depth change it causes
on the current stack, whenever it returns `Ok` without switching stacks. Ten
disagreed:

| word | declared | observed | now | why |
|---|---|---|---|---|
| `clear` | `0 -> 0` | 2 → 0 | `opaque(0)` | empties the stack; the probed column already reads `0+` |
| `fold` | `0 -> 1` | 3 → 1 | `opaque(0)` | the whole stack into one LIST |
| `move` | `2 -> 0` | 4 → 0 | `opaque(2)` | drains below the name; `EFFECTS.txt` already called it variadic |
| `dup_many` | `1 -> 0` | 2 → 4 | `opaque(2)` | pushes a copy per unit of its count |
| `format` | `1 -> 1` | 3 → 2 | `opaque(1)` | one value per placeholder; `format.` was already `opaque(0)` |
| `pull` | `1 -> 1` | 5 → 2 | `opaque(1)` | one value per name in its list; `pull.` was already `opaque(0)` |
| `fold_stack` | `1 -> 1` | 1 → 0 | `1 -> 0` | the LIST goes to the *named* stack |
| `swap_in` | `1 -> 0` | 2 → 0 | `2 -> 0` | the name and the count both come off the stack, as its guard says |
| `print.` | `1 -> 0` | 2 → 2 | `1 -> 1` | F77's floor of one is kept; the value comes off the workbench |
| `println.` | `1 -> 0` | 1 → 1 | `1 -> 1` | the same |

**Why it matters.** RFC-0005 §S5 models the depth after a call as
`before − consumes + produces`, and keeps values below `consumes` in
registers. A pair that is off in either direction misplaces every promoted value
after the call. `clear` kept values that the program had cleared.

**Status:** FIXED 2026-09-10 (above). The audit passes. Conformance 82/89,
ceiling 82/89, before and after, on the same working tree as F91.

**Open: `tests/golden/EFFECTS.txt`.** `cargo xtask effects` now reports
`format` as declared `1->0` against a probed `1->1`. The probe fed a STRING,
a template with no placeholder, so it measured one arm. The line below belongs
in `EFFECTS.txt`, and `move`'s existing line should say that it is opaque since
F92. `tests/golden/` was outside what the session making this fix could edit,
so both wait for the repository owner:

    format	arm — the probe fed a STRING sentinel, and a string with no placeholder is a template that pulls nothing, so it read 1->1. `format` pulls one stack value per distinct placeholder in its template (`crates/bund2-stdlib/src/control.rs`, `format_base`), which is not known until run time. Declared `StackEffect::opaque(1)` since F92, which the table shows as 1->0.

**Not reached, and not decided here.** Named-stack words whose current-stack
effect depends on whether the name *is* the current stack: `clear_in`,
`drop_in`, `drop_stack`, `dup_one_in`, `dup_many_in`, `move_from`, `return_to`,
`return_from`, `rotate_stack_left` and `rotate_stack_right`. The corpus never
hands one the current stack's name, so the audit cannot see them. Making them
opaque would decide Q34's named-stack case, which is open. `fold_stack` and
`swap_in` above are corrected for the other case only.

**Note, 2026-09-10 (F94).** `drop_stack` does not belong in that list. It
takes no name and always removes the current stack, so its pair was wrong on
every operand, not only when a name happens to be the current stack's. It is
opaque since F94.

## F93 — `effect_of` answers with a native's effect for a lambda that shadows it

**A Bund2 defect**, found by RFC-0005's ninth review (S1).

`Registry::effect_of` followed aliases, then returned
`slot.native.or(slot.command)`. It never looked at the slot's `lambda`, and
`register_lambda` leaves the native binding in place by design. Its doc comment
said "`None` when the name resolves … to something with no declared effect — a
lambda", which held only for a lambda with no native beside it. After
`:drop { 1 } register`, `5 drop` left `5 1`, because the lambda ran, while
`:drop ?effect` answered `consumes=1 produces=0`. `$drop` answered `None`,
because the lookup did not strip the sigil.

**Why it matters.** RFC-0005 §S5 and D46 named `effect_of` as what tells
compiled code a callee is a native with an effect it may trust. An
implementation that followed that citation would keep values in registers
across a lambda shadowing a native, which is the case D46 forbids.

**Status:** FIXED 2026-09-10. `effect_of` resolves the name exactly as
`Registry::resolve` does, `$` included: `None` for anything that reaches a
lambda, and the native's effect for `$name`
(`effect_of_answers_for_what_dispatch_reaches`, `crates/bund2-api/src/lib.rs`).
`?effect` on a shadowed name now pushes `nodata`, as it does for any lambda.
`bund2 check` analyses a program against the registry before the program runs,
so it still cannot see a `register` inside the program. That is RFC-0004's
limitation, and this does not change it. RFC-0005 §S5 now classifies each
callee with `resolve`, and criterion 22 has a shadowing case. Conformance
82/89, ceiling 82/89, before and after.

## F94 — `drop_stack` declares `1 -> 0` and removes the whole current stack

**A Bund2 defect in a declared effect**, RFC-0004's annotation, found by
RFC-0005's tenth review (B1) and confirmed by criterion 28's palette.

`drop_stack` takes no operand and removes the current stack by name
(`drop_stack`, `crates/bund2-stdlib/src/stack.rs`), as the reference's does:
`stdlib_drop_stack` ignores both operands
(`reference/rust_multistack/src/stdlib/drop.rs:54`), and `TS::drop_stack`
removes the current stack outright
(`reference/rust_multistack/src/ts_drop_stack.rs:10-23`). It was registered
`eff(1, 0)`, although `docs/arity.md` has always read `0+`. A compiled body
promoting across it would keep a value in a register that Tier 0 had thrown
away. No golden runs it, because the reference names the next stack with a
fresh nanoid on every run, so criterion 24's corpus audit could not see it.

**Status:** FIXED 2026-09-10. `drop_stack` is `StackEffect::opaque(0)`.
Before the fix, `every_fixed_effect_native_keeps_its_pair_over_the_promotable_palette`
(`crates/bund2-stdlib/src/lib.rs`) named it and nothing else: "declares (1, 0)
and moved `main` from 4 to 0". F92 gains a dated note, since its "not
reached" list filed `drop_stack` among the named-stack words.

## F95 — `string.distance.jarowinkler` panics inside the `natural` crate

**A Bund2 defect under D37, shared with the reference**, found by RFC-0005's
tenth review (B2).

`string.distance.jarowinkler` and its workbench form hand their operands to
`natural::distance::jaro_winkler_distance` (`crates/bund2-stdlib/src/singles.rs`,
`crates/bund2-stdlib/src/library_string.rs`). `natural` 0.5.0 panics there in
two places: `(max_length / 2) - 1` underflows when the longer string has fewer
than two characters (`src/distance.rs:19`), and a slice runs past the end of
the shorter string (`src/distance.rs:41`). The reference links the same crate
(`reference/Bund/Cargo.toml:58`). `"zz_nofile" "A" string.distance.jarowinkler`
unwound out of Bund2's evaluation thread, and the process exited 1 with a Rust
backtrace. The D37 lint cannot see into a dependency.

**Status:** FIXED 2026-09-10, by D49. The program now reports
`internal error: native string.distance.jarowinkler panicked: start byte index
2 is out of bounds of A (at …/natural-0.5.0/src/distance.rs:41)` through the
reporter, prints nothing to stderr, and exits 0 as any reported error does.
The reference still crashes, so this is a divergence in Bund2's favour with no
golden, as F85 is. The distance itself stays undefined for those operands:
computing one would be an answer the reference never gives. Criterion 28's
palette reaches the panic on many operand pairs and runs on.

## F96 — a native that files a tail request and then fails leaves the request to run later

**A Bund2 defect, latent**, found by RFC-0005's tenth review (S3.2).

`Vm::tail_lambda` files a body in `pending_tail` for the loop to run after the
current native returns (`Interp::request_tail`, `crates/bund2-interp/src/lib.rs`).
Nothing cleared it on an error, because `run_to`'s error arm unwinds frames
only. A native that filed a request and then returned `Err` left it set, and
after `?try` had caught the error, the next `take_pending` ran the stale body,
which no caller had asked for. No `bund2-stdlib` native does this: all four
request sites return `Ok` straight after filing. An embedder's native can.

**Status:** FIXED 2026-09-10. `Interp::invoke`, the one place Tier 0 calls a
native, clears `pending_tail` when the native fails.
`a_failed_native_leaves_no_tail_request` shows the stale body not running.

**Dated note, 2026-09-11 — a `bund2-stdlib` native does reach this, and a
compiled call would not inherit the fix** (RFC-0005's fifteenth review, B1).
The entry above says no `bund2-stdlib` native files a request and then fails.
`execute_value`'s LIST arm does (`crates/bund2-stdlib/src/values.rs`): it
pushes and executes each item in turn, so a LAMBDA item files a request
through `Vm::tail_lambda` and a later item can fail. Run 2026-09-11,
`?try :try { [ { 10 } 5 ] ! } set :except { "EXCEPT" println } set :recovery { "RECOVERY" println } set !`
prints `EXCEPT` and `RECOVERY` and leaves only the `error` CONDITIONAL — no
`10` — which is `Interp::invoke`'s clearing at work. RFC-0005's per-native
adapter calls a `NativeFn` directly and so never reaches `invoke`, which is
why its `status_of` clears the request cell on any error (RFC-0005 §S5,
criterion 26).

**Dated note, 2026-09-11 (second) — the entry above is right again.** F113's
fix has `execute_value`'s LIST arm run a lambda item at once instead of
filing it, so no `bund2-stdlib` native files a tail request and then fails.
The program in the note above now leaves `10` beneath `?try`'s CONDITIONAL
(`a_list_execute_that_fails_keeps_what_earlier_items_left`,
`crates/bund2-stdlib/src/host.rs`). An embedder's native remains the case
F96 and RFC-0005's criterion 26 describe.

**Dated note, 2026-09-11 (third) — the note above was premature.** F113's
first fix reached a lambda that was itself a list item, not one held in a dict
inside the list, so `execute_value` could still file a request and then fail
(RFC-0005's sixteenth review, B1). F113 as completed decides by reach, and the
entry above holds again from that commit.

**Dated note, 2026-09-13 — the compiled half is closed.** RFC-0005's per-native
adapter exists (`crates/bund2-jit/src/lower.rs`, `jit_call_native`) and calls
`Vm::clear_tail_request` whenever it answers an error, so a native that files a
body and then fails leaves nothing for the next `take_pending`. D56 put that
method on the trait for exactly this caller, and this is its first consumer. The
note above expected `status_of` to do it; `status_of` does not exist yet, so the
clearing sits in the adapter, where the obligation is either way.

**Two tests, because one passes without meaning anything.**
`a_failing_native_leaves_no_tail_request_behind_compiled_code` is this entry's
Tier 0 test run through compiled code. The second is a **positive control** —
`a_succeeding_native_keeps_the_tail_request_it_filed` — because an adapter that
cleared unconditionally would satisfy the first while discarding every tail
request a compiled call ever filed: the same place as F96 and a worse defect.
Both in `crates/bund2-jit/src/lower.rs`.

## F97 — `notifthenelse` does not negate

**An original-implementation defect, reproduced**, found while implementing the
uncovered core words.

`notifthenelse` runs through the same base as `ifthenelse`, with
`TypeCond::IfFalse` in place of `TypeCond::IfTrue`
(`reference/rust_multistackvm/src/stdlib/logic/ifthenelse_fun.rs:92-98`). That
arm reads `if ! cond_bool { else } else { then }` (`:64-70`), which is the
`IfTrue` arm's choice written the other way round. So both words take the
`then` lambda when the condition is true, and differ only in their error
prefix. Confirmed against the oracle on 2026-09-11:
`true { "A" } { "B" } notifthenelse` leaves `"B"`, the lambda on top, as
`ifthenelse` does, and `?false*` does the same.

**Disposition: reproduce.** The goldens capture the oracle, and a word that
behaved as its name says would fail them. Bund2's `notifthenelse` and
`notifthenelse.` go through `ifthenelse_base` with their own prefixes
(`crates/bund2-stdlib/src/control.rs`, `notifthenelse`), and the probe
`tests/probes/negated-conditionals.bund` records the behaviour.

## F98 — Bund2's `conv` had no arm for a LIST source

**A Bund2 defect**, found by the probe `tests/probes/list-push-and-unfold.bund`.

The reference's `conv` sends a LIST to `value_list_conversion`
(`reference/rust_dynamic/src/conv.rs:707-708`). That converts it to itself for
LIST, to its length for INTEGER and FLOAT, to whether it is empty for BOOL, and
to a MAP keyed by position for MAP (`:293-341`). Bund2's `conv_value`
(`crates/bund2-stdlib/src/convert.rs`) had no LIST arm, so every conversion from
a list fell through to `Can not convert Value from 9`. `push` converts both of
its operands with `conv(LIST)`, and `[ 1 2 ] 3 push` failed where the oracle
answers `[3, [1, 2]]`.

**Status:** FIXED 2026-09-11. `conv_value` has a LIST arm for LIST, INTEGER,
FLOAT, BOOL and MAP. STRING and TEXTBUFFER were already answered through
`display`. The reference's RESULT, QUEUE, FIFO and MATRIX targets are not
built, because Bund2 constructs none of those kinds. Conformance 85/93,
ceiling 85/93, before and after.

## F99 — `json.path` prints every match to stdout

**An original-implementation defect, reproduced**, found while implementing
`json.path`.

The word collects each `JsonPathValue::Slice` into its answer, then
`println!("{:?}", &s)` on it (`reference/rust_multistackvm/src/stdlib/json/json_path.rs:30-38`).
A program that asks for `$.a` on `{"a": 1}` therefore prints
`Slice(Number(1), "$.['a']")` before anything it prints itself. It looks like a
debugging line left in. Confirmed against the oracle on 2026-09-11 with
`tests/probes/json-path.bund`.

**Disposition: reproduce.** The golden captures the line, so a Bund2 that kept
quiet would fail it. Bund2 prints the same `Debug` form with the same crate
version, jsonpath-rust 0.7.5, so the text is the crate's and not a copy
(`crates/bund2-stdlib/src/json.rs`, `json_path`). It is output the word writes,
not a diagnostic, so the Reporter seam (D36) does not apply.

## F100 — Bund2 printed a JSON value in its raw `Debug` form

**A Bund2 defect**, found by the probe `tests/probes/json-path.bund`.

The reference converts JSON to STRING as compact `serde_json::to_string` text
(`reference/rust_dynamic/src/conv.rs:662-679`), reached from `conv` at `:705`.
So `println` of a JSON array prints `[1]`. Bund2's `BundValue::display` had no
arm for a JSON payload and fell through to `render`, printing
`Value { id: …, dt: 24, … data: Json(Array [Number(1)]) … }`. No earlier
golden printed a value while it was still JSON: `json.to_value` converts first.

**Status:** FIXED 2026-09-11. `display` renders a JSON payload with
`serde_json::to_string` (`crates/bund2-value/src/lib.rs`, `display`).
Conformance 90/98, ceiling 90/98, before and after.

## F101 — Bund2's `graph.path` builder skipped bad nodes and misread four-element edges

**A Bund2 defect**, found by re-reading `make_graph.rs` for the `algos` graph
words.

`make_fast_graph` fails on a node that is not a string, with
`MAKE_FAST_GRAPH: error casting node name` (`reference/Bund/src/stdlib/functions/graph/make_graph.rs:22-26`).
It takes a weight only from an edge of exactly three elements (`:59`); an edge
of four or more gets 100. Bund2's `build` (`crates/bund2-stdlib/src/graph.rs`)
silently skipped a non-string node, and took the third element of any edge
longer than two as its weight. Neither path is reached by a golden: the two
graph goldens use string nodes and three-element edges.

**Status:** FIXED 2026-09-11. `build` reports the non-string node and applies
the exactly-three rule. The new `build_algos` port follows `make_graph`
(`:78-141`) the same way. Unit tests in `graph.rs` cover both rules.
Conformance 90/98, ceiling 90/98, before and after.

## F102 — the alias `$` can never be called

**An original-implementation defect, reproduced**, found while probing the last
core aliases.

The reference registers `$` as an alias of `take`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:36`). But `apply`
tests a call's first character for `$` **before** it resolves aliases, and a
`$` call means "the internal word named by the rest"
(`reference/rust_multistackvm/src/multistackvm_apply.rs:30-35`).
`call_internal_word` drops the first character and calls `i` on the remainder
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:6-9`). For
a bare `$` the remainder is the empty string, so `42 . $` fails with
`i() for stack returned: Inline not registered`. Confirmed against the oracle
on 2026-09-11. The alias is registered and dead.

**Disposition: reproduce.** Bund2 separates the sigil in `Interner::lookup_call`
(`crates/bund2-api/src/lib.rs`), so a bare `$` looks up the empty name and
fails with `$ not registered`, which is the same outcome. Bund2 registers the
alias anyway (`crates/bund2-stdlib/src/stack.rs`), as the reference does, so
`bund2 words` lists it. No probe pins it, because the only observable result
is an error, whose presentation differs by D36.

## F103 — `sleep.seconds` with a negative count waits for about 585 billion years

**An original-implementation defect, reproduced**, found while implementing the
host words.

`sleep.seconds`, and its alias `sleep`, cast the count with `cast_int` and then
wait `Duration::new(n as u64, 0)`
(`reference/Bund/src/stdlib/functions/system/sleep.rs:13-19`). A negative
count wraps to a count near `u64::MAX` seconds, so `-1 sleep` never returns.
Not run against the oracle, because it would not return.

**Disposition: reproduce.** The cast is the reference's, and refusing a
negative count would be a behaviour it does not have. Bund2's `sleep_seconds`
(`crates/bund2-stdlib/src/host.rs`) waits the same way. No golden can capture
it.

## F104 — `io.graph` panics inside `rasciigraph` on an empty or all-NaN list

**A Bund2 defect under D37, shared with the reference**, found while probing
`io.graph`.

`io.graph` hands its floats to `rasciigraph::plot`
(`reference/Bund/src/stdlib/functions/io/graph.rs:44-47`). `rasciigraph` 0.2.0
indexes an empty row there (`src/lib.rs:146`) when the series has no finite
values: for `list io.graph`, and for a list of `float.NaN`. Confirmed against
the oracle on 2026-09-11: it prints `index out of bounds: the len is 0 but the
index is 0` and exits 101. A one-element list draws.

**Status:** handled by D49, as F95 is. Bund2 reports
`internal error: native io.graph panicked: …` through the reporter and exits 0
(`crates/bund2-stdlib/src/host.rs`, `io_graph`). No chart is invented for
these inputs, because the reference never draws one.

## F105 — `generator` crashes on a parameter of the wrong kind

**An original-implementation defect, not reproduced**, found while implementing
the random words.

`generator` reads its configuration's `type` with `cast_string().unwrap()`
(`reference/Bund/src/stdlib/functions/generators/generator.rs:24-25`), and
each kind reads its parameters with `cast_float().unwrap()` or
`cast_int().unwrap()` (`reference/Bund/src/stdlib/functions/generators/normal.rs:24-25`,
and the same in each sibling). A parameter of the wrong kind panics:
`"Mean" 1 set` for a `normal` generator, where the default is the FLOAT `0.0`.
Confirmed against the oracle on 2026-09-11: it prints ``called
`Result::unwrap()` on an `Err` value: "This Dynamic type is not float: 2"``
and exits 101.

**Disposition: Bund2 reports it**, as it does for F68, where the reference also
crashes. `generator` fails with `GENERATOR: parameter Mean must be a FLOAT, not
2` (`crates/bund2-stdlib/src/random.rs`, `float_param`). No golden can hold a
crash, so no conformance number moves.

## F106 — `string.random.lorem` with a negative count runs out of memory

**An original-implementation defect, not reproduced**, found while implementing
the random words.

The count is cast `n as usize`
(`reference/Bund/src/stdlib/functions/string/random.rs:59-60`), so `-1` asks
`lipsum` for nearly `usize::MAX` words, which it keeps allocating until the
process dies. Not run against the oracle, because it would take the machine's
memory with it.

**Disposition: Bund2 refuses the count**, with `Error casting in
STRING.RANDOM.LOREM: a word count cannot be negative, got -1`
(`crates/bund2-stdlib/src/random.rs`, `lorem`), for the same reason as F105.
`sleep.seconds` looks alike, but it is different (F103): a negative sleep only
waits, which the reference really does, so that one is reproduced.

## F107 — `generator.sample*` skips one value of a sequence after each batch

**An original-implementation defect, reproduced**, found while implementing
the random words.

A sequence generator (`sawtooth`, `periodic`, `sinusoidal`, `square`) keeps a
position. `generator.sample*` advances it once per value, and then once more
after the loop (`reference/Bund/src/stdlib/functions/generators/sawtooth.rs:81-89`,
and the same in each sequence file). So a batch followed by a single sample
misses a value. Confirmed against the oracle on 2026-09-11 with
`tests/probes/random-words.bund`. For the default sawtooth, `4
generator.sample*` answers `0.111…` through `0.444…`, and the next
`generator.sample` answers `0.666…`, skipping `0.555…`.

**Disposition: reproduce.** The probe's golden captures it
(`crates/bund2-stdlib/src/random.rs`, `generator_sample_n`).

## F108 — `input*` never checks that its lambda is a lambda

**An original-implementation defect, reproduced**, found while implementing the
terminal words.

`input*` guards its operand with `if ! lambda_value.type_of() == LAMBDA`
(`reference/Bund/src/stdlib/functions/io/input.rs:91-93`). In Rust `!` on a
`u16` is bitwise NOT, so the test compares the inverted tag with `LAMBDA`,
which is never true, and `INPUT*: #1 must be a LAMBDA` is never reported. Any
value is accepted. A value that is not a lambda fails only when the first line
arrives, where `lambda_eval` refuses it with `This is not a lambda`
(`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:27-29`).

**Disposition: reproduce.** Bund2's `input_loop` accepts any value in the same
way, and fails at the first line with
`INPUT* returned error from LAMBDA: This is not a lambda`
(`crates/bund2-stdlib/src/terminal.rs`). The capture feeds no input, so no
golden reaches the difference.

## F109 — a `sqlite` BLOB cell is a serialised Bund value to the reference

**A Bund2 gap**, found while implementing `sqlite`.

The reference turns a BLOB cell into a value with `Value::from_binary`
(`reference/Bund/src/stdlib/functions/conditional/conditional_sqlite.rs:169-171`).
That does not wrap the bytes. It bincode-decodes them as a serialised
`rust_dynamic::Value`, and parses a JSON-wrapped value as JSON
(`reference/rust_dynamic/src/bincode.rs:51-77`). So a BLOB that the reference
wrote itself, as its world file does, comes back as the value it was. Any other
bytes fail the whole query with the decoder's message, which is how the `?` at
`:170` propagates it.

Bund2 has the wire types, byte-identical to the reference's
(`crates/bund2-value/src/wire.rs`, D20), but nothing converts a wire value into
a `BundValue` yet.

**Status:** OPEN. A non-NULL BLOB is refused with `CONTEXT.RUN: a BLOB column
holds a serialised Bund value in the reference, which Bund2 cannot decode`
(`crates/bund2-stdlib/src/data.rs`, `sql_cell`). The fix is the wire-to-value
conversion, which the world file (D27) will need anyway. No golden reaches a
BLOB: the probe's `tag` column is all NULL.

**Status: FIXED 2026-09-11.** `bund2_value::wire` now converts both ways,
between the reference's byte-identical wire value and `BundValue`
(`WireValue::from_value`, `into_value`, `to_binary`, `from_binary`). Its tests
decode seven values the oracle itself saved with `save.model`, taken from its
SQLite world file, and check that re-encoding keeps every field but the id. A
decoded value mints a fresh identity, which no golden can see (F14). `sqlite`'s
`sql_cell` decodes a BLOB through it.

## F110 — `load.model` ignores the model's name

**An original-implementation defect, reproduced**, found while implementing the
model words.

`load.model` takes a world file and a model name, but the query it runs is
`SELECT name, model FROM MODELS` with no condition
(`reference/Bund/src/stdlib/helpers/world/models.rs:22`). It pushes every model
in the world, in save order, and the name appears only in the error reported
when there are none (`:69-71`). Confirmed against the oracle on 2026-09-11:
seven models were saved as `m1` to `m7`, and `"anything" "tst" load.model`
pushed all seven. Under `--noio`, `load.model`'s stub also reports
`bund SAVE.MODEL functions disabled with --noio`
(`reference/Bund/src/stdlib/functions/bund/bund_models.rs:128-130`).

**Disposition: reproduce, both.** Bund2's `load_model` pushes every model in
save order, and both stubs say `SAVE.MODEL`
(`crates/bund2-stdlib/src/world.rs`). The probe
`tests/probes/models-world.bund` records the first.

## F111 — six named-stack words declare a pair that is wrong on the current stack

**A Bund2 defect in declared effects**, found by the first run of D55's audit,
which added the current stack's name, `"main"`, to criterion 28's palette.

Each of these words takes a stack by name and declares that it consumes only
its operands from the current stack. When the name *is* the current stack's,
it also acts on the current stack, so its depth change breaks the pair. The
effect audit recorded it on the first run with `"main"` as the operand:

| word | declared | observed with `"main"` | now | what it does to the current stack |
|---|---|---|---|---|
| `clear_in` | `1 -> 0` | 4 → 0 | `opaque(1)` | clears it |
| `drop_in` | `1 -> 0` | 4 → 2 | `opaque(1)` | drops from it |
| `dup_one_in` | `1 -> 0` | 4 → 4 | `opaque(1)` | duplicates onto it |
| `dup_many_in` | `2 -> 0` | 5 → 10 | `opaque(2)` | duplicates onto it, a copy per unit of its count |
| `fold_stack` | `1 -> 0` | 4 → 1 | `opaque(1)` | folds it into one LIST and leaves the list there |
| `return_from` | `1 -> 0` | 4 → 2 | `opaque(1)` | moves a value from it to the workbench |

`fold_stack` was corrected by F92 to `1 -> 0` for a *named* stack, which the
corpus audit saw. The current-stack case is the one no program in the corpus
reached. It is the same shape as Q34's named-stack case (D55).

**Why it matters.** RFC-0005 §S5 models the depth after a call from the pair
and keeps the values below `consumes` in registers. A word that empties the
current stack while claiming to consume one value would leave compiled code
holding values the program had removed. `opaque` makes each a promotion
barrier, and makes `bund2 check` stop at it, which is honest: its effect
depends on the name it is given.

**Status:** FIXED 2026-09-11 (`crates/bund2-stdlib/src/stack.rs`). The
palette passes. `bund2 check` now stops at these six, so RFC-0005's promotion
stops are re-derived with them.

**Note, 2026-09-11:** re-derived after this fix, the stops are unchanged, at
189 programs and 137 = 116 + 21, because no program reaches the six.

## F112 — after an exit, the native that ran the body still ran

**A Bund2 defect in Tier 0 against D52**, found by RFC-0005's thirteenth
review (B1).

D52 says the interpreter "stops at the next word boundary and runs nothing
more of the program", and `Vm::request_exit`'s documentation says the same.
The reference ends the process inside the word
(`reference/Bund/src/stdlib/functions/bund/bund_exit.rs:30`). Tier 0 gated
only at the top of `apply_step` and of `Vm::eval_lambda`. `Interp::run_to`
pops a finished frame without the gate, so a body a native ran synchronously,
whose last word was `bund.exit`, returned `Ok` to that native, which went on
in Rust until its next step:

| program | before | after |
|---|---|---|
| `[1] { 7 exit } map` | exit 7, `[1]` left (`map` collected) | exit 7, `1` left |
| `[1 2] { 7 exit } map` | exit 7, `2` left (the second item pushed) | exit 7, `1` left |
| `"p> " { println 7 exit } input*` | read and waited for a second line | stops after the first |

A body in which `exit` was not last already stopped at its next step.

**Why it matters.** RFC-0005's compiled code reports the exit to that native
as an error, so the tiers would have disagreed on the final stack and on what
they read from the terminal. Criterion 2 cannot see it, since no golden
captures anything after an exit.

**Status:** FIXED 2026-09-11 (`crates/bund2-interp/src/lib.rs`).
`Vm::eval_lambda`, `Interp::apply` and `Vm::scoped_call` consult `exit_gate`
after `run_to`. A native that catches errors still runs its handler: `?try`
pushes its `error` CONDITIONAL before its `except` body is refused, which is
RFC-0005's assumption 25. No golden moves.
`an_exit_ending_a_synchronous_body_stops_the_native_that_ran_it`
(`crates/bund2-stdlib/src/host.rs`).

**Note, 2026-09-11 — the fix also changed error text** (RFC-0005's
fourteenth review, S1). The gates in `Interp::apply` and `Vm::scoped_call`
change no native's behaviour: `eval_source`, `execute_value`, `apply`, `text`
and `run_context` return straight after the call. What they change is the
error that propagates. Before the fix, a `bund.eval`, `use`, `apply`,
`!`-by-name or `context` body ending in `exit` returned `Ok`. The refusal came
later, from `Vm::eval_lambda`'s gate, without the native's own wrapper. Now it
carries that wrapper (`Attempt to evaluate value … returned error: …`,
`CONTEXT lambda returns: …`). Only `?try` shows it, as the `context` slot of
the CONDITIONAL it leaves. `try_keeps_the_error_its_body_returned_after_an_exit`
(`crates/bund2-stdlib/src/host.rs`) pins both gates.

**Correction to the note above, 2026-09-11** (RFC-0005's fifteenth review,
S3). It says the five natives "return straight after the call". Two do not.
`eval_source` loops over the parsed stream, applying each value
(`crates/bund2-stdlib/src/singles.rs`, `eval_source`). The LIST and MAP arms
of `execute_value` loop and recurse (`crates/bund2-stdlib/src/values.rs`,
`execute_value`). Before F112, an
`exit` that was not a `bund.eval` source's last value was refused at the next
value's `apply_step`, so the wrapper named that value rather than the `exit`;
and in a LIST the loop went on to the next item. The note's conclusion stands
for reachable programs, because a list literal does not evaluate its items —
`[ 1 2 + ]` keeps `+` as a CALL (run 2026-09-11) — so the arms that do Rust
work before re-entering evaluation cannot appear in one (RFC-0005's
assumption 28). The reason given was wrong; the outcome was not.

## F113 — a second tail request overwrites the first, so `!` on a list runs only the last lambda

**A Bund2 defect**, found by RFC-0005's fifteenth review (S3) while checking
the request model.

The reference's `execute` runs a LAMBDA item **at once**: its list loop pushes
each item and recurses (`reference/rust_multistackvm/src/stdlib/execute.rs:40-48`),
and the LAMBDA arm is `return vm.lambda_eval(ptr_value)` (`:93-95`). Bund2's
`execute_value` files each lambda item through `Vm::tail_lambda` instead
(`crates/bund2-stdlib/src/values.rs`), which is right for a single lambda —
the body runs after the native returns, costing no Rust frame (RFC-0003 §S4,
*The flat frame loop*).
But `Interp::request_tail` **assigns** (`crates/bund2-interp/src/lib.rs`), so
a second request in the same native replaces the first, and the first body
never runs:

| program | Bund2, 2026-09-11 | the reference, from the lines above |
|---|---|---|
| `[ { 10 } { 20 } ] !` | `20` | `10` beneath `20` |

One lambda in a list is unaffected, and so is every other request site: the
other four file once and return.

**Why it matters.** It is not a tier divergence — compiled code calls the same
native — so RFC-0005's criterion 2 cannot move on it. RFC-0005's §S5 says "a
request is never lost", which is a rule about compiled call sites; its
assumption 29 now says this is not a property of Tier 0.

**Status:** FIXED 2026-09-11 (`crates/bund2-stdlib/src/values.rs`). The
repository owner chose to match the reference: a LAMBDA item in the LIST arm
runs at once through `Vm::eval_lambda` rather than being filed. That spends a
Rust frame per lambda inside a list, bounded by the Tier 0 floor (F85). The
tail path is untouched where it matters — a bare `{ 10 } !` is the LAMBDA arm,
which still files — so RFC-0003 §S4's frame loop keeps its guarantee. (Both
citations here read `§S4a` until the seventeenth review's S1: §S4a is about
`endcontext`, not about tail requests.)

**Confirmed against the oracle**, built out of tree and run 2026-09-11:
`[ { 10 } { 20 } ] !` leaves `10` beneath `20`, and `[ { 10 } 5 ] !` fails
with `Received value is not of executable type` while keeping the `10` the
first item left. Bund2 now does both, and no golden moves — conformance stays
at its ceiling. Two tests cover it:
`every_lambda_in_an_executed_list_runs_in_order`
(`crates/bund2-stdlib/src/values.rs`) and
`a_list_execute_that_fails_keeps_what_earlier_items_left`
(`crates/bund2-stdlib/src/host.rs`).

**A consequence for F96.** The LIST arm was the one `bund2-stdlib` native that
filed a tail request and could then fail. It no longer files, so F96's case is
an embedder's native again, and RFC-0005's criterion 26 states it that way.

**Dated note, 2026-09-11 — the first fix was not complete** (RFC-0005's
sixteenth review, B1). It tested whether the *item* was a LAMBDA, so a lambda
held in a **dict** inside the list still took the general path and reached the
filing arm. The defect survived in that shape:

| program, with `D` a dict whose `k` holds a lambda | before | now |
|---|---|---|
| `[ D(k→{10}) { 20 } ] !` | `20` alone | `10` beneath `20` |
| `[ D(k→{10}) 5 ] !` | the error, nothing left | the error, `10` left |

Two dicts cannot be posed together: the first member's result comes to rest on
the second dict's key, and the reference's shape does that too, so the pair
above uses a dict item and then a plain lambda item.

What decides is **reach**, not the item's type, which is the shape the
reference has: one function recursing into itself, whose LAMBDA arm runs the
body at once whichever arm reached it
(`reference/rust_multistackvm/src/stdlib/execute.rs:93-95`, from the LIST loop
at `:40-48` and the dict arm at `:55-83`). `execute_value` now carries a
`Reach`: `Top` files, as the program's own `{ 10 } !` should, and `Nested`
runs at once. `a_lambda_reached_through_a_dict_in_a_list_runs_at_once`
(`crates/bund2-stdlib/src/values.rs`) holds both rows. Neither row is
reachable from Bund source today — `push` converts its operands and a literal
does not evaluate items — so the values are built through the API, and the
first word returning a list of dicts would make it reachable without touching
this file.

## F114 — executing a deeply nested list aborted the process

**A Bund2 defect against D37**, found by RFC-0005's seventeenth review (B2),
which measured what the RFC's assumption 30 had asserted without a number.

`execute_reached` walked a container by recursing into itself, so the depth of
the *value* bought Rust frames, and the Tier 0 floor was consulted only where
an arm reached `Vm::apply` or `Vm::eval_lambda` — never between two of those
frames. RFC-0005 §S8 promises that a compiled body "and anything it calls
without re-entering evaluation" stays inside `STACK_RESERVE`, which is 256 KiB
(`crates/bund2-interp/src/lib.rs`). At roughly 550 bytes a level that carries
about 470 levels, not the depth a program chooses. Measured 2026-09-11 on the
release binary:

    list
    20000 { drop list push } times
    "built" println
    !
    "executed" println

printed `built`, then `thread 'bund2' has overflowed its stack`, `fatal
runtime error: stack overflow, aborting`, exit 134. Without the `!` the same
program exited 0, so the abort was the traversal and nothing else.

**Status:** FIXED 2026-09-11 (`crates/bund2-stdlib/src/values.rs`). The
traversal is driven from a `Vec` worklist on the heap, so a value's depth
costs no stack at all: the program above now prints `built` and `executed` and
exits 0. Evaluation nesting still spends a Rust frame per level and is still
bounded by the floor in `Vm::eval_lambda`. Order is unchanged — an item is
finished, with everything under it, before the next begins — and the tag write
each item passes through still happens when the item is reached.
`a_deeply_nested_list_executes_without_touching_the_stack_floor`
(`crates/bund2-stdlib/src/values.rs`). No golden moves.

**What it does not reach.** The same measurement found two more aborts on
paths with no floor at all: F115 (dropping a deep value) and F116 (parsing a
deep run-time string). RFC-0005's assumptions 34 and 35 record that §S8's
floors are silent about both.

## F115 — dropping a deeply nested value aborts the process

**A Bund2 defect against D37**, found by RFC-0005's seventeenth review (B2)
while measuring the reserve.

A value holds its members, so dropping one drops them, and a nested LIST drops
on the depth of the nesting. Nothing bounds that recursion, and no floor can
be put on it: the drop runs wherever the value dies. Measured 2026-09-11 on
the release binary:

    list
    30000 { drop list push } times
    "built" println

prints `built` — the program's own work is finished — and then
`thread 'bund2' has overflowed its stack`, `fatal runtime error: stack
overflow, aborting`, exit 134. At 20,000 the same program exits 0.

**Why it matters.** D37 is absolute: shipped code does not abort. The stack
this takes with it is the user's, and the trace names Rust frames and no Bund
word. It is not a tier divergence — no compiled code is involved — so
RFC-0005's criterion 2 cannot move on it, which is why RFC-0005 records it as
assumption 35 rather than leaving it to conformance.

**Status:** FIXED 2026-09-11 (`crates/bund2-value/src/lib.rs`). The repository
owner chose the iterative drop. `Drop for HeapValue` takes each level's
members onto a heap worklist and frees them level by level, so a value's depth
costs heap rather than stack. Each level is emptied **before** it is dropped,
so the drop the loop triggers finds nothing to descend into. A payload another
value still shares is left alone: `take_members` asks `Rc::get_mut` first, and
answers `None` while a second owner holds it — which is the case `dup` makes,
sharing the payload on a different schedule from the header (D13). `attr` and
every member-bearing payload are covered: `List`, `Lambda`, `Map`, `ValueMap`
and `Scalar`.

The program above now prints `built` and exits 0.
`a_deeply_nested_value_drops_without_recursing` drops 100,000 levels, and
`dropping_one_owner_leaves_a_shared_payload_intact` holds the sharing rule
(both in `crates/bund2-value/src/lib.rs`).

**The cost was measured**, because a `Drop` impl on the hottest type in the
system is what RFC-0005's criterion 1 bounds. `value/push_pull/balanced` reads
**9.47 ns** against the criterion's 20 ns floor, 3.4% faster than the previous
run rather than slower. No golden moves.

**What it does not reach.** `render`, `PartialEq`, `Hash` and the wire format
walk a value's members too, and each still recurses on depth. RFC-0005's
assumption 36 records the rendering half; none of them is filed yet, and each
would need its own iterative rewrite and its own test.

## F117 — rendering a deeply nested value aborts the process

**A Bund2 defect against D37**, found by RFC-0005's eighteenth review (B2),
which measured what that RFC's assumption 36 had called unmeasured.

`BundValue::render_into` and `render_payload` walk a value's members by
recursing: `render_into` descends through `attr`, and `render_payload` through
the members of `Lambda`, `List`, `Map` and `ValueMap` — the last twice over,
since it renders each key to sort by it
(`crates/bund2-value/src/lib.rs`). Nothing bounds that, so the depth of the
value is Rust frames. Measured 2026-09-11 on the release binary:

    list
    24000 { drop list push } times
    "built" println
    debug.display_stack

prints `built` and then `thread 'bund2' has overflowed its stack`, `fatal
runtime error: stack overflow, aborting`, exit 134. At 20,000 the same program
exits 0.

**Why it matters.** This is the same class as F114, F115 and F116, on a word
the conformance path runs: the golden capture epilogue calls
`debug.display_stack`, and RFC-0005's criterion 14 names it. `--raw-values`
reaches the same renderer. The report path is unaffected, because a
diagnostic's values go through `BundValue::summary`, which is bounded at depth
2 by design (D36).

**Status:** FIXED 2026-09-11 (`crates/bund2-value/src/lib.rs`). The repository
owner chose the iterative renderer, so the text is byte-identical at every
depth and no golden can move. `render_into` drives a `Vec<RenderStep>`, where a
step is either already-formatted `Text` or a `Value` still to render;
`render_step` emits a value's header, queues its `attr` members and its tag
text, and lets the payload queue its own. The container arms of
`render_payload` — `Lambda`, `List`, `Map`, `ValueMap` — push their members and
separators instead of recursing, in reverse, since the worklist is a stack. A
boxed `Scalar` is flat and renders in place. `ValueMap`'s keys are still
rendered to sort by them, and each of those is a walk the same driver bounds.

The program above now prints `built` and `rendered` and exits 0, at 24,000 and
at 30,000. `a_deeply_nested_value_renders_without_recursing` renders 30,000
levels, and `the_rendered_text_is_unchanged_by_the_worklist` pins the bytes for
a list holding a scalar and a map. The 49 tests in the crate, several of which
assert exact rendered text, pass unchanged, and conformance is unmoved.

**The last of the four.** F114 (executing a container), F115 (dropping a
value), F116 (parsing a run-time string) and F117 (rendering a value) were the
same defect in four places: Rust recursion driven by the depth of data a
program chooses. D39's dated note of 2026-09-11 states the rule they share.

## F116 — parsing a deeply nested run-time string aborts the process

**A Bund2 defect against D37**, found by RFC-0005's seventeenth review (B2).

`bund.eval`, `!!` and `use` parse a string the program produced at run time
(`eval_source`, `crates/bund2-stdlib/src/singles.rs`). `eval_source` checks
the Tier 0 floor before it re-enters evaluation, but the *parse* happens
first, on the same thread, and the parser recurses on the nesting depth of
what it is reading. Measured 2026-09-11 on the release binary: a 16,000-deep
list literal, built as a string and handed to `bund.eval`, aborts with the
same stack overflow, exit 134.

**Why it matters.** RFC-0003's parse-reach criterion covers a source *file*,
which an author writes; this depth comes from a value the program built, so a
program can choose it. D37 applies either way. RFC-0005's assumption 34
records that its floors do not reach this path.

**Status:** FIXED 2026-09-11 (`crates/bund2-syntax/src/lib.rs`). The parser
carries the nesting depth and refuses past `MAX_NESTING`, which is **1024**.
The check sits in `Parser::nested`, before the frame for the refused level is
entered, so a program never spends more than the bound — the refusal is not a
rescue after the fact. All three bracket forms go through it. The program
above now reports

    nesting deeper than 1024 blocks, which Bund2 will not parse

with the opening bracket's line and column, as any other parse failure does,
and it is catchable by `?try` because `bund.eval` returns it as an ordinary
error. The script itself ends with exit 0, since the error is reported rather
than fatal.

**Why 1024**, chosen as D39 chooses a threshold — far past any real program,
far below where it breaks (all measured 2026-09-11):

| | nesting |
|---|---|
| deepest in the corpus, its probes and its examples | **2** |
| a debug build aborted between | 2,000 and 4,000 |
| a release build aborted between | 12,000 and 16,000 |
| the bound | **1,024** |

`nesting_past_the_bound_is_reported_not_fatal`
(`crates/bund2-syntax/src/lib.rs`) parses at the bound and refuses one past
it, for all three forms;
`evaluating_a_too_deeply_nested_string_reports_instead_of_aborting`
(`crates/bund2-stdlib/src/host.rs`) is the `bund.eval` case F116 was filed
for. Conformance is unmoved at 105/113: no golden nests past 2.

**A deviation from the reference**, which has no such bound and aborts instead
— the same disposition F85 took for evaluation nesting, and for the same
reason: D37 is absolute, and a program's own data must not be able to end the
process.

## F118 — the wire codec recurses on a value's depth, so `save.model` aborts

**A Bund2 defect against D37**, found by RFC-0005's nineteenth review (B1),
which read the sentence written a day earlier claiming no wire encoder existed
to recurse.

`crates/bund2-value/src/wire.rs` is the bincode codec (D4) and it descends per
level in both directions: `val_of` maps `Payload::List`, `Lambda`, `Map` and
`ValueMap` through `WireValue::from_value`, which calls `val_of` again, and
`Payload::Scalar` calls it directly; `into_value` decodes `List` and `Lambda`
through `decode_all` per item. `bincode`'s derived `Serialize`/`Deserialize`
on the nested `WireValue` recurse on top of that, and `WireValue`'s derived
`Drop` once more. `save.model` reaches it (`crates/bund2-stdlib/src/world.rs`,
`save_model`), and so do `load.model` and the sqlite blob path.

Measured 2026-09-11 and 2026-09-12, release binary:

    list
    3400 { drop list push } times
    "built" println
    "m" swap "wx" save.model

prints `built`, then aborts with a stack overflow, exit 134. **3,350 exits 0
and 3,400 aborts**, and each iteration adds exactly one level.

**It is the shallowest of the class by an order of magnitude** — F114 needed
20,000, F115 30,000, F116 16,000, F117 24,000, F119 28,000 — and the depth is
one a program could reach without meaning to.

**Attributed by measurement, after two false starts.** A probe calling
`from_value` and `to_binary` directly survived 12,000 levels, which looked
like an exoneration; it was not. Re-run on smaller threads the threshold
tracks the stack almost exactly — 2 MiB aborts at 3,400 and survives 1,700,
4 MiB survives 3,400 and aborts at 6,800, 8 MiB survives 6,800 — so the codec
spends about **600 bytes of stack per level** and the earlier probe simply had
more room than the CLI does. Ruled out along the way: redb (a shallow value
saves fine after the same deep build), the reporter (`--no-dump-stack` still
aborts), `swap`, and the stack left at exit.

**Not reachable by any existing audit.** `save.model` re-enters no evaluation,
so it is absent from `every_reentering_function_is_named`'s set and from §S8's
`c_p`; and it is one of the six `ACTS_ON_HOST` natives criterion 28's palette
never runs. RFC-0005's assumption 21 calls that exclusion conservative, which
is true for promotion and false for D37: an unrun native can still abort.

**Status:** PARTLY FIXED 2026-09-12 (`crates/bund2-value/src/wire.rs`), the
remainder awaiting a disposition. The repository owner chose to flatten what
Bund2 owns and then measure the residual.

**What was flattened.** `from_value` and `into_value` both build from an
arena: each value is given an index on the way down and its children are
queued, then the tree is assembled bottom-up, so a parent finds its children
already built. `WireValue` gains an iterative `Drop`, as `HeapValue` did in
F115 — it holds `WireValue`s in `attr` and in five `Val` variants, so the
derived drop recursed too. `decode_all` went with the recursive decoder.
Adding `Drop` makes `WireValue` non-destructurable, so three sites now take
their fields with `mem::take`/`replace` instead of moving out.

**What it bought, measured on the release binary:**

| direction | before | after |
|---|---|---|
| `save.model` (encode) | aborts at **3,400** | survives 38,000, aborts at **40,000** |
| `load.model` (decode) | — | aborts at **6,000**, survives 5,500 |

The encode ceiling moved by an order of magnitude and is now bincode's own
`Serialize` recursion. **The decode side is the binding limit**: bincode's
derived `Deserialize` builds the nested `WireValue` before any Bund2 code
runs, at roughly 1.4 KB of stack a level on an 8 MiB thread — reproducible,
5,500 passing twice and 6,000, 6,500 and 8,000 aborting twice each. A worklist
cannot reach inside the dependency.

The bytes are unchanged: `wire_fixtures/*.hex` and
`a_bund2_value_survives_the_wire` pass untouched, and conformance does not
move.

**Status:** FIXED 2026-09-12. The repository owner chose to bound the codec.

**The bound is on writing, because a decode cannot be bounded.** bincode
builds the whole nested `WireValue` before any Bund2 code runs, so there is no
point at which a depth check could refuse a blob. `to_binary` therefore
measures the value's depth first — on the heap, so the measuring cannot itself
overflow — and refuses past `bund2_value::wire::MAX_WIRE_DEPTH`, **256**. A
value too deep to read back is never stored. The alternative was the worse
failure: `save.model` succeeding on a value `load.model` then aborts on.

**256 comes from the worst case, not the best.** Decoding aborts at (measured
2026-09-12, nested one-element lists):

| build | thread | aborts at |
|---|---|---|
| debug | 2 MiB — an embedder's default | 512 |
| debug | 8 MiB | 2,048 |
| release | 8 MiB — what `bund2` gives evaluation | 6,000 |

The deepest nesting anywhere in the corpus is 2, so the bound is 128× what any
program has needed and clear of the shallowest abort, which is how D39 chooses
a threshold.

What a program sees, run 2026-09-12 at depth 300:

    SAVE.MODEL returns: Error compiling mode: the value nests 257 deep,
    and 256 is the most the wire format can carry

with the source line and the stack, as any other word's error is reported. The
script exits 0, because the error is reported rather than fatal — the same
shape F116's parser refusal has. (`Error compiling mode` is the reference's
own wording, typo included, reproduced by `save_model`.)

`a_value_too_deep_for_the_wire_is_refused_not_written` holds the refusal and
the at-bound round trip, and
`a_value_at_the_bound_survives_the_wire_on_a_small_thread` runs a value at the
bound through both directions on a **2 MiB** thread, the worst case the number
was chosen against. `save.model` at 255 saves; at 300 and at 3,400 it reports.
Conformance does not move.

**The caveat, stated rather than left implicit.** A blob produced somewhere
else — the oracle, or a future writer — deeper than the bound still aborts on
read, because the recursion is inside bincode. D31 ruled that nothing outside
Bund reads or writes a world file, which is what makes a write-side bound
sufficient here; if that ever changes, this needs revisiting.

**Dated note, 2026-09-12 — the caveat had a live path, and it is now bounded
and recorded.** RFC-0005's twenty-first review found the third decode site the
write-side bound never covered: `sqlite` decodes every BLOB in any SQLite file
a program names (`sql_cell`, `crates/bund2-stdlib/src/data.rs`), and D31
cannot justify it, since a SQLite database is not a world file and has not
been one since D27 made world files redb. Reproduced 2026-09-12 on the dev
binary, with BLOBs hand-built in bincode's legacy layout: depths 10, 256 and
1,000 (630 B to 58 KB) decode and run the lambda; **depth 3,000, a 174 KB
BLOB, aborts with a stack overflow, exit 134**.

The repository owner's disposition is D58: `sqlite` refuses a BLOB over
`MAX_BLOB_BYTES`, 16 KiB — the write-side bound read through size, at a
measured 58 bytes a level, so `MAX_WIRE_DEPTH`'s 256 levels is about 14.8 KB —
and D37 gains a stated exception for bytes read from a file Bund2 did not
write, because bincode builds the nested value before any check could run. A
wide, shallow BLOB over the cap is refused too, which is the conservative
side. No corpus program reads a BLOB, so conformance does not move.

## F136 — F133's rule asks whether a body gains *anything*, not whether it gains *enough*

**A Bund2 defect**, found by decomposing criterion 10's per-value cost into the
three regimes a value can take.

F133 gave the tier a rule: refuse a body with **no** inlinable site and **nothing**
to promote, because such a body pays the boundary and is repaid nothing. That
fixed a measured +24% regression and was right as far as it went. It tests the
gain against **zero**. It does not test the gain against the **cost**.

### The measurement

`regimes` times the three paths a value can take, matched lengths, same call,
two Tier 0 blocks and two tier blocks, every window `CLEAN`, R² ≥ 0.996:

| regime | Tier 0 | with the tier | delta |
|---|---|---|---|
| promoted literal | 7.85 ns | 6.73 ns | **−1.13 ns** |
| generic call | 34.01 ns | 39.14 ns | **+5.13 ns** |
| inlined site | 38.81 ns | 14.62 ns | **−24.19 ns** |

**A generic call is 5.13 ns dearer compiled than interpreted.** `jit_apply`
calls `Vm::apply`, which is Tier 0's own path, and the boundary — slot load,
`call_indirect`, request-cell load, status protocol — is added around it.

### The hole

A body of **one int literal and twenty `clear` calls** has one promoted value,
so `would_gain` answers true and the tier compiles it. It then pays
20 × 5.13 ≈ **103 ns per entry** to save 1.13 ns. F133's rule admits it because
the rule asks "is the gain non-zero?" when the question is "does the gain
exceed the cost?".

This is not hypothetical arithmetic: the `generic` family above **is** that
shape, one literal and N calls, and it compiles at every length measured
(`bodies 1`, checked per size).

### What a sufficient rule would weigh

`plan_body` already computes everything needed before a byte is emitted — the
site count, the promoted count, and the number of generic calls. The measured
coefficients give the comparison directly:

    24.19 × sites  +  1.13 × promoted   >   5.13 × generic_calls

**Not proposed as those constants.** They are one host's numbers, they move with
the machine, and baking measured nanoseconds into a compilation rule would make
the tier's behaviour depend on the laptop it was profiled on. A ratio test — a
body must inline some fraction of its values — is the shape that survives
re-measurement, and picking the fraction is a §S7 decision.

### Why it is filed rather than fixed

It changes which bodies compile, which is §S7's to state and the repository
owner's to decide, exactly as F133 was. F133 is **not withdrawn**: refusing
zero-gain bodies was correct and remains correct; this is a strengthening of the
same rule, from a test against zero to a test against cost.

- Found: 2026-09-16, decomposing criterion 10's per-value cost
- Status: **OPEN**. No rule changed. The tier currently compiles bodies that
  lose, provided they contain at least one literal.
- Depends on: F133 (the rule this strengthens), §S6 (the fragment table that
  decides what inlines), §S7 (where the policy lives), criterion 10

## F135 — the whole `value` group's no-tier control exceeds the band it polices — RESOLVED as a measurement protocol

**A defect in a benchmark group**, found taking criterion 7's verdict under D70.
The sixth of this kind after F124, F127, F128, F129 and F132.

`value` is a group criterion 7 **protects**: D41's work sits below the tier and
the tier must not disturb it, so a move beyond 5% in either direction fails. The
group cannot currently answer that question about itself.

### The measurement

A feature-**off** binary against a feature-off baseline — the same binary, no
tier in either half, **nothing changed** — re-baselined with the allocator
settled and run three times:

| row | control 1 | control 2 | control 3 |
|---|---|---|---|
| `with_tag/scalar_unique` | −5.85% | +1.52% | −5.74% |
| `with_tag/heap_shared` | −3.55% | **+25.56%** | **+22.49%** |
| `promote/scalar` | −5.49% | +2.00% | −2.65% |
| `clone/scalar` | −1.77% | −0.78% | −1.72% |
| `push_pull/balanced` | −1.56% | −0.85% | −3.37% |

Nearly all p = 0.00. **Three of five rows exceed ±5% against themselves**, and
`with_tag/heap_shared` — which had looked steady at 41.0–42.0 ns all session —
swings **+25%**.

### Two claims in this entry's first draft are withdrawn

**"The defect is in this row's stability, not in the group."** False. It was
filed against `value/promote/scalar` alone, on the grounds that the other four
rows were steady: `clone/scalar` had read +0.90%, +1.69%, +0.82%. Under a fresh
baseline `heap_shared` is the worst row in the group and `clone/scalar` moves
consistently negative. The group is the unit, not the row.

**"It drifts in one direction, which is the tell."** Also false, and it was the
basis of a diagnosis this entry no longer makes. `promote/scalar` read 42.23,
40.00, 38.37, 38.65 ns and then appeared to settle across six consecutive runs
at 39.0–39.9 — which looked like an allocator warming to a plateau. The next
baseline taken immediately afterwards read **42.46 ns**, above where the session
began. The apparent saturation was six runs inside one quiet window, not a
property of the benchmark. **Allocator warm-up is withdrawn as the cause.**

### The cause is not known

What is established: the movement is present with **no compiled code on either
side**, so it is not the tier's; it is far larger than the ±1.5% floor the same
suite shows elsewhere; and it is not explained by a stale baseline, because
re-baselining warm did not remove it. Machine state, thermal behaviour, or
something in the process are all candidates and **none has been tested**. This
entry names no mechanism: three diagnoses were fitted to numbers in this
register on 2026-09-15 alone (F130's probes, F132's cold start, this entry's
allocator), and each survived several consistent runs before failing.

**A smaller observation, recorded and not chased.** Criterion's own
`change/estimates.json` disagrees with its printed intervals on these rows —
`clone_scalar` at −1.72% mean against −2.97% median, `heap_shared` +22.49% mean
against +25.53% median. Worth knowing before either figure is quoted.

### Narrowed by a discriminator, 2026-09-15 (after a host restart)

Three byte-identical copies of `promote/scalar` were added to the group:
`scalar`, `scalar_adjacent` (immediately after it) and `scalar_late` (after the
rest of the group). Within one process, `scalar` against `_adjacent` is variance
at the same moment and against `_late` is drift across the run; across
processes, `scalar` against `scalar` carries everything fixed at process start.
Twelve processes, load sampled before each:

| | within-process spread | cross-process spread |
|---|---|---|
| runs 1–6, load **12.7–14.8** | ≤1.3% | 32.08–34.25 ns = **6.8%** |
| runs 7–12, load **2.9–5.3** | 0.3–6.0% | 31.57–33.08 ns = **4.8%** |
| all twelve | ≤6.0% | 31.57–34.25 ns = **8.5%** |

**Process-start causes are excluded.** In the pre-restart data one process began
at 40.07 ns and climbed to 56.44 ns *inside itself* — a 41% move with address
layout, allocator arena and binary all fixed. ASLR and arena placement cannot do
that.

**Machine load is excluded, and this entry withdraws it.** Between the
observations above and this section, load was the working explanation for the
±25% excursions. Runs 1–6 above sat at load 13–15 with CoreServices at up to
254% CPU and produced a *tighter* spread than runs 7–12 at load 3–5. On an
18-core host the competing work lands on other cores. That is a fourth
hypothesis offered and refuted in this register in one day, after F130's probes,
F132's cold start and this entry's allocator.

**The fixture is not noisy.** Adjacent identical copies agree to −0.28%, −0.02%,
+0.55%, +0.57% in quiet runs. `promote` is measured steadily; what moves is
between processes.

**What is left**, untested and named as candidates rather than causes: per-core
frequency scaling, thermal state, and scheduler placement of the benchmark's own
thread. Nothing here claims one of them.

### The floor this suite actually has, which is the finding beyond F135

The residual cross-process spread is **~5–8% on a 33 ns row**, consistent across
both load regimes. **Superseded below**: on a genuinely idle host the same row
resolves to **0.7%** inside a **1.7%** thermal envelope. The 5–8% was measured
while the machine was still reindexing after a restart. The **±1.5% floor quoted elsewhere in these registers and in
RFC-0005 was measured on `dispatch/dup_drop` at 88 µs** — three orders of
magnitude larger — and it does not transfer to rows of tens of nanoseconds.

That is why `value` cannot police a 5% band across processes: the band is inside
the instrument's own noise at this magnitude. It also bounds any future
measurement of the same size — the boundary decomposition's components are
1–3 ns, so they cannot be resolved by comparing separate processes at all.

### Everything measured before the restart is incomparable

The host was restarted mid-investigation and came up on Darwin 27.0.0. The same
row moved from ~40 ns to ~33 ns. The saved baselines `c7`, `warm` and `len` are
dead, and **absolute** figures taken before the restart — including the boundary
split in RFC-0005 criterion 10 — must be re-taken on this host before they are
quoted again. The ratios are likely to hold; the nanosecond figures are not.

### The cause, found — 2026-09-15, on a quiet host after a restart

**Core placement is excluded, and it is excluded by measuring it.** This is an
Apple M5 Pro: 6 P-cores and 12 E-cores (`sysctl hw.perflevel0/1.logicalcpu`).
Forcing the benchmark to E-cores with `taskpolicy -b`, interleaved against
default scheduling so drift hits both:

| pair | default | forced E-core |
|---|---|---|
| 1 | 31.68 ns | 123.60 ns |
| 2 | 31.71 ns | 124.77 ns |
| 3 | 31.48 ns | 122.76 ns |
| 4 | 31.61 ns | 122.68 ns |

**3.88×.** If the thread were occasionally landing on an E-core the signature
would be a 290% jump and a sharply bimodal distribution at ~32 and ~123 ns.
Nothing in twelve prior runs came near 123 ns. Core placement is out — the sixth
hypothesis eliminated.

That table also establishes what this instrument can do: **four consecutive
default runs spanning 31.48–31.71 ns, a spread of 0.7%** — tighter than the µs
floor, and an order tighter than anything this entry previously recorded.

**Thermal drift is the mechanism, confirmed by a cooldown.** Twelve runs
back-to-back with no pause, then 150 s idle, then six more:

| block | runs | mean |
|---|---|---|
| A, first four (37.1 °C) | 31.74, 31.89, 31.78, 31.80 | **31.80 ns** |
| A, last four (46.9 °C) | 32.14, 32.24, 32.51, 32.42 | **32.33 ns** |
| B, after 150 s cooldown | 31.81, 31.86, 31.79, 31.91, 31.72, 32.27 | **31.89 ns** |

The rise is **+1.7%**, and cooling **resets it**: block B returns to block A's
cold value within 0.3%. The covariate moved with the effect — package
temperature 37.1 → 46.9 °C across block A, P-cluster frequency 2001 → 1816 MHz
— which is the first time in this register that a predicted covariate actually
tracked a predicted effect. B6 already shows the climb restarting at 32.27.

**And thermal does not account for the ±25%.** 1.7% is real and now measured; it
is nowhere near the excursions this entry was opened over. What is left is the
plainest explanation and the one this entry should have reached first: those
excursions were measured **while Spotlight reindexed after a restart**, and
nothing resembling them has occurred since the host settled. They were a busy
machine, and load was sampled once per run rather than throughout.

### What it costs — much less than this entry claimed

On a quiet host the instrument resolves to **0.7%**, inside a **1.7% thermal
envelope** under sustained back-to-back load. Both sit well within criterion 7's
5% band. **The `value` group is not inherently unable to police that band** — it
was measured on a host that was never quiet, by an entry that sampled load once
per run and drew a conclusion from three samples.

### Disposition

Not a fixture defect and not a criterion defect: a **measurement-protocol**
defect. The rows measure what they claim, and D70's band is sound. What was
missing is the condition under which the band means anything.

**The protocol, which is what this resolves to.** Criterion 7's `value` group —
and any row at tens of nanoseconds — is measured on an idle host, with a
cooldown between back-to-back blocks so the 1.7% thermal envelope does not
accumulate into the reading, and with interference **sampled throughout each
run** rather than before it. A run whose window was contaminated is discarded,
not recorded.

**Sampling before the run is not enough, and that was learned the hard way.**
This entry first said "checked *before each run* rather than once". Criterion
7's re-run then produced a window that opened at load 1.13 and went bad inside
the three minutes that followed: every group regressed together — `startup`
+2.6%, `dispatch` +8.6%, `corpus` +4.9%, and `startup/parse/mixed`, which parses
a string and never touches an interpreter, +2.5%. Rows sharing no mechanism
cannot move together through any code path, so the run was caught by **reading
the shape of the result**, which is exactly the inference a protocol is supposed
to make unnecessary.

**Built as a guard around the run.** A sampler records the busiest process that
is not the benchmark's own every 3 s for the whole window, and the run prints
its own verdict beside the numbers:

    GUARD [CLEAN] samples=69 peak=13.9% (…/MenuBarAgent) mean=6.7%

`CLEAN` below a 15% peak, `CONTAMINATED` above it. The contaminated case is then
labelled in the output rather than deduced from which rows moved, and a
discarded run is discarded on its window, never on its result — which is the
distinction that keeps this from becoming a way to drop readings one dislikes.
The script is `crates/bund2-bench/scripts/guarded_bench.sh`.

**Its first catch was the measuring apparatus itself.** The guard's second run
came back `CONTAMINATED  peak=23.2% (claude)` — the agent session polling the
output file and listing directories *during* the window. The numbers that run
produced were unremarkable, so it was discarded on its window while its result
was one anybody would have been content to keep: the rule working in the
direction that costs something, which is the only direction that tests it.

**So the protocol has one more clause: do not touch the host during a window.**
No reads, no greps, no parallel tool calls, nothing. Excluding the observer from
the sampler would be the wrong fix — the observer really does compete for the
machine — so the measurement is left alone instead, and a run that had to be
watched is a run that has to be repeated.

**And one more, found by using it: cool the baseline block too.** Criterion 7's
clean re-run came back met, with every row inside the band — and *every* row
reading slightly negative across all three runs. That is not the tier winning
uniformly; it is the baseline having been taken warm, as the first block after a
build with no cooldown before it. **A slow baseline flatters the A/B**, which is
the dangerous direction for a must-not-regress criterion: with 1–2% of offset, a
true +6% regression reads as +4% and passes. The baseline is a block like any
other and gets the same cooldown. Until it does, a verdict carries that much
slack, and the criterion's note says so.

The three dispositions this entry previously listed are all withdrawn. **A**
(change the fixture) would have altered what the rows measure to fix a host
problem. **B** (judge against a same-session control) would have set the gate at
the width of whatever noise the day happened to have. **C** (find the cause
first) is what was done, and it found that five of the six candidate mechanisms
were not present at all.

- Found: 2026-09-15, taking criterion 7's verdict under D70
- Narrowed: 2026-09-15, by a three-copy discriminator over twelve processes.
  Process-start causes excluded; machine load excluded and withdrawn as this
  entry's working explanation; the fixture shown steady within a process. The
  ±25% figure is superseded by ~8.5% — the earlier controls were excursions
  sampled without checking load. Cause still unknown.
- Rewritten: 2026-09-15, after a fresh warm baseline showed the group, not the
  row, and withdrew the allocator diagnosis
- Resolved: 2026-09-15, on a quiet host. Core placement excluded by measurement
  (3.88× E-vs-P, so it cannot hide in an 8% spread); thermal drift confirmed and
  bounded at **1.7%**, reversible by cooldown; the instrument shown to resolve
  to **0.7%**. The excursions this entry was opened over were Spotlight
  reindexing after a restart.
- Status: **RESOLVED**, 2026-09-15, as a **measurement-protocol** defect rather
  than a fixture or criterion defect. `value` can police D70's band on an idle
  host. The protocol above — check load before each run, cool between blocks,
  discard runs taken against a busy machine — is what this entry resolves to; no
  fixture and no criterion changed. Six hypotheses were eliminated on the way:
  allocator warming, stale baseline, process-start layout, arena placement,
  machine load, core placement.
- Depends on: criterion 7, D41 (the work the group protects), D70 (the band it
  polices), F134

## F134 — criterion 7's "no statistically significant difference" clause cannot be satisfied by any build — RESOLVED as D70

**A defect in an acceptance criterion's wording**, not in Bund2. Found by
running criterion 7 in full on 2026-09-15 and having no defensible way to
declare it met or unmet.

### The clause

RFC-0005 criterion 7 asks two different things of its two protected groups:

| group | requirement |
|---|---|
| `startup` | **no change**: Criterion reports no statistically significant difference, and the point estimate moves by **< 5%** |
| `value` | no change, same tolerance |

The `< 5%` half is a band and works. The **significance** half is the defect.

### Why no build can satisfy it

Criterion's significance test asks whether a difference is *distinguishable from
zero* given the observed variance — **not** whether it is large. With 100
samples of a microbenchmark, differences far below any band are reliably
detectable: binary layout, allocator state and code placement differ between two
builds and are perfectly systematic, so they register as significant however
small they are.

**This is measured, not argued.** The feature-**off** control — one binary
against its own baseline, *nothing changed* — reads:

| benchmark | three runs |
|---|---|
| `dispatch/dup_drop/w3000` | −0.28% (p = 0.57), +0.06% (p = 0.88), −0.85% (p = 0.05) |
| `dispatch/native_call/w4000` | **−1.02% (p = 0.00)**, −0.51% (p = 0.09), −0.02% (p = 0.98) |

A binary compared with itself reports a statistically significant change. A
clause that this fails cannot be met by a *different* binary, which is what the
criterion actually compares.

The full criterion 7 run the same day shows the same thing on the protected
groups, at movements nobody would call a regression:

| row | three runs |
|---|---|
| `startup/registry/register_all` | +0.95% (p = 0.00), +1.25% (p = 0.00), +0.50% (p = 0.16) |
| `value/clone/scalar` | +0.90% (p = 0.01), +1.69% (p = 0.00), +0.82% (p = 0.02) |
| `value/push_pull/balanced` | +1.88% (p = 0.00), +1.28% (p = 0.00), +2.56% (p = 0.00) |

Every one is inside the 5% band by a wide margin. Every one is "statistically
significant" more often than not.

### Why this matters rather than being pedantry

A criterion that **cannot be met** stops working as a gate. It is either read
literally and fails forever — in which case it says nothing about any particular
build, and a real regression in `startup` would be indistinguishable from the
permanent failure — or it is read loosely, in which case the reading is the
reader's and not the RFC's. Both outcomes remove the protection the criterion
exists to provide, and the second invites exactly the move this RFC forbids
elsewhere: explaining a failure away at the point of reporting it.

It also blocked a verdict today. On the band, criterion 7 passes on every row of
all five groups across three runs. On the significance clause it cannot pass.
The criterion was left unresolved rather than decided either way, because
choosing a reading is a decision about what the criterion means.

### Dispositions, none taken

- **A — drop the significance clause**, keep `< 5%`. The band is what every
  other row in the table uses, and it is the thing with a stated basis (§S1's
  run-to-run spread). Simplest, and makes the two halves of the table
  consistent.
- **B — require significance only above the band.** "A change beyond 5% that is
  also statistically significant fails." Significance then filters false alarms
  rather than generating them, which is the role it can actually play.
- **C — compare against the day's own drift floor.** Criterion 7 already
  measures a no-tier control before the A/B; the requirement becomes "inside the
  control spread measured in the same session". Strictest and most honest, and
  the most work: it needs the control taken every time, which the procedure
  already does.

**Not taken here.** Rewriting an acceptance criterion so that it can be passed
is the move this RFC warns about in terms — "a criterion whose failure can be
explained away by changing its denominator would not be worth having" — so the
choice is the repository owner's, and the criterion stays unresolved until then.

### Resolved, 2026-09-15 — option B, as D70

The repository owner chose **B**: a change beyond 5% fails only if Criterion
also reports it significant. Significance now filters movements that already
exceed the band instead of firing on movements far inside it, and the band
governs in **either direction** for `startup` and `value`, since a protected row
moving 5% *faster* with the tier on is as much a sign of disturbance as one
moving slower. Criterion 7's table states this once; D70 carries the reasoning
and why A and C were declined.

**It made a verdict possible, and one row still blocks it** — but not for this
entry's reason. Under D70 every row of all five groups is inside the band across
three runs **except** `value/promote/scalar`, whose own no-tier control drifts
−5.85% to +25.56% across the group on a warm baseline, worst on a row that had
looked steady all session. That is F135, a defect in the benchmark rather than in the
criterion or the tier.

- Found: 2026-09-15, running criterion 7 in full for the first time since F131,
  D69 and F133
- Status: **RESOLVED as D70**, 2026-09-15. The `< 5%` band was always sound and
  is unchanged; only the significance clause's role moved.
- Depends on: RFC-0005 criterion 7, §S1 (the run-to-run spread the band rests
  on), F132 (the same suite's floor, measured)

## F133 — a body with no inlinable site is compiled anyway, and always loses — RESOLVED

**A Bund2 defect**, found by restating criterion 7's `arith` to measure steady
state (D69).

A compiled body runs every value that is **not** an inlined site through
`jit_apply`, which calls `Vm::apply` — Tier 0's own path — and adds the
boundary around it: an entry trampoline, a slot load and an indirect call per
value, plus the request-cell load after each. Inlining and promotion are what
repay that. A body with **no** inlinable sites repays none of it and is
guaranteed to lose, yet §S7's threshold admits it on entry count alone.

**Measured.** `arith/float_mul/1000` — `1.0 1.000001 * …`, registered as a word
and entered on a warm interpreter — against the same fixture with the feature
off:

| run | change |
|---|---|
| 1 | **+24.63%** |
| 2 | **+22.66%** |
| 3 | **+26.43%** |

all p = 0.00, against a suite floor of ±1.5%. `bund2 --stats` attributes it:
**2 bodies compiled, 0 sites inlined, 0 values promoted.** The body compiles and
gains nothing.

**The contrast is the proof.** The same program measured *cold* — as a
straight-line stream rather than a word — moves +1.0% to +1.2%, because a stream
is never a body and never compiles (§S3). The regression appears exactly when
the body is compiled. And `arith/int_add/1000`, identical in shape but on ints,
reads **1000 sites inlined, 1001 values promoted** and runs 46× faster. Same
harness, same length, opposite outcome, and the attribution separates them.

**Why floats get nothing.** D66 promotes int literals — `Plan::Literal` carries
an `i64` — and §S6's published fragment table (`bund2_stdlib::fragments`) has no
arm for float multiplication. So every value of a float body plans as a generic
call. Nothing here is wrong with the lowering; the defect is that a body it
cannot help is compiled regardless.

**The shape of a fix, not taken.** `Compiler::plan_body` already knows the
answer before any code is emitted: it returns the plan and the sites. A body
whose plan admits no site and promotes nothing could be refused there, or by
`JitTier::enter` on the counts, leaving it interpreted — which is what §S7's
function cap already does for a body past the cap, so the machinery for
"compiled code is not available, interpret" exists and is exercised. That would
also stop the body consuming one of §S7's 1024 function slots to no purpose.

**This is not F132 returning.** F132 proposed refusing compilation by body
*size*, was withdrawn when the measurement behind it turned out to be a
cold-start artifact, and would have refused the bodies that gain most. This
refuses by *attribution* — zero sites, zero promoted — which is measured per
body, already computed, and is the direct statement of "the tier cannot help
this body".

### Fixed, 2026-09-15 — the tier refuses a body it cannot help

Three pieces, each where it belongs.

**The answer comes from the planner.** `Compiler::would_gain`
(`crates/bund2-jit/src/lower.rs`) runs `plan_body` and reports whether the plan
holds any inlined site or any promoted literal. It is the *same* planning that
decides what gets emitted, so the rule cannot drift from the lowering it guards
— which is how F127 went wrong, with a fixture that merely resembled the real
path.

**The decision belongs to the tier.** `JitTier::enter`
(`crates/bund2-runtime/src/tier.rs`) asks before compiling and leaves the body
interpreted when the answer is no. Putting the refusal in `compile_word` would
have been wrong twice over: §S7's policy lives in the tier, and the lowering's
own tests deliberately compile site-free bodies through an empty fragment table.

**The refusal is recorded, not re-decided.** `Tiering::demote`
(`crates/bund2-jit/src/cache.rs`) marks the body permanently, reusing the
demotion `redefined` already had. Without it the counter would stay hot and
every later entry would re-plan the body to reach the same answer, paying
`plan_body` forever to learn what was known. It also keeps a useless body out of
§S7's 1024 function slots.

**Measured, three runs each side against one feature-off baseline**, same
session and machine:

| row | before | after |
|---|---|---|
| `arith/float_mul/1000` warm | +24.63%, +22.66%, +26.43% | **−0.26% (p = 0.18), −2.33%, −3.11%** |
| `arith/int_add/1000` warm | −97.84%, −97.85%, −97.87% | −97.91%, −97.90%, −97.91% |
| `arith/times_body/1000` warm | −0.53%, +0.58%, −0.89% | +0.29% (p = 0.34), −0.94%, −1.17% |

**`--stats` on the same build attributes it at the source**: the float body now
reads **0 bodies compiled** where it read 2, and the int body is untouched at
**1 body, 1000 sites, 1001 promoted**. The rule refuses exactly what it was
meant to and leaves everything else alone.

**One reading is not claimed as a win.** `float_mul` warm now measures −2.33%
and −3.11% on two of three runs, and a body that is no longer compiled should
read ~0%, not faster than Tier 0. The feature-on and feature-off binaries differ
in ways unrelated to this body and the suite's floor is ±1.5%, but −3.11% sits
outside that, so it is recorded as measured and unexplained rather than
described as an improvement.

Three tests pin the rule, in `crates/bund2-runtime/src/tier.rs`:
`a_body_with_nothing_to_gain_is_not_compiled` (a float body stays at Tier 0),
`a_body_with_a_site_still_compiles` (the other direction, so the rule cannot
"fix" the regression by turning the tier off), and
`a_refused_body_is_demoted_rather_than_re_planned` (forty entries, still zero
compiled). The first uses `inlining_runtime_with`, not `runtime_with`, because
the latter installs an empty table under which *every* body has zero sites and
the test would pass for the wrong reason.

Conformance unmoved: **CONFORMANCE 106/114** with 8 approved deviations,
**CEILING 106/114**, 0 goldens still to reach it, nothing failing.

- Found: 2026-09-15, restating criterion 7's `arith` under D69
- Status: **RESOLVED**, 2026-09-15. The regression is gone and the attribution
  confirms the mechanism rather than only the timing.
- Depends on: §S6 (the fragment table and the inlining join), §S7 (the
  threshold), D66 (int literals promote), D69

## F132 — WITHDRAWN: the benchmark rebuilt the interpreter, so every timed entry ran cold

**Not a Bund2 defect. A defect in the benchmark that found it**, and the fifth
of that kind after F124, F127, F128 and F129.

### What was claimed

That entering a compiled body costs **more** than interpreting it on a small
body: `compile`'s third arm against its second on `1 2 + drop`, +4% across four
runs, positive every time. A size sweep then appeared to make it worse with
length — `size_crossover` read +1.4% at 2 values rising to **+41%** at 64, and
`--stats` confirmed the lowering was working on exactly those bodies (32 inlined
sites, 32 promoted values at v64). On that reading the tier lost at every size
and lost hardest where it did most work, which would have put criterion 10 and
§S1's gate in question.

### Why it was wrong

Both groups pass `interp` to `iter_batched` as its **setup**, and `interp()`
builds a whole `bund2_runtime::Runtime` — `register_all`, a fresh `Interp`, and
with the feature a fresh `JITModule`. Setup is not timed, but its *effects* are:
every timed entry then ran against cold data caches and, on the tier side,
compiled code no instruction cache had seen, freshly emitted microseconds
earlier. Tier 0's interpreter loop is the same hot code on every iteration and
pays none of that.

The absolute numbers say it plainly. One entry of the same two-value body reads
**9.7 µs** under `size_crossover` and **66.9 ns** under `hot_body`, which warms
one `Interp` and keeps it — a factor of **147**. The measurement was dominated
by a pedestal 147× the size of the thing being compared.

### What is true instead

`hot_body` is criterion 10's shape: one `Interp`, warmed past the threshold
once, then entered repeatedly. Feature-off baseline against three feature-on
runs, `compiled bodies` reported as 0 and 1 respectively:

| body | Tier 0 | with the tier | three runs | |
|---|---|---|---|---|
| `1 drop` (2 values) | 66.87 ns | 50.81 ns | −24.6%, −25.5%, −24.6% | **1.32×** |
| ×4 (8 values) | 183.97 ns | 89.22 ns | −51.9%, −52.1%, −50.6% | **2.06×** |
| ×32 (64 values) | 1.2014 µs | 501.3 ns | −58.3%, −58.3%, −57.9% | **2.40×** |

All p = 0.00. **The slope reverses**: cold, the loss grew with body length;
hot, the gain does — 18.8 ns per value interpreted against 7.8 ns compiled at
64 values. That is the tier doing what it was built to do, and it agrees in
direction with criterion 10 rather than contradicting it.

### What this does not excuse

The claim survived four runs of `compile` and three of `size_crossover`, all
consistent, all p = 0.00. **Repetition did not catch it**, because a systematic
harness fault reproduces perfectly — this session had just finished using
repetition to withdraw `dispatch`'s failure, and that success did not transfer.
What caught it was a figure that made no sense against a known one: 0.93× per
entry against criterion 10's 1.06× per program, two measurements of the same
tier disagreeing in sign. Reconciling those, rather than trusting the newer one,
is what found the pedestal.

**F130's compile time is not withdrawn, and the qualification first written
here has itself been corrected by measurement.** This entry said the magnitude
"may be inflated by the same cold-start effect" and should be re-taken warm. It
was, the same day: `compile_warm` drives the compiler directly on a warm module
and reads **140.6–141.4 µs**, *higher* than the cold subtraction's
125.4–126.2 µs. So the pedestal did not distort that figure — a difference of
two identically-cold arms cancels it — and the suspicion recorded here was
wrong. See F130's *Re-taken warm*.

- Found: 2026-09-15, in the `compile` group written for F130
- Withdrawn: 2026-09-15, by `hot_body` and the reconciliation against
  criterion 10
- Status: **RESOLVED — withdrawn.** No rule was added to §S7 and none is
  needed. A minimum-body-size rule was the fix under consideration, and on the
  real numbers it would have been precisely backwards: it would have refused
  compilation to the bodies that gain **2.40×**.
- Depends on: §S7 (the threshold), F130, criterion 10

## F131 — an installed tier that compiles nothing still costs ~18% — RESOLVED

**A Bund2 defect**, found while decomposing F130 with the `BUND2_JIT_THRESHOLD`
knob.

Set the threshold to 1,000,000 and the tier is installed, consulted on every
entry, and compiles nothing. `arith/times_body/1000` — a program with a
thousand body entries — then reads **~85.4 µs against a 72.596 µs no-tier
baseline**, +17.2% and +19.7% on two quiet runs. Nothing has been compiled and
nothing has been run compiled; the whole of it is `Tiering::observe` being
asked, per entry, whether this body is known.

That is roughly **13 ns per entry**, against ~22 ns for a whole interpreted
word — so the question "should this run compiled?" costs over half of what
running it interpreted costs, and is paid by every body every time, including
the overwhelming majority that never reach the threshold.

**It is charged to programs that can never benefit.** A body entered once pays
the probe once and is never compiled. The cost falls hardest on exactly the
shape the tier cannot help.

Recorded here rather than in F130 because it is independent of compilation: it
is present when the tier is installed and no compilation ever occurs, which is
the configuration in which it was measured.

### Fixed, 2026-09-15 — ~15 ns per entry down to ~1 ns

Four changes in `Tiering::observe` (`crates/bund2-jit/src/cache.rs`), none of
which alters a decision the cache returns:

1. **`payload_weak` is taken only where an entry is filed.** It was taken beside
   the key on every entry: `Rc::downgrade` writes the weak count, and on every
   path but the first sight of a body the `Weak` is dropped a few lines later,
   writing it again. `payload_weak` answers `Some` for exactly the values
   `payload_key` does — both are the `Heap` arm — so nothing counted before is
   missed now.
2. **The counter's cap test precedes its membership probe.** Written
   `!contains_key(&key) && len() >= cap`, Rust evaluates the probe first, so
   every entry paid a full lookup to guard a condition false until the counter
   fills. The length compare answers it for nothing.
3. **`demoted` and `cache` are tested for emptiness before being hashed.**
   Demotion is rare and the cache is empty until something compiles, which is
   the state every body below the threshold is looked up in.
4. **The three pointer-keyed maps use `AddrHasher`**, a Fibonacci multiply, in
   place of SipHash. D35 fixes the *key* — the payload address — and says
   nothing about how a map hashes it, so this is an implementation change and
   not a decision. The high half is folded down in `finish` because a map wants
   bits at both ends, and an unmixed pointer would be worse than SipHash rather
   than better: allocations are aligned, so their low bits are constant.

**Measured, three runs each side in one session on one machine**, in the
configuration this defect was found in — threshold 1,000,000, so the tier is
installed, consulted on every entry, and compiles nothing — against the same
72.596 µs no-tier baseline:

| | runs | absolute | over baseline |
|---|---|---|---|
| before | +19.98%, +20.71%, +20.28% | ~87.5 µs | ~14.9 µs / 1000 entries |
| after | +1.80%, +1.70%, +1.45% | ~73.7 µs | **~1.1 µs / 1000 entries** |

**~15 ns per entry to ~1 ns.** Two controls: compilation is unchanged at
137.31 and 138.47 µs (`compile/crossing_entry`, against 133.87–140.01 before),
so the entry path moved and the compiler did not; and `arith/times_body` at the
default threshold went from ~+121% to +119.19%, +118.29%, +120.11% — an
improvement of about the share F131 was contributing, which is what the
decomposition predicted and is the honest size of it. F130 is untouched, as it
must be: compilation is ~85% of that figure and this defect was never part of it.

**The residue is real and is not claimed away.** +1.45% to +1.80% at p = 0.00,
positive in every run and outside the suite's ±1.5% floor. An installed tier
still costs ~1 ns per entry, which is the map probe that genuinely has to
happen. Driving it to zero means not probing at all for bodies that cannot
benefit, and that is F132's question about §S7, not this one.

Two tests pin what the changes could have broken:
`a_counted_body_is_still_swept_when_its_body_dies` — the lazy downgrade must
leave the counter holding a `Weak`, or the map would key on an address the
allocator could reuse, which is §S3's false hit — and
`the_address_hasher_spreads_aligned_neighbours`, which asserts aligned
neighbours land in different buckets rather than asserting the speed.

- Found: 2026-09-15, decomposing F130 by threshold
- Status: **RESOLVED**, 2026-09-15. ~93% of the cost removed; the ~1 ns residue
  is recorded above rather than rounded to nothing. **Conformance unmoved and at
  its ceiling** — `CONFORMANCE 106/114` with 8 approved deviations,
  `CEILING 106/114`, 0 goldens still to reach it, nothing failing. This is the
  invariant the tier exists under: it changes speed, not meaning, so any
  movement here would have been a bug rather than a result.
  (`tests/golden/CONFORMANCE.txt` still records the older 105/113 high-water
  mark; `conform --accept` is the repository owner's to run.)
- Depends on: D35 (the payload-pointer key), §S7 (the threshold), F130

## F130 — a body compiled once per iteration costs +121%, and the cause is one Cranelift compilation

**A Bund2 defect**, found by running RFC-0005 criterion 7 for the first time
(F129 is why it could not be run before).

**The measurement, narrowed to what reproduces.** `arith/times_body/1000` —
`1000 { 1 + } times drop` — regresses **~+121%**, and that is the whole of the
defect. Five consecutive feature-on runs against one 72.596 µs baseline on
2026-09-15: **+116.87%, +121.82%, +117.83%, +127.46%, +121.08%**, every one
p = 0.00. `bund2 --stats` confirms the body compiles (1 body, 1 inlined site,
1 promoted value).

**The `dispatch` half of this entry is withdrawn.** It claimed
`dup_drop/w3000` +9.96%, `native_call/w4000` +12.62%, `literal_push/w2000`
+11.94%, with `literal_only/w1000` at +3.82% as a control that stayed put while
its siblings moved. Each was one run. Three runs against one baseline read
+2–3% on every program in the quiet two and +7–12% on all six at once in the
third — a whole-run excursion. **The control argument was false**:
`literal_only` moves with its siblings every time. And none of these programs
compiles a body even at `--jit-threshold 1`, because a straight-line stream
never reaches `push_frame` and the tier is consulted only for a body with a
payload key — so there was never a mechanism for the cost this entry attributed
to them.

**This suite labels noise "significant", which is how that happened.** Three
feature-**off** runs of one binary against its own baseline: `dup_drop` −0.28%
(p = 0.57), +0.06% (p = 0.88), −0.85% (p = 0.05); `native_call` **−1.02% at
p = 0.00**, then −0.51%, −0.02%. The floor is ~±1.5% and p < 0.05 occurs inside
it with nothing changed.

**That ±1.5% is a µs-scale figure and does not generalise** (F135). It was taken
on `dispatch/dup_drop` at 88 µs. On rows of tens of nanoseconds the same suite's
cross-process spread was measured at ~5–8% — **and that figure is superseded**:
on an idle host the same row resolves to **0.7%**, within a **1.7%** thermal
envelope under sustained load. The floor must still be read at the magnitude it
was taken at rather than as a property of the harness, but at ns scale the
binding constraint is the **host's state**, not the instrument. A single run within the band is not evidence, and this
entry recorded four of them as a failure.

### The first diagnosis was wrong, and is withdrawn

This entry previously claimed the cost was **three `HashMap` probes and a
`Weak` upgrade per entry** — `payload_key` and `payload_weak` in
`Tiering::observe`, then `Tiering::compiled` recomputing the key and probing
the cache again from `Decision::Compiled`. The arithmetic offered was
"~93 ns × 1000 entries". **That was fitted to the number, not derived, and it
is false.**

Two errors produced it.

**The evidence benchmark measures compilation, not entry.** `timed_eval`
(`crates/bund2-bench/benches/interpret.rs`) passes `interp` as
`iter_batched`'s *setup* closure, so **every Criterion iteration builds a fresh
`Interp` with an empty cache**. One eval of `1000 { 1 + } times drop` crosses
§S7's threshold of 64 within itself, so each iteration pays 64 interpreted
entries, **one full Cranelift compilation**, and ~936 compiled entries. The
+136% is a sum of those three, and attributing all of it to per-entry cost was
unjustified.

**The fix that followed changed nothing.** `Decision::Compiled` now carries the
`WordHandle`, removing the second key computation and the third probe. Measured
fairly — both binaries built at the same release profile in one session,
19,306,448 against 19,306,256 bytes — it is **neutral**:

| | 20k entries | 80k entries |
|---|---|---|
| Tier 0, no feature | 0.0055 s | 0.0092 s |
| pre-fix | 0.0057 s | 0.0085 s |
| post-fix | 0.0052 s | 0.0081 s |

and Criterion's `times_body` moved +141.89% → +136.56%, inside its own spread.
So the per-entry path was not the bottleneck.

**Two intermediate readings were also wrong and are recorded so they are not
repeated.** A "~5× improvement" compared a 19 MB release binary against an
80 MB one from a different build profile; a "3.5× faster than Tier 0" used a
67 MB stale Tier 0 artefact. Both were CLI medians on a ~2.3 ms process floor.
Stale binaries in a scratch directory are indistinguishable from fresh ones by
name alone, and the size is the tell.

### The cause, measured directly

**Compilation is the cost, and it is of order 125–141 µs per body** — two
shapes sharing no harness, bracketed in *Re-taken warm* below; the ~128 µs this
section first recorded is the lower shape's figure. The `compile` group in
`crates/bund2-bench/benches/interpret.rs` times the single entry that crosses
§S7's threshold against the identical entry one before it, with all warming in
`iter_batched`'s untimed setup. The body is `1 2 + drop` — one frame per call,
no inner loop, so entries equal calls and the threshold arithmetic is exact.
Four runs with the tier on:

| arm | run 1 | run 2 | run 3 | run 4 |
|---|---|---|---|---|
| `crossing_entry` — compiles | 140.01 µs | 137.64 µs | 137.94 µs | 133.87 µs |
| `ordinary_entry` — does not | 9.85 µs | 9.83 µs | 9.56 µs | 9.24 µs |
| `compiled_entry` — steady state | 10.56 µs | 10.31 µs | 9.89 µs | 9.96 µs |

The difference is **124.6–130.2 µs**, one Cranelift compilation, about **13×**
a whole interpreted entry of the same body.

**The no-tier control is what makes it compilation rather than setup.** The
three arms warm different amounts (`t-1`, `t-2`, `t+8`), so they leave different
heap state. With the feature off and no tier to compile anything, all three sit
at **9.2–9.8 µs** and `crossing_entry` is indistinguishable from its siblings.
The difference appears only when a compilation happens. The benchmark prints its
premise every run — `crossing entry 0 -> 1 bodies … ordinary entry -> 0` with
the tier, `0 -> 0` without — so this is not another F129.

### Re-taken warm, 2026-09-15 — the figure holds, and was not a cold-start artifact

F132's withdrawal cast doubt on every number taken through `iter_batched`'s
setup, this one included, so it was re-taken in a shape that shares none of that
harness. `compile_warm` drives `Compiler::compile_word` directly with the
compiler **reused across iterations**, so the module, its tables and the
allocator are warm; the fragment table comes from
`bund2_stdlib::fragments::published`, the call `bund2-runtime`'s tier itself
makes, rather than one assembled in the benchmark.

| shape | per compilation of `1 2 + drop` |
|---|---|
| warm, direct (`compile_warm`), three runs | **140.64, 141.38, 140.70 µs** |
| cold, by subtraction (`compile`), two runs same session | **125.35, 126.23 µs** |

**The cold figure was not inflated.** It is if anything ~12% *lower* than the
warm one, so the pedestal that sank F132 did not distort this measurement — the
difference of two identically-cold arms cancels it, as the entry argued it
would.

**A batch-depth sweep refutes the obvious objection.** Every compilation in a
batch adds a function to the same `JITModule`, so a deep batch would report
inflated times if finalisation cost grew with what the module already holds.
It does not: 142.02, 141.80, 141.70, 140.74 µs at depths 8, 32, 128 and 256,
where the module holds 16 functions at the shallow end and 264 at the deep one.
The knob stays in the benchmark (`BUND2_BENCH_COMPILE_BATCH`) because the
question will be asked again.

**The 12% gap between the two shapes is unexplained and is left that way.** The
cold subtraction covers compilation *and* the cache insert, so it should be the
larger of the two and is the smaller. Candidate causes — a difference in what
each shape hands `compile_word`, or in the interpreter state `plan_body`
consults — were not tested, and this entry will not name one it has not
verified: that is exactly how its first diagnosis went wrong. What both shapes
agree on is the magnitude: **a compilation of a four-value body costs of order
125–141 µs**, three orders above an interpreted entry of the same body.

**Per-entry cost is bounded below ~1.4 ns and is not distinguishable from
zero** (measured 2026-09-16; the wording below is the third attempt at this
quantity and the first that anchors it).

`hot_body`'s six-length sweep gives the per-entry cost as a regression
*intercept*, and an intercept is an extrapolation to zero values with nothing
near zero holding it down. Two Tier 0 blocks six minutes apart, **both `CLEAN`
and agreeing row-by-row to within 2%**, fitted intercepts of **35.92 ns and
27.29 ns**; a later run of the same shape read the tier-minus-Tier 0 difference
as **+6.28 ns**, where the pre-restart sweep had read **−0.57 ns**. All three
were noise in an unconstrained parameter, not measurements of the tier.

`entry_anchored` measures it on anchored points instead. Bodies are int
literals, one to sixteen — literals because **F133's rule refuses a body with no
site and nothing to promote**, so the obvious balanced choice (a body of `clear`
calls) compiles 0 bodies, checked before the fixture was trusted. `eval_empty`
times `eval` of an empty stream and reads **2.6700, 2.6674, 2.6668, 2.6711 ns**
across four blocks, a 0.16% spread:

| | `call/v1 − eval_empty` |
|---|---|
| Tier 0 | 66.10, 64.71 ns (mean **65.41**) |
| with the tier | 66.00, 66.42 ns (mean **66.21**) |
| **delta** | **+0.80 ns**, against Tier 0's own 1.39 ns spread at v1 |

Anchored, the two arms' fitted intercepts agree to **−0.08 ns** (61.82 against
61.74) where unanchored they had disagreed by 8.6 ns. The three-point fit that
first reported "~7 ns of fixed entry cost" and the six-point fits that reported
−0.57 and +6.28 ns are all withdrawn: the quantity is **under ~1.4 ns and
indistinguishable from zero**, which is a bound rather than a value.

**And the per-value cost is three numbers, not one** (2026-09-16). Decomposed by
regime: a promoted literal saves **1.13 ns**, a generic call **costs 5.13 ns**,
an inlined site saves **24.19 ns**. The boundary this entry discusses is that
+5.13 ns — `jit_apply` runs `Vm::apply`, Tier 0's own path, with a slot load, a
`call_indirect`, the request-cell load and the status protocol around it. The
tier's advantage is dispatch removal and nothing else. F136 files what that
implies for F133's rule; criterion 10 carries the table.

**Per-entry cost was excluded first, by the `entry` group.** It warms past the
threshold and times entries only: +1.74%, +3.74%, −0.05% (p = 0.78), −0.65%,
+1.94% across five runs, straddling zero. That is what left compilation as the
only remaining term.

**The decomposition of `times_body`'s +121%,** by the `BUND2_JIT_THRESHOLD`
knob F125 added, against the 72.596 µs no-tier baseline:

| configuration | time | share |
|---|---|---|
| threshold 1,000,000 — tier installed, **never compiles** | ~85.4 µs | +18% — `observe`'s probe, see F131 |
| threshold 64 (default) — one compilation per iteration | ~159 µs | the remaining ~+101% |
| threshold 1 — compiles at the first entry | ~154–158 µs | same, as it must be: also one compilation |

Threshold 1 and threshold 64 agree because both pay exactly one compilation;
only the interpreted-to-compiled split differs, and that split is nearly free.
So **compilation is roughly 85% of criterion 7's `arith` failure**, and the
benchmark pays it **once per Criterion iteration** because `timed_eval` rebuilds
the `Interp` in setup. No session recompiles a body on every call.

**What this means for the criterion, stated rather than acted on.** The `arith`
figure is substantially a property of how the benchmark is written, not of the
tier in use. That is an argument for saying what `arith` measures — not for
changing the tier — but it is a decision about a criterion's meaning and it is
not taken here. Two real costs did surface on the way and are filed separately:
F131 (the probe on every entry) and F132 (compiled entry is dearer than
interpreted on a small body).

**What this is not.** Not a defect in the lowering: it computes correctly,
inlines behind §S6's guards, and promotes as §S5 specifies. Conformance is
unmoved at 106/114 across all three configurations.

**The handle-carrying change is kept, on correctness grounds and not
performance.** `observe` used to answer `Decision::Compiled` on a bare
`contains_key` while the liveness test happened afterwards in
`Tiering::compiled`, so a body whose payload had been dropped reported
`Compiled` and then quietly did not run compiled — `JitTier::enter`'s `?`
swallowed it into interpretation. One lookup now decides both, which is what
D35 as amended by Q32 always meant.

- Found: 2026-09-14, running criterion 7 on the fixed harness
- Narrowed: 2026-09-15. Scope went from four benchmarks to one: the `dispatch`
  rows are withdrawn as unreproducible, and the per-entry path is excluded by
  the `entry` group rather than merely unproven. One suspect remains —
  compilation inside the timed region.
- Cause established: 2026-09-15. One Cranelift compilation, **of order
  125–141 µs**, measured two ways that share no harness — by subtraction
  through the tier (125.4, 126.2 µs) and directly on a warm reused compiler
  (140.6–141.4 µs) — with a no-tier control and a batch-depth sweep behind
  them. It is ~85% of the figure; the rest is F131's probe. The ~12% spread
  between the two shapes is recorded above as unexplained.
- Status: **OPEN — the cause is known, the disposition is not.** Two diagnoses
  were offered and both are withdrawn above: the per-entry `HashMap` arithmetic
  (falsified by a neutral fix) and per-entry cost in general (refuted by the
  `entry` group). What remains is a question about criterion 7's meaning rather
  than a fault in the tier — a benchmark that recompiles per iteration measures
  something no session does — and that is the owner's decision, not a change to
  make quietly. F131 and F132 carry the two real costs found on the way.
- Depends on: D35 (the payload-pointer key), §S7 (the threshold), F129

## F129 — `bund2-bench` built a bare `Interp`, so criterion 7's A/B measured no tier

**A Bund2 defect, in the benchmark harness** — the third instance of one
pattern, after F124 and F127.

RFC-0005 criterion 7 is an A/B across the `jit` feature. Every group in
`crates/bund2-bench` built its interpreter with `Interp::new` plus
`register_all`, and **only `bund2_runtime::Runtime` installs a tier**. So
`--features jit` changed the binary and changed nothing the benchmarks
executed.

The first run of the criterion reported **26 "regressions"** — in `boxing`,
which never touches an `Interp`; in `value/clone/scalar`, which is
allocation-free; in `startup/parse/mixed`, which is `bund2_syntax::compile`
alone. In the same run `value/push_pull/balanced` reported *no significant
difference*. Both cannot be true of one tier, and neither was: the two halves
ran under different machine load, and the feature was inert.

**Disposition: FIXED.** `interp()` is feature-gated — under `jit` it returns
`Runtime::new().interp`, otherwise the previous construction — so `timed_eval`,
and with it `dispatch`, `arith`, `corpus`, `lambda` and `rendering`, route
through a real tier. The two arms register the same vocabulary, since
`register_all` *is* `register_all_with(r, &HostOptions::default())`, which is
what `Runtime::with_options` calls; if they differed the A/B would compare two
word tables rather than two tiers.

`startup` deliberately keeps its bare `Interp`: it times `register_all`, the
fixed cost the criterion forbids the tier to move, and routing it through
`Runtime` would fold tier construction into the number being protected.

**And the benchmark now says which half it is.** `say_whether_the_tier_is_installed`
prints `bench: tier installed (threshold …)` or a warning naming this defect,
above every number in the run. A benchmark cannot assert, so it reports — and
an A/B whose feature-on half reached no tier is not a passing criterion, it is
the absence of a measurement that looks exactly like one.

**The pattern, three times.** F124: the CLI never installed a tier, so every
`--features jit` measurement compared Tier 0 with itself. F127: the lowering's
differential helper used an empty fragment table, so no test through it reached
an inlined site. F129: the benchmarks built a bare `Interp`. **None was found by
reading the code** — each was found by trying to take a measurement and asking
what it had actually touched.

- Found: 2026-09-14, running criterion 7
- Status: **RESOLVED — fixed.**
- Depends on: F124 (the same defect in the CLI), criterion 7

## F128 — criterion 21's six programs passed while testing nothing, because `ensure_stack` left the wrong stack current

**A Bund2 test defect**, found by probing what the tests reached rather than by
reading them, and recorded because it is the *third* instance of one pattern.

Criterion 21's programs each push two literals, switch the current stack, and
then do something that can only be right if the switch was seen. The fixture's
setup line was `:s ensure_stack`. But `Interp::ensure_stack` calls
`add_as_current` for a name not yet in the ring, so **`s` was already current
before the body ran** — and every switch inside every body was a switch to the
stack already current. A no-op.

All six passed. Four of them recorded answers (`3` on `s`) that the corrected
setup shows to be **failures**: with the body genuinely starting on `main`, the
two literals stay there, the switch makes an empty stack current, and `+` fails
with `Stack is too shallow for inline ADD()`. That failure is §S5's own
sixth-review example, and it is the case the criterion exists to protect — a
lowering that synced promoted values to the stack current *at the sync* rather
than to the stack each value came from would add `1` and `2` and push `3`,
turning a failure into an answer. A criterion whose programs all succeeded
could not catch it.

**Disposition: FIXED.** The fixture returns to `main` after the setup
(`:s ensure_stack :main to_stack`) and **asserts that it did**, on both the
tiered and the untiered runtime, so a later change to the setup cannot restore
the no-op silently. The six Tier 0 answers were re-derived under the corrected
setup and the RFC's table replaced.

**The pattern, three times in one area.** F124: the CLI never installed a tier,
so every `--features jit` measurement compared Tier 0 with itself. F127: the
lowering's differential helper compiled with an empty fragment table, so no
test through it reached an inlined site. This one: the fixture left the body on
the stack it was about to switch to. Each looked like it exercised a feature,
none did, and **none was found by reading the test** — F124 by trying to take a
measurement, F127 and F128 by instrumenting the fixture to print what it
actually reached. The lesson is narrow enough to act on: a test that asserts a
*path* must assert that the path was taken, in the test, not in a comment.
`inlined_sites`, `promoted_values` and the current-stack guard are those
assertions here.

## F127 — the lowering's differential helper compiled with an empty fragment table, so no test routed through it reached an inlined site

**A Bund2 test defect**, found while building §S5's residual (D67) and recorded
because of what it silently withheld rather than what it broke.

`assert_matches_tier0` (`crates/bund2-jit/src/lower.rs`) builds its compiler
with `Compiler::new(Vec::new())`. An empty table publishes no fragment, so
`plan_body` inlines nothing, and **every body compared through that helper was
compiled with zero sites**. The helper is the differential the lowering's tests
lean on, so its coverage stopped at the generic path and literal promotion.

Four tests added the same day inherited it. The worst was
`a_rebound_name_syncs_the_operands_it_promoted`: it rebinds `+` after
compiling, and its name claims it forces the meaning guard to refuse and the
residual to run. With no fragment for `+` there was no site, no generation
guard, and no residual — the test asserted a path it could not take, and
passed. A test that names a path it never reaches is worse than an absent one,
because it is counted as coverage.

**Disposition: FIXED.** The rebound-name test now compiles with
`with_fragments()` and asserts `inlined_sites(word) == Some(1)` *before*
rebinding, so the site must exist or the test fails rather than passing
vacuously. Three tests were added against the same fixture — chaining, the
`autoadd` residual, and the resume table — each asserting its site count.
`assert_matches_tier0` keeps its empty table on purpose: it is the
generic-path differential, and `without_a_table_the_same_body_takes_the_generic_path`
is the test that pins that reading.

**The same shape, one layer up, on the same day.** `bund2-runtime`'s tier tests
all used `runtime_with`, which installs `JitTier::new` — documented in its own
source as "a tier that inlines nothing". So no test in that crate had ever
exercised inlining through the seam either; `inlining_runtime_with` is the
fixture that does, and it asserts its table is non-empty. This is F124's
lesson repeating: F124 was the CLI never installing a tier, so every
`--features jit` measurement compared Tier 0 with itself. Both were fixtures
that looked like they exercised a feature and did not, and both were found by
trying to *measure* rather than by reading.

## F126 — promotion baked a body's literals into code that fetches every other value at run time

**A Bund2 defect**, introduced and caught within one session while building
D66's promotion, and recorded because the way it was caught is the point.

`Plan::Literal` lowered an int literal to an `iconst` carrying the constant
*seen while planning*. But a compiled word does not own its body: every other
value is fetched from `Ctx::body` at run time, by index, in `jit_apply`
(`crates/bund2-jit/src/lower.rs`). So a word is otherwise indifferent to which
body of the right length it is handed, and the test helper `word_in` relied on
exactly that — it compiled `0 1 2` and let callers run some *other* three-value
body through the result.

The symptom was a compiled body that pushed 0, 1, 2 and ignored the `1 2 +` it
was given: depth 3 where 1 was expected, and depth 1 where 3 was. Eight tests
failed at once.

**Why it was nevertheless sound in production, and why that is not enough.**
`Tiering`'s cache keys on `BundValue::payload_key` — the payload's `Rc` pointer,
D35's option 4 — and `Tiering::compiled` rejects a dead entry through its
`Weak`, so a cache hit names the same payload allocation, and a lambda's payload
*is* its body. Every real caller therefore hands a compiled word the identical
values it was compiled from. The assumption was true; it was never *checked*,
and it is an invariant belonging to a different crate, which nothing in the
lowering would notice changing. The failure mode if it did change is a wrong
number pushed silently, not an error.

**Disposition: FIXED.** `Word::literals` records each baked `(index, constant)`
pair and `Compiler::run` compares them against the body it is handed, refusing a
mismatch as `Error::internal` — D37's third way out, a named broken invariant
rather than a silent wrong answer. `word_in` now takes the body it compiles.

**What this says about the method.** `jit_apply` was read in the same session,
before the promotion was written, and it is the single function that defines
the contract the design broke. The tests caught it; the reasoning did not. It
is the shape CLAUDE.md's *follow the call one level further* warns about,
applied to a helper rather than to the reference.

## F125 — `--jit-threshold` is specified in three places and built in none

**A Bund2 defect**, the sibling of F124 and found the same way: by trying to
take a measurement the RFC already asks for.

RFC-0005 names the flag three times, and criterion 2 depends on it:

- criterion 2's second `jit` run is `cargo xtask conform --features jit
  --jit-threshold 1`, "so every body compiles on its first evaluation. That is
  the strongest test of meaning the corpus can give, and it is why §S7 makes
  the threshold configurable and recorded";
- §S5's exit passage reasons about a program's behaviour "under
  `--jit-threshold 1`";
- a dated note of 2026-09-11 says the flag "arrives with the tier".

The tier has arrived. The flag has not: `threshold` does not appear in
`crates/bund2-cli/src/main.rs`, and `Caps::default` fixes it at 64 with no way
to override it from a run.

**What it costs.** Criterion 2's strongest configuration cannot be run, so the
corpus is only ever exercised at the default threshold — where most programs
are too short to compile anything. Measured today: a loop of 200 iterations
compiles **0** bodies; 1000 compiles 2. So "the same N/M with the feature on
and off" is currently a comparison in which the feature does very little over
most of the corpus, which is the weaker half of what criterion 2 intends.

It also blocks the honest form of criterion 10's A/B over the corpus rather
than over hand-written kernels.

**Disposition: FIXED, 2026-09-14.** Two knobs, with a stated precedence:

- `--jit-threshold <n>` on `bund2 script`, parsed in `parse_args` and threaded
  through `Runtime::with_options_and_threshold` into `Caps`;
- `BUND2_JIT_THRESHOLD` in the environment, read by `threshold_from_env`;
- **the flag wins**, because it is the more specific statement: a command line
  is about *this* run, an environment variable about the shell it ran in.
  Neither set falls back to §S7's default of 64.

`xtask` gained `take_jit_threshold` beside `take_features`, `golden::run_once`
gained an `extra` argument list — **empty for every oracle run**, since the
oracle has never heard of the flag and passing it would turn a capture into a
refusal — and `conform` threads it through and names it in the `measured:`
line.

**A malformed value is refused on the command line and ignored in the
environment**, and the asymmetry is deliberate: `parse_args` can report and
exits **2**, while `threshold_from_env` runs inside a constructor that cannot.
So an ignored environment value would be a silent misconfiguration — the shape
of F124, F127 and F128 — and `bund2 --stats` therefore prints the threshold the
tier *actually adopted*, not the one that was asked for. `BUND2_JIT_THRESHOLD=banana`
reports `at threshold 64`.

A build without the `jit` feature accepts the flag and ignores it. Refusing it
would make a uniform `conform` command line fail on the run that has no tier to
configure.

**Criterion 2's third run, measured for the first time.** All three now read
the same number:

    cargo xtask conform                                    106/114, ceiling 106/114
    cargo xtask conform --features jit                     106/114, ceiling 106/114
    cargo xtask conform --features jit --jit-threshold 1   106/114, ceiling 106/114

At threshold 1 every body compiles on its first evaluation, which is the
strongest test of meaning the corpus can give — and it moves conformance by
exactly zero, which is what §S2 requires. The knob is a tuning parameter (§S7),
not a semantic boundary, and this is the first evidence for that rather than an
assertion of it.

- Found: 2026-09-14, while measuring criterion 10 on the shipped lowering
- Status: **RESOLVED — built.**
- Depends on: D59, criterion 2, §S7's knobs

## F124 — `bund2` never installs a tier, so every `--features jit` measurement compares Tier 0 with itself

**A Bund2 defect**, found while trying to benchmark the JIT on a fractal.

`bund2 script --features jit` runs **Tier 0**. The feature is enabled, the
binary differs, and nothing in it ever reaches Tier 1.

Four independent places say so:

- `crates/bund2-cli/src/main.rs`, `run` — builds the interpreter directly:
  `Interp::new()` then `register_all_with`, then evaluates. The word `tier`
  appears nowhere in the file outside a comment about `EVAL_STACK`;
- `crates/bund2-cli/Cargo.toml` — its `jit` feature was `["bund2/jit"]`,
  routed through the facade; and `crates/bund2/src/lib.rs` is **twelve lines of
  attributes with no re-exports**, so the feature enabled something on a crate
  whose code never runs;
- `Runtime::with_options` (`crates/bund2-runtime/src/lib.rs`) is the only code
  that installs a `JitTier`, builds §S6's fragment table and declares §S8's
  Tier 1 share — and **nothing outside tests constructs a `Runtime`**;
- RFC-0000's crate table, dated note of 2026-09-13, assigns exactly this job to
  `bund2-runtime`: it "owns the `Interp` and installs RFC-0005's `Tier`".

**What it invalidates.** `cargo xtask conform` takes `--features`, builds the
binary with them (`xtask/src/conform/mod.rs`, `take_features` and
`bund2_binary`), and criterion 2 compares N/M with the feature on and off. Both
sides are the same interpreter, so the criterion has been passing without
exercising a tier at all. The measured 106/114 is correct as a Tier 0 number and
says nothing about Tier 1.

Timing confirms it: a 300,000-iteration loop measures 0.057 s on the default
binary and 0.059 s with `--features jit` — a dead heat, because the same code
ran twice.

**Why nothing caught it.** Criterion 2 anticipated vacuity and asked for the
guard: "`bund2` reports how many bodies it compiled when asked, through a
statistics flag… A `jit` run that compiles no body over the corpus fails." That
flag was never built, so the only evidence available was timing — which is
exactly the evidence the criterion was written to replace.

**Disposition.** Two pieces, in order:

1. `run` constructs a `Runtime` rather than an `Interp`, so the tier is
   installed on the path every program takes. The CLI needs `eval_indexed` for
   its span-accurate error reporting, which `Runtime` does not expose, so either
   it gains that or the CLI reaches through `Runtime::interp`.
2. The statistics flag criterion 2 specifies, reporting compiled bodies, so a
   run that compiled nothing says so instead of being inferred.

**What it does not affect.** Conformance and coverage are unmoved — 106/114,
ceiling 106/114 — because Tier 0's behaviour is what they measure and it has not
changed. Every unit test of the tier is unaffected: those construct `Runtime`
or `Compiler` directly and do exercise Tier 1.

- Found: 2026-09-14, benchmarking a Julia set through the CLI
- Status: **RESOLVED — fixed 2026-09-14.**
- Depends on: D59 (Tier 1 authorised), D62 (the share), criterion 2

**The fix, and the evidence that it is one.** `run` constructs a
`bund2_runtime::Runtime` rather than an `Interp`, so the tier is installed on
the path every program takes; `bund2-cli` depends on `bund2-runtime` directly
and chains `jit`/`aot` through it rather than through the facade that
re-exports nothing. Criterion 2's statistics flag is built: `--stats` reports
compiled bodies **and** inlined sites, on stderr so no golden is disturbed.

The same program, the same flag, the two builds:

    bund2-jit      script … --stats  ->  tier compiled 2 bodies, inlined 3 sites
    bund2-default  script … --stats  ->  no tier (built without `jit`)

That distinction is what the defect was: before, both said the same thing by
doing the same thing. `conform --features jit` now reads 106/114, ceiling
106/114, with a tier genuinely installed — the invariant that a tier moves
meaning by zero, tested for the first time rather than passing vacuously.

**Its sibling stays open.** `--jit-threshold` is still unbuilt (F125), so
criterion 2's second `jit` run — every body compiled on first evaluation —
cannot be performed.

## F123 — `PROMOTABLE.txt` cannot satisfy both feature sets, so the palette audit fails in one of them

**A Bund2 defect**, in the audit rather than in the language. Found while
regenerating the list after RFC-0005's cell work.

`every_fixed_effect_native_keeps_its_pair_over_the_promotable_palette` compares
the palette's result against the file as a **symmetric set difference** and
asserts both halves are empty (`crates/bund2-stdlib/src/lib.rs`):

```rust
let gained: Vec<&String> = promotable.difference(&listed).collect();
let lost: Vec<&String> = listed.difference(&promotable).collect();
assert!(gained.is_empty() && lost.is_empty(), …);
```

Nothing in the audit is feature-aware: the palette is drawn from whatever
`register_all` bound, and `string.grok` and `string.grok.` are registered only
under the `grok` feature (`crates/bund2-stdlib/src/library_string.rs`, behind
`#[cfg(feature = "grok")]`; D10 and D40 keep it off by default).

So the file has no configuration that passes both:

- regenerated **without** `--all-features` — the state today, 279 lines, no
  `grok` entries — `cargo test --workspace --all-features` fails with
  `now reached ["string.grok", "string.grok."]`;
- regenerated **with** `--all-features`, the default build would fail the same
  assertion from the other side, with `no longer reached` naming the same two.

D48 says the list is regenerated and its diff reviewed, and gives the
regeneration command without a feature list; it never contemplated a
feature-gated native. The list is `tests/golden/`'s, so the fix is the
repository owner's to choose and not something to patch around.

**Dispositions, for the owner.** Each is a decision, not an edit:

- make the audit feature-aware — skip a native the current build did not
  register, comparing only what both can see;
- record the feature-gated names in the file as comments, which the comparison
  already ignores, so neither build counts them;
- own the file at `--all-features` and run the audit only there.

**What it does not affect.** Conformance and coverage are unmoved — the audit
is an honesty check on promotion's eligibility list, and promotion does not
exist yet, so nothing reads `PROMOTABLE.txt` at run time. The two `grok` words
are correctly promotable under the feature; the defect is that the file cannot
say so and stay true for the default build.

**Disposition: FIXED, 2026-09-14 — the owner took the first option, making the
audit feature-aware.**

`FEATURE_GATED` (`crates/bund2-stdlib/src/lib.rs`) names `string.grok` and
`string.grok.` and is filtered out where the native list is built, beside
`ACTS_ON_HOST` and in the same shape: kept by hand, so a new feature-gated
native is audited for real until it is named there, and conservative, since an
excluded native is unlisted and promotion syncs before it exactly as before an
embedder's — speed, never meaning. `grok` is the only feature this crate has,
and it binds exactly those two words.

**Excluded at the source rather than subtracted after the run.** Filtering
where the list is built means the audit never *claims* to have checked them.
Subtracting from both sides of the set difference afterwards would run them
under `--all-features` and then discard the result, which is a measurement
taken and thrown away.

**`tests/golden/PROMOTABLE.txt` was not regenerated, and did not need to be.**
Once the audit stopped drawing the gated pair, the existing **222** entries
were already correct for both builds — default passes, `--all-features`
passes, same file, untouched. That is the better outcome, since the file is the
owner's to own. The generator's header and a new `# feature-gated:` block will
record the exclusion in the file from its next regeneration onward, whenever
one is next warranted for some other reason.

**Verified both ways**: `cargo test --workspace` and `cargo test --workspace
--all-features` are both green, where the latter was the failing command this
entry was filed for. Clippy clean under both feature sets. Conformance is
unmoved, as predicted — the audit is an honesty check on promotion's
eligibility list and nothing reads the file at run time.

D48 carries a dated note recording the same, and RFC-0005's criterion 28 now
states **three** limits rather than two.

- Found: 2026-09-13, regenerating after the cell work
- Status: **RESOLVED — fixed.**
- Depends on: D48 (the list), D40 and D10 (why `grok` is optional)

## F122 — the `$` alias is registered behind a path that never reaches it

**An original-implementation defect**, found while trying to write a golden
for the last core word that had none.

`create_aliases.rs` binds `$` to `take`:

```rust reference/rust_multistackvm/src/stdlib/create_aliases.rs:36
    let _ = vm.register_alias("$".to_string(), "take".to_string());
```

but `apply` tests the sigil **before** it resolves aliases, and says so in its
own comment — "If function name starts with '$' we are forcing to call
internal function without lambda check or alias resolution"
(`reference/rust_multistackvm/src/multistackvm_apply.rs:33-34`). The sigil
branch calls `call_internal_word`, which strips the first character and
dispatches what is left
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`).
For a bare `$` what is left is the **empty name**, so the alias is dead by
construction: no spelling of `$` can reach `take`.

Confirmed against the oracle, 2026-09-12. `42 return $ println` reports
`i() for stack returned: Inline  not registered` — the doubled space is the
empty name the sigil left behind.

This is F71's shape with a different mechanism. F71 is a registration that
binds only one of two forms; this is a registration that is correct in
isolation and unreachable because an earlier branch consumes the name.

**Disposition: REPRODUCED.** Bund2 refuses it the same way and for the same
reason: `Interner::lookup_call` (`crates/bund2-api/src/lib.rs`) strips a
leading `$` and looks up the remainder, which for `$` is `""` and is never
interned, so dispatch answers `$ not registered`. The message differs from
the oracle's — Bund2 names `$`, the reference names the empty string — but
both refuse, and the path is unreachable in both, so no program can observe
the difference and no golden can pin it.

**Consequence for coverage.** `$` is one of the four core words `cargo xtask
coverage` reports as run by no golden, and it can never have one. With
`convert.to_dict` and `convert.to_dict.` deviating by decision (D57, F120) and
`drop_stack` unreproducible by construction (Q22), **core coverage 282/286 is
a ceiling and not remaining work**, which RFC-0000's D14 bullet now records.

## F121 — two of a `Slot`'s six fields are never written

**A Bund2 defect, latent**, found by RFC-0005's twentieth review (S2) while
checking what §S6's meaning guard covers.

`Slot` declares six binding fields — `command`, `alias`, `lambda`, `native`,
`class`, `method` (`crates/bund2-api/src/lib.rs`, `Slot`). Only the first four
are ever written. There is no `.class =` or `.method =` anywhere in the
workspace: `register_class` writes `self.classes` and bumps
`class_generation`, `register_method` writes `self.methods` and bumps
`method_generation`, and `oop_generation` hands out that pair. So class and
method resolution does not pass through a `Slot`, and two fields sit on the
struct carrying nothing.

**Why it matters, and why it is latent.** Nothing reads them, so no program
can see it today. The cost is to reasoning: RFC-0005 §S4 described the `Slot`
as holding "six independent bindings", and §S6 builds its meaning guard on the
claim that every writer of a binding §S4's chain consults bumps that slot's
generation. That is true and tight for the four live fields. If a later change
made `class` or `method` real on the `Slot`, the slot's generation would cover
them by accident while `class_generation` and `method_generation` kept
counting separately — two uncoordinated counters for one binding, which is the
shape RFC-0005's assumption 37 exists to prevent.

**Status:** OPEN. The fix is to delete the two fields, or to move class and
method resolution onto the `Slot` and retire the separate counters — a
question for RFC-0009's territory rather than this register. Until then
RFC-0005 says four, and names where class and method actually live.

## F120 — `convert.to_dict` converts to a matrix, not a dict

**An original-implementation defect**, found while reviewing the MATRIX family
for implementation.

Both bodies of the word pass `MATRIX` as the conversion target while their
error prefixes say `CONVERT.TO_DICT`:

```rust reference/rust_multistackvm/src/stdlib/convert/internal.rs:100
pub fn stdlib_convert_to_map_in_workbench(vm: &mut VM) -> Result<&mut VM, Error> {
```

with `MATRIX` as the target on the line below (`:101`), and
`stdlib_convert_to_map_in_stack` the same at `:103-104`, both registered at
`:122-123`. So `convert.to_dict` and `convert.to_matrix` are the same word
under two names. Confirmed against the oracle, 2026-09-12:

| program | oracle |
|---|---|
| `[ [ 1 2 ] [ 3 4 ] ] convert.to_dict` | `dt: 26` (MATRIX) |
| `[ [ 1 2 ] [ 3 4 ] ] convert.to_matrix` | `dt: 26` (MATRIX) |
| `[ [ 1 2 ] [ 3 4 ] ] convert.to_list` | `dt: 9` (LIST) |

**The intended conversion exists and is unreachable.** `conv` has a working
MAP target that keys a list's items by position as strings — `"0"`, `"1"`, … —
at `reference/rust_dynamic/src/conv.rs:426-434`, with the identical arm for
the other container source at `:331-339`. Nothing calls it with a MAP target
from the word layer, so the code is live but orphaned.

**Disposition: CORRECTED, per D57.** Bund2's `convert.to_dict` targets MAP.
This is a deviation rather than a reproduction, which is why it needed a
decision: reproducing it would have shipped two spellings of one word and left
`conv`'s MAP arm as dead code in Bund2 too. No golden covers either word — no
corpus program uses any of the six MATRIX-family words — so conformance does
not move, and there is no golden to record the disagreement against, which is
F48's gap.

`to_dict_answers_a_map_keyed_by_position` (`crates/bund2-stdlib/src/convert.rs`).

## F119 — `display` recurses on a value's depth, so `println` aborts

**A Bund2 defect against D37**, found by RFC-0005's nineteenth review (B2).

`BundValue::display` is a second renderer, distinct from the `render_into`
F117 fixed, and F117 did not touch it. Its `List` and `Map` arms called
`display` per member (`crates/bund2-value/src/lib.rs`). It is what `println`,
`print`, `pull` and the string conversions all reach, so it is on every
conformance path there is.

Measured 2026-09-11, release binary: `dup println` on a nested list **exits 0
at 27,000 and aborts at 28,000**, exit 134, while the same build without the
`println` exits 0 at 30,000 and `debug.display_stack` at 30,000 exits 0 — so
it is `display`, not the build, and not the path F117 fixed.

**Status:** FIXED 2026-09-12 (`crates/bund2-value/src/lib.rs`). `display`
drives a `Vec<DisplayStep>` — either already-formatted `Text` or a `Value`
still to render — and `display_step` queues a container's members and their
separators in reverse instead of recursing, exactly as F117's renderer does.
The bytes are unchanged at every depth: the crate's 51 tests, several of which
assert exact printed text, pass unchanged, and conformance stays 105/113 on
both tiers.
