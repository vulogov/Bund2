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
no files in the wild to break. External dependence would require a Rust
consumer calling `rust_dynamic` directly, which is a different question from
the one this entry asks and one the repository owner is positioned to answer
outright.

Two things this unblocks besides RFC-0003. **D20**'s deferred step — encoding
"unset" in the wire format so laziness survives serialisation — becomes
available, since it was held back only on this. And it does **not** discharge
D27's condition, which is D31.

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
- Status: **RESOLVED — decided. Not yet built**, because promotion across calls
  is not yet built: D66/D67 sync before every call. The rule is settled for
  when it is.

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
