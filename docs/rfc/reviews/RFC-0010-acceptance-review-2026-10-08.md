# Acceptance review — RFC-0010: The class hierarchy

- Reviewed: `docs/rfc/RFC-0010-class-hierarchy.md` as of `9c33a0f` (305
  lines), read whole, as an adversarial reader. **First review of any kind**:
  `docs/rfc/reviews/` holds none for this document.
- Date: 2026-10-08.
- Reference SHAs: `git submodule status` matches `reference/PINNED.txt` in all
  six submodules; `git status --porcelain` is empty in each.
- Nothing in the RFC, the registers or ERRATA was edited.
- `cargo xtask conform`: **133/145, CEILING 133/145**, twelve approved
  deviations. `cargo xtask coverage`: IMPLEMENTED 500/505, COVERAGE 489/505.

**Verdict: not yet. The design is sound and every claim about the reference
that was checked holds. What is built matches what is written. Three things
block: a criterion with no test behind it, a criterion that cannot fail, and a
criteria list that records no evidence at all.**

## What was checked, and how

- **Every file in `reference/Bund/src/stdlib/functions/oop/` was grepped for
  its registrations, `.super` lists, method slots and messages**, and
  `locate_value_in_object` (`value_class.rs:10-29`) and
  `stdlib_object_value_is` (`value_class.rs:127-146`) were read whole.
- **Followed one call further**: `make_bund_object`
  (`reference/rust_multistackvm/src/stdlib/bund_object.rs:27-75`), for §3.
- **The diagram was compared with Bund2's registry by reading**
  `register_base` and `register_wrapped` in `crates/bund2-stdlib/src/oop.rs`.
- **Every test in `oop.rs`'s test module was listed by name**, looking for
  the one criterion 4 asks for.
- **`iset`** in the oracle's manifest and lock file, and in Bund2's.
- **`git log` on `tests/golden/probes/bool-objects.golden`**, for criterion 2.
- **Registers**: D96 read; D23, D25 and D72 by their heads.

## The diagram against the reference

Every edge and every method slot in §1's diagram matches the source:

| class | `.super` | slots | read at |
|---|---|---|---|
| `Display` | `[]` | `format`, `display` | `display_class.rs:101-105` |
| `Printable` | `[Display]` | `str`, `print`, `println` | `base_classes.rs:109-115` |
| `Object` | `[Printable]` | `.id`, `.timestamp` | `base_classes.rs:95-100` |
| `Value` | `[Object]` | `.init` | `value_class.rs:76-80` |
| `Integer` | `[Value]` | `.init` | `int_class.rs:35-39` |
| `Float` | `[Value]` | `.init` | `float_class.rs:35-39` |
| `Bool` | `[Value]` | `.init` | `bool_class.rs:35-39` |
| `List` | `[Value]` | `.init`, `push` | `list_class.rs:63-68` |
| `Floats` | `[List]` | `.init`, `push` | `floatlist_class.rs:72-77` |
| `Intervals` | `[List]` | `.init`, `push`, `overlap` | `intervals_class.rs:204-210` |

Bund2's `register_base` and `register_wrapped` set the same ten, slot for
slot. So the diagram is right today. Nothing holds it there — B1.

The three preserved typos are where §S1 says: `list_class.rs:23`,
`floatlist_class.rs:53`, `floatlist_class.rs:28` and `:35`. `is` behaves as
§4 says, and `IS: NO OBJECT IN #1` is at `value_class.rs:135`.

## Criteria

The document records no evidence for any of them. Found:

| # | asks for | found |
|---|---|---|
| 1 | the three classes registered as declared, and the constructors agreeing with the oracle as values | **Met.** `tests/probes/oop-collections.bund` and its golden; `conform` passes it. `the_collection_classes_keep_the_references_misnamed_messages` holds the typos. |
| 2 | `bool-objects.golden` unchanged | **Met.** Its last change is `088fe0c`, 2026-10-04, the day before this RFC. |
| 3 | `?is`, pinned by "a probe" | **Met by a unit test, not a probe** — `is_a_walks_the_ancestry_up_and_not_down`. D96 says no golden can ever hold the word; the criterion still says probe. |
| 4 | the diagram checked against the registry by a test | **Not met. No such test exists** — B1. |
| 5 | the five departures each cite their file, and `cite` is clean | **Vacuous** — B2. |

## Blocking

**B1 — Criterion 4 has no test.** It asks that "this document's hierarchy
diagram is checked against the registry by a test rather than by a reader, so
it cannot drift". The 23 tests in `oop.rs` were listed. The nearest are
`the_wrapped_value_classes_and_words_are_registered`, which asserts that four
classes exist and says nothing of their parents, and the `?is` test, which
walks ancestry for `List`, `Floats`, `Intervals` and `Bool` and never touches
`Integer`, `Float` or any method slot. This review checked the diagram by
reading, which is the thing the criterion exists to replace.

**B2 — Criterion 5 cannot fail, because the document has no citation `cite`
can check.** It contains no `reference/…:N` citation at all. Every claim
about the reference names a file and a function — "`base_classes.rs`,
`register_object`" — with no line. `cargo xtask cite` verifies lines, so it
is clean on this document by having nothing to read. CLAUDE.md requires a
`path:line` on every factual claim about existing behaviour. The table above
supplies the lines for §1; §2, §3 and §4 need theirs
(`list_class.rs:71-73` and `:96`; `bund_object.rs:27-75`;
`value_class.rs:127-146`).

**B3 — The criteria carry no evidence and the status line does not tally
them.** Every other RFC at this stage has a dated note under each criterion
naming the test or tool that decided it. This one has five bare criteria and
a status that says "§S1 and §S4 are built". Three of the five are in fact met,
and a reader cannot learn that from the document. Criterion 3's wording
should move from "a probe" to the test D96 says is its whole verification.

## Should fix

**S1 — "Only five of the ten classes have a constructor word" miscounts.**
Four classes have one — `Bool`, `List`, `Floats`, `Intervals` — through five
words, because `True` and `False` both build `Bool` (`bool_class.rs:73-74`).
The paragraph under the heading then lists the six classes without, which is
the right complement of four, not of five.

**S2 — "Bottom-up" in §3 reads backwards.** `make_bund_object` recurses into
a parent before it runs that parent's `.init`, so the root's initialiser runs
first and the leaf's last. The paragraph that follows describes exactly that
order — `Value`'s `.init`, then `List`'s own. The heading says the opposite
of the paragraph.

**S3 — `iset` is described as pinned at 0.3.1 and is not.** The oracle's
manifest asks for `"0.3.1"`, a caret requirement
(`reference/Bund/Cargo.toml:103`); its lock file resolves 0.3.3. Bund2 pins
`=0.3.3`, which is the version the oracle was built with. The code is right
and the sentence would send a reader to the wrong version.

**S4 — The motivation is in the present tense about work that is done.**
"Three words are unimplemented in `bund/oop`". All three are implemented and
run by a golden.

## What accepting would mean

- **§S2's rewiring is declined, not deferred** (D96). The five departures
  from Smalltalk stay as recorded facts about the reference.
- **`?is` is permanently outside every golden**, as D96 says. Its only check
  is one unit test.
- **Two open questions stay open** and neither adopts a default: whether
  anything wants multiple inheritance, whose resolution order no built-in
  exercises, and whether `Integer` and `Float` get constructor words.

## Summary

| | count |
|---|---|
| Blocking | 3 |
| Should fix | 4 |
| Criteria met | 2 of 5 as written, and a third by a test the criterion does not name |

B1 is one test. B2 and B3 are edits. This is the smallest distance to
acceptance of the five documents reviewed today.
