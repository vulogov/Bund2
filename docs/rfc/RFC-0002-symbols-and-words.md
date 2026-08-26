# RFC-0002: Symbols, the word slot table, and `bund2-api`

- Status: **Draft — blocked.** See "Blocking decisions". Three OPEN decisions
  gate parts of this design, and this draft marks each spot rather than taking
  a default.
- Depends on: RFC-0001
- Decisions consumed: D16
- Blocked on: D9, D14, D29
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: nothing

## Blocking decisions

Per `CLAUDE.md`, an OPEN decision is not silently defaulted. Three gate this
RFC, and each is marked `[BLOCKED: Dn]` at the point where it bites:

- **D9** — whether `Intrinsic` lowerings are exposed through `bund2-api`.
  Gates one item in the stable surface, nothing else.
- **D14** — which words are language core and which are library. Gates the
  **shape** of `bund2-api`: whether it is one surface or two.
- **D29** — revive or drop the four dead words. Gates the **word set** the
  slot table is populated with, and hence RFC-0000's B2 denominator.

Everything else here is groundable and drafted. The design does not depend on
how the three resolve; the surface does.

## Summary

Names become interned `Symbol`s resolved once at parse time. The seven
name-keyed `HashMap<String, _>` tables in `VM` collapse to one slot table
indexed by `Symbol`, each slot carrying a generation counter so a redefinition
invalidates inline caches without a scan. Native words declare a `StackEffect`
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
`get_method` (`reference/rust_multistackvm/src/multistackvm_methods.rs:32`).
Each hashes the key twice to answer one question.

### The suffix convention is a defect surface

`register_inline` stores under `format!("{}_inline", &name)` in both layers
(`reference/rust_multistackvm/src/multistackvm_inline.rs:8`,
`reference/rust_multistack/src/ts_inline.rs:8`). The suffix must then be
reproduced at every read site. One site dropped it — `TS::is_inline` tests the
bare name (`reference/rust_multistack/src/ts_inline.rs:24`) — and the result
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
(`reference/Bund/src/stdlib/functions/bund/bund_eval.rs:30`), which is what
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

### Resolution order

`VM::apply` on a `CALL` value
(`reference/rust_multistackvm/src/multistackvm_apply.rs:9-62`):

1. Empty name → error (`:13-15`).
2. `is_command` → `c()` (`:16-17`).
3. Otherwise, **under an `autoadd` branch** (`:19`): if `autoadd` is set the
   name is appended to the value beneath it on the stack (`:20-27`) rather
   than executed. Only in the `else` does resolution continue.
4. `$`-prefix → `call_internal_word` (`:33-35`).
5. Alias resolution (`:39-42`).
6. Lambda (`:46-54`).
7. Inline (`:59`).

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

### One slot table

```rust
pub struct Slot {
    generation: u32,
    entry:      WordEntry,
}

pub enum WordEntry {
    Native { f: NativeFn, effect: StackEffect, kind: WordKind },
    Lambda(BundValue),
    Alias(Symbol),
    Class(BundValue),
    Vacant,
}
```

`Vec<Slot>` indexed by `Symbol`. The seven `HashMap`s collapse into the
discriminant, so a name has exactly one meaning and the namespaces that must
stay distinct — commands, methods, vars — keep separate tables rather than
separate string conventions.

**The `_inline` suffix does not exist.** F31 is not expressible here: there is
no second spelling of a key to get wrong.

`generation` increments when a slot is rewritten. An inline cache records the
generation it was built against, so a redefinition invalidates caches without
a scan — which is what makes D16's permanently-open world affordable.

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
`Registry` shared by `Arc`. The `BUND` global mutex disappears. Runtime
mutation — `register`, `alias` — writes through a per-VM overlay, so D16's
open world survives without a process-wide lock.

This is the change RFC-0007's actor model needs; it is specified here because
the slot table is what makes it possible, not because concurrency is in scope.

### `bund2-api`

The one crate with a stability guarantee: `Symbol`, `StackEffect`, `WordKind`,
`NativeFn`, and the registration surface.

`[BLOCKED: D9]` — whether `Intrinsic` lowerings are exposed here. Exposing
them pins external packages to an exact Cranelift version. The default
recorded is "no"; this RFC does not take it.

`[BLOCKED: D14]` — whether `bund2-api` is one surface or two. If words split
into language core and library, the library half needs a surface that
out-of-tree word packages compile against, and that is a different stability
promise from the core's. The shape of this section depends on the answer.

`[BLOCKED: D29]` — the initial word set. Whether `dup_in`, `from_workbench`,
`push_to` and `stacks_left` get slots at all, and whether `<-` and `←` resolve
to anything.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| Resolution order: command, `$`-internal, alias, lambda, inline | **Preserved exactly**, including the order. |
| The `autoadd` branch preceding all of it | **Preserved exactly.** |
| `$` skipping the lambda check but not aliases (F26) | **Preserved exactly**, including the surprise. The comment is wrong; the behaviour is the contract. |
| Fallthrough from the VM table to the stack table | **Preserved in effect.** One slot table has no two tiers to fall between; the words keep their names and meanings. |
| Last-write-wins registration | **Preserved exactly**, as a generation bump. |
| Open world — `register` / `alias` at runtime | **Preserved exactly.** D16. |
| Alias resolved twice | **Preserved for one level, deliberately changed for chains.** Resolving to a fixed point differs from resolving exactly twice only for an alias chain three deep or more. No such chain exists in the reference's own registrations, and none is constructible in the corpus. Stated as a deviation rather than assumed away. |
| `resolve` failing on stack-layer words (F31) | **Deliberately fixed.** No golden covers `resolve`, so conformance cannot move. |
| `BUND` held for the whole run | **Deliberately changed.** Observably identical for a single-threaded program, which is every program today. |

## Alternatives considered

**Keep string keys, drop the suffix.** Fixes F31 and nothing else: the
thirteen allocations and eight hashes remain, because the cost is in hashing
strings at dispatch, not in the suffix.

**Keep the seven tables, index them by `Symbol`.** Cheaper to write, and it
keeps the tables' independence. Rejected because it preserves the thing that
makes resolution order hard to reason about: seven places a name can live,
consulted in a fixed order that is written out longhand in `apply`.

**Intern at registration only, not at parse.** Halves the benefit. A `CALL`
built at runtime by `bund.eval` still needs interning, so the interner must
be live anyway; interning at parse costs nothing extra.

## Acceptance criteria

1. `cargo xtask conform` does not fall below the mark in
   `tests/golden/CONFORMANCE.txt`. RFC-0002 changes dispatch, not meaning.
2. `cargo xtask coverage` reports its number against the word set D14 and D29
   settle. **The figure cannot be written here** until they do — writing one
   would take their defaults. `[BLOCKED: D14, D29]`
3. Dispatching a word allocates **0** times, measured the way
   `cargo xtask layout` measures the value: a counting allocator across a
   window around one `apply` of a `CALL`.
4. `cargo tree -p bund2-api` lists no `bund2-interp` and no `bund2-jit` — the
   stable surface does not drag the implementation behind it.
5. `"dup_one" resolve` succeeds, and so does `resolve` for each of the 31
   stack-layer words. This is F31's regression test.
6. `cargo xtask cite` reports zero defects.

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
