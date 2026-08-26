# Decision register

Append-only. Add entries, change `status`, never delete or renumber.

Status: OPEN | RESOLVED | SUPERSEDED. A default is for planning only — an RFC
or work item may never adopt one silently.

---

## D1 — `.id` format contract
Does `.id`'s exact nanoid format matter to existing programs, or is the
contract "unique opaque string"?
- Blocks: RFC-0001
- Default: lazy nanoid derived from counter plus VM seed (preserves format)
- Evidence: `cargo xtask corpus`
- Status: **RESOLVED — lazy.** Decided by the repository owner. This matches
  the recorded default, now adopted explicitly rather than by omission.

Corpus evidence was empty: `.id` has zero uses across all 132 programs. But
`id` is not merely a label a program may print, and laziness is constrained by
three internal readers:

- **Equality.** For every non-scalar type, `==` *is* id comparison —
  `reference/rust_dynamic/src/eq.rs:53`, with the same fallback for mismatched
  scalar pairs at `:15,26,34,42`. Two structurally identical lists are unequal
  unless they share an id.
- **Ordering.** `reference/rust_dynamic/src/ord.rs:175,183,191,199` fall back
  to `self.id.cmp(&other.id)`. See F12 — that path is inconsistent with
  `PartialOrd` and currently unreachable, but it exists.
- **Hashing.** `reference/rust_dynamic/src/hash.rs:6` hashes the id and
  nothing else.

**Laziness is the deferred-generation form**: nothing is computed at
construction, and an identity is minted on first *need* — where "need" is any
of `.id`, equality, ordering, hashing, or serialisation. Chosen by the owner
over the capture-a-token-at-construction alternative.

The hazard RFC-0001 must solve. Today `Clone` is derived, so a clone carries
the same id and `A == A.clone()` is **true** for non-scalars via
`reference/rust_dynamic/src/eq.rs:53`. Under naive deferred generation, an
unobserved value and its clone each hold an empty slot, materialise
independently, and compare **unequal** — a silent behaviour change on the most
ordinary operation there is. The lazy slot therefore has to be *shared across
clones* and reset only at the sites that mint fresh identity today:
`reference/rust_dynamic/src/dup.rs:11`, `attr.rs:7`, `push.rs:165`,
`set.rs:16,31,43,58,76,91`, `bincode.rs:95`, `id.rs:6`. Clone-equal versus
dup-unequal is the contract to preserve.

If that cannot be made cheaper than a construction-time counter token, the
counter is the fallback — it preserves everything exactly at one atomic
increment. RFC-0001 picks the mechanism and must show which it chose against
this hazard.

Secondary consequence: if identity ends up counter-derived, the `Ord::cmp`
fallback changes from "lexicographic on a random nanoid" to "creation order".
Low risk given F12, but it is a behaviour change on a path the reference has.

**Where identity lives, settled by scan** (see "The `id` / `stamp` layout
scan" in `open-questions.md`): the heap header, not the inline value. The
`==` fallback to `id` fires only when both operands are non-scalar — already
heap-allocated, so the header read is free — or when their kinds differ, in
which case the answer is provably always `false` and needs no identity at
all. Scalars therefore carry no identity, and `BundValue` is 16 bytes rather
than 24.

## D2 — `.timestamp` precision
Is millisecond granularity the contract, or must two values constructed in
sequence differ?
- Blocks: RFC-0001
- Default: sampled clock at millisecond granularity
- Evidence: `cargo xtask corpus`
- Status: **RESOLVED — lazy.** Decided by the repository owner. Note this
  *departs* from the recorded default, which sampled the clock eagerly.

Corpus evidence was empty: `.timestamp`, `time.timestamp` and `time.now` all
have zero uses. Two constraints survive that emptiness:

- **`stamp` is creation time, and is read outside `.timestamp`.** Iterating a
  METRICS value materialises it into a `"ts"` field —
  `reference/rust_dynamic/src/iter.rs:67,82` and
  `reference/rust_dynamic/src/carcdr.rs:127,203`. So a lazy stamp must still
  answer "when was this value constructed", which rules out sampling the clock
  at observation time: that would silently redefine `.timestamp` as
  observation time. (Neither path is corpus-reachable — `metrics` and `sample`
  are unused — but both are live code.)
- **`stamp` is `f64` milliseconds**, from
  `SystemTime::now().duration_since(UNIX_EPOCH).as_millis() as f64` —
  `reference/rust_dynamic/src/value.rs:7-9`, assigned at
  `reference/rust_dynamic/src/value.rs:36`. `set` and `push` refresh it
  (`set.rs:32,59,92`, `push.rs:166`).

**The clock is not read at construction at all.** The stamp is sampled when it
is first observed. Chosen by the owner, over the alternative of a cheapened
construction-time capture, after the consequence below was put to them.

This is a **deviation from 100% preservation**, and it must be listed in the
Deviate section of whichever work item implements `Value`:

- `.timestamp` stops meaning "when this value was constructed" and starts
  meaning "when this value was first asked". Two values constructed together
  and observed apart report different stamps; two constructed apart and
  observed together report the same one.
- The `"ts"` field that metric iteration materialises
  (`reference/rust_dynamic/src/iter.rs:67,82`,
  `reference/rust_dynamic/src/carcdr.rs:127,203`) becomes iteration time.
- `get_timestamp_diff` (`reference/rust_dynamic/src/timestamp.rs:22`) becomes
  a difference of observation times rather than of construction times.

Why it is nevertheless safe against the goldens: nothing in the corpus can see
it. `.timestamp`, `time.timestamp` and `time.now` have zero uses, and the
metric-iteration path needs `metrics` or `sample`, both also unused. No golden
changes. The deviation is real but currently unobservable — which is exactly
why it is recorded here rather than discovered later.

`stamp` stays `f64` milliseconds (`reference/rust_dynamic/src/value.rs:7-9`).
`set` and `push` continue to reset it (`set.rs:32,59,92`, `push.rs:166`),
which under deferred sampling means resetting it back to unobserved.

**Where the stamp lives, settled by scan** (see "The `id` / `stamp` layout
scan" in `open-questions.md`): the heap header. Every read is cold — the
`.timestamp` method, four METRICS-iteration sites, and two public functions
(`get_timestamp`, `timestamp_diff`) that **no word in either crate reaches**.
Nothing on a hot path touches it, so the move costs nothing.

## D3 — tier policy for `bund.eval` output
JIT-eligible, or permanently Tier 0? Permanently Tier 0 removes a whole class
of unbounded code-memory growth.
- Blocks: RFC-0003, RFC-0005
- Default: permanently Tier 0
- Status: OPEN

## D4 — integer width
Is full `i64` required, or are 51-bit integers acceptable? The latter allows
NaN-boxing and a smaller value.
- Blocks: RFC-0001
- Default: full `i64`
- Evidence: `cargo xtask layout`
- Status: OPEN

Measurement narrows what NaN-boxing is worth. With identity in the heap
header (D1, D2, and the layout scan), the candidate value is **already 16
bytes at full `i64`** — one tag word plus one payload word. NaN-boxing would
fold the tag into unused float bits to reach 8, so the question is whether
halving 16 justifies capping integers at 51 bits.

For contrast the same measurement puts the reference's `Value` replica at
**176 bytes**, and identity carried inline at 32. The large win is already
taken by moving identity off the value; NaN-boxing is a second, smaller step
with a semantic cost attached.

## D5 — lambda body mutability
Can a LAMBDA body be mutated after construction, or is it write-once? Write-once
lets the compiled cache skip invalidation.
- Blocks: RFC-0003
- Default: assume mutable; invalidate on write
- Evidence: `cargo xtask corpus`
- Status: **RESOLVED — write-once.** Note this *departs* from the recorded
  default, which assumed mutability and paid for invalidation.

A LAMBDA body is never written through. Two paths could plausibly do it and
neither does:

- `set` applied to a LAMBDA **replaces** the body and returns a *new* value:
  `LAMBDA => { return Value::to_lambda(vec![value]); }`
  (`reference/rust_dynamic/src/set.rs:11-13`). The original is untouched.
- `push` cannot reach a LAMBDA at all: it converts its receiver with
  `conv(LIST)` before appending
  (`reference/Bund/src/stdlib/functions/values/push.rs:34`), so the result is
  a LIST, not a mutated LAMBDA.

The corpus agrees: zero post-construction mutations across 132 programs. The
45 sites where a mutator follows a closing `}` are all the same shape —
`:.init { ... } set` — where `set` pulls stored-value, key, receiver in that
order (`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:10-27`),
so the lambda is the value being filed into a class, not the receiver.

**Consequence for RFC-0003: the compiled cache needs no invalidation
machinery.** Every mutating path returns a new value with a regenerated
identity (`reference/rust_dynamic/src/set.rs:16,31,43,58,76,91`,
`push.rs:165`, `attr.rs:7,13,19`), so a cache keyed on identity simply does
not contain the replacement. The default would have built a guard against
something the value model makes impossible.

## D6 — async granularity
Is fine-grained suspension inside a word required, or is VM-per-task enough?
- Blocks: RFC-0007
- Default: VM-per-task
- Status: OPEN

## D7 — concurrent VM count
Tens (actor model is fine) or thousands (per-VM word tables become the memory
story)?
- Blocks: RFC-0007
- Default: tens
- Status: OPEN

## D8 — existing external word packages
Do any Rust word packages exist outside this repository? If so, `bund2-api` is
a migration rather than a clean design.
- Blocks: RFC-0002
- Default: none; design freely
- Status: **RESOLVED — none. `bund2-api` is a clean design.** Extension has
  never been out-of-tree Rust; it is Bund-source-level, by two mechanisms.

**`use <uri>`** fetches Bund *source* and compiles and evaluates it
(`reference/Bund/src/stdlib/functions/bund/bund_use.rs:31-33`). Note the
transport: `get_file_from_uri` is curl, not the filesystem
(`reference/Bund/src/stdlib/helpers/file_helper.rs:42-54`), so `use` is a
network word. The effect classification in `xtask` had it as filesystem and
has been corrected.

**The world file**, a SQLite database holding `LAMBDAS`, `ALIASES`, `STACKS`,
`STACK_DATA`, `MODELS` and `BOOTSTRAP`
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:69`, `aliases.rs:60`,
`stacks.rs:129,187`, `models.rs:11,79`, `bootstrap.rs:179`), written by
`save.*` and read by `load.*`/`bootstrap`. Whole `Value`s go in as bincode
BLOBs (`reference/Bund/src/stdlib/helpers/world/lambdas.rs:81-84`).

So `bund2-api` designs freely — there is no Rust package ecosystem to
migrate. What Bund2 must keep working is the two *artifact* paths: source
fetched over a URI, and the world file.

**Backend for the world file: SQLite or redb, owner's option.** redb is a
pure-Rust embedded store and drops the C dependency, which also bears on D10.
Either is acceptable *provided nothing outside Bund2 reads the world file* —
and that is exactly D11, still OPEN. If an external reader exists, changing
the backend is a breaking format change; if not, it is free. Decide D11 before
committing to redb.

- Read against the pinned submodule, not GitHub `main`. `reference/Bund` is
  pinned (`reference/PINNED.txt`) and upstream may have moved since; per
  CLAUDE.md the pinned copy is what citations resolve against.

## D9 — third-party CLIF lowerings
Should `Intrinsic` lowerings ever be exposed through `bund2-api`? Doing so pins
external packages to an exact Cranelift version.
- Blocks: RFC-0002
- Default: no
- Status: OPEN

## D10 — C toolchain requirement
May `bund2 build` require `cc`, or must the compiler be self-contained?
- Blocks: RFC-0006
- Default: yes, `cc`; `--emit=bundle` covers toolchain-free targets
- Status: OPEN

## D11 — external dependents of `compile_to_binary`
Does anything outside the project depend on the current bincode object format?
- Blocks: RFC-0003
- Default: no; version the IR format freshly
- Status: OPEN

## D12 — the `*` fold-family
Restrict the whole-stack variadic words in JIT-able positions, or accept them
as a permanent optimization barrier?
- Blocks: RFC-0004
- Default: accept as barrier
- Evidence: `cargo xtask corpus`
- Status: **RESOLVED — accept as a permanent barrier.** This adopts the
  recorded default, now explicitly rather than by omission.

The barrier is real. `stdlib_math_op_multiple_inline`
(`reference/rust_multistackvm/src/stdlib/math/math_op.rs:22`) loops pulling
operands until the stack yields NODATA (`:38-52`), so arity is not statically
known at these sites and a JIT cannot fix a frame shape for them.

But it costs nothing measurable, because **the corpus never uses them**:
`*+`, `*+.`, `*-`, `*-.`, `**`, `**.`, `*/`, `*/.`, `*loop`, `*loop.` are all
zero, as are the Unicode aliases `Σ` and `Σ.`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:37,38`). The three
`*`-suffixed words the corpus does use are not folds: `generator.sample*`
takes an explicit count immediately before it, `lambda*` folds the stack into
a LAMBDA, and `input*` is a read loop.

Restricting them in JIT-able positions was rejected: it would be a
preservation risk on words nothing calls, in exchange for no measured gain.
A site using a fold bails to Tier 0; nothing else changes.

Note the naming does not follow the sigil — `*` alone is ordinary
multiplication (`reference/rust_multistackvm/src/stdlib/math/mul.rs:23`) and
`**` is the variadic one. RFC-0004 should key on the registration, not the
name.

## D13 — value semantics under Rc
Today `Value` is deep-cloned everywhere, so Bund has value semantics and cycles
are impossible by construction. Naive `Rc` would give reference semantics and
make cycles constructible. Confirm `Rc::make_mut` (clone-on-write) as the
correct preservation.
- Blocks: RFC-0001
- Default: yes, `Rc::make_mut`
- Status: **RESOLVED — yes, `Rc::make_mut`.** This adopts the recorded default,
  now explicitly, and the fit is closer than the entry suggested.

Every mutating operation already returns a **new value with a fresh
identity** — `reference/rust_dynamic/src/set.rs:16,31,43,58,76,91`,
`push.rs:165`, `attr.rs:7,13,19`. `set` even takes `&mut self` yet returns
`Self` (`reference/rust_dynamic/src/set.rs:6`), which is a copy-returning API
wearing a mutable signature.

So clone-on-write does not approximate the semantics — **its split points
coincide exactly with the points where the reference regenerates the id.**
Both properties the entry worried about are preserved:

- **Cycles stay impossible.** Constructing one needs a value to contain a
  reference to itself, but every mutation yields a fresh snapshot, so a
  contained copy is a snapshot and never a back-reference.
- **Clone-equality holds.** `Clone` copies the id and `==` compares ids for
  every non-scalar type (`reference/rust_dynamic/src/eq.rs:53`), so
  `A == A.clone()` is true before and after.

Naive `Rc` is disqualified outright: reference semantics would make cycles
constructible and change what `==` means.

**Constraint carried from D1, which this entry did not previously record:**
the lazy identity slot must be shared across clones and must split at the same
points the `Rc` does. If the two split policies disagree, an unobserved value
and its clone materialise different ids and `A == A.clone()` silently becomes
false. RFC-0001 must show one policy governing both.

## D14 — library scope
Which of the 357 words are language core (100% preservation) and which are
library (deferrable, re-implementable as out-of-tree word packages)?
Preservation applies to Bund syntax and logic, not to the domain libraries.
- Blocks: RFC-0002 (bund2-api shape), RFC-0004, the M6 target and denominator
- Default: none — decide from corpus evidence
- Evidence: cargo xtask corpus
- Status: OPEN — being resolved **per word**, not per subsystem. D17
  (`format`) and D19 (`display`) are the rulings so far; D18 attaches the
  workbench form to each.
- Method, settled under Q4: subsystem grouping is a reporting aid only. Each
  per-word ruling must state its **implementation closure** — what that word's
  implementation reaches into — which `cargo xtask corpus` now reports. D19 is
  the reason: preserving `display` also preserves `conditional_fmt`, and that
  was found by reading the file rather than by the evidence. Of the 91 files
  providing corpus-used words, 81 are self-contained and 10 cross a subsystem
  boundary; the `bund/forecast` family reaching `bund/statistics` is the
  largest such commitment.

## D15 — console presentation scope
Only basic console output is in scope: `print`, `println`, `nl`, `space` and
their workbench forms (`reference/rust_multistackvm/src/stdlib/print.rs:63-68`).
No spinners, no animations, no colour.

This defers the whole `bund/console` subsystem
(`reference/Bund/src/stdlib/functions/console`, 31 words). Every word there is
presentation: `console.spinner*` and `spinner.text*` drive a `spinoff` spinner
(`console/spinner.rs:11`), `console.text*` emit `rusty_termcolor` colour
(`console/spinner.rs:12`), and `console.typewriter` is a timed character
animation (`console/terminal.rs:35`). `console.clear`, `console.title` and
`console.box` are terminal control.

Corpus cost: 5 programs, 22 distinct words —
`console/spinner_demo`, `console/text_color_demo`, `console/typewriter_demo`,
`ai/ollama_api_demo`, `code_snippets/string_wrap_demo`. They leave the
conformance suite; `cargo xtask corpus` names them rather than dropping them
silently.

This settles one subsystem. It does **not** resolve D14: the partition for
every other subsystem is still open.

- Decided by: repository owner, answering open question Q6
- Blocks: nothing; unblocks the golden capture for the console examples
- Status: RESOLVED
- Scope boundary (see also D16 below): `display`
  (`reference/Bund/src/stdlib/functions/system/display.rs:88`) is **in scope**,
  decided by the owner answering Q7. It renders markdown through
  `termimad::print_text` (`system/display.rs:11`), which emits ANSI styling,
  but it is not *chosen* colour the way `console.text.red` is — the styling is
  incidental to rendering, and 12 programs depend on the word. D15 defers the
  `bund/console` subsystem only.

## D16 — dynamic dispatch by computed name
`<string> ptr !` and `` `<name> ! `` are preserved exactly as they behave
today. **The world is permanently open**: a call target may be named by a
string that exists only at run time.

Mechanism, so the contract is unambiguous. `ptr` pulls a value, casts it to a
string, and pushes a PTR carrying that name
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:80-93`; applying a PTR
falls to the push arm at
`reference/rust_multistackvm/src/multistackvm_apply.rs:88-99`). `` ` `` is the
lexical spelling of the same thing (`reference/bund_language_parser/bund.pest:29`).
`!` is an alias of `execute`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:5`), which for
`PTR | STRING | CALL` hands the name to `vm.call(...)`
(`reference/rust_multistackvm/src/stdlib/execute.rs:26-33`).

What this forecloses:

1. **No AOT tree-shaking by word reachability** (RFC-0006). Any registered
   word may be the target of a name assembled at run time, so none can be
   proven dead. An AOT image retains the word table and the name resolver.
2. **`!` is not statically devirtualisable** (RFC-0005). Speculation behind a
   guard with a full-resolution fallback is permitted — that is speed, not
   meaning — but the health metric must still move by exactly zero.
3. **The resolution chain is observable in full and its order is contract**:
   command, then `$`-forced internal, then alias, then lambda, then inline
   (`reference/rust_multistackvm/src/multistackvm_apply.rs:16-60`), including
   the fall-through from the VM inline table to the stack layer's
   (`reference/rust_multistackvm/src/multistackvm_inline.rs:42,52`).
4. **`execute`'s other input types are part of the same contract**: a bare
   `STRING` needs no `ptr` at all
   (`reference/rust_multistackvm/src/stdlib/execute.rs:27`); a LIST recurses
   over its elements (`:36-48`); a `MAP | INFO | CONFIG | ASSOCIATION` pulls a
   key off the stack and dispatches on it (`:53+`).
5. **`bund2-api` cannot assume a compile-time-fixed word set** (RFC-0002). The
   table stays queryable by computed name at run time, in every tier.

Interaction with D3: D3 may still put `bund.eval` output permanently at
Tier 0, which bounds unbounded code *growth*. It does not purchase a closed
world, and D16 forecloses trying to reach one by restricting eval.

- Decided by: repository owner, answering open question Q2
- Blocks: RFC-0002, RFC-0005, RFC-0006
- Status: RESOLVED
- On the backtick: it is not an alternative spelling that D16 rescues. It is a
  first-class grammar production — `ptr` is one of the twelve alternatives in
  `value` (`reference/bund_language_parser/bund.pest:7-20`, rule at `:29`),
  it has its own parser handler
  (`reference/bund_language_parser/src/vm/ptr.rs:7-10`), and `name` carries an
  explicit negative lookahead `!("`")` (`bund.pest:28`) precisely to reserve
  the character for it. Its preservation follows from "Bund syntax and logic
  are preserved 100%" and is not contingent on this or any decision. Bund2's
  parser implements it whether or not any program uses it.
- Testing gap, recorded as Q9: `cargo xtask conform` cannot regress-test what
  no program exercises. `ptr` has 5 corpus uses across 3 programs, but the
  backtick form has zero — every backtick in the corpus sits inside a comment
  or a markdown literal — as do the bare-`STRING` and `MAP`-with-key forms of
  `execute`. That calls for hand-written tests, not a decision.

## D17 — `format` is language core
The `format` word (`reference/rust_multistackvm/src/stdlib/string/format.rs:135`)
is an important formatting feature and is preserved. It is not deferrable to
an out-of-tree word package.

Evidence behind it: 119 invocations across 52 of 132 programs — the third
most-depended-on word in the corpus, after `println` and `set`. In 13 of those
programs everything else used is stack/math/logic/lambda/oop, so `format` is
the single word that would break an otherwise-basic program.

Note what the claim does **not** rest on. `format` is not reachable from the
OOP layer: the `.format` method has its own implementation resolving
placeholders from object attributes
(`reference/Bund/src/stdlib/functions/oop/display_class.rs:14-85`), while the
`format` word pulls them off the stack
(`reference/rust_multistackvm/src/stdlib/string/format.rs:28-36`). They share
only `leon::Template` as a parser (`display_class.rs:29`, `format.rs:17`).
Nothing internal calls `stdlib_string_format`
(`reference/rust_multistackvm/src/stdlib/string/mod.rs:10` is its only other
mention). So this is a decision made on corpus dependency, deliberately, not
on structural reachability.

This is the **first per-word ruling under D14**, and it demonstrates that
subsystem grouping is a reporting aid rather than the partition itself: the
other 8 names in `vm/string` (`concat_with_space`
(`string/concat_with_space.rs:52`), `string.upper`, `string.lower`,
`string.snake`, `string.title`, `string.camel` (`string/case.rs:148-152`), and
the `sp` alias) have zero corpus uses and are not settled by this entry.

- Decided by: repository owner, answering open question Q3
- Blocks: nothing; contributes one word to D14
- Status: RESOLVED
- `format.` (`reference/rust_multistackvm/src/stdlib/string/format.rs:136`) is
  preserved with it, under D18.

## D18 — a preserved word carries its workbench form, and gaps are filled
When D14 preserves a word `W`, `W.` — the workbench-stack form — comes with
it. Where the reference already provides `W.`, it is preserved. **Where the
reference does not, Bund2 adds it.** The convention is to be made consistent,
not merely reproduced.

Rationale: `.` is not a naming flourish, it selects the operand source.
`stdlib_push_list_stack` and `stdlib_push_list_workbench` are the same
operation over `StackOps::FromStack` versus `StackOps::FromWorkBench`
(`reference/Bund/src/stdlib/functions/values/push.rs:11-25,56-62`). A word
with no `.` form is a hole in the workbench half of the language.

Corpus coverage is no guide here and must not be used as one: `format.` has
zero uses while `format` has 119, and `+++.` has 8 uses while `+++` has zero.
Which half of a pair a demo corpus exercises is an accident.

Scale. The `.` form exists for **124 of 386 base names** (32%); **262 lack
one**. By shape:

| Shape | Missing `.` form | Examples |
|---|---|---|
| namespaced `x.y` | 138 | `args.parse`, `bund.exit`, `console.box`, `debug.display_hostinfo` |
| plain | 100 | `alias`, `and`, `class`, `compile`, `display`, `dict`, `curry` |
| predicates `?x` | 13 | `?class`, `?key`, `?lambda`, `?try`, `?type` |
| operators | 6 | `!=`, `<`, `<=`, `==`, `>`, `>=` |
| capitalised | 5 | `True`, `False`, `List`, `Floats`, `Intervals` |

This is **additive**, so it does not touch preservation: no existing word
changes behaviour, every golden still passes, and `cargo xtask conform` is
unaffected. But the added words are new surface with no reference behaviour to
capture — nothing in `tests/golden/` can validate them, the same gap class as
Q9.

- Decided by: repository owner, following D17
- Blocks: nothing; a standing rule applied to every D14 per-word ruling
- Status: RESOLVED
- Open: whether gap-filling is universal or only where a workbench form is
  semantically meaningful. It is not obvious what `True.`, `Intervals.` or
  `==.` would do — the capitalised names are class constructors and the
  operators take two stack operands. Recorded as Q11.
- Not settled by this entry: the `,` "keep" suffix, a different axis (keep the
  operand versus consume it) that pairs with `.` to give a four-way family —
  e.g. `forecast.markov`, `forecast.markov.`, `forecast.markov,`,
  `forecast.markov.,` (`reference/Bund/src/stdlib/functions/forecast/markov.rs:74-77`).
  16 base names carry a `,` form and 16 carry `.,`. Recorded as Q10.

## D19 — `display` is language core
The `display` word (`reference/Bund/src/stdlib/functions/system/display.rs:88`)
is preserved. Second per-word ruling under D14, after D17.

Evidence: 12 invocations across 12 of 132 programs. Two of those —
`code_snippets/fmt_conditional_with_display_demo` and
`object_oriented_programming/class_display_demo_2` — touch nothing else
outside stack/math/logic/lambda/oop.

**This one does carry a structural dependency, unlike D17.** `display` reads
the `type` attribute of its operand and, for `"fmt"`, hands the value to
`conditional_fmt::conditional_run`
(`reference/Bund/src/stdlib/functions/system/display.rs:36`) before rendering
with `termimad::print_text` (`:45`, imported at `:11`). That is the same
module that implements the `fmt` word
(`reference/Bund/src/stdlib/functions/conditional/mod.rs:41` registers `fmt`
as `conditional_fmt::stdlib_conditional_fmt`; `conditional_run` is
`reference/Bund/src/stdlib/functions/conditional/conditional_fmt.rs:140`).
Preserving `display` therefore preserves the `fmt` machinery by reachability.
Whether the `fmt` *word* is separately core is not settled here — it has 11
programs of its own and needs its own ruling.

Do not confuse it with the `.display` **method**, which is a different
implementation: `.format` followed by `stdlib_print_inline`
(`reference/Bund/src/stdlib/functions/oop/display_class.rs:87-95`), with no
`conditional_fmt` and no `termimad`. The `Display` class exposes both as PTR
attributes (`display_class.rs:104-105`). Word and method are independent; a
ruling on one says nothing about the other.

Scope: already ruled in scope under Q7 — the ANSI styling `termimad` emits is
incidental to markdown rendering, not chosen colour, so D15 does not reach it.

- Decided by: repository owner
- Blocks: nothing; contributes one word to D14
- Status: RESOLVED
- Under D18, `display.` is added: the reference registers only `display`
  (`reference/Bund/src/stdlib/functions/system/display.rs:88`), with no
  workbench form.
- Not settled by this entry: the rest of `bund/system`. `display` is mis-filed
  there — it sits beside `system.shell` and `system.setproctitle` while being
  a renderer (Q4). Its neighbours get no ruling from this.

## D20 — lazy identity materialises on serialisation; `dup` does not serialise
`id` and `stamp` are generated lazily (D1, D2), and **serialisation is a
materialisation point**. `to_binary` writes concrete values, so the bincode
wire format stays byte-identical to the reference
(`reference/rust_dynamic/src/bincode.rs:30`) and `from_binary` restores them
verbatim (`reference/rust_dynamic/src/bincode.rs:71`).

**Both of those citations are in the non-JSON branches** — `:30` inside the
`} else {` at `:29`, `:71` inside the one at `:70` — so this paragraph states
the rule for every type *except* JSON. For a JSON value the round trip is not
identity-preserving: see the corrected scope below.

That is only affordable because **`dup` stops serialising**. Today
`Value::dup` is a bincode round-trip
(`reference/rust_dynamic/src/dup.rs:7-13`), reached from the `dup` word
through `dup_one` -> `dup_in_current_stack` -> `val.dup()`
(`reference/rust_multistack/src/ts_stack_op.rs:34`) on 38 of 132 programs,
with `?move`/`?.` hitting the same code at `ts_stack_op.rs:72,84`. Leaving it
in place would materialise `id` and `stamp` on one of the ten most-used words
and make D1 and D2 decorative. Bund2 implements `dup` as a structural clone
plus fresh identity — F13, dispositioned FIX.

After that fix, serialisation is confined to operations that genuinely
persist: `save.lambdas`/`save.stacks`/`save.aliases`
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:81-84`), `compile`
(`reference/rust_dynamic/src/bincode.rs:38-50`), and `wrap`
(`reference/rust_dynamic/src/bincode.rs:79`). Materialising there is correct:
those write values meant to be read by another process.

On the cross-run contract Q8 asked about — the id format and stamp precision
are observable only to a reader *outside* Bund2, because round-tripping is
self-consistent whatever is written (`bincode.rs:71` restores exactly what
`:30` wrote).

**Corrected scope.** Both of those citations sit in the *non-JSON* branches:
`:30` is inside the `} else {` at `reference/rust_dynamic/src/bincode.rs:29`,
and `:71` inside the one at `:70`. For a JSON value the round trip is **not**
self-consistent — `to_binary` converts to a string and re-wraps (`:9-28`) and
`from_binary` re-parses through `serde_json` (`:54-69`), so the reconstructed
value carries a fresh identity rather than the original's —
`Value::json` mints one with `nanoid!()`
(`reference/rust_dynamic/src/create_special.rs:205,207`). D20's rule holds as
stated for every other type; for JSON, identity is discarded by the round trip
and RFC-0001 must decide whether to preserve that or fix it. This is the same
asymmetry F13 records for `dup`. Whether such readers exist is **D11, still OPEN**. If D11
resolves to its default of none, a further step becomes available: encode
"unset" in the format so laziness survives serialisation entirely. Not adopted
now — it diverges the wire format, and D20 deliberately does not prejudge D11.

- Decided by: repository owner, answering Q8 (Option 3)
- Blocks: RFC-0001 (value representation)
- Depends on: F13, dispositioned FIX
- Status: RESOLVED

## D21 — authored probes are a separate corpus, with oracle-captured goldens
Behaviour the reference examples never exercise is tested by **probes**:
`.bund` programs we author, whose expected output is captured from the oracle
exactly as corpus goldens are. Probes live in `tests/probes/`, their goldens in
`tests/golden/probes/`.

The rule that makes this work: **we never hand-write expected output.** A probe
states what to run; the reference states what it does. Reading the reference
source and asserting the reading in a Rust test would encode our
interpretation rather than the behaviour — and the `execute` DICT arm shows why
that is not paranoia: it pulls the dictionary first and *then* pulls a key from
underneath it (`reference/rust_multistackvm/src/stdlib/execute.rs:55-61`), so
the source-level operand order is not evident from the code.

Provenance is kept separate on purpose. `tests/golden/` holds the reference's
own examples and is the preservation contract; the "three dispositions" rule in
CLAUDE.md is calibrated for those. A probe is ours, so a probe that turns out
to encode a reference bug is a fourth situation, and mixing the two would blur
the rule that keeps corpus goldens sacred.

**Probes target behaviours, not words.** A word-level probe is not enough where
a word is polymorphic. `execute` (spelled `!`, 69 invocations across 39 of 132
programs) dispatches on eight arms
(`reference/rust_multistackvm/src/stdlib/execute.rs:26-97`): `PTR|STRING|CALL`,
`LAMBDA`, `CONDITIONAL`, `OBJECT`, `CLASS`, `LIST`,
`MAP|INFO|CONFIG|ASSOCIATION`, and the non-executable error arm. The corpus
reaches PTR, LAMBDA, CONDITIONAL and OBJECT; the rest are untested while the
coverage metric counts the word as covered. One probe per arm, not one per
word.

- Decided by: repository owner, answering Q9 (Option C)
- Blocks: nothing; unblocks the "covered by a hand test" term in
  `cargo xtask coverage` — though capture is not implemented yet, carried as Q16
- Status: RESOLVED
- Known limitation, carried on Q5: coverage is measured per *word*, not per
  behaviour. `!` counts as covered today with half its dispatch untested.
  Probes fix the testing gap; they do not make the metric finer.
- Capturing a probe golden requires building the oracle. That is the one
  sanctioned reason to build `reference/`, and it stays out-of-tree:
  `cargo build --release --manifest-path reference/Bund/Cargo.toml
  --target-dir target/oracle`.

## D22 — the `,` suffix axis is not extended
D18 fills missing `.` workbench forms. That does **not** extend to `,`. Forms
that exist in the reference are preserved with their base word; none are
invented.

The reason is that `,` fails the property D18 relies on. `.` is mechanically
determined: `W.` is the same operation over `StackOps::FromWorkBench` instead
of `FromStack` (`reference/Bund/src/stdlib/functions/values/push.rs:11-25`), so
its meaning is fully implied by `W` and filling a gap invents nothing.

`,` carries **two unrelated meanings**:

| Meaning | Evidence |
|---|---|
| keep the operand rather than consume it | `stat.count,` -> `stdlib_stats_stack_keep_count` against `stat.count` -> `..._consume_count` (`reference/Bund/src/stdlib/functions/statistics/count.rs:52-55`); likewise `reference/Bund/src/stdlib/functions/forecast/markov.rs:74-77` |
| operate in place | `get,` -> `DictOp::GetInplace`, `set,` -> `SetInplace` (`reference/Bund/src/stdlib/functions/values/getsetinplace.rs:101,109`) |

Filling `,` gaps would therefore mean choosing, per word, between two
conventions — and for most words neither is meaningful. There is no sense in
which `println,` or `dup,` exists to be discovered. That is language design,
not preservation.

Scale confirms the demand is negligible: 16 base names carry `,`, each with a
`.,` partner (32 forms), all in forecast, statistics, math and values. **The
corpus uses exactly one of them**, `get,`, at 34 invocations across 10
programs — and it arrives free when `get` is preserved.

Spelling rule, which holds either way: when both suffixes apply, `.` precedes
`,`. The reference registers `get.,` and `stat.count.,`, never `get,.`. So a
`,` form's workbench partner is `W.,`.

A specific `,` form wanted later can be added on its own evidence by its own
decision.

- Decided by: repository owner, answering Q10 (option B)
- Blocks: nothing; bounds D18
- Status: RESOLVED
- Unrelated to the suffix: `,` is also a standalone word, an alias of `set`
  (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:21`), used 21
  times across 11 programs. The character is overloaded; the two uses do not
  interact.

## D23 — `<class> !` creates an object of that class, however the class arrived
Resolves Q12 and completes F16's FIX. Executing a CLASS value builds an object
of that class. Both provenances are supported and behave identically:

- a class registered earlier and resolved back onto the stack, and
- a class constructed dynamically on the stack and never registered.

So `!` must build from the CLASS **value** it is given, not by looking a name
up in the class registry. The registry is still consulted for *parents*:
`.super` holds parent class names
(`reference/Bund/src/stdlib/functions/oop/base_classes.rs:95`), and
construction walks them (`reference/rust_multistackvm/src/stdlib/bund_object.rs:44-48`).
A class whose parents are unregistered still fails, on the parents.

- Decided by: repository owner, answering Q12
- Blocks: nothing; completes F16
- Status: RESOLVED
- Residual, recorded as Q13: object construction stamps `.class_name` from the
  name it was handed (`reference/rust_multistackvm/src/stdlib/bund_object.rs:36`),
  and a dynamically built class has no name — `class` sets only `.super`
  (`reference/rust_multistackvm/src/stdlib/artefacts.rs:69-73`) and
  `register_class` stores the value under a name without injecting it
  (`reference/rust_multistackvm/src/multistackvm_classes.rs:7-20`). An object
  made from an anonymous class would therefore lack `.class_name`, which
  `.str`, `.print` and `.println` all require
  (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:36-38`).

## D24 — `.` gap-filling is for operand-sourcing words only
D18 fills missing `.` workbench forms. D24 bounds which: a missing `W.` is
added **only where `W` sources a primary operand**. Pure producers get none.

The reason a producer needs none: `.` is *also* a word — an alias of `return`,
which moves stack to workbench
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:4`,
`reference/rust_multistack/src/ts_workbench.rs:29-34`), used 69 times across 27
programs. `take` goes the other way, 69 uses across 29 programs. So `True .`
already puts a constructed value on the workbench, and a `True.` suffix would
duplicate an idiom the language has and programs already use. For a word that
*consumes*, no such composition exists: only the suffix lets the operand stay
on the workbench.

The `.` contract, to be stated once rather than rediscovered 262 times: the
primary operand is pulled from the workbench and the result pushed back to the
workbench, while secondary operands still come from the main stack
(`reference/Bund/src/stdlib/functions/values/push.rs:26-52`).

**This decision cannot yet be applied, and the reason is worth recording.** An
audit was added to `cargo xtask corpus` that classified words by whether their
handler threads `StackOps`, and it reported 98.3% agreement with "has a `.`
form". That number is **circular and proves nothing**: `StackOps` is the
mechanism by which a `.` form is implemented
(`reference/Bund/src/stdlib/functions/values/push.rs:10`), so the two
properties are near-tautologically linked. The bucket it labelled "producers"
contains `set`, `get`, `len`, `math.sqrt`, `string.upper` and `==` — plain
consumers that merely lack a workbench variant. The audit remains in the tool,
relabelled, for its two genuinely useful outputs; it is not evidence for this
decision.

Partitioning the 262 needs real stack-effect data, which is
`cargo xtask arity` — "probe every registered word against instrumented stacks
and emit a first-cut stack-effect table". That command is not implemented, so
**D24 is decided but blocked on it.** No `.` forms are to be added by hand
before that table exists.

Two side findings from the audit that do stand:

- 7 words implement their two forms as separate functions rather than one
  parameterised base — `?`, `do`, `for`, `times`, `while`, `format`, `stdin`.
  Bund2 should not assume a single parameterised base everywhere.
- 7 look like gaps but are naming artefacts: the `if.*` family spells its
  workbench form `.in_workbench` rather than with a `.` suffix
  (`reference/rust_multistackvm/src/stdlib/logic/if_fun.rs`). Whether those are
  renamed to the suffix convention is not settled here.
- 23 handlers could not be resolved by static scan (registered through a path
  or macro) and need checking by hand.

- Decided by: repository owner, answering Q11 (option B)
- Blocks: nothing; bounds D18
- Blocked by: `cargo xtask arity` — **now implemented**, writing
  `docs/arity.md`. It reports two independent columns per word: the declared
  `current_stack_len() < N` guard, and consumed/produced observed by running
  the word against the oracle. Applying D24 means reading that table, not
  re-deriving arity by hand. Note it is explicitly a *first cut*: words that
  reject every sentinel type on type grounds are recorded as
  type-constrained rather than guessed at, so the table marks its own
  uncertainty.
- Status: RESOLVED (application deferred until the arity table is reviewed)

## D25 — an anonymous class must name itself
Resolves Q13. When `<class> !` (D23) builds an object from a CLASS value, the
object's `.class_name` comes from the class value's own `.class_name`
attribute. If the class does not carry one, construction **fails at that
point** rather than producing an object without a class name.

This is not a new mechanism. Every built-in class already sets `.class_name`
on the class value itself — `Object`
(`reference/Bund/src/stdlib/functions/oop/base_classes.rs:96`), `Printable`
(`:110`), and ten more across `oop/`, `image/` and `ai/`. A program writing
`:X class :.class_name "X" set ... !` is following the convention the
reference already uses.

Registered classes are unaffected: `make_bund_object` stamps `.class_name`
from the registry key (`reference/rust_multistackvm/src/stdlib/bund_object.rs:36`)
and keeps doing so. The rule only supplies the missing source for the
anonymous case.

Why failing beats improvising: an OBJECT without `.class_name` is malformed.
Two code paths read it — `.str`/`.print`/`.println`
(`reference/Bund/src/stdlib/functions/oop/base_classes.rs:36-38`) and the
object-reuse test `if_object_of_class_in_stack`
(`reference/rust_multistackvm/src/stdlib/bund_object.rs:15-21`) — and the
alternatives are worse. Synthesising a name from the value's id reintroduces
F14's unreproducible output and cuts against D1's lazy identity. Synthesising
a constant such as "Anonymous" is reproducible but **collides**: two unrelated
anonymous objects would then satisfy `if_object_of_class_in_stack` for each
other, silently confusing the reuse path.

Also rejected: making `class` consume the name atom and stamp it. That would
make every class self-describing, but it changes `class` from 0->1 to 1->1
arity and changes `register`'s operand shape, breaking the
`:A class ... register` idiom used by 10 corpus programs — including
`3.14 :Answer dup class`
(`reference/Bund/examples/object_oriented_programming/class_display_demo.bund`),
which duplicates the atom deliberately. That is a preservation break.

No golden changes: `<class> !` errors today under any spelling (F16), so this
is added behaviour.

- Decided by: repository owner, answering Q13 (option A)
- Blocks: nothing; completes D23 and F16
- Status: RESOLVED
- Follow-on: `tests/probes/execute-arm-class.bund` builds a nameless class and
  must be reshaped to set `.class_name`, since it is the probe that will prove
  F16's fix.

## D26 — the embedded database layer is not a mandatory layer
`bund/internaldb` — `internaldb.sql`, `internaldb.execute`, `internaldb.prql`,
`internaldb.version` (`reference/Bund/src/stdlib/functions/internaldb/mod.rs:65-67`)
— is **deferred**. It is an external data layer, not part of the language, and
what replaces it is left open.

Corpus cost: 2 programs, `data/internaldb_demo` and
`data/internaldb_demo_with_prql`. Both were already outside the conformance
suite — every word in the subsystem carries a database effect — so deferring
it removes nothing that was being verified.

Not to be confused with D27: this is the *user-facing* database layer a Bund
program queries. D27 concerns the world file, which is Bund2's own
persistence.

- Decided by: repository owner
- Blocks: nothing; narrows D14 by one subsystem
- Status: RESOLVED (revisit when the replacement is chosen)

## D27 — the world file is redb
Bund2's world file uses **redb** — pure Rust, single file, embedded, ACID —
replacing the reference's SQLite
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:69` and siblings).

A graph database was considered and rejected on evidence: the world file is
not a graph workload. It is six key-to-blob tables — `LAMBDAS`, `ALIASES`,
`STACKS`, `STACK_DATA`, `MODELS`, `BOOTSTRAP`
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:69`, `aliases.rs:60`,
`stacks.rs:129,187`, `models.rs:11,79`, `bootstrap.rs:179`) — storing whole
`Value`s as bincode BLOBs (`lambdas.rs:81-84`). Nothing traverses edges.

Nor is there a mature crate meeting both constraints: of the current
candidates, the SQLite-backed ones are not pure Rust and need a C toolchain,
while the pure-Rust ones store a directory rather than a single file. redb is
the one crate that is both, and dropping the C dependency also eases **D10**
(whether `bund2 build` may require `cc`).

**This is a format change, and it is gated on D11** — "does anything outside
the project depend on the current format". If an external reader of the world
file exists, switching backends breaks it; if not, the change is free. D11 is
still OPEN, so this decision is taken on the expectation that it resolves to
its default of "no external dependents". If it does not, D27 must be revisited
before implementation.

Unaffected: what goes *into* the world is still bincode-serialised `Value`s,
so D20's rule stands — serialisation materialises lazy identity, and the value
encoding is unchanged. Only the container changes.

- Decided by: repository owner
- Blocks: nothing; informs RFC-0003 and D10
- Depends on: D11
- Status: RESOLVED (conditional on D11)

## D28 — only essential features in the default build
Bund2's default build enables only what the language needs. The heavyweight
subsystems are feature-gated and **off by default**; nothing is deleted, but
nothing non-essential is linked unless asked for.

Measured cause, `cargo xtask bench` plus a decomposition of the floor:

| stage | best of 9 |
|---|---|
| process spawn floor | 1.4 ms |
| `bund --version` — load and link only, before any stdlib init | **11.0 ms** |
| plus stdlib registration (empty program) | 15.8 ms |
| plus parse and run (hello world) | 14.0 ms |

So roughly **9.6 ms of every run is spent before `main` does anything**,
loading and linking a **381 MB** binary. Stdlib registration adds 3-5 ms.
Interpretation is below the noise floor. The corpus baseline of ~14 ms per
program is almost entirely the cost of the dependency set.

That dependency set is what the decision targets. `reference/Bund/Cargo.toml`
links, among others: `lingua` (language detection), `hyphenation` with
`embed_all`, `duckdb` with `bundled`, `polars` and `polars-io`, `arrow`,
`prqlc`, `charabia`, `neurons`, `augurs`, `rustface`, `imageproc`, `viuer`,
`zenoh`, `dryoc`, `reqwest`, `deepseek-api`. Each embeds data or a large
native library, and together they are the 381 MB.

The subsystems they serve are the ones already being deferred or shown unused:
`bund/ai` and the classifiers, `bund/internaldb` (D26), `bund/image`,
`bund/bus`, `bund/forecast` and `bund/statistics` (42 registered names, zero
corpus uses), `bund/console` (D15), and the random-string and hyphenation
corners of `bund/string`.

**Consequence for Q14, which this partly overturns.** The Phase 0 finding that
"93% of every run is fixed cost" is a property of the *reference's*
dependency set, not an intrinsic cost of running Bund. Bund2 does not inherit
those dependencies, so its floor will be far lower and the corpus will resolve
interpretation far better than the baseline suggests. Comparing Bund2's
wall-clock against `docs/bench-baseline.md` compares two dependency sets, not
two interpreters, and any performance criterion has to say which it means.

- Decided by: repository owner
- Blocks: nothing; constrains RFC-0002's crate and feature layout, and the
  M6 denominator alongside D14
- Status: RESOLVED
- Not a deletion: a feature-gated word can be enabled. What this forbids is
  linking it by default.

## D29 — the four dead words: revive or drop
F19, F22 and F24 leave four names registered only into the stack layer's dead
`functions` table, unreachable by any dispatch path: `dup_in`,
`from_workbench`, `push_to`, `stacks_left`. Two aliases, `<-` and `←`, point
at `stacks_left` and are therefore dead too (F22).

Bund2 must either implement them as real words or omit them, and either choice
is a deviation from the oracle in one direction or the other. **Not decided
here** — dropping a word the reference documents, or adding one it cannot
execute, is a preservation call for the owner.

The evidence, so the call is cheap to make:

- **No golden moves either way.** No corpus program uses any of the four, nor
  `<-` or `←`, so `cargo xtask conform` is unaffected at 0/63.
- **Coverage moves against reviving.** Implementing all four takes the
  in-scope denominator from 497 to 501, with all four untested — so reviving
  them lowers coverage while adding no verified behaviour.
- **Reviving `stacks_left` is the one with a real payoff.** It repairs `<-`
  and `←`, and makes the `stacks_left`/`stacks_right` pair symmetric for the
  first time. Note this is separate from F23, which is `rotate_stack_right`
  calling the left rotation — a different word and a different bug.
- **Nothing depends on the other three.** `dup_in`, `from_workbench` and
  `push_to` have no aliases, no corpus uses, and inline siblings that already
  cover the same ground (`dup_one`, `take`, `push`).

Options: revive all four; revive `stacks_left` alone, on the strength of the
two aliases pointing at it; or omit all four and drop `<-`/`←` with them,
recording the removal as a stated deviation.

- Blocks: RFC-0002's word set, and the M6 denominator alongside D14
- Default: none — the arguments cut both ways and the owner should pick
- Evidence: F19, F22, F23, F24; `cargo xtask corpus`, `cargo xtask coverage`
- Status: OPEN
