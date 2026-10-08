# RFC-0007: Concurrency, and the two buses

- Status: **Proposed** (2026-10-04, on the owner's authorisation — **D93**),
  after one adversarial review of the document and the implementation of
  everything in it. **All fourteen criteria are discharged**: 1, 2, 3, 5, 6 and
  8 met against the oracle; 9, 10, 11, 12, 13 and 14 met as design; 4 deferred
  as a deviation filed as preservation; 7 withdrawn for naming no check.

  **What "Proposed" means here is narrower than in RFC-0005 or RFC-0006, and
  that is the thing to carry away.** Those were preservation documents: the
  oracle could adjudicate every criterion, and a disagreement was a bug.
  **§C8 is design.** The reference has one VM and spawns no threads, so its bus
  is a queue from a VM to itself, and criteria 9–14 are the first criteria in
  this repository the oracle cannot settle either way. They say what Bund2 does
  and that a test holds it; they cannot say the reference agrees, because the
  reference has no opinion. **Do not cite them as preservation claims.**

  **What is not authorised: zenoh.** §C4 stays deferred under D28 and D87, the
  `--distributed` flag is unbuilt, and how a channel address selects a transport
  is still an open question.

  **The status line this replaces was false for three sessions**, and it is kept
  here because the pattern is the subject. It said the RFC "cannot be proposed"
  because the eight words were out of scope under D28 and because nothing in it
  involved a second VM — and D87 had scoped the words in and §C8 had been
  written. A status nothing contradicts is read as authoritative; see F109's
  addendum and F146. Two further things that line never mentioned were found by
  correcting it: the criteria did not reach the section they were supposed to
  govern, which was the first draft's "specified the exchange layer and called it
  concurrency" surviving its own revision; and §C8 had recorded a decision and
  left it there, now D91.

  The first adversarial review found **four blockers, two of them errors in
  measurements this document claimed to have taken against the oracle**
  (`docs/rfc/reviews/RFC-0007-review-2026-10-01.md`). That is what the review
  was worth: not that the design was wrong, but that two numbers in it had not
  been measured and were written as though they had.
- Depends on: RFC-0002 (the word kinds this defines), RFC-0003 (the flat frame
  loop)
- Decisions consumed: D10, D20, D28, D40, D44, D84, D85, D86, D87, D88, D91,
  D92, D93
- Touched but not consumed: D16, D27, D31
- Reference SHA: `reference/Bund` at `21b40b0`, per `reference/PINNED.txt`
- Supersedes: `docs/research/01-extensibility-async.md` §2 in two places, now
  recorded in `docs/research/ERRATA.md`. **The first draft claimed it "departs
  nowhere", and the review found five places where it does** — §C3's ENVELOPE
  and the shared compile service among them.

## Summary

Three decisions taken on 2026-09-30 settled what this RFC is, so it is smaller
than its roadmap entry suggests.

**D85** fixes the count at tens, because scale comes from processes rather than
VMs in a process. **D84** fixes a task as a VM, with no suspension inside a
word. **D86** fixes the exchange layer as two orthogonal transports —
crossbeam in process, zenoh across it — the owner's framing being Erlang's,
"distributed and local message exchange coexist".

What remains is a **preservation contract** for eight bus words that already
exist in the reference, the meaning of two `WordKind` variants that exist with
no writer, and the one thing none of the above settles: what a Bund program can
say about concurrency, as opposed to what the runtime can do.

## Motivation

The research disambiguates the question four ways
(`docs/research/01-extensibility-async.md:224-236`), and the verdicts still
hold:

| | feature | verdict |
|---|---|---|
| (a) | `run()` as an `async fn` that does not block the executor | do it |
| (b) | many independent VMs concurrently on a thread pool | do it |
| (c) | a word performs I/O and yields mid-execution | achievable at Tier 0 |
| (d) | parallel execution within one program | **not recommended** — "a different language" |

**D84 answered (c) in the negative and that is now settled**: no suspension
inside a word, because stepping and awaiting both need only that a task *stop*,
and a task is a VM with its own thread. So this RFC is (a) and (b), and (d)
stays out by the research's own verdict.

**The research reached today's design independently, and said so in 2026-08.**
"**Recommend `Rc` with the actor model.** This is not a compromise — it is the
design that matches what Bund already is… each VM is `!Send` and owns its
values; VMs run on their own task; values crossing a VM boundary are converted
to an owned, `Send` representation" (`:285-291`). D85 and D86 arrived at the
same place from a measurement and from the reference's code. **Three
independent arrivals at one design is the strongest evidence this RFC has**,
and it is worth stating plainly because it is the reason this document proposes
so little that is new.

## Current behaviour

### The local bus exists, and it is crossbeam

Eight words, all unimplemented in Bund2, all in the in-scope set. Every fact
below was measured against the oracle on 2026-09-30, not read off the
research — which describes a different payload (§C3).

| word | behaviour |
|---|---|
| `send` / `send.` | Name then object, **object on top**; serialises with `to_binary` and pushes down a channel named by the string; pushes a bool. `eff(2, 1)` |
| `send.quick` / `send.quick.` | Identical, and **discards the bool**. `eff(2, 0)` — the only difference between the two pairs |
| `recv` / `recv.` | Name; pushes the decoded value. **An existing but empty channel pushes `NODATA`; only an absent channel errors** |
| `bus.data` | Name; pushes whether the channel has data — and **creates the channel if absent** |
| `bus.data.current` | The same, using **the current stack's own name** as the channel |

The transport is `PIPES`, a
`Mutex<BTreeMap<String, (Sender<Vec<u8>>, Receiver<Vec<u8>>)>>`, with
`crossbeam::channel::{Sender, Receiver, unbounded}`
(`reference/Bund/src/stdlib/functions/bus/mod.rs:9,16`), and `crossbeam` is a
direct dependency (`reference/Bund/Cargo.toml:40-41`).

**Three measured facts that decide this RFC's criteria.**

**The local bus needs no network and no `--distributed`.** `"chan" 42 send`
then `"chan" recv println` prints `42` under a plain `bund script`. Zenoh is a
separate transport in the same module — `ZENOH` is a `Mutex<zenoh::Session>`
at `mod.rs:23`, used by the globals paths in `bus/globals.rs` and **not** by
these words.

**`recv` does not block**, so a golden cannot hang on it — but the first draft
got *why* wrong, and the distinction matters. `bus_pull` has two arms: an
**absent** channel bails with `bus::internal::pipe no pipe: {name}`, and an
**existing but empty** one returns `Ok(Value::nodata())`
(`bus/mod.rs:122-131`). The cited `crossbus.rs:144-147` is only the wrapper
that propagates whichever comes back. The draft's single measurement —
`"empty" recv` on a channel nothing had created — hit the *absent* arm and was
generalised to the empty one.

**And two channels exist before any program runs.** `"in"` and `"out"` are
inserted at initialisation (`bus/mod.rs:46-47`), so `"in" recv` yields `NODATA`
where any other unused name errors. The first draft never mentioned them.

Combined with captures normalising id and stamp, **the local bus is still
goldenable** — which no zenoh-backed word is — but §C7 is now whether a golden
may exist at all.

**`bus.data` is not a pure predicate.** `ensure_bus` inserts the channel before
reporting whether it has data (`mod.rs:58-70`), so asking the question creates
the pipe. Preserved, and stated because a reader will assume otherwise.

### Two word kinds exist with no writer

`WordKind` is `Sync`, `Blocking`, `Async` (`crates/bund2-api/src/lib.rs`). All
**258** stdlib declarations say `Sync`; nothing writes the other two, and
nothing reads `kind` at all beyond a `Debug` impl.

**So this RFC defines their meaning for the first time rather than inheriting
one.** Unlike F121's dead `Slot` fields — deleted the same day because the
slot's generation would have covered them by accident — there is no latent
hazard here, because no code branches on the value. What there is, is an
unspecified vocabulary that RFC-0002 added in anticipation of this document.

## Design

### §C1 — A task is a VM, and there are tens of them

D84 and D85, restated once so this document stands alone: a task owns a VM, a
VM is `!Send`, nothing suspends inside a word, and the expected count is tens
in a process because scale comes from processes on a bus. Measured cost:
`Slot` is 96 bytes over 396 names, so ~75 KiB a VM — 5 MB at 64 VMs, ~300 MB
at thousands, which is why tens.

### §C2 — The local bus, preserved

The eight words above, with the behaviour the contract records: operand order,
the `eff(2,1)`/`eff(2,0)` split, a non-blocking `recv`, `bus.data`'s channel
creation, and `bus.data.current` keyed by the current stack's name.

**The payload is the wire format**, which Bund2 already has in
`bund2_value::wire` — fixture-tested, with `MAX_WIRE_DEPTH` at 256 refusing
what it could not read back. D86 fixed this, and it is not a choice: a
`BundValue` carries `Rc` and a VM is `!Send`, so a channel between VMs **must**
carry bytes.

**The reference preserves identity and stamp across the bus, and the first
draft asserted the opposite.** Measured: one value through `send`/`recv` reads
`id: "gHKQUMqeE5RYPQCCvBC9c" stamp: 1790837306409.0` on both sides.
`Value::from_binary` decodes a serialised `Value` whose `id` and `stamp` are
fields, so it restores them. The error came from reading D20's *materialises* as
*replaces*; it means **set if unset**.

**Bund2's decoder does not preserve the identity**, and F109 already recorded
that: `wire::into_value` builds with `identity: Cell::new(0)`, so a decoded
value is unminted. **The bus is the first place that deviation becomes program
visible** — comparing identity across `send`/`recv` answers *same* on the oracle
and *different* on Bund2, and no golden catches it because captures normalise
ids. D86 carries the correction and names the choice it obliges: preserve the
decoded identity and narrow F109, or keep the deviation and record that the bus
exposes it.

**The channel is unbounded**, so a producer outrunning a consumer grows memory
without limit. The reference's choice, preserved; bounding it would be a
deviation needing a decision.

### §C3 — ENVELOPE: a layer the research names and the reference does not use

The research proposes the boundary value: "`rust_dynamic` already has exactly
the right primitive… `Value::wrap()` produces an `ENVELOPE` containing a
bincode serialisation, and `unwrap()` reconstructs it. An envelope is by
construction `Send`. The message-passing boundary between VMs is a mechanism
the language already has a word for" (`:287-291`).

**The reference's bus does not use it.** `bus_push` calls `to_binary` on the
value directly (`mod.rs:88`); no ENVELOPE is constructed, and `wrap`/`unwrap`
are separate words producing a value that stays on the stack (D11).

**So there are two layers, not a contradiction**, and this RFC keeps them
apart: the bus's payload is raw bincode bytes, invisible to the program;
`wrap`/`unwrap` are program-visible words a Bund author may use to hold a
serialised value *as a value*. **Whether `send` should accept an ENVELOPE
without re-serialising it** is an optimisation this RFC does not take, because
it would make `send` behave differently for one `dt` and the reference does
not.

### §C4 — Zenoh, deferred with its boundary stated

D86 makes the two transports orthogonal. The distributed half is **not
specified here** beyond what D86 fixes, for a reason worth recording: zenoh is
a large dependency, D40's precedent is that a transport's dependencies reach
the toolchain-free artefact and have to be feature-gated, and the roadmap flags
the distributed layer as something "no document has accounted for".

**What is fixed**: the local transport is not zenoh, the payload is the wire
format either way, and neither transport is layered on the other. **What is
not**: the globals path's semantics, what `--distributed` means for a Bund2
node, and whether zenoh is a default feature. A later RFC or an amendment takes
them.

### §C5 — `Blocking` and `Async`: what this RFC proposes they mean

Nothing reads `kind` today, so these definitions are the first.

- **`Sync`** — returns without yielding the thread. Every word today.
- **`Blocking`** — may park the thread for an unbounded time: the host words,
  `recv` if it ever gained a blocking form, a fetch. A task running one blocks
  only its own VM, which is what D84's model makes safe.
- **`Async`** — **this RFC does not define it, and says so.** D84 removed
  suspension inside a word, so there is no mechanism an `Async` word could use
  that a `Blocking` one does not. Defining it now would be inventing a
  vocabulary with no consumer, which is what §C6 declines.

### §C6 — What a program can say, which this RFC does not settle

Everything above is about what the *runtime* does. **What a Bund program can
say about concurrency is undecided**, and the honest reason is that nothing in
the reference says anything: there is no `spawn`, no task word, no way to ask
how many VMs exist or to run code on another. The bus words assume the other
end exists and was started by something outside the language.

So the shape of the question is: does a Bund program create tasks, or does an
embedder create them and hand each a program? **The reference answers the
second by omission** — nodes are processes, started with `--distributed`,
joined by a bus. This RFC follows that and **adds no spawn word**, which means
concurrency in Bund2 is an *embedding* feature with a message-passing
vocabulary, not a language feature.

That is a defensible position and it is not obviously the right one. It is in
the open questions rather than settled here.

### §C7 — The local bus is in scope; zenoh is not — D87

**D28 deferred `Bund/src/stdlib/functions/bus` whole**, which the first draft
neither cited nor noticed. `DEFERRED_PATHS` in
`xtask/src/corpus/classify.rs` carried one entry for that directory, reason
"zenoh distributed bus — not essential".

**The directory is mixed**, which is why the reason was half wrong:
`crossbus.rs` is the crossbeam bus, `globals.rs` is zenoh's (`global`,
`global*`, `?global`), and `mod.rs` holds both transports while registering
nothing. **That reason was the third of three places attributing `send`/`recv`
to zenoh from a directory name** — the path audit and this RFC's own first draft
being the others, the draft having claimed the opposite.

**D87 narrows the deferral to `globals.rs`** and states the local bus's
purpose: exchanging data between VMs running locally. The eight words are in
scope as of 2026-10-01.

**Measured consequence, recorded because a completeness number that only
improves is measuring its own scope:**

| | before | scoped in | built and probed |
|---|---|---|---|
| out of scope by decision | 120 | **112** | 112 |
| words in scope | 497 | **505** | 505 |
| IMPLEMENTED | 391/497 (78.7%) | **391/505 (77.4%)** | **399/505 (79.0%)** |
| COVERAGE | 387/497 (77.9%) | **387/505 (76.6%)** | **395/505 (78.2%)** |
| CONFORMANCE | 107/116 | **107/116**, unmoved | **109/118** |

Eight unimplemented words entered the denominator, so both completeness figures
fell about 1.2 points with no code changed. That is the honest direction.

**The third column is 2026-10-01, after the words landed and
`tests/golden/probes/bus-words.golden` was captured.** All eight are implemented and
all eight are now run by a golden, so IMPLEMENTED and COVERAGE each moved by
exactly eight from the scoped-in baseline — the figure criterion 5 predicted,
395/505, read back from the binary rather than argued. Conformance grew by two
in both halves: this RFC's probe, and the corpus program D89 admitted. Both
pass, so the ceiling still equals the numerator.

**Earlier RFCs' `107/116` is as of their dates**; 116 was the denominator before
these two captures, and criterion 6 is where that is accounted for. COVERAGE
did not move with the second capture, because the probe had already reached all
eight words — the corpus program covers the *reference's* use of them, not a
word the probe missed.

**Transparency is the intent and it does not strain D86.** The owner's second
clause — `send`/`recv` should eventually be transparent across zenoh — is
Erlang's model: **one vocabulary, two transports**, where a channel's address
decides which carries it. D86's orthogonality is a claim about the transports,
not the words. **How an address selects a transport is undecided**: zenoh's side
is keyed by paths through `get_globals_path`, a local channel is a bare name in
`PIPES`, and the reference has no answer because its `send`/`recv` never reach
zenoh. It is in the open questions.

### §C8 — The second VM: a host, a thread each, and one shared map

The first draft specified the *exchange layer* and called it concurrency. The
reference cannot help here — it has one VM and spawns no threads, so its bus is
a queue from a VM to itself and every criterion below passes with one VM. **So
this section is design, not preservation**, and it is marked as such throughout.

**(a) and (b), the two the research rated "do it"** (`:224-236`): `run()` as an
`async fn` that does not block the executor, and many independent VMs
concurrently.

#### A VM is `!Send`, so the host owns threads rather than futures

`Interp` is not `Send` — `Box<dyn Reporter>` and, through every value,
`Rc<HeapValue>` (verified for D84). So a future holding one is not `Send`
either, and the shapes available are a `LocalSet` or **one thread per VM**,
which is what the research names (`:293`).

**Bund2 already has the thread.** `bund2-cli`'s `main` spawns one sized
`TIER0_PART + TIER1_SHARE + STACK_RESERVE` and declares the region with
`set_stack_region_with_share` (D44, §S8). A VM host spawns N of those, and
**each must declare its own share** — the lesson criterion 18 and
`compiled_entries` exist for: a thread that declares none puts the Tier 1 floor
above its own top and every compiled body declines while every figure reports
success.

**`bund2-async` is where this goes.** The crate exists as a twelve-line stub
whose doc reads "Executor integration and async native words. Feature-gated.
See RFC-0007", which RFC-0000 assigns here. §C8 fills it: a host that spawns
VM threads, the `async fn` façade over one, and nothing else — the words stay
in `bund2-stdlib`.

#### `PIPES` stays process-global, which is what makes it a bus

The reference's map is a process-global `Mutex<BTreeMap<…>>`, and under tens of
VMs that is **correct rather than incidental**: a bus between VMs needs a shared
medium, and a per-VM map would be a bus to oneself. It is also why the payload
must be bytes — the map is the one thing every VM touches, and a `BundValue`
could not live in it.

**Two races the reference cannot exhibit and tens of VMs can**, stated because
they are invisible with one consumer:

- **`bus.data` then `recv` is not atomic.** `ensure_bus` reports
  `!r.is_empty()` and releases the lock; another VM may consume in between, so a
  true answer followed by `recv` can still yield `NODATA`. **Preserved** — the
  words have no combined form and inventing one would be new behaviour — and
  documented as advisory rather than a guarantee.
- **`recv` has no fairness.** `unbounded` with several receivers delivers to
  whichever wakes first. The reference's choice carries no ordering promise
  between consumers because it never had any.

#### The JIT conflict, which this RFC records rather than resolves

**D60 chose one `JITModule` per `Interp`** — criterion 23's module half — and
research §2.6 calls that option "Bad" under concurrency in terms: "every
thread recompiles the same hot words and every thread's code memory grows
independently… N threads means N× the unbounded growth". It recommends **one
shared compile service**, a `Mutex<JITModule>` on a dedicated task, and notes
that only that makes the code-memory cap "a single enforceable number".
§2.7 lists it as "required under concurrency".

**Bund2 has the per-VM form, deliberately**: D60 derives it from a lifetime —
compiled code dies with its `Interp` — and criterion 23 asserts that a body
compiled for one `Interp` is never run by another. Those are not reversible by
this RFC.

**Why it is tolerable today, measured rather than assumed.** D85 bounds the
count at tens; F139 found that **no program in `tests/golden/HERMETIC.txt`
compiles a single body even at threshold 64**, because they run once; and D74
set the shipped threshold to 1024. So N× growth over N VMs that compile nothing
is N× zero. **It is bounded in practice and unbounded in principle**, since
§S4 reclaims no code memory.

**The trigger, recorded so this is not rediscovered**: a workload where several
VMs each tier up the same hot word. Then the shared compile service becomes
required as §2.7 says, and it collides with criterion 23 — a shared module
means a body compiled under one `Interp`'s cells being called from another,
which is precisely what that criterion forbids. **That is a decision, and it is
not this RFC's to take.** `docs/research/ERRATA.md` records the departure.

**It is now D91**, so a work item can be blocked on it rather than rediscover
it. D91 adds a third option the research did not consider and this section did
not either: **share the compile *work* without sharing the emitted code** — one
translation of a body to Cranelift IR, cached by `payload_key`, with each
`Interp` still emitting and owning its own. That takes the N× compile-time half
of the win and leaves criterion 23 untouched, which is why D91 recommends it
*before* the shared module rather than instead of it.

#### What tens of VMs costs in address space

Each VM thread reserves `EVAL_STACK` — 8 MiB for Tier 0's part, 8 MiB for Tier
1's share under `jit`, plus `STACK_RESERVE`. At sixty-four VMs that is **about
a gigabyte of reserved address space**, virtual rather than resident, which is
free on 64-bit and is not on 32-bit.

**And 32-bit is exactly where `--emit=bundle` is aimed** — D10 makes it the
answer for targets Cranelift does not support, which include the 32-bit ones.
There `TIER1_SHARE` is zero, so it is 8 MiB a VM, and tens of VMs is still
half a gigabyte. **So D85's "tens" is a 64-bit statement.** On a 32-bit target
the practical count is lower, and this RFC says so rather than leaving a
number that does not travel.

## Preservation analysis

| behaviour | disposition |
|---|---|
| `send`, `send.`, `send.quick`, `send.quick.` | **Preserved**, including the bool/no-bool split and the operand order. |
| `recv`, `recv.` | **Preserved**: an absent channel is an immediate error and an existing, empty one answers `NODATA` (`reference/Bund/src/stdlib/functions/bus/mod.rs:122-131`). Until 2026-10-08 this row said an empty channel was the error, which the summary's own correction had already withdrawn. |
| A value nested past 256 levels, at `send` | **Deliberately changed — D114.** The oracle sends it; Bund2 refuses with the reference's prefix for a failed encode. Criterion 4. |
| `bus.data`, `bus.data.current` | **Preserved**, including that asking creates the channel, and the current-stack keying. |
| The payload's identity | **Preserved as a deviation already recorded** — D20 materialises id and stamp, so a received value is equal and not identical. |
| Unbounded channels | **Preserved.** A bound would be a deviation with a decision. |
| `WordKind::Sync` | **Unchanged.** 258 declarations, now with a stated meaning. |
| `WordKind::Blocking` | **Defined here for the first time.** No writer existed. |
| `WordKind::Async` | **Deliberately left undefined** — §C5. |
| Zenoh's globals paths | **Out of scope**, §C4. |
| Parallelism within one program | **Excluded**, on the research's own verdict (d). |

## Alternatives considered

- **`Arc` everywhere**, so values cross threads directly. Rejected by the
  research in 2026-08 — "a permanent tax on the hot path, paid by every
  program, to enable a feature most programs won't use" — and by D86's
  measurement that the payload must serialise regardless.
- **Fine-grained suspension**, so a word can yield. Rejected by D84, which
  priced it at 27 `eval_lambda` call sites across nine files, several holding
  Rust state across the body.
- **Thousands of VMs with a shared word table.** Rejected by D85: ~300 MB of
  word tables, a copy-on-write overlay against §S6's per-`Interp` generation
  cells, and no consumer to validate it.
- **One transport, with the local case as a loopback over zenoh.** Rejected by
  D86 on the owner's framing: local and distributed coexist, as in Erlang, and
  the reference already implements them as separate transports.
- **`send` accepting an ENVELOPE without re-serialising.** Not taken, §C3.

## Acceptance criteria

**§C7 is answered by D87**, so these are no longer conditional — the eight
words are in scope and may carry goldens.

1. **The eight bus words match the oracle**, by probe: operand order; the bool
   from `send` and its absence from `send.quick`; **`recv` pushing `NODATA` on
   an existing-but-empty channel and erroring only on an absent one**; the
   pre-created `"in"` and `"out"`; and `bus.data` returning false *after*
   creating a channel that did not exist.

   **Met for everything a golden can hold.** `tests/probes/bus-words.bund`
   covers all eight words and every claim above except the absent-channel error,
   and diffs byte-identical against the oracle. The error cannot go in a probe:
   it renders through the reference's diagnostic table, which F66 already
   records as text no second machine reproduces, so a probe for it would be a
   third approved deviation for the frame rather than the message. Its exact
   text — `RECV returns error bus::internal::pipe no pipe: <name>`, prefixed by
   the calling word and not by `BUS.DATA` — is asserted in
   `crates/bund2-stdlib/src/bus.rs` instead, as are the six depth guards and the
   `BUS.DATA:` name-casting prefix every one of the eight shares.
2. **A `send`/`recv` round trip is goldenable** — `"chan" 42 send "chan" recv`
   captured from the oracle and matched.

   **Met, 2026-10-01.** `tests/golden/probes/bus-words.golden` is captured and passes.
   The capture refused nothing: `cargo xtask golden` runs each program twice and
   drops it if the two runs differ, which is the funnel's own test of
   reproducibility, and a process-global channel map passes it because a fresh
   process starts with `"in"` and `"out"` empty and nothing else.

   **And the corpus program came in with it.** D89 narrowed the hermetic
   funnel — `Effect::LocalBus`, hermetic, for `crossbus.rs` only — which
   admitted `examples/code_snippets/internal_bus_demo.bund`: fifteen sends to
   channel `A` drained by a `bus.data`/`recv` loop. That is the reference's own
   use of the bus rather than ours, and it diffs byte-identical. `HERMETIC.txt`
   goes 57 → 58, exactly one program.
3. **Identity across the bus is decided and then asserted.** The oracle
   preserves id and stamp; Bund2's decoder mints a fresh identity (F109). The
   criterion is that whichever D86's amendment settles, a test asserts it —
   **not** that the values differ, which is what the first draft asserted and
   the oracle contradicts.

   **Met, 2026-10-01. D88 resolves it: keep F109, and it is the rule for every
   decoder** — `recv`, `sqlite`'s BLOB cells and the world file all decode to a
   value carrying the sender's stamp and no identity. The assertion half is
   `a_received_value_keeps_its_stamp`, which asserts the stamp because that is
   the part the reference defines, and does not assert the id because nothing
   can observe it. Restoring the id is also not representable: the identity slot
   is a `Cell<u64>` and the reference's is 21 random digits over nanoid's
   alphabet. F109 carries the addendum.
4. **Deferred, and it was a deviation filed as preservation.** The first draft
   required a refusal at `MAX_WIRE_DEPTH`'s 256 levels; the oracle sends a
   300-deep list without complaint. Refusing would be **new behaviour**, so it
   needs a decision and a deviation entry, not a criterion.

   **The refusal exists, and is now decided — D114, 2026-10-08.** The
   acceptance review of that date measured it: `send` has refused a value
   nested past 256 levels since the bus words were built, because it encodes
   through the codec F118 bounded, and nothing recorded that for the bus. The
   owner ruled that it stays, as an approved deviation. 256 levels cross and
   257 are refused with `SEND returns error Error enveloping data: the value
   nests 257 deep, and 256 is the most the wire format can carry`;
   `a_value_nested_past_the_wire_bound_is_refused_at_send` holds both sides.
   So this criterion is **met as a deviation**, and the paragraph above
   described a state the binary was never in.
5. **Coverage moves by eight, from a denominator D87 already moved.** The
   baseline is **387/505**, not 387/497: scoping the words in lowered both
   completeness figures ~1.2 points before any were implemented. Implementing
   and probing all eight takes COVERAGE to 395/505; each word not probed is
   listed with its state. The first draft's "moves by exactly eight" was
   unmeetable under D28 and is now meetable against the right denominator.

   **Met exactly, 2026-10-01: COVERAGE 395/505, IMPLEMENTED 399/505.** The two
   halves were measured separately and the gap between them is the point.
   Implementing the eight took IMPLEMENTED to 399/505 and COVERAGE only to
   390/505, because at that moment just `send`, `recv` and `bus.data` were run
   by any golden — the other five sat under "implemented but run by no golden",
   which is the gap the criterion is about and the reason COVERAGE and not
   IMPLEMENTED is the completeness number. Capturing
   `tests/golden/probes/bus-words.golden` closed all five, and `implemented but run by
   no golden` fell from 9 to 4 with no bus word left in it.
6. **Conformance does not regress, measured after any new probes are
   captured.** Not "moves by exactly zero": criteria 1 and 2 add probes, which
   enlarge the denominator — the same contradiction the review found in
   RFC-0006's first draft and that this draft reproduced.

   **Met, 2026-10-01: 107/116 ceiling 107/116 before, 109/118 ceiling 109/118
   after**, in the default build and under `--features jit`. Every golden that
   passed still passes and the nine approved deviations are the same nine; M
   grew by exactly two — this RFC's probe and the corpus program D89 admitted.
   The ceiling still equals the numerator, so no golden is left unreached, which
   is the check that both new captures pass rather than merely existing.
7. **Withdrawn.** The first draft's "nothing reads `WordKind` yet, and that
   stays true" named no check and could not fail. What replaces it belongs to
   whichever RFC gives `Blocking` a reader.
8. **`--noio` disables all eight words**, which the first draft had no row or
   criterion for, and the probe asserts the stub message rather than silence.
   **Met** — a flag cannot appear in a golden, so the assertion is a unit test
   over all eight names (`noio_replaces_every_one_of_the_eight`), checked
   against the oracle's own `--noio` run for each of the three shapes.

### §C8's own criteria — added 2026-10-03, because criteria 1–8 all pass with one VM

§C8 states the gap in terms: "every criterion below passes with one VM". These
are the ones that cannot. **Each names what would fail if §C8 were wrong**, and
every one is design rather than preservation — the reference has one VM and
spawns no threads, so the oracle cannot adjudicate any of them.

9. **`Interp` is not `Send`, and a refactor cannot quietly make it so.**
   D84's whole shape rests on it: the host owns threads rather than futures, the
   debuggee inspects itself (RFC-0008 §D1), and the bus carries bytes rather
   than values because nothing else can cross. If `Interp` ever became `Send`,
   three documents would silently describe a constraint that had stopped
   existing. Checked by a `compile_fail` doctest rather than by a comment,
   because the claim is about the type system and only the compiler can hold it.

   **Met, 2026-10-03** — `crates/bund2-interp/src/lib.rs`, the doctest on
   `Interp`. It is the one criterion here that needed no new code.

10. **A host spawns N VM threads and each declares its own stack region with
    its own Tier 1 share**, asserted by `compiled_entries()` reading `> 0` on
    **every** thread rather than in aggregate. §C8 names this as the lesson
    criterion 18 and `compiled_entries` exist for: a thread that declares no
    share puts the Tier 1 floor above its own stack top, so every compiled body
    declines at entry while every figure still reports success. A sum over
    threads would be satisfied by one thread compiling and N−1 declining, which
    is exactly the failure being guarded.

    **Met, 2026-10-03** — `every_vm_thread_declares_its_own_region_and_share`,
    four VMs under `--features jit`, each asserted separately.

    **Verified load-bearing**, which matters more than that it passes: with
    `declare_region()` removed from `Host::spawn` it fails with `VM 0 entered
    no compiled body, so its share was not declared: Some(0)`. That is §C8's
    stated failure reproduced on demand, and `Some(0)` rather than `None` is
    what makes it the right assertion — a tier *exists*, it simply declines
    everything.

    **The first version of this test passed for the wrong reason**, which is
    worth recording. `Host::spawn` built `Runtime::new()`, which takes D74's
    shipped threshold of 1024, so a four-iteration body compiled nothing and
    every thread read `Some(0)` — F139's finding arriving as a false pass. The
    host gained `with_jit_threshold` so the tier can be made to act. Had the
    assertion been "no error" rather than "`> 0`", it would have passed with no
    compiled code anywhere, which is the shape this criterion is *about*.

    **One function, not two copies.** `TIER0_PART`, `TIER1_SHARE`, `EVAL_STACK`
    and the two-line declaration moved from `bund2-cli`'s `main` into
    `bund2_runtime::declare_region`, which both callers now use. Two copies
    would be two chances to omit the share; this criterion is the assertion
    that none did, and one function removes the opportunity rather than testing
    for it.

11. **A value sent on one VM thread is received on another**, which is the
    first test the bus has had of being a bus. Criteria 1 and 2 exercise
    `send`/`recv` within one VM — the reference's own shape, "a queue from a VM
    to itself" — so neither can fail if `PIPES` were per-VM instead of
    process-global. This one can.

    **Met, 2026-10-03** — `a_value_crosses_from_one_vm_to_another`. One VM
    sends `4242` and exits; a second VM, on a second thread, with its own word
    table and its own `Rc`s, receives it. What crossed was bytes, which is the
    whole of D86's reasoning made observable.

12. **`bus.data` is advisory across threads, and the race is tolerated rather
    than fixed.** §C8 states it: `ensure_bus` releases the lock before the
    caller acts, so under several VMs a `true` answer may be followed by
    `NODATA`. The criterion is that this is *asserted* — a test that observes
    the interleaving and accepts it — not that it is prevented. Preventing it
    would mean a `bus.data`/`recv` pair the reference does not have, which is
    new behaviour and a deviation, and the corpus's own
    `internal_bus_demo.bund` is written as a `bus.data`-guarded drain loop that
    is correct only because it has one VM.

    **Met, 2026-10-03** — `bus_data_is_advisory_once_there_is_more_than_one_vm`.
    One value, two VMs racing for it: both may see `true`, exactly one takes the
    value, and the loser gets `NODATA` **without erroring** — because
    `bus.data` created the channel, so `bus_pull`'s absent-channel arm cannot
    fire. The test asserts exactly one `NODATA`, which is the race tolerated
    rather than prevented.

13. **The `async fn` façade does not block the executor** — research (a), rated
    "do it". A VM runs on its own thread and the façade awaits it, so an
    executor driving two VM façades makes progress on both. What would fail:
    an implementation that ran `eval` inside the future.

    **Met, 2026-10-03, under D92: a hand-rolled `Future`, no executor
    linked.** The dependency question was taken as a decision rather than
    assumed — no executor exists anywhere in Bund2's or the reference's tree on
    a native target, so any choice would have been the project's first, and
    D38 had nothing to pin to. `Vm<T>` implements `Future`, so the same handle
    awaits under tokio, smol, async-std or a bare `block_on` without linking
    any of them.

    `an_executor_driving_two_vms_makes_progress_on_both` drives two VMs
    round-robin on one thread; each needs the other's value, so neither can
    finish until both have run. **What that does and does not prove**, stated
    because the criterion is easy to overclaim: the VMs run on their own
    threads whatever the executor does, so the test shows the façade does not
    *serialise* them, and it is the named wrong design — running `eval` inside
    `poll` — that it would catch, by deadlocking on the first poll.

    **The lost-wakeup window is closed by one mutex over both the answer and
    the waker**, and tested from both sides: a VM that finishes before the
    first poll, thirty-two times over, and a VM held past the first poll so the
    answer can only arrive by `wake`. The test executor is a condvar built from
    `std::task::Wake` — safe, since this crate forbids `unsafe` — and it
    **waits with a timeout**, because a no-op waker and a busy-poll loop would
    pass even if `wake` were never called.

    A panicking VM resolves as an error rather than hanging, through a `Drop`
    guard that marks it done, wakes its waiter and returns its host slot on the
    unwind — F57's reasoning about `Frame`'s exit action, applied to a thread
    boundary.

    **D92's scope is this criterion and nothing else.** Async native words
    still need a runtime, and that decision is untouched.

14. **The VM count is bounded and the bound is the owner's** (D85, tens of
    VMs). A host takes N, refuses what it cannot afford, and the per-VM cost is
    the one D85 measured — ~96 bytes a slot over ~396 names, about 75 KiB of
    word table per VM. What would fail: a host that spawns on demand with no
    ceiling, which turns D85's measured bound into a comment.

    **Met, 2026-10-03** — `the_vm_count_is_bounded_and_the_bound_is_the_owners`.
    `Host::new` holds D85's 64; `Host::with_limit` takes the owner's and refuses
    three things: zero, because a host with room for no VM is a configuration
    error rather than an empty one; a limit beyond 16× the default, with a
    refusal naming D85, because a four-digit limit set by accident should be
    refused where it is written rather than discovered as ~300 MB of word
    tables; and one spawn past the limit.

    **The cap is on VMs that exist at once, not on VMs ever spawned**, because
    that is what D85 measured — ~75 KiB of word table *per live VM*. A lifetime
    cap would retire a host after its 64th request, which no long-running
    embedder could use, and it would have been found in use rather than here.
    Each VM returns its slot through the same `Drop` guard that wakes its
    waiter, so a crash cannot permanently shrink the host.

    **The test holds its VMs live on a three-way barrier**, which is what makes
    the refusal deterministic: two trivial closures would very likely have
    finished and freed their slots before the third spawn was attempted, and
    the test would then pass or fail on scheduling.

**Conformance and coverage are unmoved by all six**, and that is a claim rather
than an omission: §C8 adds no word. **Measured 2026-10-03 after the host
landed: `conform` 110/119 ceiling 110/119, `coverage` 396/505, IMPLEMENTED
400/505, core 285/286** — every figure identical to the commit before. `cargo xtask conform` must read exactly what
it read before — the invariant the health metric exists for — and `coverage`
cannot move either, because the denominator is the reference's registry and the
reference has no concurrency vocabulary at all.

## Open questions

- **How a channel address selects a transport.** D87 fixes the intent —
  `send`/`recv` transparent across both buses, one vocabulary — and leaves the
  mechanism open. Zenoh is keyed by paths, a local channel is a bare name, and
  the reference has no answer because its `send`/`recv` never reach zenoh. A
  naming convention, a registry, or an explicit prefix are all available, and
  choosing is a language decision because a program writes the name.
- **How identity behaves across the bus — now D88, with the choice measured.**
  D86's amendment named the two options; D88 records what each costs and finds
  that the second half of one of them is false. **The bus does not make the id
  program-visible**: `==` refuses a LIST, the ordering fallback is F12's
  unreachable path, the VALUEMAP key reader is already an approved deviation
  under F29, and no word returns an id at all — the oracle answers `Inline .id
  not registered`. Both sides carry the *stamp*. So the question is not which
  behaviour a program sees, it is whether "a decoded value mints a fresh
  identity" is the rule for every decoder now that the bus, `sqlite` and the
  world file share one. D88 is OPEN with keeping F109 as its default.
- **Whether a depth refusal at `send` is wanted — answered by D114**,
  2026-10-08: yes. The oracle sends a 300-deep list without complaint, so
  refusing at `MAX_WIRE_DEPTH` is new behaviour, and it is recorded as an
  approved deviation. Criterion 4 carries the measurement.
- **Does a Bund program create tasks?** §C6. This RFC says no, following the
  reference by omission, which makes concurrency an embedding feature with a
  message vocabulary. The alternative — a `spawn` word — is a language decision.
- **What `Async` means**, if anything, now that D84 removed the mechanism it
  would have used. §C5 declines to invent it.
- **Zenoh's scope**: the globals semantics, `--distributed` for a Bund2 node,
  and whether the dependency is default. §C4, with D40 as precedent.
- **Whether the hermetic funnel's `bus` exclusion narrows too — now D89.**
  D87 narrowed the *deferral* to `globals.rs` and the narrowing reached
  `DEFERRED_PATHS`; the funnel's own table still maps the whole directory to
  `Effect::Bus`, which `Effect::hermetic` refuses. One corpus program is held
  out by it — `examples/code_snippets/internal_bus_demo.bund`, which is what
  criterion 2 wants. Narrowing grows the conformance denominator 116 → 117 and
  changes `HERMETIC.txt`, so it is the owner's.
- **Whether the audit's classification of `send`/`recv` should change.** It
  calls them effectful because "zenoh is reached through `helpers/zenoh`"
  (`docs/registers/open-questions.md:624-626`) — the wrong mechanism, since they
  use crossbeam. Probably still the right classification, because a
  process-global `PIPES` map is a side effect, but not for that reason. **This
  is the third place today that a reader attributed these words to zenoh from a
  directory name**, which is itself the finding: the deferral reason, the audit
  reason, and this RFC's first draft all did it.

**What the review found, recorded because the pattern is the point.** Four
blockers, and **two were errors in measurements this document claimed to have
taken against the oracle** — `recv`'s behaviour on an empty channel, and
identity across the bus. Both came from generalising a single probe: one
measured an absent channel and called it empty, the other read D20's
*materialises* as *replaces*. The remaining two were things not looked for at
all: a deferral the document should have cited, and the absence of any second
VM in a document about concurrency.
