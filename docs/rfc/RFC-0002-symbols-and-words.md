# RFC-0002: Symbols, the word slot table, and `bund2-api`

- Status: Draft. **Nothing gates this RFC**: D9, D14 and D29 are settled, and
  F18, F19 and F25 — the three defects that named it as the consumer of their
  consequence — now carry dispositions. Three reviews have rejected it on
  grounds independent of all six. See "Blocking decisions" and
  "Review history".
- Depends on: RFC-0001
- Decisions consumed: D9, D14, D16, D20, D27, D28, D29
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

**Three defects gated acceptance and all three now have dispositions.** Each
names RFC-0002 as the consumer of its consequence:

- **F18 — FIX.** `StackEffect` declares the **probed** arity, not the guard's.
  Fourteen words consequently report "Stack is too shallow" where the
  reference reports `NO DATA #2`. Preserving the guard would have made
  `StackEffect` declare 1 for a word that consumes 2, and RFC-0004 infers
  effects from it while RFC-0005 orders JIT guards by it — a static arity that
  lies does not stay cosmetic.
- **F19 — OMIT the table.** The slot table absorbs three tiers, not four.
  Names registered both ways are unaffected, since the inline registration is
  what makes them callable; names registered only there are the dead words,
  and D29 already ruled on those.
- **F26 — PRESERVE.** `$name` skips the lambda check and not alias
  resolution; the source comment claims both and is wrong. This RFC's
  preservation table takes that behaviour, and F26 carries a "Consequence for
  RFC-0002" clause like the other three, so it gates this RFC in the same way.
  Its disposition also supplies the alias-chain consequence: `$name` enters at
  `i` and resolves one link where a plain name resolves two.
- **F25 — OMIT the cluster.** `apply_in` is unreachable, so there is one
  resolution order to specify. The forward constraint is the part that binds:
  per-stack dispatch, when RFC-0007 needs it, is the single order with the
  stack as a parameter, not a second dispatcher.

Three further decisions this RFC consumes were not listed by an earlier
draft. **D20** — serialisation is a materialisation point — is what lets
`Symbol` be internal while names cross the wire. **D28** is the whole basis of
the D-1 handoff below. And **D27** rules the world file is **redb**, which
criterion 7 rides on: a lambda saved in one process and reloaded in another
crosses that format. D27 is RESOLVED but **gated on D11, which is OPEN** — if
an external reader of the world file exists, redb is a breaking format change.
So criterion 7 is checkable only once D11 rules, and the Design's description
of the reference's world file as SQLite is a statement about the reference,
not about Bund2's.

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
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:18`), where
`dup_one` is registered as an inline in the **stack** layer
(`reference/rust_multistack/src/stdlib/dup.rs:87`).

The name `dup` is *also* registered at
`reference/rust_multistack/src/stdlib/dup.rs:85`, but by `register_function`,
into the table no dispatch path consults — F19. So the alias is what makes
`dup` callable, and the `register_function` entry beside it is dead, exactly
as `dup_in` at `:88` is. That is the table D29 ruled on. An earlier draft
cited `:87` for `dup` itself, which is `dup_one`.

Following
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

### `apply` is not the only resolution order — `apply_in` is a second

Everything above describes `VM::apply`. `VM::apply_in`
(`reference/rust_multistackvm/src/multistackvm_apply_in.rs:7`) is a parallel
dispatcher taking a stack name, and it is **not** the same order:

- it has its own `CALL`, `CONTEXT` and catch-all arms with their own `autoadd`
  tests (`:15`, `:45`, `:62`), and under `autoadd` it pushes the name onto the
  named stack whole (`:16`) where `apply` pulls the top value and appends to
  it (`multistackvm_apply.rs:20-27`);
- and it has **no `$` arm at all**. `call_internal_word` appears nowhere in
  it, so a `$`-prefixed name dispatched through `apply_in` does not reach the
  internal word.

This is F25, whose disposition is **OMIT** — see Blocking decisions. An earlier draft of this RFC specified dispatch
against `apply` alone and said "three branches, not one" of a fact that holds
of one of two functions.

**But the cluster is dead.** `call_in` calls `apply_in`
(`reference/rust_multistackvm/src/multistackvm_call.rs:12`), `apply_in` calls
`lambda_eval_in` (`:25`), `lambda_eval_in` calls `apply_in` back
(`reference/rust_multistackvm/src/multistackvm_lambda_eval_in.rs:14`), and
**nothing outside the three calls any of them**. The live path is
`VM::call` → `apply` (`reference/rust_multistackvm/src/multistackvm_call.rs:8`),
which is what `execute` reaches
(`reference/rust_multistackvm/src/stdlib/execute.rs:30`).

So there is nothing to unify. RFC-0002 specifies **one** resolution order
because the reference has one that runs; the second is unreachable and is not
ported. `autoadd` therefore has **three live branches**, in `apply` — an
earlier note in this section said six, which counts the dead ones.

**F25's disposition is OMIT**, and it carries a forward constraint this RFC
inherits: when a later RFC needs "dispatch onto a named stack" — RFC-0007's
actor model is the likely one — it is built on the single resolution order
with the stack as a parameter, not by reviving a second dispatcher. The
reference's own second dispatcher drifted into disagreeing with the live path
about `$`; that is what a parallel code path costs.

### `autoadd` has three branches, all in `apply`

`self.autoadd` is tested in three places, each doing something different:

- `:19`, in the `CALL` arm — the name is appended to the value beneath it
  rather than executed (`:20-27`).
- `:72`, in the `CONTEXT` arm — the context value is pushed instead of
  switching stacks (`:73`), so **stack switching is suppressed**.
- `:89`, in the catch-all — any other value is appended to the value beneath
  it (`:90-96`).

An earlier draft covered only the first. The mode is a global on the `VM`
(`reference/rust_multistackvm/src/multistackvm.rs:22`), so all three are live
whenever it is set. `apply_in` has three more, but nothing can reach them —
see above.

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
runtime and no closed-world assumption is available.

**That is only half of D16, and it is the easier half.** The other half is
that a call *target* need not be a literal at all: `execute` turns a string
into a call, `ptr` builds a `PTR` from one
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:88`), and `bund.eval`
compiles a snippet. So a name can be computed, and the `$` sigil can be
computed with it — which is exactly what breaks a parse-time interner, as the
Design section records. An earlier draft consumed D16 and reduced it to table
mutability, which is the half that does not stress this design.

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

### The `$` sigil is honoured at dispatch, and interning happens wherever a
name becomes a call

`element` admits `SYMBOL` (`reference/bund_language_parser/bund.pest:36`) and
`name` is `element ~ nelement*` (`:28`), so **`$println` lexes as a single
name** — the grammar has no rule that separates the sigil. The reference
strips it at dispatch: `call_internal_word` takes `&name[1..]` and calls
`self.i` on the remainder
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`).

An earlier draft moved that to parse time, in `bund2-syntax`. **That is
wrong, and D16 is why.** The world never closes, so a call target can be a
string built at run time that no parser ever sees as a name. Confirmed against
the oracle: `"$println" ptr !` and `"$println" !` both reach the native, and
they still do after `println` has been shadowed by a lambda. Under
parse-time-only stripping all three fail.

So:

- **The sigil is honoured at dispatch**, as the reference does it. A slot is
  reached by the `Symbol` for the bare name; the sigil decides whether the
  `lambda` binding is consulted.
- **Interning happens wherever a name becomes a call**, which is *not* only
  the parser. `execute` (`!`) turns a string or a `PTR` into a call and is
  used 69 times across 39 corpus programs; `bund.eval` compiles a snippet;
  `ptr` builds a `PTR` from a string
  (`reference/rust_multistackvm/src/stdlib/artefacts.rs:88`). Each interns
  through the same table, and interning a name not seen before **adds a
  slot** — that is what D16's open world means for this design, and an
  earlier draft's single sentence named `bund.eval` alone.

Interning is therefore a runtime operation with a parse-time fast path, not a
parse-time operation. The fast path matters — it is where the thirteen
allocations go — but it is an optimisation of the general case, not the
definition of it.

### Alias resolution

Resolved once, at the point of dispatch, following `Alias(Symbol)` links. The
reference's double resolution is preserved in **effect**: an alias chain
resolves to the same target it does today. See Preservation analysis for the
one case where "twice" and "to a fixed point" differ.

### Native word declaration

```rust
pub enum WordKind { Sync, Blocking, Async }

pub type NativeFn = fn(&mut Vm) -> Result<(), Error>;
```

**The two tiers have different signatures, and merging them is a deviation an
earlier draft left unstated.** The VM tier is
`fn(&mut VM) -> Result<&mut VM, Error>`
(`reference/rust_multistackvm/src/multistackvm.rs:8`); the stack tier is
`fn(&mut TS) -> Result<&mut TS, Error>` (`reference/rust_multistack/src/ts.rs:9`).
They are different types over different receivers, which is *why* the
reference needs two tables and a fallthrough rather than one.

`NativeFn` is the VM-receiver form, and it is one of the five types
`bund2-api` guarantees. A stack-tier word becomes a `NativeFn` that reaches
the stacks through the VM, which is what the fallthrough already achieves at
`reference/rust_multistackvm/src/multistackvm_inline.rs:52` — the VM hands
`&mut self.stack` to the stack-tier function. Merging removes a receiver
distinction that no Bund program can observe, because dispatch reaches both
through `i_direct`.

It does remove one thing that *is* observable, and the preservation table now
carries it: the stack tier's errors are wrapped —
`i({}) for stack returned: {}` (`:64`) and
`VM inline function returned error: {}` (`:59`) — where VM-tier errors are
wrapped as `i({}) returned: {}` (`:48`). One tier means one wrapping.

`StackEffect` is declared at registration for natives.
`cargo xtask arity` already extracts the reference's guards mechanically and
found 14 under-declared ones (F18), so the initial declarations are derived
rather than hand-written.

Effect *inference* for Bund-defined words is RFC-0004, not here.

**Declaring the probed arity changes observable error text, and F18 is why.**
Fourteen words guard on a depth smaller than they consume, so the guard passes
and the *second* pull fails with a different message: `1 pair` reports
`NO DATA #2` rather than "Stack is too shallow for inline pair()". If
`StackEffect` carries the probed arity, the guard fires first and the message
becomes the "too shallow" one — a behaviour change on fourteen words, none of
which any corpus program reaches, so no golden covers it and nothing would
catch it.

**F18's disposition is FIX**: `StackEffect` declares the probed arity. The
fourteen words change error text, on inputs that error either way, and no
golden covers them.

### Registry builder

Registration happens against a builder, which freezes into an immutable
`Registry` shared by **`Rc`**, not `Arc`. The `BUND` global mutex disappears.
Runtime mutation — `register`, `alias` — writes through a per-VM overlay, so
D16's open world survives without a process-wide lock.

**`Arc` would be a lie, and an earlier draft told it — on a misquoted
premise.** A `Slot` holds a `BundValue` for its `lambda` and `class` bindings.
RFC-0001 defines `BundValue` as an **enum** whose non-scalar arm is
`Heap(Rc<HeapValue>)`, and it is `HeapValue` that carries `Cell` fields —
`identity` and `stamp`, not `id`, and not `curr`, which is a plain field
precisely so clones do not share a cursor. The conclusion survives the
correction and the premise did not: an `Rc` is neither `Send` nor `Sync` and a
`Cell` is not `Sync`, so a `Registry` reachable to one cannot cross a thread
whatever pointer wraps it. `Arc` would
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
| Last-write-wins registration | **Preserved exactly**, as a generation bump, with registrations replayed in source order and never deduped. Deduping would silently change which handler wins; the row below is what that costs today. |
| A name bound as lambda and native at once | **Preserved exactly.** Slots carry one `Option` per namespace, so writing a lambda does not disturb the native and `$name` still reaches it. |
| `CALL` carrying a name string, golden-visible | **Preserved exactly as text.** `Symbol` is internal; the `Debug` rendering resolves it back through the interner, so `data: String("println")` renders unchanged. |
| Lambda bodies bincoded into the world file | **Preserved exactly.** Serialisation is a materialisation point: names are written as strings, so a saved lambda reloads in another process. |
| `unregister` bound twice, no lambda unregisterable (F32) | **Deliberately fixed**, by giving the two words distinct names. No corpus program calls `unregister`, so conformance cannot move. |
| Open world — `register` / `alias` at runtime | **Preserved exactly.** D16. |
| Alias resolved twice, and once under `$` | **Deliberately changed, and the divergence starts at two links, not three.** `apply` resolves one link (`:39`) and `i` resolves another (`multistackvm_inline.rs:71`), so a plain name follows two. `$name` reaches `i` directly through `call_internal_word` and follows **one**. Confirmed on the oracle: with `a2 → b2 → println`, plain `a2` succeeds and `$a2` fails with `Inline b2 not registered`. Fixed-point resolution makes both succeed, so it changes `$`-dispatch at two links and plain dispatch at three. No such chain exists in the reference's 70 registrations and none is constructible in the corpus — but D16 means a program can build one. |
| `resolve` failing on stack-layer words (F31) | **Deliberately fixed.** No golden covers `resolve`, so conformance cannot move. |
| Interning is a pure read on lookup | **Deliberately changed, and this is the largest gap an earlier draft left.** The reference's tables are read-only on lookup: a miss returns an error and allocates nothing. Interning at dispatch **adds a slot** for a name never seen before, so a program that repeatedly dispatches on a *computed* miss — which D16 makes expressible — grows memory without bound where the reference does not. Bund2 interns a miss into a lookup-only form that does not retain, and only a successful bind creates a slot. |
| Stack-tier error wrapping | **Deliberately changed.** `i({}) for stack returned:` and `VM inline function returned error:` (`reference/rust_multistackvm/src/multistackvm_inline.rs:59,64`) disappear with the tier merge; one tier means one wrapping, `i({}) returned:` (`:48`). No golden captures either string. |
| The VM/stack fallthrough | **Preserved in effect, and here is the measurement it rests on**, which an earlier draft asserted without: the stack tier registers 31 inline names, the VM tier 156, and **the intersection is empty**. So no name resolves differently depending on which table is consulted first, and one table produces the same answer for all 187. |
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
   `name` and to the native by `$name`, **reaching the same slot**, and it
   does so **for a name that never passed the parser**: `"$println" ptr !`
   and `"$println" !` both reach the native, including after `println` has
   been shadowed. That third clause is the one that matters — an earlier
   draft stripped the sigil at intern time in `bund2-syntax`, which passes the
   first two clauses and fails this one, and D16 guarantees the case arises.
   **Tool:** a Bund2 unit test plus a probe, since the oracle passes all
   three.
6a. **A two-deep alias chain resolves the same way through `$name` as through
   `name`.** The reference does not: plain `a2` succeeds and `$a2` fails with
   `Inline b2 not registered`, because `apply` and `i` each resolve one link
   while `call_internal_word` reaches `i` directly and resolves one. This is
   the deviation the preservation table records, and it is asserted rather
   than left implicit.
7. A lambda saved to the world file in one process reloads and runs in
   another, with the `CALL` names intact. **Tool:** a Bund2 integration test.
   This is the criterion that `Symbol`-in-the-payload would have failed.
   **It rides on D27** — the world file is redb, not the reference's SQLite —
   and D27 is gated on **D11**, which is OPEN. Until D11 rules on whether an
   external reader exists, this criterion tests a format that may still
   change.
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
- `methods_fun` is **settled**, not open: it is a `method` binding in the
  `Slot` struct, alongside the other five. An earlier draft declared it open
  here while the Design had already placed it, which is the kind of
  disagreement that survives because the two statements are pages apart.
  `vars` stays a separate structure, for the reason the Design gives — it is
  keyed twice.

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

- **2026-08-26, review 3** — `docs/rfc/reviews/RFC-0002-review-2026-08-26-3.md`.
  Verdict: do not accept. Every citation resolved, the thirteen allocation
  rows were traced individually and correctly attributed — including the two
  easiest to get wrong, row 8 being `:81` not `:74` because `i` receives
  `dup_one`, which is not an alias, and row 10 naming only `is_inline`'s
  `format!` because the VM lookup fails before `get_inline` — and every count
  reproduced. Review 2's duplicated-`Design` blocker was gone.

  **Both blockers were the same mistake: specifying a mechanism against
  `VM::apply` alone.**

  The `$` sigil was moved to parse-time interning. The reference strips it at
  dispatch, and D16 means a call target can be a string built at run time that
  no parser sees. On the oracle `"$println" ptr !` and `"$println" !` both
  reach the native, and still do after `println` is shadowed by a lambda —
  three cases parse-time stripping fails. The draft's single sentence on
  runtime interning named `bund.eval` and not `execute`, which the corpus uses
  69 times across 39 programs. Interning is now a runtime operation with a
  parse-time fast path.

  The alias-chain row put the divergence at three links. It is **two** for
  `$name`, which reaches `i` directly and resolves one link where a plain name
  resolves two. Oracle, on `a2 → b2 → println`: plain `a2` succeeds, `$a2`
  fails with `Inline b2 not registered`. The RFC states that mechanism two
  sections earlier under F26 and had not drawn the conclusion.

  Three register entries — **F18**, **F19**, **F25** — name RFC-0002 as the
  consumer of their consequence, all three have empty dispositions, and the
  RFC cited none of them. They are now the recorded blockers. F25 in
  particular records `apply_in` as a **second resolution order with no `$` arm
  and three more `autoadd` branches**, so "three branches, not one" was true
  of one of two functions; and F18 collides with `StackEffect`, since
  declaring the probed arity changes error text on fourteen words. D16 was
  consumed and reduced to table mutability — its other half, dispatch by
  computed name, is precisely what broke the parse-time interner.

  Two citation-precision defects. `Library_introduction.typ:16` says
  `rust_multistack` "incorporates elements of the standard library", so the
  guide gives D14 its **axis and not its cut** — RFC-0000 was careful about
  exactly this and D14's entry was not; the cut now rests on the measured
  result, that every misfiling in B''s additions is in `bund/`. And
  `dup.rs:87` registers `dup_one`, not `dup`: `dup` is at `:85` through
  `register_function` into F19's dead table, beside `dup_in` at `:88`, which
  is the table D29 ruled on.

  Recorded separately: `cargo xtask lint` caught this session's own edit
  re-introducing a duplicate `## Design` — the very error it was built for,
  on the first real edit after it was built.

- **2026-08-26, review 4** — `docs/rfc/reviews/RFC-0002-review-2026-08-26-4.md`.
  Verdict: do not accept, **for the fourth time not for the reasons this RFC
  lists as its blockers**. D9, D14 and D29 are used correctly and F18, F19 and
  F25's dispositions are stated accurately. The thirteen allocation rows, the
  eight hash lookups, the 119/110 lock split, the dead `apply_in` cluster and
  criterion 2's 121/497 and 121/286 all verified.

  **F26 was the fourth blocker and this RFC missed it.** Its disposition was
  empty, it carries a "Consequence for RFC-0002" clause exactly like the three
  the RFC cited, and the RFC leans on it three times — including a
  preservation row that takes `$` skipping the lambda check as "preserved
  exactly". The register's rule applies. F26 now carries **PRESERVE**, and its
  disposition is also where the alias-chain consequence belongs: `$name`
  enters at `i` and resolves one link where a plain name resolves two.

  **F18's fix changes the residual stack, not only the error text.** `pair`
  guards `< 1` and pulls `x` before failing on the second pull, so `1 pair`
  errors today with an **empty** stack where a guard at 2 leaves the value —
  and the error path prints the stack
  (`reference/Bund/src/stdlib/helpers/print_error.rs:126-131`), which
  `execute-arm-not-executable.golden` captures. The promised replacement
  message is also three different messages: `complex` bails as `pair`
  (now **F40**), and the ten string words bail without parentheses. F18's
  disposition is corrected on both counts.

  **The two inline tiers have different signatures** — `fn(&mut VM)` against
  `fn(&mut TS)` — which is *why* the reference needs two tables and a
  fallthrough. The Design merged them into `native: Option<NativeFn>` and
  never defined `NativeFn`, one of the five types `bund2-api` guarantees. It
  is defined now, with the merge stated as a deviation and the stack tier's
  distinct error wrapping given a preservation row.

  Citation defects: `execute.rs:32` is the failure arm, not the call —
  `vm.call` is at `:30`. Inherited from F25, which cited it twice; corrected
  in both. And the `Rc`/`Arc` paragraph misquoted RFC-0001: `BundValue` is an
  enum whose non-scalar arm holds the `Rc`, and the `Cell` fields are
  `identity` and `stamp` on `HeapValue`, not `id`, and not `curr`. The
  conclusion survived; the premise did not.

  Two internal contradictions, both of the kind that survives because the two
  statements are pages apart: the RFC said F25's disposition "is still empty"
  forty lines after its own header said OMIT, and `methods_fun` was settled in
  the `Slot` struct and declared open in Open questions.

  Four preservation gaps, the largest being that **interning at dispatch adds
  a slot** — so repeated dispatch on a computed miss grows memory where the
  reference's tables are pure reads. Also the stack tier's error wrapping,
  `generation`'s missing overflow policy, and the fallthrough row, which
  rested on an unstated measurement: 31 stack-layer names against 156 VM-layer
  names with an **empty intersection**, now verified and stated.

  And the decisions-consumed list was short by three: **D20**, **D28**, and
  **D27** — which criterion 7 rides on, and which is itself gated on **D11**,
  still OPEN.
