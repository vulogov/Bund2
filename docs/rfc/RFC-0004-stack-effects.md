# RFC-0004: Stack effects, inference, and `bund2 check`

- Status: **Accepted** (2026-09-07), **amended 2026-09-09**, **the
  amendment's figures corrected 2026-09-10** — §S1's closing sentence about
  how Tier 1 treats `Opaque` is withdrawn and replaced by RFC-0005 §S5; see the
  amendment and its correction at the end. **All seven criteria re-run at
  acceptance**, over the tool's own corpus roots
  (`xtask/src/corpus/mod.rs`) rather than a hand-written list — 160
  programs, 132 corpus plus 28 probes:

  | # | at acceptance |
  |---|---|
  | 1 | `conform` **72/85**, up from 71/84 when proposed. Movement is +1 and it is a *new golden*, not a changed one; no captured golden moved |
  | 2 | `effects`: 203 natives declare an effect, 128 agree, 3 recorded as arms, **0 disagree**, 72 not in the table and listed by name |
  | 3 | F18's fourteen: both per-word tests pass |
  | 4 | `check` over 160 programs: **1 finding**, in `reference/Bund/examples/distributed/hello_distributed_world_with_value.bund` — the same true positive, still at the call site |
  | 5 | the same run states **916 sites analysed, 127 abandoned** |
  | 6 | `infer`: **22 registered words — 21 agree, 1 opaque, 0 wrong** |
  | 7 | `cite` and `lint` both exit 0 |

  **§S1's second axis is still unbuilt, and acceptance does not pretend
  otherwise** — see the paragraph below, which is the gap this RFC is accepted
  *with*, not one acceptance closes. It is now the property of whoever extends
  `StackEffect`; F73 is fresh evidence that the workbench half of a word's
  contract is neither uniform nor inferable from its sibling.

  When proposed, **all seven acceptance criteria were met
  and each named the tool that decided it** — `cargo xtask effects` (2, 0
  disagreements), per-word tests for F18's fourteen (3), `bund2 check` over 157
  programs with one finding that is a true positive (4, 5), `bund2 infer` at
  21 agree / 1 opaque / 0 wrong (6), and `cite` and `lint` clean (7).

  **One design section is only partly built, and it is the one to read before
  accepting.** §S1 asks for an effect over *two axes*; what exists is one axis
  plus an `Opaque` flag. The flag earned its place immediately — nine
  control-flow words were declaring a fixed pair for an effect that is whatever
  the lambda they run leaves. The second axis did not get built, and its
  absence is not theoretical: `take` needs a value on the **workbench**, and
  recording that on the main-stack number made `check` report a program that
  runs. The current shape says "0 from the main stack" and stays silent about
  the rest, which is truthful but blind.

  So a workbench underflow is invisible to `check` rather than wrong in it.
  That is the gap this RFC is proposed with.
- Depends on: RFC-0001 (the value), RFC-0002 (`StackEffect` and the word
  table), RFC-0003 (the lowered stream and its spans, which are what an
  analysis walks)
- Decisions consumed: **D12** (the `*` fold family is a permanent optimization
  barrier), **D14** (core 286 / library 211, which is this RFC's denominator),
  **D24** (the `.` contract: primary operand from the workbench, secondary from
  the main stack, result to the workbench), **D36** (findings are reported
  through a `Reporter`, never printed), **D37** (no panic; an impossible state
  is explained)
- **Method note.** D24 declares `Blocks: nothing` and names no RFC, but it is
  about this RFC's subject — it defines the workbench half of a stack effect
  and was explicitly deferred until `cargo xtask arity` existed. It was found
  by grepping the register for *arity*, *effect* and *variadic* rather than for
  `Blocks: RFC-0004`, which is the search CLAUDE.md requires and which two
  earlier RFCs skipped.
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: `05-rfc-roadmap.md` §3's assessment that RFC-4 is "blocked on the
  largest single gap: stack effects for all 357 words". The gap is measured:
  `cargo xtask arity` probes 617 registered names and resolves 213.
- **Oracle caveat.** Every "confirmed against the oracle" claim inherits F21:
  the built oracle links crates.io releases, not the pinned submodules.
  `cargo xtask cite` compares the two `src/` trees byte for byte on every run
  and currently passes, so the claims hold conditionally on that check.

## Summary

A stack effect says how many values a word takes and leaves. Bund2 already has
a place to put one — `StackEffect { consumes, produces }` from RFC-0002 — and
this RFC fills it, widens it, and uses it.

Three things follow, and they are separable. **Natives are annotated**, from
the probed column of `docs/arity.md` rather than from the depth guard, because
for fourteen words the guard is smaller than what the word pulls (F18).
**Bund words are inferred**, by composing the effects of the words in a lambda
body. And **`bund2 check`** walks a lowered program with an abstract stack and
reports where it would underflow — a diagnostic, through D36's `Reporter`, not
a new failure mode.

The widening is the part an earlier plan missed. `{consumes, produces}` cannot
express two things the reference does: a word that folds the **whole stack**
(D12), and a `.`-suffixed word that takes one operand from the **workbench**
and another from the main stack (D24). An effect is a pair of deltas over two
stacks, with a floor rather than a count where the reference folds.

## Motivation

**Nothing in the reference declares an effect, and what it does declare is not
one.** A word opens with a depth guard — `if vm.stack.current_stack_len() < 2 {
bail!("Stack is too shallow for inline {}", &err_prefix); }`
(`reference/Bund/src/stdlib/functions/values/push.rs:12-16`). That is a
minimum, checked once, and nothing composes it or checks it against what the
word then pulls.

**For fourteen words it is smaller than what the word pulls.** `pair` guards
`< 1` (`reference/rust_multistackvm/src/stdlib/artefacts.rs:7`), pulls `x`
(`:10`), then pulls `y` (`:16`) and reports `PAIR returns NO DATA #2` (`:19`).
So `1 pair` fails with a message no other arity failure produces, and — because
the first pull already happened — with an **empty** stack rather than the one
value it was given. That is F18, whose disposition is FIX.

**A static arity that lies does not stay cosmetic.** This RFC infers effects
from the annotation and RFC-0005 orders JIT guards by it. F18's disposition
says so in as many words, and rejects preserving the guard for that reason.

**Underflow is currently a runtime surprise.** `1 pair` is a static error in any
language with effects, and here it is a message at the moment of failure with
the stack already half-consumed.

## Current behaviour

### 1. A guard is a minimum on one stack, written by hand

The shape is uniform enough to scan for and inconsistent enough not to trust.
`push_list_base` guards the main stack at 2 for its stack form and the
workbench at 1 *and* the main stack at 1 for its workbench form
(`push.rs:11-25`). `stdlib_math_op_multiple_inline` guards `< depth`, a
**parameter** (`reference/rust_multistackvm/src/stdlib/math/math_op.rs:25`), so
no static scan can read a number out of it at all — which is why the four folds
have a blank `declared` column in `docs/arity.md`.

### 2. The `.` form crosses two stacks

D24 states the contract; here it is at the source. In `push_list_base`'s
workbench arm, operand #1 comes from the workbench
(`push.rs:29`), operand #2 comes from the **main stack** with no `op` match at
all (`:38`), and the result goes back to the workbench (`:50`):

```rust reference/Bund/src/stdlib/functions/values/push.rs:38
    let op2_value = vm.stack.pull();
```

So `+++.` is not "`+++` but on the workbench". It is `-1` on the workbench,
`-1` on the main stack, `+1` on the workbench. A single `{consumes, produces}`
pair has nowhere to say that, and 262 words are in the `.` family.

### 3. Four words fold the whole stack

`stdlib_math_op_multiple_inline` loops: pull operand 1, pull operand 2, and if
the second is `NODATA`, push the first back and break
(`math_op.rs:38-52`). So it consumes the stack down to empty. D12 accepts this
as a permanent barrier on the evidence that the corpus never uses them.

Two details from reading it that matter here and are not in D12:

- **The guard is parameterised**, so these words declare nothing statically
  (`:25`).
- **An exhausted stack yields a `NODATA` *value*, not `None`** (`:45`). The
  loop tests `value2.type_of() == NODATA`, where `pair` tests `None` (`:17`).
  Both exhaustion signals exist, which is worth knowing before writing an
  analysis that assumes one.

### 4. `!` has eight arms and no effect

`execute` matches on the tag of the value it pulls:
`PTR | STRING | CALL` (`reference/rust_multistackvm/src/stdlib/execute.rs:27`),
`LIST` (`:37`), `MAP | INFO | CONFIG | ASSOCIATION` (`:55`), `CONDITIONAL`
(`:84`), `CLASS` (`:87`), `OBJECT` (`:90`), `LAMBDA` (`:93`), and a fallback.
The arms differ in effect: the MAP arm pulls a key as a second operand (`:59`),
the LAMBDA arm evaluates a body whose effect is whatever the body is, and the
OBJECT arm dispatches a named method.

This is not a corner. ERRATA records that `!` is one of the five most-used
words in the corpus — 69 invocations across 39 of 132 programs, 13 inside a
lambda body — and that a survey keyed on the registered name `execute` returns
zero.

### 5. There is no `?effect`, and no static analysis of any kind

A scan of every registered name across `reference/rust_multistackvm/src/stdlib/`
and `reference/Bund/src/stdlib/` finds no word containing `effect`, `arity` or
`check`. Whatever this RFC adds there is an **addition**, not a preservation.

### 6. What the harness knows today

`cargo xtask arity` exists and writes `docs/arity.md`. It reports two
independent columns: the declared guard, read statically, and consumed/produced,
observed by running each word against the oracle with sentinel operands.

**A correction to it was required before this RFC could rest on it, and the
correction is the reason to state the method rather than the number.** The probe
finds the smallest sentinel depth at which a word stops complaining, and
concluded the word "needs exactly `k` operands, and consumes them all". For a
fold that inference is invalid: it refuses shallower stacks too, so it reported
`**`, `*+`, `*-` and `*/` as consuming **2** — which is simply what a two-deep
sentinel stack holds.

The check that separates them is two extra operands: a fixed-arity word leaves
two more behind. Confirmed against the oracle at depth 4 — `+` and `++` leave
3, while `**`, `*+`, `*-` and `*/` leave 1.

**A second defect, found while re-running it.** The probe wrote every candidate
program to one fixed path, `target/arity/arity_probe.bund`, so two concurrent
runs fed each other's programs to the oracle. The failure is silent and the
output is not an error but a plausible wrong number: `!=` was reported as
`probed 0->3` — three values left by a word probed against an *empty* stack —
and the successfully-probed count moved by fourteen between runs of a tool that
had not changed. The path is now per-process.

Both defects share a shape worth naming, because this RFC's whole substrate is
a generated table: **the harness's wrong answers look like answers.** Neither
raised an error, and the fold one had been sitting in `docs/arity.md` since the
tool was written. That is the argument for criterion 2 cross-checking Bund2's
annotations against the table rather than generating them from it.

**With both fixed, the table reproduces, and it corroborates F18 independently.**
A clean run reports 617 registered names, 213 probed, 232 refused as unsafe to
run, 18 depth-insensitive, and **14** words whose declared guard disagrees with
what they consume. Those fourteen are `at`, `complex`, `head`, `pair`,
`string.distance` with its five variants, `string.regex`,
`string.regex.matches`, `string.wildcard` and `tail` — the same fourteen F18
lists, arrived at by a probe that does not read F18. That agreement is the
reason to trust the column this RFC annotates from.

**What the depth check proves is weaker than "variadic", and it is reported as
the weaker thing.** It flags a *depth-insensitive residual*, which is a superset: a
word that switches stacks reads its residual on a different stack, and a word
that empties the stack reads 0 either way. `ensure_stack` is the worked
counter-example — flagged, but 2 operands in leaves 2 and 6 leaves 6, verified
against the oracle. So the table writes `N+` and names the set; which cause
applies is a source question, and for D12's four the source answers it
(`math_op.rs:38-52`).

## Design

### S1. `StackEffect` becomes two axes and a floor

> **Partly built.** The `Opaque` arm exists — `StackEffect` carries an `opaque`
> flag, and `bund2 check` stops tracking at one — and it earned its place
> immediately: nine control-flow words had been declaring a fixed pair for an
> effect that is whatever the lambda they run leaves.
>
> **The second axis does not.** `take` needs a value on the *workbench* and
> there is nowhere to say so, which is not a theoretical gap: declaring the
> requirement on the main-stack number made `check` report a program that runs.
> The current shape can say "0 from the main stack" and no more, so a
> workbench requirement is invisible to the checker rather than wrong in it.
> That is the honest position until the type is widened, and it is why §S1
> asks for two axes rather than one plus a convention.


Today it is `{ consumes: u8, produces: u8 }`
(`crates/bund2-api/src/lib.rs`). That shape cannot express §2 or §3.
It becomes:

```
Effect {
    stack:     Delta,   // main stack
    workbench: Delta,   // the auxiliary stack
}

Delta = Fixed { consumes: u8, produces: u8 }
      | Fold  { floor: u8, produces: u8 }   // consumes to exhaustion
      | Opaque                              // not statically known
```

Three arms, each earning its place from §1-§4: `Fixed` is the ordinary case,
`Fold` is D12's four, `Opaque` is `!` and everything reachable through it.

**`Opaque` is not a failure of the design; it is the honest answer**, and having
a name for it is what stops the analysis being quietly wrong. RFC-0005 reads the
same enum: a `Fold` or an `Opaque` site bails to Tier 0, which is exactly D12's
accepted barrier.

### S2. Natives are annotated from the probed column, never the guard

F18's disposition, applied. A native's declared `Effect` is what
`docs/arity.md` observed, and where the declared guard disagrees the guard
loses. The consequence F18 already accepted: those fourteen words report
`Stack is too shallow for inline <word>()` where the reference reports
`NO DATA #2`, and they leave their operand in place where the reference had
already consumed it.

Bund2 declares 57 natives today, so this is a table to fill as words land, not
a migration.

### S3. Bund words are inferred by composition

A Bund word is a lambda body — a `Vec<BundValue>`. Its effect is the fold of
its terms' effects over an abstract stack: a literal is `+1`, a call is that
word's `Effect`, and the composite is the pair of running minima and final
deltas. Composition saturates: anything composed with `Opaque` is `Opaque`, and
anything composed with `Fold` is `Fold` from that point.

Inference runs on the **lowered stream with spans** RFC-0003 already produces
(`crates/bund2-syntax/src/lib.rs`), so a finding can name a line and
column without the IR §S5 defers.

### S4. `bund2 check` reports; it does not fail

A new subcommand walking a lowered program with the abstract stack of S3 and
emitting a `Diagnostic` per site that would underflow. Severity is `Warning`,
because D36 makes delivery proportional: the program has not run and nothing
has stopped, so it gets a line on stderr rather than a table and a stack dump.

It goes through `Vm::report`, so a TUI receives the findings as values. That is
D36's seam and this is its second consumer, which is the first evidence that the
seam is a seam rather than an indirection.

`check` **never changes what `script` does.** A program that `check` warns about
still runs, and still fails the way it fails today.

### S5. `?effect` answers the table at runtime

> **Built.** `:+ ?effect` answers
> `{ consumes=2 :: opaque=false :: produces=1 :: }`, `:! ?effect` answers
> `opaque=true`, and a name with no declared effect answers `nodata` rather
> than a zero pair — "takes nothing" and "cannot say" are different answers,
> and D16 makes the second common. It moved neither health number, exactly as
> predicted below.


A word pushing the effect of a named word, for the debugger's `words` view and
for programs that reflect. It is an **addition** (§5): no golden reaches it, so
it cannot move `conform`, and it is not among the 497 in-scope words, so it
cannot move `coverage` either. That is worth stating because a reader watching
the health metric would otherwise expect movement.

### S6. Where the analysis stops, and why that is stated rather than hidden

Four barriers, all grounded above:

1. **`!`** — eight arms, tag-dependent (§4). `Opaque`.
2. **`bund.eval`** — a program built at run time. ERRATA records that its
   existence is why closed-world analysis rarely fires.
3. **`@name`** — a stack switch changes which stack the deltas apply to. The
   analysis tracks the current stack name and abandons a region it cannot
   resolve statically.
4. **The `CF` conditional registry** — dispatch through a string lookup, which
   RFC-0003 §S7 already separates from the statically analysable `if`/`times`/
   `while`/`loop` family.

A checker that silently skips these would report "no problems" on the corpus's
most-used word. Each is a named `Opaque`, and `check` says how many sites it
could not analyse alongside how many it could.

## Preservation analysis

| Current behaviour | Disposition |
|---|---|
| §1 depth guards fire at their declared minimum | **Deviated for 14 words**, F18's FIX. The guard fires at the probed arity instead: different message, and the operand survives. |
| §2 the `.` contract crosses two stacks | **Preserved**, and now expressible. S1's two axes exist because of it. |
| §3 folds consume the whole stack | **Preserved.** `Fold` records it; D12's barrier stands and nothing restricts them. |
| §4 `!` dispatches on eight arms | **Preserved.** `Opaque` describes it; nothing about dispatch changes. |
| §5 no `?effect`, no static analysis | **Added**, not deviated. Neither `script` nor any word's behaviour changes. |
| Runtime underflow messages | **Preserved** except for F18's fourteen. `check` adds a warning beforehand; it does not replace the failure. |

The single deviation is F18's, already recorded and dispositioned FIX.

## Alternatives considered

**Declare the guard, not the probe.** F18's Option A. Rejected there, and the
reason is this RFC: the guard is already not the contract, and a static arity
that lies propagates into RFC-0005's guard ordering.

**Keep `{consumes, produces}` and treat the `.` family as opaque.** Cheap, and
it discards 262 words — the whole workbench half of the language — into the
arm that disables analysis. The two-axis shape costs one more field.

**Infer natives too, instead of annotating them.** The harness already infers
them, against the oracle, and writes `docs/arity.md`. But that is a build-time
artefact of a *different binary*; a native's effect in Bund2 is a property of
Bund2's implementation. Annotating and then cross-checking against the table
(criterion 3) keeps both, and catches the case where Bund2's word diverges from
the reference's.

**Make `check` fail the build.** Rejected: `!` is `Opaque` and one of the five
most-used words, so a false positive rate above zero is certain, and a checker
that blocks on it would be turned off. A warning that is right is worth more
than an error that is disabled.

## Acceptance criteria

1. **Conformance does not regress.** `cargo xtask conform` must not drop below
   its recorded baseline. This RFC changes no word's runtime behaviour except
   F18's fourteen, and **no golden reaches any of them** — F18 records that,
   and it is why the deviation is affordable. So the expected movement is
   exactly zero, and any movement is a bug.

2. **Every native Bund2 registers declares a non-default effect, and it agrees
   with the probed table.**

   **Met.** `cargo xtask effects` exists and reports 93 agree, 0 disagree, 4
   recorded as arms, 51 not in the table — the last listed by name rather than
   counted, as this criterion requires.

   It earned its place on the first run by finding **three defects nothing else
   could see**, in effects written by hand at the registration site where no
   test reads them:

   - **`?object` declared `1 -> 1`.** It peeks, so it leaves the receiver and
     the answer: `1 -> 2`. The peek had been fixed earlier the same day — it is
     what unblocked `create_object.bund` — and the declaration beside it was
     not updated.
   - **`swap` declared `1 -> 0`.** The stack comes back the size it went in:
     `10 20 30 1 swap` answers `10 20 1 30` in both engines. Now `2 -> 2`, the
     probed column, per F18's rule.
   - **`move` and `move_from` consumed their operand before rejecting it.** The
     arity was right; the *residual stack* was wrong. `1 1 move` fails in both
     engines, but the reference leaves both values and Bund2 left none. That is
     observable because the error path prints the stack, which is F18's own
     argument — and it was reached from an arity mismatch rather than from
     anything looking for it.

   **The three buckets are not decoration.** A disagreement is not always a
   Bund2 defect: the probe feeds sentinels of one type, so for a polymorphic
   word it measures one arm. `apply` reads `1 -> 1` because an INTEGER sentinel
   is pushed back unchanged; `graph!` reads `0 -> 1` because a non-LIST is
   pushed back and an empty graph built. Collapsing those into "disagree" would
   make the criterion unfixable, and collapsing them into "agree" would make it
   vacuous. They are recorded in `tests/golden/EFFECTS.txt` with a reason, the
   same shape as `DEVIATIONS.txt` and for the same reason.

   *(A fourth harness limitation, alongside §6's two: for `move` and
   `move_from` the probe measured a **failure path**, because both need a
   STRING name and the sentinel offered was an INTEGER. `docs/arity.md` marks
   such words `type-constrained`; these two slipped through because the
   rejection happened after the depth guard passed rather than at it.)* Decided by a new `cargo xtask effects`, which joins
   Bund2's registrations against `docs/arity.md` and reports three columns:
   agree, disagree, and not-in-table. The criterion is **zero disagreements**,
   with the not-in-table set listed rather than counted as agreement — a word
   the probe could not reach is not evidence of anything, and folding it into a
   pass is how criterion 1 of RFC-0009 nearly passed vacuously.

3. **F18's words fire at the probed arity.** Named from **F18's own list**,
   not from a count in the table: `at`, `complex`, `head`, `pair`,
   `string.distance` and its five algorithm variants, `string.regex`,
   `string.regex.matches`, `string.wildcard`, `tail`. A test per word: the
   declared effect is `2 -> 1`, and running the word one operand short reports
   `Stack is too shallow` with the operand still on the stack.

   The list is taken from the register rather than recomputed because the
   register's copy is fixed and a regenerated table's is not — §6's second
   defect is exactly a case where the table changed under a tool that had not
   changed. A criterion keyed to a moving number is not a criterion.

   **Met.** All fourteen are implemented, and
   `f18_words_guard_before_pulling` checks each: one operand short reports
   `Stack is too shallow` naming that word. `f18_words_leave_the_operand`
   checks the half a message alone would miss — the operand is **still there**,
   where the reference leaves an empty stack. That residual is what F18 calls
   the observable part, because the error path prints the stack.

   Ten of the fourteen were implemented for this criterion, and **three
   operand orders came out backwards first**, each caught by the oracle and
   none by reading:

   - `head`, `tail` and `at` take the **list on top**, count beneath. The count
     is pulled inside each arm, which reads like the first operand; the value
     is pulled before the match (`value_carcdr.rs:27-33`).
   - `string.regex` and `string.wildcard` take the **subject on top**, pattern
     beneath (`regex.rs:28,31`) — the opposite of `string.expressionmatch`,
     which I had just written. Two neighbouring words, same shape, opposite
     orders.
   - `complex` builds a **`CFLOAT`**, not a list of two floats, and printing
     one is refused: `Can not convert Value from 15`, exactly as a PAIR reports
     10. Pushing a plain LIST printed `[ 2.0 :: 1.0 :: ]` and looked right.

   `tests/probes/f18-arity-words.bund` captures the cases that *work*, so the
   orders cannot silently swap back. The deviation itself is pinned by test
   rather than by golden, because a golden of it would capture the reference's
   `~/.cargo` path (F66).

4. **`bund2 check` finds a planted underflow and does not find a false one.**
   Over the corpus: `check` reports zero findings on all 132 programs — they
   run — and reports exactly one on each of a small set of planted cases
   (`1 pair`, `+` on an empty stack, a lambda body that over-pulls). The
   negative half is the half that matters; a checker that fires on working
   programs is the failure mode this criterion exists to catch.

   **Met, with one planted case amended.** `bund2 check --file <path>` exists.
   Over all 157 programs — 132 corpus plus 25 probes — it reports **one**
   finding, and that one is a **true positive**:
   `hello_distributed_world_with_value.bund:9:3` pushes one atom and calls
   `swap`, which needs two. Confirmed by running the fragment against the
   oracle: `Stack is too shallow for inline swap()`. The program is not
   hermetic, so no golden covers it and nothing else was ever going to say so.

   All three planted cases are found: `1 pair`, `+` on an empty stack, and —
   **after criterion 6 landed** — a lambda body that over-pulls.

   The third was amended to "not met" while inference was missing, on the
   reasoning that a body's entry depth is unknown so an underflow *inside* it
   is unprovable. That reasoning was right and the conclusion was too weak:
   the finding belongs at the **call site**, not inside the body.
   `:OverPulls { + + } register  1 OverPulls` is reported, because inference
   gives the word a floor of 3 and the site can prove only 1. Nothing is
   claimed about where in the body it would fail, which is the part that
   genuinely is not provable.

   **Getting the negative half to zero cost three declaration fixes**, each of
   which was a real defect the checker surfaced:

   - **`dup_one` declared `0 -> 1`.** It peeks one and pushes one, so the net
     is right and the *floor* is wrong: it needs a value present. Now `1 -> 2`.
     `consumes` is a depth floor, which is F18's reading; a net of zero let a
     bare `dup` pass unremarked. Two siblings had the same shape.
   - **`take` declared `1 -> 1`** after I set it that way to record "not free".
     Its requirement is on the **workbench** — §S1's second axis — and putting
     it on the main-stack number made `check` report `test_times_loop.bund`, a
     program that runs. A number on the wrong axis is worse than no number,
     because the checker believes it. Back to `0 -> 1`.
   - **Nine control-flow words declared a fixed pair.** `if`, `if.false`,
     `ifthenelse`, `while`, `times`, `loop`, `map`, `?.` and `?move` all
     *evaluate a lambda*, so what they leave is the body's business, not
     theirs. They are `Opaque` with their floors kept. `ifthenelse` declaring
     `3 -> 0` made `check` report
     `42 42 != { true } { false } ifthenelse println`, which runs fine.

   That is 12 of the 157 programs going quiet for the right reason rather than
   by loosening the check.

5. **`check` names how much it could not analyse.** Its output states the count
   of `Opaque` sites alongside the count analysed. A run that says "no problems"
   without saying it skipped every `!` is misleading, and §S6 exists so that it
   cannot.

   **Met.** Every run ends with `N finding(s), M site(s) analysed, K
   abandoned`, and when anything was abandoned it lists why, most common first
   — `a stack switch`, `a word with no declared effect`, or the word itself for
   an `Opaque` one. Across the 157 programs: **770 sites analysed, 128
   abandoned.** The line "A finding is only ever about the sites above that
   were tracked. Nothing is claimed about the rest." is printed with the
   counts, so the caveat travels with the number rather than living in this
   document.

6. **The effect of a Bund word is inferred and matches a run.** For each corpus
   program's registered words, the inferred effect equals the depth delta
   observed by running the word against an instrumented stack, or is `Opaque`.
   The disjunction is the point: `Opaque` is an allowed answer and a *wrong
   number* is not.

   **Met.** `bund2 infer --file <path>` runs a program so its `register` calls
   happen, then for every word it registered compares the inferred effect
   against the depth delta of actually calling it. Across the corpus and the
   probes: **22 registered words — 21 agree, 1 opaque, 0 wrong.**

   The one opaque is right: its body reaches a word whose own effect is not a
   fixed pair, and composition saturates.

   **`consumes` is the dip, not the term count.** `{ + + }` needs **three**,
   not four: the first addition leaves a value the second consumes. The
   abstract machine runs relative to an unknown entry depth and lets the
   simulated depth go negative; how far it goes *is* what the body needs.
   Pinned by `composition_counts_the_dip_not_the_terms`.

   **Unobservable is its own bucket.** A word that fails on integer sentinels —
   it wanted a string, a list, a lambda — is reported as unobservable rather
   than counted as agreeing. An unrun word is evidence of nothing, and folding
   it into a pass is how `xtask effects`'s "not in the table" bucket would have
   gone vacuous.

   Inference also feeds `bund2 check`, which is where it pays: sites analysed
   went from 770 to **799** with no new findings, because a call to a program's
   own word no longer ends tracking.

7. **`cite` and `lint` clean**, with the load-bearing citations quoted as
   fenced blocks so `cite` verifies content rather than line numbers. The four
   this design rests on:

   ```rust reference/Bund/src/stdlib/functions/values/push.rs:38
    let op2_value = vm.stack.pull();
   ```

   ```rust reference/rust_multistackvm/src/stdlib/artefacts.rs:7
    if vm.stack.current_stack_len() < 1 {
   ```

   ```rust reference/rust_multistackvm/src/stdlib/math/math_op.rs:25
            if vm.stack.current_stack_len() < depth {
   ```

   ```rust reference/rust_multistackvm/src/stdlib/execute.rs:93
                LAMBDA => {
   ```

## Open questions

- **Q23 — answered: yes, for the literal form.** `check` pre-binds every
  `:Name { … } register` it finds in the stream — three adjacent values —
  before walking. Without it the checker abandoned at every word a program
  defines, which is most of what a real program calls, and criterion 4's third
  planted case was unreachable.

  It handles **only** the literal form. `<computed> <lambda> register` stays
  unknown, as does a body built at run time: that is D16's open world, and
  recognising a shape that is actually present is not the same as assuming one
  that might be.
- **Q24 — the `.` gap-filling that D24 authorises is now unblocked** but not
  done. D24 says no `.` forms are to be added by hand before the arity table
  exists; it exists. Partitioning the 262 is work this RFC enables and does not
  perform, and it needs the table read by a human first.
- **F40 is adjacent and undispositioned.** Guard messages naming a neighbouring
  word (`docs/registers/defects.md`) is about the same guards this RFC reads,
  but it concerns the message rather than the number, so nothing here depends
  on its disposition.
- **The depth-insensitive set has 18 members and 4 confirmed causes.** §6 lists
  what the flag proves. Classifying the other 14 by source is work; until then
  they are `Opaque` rather than `Fold`, which is the safe direction.

---

# Amendment, 2026-09-09 — §S1's forward claim about Tier 1 is superseded

§S1 closes its description of the `Effect` enum with a sentence about a
downstream RFC:

> RFC-0005 reads the same enum: a `Fold` or an `Opaque` site bails to Tier 0,
> which is exactly D12's accepted barrier.

**That sentence is withdrawn.** It was written before RFC-0005 existed, in a
section whose subject is the enum rather than code generation, and it does not
survive contact with what `Opaque` means to a compiler.

## Why it does not hold

It reads one of two ways, and neither works:

- **Run-time bail** — compiled code hands control back mid-body. That is the
  on-stack-replacement machinery `docs/research/00-jit-feasibility.md` §3.2b
  says Cranelift does not have. RFC-0005 §S5 already refuses it for type
  guards, on exactly these grounds; it cannot then require it for effects.
- **Whole-body exclusion** — a body containing an `Opaque` site is never
  promoted. Coherent, and **measured to be ruinous**. `cargo xtask check` over
  the 160 corpus programs and probes abandons at 55 opaque sites, and 18 of
  them are ordinary control flow: `times` 7, `if` 6, `loop` 5. **`if` is
  `Opaque`**, because it runs a lambda. 81 of the 132 corpus programs contain a
  control-flow word or `!`. Excluding every body containing an `if` leaves
  almost nothing worth compiling, and RFC-0005 would specify a tier that
  practically never fires.

There is also a category error underneath it, which RFC-0005 §S5 now states
outright: **`Opaque` is not a type question.** A type guard has a generic
counterpart — the boxed arithmetic the interpreter would have run — and can
branch to it. Effect opacity has none: once the stack depth is unknown it stays
unknown, and there is no path that recovers it.

## What replaces it

**RFC-0005 §S5 owns the question**, and answers it: promotion *stops* at an
opaque site. Values held in Cranelift `Variable`s are synced back to the real
stack before the opaque call, and everything after it runs through runtime
helpers. Control never leaves compiled code, so no OSR is needed, and the
straight-line region *before* the site still gets promoted — which is where
§S6's win actually lives.

**D12 is untouched.** `Fold` remains a barrier that cannot be optimised across.
What changes is that a barrier no longer kills the body it appears in.

## What this amendment is not

It is **not a deviation**. Deviations record departures from the reference's
behaviour, and nothing here changes what any program does — the health metric
must still move by exactly zero. It is one RFC correcting another's
forward-looking guess about its own subject, which is why it needs no D-number.

Nothing else in this RFC depends on the withdrawn sentence: the enum, the
inference rule, `bund2 check` and all seven acceptance criteria stand as
accepted.

- Amended by: repository owner, 2026-09-09, on an adversarial review of
  RFC-0005 that found the contradiction

## Correction, 2026-09-10 — the amendment's figures

The amendment above is **left as written and its decision stands**: §S1's
closing sentence remains withdrawn, and RFC-0005 §S5 owns what Tier 1 does at
an `Opaque` site. What it got wrong is the evidence it quoted. RFC-0005's fifth
review (B3) found that the measurement it cited had been deleted from RFC-0005,
and that the figures here disagreed with both RFC-0005 and the tool. Each is
re-derived below with the command RFC-0005 §S6, *Where promotion stops*,
records beside its table:

| the amendment said | now |
|---|---|
| "**measured to be ruinous**" | **counted, not measured.** No benchmark stands behind "ruinous". What the count shows is that whole-body exclusion would refuse every body that branches or loops, which is how RFC-0005 §S5 now puts it |
| "`cargo xtask check`" | **no such subcommand.** The tool is `bund2 check` |
| "the 160 corpus programs and probes" | **161 today**: the 132 corpus programs, plus probes, which grew by one |
| "abandons at 55 opaque sites" | **113 sites** where analysis stops — **63** whose word's effect is not a fixed pair, and **50** naming words Bund2 does not register at all |
| "18 of them are ordinary control flow: `times` 7, `if` 6, `loop` 5" | **19 of the 63**: `times` 7, `if` **7**, `loop` 5. `if` gained a site as words landed |
| "81 of the 132 corpus programs contain a control-flow word or `!`" | **not re-derived**, because it measures something different. `bund2 check` reports where analysis of a body *stops*, so a program can contain more control-flow words than it reports. On that measure, analysis stops on control flow or `!` in **48** of the 132, and on some opaque site in **54** |
| "the straight-line region *before* the site still gets promoted — which is where §S6's win actually lives" | **true of promotion, not of the tier.** RFC-0005 §S6's 2026-09-10 measurement makes promotion the larger multiplier — 4.3–4.4× over an inlining ceiling of 2.0–3.0× — and promotion does live only in such regions. But inlining continues past an opaque site behind a per-site meaning guard, so the region before the site is not where all of the tier's win lives |

Why the figures moved: words were implemented and effects declared between
the two counts, and the amendment carried its numbers without the command that
produced them. RFC-0005 now keeps the command beside the table so a count can
be re-run rather than trusted.

**Still not a deviation**, for the reason the amendment gives: nothing a
program does changes.

- Corrected by: repository owner, 2026-09-10, on RFC-0005's fifth review
  (`docs/rfc/reviews/RFC-0005-review-2026-09-10.md`, B3)
