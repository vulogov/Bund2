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
- Status: **RESOLVED — no eval-specific tier rule. Eligibility falls out of the
  content-hash compiled cache, bounded by that cache's cap.**

### The question was miscast

It asks which tier eval'd code belongs to, which presumes eval'd code is a
thing a tier can hold. It is not. `bund_compile_and_eval` parses the string and
`apply`s each token straight into the VM, retaining nothing
(`reference/Bund/src/stdlib/helpers/eval.rs`); `Bund::eval` is the same loop
(`reference/bundcore/src/bundcore_eval.rs:7-45`, and F52 records the
duplication). There is no artifact, no name, and nothing to attach compiled
code to. A tier policy needs a subject before it needs a verdict.

### Code arriving through eval has two fates, and only one is in scope

A snippet that calls `register` installs a **named lambda** in the ordinary
table (`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:5-22`).
From that moment it is indistinguishable from a lambda written in source —
reached through `apply` → `is_lambda` → `lambda_eval` — and it is JIT-eligible
by the normal word path whatever this decision says. D3 governs only the
*other* fate: the token stream applied once and discarded.

### What eligibility would actually require

Three things, none of them specific to eval:

1. **A stable key.** A content hash of the snippet, since it has no name.
2. **Reuse.** A hit counter on that key, promoting at a threshold.
3. **A bound.** A cap with eviction, because distinct snippets are distinct
   compilation units — which is exactly the unbounded growth this decision was
   opened to prevent.

RFC-0003 already owes (1) — "compiled cache keyed by content hash" is its
assigned improvement for lambda bodies — and RFC-0005 already owes (3), as
"code-memory caps; recompile caps; demotion policy". Both exist for reasons
that have nothing to do with eval.

### The decision

**Do not legislate a tier for eval output.** Hash the token stream as any
other body is hashed and let the ordinary promotion threshold decide. A snippet
that repeats gets hot and compiles; a snippet that never repeats never crosses
the threshold and is never compiled. Code memory is bounded by the cache cap,
not by a carve-out.

"Permanently Tier 0" is only *necessary* if the cache is not content-keyed. It
is available at any time as a one-line policy if evidence later demands it, and
resolving this way does not spend that option.

### Why this is safe on the evidence

`bund.eval` appears **once in the 132-program corpus**, at
`reference/Bund/examples/code_snippets/bund_shell.bund:24` — and it is a REPL.
`input*` is a readline loop that pushes each typed line and evaluates the body
per iteration (`reference/Bund/src/stdlib/functions/io/input.rs:105-116`), so
every evaluation sees a different string.

That is the adversarial case for JIT eligibility, and content-hash keying
handles it correctly *without* a special rule: N distinct lines produce N keys
with zero hits, nothing reaches the threshold, and nothing is compiled. A blunt
"permanently Tier 0" gets the same answer here but also excludes
`bund.eval-file` re-run in a loop — same file, same hash, genuinely hot — which
it cannot distinguish.

### Consequences

- **RFC-0003 is unblocked**, and gains no eval-specific machinery. It specifies
  one evaluator (F52) and one content-hash cache; eval uses both.
- **RFC-0005 inherits the bound.** The cap and demotion policy it already owes
  are now load-bearing for this decision too, and must be stated as such rather
  than left as tuning.
- **A Tier-0 win is available in the same place.** The reference re-parses
  identical strings on every call — there is no parse cache at all. Keying
  parses by the same content hash helps the REPL case with no JIT involved.
- **`--noeval` remains the hard answer.** It already replaces all four eval
  words with a bailing stub (`reference/Bund/src/stdlib/functions/bund/bund_eval.rs:117-121`),
  so a build in which this question cannot arise is a supported configuration
  and not something Bund2 has to invent.
- **Revisit if a workload repeats snippets.** The evidence here is one corpus
  program. If a real workload evals the same string hot, this decision already
  does the right thing; if one evals varying strings faster than the cache
  evicts, the cap is the lever, and that is RFC-0005's to tune.

### Amendment (2026-08-29) — the mechanism was wrong; the conclusion survives

RFC-0003's first review found this resolution had not consulted **D5**, which
was already RESOLVED and blocks RFC-0003. D5 finds lambda bodies write-once and
states the consequence: "the compiled cache needs no invalidation machinery …
a cache keyed on **identity** simply does not contain the replacement."

This entry assumed a **content hash**. Two things follow, and the second
reverses an argument made above.

**Content hashing does not work naively.** Every `BundValue` carries a lazily
minted `id` and a sampled `stamp` (D1, D2). A hash over a freshly parsed body
therefore never equals the hash of an earlier parse of the same text unless the
hash is explicitly defined to exclude identity and stamp. Nothing had defined
that. As written, the cache would never hit and this resolution would have
failed silently.

**Under D5's identity keying, eval'd code is never promoted.** Each
`bund.eval` re-parses and mints fresh values, so an identity-keyed cache cannot
hit for eval output at all — no matter how often the same string is evaluated.

The conclusion is unchanged: **no eval-specific tier rule is needed.** But the
reason is now the opposite of the one given above. It is not that repeated
snippets get promoted and unrepeated ones do not; it is that eval output is
structurally incapable of hitting an identity-keyed cache, so it stays at Tier 0
without anything being legislated. That converges with this entry's recorded
default rather than departing from it.

**Withdrawn:** the argument that a blunt "permanently Tier 0" rule would wrongly
exclude `bund.eval-file` re-run in a loop. Under identity keying that case is
not promoted either, so the rule and the cache agree and the distinction the
argument rested on does not exist.

**Still standing:** the Tier-0 parse-cache observation. The reference re-parses
identical strings on every call; caching *parses* by source text is independent
of how compiled code is keyed, and is where the REPL win actually is.

If a future decision adopts content hashing over identity — for the separate
benefit of letting structurally identical lambdas share compiled code — it must
define the hash to exclude `id` and `stamp`, and this entry's original reasoning
becomes live again. That is not decided here.

## D4 — integer width
Is full `i64` required, or are 51-bit integers acceptable? The latter allows
NaN-boxing and a smaller value.
- Blocks: RFC-0001
- Default: full `i64`
- Evidence: `cargo xtask layout`
- Status: **RESOLVED — full `i64`.** NaN-boxing is not taken.

Measurement narrows what NaN-boxing is worth. With identity in the heap
header (D1, D2, and the layout scan), the candidate value is **already 16
bytes at full `i64`** — one tag word plus one payload word. NaN-boxing would
fold the tag into unused float bits to reach 8, so the question is whether
halving 16 justifies capping integers at 51 bits.

For contrast the same measurement puts the reference's `Value` replica at
**176 bytes**, and identity carried inline at 32. The large win is already
taken by moving identity off the value; NaN-boxing is a second, smaller step
with a semantic cost attached.

**Resolution.** Full `i64`. 176 -> 16 is the win that mattered and it is
already banked; 16 -> 8 buys packing density in lists, not speed on the common
path, because a 16-byte value is two machine words and a scalar never reaches
the heap either way (`cargo xtask layout` measures 0 allocations for
constructing and cloning a scalar under candidate A).

What it would cost is a 51-bit integer cap, and Bund is dynamically typed:
nothing in the language warns before an integer wraps. `cast_int` returns
`i64` (`reference/rust_dynamic/src/cast.rs:17`) and the reference stores
`Val::I64(i64)` (`reference/rust_dynamic/src/types.rs:72`), so 51 bits would
be a narrowing of an existing observable range, not a choice about a new one.

The asymmetry decides it: representation is private behind `BundValue`'s API
and can change later, so NaN-boxing stays available as an optimisation.
Integer width is semantic and observable, so capping it is not reversible once
programs depend on it.

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
- Blocks: RFC-0007, **and RFC-0008** — which this line did not say, and
  RFC-0003:944-946 did. A reader who searched only `Blocks:` would have missed
  it, which is why CLAUDE.md says to search the registers by subject.
- Default: VM-per-task
- Status: **RESOLVED — VM-per-task, and the debugger stops a thread rather than
  suspending a word**, 2026-09-30. See **D84**.

## D7 — concurrent VM count
Tens (actor model is fine) or thousands (per-VM word tables become the memory
story)?
- Blocks: RFC-0007
- Default: tens
- Status: **RESOLVED — tens**, 2026-09-30, on a measurement the question had
  never been given. See **D85**.

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
and that is **D31**, still OPEN. (It was recorded against D11 until D11 was
split; D11 asks about the object format, which is a different artefact.) If an
external reader exists, changing the backend is a breaking format change; if
not, it is free. D27 takes redb, conditional on D31.

- Read against the pinned submodule, not GitHub `main`. `reference/Bund` is
  pinned (`reference/PINNED.txt`) and upstream may have moved since; per
  CLAUDE.md the pinned copy is what citations resolve against.

## D9 — third-party CLIF lowerings
Should `Intrinsic` lowerings ever be exposed through `bund2-api`? Doing so pins
external packages to an exact Cranelift version.
- Blocks: RFC-0002
- Default: no
- Status: **RESOLVED — no**, and **amended 2026-09-09**: the answer is about
  `bund2-api`'s surface, not about where Bund2's own lowerings live. See the
  amendment below. Decided by the repository owner; this adopts the recorded
  default, now explicitly.

Three reasons, the third of which is not in the research note.

A `LowerFn` signature contains `cranelift_frontend::FunctionBuilder`, so every
external package using one is pinned to the exact Cranelift version Bund2 was
built with. `cranelift-jit` self-describes as extremely experimental and moves
on a monthly cadence, which would make `bund2-api`'s stability guarantee false
(`docs/research/01-extensibility-async.md:184-189`).

**RFC-0000's B3 requires `bund2-stdlib` not to depend on `bund2-jit`** — Tier 1
stays optional. Exposing lowerings through the stable surface would make that
surface depend on the optional subsystem, which is the inversion B3 exists to
prevent.

And it is the reversible direction: not exposing something can be undone later,
exposing it cannot. The same asymmetry decided D4.

External crates get `Native` with a declared effect, which is already a large
improvement on today's opaque `fn(&mut VM)`.

### Amendment, 2026-09-09 — the scope is the stable surface, not the placement

The concluding sentence above — "`Intrinsic` stays internal to `bund2-stdlib`"
— has been read as deciding **where Bund2's own lowerings may live**. It does
not, and RFC-0005 §S6 spent a review cycle apparently trapped by it.

Read the question this entry asks: *"Should `Intrinsic` lowerings ever be
exposed through `bund2-api`?"* All three reasons are about the stable ABI — a
`LowerFn` pinning external packages to a Cranelift version, the stable surface
acquiring a dependency on an optional subsystem, and the reversibility of not
exposing. None of them speaks to Tier 1 lowering Bund2's own words.

**What is actually forbidden**, and it is RFC-0000's rule rather than this one:
`bund2-stdlib` must not depend on `bund2-jit` (criterion B3). **The reverse is
not forbidden** — `bund2-jit` may depend on `bund2-stdlib`, and nothing in
RFC-0000 says otherwise. So the constraint that binds is narrower than it
looked: **no Cranelift type may appear in `bund2-stdlib` or `bund2-api`.**

RFC-0005 §S6 satisfies that by having a word publish a **BundIR** fragment
rather than a CLIF lowering. BundIR mentions no Cranelift type, `bund2-ir` is
already a dependency of `bund2-jit`, and the code generator does the CLIF work
on its own side of the boundary. This entry's resolution is unchanged: nothing
of the sort is exposed through `bund2-api`, and an external package still gets
`Native` with a declared effect and no more.

- Amended by: repository owner, 2026-09-09, on Q33

## D10 — C toolchain requirement
May `bund2 build` require `cc`, or must the compiler be self-contained?
- Blocks: RFC-0006
- Default: yes, `cc`; `--emit=bundle` covers toolchain-free targets

### Resolution

**Yes.** `bund2 build --emit=native` may require a C toolchain. `cranelift-object`
produces a `.o` and something has to link it; a self-contained linker is a
project of its own and not one this language needs.

The clause after the semicolon is **load-bearing, not decoration**.
`--emit=bundle` — runtime plus embedded IR, pure interpreter, no Cranelift and
no `cc` — is what a target outside x86-64/aarch64/s390x/riscv64 gets, and
`docs/research/02-native-binaries.md:228-231` calls it "a first-class output of
`bund2 build`, not an afterthought". RFC-0006 inherits both halves: it may
shell out to `cc` for `--emit=native`, and it must keep `--emit=bundle`
buildable without one.

**What this resolution does not permit.** It scopes the requirement to *one
output mode of one subcommand*. It does not permit the interpreter, the crates
it is built from, or `--emit=bundle` itself to require a C compiler — that
would invert the escape hatch, since producing the toolchain-free artefact
would then need a toolchain. D40's `grok` dependency did exactly that and is
feature-gated off by default as a result; see its amendment.

- Decided by: repository owner, 2026-09-07, asked directly rather than taken as
  a default after D40 was found to have spent it silently
- Blocks: RFC-0006
- Status: **RESOLVED — yes for `--emit=native`; `--emit=bundle` stays
  toolchain-free, and nothing below `bund2 build` may require `cc`.**

## D11 — external dependents of `compile_to_binary`
Does anything outside the project depend on the current bincode object format?
- Blocks: RFC-0003
- Default: no; version the IR format freshly
- Status: **RESOLVED — no external dependents. Version the IR format freshly.**

**This entry was being asked two questions and it only ever posed one.** As
written it is about the *object format* — what `compile_to_binary`
bincode-serialises. D27 was leaning on it for the **world file**, which is a
different artefact with a different exposure profile. The world-file question
is split out as **D31**; this entry answers the one it asks.

For the object format the answer is close to a proof rather than a judgement:
**nothing reachable from Bund produces one.**

- `Value::compile` is the object-format producer
  (`reference/rust_dynamic/src/bincode.rs:38`), and it has **zero callers** in
  any of the six crates.
- The Bund word `compile` is a different thing entirely: it parses a string
  and pushes a `LIST` onto the stack
  (`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:43`). It
  never serialises and never writes a file.
- No other word emits a compiled object. The words that reach a file are
  `save.*` and `load.*`, which are the world file — D31 — and `wrap`/`unwrap`,
  which produce an `ENVELOPE` value that stays on the stack.

So no Bund program can ever have produced a file in this format, and there are
no files in the wild to break.

**Amended 2026-09-29, on RFC-0006's second review: the clause "No other word
emits a compiled object" is wrong, and the resolution survives anyway.**

`encode.base64` and `encode.base64.` pull a value and call **`to_binary`** on
it — the same serialiser `Value::compile` delegates to
(`reference/Bund/src/stdlib/functions/encoding/base64.rs:33`). So
`"1 2 +" compile encode.base64` yields the base64 of exactly the bytes
`compile_to_binary` would write, and a Bund program *can* produce them. This
entry's own list named `wrap`/`unwrap` and the world file and missed the
encoding group.

**There is also a second producer in the Rust code this entry does not
mention**: `compile_to_binary`, which parses source, folds the token vector
into one LIST through `bund_vec_to_list`, and serialises that
(`reference/bund_language_parser/src/compile.rs:6-30`). It has zero callers.

**Why the resolution is unchanged.** The question this entry asks is whether
anything *outside the project* depends on the format. Those bytes reach a Bund
program only as a base64 string on the stack, read back by `decode.base64`
within Bund; no word writes the raw format to a file, and no external consumer
is implied. "Version the IR format freshly" stands. What was wrong was the
proof's completeness, not its answer — and RFC-0006 repeated the gap as "no
artefact of either shape exists in the wild", which is false.

**Amended again 2026-09-29, on RFC-0006's third review: "no word writes the raw
format to a file" is also wrong, and the amendment above is what got it
wrong.**

`save.*` writes **every lambda** to the world file with `to_binary` — exactly
`Value::compile`'s format — as a BLOB in the `LAMBDAS` table of a SQLite
database on disk
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:80-95`).

So the object format reaches a file, routinely, through the word group this
entry's own text points at while claiming the opposite. **Whether anything
outside Bund2 reads that file is D31, which is OPEN.** This entry's resolution
is therefore *not* independent of D31 in the way its split from D31 assumed:
the two questions were separated as different artefacts, and they share a byte
format.

**What follows, and what does not.** "Version the IR format freshly" is still
the right answer for a format Bund2 designs fresh — RFC-0006 §B2 embeds source
text and uses neither format, so nothing built on this RFC rests on it. What
cannot stand is this entry being read as a closed proof: it is contingent on
D31, and D31 is the owner's.

 External dependence would require a Rust
consumer calling `rust_dynamic` directly, which is a different question from
the one this entry asks and one the repository owner is positioned to answer
outright.

Two things this unblocks besides RFC-0003. **D20**'s deferred step — encoding
"unset" in the wire format so laziness survives serialisation — becomes
available, since it was held back only on this. And it does **not** discharge
D27's condition, which is D31.

**Note, 2026-10-08 (RFC-0006's sixth review, S2 and S3).** Two sentences in
the amendments above are wrong. **D31 was already RESOLVED** — 2026-09-11, no
external readers — when the second amendment called it open and "the owner's",
so the dependency drawn there is closed. And `"1 2 +" compile encode.base64`
does not yield "exactly the bytes" `compile_to_binary` would write: the Bund
word `compile` appends a newline and stops at the first `EXIT`
(`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:29-36`), and a
`Value` serialises an `id` and `stamp` minted at construction
(`reference/rust_dynamic/src/value.rs:15-36`), so no two parses give equal
bytes. What holds is that it is the same *format*. The resolution stands.

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

**Clarified 2026-09-10.** "A site using a fold bails to Tier 0", above, was
written before RFC-0004's amendment of 2026-09-09, which withdrew any reading
in which compiled code hands control back mid-body: there is no OSR to do it
with. It is read as RFC-0005 §S5's rule — **promotion stops at the site**, the
promoted values are synced, and what follows runs through runtime helpers —
which is what this decision's barrier needs, and nothing more. Bund2 registers
none of these words yet; when it does, each declares `StackEffect::opaque`.

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
- Status: **RESOLVED — method B″.** Core 286, library 211, of 497 in scope.
  The partition is generated into `docs/core-words.md` by
  `cargo xtask scope --write`, so it is an artefact rather than a list in
  prose. Decided by the repository owner. The per-word method survives for
  overrides — D17 (`format`) and D19 (`display`) were made that way, and
  anything B″ misfiles is corrected the same way.
### Method B″, and why not the alternatives

The core set is **computed**, in four steps, each re-derivable by re-running
`cargo xtask scope`:

1. **Seed** — the words the corpus invokes, plus the words the authored probes
   invoke. Aliases seed their targets, since calling an alias calls the target.
2. **Closure** — every word in a subsystem that a seed word's implementation
   reaches into. This is what D19 established when preserving `display` also
   preserved `conditional_fmt`.
3. **File completion, in `vm/` and `stack/` only** — a file with any core word
   is wholly core there, and never in `bund/`.
4. **D18 workbench forms** — a core word's `.` sibling is core.

Five partitions were priced before choosing:

| variant | core | library | core % | closes by |
|---|---|---|---|---|
| B | 214 | 283 | 43.1% | closure + D18 only |
| B+probes | 218 | 279 | 43.9% | probes seed too |
| B' | 347 | 150 | 69.8% | then file completion everywhere |
| **B″** | **286** | **211** | **57.5%** | **file completion in `vm/` and `stack/` only** |
| C | 460 | 37 | 92.6% | subsystem completion |

**B alone under-includes.** The 132-program corpus never invokes `<=`, `>=`,
`and`, `or`, `?true` or `convert.to_int`, so B files comparison and boolean
operators as library. That is B working as specified, not a tool defect.

**B' over-includes.** Its 129 additions bring in the whole `string.distance.*`
and `string.random.*` families, `fs.cp`, `fs.mv`, `url` and
`sysinfo.hostname` — library by any reading.

**Step 3's restriction is what separates them, and it is the reference
author's own line rather than a judgement of ours.**
`reference/Bund/Documentation/Bund_Library_Guide/Library_introduction.typ:15-19`
divides the implementation into `rust_multistack` (stack operations),
`rust_multistackvm` — "the core logic of the BUND language remains intact
within this crate" — and the Bund runtime, which "encompasses implementing all
standard library functions".

**Read precisely, that is an axis and not a cut.** `:16` says `rust_multistack`
"incorporates elements of the standard library that pertain specifically to
these operations", so the guide does *not* say the library lives only in the
runtime. RFC-0000 was careful about exactly this — "the axis, not the cut" —
and an earlier version of this entry was not. What the guide supports is that
the three layers are the right axis; what justifies cutting on it is the
empirical result below. `cargo xtask guide` cross-checked that split against
the registration paths and found **96 agreements and 0 disagreements**.

The empirical result is what carries the rule: of B''s 129 additions, **every
one in `vm/` or `stack/` is a language feature and every misfiling is in
`bund/`** — the `string.distance.*` and `string.random.*` families, `fs.cp`,
`fs.mv`, `url`, `sysinfo.hostname`. The layer boundary is where the errors
stop, which is a measurement rather than a reading of the guide.

So completing a file is right in `vm/` and `stack/`, where a partly-used file
has meant a partly-used *language feature*, and wrong in `bund/`, where it has
meant a partly-used *library*.

**C is rejected** on two grounds: it takes 92.6% of the in-scope set as core,
which makes the distinction meaningless, and it completes by subsystem, which
Q4 already ruled a reporting aid rather than a decision axis.

The 68 words B″ adds over B+probes are the check on it: stack operations
(`drop_in`, `dup_many`, `swap_in`, `return_to`, `$`), the converters
(`convert.to_*`, `matrix`), the logic operators (`<=`, `>=`, `and`, `or`,
`?true`, `≠`, `⩽`, `⩾`), the constructors (`pair`, `lambda`, `match`, `λ`,
`∅`) and the list primitives (`cdr`, `head`, `tail`). Every one is a language
feature. And `math.ln` stays library, which is the tell — B″ is the widest
variant that still calls natural log a library word.

### Consequences

- **RFC-0002's criterion 2 has its number.** Only the core 286 is a
  preservation target; the library 211 is deferrable and re-implementable out
  of tree.
- **`bund2-api` is two surfaces**, which is what D14 gated: the core surface
  carries the stability guarantee, and a second is what out-of-tree word
  packages compile against.
- **The M6 denominator is 286**, not 497.
- `cargo xtask coverage` keeps reporting over the in-scope 497, because
  CLAUDE.md defines coverage that way and the library half still needs tests.
  The core figure is reported alongside it.

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
asymmetry F13 records for `dup`. Whether such readers exist was **D11**, now
**RESOLVED — none**, on the evidence that nothing reachable from Bund produces
a file in the format. So the further step is now *available*: encode "unset"
in the format so laziness survives serialisation entirely. It is still **not
adopted** — it diverges the wire format from the reference, which D20's own
first clause forbids, and RFC-0001 would have to take that deviation
explicitly. Recorded as available rather than taken.

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
  `cargo xtask coverage` — capture landed in `golden::capture_jobs`, so Q16 is closed
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

**This is a format change, and it is gated on D31** — not on D11, which asks
about the *object* format and is now RESOLVED on evidence that does not
transfer. D31 asks whether anything outside the project reads the **world
file**, which is a user-named SQLite database passed explicitly as
`bund load --world <path>` (`reference/Bund/src/cmd/mod.rs:318`). If an
external reader exists, switching backends breaks it; if not, the change is
free. D31 is OPEN, and it carries a mitigation — a one-way importer — that
makes the question moot rather than requiring it to be answered.

Unaffected: what goes *into* the world is still bincode-serialised `Value`s,
so D20's rule stands — serialisation materialises lazy identity, and the value
encoding is unchanged. Only the container changes.

- Decided by: repository owner
- Blocks: nothing; informs RFC-0003 and D10
- Depends on: D31
- Status: RESOLVED (conditional on D31)

**Dated note, 2026-09-11 — the condition is met.** D31 resolved "no": nothing
outside Bund reads a world file. D27 therefore holds without condition. This
unblocks `save.model` and `load.model`, which D51 deferred to this point. They
also need the conversion between `bund2_value::wire` and `BundValue` that F109
names.

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


### Narrowed on 2026-10-01 by D87 — the bus deferral was per-directory

This entry deferred `Bund/src/stdlib/functions/bus` whole, with the reason
"zenoh distributed bus — not essential". The directory is **mixed**:
`crossbus.rs` is a crossbeam in-process bus and `globals.rs` is the zenoh one.
D87 brings the eight local words into scope and narrows the deferral to
`globals.rs`, which also corrects the reason. The rest of this entry is
unchanged, and the principle — nothing non-essential linked unless asked for —
is what D87 relies on for keeping zenoh out.
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

**Addendum, after reading the Library Guide (Q17).** The four are not
symmetric. `stacks_left` has a full page in the guide — description,
algorithm, and a worked sample ending "// Now stacks are in order B C A" — and
is the only one of the 99 documented words that does not resolve to a callable
name. `dup_in`, `from_workbench` and `push_to` have no page at all. So the
reference documents `stacks_left` as part of the language and cannot execute
it, while the other three are undocumented as well as dead. That strengthens
the second option specifically; it does not decide between them.

- Blocks: RFC-0002's word set, and the M6 denominator alongside D14
- Default: none — the arguments cut both ways and the owner should pick
- Evidence: F19, F22, F23, F24; `cargo xtask corpus`, `cargo xtask coverage`
- Status: **RESOLVED — revive `stacks_left` alone.** Decided by the repository
  owner. `dup_in`, `from_workbench` and `push_to` are omitted.

`stacks_left` is the only one of the Library Guide's 99 documented words that
cannot be called: it carries a description, an algorithm and a worked sample
ending "// Now stacks are in order B C A". The reference documents it as part
of the language and cannot execute it. Two aliases, `<-` and `←`, point at it
and are dead because of it.

The other three have no guide page, no alias, no corpus use, and live inline
siblings covering the same ground — `dup_one`, `take`, `push`.

**Denominator effect.** `<-` and `←` are already counted, since they are
registered aliases; their target is not. So reviving `stacks_left` alone takes
the in-scope set from 497 to **498** and repairs two already-counted aliases
that today resolve nowhere. Reviving all four would have reached 501; omitting
all four would have dropped to 495 by removing the two aliases.

Not to be conflated with **F23**, which is `rotate_stack_right` calling the
left rotation — a different word and a different bug, unaffected by this.

## D30 — `valuemap` becomes readable: hash by content, and `get` mirrors `set`
F29 and F30 together left `valuemap` unimplementable and blocked RFC-0001.
Decided by the repository owner: **hash by content, plus mirroring `set`'s
pass-through in the `get` word.** Both halves are needed; neither works alone.

### The `get` mirror

`set` pulls all three operands, branches on the container's type
(`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:16`), and passes
the key through **as a `Value`** for a valuemap (`:18`), casting to string only
in the fallback (`:21-27`).

`get` does not. It casts the key to a string immediately
(`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:54`) and only
then pulls the container (`:60`), so it can never observe that the container
is a valuemap, and a non-string key fails before the container is examined.

Bund2's `get` pulls both, branches on the container's type, looks up by the
key `Value` for a valuemap, and otherwise casts to string and behaves exactly
as today. That is the same shape as `set`, which is why this is a mirror
rather than a new rule.

It also removes a second asymmetry: `set` accepts a non-string key on a
valuemap and `get` rejects one. After the mirror both accept it.

### Hash by content, mirroring equality

`Hash` today hashes the id alone (`reference/rust_dynamic/src/hash.rs:6`).
`PartialEq` compares **content** for four payload kinds — `I64`
(`reference/rust_dynamic/src/eq.rs:10`), `F64` (`:21`), `String` (`:32`),
`Time` (`:40`) — and **identity** for the other sixteen, through the catch-all
at `:45` and its fallback at `:53`.

So `BundValue::hash` mirrors `BundValue::eq`, kind by kind:

- content-compared kinds hash their content,
- identity-compared kinds hash their identity.

This is what makes `valuemap "k" 42 set "k" get` return `42`: the fresh `"k"`
hashes into the same bucket, and `String` equality then compares text.

Mirroring is also what keeps the `Hash`/`Eq` contract. Hashing *everything* by
content would satisfy the contract too, but it would compute an O(size) hash
for a list or map whose equality is then decided by identity — wasted work
that buys no lookup, since the entry still would not be found.

### What this does not do

**Composite keys stay identity-keyed.** A freshly built list equal to a stored
key still will not find it, because `eq` for lists is identity
(`reference/rust_dynamic/src/eq.rs:53`). Fixing that means changing equality
itself, which reaches far past `valuemap` — and is a much larger deviation
that this decision does not take. Scalar keys, which is what a valuemap is
useful for, work.

### Consequences

- **A deviation from the reference, and a deliberate one.** `Hash` changes
  and `get` gains a branch. It is unobservable *today* only because F29 leaves
  no read path at all; the point of the decision is to create one.
- **`tests/golden/probes/valuemap-hash-eq.golden` pins the broken behaviour**
  — `get` returning the whole map. When Bund2 implements this, that golden
  disagrees, and the disposition is the second of CLAUDE.md's three: an
  original-implementation bug. Regenerate with
  `cargo xtask golden --accept valuemap-hash-eq --reason F29`.
- **It makes D1 cheaper.** D1 lists hashing among the five needs that force a
  lazy identity to materialise. Under content-hashing a scalar key never
  needs one, so a valuemap lookup on scalar keys mints nothing. Composite
  keys still materialise.
- `?key` is **not** covered. `Value::has_key` omits `VALUEMAP` from its arm
  and answers false for everything else
  (`reference/rust_dynamic/src/has_key.rs:7,19-20`), and `set` has no
  `?key` counterpart to mirror. The same reasoning would fix it, but the
  decision as given names `get`. Carried as Q19.

### Amendment: the contract is made bidirectional (F33, Q20)

RFC-0001's second review found that content hashing cannot be implemented on
top of the reference's equality, because that equality is **asymmetric**:
`Val::I64` against `Val::F64` truncates the float
(`reference/rust_dynamic/src/eq.rs:13`) while `Val::F64` against `Val::I64`
widens the int (`:24`), so `42 == 42.5` and `42.5 == 42` disagree. `impl Eq`
asserts the symmetry it does not have (`:59-62`). No bucket assignment can be
consistent with an asymmetric equality.

Decided by the repository owner: **make the contract bidirectional.**

That instruction has exactly one consistent implementation, and the two
obvious readings are both ruled out by probe:

- **Truncate in both directions** — not transitive. `42 == 42.5` is true and
  `42 == 42.9` is true, but `42.5 == 42.9` is false. Confirmed against the
  oracle, `tests/probes/eq-asymmetry.bund`.
- **Widen in both directions** — not transitive either, above 2^53, and the
  operand orientation has to be named because the two disagree. With the
  **float on top** the receiver widens and the oracle answers **true**:
  widening `2^53+1` to `f64` loses the low bit, so two distinct integers
  compare equal to one float and therefore to each other. With the **int on
  top** the receiver truncates and it answers **false**. Both are now pinned
  in `tests/probes/eq-asymmetry.bund`; an earlier version of this amendment
  claimed both were pinned when only the truncating one was, and the probe's
  label had them the wrong way round.

So bidirectional forces **exact numeric comparison**: an integer and a float
are equal when they denote the same mathematical value, and not otherwise.

    i == f  ⟺  f is finite and integral, f is within i64 range, and f as i64 == i

That is symmetric by construction, transitive, and hashable — which is the
whole point, since D30 needs a bucket assignment. `42 == 42.0` is true,
`42 == 42.5` is false in both orientations, and `2^53+1` versus `2^53.0` is
false in both.

**Hashing follows from it**, which also settles the float-bearing arms the
review flagged:

- a float that is finite, integral and in range hashes **as the `i64` it
  denotes**, so `42` and `42.0` share a bucket;
- `-0.0` is normalised to `0.0` before hashing, since the two are equal and
  equal values must hash alike;
- `NaN` is never equal to anything including itself, so its hash is
  unconstrained; it takes a fixed value and simply never matches.

This is a **deviation from the reference in both directions**: `42 == 42.5`
changes from true to false, and `42.5 == 42` stays false. The reference's
behaviour is pinned in `tests/golden/probes/eq-asymmetry.golden`, which Bund2
will disagree with; the disposition is F33, an original-implementation bug.

- Blocks: RFC-0001 (now unblocked), F29, F30, F33
- Evidence: `tests/probes/valuemap-hash-eq.bund`,
  `tests/probes/eq-asymmetry.bund`, both confirmed against the oracle
- Status: **RESOLVED — hash mirrors equality; `get` mirrors `set`; equality is
  exact across int/float, and therefore symmetric, transitive and hashable.**


## D31 — external readers of the world file
Does anything outside the project read a Bund **world file**? Split from D11,
which asks the same shape of question about the *object* format and is
resolved on evidence that does not transfer.

The two artefacts are not comparable. The object format has no producer
reachable from Bund, so no file in it exists. The world file is the opposite:
it is created by `save.*`, it is a **user-named path** supplied on the command
line — `bund load --world <path>` and `bund wscript --world <path>`
(`reference/Bund/src/cmd/mod.rs:318,328`) — and it is a **SQLite** database
(`reference/Bund/Cargo.toml:131`) holding six tables of user state: `ALIASES`,
`BOOTSTRAP`, `LAMBDAS`, `MODELS`, `STACKS`, `STACK_DATA`. A user-named SQLite
file in a user's directory is exactly the artefact somebody opens with
`sqlite3`, and no obscurity argument is available for it.

- Blocks: D27, and therefore RFC-0002's criterion 7
- Default: none. **A negative about other people's files is not provable**,
  and adopting "no" by default is the failure mode this register exists to
  prevent.
- Status: **RESOLVED 2026-09-11 — no external readers** (see *Resolution*
  below; was OPEN)

**Recommended resolution: do not answer it — make it moot.** Ship a one-way
importer, `bund2 world import <sqlite-path>` producing a redb world file. The
original is never written, so any external reader keeps working on it, and a
migrating user keeps their lambdas and stacks. That converts an unprovable
negative into a bounded engineering cost.

The cost is a read-only SQLite dependency, and it must stay confined: the
importer is a **tool path, feature-gated off by default**, so D27's pure-Rust
rationale and D28's "only essential features in the default build" both
survive. `bund2` itself never links SQLite.

The alternative is to document the break. It is cheaper and defensible if the
world file is treated as regenerable — but `LAMBDAS` and `STACK_DATA` are user
state, regenerable only if the user still holds the source that produced them.

**The cheapest resolution is one only the repository owner can give.** If
nobody outside this project has run `bund load --world`, this closes as "no",
D27's condition discharges, and the importer becomes a convenience rather than
a mitigation. That is knowledge, not analysis, and it is the one input this
entry cannot gather for itself.

**Resolution, 2026-09-11 — no.** The repository owner confirms that nothing
outside Bund opens a world file, so switching the format breaks no reader.
D27's condition is met, and the world file is redb. The importer is no longer
a mitigation. It stays available as a convenience for a user migrating an old
world file, and nothing requires it.

- Decided by: repository owner, 2026-09-11
- Status: **RESOLVED — no external readers; D27 holds unconditionally.**

## D32 — `q` is reserved for fuzzy math, and its propagation is preserved
Stated by the repository owner: **`q` is the mechanism for a future "fuzzy
math" feature.** It is not vestigial, and it is not a rendering artefact.

That resolves a reading the evidence alone left open. Two RFC-0001 drafts
treated `q` as a constant to be rendered, and the register closed Q18 once on
a scan that never opened `q.rs`. The evidence that corrected them showed `q`
*varies* — `calc_q` averages it on every arithmetic operation
(`reference/rust_dynamic/src/q.rs:5`, reached from `impl Add` at
`reference/rust_dynamic/src/math.rs:416-420`), and `Value::none` starts at
`0.0` (`reference/rust_dynamic/src/create_special.rs:19-21`,
`value.rs:38`) where every other constructor starts at 100.0.

But "varies" was as far as the evidence went, and a defensible reading of a
varying field with no reader is *inert state to carry along*. The owner's
statement rules that out: the averaging **is** the feature, in embryo.

**What Bund2 preserves is therefore the propagation, not just the field.**

- `q` is a field on the header, which RFC-0001 already carries.
- Arithmetic averages it: `q(a ⊕ b) = (q(a) + q(b)) / 2`, matching `calc_q`.
- Constructors start at 100.0 and `none` at 0.0, so the 100.0 the goldens
  show everywhere is a **fixpoint**, not a default — an average of full
  confidence with full confidence.
- A word that consumes or reports `q` does not exist yet and is not invented
  here. D18's gap-filling does not extend to a feature the reference has not
  built.

The interpretation matters for RFC-0004 and RFC-0005: a field that merely
rides along can be dropped from a JIT fast path, and a field that carries a
propagating computation cannot.

### Amended 2026-09-10 — `q` is kept, not averaged (Q35)

Decided by the repository owner, on Q35's evidence. **The premise above was
grounded one call short.** `calc_q` has no caller anywhere in `rust_dynamic`
(`reference/rust_dynamic/src/q.rs:4-7`). `impl Add for Value` does average,
through `set_q` (`reference/rust_dynamic/src/math.rs:413-424`), but it is a Rust
operator overload no word uses: `+` reaches `math_op`, which calls
`Value::numeric_op` directly and pushes the result
(`reference/rust_multistackvm/src/stdlib/math/add.rs:6-8`,
`reference/rust_multistackvm/src/stdlib/math/math_op.rs:7-19`). Nothing in
`rust_multistackvm` or `Bund` calls `set_q` or `calc_q` at all. So the
reference's *language* never averages `q`, and no Bund program can observe it
doing so.

What Bund2 preserves is therefore:

- `q` as a field on every value, rendered where the reference renders it;
- constructors at 100.0 and `none` at 0.0;
- **no averaging by any word.** An arithmetic result is a fresh value at 100.0
  whatever its operands carried — which `crates/bund2-stdlib/src/math.rs`
  already did, and every golden shows.

The bullet "Arithmetic averages it" above is superseded, and with it the
"fixpoint" reading: the 100.0 the goldens show is the constructors' value, not
the stable point of an average. The reservation stands — `q` remains the
mechanism for a future fuzzy-math feature — but how `q` combines will be
designed when that feature is, as new behaviour, not inherited from an
operator overload the language never calls.

**Not a deviation.** Bund2 now matches the reference exactly where the
original reading would have made it diverge.

- Decided by: repository owner
- Blocks: nothing; constrains RFC-0001's `q` handling and any later fuzzy-math
  work
- Status: **RESOLVED — amended 2026-09-10: `q` is kept but not averaged
  (Q35).** Originally resolved as "preserve the propagation".

## D33 — does D30's exact numeric comparison extend to ordering?

D30's amendment made equality exact across int and float, because a valuemap
needs a bucket assignment and an asymmetric equality cannot have one. Ordering
was never in its path: hashing does not consult `<`.

So `==` is now exact and symmetric while `<`, `>`, `<=` and `>=` still answer
**true to all four at once** on an int against a float (F47). That leaves the
two halves of comparison disagreeing about what a number is.

### Options

1. **Preserve.** Ordering keeps the reference's answers; only equality
   deviates, and only where D30 said. Smallest deviation surface, and the
   goldens keep their meaning.
2. **Extend exactness to ordering.** `1 2.0 <` becomes true and the other
   three become false, by comparing the mathematical values as D30 defines
   them. Comparison becomes internally consistent — `a < b`, `a == b`,
   `a > b` exactly one of which holds — at the cost of a second deviation and
   the goldens that pin the current answers.
3. **Extend, and reject mixed kinds instead.** Bail as the gate already does
   for a string operand. Consistent, but it breaks programs that today get an
   answer, and the reference does admit the comparison.

### Recommendation

**Option 2.** D30's reasoning already applies: it chose exactness because the
alternatives were not valid relations, and neither truncation nor widening
gives a valid *order* either. Option 1 leaves `42 == 42.0` true while
`42 < 42.0` and `42 > 42.0` are also both true, which no program can reason
about. The deviation is the same one D30 already took, finished rather than
extended.

Against it: option 1 is free and this is not. Ordering across kinds may simply
be rare in the corpus, in which case the inconsistency costs little — that is
measurable and worth measuring before deciding.

### What is implemented meanwhile

**Option 1**, because preserving the reference is the default and needs no
decision. `crates/bund2-stdlib/src/logic.rs` pins all four mixed-kind answers
in a test, so whichever way this resolves the change is one edit and one test.

### Decision

Decided by the repository owner, 2026-09-11: **option 2 — exactness extends to
ordering.** An integer and a float order by the mathematical values they
denote, so exactly one of `a < b`, `a == b`, `a > b` holds. **In stack order**,
where the word compares the value beneath against the top: `2.0 1 <` is true
and `2.0 1 >`, `2.0 1 >=` are false, while the reference answers true to all
four. (`1 2 <` is false and `1 2 >` true, in Bund2 and in the oracle alike —
the operand order reads backwards from the infix spelling.) NaN orders against nothing, so all four answer
false where the reference answers true.

**The measurement this decision asked for.** The text above said the
inconsistency might cost little if mixed-kind ordering is rare, and that it was
worth measuring first. It was measured on 2026-09-11: across `examples/` and
`tests/probes/`, three files use an ordering operator at all —
`tests/probes/stack-loops.bund` (`dup 3 >`), `tests/probes/value-builders.bund`
(`1 2 <=`, `2 1 >=`) and `tests/probes/sqlite-conditional.bund`, whose `>` is
inside a PRQL string and never reaches Bund's word. **Every one compares an
integer against an integer.** So option 2's cost in goldens is zero, and the
argument against it — that option 1 is free and this is not — does not apply.

`numeric_ord` mirrors `numeric_eq`, and `exact_int_float_ord` spells out the
comparison (`crates/bund2-stdlib/src/logic.rs`): `i as f64` is lossy above 2^53
and `f as i64` saturates, so the exact route compares in `f64` only where the
integer converts exactly and by the float's floor above that.

This also unblocks RFC-0005: §S6 forbade lowering a mixed-kind comparison to a
machine compare while D33 stood, and said the lowering becomes available if
D33 resolved this way.

### Rejected

- **Option 1, preserve.** It leaves `42 == 42.0` true while `42 < 42.0` and
  `42 > 42.0` are both true, which no program can reason about.
- **Option 3, reject mixed kinds.** Internally consistent, but it refuses a
  comparison the reference answers, breaking programs that get an answer today.

- Decided by: repository owner, 2026-09-11
- Depends on: D30 (equality, resolved), F47 (the defect), F48 (there is
  currently no way to record the resulting golden disagreement as approved —
  which applies to D30's two existing deviations already, and applies to this
  one too)
- Status: **RESOLVED — ordering is exact; F47 carries the dated note.**

## D34 — does `( … )` keep hoisting out of an enclosing block?

F58: a `( … )` nested inside `{ … }` or `[ … ]` writes its CONTEXT marker and
inner terms to the top-level token stream, leaving only `endcontext` in the
block (`reference/bund_language_parser/src/vm/ctx.rs:8-20` against
`lambda.rs:11`, `list.rs:11`). Observable — `:F { ( 7 ) 9 } register` fails on
the oracle where `:F { 7 9 } register` succeeds.

RFC-0003's assigned improvement is "scoped blocks replacing the parser side
channel (F9)", which authorises changing the *representation*. It does not by
itself authorise changing what a nested `( … )` does, and an unplanned
deviation is a decision.

### Options

1. **Preserve the hoist.** Lowering emits the inner terms into the enclosing
   stream exactly as today. Bit-faithful, and it means a scoped AST node whose
   lowering is deliberately non-local — the side channel survives in the
   lowering even though it left the parser.
2. **Lower in place.** `( … )` becomes a context scope within its block.
   `:F { ( 7 ) 9 } register` starts working. Programs relying on the hoist
   change meaning; no corpus program is currently known to.
3. **Reject nested `( … )` at parse time.** Makes the unsupported case loud
   instead of silently wrong. Rejects programs the reference accepts, including
   any that hoist deliberately.

### Recommendation

**Option 2**, contingent on evidence. The hoist has no plausible intended
semantics — it produces a block that does not contain what it lexically
contains — and option 1 preserves a shape that would have to be described in
the RFC as deliberate. But the deciding evidence has not been gathered: no
corpus program has been checked for a nested `( … )`. That check is cheap and
should precede the decision.

### Evidence (gathered 2026-08-29)

**No corpus program nests `( … )` inside a block.** Exactly one of the 132
programs uses `( … )` at all —
`reference/Bund/examples/bund_dynamic_demos/create_lambda_on_the_fly_in_the_context.bund:10,20`
— and it is top level, wrapping the canonical metaprogramming idiom
(`call,` → `lambda*` → `register`) in a temporary context.

At top level the hoist is a no-op: `state` *is* the top-level stream, so
pushing into it and returning a node are indistinguishable. The reference's
behaviour and option 2 therefore agree on every program in the corpus, and no
golden can distinguish them.

That removes the risk from option 2 without deciding it: the question is no
longer "what breaks" but "what should a nested `( … )` mean", which is a
language question and the owner's.

### Decision

Decided by the repository owner: **option 2 — lower in place.** `( … )` is a
scope within the block that lexically contains it. Lowering emits the CONTEXT
marker, the inner terms and the `endcontext` call together, inside the
enclosing block, so the bracket is balanced wherever it appears.

Top-level `( … )` is unchanged, which is where every corpus use is, so no
golden moves.

**What the hoist actually was.** Grounding gathered for this decision shows it
is not an alternative scoping rule. `Value::context()` names a fresh anonymous
scratch stack (`reference/rust_dynamic/src/create_special.rs:22-33`); `apply`
switches to it through `to_stack`, which also pushes the name onto the runtime
`stacks_stack` (`reference/rust_multistackvm/src/multistackvm_to_stack.rs:5-19`);
`endcontext` carries the top value out to the workbench, drops the stack and
pops that nesting stack (`reference/rust_multistackvm/src/stdlib/ctx.rs:5-27`).
CONTEXT and `endcontext` are therefore a balanced pair over runtime state, and
the hoist separates the halves in *time*: the open runs when the enclosing
stream is evaluated, the close only when the block is called — never, once, or
many times.

Measured on the oracle with `{ ( 7 ) 9 } :F swap register`, then `111 222`
pushed and `F` called twice:

    before any call     7, 111, 222     the 7 escaped the parentheses
    after first call    9               7, 111 and 222 destroyed
    after second call   9               another stack dropped, silently

`111` and `222` were never inside the parentheses. Nothing errors.

**Why option 2 rather than 1 or 3.** Option 1 would mean specifying, as
intended behaviour, that a block does not contain what it lexically contains
and that a call may destroy its caller's stack; it also fights RFC-0003's frame
loop, where a context is naturally a frame with an exit action — the mechanism
that already fixes F57 — so reproducing the hoist would mean breaking frame
discipline deliberately. Option 3 (reject nested `( … )`) is strictly safer
than either and remains available as an interim, but it forecloses the corpus's
own idiom — bounding `lambda*` with a context — from being used inside a word
body, which is the one place it would be reusable.

### Consequences

- **F58 closes** with disposition "deliberate deviation, approved here."
- `:F { ( 7 ) 9 } register` starts working. No corpus program relies on the
  hoist; a program outside the corpus that did would be relying on its caller's
  stack being destroyed at an unpredictable time.
- **This fixes the parse half only.** `endcontext` remains unbalanced-callable
  by hand, and its guard cannot fire — F60, fixed separately.

- Blocks: RFC-0003 (D2's preservation claim) — now unblocked
- Status: **RESOLVED — lower in place. `( … )` is a scope in the block that
  contains it.**

## D35 — what does the compiled cache key on, given `dup` and D20?

RFC-0003 keys the compiled cache on the lambda body's **identity**, following
D5's "a cache keyed on identity simply does not contain the replacement". Two
things that D5 did not weigh make that untenable as stated.

**It is an unlisted materialisation point.** D20 enumerates where lazy identity
ends — `save.lambdas`/`save.stacks`/`save.aliases`, `compile`, and `wrap` — and
says materialising there is correct because "those write values meant to be read
by another process." **Executing a lambda is not on that list.** An
identity-keyed cache forces every lambda hot enough to promote to materialise an
identity it otherwise never needs, which is a new materialisation point on the
hottest path in the language.

**`dup` makes it miss.** F13's disposition is that Bund2 implements `dup` as a
structural clone **plus fresh identity**. `dup` is 55 invocations across 38 of
132 programs. So a lambda that reached the stack through `dup` has an identity
its original does not, and an identity-keyed cache treats the two as unrelated
bodies — it cannot hit for any dup'd lambda, in principle, forever.

D5's reasoning is not wrong; it is answering a different question. D5 asked
whether a cache can go **stale**, and identity keying cannot. It did not ask
whether the cache can **hit**.

### Options

1. **Identity keying, as D5's letter.** No invalidation, and no hits for dup'd
   lambdas. Adds a materialisation point D20 does not list, so D20 needs
   amending either way.
2. **Content keying with identity and stamp excluded from the hash.** Hits for
   dup'd and re-parsed bodies alike; still needs no invalidation, because a
   changed body is a new body with new content. Requires defining the hash to
   skip `id` and `stamp` — which D3's amendment already flagged as undecided —
   and pays an O(body) hash on first promotion.
3. **Key on the registered name plus a generation.** Sidesteps value identity
   entirely: the cache belongs to the word table, not to the value. Anonymous
   lambdas — the `{ … } if` argument form — become uncacheable, which is most
   of the corpus's block usage.

4. **The body's `Rc` pointer.** `BundValue::Heap(Rc<HeapValue>)`
   (`crates/bund2-value/src/lib.rs`) holds `payload: Rc<Payload>` (`:210`),
   and a lambda's payload *is* its body. Key on `Rc::as_ptr(&payload)`, with the
   cache holding a strong clone so the address cannot be reused while an entry
   refers to it.

### Decision

Decided by the repository owner: **option 4 — key on the body's `Rc` pointer.**

**The dup objection dissolves rather than being paid for.** `dup` gives the copy
a fresh header with a cleared identity but a **shared payload** —
`payload: Rc::clone(&h.payload)` (`crates/bund2-value/src/lib.rs`). So a
`dup`'d lambda and its original share one payload pointer and therefore one
cache entry. That is the objection F13 raised, answered by keying on the thing
that is actually shared rather than on the thing `dup` deliberately replaces.

**Nothing is materialised**, so D20's enumeration is untouched and needs no
amendment — which was the other half of what made option 1 untenable.

**D5's reasoning is preserved exactly, applied to the pointer instead of the
id.** Bodies are write-once, so a changed body is a new `Rc` and a stale entry
is unreachable; no invalidation machinery is needed. `Rc::make_mut` (D13) keeps
that true even if a body were ever mutated, since the clone-on-write produces a
new pointer.

| | 1 identity | 2 content | 4 pointer |
|---|---|---|---|
| materialises identity | yes | no | no |
| hits for `dup`'d bodies | no | yes | yes |
| key cost | O(1) | O(body) | O(1) |
| needs invalidation | no | no | no |
| D20 amendment required | yes | no | no |

**Evidence.** No corpus program `dup`s a lambda — 0 of the 55 `dup` invocations
across 38 of 132 programs — so the dup case was structural rather than observed
even before option 4 removed it. That measurement is why option 2's O(body) hash
is not worth paying today.

**Option 2 remains a strict upgrade** and is not foreclosed. Its advantages over
option 4 are that structurally identical lambdas share compiled code and that
re-parsed bodies hit; if a workload shows either, only the key function changes.
This entry supersedes its own earlier recommendation of option 2, which was made
before `dup`'s payload sharing was checked.

### Consequences

- **RFC-0003's S3 states the key** and is unblocked.
- **The cache must hold the strong `Rc`.** If an entry outlives its body, the
  allocator may reuse the address and a stale entry becomes a *false hit* —
  wrong code executed, which is the worst failure class available here. This is
  an invariant to test, not merely to document.
- **The cache pins bodies alive**, so RFC-0005's cap is load-bearing for heap as
  well as code memory. D3's amendment reached the same conclusion by a different
  route; RFC-0005 should state it once for both.
- **Eval'd code still does not hit**, under option 4 exactly as under option 1,
  which is what D3's amended resolution already relies on. The two stay
  consistent.

- Blocks: RFC-0003 (now unblocked), RFC-0005
- Depends on: D5 (write-once), D13 (`Rc::make_mut`), D20 (materialisation
  points, untouched by this), F13 (`dup` mints fresh identity but shares the
  payload)
**Citations corrected 2026-09-08 (F2).** The three line numbers into
`crates/bund2-value` in this entry — `:219`, `:159`, `:480` — had decayed to a
doc comment, `q: 100.0` and a blank line. They were never checked: `cargo xtask
cite` extracted only `reference/`-prefixed paths, so citations into live code
rotted invisibly while the tool reported zero defects. The claims are unchanged
and were re-verified against the current lines; only the numbers moved. See F2.

### Amended 2026-09-10 — the cache holds a `Weak` (Q32)

Decided by the repository owner, on Q32. **The key is unchanged**: the body's
`Rc` pointer. What changes is the strength of the cache's reference, and the
reason given for it.

The consequence above — "the cache must hold the strong `Rc`", because "the
allocator may reuse the address" — does not follow. An `Rc`'s allocation is
freed only when its strong **and** weak counts both reach zero, so a live
`Weak` keeps the address out of reuse as surely as a strong reference does
(RFC-0005 §S7). The strong reference was buying liveness, not safety.

D42 removes the need for that liveness. Every running frame now holds its own
clone of the body's `Rc`, so a body is alive whenever compiled code for it can
run, and compiled code is only ever entered by looking its body up. Nothing
needs the cache to keep a body alive.

- **The cache holds a `Weak`**, like RFC-0005's promotion counter. An entry
  whose `Weak` no longer upgrades is dead and is swept; it cannot answer for a
  different body, because its address cannot be reused while it lives.
- **"The cache pins bodies alive" is withdrawn.** RFC-0005's 1024-body cap
  bounds code memory, not heap.
- **"An invariant to test" stands**, retargeted: the test is that a dead entry
  answers for nothing, not that a strong count stays at one (RFC-0005
  criterion 3).

- Status: **RESOLVED — key on the body's `Rc` pointer. Amended 2026-09-10
  (Q32): the cache holds a `Weak`, not a strong reference.**

## D36 — error presentation: the reference's frame, a Bund location, and a reporter seam

Requested by the repository owner: deliver a precise reason and location, make
the stack dump optional, keep non-critical reports quiet, and leave hooks for a
future TUI.

### What is preserved

The **frame** and the **exit code**. An uncaught error prints a `comfy_table`
report to stdout, then `[BUND] Content of the stack` and the stack box, and the
process exits **0**
(`reference/Bund/src/stdlib/helpers/print_error.rs:104-132`;
`reference/Bund/src/stdlib/helpers/run_snippet.rs` sets no code). Every golden
capturing a failing program pins that, and changing it is a deviation nobody
asked for.

### What changes

**The `Location` row names a position in the Bund program.** The reference has
no source positions at all: it recovers a **Rust** file and line by regex from
the tail of the message (`print_error.rs:12-46`), which `easy_error`'s `bail!`
appended. On the capture machine that path runs through `~/.cargo/registry` and
names a crate version — F66, and unreproducible anywhere else. Bund2's parser
has spans, so the row names the `.bund` file, line and column, and a `Source`
row shows the line itself.

**The reason carries no location.** The reference concatenates the two and
recovers them by regex, which fails for any message ending in a parenthesis.
Here they are separate fields and nothing parses a message.

**The reason is the precise one.** The reference wraps it —
`Attempt to evaluate value Value { id: … } returned error: …` — because
interpolating the offending value is the only way it can say *where*. With a
real location that wrapper is noise, so the report shows the inner reason.
`Interp::eval` still produces the wrapped text for callers that want it.

**Delivery is proportional to severity.** `Error` gets the report; `Warning`
and `Notice` get one line on stderr with no table and no stack. `?error`'s
message is a `Notice` — the program asked for it to be said and is still
running.

**The stack dump is a switch**, `--dump-stack` / `--no-dump-stack`, default on
as the reference always dumps. It gates *collection*, not just display: the
evaluator asks `Reporter::wants_stack` before rendering every value on every
stack.

### The TUI seam

`bund2-api::diag` defines a structured `Diagnostic` — severity, reason,
location, optional stack and workbench snapshots, current stack name — and a
`Reporter` trait with one method. `TextReporter` is one implementation;
`CollectingReporter` and `SilentReporter` are two more. A TUI implements the
trait, receives values rather than text, and lays out the parts itself.

`Vm::report` is the seam at the language level: a word emits a diagnostic
without deciding how it looks, which is what `?error` now does instead of
printing.

### Consequences

- **Conformance is unchanged at 17/69, and the four F66 goldens still cannot
  pass.** Two of them capture the `Attempt to evaluate value …` wrapper and two
  capture a `~/.cargo` path; both are unreproducible for reasons predating this
  decision. F48 applies — `conform` still cannot record an approved deviation.
- **Positions are top-level only.** A failure inside a lambda body reports the
  top-level term that started it, because a `Vec<BundValue>` stream has nowhere
  to put a span for a nested value. Spans ride in a parallel vector so the
  stream itself is unchanged and `xtask parity` still compares like for like.
  Nested positions need RFC-0003 §S5's IR.
- **`Severity::Warning` has no producer yet.** The path exists and is tested;
  nothing in the current word set warns.

**Amended 2026-09-10 by D45.** `Reporter::wants_stack` now takes the
diagnostic's severity, and `TextReporter` wants a snapshot only for a fatal
report, which is the only kind it shows one under. "It gates *collection*"
above still holds, per severity.

- Status: **RESOLVED — frame preserved, location and delivery redesigned,
  reporting behind a trait.**

## D37 — Bund2 does not panic, and an impossible state is explained

Required by the repository owner: no `panic!()`, no `unwrap()`, no unhandled
exceptions; and every unrecoverable internal error handled with a meaningful
explanation.

### Why an interpreter in particular

A panic aborts the process. In a compiler that is survivable — the input is a
file and the user re-runs it. In an interpreter it takes the user's *program
state* with it: the stacks, the word table, anything a REPL session had built.
It also explains nothing useful, because the stack trace names Rust frames
inside `rust_multistackvm`-shaped code and no Bund word at all.

The reference does not assert either, and its habit is worth copying. It guards
a word's depth *and* writes a real failure arm for the pull that follows —
`SET returns: NO DATA #1`
(`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:30-42`) — for
exactly the pulls a Rust programmer would call unreachable.

### What was done

**Enforced, not asserted.** `[workspace.lints.clippy]` denies `unwrap_used`,
`expect_used`, `panic`, `unreachable`, `todo`, `unimplemented` and `exit`
across every crate including `xtask`. Tests opt out at each crate root, because
a panicking assertion in a test is the point.

Sixty sites were removed, in three kinds:

- **Made structural.** `BundValue::into_heap` returns the `Rc` directly, so the
  "promote always yields a heap value" invariant is a type rather than a
  comment with an `unreachable!` under it. Six sites disappeared with it. The
  parser's `take_into` does the same for `bump()`-after-`peek()`, removing
  ten more. `format_id` builds from `char`s, so there is no fallible UTF-8
  conversion left to explain.
- **Given the reference's own error.** Every `expect("depth checked")` became
  a real `NO DATA #n` arm, which is both safer and closer to the reference than
  the assertion was.
- **Reported as internal.** Where a broken invariant genuinely has no sensible
  continuation, `Error::internal` says which invariant, that it is a defect in
  Bund2 rather than in the program being run, and that it should be reported —
  and routes through the same diagnostic path as any other error, so it reaches
  the reporter a TUI installed rather than stderr.

### Consequences

- **`Error::is_internal` distinguishes the two audiences.** A reporter can
  present "your program is wrong" differently from "Bund2 is wrong"; nothing
  does yet, and the seam exists.
- **A few behaviours changed in unreachable cases**, deliberately and in the
  safe direction: `current_mut` creates a missing current stack rather than
  asserting one, stack rotation uses `rotate_left` so there is no `Option`, and
  `numeric_ord` answers `false` for an ordering question equality never asks.
  None is reachable from any program.
- **Verified by planting one.** A deliberate `unwrap()` added to
  `bund2-value` was rejected by the lint, so the rule is enforced by the build
  and not by review.

- Status: **RESOLVED — enforced workspace-wide; internal errors carry an
  explanation and an audience.**

**Dated note, 2026-09-12 — one stated exception, and where it stops.** This
decision is about code Bund2 writes and values Bund2 builds, and it stays
absolute there: F85, F114, F115, F116, F117, F118 and F119 were each fixed
rather than excused. **Bytes read from a file Bund2 did not write are outside
it**, by D58. `sqlite` decodes a BLOB through bincode's derived
`Deserialize`, which builds the whole nested value before any Bund2 code runs,
so no depth check can refuse one — a 174 KB BLOB nested 3,000 deep aborts the
process (F118's dated note, measured 2026-09-12). D58 refuses a BLOB over
16 KiB so the common case reports, and names the residue rather than leaving
it to be rediscovered. Nothing else is excepted: if another path ever decodes
third-party bytes, it inherits this exception only by being recorded here.

## D38 — third-party crates the reference uses: pin, do not vendor

When the reference reaches a crate whose *rendered output or ordering* a golden
captures, Bund2 uses the same crate rather than reimplementing it. That rule
was already in force for `comfy-table` and `leon`; `dtoa` joined it when
`2.0 math.sqrt` turned out to print `1.4142135623730952` in the reference and
`…951` under Rust's `{:?}` from identical bits.

`algos` forced the question of *how* to take such a dependency, because it is
the first one the reference declares as **git**:

```toml reference/Bund/Cargo.toml:78
algos = { version="0.6.*", git="https://github.com/Brad-Edwards/algos.git" }
```

No `rev`, no `tag`. The oracle builds against whatever the branch head is, so
the reference's own sort order is a function of when it was compiled.

### The decision

**Pin a rev; do not vendor.**

```toml
algos = { git = "https://github.com/Brad-Edwards/algos.git", rev = "4c08437" }
```

`4c08437` is v0.6.8, the checkout the built oracle links, so Bund2 is pinned
*more* tightly than the reference is.

Vendoring was considered and rejected on size and on D37, not on licensing:
the crate is BSD 3-Clause, so copying it is permitted with attribution. It is
201 files and ~51,000 lines, of which four entry points are used, and
`[workspace.lints.clippy]` denies `unwrap`/`expect`/`panic` in every workspace
member. Vendored code becomes a workspace member, so it would need blanket
`allow`s — a hole in D37 in the least-reviewed code in the tree.
`cs/graph/dijkstra.rs` alone has two such calls outside its tests.

### What this does not fix

**D37 is weakened either way.** A panic inside `dijkstra` aborts the process
whether the code is a dependency or vendored; lint scope changes who *may* fix
it, not whether it can happen. If one is ever observed, the response is to
vendor **that module only** — about 400 lines, BSD notice retained, each panic
path rewritten to D37 — rather than the crate.

### What was reimplemented instead, and why that is not a contradiction

`sort` does **not** take this dependency. `algos`' quicksort is ~90 non-test
lines, self-contained, and free of `unwrap`/`expect`, so it is transcribed into
`crates/bund2-stdlib/src/sort.rs` and verified against the oracle over ten
cases including heavy ties and strings. The rule is "pin what you cannot
faithfully reproduce", not "depend by default": a 90-line unstable sort can be
reproduced exactly and checked, and reproducing it removes a runtime
dependency from the hottest correctness path. The graph family cannot be
reproduced that cheaply, and that is where the pin earns its place.

- Decided by: repository owner
- Blocks: `graph!` and the graph family; `listop`'s `fibonacci` search
- Status: **RESOLVED — pin `rev = "4c08437"`; vendor only a module, only if D37
  forces it.**

## D39 — an internal loop must be bounded; a program's loop may not be

Two questions arrived together — "fix hangs that are internal, not caused by
user code" and "can we detect user error and at least warn" — and they have
different answers, so the boundary is drawn once here.

### Internal loops are bounded, and the shape is checked

A loop Bund2 runs on its own behalf must terminate on data it has already
taken. F70 is what the alternative looks like: the reference's `move` drains
the current stack in a loop that pushes as it pulls, and pushing to a stack
that does not exist yet **creates** it — which makes it current, because the
current stack is the back of the deque
(`reference/rust_multistack/src/ts_current.rs:7`). The loop then pulls back
what it just pushed. `1 2 3 :box move` never returns.

Every drain in `crates/` collects first and pushes afterwards, so none can feed
itself, and `cargo xtask lint` reports a `while let Some(..) = vm.pull…` loop
whose body pushes. The check was verified by reintroducing the hang and
watching the lint fail.

Two loops were tightened rather than left correct-by-argument:

- `Stacks::to_stack`'s rotation spun `while current_name() != name`. A
  membership test ten lines above guarantees the name is present, so it did
  terminate — but that is the same reasoning F70 punishes, and it is now
  bounded by the deque's length. The reference bounds the same rotation by
  counting a full circle and failing (`ts_to_current.rs:24-26`).
- `Registry::follow` already stopped after 64 links, because D16 lets a program
  build an alias cycle.

**A hang is worse than a panic**, which is why this earns a rule of its own
next to D37. A panic at least produces a trace and a non-zero exit; a hang
cannot be reported, cannot be caught by `?try`, and cannot be told apart from a
slow program.

### A program's loop is not ours to stop — but it can be reported

`true { } while` runs forever, and so it should: that is Turing-completeness,
and the reference spins too. Refusing it would change what the language means.

So `Warning` gets its first producers — D36 defined the severity and noted
nothing yet emitted one:

- **`while` at 10,000,000 iterations.** One line on stderr, emitted **once**,
  saying what the body must do to terminate. It does not stop the loop and does
  not change the result.
- **`alias` closing a cycle.** The binding is performed exactly as the
  reference performs it (`multistackvm_alias.rs:5-15` has no check), and the
  warning names the circle. Bund2 cannot spin on it — `follow` is guarded — but
  it would otherwise resolve to whichever link the guard stopped on, which is a
  silent wrong answer.

Both thresholds are chosen so no corpus program reaches them, and `conform`
folds stderr into the captured output, so a golden fails immediately if one
ever does. That is the safety net for a heuristic: it cannot fire unnoticed.

### What is deliberately not attempted

Static detection of non-termination. It is undecidable in general, and the
useful approximation — a loop whose condition cannot change — is RFC-0004's
`bund2 check` working over an abstract stack, not a run-time guess.

- Decided by: repository owner, in response to F70
- Blocks: nothing; extends D36's severity ladder and sits beside D37
- Status: **RESOLVED — internal loops bounded and linted; program loops warned,
  never stopped.**

**Dated note, 2026-09-11 — a third shape: recursion on a program's data.**
This decision drew the line between a loop Bund2 runs on its own behalf and a
loop the program wrote. F114, F115 and F116 are neither: they are Rust
recursion whose depth is a *value the program built*, and each ended the
process, which D37 forbids.

Two were removed rather than bounded, because nothing is lost by walking the
same work on the heap: executing a nested container (F114) and dropping a
nested value (F115) now drive worklists. The parser could not be, since a
recursive descent is the parse, so F116 takes **an explicit bound**:
`bund2_syntax::MAX_NESTING`, 1024 blocks, refused before the frame is entered
and reported as an ordinary parse error.

The threshold is chosen as this decision's others are — `while`'s 10,000,000,
`follow`'s 64 — far past any real program and far below where it breaks. The
deepest nesting anywhere in the corpus is 2; a debug build aborted between
2,000 and 4,000 levels and a release build between 12,000 and 16,000. The
safety net is the same one: `conform` folds stderr into captured output, so a
golden would fail immediately if one ever reached the bound.

The rule this adds: **no Rust recursion may be driven by the depth of a value
a program controls.** Where the work can be moved to the heap, move it; where
it cannot, bound it and report.

*(Dated note, 2026-09-12: the rule's phrasing — "a value a program controls" —
exempts depth Bund2 **reads**, which is where F118's live half turned out to
sit. A BLOB in someone else's SQLite file is not a value this program built,
and decoding one aborted the process. D58 answers that case: where Bund2
controls neither the file nor the value, there is no side to bound, so the
check is on what can be measured before the decoder runs — the byte length —
and the residue is a stated exception to D37 rather than a defect.)*

**Dated note, 2026-09-12 — the shape had six instances, not three.** The note
above named F114, F115 and F116. RFC-0005's eighteenth and nineteenth reviews
found three more of the same kind, each measured before it was believed:
**F117**, rendering a value through `debug.display_stack` (aborted at 24,000
levels); **F119**, the second renderer `display`, which `println` reaches
(28,000); and **F118**, the bincode wire codec, which `save.model` reaches
(3,400 — the shallowest of the six by an order of magnitude).

F117 and F119 took the worklist, as F114 and F115 had. F118 took it as far as
Bund2 owns the code: encoding and decoding now build from an arena and
`WireValue` has an iterative `Drop`, which moved `save.model`'s ceiling from
3,400 to ~39,000. **The rest is not ours to move** — bincode's derived
`Serialize` and `Deserialize` recurse inside the dependency, and decoding caps
at ~5,500 levels on an 8 MiB thread. That is the first instance of this shape
where "move it to the heap" runs out, and the choice between bounding the
codec and recording the ceiling is still open.

Two lessons the six share, worth stating once. **Each was found by measuring,
not by reading** — three of them falsified a sentence written the day before
asserting the path was safe. And **the audits do not reach this class**: a
native that re-enters no evaluation is outside the re-entry scan, and one that
acts on the host is outside criterion 28's palette, which is where F118 lived.

**Dated note, 2026-09-12 (second) — where "move it to the heap" runs out.**
F118 is the first instance of this shape whose recursion is not Bund2's to
move: bincode's derived `Serialize` and `Deserialize` walk the nested
`WireValue`, and a decode cannot be checked at all, since the tree is built
before any Bund2 code runs. So F118 took **both** halves of this decision's
rule — the arena for the part Bund2 owns, which moved `save.model`'s ceiling
from 3,400 to ~39,000, and a bound for the part it does not:
`bund2_value::wire::MAX_WIRE_DEPTH`, 256, refusing to *write* a value that
could not be read back.

That adds a third clause to the rule. Where the work can be moved to the heap,
move it; where it cannot, bound it and report — and **where the recursion is a
dependency's, bound the side you control, and say which side that is.** A
bound on writing is not a bound on reading: a blob from another writer deeper
than 256 still aborts, and only D31's ruling that nothing outside Bund touches
a world file makes that acceptable.

## D40 — `string.grok` brings a C dependency, and that reaches the AOT milestone

D38 settled *how* to take a crate the reference reaches (pin it, do not vendor
it) for crates whose output a golden captures. `string.grok` satisfies that
rule and adds a second question D38 did not face: the crate is not pure Rust.

```toml reference/Bund/Cargo.toml:62
grok = "2.0.0"
```

`grok 2.4.1` depends on `onig`, which depends on `onig_sys`, which **compiles
Oniguruma from C sources at build time**. Nothing else in the workspace does.

### Why not reimplement it on `fancy_regex`

Because a grok pattern is a regex after expansion, and the two engines are not
the same language. Oniguruma and `fancy_regex` differ on backreference
semantics, on some character-class shorthands, and on what a malformed pattern
does. `string.grok`'s answer *is* Oniguruma's answer — the same argument that
made `dtoa` non-negotiable in D38, one layer up: a near-miss engine produces a
MAP that is right on the easy patterns and silently different on the rest,
which is worse than not having the word.

### The decision

**Take the dependency, and state its cost where the cost lands.**

- Bund2's build now requires a **C compiler**, on every target, for one word.
  That was previously not true.
- The **AOT milestone inherits it.** Cross-compiling a `cranelift-object`
  build to a target without a working C cross-toolchain will fail on
  `onig_sys` before it reaches any Bund2 code, and the failure will name a
  crate no one was thinking about.

The exit, if that cost is ever refused, is a **feature flag** rather than a
reimplementation: `string.grok` is one word in the library half, and D14 makes
the library half deferrable by design. Dropping it costs one word and one
probe stanza. Reimplementing it costs correctness that nothing would measure.

### Amendment, 2026-09-07 — the flag is taken, not merely named

The paragraph above named a feature flag as "the exit, if that cost is ever
refused", and then shipped the dependency on by default. That was wrong on a
point the entry itself did not notice: **D10 was OPEN**, and its default —
"yes, `cc`" — is about `bund2 build`, not about every `cargo build` of the
workspace. Taking it for the interpreter is a strictly stronger claim than the
default authorises, and CLAUDE.md forbids adopting an OPEN decision's default
at all, let alone a widened one.

The inversion is the substance, not the procedure. `--emit=bundle` exists so a
target with no C toolchain can still run Bund; a default-on `grok` means
building the runtime for that target needs a C toolchain. The escape hatch
stops being an escape.

So:

- `grok` is **`optional = true`** with a `grok` feature, **off by default**.
  `cargo tree -e normal,build` on the default build now matches no `onig` and
  no `cc`.
- `string.grok` and `string.grok.` leave the default registry. `coverage`
  measures the default build and therefore does not count them, which is the
  truth about what ships.
- The probe moves to `tests/probes/features/`, which `collect_probes` does not
  reach because it reads `tests/probes` non-recursively
  (`xtask/src/golden/mod.rs`). A probe for a word the default build
  does not bind would fail the default build.

**The flag is a holding position, not the end state.** D14 already calls the
library half "re-implementable as out-of-tree word packages", and a word that
changes the *build's* toolchain requirements is exactly what belongs outside
the core. `string.grok` moves out of tree once RFC-0002's external package
loading exists; until then the feature flag stands in for it.

- Decided by: this session, on the evidence that grok's contents matched the
  oracle exactly on the first run and only the MAP order differed (F76);
  **amended by the repository owner**, 2026-09-07, who took option A
- Blocks: nothing today; the AOT milestone reads D10, now RESOLVED
- Status: **RESOLVED — dependency taken but feature-gated off; the default
  build requires no C compiler, and out-of-tree is the end state.**

## D41 — the stack tag rides in the value's padding for scalars (Q29)

Q25 measured the stack tag at 60–75% of every word. Options 1 and 2 of
RFC-0001's amendment took `value/push_pull/balanced` from **126.9 ns to
52.0 ns** without changing any layout. Of the remaining 52, **43.7 is
`with_tag` and 25.1 of that is `promote/scalar` — boxing, with no tag written
at all**.

That is the floor, and it is measured rather than argued: **a scalar has no
header, a tag needs one, and building one costs an allocation.** RFC-0005 §S1
asks for under 20 ns, and no amount of tuning the tag reaches it while the tag
lives inside a header.

### The decision

**A scalar carries its stack tag inline, as an interned symbol in the padding
the enum already has.**

```rust
pub struct StackSym(u32);   // 0 == never pushed

pub enum BundValue {
    Int(i64, StackSym),
    Float(f64, StackSym),
    Bool(bool, StackSym),
    Nodata(StackSym),
    None(StackSym),
    Heap(Rc<HeapValue>),    // keeps its tag in the map, as today
}
```

**It is free.** Measured: `BundValue` is 16 bytes today and 16 bytes with a
`u32` on every scalar variant, because `Int(i64)` already pads 9 bytes to 16.
A `u16` is also 16, so the width is chosen for headroom, not size.

### The rules that make it correct

1. **The symbol is excluded from equality, hashing and ordering.** It is
   metadata about where a value has been, not part of what it is — the
   repository owner's framing, and the reason this is a *secondary* attribute.
   `PartialEq`, `Hash` and `Ord` are all hand-written here, so every site is a
   compile error rather than a silent inclusion.
2. **`promote` translates.** Boxing a tagged scalar must move the symbol into
   the header's `tags` map, or a value that acquires an `attr` would lose its
   stack tag. This is the one rule whose violation is silent.
3. **`tags()` synthesises.** A tagged scalar reports `{"stack": <name>}`, so
   the render path, the wire format and the 39 goldens see no difference.
4. **A thread-local interner resolves symbol → name**, in `bund2-value`.
   `BundValue` is neither `Send` nor `Sync`, so a thread-local is the right
   shape and no lock is involved.
5. **The `tag` word still writes the map.** A program-set key forces boxing, as
   any non-stack tag does; a later push writes the symbol on the boxed value's
   map, which is the reference's "push wins" ordering
   (`reference/rust_multistackvm/src/stdlib/values/value_tag.rs:49-50`).

### What was rejected, and why it is recorded

- **Option 3, an inline payload slot.** Measured at ~41 ns and **+24 bytes on
  every heap value** (`HeapValue` 88 → 112), including strings and lists that
  gain nothing. It buys less than A and costs memory A does not.
- **Option B, the tag out of the value entirely**, materialised where a value
  escapes into a container or is rendered. The most principled reading of
  "secondary attribute", and rejected on failure mode rather than on principle:
  its correctness rests on enumerating every escape site, the compiler cannot
  check that enumeration, and a miss prints `tags: {}` in a golden rather than
  failing to build. A's churn is large but every site of it is a type error.

### Consequences

- **~125 construction sites and ~102 pattern sites** change. All are
  compiler-enforced; none is a judgement call.
- **RFC-0001's stated layout is unchanged at 16 bytes**, so its headline
  survives. The amendment's outcome section records the measurement chain.
- RFC-0005 §S1's gate becomes reachable for the first time: boxing leaves the
  push path entirely for scalars.

- Decided by: repository owner, 2026-09-08, choosing A from the four options in
  RFC-0001's Q25 amendment after the boxing experiment
  (`crates/bund2-bench/benches/boxing.rs`) measured the floor
- Blocks: nothing; unblocks RFC-0005 §S1's precondition
- Depends on: D1 (lazy identity — a scalar has none, and gaining a symbol must
  not change that), D13 (the CoW split policy, which the symbol does not
  participate in), D30 (equality is content for scalars — the symbol must not
  enter it)
### Implemented, 2026-09-08

Landed, and **the gate it existed to unblock is met.**

| | baseline | opt 1 | opt 2 | **D41** |
|---|---|---|---|---|
| `value/push_pull/balanced` | 126.9 ns | 96.7 ns | 52.0 ns | **10.1 ns** |
| `dispatch/literal_push` per word | 94 ns | 64 ns | 45 ns | **24.5 ns** |
| `dispatch/native_call` per word | 112 ns | 88 ns | 58 ns | **28.8 ns** |
| `dispatch/dup_drop` per word | 122 ns | 92 ns | 59 ns | **44.9 ns** |

**A 12.6× improvement on the push/pull round trip**, and `BundValue` still
measures **16 bytes, 8-aligned** — the prediction that it was free in padding
held. Conformance stayed at **73/86 at every step**, with all 39 tag-bearing
goldens green.

RFC-0005 §S1 asked for `value/push_pull/balanced` under 20 ns. It is 10.1.
**The value layer is now ~10 ns of a ~25 ns word, so dispatch is the dominant
term** — which is the second half of that gate, and the first time the study's
§2.2 condition has been satisfied.

**Corrected 2026-09-10: the paragraph above overreaches, and RFC-0005 §S1
withdraws it.** These benchmarks cannot separate dispatching a word from the
work the word does once dispatched. `dispatch/literal_only/w1000` is the only
path with no dispatch, and subtracting it bounds dispatch from above — at most
33.7 ns per word — without saying how much of that bound *is* dispatch. So the
gate's second half is **not settled**; RFC-0005 criterion 10 is the experiment
that settles it. Two smaller corrections: 10.1 ns is this entry's run, and
RFC-0005 quotes 9.8 ns for the same benchmark from another; and the three
`dispatch/*` rows in the table above name benchmark families, whose full IDs
end `/w2000`, `/w4000` and `/w3000`. Raised by RFC-0005's fifth review (S9).

**One test inverted, which is the tell that this worked.**
`push_tags_with_the_current_stack` asserted `v.is_boxed()` — "tagging a scalar
boxes it" — and had done so, correctly, since the interpreter was written. It
now asserts the negation. A second test was added for rule 2, the one no
compiler catches: boxing a tagged scalar must carry the symbol into the map.

The mechanical churn was as estimated and entirely compiler-driven. Routing
constructions through `BundValue::int`/`float`/`boolean`/`nodata`/`none` was
deliberate: a scripted rewrite that left `StackSym::NONE` in *pattern* position
would have compiled and matched only untagged values — a silent wrong answer —
whereas a function call is a hard error in a pattern.

- Status: **RESOLVED — inline interned symbol on the scalar variants.
  Implemented 2026-09-08; push/pull 126.9 -> 10.1 ns, value still 16 bytes,
  conformance unmoved.**

## D42 — a body's `Rc` reaches the point where it starts running (Q36)

RFC-0005's compiled cache and promotion counter key on the body's `Rc` (D35),
and Bund2 discarded it before any body ran. `Vm::eval_body` took
`&[BundValue]`, `Vm::tail_call` took `Vec<BundValue>`, RFC-0003's `Frame` held
a copied `Vec<BundValue>`, and `times` copied the body out of its lambda once
per call. Only `dispatch`'s named-lambda arm held the `Rc`, one line before
`.to_vec()`. The reference's `times` passes `lambda_val.clone()` to
`lambda_eval` on every iteration
(`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:20`), so its body
arrives as the same value each time.

### Decision

Decided by the repository owner: **Q36's option A.**

- `Vm::eval_body(&[BundValue])` becomes **`Vm::eval_lambda(&BundValue)`**, and
  `Vm::tail_call(Vec<BundValue>)` becomes **`Vm::tail_lambda(BundValue)`**.
  Both take the LAMBDA value, so its `Rc` survives to the entry.
- RFC-0003's **`Frame` holds the body's value** and an instruction pointer,
  not a copied `Vec`.
- `Vm::scoped_call` keeps its `Vec`: its body is assembled per call from three
  slots, so it never had a key to keep. The frame wraps it as a LIST value.

### Consequences

- **A body is alive whenever it is running**, because its frame holds a clone
  of its `Rc`. That is what lets D35's amendment (Q32) move the cache to a
  `Weak`.
- **A body copy per call is gone** from every loop word, conditional and
  method path — the eighteen stdlib call sites that used the two methods.
- **Amends RFC-0002** (the `Vm` trait) **and RFC-0003** (the frame), both
  Accepted; each carries a dated amendment.

### Implemented, 2026-09-10

Landed the same day. `Vm::eval_lambda` and `Vm::tail_lambda` replace the two
methods, the frame holds the body's value and reads each item through it, and
`push_frame` is the one place a body starts running. `conditional.rs` splits
its slot reader in two: `slot_body` hands out the value, for running, and
`slot_items` hands out the items for `context`, which assembles a fresh body per
call. The eighteen stdlib call sites and the CLI's `observe` pass the value
they already held.

**It shows one key where there was none.** `times_enters_one_body_under_one_key`
runs `100 { drop } times` with Tier 0's `entry_log` seam on, and sees 100
entries under a single key (`crates/bund2-stdlib/src/seq.rs`). The key is
`payload_key`, D35's `Rc` pointer (`crates/bund2-value/src/lib.rs`).

**Tier 0 got slightly faster, not slower.** Reading through a held value costs
less than the per-call body copy it replaced. Criterion, `--baseline pre-q36`,
taken immediately before the change on the same machine:

| benchmark | before | after | Criterion's verdict |
|---|---|---|---|
| `dispatch/dup_drop/w3000` | 96.5 µs | 92.8 µs | −2.5%, improved (p = 0.00) |
| `dispatch/native_call/w4000` | 105.6 µs | 102.5 µs | −2.8%, improved (p = 0.00) |
| `dispatch/literal_push/w2000` | 47.8 µs | 47.1 µs | −1.2%, within noise |
| `dispatch/literal_only/w1000` | 14.1 µs | 14.1 µs | no change (p = 0.81) |
| `fragment/int_add/tier0` | 60.4 µs | 59.8 µs | −1.8%, improved (p = 0.00) |
| `fragment/dup_drop/tier0` | 88.5 µs | 87.8 µs | no change (p = 0.28) |

`literal_only` runs no body at all, and it is the one row that did not move.

Conformance held at 79/86, ceiling 79/86; `cargo xtask depth` still completes
a 100,000-deep call; 307 tests pass.

- Decided by: repository owner, 2026-09-10, choosing A from Q36's three options
- Blocks: nothing; unblocks RFC-0005 §S3, §S5's loop argument, §S7's counter
  and criterion 20
- Depends on: D35 (the key), D5 (bodies are write-once, so holding the value
  never observes a change)
- Status: **RESOLVED — the value reaches the entry point.**

## D43 — the `bund2-api` additions RFC-0005's guards need (Q37)

RFC-0005 §S6's meaning guard needs two things `bund2-api` does not have.

### Decision

Decided by the repository owner: **Q37's options A and A**, as one RFC-0002
amendment.

1. **A registration id on `Native`**, assigned by `Registry::register_native`
   from a counter. The JIT inlines a fragment only when the slot holds
   `bund2-stdlib`'s own registration, which a name cannot show and a function
   address cannot either (`std::ptr::fn_addr_eq`). Ids are per `Registry`, and
   F32's replay gives a re-registration a fresh one, so `bund2-stdlib`'s
   `(registration id, Fragment)` table is built at registration time.
2. **Stable per-name generation cells**: a mirror of each registry `Slot`'s
   generation, written by the same `touch()`, in fixed-size chunks that never
   move, handed out by a `Registry` accessor. *(Dated note, 2026-09-12: "the
   same `touch()`" could not be built as written — `Slot::touch` took `&mut
   self` alone, with no `Symbol` and no `Registry`, so it could not reach the
   cell. RFC-0005's twentieth review found it. `touch` now lives on `Registry`
   as `touch(&mut self, s: Symbol)`, the one function that bumps a generation,
   and `Slot::bump` is private to it; the mirror write belongs there. RFC-0005's
   assumption 37 states the property and
   `every_writer_of_a_slot_generation_is_named` derives it.)* `slots: Vec<Slot>` reallocates on
   a new name, so nothing may point into it.

Rejected: comparing function addresses (a guarantee Rust does not make);
registering fragments with natives (a `bund2-ir` type on the stable surface,
and fragments from external packages, which D9's amendment rules out); stable
slots themselves (a pointer hop on every Tier 0 dispatch); a helper call per
guarded site (the cost the guard exists to avoid).

**When:** implemented when RFC-0005 reaches Proposed. Nothing reads either
until a lowering exists, and surface added with no consumer is surface
designed without one.

- Decided by: repository owner, 2026-09-10
- Blocks: RFC-0005's meaning guard (§S6) and criterion 17
- Depends on: D9 as amended (no Cranelift type in `bund2-api`), F32 (replayed
  registrations)
- Status: **RESOLVED — decided 2026-09-10, built 2026-09-12.**

*Dated note, 2026-09-12 — built, on D59's authorisation.* Both additions are in
`crates/bund2-api/src/lib.rs`. `RegistrationId` is an opaque per-`Registry`
counter value carried as `Native::id`: `register_native` mints one and
`register_command` leaves `None`, which is RFC-0005 §S5's rule that a command
carries no D43 id rather than an omission. `Registry::generation_cell` hands
out one `Cell<u32>` per symbol from fixed-size boxed chunks of 256, so growing
the `Vec` of chunks never moves a cell already handed out — the property
`slots: Vec<Slot>` cannot offer. `Registry::touch` writes the mirror beside the
bump and stays the only function that moves a generation, which
`every_writer_of_a_slot_generation_is_named` derives.

Two consequences worth recording. The cells **deep-copy** with `Registry`'s
`Clone`, which the three cloning callers need — the CLI's per-word observer,
`check`'s `prebind` and the effect palette's template each want an independent
registry — so no two registries share a cell and no guard can observe a
foreign registry's rewrites. And an address therefore does not outlive the
registry it came from: `vm.registry = other.clone()` replaces the cells
wholesale.

**Not built, deliberately.** §S5 also wants `register_all` to record the ids it
mints *in the `Registry`*, for D47 and D48's crossing set. That is consumer
surface for `bund2-jit`, which is still a twelve-line placeholder, and this
entry's own caution is that surface added with no consumer is surface designed
without one.

## D44 — the level at which evaluation reports stack exhaustion is not part of a program's meaning

RFC-0005's seventh review, B1. F85's fix gives Tier 0 a floor on the machine
stack: below it, a native that would re-enter evaluation returns
`machine stack exhausted` instead of nesting until the process aborts
(RFC-0005 §S8). How many Bund levels fit above that floor depends on the size
of the Rust frames each level spends, so it depends on the build.

RFC-0005 promised two things about it that no fixed floor can both give: that
Tier 0's capacity with the tier on is *never less* than with it off, and that
the floor fires at *the same* level either way. Enlarging the stack by Tier 1's
share moves the Tier 0 floor down by that share, which gives "more". Pinning
the floor instead makes compiled frames come out of Tier 0's part. The stack is
LIFO, so a floor says where compiled frames may *start* and reserves no region
for them.

### Decision

Decided by the repository owner, 2026-09-10: **not meaning.** The level at
which evaluation reports `machine stack exhausted` may differ between builds,
profiles, platforms, and with Tier 1 on or off, and so may the number of side
effects a program performs before it is reported. What *is* required:

1. **Evaluation nesting never aborts the process.** It completes, or it returns
   a Bund-level error that is reported and that `?try` can catch (D37).
2. **With Tier 1 on, Tier 0's capacity is never less than with it off.** A
   program whose native nesting fits under the default binary fits under the
   `jit` binary.

### Evidence

It already differs between Bund2's own builds. The `loop` axis of
`cargo xtask depth` (100,000 levels through `times`) reports at **level
10,923 in release** and at **level 2,371 in the dev profile**, measured
2026-09-10 on the same source. `conform` runs the dev profile and `depth` the
release one. The oracle aborts at every depth (F85), so there is no golden to
pin a level to, and none could be captured.

### Rejected

**Meaning.** Keeping the level identical with the tier on would need a floor
that moves by the bytes live compiled frames hold, or compiled code on a stack
of its own. Either would be new machinery to preserve a number the oracle
never defines and Bund2's two build profiles already disagree on.

### Consequences

- RFC-0005 criterion 11 compares the level with the feature on and off and
  requires **on ≥ off**, not equality. `cargo xtask depth` prints the level on
  its `loop` axis, so the comparison has something to compare.
- RFC-0005 §S8 drops "the tier's share sits above Tier 0's, never inside it".

- Decided by: repository owner, 2026-09-10, on RFC-0005's seventh review (B1)
- Blocks: nothing; unblocks RFC-0005 §S8 and criterion 11
- Depends on: D37 (no abort), F85 (the floor)
- Status: **RESOLVED — the level is not meaning; never aborting, and never
  less with the tier, are.**

## D45 — a reporter says, per severity, whether it wants a stack snapshot

RFC-0005's seventh review, S1. `Reporter::wants_stack` took no argument, and
`Interp::report` snapshotted every stack for any diagnostic whenever it
returned true. The CLI's `TextReporter` returned true by default, and only
`--no-dump-stack` turned it off. So a native that reported a warning or notice
mid-body read the whole stack. RFC-0005 §S5 answered that by keeping no value
promoted across any call while the reporter wants stacks, and that meant no
promotion across calls in any `bund2 script` run, including every
`cargo xtask conform` run.

`TextReporter` never showed that snapshot. A warning or notice is one line on
stderr with no stack, and only a fatal report prints `[BUND]  Content of the
stack` (`crates/bund2-stdlib/src/report.rs`, `TextReporter::report`).

### Decision

Decided by the repository owner, 2026-09-10: **`wants_stack` takes the
severity**, as `Reporter::wants_stack` in `crates/bund2-api/src/diag.rs`
shows: `fn wants_stack(&self, severity: Severity) -> bool`. `TextReporter`
wants a snapshot only for `Severity::Error`, and only when its dump is on.

**Nothing printed changes**, because nothing printed a stack under a
non-fatal diagnostic before. The one fatal report the CLI makes happens after
evaluation has returned (`run` in `crates/bund2-cli/src/main.rs`), by which
time RFC-0005's compiled bodies have synced. Natives do not report errors: they
return them (RFC-0005 §S11). The natives that report today report warnings
(`while` in `crates/bund2-stdlib/src/control.rs`, and one in `singles.rs`) or a
notice (`?error` in `conditional.rs`).

### Rejected

- **Accept it**: promotion across calls only under `--no-dump-stack` or a
  silent embedder, so the configuration users run would never get it.
- **Sync before calls to reporting natives**: that means classifying which
  natives can call `Vm::report`. The classification has Q34's shape, and it
  would be wrong on the day a native started to warn.

### Consequences

- RFC-0005 §S5's rule reads the reporter **per severity a native reports
  mid-body**, at each compiled body's entry. It cannot be read once when the
  `Interp` is built, because the CLI replaces the reporter afterwards and the
  field is public. It cannot change while a compiled body runs, because no
  `Vm` method reaches it.
- A reporter that does want mid-body snapshots — `CollectingReporter` with
  `wants_stack` set — still gets exact ones, by §S5's rule.
- `Reporter` is D36's seam, and its signature changed. All three implementors
  are in-tree (`TextReporter`, `CollectingReporter`, `SilentReporter`), and
  nothing outside the workspace implements it.
- Conformance before and after: 79/86, ceiling 79/86.

- Decided by: repository owner, 2026-09-10, on RFC-0005's seventh review (S1)
- Blocks: nothing; unblocks promotion across calls in RFC-0005 §S5 under the
  CLI's default reporter
- Depends on: D36 (the reporter seam)
- Status: **RESOLVED — built.**

## D46 — nothing stays promoted across a call that resolves to a lambda

RFC-0005's eighth review, B1. §S5 keeps the values below a callee's arity in
registers across the call, trusting the callee's effect as it was at compile
time, and before the call compares the callee's slot generation with the one
it was compiled against. For a native that pins the effect, because a native's
effect is declared in its own slot. **A lambda has no declared effect.**
`Registry::effect_of` returns `None` for one, since "a lambda's is inferred
rather than declared" (`crates/bund2-api/src/lib.rs`, `Registry::effect_of`).
RFC-0004 §S3 infers it by composition, so it is read from every slot the
lambda's body calls through, and the check reads one of them.

Two programs break it:

1. **Rebound before the call.** `:g { drop } register  :f { g } register`,
   and a body `1 2 3 f` that promotes `1` and `2` across `f`, which infers as
   `(1, 0)`. Then `:g { drop drop drop } register`. `f`'s slot is untouched,
   the check passes, and `g` pulls three values from a stack holding one.
2. **Rebound during the call.** `:f { :g { drop drop drop } register g }
   register`. No check made before the call can see a rebind the callee
   performs itself.

### Decision

Decided by the repository owner, 2026-09-10: **nothing stays promoted across
a call whose name resolves to a lambda when the body is compiled.** The body
syncs every promoted value before such a call, as at an opaque site.
Promotion crosses calls to natives only. A name that resolves to a native at
compile time and has a lambda registered later is still caught:
`register_lambda` touches that name's slot, so the pre-call check fails and the
body takes the residual path.

### Rejected

**Pin the inference**: record every slot a lambda's inferred effect was read
from, transitively, check all of their generations before the call, and treat
as opaque any lambda whose body can rebind a name. That needs a dependency
list per call and a purity analysis, every future rebinding word has to be
classified, and D16's computed names mean a body can reach a rebinding word the
analysis never saw.

### Consequences

- An inferred effect is never trusted across a call. The effects compiled code
  trusts are declared ones, read from the native's own slot.
- RFC-0005 criterion 22 gains both programs above.
- None of RFC-0005 §S6's measured figures depends on promotion across a user
  word: they measure straight-line runs and native calls.

- Decided by: repository owner, 2026-09-10, on RFC-0005's eighth review (B1)
- Blocks: nothing; unblocks RFC-0005 §S5
- Depends on: D16 (the open world), RFC-0004 §S3 (effects are inferred by
  composition), D43 (the generation cells)
- Status: **RESOLVED — decided; implemented with the tier.**

**Note, 2026-09-10 (F93).** The premise's "`Registry::effect_of` returns `None`
for one" was true only of a lambda with no native in the same slot. For a
lambda that shadows a native, `effect_of` returned the native's effect until
F93, found by RFC-0005's ninth review (S1). The decision is unaffected, since it
is stated in terms of what a name *resolves* to. RFC-0005 §S5 classifies
callees with `Registry::resolve`, and `effect_of` now follows the same order.

## D47 — promotion crosses only the natives `bund2-stdlib` registered

RFC-0005's ninth review, B3. §S5 keeps values in registers across a call to a
native with a fixed effect. That is sound only if the native runs no body,
reports nothing at `Error`, observes nothing beyond its operands, and moves the
stack by what it declares. RFC-0005's criteria 24 and 25 check the first, second
and fourth for `bund2-stdlib`. They read `bund2_stdlib::register_all` and
`crates/bund2-stdlib/src/`.

But `Registry::register_native` is public (`crates/bund2-api/src/lib.rs`), and
D9's resolution gives external crates "`Native` with a declared effect". An
embedder's `eff(1, 0)` native that calls `vm.eval_lambda`, or reports an error,
would get wrong answers with `--features jit`, and no criterion could fail. F87
and F91 show that even the stdlib's own declarations were wrong until a
sufficient check existed.

### Decision

Decided by the repository owner, 2026-09-10: **promotion crosses a call only
when the callee's registration id is one `bund2-stdlib` made.** D43 already
gives every registration an id, and `bund2-stdlib`'s fragments are keyed by the
ids of the registrations it made. Every other native is synced before, as at an
opaque site. The body is still compiled, and the call is still made through its
slot. An embedder's native costs speed, never meaning.

### Rejected

- **An opt-in declaration** on `Native`, by which an external native asserts
  that it runs no body, reports no error and reads only its operands. It adds to
  `bund2-api`'s stable surface, and the assertion is as unverifiable as a
  declared effect is today. It can be proposed later, if an embedder needs the
  speed.
- **Trust every declared effect, and document it.** A mis-declared embedder
  native would then give wrong answers only with the tier on. That breaks "the
  JIT changes speed, not meaning" for the users least able to diagnose it.
- **A run-time guard** refusing re-entry from any fixed-effect native. It sees
  re-entry, but not a report or an observation beyond the operands.

### Consequences

- RFC-0005 §S5 gains the rule, and its assumptions 7 and 8 say
  "`bund2-stdlib`".
- RFC-0005 criterion 27 checks that promotion does not cross an embedder's
  native.
- D9 is unchanged. External crates still get `Native` with a declared effect,
  and `bund2 check` still reads it.

- Decided by: repository owner, 2026-09-10, on RFC-0005's ninth review (B3)
- Blocks: nothing; unblocks RFC-0005 §S5
- Depends on: D9 (what external crates get), D43 (registration ids)
- Status: **RESOLVED — decided; implemented with the tier.**

**Amended 2026-09-10 by D48, on RFC-0005's tenth review.** "The same set its
fragments are keyed by" was wrong: the fragment table holds three ids. The set
is every id `register_all` mints, recorded in the `Registry` and re-recorded on
a replay (F32). D48 narrows it further, to the natives criterion 28's palette
brought to `Ok`. A command carries no D43 id, so no command is ever crossed.

## D48 — promotion crosses only the natives an audit has brought to `Ok`

RFC-0005's tenth review, B1. D47 restricted promotion to the natives
`bund2-stdlib` registered, on the premise that their declared pairs are
checked. Criterion 24 checks them only on the arms the corpus reaches, and
`drop_stack`, which no golden runs, declared `1 -> 0` while removing the whole
current stack (F94).

### Decision

Decided by the repository owner, 2026-09-10: **promotion crosses a native only
if it is listed in `tests/golden/PROMOTABLE.txt`.** The list holds the
fixed-effect natives that criterion 28's palette has brought to `Ok` with no
breach of their declared effect. The palette covers fourteen operand kinds for
the top two operands and, for a workbench form, every kind on the workbench
too. Every other native, whether never brought to `Ok` or not `bund2-stdlib`'s
(D47), is synced before, as at an opaque site. That costs speed, never meaning.

The test writes the list (`BUND2_UPDATE_PROMOTABLE=1`) and otherwise compares
against it, so the list cannot drift. A native that newly reaches `Ok`, or no
longer does, fails the test until the list is regenerated and its diff
reviewed. On 2026-09-10 it lists 178 natives; its header gives the count of
fixed-effect natives it was drawn from.

### Rejected

- **Widen the audit, then exclude.** It reaches the same end state with more
  machinery before anything is trusted. The palette is that first step, and it
  can grow.
- **Trust declarations.** A wrong pair is a silent wrong answer in compiled
  code.

### Scope

The list certifies a native's pair, not what else the native observes. A Q34
observer such as `debug.display_stack` can be listed and is still a promotion
barrier, under RFC-0005 criterion 14.

- Decided by: repository owner, 2026-09-10, on RFC-0005's tenth review (B1)
- Blocks: nothing; narrows RFC-0005 §S5
- Depends on: D47, D43 (registration ids)
- Status: **RESOLVED — the list and its check are built; the tier consumes
  them.**

**Dated note, 2026-09-11 — natives that act on the host are not run.** The
palette audit calls every fixed-effect native for real. Once the host words
landed, that meant `sleep.seconds` waited seven seconds on the palette's `7`,
and `fs.rm` was handed the palette's strings as relative paths to delete.
`fs.rm`, `sleep.seconds`, `system.setproctitle` and `system.setproctitle.` are
therefore left out of the run (`crates/bund2-stdlib/src/lib.rs`,
`ACTS_ON_HOST`). Unrun, they are not reached and not listed, so promotion syncs
before them. That is the conservative side of this decision and changes no
answer.

**Dated note, 2026-09-11 (second) — six, not four.** `password`, which waits
on the terminal, and `save.model`, which would write a world file for each
palette string, joined the unrun set later that day. The exclusions are kept
by hand, so a new host-acting native runs for real under `cargo test` until
it is added. RFC-0005's criterion 28 now states both limits, this and that the
list certifies the default registration (the eleventh review's S4).

**Dated note, 2026-09-14 — a feature-gated native is excluded in every build
(F123).** This decision's regeneration command carries no feature list, and its
scope paragraph says the list "certifies a native's pair" — neither
contemplated a native whose *existence* depends on a Cargo feature.
`string.grok` and `string.grok.` are registered only under `--features grok`
(D10, D40), so the audit drew them in one build and not the other, and no
content of the file satisfied both: listed, the default build reported them
`no longer reached`; absent, `--all-features` reported them `now reached`.
`cargo test --workspace --all-features` failed on it.

The owner chose F123's first disposition, 2026-09-14: **make the audit
feature-aware.** `FEATURE_GATED` (`crates/bund2-stdlib/src/lib.rs`) excludes
such natives where the native list is built, in both builds, beside
`ACTS_ON_HOST` and for the same reasons — kept by hand, so a new one is
audited for real until it is named; conservative, since unlisted means
promotion syncs before it as before an embedder's native.

**The list did not change.** The existing 222 entries were already correct for
both builds once the audit stopped drawing the gated pair, so
`PROMOTABLE.txt` was not regenerated. The regeneration command in this
decision therefore still stands as written, and needs no feature list: it
produces the same content under either build.

**Dated note, 2026-09-11 (third) — the list certifies more than a pair.**
*Scope* above says the list "certifies a native's pair, not what else the
native observes". Since D55 it certifies both: a native that D55's audit sees
reading beyond its operands is kept off the list. RFC-0005's criterion 28 now
says so (the twelfth review's S2).

## D49 — a panic in a native is caught where the native is called, in both tiers

RFC-0005's tenth review, B2. D37 governs Bund2's code, not its dependencies,
and `string.distance.jarowinkler` panics inside `natural` (F95). At Tier 0 the
panic unwound out of the evaluation thread and the process exited 1. Through
compiled code it would unwind out of an `extern "C"` shim, which aborts the
process.

### Decision

Decided by the repository owner, 2026-09-10: **every native call catches a
panic and returns `Error::internal` naming what was running**, through
`bund2_api::catch_panic`. That covers `Interp::invoke` for registered natives,
and the direct calls `bund2-stdlib` makes to method natives (`oop.rs`) and
conditional runners (`conditional.rs`). RFC-0005's per-native adapter applies
the same rule, so both tiers agree. A caught panic is reported, not printed.
While a catch is active, a hook installed on first use prints nothing and
keeps the panic's location for the message. Every other panic goes to the hook
that was there before.

### Consequences

- Tier 0 changes. F95's program reports an internal error and exits 0 where it
  used to exit 1 with a backtrace. `?try` can catch the error, as it can any
  error. The native may have left the `Vm` half-updated, which is what
  "internal error" says.
- The build must keep unwinding: a `panic = "abort"` profile would disable
  this. Nothing sets one; `Cargo.toml`'s `panic = "deny"` is the clippy lint.

### Rejected

- **Abort in both tiers.** It contradicts D37's "every unrecoverable internal
  error is handled with a meaningful explanation".
- **Fix panics case by case.** The next one nobody has found would abort under
  Tier 1.

- Decided by: repository owner, 2026-09-10, on RFC-0005's tenth review (B2)
- Blocks: nothing; unblocks RFC-0005 §S8's call boundary
- Depends on: D37, D36
- Status: **RESOLVED — built.**

## D50 — `sysinfo.version` reports Bund2's own version, an approved deviation

`sysinfo.version`, and its alias `version`, push the interpreter's version
string. The reference pushes `env!("CARGO_PKG_VERSION")` of the `bund` crate,
`0.22.0` at the pinned SHA (`reference/Bund/src/stdlib/functions/sysinfo/host.rs:9-12`).
Bund2 pushes its own crate's version (`crates/bund2-stdlib/src/sysinfo.rs`,
`version`). The code has always called this a deviation "with no golden to
record it against", which kept the word among the five that coverage never
counts.

### Decision

Decided by the repository owner, 2026-09-11: **Bund2 reports its own
version**, and the difference is recorded. Reporting `0.22.0` would have Bund2
claim to be a version of Bund it is not. The probe
`tests/probes/sysinfo-version.bund` runs both spellings against the oracle.
The owner captured its golden with `cargo xtask golden` and listed it in
`tests/golden/DEVIATIONS.txt` under this decision with
`cargo xtask conform --accept-deviation probes/sysinfo-version.golden --reason
D50`. The row records the hash of Bund2's expected output, so an unintended
change still fails. Conformance is 82/90, ceiling 82/90, with eight approved
deviations; coverage gained the word (265/497).

### Rejected

- **Report the reference's `0.22.0`.** It would pass the probe, but it is a
  compatibility claim no one has asked for.
- **Leave the word unprobed.** The difference stays implied rather than
  recorded, and coverage never counts the word.

- Decided by: repository owner, 2026-09-11
- Blocks: nothing
- Depends on: D21 (probes)
- Status: **RESOLVED — the probe, its golden and its deviation row are
  recorded (2026-09-11).**

## D51 — pure-Rust substitutes for the reference's C-compiling crates

Three library words reach crates that compile C at build time:

- `sqlite` reaches rusqlite with `bundled`
  (`reference/Bund/Cargo.toml:131-133`), which builds SQLite from C source.
- `csv` reaches polars:
  `reference/Bund/Cargo.toml:93-95`.
- `file` and `file.` reach curl:
  `reference/Bund/Cargo.toml:16`. They fetch a URL through libcurl
  (`reference/Bund/src/stdlib/helpers/file_helper.rs:42-59`).

D10 and D40 keep a default build free of a C toolchain. `grok` met that by
becoming an optional feature that is off by default.

### Decision

Decided by the repository owner, 2026-09-11: **these words use pure-Rust
substitutes and are always built**. They are not feature-gated. `file` reads
with `std::fs`, `csv` uses the `csv` crate, and `sqlite` uses a pure-Rust
engine that can read the SQLite file format.

**Every observable difference from the reference is an approved deviation
under this decision.** Most are in error text, which comes from the library
that failed. Each difference is recorded where the word is implemented, with a
probe where one can be run.

`file` is the closest case. The reference turns any curl failure into `None`
and reports it as `FILE gets no data`
(`reference/Bund/src/stdlib/functions/filesystem/file.rs:47-48`), and it
decodes the bytes lossily (`file_helper.rs:54`). A `std::fs` read that does
both the same way gives the same answers.

**Dated note, 2026-09-11 — the sentence above held only for absolute paths.**
`file` fetches `file://{path}`, and curl reads a relative path there as a host
name. So `"tests/…/scores.csv" file` fails in the reference, and the same file
named absolutely reads; confirmed against the oracle. A plain `std::fs` read
accepted the relative path. `file` now goes through the same `file://` fetch
as `use` (D54, `host.rs` `fetch_uri`), so a relative path fails as it does in
the reference.

### Rejected

- **The reference's crates behind an off-by-default feature**, as D40 did for
  grok. A default build would then lack `sqlite`, `csv` and `file`.
- **Deferring the three words.**

### Not covered

`save.model` and `load.model` also reach rusqlite, through the world file
(`reference/Bund/src/stdlib/helpers/world/mod.rs`). **They are deferred until
D31 is decided**, because D27 has already moved the world file to redb, and
D27 depends on D31. Decided by the repository owner, 2026-09-11.

- Decided by: repository owner, 2026-09-11
- Blocks: nothing
- Depends on: D10, D40 (the toolchain rule); D27 and D31 (for the model words)
- Status: **RESOLVED**

## D52 — `bund.exit` is a request the embedder honours

The reference's `bund.exit` calls `process::exit`: with code 0 when the stack
is empty, and otherwise with the popped INTEGER, or 0 if the value will not
cast (`reference/Bund/src/stdlib/functions/bund/bund_exit.rs:10-31`). Bund2's
shipped code may not end the process (CLAUDE.md, D37). An embedder such as a
TUI has its own idea of what ending means.

### Decision

Decided by the repository owner, 2026-09-11: **`bund2-api` gains
`Vm::request_exit(code)`**. `bund.exit` calls it. The interpreter stops at the
next word boundary and runs nothing more of the program. The embedder reads
the requested code. The CLI returns it as the process exit code, keeping the
low 8 bits as Unix `exit` does.

This widens the API, which is why it is recorded here. Every `Vm`
implementation has to answer the request.

### Rejected

- **A marked `Error` the CLI recognises.** It needs no API change, but it
  carries the exit as a magic string, and every other embedder would see an
  ordinary error.
- **Deferring `bund.exit`.**

- Decided by: repository owner, 2026-09-11
- Blocks: nothing
- Depends on: D37 (no process exit in shipped code), D14 (the API surface)
- Status: **RESOLVED**

**Dated note, 2026-09-11 — "nothing more" includes the native, and the state
after an exit is meaning.** RFC-0005's thirteenth review found that Tier 0
did not keep this to the letter. A body a native ran synchronously, ending in
`bund.exit`, returned `Ok`, and the native went on (F112). The repository
owner chose to fix Tier 0 rather than have the compiled tier copy it: the
refusal now also comes where a synchronous run returns to Rust. The owner
also ruled that the stacks and workbench left after an exit are part of the
program's meaning, and the tiers must agree on them. An embedder such as a TUI
may show them. RFC-0005's criterion 30 compares them.

**Dated note, 2026-09-11 (second) — one native still runs its handler.** The
note above says "nothing more" includes the native. That holds for every
native that passes the refusal up, which in `bund2-stdlib` is all of them but
`?try`. `?try` catches the error, pushes its `error` CONDITIONAL, and only
then is its `except` body refused. That CONDITIONAL is part of the final
stack, which the owner has ruled is meaning. Both tiers leave it, with the same
`context` text (RFC-0005's assumption 25 and its fourteenth review, S3). An
embedder's native that catches the refusal likewise runs whatever it does next.

**Dated note, 2026-09-11 (third) — `?try` is the only one that then acts.**
The note above says every `bund2-stdlib` native but `?try` passes the refusal
up. `#` and `#.` do neither: they catch one and discard it
(`let _ = crate::values::execute_top(vm)`, `object_execute_base`,
`crates/bund2-stdlib/src/oop.rs`), by design, so that a failing `unwrap` does
not stop `#`. They do nothing afterwards and answer `Ok`, so no state differs
and no tier can disagree. The claim to keep is narrower: `?try` is the only
`bund2-stdlib` native that catches the refusal **and then does work**
(RFC-0005's fifteenth review, S1).

## D53 — `debug.display_hostinfo` reports Bund2's own crates, an approved deviation

The reference's `debug.display_hostinfo` prints a table
(`reference/Bund/src/stdlib/functions/debug_fun/debug_display_hostinfo.rs:12-73`).
The first six rows give the versions of its internal crates: `rust_dynamic`,
`rust_multistack`, `rust_multistackvm`, `bundcore`, `bund_language_parser` and
`internaldb` (`:39-56`). The table then shows distributed mode, hostname, OS
version, virtualization and kernel version (`:57-71`). Bund2 has none of those
six crates.

### Decision

Decided by the repository owner, 2026-09-11: **the six version rows name
Bund2's own crates and versions**. The host rows follow as the reference has
them. The difference is recorded as an approved deviation, the same way D50
recorded `sysinfo.version`. The table depends on the host in any case, so no
golden can hold it and no conformance number moves.

### Rejected

- **Keeping the reference's six row labels with Bund2's version in each.**
  The table would name crates Bund2 does not contain.
- **Dropping the version rows.**

- Decided by: repository owner, 2026-09-11
- Blocks: nothing
- Depends on: D50 (the same question for `sysinfo.version`)
- Status: **RESOLVED**

## D54 — `use` and `url` fetch `file://` and `http://`; `https://` is deferred

The reference fetches through libcurl in three places:
- `use` and `use.` evaluate what they fetch
  (`reference/Bund/src/stdlib/functions/bund/bund_use.rs:31-33`);
- `url` and `url.` push it
  (`reference/Bund/src/stdlib/functions/filesystem/file.rs:72-78`);
- `file` and `file.` fetch `file://{path}`
  (`reference/Bund/src/stdlib/helpers/file_helper.rs:57-59`).

curl takes every string as a URL. So a bare path never loads a library in the
reference. Confirmed against the oracle on 2026-09-11:
- `use "lib.bund"` fails with `Couldn't resolve host name`;
- `use "/abs/lib.bund"` fails with `URL using bad/illegal format`;
- `use "file://relative/lib.bund"` fails;
- `use "file:///abs/lib.bund"` loads.

D51 took curl out of the build.

### Decision

Decided by the repository owner, 2026-09-11: **`file://` and `http://`, not
`https://` for now.**

- `file://` follows curl's rules. It takes an absolute path, optionally after
  the host `localhost`, and decodes `%xx`.
- `http://` is fetched by `ureq` built without TLS. It keeps the defaults
  curl has as the reference leaves them: no redirect is followed, the body of
  an error status is still the answer, the body has no size limit, and the
  user agent is `ZBUS` (`file_helper.rs:43`).
- **`https://` is deferred.** A TLS stack is the question: rustls's usual
  crypto providers compile C or assembly, which D10 does not allow below
  `bund2 build`.

**Approved deviations under this decision:**
- A string with no scheme is refused. curl would guess `http://` for one, so
  in the reference `use "example.com/lib.bund"` fetches over HTTP.
- `https://` is refused.

Implemented in `crates/bund2-stdlib/src/host.rs` (`fetch_uri`), shared by
`file`, `url` and `use`.

**Not decided here: `use` in a built artefact.** A `use` path can be a
run-time string (D16), so `bund2 build` cannot always know what to embed, and
`--emit=bundle` must stay self-contained (D10). That belongs to RFC-0006, and
it is carried as Q38 so that no default is taken silently.

- Decided by: repository owner, 2026-09-11
- Blocks: nothing
- Depends on: D51 (no curl), D10 (no C below `bund2 build`)
- Status: **RESOLVED for the interpreter; the AOT half is Q38.**

## D55 — the words that read beyond their arity are found by audit (Q34)

Q34 asks what identifies a word that reads the stack beyond what it consumes.
RFC-0005's promotion keeps the values below a callee's operands in registers,
and puts only the operands on the real stack (§S5). So such a word would see a
stack shorter or different from Tier 0's. There are three shapes of it:
- a whole-stack reader, such as `debug.display_stack`, which declares
  `eff(0, 0)` and reads everything;
- a depth guard larger than the word's arity;
- a word that reaches a stack by name, such as `swap_in` or `rotate_stack_*`,
  when the name is the current stack's.

RFC-0005's criterion 14 needs these words to be barriers, and `StackEffect`
records what a word consumes, not what it observes.

### Decision

Decided by the repository owner, 2026-09-11: **derive the set by audit, and
change no API** (option 2 of four).

- **A four-run differential in criterion 28's palette (D48).** Each
  fixed-effect native, over each operand tuple, runs four times:
  - with its operands padded beneath as before, twice, which tells a
    deterministic native from a random one;
  - with nothing beneath its operands;
  - with different values beneath.

  If a deterministic native's runs differ in status, error text (ids and
  stamps normalised, F14), produced values or workbench, it observes beyond
  its operands. So does a nondeterministic native whose runs differ in status
  or in the kinds it produces. Whether a native is nondeterministic is decided
  once, over all its tuples: two identical runs that ever differ make it so.
  A tuple holding an operand that displays with its id and stamp (a lambda, an
  object) is left out of the comparison, because an answer built from one
  depends on identity and time, which F14 says are not behaviour. The other
  checks still run on it.
- **The padding must survive.** After a run that returns `Ok` on the same
  current stack, the values beneath the operands must still be the padding. A
  word that returns nothing and reorders what lies beneath it, such as
  `rotate_stack_left` given the current stack's name, shows nothing else a run
  could compare.
- **An observation audit.** While the effect audit is on, the interpreter
  records any fixed-effect native that reads the whole stack or the workbench
  (`snapshot`, `snapshot_workbench`), or reads a stack's depth by name
  (`depth_of`). Asking for the current stack's *name* is not recorded: the
  name is not a value promotion holds. A word that goes on to act on the
  current stack by name is caught by the differential instead, as F111's six
  were on the audit's first run. This takes the place of the source scan the
  option first proposed. A scan finds Rust functions, many
  of them closures in a registration call, not the words they are registered
  under; the audit records the word.
- **The palette gains the current stack's name, `"main"`,** as an operand
  kind, so a word that takes a stack name is tried on the current stack.

`tests/golden/PROMOTABLE.txt` lists a native only if the palette brought it to
`Ok` with no breach **and** neither check flagged it. Promotion already syncs
before any native not on the list, so a flagged native is a barrier with no
new mechanism. The file lists the flagged natives as comments, each with its
reason.

**What it cannot see.** An observation that changes nothing a run returns, and
calls none of the four methods, is missed: say, a native that reads `depth()`
and only prints it. RFC-0005's assumption 22 names this.

### Rejected

- **A declared "observes" flag on the registration.** It is precise, but it
  changes `bund2-api` under RFC-0002, which is Accepted, and it is kept by hand.
- **No value promoted across any call.** It makes the question moot and costs
  promotion across calls.
- **Staging: that rule first, the audit later.**

**First run, 2026-09-11.** Stable over three runs, the audit flags eight
natives, as the regenerated `PROMOTABLE.txt` lists them:
- `debug.display_stack` and `debug.display_workbench`, which read the whole
  stack or workbench;
- `move_from`, `rotate_current_left`, `rotate_current_right`,
  `rotate_stack_left` and `rotate_stack_right`, which change the values
  beneath their operands;
- `swap_in`, which reads a stack's depth by name.

Getting there corrected the audit twice:
- `+.`, `-.`, `*.` and `/.` were flagged for checking an empty workbench with
  a whole snapshot. They now ask `workbench_depth()`.
- An observation hook on `current_name` flagged `current`, which reads no
  value, and was removed.

The run also found F111: six named-stack words whose declared pair is wrong
on the current stack, now declared opaque.

- Decided by: repository owner, 2026-09-11
- Blocks: nothing; answers Q34, and makes RFC-0005's criterion 14 satisfiable
- Depends on: D48 (the palette and `PROMOTABLE.txt`), D47
- Status: **RESOLVED — built, and `PROMOTABLE.txt` regenerated by the
  repository owner on 2026-09-11 (`bde0eed`), carrying D55's and F111's
  figures.**

## D58 — bytes Bund2 did not write are a stated exception to D37, with a size refusal

RFC-0005's twenty-first review measured what F118's write-side bound does not
reach. `sqlite` decodes every BLOB in any SQLite file a program names, through
the same `from_binary` (`crates/bund2-stdlib/src/data.rs`, `sql_cell`), and a
BLOB nested 3,000 deep aborts the process:

| depth | BLOB | result |
|---|---|---|
| 10, 256, 1,000 | 630 B – 58 KB | decodes, and the lambda runs |
| 3,000 | 174 KB | `fatal runtime error: stack overflow`, exit 134 |

Reproduced 2026-09-12 on the dev binary, with BLOBs hand-built in bincode's
legacy layout and inserted with `sqlite3`.

**D31 cannot justify this path.** D31 asks whether anything outside the project
reads a Bund *world file*, and D27 made a world file redb, so a SQLite database
is never one. D39's third clause says "bound the side you control" — and here
Bund2 controls neither: it wrote neither the file nor the value.

### Decision

Decided by the repository owner, 2026-09-12: **record the exception, and add a
size refusal.**

1. **D37 gains a stated exception.** Evaluation still never aborts on anything
   Bund2 produced. Bytes read from a file Bund2 did not write are outside that
   guarantee, because the recursion is bincode's derived `Deserialize`, which
   builds the whole nested value before any Bund2 code runs. There is no point
   at which a depth check could refuse.
2. **`sqlite` refuses a BLOB over `MAX_BLOB_BYTES`, 16 KiB**
   (`crates/bund2-stdlib/src/data.rs`), reporting as any other word's error
   does. The number is the write-side bound read through size: a BLOB that is
   all depth costs about 58 bytes a level (measured), so `MAX_WIRE_DEPTH`'s 256
   levels is about 14.8 KB. A wide, shallow BLOB over the cap is refused too.
   That is the conservative side, and no corpus program reads a BLOB at all.

So the common case reports instead of aborting, and the residue — a BLOB under
16 KiB but pathologically deep, or a future third-party decode path — is a
limit this register now names rather than a defect.

### Rejected

- **Bounding the decode before bincode sees the bytes.** The legacy layout is
  fixed-width for most variants, but `Val::Json` carries a
  `serde_json::Value` whose encoding is not fixed-shape, so a non-recursive
  scan of the byte structure could not measure nesting in general.
- **Leaving it unbounded and recording only the exception.** A 174 KB file
  would still abort a program that merely queried it, which is the worse
  failure mode: the exception is meant to name a residue, not the common case.

- Decided by: repository owner, 2026-09-12
- Blocks: nothing; closes RFC-0005's twenty-first review B1
- Depends on: D37 (which it narrows), D39 (whose third clause has no side to
  bound here), F118, D27 and D31 (which do not govern this path)
- Status: **RESOLVED**

## D57 — `convert.to_dict` is corrected, and a matrix is a tagged list

Two questions arrived together with the MATRIX family, the last six core words
Bund2 had not implemented, and both are decisions rather than readings.

### `convert.to_dict` converts to a dict

The reference's word converts to a **matrix**: both bodies pass `MATRIX` while
their error prefixes say `CONVERT.TO_DICT`
(`reference/rust_multistackvm/src/stdlib/convert/internal.rs:99-105`), so
`convert.to_dict` and `convert.to_matrix` are one word under two names —
confirmed on the oracle, `dt: 26` from both. That is F120. The conversion the
name promises exists one layer down and is never called: `conv`'s MAP target
keys a container's items by position as strings
(`reference/rust_dynamic/src/conv.rs:426-434`).

Decided by the repository owner, 2026-09-12: **correct it.**
`convert.to_dict` and `convert.to_dict.` target MAP, so
`[ 7 8 ] convert.to_dict` answers `{ "0": 7, "1": 8 }`.

- **Why not reproduce it.** Reproducing would ship two spellings of one word,
  leave `conv`'s MAP arm dead in Bund2 as well, and give a program no way to
  reach a conversion the language names. The usual rule — an
  original-implementation bug is recorded, not fixed — is about behaviour a
  program can depend on; nothing can depend on this, because no corpus program
  uses any of the six words and the word does not do what it says.
- **The cost.** A deviation with no golden to record it against, which is
  F48's gap, the same one D30's and D33's deviations sit in. Conformance does
  not move: 105/113 on both tiers, before and after.

**That gap is closed, 2026-09-29.** `tests/probes/convert-to-dict.bund`
captures the oracle's answer for both dict spellings and for
`convert.to_matrix` as a control, so the golden records where the deviation is
*and* where it stops. Bund2 fails two of its three rows by design, and the
failure is accepted against this decision. The cost line above stands as what
was true for two and a half weeks; what replaces it is that this deviation is
now the only kind worth having — one an accepted golden states rather than one
a register asserts.

### A matrix carries rows of its own

The reference gives a matrix its own payload, `Val::Matrix(Vec<Vec<Value>>)`
(`reference/rust_dynamic/src/types.rs:75`), tagged `dt = 26`. **Bund2 does the
same**, as `Payload::Matrix(Vec<Vec<BundValue>>)`.

**A tagged LIST was tried first, and withdrawn.** The attraction was that
`dt` and payload are independent axes here — as they already are for PTR and
CALL over a string — so carrying rows in `Payload::List` under `dt = 26` would
have needed no new arm in `Drop`, `render`, `display`, equality, hashing or
the wire codec, and could not have reintroduced the recursion F114 through
F119 removed.

It is wrong all the same, because **`render` is observable**. A golden
captures `debug.display_stack` byte for byte, and the two shapes do not agree:

| | rendered |
|---|---|
| oracle, and Bund2 now | `data: Matrix([[Value { … I64(1) … }]])` |
| the tagged LIST | `data: List([Value { dt: 9, … data: List([…]) }])` |

A different payload name, one more level of nesting, and a header per row.
No normaliser reconciles that, so no golden could have been captured for the
MATRIX family at all. The probe written to cover the family is what found it:
the design's own claim — that nothing could see the difference — was false.

So every walk gains a `Matrix` arm, and **each is written as a worklist**:
`take_members` flattens rows into the drop worklist, `render_payload` queues
row brackets and cells as steps, and both wire directions carry rows of child
indices. The cost the tagged list was meant to avoid is paid deliberately, in
the shape F114 through F119 established.

What this still gives up: ragged rows are representable, so rectangularity is
a property of construction rather than of the type. The reference's is no
better — `push` appends a row of any width and silently ignores a non-list
(`reference/rust_dynamic/src/push.rs:50-69`) — and only arithmetic checks
widths, by returning its left operand unchanged (`math.rs:36-64`).

**A second finding from the same probe.** A MATRIX converts to a LIST and to
nothing else: `value_matrix_conversion` accepts a LIST target and falls to its
`_` arm otherwise (`reference/rust_dynamic/src/conv.rs:268-290`), so
`[ [ 1 2 ] ] matrix matrix` is refused with `Can not convert list to 26` —
"list", because the wording is shared with the list converter. An earlier
draft had it as the identity, read from the LIST converter's MATRIX arm
instead of the MATRIX converter's. Bund2 refuses it with the same text.

- Decided by: repository owner, 2026-09-12
- Blocks: nothing; closes the last of the core words (`core words not
  implemented` reads 0)
- Depends on: D48 and D55 (the palette runs the four new fixed-effect
  natives), F48 (no way to record the deviation against a golden), F120
- Status: **RESOLVED**

## D120 — `--noeval` gates `debug.run`

**Raised 2026-10-08** by RFC-0006's seventh review (B1). D113.5 added
`debug.run`, which evaluates a STRING of Bund with a stop offered before each
term, and D115 kept it running its string in a bundle. Neither asked what
`--noeval` does to it. Measured before this decision, on a bundle built
`--noeval --noio` with standard input closed:
`"40 2 + println" debug.run` printed `42`.

- Blocks: RFC-0006's acceptance
- Depends on: D78, D79, D113 (part 5), D115, RFC-0006 §B3a, RFC-0008
- Status: **RESOLVED** by the repository owner, 2026-10-08: "`--noeval` gates
  `debug.run`". Built the same day.

### Why it is not D79's boundary

D79 reads `--noeval` as the reference reads it: the flag disables the
`bund.eval` group, six words, and what lies outside the group is the
reference's boundary and not a shortfall. **`debug.run` is not the
reference's.** It is Bund2's own word, and its own registration comment calls
it "`bund.eval` with a safepoint offered before each term"
(`crates/bund2-stdlib/src/lib.rs`). There was no boundary to inherit: the
word was `bund.eval` under another name, added outside the group, and nobody
had chosen that.

### The decision

**Under `--noeval`, `debug.run` is a stub.** It fails with
`bund DEBUG.RUN functions disabled with --noeval`. The shape is the
reference's stub message; the group name is the word, since the reference
has none for it.

In a bundle the stub holds wherever D78's floor does: built `--noeval`, or
`BUND2_NOEVAL` set at start. D115 is otherwise unchanged — without the flag a
bundle's `debug.run` runs its string and offers no stop.

**Not a deviation.** The reference has no `debug.run`, so no behaviour of its
is changed, no golden moves, and `conform`, `COVERAGE` and `IMPLEMENTED` move
by zero.

### What it does not cover

**The ruling names one word and gates one.** Three routes still run text
under `--noeval`, and RFC-0006 §B3a names each:

- `compile lambda! !`, on both binaries (D79).
- `debug` and `debug.shell`, which read lines and run them, and which the
  reference registers ungated.
- **`debug.feed` before `debug.shell`.** `debug.feed` is Bund2's and queues a
  line for the next word that reads; `debug.shell` then runs it. Measured
  after this decision, on a bundle built `--noeval --noio` with standard
  input closed: `"40 2 + println" debug.feed debug.shell` prints `42`. So a
  program can supply `debug.shell`'s line itself, with nobody at the
  keyboard. The evaluating word is the reference's and ungated there; the
  word that makes it need no operator is Bund2's. It was put beside this
  question and not ruled on, so it stays as it is and is named.

`noeval_reaches_debug_run` holds the stub
(`crates/bund2-stdlib/src/host.rs`), and
`both_floors_hold_for_every_word_they_name` holds it in a bundle against
`BUND2_NOEVAL` set to `0` and to empty.

## D119 — `--noio` gates `csv` and `sqlite`

**Raised 2026-10-08** while answering RFC-0006's sixth review (B2), which
asked for `--noio`'s ungated surface by name. Running every registered name
under the flag showed two words that read a file and are not gated. Measured
on a bundle built `--noio --noeval`:
`"…/t.csv" csv :lambda { println } set !` printed the file's rows.

- Blocks: nothing
- Depends on: D78, D79, RFC-0006 §B3a
- Status: **RESOLVED** by the repository owner, 2026-10-08: "`--noio` should
  gate `csv` and `sqlite`". Built the same day. **An approved deviation.**

### The reference

It registers both with no gate
(`reference/Bund/src/stdlib/functions/conditional/mod.rs:42-43`) and opens the
named file at `conditional_csv.rs:62` and `conditional_sqlite.rs:38`. So under
the reference's `--noio` a program reads any CSV file or SQLite database it
can name.

### The decision

**Under `--noio`, `csv`, `csv.` and `sqlite` are stubs**, and so are the two
conditional handlers. They fail with `bund CSV functions disabled with --noio`
and `bund SQLITE functions disabled with --noio`. The shape is the reference's
stub message; the group names are Bund2's, since the reference has no stub for
either.

**The handlers are stubbed because the words are not the only route.**
`conditional :type "csv" set :name "…" set :lambda { … } set !` builds the
same CONDITIONAL without calling `csv`, and read the file before this
decision. It is refused now.

### What it does not change

- Without `--noio`, nothing. No golden moves: none runs `csv` or `sqlite`
  under the flag.
- **The flag is still not a sandbox** (D78). Standard input, the host's
  address and name, the clock, and the debugger's history file stay ungated,
  and RFC-0006 §B3a names them. The ruling names two words and gates two.

`noio_reaches_the_two_words_that_read_a_data_file` holds it
(`crates/bund2-stdlib/src/host.rs`).

## D118 — a bundle's runtime is the binary that builds it

**Raised 2026-10-08** by RFC-0006's sixth review (B4). §B1 specified that
`bund2 build` "ships or locates a prebuilt runtime per target it can bundle
for", and §B4 and §B6 leaned on it. What was built copies
`std::env::current_exe()` (`crates/bund2-cli/src/main.rs`, `build`), and every
criterion of RFC-0006 was met against that.

- Blocks: RFC-0006's acceptance
- Depends on: D10, D80, D82, D116, RFC-0006 §B1, §B4, §B6
- Status: **RESOLVED** by the repository owner, 2026-10-08: "bundle runtime
  is the building binary".

### The decision

**A bundle is a copy of the `bund2` that built it, with the program written
into its region.** There is no separate prebuilt runtime, and `bund2 build`
neither ships nor looks for one.

### What follows

- **A bundle is for the builder's own target.** To bundle for a target, run a
  `bund2` built for that target. A target Cranelift does not support is served
  by a default-feature `bund2` built there, which is D10's toolchain-free
  product as before; what is given up is producing it from another machine.
- **A bundle carries the builder's own features.** A JIT bundle (D80) comes
  from a `bund2` built with `jit`. `bund2 build --features` is refused and
  says why.
- **Builder and runtime cannot be different Bund2 builds.** The version the
  region records is for a reader, through `--inspect`. The runtime does not
  check it, and need not.
- **Parse-at-build (D116) is exact.** The parser that accepts a program at
  build is the one that reads it at start, so a program cannot build and then
  fail to parse.

### What is not decided

Cross-target bundling is **deferred, not foreclosed**. Its trigger is a need
to produce an artefact for a target from a machine that cannot run that
target's `bund2`. It would need runtime discovery, a version check the
runtime acts on, and a new answer to the bullet above, and it would be a new
decision.

No golden moves and no code changes: this ratifies what is built.

## D117 — the hot path's cost is read in instructions retired

**Raised 2026-10-08** by the acceptance reviews of RFC-0005 (B5) and RFC-0008
(B2): D113 added a branch at the frame push, a check before every native and
a check at dispatch, after both documents' cost criteria were measured. And
RFC-0008's criterion 12 records that this host's wall-clock spread on the
sharpest row went 0.5% → 2.6% → 8.4% across three sessions, so a 5% band can
no longer be resolved by timing here.

- Blocks: nothing
- Depends on: D70, D113, F134, F135, RFC-0005 criterion 7, RFC-0008 criteria
  11 and 12
- Status: **RESOLVED** by the repository owner, 2026-10-08, as recommended:
  decide the method before measuring, and prefer a count to a clock.

### The decision

A claim that a change to the interpreter's hot path stays inside a band is
shown by **instructions retired**, read with `/usr/bin/time -l` on two release
binaries running the same program, the cost of an empty program subtracted
from both. Five runs each; the median is the reading.

**The 5% band is kept.** It was chosen against timing noise and a count has
far less: on every looping row below the lowest of five runs is within 0.11%
of the median. A tighter band could be held. Tightening it is a separate
decision and is not taken here.

**The median, because one run in five can still be off.** The highest run of
the empty program was 13% above its median on three of the four binaries, and
one looping row had a run 2% high.

### What a count does not say

- **It is not time.** A branch that mispredicts and a load that misses cost
  cycles and no instructions. A change that adds few instructions and is slow
  would pass this and fail a clock. Whether D113's three sites are of that
  kind, this method cannot show.
- **It is one host.** Apple M5 Pro, Darwin 27.0.0, arm64. `/usr/bin/time -l`
  is macOS's; Linux reads the same counter with `perf stat`.
- **It is the CLI end to end, not `bund2-bench`'s in-process groups.** The
  programs below are the `dispatch` group's four shapes run 300,000 times
  inside `times`, plus a call to a user's word. `value`, `arith`'s cold rows
  and `corpus` have no reading by this method.

### The first reading: D113, before and after

`dff32be` (the commit before D113) against the tree of 2026-10-08. Instructions
retired, median of five, the empty program's cost subtracted.

| program, ×300,000 | Tier 0 before | Tier 0 after | change | `jit` before | `jit` after | change |
|---|---|---|---|---|---|---|
| `1 drop` | 474.8 M | 488.6 M | **+2.89%** | 317.6 M | 317.0 M | −0.17% |
| eight literals, eight `drop` | 2,929.9 M | 2,977.1 M | **+1.61%** | 1,318.9 M | 1,314.2 M | −0.36% |
| `1 dup drop` | 828.0 M | 846.8 M | **+2.26%** | 711.7 M | 716.5 M | +0.67% |
| `1 2 + drop` | 949.6 M | 966.7 M | **+1.79%** | 328.9 M | 328.6 M | −0.07% |
| `f`, where `:f { 1 drop } register` | 710.9 M | 736.0 M | **+3.53%** | 591.0 M | 602.3 M | +1.91% |

The empty program: 37.4 M before, 36.9 M after; 37.8 M and 37.7 M with `jit`.

**Every row is inside the band, and the cost is not nothing.** On Tier 0 one
turn of `1 drop` costs 46 instructions more than it did and one turn of eight
literals and eight `drop`s costs 157 more — about 16 for each further native
called, which is more than "a branch" suggests. With a tier the compiled
bodies do not pass the three sites and the reading is flat, except where a
word is called by name.

**The comparison is of two trees, not of one change.** Between them lie all of
D113, D114 and D115 and the replacement of `println!` (F179). None of the last
three is on these programs' path.

### The second reading: the tier against no tier, today

RFC-0005 criterion 7 compares a build with `jit` against one without. Same
tree, 2026-10-08, same method:

| | no tier | `jit` | change |
|---|---|---|---|
| the empty program (`startup`) | 36.9 M | 37.7 M | **+2.2%** |
| `1 drop` ×300,000 | 488.6 M | 317.0 M | −35% |
| `1 dup drop` | 846.8 M | 716.5 M | −15% |
| `1 2 + drop` | 966.7 M | 328.6 M | −66% |
| a user's word | 736.0 M | 602.3 M | −18% |

`startup` is inside the band and nothing in `dispatch`'s shape regresses.

## D116 — `bund2 build` refuses a program that does not parse

**Raised 2026-10-08** by the acceptance review of RFC-0006 (B3). The
behaviour was built with the bundle, held by
`a_syntax_error_fails_the_build_and_writes_nothing`, listed by D82 under
"what is not settled", and never ruled on.

- Blocks: nothing
- Depends on: D76, D82, RFC-0006 §B3
- Status: **RESOLVED** by the repository owner, 2026-10-08: ratified as
  built.

### The decision

`bund2 build` parses the program and writes nothing if it does not parse. The
error names the file, line and column and says that nothing was written.

**What it changes.** A `script` run finds a syntax error when it runs. A
bundle's author finds it when they build, and the person running the artefact
never does. That moves *when* an error is seen and *who* sees it, which is
observable, and is why it needed a sentence from the owner.

**What it does not change.** The artefact carries source, and the bundle
parses it again when it starts (D77). So this is a check at build time and
not a different representation, and a bundle that was built runs the text
`script` would run.

No golden moves: no golden builds a bundle from a program that does not parse.

## D115 — a bundle is never given a debugger, by a flag or by a word

**Raised 2026-10-08** by the acceptance review of RFC-0006 (B5). D113.5 lets
the first arming or moving debugger word attach a console. The bundle front
end's comment said a bundle is never dropped into a debugger, and it was
written before a word could do it. Measured before this decision, a bundle of
`"a" println debug.step "b" println`: with input closed it printed `a`, the
console's banner, `detached`, `b`; with input open it stopped and ran a typed
line.

- Blocks: nothing
- Depends on: D113 (part 5), D76, D78, RFC-0006 §B3, RFC-0008 §W5
- Status: **RESOLVED** by the repository owner, 2026-10-08, as recommended.
  Built the same day.

### The decision

**In a bundle the arming and moving words do nothing**: `debug.break`,
`debug.break.if`, `debug.watch`, `debug.watch.workbench`, `debug.delete`,
`debug.step`, `debug.next`, `debug.finish` and `debug.continue`. They take
their operands, attach nothing, stop nowhere and leave the tier on.
`debug.run` runs its string and offers no stop. They do not refuse: a
`debug.break` left in a program is not a reason for the shipped program to
fail.

`Interp` gains `console_refused`, which a front end sets to say this VM is
never to have a debugger. The bundle path sets it and gives no console
factory. A `script` run is unchanged.

**Why not let the author decide, as `input` does.** A program that calls
`input` waits because reading is what it is for. A program that reaches a
breakpoint its author forgot waits for a reason nobody at the keyboard can
see, for as long as its input stays open. The debuggable form of the same
program is `bund2 script --file`.

### What it does not cover

- **The views still print** — `debug.backtrace`, `debug.stacks` and
  `debug.info`. They read the VM and wait for nobody.
- **`debug` and `debug.shell` still read lines and run them**, in a bundle
  and under `--noeval --noio`. They are the reference's words and it
  registers both ungated
  (`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:155`,
  `reference/Bund/src/stdlib/functions/debug_fun/debug_shell.rs:67`). That is
  D79's boundary and RFC-0006 §B3a now names it.

`a_bundles_debugger_words_do_nothing` holds it, with a line waiting on an
input that is kept open.

**Note, 2026-10-08 (D120).** "`debug.run` runs its string and offers no stop"
holds without `--noeval`. Under the flag — built in, or added by
`BUND2_NOEVAL` — it is a stub, in a bundle and in a `script` run alike.

## D114 — `send` refuses a value nested past the wire format's bound

**Raised 2026-10-08** by the acceptance review of RFC-0007 (B1). The
refusal has existed since the bus words were built: `send` encodes through
the codec F118 bounded. RFC-0007's criterion 4 and its open questions both
say such a refusal "would be new behaviour" needing a decision, and no entry
recorded it for the bus.

- Blocks: nothing
- Depends on: F118, D37, D39, D87, RFC-0007 criterion 4
- Status: **RESOLVED** by the repository owner, 2026-10-08, as recommended:
  the refusal stays, as an approved deviation.

### The deviation

Measured 2026-10-08, release binaries, a list nested by
`list N { drop list push } times`, sent and received on one channel:

| levels | oracle | Bund2 |
|---|---|---|
| 256 | sends and receives | sends and receives |
| 257 | sends and receives | `SEND returns error Error enveloping data: the value nests 257 deep, and 256 is the most the wire format can carry` |
| 301 | sends and receives | the same refusal |

The prefix is the reference's own for an encode that failed
(`reference/Bund/src/stdlib/functions/bus/mod.rs:114`); the reason after it
is Bund2's.

### Why it stays

The codec descends once per level in both directions, and a value that
crossed unbounded would abort the VM that decoded it — not the one that sent
it. F118 measured that abort at about 3,400 levels for the world file, and
the owner bounded the format at 256 on 2026-09-12. The bus is the same
encoder with a second VM on the far side, so the argument is stronger there:
the failure would land in a thread that did nothing wrong.

**No golden moves.** No corpus program sends a value 257 levels deep.
`a_value_nested_past_the_wire_bound_is_refused_at_send` holds both sides of
the bound.

## D113 — debugger words: one vocabulary for a script, a shell and the console

**Raised 2026-10-07** by the repository owner's idea that the debugger's
commands be words, and drafted as RFC-0008's amendment of this date.

- Blocks: RFC-0008's amendment "debugger words", every part
- Depends on: D84, D94, D112, D36, D55, F165, F139
- Status: **RESOLVED** by the repository owner, 2026-10-07 — all six as
  recommended. Parts A and B built the same day. *(As raised:
  OPEN, six questions, each default for planning only.)*

The reference's debugger evaluates every typed line as Bund in the program's
VM (`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:81-95`) and
has no stepping or breakpoint commands. Bund2's `--debugger` console has
those commands and refuses Bund. The amendment proposes thirteen `debug.*`
words and a console that evaluates Bund, in two parts: A, inspecting and
feeding; B, arming and moving from a script.

| # | question | default, for planning |
|---|---|---|
| 1 | Adopt the direction at all — words as the debugger's vocabulary, the console evaluating Bund? | yes |
| 2 | Part A alone first, or A and B together? | A first |
| 3 | Where do the inspecting words write: stdout, as `debug.display_stack` does, or the session's stderr? | stdout from a word; the console's short forms keep stderr |
| 4 | `stack` at the console: the console's render, or the Bund alias of `ensure_stack`? | the Bund word; the render becomes `debug.display_stack` and `st` |
| 5 | A breakpoint armed by a script with no `--debugger`: refuse, inert with a notice, or a console over the VM's `Input`? | a console over `Input` |
| 6 | A breakpoint armed while a tier is installed: refuse to arm, uninstall the tier from then on, or pin per word? | uninstall from then on |

**Risks the amendment names and does not resolve.** A typed line can change
the program and can exit it, which §D3 forbade for conditions and the
reference allows for typed lines. Evaluating at a watch stop happens inside a
native that is mid-push; whether that is sound is unread (Q41). Whether a
breakpoint on a native such as `input` stops before it runs is unmeasured, and
`debug.feed`'s interactive use rests on it.

**Dated note, 2026-10-07 — ruled, and Part A built.** The owner took all six
recommendations: adopt the direction; Part A first; a word writes to standard
output and the console's short forms keep standard error; `stack` at the
console is the Bund word and `st` is the session's view; a script's breakpoint
with nothing attached opens a console over the VM's `Input` (Part B); arming
while a tier is installed uninstalls it from then on (Part B).

*What Part A is.* Four words — `debug.backtrace`, `debug.stacks`,
`debug.info`, `debug.feed` — and a `--debugger` console that runs any line it
does not know as Bund in the program's VM. `Vm` gains `debugging` and
`feed_line`; `Command` gains `Eval`; `Interp` gains a queue of fed lines and
an `evaluator` the embedder installs, which `Runtime` fills.

*What building it settled.*

- **Q41, first half: a line may be typed at every stop**, with one guard that
  reading had not found. A watch stops inside the push of the native that is
  pushing, and that native may have filed a body to run when it returns. A
  failing native in the typed line cleared that request (F96's clear) and the
  body was silently dropped. `evaluate_typed` now sets the request aside for
  the line and puts it back. A unit test builds exactly that case and fails
  without the guard.
- **Q41, second half: a breakpoint on a native never fires.** Measured:
  `break input` arms and is not reached, because §D3 breaks at a frame push
  and a native pushes none. So a read is fed by stepping to it, or by feeding
  ahead. A stop-on-read stays undesigned. Pinned by a test.
- **The debugger is no longer taken out of the interpreter for a typed
  line.** It is put back marked busy, so `debug.info` typed at a stop reads
  what the console armed, and nothing inside the line is itself a stop.
- **A typed line's failure is shown in the word's own words**, not behind
  `bund.eval`'s `Attempt to evaluate value …`, which carries a raw rendering.
  `bund2_stdlib::eval_line` is the same loop without that prefix.
- **`debug.info` says what is armed and not the stepping mode**, which the
  draft's table had also promised. The mode is the session's and changes
  with every command; nothing reads it.

*What a person at the console will notice.* A mistyped command is now an
unknown word, reported, where it was "not a command" with the list. `?` and
`help` print the list. `stack` runs the Bund word, which makes a stack and
moves to it.

*Checked by* nine tests at process level in `debugger_words.rs` (W1, W2, W3,
W5, W6, the native breakpoint, `stack`), four unit tests on the words, two on
the interpreter. Criterion 6's sweep passes unchanged, with no exclusion (W4).

**Dated note, 2026-10-07 — Part B built.** Nine words arm and move:
`debug.break`, `debug.break.if`, `debug.watch`, `debug.watch.workbench`,
`debug.delete`, `debug.step`, `debug.next`, `debug.finish`, `debug.continue`.
They work in a script, in `debug.shell` and at a stop, on one state.

*D113.5 as built.* The first arming or moving word in a run with no
`--debugger` attaches a debugger quietly — running, not stopped — whose
console reads through the VM's own `Input`. At a terminal that is a prompt
with editing and a history named `bund2_debugger_history.txt`, which is §D8
for this console at no further cost. With no input it detaches. The embedder
supplies the console (`Interp::console_factory`), because its commands are
parsed and its conditions run by code the interpreter does not have; a VM
given none refuses those words in words.

*D113.6 as built.* The first such word turns the tier off for every body
from then on. **Not uninstalled, as the ruling's words had it**: the tier is
taken out of the interpreter while it runs, so a word called from compiled
code finds nothing to drop, and the tier's statistics stay readable. The
effect ruled on is the effect built — no body is offered to it again.
Checked with a tier: `w` is compiled, then armed, and the stop fires; with
the rule removed the same test fails, because a body is offered to the tier
before its breakpoint is looked at.

*Four things the build decided that the ruling did not.*

- **A detach is final.** Once the host has gone nothing stops and nothing
  armed fires. Before, a breakpoint hit after a detach said where it was to
  nobody, once a hit, for the rest of the run. This changes `--debugger` too:
  after end-of-input on standard input, armed breakpoints are silent.
- **`debug.break.if` takes its condition as a STRING of Bund source**, where
  the amendment's table said LAMBDA. The condition runs in a child VM so it
  cannot change the program (§D3); the child is handed text, and a lambda
  value has no source form to hand it. The string may hold a lambda literal.
- **The arming words print nothing.** The console's `break w` answers with
  what is armed; a word in a script must not write into the program's output.
- **`debug` is unchanged.** RFC-0008's Preservation row says it becomes a
  wrapper that steps into things. `"…" debug` is still the reference's loop
  over top-level terms; a script that wants the other now writes `debug.step`.
  Whether `debug` itself should change is a deviation from the reference's
  word and was not ruled on.

*What it does not do.* The body a compiled word was running when it armed
finishes compiled, with no stop inside it; every body entered after is Tier 0.
A breakpoint on a native still never fires.

*Checked by* seven more tests at process level (a script's breakpoint,
`debug.step`, running on with nobody there, W7, the moving words, a
conditional, W8) and one unit test of the refusals.

**Dated note, 2026-10-07 — `debug`, ruled.** The bullet above left open
whether `debug` should change. The owner was shown three options — leave it
and withdraw RFC-0008's row; make it the stepping wrapper, a deviation from
the reference's word; leave it and add a word — and ruled the third.

- **`debug` stays the reference's word**
  (`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:81-95`).
  RFC-0008's Preservation row for it is withdrawn.
- **`debug.run` is added**, a fourteenth Bund2-only word: a STRING of Bund,
  evaluated with a stop offered before each of its terms. `s` goes into the
  bodies the terms enter and `n` does not.
- **Why a word was needed and not a recipe.** `debug.step` before `bund.eval`
  stops inside the words a string calls and at none of its own terms: the
  terms reach the VM through `apply`, where neither of the VM's loops has a
  safepoint. Measured before the ruling, and asserted in the word's test.
- **`Debugging` gains `Term`**, the safepoint a word offers. No new `Vm`
  method.
- **Typed at a stop, the string is not stepped**, since nothing in a typed
  line is a stop.

Also built, on the same instruction: the console says
`bund2: detached; the program runs on.` when its input ends. The console was
driven at a pseudo-terminal — prompt, editing, history, the program's own
`input` and `password`, Ctrl-C and Ctrl-D — and that was all it lacked. No
test in the suite holds a terminal.

Health after: suite 622 passed, 0 failed; conform 133/145, ceiling 133/145;
implemented 500/505; coverage 489/505.

**Dated note, 2026-10-07 — a breakpoint on a native and on an alias; a
terminal in the suite.** Both on the owner's instruction.

- **"A breakpoint on a native still never fires", above, is closed.** A
  native stops where it is about to be called, operands still on the stack.
  The choice of *before* was the builder's: it is what a condition is shown
  for a lambda, and it is the only point at which a line fed to `input` can
  reach the read.
- **An alias stops too**, under the name it was armed by. Found while testing
  the first: `break dup` armed a name nothing is called under, because a
  frame and a native are known by the word an alias resolves to. It held for
  lambdas as much as natives and predates this work.
- **`tests/terminal.rs` drives the binary at a pseudo-terminal** — prompt,
  recall, history, Ctrl-C and Ctrl-D, `input` and `password`, `--debugger`.
  One dev-dependency's features, `rustix`, already in the lock file.

Not built, though asked for in the same message: `stdin`, `stdin.` and
`fs.is_file.`. **They are not work**, and the list that offered them was
wrong. F150 and F152 settled it on 2026-10-04: the reference binds all three
only under `--noio`, as the stub that refuses, and Bund2 does the same.
Adding them to the default build would invent words the language never had.

## D112 — the input seam: a word asks the VM for its line

**Authorised by the repository owner, 2026-10-07**, on being shown four
options: a minimal seam shaped by F165, a fix for F165 in the CLI alone, the
full seam a TUI would want, or keeping D101's deferral.

- Blocks: nothing
- Depends on: D99, D101, D36, F158, F165, RFC-0008 §D8
- Status: **RESOLVED**, built 2026-10-07. Amends RFC-0002.
- **Ends D101's deferral.** Its trigger was a second implementor, and it
  named F165 as one.

### The decision

`bund2-api` gains `Input` — a line, a line that is not echoed, and a note of
which lines a history should keep — and `Vm` gains `read_line`,
`remember_line` and `read_secret`. `input`, `input*`, `debug`, `debug.shell`
and `password` ask the VM. An interpreter holds an input beside its reporter
and starts with none. RFC-0002's amendment of this date is the design.

### The three implementors

| who | answers with | why |
|---|---|---|
| the CLI, a plain run | the terminal: `rustyline` and `yapp`, as before | nothing a user sees changes |
| the CLI, under `--debugger` | no input: every read is the end | **F165** — see below |
| a test | lines given in advance | the words' other arms can be reached |

### F165 is closed, and how

The debugger takes its commands from standard input and the reading words
took their lines from the same stream, so a program that read input ate the
session's commands and never finished. **A debugged program is now given no
input of its own.** Its reads end at once, which is what a capture gives it,
so stepping changes nothing about what it does.

`stepping_agrees_with_an_uninterrupted_run_over_the_whole_suite` excluded the
two suite programs that read input and said why. It excludes none now, and
still finds the two from their source — to show they were stepped.

**What this does not give: a way to type into a debugged program.** That needs
a second channel, and which — a file named by a flag, a second descriptor, a
session command that feeds a line — is a choice not made here. The seam is
what any of them plugs into.

### What changed that a reader might not expect

- **The terminal moved from the words to the embedder.** `bund2-stdlib` still
  holds the `rustyline` and `yapp` code, as `terminal::Terminal`, an `Input`;
  the CLI installs it. D99's two helpers are still the only two reads and its
  scan still says so. Its test-build stub is still there and is now the second
  lock: the first is that an interpreter has no input until given one.
- **One editor per history, kept for the program**, where each word used to
  open its own. So recall carries from one `debug.shell` to the next.
- **A history is written as each line is remembered**, where it was written
  when the word returned. An editor that outlives the word has no such moment.
- **A terminal that will not open is reported as `INPUT line returns: …`**,
  where it was `INPUT returns: …`. One failure path where there were two.

### What checks it

Six unit tests type into the words through a scripted input: `input` with a
line, with a prompt that is not a string, and at the end of input; `input*`
over three lines and with a lambda that is not one (F108); `password` with a
secret and with no terminal; `debug.shell` over four lines, one failing and
one empty; `debug` reading between the values of its snippet; and an
interpreter given no input running all of them without waiting.

The terminal itself was tried by hand through a pseudo-terminal: `input`
answered the line typed and `password` showed a dot a key and answered the
secret. No test does that, and none did before.

## D111 — D24 applied to the words that take one value

**Authorised by the repository owner, 2026-10-07**, on being shown four
options: the one-operand value words, every fixed-effect consumer, only the
half-finished families, or leaving D24 deferred.

- Blocks: nothing
- Depends on: D18, D24, D14, F73
- Status: **RESOLVED for one operand**, built 2026-10-07. Two and three
  operands are open, and are this entry's last section.

### Where D24 stood

D18 fills missing workbench forms; D24 bounds it to words that source a
primary operand and was deferred until a stack-effect table existed, with no
forms to be added by hand before then. The table exists, and Bund2's registry
declares each word's effect, which is what this was counted from.

Of 377 base words, 117 had a `.` form and 260 did not:

| declared effect | words | |
|---|---|---|
| consumes nothing | 76 | producers — none, by D24 |
| opaque: runs a body, or variadic | 48 | not addressed by D24 |
| consumes one | 88 | candidates |
| consumes two or three | 47 | candidates, once "primary" is defined |
| `$` | 1 | not a word of this kind |

### The decision

**58 of the 88 one-operand words get a workbench form**: those that take a
value and answer a value.

- `math.` — `abs acos asin atan cbrt ceil cos cosecant cosh exp factorial
  floor fract ln log10 round signum sin sinh sqrt tan tanh`
- `string.` — `camel lower snake title upper`
- `len not type type.of ptr unwrap seq is curry context csv compile
  resolve.class time.timestamp generator.sample`
- `json json.from_value json.to_value`
- `graph.allpath graph.transitiveclosure`
- `?alias ?class ?effect ?lambda ?object ?stdlib ?word alias= lambda= lambda!
  var?`

**The other 30 get none**, because a workbench form of them would be
meaningless or a second name for something that exists: the stack words
(`drop dup dup_one return return_to to_stack to_current stack ensure_stack
stack_exists rotate_stack_left rotate_stack_right`), words run for a side
effect (`log.* sleep sleep.seconds rm fs.rm raise debug.dump password
bus.data sqlite`), the registry's (`unalias unregister var-`), and
`fs.is_file`, whose workbench name F150 already settles.

### The contract, for one operand

**The operand is taken from the workbench and everything the word answers
goes back to the workbench.** It is D24's shape with no second operand to
place, which is why one function serves all 58 (`bench_form`,
`crates/bund2-stdlib/src/wb.rs`). The plain word runs unchanged over the
operand, so the two cannot drift: they answer the same, refuse the same, and
word a refusal the same.

Three things follow, and each is a choice recorded here:

- **A word that leaves its operand in place leaves it on the workbench.**
  `len` answers beside its operand, so `[ 1 2 3 ] . len.` leaves the list and
  then `3` on the workbench.
- **A refusal leaves the stack as it was.** Whatever the plain word would have
  left behind goes back to the workbench.
- **An empty workbench is reported under the dotted name**, `Workbench is too
  shallow for inline MATH.SQRT.` — the sentence the reference's own workbench
  forms use, and the one message this adds.

### What checks it

Nothing in the oracle can: the reference has none of these words. Three unit
tests run all 58 over eight operands — that each form answers what its plain
word answers and leaves the stack beneath alone, that a refusal restores, and
that an empty workbench is reported. The list of 58 is written out in the
test, so dropping a registration fails it.

IMPLEMENTED and COVERAGE do not move: both are counted over the reference's
words, and these are not among them. `PROMOTABLE.txt` gains them.

### Open: two and three operands

47 words — the comparisons, `and or`, `get set ?key`, `pair complex wrap tag
attribute`, `math.nroot math.power math.perimeter`, `concat_with_space`, and
the rest. D24 says the primary operand comes from the workbench and the
others from the stack, and does not say which operand is primary. The
reference's existing forms do not agree with each other on it (F73): three
shapes are in use. This needs a rule per family and the owner's ruling
before any is written.

### Dated note, 2026-10-07 — two and three operands, ruled and built

**Authorised by the repository owner**, on being shown four options: two
rules by family, the top operand always, every operand from the workbench, or
stopping at one operand. The owner took the first.

**What the reference's own forms do**, measured over twenty of its
two-operand workbench forms by trying each arrangement: seventeen take one
operand from the workbench and one from the stack — the arithmetic, `push.`
and its kin, `merge.`, the `string.` forms — and three take both from the
workbench (`at.`, `head.`, `tail.`). Where only one arrangement is accepted,
the workbench's operand is the one the plain word has on top. Where the
answer lands was not measured; F73 records that both sides occur.

**The ruling.** One operand from the workbench, the rest from the stack, the
answer to the workbench — and which operand is by family:

| family | the workbench holds | words |
|---|---|---|
| works on a container | the container | `set` `get` `?key` `tag` `attribute` `wrap` `concat_with_space` `json.path` `graph.path` `graph.paths` `?is` `?type` |
| anything else | the operand the plain word has on top | `==` `!=` `<` `<=` `>` `>=` `and` `or` `pair` `complex` `math.nroot` `math.power` `math.perimeter` `seq.asc` `seq.desc` |

27 words, and five aliases that follow their targets: `≠.` `⩽.` `⩾.` `∈.`
`sp.`. So `dict . "a" 1 set. "b" 2 set.` builds a dict on the workbench, as
`push.` builds a list.

Where the container sits among a word's operands differs by word and was
read from each one's source: beneath the key for `get` and `?key`, beneath
key and value for `set` and `tag`, **on top** for `wrap`, `json.path` and the
two graph queries. The slot is written at each registration.

**Two corrections to what the owner was shown.** `seq.asc` and `seq.desc`
were listed with the container words; they take three numbers and have no
container, so they are in the second family. `∈` was listed as a word of its
own; it is an alias of `set`.

**Not given a form**, and not asked about: `generator` and
`generator.sample*`, which were not classified; and `,`, the other alias of
`set`, because `,.` would read as the keep suffix.

**Given none, by the ruling:** `swap` `swap_in` `swap_one` `move_from`
`ensure_stack_with_capacity` `alias` `register` `var` `cp` `mv` `fs.cp`
`fs.mv` `save.model`.

**One consequence worth knowing.** With the top operand on the workbench, a
comparison reads its *left* side from the workbench: `3 . 4 <.` asks whether
3 is less than 4 only because the plain word's top is its left operand. It
follows from the rule and may not read that way.

**What checks it.** Four more tests. One runs all 32 names with operands that
answer and compares each with its plain word. One puts the operand in the
*other* slot for every container word and requires a different answer, which
is what says each registration's slot is the right one. One builds a dict and
a text buffer up on the workbench. One covers the second message the form
adds, `Stack is too shallow for inline SET.`, for operands the stack was to
supply.

**A mistake the effect audit caught.** Every form was first registered with
the effect `(0, 0)`, which is right for one operand and wrong for several: a
form of several operands takes the others from the stack. The audit that
checks declared effects against a palette refused ten of them when the owner
regenerated `PROMOTABLE.txt`. Each now declares what it takes from the stack,
all its operands but one, and leaves nothing there. The unit tests written
for the forms did not catch it; they compare answers and not declarations.

- Status: **RESOLVED.** Of the 260 words that had no workbench form, 85 have
  one. The rest have none by D24, by this entry, or — the 48 opaque words —
  because nothing has asked.

## D110 — a complex number is a valuemap key by its parts

**Authorised by the repository owner, 2026-10-07**, on F177's question.

- Blocks: nothing
- Depends on: D30, F29, F177
- Status: **RESOLVED**, built 2026-10-07.
- **Amends D30** in one row.

D30 hashes by content exactly what the reference compares by content, and
lists four kinds: integers, floats, strings, times. It leaves "the other
sixteen" to identity. The reference compares a fifth by content — two complex
numbers, both parts (`reference/rust_dynamic/src/eq.rs:47-52`) — which D30
missed because that arm is reached through the tag and not the payload.

**Two complex numbers with equal parts are equal, and hash alike.** So
`valuemap 1.0 2.0 complex "z" set 1.0 2.0 complex get` answers `z`; it
answered `key not found`. As a key the parts compare the way a float key
does: every NaN is one NaN and `-0.0` is `0.0`, because a key that is not
equal to itself cannot be found.

D30's reason for leaving composites to identity does not reach this. That
reason is cost and reach — hashing a list of any size for an equality that is
then decided by identity. A complex number is two floats and is a scalar in
every way but how it is stored. **A PAIR of the same two floats is still a
list**, equal only to itself, and a test says so.

The `==` word is untouched; it has compared complex numbers by both parts
since F169, by IEEE, where NaN is not equal to NaN. That difference between
the word and the key is D30's own, stated there for floats.

The oracle cannot be asked: its hash is the id (F29), so it finds no key of
any kind.

## D109 — `unique` drops what is equal to something kept, for every kind

**Authorised by the repository owner, 2026-10-07**, taking the recommendation
as written: a member is dropped when an equal one is already kept, first
occurrences stay in order, the word never refuses, and the Fibonacci search
leaves Bund2.

- Blocks: nothing
- Depends on: D107, D100, D30, F149, F164
- Status: **RESOLVED**, built 2026-10-07.
- **Supersedes D100 in two places**, named below.

### The decision

`unique` keeps a set of what it has kept and asks it. D107 did this for a
list of strings; this does it for everything, so the word is one rule and not
three. The port of `algos`' Fibonacci walk, its sortedness test, and the test
of that walk are removed.

What "equal" means, by kind:

| kind | equal when |
|---|---|
| integers, floats | they denote the same value — `==`'s answer (D30) |
| times | the counts are equal |
| complex numbers | both parts are |
| text — a STRING or a CALL | the characters are |
| NaN | never, as `==` has it |
| a list, a dict, a lambda, anything else | never: the reference equates these by id, and two members of one list do not share one |

### The equality between an integer and a float

**This row was the recommendation's and the owner was not asked it as its own
question; it is recorded so that it can be overturned on its own.** The
alternative is the reference's, which D100 kept: a new integer is compared
with a kept float by truncating the float, so `[ 1.9 1 ]` answers `[ 1.9 ]`,
while `[ 1 1.9 ]` keeps both. Bund2 now keeps both either way, and `1` with
`1.0` is one member either way. `unique` and `==` agree.

**Dated note, 2026-10-07 — confirmed by the owner**, on being shown both
implementations run on seven lists. `1` and `1.0` are one member in either
order in both, and the first seen is the one kept. The two differ on
`[ 1.9 1 ]`, where the oracle answers `[ 1.9 ]` and Bund2 keeps both, and on
`[ 3 2.0 3.0 2 ]`, where the oracle keeps all four. The row stands as built.

### What D100 said that no longer holds

- *"Disordered numbers fail at the third member"* — kept on purpose there.
  They are no longer refused. `[ 2 1 1 ]` answers `[ 2 1 ]`.
- *"Equality between an integer and a float is not symmetric"* — kept on
  purpose there. It is symmetric now.

D100's refusal to give `BundValue` a global `Ord` stands, and so does its
answer for floats, which this generalises.

### Measured

300 generated integer lists, one to ten members from six values, a third of
them sorted first: 162 identical to the oracle and 138 where the oracle
refuses. None where the two both answered and differed. Bund2's answer was the
list's first occurrences in all 300. D107's 300 string lists stand.

### What it costs

Nothing new in conformance: every refusal ends a program, so no golden holds
one, and `unique-words` — already a deviation under D107 — prints what it
printed. A program that relied on `requires sorted input` no longer gets it.

## D108 — `sort` has one order for a list of any kinds

**Authorised by the repository owner, 2026-10-07**, taking the recommendation
as written: order by kind first, then by value within the kind — numbers,
then times, then text, then everything else — and refuse nothing.

- Blocks: nothing
- Depends on: D106, D33, D38, F47
- Status: **RESOLVED**, built 2026-10-07.

### What the reference does

Its `>` and `<=` answer `true` for any two values of different kinds
(`reference/rust_dynamic/src/ord.rs:94,102,110,121` and
`:55,63,71,82`), so each is both greater than and not greater than the other
and the sorted list is whatever the swaps left. That includes **an integer
against a float**: `[ 3 1.5 2 0.5 1 ] sort` is not in order there, although
`<` on the same two values has had one answer in Bund2 since D33.

### The decision

The comparator is a total order.

| rank | kind | within it |
|---|---|---|
| 1 | integers and floats, together | by the value they denote (D33); NaN last, and equal to itself |
| 2 | times | by count |
| 3 | text — strings, pointers, text buffers | by code point (D106) |
| 4 | everything else | no order; they come out together at the end |

`[ "b" 2 "a" 1.5 1 "c" 0.5 ] sort` answers `0.5 1 1.5 2 a b c`. The quicksort
is still D38's transcription, so members that compare equal may still change
places — which now matters only for rank 4, where the order among a list, a
dict and a lambda is fixed for a given input and means nothing.

### Three things decided inside it

- **The rank is a convention.** Numbers before text is the common one. Nothing
  in the reference suggests an order between kinds.
- **NaN sorts last among numbers.** It has no value; the reference's float arm
  answers `false` to every comparison with it, which is not an order either.
- **A complex number sorts as its real part, among the numbers.** The
  reference orders two of them that way (`ord.rs:40-42,118-120`).
  `[UNGROUNDED]` as behaviour: no word was found that puts a complex number in
  a list — `+` refuses to append one — so the oracle could not be asked.

### What it costs

No golden. No captured program sorts a list of mixed kinds or of integers
with floats; `workbench-variants`, already a deviation under D106, prints what
it printed. Any program that does sort such a list gets a different answer
from the reference's, and a defined one.

## D107 — `unique` over strings leaves no repeat

**Authorised by the repository owner, 2026-10-07**, on being shown three
options: a true de-duplication, refusing an unsorted string list as numbers
are refused, or leaving it.

- Blocks: nothing
- Depends on: D100, D106, F149, F164
- Status: **RESOLVED**, built 2026-10-07.

### What the reference does

`unique` asks whether each member is already kept with a Fibonacci search,
which wants sorted data (F149). For numbers of one kind it is told when the
data is not, and refuses. For strings the test it makes first is `<=`, which
answers `true` for any two strings (`reference/rust_dynamic/src/ord.rs:48-85`),
so it is never told, searches an unsorted list as if it were sorted, and
misses: `[ "c" "b" "a" "c" "a" ] unique` answers `[ c b a a ]`. The same
every run. D100 recorded this and kept it.

### The decision

**While everything kept is text, a text member is dropped when an equal one
is already kept.** First occurrences stay, in the order they came, and nothing
is refused. `[ "c" "b" "a" "c" "a" ]` answers `[ c b a ]`.

It is D100's rule for floats — *found, if and only if some kept member is
equal* — extended to strings. D100 could say it was choosing among the
reference's answers, because for floats the reference has several. **Here it
has one, and Bund2 declines it.** This is a deviation in the plain sense.

Measured over 300 generated string lists, one to fourteen members over
alphabets of three, six and ten letters: 149 identical to the oracle, 151
different, and in every one of the 151 the oracle's answer still held a
repeat. Bund2's answer was the list's first occurrences in all 300.

### What it does not change

- **Numbers.** A disordered list of integers is still refused at its third
  member, as the reference refuses it. Strings and numbers therefore behave
  differently under one word; they did before, less visibly.
- **A list that mixes kinds** is searched as it was. The direct question is
  asked only while every kept member is text, so `[ 1 "a" 1 ]` answers what it
  answered.
- **What counts as text.** A STRING and a CALL, as D100 has it, which is why
  `[ true true false ]` is two members.

### What it costs

One golden. `tests/probes/unique-words.bund` runs the five-member list above
and its label says what the oracle does with it — "the second c goes, the
second a stays". The label is left alone: the golden is the oracle's record,
and it is true of the oracle. It needs
`cargo xtask conform --accept-deviation probes/unique-words.golden --reason D107`,
after which conformance reads 133/144 against a ceiling of 133.

## D106 — `sort` orders strings, by code point

**Authorised by the repository owner, 2026-10-06**, on being shown three
options: code point order, a language's collation, or leaving it. The owner
stated the intent first — a list of strings is to sort by its alphabet — and
took code point order.

- Blocks: nothing
- Depends on: D38 (the transcribed quicksort), F12, F74, F175
- Status: **RESOLVED**, built 2026-10-06.

### What the reference does

`sort` is `algos`' quicksort
(`reference/Bund/src/stdlib/functions/values/sort_lists.rs:36`), which compares
with `>` and `<=`. `Value` overrides both, and each has arms for integers,
floats and times and answers `true` for everything else
(`reference/rust_dynamic/src/ord.rs:48-85`, `:87-124`). Every string is
therefore greater than every other and also not, and the result is whatever
the partition's swaps leave: `[ "b" "a" "c" ]` sorts to `[ c a b ]`, and
thirteen fruit names to `plum cherry peach lime lemon kiwi fig apple date
grape melon mango pear`. The same every run — it is a shuffle, not chance.

The reference does have a string ordering. `Ord::cmp` compares two strings by
value (`ord.rs:186-191`). The sort does not call it.

No register entry recorded an intent for this. F74 says in passing that `sort`
is not alphabetical; D38 transcribed the sort to match the oracle's order,
strings included. It was preserved because it matched, not because anyone
chose it.

### The decision

**Two values that both hold text order by code point.** That is the
reference's own `cmp` arm, reached. It asks about the payload and not the tag,
as that arm does, so pointers and text buffers order among themselves and with
strings. Nothing else about the sort changes: same quicksort, same threshold,
same instability among equals.

Code point order is not a dictionary's, and this entry does not claim it is:
the empty string first, `"10"` before `"9"`, capitals before small letters,
an accented letter after `z`. Collation by a language was the other option
and was not taken; it would need a collation library and a ruling on where the
language comes from.

### What it does not settle

- **A list of mixed kinds still has no order.** A string against a number
  answers `true` both ways, as before. The output for such a list does move —
  `[ "b" 2 "a" 1.5 1 "c" 0.5 ]` was `0.5 c 1 1.5 a 2 b` and is now
  `0.5 1 1.5 a 2 b c` — because its strings now order among themselves, and
  neither answer means anything.
- **`unique` is untouched.** It asks whether a list is already ascending with
  the same `<=`, and for strings still hears yes (F149). Whether it should
  follow is a separate question, because there the answer decides a refusal.

### What it costs

One golden. `tests/probes/workbench-variants.bund` sorts `[ "c" "a" "b" ]` on
the workbench and the oracle prints `[ b a c ]`; Bund2 now prints `[ a b c ]`.
That is this deviation and needs `--accept workbench-variants --reason D106`.
Conformance reads 134/144 until it is accepted and 134/144 against a ceiling
of 134 after.

### Found on the way: times did not sort

The time arm was left out of Bund2's comparator when Bund2 had the tag and no
way to make a value of it. D103 made them and the comparator was not revisited,
so a list of times came back unsorted where the oracle orders it — measured:
5 1 9 3 7 is `1 3 5 7 9` there. A Bund2 defect, fixed here; no golden held it.

## D105 — error wording: match what shared code decides, record the rest

**Authorised by the repository owner, 2026-10-06**, on being shown three
options and their risks.

- Blocks: nothing
- Depends on: F171 (the survey), F18, D57, F68, D36
- Status: **RESOLVED**, built 2026-10-06.

### The question

F171's survey ran 61,016 programs on both implementations. In 14,637 of them
both refused and said so in different words. That matters to a program that
reads error text through `?try`, and to nothing else.

### The decision

Match the three families whose wording comes from shared code; record what is
left. Not all of it (the last part is per-word reading for wording alone, where
a slip is likeliest), and not none of it (error text has been reproduced
elsewhere all session because `?try` reads it).

1. **The stack layer's wrapper.** Every stack-layer word reports a failure as
   `VM inline function returned error: {err}`
   (`reference/rust_multistackvm/src/multistackvm_inline.rs:59`), and inside
   it names the operation as that function's author typed it — `dup_in` for
   both `dup_one_in` and `dup_many_in`, `rotate_stack_left` for
   `rotate_stack_right`. Each native in `crates/bund2-stdlib/src/stack.rs` is
   registered through a shim that adds the lead-in.
2. **`FLOAT_OP`.** Eight `math.*` words share one function and report under
   its name, not their own (`reference/Bund/src/stdlib/functions/math/math.rs`).
   Bund2 said `MATH.EXP returns error`, which reads better and is not what the
   reference says. The owner was told this makes the message less helpful.
3. **Cast failures.** `This Dynamic type is not string` where Bund2 said
   `not a string` or dropped the reason; the tag-naming form of `cast_list`;
   and which of two string families numbers its operands
   (`returned for #2:`) and which does not (`returns:`).

### Measured

| | before | after |
|---|---|---|
| programs agreeing, of 61,016 | 42,022 | 55,938 |
| both refuse, different words | 14,637 | 1,665 |
| words agreeing on every program, of 263 | 83 | 143 |

### What is left, recorded and not matched

| count | what | why it stays |
|---|---|---|
| 757 | `$`, and words only Bund2 has (`?is`, `?effect`) | the oracle says `not registered`; nothing to match |
| 651 | shallow-stack wording on about twenty words | **F18**: Bund2 guards at the arity a word consumes and says `Stack is too shallow`, where the reference pulls first and says `NO DATA #2`. Decided, and a golden holds it |
| 175 | `convert.to_dict` | D57 |
| 82 | scattered: `var?`, `lambda=`, `decode.base64`, `display` | per-word wording, not read |

### Two things the work corrected in itself

**Part of it was undone because it contradicted F18.** The first pass lowered
the depth guard on the string-distance, regex and two-operand math words to
reproduce `NO DATA #2`. F18 had already decided those words guard first. The
guards were restored; only the cast wording changed.

**A golden caught a transposition.** Rewriting `string.regex`'s operand casts
swapped the subject and the pattern. `f18-arity-words` failed and conformance
fell by one until it was put right. No survey would have shown it as a
*message* difference — the answers were simply wrong — which is the argument
for the goldens over the survey as the thing that gates a change.

### Estimate against outcome

The owner was told the cast family was "one cast helper". It was about 56
message sites. The decision would have been the same; the estimate was wrong.

## D104 — where the reference aborts: refuse the incoherent answer, keep the coherent one

**Authorised by the repository owner, 2026-10-06**, on being shown three
options.

- Blocks: nothing
- Depends on: D37, D98, F68, F47, F168, F170
- Status: **RESOLVED**, built 2026-10-06.

### The decision

F170 found two places where the reference aborts and Bund2 returned a value
nobody had chosen. They are settled differently, and on one test: **is the
quiet answer a meaning, or an accident?**

1. **A complex number compared with a plain one is refused.** With a CFLOAT on
   top and an INTEGER, FLOAT or TIME underneath, every comparison word now
   fails with `COMPARE: unsupported operand #2` — the gate's existing sentence
   for a second operand the first cannot be compared with
   (`crates/bund2-stdlib/src/logic.rs`, `compare`). Bund2 had answered `<` and
   `>` both true of the same two values, which is F47's fallback and not an
   answer. D98's treatment.
2. **A string repeated a negative number of times stays the empty string.**
   That is what repeating zero or fewer times conventionally means, it is what
   Bund2 has always done, and the reference itself takes a negative *float*
   count as zero. F68's treatment.

### What it does not change

The other orientation — a plain number on top of a complex one — does not
abort the reference. Its integer or float arm answers (`false` to `==`, `true`
to an ordering), and Bund2 answers the same.

### Not a standing rule

The owner was offered "reference aborts, Bund2 reports" as a uniform policy
and chose case by case instead. A third case is a third question.

## D103 — TIME is a payload of its own, and does as little as the reference's

**Authorised by the repository owner, 2026-10-06**, on being shown the
options.

- Blocks: nothing
- Depends on: D37, F155, F166
- Status: **RESOLVED**, built 2026-10-06.

### The decision

`time.now` and `time.timestamp` are implemented over a new
`Payload::Time(u128)` (`crates/bund2-value/src/lib.rs`), rather than left
unimplemented or carried as a tagged integer.

### Why a payload and not a tagged INTEGER

`Val::Time` holds a `u128` (`reference/rust_dynamic/src/create.rs:175`), and
`time.timestamp` casts its operand `as u128`, so `-1 time.timestamp` is the
largest one. An `i64` under a TIME tag cannot hold that, and
`debug.display_stack` renders the payload by name — `data: Time(n)` — so the
representation is observable, as it was for MATRIX.

It is a heap payload because `BundValue` is sixteen bytes and a `u128` is
sixteen alone.

### What a TIME value does

Only what the reference gives it arms for, each measured against the oracle:

- **Equality and the four orderings**, by nanosecond count, against another
  TIME (`reference/rust_dynamic/src/eq.rs:37-43`,
  `ord.rs:27-33,66-72,105-111,144-150`). Against anything else, equality is
  false.
- **Not printed and not converted.** `conv` never names `Val::Time`, so
  `println` and every `convert.to_*` reach its final refusal,
  `Can not convert Value from 13` (`reference/rust_dynamic/src/conv.rs:743`).
- **Not added to.** With the TIME on top, `numeric_op` reaches its last arm:
  `Incompartible X argument for the math operations: 13`
  (`reference/rust_dynamic/src/math.rs:369-405`). Underneath a number, the
  number's arm refuses it with the `Y` sentence. The one admitted form is
  `+` onto a LIST, which appends.
- **It crosses the wire.** `Val::Time` was the kind the wire test used as its
  example of a kind Bund2 lacks; that test now uses `Embedding`.

Nineteen programs compared with the oracle, nineteen agreeing.

### What it does not settle

Nothing in the reference turns a TIME back into a number or a string, so the
value is close to write-only. That is preserved, not repaired: adding a
conversion would be a language decision nobody has asked for.

`time.now` reports a clock set before the epoch. The reference unwraps that
read and aborts (`reference/rust_dynamic/src/value.rs:11-13`), so this is the
D37 treatment — a report where the reference panics — and not a new deviation.

## D102 — RFC-0008's remainder: the sweep, then history; the trace waits

**Authorised by the repository owner, 2026-10-06.**

- Blocks: nothing
- Depends on: D94 (RFC-0008 Proposed), F10, F165
- Status: **RESOLVED**; the first two parts built 2026-10-06.

Three pieces of RFC-0008 were unbuilt or unfinished, and they are not alike.

1. **Criterion 6's suite-wide sweep — done first.** It verifies code that
   already exists, costs a test, and could find defects. It found one (F165)
   and met the criterion for 134 of 136 programs.
2. **§D8, history in a config directory — done second.** F10's disposition was
   FIX. It was also a gap in work done that week: `debug` and `debug.shell`
   had been implemented writing no history at all.
3. **§D7, the execution trace — deferred.** A hook in the frame loop behind a
   flag, emitting a stream nothing reads. It has no criterion over it (D94
   says so) and no consumer; building it now would be designing an interface
   for a user who does not exist yet.

## D101 — the input seam waits for its second implementor

**Authorised by the repository owner, 2026-10-06**, on being shown three
options.

- Blocks: nothing
- Depends on: D99, D36, F158, F165
- Status: **RESOLVED — deferred**, with a named trigger.

### The decision

The seam D99 records as owed — terminal words reading through the VM as they
report through it — is **not built now**. It is built when there is a second
implementor to design it against.

### Why not now

F158 is closed and enforced, so nothing is broken that the seam would mend. A
seam designed with one implementor usually has the wrong shape, and what a
TUI needs of it is not yet known: structured prompts, cancellation, history,
whether reads are asynchronous. `Reporter` was shaped by having the CLI and
the tier both consume it.

### What changed the day this was decided

**A second consumer turned out to exist already.** F165, found the same day:
the debugger and the debugged program share standard input, so a program that
reads input cannot be debugged. That is the seam's problem stated without a
TUI — two readers that need two channels.

It does not reverse this decision, and it does sharpen it. The trigger is no
longer only "when a TUI is built": **whichever comes first, a TUI or the wish
to debug a program that reads input**, is the second implementor, and F165 is
the specification of the minimum it must do. RFC-0008's `Console` is already
half of this — an input abstraction for the session — and the amendment to
RFC-0002 should be designed together with it rather than beside it.

**Dated note, 2026-10-07 — the deferral is ended by D112.** The owner took the
minimal seam, shaped by F165 as this entry said it should be.

## D100 — `unique` gets a comparator of its own, and one answer where the reference has several

**Authorised by the repository owner, 2026-10-06**, on being shown the
grounding and three options.

- Blocks: nothing
- Depends on: F12 (disposition FIX), F149, F164, F74
- Status: **RESOLVED — decided and built**, 2026-10-06.

### The decision

`unique` is implemented with a comparison local to the word. **There is no
`impl Ord for BundValue`**, and none was needed: F149 recorded that the word
"needs a decision about `BundValue: Ord`", reading the bound on the crate the
reference calls as a requirement on Bund2. Bund2 does not call that crate. It
ports the Fibonacci walk and gives it the three comparisons the walk makes.

**Where the reference compares by value, Bund2 is identical** — integers and
strings, including the walk's quirks over an unsorted accumulator. Checked
against the oracle over 300 generated lists: all identical, 64 of them the
"requires sorted input" error.

**Where the reference compares by random id, Bund2 answers by the
reference's own equality.** Its ordering has no arm for a float, or for an
integer against a float, and falls through to comparing ids — so its walk
takes arbitrary branches and the answer changes between runs (F164). There
the question is put directly: *found, if and only if some kept member is
equal by the reference's `==`*. Floats themselves are ordered by value, which
is F12's disposition and was already FIX.

### Why this is not a deviation in the usual sense

A behaviour that differs between two runs of the reference is not a contract
— F74's reasoning for `string.tokenize.unique`, applied here. No golden could
hold it; a capture that agreed twice by chance would be F155's trap. So Bund2
is not choosing against the reference's answer. It is choosing *among* them,
and it chooses the one a reader of the word's name expects.

### What is preserved that looks wrong

Three things, each the reference's and each kept:

- **Disordered numbers fail at the third member.** The accumulator's order is
  tested when it is next searched, so `[ 2 1 ]` is returned untouched and
  `[ 2 1 1 ]` is an error.
- **Strings are never refused**, and the search over them skips:
  `[ "c" "b" "a" "c" "a" ]` drops the second `c` and keeps the second `a`.
- **Equality between an integer and a float is not symmetric.** A new integer
  is compared against a kept float by *truncating the float*, so
  `[ 1.9 1 ] unique` answers `[ 1.9 ]`; a new float is compared by widening
  the integer, so `[ 1 1.9 ] unique` keeps both.

### What was declined

- **A global `Ord` for `BundValue`.** It would have settled ordering for
  every type and every caller to satisfy one word, which F149 itself warned
  against.
- **Leaving it unimplemented.** The blocker recorded was not real.

**Dated note, 2026-10-07 — superseded in part by D107 and D109.** Two of the
three things listed under "What is preserved that looks wrong" are no longer
preserved: disordered numbers are not refused, and an integer and a float are
equal when they denote one value, in either order. The third went with D107.
The Fibonacci walk this entry ported is removed. What stands is the refusal
to give `BundValue` a global `Ord`, and the answer for floats.

## D99 — standard input is read in two helpers, and a scan says so

**Authorised by the repository owner, 2026-10-05**, on being shown F158 and
four options.

- Blocks: nothing
- Depends on: D36 (words report, they do not print), D39, F141, F158
- Status: **RESOLVED — decided and built**, 2026-10-05.

### The decision

Every read of standard input in `bund2-stdlib` goes through one of two
helpers in `terminal.rs`: `Terminal::line`, for `input`, `input*`, `debug` and
`debug.shell`; and `secret`, for `password`. Under the crate's **own test
build** both answer without touching standard input — the first with
end-of-file, the second with a refusal.

`every_terminal_read_goes_through_the_two_helpers` scans the crate for
`rustyline`'s read, `yapp`'s and the standard library's handle, and fails if
any appears outside them.

### Why this one

F158 was the second time. F141 had recorded the same hang a month earlier,
fixed the path it found, and left a rule — give `cargo test` a closed stdin —
that nothing checked. The rule was true and it failed anyway, through a
harness F141's fix did not cover. **So the criterion for this decision was
enforcement**, not coverage: an option that depends on someone remembering was
the option that had just been tried.

### What was declined

- **The input seam on `Vm`, now.** The right shape, and owed — see below. It
  changes RFC-0002's public trait and the interpreter and CLI behind it, which
  is a language-runtime change to close a test-infrastructure hang.
- **Redirecting descriptor 0 at test start.** The only option closed by
  construction against readers not yet written. `unsafe`, a `libc`
  dev-dependency, Unix-only, and Rust's test harness has no global setup
  hook, so it needs either a `ctor`-style initializer or a list of tests that
  remember to call it. (`unsafe` is not new to the workspace — the JIT crates
  use it — though it would have been new to this crate. An earlier statement
  of this option called it "introducing `unsafe`", which overstated it.)
- **A `cargo xtask test` wrapper.** Today's mitigation with a name. It
  protects whoever uses it and documents the hazard for everyone else.

### What it costs

Under test, the four words meet a stub and not `rustyline`. They exercise
their **end-of-input arm**, which is the only arm a test or a capture has ever
reached — the golden runner gives a program stdin at `/dev/null`. No test
could type before, so no test lost anything; but the claim "tested" for these
words means that arm and no other, and it is better said here than assumed.

### What remains owed

**An input seam.** `Vm::report` is the seam a TUI implements for output. A TUI
needs the same for input, since a word that calls `rustyline` on raw standard
input cannot run inside one. This decision does not build it. The two helpers
are the consolidation it needs — five call sites reduced to two — and where it
would plug in. It will be an amendment to RFC-0002 when it is taken up.

## D98 — `io.textfile` reports where the reference aborts, and truncates where it truncates

**Authorised by the repository owner, 2026-10-05**, on being shown F161 and
three options. It confirms what `3f17eea` had already built under D37; the
ruling is recorded because that commit applied D97's reasoning to a second
defect without one.

- Blocks: nothing
- Depends on: D37 (Bund2 does not abort), D97, F161
- Status: **RESOLVED**, 2026-10-05.

### The situation

Invalid UTF-8 in a file `io.textfile` reads has two outcomes in the reference,
and which one depends only on how many readable lines precede it:

| file | reference |
|---|---|
| `\xff\n` — none | `[]` |
| `a\n\xff\nz\n` — one | `[ a ]`, silently; `z` is lost |
| `a\nb\n\xff\nz\n` — two or more | **aborts**, `index out of bounds`, exit 101 |

D37 forbids the abort. The question was what replaces it, and whether the two
truncating rows should change with it.

### The decision

**Report on the abort case only.** With two or more readable lines before the
bad one, Bund2 fails with `IO.TEXTFILE returns error: The line starting at
byte: … and ending at byte: … is not valid UTF-8`, the crate's own description
of the line. With zero or one, it answers what the reference answers,
truncation included.

**An error is the nearest survivable thing to an abort**, and that is the
reason rather than tidiness. A crash and an error both *stop the program*, so
in neither is a caller handed partial data it believes is whole. Replacing the
crash with a truncated list would make a program **continue** where the
reference stopped — a larger change in what happens than replacing a crash
with an error.

### What was declined

- **Truncate everywhere** — stop at the first unreadable line and answer what
  was read, at any count. It has the best argument of the three and it should
  be stated fairly: the reference's own loop says `_ => break`
  (`reference/Bund/src/stdlib/functions/io/textfile.rs`), so truncation is
  what the author *wrote*, and the abort is an accident of the crate beneath.
  Declined because it extends silent data loss to every file: a log with one
  corrupt byte on line 5,000 would answer 4,999 lines and say nothing, where
  the reference at least stops.
- **Report everywhere** — an error on any unreadable line. No cliff and no
  silent loss, but it deviates from behaviour the reference actually has in
  the zero- and one-line rows, and no golden could witness it.

### What it costs

**A cliff at two lines.** Whether a corrupt file is truncated quietly or
refused loudly depends on how many good lines come first, which is arbitrary
from where a user stands. It is the reference's cliff — truncate-versus-crash
there, truncate-versus-error here — kept in the same place and made
survivable, not introduced.

## D97 — `io.textfile` reports an error where the reference never returns

**Authorised by the repository owner, 2026-10-05**, on being shown F160 and
three options.

- Blocks: nothing
- Depends on: D39 (an internal loop must be bounded), D37 (Bund2 does not
  abort), F160
- Status: **RESOLVED**, 2026-10-05.

### The situation

The reference's `io.textfile` does not return on a file that **begins with a
line terminator** (F160): `easy_reader`'s `build_index` loops without end and
allocates as it goes. This was put to the owner as "a file of only blank
lines", which is the input it was found on; the boundary was tested afterwards
and a single leading newline is enough. The decision does not depend on which. D39 already forbids reproducing that — "a hang is worse than a panic" —
so *whether* Bund2 terminates was never the question. What it **answers** was,
because the reference answers nothing and there is no behaviour to preserve.

### The decision

**Bund2 fails with an error naming the condition.** The read is bounded by
the file's size — a file of *n* bytes holds at most *n + 1* lines, and F159
adds one — and when the reader has not reached end-of-file within that bound,
the word reports `IO.TEXTFILE returns error: the reader did not reach the end
of the file`.

The message describes the *mechanism* and not the input, deliberately — and
that turned out to matter within the hour. F160 was first measured on `\n`
and `\n\n` and described as "only blank lines"; the real condition is a
*leading* terminator, so `\nlead\n` hangs too. A message naming blank lines
would have been wrong for that file. Naming what the reader failed to do is
right for both.

**How the bound is applied, which the first implementation got wrong.** The
scan `build_index` runs is run first, bounded, on a reader of its own; only if
it ends at end-of-file is `build_index` then called. Driving `next_line`
un-indexed and stopping there — the first attempt — terminates correctly and
answers *differently*: its last element is an empty string where the
reference's indexed replay gives F159's duplicated line. Eight of twelve file
shapes disagreed with the oracle before that was caught.

### What was declined, and why

- **Answering the intended lines** — one empty string per line. It is what a
  caller wants, and it is Bund2 deciding what a word means where the reference
  is silent. It would also sit oddly beside F159, where a trailing newline
  *duplicates* the last line rather than being read cleanly.
- **Extrapolating F159's shape** — the blank lines, then the last again with
  its terminator. Consistent, and a guess at what broken code would have
  produced had it not been broken.

An error invents no semantics, and `?try` can catch it — which a silently
invented list could not be told apart from a real answer.

### What it costs

A legitimate file of blank lines is **refused** rather than read. That is the
price of not inventing an answer, and it is the same file the reference cannot
process either.

## D96 — the class hierarchy is extended additively, and `?is` is added

**Authorised by the repository owner, 2026-10-05**, answering RFC-0010's three
questions on the options that document set out.

- Blocks: nothing. RFC-0010 moves to Proposed on this
- Depends on: RFC-0009 (the mechanism), D23, D25, D72 (Bund2 may add a word)
- Status: **RESOLVED**, 2026-10-05.

### 1. Additive only — the hierarchy is not rewired

`List`, `Floats` and `Intervals` are registered with the parents the reference
declares: `List` under `Value`, the other two under `List`. **No existing
ancestry changes**, so `bool-objects.golden` is untouched and nothing deviates.

The Smalltalk-shaped alternatives — `Magnitude`, `Number`, `Collection`,
`Boolean` — are **declined**, and RFC-0010 §S2 records what each would have
cost. The asymmetry worth keeping in view: inserting `Magnitude` above
`Integer`/`Float` would be *invisible*, because neither class has a
constructor word and no program can build one, while moving `Bool` moves a
golden. A deviation with no present symptom is the harder kind to remember,
which is part of why the additive answer was taken.

So Bund2's hierarchy is the reference's, root-first: **`Display` ←
`Printable` ← `Object` ← `Value` ← {`Integer`, `Float`, `Bool`, `List`} ←
{`Floats`, `Intervals`}**. `Object` is *not* the root; the two most abstract
classes are output concerns. That is written down in RFC-0010 rather than
corrected in code.

### 2. `?is` is added, under D72

The reference has **no membership test**. `is` pushes the object back together
with its `.data` and says nothing about classes; `?object` answers only
whether a value is an object at all. `?is` takes an object and a class name
and answers a BOOL, walking the ancestry construction already materialised in
the same depth-first order `locate` uses — so it agrees with dispatch by
construction rather than by a second traversal written to match.

Named with the `?` predicates, and deliberately **not** `is`, which is taken
by the unrelated word above.

**It can never be covered by a golden**, and that is a standing consequence of
D72 rather than a gap in this change: a golden is captured from the *oracle*,
which has no such word. So `?is` joins `$`, `<-`, `←`, `password` and
`log.error` in `implemented but run by no golden`, and its verification is a
Rust test. `noop`, D72's first exercise, has the same property; this is the
first time it has been written down.

### 3. `iset` is taken for `Intervals`

`iset = "0.3.3"`, the oracle's version, and it has **no dependencies of its
own** — unlike `rnltk`, which F-less precedent in `library_string.rs` declined
for pulling `nalgebra`.

Two behaviours are the reason it is a dependency rather than a `Vec` scan, and
both are now pinned by `tests/probes/oop-collections.bund`: a repeated range
is **skipped** rather than replacing the first, and when ranges overlap the
answer is the **lowest-starting** one rather than the first stored. Measured
on both engines: `[ 2.0 8.0 ]` stored before `[ 1.0 5.0 ]`, probed at `3.0`,
answers `1..5`. A linear scan in insertion order would answer `2..8`.

### What this does not disturb

`is`, `wrap`, `unwrap`, `#`, `#.`, `True`, `False`, `?object`, `object` and
`class` are untouched. Conformance gains one golden and loses none.

## D95 — COVERAGE's numerator requires a golden

**Authorised by the repository owner, 2026-10-05**, on being shown F156 and
the two figures.

- Blocks: nothing. It changes what a reported number means, not any rule
- Depends on: D21 (probes), and CLAUDE.md's definition of the health metric
- Status: **RESOLVED**, 2026-10-05.

### What is authorised

`cargo xtask coverage`'s numerator counts a word only when Bund2 registers it
**and a golden-backed program runs it** — a `HERMETIC.txt` entry, or a probe
that has a `tests/golden/probes/<stem>.golden`. A probe without a golden
contributes nothing, which is what the report already claimed.

The **ceiling keeps the full mention set**: every corpus program and every
probe, captured or not. That is the bound it always was — no word outside it
can be exercised by a golden whatever Bund2 implements — and it is a property
of the corpus, which is why it is the ceiling and not the numerator.

The two are now separate variables rather than one `used` set behind both.
F156 happened because they shared it, so the distinction CLAUDE.md draws had
no representation in the code.

### What it costs

**COVERAGE drops from 467/505 (92.5%) to 460/505 (91.1%)** on the tree where
the decision was taken. Nothing regressed; seven words stopped being counted
as tested because nothing tests them:

`?ifthenelse`, `?key`, `debug.display_hostinfo`, `ls`,
`math.securerandom.int`, `rm`, `string.random.name`

They move into `implemented but run by no golden`, which is the worklist, and
that is the honest place for them. `debug.display_hostinfo` is the one worth
naming: its only mention is `bund_shell.bund`, which calls `bund.prompt` and
so can never be captured.

**CORE COVERAGE moves for the same reason**, and by the same words: all seven
are core (D14), so it reads **278/286 (97.2%)** where it read 285/286
(99.7%). `core words not implemented` stays at 0 — nothing became
unimplemented; eight core words are implemented and exercised by no golden,
`$` being the one that already was.

A number that falls on being corrected is the point of having it. The
alternative on offer was to record F156 and leave the figure alone, and the
owner declined it.

## D94 — RFC-0008 is Proposed

**Authorised by the repository owner, 2026-10-04**, after one adversarial review
of the document and the implementation of its design sections.

- Blocks: nothing. It changes the document's status, not any rule
- Depends on: D36, D44, D45, D52, D53, D84, D90 — and D6, which D84 closed
- Status: **RESOLVED — Proposed**, 2026-10-04.

### What is authorised

§D1 through §D6 as specified and built: the safepoint, the frame's symbol, the
three breakpoint forms, the two watchpoint hooks, the inspection words Bund2
already had, and the tier declining to install itself under the debugger. The
`log.*` words and `debug.dump` with them. **All thirteen criteria are
discharged** — eleven met, one deferred with a named blocker, one withdrawn for
naming no check.

### What is specified and **not** built, with no criterion over it

Two sections, and neither has a criterion, which is why they are named here
rather than left to a reader of the criteria list:

- **§D7's execution trace.** "A separate stream behind a flag, emitting
  `(depth, symbol, value, stack effect)` per step." No flag exists. The
  section's own correction stands — `eval_observed` cannot drive it, because the
  observer is called from the top-level loop only — and §D1's safepoint is now
  the hook it would use.
- **§D8's history.** The reference writes
  `bund_debug_debugger_history.txt` in the working directory; §D8 makes it a
  deviation and puts it in the platform's config directory (F10, disposition
  FIX). Nothing is written anywhere: the debugger console reads plain lines and
  keeps no history, which its module doc says in terms.

So a debugged session today steps, breaks, watches and inspects, and does not
trace or remember. That is the whole gap and it is bounded.

### Two things the criteria record as owed rather than met

Carried here because a status line is what gets read:

- **Criterion 6 says "every suite program" and four were used.** The
  differential runs `s`, `n` and `f` to completion against a plain run over four
  programs chosen to cover what the safepoint has to reach. The sweep over the
  whole suite belongs in `xtask conform`, beside the capture that already
  enumerates them.
- **§D2's span half is withdrawn from criterion 12.** A frame carries the symbol
  it was pushed for; it carries no source position, because
  `lower_with_spans` is top level only and producing per-value spans is
  RFC-0003 §S5's IR, which does not exist. A backtrace names a word and a value
  index. The span's cost is owed to whichever work produces spans.

### What the review and the building cost, recorded because it is the argument

The adversarial review found that §D1 was blocked on **D6**, which the first
draft treated as settled — "the failure CLAUDE.md names in terms".

**Four criteria were wrong as written, and every one was caught by building
rather than by reading:**

| criterion | what it asked | what is true |
|---|---|---|
| 2 | `log.error`'s line matches the oracle's | the line carries a wall-clock timestamp and the reference's own Rust module path; only *which words are silent* is checkable (D90) |
| 3 | all seven `DUMP:` texts match | five are vacuous — each sits behind the tag check that guarantees its cast |
| 5 | coverage moves by ten | it moved by five, and the other five are named word by word |
| 10 | `compiled_entries()` reads `Some(0)` | it reads `None`: §D6 declines to *install* the tier, which is the stronger statement |

That is the pattern this RFC is the clearest instance of: the criteria were
written from the design, and the design was right about mechanism and wrong
about measurement four times. **A criterion that has not been run is a claim,
not a check.**

## D93 — RFC-0007 is Proposed

**Authorised by the repository owner, 2026-10-04**, after one adversarial review
of the document and the implementation of everything in it.

- Blocks: nothing. It changes the document's status, not any rule
- Depends on: D84, D85, D86, D87, D88, D91, D92 — the seven decisions the RFC
  needed, all RESOLVED
- Status: **RESOLVED — Proposed**, 2026-10-04.

### What is authorised

The eight bus words as specified and built, §C8's VM host and its `async`
façade as specified and built, and **the rest of the document as specification
only**. Nothing here authorises zenoh: §C4 stays deferred under D28 and D87, the
`--distributed` flag is unbuilt, and the question of how a channel address
selects a transport is still open.

### The thing that makes this different from D59 and D82

RFC-0005 and RFC-0006 were preservation documents: the oracle could adjudicate
every criterion, and a disagreement was a bug in Bund2. **§C8 is design.** The
reference has one VM and spawns no threads, so its bus is a queue from a VM to
itself, and criteria 9–14 are the first criteria in this repository that the
oracle cannot settle either way.

So what "Proposed" means here is narrower, and saying so is the point:

- Criteria **1, 2, 3, 5, 6 and 8** are preservation, measured against the
  oracle, and mean what they mean in any other RFC.
- Criteria **9–14** are design. They say what Bund2 does and that a test holds
  it. They cannot say the reference agrees, because the reference has no
  opinion.

**A reader must not take criteria 9–14 as preservation claims**, and a later
session must not cite them as evidence that the reference behaves some way. The
criteria themselves say "this section is design, not preservation" and are
marked throughout; this decision repeats it because a status line is what gets
read.

### What the review cost

One document review, four blockers — **two of them errors in measurements the
document claimed to have taken against the oracle**. That is the finding the
review was worth: not that the design was wrong, but that two numbers in it had
not been measured and were stated as if they had.

Two further corrections came from implementation rather than review, and both
were in the criteria rather than the code: criterion 4 was a deviation filed as
preservation and is withdrawn, and criterion 7 named no check and could not
fail, so it is withdrawn too.

### The status line that preceded this

Recorded because it is the failure this repository has paid for three times. The
previous line said the RFC "cannot be proposed" for two reasons — the words were
out of scope under D28, and nothing in it involved a second VM — and **both had
been false for three sessions** when it was finally corrected on 2026-10-03.
D87 scoped the words in and §C8 was written. A status nothing contradicts is
read as authoritative; see F109's addendum and F146.

## D92 — criterion 13's façade links no executor

RFC-0007 §C8's second half is an `async` façade over one VM, which research (a)
rates "do it". Implementing it needs something to await on, and that is a
dependency question rather than a design one, so it was taken rather than
assumed.

- Blocks: nothing further. It closes RFC-0007 §C8 criterion 13
- Status: **RESOLVED — a hand-rolled `Future`, no executor.** Decided by the
  repository owner, 2026-10-03.

### What was there to build on, measured

**No executor exists anywhere in Bund2's or the reference's tree on a native
target.** `tokio` is absent from `Cargo.lock` entirely. `futures-core` appears,
but only through `js-sys` under `chrono` under `prqlc` — a **wasm-only** path
that never compiles on this target. `reference/Bund/Cargo.toml` names no async
runtime, and nothing in its sources is an `async fn`.

So whichever option was taken would have been the project's first executor, and
D38's rule — pin to the version the oracle's lock resolves — had nothing to pin
to.

### Measured again, 2026-10-03, and it changes the shape of the question

**Four of the five words are goldenable; the fifth can never be.** A golden's
capture folds stderr into stdout (`run_once` in `xtask/src/golden/mod.rs`) and
refuses any program whose two runs differ. Measured on the oracle:

| program | two runs | why |
|---|---|---|
| `log.info`, `log.warning`, `log.debug`, `log.trace` | **identical** | the default filter is `error`, so all four emit nothing, and a golden pins exactly that |
| `log.error` | **differ** | `[2026-10-04T01:15:31Z …]` against `…:32Z …`, the wall-clock second |

So the line this decision is about — the only one any of the five ever prints at
the default level — **is invisible to every golden**, for the same reason
`debug` and `debug.shell` are (criterion 13, `UNSTABLE.txt`). D90 therefore
governs what a *user* sees and nothing a capture can hold.

**Which means a normaliser is the thing that would create the conflict, not
resolve it.** A normaliser for the ISO timestamp would be defensible on F14's
own grounds — a wall clock is not behaviour the reference defines — but it would
make `log.error` capturable, and the golden would then hold
`bund::stdlib::functions::debug_fun::debug_trace`, which is the reference's own
Rust module path and the one field Bund2 cannot honestly produce. **Not adding
the normaliser keeps the question out of the goldens entirely.**

**And the four goldens force the level filter, whichever line is chosen.** They
pin *silence*. A Bund2 that emitted `log.info` where the oracle is quiet would
fail all four, so reproducing `setloglevel`'s filter — default `error`,
`BUND_LOG_LEVEL` honoured — is a requirement rather than a nicety.

**One wrinkle inside option 2.** `Severity` has three rungs and `Error` means
"evaluation stopped here", which `log.error` has not done — so it must map to
`Warning`, not `Error`, or a non-fatal log line would draw the fatal table.
Five levels onto two rungs is lossy, and the sub-options are: collapse
(`error`/`warning` → `Warning`, the rest → `Notice`), losing which word was
called; or keep the level in the reason, `Notice: log.trace: …`, which preserves
it.

### The options, and what each cost

1. **tokio behind the existing `async` feature**, `features = ["rt", "sync"]`.
   D28's letter satisfied, since the feature is off by default. The first
   runtime in the tree, on a version Bund2 chooses alone.
2. **A hand-rolled `Future`** over the VM thread. No dependency, shipped or
   dev. Carries the lost-wakeup hazard.
3. **`futures-channel`'s oneshot**, whose `Receiver` is already a `Future`,
   with `futures-executor` for the test. Three small pure-Rust crates, no
   hand-written waker.
4. **Withdraw the criterion**, documenting that an embedder awaits `Vm<T>`
   through their own executor's `spawn_blocking`.

### Why option 2

**The `Future` trait is in std; what was missing is an adapter, not an
executor.** What this project declines to reimplement is *answers a golden
captures* — comfy-table's box, leon's templates, a hyphenation dictionary,
`hexdump`'s padding. Plumbing between `std::future::Future` and a thread handle
is not in that class, and the comparison to those cases does not hold.

**It keeps `bund2-async` executor-agnostic, which is stronger than
executor-integrated.** Option 1 would pick tokio *for the embedder*. As built,
the same `Vm<T>` awaits under tokio, smol, async-std or a bare `block_on`
without linking any of them, and an embedder already running one is not asked
to link a second. For a crate whose stated job is "integration", being agnostic
is the better position.

**It defers the real dependency decision to where it is forced.** That point is
**async native words** — `bund2-async`'s other stated purpose — which needs a
runtime rather than an adapter. Criterion 13 did not. D28's letter is met by
every option here; its spirit, "nothing non-essential is linked unless asked
for", is best served by linking nothing.

### The hazard, and how it is closed

A hand-rolled future can lose a wakeup: the thread finishes between the poll's
check of the answer and its store of the waker, so a waker is parked that
nothing will ever call. **One mutex over both the answer and the waker closes
the window** — either the poll stored a waker before the thread took the lock,
and the thread wakes it, or the thread finished first and the poll finds the
answer. There is no interleaving in between.

Two further things the design owes, each with a test:

- **A panicking VM must resolve, not hang.** The thread marks itself done and
  wakes through a `Drop` guard, so an unwind takes the same path a return does
  — F57's reasoning about `Frame`'s exit action, applied to a thread boundary.
  It also returns the host slot, or a crash would permanently shrink the host
  and D85's bound would be measuring the wrong thing.
- **The test executor must be able to catch a lost wakeup.** A no-op waker and
  a busy-poll loop would pass even if `wake` were never called. So the harness
  is a condvar built from `std::task::Wake` — safe, which this crate requires
  since it forbids `unsafe` — and it **waits with a timeout**, so a lost wakeup
  fails a test rather than hanging one.

### What reopens this

A decision that async native words are in scope. A runtime then becomes
required, this adapter becomes redundant plumbing, and options 1 and 3 are the
right starting points. **D92's scope is criterion 13 and nothing else**, and it
should not be cited as having settled the runtime question.

## D91 — a shared compile service against D60's per-`Interp` module

§C8 records this and declines to take it: "That is a decision, and it is not
this RFC's to take." It is now a decision entry, so a work item can be blocked
on it rather than discovering it.

- Blocks: RFC-0007 §C8's reach — not whether the second VM can be built, only
  how far several VMs can be driven before the tier wastes work in proportion
  to their number
- Default: **keep the per-`Interp` form**, on the measurement below
- Status: **RESOLVED — option 1, keep per-`Interp` modules.** Decided by the
  repository owner, 2026-10-03, on the measurement below rather than on the
  research's recommendation.

**What is kept and what is owed.** The per-`Interp` module stays, criterion 23
keeps its meaning, and the N× is a known and pinned cost rather than an
oversight. **The magnitude is still owed**: `JITModule` exposes no size, so how
much one compiled body costs is unmeasured, and that figure is what decides
whether N× matters at D85's tens. The owner asked for it to be taken on a quiet
host; `one_compiled_bodys_code_memory` is the harness, and until it has run this
entry's cost column is a multiple without a unit.

**What reopens this is a symptom, and the symptoms have different answers** —
the table at the end of this entry, which is the part worth re-reading before
any work on it. Option 1 being chosen does not make option 2 the fallback;
options 3 and 4 both sit in front of it.

### What the research says, and what Bund2 did instead

`docs/research/01-extensibility-async.md` §2.6 calls the per-VM module "Bad"
under concurrency in terms: "every thread recompiles the same hot words and
every thread's code memory grows independently… N threads means N× the
unbounded growth". It recommends **one shared compile service** — a
`Mutex<JITModule>` on a dedicated task — and notes that only that makes the
code-memory cap "a single enforceable number". §2.7 lists it as "required under
concurrency".

**Bund2 has the per-VM form deliberately.** D60 derives it from a lifetime:
compiled code dies with its `Interp`. RFC-0005 criterion 23 turns that into a
test — "a body compiled for one `Interp` is never run by another", which "fails
for any cache, module or cell shared across `Interp`s". Neither is reversible
by RFC-0007.

### Why the default is the status quo, measured rather than assumed

**N× growth over N VMs that compile nothing is N× zero.** D85 bounds the VM
count at tens; **F139 found that no program in `tests/golden/HERMETIC.txt`
compiles a single body even at threshold 64**, because they run once; and D74
set the shipped threshold to 1024. So the cost the research names is real in
principle and currently unmeasurable in practice.

It is **bounded in practice and unbounded in principle**, since RFC-0005 §S4
reclaims no code memory at all. The trigger, recorded so it is not
rediscovered: **a workload where several VMs each tier up the same hot word.**

Trading criterion 23's invariant for a saving that measures zero is the wrong
order, and it is the order D75 already applied once — D68's crossing was
withdrawn from the shipped build because the measurement said it cost ~1.7 ns a
call rather than saving.

### Measured, 2026-10-03 — the host made the argument a number

§C8's VM host exists, so D91's own "what would change this" is now takeable.
At threshold 1, one hot word, 64 evaluations per VM:

| VMs | bodies compiled | compiled entries |
|---|---|---|
| 1 | 1 | 63 |
| 2 | 2 | 126 |
| 4 | 4 | 252 |
| 8 | 8 | 504 |

**Exactly linear, exactly as research §2.6 predicted.** Each VM compiles the one
body once and enters it 63 times, so what is duplicated is the *compilation*,
not the running. Pinned by
`n_vms_each_compile_the_same_hot_word_which_is_d91s_cost`, so a future change
that shares compiled code fails a test and forces this decision rather than
drifting past it.

### The magnitude, taken 2026-10-03 on a quiet host — and it points elsewhere

Three CLEAN runs of `one_compiled_bodys_code_memory`, 200 distinct bodies each,
resident set size before and after:

| run | RSS delta | per body |
|---|---|---|
| A | 6,016 KiB | **30.08 KiB** |
| B | 6,032 KiB | **30.16 KiB** |
| C | 6,128 KiB | **30.64 KiB** |

**Spread 1.9%** — far tighter than the timing rows RFC-0008 measured, so the
instrument is adequate for the question. All 200 bodies compiled and all 200
were entered in every run.

**30 KiB is not plausible as machine code for `{ i 2 + noop drop }`**, so the
figure was decomposed by varying the body size — the one experiment that
distinguishes emitted code from fixed overhead:

| body | per body | marginal |
|---|---|---|
| 1 × `1 2 + drop` | 31.60 KiB | — |
| 8 × | 38.72 KiB | ~1.02 KiB per extra group |
| 32 × | 58.08 KiB | ~0.85 KiB per extra group |

So the cost is **a fixed floor of ~30 KiB per compiled body, plus about 1 KiB
per unit of body**. Extrapolated to an empty body the floor is ~30.6 KiB, which
matches the first table.

For a realistic small Bund word, **the floor is ~97% of the cost and the body
itself is ~3%.**

### What the floor is — and a retraction

**First reading, wrong and retracted.** This entry briefly attributed the floor
to "one `JITModule` per compiled body", citing RFC-0005 criterion 23's dated
note of 2026-09-13. **That note is superseded two paragraphs later in its own
entry**: `Compiler` holds one `Emitter<JITModule>`, a `JitTier` owns one
`Compiler`, and the module is shared per `Interp` — the work landed, and
criterion 23's module half is met. Citing a passage without reading the ones
that follow it is the mistake CLAUDE.md names in terms, and it was made inside a
single document.

**Confirmed 2026-10-04 by reading `cranelift-jit` 0.135.0**, which the earlier
note left as the leading explanation. It is page rounding at finalisation, and
the mechanism is exact:

- `Memory::allocate` is a **bump allocator**: it serves from `current` when the
  size fits, and otherwise calls `finish_current()` and takes a fresh
  `PtrLen::with_size(size)`, whose size is `region::page::ceil(size)` — an
  `mmap` **rounded up to a page**.
- `SystemMemoryProvider::finalize` calls `readonly.set_readonly()` and
  `code.set_readable_and_executable()`, and **both call `finish_current()`**.
  So finalising closes the code block and the readonly block; the *writable*
  one is not finalised and survives.
- `JITModule::finalize_definitions` calls that provider `finalize`, and Bund2
  calls it **once per compiled body** (`finish_jit` in `lower.rs`), because a
  body must be executable before it can be called.

So each compiled body closes two bump blocks and the next body starts two fresh
page-rounded ones. **At this host's 16 KiB pages that is 32 KiB per body**,
against a measured ~30 KiB — the shortfall being untouched pages, which RSS
does not count.

**It is not configurable away, which is the part worth knowing.** The other
provider, `ArenaMemoryProvider`, pre-reserves one region and suballocates — but
its `allocate_inner` skips any segment that is `finalized` and then allocates a
new page-aligned one, so it pays the same cost by the same rule. Swapping
providers changes the address stability and the failure mode, not the floor.

**The floor is therefore structural in the dependency rather than a Bund2
mistake**, and the levers are only these three: finalise less often, which the
compile-then-immediately-run pattern barely allows; compile fewer bodies, which
D74's threshold of 1024 already does; or cap the total, which is option 4.

### What that means for this decision

**The dominant cost is inside one VM, not across several — and it is not this
decision's.** D91 asks whether to share a compile service *between* VMs. The
floor is paid once per compiled body by a VM that compiles it, so N VMs pay it N
times and sharing a module between them would indeed remove the multiple; but
the same floor is paid 200 times over by a *single* VM compiling 200 bodies, and
nothing in D91's options touches that. **Reducing the floor is an §S4 question**
about how code memory is allocated per compilation, and §S4's lack of
reclamation is what makes it accumulate rather than merely exist.

So the honest reading: D91's N× is real, linear, and now priced at ~30 KiB a
body — ~380 MiB at D85's 64 VMs compiling 200 bodies each. That figure is
dominated by a per-body floor which is **97% overhead on a small word**, so the
first question it raises is not "share between VMs" but "why does a 1 KiB body
cost 30 KiB at all", and that question is answerable without touching any
criterion.

**It also sharpens what the hazard actually is.** The multiple is linear and
immediate; the *unboundedness* is neither, and it comes from §S4 reclaiming no
code memory at all. A long-running VM accumulates code for every body it ever
tiers, and N VMs accumulate N times that. **The multiple is not the hazard — it
scales one.** That is an argument for option 4 which this entry originally
under-weighted.

### The options

1. **Keep per-`Interp` modules.** Criterion 23 intact, compiled code's lifetime
   stays a consequence of ownership, and N VMs that tier up one hot word do N×
   the compile work and hold N× the code memory.
2. **One shared compile service**, as the research asks. It collides with
   criterion 23 head-on: a shared module means a body compiled under one
   `Interp`'s cells being callable from another, which is the failure that
   criterion names. It is not only a refactor — **§S6's mirror cells are
   per-`Interp`**, and compiled code reads them, so sharing emitted code
   requires the cells to become a parameter of the call rather than a property
   of the compiler. That is a change to RFC-0005's addressing, not to RFC-0007.
3. **Share the compile *work*, not the emitted code.** One translation of a
   body to Cranelift IR, cached by the body's `payload_key` (D42, D35), with
   each `Interp` still emitting and owning its own code. Recovers most of the N×
   *compile time* and none of the N× *memory*, and **criterion 23 is untouched**
   because nothing emitted is shared. Worth naming because it is the cheapest
   thing to do first if the trigger appears, and nothing in the registers has
   considered it.
4. **Attack the unboundedness instead of the sharing** — give §S4 a reclaim
   path and a global code-memory cap. Orthogonal to how many modules there are,
   and it is what makes the cap "a single enforceable number" whether or not
   the module is shared.

### The recommendation

**Option 1 now**, and the measurement above is why rather than an assumption:
the N× is real and linear, but D74 ships a threshold of 1024 and F139 found that
no corpus program compiles a body even at 64. The trigger has not fired, and
8 × zero is zero.

**When it fires, which option follows depends on the symptom**, and this entry
declines to pre-commit because the two symptoms have different answers:

| symptom | option | why |
|---|---|---|
| N VMs are slow to start because each compiles the same words | **3** | it removes exactly the duplicated work the measurement found, and leaves criterion 23 untouched |
| a long-running process grows without bound | **4** | §S4's missing reclamation is the actual cause; sharing a module would slow the growth and not stop it |
| both, and 3 + 4 together are not enough | **2** | and only then, because it costs criterion 23 and makes §S6's per-`Interp` cells a parameter of the call |

**Option 4 now has a ready-made mechanism**, found while confirming the cause.
`ArenaMemoryProvider::new_with_size(n)` reserves `n` bytes up front and answers
`pre-allocated jit memory region exhausted` when they are gone, instead of
growing without bound. That is exactly the "single enforceable number" research
§2.6 said only a shared compile service could give — and it turns out a memory
provider gives it per `Interp`, without touching criterion 23 or §S6's cells.
Whoever takes option 4 should start there rather than building a cap.

An earlier version of this recommendation said "option 3 before option 2" and
did not rank option 4 at all. The measurement moved it up: option 4 is the only
one of the four that **bounds** anything, and it works whether or not the module
is shared. The research's claim that only a shared service makes the cap "a
single enforceable number" is about where the cap lives, not whether one can
exist — a per-VM cap of M over N VMs is still a bound, at N·M.

**What would change this further**: not much, now that the floor is understood.
It is a dependency's page-rounding rule and cannot be reduced by configuration,
so the ~30 KiB per compiled body stands until either Bund2 finalises less often
or `cranelift-jit` changes. The N× this decision is about sits on top of a
constant nobody here controls, which is an argument for option 4 over options 2
and 3 that the first version of this entry did not have.

## D90 — what a `log.*` line looks like, now that the oracle's cannot be reproduced

RFC-0008 criterion 2 claims `log.error`'s emitted line is checkable against the
oracle at the default level, and corrects the first draft for denying it. The
claim is half right: **the line exists and only one of the five words emits it**,
which is what the first draft got wrong. But the line itself cannot be matched.

- Blocks: RFC-0008 criteria 1, 2 and 5 — the five `log.*` words are unimplemented
  until this is taken, and they are five of criterion 5's ten
- Default: **none.** Every option below changes what a program's stderr says.
- Evidence: measured against the oracle, 2026-10-01, and again 2026-10-03
- Status: **RESOLVED — option 2**, decided by the repository owner 2026-10-03.
  The five words route through `Vm::report`; `log.error` maps to `Warning`, the
  level is kept in the reason text, and `setloglevel`'s filter is reproduced.

### What was built, and the three things it cost that were not foreseen

`crates/bund2-stdlib/src/logging.rs`. A line reads `Warning: log.error: …` or
`Notice: log.trace: …`; the four quiet words emit nothing at the default level
and `BUND_LOG_LEVEL` raises it. Verified against the oracle: the four quiet
words are byte-identical, both error texts match, and `7 log.info` fails with a
casting error at the default level **even though `log.info` says nothing there**
— the reference pulls and casts before consulting the level, which is
observable and preserved.

**1. The five words are not crossable by promotion, and the honesty test caught
it on the commit that added them.** Each has a *fixed* effect, `eff(1, 0)`, so
`StackEffect::opaque` does not refuse them the way it refuses every other
reporting site — they are exactly `alias`'s case (F137, D71). They are now in
`promotable::REPORTS_MID_BODY`, and a new test asserts the **composition**:
`PROMOTABLE.txt` is D55's half alone and lists them, while `crossable()`
subtracts D71's half, so each is *certified and still not crossable*. A test
reading only the file would have missed it.

**2. Criterion 25's scan matched a doc comment.** It looks for the literal
`Severity::Error` in shipped code, and `logging.rs` *explains* why `log.error`
maps to `Warning` rather than `Error` — so the explanation was reported as the
violation. The scan now skips comment lines, as D71's sibling scan already did.
A comment cannot report anything, so this is strictly more precise rather than
weaker.

**3. `Severity`'s three rungs cannot hold five levels.** `Error` means
"evaluation stopped here", which `log.error` has not done, and criterion 25
forbids a native reporting at `Error` at all — so `log.error` is a `Warning`.
Collapsing the rest would make `log.debug` and `log.trace` indistinguishable in
output, so the level stays in the reason text.

**What it did not cost: a deviation.** As the measurement predicted, none. The
four quiet words' goldens pin silence, which Bund2 reproduces exactly, and
`log.error` appears in no golden because the capture refuses it.

### What the oracle emits, measured

    [2026-10-01T15:35:51Z ERROR bund::stdlib::functions::debug_fun::debug_trace] hello from error

At the default level that is the whole output of a program calling all five
words: `setloglevel` filters at `error`, so `log.error` passes and
`log.info`, `log.warning`, `log.debug` and `log.trace` emit nothing. That half
of criterion 2 is confirmed.

**Two parts of the line are unreproducible, for different reasons.**

- **The timestamp** is wall-clock to the second. That is F14's class — not
  behaviour the reference defines — and a golden would need it normalised, which
  no normaliser does today.
- **`bund::stdlib::functions::debug_fun::debug_trace` is a Rust module path in
  the reference's own source tree.** `env_logger` prints the target of the
  `log::error!` macro, which is the module that called it. Bund2 has no such
  module and cannot have one: the equivalent code is
  `bund2_stdlib::console`. **Emitting the reference's path verbatim would be
  printing a false statement about where the code is**, and emitting Bund2's own
  is a different line.

So the line is not a contract Bund2 can keep, and the words cannot be written
until it is decided what they say instead.

### The options

1. **Bund2's own module path, timestamp normalised.** Byte-identical in shape,
   different in one field. **Now the worst option rather than the obvious one**:
   the normaliser is what makes `log.error` capturable, and the capture would
   then demand the reference's Rust module path. It converts a question no
   golden can see into an approved deviation.
2. **Route them through `Vm::report` at `Warning` and `Notice`**, which is
   Bund2's architecture: "a `Warning` or `Notice` has not stopped the program and
   gets one line on stderr" (D36). The line becomes `Warning: hello from
   warning` — no timestamp, no module path, nothing unreproducible, and a TUI
   receives it through the seam like every other diagnostic. **It is also a
   different line for all five words, and it changes which of them are silent**:
   the reporter has no level filter, so `log.info` would emit where the oracle's
   is quiet, unless the filter is reproduced too.
3. **Emit the reference's path verbatim.** Byte-identical, and a lie: a user
   grepping for the module finds nothing. Recorded because it is the only option
   that satisfies criterion 2 as written, which is itself an argument that the
   criterion is wrong.
4. **Implement the words silently** — accept the message, emit nothing. Cheapest
   and worst: a program that logs its progress would appear to work while saying
   nothing, and the five words would read as covered.

### The recommendation, with what it costs

**Option 2, mapping `log.error` to `Warning`, keeping the level in the reason,
and reproducing `setloglevel`'s filter** — so `log.error` emits and the other
four stay quiet at the default, and `BUND_LOG_LEVEL` still selects.

It puts logging on the seam a TUI will implement, which is the reason D36
exists, and it removes both unreproducible fields instead of normalising one of
them. The level stays in the reason text because collapsing five words onto two
rungs would make `log.debug` and `log.trace` indistinguishable in output, and a
program that logs at two levels to tell them apart would stop being able to.

**What it costs, and the cost is smaller than it first looked: no deviation at
all.** The four quiet words' goldens pin silence, which Bund2 reproduces
exactly, and `log.error`'s line appears in no golden because the capture refuses
it. So coverage moves by four, `log.error` joins `debug`, `debug.shell` and
`password` as named-unreachable, and **nothing needs
`conform --accept-deviation`**. The earlier draft of this entry expected one
deviation covering five words; measurement says none.

**Criterion 2 is wrong either way** and is corrected in the RFC: what is
checkable is *which* words emit at the default level, not what the line says.

## D89 — whether the hermetic funnel's `bus` exclusion narrows the way D87's deferral did

D87 narrowed D28's deferral from the whole `bus` directory to `globals.rs`, and
the narrowing was applied in one of the two places that read the directory as a
unit. **The other is still whole.** `xtask/src/corpus/classify.rs` maps
`Bund/src/stdlib/functions/bus` to `Effect::Bus` under the heading "wholly
effectful subsystems", and `Effect::hermetic` admits only `Pure`, `Stdout` and
`Diagnostic` — so every corpus program that calls `send`, `recv` or `bus.data`
is dropped by the hermetic funnel before any capture is attempted.

One program is affected: `examples/code_snippets/internal_bus_demo.bund`, which
sends fifteen integers to channel `A` and drains them with `bus.data` and
`recv`. It is absent from `tests/golden/HERMETIC.txt` for the directory's sake,
not its own.

- Blocks: RFC-0007 §C8 criteria 2 and 5 — the round-trip golden and the coverage
  figure both want this program
- Default: **none.** Changing the funnel changes which goldens exist, which is
  the owner's.
- Status: **RESOLVED — narrow it, with a new effect rather than a false label.**
  Decided by the repository owner, 2026-10-01.

**`Effect::LocalBus`**, whose `hermetic()` answers true. `crossbus.rs` maps to
it, `globals.rs` keeps `Effect::Bus`, and `mod.rs` — which registers no word —
falls under whichever rule follows, since nothing a program can invoke resolves
there.

**Why a new variant and not `Pure`.** A process-global queue is a side effect,
and `Pure` would have said otherwise in the one table a reader consults to find
out. The enum had no category for "process-global state, no I/O", and **that
missing category is why the directory was read as a unit in the first place** —
D28's deferral, the path audit, and RFC-0007's first draft all collapsed the two
files into one, and the funnel was the fourth place to do it.

Reproducibility is a property of the process, not of the words: a fresh process
starts with `in` and `out` empty and nothing else, and both the capture and
every conformance run are fresh processes. `cargo xtask golden` runs each
program twice and refuses it if the two runs differ, which is the funnel's own
test of that claim and the one the admitted program had to pass.

**What it admitted, measured:** exactly one program. `HERMETIC.txt` goes 57 →
58 and the dropped-by-filter-1 count 52 → 51; nothing else moved.
`internal_bus_demo.bund` sends fifteen integers to channel `A` and drains them
with a `bus.data`/`recv` loop, and it diffs byte-identical between Bund2 and the
oracle. The conformance denominator grows by one under criterion 6's rule,
stated rather than absorbed.

### What narrowing would buy, and what it costs

The local bus needs no network, no file and no clock. A fresh process starts
with `"in"` and `"out"` empty and nothing else, so the demo is reproducible in
the sense the funnel means by hermetic — `cargo xtask golden` runs each program
twice and refuses it if the two runs differ, which is the funnel's own test of
that claim and the one this program would have to pass.

The cost is stated rather than hidden: **the conformance denominator grows**,
116 → 117, and so does `tests/golden/HERMETIC.txt`. Criterion 6 of RFC-0007
already allows that for new probes and gives the reason — a number that absorbs
new tests silently stops being a regression number — but it is still a change
to `tests/golden`, and the capture is the owner's to run.

### The classification question underneath it

`Effect::Bus` is both an exclusion and a label, and the enum has no category for
"process-global state, no I/O". Narrowing by file means giving `crossbus.rs`
some other effect, and the honest candidates are `Pure` — which it is not, since
two programs in one process would share a queue — or a new variant whose
`hermetic()` answer is itself the decision. The open-questions register already
carries the related half: the audit calls `send`/`recv` effectful because "zenoh
is reached through `helpers/zenoh`", which is the wrong mechanism, and RFC-0007
notes the classification is probably still right for a different reason.

**This was not folded into D87.** D87 narrowed the deferral and corrected its
reason; nothing in it said the funnel should follow, and a session that quietly
made it follow would have changed the golden corpus on the strength of a
decision that did not mention it.

## D88 — identity across the bus: the decoded value keeps its stamp and mints a fresh id

D86's amendment left this open and said explicitly that it is a decision, not an
implementation detail: whoever implements `recv` either preserves the decoded
identity, matching the oracle and narrowing F109, or keeps F109's deviation and
records that the bus makes it observable. The eight words are now built, so the
question is in front of something rather than ahead of it.

- Blocks: RFC-0007 §C8 criterion 3, which requires that whichever is chosen is
  then asserted by a test
- Default: **keep F109's behaviour.** The stamp crosses, the id does not.
- Evidence: measured against the oracle, below
- Status: **RESOLVED — keep F109, and it is now the rule for every decoder.**
  Decided by the repository owner, 2026-10-01. This matches the recorded
  default, adopted explicitly rather than by omission.

**The decided rule, stated once so the three decoders share one sentence.** A
value decoded from the wire arrives with the stamp it was sent with and no
identity, and mints one on first need like any other value. That holds for
`recv`, for `sqlite`'s BLOB cells and for the world file, because all three go
through `bund2_value::wire::from_binary` and none of them is a "need" under D1.

**And restoring the id is not merely unwanted, it does not fit.** Bund2's
identity slot is a `Cell<u64>` rendered through `format_id` as base-64 digits of
a counter over nanoid's own 64-character alphabet (D1, D5). The reference's id
is 21 *random* digits over that alphabet — about 126 bits. A u64 cannot hold
one, so restoring an id the oracle wrote would mean widening the value header to
carry a string, which `Hash`, `Eq`, `Ord` and `dup`'s identity reset all read.
That cost buys nothing a program can observe, which is the measurement below.

RFC-0007 §C8 criterion 3 is met by `a_received_value_keeps_its_stamp` in
`crates/bund2-stdlib/src/bus.rs`, which asserts the half that is defined and
does not assert the half that is not.

### What each side does, measured

Both sides carry the stamp through the wire format. `WireValue` has an id field
and the reference restores it; Bund2's `into_value` builds the heap value with
`identity: Cell::new(0)` and `stamp: Cell::new(node.stamp)`
(`crates/bund2-value/src/wire.rs`, `into_value`), so a received value arrives
stamped and unidentified, and mints an id on first observation as every other
value does.

### Why the "makes it observable" half is false

D86's amendment offered "record that the bus makes it program-visible" as the
second half of the second option. **It does not.** Three readers of identity
exist in the reference — equality for non-scalars (`rust_dynamic/src/eq.rs`),
the ordering fallback, and hashing (`rust_dynamic/src/hash.rs`) — and none is
reachable across the bus from a Bund program:

| reader | why a program cannot use it to see the id |
|---|---|
| `==` | refuses a LIST: `== returns error: COMPARE: unsupported operand #1`. Measured on the oracle with `[ 1 2 ] dup "c" swap send.quick "c" recv ==`. |
| ordering | F12 — the fallback is inconsistent with `PartialOrd` and unreachable |
| VALUEMAP key | reachable, and **already an approved deviation**: `probes/valuemap-hash-eq.golden` is accepted under F29, so Bund2's lookup does not hash the id either way |

And **no word returns a value's id**. `.id` is not registered — the oracle
answers `Inline .id not registered` — so D1's `.id` is an object member, not a
VM word. The id's only appearance is in `debug.display_stack`'s dump, which
F14 normalises out of every golden comparison because it is not behaviour the
reference defines.

So no program can tell the two options apart. That is the opposite of the first
draft's claim, which asserted that the values *differ* — the oracle contradicts
that too, since the stamp is identical on both sides.

### The argument for the default, which is D1 and not convenience

Restoring the id would make a received value **the only value in Bund2 born
already identified.** D1 chose lazy identity: nothing is computed at
construction and an id is minted on first need. A decoder that writes one in
has decided that the wire is a "need", which is a different rule from the one
D1 settled, and it would apply to `sqlite`'s BLOBs and the world file through
the same `from_binary` — not only to the bus.

The faithful-looking option is therefore the one that changes a language rule,
and the deviation-keeping option is the one that leaves it alone. That is why
the default is to keep F109 rather than to narrow it.

### What the owner is actually being asked

Whether F109's "a decoded value mints a fresh identity" is **the rule for every
decoder** — which is what keeping it means now that three decoders share the
path — or an accepted gap in `sqlite` alone that the bus should not inherit.

## D87 — the local crossbeam bus is in scope; zenoh stays deferred

**Decided by the repository owner, 2026-10-01**, on RFC-0007's §C7:

> let's scope local crossbeam bus — main purpose is to exchange data between
> localy running VM; in future shall be transparently integrated with zenoh bus
> (send/recv somehow shall make it transparent)

- Blocks: RFC-0007's criteria, which were conditional on this
- Depends on: D28 (whose deferral this narrows), D85, D86
- Status: **RESOLVED — the local half is in scope**, 2026-10-01.

### The decision

**The eight words of `bus/crossbus.rs` are in scope**: `send`, `send.`,
`send.quick`, `send.quick.`, `recv`, `recv.`, `bus.data`, `bus.data.current`.
Their stated purpose is **exchanging data between VMs running locally**.

**`bus/globals.rs` stays deferred under D28** — `global`, `global*`, `?global`,
the three zenoh-backed words.

### D28's deferral was per-directory, and the directory was mixed

D28 deferred `Bund/src/stdlib/functions/bus` whole, with the reason "zenoh
distributed bus — not essential". **That reason was true of one file in the
directory and false of the other.** `crossbus.rs` is crossbeam; `globals.rs` is
zenoh; `mod.rs` holds both transports and registers nothing.

So the deferral narrows to `Bund/src/stdlib/functions/bus/globals.rs`
(`DEFERRED_PATHS` in `xtask/src/corpus/classify.rs`, which matches by path
prefix and already carries per-file entries elsewhere). **Narrowing it also
corrects the reason**, which was the third of three places that attributed
`send`/`recv` to zenoh from a directory name — the other two being the path
audit and RFC-0007's own first draft.

### Transparency is the intent, and it does not conflict with D86

The owner's second clause — "in future shall be transparently integrated with
zenoh bus (send/recv somehow shall make it transparent)" — is **Erlang's model
again**, and it reconciles with D86 rather than straining it: **one vocabulary,
two transports.** `Pid ! msg` does not change shape when the pid is remote, and
`send` should not either. D86 said the transports are orthogonal and neither is
layered on the other; that is about the *transports*, not the words.

So: the words are shared, the transports are not, and a channel's **address**
decides which carries it.

**How an address selects a transport is not decided here**, and it is the one
real design question this creates. Zenoh's side is keyed by paths — the globals
words go through `get_globals_path` and `get_receiving_path` — while a local
channel is a bare name in `PIPES`. A convention (a path-shaped name goes
distributed), a registry, or an explicit prefix are all available, and the
reference has no answer because its `send`/`recv` never reach zenoh at all.
RFC-0007 carries it as an open question.

### What it costs, stated

**The completeness numbers get worse, correctly.** Eight unimplemented words
enter the denominator, so in-scope rises 497 → 505 and both IMPLEMENTED and
COVERAGE fall by about 1.2 points without anything changing in the code.
Scoping in work that is not done is supposed to lower a completeness number;
a figure that only rose when scope narrowed would be measuring the scope.

**`conform` does not move.** No golden exists for these words yet, and
RFC-0007's criteria now say what capturing one would do to the denominator.

## D86 — two buses, orthogonal: crossbeam in process, zenoh across it

**Decided by the repository owner, 2026-09-30**: "zenoh and crossbeam must be
orthogonal. Same idea as in Erlang: distributed and local message exchange
coexist." **Recorded now; implementation deferred**, at the owner's direction.

- Blocks: nothing. It fixes RFC-0007's exchange layer before RFC-0007 is written
- Depends on: D7/D85 (tens of VMs), D84 (`Interp` is not `Send`), D20
  (serialisation materialises), RFC-0001's wire codec
- Status: **RESOLVED — decided, not built**, 2026-09-30.

### The decision

**Local exchange is crossbeam channels; distributed exchange is zenoh; neither
substitutes for the other.** The owner's framing is Erlang's: a message to a
process on this node and a message to one on another node are the same idea
expressed over different transports, and both exist at once.

### This is a preservation contract, not a design

**The reference already does it**, and the shape was proposed here before that
was noticed — which is worth recording because it means the design was arrived
at twice, independently, from the same constraint.

`bus_push` serialises the value with `to_binary()` and sends the bytes down an
**unbounded crossbeam channel keyed by name**, created lazily on first push:

```rust reference/Bund/src/stdlib/functions/bus/mod.rs:88-95
    match d.to_binary() {
        Ok(res) => {
            let mut q = PIPES.lock().unwrap();
            if ! q.contains_key(&k) {
                log::trace!("new bus::internal::pipe : {}", &k);
                let (s,r) = unbounded::<Vec<u8>>();
```

`PIPES` is a `Mutex<BTreeMap<String, (Sender<Vec<u8>>, Receiver<Vec<u8>>)>>`
(`:16`), `crossbeam::channel::{Sender, Receiver, unbounded}` is imported at
`:9`, and `crossbeam = "0.8.*"` with `crossbeam-channel = "0.5.*"` are direct
dependencies (`reference/Bund/Cargo.toml:40-41`).

**Zenoh is a separate transport in the same module** — `ZENOH` is a
`Mutex<zenoh::Session>` at `:23`, used by the globals paths in
`bus/globals.rs`, **not** by `send` and `recv`. An earlier statement in this
work that "`send`/`recv` are zenoh, so the bus is distributed" was wrong and is
corrected here.

**The words, unimplemented in Bund2 and in scope**: `send`, `send.`,
`send.quick`, `send.quick.`, `recv`, `recv.`, `bus.data`, `bus.data.current`
(`bus/crossbus.rs`). They take a **channel name off the stack** — "No channel
name discovered on the stack" is the error — and the pipe error text is
`bus::internal::pipe error: {err}`.

### Why bytes rather than values, which is not a choice

`Interp` is **not `Send`** — `Box<dyn Reporter>` and, through every value,
`Rc<HeapValue>` (verified for D84). So a `BundValue` cannot cross a thread
boundary at all, and a channel between VMs **must** carry serialised bytes. The
reference reached the same conclusion; Bund2 already has the codec, in
`bund2_value::wire`, fixture-tested, with `MAX_WIRE_DEPTH` at 256 refusing what
it could not read back.

### Two properties to preserve, both easy to discover late

**A sent value is not the value that arrives.** `to_binary` materialises the
identity and the stamp (D20), so what comes out of `recv` is an *equal* value
with its own identity — the same property that made the encoded-stream bundle
untenable in D77. A program comparing `.id` across a `send`/`recv` sees two
values, and that is the reference's behaviour.

**The channel is unbounded.** A producer outrunning a consumer grows memory
without limit. That is `unbounded::<Vec<u8>>()`, the reference's choice, and it
is preserved rather than quietly bounded — a bound would be a deviation with a
decision behind it.

### What deferral means

RFC-0007 specifies these words and their two transports; nothing is built until
then. **What is fixed now** is that the local transport is not zenoh, the
payload is the wire format, and the two coexist rather than one being layered on
the other.

### Amended on RFC-0007's first review, 2026-10-01 — the identity claim is wrong

**The decision stands. One of its two "properties to preserve" is false**, and
it is left above because this register is append-only.

Above reads "A sent value is not the value that arrives… what comes out of
`recv` is an *equal* value with its own identity". **The reference preserves
both id and stamp verbatim.** Measured on the oracle: one value pushed through
`send` and taken back with `recv` reads
`id: "gHKQUMqeE5RYPQCCvBC9c" stamp: 1790837306409.0` on **both** sides.
`Value::from_binary` bincode-decodes a serialised `Value` whose `id` and `stamp`
are fields, so it restores them rather than minting.

**Where the error came from.** D20 says serialisation *materialises* identity
and stamp, and that was read as *replaces*. It means **set if unset**: an
unminted value gains an id when written, and a value that already had one keeps
it. The first amendment to D77 made the same misreading in the other direction.

**What is true, and is more interesting than either version.** Bund2's decoder
does **not** preserve the identity: `wire::into_value` builds its heap value
with `identity: Cell::new(0)`, so a decoded value is unminted and will mint a
fresh id on first observation. **F109 already recorded that** — "A decoded value
mints a fresh identity, which no golden can see (F14)".

So the bus is the first place that recorded deviation becomes **program
visible**: `"c" swap send drop "c" recv` compared by identity answers *same* on
the oracle and *different* on Bund2. No golden catches it, because captures
normalise ids — which is precisely why it is written here.

**What this obliges.** Whoever implements `recv` either preserves the decoded
identity, making Bund2 match the oracle and narrowing F109, or keeps F109's
deviation and records that the bus makes it observable. That is a decision, not
an implementation detail, and it is not taken here.

## D85 — D7 is tens, because scale comes from processes rather than VMs

**Decided by the repository owner, 2026-09-30**, taking D7's recorded default
after the measurement it had never been given.

- Blocks: nothing further. It closes D7, the last OPEN decision in this register
- Depends on: D7, D84 (a task is a VM), D86 (how VMs exchange data)
- Status: **RESOLVED — tens**, 2026-09-30.

### The measurement D7 was missing

D7 asks whether per-VM word tables become the memory story. Measured, on this
build:

| | |
|---|---|
| `Slot` | **96 bytes** |
| `Native` | 32 bytes |
| `Interp` | 608 bytes |
| distinct registered names | **396** |
| name text | 3,557 bytes |

So slots are 396 × 96 ≈ **37 KiB** a VM, and with the interner's strings and
index about **75 KiB** all in. At tens — 64 VMs — that is ~5 MB and irrelevant.
At thousands it is ~300 MB **before any program data**, and at ten thousand it
is prohibitive. **The crossover is exactly where D7 said it was**, which is why
the question was worth asking and worth measuring rather than assuming.

### Why tens is the answer rather than the fallback

**Scale comes from processes, not from VMs in a process.** The owner's model is
one program per CPU, nodes joined by a bus — which is the reference's
architecture, `--distributed` and the zenoh transport. Under it the VM count
*inside* a process is about one, so the per-VM table cost does not compound:
4,096 processes carry one table each, in separate address spaces, and the memory
story is per-process regardless.

**Nothing can exercise the alternative.** RFC-0007 is unwritten, no corpus
program is concurrent, and D84 has only just fixed what a task is. Choosing
thousands now means building a copy-on-write word-table overlay against §S6's
per-`Interp` generation cells, with **no consumer to validate it** — the shape
D68 and D75 taught this project to avoid: machinery built, gated, measured
negative, withdrawn one call from use.

**The direction is asymmetric.** Tens can become thousands by adding sharing;
thousands cannot be un-built once the overlay exists and §S6 has been reworked
around it.

### What would reopen it

A program that needs more than tens of concurrent VMs **in one process** —
which, on the model above, nothing does. Whoever revisits starts from 96 bytes
and 396 names rather than re-deriving them, and from §S6's cells and RFC-0005
criterion 23 ("a body compiled for one `Interp` is never run by another") as the
two things a shared table would have to reconcile with.

## D84 — D6 is VM-per-task; the debugger stops a thread and inspects from inside it

**Decided by the repository owner, 2026-09-30**, on the options weighed for D6:
"Option 3."

- Blocks: nothing further. It closes D6, which blocked RFC-0007 and — per
  RFC-0003:944-946 rather than D6's own `Blocks:` line — RFC-0008
- Depends on: D6, D44 (the evaluation thread), RFC-0003's frame loop,
  RFC-0005 §S8
- Status: **RESOLVED — decided; the mechanism corrected the same day**,
  2026-09-30.

### The decision

**Fine-grained suspension inside a word is not required.** D6's recorded
default stands: a task owns a VM. No native becomes a state machine, no
accumulator moves into a frame, and `ExitAction` stays the one variant it has.

**The debugger gets its reach a different way.** Stepping needs a program to
*stop*, not to *suspend*. The debuggee runs on its own thread — the shape
`bund2-cli`'s `main` already uses, `stack_size(EVAL_STACK)` and a declared Tier
1 share — and blocks at a safepoint each step. A thread blocked inside `map`'s
Rust frame has its state intact on its own stack, so nothing has to be reified
for the debugger to see a consistent program.

### The mechanism is not what the options said, and this correction matters

**The option as put to the owner said the debugger "inspects the blocked
thread's VM state". It cannot.** `Interp` is **not `Send`**, for two
independent reasons — it holds `Box<dyn Reporter>` and, through every value,
`Rc<HeapValue>`. A debugger thread may not hold a reference to the debuggee's
`Interp` at all. The owner chose on a description that was wrong in that one
respect, and it was flagged as unverified when put to them; verifying it is what
found this.

**The corrected mechanism: the debuggee inspects itself.** At a safepoint the
debuggee blocks on a channel, receives a **command**, executes it against its
own `Interp`, and sends back **rendered text**. Only commands and text cross the
thread boundary, and both are `Send`; the VM never does.

Three things follow, and each is better than the original framing rather than a
concession:

- **It is the `Reporter` seam's shape** — a structured message out, no shared
  state — which D36 and D45 already committed to and which CLAUDE.md says the
  seam exists for.
- **It is what the reference already does.** `debug`'s readline loop hands each
  line to `bund_compile_and_eval` **in the same VM**
  (`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:81-95`). A
  command executed on the debuggee's own thread against its own VM preserves
  that exactly.
- **It keeps §3.3(j) plausible.** A command/response protocol over a channel is
  what a Debug Adapter Protocol server needs; shared memory is not.

### What is unavailable, stated

**No unwinding or restarting a native mid-call.** A blocked thread can be
inspected and released; it cannot be rewound. So "step out of a native", and
time travel, stay unavailable — §3.3(j) already declined to promise the second.

**Step-into works**, because the debuggee stops *inside* the native's nested
`run_to` loop rather than needing to escape it. That is the whole gain over
D6's default read narrowly, and it is what unblocks RFC-0008 §D1.

### What replaces the blocker

RFC-0008 is no longer blocked on a decision. It is now blocked on a
**measurement**: the safepoint check costs a read per step, and RFC-0005
criterion 7 protects the `startup` and `dispatch` groups within 5%. That is the
same shape as §D2's frame growth, and both are criteria rather than questions.

### Why not the alternatives

- **Fine-grained suspension.** 27 `eval_lambda` call sites across nine files,
  several holding Rust state across the body — `map_base` keeps a
  `Vec<BundValue>` and a loop position. It would be the largest single piece of
  work proposed in this project, on the path criterion 7 protects, and it buys
  the debugger something a thread already buys.
- **VM-per-task read narrowly**, with top-level stepping only. Rejected because
  it is a worse debugger than the reference's for bodies a native drives, and
  the reference's is already the weaker design.

## D83 — `--emit=native` is withdrawn; criterion 4 is discharged by inspection

**Decided by the repository owner, 2026-09-30**, on the options weighed after
criterion 1's measurement: "Option 4, then 2."

- Blocks: nothing. It removes an unbuilt mode from RFC-0006's scope
- Depends on: D10 (whose `cc` permission for this mode becomes moot), D16,
  D82, RFC-0005 criterion 4, RFC-0006 criterion 1 and §B5
- Status: **RESOLVED — withdrawn, and criterion 4 is discharged**, 2026-09-30.

### The decision, in two parts

**RFC-0005's criterion 4 is discharged by inspection, not by a product.** The
criterion asks that no relocation names a compiled body's `FuncId` from inside
another body. `cranelift-jit` consumes relocations when it finalises;
`cranelift-object` keeps them. Making `Emitter` generic over
`cranelift_module::Module` — everything `emit_into` uses is trait surface — was
the whole prerequisite, and `no_relocation_names_a_compiled_body_from_inside_another`
(`crates/bund2-jit/src/lower.rs`) reads them.

**§B5's three items were not needed for it**, which is what made this possible
and is worth stating because §B5 implies otherwise. The lowering bakes the
compiling process's heap addresses in as **immediates**, and an immediate is
not a relocation. The object is read, never run, so the addresses being wrong
in another process does not matter.

**`--emit=native` as a shipped mode is withdrawn.**

### Why

**Its stated justification was measured away.** The research made shedding the
code generator Product B's strongest case, "a bigger practical win than the
arithmetic speedup". RFC-0006 criterion 1 measured it at **12.45% of the
binary**, with Cranelift and `regalloc2` owning 8.9% of `__text` where
`graphitesql` alone owns 13.0%. ERRATA records the supersession.

**What remained was start-up without warm-up**, which is real — D74 set §S7's
threshold at 1024 entries of one body and F139 found no corpus program compiles
a single body even at 64, so Tier 1 contributes nothing to a program that runs
once. It was not enough against the costs:

- **The meaning guards have no cells at build time.** §S6's guard compares a
  per-name generation cell whose address is baked into the code, and D43 mints
  those cells at registration, per `Interp`. Nothing is registered at build
  time. The cells would have to be created at load and found by name through
  relocations resolved then — feasible, but it is the machinery that makes a
  redefinition observable, so an error there is silent wrongness rather than a
  crash. **This risk is not in §B5's list**, and it is the largest.
- **Which bodies to compile may not be knowable.** D16 means a call target can
  be a run-time string; a program's word bodies are bound by `register` when it
  runs. The register already forecloses tree-shaking for this reason: "An AOT
  image retains the word table and the name resolver."
- **It reopens RFC-0005 criterion 30's two excluded mirrors**, on that row's
  own trigger: the exclusion holds "only while §S7 compiles on an entry rather
  than ahead of one".
- The configuration matrix gains per-target linking and D40's
  cross-compilation question, and criterion 8 would want native matching Tier 0
  per golden in each.

### What withdrawal does not mean

**Not that AOT is impossible**, and the reasoning above is partly reasoning
rather than measurement — the fraction of a corpus program's bodies that are
statically known was never measured. **What would reopen this** is that figure,
plus an answer to where the guards get their cells. Both are questions, not
work, and taking them is the first step if the mode is ever wanted.

**Not a repeal of D10's permission.** D10 permits `cc` for `--emit=native`, and
that permission simply has nothing to apply to while the mode is withdrawn.

## D82 — RFC-0006 is Proposed

**Authorised by the repository owner, 2026-09-30**, after three adversarial
reviews of the document and one review of the implementation.

- Blocks: nothing. It changes the document's status, not any rule
- Depends on: D76, D77, D78, D79, D80, D81 — the six decisions the RFC needed
- Status: **RESOLVED — Proposed**, 2026-09-30.

### What is authorised

`--emit=bundle` as specified and built, and `--emit=native` as *specified*
only. Nothing in this authorises building the native mode: §B5's three blocking
items stand, criterion 7 stays deferred, and criterion 1's measurement has
removed that mode's stated justification.

### What the reviews cost, recorded because it is the argument for doing them

| review | blockers |
|---|---|
| document, first | 3 |
| document, second | 1 |
| document, third | 2 |
| implementation (a self-review) | 2 |

**Eight blockers across four reviews**, and two of the eight would have shipped
an artefact that reported success and silently was not one. The document was
"complete and awaiting review" after the first revision and wrong twice more
after that.

### What is not settled

- Whether `--emit=native` is built at all — the owner's, on start-up rather
  than on size.
- **Parse-at-build**, filed as a design call rather than a decision: a syntax
  error becomes a build error, which moves when it is observed.
- Q40's unmeasured limits: a Developer-ID notarised binary, and the ELF and PE
  cases.
- `cargo xtask bundle` is outside `cargo test` and outside `conform`, so its
  finding-power depends on someone running it.

## D81 — `bund2 build` may invoke `codesign` on macOS

**Decided by the repository owner, 2026-09-30**: "yes, codesign is fine — it
ships with macOS."

- Blocks: RFC-0006 §B1, which could not ship a runnable macOS artefact without
  it
- Depends on: D10 (whose toolchain-free half this tests), Q40
- Status: **RESOLVED — permitted**, 2026-09-30.

### The decision

`bund2 build --emit=bundle` may run `/usr/bin/codesign -f -s -` on the
artefact it has just written, on macOS. Without it the artefact **cannot run at
all**: the signature covers the bytes the build writes in place, so the kernel
kills an unsigned edit — exit 137, no output — rather than merely failing
validation (Q40, measured).

### Why this does not spend D10

D10 forbids a **C toolchain** below `bund2 build`: "nothing below `bund2 build`
may require `cc`". `/usr/bin/codesign` is not that. It is root-owned, on the
root volume, outside any Xcode or Command Line Tools path
(`xcode-select -p` reports `/Library/Developer/CommandLineTools`, which is not
where it lives), and it is present on a stock macOS.

The distinction D10 draws is between a build that needs a compiler installed
and one that does not. This needs neither a compiler nor an install.

**One limit, recorded rather than glossed.** Several `/usr/bin` tools on macOS
are stubs that prompt for Command Line Tools when first run. Whether
`codesign` is among them could not be checked here, because CLT is installed on
this machine. If it turns out to be a stub on a bare system, this decision is
the one to revisit, and RFC-0006 §B1's appending form is the fallback — at the
cost of an artefact that can never be notarised.

### What it is not permission for

**Not a signing identity.** The build signs ad hoc, which restores execution
and validation. A Developer ID signature and notarisation are a distribution
step, outside `bund2 build`, and remain the distributor's.

**Not a precedent for other platform tools.** `--emit=native` needs a linker,
which D10 already permits for that mode alone; nothing here widens that.

## D80 — a bundle may carry the JIT, opt-in and never by default

**Decided by the repository owner, 2026-09-29**: "yes, as an opt-in feature
choice, never the default."

- Blocks: RFC-0006 criteria 1, 2, 3 and 6, and §B4's `--features`
- Depends on: D10, D40, D74, F139, RFC-0005 criterion 2, and
  `docs/research/02-native-binaries.md` §1
- Status: **RESOLVED — opt-in**, 2026-09-29.

### The decision

`bund2 build --emit=bundle` may ship a runtime built with the `jit` feature,
**only when asked for**. The default carries no code generator, and that
default is what a target Cranelift does not support receives.

### This is not a deviation from D10, and that was worth checking

RFC-0006's third review filed it as one. Read whole, D10's sentence describes a
*case*: "`--emit=bundle` — runtime plus embedded IR, pure interpreter, no
Cranelift and no `cc` — **is what a target outside
x86-64/aarch64/s390x/riscv64 gets**". Its subject is the unsupported target,
where a bundle is necessarily a pure interpreter.

**The research D10 cites in that same sentence marks Cranelift optional for
this product.** Its table gives Product A's "Contains Cranelift?" as "optional
(for JIT tiering)" and its peak speed as "JIT-tier, after warmup", against
Product B's bold "no"
(`docs/research/02-native-binaries.md:38-42`). So the phrase constrains the
fallback configuration, not the product.

D77 departed from D10's "embedded IR" wording and said so. **This decision
departs from nothing**; it records a reading.

### Why opt-in rather than available

**Size is the cost, and it is the one number nobody has.** The research calls
`cranelift-codegen` with its ISLE tables "the single largest code contributor
to any binary that embeds the JIT", and makes shedding it Product B's strongest
argument — above speed. RFC-0006's criterion 1 is exactly that measurement, and
**it only exists because this decision does**: `cargo bloat` on a bundle with
and without `jit` is unmeasurable if a JIT bundle cannot be built, and without
it `--emit=native` could never be justified.

**The threshold makes it useless for short programs.** D74 set §S7's threshold
to 1024, and F139 found that **no program in `tests/golden/HERMETIC.txt`
compiles a single body even at 64**, "because they are demonstrations that run
once". A JIT bundle of a one-shot script compiles nothing and pays only size.
The benefit needs a program that crosses 1024 entries on one body.

**Never the default, or D10's escape hatch inverts.** The reason
`--emit=bundle` is mandatory is the targets Cranelift cannot serve; if the
JIT-carrying bundle were the default they would have no default at all.

### The conditions

1. **The default carries no code generator**, and is the answer for
   unsupported targets.
2. **RFC-0005's criterion 2 holds per golden in both configurations.** A JIT
   bundle adds two, and conformance must move by exactly zero in each — the
   tier changes speed, not meaning, so any movement is a bug.
3. **RFC-0006 §B4 states what a JIT bundle is for** — long-running programs —
   with F139's finding as the reason, so nobody ships one for a script and
   finds only that it is larger.
4. **The prebuilt runtime matrix doubles**: per target, times jit/no-jit. §B1
   already requires one runtime per target, and this multiplies it.

## D79 — `--noeval` disables the eval function group, and is not a claim about evaluation

**Decided by the repository owner, 2026-09-29**, on the question RFC-0006's
third review raised: "`--noeval` mean disable eval functions."

- Blocks: RFC-0006 §B3a's wording, and criterion 10's
- Depends on: D76, D78, and the reference's own command line
- Status: **RESOLVED — the group, as named**, 2026-09-29.

### The decision

`--noeval` disables the **`bund.eval` group of functions**. It is not a
statement that a program evaluates nothing, and it is not to be described as
one. Behaviour is unchanged, in Bund2 and against the oracle.

**The reference's own help text is the definition**: `Disable bund.eval group
of functions` (`reference/Bund/src/cmd/mod.rs:141-142`). Bund2 stubs exactly
that group — `bund.eval`, `bund.eval.`, `use`, `use.`
(`crates/bund2-stdlib/src/host.rs`, `register_noeval_stubs`).

### What the third review found, and where the defect actually was

The review reproduced `"40 2 +" compile lambda! !` printing `42` under
`--noeval --noio`, on both binaries, because `compile` is registered
unconditionally
(`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:76`). That
reproduction is correct and worth keeping.

**The defect it exposed was in RFC-0006's prose and in D78's amendment, not in
the flag.** Both had described the flag as failing to "stop evaluation" — a
promise the flag never made and its help text never implied. A group switch
that disables its group is working.

**So the reproduction stays and the framing changes.** RFC-0006 §B3a states
which words the group contains and which paths remain reachable — `compile`,
`lambda!`, `!` and, under `--noeval` alone, `url` — not as a shortfall but as
the boundary of what the flag is for. This is what D78 asked for when it
required each flag's ungated surface to be named.

### Why the behaviour is not changed

Making `--noeval` gate `compile` would deviate from the reference for a flag
**no corpus program uses**, and it would not achieve what a reader might want
anyway: D16 means a call target can be assembled at run time, so evaluation
cannot be switched off by name-gating a word list. The honest artefact is one
whose restrictions are documented exactly, which D78 requires and §B3a now
does.

**This closes the question D78's amendment left open** — "whether `--noeval`
should be made to mean 'evaluates nothing' is a separate question and is not
decided here." It is decided here: no.

**Note, 2026-10-08 (RFC-0006's sixth review, B1).** The group is **six**
words, not the four named above: the reference also stubs `bund.eval-file` and
`bund.eval-file.` (`reference/Bund/src/stdlib/functions/bund/bund_eval.rs:117-121`;
`use` and `use.` are at `bund_use.rs:74-76`), and Bund2's
`register_noeval_stubs` stubs the same six. Nothing about the ruling changes:
it is the list that was short, by the two words that read a file and run it.
The help text is at `reference/Bund/src/cmd/mod.rs:142-143`; the range above is
off by one line.

**Note, 2026-10-08 (D120).** Six is the reference's group. Bund2 stubs a
seventh word under the flag, `debug.run`, which is its own and is `bund.eval`
with a safepoint. The reading above — the group, and not a claim about
evaluation — is unchanged, and `compile lambda! !` still evaluates.

## D78 — a bundle's restrictions have a build-time floor that run time may only tighten

**Decided by the repository owner, 2026-09-29**, on RFC-0006's second review
blocker: "D, and never call it a sandbox."

- Blocks: RFC-0006 §B3 and §B4, which were drafted with this open
- Depends on: D10, D16, D40, D54, D76 (whose risk clause this bears on), and
  the reference's own command line
- Status: **RESOLVED — build-time floor, run-time may only add**, 2026-09-29.

### The decision

`bund2 build` may record `--noio` and `--noeval` in the artefact's trailer.
An environment variable read at start-up may **add** either restriction and
**may never remove one**.

That direction is the whole decision. **A restriction an environment variable
can switch off is not a restriction**, and the failure would be silent: the
artefact would still report itself as built `--noeval` while evaluating
everything. Monotonicity is what separates this from a trailer plus a
convenience.

**The trailer is readable.** A restriction the runner cannot observe is one
they cannot rely on, so `bund2 build` gains an inspect path printing an
artefact's features, restrictions and pinned SHAs.

**`--nocolor` is not in this class** and is not covered here. It is
presentational — it changes how one debug word draws its table — so it joins
`--stats`, `--dump-stack` and `--raw-values` in RFC-0006 §B3's
environment-variable channel.

### It is not a sandbox, and the word is not to be used

**Ruled explicitly by the owner**, and the reason is in the code rather than in
taste. `--noio` swaps a word group for stubs that fail *at registration*, and
**`args`, `sleep.seconds` and `io.graph` have no gate at all** — not in the
reference, not in Bund2 (`crates/bund2-stdlib/src/host.rs`, module
documentation). Fetching is gated by `--noeval`, not by `--noio`, so a
`--noio` artefact can still pull and evaluate remote code.

These are **word-group switches**. Calling them a sandbox would assert a
boundary that does not exist, and it would do so exactly where it is most
likely to mislead: D76 places the risk of what an artefact fetches on the
person running it, and a builder who believed `--noio` protected that person
would be wrong. RFC-0006 states what each flag leaves ungated, by name.

### Why not the alternatives

- **Not available in bundles at all.** Defensible, and reversible in one
  direction only — it can become this decision later, where this cannot become
  it. Rejected because it answers neither party: a tool that should not fetch
  could not be built that way.
- **Run time only**, by environment variable. Matches the CLI's per-run
  semantics, but nothing shippable is restricted — the artefact is only as
  restricted as its runner remembers to ask.
- **Build time only.** Answers the shipper and not the runner, who may
  reasonably want someone else's artefact kept off their filesystem.

**D16 is why neither party can be satisfied by analysis instead.** No build can
prove a program never calls `fs.rm`: the name may be assembled at run time, so
the restriction has to be a switch rather than a proof.

### Amended on RFC-0006's third review, 2026-09-29 — the gate sentence is half wrong

**The decision stands. One sentence above does not**, and it is left in place
because this register is append-only.

Above reads "Fetching is gated by `--noeval`, not by `--noio`". `url`, `url.`,
`file` and `file.` **are** `--noio` stubs, so `--noio` does gate fetching
through them; `use` is gated by `--noeval`. The accurate statement is narrower
and worse: **no single flag stops a program obtaining text and running it.**

**`--noeval` does not stop evaluation at all.** It stubs `bund.eval`, `use` and
`use.`, while `compile` is registered unconditionally
(`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:76`), so
`"40 2 +" compile lambda! !` prints `42` under `--noeval --noio`. **Verified on
both binaries**, Bund2 and the oracle at the pinned SHA: it is the reference's
gap, faithfully reproduced. Under `--noeval` alone, `url` fetches and those
three words evaluate the result.

**This strengthens the ruling that the word "sandbox" is not to be used**, and
it is why D78 required the RFC to name what each flag leaves ungated — a
requirement the RFC had met for `--noio` only. RFC-0006 §B3a now names both,
with the reproduction, and criterion 10 states in terms that it checks the
stubs and may not be read as "the artefact evaluates nothing".

**Whether `--noeval` should be made to mean "evaluates nothing" is a separate
question and is not decided here.** It would deviate from the reference.


**Note, 2026-10-08 (RFC-0006's sixth review, B2).** "`args`, `sleep.seconds`
and `io.graph` have no gate" is true and is not the flag's ungated surface: it
is the ungated words of one module. Measured on Bund2 by running each of 610
registered names under `--noio`, 56 answer with the stub. Among the 554 that do
not, `csv` and `sqlite` read a file the program names
(`reference/Bund/src/stdlib/functions/conditional/mod.rs:42-43`, registered with
no gate), `debug.shell` writes a history file, `input`, `input*`, `password` and
`bund.prompt` read standard input, and `system.ip`, `system.locale` and the
`sysinfo.*` words disclose the host. RFC-0006 §B3a carries both lists by name.
The ruling — a floor, and not a sandbox — is unchanged, and is the stronger
for it.

## D77 — a bundle embeds source text, and the encoded container is closed

**Decided by the repository owner, 2026-09-29**, on the options RFC-0006's
first review forced: "option 1, and treat the parse measurement as closing
option 2 rather than deferring it."

- Blocks: RFC-0006 §B2, which was drafted with this as its one open deviation
- Depends on: D10 (the description this departs from), D11 (which licenced the
  alternative), D20 (serialisation materialises identity and stamp), D2,
  D36, D40
- Status: **RESOLVED — source text, and the container is closed**, 2026-09-29.

### The decision

`bund2 build --emit=bundle` embeds the program's **source text**. The artefact
parses and evaluates it exactly as `bund2 script` does.

**This is an approved deviation from D10's description**, not from its
resolution. D10's parenthetical calls `--emit=bundle` "runtime plus embedded
IR"; that phrase predates RFC-0003's narrowing of BundIR to "a cache over a
body, never the body itself" (RFC-0003:422) and is read here as "the program".
D10's resolution — `--emit=native` may require `cc`, `--emit=bundle` stays
toolchain-free, nothing below `bund2 build` may require `cc` — is untouched.

### Why the alternative is closed rather than deferred

Encoding the program with the `wire` codec was RFC-0006's first draft and was
withdrawn on four changes in meaning its own review found, each reproduced
against the code: stamps fixed at build time (against D2's "stamp is creation
time"); one context name for every run; spans and so diagnostic locations gone
(D36); and — the one the review understated — **every scalar returning boxed**,
so `Guard::TopAreInt` admits nothing and Tier 0's fragment path and all of
§S6's inlining die for every literal in the program, while every figure reports
success.

A *fresh* container could avoid all four, and D11's "version the IR format
freshly" licences one. **The measurement says it is not worth building:**

| | measured |
|---|---|
| `startup/parse/mixed`, 891 bytes | **4.29 µs** |
| `startup/registry/register_all` | **41.8 µs** |

The parse is about a tenth of the registry construction every bundle performs
regardless. Extrapolating to the corpus's largest program gives roughly 24 µs,
which is arithmetic on bytes rather than a measurement; even a tenfold error
leaves it under the setup cost. Two absolute figures an order of magnitude
apart, on an unguarded host — not an A/B, so F135's protocol does not apply,
and the claim they support is the order of magnitude.

So a container buys about 20 µs in exchange for designing, testing and
versioning a second format that must independently re-solve spans, unboxed
scalars, and an "unset" stamp encoding — **D20's deferred step, unblocked by
D11 and never built**. The parser already gets all three right.

**Deferring it further was the thing rejected.** A deferral with a trigger
nobody can meet is an invitation: the measurement was the trigger, it has been
taken, and leaving the option open would have someone build the format for a
saving the numbers call negligible.

### What this costs

The program is recoverable from the artefact in readable form. **That is a
property, not a defect** — Bund programs are text and no register asks for
secrecy. If opacity is ever wanted, the answer is compressing the source, which
preserves meaning exactly and needs one pure-Rust dependency, rather than a
container that earns opacity only as a side effect of solving three problems
the parser has already solved.

### What would reopen it

A start-up parse cost that matters: a program large enough, or a start-up
budget tight enough, that ~24 µs is visible against ~42 µs of registry
construction and process spawn. Then the container, and it must carry spans,
encode scalars unboxed, and answer D20's unset-stamp question before it
encodes anything.

### Amended on RFC-0006's second review, 2026-09-29 — two of the reasons above are wrong

**The decision stands. Two of the arguments for it do not**, and they are left
above rather than edited, because this register is append-only and a corrected
argument is only legible beside the one it replaces.

**The boxing claim is overstated.** Above it reads that a boxed scalar would
leave `Guard::TopAreInt` admitting nothing and so kill "Tier 0's fragment path
and all of §S6's inlining". Only the first half is true. Tier 1 looks *through*
the box: `plan_body` tests `dt() == INTEGER` and then `as_int()`, and `as_int`
descends `BundValue::Heap` into `Payload::Scalar` before answering
(`crates/bund2-jit/src/lower.rs`, `plan_body`; `crates/bund2-value/src/lib.rs`,
`as_int`). So promotion and inlining would still fire on a boxed literal, and
only Tier 0's fragments would decline. The overstatement was mine, made while
reporting the first review's finding as "worse than it reported".

**D2 is cited for the opposite of what it decided.** Above quotes D2's "stamp
is creation time" as though it were D2's ruling. That sentence is a *constraint
being weighed*; D2's resolution is the other way — "The clock is not read at
construction at all. The stamp is sampled when it is first observed", itself an
approved deviation from preservation (`docs/registers/decisions.md`, D2).

**The conclusion survives on a restated reason.** Encoding materialises the
stamp (D20), so a literal would arrive at run time *already stamped*. Under
D2's actual rule the first observation in that run must set it; instead every
run would report the build moment. So the encoded stream breaks D2 as decided,
not D2 as this entry first paraphrased it.

**What still carries the decision**, unaffected by either correction: the
measurement (~4.3 µs parse against ~41.8 µs of registry construction), the
context-name collapse, the loss of spans and so of D36's diagnostic locations,
Tier 0's fragments declining, and the second format's obligation to answer
D20's unset-stamp question.

## D76 — `use` in a built artefact fetches at run time, as the interpreter does

**Decided by the repository owner, 2026-09-29**, answering Q38 on the options
that question recorded, plus a fourth this ruling considered and rejected.

- Blocks: RFC-0006 §B7, which was drafted blocked on this and is now written
- Depends on: D10 (the toolchain-free half), D16 (a call target may be a
  run-time string), D54 (the scheme set and the fetch's own defaults), Q38
- Status: **RESOLVED — fetch at run time**, 2026-09-29.

### The decision

A bundle's `use` and `use.` behave exactly as the interpreter's do: the operand
is fetched when the word runs and the text is evaluated in the running VM.
Nothing is embedded at build time and nothing is refused that the interpreter
would accept.

**The fetch inherits D54 whole**, and RFC-0006 §B7 states it so the artefact's
behaviour is documented rather than discovered: `file://` follows curl's rules
and takes an absolute path; `http://` is fetched by `ureq` built without TLS,
follows no redirect, treats an error status's body as the answer, imposes no
size limit, and sends `ZBUS` as its user agent; a string with no scheme is
refused, and `https://` is refused, both already approved deviations under D54.

### Why not embed

**Q38's proposed option was to embed the files named by a `use` with a literal
operand and fall back to fetching.** It was not adopted, and the evidence
against it is that its premise is unexercised.

No corpus program calls `use` — the apparent matches in
`reference/Bund/examples` are comments. The entire exposure is one authored
probe, `tests/probes/use-word.bund`, and it **builds its operand at run time**:
`cwd "file://{A}/tests/probes/data/uselib.bund" format use`. So embedding would
fall back to fetching in the one place `use` is exercised, and would buy
self-containment for a case the corpus contains none of — at the cost of a
build-time file resolver, a transitive `use` walk, and a staleness rule that
Q38 itself identifies as a change in meaning.

**Q38 also attributed run-time self-containment to D10, and D10 does not say
it.** D10's resolution is about the toolchain: "`--emit=bundle` stays
toolchain-free, and nothing below `bund2 build` may require `cc`". The
self-containment claim traces to `docs/research/02-native-binaries.md:44-49`,
which describes Product A as "one file, no external dependencies" in a passage
about build and link dependencies. A bundle that fetches at run time is
toolchain-free to build, so this decision does not spend D10.

**A sub-choice inside the embedding option, recorded because it would have been
missed.** Q38 does not say whether an embedded file would carry its *source
text*, still compiled when `use` runs, or its *IR*, compiled at build time. The
second moves a used file's parse errors from run time to build time, which is a
second deviation on top of the staleness one. Only the source-text form would
have been cheap. If embedding is ever built, it takes the source-text form.

### Whose risk this is

**The person running the artefact is responsible for what it fetches**, and for
the safety and security of doing so. This is the owner's ruling and it is
recorded rather than implied, because the alternative reading — that the
artefact should police its own fetches — would have argued for option 3 or 4.
`use` evaluates what it retrieves; that is what the word does in the reference
and Bund2 preserves it. A bundle does not add a check the interpreter does not
have, and does not claim one.

### What would reopen it

A real case for embedding: a program distributed to users whose library must
travel with it. Then a flag — `bund2 build --embed-use` or similar — with a
decision of its own, in the source-text form above. Deferring it this way keeps
option 2 one call away rather than foreclosed, which is the shape D75 chose for
D68's crossing.

## D75 — D68's crossing is withdrawn from use, and the rule it states is kept

**Decided by the repository owner, 2026-09-28**, on the measurement it asked
for: "E first, then B if it confirms."

- Blocks: nothing. It changes which table a `Runtime` installs, not what any
  rule says
- Depends on: D68 (the rule), D71, D73 and F143 (the gates crossing needed),
  F140 (what it cost), §S7
- Status: **RESOLVED — decided and built**, 2026-09-28.

### The decision

`Runtime::with_options_and_threshold` no longer chains
`JitTier::with_crossable`. The set stays empty, so promotion syncs before every
call — "the conservative answer, and the behaviour every caller had before
D68". **D68's rule is not repealed and its machinery is not removed**: the
table still builds, every gate still answers, and the tests and benches that
exercise crossing pass it in explicitly. Restoring it is one call.

### What the measurement found

D68's payoff first read as zero — ±0.5%, sign random. The fixture was wrong and
the reason is worth keeping: its body was `1 2 + (:s 7 var)*N drop`, and `var`
takes operands. **Pushing an operand is a generic apply, which syncs every
promoted value first**, so the sum was already on the stack before the crossed
call and the crossing had nothing left to hold.

Crossing does not avoid a push; it **defers** one. The saving appears only when
a promoted value crosses a call and is then consumed by an **inlined site**, so
it is never pushed at all — and that needs an **operand-free** callee.
`crossing_isolated` is that shape: `(1 2 + noop drop)*N`, with `noop` at
`eff(0, 0)` (D72) and `drop` consuming the sum from its register.

On it, crossing is a **loss**, at every size measured:

| pairs | crossed − synced | per crossed call |
|---|---|---|
| 1 | +0.79 ns | 0.79 |
| 2 | +3.14 ns | 1.57 |
| 4 | +6.96 ns, and +6.87 on a second run | 1.72 |
| 8 | +7.65 ns | 0.96 |
| 16 | +16.95 ns | 1.06 |

**The cause is the safety it needed.** Each crossed call now carries F143's
generation compare and D73's epoch compare — two loads, two compares, two
branches — and those exceed the push and the pop they avoid. D68 was decided
before either existed.

### The ledger, stated plainly

- **Payoff**: negative, ~1–1.7 ns a crossed call, on the shape it was for.
- **Defects**: three, all silent wrong answers — F137 (the reporter gate never
  asked), F140 (a value synced to the wrong stack), F143 (a rebound callee
  reversing the stack). None was found by a test; each was found by reading a
  specification against the code.
- **Safety**: six compile-time gates and two runtime compares, four of which
  arrived *after* D68 shipped. The argument is exclusion-based and open-ended:
  every new `bund2-stdlib` native must be classified against D71's and D73's
  lists.
- **Reach**: none. At §S7's threshold of 1024 no corpus program compiles a body
  (F139), so nothing crossed anything in any real program even before this.

### Rejected

- **Keep it as built.** The gates are in place, but a feature that needs a new
  gate each time someone reads the spec more carefully, and measures negative
  when they are all present, is not paying for itself.
- **Delete D68's code.** That converts a reversible decision into an
  irreversible one for tidiness. The machinery is correct and well-tested now;
  it costs nothing to keep dormant, and it is the only thing that would make a
  later measurement possible.

### What would reopen it

A lowering where a promoted value can cross a call **without** the two compares
— for instance if the generation and epoch checks could be hoisted out of a
loop, or if a callee could be proven immutable for a body's lifetime. The
arithmetic above is then different, and this decision is one call away from
being undone.

### Both halves are pinned by a test, 2026-09-29

The two halves above were the two things nothing could observe. The withdrawal
is a call **not** made, and a call not made leaves no trace; the dormancy was a
paragraph promising the machinery still works while nothing exercised it in the
shipped configuration.

`the_shipped_runtime_crosses_nothing_and_the_table_still_works`
(`crates/bund2-runtime/src/tier.rs`) runs one body —
`:w { 1 2 + noop drop } register`, the operand-free shape this decision was
measured on — two ways. On `Runtime::with_options_and_threshold` it must report
**zero** crossed calls; on a runtime handed `promotable::crossable` explicitly
the same body must report some. Re-chaining `with_crossable` fails the first
half (verified: it reads `Some(1)` against `Some(0)`), and a body that could
cross nothing at all fails the second, so neither half can pass vacuously.

This does not change the decision. It makes "restoring it is one call" a
statement a reader can check rather than take.

## D74 — §S7's promotion threshold is 1024, not 64

**Decided by the repository owner, 2026-09-28**, on the option F139's §S7
section left open.

- Blocks: nothing; it is a tuning knob with no correctness argument resting on
  it, which §S7 says in terms
- Depends on: F139 (which measured what 64 costs), F130 (the compile cost, and
  the thunk cache that halved it), F136 (net-negative bodies at a low
  threshold), §S7
- Status: **RESOLVED — decided and built**, 2026-09-28.

### Why 64 was wrong

64 predates every measurement of what a compilation costs or what an entry
saves. Both are measured now:

- compilation is **~34 µs fixed plus ~9.4 µs a value** (F130, after the thunk
  cache — it was ~51 µs and ~20.4 µs before);
- a compiled entry saves **~10 ns a value** (`gain_size`).

Two linear terms divide to a near-constant: **break-even is of order a thousand
entries and barely moves with body size** — ~2,100 at four values down to
~1,012 at sixty-four. At a threshold of 64 a body was compiled **sixteen to
thirty times before it could repay the compilation**, and any body that stopped
short of break-even lost outright. F136 measured three of nine corpus bodies
net-negative at threshold 1 for exactly that reason.

### What it costs

**Latency.** A body now runs about a thousand interpreted entries before it is
compiled, where it ran sixty-four. That is the trade, and it is the right side
of it: the entries before the threshold are interpreted at Tier 0 speed, while
the compilation they would have paid for is of order a hundred microseconds —
three orders above an interpreted entry of the same body.

**Fewer bodies compile at all**, which F139 already establishes is not a loss:
no program in `tests/golden/HERMETIC.txt` compiled a single body at 64 either,
because they are demonstrations that run once. A body that never gets hot is
one §S7's threshold exists to decline.

### A consequence in the benchmarks, recorded rather than discovered later

`arith/times_body/1000` is `1000 { 1 + } times drop`, and F130's analysis turns
on that program crossing the threshold **within a single evaluation**: it enters
the body 1000 times, which cleared 64 and does not clear 1024. So that row no
longer compiles within one eval, and the +121% F130 attributed to one Cranelift
compilation per Criterion iteration is no longer produced by it. F130 is
resolved and its figure stands as a record of what was measured at 64; the row
now measures interpretation, which is what D69 restated `arith` to be about in
its warm form anyway.

Benchmarks that need a compiled body warm past `bund2_runtime_threshold()`
explicitly and adapt on their own.

### Rejected

- **A cost model** — compile when expected remaining entries × per-entry saving
  exceeds the compile cost. §S7 argued for it when break-even was thought to
  vary tenfold with body size; it does not, so a fixed count is the right
  instrument and the model would be machinery for a number that barely moves.
  `plan_body` has the inputs if that changes.
- **Leaving 64 and documenting it**, which is what F139 did. Defensible while
  the compile cost was unmeasured; not once break-even is known to be sixteen
  to thirty times the threshold.

## D73 — promotion never crosses a callee that changes which stack is current

**Decided by the repository owner, 2026-09-23**, resolving F140.

- Blocks: nothing; it is the sixth gate on §S5's promotion across calls
- Depends on: D68 (the crossing), D71 (the precedent, and its fifth gate), §S5
  (the epoch check, specified and unbuilt), F140
- Status: **RESOLVED — decided and built**, 2026-09-23.

### The defect it answers

Promotion syncs by **pushing**, and a push goes to whatever stack is current at
that moment. A callee that changes the current stack while values are held in
registers therefore *moves* them: the body's final sync lands them on the stack
in force after the call, where Tier 0 pushed them before it.

F140 measured it at the **shipped** threshold: `:w { 1 2 + stacks_left }` over
65 entries left `main` holding 33 values without the tier and **32** with it.
`stacks_left` is `eff(0, 0)`, so a crossing syncs *nothing* ahead of it and
every promoted value rides across the rotation. Wrong answers, silently.

### The decision

**A static gate**, in the shape D71 established: `SWITCHES_STACK`
(`crates/bund2-stdlib/src/promotable.rs`) keeps every native that changes the
current stack out of the crossable table. Eight names — `endcontext`,
`rotate_stack_left`, `rotate_stack_right`, `stacks_left`, `stacks_right`,
`swap_in`, `to_current`, `to_stack` — of which four are on `PROMOTABLE.txt` and
would otherwise be crossed.

### Why the gate names more than the exposure

Only the **operand-free** switchers can actually be crossed today. `to_stack`
and `to_current` are `eff(1, 0)`, and their name operand has to be pushed
*after* the promoted values; a symbol is not a promotable literal, so pushing it
is a generic apply that syncs everything first. Nothing is ever held across
them.

That protection is a consequence of what happens to be promotable, not a rule.
It would disappear quietly the day symbols became promotable, and nothing would
fail until a program put a value on the wrong stack. The gate does not lean on
it. Some of the eight also *restore* the stack before returning (`swap_in`,
`rotate_stack_left`); the gate does not try to tell restoring from not, because
a source scan cannot and refusing to cross them costs nothing measurable.

### The dynamic half, built 2026-09-23 as **reporting** rather than recovery

The owner ruled for the epoch check after the static gate landed, in the
narrower of the two shapes offered.

`jit_stack_moved` (`crates/bund2-jit/src/lower.rs`) is a helper that parks an
`Error::internal` and fails. A crossed call now loads `Cells::epoch` **before**
it and compares **after**; on a change the body calls that helper and takes its
`fail` edge. The message names the invariant: "a crossed call changed the
current stack, so values promotion was holding belong to a stack that is no
longer in force".

**This is a deviation from §S5 and is recorded as one.** §S5 calls for the
residual path, which "syncs every promoted value to **the stack it was taken
from** — recorded when the value was promoted, not the stack current now".
Recovering that way needs each value's home stack recorded and a second sync
flavour that pushes there — a new `Ctx` field, a new bound symbol, and surgery
on the residual, in the emission code most likely to gain a fresh defect.
Reporting needs none of it and delivers the property F140 was actually about:
**the wrong answer stops being silent**. D37's third way out — a broken
invariant with no sensible continuation, named, and routed through the
diagnostic path so a reporter receives it.

**Under the static gate this branch is unreachable**, which is why it has a test
that reaches it. `a_crossed_call_that_moves_the_stack_is_an_internal_error`
hands `Compiler::with_crossable` a set containing `stacks_left`'s registration
id directly — the table `promotable::crossable` refuses to build — and runs the
shipped lowering against it. That is the configuration a missed switcher would
produce, and it answers with the internal error rather than a misplaced value.
A check nothing can trigger is a check nothing has verified.

**What is still not built** is the recovery, and with it criterion 17's per-call
bound as stated. After a call the lowering now loads the request cell (every
call) and the epoch (crossed calls only); `autoadd` is still read at inlined
sites alone. The criterion names three loads after *every* call, and that shape
does not exist.

### Rejected at the time, and partly superseded above: emitting §S5's epoch check

§S5 already specifies the dynamic form. `Cells::epoch` is documented as telling
compiled code "that the stack it resolved against is still the one in force",
`Interp` bumps it on every stack change and mirrors it into the cells — and
**no emitted code has ever read it**. Criterion 17's per-call bound names "the
three loads after every call (epoch, `autoadd`, request)"; the lowering emits
one.

The check would cover a native that changes the current stack for a reason
nobody has enumerated, where the static gate covers only the names someone
thought of. It is the better long-term answer and it is **not taken here**:
D71's ruling took both halves, and this one takes the static half first because
it is total, costs nothing at run time, and the defect is live. The epoch check
remains specified and unbuilt, and criterion 17's per-call bound stays
unmeasured because most of what it bounds is not emitted.

### Consequences

- The set is pinned by `every_native_that_changes_the_current_stack_is_named`
  (`crates/bund2-stdlib/src/lib.rs`), a source scan over `Vm::to_stack` and
  `rotate_stacks_*`, so a ninth switcher fails the build until it is named.
- `a_native_that_changes_the_current_stack_is_not_crossable` asserts the table
  end; `a_promoted_value_does_not_ride_across_a_stack_switch`
  (`crates/bund2-runtime/src/tier.rs`) is the differential, comparing **every**
  stack — the counts were right and the placement was wrong, so a test reading
  the current stack alone would have passed.
- `PROMOTABLE.txt` is unchanged: the exclusion is in the table this module
  builds, not in the audit's output, exactly as D71's is.
- **The corpus cannot see this class of defect at all** (F139), which is how it
  survived. That is an argument for the epoch check, not against this gate.

## D72 — Bund2 may carry a word the reference does not, and `noop` is the first

**Decided by the repository owner, 2026-09-22**, on a proposal made while
writing criterion 30's fixtures.

- Blocks: nothing
- Depends on: the health metric's rule that conformance counts goldens and never
  words; §S2 (conformance as the regression number)
- Status: **RESOLVED — decided and built**, 2026-09-22.

### The problem it answers

**Neither parser accepts an empty block.** `:f { } register` is refused by Bund2
with "empty block: `}` needs a term before it", and the oracle refuses it too —
checked directly, 2026-09-22, `target/oracle/release/bund` answering "expected
integer, float, string, literal, atom, stack, name, ptr, command, lambda, list,
or ctx". So Bund2's refusal is **faithful**, and there is no defect here to fix.

What there is instead is an expression problem. A body that should do nothing
has to be written `{ 1 drop }` or some other balanced pair, which says "push a
value and discard it" where the author means "do nothing". Every fixture that
needs a no-op callee pays that, and a reader has to work out that the pair is
noise.

### The decision

**`noop` — consumes nothing, produces nothing, does nothing.** `eff(0, 0)`,
registered by `bund2-stdlib`.

### The precedent, which is the part worth deciding

This is **the first word Bund2 carries that the reference does not**, and the
repository's premise is that "Bund syntax and logic are preserved 100%". So the
decision is not really about one word; it is that **Bund2 may add a word the
oracle lacks**, and this is the first exercise of it.

What that does and does not disturb:

- **Conformance is unmoved.** It counts goldens, never words, and no golden
  gains a program. The health metric's own rule — "never add words to that
  denominator" — is about the *conformance* denominator, which this does not
  touch.
- **Coverage is unmoved.** Its in-scope set is the reference's registry, and
  `noop` is not in it, so it is out of scope by construction rather than an
  uncovered word.
- **`bund2 words` gains an entry**, which is the honest visible consequence.
- **`PROMOTABLE.txt` regenerates**: `noop` is a fixed-effect native, so
  criterion 28's palette reaches it and D48's table gains it. That file is
  written by `BUND2_UPDATE_PROMOTABLE=1 cargo test -p bund2-stdlib promotable`
  and is the repository owner's to run.

**A word the oracle lacks can never be exercised by a golden**, because goldens
are captured from the oracle. So anything added under this decision is tested by
Bund2's own tests alone, and that is a real asymmetry to keep in view: the
oracle cannot referee it.

### Rejected

- **Teach the parser to accept `{ }`.** This was the first thing checked and it
  is the worse deviation: syntax is what the repository preserves most strictly,
  the oracle rejects an empty block, and accepting one would make Bund2 parse
  programs the reference refuses. A vocabulary addition is visible in
  `bund2 words`; a syntax divergence is visible only when a program that should
  have failed does not.
- **Leave it, and keep writing `{ 1 drop }`.** Workable, and what every fixture
  did until now. Rejected because the pair is read as intent and it is not.

## D71 — promotion never crosses a callee that can report mid-body, and the tier asks the reporter besides

**Decided by the repository owner, 2026-09-16**, resolving F137.

- Blocks: nothing; it is the fifth gate on §S5's promotion across calls
- Depends on: D68 (the crossing this gates), D45 (which narrowed when §S5's
  rule bites), D36 (the reporter seam), D55 (the audit this extends), F137
- Status: **RESOLVED — decided and built**, 2026-09-16.

### The rule, in two halves

**The gate is static.** `bund2-stdlib` excludes from the crossable table every
native that reports a `Warning` or `Notice` mid-body
(`REPORTS_MID_BODY`, `crates/bund2-stdlib/src/promotable.rs`). Today that is
one name, `alias`.

**The seam exists as well.** `Vm::wants_stack(Severity)` is new, defaulted to
`false`, forwarded by `Interp` to its reporter; `JitTier::enter` asks it at each
body's entry and hands a crossing body to Tier 0 when the reporter wants a
mid-body snapshot. That is §S5 read literally — "the reporter is read at each
compiled body's entry" — and it is the second of two locks.

### Why the static half carries the weight

§S5's rule is about Q34's shape: `Interp::report` snapshots the stack when the
reporter wants one, so a native reporting while values below its arity sit in
registers shows a **short stack**. Shipped `bund2-stdlib` reports mid-body in
five places, and four are already unreachable as a crossed call — `while`,
`for` and `*loop` are `StackEffect::opaque`, which D68 refuses, and `run_error`
is a conditional arm run by `!`, opaque since F87. **`alias` was the whole
exposure**: `eff(2, 0)`, certified by criterion 28's palette, admitted by D46,
D47, D48 and D68 alike.

So the static form costs **one native out of 56 survivors** and buys a property
the dynamic form cannot: a compiled body is sound *unconditionally*, rather than
sound while a mutable public field holds a particular value. A cached body whose
correctness depends on reporter state read at entry is a hazard that would
outlive this defect.

**What it rests on**, stated because it is load-bearing rather than obvious: a
**non-opaque** native never re-enters evaluation, and so cannot reach a
reporting native indirectly. `StackEffect::opaque` is exactly the marker for a
word that runs a body, and this is the same assumption `PROMOTABLE.txt` already
rests on (D55).

### Why both, rather than either

The static gate is an audit-based claim, the class of claim that goes stale. It
is pinned by `every_native_reporting_mid_body_is_named`
(`crates/bund2-stdlib/src/lib.rs`), a source scan in criterion 25's shape: a
sixth report site fails the build until someone names it and decides whether it
is reachable as a crossed call. The `Vm` gate is what keeps §S5's rule true in
the window where shipped code has gained a report and the table has not caught
up. Under today's vocabulary it never changes an outcome, which is the intended
state for a second lock.

### Rejected

- **The dynamic gate alone** (§S5 literal). New permanent surface on D36's seam
  as the *only* thing between a warning and a short stack, defaulted `false` so
  an embedder's `Vm` that forgets to forward it is silently wrong. The static
  table makes that default safe.
- **Two compiled variants, selected at entry.** Doubles F130's 125–141 µs
  compile cost for a configuration nobody runs by default.
- **Refusing to compile any body that would cross while the reporter wants
  snapshots.** Same effect as declining at entry, but it burns the body's cache
  slot and re-plans it on every later entry, which is what F133's demotion
  exists to avoid.

### Consequences

- §S5's rule is implemented by a stronger mechanism than its text describes.
  The section gains a dated note saying so. **No `ERRATA.md` entry**: that file
  records supersessions of `docs/research/`, and no research document states
  this rule — it is RFC-0005's, and an RFC is editable.
- Criterion 22's last bullet is unblocked: `alias` is never crossed, so the
  snapshot it takes is exact under any reporter.
- `Vm` gains a defaulted method, so no test double breaks. It is defaulted
  rather than required — the argument `Vm::cells` makes for requiring — because
  it is the second lock rather than the only one.

## D70 — criterion 7's significance test filters false alarms; it does not generate them

**Decided by the repository owner, 2026-09-15**, resolving F134. Option B of the
three that entry set out.

- Blocks: nothing; it repairs a clause in one acceptance criterion

### The clause it replaces

Criterion 7 asked its two protected groups for "no statistically significant
difference" **and** a point estimate under 5%. The second half is a band with a
stated basis. The first half **cannot be satisfied by any build**: Criterion's
test asks whether a difference is distinguishable from zero given the observed
variance, not whether it is large, and two builds differ systematically in
binary layout, code placement and allocator state. Measured: a feature-off
binary against its own baseline, nothing changed, reports **−1.02% at p = 0.00**.

### The decision

**A change beyond 5% fails only if Criterion also reports it significant.**
Significance becomes a filter on movements that already exceed the band, which
is the role it can play, instead of a second gate that fires on movements far
inside it.

The band is unchanged and still governs, in **either direction** for `startup`
and `value`: those groups exist because the tier must not disturb work beneath
it, and a protected row moving 5% *faster* with the tier on is as much a sign of
disturbance as one moving slower.

### Why not the alternatives

**Dropping the significance clause entirely** (option A) would leave a bare
band, so a single sample landing at 4.9% would pass and one at 5.1% would fail,
with nothing to say whether either was noise. The filter costs little and
catches exactly that.

**Comparing against the day's own drift floor** (option C) is stricter and more
faithful to the machine, and it was declined for what it does to the record: it
makes "criterion 7 met" a statement about one afternoon's noise. This RFC
already carries enough figures whose meaning depends on which machine produced
them — §S1's ceiling crosses the 2× rule on one machine and not another — and a
gate that moves with the weather is harder to quote than one that does not.

### Consequences

- Criterion 7's table states the band once, in both directions, with
  significance as a filter above it.
- A verdict on the criterion becomes possible; F134 is resolved.
- Nothing about the 5% number changes, so §S1's basis for it still stands.

## D69 — criterion 7's `arith` measures a warm session, and keeps its cold rows beside it

**Decided by the repository owner, 2026-09-15.**

- Blocks: nothing; it restates what one acceptance criterion measures

### The problem

`arith`'s three programs ran through `timed_eval`, which passes `interp` as
`iter_batched`'s setup, so **every Criterion iteration built a fresh `Interp`
with an empty cache**. `1000 { 1 + } times drop` crosses §S7's threshold of 64
within a single eval, so each iteration paid one full Cranelift compilation —
of order 125–141 µs, F130 — and the row read **+121%**. No session recompiles a
body on every call, so the criterion's headline failure was substantially a
property of the harness.

### The decision

`arith` measures the **steady state**: each program is `register`ed once and
entered repeatedly on one warmed interpreter (`warm_eval`), which is criterion
10's shape and a session's.

**The cold rows are kept, renamed `/cold`, not deleted.** They are the only
thing that measures first-entry cost, which a short-lived process genuinely
pays, and keeping them is what stops this from being a failure redefined away.
RFC-0005 says in terms that "a criterion whose failure can be explained away by
changing its denominator would not be worth having"; both denominators now
appear, and the entry below records what each one showed.

### Why this is not an exemption

Restating did not clear the criterion. `times_body` warm straddles zero, which
confirms its +121% was compilation — but `float_mul` warm regresses **+22.7% to
+26.4%** across three runs, a steady-state failure that the cold fixture had
hidden, because a straight-line stream never becomes a body and never compiles.
The criterion still **FAILS**, now on a row whose cause is attributed: 0 sites
inlined and 0 values promoted (F133).

`int_add` warm reads −97.9%, and that is **not** a speedup to claim: with 1000
sites inlined and 1001 values promoted, Cranelift folds the literal chain to a
constant, so the row measures folding rather than arithmetic. It is recorded
with that caveat rather than quoted as a win.

### Consequences

- RFC-0005 criterion 7 carries both sets of rows and the verdict is taken on the
  warm ones, with the cold ones retained as first-entry evidence.
- F130's +121% is explained rather than outstanding; F133 is opened for the
  steady-state regression the restatement exposed.

**What it exposed, and what came of it — 2026-09-15.** The restatement paid for
itself the same day. `float_mul`'s +24% was invisible under the cold fixture,
because a straight-line stream never becomes a body and never compiles; warm, it
was a body the tier could not help being compiled anyway. §S7 gained a fifth
rule — refuse a body with no inlinable site and nothing to promote, and demote
it so the refusal is not re-decided — and the row now reads −0.26% to −3.11%,
with `--stats` confirming 0 bodies compiled where it had been 2. F133 is
RESOLVED. This is the point of the decision: restating what a benchmark measures
found a real defect, rather than retiring a number that was inconvenient.
- `timed_eval` is unchanged and still serves `dispatch`, `lambda` and `corpus`:
  this decision restates one group, not four.

## D68 — promotion crosses a call only when the callee produces nothing

**Decided by the repository owner, 2026-09-14**, resolving Q39.

- Blocks: nothing; it is the fourth gate on §S5's promotion across calls
- Depends on: D46 (not across a lambda), D47 (stdlib natives only), D48
  (`PROMOTABLE.txt`), D66/D67 (promotion as built)
- Status: **RESOLVED — decided and built**, 2026-09-16. `Compiler::would_gain`'s
  sibling `crossable_callee` asks all four gates at plan time,
  `Plan::Call { cross: Option<u8> }` carries the verdict and the operand count,
  `emit_sync_top_n` pushes the callee's operands while deeper values stay in
  registers, and each crossed call emits a spill block so a failure still
  reports what it held. `Word::crossings` records the verdict per call and
  `Word::syncs` counts the pushes emitted ahead of a call, which is what
  witnesses the crossing in emitted code rather than in the plan.
  **In the corpus: 4 of 13 generic calls are crossed**, across 4 of the 7
  programs that compile anything.
- **One gate is not built: §S5's reporter gate.** The crossing happens today
  regardless of what the reporter wants, and §S5 requires otherwise. **F137**
  carries it.
- **Its reach is narrowed by F136, and that is accepted, 2026-09-16.** F136
  refuses a body with no inlinable site, and a site comes only from §S6's
  `PUBLISHED` — `+`, `dup_one`, `drop` (`crates/bund2-stdlib/src/fragments.rs`).
  Of the sixteen survivors named below, `drop` is the only one also published,
  and a published word inlines rather than being called, so **the other fifteen
  reach D68 only in a body that already inlines something else**. A body of
  `1 2 println nl` — the shape the survivor list is about — is refused before
  D68 is consulted.
  **The repository owner ruled: leave both rules as they stand.** The corpus
  says the narrowing is not a nullification — **4 of 13 generic calls crossed,
  across 4 of the 7 programs that compile anything**. Relaxing F136 to admit a
  zero-site body that crosses a call was considered and refused: the two bodies
  F136 drops measured −4.0 ns each, crossing does not touch the +5.13 ns a
  generic call costs compiled, and against F130's 125–141 µs compile cost a body
  whose whole gain is one held value needs on the order of forty thousand
  entries to repay. **Publishing more fragments is the lever that dissolves this
  rather than trading against it** — every arm added admits more bodies past
  F136 *and* removes a call, since inlining and crossing are complementary — and
  that is a §S6 decision, not a patch.
- **Correction, appended rather than rewritten.** The line above first read
  "One half is not built: D45's suppression", which inverts D45. D45's decision
  is that `wants_stack` **takes the severity**, and its status is
  *RESOLVED — built*; it narrowed when §S5's rule bites (the CLI's
  `TextReporter` wants a snapshot only for `Severity::Error`, so ordinary runs
  promote across calls) rather than removing the rule. The rule itself stands,
  structurally, in §S5: "while the reporter wants a snapshot for a severity
  natives report mid-body, `Warning` or `Notice`, no value stays promoted
  across a call", read "at each compiled body's entry". Assumption 2 repeats
  it. Mistaking a narrowing for a repeal is the error, and it is left on the
  page because the register is append-only and the misreading is the easy one.

- **Withdrawn from use 2026-09-28 by D75**, which keeps this rule and stops
  shipping it: on the shape crossing was designed for it costs ~1–1.7 ns a
  crossed call, because F143's generation compare and D73's epoch compare
  exceed the push and pop they avoid. The measurement below was taken on a
  fixture that could not show a saving — see D75.
- **Measured 2026-09-22: the payoff is zero, within noise.** D68 was decided on
  soundness and on a corpus count, never on a time. The `crossing` group
  (`crates/bund2-bench/benches/interpret.rs`) is that A/B: one binary, one run,
  two arms differing only in the crossable table — the crossed arm is the tier
  `Runtime::new` installs, the synced arm the same tier with the table left
  empty, which is "the behaviour every caller had before D68". Body
  `1 2 + (:s 7 var)*N drop`; `var` is `eff(2, 0)`, certified and unpublished, so
  it is crossed; `+` and `drop` inline and satisfy F136. The pre-flight confirms
  the arms differ — crossings scale **1, 2, 4, 8** against **0**.
  In the cleanest of four windows (guard mean 7.8%): **−0.04%, +0.13%, −0.35%,
  −0.08%** at v1/v2/v4/v8, every interval overlapping. The three noisier windows
  scatter to ±2.4% and **flip sign**, tracking the guard's mean rather than the
  arm. No effect is resolvable above about 0.5%.
- **Why, structurally, and this part needs no host.** A sync pushes each
  promoted value **once**. After the first crossed call the values are on the
  stack and no longer promoted, so the second and eighth crossings have nothing
  left to save, while each still emits its spill block. **D68's saving is
  bounded by the number of distinct promoted values, not by the number of calls
  crossed** — which is what the flat measurement against 1/2/4/8 crossings
  shows. It follows that "4 of 13 generic calls crossed" is a weaker figure than
  it reads: it counts calls, and calls are not what the saving is proportional
  to.
- **What this does not say.** It does not say D68 was wrong. D68 is what makes
  crossing *sound*, and the alternative it replaced was a silent wrong-order
  sync. It says the speed case for crossing is unproven, and that a body holding
  many distinct values across a call — which no corpus program does — is where
  any payoff would have to come from.

**The invariant, stated properly.** Q39 as filed said the promoted set is a
strict suffix of the abstract stack, so any callee consuming promoted values
forces a full sync. That was too strong, and the correction matters because it
is what makes the rule narrow rather than fatal. The real invariant is weaker:
**values on the real stack must appear in abstract order** — they need not be
contiguous. So `1 2 3 f` may push `3` alone while `1` and `2` stay in
registers, because those two sit below everything the real stack holds.

**What actually breaks is the final sync.** Pushing only appends, so promoted
values must be the *top* of the abstract stack at the moment they are synced. A
callee that produces output leaves its result above them permanently. `1 2 3 f`
with `f` at `eff(1, 1)` ends with real `[…, r]` and abstract `[…, 1, 2, r]`;
syncing gives `[…, r, 1, 2]`. Wrong silently, and no guard catches it.

### Decision

**Promotion crosses a call only when the callee's declared effect produces
nothing, and the values the callee consumes are never promoted** — they are
pushed, and what stays in registers is what lies below them. After a
`produces == 0` callee returns, the promoted values are the top of the abstract
stack again and the sync is sound.

**56 of `PROMOTABLE.txt`'s 222 natives survive this**, and they are the ones
that matter: `println`, `print`, `nl`, `space`, `drop`, `return`, `to_stack`,
`to_current`, `stacks_left`, `stacks_right`, `register`, `unregister`, `alias`,
`unalias`, `var`, `ensure_stack`, and the whole `.`-suffixed workbench family.
The 166 excluded are value-producing arithmetic and conversions — and the
arithmetic is what *inlining* takes, where there is no call to cross.

### Rejected

- **Pull the callee's outputs back into registers.** Sound, and it would lift
  the restriction entirely, but a produced value may be any type while the
  register file holds `i64`. It needs a `BundValue`-in-slot register class and
  `2p` stack operations per call.
- **Widen the register file to `BundValue`.** A different feature, and it loses
  `iadd` on promoted ints, which is what promotion exists for.
- **Sync at depth**: insert promoted values beneath the callee's output. Needs a
  new `Stack` operation, is O(depth), and must preserve D41's tagging.
- **Abandon promotion across calls.** Today's built state, and the cheapest — but
  it supersedes parts of D46, D47, D48 and criterion 22 rather than narrowing
  them.

**The measurement is why the cheap option wins.** Criterion 10 read
**1.055–1.073×** on `1 2 + drop` after chaining against **1.057–1.067×**
before it: keeping results in registers across inlined sites — removing exactly
the value traffic promotion exists to remove — bought nothing measurable. The
rejected options are larger versions of the same bet. If a later measurement
contradicts that, pulling outputs back is the upgrade path, and this decision
does not block it.

### Consequences

- §S5 gains the rule as a fourth gate beside D46, D47 and D48.
- Criterion 22's promotion-across-a-call parts already use qualifying callees —
  `f` inferring as `(1, 0)` and `alias` at `eff(2, 0)` — so no example changes.
- `PROMOTABLE.txt`'s audit gains the check, or the lowering reads `produces`
  from the callee's slot at compile time. The latter is cheaper and needs no
  file change.

## D67 — the residual path applies the rest of the body, and never rejoins

**Decided by the repository owner, 2026-09-14.** RFC-0005 §S5's residual, built,
with assumption 38's resume index. It supersedes D66's closing restriction: a
site's result now stays in a register.

- Blocks: nothing; it is what criterion 21 asserts and what D66 deferred
- Depends on: D66 (promotion's first stage), D43 (generation cells), D65 (the
  cells' address)
- Status: **RESOLVED — built.**

**The residual is not "take the generic call and rejoin".** That reading is what
forced D66 to sync every inlined site's result: the region left the sum in a
`Variable`, the generic path left it on the stack, and two register models
meeting at one join is unsound. §S5 says something different — the residual
"syncs every promoted value … and then applies the rest of the body's values one
at a time through the runtime's `apply`, exactly as Tier 0 would". **It never
comes back.** With no join to merge at, the fast path owes nothing, and a
site's result stays promoted: `1 2 + 3 +` keeps the first sum in a register and
feeds it to the second `+` as an operand.

**Still guard-and-branch.** `jit_residual` is a runtime helper called from the
emitted body, like `jit_apply`; the body returns its status. Control never
leaves compiled code, so there is no OSR and no frame handed back to Tier 0.

**The resume index is the site's own position, not the next one.** A guard
refuses *before* its value has run, so the value is still owed and the residual
re-applies it — which is exactly the generic call that block used to emit
inline. `Word::resumes` carries the map, and `Compiler::resume_table` exposes
it, because criterion 21 requires the table to be asserted directly: "stacks
alone would pass a lowering that resumed at the wrong index on a seventh
program."

**Every value goes through `status_of` inside the loop**, not just the last. It
is §S5's one status-maker — it substitutes `Error::exited` after an `Ok` with an
exit recorded, passes an `Err` through unchanged, and clears a tail request on
every error. A loop testing only `is_err` would run the value after
`bund.exit`.

**What it did not buy, measured rather than assumed.** Criterion 10 reads
**1.073× and 1.055×** on `1 2 + drop` after chaining, against **1.057× and
1.067×** before it. Four batches of ten alternating pairs, and the two after
*bracket* the two before — one population, not a movement. The Julia step reads
**1.097×** against 1.108×, which is marginally *worse*. Chaining removed the
stack round trip it was built to remove, and the figure did not move.

That is a result about where the cost is, not a failure to tune. What remains is
the boundary itself — an entry trampoline, a slot load and an indirect call per
value, the request-cell load after each — and `Vm::apply` for every value that
is not an inlined site. `1 2 + drop` has four values and three sites, so what
promotion and inlining could reach was already small beside what the boundary
costs. The stop rule still fires at 1.2× and this entry does not explain that
away. See Q39 for the restriction that blocks promotion across calls, which is
the next place the cost could come from.

Conform is unmoved at **106/114, ceiling 106/114**, identical with `--features
jit` and without.

## D66 — promotion keeps a site's operands in registers and syncs its result

**Decided by the repository owner, 2026-09-14.** RFC-0005 §S5's promotion, built
in its first stage: an int literal in a compiled body becomes an `iconst` in a
Cranelift `Variable` and never reaches the stack, and an inlined site whose
operands are all promoted reads them from those registers instead of calling
the pop helper. `1 2 +` lowers to two `iconst`s and an `iadd`.

- Blocks: nothing; it is the "prize" §S6 names and criterion 10's remaining
  headroom
- Depends on: D43 (generation cells), D46/D47/D48 (what promotion may cross),
  D65 (the cells' layout and address)
- Status: **RESOLVED — built, in the stage described below.**

**Superseded in this part by D67, 2026-09-14 (same day).** The paragraph below
is correct about *why* a result could not stay promoted while the residual
rejoined the fast path — and wrong about the premise. §S5's residual does not
rejoin: it applies the rest of the body and returns. With no join there is no
merge, and D67 keeps the result in a register. The reasoning is kept because it
is the argument that had to be answered, not deleted.

**The result is synced; only the operands stay promoted.** An inlined site keeps
its residual path, because a name can be rebound however its operands arrive.
The region leaves the sum in a `Variable` and the residual leaves it on the
stack, and **two different register models meeting at one join is unsound**. The
obvious repair — pop the residual's result back into a register — needs proof
that the rebound name returned an `Int`, which is precisely what the meaning
guard has just said cannot be proven. So the arm pushes its result like any
other, and what promotion removes is the *operand* traffic. Chaining (`1 2 + 3
+` folding whole) is a later stage.

**A sync precedes every call.** That is the structural form of §S5's rule, and
it buys three properties rather than one: the promoted model is empty at every
edge that can branch to `fail`, so the error paths are correct by construction
rather than by enumeration; nothing stays promoted across a call, so D46, D47
and D48 are satisfied without a check; and only a `CALL` or a `CONTEXT` literal
can move the current stack, both of which are `Plan::Call` and therefore sync
first — so "the stack current at the sync" and "the stack the value came from"
are the same stack, which is what criterion 21 tests.

**The excess is synced before a site, and that is arithmetic rather than a
check.** A site consumes the top `needs` values; anything promoted below them is
pushed first, or the result would land beneath a value still in a register. No
guard would catch that misordering.

**A promoted site asks two guards, not three.** The type guard is discharged at
compile time — every operand is a known int literal, so `Guard::TopAreInt`
is answered statically — and calling `jit_admits` would be *wrong* rather than
merely redundant, since with the operands held back it would interrogate a stack
that is missing them. The meaning guards (generation, `autoadd`) still run.

**A literal needs no meaning guard of its own.** `Interp::apply_step` sends
`CALL` to `dispatch_name` and `CONTEXT` to the context switch; every other kind
falls to a default arm that is `self.push(v)` and nothing else. `autoadd` lives
inside `dispatch_name` and cannot reach a literal.

**What is not built, and is not claimed.** Assumption 38's **resume index is not
implemented and criterion 21 is not satisfied.** The resume index exists for a
residual that applies "the rest of the body's values" after values stayed
promoted *across* a call; in this stage nothing does, so at every call site the
model is already empty and there is nothing to resume with. The full residual,
the epoch guard at call sites, and promotion across natives on
`PROMOTABLE.txt` all remain ahead.

**Promotion is not gated on a body length.** Criterion 9 measured a crossover of
4 for *compiled bodies against interpretation*; promotion is a different
question — it removes a slot call and adds no per-use cost — and borrowing that
number for a claim it never made would be adopting a measurement by analogy.

Conform is unmoved at **106/114, ceiling 106/114**, identical with `--features
jit` and without. Coverage 383/497, implemented 391/497.

## D65 — the cells have a guaranteed layout, and a lowering is given their address

**Decided by the repository owner, 2026-09-13.** RFC-0005 §S6's *Addressing* is
built for the request cell: compiled code loads it through an address embedded
at compile time and branches, rather than calling into Rust to ask.

- Blocks: nothing. It is the prerequisite every §S6 guard needs
- Status: **RESOLVED — built.**

**A latent layout defect had to be fixed first.** `Cells` was a plain
`#[derive(Debug, Default)] struct` with no `repr`, and Rust may reorder such
fields between compiler versions. A base-plus-offset load emitted into machine
code against that layout is undefined: it would work on the machine that built
it and could break elsewhere, silently, with no Rust-level symptom. `Cells` is
now `#[repr(C)]`, and the offsets are taken with `std::mem::offset_of!` rather
than written by hand — so the number a lowering uses is produced by the same
compiler that laid the struct out, and cannot drift from it.

**The address reaches the lowering as a parameter.** `Compiler::compile_word`
and `compile_body` take the base from `Cells::base`, and `JitTier::enter`
supplies it because it holds the `&mut dyn Vm` that the per-`Interp` compiler
serves. A zero base is **refused** rather than emitted: a body lowered without
cells would read address zero from machine code, which is a fault rather than
an error a caller can act on.

**What this makes possible, and what it does not.** The request cell is now
loaded and branched on in emitted code, which is what §S5 specifies for a
non-tail call. The **`autoadd`, epoch and generation guards are not built**, and
deliberately: criterion 17 requires an inlined region for the guard to protect,
recorded in a side table and checked for dominance; the body lowering inlines
nothing, so every site already *is* the generic slot call. A guard emitted now
would compile to a load, a compare, and two branches to identical code, and
criterion 17 would pass on it vacuously — the exact failure the fifth review's
B1 had it rewritten to prevent. They land with fragment inlining.

Conform is unmoved at 106/114, ceiling 106/114.

## D64 — the drain helper is a `Vm` method, and tail position reaches the adapter through its thunk

**Decided by the repository owner, 2026-09-13.** RFC-0005 §S5's drain helper is
built, and two shapes it needed were not specified.

- Blocks: nothing. It gives §S6's request cell its first acting reader
- Status: **RESOLVED — built.**

**Dated note, 2026-09-13 (2) — the two-adapter split is superseded by D65.**
This entry gave the lowering two adapter symbols per kind, because "the adapter
cannot see where it was called from" and a non-tail call had to drain where a
tail call did not. That reasoning stood only while the decision was made in
Rust. With §S6's addressing built, the *emitted body* loads the request cell
after a non-tail call and not after a tail one, so position is decided where it
was always known — at emit time, in CLIF. `jit_call_native_tail` and
`jit_apply_tail` are removed, one `jit_drain` adapter serves both lowerings, and
the body reaches it through its own slot, which keeps criterion 4's relocation
rule intact. The rest of this entry — `Vm::drain_tail_request`, the writer set
staying at four, and the drain leaving the exit gate to the status-maker —
stands unchanged.

**The helper had to widen `Vm`.** §S5 says the drain does "what `Interp::apply`
does after `apply_step`: `take_pending`, then `run_to` down to the frame count
it found" — and all three of `take_pending`, `run_to` and `unwind_to` are
private to `Interp`, while `bund2-jit` holds only `&mut dyn Vm`. So the helper
cannot live in `bund2-jit` alone: `Vm::drain_tail_request` is the seam, with
`Interp`'s implementation carrying the floor check, the unwind on error and the
`run_to`.

It **writes no `pending_tail` of its own** — `take_pending` and
`clear_tail_request` do that, and both are already in assumption 33's named set
— so the request cell's writer set stays at four and the mirror stays paired
without widening the scan.

**It does not consult the exit gate.** `status_of` substitutes the refusal
after an `Ok`; a second consultation here would substitute twice for one exit.
A drained body that records an exit therefore drains successfully, and the
refusal is made by the one status-maker. `draining_leaves_the_exit_to_the_status_maker`
(`crates/bund2-interp/src/lib.rs`) pins it.

**Tail position reaches the adapter through the thunk, because the adapter
cannot see its caller.** §S5 has a non-tail call drain and a body's last call
hand the request back — draining in tail position would spend a Rust frame per
level and break RFC-0003's criterion 2. The adapter is one Rust function shared
by every call site, so the *lowering* carries the distinction: two adapter
symbols per lowering (`jit_call_native`/`jit_apply` and their `_tail` twins),
both bound on the module, and `emit_into` imports the tail symbol for the last
thunk when the body claims a tail call.

**Rejected: one adapter with a position argument.** It would widen the boundary
signature that §S8 fixes at `(ctx, index) -> status`, and the verifier's rule
rests on that uniformity. Two symbols cost an import nothing calls.

**What this does not decide.** The cell's *load* is still Rust's: compiled code
calls the adapter unconditionally and the adapter asks the `Vm`, where §S6 has
compiled code load the cell and branch. That arrives with the guards. The
resolving trampoline and the residual path's `apply` are unbuilt, and criterion
26 stays unmet — its cases need a compiled body that continues past a call.
Conform is unmoved at 106/114, ceiling 106/114.

## D63 — both tiers make the exit refusal through one constructor

**Decided by the repository owner, 2026-09-13.** RFC-0005 §S5's `status_of` is
built, and the refusal it substitutes comes from `Error::exited(code)` in
`bund2-api` rather than from a format string in each tier.

- Blocks: nothing. It is the precondition for `status_of` agreeing with
  `Interp::exit_gate` under criterion 30
- Status: **RESOLVED — built.**

**The text is compared, not merely shown.** D52 makes `bund.exit` record a code
and return `Ok`; Tier 0 refuses at its next step, in `Interp::exit_gate`.
Compiled code has no next step, so §S5 has `status_of` make the same refusal.
Criterion 30 compares `?try`'s `error` CONDITIONAL `context` slot **as text**
across the two tiers, and `crates/bund2-stdlib/src/host.rs` pins the wording
with `ends_with`. Spelling the message in `Interp` and again in `bund2-jit`
would put two literals one crate apart that must agree forever, and the failure
would be a criterion-30 mismatch with every other stated reason still holding.

**So there is one constructor**, `Error::exited`, with an `is_exited`
predicate and a shared prefix constant, in the style `stack_exhausted` and
`internal` already use. `exit_gate` returns it; `status_of` substitutes it.
This is the same structural argument as `Registry::touch` for the generation
mirror and `Interp::set_autoadd` for the `autoadd` cell: one writer, so the
invariant cannot be forgotten rather than merely documented.

**`status_of` is also the request cell's first reader.** It clears the tail
request on every error it answers, through `Vm::clear_tail_request` — one of
the four writers that write §S6's mirror beside `pending_tail` — which moved
the clearing out of both adapters, where §S5 had recorded it as a gap left for
this helper.

**What this does not decide.** It does not build the other three helpers §S5
names: the resolving trampoline, the drain helper and the residual path's
`apply`. The drain helper in particular needs a `Vm` method, since
`take_pending`, `run_to` and `drain_frames` are private to `Interp`, and
draining writes `pending_tail` — so it falls inside assumption 33's named-writer
set and §S6's pairing rule. Criterion 30 stays unmet: its cases need a compiled
body that continues past a call. Conform is unmoved at 106/114, ceiling
106/114.

## D62 — `bund2` declares an 8 MiB Tier 1 share, and Tier 0's part carries no margin yet

**Decided by the repository owner, 2026-09-13.** §S8's stack-floor cell is
built, and the CLI adopts §S8's proposed default for the share.

- Blocks: nothing. It completes §S6's cell families and answers the ninth
  review's S3 in code
- Status: **RESOLVED — built.**

**The mechanism was already decided; this adopts the number.** §S8 resolved the
ninth review's S3 with "a share is declared, never inferred":
`set_stack_region_with_share(top, size, share)` names Tier 1's part, plain
`set_stack_region` leaves it at zero, and a default split was rejected because
it would take room from every embedder's Tier 0 without asking. That function
now exists, and `bund2`'s evaluation thread declares through it under `jit`.
Without that, §S8's own warning holds: criterion 2 would pass with no compiled
code having run.

**The share is 8 MiB, §S8's proposed default**, added *above* Tier 0's part
rather than carved out of it, so `EVAL_STACK` is `8 MiB + 8 MiB + STACK_RESERVE`
under `jit` and unchanged without it. Adding rather than carving is what keeps
D44's second requirement: a program whose native nesting fits with the tier off
still fits with it on.

**Tier 0's part stays at 8 MiB, not `8 MiB + m`.** §S8 has it carry a margin
`m ≥ 8 MiB × max_p(δ_p / c_p)` over every re-entering path under `jit`, and
says `m` is "a measurement taken when the tier exists, not a number guessed
now". Criterion 11 measures it. Guessing a margin here would be the guess the
RFC forbids, so the part is left as it is and the gap is recorded rather than
filled.

**The floor cell holds the Tier 1 floor, `top − share + STACK_RESERVE`, not
Tier 0's.** They are different addresses and the Tier 1 floor is the higher:
Tier 0's sits one reserve above the stack's *end*. Mirroring the wrong one
would admit compiled frames into Tier 0's part. It is written once, at
construction, because the floor is a property of the thread and never moves —
so unlike the epoch and the two flags it needs no writer discipline.

**A thread with no share gets a floor above its own top**, since the share is
zero and the reserve is added to the top. Every stack pointer is then beneath
it and every compiled body declines. §S8's "compiled code does not run there"
is therefore arithmetic rather than a flag, and an undeclared thread reaches the
same answer through `stack_marker`.

**What this does not decide.** It does not measure `m`. It gives the cell no
reader: no compiled body entry compares `get_stack_pointer` against it yet,
because no lowering emits that check. And it changes no meaning — conform is
unmoved at 106/114 with a ceiling of 106/114.

## D61 — the current-stack ring is sealed, and `Vm::cells` is required

**Decided by the repository owner, 2026-09-13.** Two shapes chosen while
building RFC-0005 §S6's cells, neither specified by the RFC.

- Blocks: nothing. It builds three of §S6's four cell families
- Status: **RESOLVED — built.**

**`Stacks::order` is private, and every current-stack change goes through a
mutator that bumps the epoch.** §S6 wants an epoch "bumped on every change of
current stack", and the obvious reading — bump at each call site — was already
violated: `drop_stack`, `rotate_stacks_left` and `rotate_stacks_right` reached
past `Stacks` into its `order` deque from `Interp`'s `Vm` impl. Instrumenting
`Stacks`'s own methods would have left three switches silent, and a silent
switch is a compiled body holding values from a stack no longer in force. The
ring is therefore sealed behind `drop_named`, `rotate_left` and `rotate_right`,
each bumping, which makes the invariant structural in the sense CLAUDE.md
prefers and assumption 37 already uses for generations. Assumption 40 states it.

The bump is **exact, not conservative**: a `to_stack` to the stack already
current, a rotation of a one-stack ring, and a drop of some other stack all
leave it alone. A conservative bump would be correct but would fire every guard
in a compiled body on programs that never switched, which defeats the
optimisation the cell exists to enable.

**`Vm::cells` is a required method, not a defaulted one — and the default was a
real bug, not a hypothetical.** It was first written with a `None` default so a
test double would owe nothing. `Interp` then inherited that default while owning
cells: the workspace compiled clean, and a tier asking through `&mut dyn Vm`
would have been told there were no cells and compiled no guard. A wrong answer
behind a green build is the exact failure this section exists to prevent, so the
method is required and each test double writes one line.

`a_tier_reaches_the_cells_through_the_trait` (`crates/bund2-interp/src/lib.rs`)
is written against `&mut dyn Vm` rather than against `Interp` deliberately:
asking `Interp` directly would have passed throughout, because the inherent
accessor was always correct. Only the trait object saw the bug.

**`Interp::autoadd` is private.** A `pub` field cannot be mirrored — any writer
bypasses the cell — so it has one writer, `Interp::set_autoadd`, which writes
the mode and the mirror together. This cost five call sites, all tests, because
`:` and `;` are unbound; it would cost much more after they land.

**What this does not decide.** It does not build §S8's stack-floor cell, the
fourth family §S6 lists. It gives the cells no reader: there is still no
residual path, no drain helper and no `status_of`, so nothing in compiled code
loads them. And it changes no meaning — conform is unmoved at 106/114 with a
ceiling of 106/114.

## D60 — compiled code is owned by a per-`Interp` compiler, and callers hold handles

**Decided by the repository owner, 2026-09-13.** One `JITModule` per `Interp`,
which is criterion 23's module half.

- Blocks: nothing. It closes the module half of RFC-0005's criterion 23
- Status: **RESOLVED — built.**

A module is a code allocator: each one reserves and finalises its own memory,
and §S4 never reclaims any of it. A module per compiled body multiplies that
fixed cost by the compiled-function cap, so the module is shared and the words
are emitted into it.

**The ownership follows from a lifetime, not from taste.** A compiled word's
entry pointer is valid only while the module that emitted it is alive. A
self-contained compiled value holding that pointer is therefore a dangling
reference waiting for its module to drop, with nothing in the type system
saying so. So the code lives in a `Compiler` and callers receive a
`WordHandle` — an index, which can only ever reach a word of the compiler it is
used against, and out of range answers `None`. It is deliberately **not** an
identity: every compiler numbers from zero, so a handle from one compiler used
against another names that other compiler's word. Nothing mixes them, because
the cache that stores handles lives in the same `JitTier` as the compiler that
issued them.

**Two Cranelift behaviours make the sharing correct**, both read against the
vendored 0.135 source rather than assumed. `JITModule::finalize_definitions`
drains its pending list with `mem::take`, so calling it once per compilation
finalises only that compilation's functions and leaves earlier code untouched
and executable. `declare_function` **merges** a duplicate name into the
existing `FuncId` and returns it rather than failing — right for the adapter
import, which every body shares, and a hazard for everything else: each body's
thunks, body and trampoline carry a sequence suffix, because without one a
second word would silently redefine the first and the symptom would be a wrong
answer rather than an error.

**What this does not decide.** It does not build §S6's cells, so criterion 23's
"one set of cells per `Interp`" is unbuilt rather than satisfied. It does not
change the two test-only entry points, `compile` and `compile_body`, which
still build a module each; neither is on an `Interp`'s path. And it changes no
meaning: conform is unmoved at 106/114 with a ceiling of 106/114.

## D59 — the JIT feasibility gate is answered, and Tier 1 is authorised

**Decided by the repository owner, 2026-09-12.** RFC-0005 moves from Draft to
**Proposed**, and §S2–§S11 are authorised to be built.

- Blocks: nothing. It **unblocks** D43's `bund2-api` additions, which that
  entry held "to be built when this RFC reaches Proposed"
- Status: **RESOLVED — authorised.**

`docs/research/00-jit-feasibility.md:271-273` made Project B conditional:
"Project B is worth doing only if Project A's measurements show that dispatch
and boxing are still the bottleneck. The recommendation is to sequence them
and put a hard decision gate between them." RFC-0005 §S1 split that into two
prerequisites and had carried "half met" since 2026-09-08.

**Both halves are now answered by measurement.** Prerequisite 1 asked for
`value/push_pull/balanced` under 20 ns; D41 delivered 9.53 ns. Prerequisite 2
asked whether dispatch had become the bottleneck, and §S1 had said these
benchmarks could not show it — true of *subtracting* them, and not true of
`crates/bund2-bench/benches/fragment.rs`, which constructs the separation in
one harness over one program. Decomposed, a word is **83.8%**
dispatch-plus-value-traffic on `Int + Int` and **88.6%** on `dup drop`, with
11–17% left for the addition and the pop. `dispatch_isolated`, added the same
day, measures dispatch outright at **22.44 ns** a word with the work held at
zero, where the older family could only bound it at ~27.9.

**What this decision does not do.** It does not pre-empt **criterion 10**,
which remains the experiment that can fail: a speedup below 1.2× on
`1 2 + drop` reopens the gate rather than being explained. It does not
authorise promotion independently of inlining — Q33's staging stands, inlining
first because promotion cannot exist without it. And it does not make the RFC
Accepted: RFC-0000's bar there is a review pass that finds nothing, the
twenty-first pass found two blockers, and most of the thirty criteria cannot
run until a `bund2-jit` exists.

**A measurement that cuts against building, recorded here so the decision is
not read as unanimous evidence.** Arithmetic's inlining ceiling — `tier0` over
`lowered` on `Int + Int` — reads **1.77× and 1.81×** across two runs on this
machine, *below* criterion 10's 2× stop rule, where RFC-0005 records 2.03×
just above it. The crossing is machine-dependent and reproducible here. It
strengthens the RFC's own prediction that arithmetic inlining alone will not
clear the rule, and it is the concrete shape the stop rule may take: the
operand-free arm has room at 3.50×/3.57×, arithmetic may not.

## D56 — a caller that runs a native can discard the tail request it filed

`Vm::tail_lambda` files a body for the loop to run after the current native
returns. F96 fixed what happens when that native then fails: `Interp::invoke`,
the one place Tier 0 calls a native, clears `pending_tail`, so nothing runs a
body no caller asked for. `invoke` is private to `Interp`.

RFC-0005's compiled tier does not reach it. Its per-native adapter calls the
native's `NativeFn` directly (§S8), so a native that files and then fails
would leave the request set, and Tier 0's next `take_pending` would run that
body — inside `?try`'s handler, where the fifteenth review found it. The RFC
answered that `status_of` "clears the request cell", and the seventeenth
review found the RFC had no way to say whether that reached Tier 0's
`pending_tail` or only the compiled mirror. Only the mirror leaves the defect
standing.

### Decision

Decided by the repository owner, 2026-09-11: **`bund2-api` gains
`Vm::clear_tail_request()`**. It discards a filed body that has not run, and
does nothing when none is filed. `Interp` implements it as `invoke`'s clear.
RFC-0005's adapter calls it whenever it answers an error, so a compiled call
leaves exactly what a Tier 0 call leaves.

This widens the API, which is why it is recorded here, as D52 was. Every `Vm`
implementation has to answer it.

`every_writer_of_the_request_cell_is_named`
(`crates/bund2-interp/src/lib.rs`) derives the write set — `request_tail`,
`take_pending`, `invoke` and this — and fails when a fifth appears. RFC-0005's
assumption 33 states the property that test enforces.

### Rejected

- **Exposing `Interp::invoke` to `bund2-jit`.** No API change, but it binds
  the compiled tier to `Interp` rather than to the `Vm` trait every embedder
  implements.
- **Clearing only the compiled mirror.** The request would survive in Tier 0,
  which is the defect.

- Decided by: repository owner, 2026-09-11
- Blocks: nothing
- Depends on: D14 (the API surface), F96
- Status: **RESOLVED — and in use since 2026-09-13.**

*Dated note, 2026-09-13 — first consumer.* The sentence above — "RFC-0005's
adapter calls it whenever it answers an error, so a compiled call leaves exactly
what a Tier 0 call leaves" — was a statement about code that did not exist. It
exists now: `jit_call_native` (`crates/bund2-jit/src/lower.rs`) calls
`Vm::clear_tail_request` on any error from the native it ran, closing F96's
parity gap for compiled code. This method was added with no consumer, which
D43's caution warns against; the caution was right about the *risk* and the
decision was right about the *need*, and the gap between them was two days.

Two things this note should be honest about. **The call is in the adapter, not
in `status_of`**, which RFC-0005 §S5 nominates and which does not exist yet —
the obligation is the adapter's either way. And **the compiled mirror this entry
rejected clearing *only*** is still not built at all: §S5's request cell has no
reader until a drain helper exists, so "clears both" is, today, "clears the one
that exists". `every_writer_of_the_request_cell_is_named` still derives four
writers and still passes: the adapter writes `pending_tail` through the trait,
not directly.
