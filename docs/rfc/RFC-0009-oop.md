# RFC-0009: Classes, objects, and method dispatch

- Status: **Draft**
- Depends on: RFC-0001 (the value), RFC-0002 (symbols and the word table),
  RFC-0003 (the frame loop, for construction's recursion)
- Decisions consumed: D1 (lazy identity — `.id` is the consumer that proves it
  must stay answerable), D2 (lazy stamp, likewise for `.timestamp`)
- Blocked on: nothing. No open decision names RFC-0009.
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- **Oracle caveat.** Every "confirmed against the oracle" claim inherits F21:
  the built oracle links crates.io releases, not the pinned submodules.
  `cargo xtask cite` compares the two `src/` trees byte for byte on every run
  and currently passes, so the claims hold conditionally on that check.

## Summary

A class in Bund is a MAP tagged `CLASS`, carrying `.class_name`, a `.super`
list of **names**, and one slot per method holding a **PTR** into a fifth
name-keyed table. Constructing an object `dup`s the class, flips the tag to
`OBJECT`, and rebuilds `.super` as a list of constructed **parent objects** —
so an instance carries a materialised copy of its whole ancestry. Dispatch then
walks that tree depth-first on **every call**.

This RFC preserves the semantics exactly and changes the representation: a
**flattened per-class method table** resolved once at registration, so dispatch
is a lookup rather than a search, with the **field tree preserved** because
programs read `.super` directly. It also states what `.id` and `.timestamp`
mean on an object, which is the constraint that fixed RFC-0001's value layout.

## Motivation

**Dispatch is a search, and it repeats.** `m()` peeks the top of the stack,
requires an `OBJECT`, and calls `locate_value_in_object`, which tries the
object's own map and then recurses into every element of `.super`
(`reference/rust_multistackvm/src/multistackvm_object.rs:6-24,43-56`). Nothing
is cached. Calling `.println` on an instance of a class three levels deep
re-walks three maps and their lists every time.

**Construction is Rust recursion over the class hierarchy.**
`make_bund_object` calls itself for each superclass
(`reference/rust_multistackvm/src/stdlib/bund_object.rs:50`), evaluates each
`.init` (`:76`, `:156`), then inspects the stack and `apply`s (`:166,169,173`).
That is a **third depth axis** — RFC-0003 §S4 names call depth and parser
nesting depth and defers this one here.

**An object is a deep copy of its ancestry.** `.super` on a class holds names;
on an instance it holds whole constructed objects (`bund_object.rs:36-45`). A
class hierarchy N deep instantiates N objects and copies N maps, once per
instantiation.

**`.id` and `.timestamp` are public API.** They are registered as methods on
the base `Object` class
(`reference/Bund/src/stdlib/functions/oop/base_classes.rs:91-92`) and `.id`
returns a **string** (`:16-17`). ERRATA records this as the constraint that
ruled out shrinking the value to 16 bytes by dropping identity — RFC-0001
resolved it by moving identity into the heap header, where both stay
answerable.

## Current behaviour

### 1. A class is a MAP with three kinds of slot

`Value::make_class()` produces a MAP tagged `CLASS`. `register_object` fills it
(`base_classes.rs:90-101`):

| slot | holds | example |
|---|---|---|
| `.class_name` | a STRING | `"Object"` |
| `.super` | a LIST of **names** | `["Printable"]` |
| method slots | a **PTR** naming a method | `.id` → `` `.id `` |

The base hierarchy is `Object` → `Printable` → `Display`
(`base_classes.rs:95,107`). Note the direction: a class's `.super` names the
class **above** it, and `Object` — the root by name — has `Printable` above it.

**Slot names and method names are not the same.** `Object` maps the slot `.id`
to the method `` `.id ``, but `Printable` maps the slot `str` to the method
`` `.str `` (`base_classes.rs:97-98,112-114`). The leading dot is part of the
method's name in the method table, not part of the slot's.

### 2. Methods live in a fifth name-keyed table

`register_method` inserts into `methods_fun`
(`reference/rust_multistackvm/src/multistackvm_methods.rs:8`), which is
separate from the inline tables, the lambda table, the alias table and the
command table. RFC-0000 counted three live *word* tables; this is not one of
them, and neither is the conditional table RFC-0003 §S7 absorbs.

So a method is reachable only through a class slot: nothing dispatches
`methods_fun` by name from a program.

### 3. Constructing an object materialises the ancestry

`make_bund_object` (`bund_object.rs:26-60`):

1. `dup`s the class — a deep copy under F13's fix, a bincode round trip today.
2. Sets `dt` to `OBJECT` and `.class_name` to the class's name.
3. **Rebuilds `.super`**: for each *name* in the class's `.super`, constructs
   that parent object recursively and pushes the **object** into the list.
4. Evaluates each parent's `.init` lambda (`:76`), then the class's own
   (`:156`).
5. Inspects the stack for an object of the same class and `apply`s
   (`:162-173`).

So an instance's `.super` is a list of objects, where its class's was a list of
strings. The two shapes share a slot name and are read by the same code.

### 4. Dispatch searches the instance

`m(name)` (`multistackvm_object.rs:43-86`):

1. **Peek**, not pull — the receiver stays on the stack for the method to use.
2. Require `OBJECT`; anything else is
   `VM there is no value of type OBJECT in the stack`.
3. `locate_value_in_object`: try `value.get(name)`, then recurse into each
   element of `.super` (`:6-24`). Depth-first, first match wins.
4. The located slot is a `PTR` → look up `methods_fun` and call it; a `LAMBDA`
   → `lambda_eval` (`:77-78`); anything else → push it (`:80-82`).

Step 4's LAMBDA arm is why **`execute_object` re-enters evaluation**, which
RFC-0003 §S4b records and which makes object dispatch a re-entrant native
under §S4a.

### 5. `execute` reaches objects two ways

`!` on a `CLASS` pushes the value and calls `stdlib_object_inline`
(`reference/rust_multistackvm/src/stdlib/bund_execute/execute_class.rs:8-10`)
— that is, executing a class **constructs** an instance. `!` on an `OBJECT`
pulls a **method name** from the main stack, pushes the object back, and calls
`m` (`bund_execute/execute_object.rs:8-20`).

Neither consults `op`, so both take their operands from the main stack whatever
`execute.` was asked for — which RFC-0003 §S4b confirms is the convention.

### 6. The word surface

`oop/` registers ten words (`oop/*.rs`): the built-in classes `True`, `False`,
`Intervals`, `List`, `Floats`, and `wrap` / `unwrap` / `is`
(`value_class.rs:168-170`) and `#` / `#.` (`object_execute.rs:59-60`).

**Almost none of `oop/` is machinery.** Of its 1,183 lines, 948 are those
built-in classes and 235 are `base_classes.rs`, `object_execute.rs` and
`mod.rs`. The dispatch and construction this RFC specifies are the 280 lines of
`bund_object.rs` (193) and `multistackvm_object.rs` (87), in a different
crate.

## Design

### S1. A class keeps its shape; a **flattened table** is added beside it

The class value stays a MAP with `.class_name`, `.super` and method slots,
because programs read those slots directly — `.str` reaches for `.class_name`
(`base_classes.rs:36-42`) and `locate_value_in_object` is exported for library
use (`oop::value_class::locate_value_in_object`).

Registration additionally computes a **flattened method table** for the class:
every slot reachable through `.super`, resolved once, with the depth-first
first-match rule applied at flatten time rather than at call time. Dispatch
becomes one lookup.

The flattening is a **cache over the field tree**, not a replacement for it, in
exactly the sense RFC-0003 §S3 makes BundIR a cache over a lambda body: the
tree is what programs can observe and what they can build, so it stays
authoritative.

### S2. The field tree is preserved, and so is its double shape

An instance's `.super` continues to hold constructed parent objects and a
class's continues to hold names. That asymmetry is observable — `wrap_unwrap`
and the class demos read both — so it is preserved rather than regularised.

### S3. Construction is a frame, not Rust recursion

`make_bund_object`'s self-call becomes a frame push under RFC-0003 §S4, so
class-hierarchy depth stops being Rust stack depth. The `.init` evaluations and
the stack inspection between them make construction a **re-entrant native**
under §S4a: it evaluates, inspects, and evaluates again, which an exit action
cannot express.

### S4. `.id` and `.timestamp` stay answerable, and stay lazy

Both are methods on the base class and both must keep working. RFC-0001 keeps
identity in the heap header rather than inline, so `.id` materialises through
`BundValue::id_string` — one of D1's five needs — and `.timestamp` through
`BundValue::timestamp`, which is D2's.

Neither is a new materialisation point: D20 already lists the needs, and
`.id` is the case that made D1 keep the format rather than the storage.

### S5. Method dispatch gets an inline cache

A `m()` call site caches `(class identity, method name) → resolved method`,
invalidated by the same generation counter RFC-0002 gives the word table. This
is the polymorphic cache the roadmap assigns here; it sits on top of S1's
flattened table, which is what makes the cached value cheap to recompute on a
miss.

### S6. `#` and `!`-on-an-object are the same operation

`#` (`object_execute.rs:59`) and `!` on an OBJECT
(`bund_execute/execute_object.rs`) both pull a method name and dispatch. They
are specified here as one path with two spellings, so the `execute` arm
RFC-0003 §S4b left unimplemented is filled by the same code `#` uses.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| A class is a MAP with `.class_name`, `.super`, method PTR slots | preserved exactly |
| `.super` holds names on a class and objects on an instance | preserved exactly, asymmetry included (S2) |
| Slot name and method name differ (`str` → `` `.str ``) | preserved exactly |
| Depth-first, first-match resolution order | preserved exactly; computed at flatten time (S1) |
| `m()` peeks rather than pulls | preserved exactly |
| Non-OBJECT receiver error text | preserved exactly |
| A `LAMBDA` in a method slot is evaluated; anything else is pushed | preserved exactly |
| `!` on a CLASS constructs; on an OBJECT dispatches | preserved exactly (S6) |
| Construction evaluates every ancestor's `.init` | preserved exactly |
| `methods_fun` as a fifth global table | **changed** to registry state, as RFC-0003 §S7 did for the conditional table |
| Construction recursing in Rust | **changed** to frames (S3) — observable only as hierarchies that no longer overflow |
| Dispatch searching on every call | **changed** to a flattened lookup (S1) — same answer, computed once |
| `.id` returns a 21-character string | preserved exactly (S4) |

No deviation here changes an answer. The two changes are representation and
recursion strategy; both are required to produce the same slot for the same
name, and the acceptance criteria say so.

## Alternatives considered

**Flatten the field tree as well.** Rejected. `.super` is readable by programs
and `locate_value_in_object` is exported for library use, so collapsing the
tree into the flattened table would change what a program sees when it walks an
object.

**Resolve methods at construction rather than registration.** Rejected: an
instance would carry its own table, paying the flattening cost per object
instead of per class, and object construction is already the expensive
operation.

**Make `.super` hold names at both levels.** Tempting, and rejected under S2:
the asymmetry is observable, and regularising it is a language change nobody
has asked for.

**Keep `methods_fun` global.** Rejected for the reason RFC-0002 rejected the
`BUND` mutex and RFC-0003 §S7 rejected `CF`: a table filled at startup and read
on every dispatch does not need process-wide mutable state, and having it
prevents more than one VM.

## Acceptance criteria

1. **Conformance does not regress and the OOP goldens move.** `cargo xtask
   conform` must not drop below its recorded baseline, and the **15** goldens
   whose first unimplemented word is `class` (11) or `object` (4) must either
   pass or fail on a word this RFC does not implement. Measured by `cargo xtask
   conform`; the split is measured, not estimated.
2. **Dispatch resolves the same slot as a search would.** A differential test
   over the built-in hierarchy and the corpus's own classes: for every class
   and every method name reachable from it, the flattened table's answer equals
   `locate_value_in_object`'s. The search implementation is kept in the test as
   the oracle for the table.
3. **Class-hierarchy depth is bounded by heap, not by the Rust stack.** A class
   chain 10,000 deep instantiates or reports a Bund-level error, within a
   60-second wall clock, without a stack overflow. This is `cargo xtask
   depth`'s third axis, alongside RFC-0003 criterion 2's call depth.
4. **`.id` and `.timestamp` answer on an object**, `.id` as a 21-character
   string, and neither materialises anything on a value that is never asked.
   Pinned as probes under `tests/probes/` per D21.
5. **`methods_fun` is registry state.** No global holds it, and two `Interp`s
   in one process can register different methods under the same name.
6. **`cite` and `lint` clean**, with the load-bearing citations quoted as
   fenced blocks so `cite` verifies their content — RFC-0003's criterion 7
   found that a document with none passes while citing wrong lines.

## Open questions

- **The `.super` direction reads backwards.** `Object`'s `.super` is
  `["Printable"]` and `Printable`'s is `["Display"]`
  (`base_classes.rs:95,107`), so the class named `Object` is not the root of
  its own hierarchy. Whether that is intended or an artefact is not settled;
  the resolution order is preserved either way, and nothing here depends on the
  answer. Worth a register entry if a program is found relying on it.
- **`dup` in construction is F13's path.** `make_bund_object` `dup`s the class
  (`bund_object.rs:29-32`), and F13's fix makes `dup` a structural clone with a
  **fresh identity**. So every instance's class-copy has an identity distinct
  from the class's. Nothing observed depends on it; stated because D35 keyed a
  cache on exactly this distinction.
- **The built-in classes are not scoped here.** `Intervals`, `List`, `Floats`,
  `True`, `False` and the `Display`/`Printable` chain are 948 of `oop/`'s
  1,183 lines and are library, not machinery. D14's core/library split governs
  which of them Bund2 ships; this RFC specifies the mechanism they use.
- **F66 applies to the OOP goldens.** Two of the four unreproducible goldens
  are `execute-arm-class` and `execute-arm-not-executable`, so criterion 1's
  denominator is understated by that much.
