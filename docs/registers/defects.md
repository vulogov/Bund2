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
