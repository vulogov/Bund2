# RFC-0009: Classes, objects, and method dispatch

- Status: **Draft**
- Depends on: RFC-0001 (the value), RFC-0002 (symbols and the word table),
  RFC-0003 (the frame loop, for construction's recursion)
- Decisions consumed: D1 (lazy identity — `.id` is the consumer that proves it
  must stay answerable), D2 (lazy stamp, likewise for `.timestamp`),
  **D23** (`<class> !` builds from the CLASS *value*, never a registry lookup),
  **D25** (an anonymous class must carry its own `.class_name`, or construction
  fails). Both complete **F16**, whose disposition is FIX.
- **Method note.** An earlier draft consumed only D1 and D2, because it searched
  the register for decisions declaring `Blocks: RFC-0009` and found none. D23
  says `Blocks: nothing`. A decision about an RFC's *subject* need not name the
  RFC, so the register must be searched by subject as well.
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
(`base_classes.rs:95,109`).

**`.super` reads upward, conventionally.** `create_class_hierarhy_demo.bund:21`
sets `".super" [ :A ]` on class `B`, so `A` is `B`'s parent. What is
unconventional is only the base chain's naming: `Display` is its root and
`Object` is the most derived of the three. That costs nothing, because a user
class inherits from `Object` and reaches the whole chain through it —
confirmed: a class with `".super" [ :Object ]` answers `.id`. Preserved as
named; renaming would break any program that spells `Printable` or `Display`
in a `.super`.

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

`make_bund_object` (`bund_object.rs:27-113`):

1. `dup`s the class — a deep copy under F13's fix, a bincode round trip today.
2. Sets `dt` to `OBJECT` and `.class_name` to the class's name — and **this is
   where the instance's identity comes from**, not from step 1. `set` on a
   map-like tag builds a fresh value through `Value::from_dict`
   (`reference/rust_dynamic/src/set.rs:14-27`), which mints a new id and stamp
   rather than carrying the `dup`'s
   (`reference/rust_dynamic/src/create_map.rs`). An earlier draft credited the
   identity to `dup`; the observable fact — two instances of one class return
   different ids, confirmed against the oracle — holds either way, but the
   reason matters for D35, whose cache keys on the payload pointer that this
   write replaces.
3. **Rebuilds `.super`**: for each *name* in the class's `.super`, constructs
   that parent object recursively and pushes the **object** into the list.
4. Evaluates each parent's `.init` (`:72-80`), which is a **PTR** in every
   built-in class — `.bool_init`, `.float_init`, `.list_init` and the rest
   (`reference/Bund/src/stdlib/functions/oop/bool_class.rs:38` and siblings) —
   with a LAMBDA arm beside it.

Steps 5 and 6 belong to `stdlib_object_inline`, not to `make_bund_object`; an
earlier draft cited them as if they were the same function.

5. Evaluates the object's own `.init` (`:134-160`).
6. Tests `if_object_of_class_in_stack` and `apply`s (`:162-173`) — so an
   `.init` **may replace the object being constructed** with one already on the
   stack.

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

**`!` on a `CLASS` always fails — F16.** `execute_class` pushes the class
**value** and calls `stdlib_object_inline`
(`reference/rust_multistackvm/src/stdlib/bund_execute/execute_class.rs:8-10`),
and that function immediately `cast_string()`s its operand to obtain a class
*name* (`reference/rust_multistackvm/src/stdlib/bund_object.rs:119-122`), then
looks the name up with `is_class` / `get_class`. A CLASS value is a MAP, so the
cast fails and the arm errors for the only type that routes to it.

An earlier draft of this section said the arm "constructs an instance" and
filed it as preserved. It does not construct anything, and the deviation this
RFC's own territory requires was recorded as a non-deviation.

`!` on an `OBJECT` does work: it pulls a **method name** from the main stack,
pushes the object back, and calls `m`
(`bund_execute/execute_object.rs:8-20`).

Neither consults `op`, so both take their operands from the main stack whatever
`execute.` was asked for — which RFC-0003 §S4b confirms is the convention.

### 6. The word surface

`oop/` registers ten words (`oop/*.rs`): the built-in classes `True`, `False`,
`Intervals`, `List`, `Floats`, and `wrap` / `unwrap` / `is`
(`value_class.rs:168-170`) and `#` / `#.` (`object_execute.rs:59-60`).

**D14 already splits them, and the split runs exactly along this RFC's seam.**
`cargo xtask scope` puts the machinery in **core** — `class`, `object`, `#`,
`#.`, `is`, `wrap`, `unwrap`, `?class`, `?object` — and the built-in classes in
**library**: `True`, `False`, `Intervals`, `List`, `Floats`. `resolve.class` is
**library** too (`docs/core-words.md`, `vm/lambdas`); an earlier draft listed it
as core. So this RFC specifies what core needs and the built-in classes are
deferrable word packages, with no further decision required.

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

A **flattened method table** is computed beside it: every slot reachable
through `.super`, resolved once, with the depth-first first-match rule applied
at flatten time rather than at call time. Dispatch becomes one lookup.

**Flattening happens at first construction, not at registration.** An earlier
draft said registration, and §S7 two pages later said parents resolve at
construction — a contradiction the RFC carried rather than resolved.
Registration is the wrong moment: `register_class` validates only the CLASS tag
and inserts (`reference/rust_multistackvm/src/multistackvm_classes.rs:7-20`),
so a class may be registered whose parents are not. Confirmed against the
oracle — `:B class ".super" [ :A ] set register` succeeds with `A` absent.
Flattening there would reject it and **narrow the language**.

Construction is the right moment because it is already where an unregistered
parent fails (§S7). The table is memoised on the class and invalidated by the
class registry's generation (§S5), so it is computed once per class per
registration epoch rather than once per instance.

The flattening is a **cache over the field tree**, not a replacement for it, in
exactly the sense RFC-0003 §S3 makes BundIR a cache over a lambda body: the
tree is what programs can observe and what they can build, so it stays
authoritative.

### S1a. A per-class table is not enough: slots are rewritten per **object**

`set_value_in_object` walks the same `.super` tree the search walks and
**rewrites a slot in one object**, rebuilding each parent it passes through
(`reference/Bund/src/stdlib/functions/oop/value_class.rs:31-52`). `wrap` uses
it. So after a `wrap`, one instance's tree differs from its class's, and a
table flattened per class would answer for the class where the search answers
for the object.

The flattened table therefore serves as a **fast path with a guard**: it is
consulted only while an object's tree is known to match its class's, and any
per-object rewrite marks that object as diverged and sends its lookups back to
the search. Marking is cheap because `set_value_in_object` is the only writer.

An earlier draft missed this, and its criterion 2 quantified over classes
alone, so it could not have caught the disagreement.

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

A `m()` call site caches `(class, method name) → resolved method`. Two details
an earlier draft got wrong:

**RFC-0002's generation counter does not cover this.** It is per word-table
slot, and classes live in their own registry
(`reference/rust_multistackvm/src/multistackvm.rs:28`) while methods live in
`methods_fun` — two tables the word table's counter never sees. The class
registry and the method table each need their own generation, bumped by
`register_class` and `register_method`, and the cache keys on both.

**The key is not class *identity*.** Keying on identity would force the lazy
materialisation D35 rejected for the compiled cache, for the same reason: it
puts a mint on the hottest path. The cache keys on the class's **`Rc` pointer**,
as D35 does, which is stable for a registered class and costs nothing.

The cache sits on top of S1's flattened table, which is what makes a miss cheap
to refill.

### S6. `#` and `!`-on-an-object are **different** operations

An earlier draft of this section claimed they were one path with two spellings.
They are not, and probing settled it.

- **`!` on an OBJECT dispatches a named method.** It pulls a method-name
  *string* from the main stack, pushes the object back, and calls `m`
  (`bund_execute/execute_object.rs:8-20`). Confirmed:
  `:.id :A object !` returns the instance's id.
- **`#` runs supplied code against the object's unwrapped value.** It pulls a
  **LAMBDA or PTR** — a name string is rejected with `# NO LAMBDA or PTR IN #1`
  — pulls the object, pushes it, calls `unwrap`, pushes the code, and calls `!`
  (`object_execute.rs:11-39`).

So `#` is not dispatch at all: it is `unwrap`-then-apply, and its `!` is the
ordinary polymorphic execute on whatever `unwrap` produced. Bund2 implements
them separately, and only the first fills the `execute` arm RFC-0003 §S4b left
open.

### S7. `<class> !` constructs, from the value — D23 and D25

F16's disposition is FIX and D23 resolves the shape: **build from the CLASS
value the stack holds, never by looking a name up in the registry.** Both
provenances behave identically — a class resolved back onto the stack, and one
constructed dynamically and never registered.

The registry is still consulted for **parents**: `.super` holds parent class
*names* and construction walks them
(`reference/rust_multistackvm/src/stdlib/bund_object.rs:44-48`), so a class
whose parents are unregistered still fails, on the parents.

**D25 supplies the name.** A CLASS value does not know its own name —
`stdlib_class_inline` creates a bare class with only `.super` set
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:69-73`) and `register`
takes the name from *beneath* the class on the stack, which is why the idiom is
`:Name class … register`. So `<class> !` takes `.class_name` from the class
value's own attribute, and **fails at that point** if there is none, rather
than producing an object without one. Every built-in class already sets it
(`base_classes.rs:96,110`).

This is the only behaviour in this RFC that changes an answer, and it adds one
where there was an error: nothing can depend on the present behaviour, because
the arm cannot succeed.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| A class is a MAP with `.class_name`, `.super`, method PTR slots | preserved exactly |
| `.super` holds names on a class and objects on an instance | preserved exactly, asymmetry included (S2) |
| Slot name and method name differ (`str` → `` `.str ``) | preserved exactly |
| Depth-first, first-match resolution order | preserved exactly; computed at flatten time (S1) |
| `set_value_in_object` rewrites a slot per object, rebuilding the parents it passes | preserved exactly; diverges that object from its class (S1a) |
| `.init` is a **PTR** in every built-in class, not a LAMBDA | preserved exactly — both arms exist and the PTR one is the common case |
| `.init` may replace the object under construction, via `if_object_of_class_in_stack` | preserved exactly |
| `set` on a map returns a **new** value: fresh id, fresh stamp, `q` reset, `attr` and `tags` dropped (`reference/rust_dynamic/src/set.rs:14-27`) | preserved exactly — so construction's identity comes from the `.class_name` write, not from `dup`, and `.timestamp` tracks the last write |
| `#` swallows the result of the words it applies (`object_execute.rs:35-38` discards both `apply` results) | **must be specified** — an error inside `#` is lost, which no row previously admitted |
| An `.init` PTR naming an unregistered method is **silently skipped** | **must be specified** — construction succeeds with an uninitialised object |
| An `.init` may substitute a parent object already on the stack, via `if_object_of_class_in_stack` (`bund_object.rs:162-173`) | preserved exactly — and it mutates the very tree S1's table precomputes, so it marks the object diverged (S1a) |
| `register_class` validates only the CLASS tag, so a class may be registered whose parents are not (`multistackvm_classes.rs:7-20`) | preserved exactly — which is why S1 flattens at construction, not registration |
| F11's inverted guard sits in the `Value` class's `.init`, on the construction path, with an **empty disposition** | **carried** — this RFC cannot preserve or fix a behaviour whose disposition nobody has taken |
| `m()` peeks rather than pulls | preserved exactly |
| Non-OBJECT receiver error text | preserved exactly |
| A `PTR` in a method slot resolves through `methods_fun` and is **called**; a `LAMBDA` is evaluated; anything else is pushed (`multistackvm_object.rs:57-82`) | preserved exactly |
| `!` on a CLASS **always errors** (F16) | **deliberately changed** — D23/D25; see S7 |
| `!` on an OBJECT dispatches a named method | preserved exactly (S6) |
| `#` is `unwrap`-then-apply, and rejects a name string | preserved exactly (S6) |
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
   conform` must not drop below its recorded baseline.

   **The affected set, stated precisely**, because an earlier draft conflated a
   directory with a measurement: `tests/golden/examples/object_oriented_programming/`
   holds **14** goldens, and `probes/execute-arm-class.golden` is a fifteenth
   golden in this RFC's territory. Bund2 currently stops on `class` in 11 of
   the 15 and on `object` in 4.

   **The mechanism, because the last two attempts were not checkable.**
   `conform -v`'s failure list reports only `output differs (N lines vs M)` —
   grepping it for `class not registered` finds nothing, today and after, so
   that wording passed vacuously. Errors also exit 0 under D36, so the exit
   code says nothing either.

   This RFC adds `cargo xtask conform --blocked-on`, which runs each failing
   golden through `bund2` and reports the **first `… not registered` word** its
   diagnostic names. The criterion: after this RFC lands, `--blocked-on` names
   neither `class` nor `object` for any golden. The tool does not exist yet and
   is part of this RFC's work, as `xtask parity` was RFC-0003's.

   *(Measured today by running `bund2` directly over the set: 11 stop on
   `class` and 4 on `object`. A reviewer measured 10/5; the per-file listing
   supports 11/4, with `execute-arm-class` stopping on `class`.)*
2. **Dispatch resolves the same slot as a search would — over *objects*, not
   only classes.** A differential test over the built-in hierarchy and the
   corpus's own classes: for every class, every method name reachable from it,
   **and every object after a `wrap`**, the flattened table's answer equals
   `locate_value_in_object`'s. The object quantifier is the point: S1a's
   per-object rewrites are exactly what a class-only comparison cannot see. The
   search implementation is kept in the test as the oracle for the table.
3. **Class-hierarchy depth is bounded by heap, not by the Rust stack.** A class
   chain 10,000 deep instantiates or reports a Bund-level error, within a
   60-second wall clock, without a stack overflow. Decided by `cargo xtask
   depth`, **which now exists** and carries this as its `class` axis alongside
   RFC-0003 criterion 2's `call`. It reports `unsupported` today, because a
   feature that is not implemented fails cleanly for a reason that has nothing
   to do with depth and must not read as a pass.
4. **`.id` and `.timestamp` answer on an object**, `.id` as a 21-character
   string, and neither materialises anything on a value that is never asked.
   Pinned as probes under `tests/probes/` per D21.
5. **`methods_fun` is registry state.** No global holds it, and two `Interp`s
   in one process can register different methods under the same name.
6. **`cite` and `lint` clean**, with the load-bearing citations quoted as
   fenced blocks so `cite` verifies their **content** and not merely that the
   line exists. RFC-0003's criterion 7 found that a document with no fenced
   blocks passes while citing wrong lines, and this RFC's first two reviews
   found six such lines between them.

   The four this design rests on:

   ```rust reference/rust_multistackvm/src/multistackvm_classes.rs:8-10
        if ! bclass.is_type(CLASS) {
            bail!("register: argument #1 is not a CLASS");
        }
   ```

   ```rust reference/rust_multistackvm/src/multistackvm_object.rs:57-60
            PTR => {
                match method_value.cast_string() {
                    Ok(method_name) => {
                        if self.is_method(method_name.clone()) {
   ```

   ```rust reference/rust_dynamic/src/set.rs:11-13
            LAMBDA => {
                return Value::to_lambda(vec![value]);
            }
   ```

   ```rust reference/rust_multistackvm/src/stdlib/bund_object.rs:119-122
        Some(name_value) => {
            match name_value.cast_string() {
                Ok(name) => {
                    if vm.is_class(name.clone()) {
   ```

## Open questions

- **F11 has no disposition.** `if ! value.type_of() == OBJECT` in the `Value`
  class's `.init` (`reference/Bund/src/stdlib/functions/oop/value_class.rs`) is
  the same precedence bug as F55 and F60's guard, and it sits **on the
  construction path this RFC specifies**. Its register entry ends
  `Behavioural. Disposition:` with nothing after it. This RFC cannot state
  whether the guard is preserved or fixed until someone takes that disposition.
- **F67 is this RFC's own, and is not yet in its preservation analysis.** The
  first review produced it — a missing parent reported under the child's name
  (`bund_object.rs:99`) — and the disposition is "Bund2 names the parent". It
  belongs in a row once the construction section is implemented.
- **F37 makes one of F16's citations point at dead code.**
  `stdlib/classes/registry.rs` is never compiled, and F16's text cites it for
  `register`'s operand order. The live path is
  `reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:20`. This RFC
  cites the live one; F16's own text should be corrected.

Three questions this RFC opened were settled by probing while it was in draft,
and are now stated as facts above rather than carried: the `.super` direction
(Current behaviour §1), `dup`'s fresh identity per instance (§3), and D14's
core/library split of the OOP words (§6). One remains, and it is not this
RFC's to close.

- **F66 applies to `execute-arm-not-executable`, but not to
  `execute-arm-class`.** An earlier draft planned to record both as F66
  deviations. That is wrong for the second: `execute-arm-class` captures the
  error F16 records, and after F16's FIX the arm **constructs** instead of
  erroring, so the probe stops proving what it was written to prove. D25's
  follow-on already says it must be reshaped. Reshape it as the proof that
  `<class> !` builds an object; do not preserve its error under F66.

  `execute-arm-not-executable` is a genuine F66 case and can be recorded with
  `cargo xtask conform --accept-deviation … --reason F66` once D36's error
  presentation stops moving — a hash of output about to change would only
  produce a drift.
