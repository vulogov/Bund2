# RFC-0002: Symbols, the word slot table, and `bund2-api`

- Status: Draft. **No OPEN decision gates this RFC** — D9, D14 and D29 are all
  settled. Reviews 1 and 2 rejected it on grounds independent of them; both are
  addressed below. See "Review history".
- Depends on: RFC-0001
- Decisions consumed: D9, D14, D16, D29
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: nothing

## Blocking decisions

Per `CLAUDE.md`, an OPEN decision is not silently defaulted. Three gated this
RFC. **All three are now settled**, and what each settled is recorded below,
because a decision that arrives after a draft is written tends to leave the
draft phrased as though it were still open.

**D9 is RESOLVED — no.** `Intrinsic` stays internal to `bund2-stdlib`;
external crates get `Native` with a declared effect. Beyond the Cranelift
version-pinning argument, exposing lowerings would make the stable surface
depend on `bund2-jit`, which RFC-0000's B3 forbids for `bund2-stdlib` and
which is the inversion B3 exists to prevent.

**D29 is RESOLVED — revive `stacks_left` alone.** `dup_in`, `from_workbench`
and `push_to` get no slot. `<-` and `←` are already registered aliases and
will start resolving once their target exists. The in-scope set **will go**
497 -> 498; the tools print 497 until Bund2 implements the word.

**D14 is RESOLVED — method B″.** Core **286**, library **211**, of 497 in
scope, generated into `docs/core-words.md` by `cargo xtask scope --write`. It
settles both things it gated: `bund2-api` is **two surfaces**, and criterion
2 has its denominator.

Everything else here is groundable and drafted.

## Summary

Names become interned `Symbol`s resolved once at parse time. **Six** of the
seven name-keyed `HashMap<String, _>` tables in `VM`, plus the stack layer's
separate inline table, become one slot table indexed by `Symbol` — each slot
carrying one `Option` per namespace, so the namespaces stay independent while
the lookup becomes an index. `vars` stays separate, being keyed twice. Each
slot carries a generation counter so a redefinition invalidates inline caches
without a scan. Native words declare a `StackEffect`
and a kind (`Sync` / `Blocking` / `Async`) at registration. A registry builder
replaces the `BUND` global mutex. `bund2-api` becomes the one crate with a
stability guarantee.

## Motivation

### Dispatching one word allocates thirteen strings

Take `dup`, an alias for `dup_one`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:18`), which is
registered in the **stack** layer
(`reference/rust_multistack/src/stdlib/dup.rs:87`). Following
`reference/rust_multistackvm/src/multistackvm_apply.rs:9-62`:

| # | allocation | site |
|---|---|---|
| 1 | the name itself | `multistackvm_apply.rs:11` `cast_string()` |
| 2 | `fun_name.clone()` | `multistackvm_apply.rs:16` for `is_command` |
| 3 | `fun_name.clone()` | `multistackvm_apply.rs:39` for `get_alias` |
| 4 | `aname.clone()` returned | `multistackvm_alias.rs:34` |
| 5 | `real_name.clone()` | `multistackvm_apply.rs:46` for `is_lambda` |
| 6 | `real_name.clone()` | `multistackvm_apply.rs:59` for `i` |
| 7 | `name.clone()` | `multistackvm_inline.rs:71` for `is_alias` |
| 8 | `name.clone()` | `multistackvm_inline.rs:81` for `i_direct` |
| 9 | `name.clone()` | `multistackvm_inline.rs:42` for `is_inline` |
| 10 | `format!("{}_inline", …)` | `multistackvm_inline.rs:25` |
| 11 | `name.clone()` | `multistackvm_inline.rs:52` for the stack fallthrough |
| 12 | `format!("{}_inline", …)` | `ts_inline.rs:33` |
| 13 | `format!("{}_inline", …)` | `ts_inline.rs:34` |

Thirteen heap allocations and eight hash lookups, to call `dup`. Nothing in
that sequence carries information a parser could not have resolved once.

### Every lookup is `contains_key` then `get`

The pattern repeats in every table: `get_inline`
(`reference/rust_multistackvm/src/multistackvm_inline.rs:32-33`), `get_alias`
(`reference/rust_multistackvm/src/multistackvm_alias.rs:32-33`),
`get_command` (`reference/rust_multistackvm/src/multistackvm_command.rs:32-33`),
`get_method` (`reference/rust_multistackvm/src/multistackvm_methods.rs:32-33`).
Each hashes the key twice to answer one question.

### The suffix convention is a defect surface

`register_inline` stores under `format!("{}_inline", &name)` in both layers
(`reference/rust_multistackvm/src/multistackvm_inline.rs:8`,
`reference/rust_multistack/src/ts_inline.rs:8`). The suffix must then be
reproduced at every read site. One site dropped it — `TS::is_inline` tests the
bare name (`reference/rust_multistack/src/ts_inline.rs:25`) — and the result
is **F31**: `resolve` cannot find any of the 31 stack-layer words. Confirmed
against the oracle.

That is the cost of encoding a namespace in a string: the convention is not
checkable, so a single omission is silent.

### The interpreter runs under one global mutex

`BUND` is a `lazy_static! { Mutex<Bund> }`
(`reference/Bund/src/stdlib/mod.rs:7-12`), locked at 119 sites. 110 of those
are `init_stdlib` registrations. The rest matter more:
`run_snippet_for_script` takes the lock and holds it for the **entire**
program run (`reference/Bund/src/stdlib/helpers/run_snippet.rs:63`).

Two consequences. There is no concurrency to be had today. And no word may
lock `BUND`, because `std::sync::Mutex` is not reentrant — the only reason
nothing deadlocks is that every in-word lock is in `init_stdlib`, which runs
before any program does. `bund.eval` re-enters the interpreter and takes `vm`
directly rather than the lock
(`reference/Bund/src/stdlib/functions/bund/bund_eval.rs:31`), which is what
keeps it safe.

## Current behaviour

This section is the preservation contract.

### Seven name-keyed tables

`VM` holds `inline_fun`, `command_fun`, `methods_fun`, `lambdas`, `classes`,
`vars` and `name_mapping` — every one a `HashMap` keyed by `String`
(`reference/rust_multistackvm/src/multistackvm.rs:24-30`). The stack layer
holds its own `inline_fun` on top of that, reached by fallthrough.

They are distinct namespaces, and conflating them produces false resolutions.
A `$`-prefixed name forces the internal word
(`reference/rust_multistackvm/src/multistackvm_apply.rs:33`); methods are
reached through object dispatch, not by name; and the stack layer's dead
`functions` table is consulted by nothing.

**The tables are independent, and one name can occupy several at once.**
`register_lambda` validates the value is a lambda and inserts into
`vm.lambdas` (`reference/rust_multistackvm/src/multistackvm_lambdas.rs:8,13`);
it does not touch `inline_fun`. So `println` can be a lambda *and* a native
simultaneously, and the two are reachable by different spellings — `println`
finds the lambda (`reference/rust_multistackvm/src/multistackvm_apply.rs:46`),
`$println` finds the native, because `call_internal_word` skips the lambda
check. Confirmed against the oracle. Lambdas and classes are independent in
the same way (`reference/rust_multistackvm/src/multistackvm.rs:27-28`).

This is the constraint that decides the slot design below, and an earlier
draft of this RFC stated the principle here and then violated it forty lines
later.

### Resolution order

`VM::apply` on a `CALL` value
(`reference/rust_multistackvm/src/multistackvm_apply.rs:9-62`):

1. Empty name → error (`:13-15`).
2. `is_command` → `c()` (`:16-17`).
3. Otherwise, **under an `autoadd` branch** (`:19`): if `autoadd` is set the
   name is appended to the value beneath it on the stack (`:20-27`) rather
   than executed. Only in the `else` does resolution continue. Note the
   ordering: `is_command` fires first (`:16`) and returns (`:17`), so
   `autoadd` does **not** precede the command check — a command runs even
   with `autoadd` set.
4. `$`-prefix → `call_internal_word` (`:33-35`).
5. Alias resolution (`:39-42`).
6. Lambda (`:46-54`).
7. Inline (`:59`).

### `autoadd` has three branches, not one

`self.autoadd` is tested in three places, each doing something different:

- `:19`, in the `CALL` arm — the name is appended to the value beneath it
  rather than executed (`:20-27`).
- `:72`, in the `CONTEXT` arm — the context value is pushed instead of
  switching stacks (`:73`), so **stack switching is suppressed**.
- `:89`, in the catch-all — any other value is appended to the value beneath
  it (`:90-96`).

An earlier draft covered only the first. The mode is a global on the `VM`
(`reference/rust_multistackvm/src/multistackvm.rs:22`), so all three are live
whenever it is set.

### Alias resolution happens twice

`apply` resolves the alias at `:39` and then calls `self.i(real_name)` at
`:59`. `VM::i` resolves the alias **again**
(`reference/rust_multistackvm/src/multistackvm_inline.rs:71-74`) before
reaching `i_direct`. For an alias onto a real word the second lookup is
wasted; for an alias onto an alias, the two together resolve two levels while
either alone resolves one.

### `$` skips the lambda check, not alias resolution

The comment says "without lambda check or alias resolution"
(`reference/rust_multistackvm/src/multistackvm_apply.rs:30-31`), but
`call_internal_word` strips the `$` and calls `self.i`
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`),
and `i` resolves aliases (`multistackvm_inline.rs:71`). Only the lambda check
is skipped. This is **F26**, confirmed by probe.

### Registration is last-write-wins, and the world never closes

`register_inline` unregisters before inserting
(`reference/rust_multistackvm/src/multistackvm_inline.rs:6-9`), so a later
registration of the same name replaces the earlier one silently. `register`,
`unregister`, `alias` and `unalias` are words, so the table is mutable at
runtime and no closed-world assumption is available. This is **D16**.

## Design

### Symbols

```rust
pub struct Symbol(u32);
```

Interned at parse time. A `CALL` value carries a `Symbol`, not a `String`, so
the thirteen allocations above become zero: resolution is an index.

Interning happens once per distinct name per program. The interner is part of
the registry, not a global.

### One slot table — but a slot is a set of bindings, not one binding

```rust
pub struct Slot {
    generation: u32,
    command: Option<NativeFn>,
    alias:   Option<Symbol>,
    lambda:  Option<BundValue>,
    native:  Option<NativeFn>,
    class:   Option<BundValue>,
    method:  Option<NativeFn>,
}
```

`Vec<Slot>` indexed by `Symbol`. **The bindings are independent, and that is
load-bearing** — an earlier draft made `WordEntry` a single enum, which cannot
represent the reference and is the thing this RFC's first review rejected.

A name can be a lambda **and** a native at the same time. `register_lambda`
checks only `is_type(LAMBDA)` and inserts into `vm.lambdas`
(`reference/rust_multistackvm/src/multistackvm_lambdas.rs:8,13`); it never
touches `inline_fun`. Resolution then splits: `apply` finds the lambda first
(`reference/rust_multistackvm/src/multistackvm_apply.rs:46`), while `$name`
goes through `call_internal_word`, which skips the lambda check entirely
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`).

Confirmed against the oracle. After registering a lambda named `println`,
`println` runs the lambda and `$println` runs the native — the same name, two
live bindings, told apart only by the `$`. That is F26's second probe, and a
single-slot enum makes it unrepresentable: writing `Lambda` would destroy
`Native` and `$` would have nothing to reach.

The same holds for lambdas versus classes, which are also independent tables
(`reference/rust_multistackvm/src/multistackvm.rs:27-28`).

So the collapse this RFC proposes is **one lookup, not one binding**: six of
the seven string-keyed maps, plus the stack layer's own inline table, become
one indexed array whose entries carry those namespaces separately. The `native`
binding is where the VM's `inline_fun` and the stack layer's `inline_fun`
merge, which is what removes the fallthrough at
`reference/rust_multistackvm/src/multistackvm_inline.rs:51-67`.

§"Seven name-keyed tables" states the principle — conflating them produces
false resolutions — and the earlier draft committed exactly that error forty
lines after stating it.

`vars` stays a separate structure: it is
`HashMap<String, HashMap<String, Value>>`
(`reference/rust_multistackvm/src/multistackvm.rs:29`), keyed twice, and does
not fit a per-name slot.

**The `_inline` suffix does not exist.** F31 is not expressible here: there is
no second spelling of a key to get wrong.

`generation` increments when any binding in a slot is rewritten. An inline
cache records the generation it was built against, so a redefinition
invalidates caches without a scan — which is what makes D16's permanently-open
world affordable.

### Registration order is replayed, never deduped

`register_inline` unregisters before inserting
(`reference/rust_multistackvm/src/multistackvm_inline.rs:6-9`), so a duplicate
registration silently replaces the earlier one. That is not a wart to clean
up: **F32 depends on it.** The lambda registry binds `unregister` twice in
consecutive statements
(`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:89,90`) and the
second wins, which is why no lambda can be unregistered today.

A registry builder is precisely where a duplicate would get deduped or
reordered, and either would change which handler runs. The builder therefore
replays registrations in source order with last-write-wins, and F32 is fixed
by giving the two words distinct names — deliberately, in the open — rather
than by the build order quietly changing underneath.

### Symbols are internal; names are what cross every boundary

A `Symbol` is an index into a per-run interner. That makes it meaningless
outside the run, and two boundaries carry names out:

- **The world file.** `save.lambdas` bincodes whole `Value`s into SQLite
  (`reference/Bund/src/stdlib/helpers/world/lambdas.rs:81`), and a lambda body
  is a list of `CALL` values. A `u32` index would be nonsense on reload, and
  it would break RFC-0001's byte-identical wire format.
- **The `Debug` rendering.** `CALL` values are golden-visible in a passing
  corpus golden, not merely in error text:
  `tests/golden/examples/bund_dynamic_demos/compile_and_apply.golden` renders
  `dt: 6, q: 100.0, data: String("println")`, and the same for `"+"` and
  `"format"`. `"get"` and `"swap"` are golden-visible too, in the
  `examples/object_oriented_programming/` goldens rather than that one. There
  are 43 `dt: 6` and 30 `dt: 7` renderings suite-wide.

So the rule is the same one RFC-0001 applies to identity: **`Symbol` inside,
string at every observation boundary.** The interner supports reverse lookup
by construction, being a `Vec<String>` indexed by `Symbol`, so rendering and
serialisation resolve back to the exact name. Serialisation is a
materialisation point, exactly as D20 has it.

Interning happens at parse time, in `bund2-syntax`, which is where the
reference's grammar `reference/bund_language_parser/bund.pest` is
re-implemented — that grammar is what defines a name, and it is the premise of
"interned at parse time". A `CALL` built at run time by `bund.eval` interns
through the same table.

### The `$` sigil is stripped at intern time, not at dispatch

`element` admits `SYMBOL` (`reference/bund_language_parser/bund.pest:36`), and
`name` is `element ~ nelement*` (`:28`), so **`$println` lexes as a single
name** — the grammar has no rule that separates the sigil. The reference
strips it at run time instead: `call_internal_word` takes `&name[1..]` and
calls `self.i` on the remainder
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`).

That matters here because a naive interner would give `println` and `$println`
**different `Symbol`s**, and then "the same slot reached two ways" would be
false — there would be two slots. An earlier draft's criterion required them
to resolve "for the same `Symbol`" without saying how, which the grammar
forbids.

So `bund2-syntax` strips the sigil when it interns: `$println` interns to the
`Symbol` for `println`, with the sigil recorded on the `CALL` as a flag rather
than as part of the name. Dispatch then reads the flag to decide whether to
skip the `lambda` binding, which is exactly what `call_internal_word` does
with `&name[1..]`, moved from run time to parse time.

The name must still render as `$println` in `Debug` output and serialise as
`$println`, by the same rule as every other boundary.

### Alias resolution

Resolved once, at the point of dispatch, following `Alias(Symbol)` links. The
reference's double resolution is preserved in **effect**: an alias chain
resolves to the same target it does today. See Preservation analysis for the
one case where "twice" and "to a fixed point" differ.

### Native word declaration

```rust
pub enum WordKind { Sync, Blocking, Async }
```

`StackEffect` is declared at registration for natives.
`cargo xtask arity` already extracts the reference's guards mechanically and
found 14 under-declared ones (F18), so the initial declarations are derived
rather than hand-written.

Effect *inference* for Bund-defined words is RFC-0004, not here.

### Registry builder

Registration happens against a builder, which freezes into an immutable
`Registry` shared by **`Rc`**, not `Arc`. The `BUND` global mutex disappears.
Runtime mutation — `register`, `alias` — writes through a per-VM overlay, so
D16's open world survives without a process-wide lock.

**`Arc` would be a lie, and an earlier draft told it.** A `Slot` holds a
`BundValue` for its `lambda` and `class` bindings, and RFC-0001 defines
`BundValue` as an `Rc<HeapValue>` whose `id`, `stamp` and `curr` are `Cell`s.
`Rc` is neither `Send` nor `Sync` and `Cell` is not `Sync`, so a `Registry`
containing one cannot cross a thread whatever pointer wraps it. `Arc` would
buy nothing and would advertise a capability the contents forbid. This is the
first real collision between RFC-0001 and RFC-0002, and neither cited the
other until this review.

It follows that **this is not "the change RFC-0007's actor model needs"**,
which an earlier draft claimed. Removing the global mutex makes a per-VM
registry possible; making a registry *shareable across threads* is a separate
problem that starts with whether `BundValue` is `Send`, and RFC-0001 says it
is not. RFC-0007 inherits that question rather than a solution.

**The overlay cannot express `unregister` of a builtin.** An overlay that
holds additions has no way to record a removal, so `unregister` of a word from
the frozen base would silently do nothing — which is F32's shape reintroduced
by the fix for F32. The overlay therefore stores an explicit
present-or-removed state per slot, not just an addition. Recorded because the
naive reading is the one that looks obviously right.

### `bund2-api`

The one crate with a stability guarantee: `Symbol`, `StackEffect`, `WordKind`,
`NativeFn`, and the registration surface.

D9 settles the first question: `Intrinsic` is **not** here. `LowerFn` would
pin every consumer to an exact Cranelift version, and it would put `bund2-jit`
behind the stable surface.

D14 settles the shape: **two surfaces**. The core surface covers the 286
words D14 records as language core and carries the stability guarantee — it is
what "100% preserved" is judged against. The second is what out-of-tree word
packages compile against to supply library words, and it carries the weaker
promise appropriate to that: `Native` registration with a declared effect, and
no `Intrinsic` (D9).

The line between them is not drawn here by hand. It is
`docs/core-words.md`, generated by `cargo xtask scope --write`, and it follows
the reference author's own three-layer split — `rust_multistack` and
`rust_multistackvm` are the language, the Bund runtime is where the standard
library lives
(`reference/Bund/Documentation/Bund_Library_Guide/Library_introduction.typ:15-19`).

D29 settles the word set at the margin: `stacks_left` gets a slot, `dup_in`,
`from_workbench` and `push_to` do not, and `<-`/`←` resolve for the first
time. That is a deliberate deviation in both directions and is recorded there.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| Resolution order: command, `$`-internal, alias, lambda, inline | **Preserved exactly**, including the order. |
| `autoadd`, all three branches | **Preserved exactly**, including that it does *not* precede the command check — `is_command` returns at `apply.rs:17` before `autoadd` is consulted at `:19` — and including `:72`, where it suppresses stack switching rather than appending. |
| `$` skipping the lambda check but not aliases (F26) | **Preserved exactly**, including the surprise. The comment is wrong; the behaviour is the contract. |
| Fallthrough from the VM table to the stack table | **Preserved in effect.** One slot table has no two tiers to fall between; the words keep their names and meanings. |
| Last-write-wins registration | **Preserved exactly**, as a generation bump, with registrations replayed in source order and never deduped — F32 depends on the second `unregister` winning. |
| A name bound as lambda and native at once | **Preserved exactly.** Slots carry one `Option` per namespace, so writing a lambda does not disturb the native and `$name` still reaches it. |
| `CALL` carrying a name string, golden-visible | **Preserved exactly as text.** `Symbol` is internal; the `Debug` rendering resolves it back through the interner, so `data: String("println")` renders unchanged. |
| Lambda bodies bincoded into the world file | **Preserved exactly.** Serialisation is a materialisation point: names are written as strings, so a saved lambda reloads in another process. |
| `unregister` bound twice, no lambda unregisterable (F32) | **Deliberately fixed**, by giving the two words distinct names. No corpus program calls `unregister`, so conformance cannot move. |
| Open world — `register` / `alias` at runtime | **Preserved exactly.** D16. |
| Alias resolved twice | **Preserved for one level, deliberately changed for chains.** Resolving to a fixed point differs from resolving exactly twice only for an alias chain three deep or more. No such chain exists in the reference's own registrations, and none is constructible in the corpus. Stated as a deviation rather than assumed away. |
| `resolve` failing on stack-layer words (F31) | **Deliberately fixed.** No golden covers `resolve`, so conformance cannot move. |
| `BUND` held for the whole run | **Deliberately changed.** Observably identical for a single-threaded program, which is every program today. |

## Alternatives considered

**Keep string keys, drop the suffix.** Fixes F31 and nothing else: the
thirteen allocations and eight hashes remain, because the cost is in hashing
strings at dispatch, not in the suffix.

**Keep the seven tables, index each by `Symbol`.** Cheaper to write, and it
keeps the independence the design needs. Rejected on cost, not on structure —
the chosen design keeps six independent bindings too, so "fewer places a name
can live" was never the argument and an earlier draft wrongly made it. Seven
`Vec`s indexed by `Symbol` means seven bounds-checked loads and seven cache
lines on a dispatch that consults several namespaces in order; one `Vec` of
slots means one. The thirteen allocations go either way; the memory traffic
does not.

**Intern at registration only, not at parse.** Halves the benefit. A `CALL`
built at runtime by `bund.eval` still needs interning, so the interner must
be live anyway; interning at parse costs nothing extra.

## Acceptance criteria

An earlier draft listed five, of which three had no mechanism that could run
them and one was vacuous without saying so. Each below names the tool.

1. `cargo xtask conform` does not fall below the mark in
   `tests/golden/CONFORMANCE.txt`. RFC-0002 changes dispatch, not meaning.
   **Tool:** `cargo xtask conform`, which exists.
2. `cargo xtask coverage` reports **CORE COVERAGE** against D14's core half:
   **121/286** at this commit, printed beside the in-scope figure of 121/497.
   Only the core half is a preservation target; the library 211 is deferrable
   and re-implementable out of tree. D29 takes the in-scope set to 498 and the
   core set to 287 once `stacks_left` is implemented — the tools print 497 and
   286 until then. **Tool:** `cargo xtask coverage`, reading the partition
   `cargo xtask scope --write` generates.
3. Dispatching a word allocates **0** times. **No tool exists yet**:
   `cargo xtask layout` measures value shapes with a counting allocator and
   has no VM to dispatch in. This criterion becomes checkable when
   `bund2-interp` can execute a `CALL`, and the harness is the counting
   allocator moved behind a `cargo xtask dispatch` that RFC-0003 adds. Listed
   now because the 13-allocation figure in Motivation is what it answers;
   flagged because listing an unrunnable criterion as though it were runnable
   is what the earlier draft did.
4. `cargo tree -p bund2-api` lists no `bund2-interp` and no `bund2-jit`.
   **Near-vacuous today**: the crate depends only on `bund2-value` and both
   forbidden crates are empty, so it passes because there is nothing to drag
   rather than because the boundary is enforced. It becomes real when
   `bund2-interp` has contents. Labelled, as RFC-0001's equivalent is.
5. `"dup_one" resolve` succeeds, and so does `resolve` for each of the 31
   stack-layer words — F31's regression test. **This cannot be a golden.**
   Goldens capture what the oracle does, and the oracle *fails* this: F31 is a
   reference defect, so `cargo xtask golden` would pin the failure as expected
   behaviour. It is a Bund2 unit test in `bund2-interp`, and F31's entry is
   the reference for the deviation. The same applies to F32's fix.
6. A name bound as both a lambda and a native resolves to the lambda by
   `name` and to the native by `$name`, **reaching the same slot** — the
   sigil is stripped when the name is interned, so `$println` and `println`
   share a `Symbol` and the `$` travels as a flag on the `CALL`. The grammar
   makes this a requirement rather than a nicety: `element` admits `SYMBOL`
   (`reference/bund_language_parser/bund.pest:36`) so `$println` lexes as one
   name, and a naive interner would produce two symbols and two slots.
   **Tool:** a Bund2 unit test; `tests/probes/` can hold the oracle side,
   since the oracle passes this one.
7. A lambda saved to the world file in one process reloads and runs in
   another, with the `CALL` names intact. **Tool:** a Bund2 integration test.
   This is the criterion that `Symbol`-in-the-payload would have failed.
8. `cargo xtask cite` reports zero defects. Note what this does **not** check:
   that a cited line means what the prose says. Two reviews have now found
   citations that resolve and mislead, and both times `cite` passed.

**D-1 is not discharged here.** RFC-0000 deferred its feature-gating criterion
with "D28 is a commitment until RFC-0002 declares the feature set", and this
RFC declares no feature — the slot table and `bund2-api` are unconditional.
D-1 therefore moves to RFC-0003, which is the first RFC that adds an optional
subsystem. Saying so is the point: an inherited criterion that no RFC picks up
is how a commitment quietly lapses.

## Open questions

- **D9, D14, D29** — see "Blocking decisions". D14 and D29 together also fix
  criterion 2's denominator, which is why RFC-0000's B2 carries both as stated
  limits.
- **Q15** — `cargo xtask unblock` needs redesigning before it can order the
  work this RFC enables. It ranks against the goldens, which see about a
  quarter of the in-scope words.
- Whether `methods_fun` and `vars` become slot-table discriminants or stay
  separate tables is left to the implementation; both preserve the namespaces.
  Recorded here so it is not read as settled.

## Review history

- **2026-08-26** — `docs/rfc/reviews/RFC-0002-review-2026-08-26.md`. Verdict:
  do not accept, **and not for the three decisions it was already blocked on**.
  Every finding held on re-verification.

  The structural blocker: one slot per name cannot hold a lambda and a native,
  and the reference holds both. `register_lambda` writes only `vm.lambdas`
  (`reference/rust_multistackvm/src/multistackvm_lambdas.rs:8,13`), so
  `println` can be a lambda and a native at once, told apart by `$` — which is
  F26's own second probe, cited two sections earlier in the same draft. Under
  a single `WordEntry` enum, writing `Lambda` destroys `Native` and `$` has
  nothing to fall through to, making the preservation row that promised F26
  "preserved exactly, including the surprise" unsatisfiable by the structure
  defined forty lines above it. The draft stated the principle —
  *conflating them produces false resolutions* — and then committed it. A slot
  is now a set of `Option` bindings, one per namespace.

  Two unlisted changes, both now carrying preservation rows. `CALL` payloads
  are golden-visible in a passing corpus golden, not just probe error text
  (`compile_and_apply.golden` renders `data: String("println")`, `"+"`,
  `"format"`), so "a `CALL` carries a `Symbol`, not a `String`" needed one.
  And `Symbol` is not serialisable across processes: `save.lambdas` bincodes
  whole `Value`s into the world file
  (`reference/Bund/src/stdlib/helpers/world/lambdas.rs:81`) and lambda bodies
  are lists of `CALL`s, so a per-run index would be meaningless on reload and
  would break RFC-0001's byte-identical wire format. The rule is now the same
  one RFC-0001 applies to identity: `Symbol` inside, string at every boundary.

  A reference defect found by reading the mechanism: `unregister` is bound
  twice in consecutive statements
  (`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:89,90`), so no
  lambda can be unregistered. Recorded as **F32**, and it is the reason the
  registry builder must replay registrations rather than dedupe them.

  Three citations were off by one — `bund_eval.rs:30` for a claim at `:31`,
  `ts_inline.rs:24` for the bare-name test at `:25` (inherited from F31, now
  corrected there too), and `methods_fun.rs:32` where its three siblings are
  cited `:32-33`. `cargo xtask cite` passed all three.

  On the criteria: RFC-0000 deferred D-1 with "D28 is a commitment until
  RFC-0002 declares the feature set" and this RFC declares no feature, so D-1
  is explicitly handed to RFC-0003 rather than left to lapse. Criterion 3 had
  no tool, criterion 5 had no mechanism — the oracle *fails* it, so no golden
  can capture it — and criterion 4 was vacuous and, unlike RFC-0001's,
  unlabelled. All are now labelled with what runs them or why nothing does.

  One correction the review owed elsewhere: F18's heading said 14 words and
  its list enumerated 13. The missing one is `tail`, re-derived from
  `docs/arity.md`.

- **2026-08-26, review 2** — `docs/rfc/reviews/RFC-0002-review-2026-08-26-2.md`.
  Verdict: do not accept, and again not for D14.

  **The blocker was mechanical and mine.** The file contained **two `## Design`
  sections**: the revision from review 1 was inserted ahead of the old text
  instead of replacing it, so the committed RFC asserted both "a slot is a set
  of bindings, not one binding" and, sixty lines later, the single-enum
  `WordEntry` that review 1 had rejected — along with a verbatim duplicate of
  the Current-behaviour tail. Deleted; nothing in the duplicated block had
  survived the revision. Worth naming plainly: a scripted edit that inserts
  rather than replaces produces a document that passes every citation check
  and contradicts itself, and `cargo xtask cite` was clean throughout.

  Beyond the deletion:

  **`autoadd` was wrong twice.** The preservation row promised it "preceded
  all of it" while §Resolution order correctly had `is_command` firing first
  and returning at `apply.rs:17`. And it has **three** branches, not one —
  `:19` in the `CALL` arm, `:72` in the `CONTEXT` arm where it suppresses
  stack switching, and `:89` in the catch-all. Review 1 raised the second
  point and it had stayed open.

  **Criterion 6 asked for what the grammar forbids.** It required `name` and
  `$name` to resolve "for the same `Symbol`", but `element` admits `SYMBOL`
  (`reference/bund_language_parser/bund.pest:36`) so `$println` lexes as a
  single name and would intern separately. The reference strips the sigil at
  run time (`multistackvm_call_internal_word.rs:7`); the RFC never said where
  `$` is handled at all. It is now stripped at intern time, with the sigil
  carried as a flag on the `CALL`.

  **`Arc<Registry>` collided with RFC-0001** — the first real collision
  between the two, and neither cited the other. A `Slot` holds a `BundValue`,
  which RFC-0001 defines as an `Rc<HeapValue>` with `Cell` fields: neither
  `Send` nor `Sync`. `Arc` buys nothing the contents allow, so it is now `Rc`,
  and the claim that this is "the change RFC-0007's actor model needs" is
  withdrawn — RFC-0007 inherits the question of whether `BundValue` can cross
  a thread, not an answer.

  **The per-VM overlay could not express `unregister` of a builtin**, which is
  F32's shape reintroduced by the fix for F32. The overlay now stores an
  explicit present-or-removed state per slot.

  One citation defect: `compile_and_apply.golden` renders `println`, `+` and
  `format`, not `get` or `swap` — those are golden-visible in the
  `object_oriented_programming` goldens. The suite total of 43 was right; the
  attribution was not, and `cite` cannot catch it because goldens are data it
  scans for citations rather than a target it checks against.

  The in-scope figure was asserted in the perfect tense — "goes 497 -> 498" —
  while both tools print 497 and will until `stacks_left` is implemented.
  Corrected to the future.

  And review 1's D1/D2 were still open: Summary, Design and Alternatives gave
  three different answers on which tables collapse. The truth is six of the
  seven plus the stack layer's inline table, with `vars` separate; the
  Alternatives rejection has been rewritten too, since "fewer places a name
  can live" was never the argument — the chosen design keeps six.

  Recorded from this pass: **F37**, `stdlib/classes/registry.rs` is source
  that no `mod` declares and that therefore never compiles. It contributes no
  unique name, so no count moves; what it exposes is that `cargo xtask corpus`
  attributes "last wins" by path order while the real order is the call
  sequence at `stdlib/mod.rs:29-51`. The two agree here by accident. Per the
  owner: registration is last-write-wins, so the outcome is order-determined
  and no tool change is warranted.
