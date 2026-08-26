# RFC-0001: `BundValue` — representation, identity, and value semantics

- Status: Draft
- Depends on: RFC-0000
- Decisions consumed: D1, D2, D4, D13, D20
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: `docs/research/05-rfc-roadmap.md` §6's "24-byte
  `{tag, payload, birth}`" row — the measured shape is 16 bytes and the tag
  does not live in the value. Recorded in `docs/research/ERRATA.md`.

## Summary

`BundValue` is a 16-byte, two-word value: an inline scalar, or an `Rc` to a
heap object that carries the payload, the reference's `dt` type tag, and a
lazily-minted identity. Mutation is clone-on-write through `Rc::make_mut`,
whose split points coincide exactly with the points where the reference mints
a fresh id (D13). Integers stay full `i64` (D4). The measured figures are
`cargo xtask layout`; the RFC is accepted against them, not against argument.

## Motivation

Three costs, all measured or cited rather than asserted.

**The value is 176 bytes.** `Value` carries eight fields — `id: String`,
`stamp: f64`, `dt: u16`, `q: f64`, `data: Val`, `attr: Vec<Value>`,
`curr: i32`, `tags: HashMap<String, String>`
(`reference/rust_dynamic/src/value.rs:16-25`). `cargo xtask layout` measures a
field-for-field replica at **176 bytes**. Every stack slot, every list
element, every map entry pays that.

**Every construction mints a nanoid and reads the clock.** `Value::new` sets
`id: nanoid!()` and `stamp: timestamp_ms()`
(`reference/rust_dynamic/src/value.rs:35-36`), and so does every constructor
beside it — `from_float` (`create.rs:10-11`), `from_int` (`:34-35`),
`from_bool` (`:58-59`). `timestamp_ms` is a `SystemTime::now()` syscall
(`reference/rust_dynamic/src/value.rs:7-9`). Constructing the integer `1`
allocates a 21-character string and calls the clock.

**`dup` is a serialise-then-deserialise round trip.** `Value::dup` calls
`to_binary()`, then `Value::from_binary()`, then mints a fresh id
(`reference/rust_dynamic/src/dup.rs:7-12`). The Bund word `dup` is an alias
for `dup_one` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:18`),
which reaches `TS::dup_in_current_stack`
(`reference/rust_multistack/src/ts_stack_op.rs:30`) and calls `val.dup()` in
its loop (`:34`). So **every `dup` in a Bund program bincode-encodes the value
and decodes it again** — 55 dup-family tokens across 38 of the 138 corpus
programs. `Value::set` does the same on every map write
(`reference/rust_dynamic/src/set.rs:41`).

None of this is a design; it is what deep-copy value semantics cost when
implemented by copying deeply.

## Current behaviour

This section is the preservation contract.

### The type tag and the payload are independent axes

`dt` is a `u16` over 40 named constants
(`reference/rust_dynamic/src/types.rs:15-56`). `Val` is a 20-arm enum
(`reference/rust_dynamic/src/types.rs:66-87`). **They do not correspond.** One
`Val` arm carries many `dt` tags:

| payload | `dt` tags written with it |
|---|---|
| `Val::Map` | `CLASS`, `CONDITIONAL`, `CONFIG`, `CURRY`, `INFO`, `MAP`, `OBJECT` |
| `Val::String` | `CALL`, `CONTEXT`, `JSON_WRAPPED`, `PTR`, `STRING`, `TEXTBUFFER` |
| `Val::List` | `CFLOAT`, `CINTEGER`, `LIST`, `RESULT` |
| `Val::Queue` | `FIFO`, `QUEUE` |
| `Val::Binary` | `BIN`, `ENVELOPE` |

The distinction is behavioural, not cosmetic: `PTR` is a name that executes
and `STRING` is text that does not, and both are `Val::String`. Dispatch is on
`dt`, not on the payload — `Value::get` matches
`MAP | INFO | CONFIG | ASSOCIATION | CURRY | MESSAGE | CONDITIONAL | OBJECT | CLASS`
(`reference/rust_dynamic/src/get.rs:7`), a list of tags that all share one
payload arm.

This is visible in the goldens, not only in source: `dt: 6` (`CALL`) appears
43 times and `dt: 7` (`PTR`) 30 times across the captured corpus, both with
`String(...)` payloads.

### Identity is minted eagerly, and again on every mutation

Every constructor sets `id: nanoid!()` (`reference/rust_dynamic/src/create.rs:10`
and each sibling). Every mutating operation returns a **new** value with a
**fresh** id rather than mutating in place —
`reference/rust_dynamic/src/set.rs:16,31,43,58,76,91`,
`reference/rust_dynamic/src/push.rs:165`,
`reference/rust_dynamic/src/attr.rs:7,13,19`. `set` takes `&mut self` and
returns `Self` (`reference/rust_dynamic/src/set.rs:6`) — a copy-returning API
wearing a mutable signature. `dup` mints a fresh id explicitly
(`reference/rust_dynamic/src/dup.rs:11`), as does `unwrap`
(`reference/rust_dynamic/src/bincode.rs:95`).

### Equality is by content; hashing is by identity

`PartialEq` compares content for like types — strings by text
(`reference/rust_dynamic/src/eq.rs:32`), integers by value (`:10`), floats by
value (`:21`) — and falls back to `self.id == other.id` only when the types
differ (`:15`, `:26`, `:34`). Integers and floats compare across the type
boundary by casting (`:13`, `:24`).

`Hash` hashes **only the id** (`reference/rust_dynamic/src/hash.rs:6`).

The two disagree, and `Val::ValueMap` is a `HashMap<Value, Value>`
(`reference/rust_dynamic/src/types.rs:79`) keyed by exactly that type. See
F30. It is unobservable today only because there is no read path at all — F29.

### Four fields are observable but never vary

`q`, `attr`, `curr` and `tags` have no Bund word that reads them back.
`attribute` and `tag` write `attr` and `tags`
(`reference/rust_multistackvm/src/stdlib/values/value_tag.rs:71-72`); nothing
reads either, and no corpus program uses either word. `q` and `curr` have no
word at all.

They are still **observable**, because `debug.display_stack` prints the Rust
`Debug` rendering and the goldens capture it. **27 of the 63 goldens** contain
raw `Value { ... }` text — 244 renderings in total. Across all 244:

- `q: 100.0` — every one
- `attr: []` — every one
- `curr: -1` — every one

`tags` is the exception and does vary: `TS::push` sets
`value.set_tag("stack", &curr.stack_id())` on **every push**
(`reference/rust_multistack/src/ts_push.rs:25`, and `push_to_stack` at `:51`),
so a value on the main stack renders `tags: {"stack": "main"}`.

### Serialisation materialises identity, except for JSON

`to_binary` serialises the concrete value (`reference/rust_dynamic/src/bincode.rs:30`)
and `from_binary` restores it verbatim (`:71`), so the wire format is
byte-identical to the reference and the round trip preserves the id — for
every type **except JSON**. A JSON value is converted to a string and wrapped
(`:9-28`), and `from_binary` re-parses it through `Value::json` (`:54-69`),
which mints a fresh id with `nanoid!()`
(`reference/rust_dynamic/src/create_special.rs:205,207`). This is D20, and its
scope correction.

Because `dup` is built on this pair, `dup` of a JSON value is a round trip
through text.

## Design

### The value

```rust
pub enum BundValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Nodata,
    Heap(Rc<HeapValue>),
}
```

**16 bytes, align 8**, measured — `cargo xtask layout`, candidate D. Two
machine words: a discriminant and a payload. A scalar never touches the heap;
`layout` measures 0 allocations for constructing and for cloning one.

### The heap object

```rust
pub struct HeapValue {
    id:      Cell<u64>,     // 0 until observed, then minted
    stamp:   Cell<f64>,     // 0.0 until observed, then sampled
    dt:      u16,           // the reference's tag, verbatim
    payload: Payload,
}
```

Identity lives here, so it is shared across clones exactly as the `Rc` is,
which is what D1 requires; `Cell` because equality and hashing take `&self`.
`dt` lives here too, and this is the point candidate D was built to test: the
tag is only ever ambiguous for heap types — `NONE`, `BOOL`, `INTEGER` and
`FLOAT` are one-to-one with their payloads — so it rides in the header and the
value stays 16 bytes. The header grows from 72 to 80 as a result.

### Value semantics

`Rc::make_mut` clone-on-write (D13). This is not an approximation of the
reference's deep copy: **its split points coincide exactly with the points
where the reference regenerates the id**, because every mutating operation
already returns a new value with a fresh identity (cited above). Cycles stay
impossible by construction, as they are today.

`dup` becomes an `Rc` bump. That is the single largest behavioural-cost change
in this RFC and it changes no observable result, because the reference's `dup`
mints a fresh id and CoW mints one at the same moment.

### Integers

Full `i64` (D4). NaN-boxing would reach 8 bytes by folding the tag into unused
float bits, at the cost of capping integers at 51 bits. `Val::I64(i64)`
(`reference/rust_dynamic/src/types.rs:72`) and `cast_int` returning `i64`
(`reference/rust_dynamic/src/cast.rs:17`) make that a narrowing of an
observable range. Representation is private behind this type's API and can
change later; integer width cannot.

### Rendering

`q`, `attr` and `curr` are **not fields**. They are rendered as the constants
the corpus shows them to be — `q: 100.0`, `attr: []`, `curr: -1` — by the
`Debug` implementation, which must reproduce the reference's text exactly,
because 27 goldens capture it. `tags` **is** a field, because it varies: the
stack tag is written on every push.

This is the one place where preservation is of a text format rather than of a
behaviour, and it is stated here so it is not discovered by a golden failure.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| `dt` and payload as independent axes | **Preserved exactly.** `dt` is carried verbatim as a `u16` over the same constants. |
| Fresh id per construction | **Preserved observably, changed mechanically.** The id is minted on first observation rather than at construction. Nothing can tell: no word reads an id without observing it. D1. |
| Fresh id per mutation | **Preserved exactly.** `Rc::make_mut` splits where the reference re-mints. D13. |
| `stamp` per construction | **Preserved observably, changed mechanically.** Sampled on first observation. D2. |
| Equality by content, cross-casting int/float | **Preserved exactly.** |
| Hash by identity | **Not preserved — deliberately open.** See F30 and Open questions. |
| `q`, `attr`, `curr` rendered as constants | **Preserved as text.** Not carried as state; the `Debug` rendering reproduces them. Checkable: 244 renderings, all identical. |
| `tags`, including the per-push stack tag | **Preserved exactly**, as a field. |
| Binary wire format | **Preserved exactly**, byte-identical. D20. |
| JSON round trip losing identity | **Preserved exactly**, including the loss. D20's scope correction. |
| `dup` as a bincode round trip | **Deliberately changed** to an `Rc` bump. Observably identical: both yield an equal value with a fresh identity. |
| `push_to_stack` capacity check | **Deliberately fixed** — F28. No golden exercises it; `ensure_stack_with_capacity` has zero corpus uses. |
| `valuemap` write-only | **Blocked.** F29 and F30 — see Open questions. |

## Alternatives considered

**Identity inline on every value** (candidate B, 32 bytes). What the
representation must look like if `id` and `stamp` cannot move off the value.
The id/stamp scan established they can. Carrying identity inline costs 16
bytes per value against candidate A — measured, not estimated.

**Identity inline as one packed token** (candidate C, 24 bytes). The cheapest
inline design, carried only to bound how much of B's cost is the stamp. It is
8 bytes cheaper than B and still 8 dearer than A.

**NaN-boxing** (8 bytes). Rejected by D4, on the grounds recorded there:
the large win is already banked, and 51-bit integers are a semantic cost that
a representation change cannot later undo.

**Folding `dt` into the payload enum** (candidate A, 16 bytes, no tag).
Rejected on evidence: it cannot distinguish `PTR` from `STRING` from `CALL`,
which are three behaviours over one payload. Candidate D shows the tag costs
the value nothing.

## Acceptance criteria

1. `size_of::<BundValue>()` is **16** and `align_of` is **8**, asserted in a
   unit test in `bund2-value`, and `cargo xtask layout` reports the same for
   candidate D.
2. Constructing and cloning a scalar allocate **0** times, measured by the
   counting allocator in `cargo xtask layout`.
3. `cargo xtask conform` does not fall below the mark in
   `tests/golden/CONFORMANCE.txt`. RFC-0001 implements a value, not a word, so
   the number may rise; it may not fall.
4. `cargo tree -p bund2-value` lists no `bund2-interp` — the value is usable
   without a VM. **This criterion is vacuous until `bund2-value` has
   dependencies at all**, exactly as RFC-0000's D-2 records; it becomes real
   with this RFC's implementation and is listed here because this is the RFC
   that makes it so.
5. The `Debug` rendering of a value reproduces the reference's text for the
   244 renderings the goldens capture, including `q: 100.0`, `attr: []` and
   `curr: -1` as constants.
6. `cargo xtask cite` reports zero defects.

## Open questions

- **F29 and F30 together block `valuemap`.** `get` has no `VALUEMAP` arm and
  returns the whole map through its catch-all
  (`reference/rust_dynamic/src/get.rs:18-19`); `?key` always answers false
  (`reference/rust_dynamic/src/has_key.rs:19-20`). Adding the arm does not fix
  it, because `Hash` keys on identity while `PartialEq` compares content, so
  an equal-but-freshly-built key cannot be found. **A decision is required**:
  either `BundValue` hashes by content, which changes `ValueMap` behaviour
  that no program can currently observe, or `ValueMap` keys by identity and
  equal-looking keys stay distinct. This RFC does not take either. It is the
  one thing that would block acceptance.
- **D14** is OPEN and governs which words exist; it does not affect this RFC's
  representation.
- The `q` field is written by `set` on the non-map path
  (`reference/rust_dynamic/src/set.rs:61`), so it is not literally a constant
  in source even though all 244 golden renderings show `100.0`. If any program
  can drive it off `100.0`, criterion 5 fails and `q` becomes a field.
  `[UNGROUNDED]` — no such program is known; recorded as Q18.
