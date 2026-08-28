# RFC-0001: `BundValue` — representation, identity, and value semantics

- Status: **Proposed** (2026-08-27), after seven reviews. Nothing gates it:
  D30 and its amendment settle F29, F30 and F33; Q18 is closed on a probe and
  Q19 divided between this RFC and RFC-0002. The four binding criteria pass;
  two are vacuous and labelled; **eight are deferred because `bund2-value` is
  three lines**, and that is why the status is Proposed rather than Accepted.

  The seventh review found the design sound and the document stale — repairs
  landing in the body and not in the sections quoting it, counts incremented
  rather than re-derived. That is a document-maintenance failure, and it is
  cheaper to fix against a compiler than against a reader: the deferred
  criteria become checkable, and a claim that contradicts itself stops
  compiling. **Implementation is the next step, not another revision.**
- Depends on: RFC-0000
- Decisions consumed: D1, D2, D4, D13, D20, D30
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
and decodes it again** — **55 uses of `dup` itself** across 38 of the 132
corpus programs. `dup_one` and `dup_many` have zero direct uses; every one of
the 55 reaches `dup_one` through the alias. An earlier draft called this a
"dup-family" count, which reads as though the siblings contributed. `Value::set` does the same on every map write
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
| `Val::Map` | `CLASS`, `CONDITIONAL`, `CONFIG`, `CURRY`, `INFO`, `MAP`, `MESSAGE`, `OBJECT` |
| `Val::String` | `CALL`, `CONTEXT`, `JSON_WRAPPED`, `PTR`, `STRING`, `TEXTBUFFER` |
| `Val::List` | `CFLOAT`, `CINTEGER`, `LIST`, `PAIR`, `RESULT` |
| `Val::Queue` | `FIFO`, `QUEUE` |
| `Val::Binary` | `BIN`, `ENVELOPE` |
| `Val::Null` | `NODATA`, `NONE` |

`MESSAGE` and `PAIR` are written by assigning `dt` **after** construction —
`from_pair` builds a list then sets `dt = PAIR`
(`reference/rust_dynamic/src/create.rs:164-167`), and the message constructor
builds a dict then sets `dt = MESSAGE` (`:186`). Two earlier drafts missed
both, because a scan for the `dt:` field-initialiser pattern cannot see them.

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

**The other sixteen arms reach the catch-all at `:45`**, which tests `dt` for
`CINTEGER` and `CFLOAT` — both `Val::List` payloads, compared as complex
numbers — and otherwise returns `self.id == other.id` (`:53`). So of the
twenty arms: four compare by content, one (`Val::List`) compares by content
when tagged `CINTEGER` or `CFLOAT` and by identity otherwise, and the
remaining fifteen compare by identity always. Two earlier drafts quoted
"sixteen" and "twelve" for the same partition; the precise statement is the
one above, and it is used consistently below.

For every identity-compared arm **`==` *is* identity comparison**: two
structurally identical lists are unequal unless they share an id.

This is what D1 already recorded, and an earlier draft of this RFC got it
wrong — it said the id fallback fires "only when the types differ". It does
not. D1 and D13 both rest on `eq.rs:53`, and the design below depends on it.

`Ord::cmp` has the same shape: content for `I64`, `Time` and `String`, and
`self.id.cmp(&other.id)` otherwise
(`reference/rust_dynamic/src/ord.rs:175,183,191,199`). So **`cmp` reads the
id**, which D1 names as one of three internal readers.

**But `cmp` is the path a program does not reach, and two earlier drafts
analysed only it.** That is a negative claim of the class this RFC has been
wrong about three times, so its basis is stated rather than assumed: it rests
on reading `PartialOrd`, which overrides all four comparisons, and not on a
probe. No probe here discriminates `cmp` from the overrides, because the two
agree on every operand pair a program can currently build. Recorded as a claim
about the source, not a measured fact.
`PartialOrd` overrides `lt`, `le`, `gt` and `ge` individually
(`reference/rust_dynamic/src/ord.rs:9,48,87,126`), and **none of the four
reads an id** — they compare content and fall back to `true` or `false` on a
type mismatch. Those four are what a comparison in Bund reaches, and what
`tests/golden/examples/code_snippets/sorting_numbers_in_list.golden` covers. `partial_cmp` delegates to
`cmp` (`:6-8`), so it cannot disagree with it; the disagreement F12 means is
between `cmp` and the four overrides, and F12 mis-cites itself on this —
it attributes `ord.rs:19-21` to `partial_cmp` when those lines are inside
`lt`.

For this RFC the consequence is narrow and worth stating: the id is read by
equality, by hashing, and by `cmp` — but the ordering a program can actually
observe does not read it, so lazy identity is not forced to materialise by a
sort.

`Hash` hashes **only the id** (`reference/rust_dynamic/src/hash.rs:6`), and
`Val::ValueMap` is a `HashMap<Value, Value>`
(`reference/rust_dynamic/src/types.rs:79`) keyed by exactly that type — F30,
unobservable today only because F29 leaves no read path. **D30 resolves both**;
see Design.

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

**It has seven callers**, and two earlier drafts said it had none while a
third said six. They sit in `reference/Bund/src/cmd/` —
`bund_cluster.rs:35,40,305,310,437` and `bund_bbus.rs:97,172`, both modules
declared at
`reference/Bund/src/cmd/mod.rs:20,21` — and each is a `for n in value` loop
over a `Value` returned by the zenoh accessors
(`reference/Bund/src/stdlib/helpers/zenoh/putget.rs:80,119`). The earlier
searches looked for `.next()` and `into_iter()` and never for the `for … in`
sugar, which is how an `Iterator` is actually consumed.

So `curr` **is** advanced, and `next` writes `self.curr` in place — a
minting-free mutation. Under this RFC's policy that makes iterating a *shared*
value a CoW split, with two consequences worth stating rather than
discovering:

- the original's cursor stops advancing when the copy takes over, where the
  reference advances the receiver's own field;
- and the split materialises an identity per step, by the rule below.

Bund2 therefore takes the cursor **out of the shared header for iteration**:
`next` operates on a locally owned cursor rather than mutating through the
value, so iterating neither splits nor mints. That is a deviation from a
`&mut self` cursor and it is invisible to any program, because every rendering
shows `-1` — iteration resets on exhaustion
(`reference/rust_dynamic/src/iter.rs:27,53,86,96`) and the callers above are
all in `bus`, which D28 defers.

**`q` is a field too, and two earlier drafts had it as a rendered constant.**
It is *averaged and propagated*, not copied: `calc_q` sets
`self.q = (self.get_q() + other.get_q())/2.0`
(`reference/rust_dynamic/src/q.rs:5`) and `set_q` writes it directly (`:9`),
and both are reached from the arithmetic operators — `impl Add` computes
`(self.q + other.q)/2.0` and calls `set_q` on the result
(`reference/rust_dynamic/src/math.rs:416-420`), with `Sub`, `Mul` and `Div`
the same. So every arithmetic operation writes `q`.

Every golden shows `100.0` because that is a **fixpoint**, not a constant: all
constructors start at 100.0 and the average of 100.0 and 100.0 is 100.0.

**And the averaging is the point, not an accident** — D32: `q` is the
mechanism for a future fuzzy-math feature, stated by the repository owner.
That rules out a reading the evidence alone left open, since a varying field
with no reader is defensibly inert state to carry along. It is not: Bund2
preserves the **propagation**, so an arithmetic result carries the mean of its
operands' `q`. The distinction matters downstream — a field that merely rides
along can be dropped from a JIT fast path and a propagating one cannot.

The fixpoint is escapable, and one in-scope word escapes it. `Value::none` is
`Value::new` (`reference/rust_dynamic/src/create_special.rs:19-21`), which
sets `q: 0.0` (`reference/rust_dynamic/src/value.rs:38`), and the JSON
converter returns `Value::none()` for a JSON null
(`reference/rust_dynamic/src/cast_json_to_value.rs:39`). A program that
converts a JSON null therefore holds a value with `q: 0.0`, and rendering
`q` as `100.0` would diverge on it.

**Q18 was reopened on this, and is now closed** — on a probe rather than a
scan: `"null" json json.to_value` yields `q: 0.0` on the oracle
(`tests/probes/q-observable.bund`). It had first been closed as "grounded" by
a scan that never opened `q.rs`.

All four are **observable**, because `debug.display_stack` prints the Rust
`Debug` rendering and the goldens capture it: **32 of the 69 goldens** contain
raw `Value { ... }` text, **303** renderings in total. 300 show `attr: []`;
the other 3 carry a populated `attr` and come from this RFC's own probe. Before that probe existed every rendering agreed, which
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
    q:        f64,              // averaged by arithmetic — see below
    curr:     i32,              // the iteration cursor, initialised to -1
    tags:     BTreeMap<String, String>,
    attr:     Vec<BundValue>,
    payload:  Rc<Payload>,      // shared separately from identity — see dup
}

pub enum Payload {
    Str(String),
    Bin(Vec<u8>),
    List(Vec<BundValue>),
    Matrix(Vec<Vec<BundValue>>),
    Map(BTreeMap<String, BundValue>),
    ValueMap(HashMap<BundValue, BundValue>),
    Json(serde_json::Value),
    Metrics(Vec<Metric>),
    Operator(Operator),
    Embedding(Vec<f32>),
    Time(u128),
    Exit,                       // the end-of-input marker
    Scalar(BundValue),          // a boxed scalar — see the scalar section
}
```

**`Payload` is defined here, and an earlier pass claimed it was without the
definition landing** — a scripted edit whose target text did not match, which
silently did nothing while the preservation rows and the review history both
asserted it was done. Recorded as F41.

`identity` and `stamp` are `Cell`s because equality, hashing and rendering
mint through `&self`. **`curr` is not**: a `Cell` inside a shared `Rc` would
give the cursor *reference* semantics across clones, where the reference
deep-copies and gives each its own. `Iterator::next` takes `&mut self`
(`reference/rust_dynamic/src/iter.rs:6`), so `curr` is reached through
`make_mut` and splits with it.

**`Map` is a `BTreeMap` and `ValueMap` is a `HashMap`, and the difference is
not an oversight.** An earlier draft made both `BTreeMap` and thereby deleted
D30's mechanism.

`Map` is keyed by `String`, whose `Ord` is total and cheap, so ordering it
satisfies F15 by construction: the reference renders a dict in `HashMap`
order, which differs between runs and cost 15 of the 18 unreproducible
programs. F15's disposition asks RFC-0001 for exactly this, and it asks it of
**dict members and tags** — not of `ValueMap`.

`ValueMap` **must hash**, because that is what D30 decided: hash by content,
mirroring equality, so an equal-but-freshly-built key finds its entry. A
`BTreeMap` does not hash — it needs a total `Ord` over `BundValue`, and this
RFC makes that impossible three ways over. F12's disposition is FIX, which
deletes `cmp`'s id fallback and leaves non-scalars unordered. `NaN` is equal
to nothing, so floats admit no total order. And the four overridden
comparisons return `true` or `false` on a type mismatch rather than an
`Ordering` (`reference/rust_dynamic/src/ord.rs:9,48,87,126`). D30's own
wording — keys "hash into the same bucket" — names the mechanism.

Determinism for `ValueMap` rendering therefore comes from the **renderer**,
not the container: entries are emitted in sorted order of their rendered form.
Two different requirements — a container that hashes, and output that is
ordered — which an earlier draft conflated by reaching for one type.

`Payload` omits `Val::Token` and the four `dt` constants with no writer —
`LITERAL`, `LARGE_FLOAT`, `ASSOCIATION`, `TOKEN` — which F36 and F38 ask this
RFC to record. **38 `dt` constants are live of the 42 declared.**

**What that claim rests on, said precisely.** It is a grep: no assignment to
`dt` names those four in any of the six crates. That method has produced a
false negative in three consecutive reviews — `Value::exit`, `impl Iterator`,
`q` — so this RFC does not leave it as the only evidence.

`tests/probes/dt-reachable.bund` enumerates in the **positive** direction:
it constructs a value of every `dt` a Bund program can be made to produce, and
the golden pins the result. Across the whole suite, **20 of the 42 constants
are reached**: `NONE`, `BOOL`, `INTEGER`, `FLOAT`, `STRING`, `CALL`, `PTR`,
`LIST`, `PAIR`, `MAP`, `CFLOAT`, `METRICS`, `LAMBDA`, `TEXTBUFFER`, `JSON`,
`CONDITIONAL`, `VALUEMAP`, `CLASS`, `OBJECT`, `NODATA`.

**The enumeration does not prove the four are dead, and saying so is the
point.** Twenty-two constants go unreached, and several of them —
`MATRIX`, `CURRY`, `MESSAGE`, `ERROR`, `ENVELOPE` — almost certainly have a
constructor this enumeration simply did not find. The four F36 names sit in
that group, not in a group of their own. So the negative claim remains
**grep-established and enumeration-corroborated**, which is weaker than proven
and stronger than an unchecked assertion, and it is the honest description of
what this RFC knows.

`TIME` is a third case worth its own line: it is reachable, through
`time.now`, and **not capturable** — a clock reading differs between runs, so
the golden refuses it. Reachable-and-unpinnable is an answer the enumeration
records rather than loses.

`tags`, `attr`, `curr` and `q` are all fields, and all four vary.

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
and payload must be shareable on different schedules.

**What `dup` does with the other header fields**, which an earlier draft left
unsaid. The reference's `dup` is a bincode round trip that then overwrites
**only** `id` (`reference/rust_dynamic/src/dup.rs:11`), so the copy keeps the
original's `stamp`, `attr`, `curr`, `tags` and `q` exactly. Bund2's `dup`
copies all five and clears only the identity slot.

`stamp` needs care: the reference guarantees a value and its `dup` share one,
and under lazy sampling two *unobserved* values would sample independently and
differ. So **`dup` materialises the stamp before copying**, by the same rule
as the identity at a CoW split and for the same reason — it is the moment two
copies could otherwise diverge. The win over the
reference is unchanged in kind — a serialise-deserialise round trip becomes
one header allocation and a refcount bump — and now it is correct.

### One policy for the `Rc` and the identity slot (D13)

D13 does not merely permit `Rc::make_mut`; it sets a condition on it, in
terms: *"the lazy identity slot must be shared across clones and must split at
the same points the `Rc` does. If the two split policies disagree, an
unobserved value and its clone materialise different ids and `A == A.clone()`
silently becomes false. RFC-0001 must show one policy governing both."*

An earlier draft showed half of it. `Clone` is an `Rc` bump, so the slot is
shared and clone-equal holds — that is D1's hazard answered. **`make_mut` was
left unanswered**, and it breaks the same property: `make_mut` clones the
`HeapValue`, an unminted slot is copied as *unminted*, and the two halves then
mint independently. Two different ids, and `A == A.clone()` flips from true to
false.

This is not a corner. `TS::push` calls `set_tag` unconditionally
(`reference/rust_multistack/src/ts_push.rs:25`), and this RFC classifies
`set_tag` as a minting-free mutation and therefore a split point. So it fires
on **every push of a cloned value**.

**The policy, one rule covering both: a CoW split materialises the identity
before it copies.** Minting is on first *need*, and a split is a need —
precisely because it is the moment at which two copies could otherwise
diverge. Both halves then carry the same concrete id, which is what the
reference produces: its `Clone` deep-copies the `id` `String`, and a
minting-free mutation like `set_tag` leaves both copies holding it.

`stamp` follows the same rule, for the same reason.

The cost is a mint per split of a shared value, which is the price of D1's
laziness being observable through equality. It is bounded by how often a
*cloned* value is mutated — an unshared value's `make_mut` does not copy and
so does not split.

**Mutations fall into three classes, not two.** An earlier draft had two and
put `attr_add` in the wrong one; the class it belongs to has no analogue in
either.

**1. Rebuild** — `set` on a map or list, `push`. These go through a
constructor, so the result carries a fresh identity, a fresh stamp, an empty
`attr`, `curr` at `-1`, empty `tags`, and **`q` reset to 100.0** —
`from_dict` writes all six (`reference/rust_dynamic/src/create_map.rs:33-40`).
The receiver is untouched. This is F34. Note the fallback arms of `set` and
`push` do *not* rebuild: they copy `q` from the receiver
(`reference/rust_dynamic/src/set.rs:33`, `push.rs:164`), so `q`'s treatment
differs by arm within one function.

**2. Regenerate in place** — `attr_add`, and it is its own class.
`attr_add` is `self.dup().regen_id()` then a push onto the result's `attr`
(`reference/rust_dynamic/src/attr.rs:19-20`), and `regen_id` writes a fresh id
**and a fresh stamp** (`reference/rust_dynamic/src/id.rs:6-7`). So it mints,
like a rebuild — but it **preserves** `attr`, `curr` and `tags`, because the
`dup` carries them. Confirmed against the oracle: `1 2 attribute 3 attribute`
renders **two** entries in `attr`, each with its `tags` intact. A two-bucket
partition has nowhere to put that, and an earlier draft filed it under
rebuild, which would have emptied the `attr` the word exists to populate.

**3. Minting-free** — mutate in place, leaving id and stamp alone. These are
the CoW splits, and *these* materialise the identity before copying so both
halves keep the same one. An earlier draft said `set_tag` was the only
reachable one. It is not, and this RFC establishes the others itself several
pages earlier: `Iterator::next` writes `curr`
(`reference/rust_dynamic/src/iter.rs:11,24`), and `calc_q`/`set_q` write `q`
(`reference/rust_dynamic/src/q.rs:5,9`) from every arithmetic operator
(`reference/rust_dynamic/src/math.rs:416-420`). Direct field writes outside
`rust_dynamic` count too —
`reference/Bund/src/stdlib/functions/values/merge.rs:83` sets `dt` in place.

Classifying `set` as both a split and a rebuild, as an earlier draft did, was
a contradiction: one rule says the halves share an id, the other says the
result gets a fresh one. Only the rebuild is the reference's behaviour.

### Value semantics

`Rc::make_mut` clone-on-write (D13). Which mutations split, and what each does
to the header, is **the three-class partition above** — rebuild,
regenerate-in-place, minting-free — and this section deliberately does not
restate it. Two earlier drafts stated a two-class version here and left it
standing after the body replaced it, twenty lines apart. The repair that
matters is having one statement, not a better second one.

Cycles stay impossible by construction, as they do today.

### Scalars have no header at all, so they must be boxable

An earlier draft repaired one field and missed five. `Bool`, `Nodata` and
`None` were given content-equality because they have no identity slot — but
they have no `tags`, `attr`, `curr` or `stamp` slot either, and the reference
writes all of those on scalars.

`TS::push` calls `value.set_tag("stack", …)` on **every** push with **no type
test** (`reference/rust_multistack/src/ts_push.rs:25`) — which is the very
fact criterion **B2** uses to explain why a heap `dup` costs four
allocations. So
every scalar that reaches a stack carries a non-empty `tags`. And `attr` is
drivable on a scalar too: `1 2 attribute` renders `dt: 2` — an integer — with
a populated `attr`, which is this RFC's own probe and the evidence cited for
`attr` being a field at all.

The goldens say how often this matters. Of the **303** renderings, **54 have
a scalar payload and 47 of those carry a non-empty `tags`** — `I64` 25,
`Bool` 14, `Null` 8, `F64` 7 — and 3 carry a populated `attr`. These figures
move whenever a probe is added and have moved three times; they are quoted
because criterion D3 rests on them, and D3 names the golden *set* rather than
a number for that reason. An earlier draft said 27 and 24, from a scan that required
a parenthesised payload and so missed `Null` entirely. Criterion 6 asks the `Debug` rendering to reproduce the reference's
text, and an inline scalar arm cannot reproduce any of those 47.

So the scalar arms are the **unadorned** form, not the only form:

```rust
pub enum BundValue {
    Int(i64), Float(f64), Bool(bool), Nodata, None,   // no header, 0 allocations
    Heap(Rc<HeapValue>),                              // header, when one is needed
}
```

A scalar is `Int(7)` until it acquires anything a header holds — an observed
identity, a stamp, a tag, an attribute, a moved cursor — at which point it is
represented as `Heap` with a scalar payload. The `dt` distinction is preserved
either way, and equality, ordering and hashing read through both forms
identically, so boxing is invisible **to equality, ordering and hashing**.

That is the limit of the claim, and an earlier draft wrote it without one.
`.id` and `.timestamp` mint on observation, and an unadorned scalar has
nowhere to keep what it minted — observing one twice would yield two different
ids where the reference yields one. The state is reachable: `"[1,2]" json
json.to_value` leaves inner scalars with `tags: {}`, so a program can hold an
unadorned scalar and then interrogate it. **So `.id` and `.timestamp` box**:
observing either promotes the value, and the promoted form is what the stack
holds.

**Promotion has to write back, and "it boxes" does not achieve that.**
`Value::get` returns a **clone** (`reference/rust_dynamic/src/get.rs:11`), so
promoting the clone leaves the container holding the original, and
`l 0 get .id` twice still mints two ids. This RFC's own `json.to_value`
example is exactly that case — the inner scalars live inside a list, and
reaching one goes through `get`.

The rule is therefore stated at the container rather than the accessor: **an
element promoted by observation is written back**, which under clone-on-write
splits the container if it is shared. That is a deviation — the reference's
`get` has no write-back — and it is what lazy identity costs when the accessor
returns a copy. Criterion D5 carries it.

**What this costs, stated plainly.** Because `push` tags unconditionally, a
scalar pushed to a stack is boxed, so "a scalar never touches the heap" holds
only *before* a push. Criteria 2 and 3 are restated against that.

There is a way to keep the common case unboxed, and it belongs to RFC-0003
rather than here: if a stack slot carries `(value, stack_tag)` rather than a
bare value, the tag that `push` writes lives in the slot and a scalar sitting
on a stack needs no header. The fossil case still needs one — a value
collected into a list keeps the tag of the stack it *was* on, which is why the
inner values of this RFC's `valuemap` probe render `tags: {"stack": "main"}`
while sitting inside a map. RFC-0001 specifies the representation that is
correct unconditionally; RFC-0003 may make the common case cheaper once it
decides how stacks are represented.

### Equality for scalars is content, and the reachability argument is narrower
than an earlier draft claimed

`Val::Bool` is not one of the four content-compared arms, so two `true`s reach
the catch-all at `reference/rust_dynamic/src/eq.rs:45` and are equal only if
they share an id (`:53`). Bund2 compares them by content instead.

**The reason two earlier drafts gave for this no longer holds.** They argued
that scalars *have no identity to compare*. Since review 3 made scalars
boxable, a boxed scalar has a header and therefore an identity slot, so the
premise is false — what survives is that an **unadorned** scalar has none, and
equality must not depend on whether either operand happens to be boxed.
Content comparison is the only rule stable under boxing, which is a better
reason than the one it replaces.

The deviation itself is carried by reachability — but an
earlier draft overstated the argument. It said the `==` word "accepts only
`INTEGER | FLOAT | CINTEGER | CFLOAT | TIME`". **It also accepts `STRING`**
(`reference/rust_multistackvm/src/stdlib/logic/logic_compare_fun.rs:47`). The
claim that survives is the one that matters: `==` has **no `BOOL` arm**, so it
bails on a bool (`:67-69`), and scalar identity-equality is unreachable
through it. The paths that do reach it are container membership and `ValueMap`
keys, and under D30 a bool key hashes by content anyway.

### Mutation resets the header, it does not copy it (F34)

`Rc::make_mut` copies the header. The reference **discards** it. `set` on a
map rebuilds through `Value::from_dict` (`reference/rust_dynamic/src/set.rs:21`)
and restores only the `dt` (`:22`); `from_dict` is a constructor and writes
`attr: Vec::new()`, `curr: -1`, `tags: HashMap::new()`
(`reference/rust_dynamic/src/create_map.rs:38-40`). `from_list` and
`from_valuemap` do the same.

Confirmed against the oracle: `dict 99 attribute "b" 2 set` renders
`attr: []`, while the control `1 2 attribute` renders it populated.

So a container mutation in Bund2 clears `attr`, `curr` and `tags` and lets the
next push re-apply the stack tag — matching the constructors rather than
matching `make_mut`. Copy-on-write is still how the payload is shared; the
header is rebuilt, not cloned. An earlier draft's preservation table did not
mention this at all.

### The `stamp` (D2)

`stamp` is sampled on first observation, exactly as identity is minted on
first need, and for the same reason: `timestamp_ms` is a `SystemTime::now()`
syscall on every construction
(`reference/rust_dynamic/src/value.rs:7-9,36`).

Its observable readers are `.timestamp`, which returns
`Value::from_float(value.stamp)`
(`reference/Bund/src/stdlib/functions/oop/base_classes.rs:25`), the `Debug`
rendering the goldens capture, and serialisation. Nothing compares, orders or
hashes on it, so unlike identity it has no internal readers — which makes it
strictly the easier of the two.

**The deviation D2 requires stating:** two values constructed at different
moments but never observed will, on first observation, receive the *same*
stamp rather than different ones, because neither sampled at construction.
Nothing in the corpus can distinguish them — `.timestamp` has zero corpus uses
— and the goldens normalise stamps away as F14. An earlier draft consumed D2
and gave `stamp` no section, no preservation row and no criterion.

### Equality across int and float is exact (D30's amendment, F33)

The reference's equality is **asymmetric**: `Val::I64` against `Val::F64`
truncates the float (`reference/rust_dynamic/src/eq.rs:13`) while `Val::F64`
against `Val::I64` widens the int (`:24`). `impl Eq` asserts the symmetry it
does not have (`:59-62`). A content hash cannot be built on that, because no
bucket assignment is consistent with an asymmetric equality.

D30's amendment makes the contract bidirectional, and that has one consistent
implementation. Both obvious symmetrisations fail, and both failures are
pinned in `tests/golden/probes/eq-asymmetry.golden`:

- **Truncate both ways** is not transitive: on the oracle `42 == 42.5` is
  true and `42 == 42.9` is true, while `42.5 == 42.9` is false.
- **Widen both ways** is not transitive above 2^53. The orientation has to be
  named, because the two disagree: with the **float on top** the receiver is
  the `F64` and it widens, and the oracle answers **true** — widening `2^53+1`
  to `f64` loses the low bit, so two distinct integers are equal to one float
  and therefore to each other. With the **int on top** the receiver truncates
  and the oracle answers **false**. An earlier draft claimed "both failures
  are pinned" while the probe wrote only the truncating orientation and
  labelled it as the widening one; the probe now writes both.

So an integer and a float are equal **when they denote the same mathematical
value**:

    i == f  ⟺  f is finite and integral, within i64 range, and f as i64 == i

Symmetric by construction, transitive, and hashable — which is what D30 needs.
`42 == 42.0` holds; `42 == 42.5` does not; `9007199254740993` does not equal
`9007199254740992.0`.

**A deviation in both directions**, and the reference's behaviour is captured,
so the golden will disagree and its disposition is F33.

### Hashing mirrors equality (D30)

The reference hashes the id alone, while equality compares content for four
payload arms and identity for the rest. The two disagree, and `Val::ValueMap`
is keyed by the type whose contract is broken — F30.

D30 settles it: **`hash` mirrors `eq`, kind by kind.** In Bund2 that is
**six** content-hashed kinds, not the reference's four: `Int`, `Float`,
`String` and `Time` as in the reference, plus `Bool` and the two nullary
scalars, which this RFC moved to content comparison for stability under
boxing. An earlier draft enumerated four here and six in the scalar section —
and a `Bool` that compares by content while hashing by identity breaks
`Hash`/`Eq` in the one structure that consumes both. Identity-compared kinds
hash their identity.

**The key equality is not the `==` word's equality**, and an earlier draft did
not separate them. `HashMap<BundValue, _>` requires `Eq`, `Eq` requires
reflexivity, and "`NaN` is equal to nothing" denies it — the same fault this
RFC convicts the reference of forty lines earlier, and now *reachable*,
because D30 creates the read path that makes `ValueMap` observable.

So there are two, named separately:

- the **word** `==` keeps IEEE semantics, `NaN != NaN`, because that is what a
  Bund program sees and `logic_compare_fun.rs` is already its own dispatch;
- the **key** equality behind `ValueMap` and `Hash` is *total*: all `NaN`s are
  one value, `-0.0` and `0.0` are one value. That makes `Eq` sound and `Hash`
  consistent with it.

The reference cannot draw this distinction, having one `PartialEq` and an
`impl Eq` that lies about it. Bund2 has two and says which is which.

Hashing *everything* by content would also satisfy the contract, and is
rejected: it computes an O(size) hash for a list whose equality is then
decided by identity — cost with no lookup to show for it, since the entry
still would not be found.

**The float-bearing arms follow from exactness**, which settles what review 2
flagged as unspecified. A float that is finite, integral and in `i64` range
hashes as the `i64` it denotes, so `42` and `42.0` share a bucket. `-0.0` is
normalised to `0.0` before hashing, since the two are equal and equal values
must hash alike. `NaN` is never equal to anything, including itself, so its
hash is unconstrained — it takes a fixed value and never matches.

Two consequences worth stating:

- **Composite keys stay identity-keyed.** A freshly built list equal to a
  stored key will not find it, because `eq` for a list is identity
  (`reference/rust_dynamic/src/eq.rs:53`). Changing that means changing
  equality across the language, which D30 deliberately does not do.
- **Laziness gets cheaper.** D1 lists hashing among the needs that force a
  lazy identity to materialise. Under this design a scalar never has an
  identity to materialise, so a scalar-keyed lookup mints nothing. Composite
  keys still do.

`valuemap` also becomes readable, which is the other half of D30: Bund2's
`get` word pulls both operands and branches on the container's type before
casting the key, mirroring `set`
(`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:16-19`). That is
a word-level change and belongs to RFC-0002's word set, but it is recorded
here because without D30's hashing it would not work.

### `BundValue` is neither `Send` nor `Sync`

Stated here because RFC-0002 cites this RFC for it and, until now, this RFC
did not say it. The non-scalar arm is `Rc<HeapValue>` and `HeapValue` carries
`Cell` fields, so neither trait is derivable: `Rc` is neither `Send` nor
`Sync`, and `Cell` is not `Sync`. The choice is deliberate — an `Arc` and an
atomic identity would cost every clone a contended refcount to buy a property
no single-threaded interpreter needs — but it propagates, and RFC-0002's
registry inherits it: a `Slot` holding a `BundValue` cannot cross a thread
whatever pointer wraps it.

RFC-0007 inherits the question rather than an answer. Making `BundValue`
shareable is not a change to the registry; it starts here.

### Integers

Full `i64` (D4). NaN-boxing would reach 8 bytes by folding the tag into unused
float bits, at the cost of capping integers at 51 bits. `Val::I64(i64)`
(`reference/rust_dynamic/src/types.rs:72`) and `cast_int` returning `i64`
(`reference/rust_dynamic/src/cast.rs:17`) make that a narrowing of an
observable range. Representation is private behind this type's API and can
change later; integer width cannot.

### Iteration

`impl Iterator for Value` has **seven callers**, all `for … in` loops in
`reference/Bund/src/cmd/` — see the `curr` discussion above. One of them,
`bund_cluster.rs:437`, pushes each element onto a VM stack, so an iteration
result reaches the language. `curr` is a field and Bund2's iteration operates on a
locally owned cursor, so iterating neither splits a shared value nor mints an
identity per step. Whether RFC-0003's own iteration re-exposes a stored cursor
is its call; carrying the field keeps that open.

### Rendering

Every field in the `Debug` output is real state, `q` included — it is a
fixpoint at 100.0, not a constant, and the JSON-null path reaches 0.0. The
rendering must reproduce **the normalised text the goldens hold**, across 31
of them, which is not the same as the reference's raw output: the goldens
normalise `id`, `stamp` and map order, and this RFC orders the dict rendering
deliberately. This is the one place where preservation is of a text format
rather than of a behaviour, and it is stated here so it is not discovered by a
golden failure.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| `dt` and payload as independent axes | **Preserved exactly.** `dt` carried verbatim as a `u16` over the **38 live constants of the 42 declared** — F36's four writerless ones are omitted, which rows 12 and 18 already say and which an earlier version of this row contradicted by saying 42. |
| `Val::Null` carrying both `NONE` and `NODATA` | **Preserved exactly**, as two scalar arms. |
| Fresh id per construction | **Preserved observably, changed mechanically.** Minted on first need. D1. |
| Fresh id per mutation | **Preserved exactly.** Every site the reference re-mints at is a CoW split. |
| **Clone-equal**: `A == A.clone()` true for non-scalars | **Preserved exactly.** `Clone` is an `Rc` bump, so the identity slot is shared. |
| **Dup-unequal**: `A == A.dup()` false for non-scalars | **Preserved exactly.** `dup` clears the identity slot on a fresh header. This is the contract an earlier draft broke. |
| `set_tag` mutating in place without minting | **Preserved exactly.** `make_mut` copies the id, which is what an in-place write does. |
| Equality: content for four arms, identity for fifteen, mixed for `Val::List` | **Preserved exactly** for every arm with a heap header. |
| Equality across `Int` and `Float` | **Deliberately changed, and this is the largest deviation in the RFC.** `42 == 42.5` is true in the reference's truncating orientation and false in Bund2, by D30's amendment. It disagrees with a captured golden, `eq-asymmetry.golden`, which is regenerated with `--reason F33`. Two earlier drafts had rows for the heap arms and for `Bool`/`Nodata`/`None`, and `Int` and `Float` fell between them. |
| `HashMap` iteration order in the `Debug` rendering (F15) | **Deliberately fixed.** `Payload` uses `BTreeMap`, so both rendering paths are ordered by construction. F15 asks RFC-0001 to choose a deterministic map; this is that choice. |
| `Val::Token`, and the four `dt` constants with no writer (F36, F38) | **Deliberately omitted.** An arm no constructor writes carries no behaviour. Recorded in the `Payload` definition. |
| Identity and stamp minted per construction, normalised out of the goldens (F14) | **Preserved observably.** F14 is why the goldens carry `<id>` and `<stamp>`; laziness is invisible to them either way, which is what makes D1 and D2 capturable at all. |
| Equality by identity for `Bool`, `Nodata`, `None` | **Deliberately changed.** Not because scalars lack an identity — a *boxed* one has a header, and that reason died when review 3 made scalars boxable — but because equality must not depend on whether an operand happens to be boxed. Unreachable through `==`, which bails on non-numeric types. See the scalar section. |
| Mutation resetting `attr`, `curr`, `tags` (F34) | **Preserved exactly.** The header is rebuilt, not copied — `make_mut` would copy it, which is the divergence this row exists to close. |
| `push` on a `RESULT` yielding a `LIST` (F35) | **Deliberately fixed.** The `dt` is preserved, as `set` already does for maps. No golden covers it. |
| `ASSOCIATION` readable but unwritable (F36) | **Deliberately omitted.** A `dt` constant with no constructor and therefore no values; omitting it removes no behaviour. |
| `stamp` sampled per construction | **Preserved observably, changed mechanically.** Sampled on first observation (D2). Two never-observed values may share a stamp; nothing can distinguish them. |
| Ordering falling back to `id.cmp` | **Deliberately fixed**, per F12's disposition, which is FIX. `Ord::cmp` and `PartialOrd::lt` disagree today — `lt` returns `true` for any type mismatch (`reference/rust_dynamic/src/ord.rs:16,24`) while `cmp` falls back to `id.cmp` (`:175,183,191,199`). An earlier draft's row said "preserved exactly", contradicting F12. |
| Hash by identity | **Deliberately changed, per D30.** `hash` mirrors `eq` arm for arm, on the same partition. Unobservable before the `get` mirror exists, since F29 leaves no read path. |
| `valuemap` unreadable (F29) | **Deliberately fixed, per D30.** The `get` word branches on the container's type before casting the key, mirroring `set`. |
| `tags`, including the per-push stack tag | **Preserved exactly**, as a field. |
| `attr`, drivable by the `attribute` word | **Preserved exactly**, as a field. |
| `curr` as the iteration cursor | **Preserved as a field.** Seven `for … in` loops advance it (`reference/Bund/src/cmd/bund_cluster.rs:35,40,305,310,437`, `bund_bbus.rs:97,172`); Bund2 iterates on a locally owned cursor, so iteration neither splits nor mints. |
| `q` averaged by arithmetic and propagated | **Preserved exactly**, as a field. Two earlier drafts rendered it as the constant `100.0`; it is a *fixpoint* at 100.0, not a constant, and `Value::none` — which the JSON converter returns for a null — starts at `0.0`. Q18 closed on a probe. |
| `Value::has_key` on a `VALUEMAP` (Q19) | **Expressible at the value layer, deferred at the word layer.** `Payload::ValueMap` is a `HashMap<BundValue, BundValue>`, so a key lookup needs no new machinery here. Whether `?key` exposes it is RFC-0002's, since D30 mirrored `set` into `get` and `?key` has no `set` counterpart to mirror. |
| `stamp` reset by `set` and `push` | **Preserved exactly.** The rebuilding constructors write `timestamp_ms()` (`reference/rust_dynamic/src/create_map.rs:34`), so a mutated container's stamp is the mutation's, not the original's — which D2 names and no earlier row carried. |
| `set` on a `LIST`/`RESULT` dropping the container | **Deliberately fixed**, with F35. `set` on a list returns `Value::from_list(vec![value])` (`reference/rust_dynamic/src/set.rs:9`) — it discards the existing container *and* the `dt`, where the map arm restores `dt` (`:22`). F35 records the same defect in `push`; this is its sibling and was unrecorded. |
| Binary wire format byte-identical | **Preserved exactly.** D20. The lazy identity materialises at this boundary. |
| `.id` returning a 21-character nanoid string | **Preserved exactly.** The counter formats into the reference's alphabet. |
| JSON round trip losing identity | **Preserved exactly**, including the loss. D20's scope correction. |
| `dup` as a bincode round trip | **Deliberately changed** to one header allocation plus a payload `Rc` bump. Observably identical: both yield an equal payload with a fresh identity. |
| `push_to_stack` capacity check | **Deliberately fixed** — F28. No golden exercises it. |


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

Three kinds, kept apart, the way RFC-0000 separates its binding criteria from
its deferred ones. **Four are binding**: each runs today and each can fail.
**Two are vacuous but real** — they run and cannot currently fail, and are
labelled rather than counted. **Eight are deferred to the implementation**,
because `bund2-value` is three lines and nothing in it can be measured yet.

Accepting this RFC means accepting the four. An earlier draft listed all
fourteen as though they were checkable, which would have made "accepted" mean
accepted against four while appearing to mean fourteen.

### Binding — these run today and can fail

**B1. Constructing and cloning an *unadorned* scalar allocate 0 times**, for
candidate D, measured by the counting allocator in `cargo xtask layout`. The
qualifier is load-bearing: `TS::push` tags unconditionally
(`reference/rust_multistack/src/ts_push.rs:25`), so a scalar on a stack is
boxed, and **boxing one costs 5 allocations and 721 bytes** — also measured,
in the row beside it, so the qualifier cannot quietly become an excuse. 47 of
the 54 scalar renderings in the goldens carry a non-empty `tags`.

**B2. `dup` of a heap list on a stack allocates 4 times and 649 bytes**, and
`dup` of a boxed scalar the same, against the reference's full bincode
serialise-and-deserialise. `cargo xtask layout`, which carries `dup` rows
because an earlier draft claimed "1 allocation — one header" and was both
wrong and unmeasurable.

**B3. `cargo xtask cite` reports zero defects** — over whatever count the tool reports at this
commit. Note what it does **not** check: that a cited line means what the
prose says. Reviews 1 through 4 each found citations that resolved and
misled, and `cite` passed every time. Review 5 found none, which is the first
pass where that was true.

**B4. `cargo xtask lint` reports no contradictions.** It cross-checks every
preservation row against the disposition of the defect it cites, catches a
duplicated section heading, catches a claimed count that has drifted from its
artefact, and catches a type used in a `rust` block that no RFC introduces —
the last of which was added because `Payload` was undefined here for two
revisions while the history said otherwise (F41). **What it structurally
cannot see is a row that is absent.** Four were missing when review 4 read
this RFC, including the largest deviation in it. B4 is a floor, not a
substitute for a reader.

### Vacuous but real — they run, and cannot fail yet

**V1. `cargo xtask conform` does not fall below the mark** in
`tests/golden/CONFORMANCE.txt`, now **0/69**. At 0/69 nothing can fall. It
becomes real with the first golden Bund2 passes.

**V2. `cargo tree -p bund2-value` lists no `bund2-interp`.** It passes because
the crate is empty, not because the boundary is enforced. Real once
`bund2-value` has contents — which is this RFC's own implementation.

### Deferred to the implementation

Each names what makes it checkable. None can run against a three-line crate.

**D1 — MET.** `size_of::<BundValue>()` is 16 and `align_of` is 8, asserted on
the real type in `crates/bund2-value`. `cargo xtask layout` reports the same
for candidate D.

**D2. `A == A.clone()` is true and `A == A.dup()` is false**, for every
**identity-compared** kind — a list, a map, a lambda. This is D13's contract,
and it is listed because an earlier draft broke it.

**Not "every kind with a heap header", which is what three earlier drafts
said.** A string has a header *and* compares by content, so `dup` cannot make
it unequal: the oracle prints `true` for `"s" dup ==`. The discriminator is
how the kind compares, not whether it has somewhere to keep an id. Found by
the implementation's own test, which failed on `str` while passing on `list`
and `map`.

**Not for a bool** either: `true == true.dup()` is true in Bund2 and false in
the reference. The reason there is stability under boxing — a boxed scalar
*does* have an identity slot — not the absence of one.

Note what the criterion can and cannot be checked through. The `==` **word**
rejects lists and maps outright (`COMPARE: unsupported operand`), so this
contract is observable only through `PartialEq` — `ValueMap` keys and
container membership — and not from Bund source.

**D3 — checked by `cargo xtask render`.** The `Debug` rendering reproduces the
normalised text the goldens hold
for **every rendering in `tests/golden/`**. The criterion names the set and
not a number: the figure drifted in four consecutive reviews because it was
incremented by hand rather than re-derived, and `cargo xtask lint` cannot see
prose arithmetic. Not the reference's raw
output: the goldens already normalise `id`, `stamp` (F14) and map order (F15),
and this RFC orders the dict rendering deliberately, so a criterion written
against raw output would demand the order this RFC removes.

The check is differential rather than sampled: `cargo xtask render` parses
every captured rendering back into a `BundValue`, renders it, and compares.
**59 of the 87 top-level renderings round-trip identically and none differs**;
the other 28 are values Bund2 cannot build yet, counted apart, because
conflating *not built* with *built wrong* is how a coverage number turns into
a pass. An earlier version of this criterion was checked against five
hand-copied strings.

**D4. The bincode wire format is byte-identical to the reference** for every
payload arm a probe can construct on the oracle. **Byte-identity is impossible for maps, and the reference is why.** `Val::Map`
and `Val::ValueMap` are `HashMap`s, bincode serialises a map by iterating it,
and Rust's `HashMap` iterates in a per-process random order — so the reference
emits different bytes for the same map on two runs. That is **F45**, F15's
defect reaching the wire. The criterion therefore reads: byte-identical for
the **map-free** arms, and *decodable in both directions* for the rest — the
reference can read what Bund2 writes and Bund2 can read what the reference
writes. An earlier draft asked for byte-identity across the board, which
presumes the reference has one answer to compare against.

**The scope is measured, the bytes are not.**
`tests/probes/payload-arms.bund` constructs one value of each reachable arm
and `tests/golden/probes/payload-arms.golden` pins them — but that golden
holds the **`Debug` rendering** and contains no bincode at all, so it settles
*which arms to compare* and checks none of them. The byte comparison needs a
probe that does not exist: something that serialises each arm and emits the
bytes in a capturable form. The set is 11 — `Bool`,
`I64`, `F64`, `String`, `Null`, `List`, `Map`, `ValueMap`, `Metrics`,
`Lambda`, `Json`, across 13 `dt` values, since `String` carries both `STRING`
and `PTR` and `List` carries both `LIST` and `PAIR`. Two earlier drafts said
twenty and then nineteen, both from counting declarations rather than
reachability.

**D5 — MET for the format, deferred for the VM seed.** `.id` returns a
21-character string over the reference's nanoid alphabet and 1000 values in
one process do not collide, both asserted in `bund2-value`. The seed is a
process counter until RFC-0002 supplies a VM. **And observation is
idempotent through a container**: `l 0 get .id` twice returns the same id,
which requires the promotion to be written back — see the scalar section. The
second clause is the one an earlier draft's rule did not deliver.

**D6. `valuemap "k" 42 set "k" get` leaves `42`**, not the map — D30's read
path. And a valuemap keyed by a freshly built equal *scalar* finds its entry
while one keyed by a freshly built equal *list* does not: that asymmetry is
D30's stated limit and is asserted, not left to chance.

**D7. `.timestamp` returns a float**, and the stamp orders by **observation,
not construction** — which is what lazy sampling gives up, and an earlier
version of this criterion asserted the opposite by demanding non-decreasing
stamps for independently *constructed* values. Under D2 a value constructed
first and observed second carries the later stamp. Two consequences are
asserted rather than worked around: two never-observed values may share a
stamp, and a value and its `dup` always do, because `dup` materialises before
copying.

**D8. Equality across int and float is exact, symmetric and transitive.**
`42 == 42.0` is true; `42 == 42.5` is false **in both operand orientations**;
`2^53+1` versus `2^53.0` is false in both. The orientation matters — the
reference already answers false for the truncating one, so asserting only that
tests nothing; it is the widening orientation, where the reference answers
true, that this constrains. And `42` and `42.0` hash alike, `-0.0` and `0.0`
hash alike, `NaN` equals nothing.

## Open questions

- **F33 is settled** by D30's amendment: equality across int and float is
  exact, and therefore symmetric, transitive and hashable. Q20 closed. Two
  goldens now pin behaviour Bund2 will deliberately change —
  `eq-asymmetry.golden` and `valuemap-hash-eq.golden` — and both are
  regenerated with `--reason F33` and `--reason F29` respectively once Bund2
  can run them.
- **Q18 is closed and Q19 divided.** Q18 asked whether any word can observe
  `q` away from the 100.0 fixpoint; `"null" json json.to_value` yields
  `q: 0.0` on the oracle, through two registered in-scope words, so `q` is a
  field. Q19's value-layer half needs no decision — `Payload::ValueMap` is a
  `HashMap<BundValue, BundValue>`, so the lookup is expressible by
  construction — and its word-layer half, whether `?key` exposes it, goes to
  RFC-0002 because D30 mirrored `set` into `get` and named only `get`.
- **F29 and F30 are settled by D30** — hash mirrors equality, and the `get`
  word mirrors `set`. Nothing OPEN now gates this RFC. Two residues:
  `tests/golden/probes/valuemap-hash-eq.golden` pins the *broken* behaviour
  and must be regenerated with
  `cargo xtask golden --accept valuemap-hash-eq --reason F29` once Bund2 can
  run it; and **Q19** asks whether `?key` gets the same branch, which D30 did
  not name.
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
  corpus programs not 138, 250 renderings across 28 goldens not 244 across 27 — a figure that has moved with every probe added since
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

- **2026-08-26, review 2** — `docs/rfc/reviews/RFC-0001-review-2026-08-26-2.md`.
  Verdict: do not accept, with review 1's ten items confirmed substantially
  done — every citation opened this pass said what the prose claimed, and
  every count reproduced. What blocked it was that the **design contradicted
  three of its own preservation rows**, which is a worse failure than a wrong
  citation because the table and the design were each internally plausible.

  Scalars carry no identity, so `Bool`/`Nodata`/`None` could not preserve
  identity-equality or identity-ordering, and criterion 4 demanded
  `A == A.dup()` be false *for a bool* — unsatisfiable by the structure two
  sections above it. The review also supplied the repair: the `==` word bails
  on every non-numeric type, so the behaviour is unreachable through `==` and
  this is a deviation rather than a redesign. The draft had not made that
  argument.

  Criterion 3 claimed `dup` allocates once. It allocates **4 times, 649
  bytes**, because `HeapValue` carries `tags` and `attr` by value and `tags`
  is never empty on a stack value. It was also unmeasurable: `layout` had no
  `dup` row for any candidate. Both fixed.

  `set` and `push` do not *split* a value, they **reset its header** —
  `from_dict`/`from_list`/`from_valuemap` clear `attr`, `curr` and `tags`,
  where `make_mut` would copy them. Oracle-confirmed and recorded as F34. It
  was absent from the preservation table entirely.

  `stamp` had no section, no preservation row and no criterion, though D2 was
  declared consumed and requires its deviation be listed.

  On D30: content hashing is sound, and the review says why — the `Hash`/`Eq`
  contract is one-directional, so content-hash plus identity-equality is legal
  and costs only bucket collisions. But two things must be settled first, and
  the larger is **F33**: `PartialEq` is asymmetric across int/float, so no
  bucket assignment can be consistent with it. Carried as **Q20**, and it is
  what now blocks acceptance. The second is a per-arm hashing rule for the
  four float-bearing arms.

  Four further defects recorded from this pass: **F33** asymmetric equality,
  **F34** the header reset, **F35** `push` on a `RESULT` yielding a `LIST`,
  **F36** `ASSOCIATION` readable with no writer. The fifth item, `get`
  stringifying the key before it sees the container, is folded into **F29** as
  its word-layer cause — it is why no change confined to `rust_dynamic` can
  fix F29, and why D30's fix is a word change. `?key` needs the same and is
  Q19.

  Note: the review was written against a tree predating commit `c7edd9a`, so
  its closing question — whether D30 should be appended — was already answered.
  D30, and F29/F30's dispositions, were recorded before it arrived.

- **2026-08-26, review 3** — `docs/rfc/reviews/RFC-0001-review-2026-08-26-3.md`.
  Verdict: do not accept. All 49 reference citations were opened and read;
  review 1's A-class defects have not returned, every count reproduced, and
  `cargo xtask layout` reproduced every figure including criterion 3's
  `dup a list  4  649`.

  **The scalar repair had been applied to one field of six.** Review 2 found
  that a `Bool` has no identity slot; it has no `tags`, `attr`, `curr` or
  `stamp` slot either, and `TS::push` tags unconditionally with no type test —
  the same fact criterion 3 cites to explain why heap `dup` costs four
  allocations. The goldens settle the scale: 27 of the 258 renderings have a
  scalar payload, **24 with a non-empty `tags`** and 3 with a populated
  `attr`, and those three are in this RFC's own probe, the one cited as the
  reason `attr` is a field. Criterion 6 was unsatisfiable for 24 renderings
  and criteria 2 and 3 contradicted it. Scalars are now the *unadorned* form
  and box when they acquire a header; the cost is stated rather than elided,
  and the stack-slot optimisation that would avoid it is handed to RFC-0003,
  where stack representation is decided.

  **The 2^53 evidence did not say what it was cited for.** The probe wrote the
  truncating orientation and labelled it as the widening one, so the widening
  failure — the one that motivates exactness — was **unpinned**, and "both
  failures are pinned" was false. Both orientations are now written out and
  captured; the golden reads `false` for truncating and `true` for widening.
  Criterion 12 was consequently asserting an answer the reference already
  agrees with, and now names the orientation that actually constrains.

  Four more, each verified: the ordering row said "preserved exactly" while
  F12's disposition is **FIX**; the equality and hash rows said "twelve" and
  "sixteen" for one partition, which is now stated precisely rather than
  counted loosely (four arms by content, `Val::List` mixed, fifteen by
  identity); "the `==` word accepts only numeric types" was **false**, there
  is a `STRING` arm at `logic_compare_fun.rs:47` — the claim that survives is
  the narrower one, that there is no `BOOL` arm; and the payload table was
  still missing `MESSAGE` (review 2's A3) and `PAIR`, both of which are set by
  assigning `dt` *after* construction, which is why a scan for the field
  initialiser could not see them.

  Registers: **F38** records that `Val::Token` has no constructor anywhere,
  which made criterion 7's "each of the 20 payload kinds, captured from the
  oracle" unsatisfiable — it now asks for nineteen. **F36** is extended from
  `ASSOCIATION` to all four `dt` constants with no writer: `LITERAL`,
  `LARGE_FLOAT`, `ASSOCIATION`, `TOKEN`, leaving 38 live of 42 declared.

- **2026-08-26, review 4** — `docs/rfc/reviews/RFC-0001-review-2026-08-26-4.md`.
  Verdict: do not accept. Review 3's items were all done; all 50 citations
  resolved and read correctly, `layout` reproduced every allocation figure,
  and the payload/`dt` table verified row by row.

  **The design contradicted D13, not merely its own table.** D13 states the
  condition in terms — *"RFC-0001 must show one policy governing both"* — and
  the draft showed one half. `Clone` as an `Rc` bump shares the identity slot,
  so clone-equal holds; `make_mut` copies the slot, an unminted slot copies as
  unminted, and the two halves then mint different ids. `set_tag` on every
  push is a minting-free mutation that this RFC itself classifies as a split,
  so the break fires on every push of a cloned value. The policy is now one
  rule: **a CoW split materialises the identity before it copies**, which is
  what the reference produces, since its `Clone` deep-copies the `id` string.
  `stamp` follows it. And `curr` was worse than unanswered — as a `Cell`
  inside the shared `Rc` it gave the cursor *reference* semantics across
  clones; it is a plain field now, split by `make_mut` like any other.

  **`Payload` was used and never defined**, which is why F15's "choose a
  deterministic map — RFC-0001 territory" and F36's "record it in RFC-0001's
  tag table" were both unanswered. It is defined now, with `BTreeMap` for both
  map kinds — F15's actual request — and with the arms F36 and F38 rule out
  omitted.

  **Criterion 7's "nineteen" was still unsatisfiable.** Review 3 cut it from
  twenty by removing `Val::Token`, but the criterion's method is oracle
  byte-capture, which needs reachability from Bund rather than a Rust
  constructor. `Value::exit`, `Value::operator` and `Value::embedding` have
  zero callers anywhere, so at most sixteen arms qualify. Rather than guess a
  third number, the criterion now says the probe suite enumerates the set.

  **The scalar counts were wrong in four places** — 34 scalar renderings with
  29 tagged, not 27 and 24 (`I64` 17/12, `Bool` 12/12, `F64` 3/3, `Null` 2/2).
  The earlier scan required a parenthesised payload and missed `Null`
  entirely. The argument was strengthened by the correction; the figures had
  simply not reproduced.

  **Four preservation rows were missing**, including the largest deviation in
  the RFC: `42 == 42.5` flipping true to false, which disagrees with a
  captured golden. The two equality rows covered heap arms and
  `Bool`/`Nodata`/`None`, and `Int` and `Float` fell between them. F15, F38
  and F14 had no rows either. Criterion 14 leaned on `cargo xtask lint` as
  though it could catch this; **a missing row is structurally invisible to
  it**, and the criterion now says so.

  And the ordering analysis covered the unreachable path. `lt`, `le`, `gt` and
  `ge` are each overridden (`reference/rust_dynamic/src/ord.rs:9,48,87,126`),
  none reads an id, and those are what a sort reaches — `partial_cmp`
  delegates to `cmp` and cannot disagree with it. F12 mis-cited itself the
  same way, attributing `ord.rs:19-21` to `partial_cmp` when those lines are
  inside `lt`; corrected there too.

  **Corrected by the repository owner during this pass:** review 4 reported
  `Value::exit` as having zero callers and this RFC repeated it. It has one —
  the parser's `EOI` handler
  (`reference/bund_language_parser/src/vm/eoi.rs:8`) — so every parsed program
  ends with an `EXIT` value and three evaluation loops break on it. Both the
  review's search and this RFC's covered four crates and not the parser.
  Recorded as **F39**, as a defect in method rather than in the reference,
  because the same four-crate habit produced F19, F25, F36 and F38. Those were
  re-checked across all six crates and hold; `Value::operator` and
  `Value::embedding` do have zero callers.

- **2026-08-26, review 5** — `docs/rfc/reviews/RFC-0001-review-2026-08-26-5.md`.
  Verdict: do not accept — but **the citation work is finished**: the first
  pass on this RFC where no citation resolved to a line saying something other
  than what the prose claimed, with every measured figure reproducing.

  What blocked it was a different class, and two of the blockers were **false
  negative claims that failed inside the search scope this document itself
  names** — the F39 failure mode, recurring.

  `impl Iterator for Value` has **six callers**, all `for … in` loops in
  `reference/Bund/src/cmd/`. Two earlier drafts said none, because both
  searched for `.next()` and `into_iter()` and never for the sugar that
  actually consumes an iterator. So `curr` is advanced, and `next` writes it in
  place — a minting-free mutation the CoW policy had never been applied to.

  `q` has **two writers**, `calc_q` and `set_q`, reached from the arithmetic
  operators, so every arithmetic operation writes it. The goldens show `100.0`
  because it is a **fixpoint**, not a constant — and the fixpoint is escapable
  in scope, since the JSON converter returns `Value::none()` for a null and
  that is `Value::new` with `q: 0.0`. `q` is a field now and **Q18 is
  reopened**; it had been closed as "grounded" on a scan that never opened
  `q.rs`.

  **`Payload` was still undefined**, and the fourth revision's history said it
  had been defined. The scripted edit's target text did not match — `id` where
  the document says `identity`, and different alignment — so `str.replace`
  returned the string unchanged and the script exited 0. Two preservation rows
  and the history then asserted work that was not in the file, and `lint` could
  not see it because they agreed with each other. Recorded as **F41**, and
  `cargo xtask lint` now carries the check that catches the shape: a type used
  in a fenced `rust` block that no RFC introduces. `NativeFn` in RFC-0002 was
  the same defect.

  The identity policy contradicted F34's header reset: `set` on a map was
  classified as both a CoW split, where both halves share an id, and a header
  rebuild, where the result gets a fresh one. Only the second is the
  reference. The two rules now apply to disjoint sets — minting mutations
  rebuild, and `set_tag` alone is a minting-free split.

  Counts: 66 goldens not 65; a leftover `24` where review 4 corrected it to
  29; and "55 dup-family tokens" is 55 uses of `dup` alone, `dup_one` and
  `dup_many` having none. Preservation row 1 said `dt` is carried over 42
  constants while rows 12 and 18 omit four — it is **38 live of 42**, which is
  what F36 asks for.

  Criterion 5 was vacuous and unlabelled where criterion 9's equivalent is
  labelled. Criterion 6 was unsatisfiable as written: the goldens already
  normalise F15's map order, so the target is the normalised text, not the
  reference's raw output — and this RFC replaces `HashMap` with `BTreeMap`
  deliberately. Criterion 7 delegated to a probe suite that does not exist.
  All three say so now.

  Three preservation rows added: `Value::has_key` on a `VALUEMAP` (Q19's
  value-layer half), `stamp`'s reset by the rebuilding constructors, which D2
  names, and `set` on a `LIST` discarding both the container and the `dt` —
  F35's defect in the sibling word, now **F42**.

- **2026-08-27, post-review-5 work.** Four items, none of which needed a
  review to identify — they were the residue review 5 left.

  **The criteria are partitioned.** Fourteen were listed as though checkable
  while `bund2-value` is three lines and eight of them cannot run at all.
  RFC-0000 separates binding from deferred and says so; this RFC now does the
  same — 4 binding, 2 vacuous-but-labelled, 8 deferred with each naming what
  makes it checkable. Without the split, accepting this RFC would have meant
  accepting against four criteria while appearing to mean fourteen.

  **Q18 is closed on a probe rather than a scan**, which is the whole point:
  `"null" json json.to_value` yields `q: 0.0` on the oracle through two
  registered in-scope words. `tests/probes/q-observable.bund`.

  **Q19 is divided** rather than left "undecided" in a table whose job is
  dispositions. The value layer needs no decision; the word layer is
  RFC-0002's.

  **Criterion D4's probe suite exists now.**
  `tests/probes/payload-arms.bund` constructs one value of each arm a Bund
  program can reach and the golden pins them: **11 payload arms across 13 `dt`
  values**. That replaces two guesses — twenty, then nineteen — both made by
  counting declarations rather than reachability.

  Enumerating them turned up one more defect: `wrap` bails with `"Stack is too
  shallow for inline UNWRAP"`
  (`reference/Bund/src/stdlib/functions/oop/value_class.rs:85`), the same
  wrong-word shape as `complex` reporting as `pair`. F40 is extended to cover
  both.

  The mark moved 0/66 to **0/68**.

- **2026-08-27, review 6** — `docs/rfc/reviews/RFC-0001-review-2026-08-27.md`.
  Verdict: do not accept. Citations held for a second consecutive pass and
  every `layout` figure reproduced. Nine findings, two confirmed by fresh
  oracle probes, and every one held on re-verification.

  **The mutation policy had two buckets and needs three.** `attr_add` is
  `dup().regen_id()` then a push (`reference/rust_dynamic/src/attr.rs:19-20`):
  it **mints and preserves**, where a rebuild mints and empties. Filing it
  under rebuild would have emptied the `attr` the word exists to populate.
  Oracle: `1 2 attribute 3 attribute` renders two entries with tags intact.
  And `set_tag` is not the only minting-free mutation — `next` writes `curr`
  and the arithmetic operators write `q`, both of which **this RFC establishes
  several pages earlier**, plus a direct `dt` write in `merge.rs:83`.

  **`Payload::ValueMap` as a `BTreeMap` deleted D30's mechanism.** D30 decides
  hash by content and says so in terms — keys "hash into the same bucket". A
  `BTreeMap` does not hash; it needs a total `Ord` that F12's FIX deletes, that
  `NaN`-equals-nothing forbids, and that the four overridden comparisons do not
  provide. `Map` stays a `BTreeMap` because `String` keys order trivially and
  that is what F15 asked for — **dict members and tags, not valuemap**.
  `ValueMap` is a `HashMap`, and rendering determinism comes from the renderer.

  The rebuild rule omitted `q`, which the constructors reset to 100.0 while
  the fallback arms copy it, and `regen_id` resets `stamp` as well as `id`.

  **Two arguments rested on a premise review 3 removed.** The preservation row
  and criterion D2 both said scalars "have no identity to compare"; a *boxed*
  scalar has one. What survives is that equality must not depend on whether an
  operand happens to be boxed, which is a better reason. And "boxing is
  invisible to the language" was defended only for equality, ordering and
  hashing: `.id` and `.timestamp` mint on observation and an unadorned scalar
  cannot keep what it minted, so those two **box**. The state is reachable —
  `"[1,2]" json json.to_value` leaves inner scalars with `tags: {}`.

  `dup`'s treatment of `stamp`, `tags`, `attr`, `curr` and `q` was
  unspecified. The reference overwrites **only** `id` (`dup.rs:11`), so a value
  and its `dup` share a stamp — which lazy sampling would break unless `dup`
  materialises first. It now does.

  Three sections still asserted what reviews 4 and 5 had corrected: §Iteration
  and the `curr` row said "no caller", §Rendering still rendered `q` as a
  constant, and Q18 was called reopened in two places and closed in two. Every
  golden-derived count had drifted again — 68 goldens, 278 renderings across
  31, 45 scalar with 40 tagged — so criterion D3 now names the golden **set**
  rather than a number.

  Criterion D7 asserted non-decreasing stamps for independently *constructed*
  values, which is exactly what lazy sampling gives up; it now says stamps
  order by observation. And RFC-0002 cited this RFC for "`BundValue` is not
  `Send`", a claim it never made — now stated here, where the dependency
  points.

- **2026-08-27, review 7** — `docs/rfc/reviews/RFC-0001-review-2026-08-27-2.md`.
  Verdict: do not accept, with review 6's items "acted on, and acted on well".
  **What blocked it was that the repairs landed in the design body and not in
  the sections that quote it** — which is the diagnosis, not a list of
  symptoms.

  One citation defect, the first in three passes and in the negative-claim
  area this RFC names as its own weakness: `bund_bbus.rs:177` is a `match`,
  not a loop, and `impl Iterator` has **seven** callers, not six —
  `bund_bbus.rs:97` and `bund_cluster.rs:437` were both missed, and the second
  pushes each element onto a VM stack.

  Eight repairs had not propagated. §"Value semantics" still carried the
  two-class mutation taxonomy twenty lines after the body replaced it with
  three; the `Bool` row and criterion D2 still gave the "scalars have no
  identity" reason the scalar section retires in terms; three places still
  called `ValueMap` a `BTreeMap`; the `curr` row still said nothing advances
  it. The fix in each case was to have **one** statement rather than a better
  second one — §"Value semantics" now points at the partition instead of
  restating it.

  Three new contradictions, each following from review 6's own repairs.
  §"Hashing" enumerated four content-hashed kinds where this RFC has six, and
  a `Bool` that compares by content while hashing by identity breaks
  `Hash`/`Eq` in the one structure consuming both. `HashMap` requires `Eq`,
  `Eq` requires reflexivity, and "`NaN` equals nothing" denies it — the fault
  this RFC convicts the reference of, made reachable by D30's read path; there
  are now two equalities, the word's IEEE one and the key's total one. And the
  `.id`/`.timestamp` promotion rule did not close its own hole: `get` returns
  a clone, so promoting it writes nothing back and `l 0 get .id` twice still
  mints two ids — the write-back is now stated at the container.

  Criterion D4 pointed at `payload-arms.golden` for a **byte**-identical wire
  format; that golden holds the `Debug` rendering and contains no bincode. It
  settles which arms to compare and checks none of them.

  Counts drifted for the fourth consecutive review — 303 renderings across 32
  goldens, 54 scalar payloads — because they were incremented rather than
  re-derived. D3 now names the golden **set** and no number.

  **This is the pass on which the RFC moves to Proposed.** The design has
  survived seven adversarial reads and the seventh called it sound; what keeps
  failing is document maintenance on a 1300-line artefact that states each
  claim in four places. A compiler enforces what prose does not, and eight of
  the criteria cannot run until `bund2-value` exists.
