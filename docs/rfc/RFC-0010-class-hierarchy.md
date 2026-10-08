# RFC-0010: The class hierarchy

- Status: **Proposed** (2026-10-05). Its three questions were answered by the
  repository owner the same day and recorded as **D96**: additive only, `?is`
  accepted, `iset` taken. §S1 and §S4 are built; §S2's rewiring is declined
  and stays as the analysis of what it would have cost. **All five criteria
  are met as of 2026-10-08**, each with the test or tool that decided it. The
  acceptance review of that date (`docs/rfc/reviews/`) found criterion 4
  without a test and criterion 5 unable to fail; both are answered below.
- Depends on: RFC-0009 (classes, objects and dispatch — Accepted), which
  specifies the *mechanism* this document reasons about and deliberately does
  not propose a hierarchy
- Decisions consumed: D23 (`!` builds from the class value), D25 (a class must
  carry its own `.class_name`), D72 (Bund2 may carry what the reference does
  not)
- Decisions produced: **D96**, answering all three questions below
- Grounded against the pinned submodules and the oracle on 2026-10-05

## Summary

Bund already has a class hierarchy. It is six levels deep, every built-in
class declares exactly one parent, and **its root is a printing protocol
rather than `Object`** — `Object`'s parent is `Printable`, whose parent is
`Display`, whose parent list is empty. That is the inverse of Smalltalk, where
`Object` is the root and printing is protocol *on* it.

This document writes the existing hierarchy down, measures where it departs
from the Smalltalk model, and proposes how to extend it. Its central finding is
that the interesting half of a Smalltalk-shaped hierarchy — `Magnitude`,
`Number`, `Collection` — cannot be **inserted** without moving a golden, so the
proposal is split into an additive half that costs nothing and a rewiring half
that is a deviation and needs a decision.

## Motivation

When this was written three words were unimplemented in `bund/oop` — `List`,
`Floats` and `Intervals` — and each is a *class* plus a constructor word, so
implementing them meant deciding where they sit. They were the last three, so
that was the moment the shape of the whole was decided rather than extended by
accident. All three are built now (§S1) and a golden runs them.

Beyond that: a reader of `Printable` and `Display` reasonably expects
Smalltalk's arrangement and will mis-predict Bund's. Writing the difference
down is cheaper than having it rediscovered per class.

## Current behaviour

All of this is read from the pinned submodules and confirmed against
`target/oracle/release/bund`.

### 1. The hierarchy, as registered

Each arrow points from a class to the parent it declares in `.super`. The
table under the diagram says where each row was read.

```
Display                     .super []            format, display
└── Printable               .super [Display]     str, print, println
    └── Object              .super [Printable]   .id, .timestamp
        └── Value           .super [Object]      .init
            ├── Integer     .super [Value]       .init
            ├── Float       .super [Value]       .init
            ├── Bool        .super [Value]       .init
            └── List        .super [Value]       .init, push
                ├── Floats      .super [List]    .init, push
                └── Intervals   .super [List]    .init, push, overlap
```

| class | declared at |
|---|---|
| `Display` | `reference/Bund/src/stdlib/functions/oop/display_class.rs:101-105` |
| `Printable` | `reference/Bund/src/stdlib/functions/oop/base_classes.rs:109-115` |
| `Object` | `reference/Bund/src/stdlib/functions/oop/base_classes.rs:95-100` |
| `Value` | `reference/Bund/src/stdlib/functions/oop/value_class.rs:76-80` |
| `Integer` | `reference/Bund/src/stdlib/functions/oop/int_class.rs:35-39` |
| `Float` | `reference/Bund/src/stdlib/functions/oop/float_class.rs:35-39` |
| `Bool` | `reference/Bund/src/stdlib/functions/oop/bool_class.rs:35-39` |
| `List` | `reference/Bund/src/stdlib/functions/oop/list_class.rs:63-68` |
| `Floats` | `reference/Bund/src/stdlib/functions/oop/floatlist_class.rs:72-77` |
| `Intervals` | `reference/Bund/src/stdlib/functions/oop/intervals_class.rs:204-210` |

**The diagram is read by a test**, `the_registry_is_the_hierarchy_rfc_0010_draws`
in `crates/bund2-stdlib/src/oop.rs`, which takes the ten rows out of this file
and compares each with what Bund2 registers: the class, its `.super`, and its
slots. Keep the three columns in the shape they have.

**`.super` is a LIST**, so multiple inheritance is representable; every
built-in uses exactly one parent, so the hierarchy is a chain in practice and a
DAG in principle. `locate_value_in_object`
(`reference/Bund/src/stdlib/functions/oop/value_class.rs:10-29`) walks it
depth-first, left to right, first match wins — which is a multiple-inheritance
resolution order already, exercised by nothing.

### 2. Only four of the ten classes have a constructor word

Four classes, through five words. `True` and `False` both construct `Bool`
(`reference/Bund/src/stdlib/functions/oop/bool_class.rs:73-74`); `List`, `Floats` and
`Intervals` construct themselves — `stdlib_object_list_value_empty`
(`reference/Bund/src/stdlib/functions/oop/list_class.rs:71-75`, registered at
`reference/Bund/src/stdlib/functions/oop/list_class.rs:96`) pushes an empty LIST, pushes
`"List"`, and applies `object`. An earlier revision's heading said five of
the ten, counting words for classes. **`Integer`,
`Float`, `Value`, `Object`, `Printable` and `Display` have no word**: they are
reachable only through `object` with a literal name, or through `!` on a class
value (D23).

### 3. Construction runs *every* ancestor's `.init`, root first

`make_bund_object` (`reference/rust_multistackvm/src/stdlib/bund_object.rs:27-106`)
copies the class, flips the tag to `OBJECT`, then for each name in `.super`
resolves that class, **recursively constructs a parent object, runs that
parent's `.init`**, and pushes the result into a new `.super` list. So a class's
`.super` holds *names* and an instance's holds *objects* — the asymmetry
RFC-0009 records — and the data an `.init` consumes comes off the stack.
The recursion constructs a parent before it runs that parent's `.init`, so
the root's initialiser runs first and the leaf's last. An earlier revision's
heading said "bottom-up", which is the opposite of the paragraph under it.

That is how `List` works: the operand is pushed under the class name, `Value`'s
`.init` (`.value_init`, `reference/Bund/src/stdlib/functions/oop/value_class.rs:54-70`)
pulls the object and then pulls the operand into
`.data`, and `List`'s own `.init` then *locates* `.data` through the ancestry
and converts it to LIST. A leaf constructor depends on its parent's constructor
having run and consumed the stack.

### 4. `is` is not a membership test

Measured: `stdlib_object_value_is`
(`reference/Bund/src/stdlib/functions/oop/value_class.rs:127-146`) takes one OBJECT and
pushes **the object back together with its `.data`**. `List "List" is` fails
with `IS: NO OBJECT IN #1`
(`reference/Bund/src/stdlib/functions/oop/value_class.rs:135`), because the string is on top. There is no
`isKindOf:`, no `respondsTo:`, and no word that answers whether an object is of
a class — `?object` answers only *whether a value is an object at all*.

### 5. What a golden already pins

`tests/golden/probes/bool-objects.golden` captures the **fully materialised
ancestry** of a `Bool` object: 20 `.super` slots and 20 `.class_name` slots,
naming `Display`, `Printable`, `Object` and `Value`. **Any change to the
ancestry above `Bool` moves that golden.** This is the binding constraint on
everything below, and it is why this document is split the way it is.

## The Smalltalk model, and the five departures

Smalltalk's arrangement, at the level this comparison needs:

```
Object                          the root; printing is protocol ON it
├── Boolean → True, False
├── Magnitude
│   ├── Number → Integer, Float, Fraction
│   ├── Character
│   └── Date, Time
└── Collection
    └── SequenceableCollection
        └── ArrayedCollection → Array, String
```

Bund departs in five ways. Each is a fact about the reference, not a complaint.

1. **The root is an output concern.** `Display` and `Printable` are the two
   most abstract classes, and `Object` descends from them
   (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:95-97`,
   `reference/Bund/src/stdlib/functions/oop/base_classes.rs:109-111`,
   `reference/Bund/src/stdlib/functions/oop/display_class.rs:101-103`). In Smalltalk
   `printOn:` and `displayString` are methods on `Object`. Consequence: in
   Bund a class cannot be non-printable, and the two names a reader expects to
   be mixins are instead ancestors.

2. **There is no metaclass layer.** Smalltalk gives every class an instance
   side and a class side, and `new` lives on the class side. Bund has no
   class-side anything: `List` is a plain inline native that happens to
   construct, registered beside `print` and `+`
   (`reference/Bund/src/stdlib/functions/oop/list_class.rs:96`). So "a class" and "the word
   that makes one" are unrelated objects, and nothing enumerates the classes.

3. **`Magnitude` and `Number` are missing, and `Bool` sits where `Number`
   would.** `Integer`, `Float` and `Bool` are siblings directly under `Value`
   (`reference/Bund/src/stdlib/functions/oop/int_class.rs:37`,
   `reference/Bund/src/stdlib/functions/oop/float_class.rs:35`,
   `reference/Bund/src/stdlib/functions/oop/bool_class.rs:35`).
   So the hierarchy does not distinguish "can be compared" from "can be
   arithmetic", and a boolean is a kind of value in exactly the way an integer
   is.

4. **`Collection` is missing.** `List` hangs directly off `Value`, and `Floats`
   and `Intervals` hang off `List`
   (`reference/Bund/src/stdlib/functions/oop/list_class.rs:63`,
   `reference/Bund/src/stdlib/functions/oop/floatlist_class.rs:74`,
   `reference/Bund/src/stdlib/functions/oop/intervals_class.rs:206`). `Floats` *is-a* `List` whose elements are
   coerced to FLOAT on construction and on `push` — in Smalltalk terms a
   constrained `ArrayedCollection`, which would not be a subclass of the
   unconstrained one.

5. **Construction is automatic and total, where Smalltalk's is explicit.**
   Bund runs every ancestor's `.init` during construction (§3 above;
   `reference/rust_multistackvm/src/stdlib/bund_object.rs:42-52`).
   Smalltalk's `initialize` is one method the author overrides and chains with
   an explicit `super initialize`. So a Bund leaf class cannot decline its
   parent's constructor, and the parent's constructor is where the operand goes.

## Design

### S1. The additive half: three leaf classes, and nothing moves

`List`, `Floats` and `Intervals` are added exactly as the reference declares
them — `List` under `Value`, `Floats` and `Intervals` under `List` — with
seven methods (`.list_init`, `.list_push`, `.floats_init`, `.floats_push`,
`.intervals_init`, `.intervals_push`, `.intervals_overlap`) and three
constructor words. No existing class changes, so no golden moves and no
decision is needed. This is the immediate work and it is not a proposal; it is
faithful reproduction.

Two things it does need:

- **`Intervals` carries a dependency.** It wraps `iset::IntervalMap<f64,
  Value>` (`reference/Bund/src/stdlib/functions/oop/intervals_class.rs:9-14`).
  The oracle's manifest asks for `iset = "0.3.1"`
  (`reference/Bund/Cargo.toml:103`), which is a caret requirement and not a
  pin; its lock file resolves **0.3.3**, and Bund2 pins `=0.3.3`, the version
  the oracle was built with. An earlier revision said "pinned at 0.3.1".
  `Floats` and `List` need nothing new.
- **Three message typos are preserved**, as the project's rule requires:
  `List: error converting to BOOL` in the *List* class
  (`reference/Bund/src/stdlib/functions/oop/list_class.rs:23`), `Flaots::push` in
  `Floats` (`reference/Bund/src/stdlib/functions/oop/floatlist_class.rs:53`), and
  `List: NO WRAPPED DATA WAS FOUND` plus
  `Stack is too shallow for method 'List::push'` inside `Floats`
  (`reference/Bund/src/stdlib/functions/oop/floatlist_class.rs:28`,
  `reference/Bund/src/stdlib/functions/oop/floatlist_class.rs:35`).

### S2. The rewiring half: what a Smalltalk shape would cost

Giving Bund `Magnitude`, `Number` and `Collection` in their Smalltalk
positions means **inserting** them into existing ancestry:

| change | what it moves |
|---|---|
| `Integer`/`Float` under a new `Number` under a new `Magnitude` under `Value` | nothing today — neither has a constructor word, so no program can build one |
| `Bool` out from under `Value` to a new `Boolean` | **`bool-objects.golden`** — the ancestry it captures gains a level |
| `List` under a new `Collection` under `Value` | **`bool-objects.golden` does not move**, but any future golden over a `List` object would differ from the oracle's |

So the cost is not uniform. The `Integer`/`Float` half is **invisible** because
those classes are unreachable from the language; the `Bool` half moves a
golden the moment it is made. That asymmetry is the useful result here, and it
was not predictable from the source — it follows from which classes have
constructor words (§2) and from what the probe happens to capture (§5).

**Recommendation: do not rewire.** Not because it is expensive, but because
the gain is a shape a reader recognises and the cost is that Bund2 and Bund
answer differently about ancestry — and ancestry is observable through `.str`,
`debug.dump` and the capture itself. D72 permits Bund2 to *add*; it does not
make a divergence in existing behaviour free.

### S3. What could be added without rewiring

If a Smalltalk-shaped hierarchy is wanted, the honest additive form is new
classes that **hang off existing ones** rather than above them:

- `Collection` as a new parent for *new* collection classes, declared as a
  sibling of `List` under `Value` rather than above `List`. Ugly, and honest:
  it says "this is Bund2's addition" in its position.
- `Magnitude` likewise beside `Value`'s numeric children.

Both are unsatisfying, and saying so is the point: **a hierarchy cannot be
retrofitted additively.** Either the existing edges change, or the new
abstractions sit in the wrong place. This is the trade the owner is being
asked to make, not a detail of it.

### S4. A membership test is the missing word, and it is additive

The gap a user is most likely to hit is §4: nothing answers "is this object a
`List`". That is **purely additive** — a new word reading the ancestry that
construction already materialises, moving no golden and changing no class.

Proposed, under D72: **`?is`** — takes an object and a class name, answers a
BOOL by walking `.super` with the same depth-first order
`locate_value_in_object` uses. Named `?is` rather than `is` because `is` is
taken by an unrelated word (§4), and prefixed `?` with the other predicates
(`?object`, `?class`, `?lambda`).

This is the one piece of the Smalltalk model that fits Bund without a
deviation, and it is recommended on its own merits whatever is decided about
S2.

### S5. Metaclasses are declined

Smalltalk's class side has no Bund analogue and adding one would mean a new
value tag, class-side dispatch, and a `new` protocol competing with `object`
and `!`. That is a language, not a hierarchy. Recorded as declined so a later
reader does not take its absence for an oversight.

## Preservation analysis

- **S1 deviates in nothing.** It reproduces three classes, seven methods and
  three words as the reference declares them, typos included.
- **S4 adds a word the reference lacks**, which is D72's precedent exactly:
  conformance counts goldens and no golden gains a program.
- **S2 deviates in observable behaviour** for `Bool` today and for `List` as
  soon as a golden covers it. It is the only part that would need an entry in
  `decisions.md` and a recorded golden regeneration.
- `is`, `wrap`, `unwrap`, `#`, `#.`, `True`, `False`, `?object` and `object`
  are untouched by every option here.

## What the owner decided — D96, 2026-10-05

1. **S2: additive only** — the hierarchy is not rewired. The five departures
   from Smalltalk stay recorded here rather than corrected in code.
2. **S4: `?is` accepted**, under D72. One correction to what this document
   claimed when it proposed it: §S4 said the word "moves no golden", which is
   true but understated — **no golden can ever cover it**, because a golden is
   captured from the oracle and the oracle has no such word. Its verification
   is a Rust test, and it joins the permanently uncovered set. That is D72's
   standing cost, true of `noop` as well, and this is the first time it has
   been written down.
3. **`iset` taken.** No transitive dependencies, and §S1's two ordering
   behaviours are now pinned by probe.

## Alternatives considered

- **Reproduce the reference's hierarchy and never discuss it.** Rejected: the
  `Printable`-above-`Object` inversion will be rediscovered by whoever reads
  `base_classes.rs` next, and the three missing classes force the question now.
- **Rewire fully to Smalltalk and record one deviation.** Rejected per S2: the
  cost lands on ancestry, which the capture reads.
- **Flatten the hierarchy** — one `Value` class with every method. Rejected: it
  would change dispatch results, and RFC-0009 §S1 already measured that the
  search is not the bottleneck.

## Acceptance criteria

1. `List`, `Floats` and `Intervals` are registered with the parents, methods
   and message texts the reference declares, and the three constructor words
   build objects the oracle agrees with — compared as values, since F74 does
   not apply here and object construction is deterministic.

   **Met, 2026-10-08.** `tests/probes/oop-collections.bund` and its golden,
   which `cargo xtask conform` passes;
   `the_collection_classes_keep_the_references_misnamed_messages` holds the
   three typos; criterion 4's test holds the parents and slots.
2. `bool-objects.golden` is **unchanged**, demonstrating S1 moved no ancestry.

   **Met, 2026-10-08.** The file's last change is `088fe0c`, 2026-10-04, the
   day before this document. `conform` passes it.
3. `?is` answers true for a class and for every ancestor of it, false
   otherwise, and a test pins all of `List`, `Value`, `Object`, `Printable`
   and `Display` against one `List` object.

   **Met, 2026-10-08**, by `is_a_walks_the_ancestry_up_and_not_down`. This
   criterion first asked for "a probe". D96 records why there cannot be one:
   a golden is captured from the oracle and the oracle has no such word. The
   wording is corrected to what D96 says the verification is.
4. This document's hierarchy diagram is checked against the registry by a test
   rather than by a reader, so it cannot drift from what is registered.

   **Met, 2026-10-08.** `the_registry_is_the_hierarchy_rfc_0010_draws` reads
   §1's diagram out of this file. It was run once against a copy of the
   diagram with `Floats` moved under `Value`, and failed on that row. Until
   that date this criterion had no test and the diagram had been checked by
   reading.
5. The five departures in "The Smalltalk model" each cite the file they were
   read from, and `cargo xtask cite` is clean.

   **Met, 2026-10-08.** Each departure now carries a `path:line`. Until that
   date the document named files and functions with no line anywhere, so
   `cite` was clean on it by having nothing to check, and this criterion
   could not fail.

## Open questions

- **Does anything want multiple inheritance?** `.super` is a LIST and the
  resolution order is already defined by `locate_value_in_object`, but no
  built-in uses more than one parent, so the order is untested. A probe could
  pin it cheaply; whether the language should encourage it is a separate
  question.
- **Should `Integer` and `Float` get constructor words?** They have classes and
  no way to build one. Adding them is additive, but it would make the
  `Magnitude` question observable where S2 says it currently is not.
