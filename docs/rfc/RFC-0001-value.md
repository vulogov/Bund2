# RFC-0001: `BundValue` — representation, identity, and value semantics

- Status: Draft — revised after review 1 (2026-08-26); the F29/F30 decision still blocks acceptance
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
heap object carrying the payload, the reference's `dt` tag, the observable
state (`tags`, `attr`, `curr`) and a lazily-minted identity. Mutation is
clone-on-write through `Rc::make_mut`; every site where the reference mints a
fresh id is a split point (D13). `dup` is **not** a bare `Rc` bump — it clears
the identity slot on a fresh header, because D1's contract is clone-equal
versus dup-unequal. Integers stay full `i64` (D4). The measured figures are
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
and decodes it again** — 55 dup-family tokens across 38 of the 132 corpus
programs. `Value::set` does the same on every map write
(`reference/rust_dynamic/src/set.rs:15`).

None of this is a design; it is what deep-copy value semantics cost when
implemented by copying deeply.

## Current behaviour

This section is the preservation contract.

### The type tag and the payload are independent axes

`dt` is a `u16` over 42 named constants
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

### Equality is identity, for all but four payload kinds

`PartialEq` compares content for exactly **four** of the twenty `Val` arms:
`I64` (`reference/rust_dynamic/src/eq.rs:10`), `F64` (`:21`), `String`
(`:32`) and `Time` (`:40`). Integers and floats also compare across the type
boundary by casting (`:13`, `:24`), and a mismatched scalar pair falls back to
`self.id == other.id` (`:15`, `:26`, `:34`, `:42`).

**Every other payload kind reaches the catch-all at `:45`**, which tests `dt`
for `CINTEGER` and `CFLOAT` and otherwise returns `self.id == other.id`
(`:53`). That is sixteen kinds — including `Bool`, `List`, `Map`, `Lambda`,
`ValueMap` and `Json`. For all of them **`==` *is* identity comparison**: two
structurally identical lists are unequal unless they share an id.

This is what D1 already recorded, and an earlier draft of this RFC got it
wrong — it said the id fallback fires "only when the types differ". It does
not. D1 and D13 both rest on `eq.rs:53`, and the design below depends on it.

`Ord::cmp` has the same shape: content for `I64`, `Time` and `String`, and
`self.id.cmp(&other.id)` otherwise
(`reference/rust_dynamic/src/ord.rs:175,183,191,199`). So **ordering reads the
id too**, which D1 names as one of three internal readers and an earlier draft
of this section omitted entirely. F12 records that this path is inconsistent
with `PartialOrd`'s `lt` (`reference/rust_dynamic/src/ord.rs:16,24`, which
returns `true` for any mismatch) and currently unreachable.

`Hash` hashes **only the id** (`reference/rust_dynamic/src/hash.rs:6`), and
`Val::ValueMap` is a `HashMap<Value, Value>`
(`reference/rust_dynamic/src/types.rs:79`) keyed by exactly that type — F30,
unobservable today only because F29 leaves no read path.

The three readers together — equality, ordering, hashing — are why identity
cannot simply be dropped, and why D1 defines laziness as minting on first
*need* rather than at construction.

### `q`, `attr`, `curr` and `tags` — four fields, four different stories

They are not interchangeable, and an earlier draft of this RFC treated them as
one group of constants. They are not.

**`tags` varies and is written on the hottest path.** `TS::push` calls
`value.set_tag("stack", &curr.stack_id())` on **every push**
(`reference/rust_multistack/src/ts_push.rs:25`, and `push_to_stack` at `:51`),
so a value on the main stack renders `tags: {"stack": "main"}`. `set_tag` is a
plain in-place insert (`reference/rust_dynamic/src/tags.rs:5`). It is a field.

**`attr` varies and one line of Bund drives it.** The `attribute` word calls
`attr_add` (`reference/rust_multistackvm/src/stdlib/values/value_tag.rs:13`,
which appends — `reference/rust_dynamic/src/attr.rs:18-22`). Confirmed against
the oracle: `1 2 attribute` renders
`attr: [Value { … data: I64(2) … }]`. It is a field. An earlier draft called
it a constant on the strength of the goldens, which only show `[]` because no
corpus program uses `attribute`.

**`curr` is the iteration cursor, not a constant.** `impl Iterator for Value`
reads and writes `self.curr` throughout
(`reference/rust_dynamic/src/iter.rs:7,11,19,22,24,27` for `LIST`/`RESULT`,
`:33,37,45,48,50,53` for `MATRIX`, `:59,63,75,78,83,86` for `METRICS`), and
`:92-97` iterates **any other value type** once, yielding itself. It resets to
`-1` on exhaustion (`:27`, `:53`, `:86`, `:96`).

So the goldens showing `curr: -1` everywhere are equally consistent with a
constant and with a cursor that resets, and the cursor reading is the true
one. An earlier draft read the evidence the wrong way and proposed deleting
the field, which would delete the state iteration needs.

What makes the disposition tractable is a separate fact: **`impl Iterator for
Value` has no caller.** Nothing in `rust_dynamic`, `rust_multistackvm`,
`rust_multistack` or the Bund runtime consumes a `Value` as an iterator, and
`.curr` is written nowhere outside `iter.rs`. That, not "iteration resets", is
why every rendering shows `-1`.

**`q` is the only one that is genuinely constant in reach.** `set` copies it
(`reference/rust_dynamic/src/set.rs:33`) and `push` copies it
(`reference/rust_dynamic/src/push.rs:164`); neither changes it, and no word
writes it. `Value::new` sets `q: 0.0` (`reference/rust_dynamic/src/value.rs:38`)
but nothing reaches it — the `nodata` word yields `dt: 97` with `q: 100.0`,
confirmed by probe. Q18 closes on this, grounded.

All four are **observable**, because `debug.display_stack` prints the Rust
`Debug` rendering and the goldens capture it: **29 of the 65 goldens** contain
raw `Value { ... }` text, 258 renderings in total. 255 show `q: 100.0`,
`attr: []` and `curr: -1`; the other 3 carry a populated `attr` and come from
this RFC's own probe. Before that probe existed every rendering agreed, which
is a fact about the corpus rather than about the fields — and is exactly how
an earlier draft mistook three of them for constants.

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
    Nodata,          // dt = NODATA (97)
    None,            // dt = NONE (0)
    Heap(Rc<HeapValue>),
}
```

**16 bytes, align 8**, measured — `cargo xtask layout`, candidate D. A scalar
never touches the heap: `layout` measures 0 allocations for constructing and
for cloning one.

`Nodata` and `None` are **two arms, not one**. `Val::Null` carries both
`NODATA` — via the `nodata` constructor — and `NONE`, which `Value::new` sets
(`reference/rust_dynamic/src/value.rs:37,39`). An earlier draft claimed the
scalar tags were one-to-one with their payloads and used that to justify
keeping the tag out of the value; the claim was false for exactly this pair,
and two arms is the repair. It costs nothing: the discriminant has room.

### The heap object

```rust
pub struct HeapValue {
    identity: Cell<u64>,        // 0 until observed, then minted
    stamp:    Cell<f64>,        // 0.0 until observed, then sampled
    dt:       u16,
    curr:     Cell<i32>,        // the iteration cursor, initialised to -1
    tags:     BTreeMap<String, String>,
    attr:     Vec<BundValue>,
    payload:  Rc<Payload>,      // shared separately from identity — see dup
}
```

`tags` and `attr` are fields because both vary (above). `curr` is a field
because it is the iteration cursor. `q` is not a field: it is rendered.

### Identity: lazy, but a nanoid-shaped string when observed

D1 resolved this as *lazy nanoid derived from counter plus VM seed, preserving
the format*. The slot is a `Cell<u64>` — zero meaning unminted — and the
**observable** id is that counter formatted into the reference's 21-character
nanoid alphabet. This matters because three things read the id as a string and
an earlier draft of this section addressed none of them:

- `.id` returns `Value::from_string(value.id)`
  (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:16`) — a string,
  not a number.
- The bincode wire format serialises `id: String`
  (`reference/rust_dynamic/src/value.rs:17`), and D20 requires it stay
  byte-identical. Serialisation is a materialisation point, which D20 already
  says, so the counter is formatted at that boundary.
- The golden normaliser matches on the `id: "` prefix, so the `Debug`
  rendering must print a quoted string of the same shape.

Minting happens on first *need*, and D1 enumerates the needs: `.id`, equality,
ordering, hashing, serialisation. All five materialise.

### `dup` is an identity reset, not a bare `Rc` bump

**This is the correction that matters most.** An earlier draft made `dup` an
`Rc` bump and called the result observably identical. It is not, and D1 says
so directly: *"Clone-equal versus dup-unequal is the contract to preserve."*

- `Clone` shares the id today, because `Clone` is derived and copies the
  `String`. So `A == A.clone()` is **true** for non-scalars, through
  `reference/rust_dynamic/src/eq.rs:53`.
- `dup` mints a fresh id (`reference/rust_dynamic/src/dup.rs:11`). So
  `A == A.dup()` is **false** for non-scalars, through the same line.

An `Rc` bump makes `dup` behave exactly like `Clone`, collapsing the two and
silently flipping `A == A.dup()` from false to true for every non-scalar.

Nor does the subsequent copy-on-write repair it. `dup_in_current_stack` pushes
the copy immediately (`reference/rust_multistack/src/ts_stack_op.rs:36`), and
`TS::push` writes the stack tag (`reference/rust_multistack/src/ts_push.rs:25`),
so the value is mutated the instant it is duplicated — but `Rc::make_mut`
**clones the existing contents**, id included. The split happens and the two
values still share an identity.

So `dup` allocates a fresh `HeapValue` with a **cleared** identity slot,
sharing the payload by `Rc`:

```rust
fn dup(&self) -> BundValue          // one small allocation, not a bincode round trip
```

That is why `payload` sits behind its own `Rc` inside `HeapValue`: identity
and payload must be shareable on different schedules. The win over the
reference is unchanged in kind — a serialise-deserialise round trip becomes
one header allocation and a refcount bump — and now it is correct.

### Value semantics

`Rc::make_mut` clone-on-write (D13), with one claim from the earlier draft
**withdrawn**. That draft said CoW split points "coincide exactly with the
points where the reference regenerates the id". They do not, and the
counterexample is on the hottest path: `set_tag` mutates in place and mints
nothing (`reference/rust_dynamic/src/tags.rs:5`), yet runs on every push.

The accurate statement is narrower and still sufficient: **every mutating
operation that the reference gives a fresh id to is a CoW split point**
(`set.rs:16,31,43,58,76,91`, `push.rs:165`, `attr.rs:7,13,19` — and `attr_add`
is explicit about it, calling `regen_id` at `attr.rs:19`). Mutations that mint
nothing, of which `set_tag` is the only one reachable, are split points too,
and `make_mut` copying the id is the correct behaviour for them, because the
reference's in-place write does not change the id either.

Cycles stay impossible by construction, as they do today.

### Integers

Full `i64` (D4). NaN-boxing would reach 8 bytes by folding the tag into unused
float bits, at the cost of capping integers at 51 bits. `Val::I64(i64)`
(`reference/rust_dynamic/src/types.rs:72`) and `cast_int` returning `i64`
(`reference/rust_dynamic/src/cast.rs:17`) make that a narrowing of an
observable range. Representation is private behind this type's API and can
change later; integer width cannot.

### Iteration

`impl Iterator for Value` has no caller in the reference, so `curr` never
advances in a running Bund. Bund2 carries `curr` as a field anyway, because
the field is observable in the `Debug` rendering and because RFC-0003 may
expose iteration. Whether Bund2's own iteration drives `curr` or uses an
external cursor is left to RFC-0003; carrying the field keeps both open.

### Rendering

`q` is rendered as the constant `100.0` rather than carried. Everything else
in the `Debug` output is real state. The rendering must reproduce the
reference's text exactly, because 29 goldens capture it — this is the one
place where preservation is of a text format rather than of a behaviour, and
it is stated here so it is not discovered by a golden failure.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| `dt` and payload as independent axes | **Preserved exactly.** `dt` carried verbatim as a `u16` over the same 42 constants. |
| `Val::Null` carrying both `NONE` and `NODATA` | **Preserved exactly**, as two scalar arms. |
| Fresh id per construction | **Preserved observably, changed mechanically.** Minted on first need. D1. |
| Fresh id per mutation | **Preserved exactly.** Every site the reference re-mints at is a CoW split. |
| **Clone-equal**: `A == A.clone()` true for non-scalars | **Preserved exactly.** `Clone` is an `Rc` bump, so the identity slot is shared. |
| **Dup-unequal**: `A == A.dup()` false for non-scalars | **Preserved exactly.** `dup` clears the identity slot on a fresh header. This is the contract an earlier draft broke. |
| `set_tag` mutating in place without minting | **Preserved exactly.** `make_mut` copies the id, which is what an in-place write does. |
| Equality: content for four payload kinds, identity for sixteen | **Preserved exactly**, including `Bool` comparing by identity. |
| Ordering falling back to `id.cmp` | **Preserved exactly**, including F12's inconsistency with `lt`. |
| Hash by identity | **Not preserved — deliberately open.** See F30 and Open questions. |
| `tags`, including the per-push stack tag | **Preserved exactly**, as a field. |
| `attr`, drivable by the `attribute` word | **Preserved exactly**, as a field. |
| `curr` as the iteration cursor | **Preserved as a field.** Nothing advances it today; RFC-0003 decides whether Bund2's iteration does. |
| `q` rendered as `100.0` | **Preserved as text.** Not carried. Grounded by Q18: no word writes it. |
| Binary wire format byte-identical | **Preserved exactly.** D20. The lazy identity materialises at this boundary. |
| `.id` returning a 21-character nanoid string | **Preserved exactly.** The counter formats into the reference's alphabet. |
| JSON round trip losing identity | **Preserved exactly**, including the loss. D20's scope correction. |
| `dup` as a bincode round trip | **Deliberately changed** to one header allocation plus a payload `Rc` bump. Observably identical: both yield an equal payload with a fresh identity. |
| `push_to_stack` capacity check | **Deliberately fixed** — F28. No golden exercises it. |
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
2. Constructing and cloning a scalar allocate **0** times for **candidate D**,
   measured by the counting allocator in `cargo xtask layout`. The table
   carried rows for candidates A and B only until this RFC's review; D's rows
   now exist, so this criterion is evaluable.
3. `dup` of a heap value allocates **1** time — one header — against the
   reference's serialise-and-deserialise. Same harness.
4. **`A == A.clone()` is true and `A == A.dup()` is false**, for a list, a
   map, a lambda and a bool. This is D1's contract, and it is listed as a
   criterion because an earlier draft of this RFC broke it.
5. `cargo xtask conform` does not fall below the mark in
   `tests/golden/CONFORMANCE.txt`, now **0/65**. RFC-0001 implements a value,
   not a word, so the number may rise; it may not fall. The grounding and
   review for this RFC moved the denominator from 63 by adding two probes —
   `tests/probes/valuemap-hash-eq.bund` pinning F29, and
   `tests/probes/value-fields.bund` pinning which fields are real state. Both
   moves are recorded in RFC-0000's provenance table.
6. The `Debug` rendering reproduces the reference's text for the **258**
   renderings across 29 goldens, including `q: 100.0` as a constant and
   `attr`, `curr`, `tags` as real state. **255 of the 258 show `attr: []` and
   3 do not** — the three come from this RFC's own probe, and they are the
   reason `attr` is a field rather than a constant.
7. **The bincode wire format is byte-identical to the reference** for one
   value of each of the 20 payload kinds, compared against bytes captured from
   the oracle. D20 asserts this and no earlier criterion checked it.
8. **`.id` returns a 21-character string** over the reference's nanoid
   alphabet, and two values minted in one VM never collide.
9. `cargo tree -p bund2-value` lists no `bund2-interp`. Vacuous until
   `bund2-value` has dependencies, exactly as RFC-0000's D-2 records; this is
   the RFC that makes it real.
10. `cargo xtask cite` reports zero defects.

## Open questions

- **F29 and F30 together block `valuemap`, and this is the one thing that
  blocks acceptance.** `get` has no `VALUEMAP` arm and returns the whole map
  through its catch-all (`reference/rust_dynamic/src/get.rs:18-19`); `?key`
  always answers false (`reference/rust_dynamic/src/has_key.rs:19-20`). Adding
  the arm does not fix it, because `Hash` keys on identity while `PartialEq`
  compares content for the four scalar kinds, so an equal-but-freshly-built
  string key cannot be found. **A decision is required**: either `BundValue`
  hashes by content, which changes `ValueMap` behaviour no program can
  currently observe, or `ValueMap` keys by identity and equal-looking keys
  stay distinct. This RFC takes neither.
- **D14** is OPEN and governs which words exist; it does not affect this RFC's
  representation.
- Whether Bund2's own iteration drives `curr` or uses an external cursor is
  RFC-0003's call. This RFC carries the field so both stay open.

## Review history

- **2026-08-26** — `docs/rfc/reviews/RFC-0001-review-2026-08-26.md`. Verdict:
  do not accept. Every finding held on re-verification against source. Three
  citations pointed at lines that existed but did not say what the prose said
  (`set.rs:41` for `set`'s dup, which is `:15`; `set.rs:61` for the `q` write,
  which is `:33`; `value_tag.rs:71-72` for the `attr`/`tags` writes, which are
  `:13` and `:49`) — `cargo xtask cite` passed all three, because it checks
  that a line exists and that a token is near it, not that the line means what
  the prose claims. Three counts were wrong (42 `dt` constants not 40, 132
  corpus programs not 138, 250 renderings across 28 goldens not 244 across 27, since risen to 258 across 29
  — this RFC's own probe moved the last one and the criterion was not
  updated).

  Three findings changed the design rather than the prose: `dup` as an `Rc`
  bump destroyed D1's clone-equal/dup-unequal contract; "split points coincide
  exactly" had a counterexample in `set_tag` on the hottest path; and `curr`
  is the `Iterator` cursor, not a constant, so deleting it would have deleted
  the state iteration needs. The equality section was wrong in the same way F30
  was — the id fallback is a catch-all covering sixteen of twenty payload
  kinds, not a mismatched-types case — and ordering, a third id reader named
  by D1, was missing entirely.
