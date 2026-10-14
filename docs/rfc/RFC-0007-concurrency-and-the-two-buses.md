# RFC-0007: Concurrency, and the two buses

- Status: **Proposed** (2026-10-04, on the owner's authorisation — **D93**),
  after one adversarial review of the document and the implementation of
  everything in it. **All fourteen criteria are discharged**: 1, 2, 3, 5, 6 and
  8 met against the oracle; 9, 10, 11, 12, 13 and 14 met as design; 4 met as
  an approved deviation (**D114**, 2026-10-08 — until then it read "deferred"
  here and "withdrawn" in the open questions, while the binary refused);
  7 withdrawn for naming no check.

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

  *(2026-10-14, RFC-0007's review of 2026-10-14 (`docs/rfc/reviews/RFC-0007-review-2026-10-14.md`):
  **the code holds and the document did not describe it.** Every criterion's
  test passed in both builds; the findings were about the text, and three
  tests that held less than their criterion's sentence. Those three now hold
  what the sentence says (criteria 3, 10 and 12). Two questions the review
  raised are decisions, **D164 and D165, open**. Answered in the amendment at
  the end and in notes at each passage.)*

  *(Later 2026-10-14, the fourth and fifth reviews
  (`docs/rfc/reviews/RFC-0007-review-2026-10-14-2.md`, `-3.md`): **"open"
  above, in the Depends and Supersedes notes and in the third review's
  amendment is of the morning. D164 was ruled C and D165 B the same day.**
  "After one adversarial review" is of 2026-10-04; there have been five.
  The fifth found a third arm of `recv` the document never described: a
  decode that fails, reachable by a JSON value inside a container, which
  lost the message on both implementations. **D166, ruled B**: Bund2
  refuses the value at the write. Answered in the second amendment at the
  end.)*
- Depends on: RFC-0002 (the word kinds this defines), RFC-0003 (the flat frame
  loop)
- Decisions consumed: D10, D20, D28, D40, D44, D84, D85, D86, D87, D88, D91,
  D92, D93
  *(2026-10-14: the body also uses D38, D60, D74, D89 and D114, which this
  list omitted; D164 and D165 are open.)*
  *(Later the same day: both RESOLVED. The body also uses D11 (§C3), D37
  (three Preservation rows), D90, D113 (the debugger row) and D166.)*
- Touched but not consumed: D16, D27, D31
- Reference SHA: `reference/Bund` at `21b40b0`, per `reference/PINNED.txt`
- Supersedes: `docs/research/01-extensibility-async.md` §2 in two places, now
  recorded in `docs/research/ERRATA.md`. **The first draft claimed it "departs
  nowhere", and the review found five places where it does** — §C3's ENVELOPE
  and the shared compile service among them.
  *(2026-10-14, the review's B2: ERRATA held two of the five. It now has the
  rest: §2.1(c), §2.4 and §2.7's await inside a word, superseded by D84;
  §2.4's `NativeKind` and §2.7's word kinds, departed from by §C5 and open
  as D164; the roadmap's "tier pinning", moot; and D91's ruling on the
  shared compile service.)*
  *(Later the same day: D164 is ruled C.)*

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
to an owned, `Send` representation" (`:285-291`).
*(2026-10-14, the review's B5, and the first review's M1 before it: the
lines are wrong and the quotation spliced. "Recommend `Rc` with the actor
model. This is not a compromise…" is `:283-284`; "each VM is `!Send` and
owns its values…" is the bullet before it, `:279-281`. "Said so in
2026-08": the research is dated at `:5`.)* D85 and D86 arrived at the
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

*(2026-10-14, the review's B3 and the first review's S1: **this table gives
the stack forms' effects to the workbench forms, and they differ.** The
reference's workbench forms take the object from the workbench and the name
from the stack (`reference/Bund/src/stdlib/functions/bus/crossbus.rs:57-60`,
`:96-99`), and `recv.` takes its name from the workbench (`:132-135`);
`bus.data.current` takes nothing (`:16-22`). Bund2 registers them so
(`crates/bund2-stdlib/src/bus.rs`, `register`):*

| word | from the stack | from the workbench | pushes | Bund2's effect |
|---|---|---|---|---|
| `send` | name, object | — | bool | `eff(2, 1)` |
| `send.` | name | object | bool | `eff(1, 1)` |
| `send.quick` | name, object | — | nothing | `eff(2, 0)` |
| `send.quick.` | name | object | nothing | `eff(1, 0)` |
| `recv` | name | — | the value | `eff(1, 1)` |
| `recv.` | — | name | the value | `eff(0, 1)` |
| `bus.data` | name | — | bool | `eff(1, 1)` |
| `bus.data.current` | — | — | bool | `eff(0, 1)` |

*The probe and `the_workbench_forms_take_different_operands_from_the_workbench`
already held this; the table did not. And "all unimplemented" is of
2026-09-30: all eight were built and probed on 2026-10-01, criterion 5.)*

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

*(Later 2026-10-14, the fifth review's B1: **three arms, not two.** After the
dequeue the bytes are decoded, and a decode that fails bails with `Error
converting from binary: {err}` after logging the same text at `ERROR`
(`bus/mod.rs:132-143`); the message has already left the channel. A program
reaches it by sending a JSON value inside a container, which `send` accepts
and no decoder reads (F208). Bund2 now refuses that value at `send` (D166),
so in Bund2 the arm exists and no program reaches it.)*

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

*(2026-10-14, the review's S1: 258 is of 2026-10-01. There are 367
`WordKind::Sync` registrations in `bund2-stdlib` today; the rest holds.)*

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

*(2026-10-14, the review's S1: D85's figures, of 2026-09-30. D85 gives
396 × 96 bytes ≈ 37 KiB of slots and ~75 KiB with the interner; `bund2
words` lists 610 names today, so the per-VM figure is about half as large
again. The conclusion, tens, is unchanged by a factor that size.)*

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
exposes it. **D88 took the second**: keep F109, as the rule for every decoder.

*(2026-10-14, the review's S2, and the first review's B2: **a JSON value is
the exception, on both sides.** The reference serialises a top-level JSON
value as its text under a fresh id and stamp, and decodes it with
`Value::json`, which mints both again
(`reference/rust_dynamic/src/bincode.rs:8-37`, `:51-77`;
`reference/rust_dynamic/src/create_special.rs:205-215`). Bund2's codec does
the same (`crates/bund2-value/src/wire.rs`, `to_binary` and `from_binary`).
So for a JSON value neither the id nor the stamp crosses, in the oracle or
in Bund2: preserved, and the identity row below now says so.)*

*(Later 2026-10-14, the fifth review's B1: **the note was half of what
those lines say.** Only a top-level JSON is wrapped
(`reference/rust_dynamic/src/bincode.rs:9`, `:54`); the fresh id and stamp
are `json_wrap`'s, `create_special.rs:241-251`, one call below `:22`. A
JSON value inside a container is written as JSON and cannot be read back,
on either side: for it nothing crosses, which is not an exception to
identity but to delivery. D166 refuses it at the write.)*

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

*(2026-10-14, the review's B1, and the first review's S4 before it: **the
next paragraph is wrong, and so is the citation above.** The quotation is
`:284-288`. And the Bund words `wrap` and `unwrap` serialise nothing: `wrap`
stores a value in an OBJECT's `.data` member and `unwrap` reads it back
(`reference/Bund/src/stdlib/functions/oop/value_class.rs:95-123`,
registered at `:168-169`), which ERRATA has recorded since 2026-10-01. So
there is **one layer**: the bus's payload is bytes no program sees, and no
Bund program can put an ENVELOPE on the stack. The "(D11)" pointer carries
the same mistake (ERRATA, §2.3's entry). The alternative this section
declines below is moot rather than declined.)*
*(Later the same day, the fourth review's M2: `wrap` is
`value_class.rs:83-106` and `unwrap` `:108-125`; the range above starts
inside `wrap`.)*

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
*(Later 2026-10-14, the fifth review's M6: D40 is `string.grok`'s C
dependency, made optional because a default-on native dependency reaches the
bundle target. "A transport's dependencies" is this RFC's analogy, not
D40's words.)*

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

*(2026-10-14, RFC-0007's review of 2026-10-14, B2, and the first review's S6 and S7: **two things here
are wrong.** "These definitions are the first" — the research defines all
three, differently (`docs/research/01-extensibility-async.md:302-307`), and
§2.7 calls them "required for any async" (`:388`); ERRATA now records the
departure. And "`Sync` … every word today" is false under this section's
own definition: `input`, `input*` and `password` wait for a person and
`sleep.seconds` for its operand, all registered `Sync`. What the variants
mean, or whether this RFC defines them at all, is **D164, open**. Until it
is ruled, the three bullets above are a proposal that the registry does
not follow, and an embedder that registers an `Async` word gets a word
that runs as every other does, because nothing reads the kind.)*

*(Later the same day: **D164 ruled C. This RFC defines none of the three.**
The bullets above are withdrawn as definitions; what each variant means is
for the RFC that gives `kind` a reader, as criterion 7's withdrawal already
said. The doc on `WordKind` says so to an embedder.)*

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

*(2026-10-14, the review: "(or one executor thread per VM)" is `:290`.)*

**Bund2 already has the thread.** `bund2-cli`'s `main` spawns one sized
`TIER0_PART + TIER1_SHARE + STACK_RESERVE` and declares the region with
`set_stack_region_with_share` (D44, §S8). A VM host spawns N of those, and
**each must declare its own share** — the lesson criterion 18 and
`compiled_entries` exist for: a thread that declares none puts the Tier 1 floor
above its own top and every compiled body declines while every figure reports
success.

*(2026-10-14, the review: "criterion 18" names no criterion of this RFC,
and RFC-0005's criterion 18 is `autoadd`. The lesson is RFC-0005 §S8's,
the ninth review's S3, checked for the bundle by RFC-0006's criterion 6,
"The front end declares its Tier 1 share"
(`docs/rfc/RFC-0006-ahead-of-time-output.md:2203-2206`). Criterion 10
below has the same slip.)*

**`bund2-async` is where this goes.** The crate exists as a twelve-line stub
whose doc reads "Executor integration and async native words. Feature-gated.
See RFC-0007", which RFC-0000 assigns here. §C8 fills it: a host that spawns
VM threads, the `async fn` façade over one, and nothing else — the words stay
in `bund2-stdlib`.

*(2026-10-14, the review's S1: "a twelve-line stub" is of 2026-10-01. The
crate holds the host and the façade, about 1,070 lines today; the quoted
description is still its `Cargo.toml`'s. Its own module docs said until
this date that the façade "is not here"; corrected with this review.)*

*(Later the same day, the fourth review's M4: 1,181 lines, `host.rs` 1,160
and `lib.rs` 21. **And nothing shipped links the crate** (the fifth
review's S4). `bund2-cli` does not depend on it; the façade crate `bund2`
declares it behind an `async` feature, off by default, and its `lib.rs`
has no items at all — it re-exports neither the host nor anything else.
So an embedder reaches §C8 only by depending on `bund2-async` directly,
and every test of criteria 10 to 14 is in the crate. What the façade
should expose is RFC-0000's question, about every crate it names, and is
not answered here.)*

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

*(2026-10-14, the review's S3: **the mechanism above is wrong, and the
right one is the guarantee this section never stated.** No receiver ever
waits on the channel. The reference tests `is_empty()` and calls `recv()`
under the one `PIPES` lock (`reference/Bund/src/stdlib/functions/bus/mod.rs:122-132`),
and Bund2 does the same (`crates/bund2-stdlib/src/bus.rs`, `pull`). So the
map lock is held across the dequeue: `recv` never blocks, whatever the
number of consumers, and consumers are ordered by the lock, which promises
no order. That is the non-blocking invariant and the fairness statement in
one.)*

*(Later 2026-10-14, the fifth review's S5: **"the mechanism above" is the
second bullet's.** The first is right: `ensure_bus` releases the lock
between `bus.data` and `recv` (`bus/mod.rs:58-70`), and criterion 12's test
forces exactly that interleaving with a barrier.)*

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

*(2026-10-14, the review's B5: **D91 was ruled before this RFC was
proposed**, on 2026-10-03: option 1, keep per-`Interp` modules. And the
magnitude D91 said was owed was taken the same day: "The magnitude, taken
2026-10-03 on a quiet host", three CLEAN runs of
`one_compiled_bodys_code_memory`, about 30 KiB a compiled body, of which
the body itself is about 3%. The review read the harness's `#[ignore]` as
the figure being unrecorded; it is in D91. The N× this section argues from
F139 is also measured directly: `n_vms_each_compile_the_same_hot_word_which_is_d91s_cost`
pins 1, 2, 4 and 8 bodies for 1, 2, 4 and 8 VMs.)*
*(Later the same day, the fourth review's M5: in a build with a tier. Without
one the test `continue`s, because there is nothing to count; it says so.)*

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
| The payload's identity | **Deliberately changed — D88, recording F109.** The oracle carries both id and stamp across. Bund2 carries the stamp and mints a fresh identity, as every decoder does. Until 2026-10-08 this row attributed the deviation to D20, which is the reading §C2 calls the error: D20's *materialises* means set if unset. |
| Unbounded channels | **Preserved.** A bound would be a deviation with a decision. |
| `WordKind::Sync` | **Unchanged.** 258 declarations, now with a stated meaning. |
| `WordKind::Blocking` | **Defined here for the first time.** No writer existed. |
| `WordKind::Async` | **Deliberately left undefined** — §C5. |
| Zenoh's globals paths | **Out of scope**, §C4. |
| Parallelism within one program | **Excluded**, on the research's own verdict (d). |

*(2026-10-14, RFC-0007's review of 2026-10-14, B3, S2 and its list of rows the table lacks. The rows
above stand with these notes: "The payload's identity" is the stamp for
every type but JSON, for which neither crosses on either side (§C2's
note); "`WordKind::Sync` — 258 declarations, now with a stated meaning" is
367, and the meaning is D164's. Rows added on this date:)*

| behaviour | disposition |
|---|---|
| the workbench forms' operands | **Preserved**: `send.` and `send.quick.` take the object from the workbench, `recv.` its name; `bus.data.current` takes nothing. The table under "Current behaviour" has them. |
| the depth guards | **Preserved**: six texts at nine sites, the stack checked before the workbench in the workbench forms (`crossbus.rs:26,44,49,52,83,88,91,122,127`); each site is reached by `the_depth_guards_name_the_word_the_reference_names`. |
| `send.quick`'s prefix | **Preserved**: `SEND` and `SEND.`, not `SEND.QUICK` (`crossbus.rs:177`, `:182`). |
| a name that will not cast | **Preserved**: `BUS.DATA: Error name casting: …` whichever of seven words was called (`crossbus.rs:35,71,110,142`); `bus.data.current` casts nothing. A failed cast has already consumed the object and the name (`:57-72`). |
| an absent channel at `recv` | **Preserved**: `RECV returns error bus::internal::pipe no pipe: <name>` (`mod.rs:126`, `crossbus.rs:146`). |
| a failed encode at `send` | **Preserved** in its prefix, `Error enveloping data:` (`mod.rs:114`); the reason after it is Bund2's codec's, and the one reachable case is D114's. |
| `Can not determine current stack name` (`crossbus.rs:19`) | **Removed by construction**: Bund2's current stack always has a name, `main` for an empty stack-of-stacks, so there is no arm (D37's first way out). |
| a poisoned channel map | **Changed**: the reference unwraps the lock (`mod.rs:45,53,59,73,90,123`) and would abort; Bund2 reports `the bus's channel map is poisoned, so … cannot proceed` (`crates/bund2-stdlib/src/bus.rs`, `with_pipes`). D37 requires it; no program can cause it. |
| channels are never removed | **Preserved**: neither has a removal path, so a process that computes names grows the map without bound, a second unbounded quantity beside the channel's own. |
| channel and stack names | **Preserved**: one flat namespace, so `bus.data.current` on a stack named `in` is the pre-created channel `in`. |
| a LAMBDA sent on the bus | **Preserved with one VM**: it crosses as bytes and runs if the receiver runs it. Under several VMs the receiver runs with whatever host options its VM was built with. *(Later the same day: **D165, ruled B** — a host takes `HostOptions` with `Host::with_options` and builds every VM with them, so an embedder's `--noeval` reaches the VMs it spawns. Tested by `a_spawned_vm_runs_under_the_hosts_options`)* |
| process-global state other than the bus | **Shared between VMs, by design and not by decision.** `GENERATORS`, the named generators, is process-wide as the reference's is (`crates/bund2-stdlib/src/random.rs`), so two VMs that use one name share one generator: a second medium between VMs beside the bus. `MT` and `CHACHA` are shared too, but each is seeded once at random and no word seeds it, so no program can tell. `ARGS` is set once by the runner. |
| the symbol interner | **Per thread** (`bund2-value`, `thread_local!`): a symbol's index differs between VMs and a received value re-interns on decode. Not visible to a program; it is why the interner needs no lock. |
| `WordKind::Async` | **Registrable and meaningless**: nothing reads `kind`, so such a word runs as any other. D164, ruled C: so are `Sync` and `Blocking`. |
| a VM the host spawned, and the debugger | A host VM has no console, so a word that needs one is refused in words, as under any embedder with no console (RFC-0008, D113's refusal). Not run for this row. |
| `bund.exit` in a host VM | **Ends that VM, not the process**: `request_exit` is per `Interp`, and `Host` returns the closure's answer. Read from the code, not run. |
| a `Vm<T>` dropped before it finishes | **Detaches the thread**, which runs to its end and returns its host slot through the same `Drop` guard (`crates/bund2-async/src/host.rs`, `Vm`'s `Drop`). |

*(Later 2026-10-14, the fourth and fifth reviews. Three rows above are
corrected here. **"`WordKind::Sync` — now with a stated meaning" and
"`WordKind::Blocking` — defined here for the first time"** are withdrawn by
D164's C: no variant has a meaning, as the `Async` row says. **`bund.exit`
in a host VM does not end the VM; it switches its `Runtime` off.** Run by
the fifth review: `eval_str` of a program that calls `bund.exit` answers
`Ok`, the rest of the program does not run, and every later `eval_str` on
that `Runtime` also answers `Ok` having evaluated nothing, because the
frame loop refuses every step once an exit is requested
(`crates/bund2-interp/src/lib.rs`, `exit_gate`) and the top level returns
cleanly on that refusal. The code is on `Vm::exit_requested`, which the
closure can read through `rt.interp` and the host neither reads nor
surfaces. That is the CLI's own contract inside a thread; the "Read from
the code, not run" label was honest and the reading was wrong. **A dropped
`Vm<T>`'s slot comes back through `Finish<T>`'s `Drop`**, not `Vm`'s,
which has none. Rows added on this date:)*

| behaviour | disposition |
|---|---|
| a failed decode at `recv` | **Deliberately changed — D166, recording F208.** The oracle reaches it with a JSON value inside a container: `RECV returns error Error converting from binary: ` and bincode's reason, an `ERROR` log line on stderr, and the message lost. Bund2 refuses that value at `send`, so its arm exists and no program reaches it. If it were reached, its reason would be bincode 2's (`crates/bund2-value/src/wire.rs`, `from_binary`) and it would print nothing beside the diagnostic (D36). |
| a JSON value inside a container, at a write | **Deliberately changed — D166.** `send`, `send.quick` and their workbench forms, `encode.base64`, `save.model`, `save` and `debug.dump` refuse it under their own prefixes; the oracle writes it, and its decoders cannot read it. A JSON value on its own still crosses. `debug.dump` only displays its bytes and refuses all the same, because the ruling named it. |
| where a host VM's diagnostics go | **Nowhere, unless the embedder says.** `Runtime` keeps `Interp`'s `SilentReporter`; only the CLI installs a `TextReporter`. An Error reaches the closure as `eval_str`'s `Err` and is reported nowhere else; a Warning or Notice is discarded. An embedder installs its own on `rt.interp.reporter` inside the closure. Run by the fifth review for an Error; the Warning half is read from the code. |
| `bund.exit` in a host VM | The note above: the `Runtime` is switched off, `eval_str` answers `Ok`, the code is on `exit_requested()`. |
| the order a finished VM gives things back | **The word table first, then the slot.** Until this date the slot was returned and the waiter woken while the `Runtime` was still alive, so `live()` could read one less than the VMs that existed and a woken waiter could spawn one more. `Host::spawn`'s thread now drops the `Runtime` before `Finish`. |
| a `Host` dropped with live VMs | **Nothing stops them.** `Host` has no `Drop`; each thread runs its closure to the end and returns its slot to a counter the dropped host no longer reads. Its `Vm<T>` handles still join. |
| every VM's default stack | **Is `main`, in every VM**, and channel and stack names are one namespace, so `bus.data.current` on N VMs' default stacks is one channel, `main`. `bus.rs`'s tests switch stack before measuring it for that reason. |
| a received value's `stack` tag | **The receiving push's**: `bus-words.golden` reads `main` for every value after crossing. |
| `--nocolor` in a spawned VM | **The host's**, with `noio` and `noeval` (D165; `HostOptions` has all three). |
| how an embedder reaches the host | **By depending on `bund2-async`.** No binary links it, and the `bund2` façade's `async` feature links it and re-exports nothing (§C8's note). |
| two `Host`s in one process | **Share `PIPES`**, and so every channel, `main` and the pre-created `in` and `out` among them: the map is process-global, not the host's. |

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
  *(2026-10-14: moot, not declined. No Bund program can produce an
  ENVELOPE to hand to `send`; §C3's note.)*

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

   *(2026-10-14, the review's S4: "every one of the eight" is seven;
   `bus.data.current` casts nothing. "The six depth guards" are six texts
   at nine sites, and the test reached seven of the sites: `send.`'s and
   `send.quick.`'s stack guard, checked before their workbench guard, and
   `send.quick.`'s workbench guard were not among them. Added the same
   day; the test now reaches all nine.)*
   *(Later 2026-10-14, the fifth review: **`bus_pull` has three arms**, and
   the criterion names two; the third is §C2's decode failure, which D166
   makes unreachable in Bund2 and `a_json_value_inside_a_container_is_refused_at_send`
   holds. And the probe reads `"in"` only; `"out"` is held by
   `in_and_out_are_the_only_channels_a_program_starts_with`.)*
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
   *(2026-10-14: of 2026-10-01. `HERMETIC.txt` lists 96 programs today.)*
   *(Later the same day, the fourth review's B2: **that note was wrong.**
   96 is the file's line count; it lists 58 programs and has not changed
   since 2026-10-01, so the sentence it annotated was true.)*
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

   *(2026-10-14, the review's B4: **the test held less than this
   sentence.** It asserted that a stamp existed, which a decoder minting a
   fresh stamp at `recv` would also pass. It now reads the stamp before the
   send, waits, and asserts the received value carries that stamp.)*
4. **Deferred, and it was a deviation filed as preservation.**
   *(2026-10-14, the review's M2: the heading's first word is of
   2026-10-01. The criterion is met, as an approved deviation, D114.)*
 The first draft
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

   *(2026-10-14, the review's S4: the test checks Bund2 alone, and no
   oracle run was cited. Run on this date, release builds, a one-line
   program per word: the oracle (`bund --noio script --file …`) and Bund2
   both print `bund BUS functions disabled with --noio` for each of the
   eight. The reference registers the stub for all eight under `--noio`
   (`crossbus.rs:208-216`).)*

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

    *(2026-10-14, the review's B4: **in a build without `jit` the test
    checked nothing**: its no-tier arm asserted `None`, which holds whether
    or not the region was declared, and its comment promised a depth check
    that was not written. It is written now, in every build: the test finds
    a depth of native recursion that an undeclared thread of the same size
    refuses, and requires every VM to complete it. Verified load-bearing
    the same way: with `declare_region()` removed from `Host::spawn` the
    default build fails with `VM 0 refused a depth of 256, which an
    undeclared thread refuses`. `bund2-async` has no `jit` feature of its
    own; the tier half runs under `--features bund2-runtime/jit`. And
    "criterion 18" above is RFC-0005 §S8's lesson; see §C8's note.)*

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

    *(2026-10-14, the review's B4: **"both may see `true`" was a statement
    about the code that the test did not establish**: one `NODATA` is also
    the outcome of the two racers running one after the other. A barrier
    now holds both between `bus.data` and `recv`, so both ask before either
    takes, and the test asserts both answers were `true` and exactly one
    receive was `NODATA`. Twenty runs, twenty passes.)*

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

    *(2026-10-14, the review: `block_on_both` busy-polls the remaining
    future once one is done, so the lost-wakeup property is held by the
    single-future tests, not by the two-VM one; "the test executor" above
    means those.)*

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
400/505, core 285/286** — every figure identical to the commit before.
*(2026-10-14: those figures are of 2026-10-03. Today `conform` reads
134/146, CEILING 134/146, and `coverage` 489/505 with IMPLEMENTED 500/505;
§C8 still adds no word, so nothing here moved them.)* `cargo xtask conform` must read exactly what
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
  world file share one. **D88 is RESOLVED: keep F109, and it is the rule for
  every decoder.** Criterion 3 carries the test. (This bullet said OPEN until
  2026-10-08.)
- **Whether a depth refusal at `send` is wanted — answered by D114**,
  2026-10-08: yes. The oracle sends a 300-deep list without complaint, so
  refusing at `MAX_WIRE_DEPTH` is new behaviour, and it is recorded as an
  approved deviation. Criterion 4 carries the measurement.
- **Does a Bund program create tasks?** §C6. This RFC says no, following the
  reference by omission, which makes concurrency an embedding feature with a
  message vocabulary. The alternative — a `spawn` word — is a language decision.
- **What `Async` means**, if anything, now that D84 removed the mechanism it
  would have used. §C5 declines to invent it.
  *(2026-10-14: D164, ruled C — no kind has a meaning until a reader
  exists.)*
- **Zenoh's scope**: the globals semantics, `--distributed` for a Bund2 node,
  and whether the dependency is default. §C4, with D40 as precedent.
- **Whether the hermetic funnel's `bus` exclusion narrows too — now D89.**
  D87 narrowed the *deferral* to `globals.rs` and the narrowing reached
  `DEFERRED_PATHS`; the funnel's own table still maps the whole directory to
  `Effect::Bus`, which `Effect::hermetic` refuses. One corpus program is held
  out by it — `examples/code_snippets/internal_bus_demo.bund`, which is what
  criterion 2 wants. Narrowing grows the conformance denominator 116 → 117 and
  changes `HERMETIC.txt`, so it was the owner's. **D89 is RESOLVED: narrowed**,
  and criterion 2 reports the program it admitted.
- **Whether the audit's classification of `send`/`recv` should change.** It
  calls them effectful because "zenoh is reached through `helpers/zenoh`"
  (`docs/registers/open-questions.md:624-626`)
  *(2026-10-14: now `:626-628`.)* — the wrong mechanism, since they
  *(Later the same day: `:627-629`.)*
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

## Amendment, 2026-10-14 — the third review

Answers `docs/rfc/reviews/RFC-0007-review-2026-10-14.md`, whose verdict was
"the code holds; the document does not yet describe it": five blocking
findings, seven significant, twelve rows the Preservation table lacked and
nine assumptions. Several had been raised on 2026-10-01 or 2026-10-08 and
not answered, which the review calls the finding in itself.

**Blocking.**
- **B1, §C3's second layer.** There is one layer; `wrap` and `unwrap` are
  OOP words. A note at §C3 and at the alternative it declined.
- **B2, §C5's word kinds.** The definitions contradict the registry and the
  research. ERRATA now has the departures; what the kinds mean is
  **D164, open**, recommending that §C5 define none of them until a reader
  exists.
- **B3, the operand table.** The workbench forms' sources and effects are
  tabulated under "Current behaviour", and the Preservation table has rows
  for every guard and error text.
- **B4, three weak tests.** Criterion 3's test compares the sender's stamp;
  criterion 12's forces the interleaving with a barrier and asserts both
  answers; criterion 10's checks the region in a build without `jit`,
  verified load-bearing. `the_depth_guards_name_the_word_the_reference_names`
  gained the three guard sites it did not reach.
- **B5, citations and D91.** The three research lines are corrected where
  they stand. D91 was ruled on 2026-10-03, and its magnitude was taken the
  same day: about 30 KiB a compiled body. The review's reading that the
  figure is unrecorded was wrong; it is in D91.

**Significant.**
- **S1, present tense.** Each figure it lists has a dated note.
- **S2, JSON.** Neither id nor stamp crosses for a JSON value, on either
  side: preserved, noted at §C2.
- **S3, fairness.** The map lock is held across the dequeue, which is both
  why `recv` never blocks and why consumers have no order; noted at §C8.
- **S4, criteria 1 and 8.** Seven words share the prefix, not eight; nine
  guard sites, now all tested; the oracle's `--noio` run was taken today for
  all eight words, and matches.
- **S5, the documents this one feeds.** RFC-0000's feature policy has a
  note: `bus` is not gated. **RFC-0001's hand-off** — "RFC-0007 inherits the
  question rather than an answer. Making `BundValue` shareable…"
  (`docs/rfc/RFC-0001-value.md:825-826`) — is answered by this RFC: values
  are not made shareable; bytes cross (§C2, D86). **RFC-0002's forward
  constraint** — "per-stack dispatch, when RFC-0007 needs it, is the single
  order with the stack as a parameter" (`docs/rfc/RFC-0002-symbols-and-words.md:68-71`)
  — is not exercised: this RFC needs no per-stack dispatch. The roadmap's
  "tier pinning" is moot, with an ERRATA entry.
- **S6, code comments.** `bund2-async`'s crate and module docs said the
  façade "is not here", and `bus.rs` said identity was "still open". Both
  corrected. The depth check criterion 10's comment promised is written.
- **S7, D92.** The section about the `log.*` words inside it has a dated
  note pointing to D90.

**The Preservation table** has seventeen rows added, among them the
process-global state VMs share (`GENERATORS` is a second medium between
VMs), a spawned VM's host options (**D165, open**, recommending that the
host take `HostOptions` for every VM it builds), and what `bund.exit`
and a dropped `Vm<T>` do.

**Assumptions the review listed.** 1, the lock across the dequeue: stated
at §C8. 2, a host VM's options: D165. 3 and 4, channels never removed and
one namespace with stacks: rows. 5, `send.quick`'s arity and prefix: rows.
6, criterion 10 as a `jit` property: checked in both builds now. 7, the
host's ceiling of 1024 VMs is 8.4 GiB of reserved address space under
`jit`, and `Host::with_limit` does not know the target: stated here and not
changed; §C8 already says "tens" is a 64-bit statement. 8, `Vm<T>` is
`Send` only if `T` is: stated here; every use in the tree returns a `Send`
value. 9, a received value's `stack` tag is the receiving push's, as in
`bus-words.golden`, where every value reads `main` after crossing: stated
here.

*(Later the same day, the fourth review's B2: **assumption 7's figure is
wrong.** 1,024 × (8 + 8 + 0.25) MiB is 16.25 GiB under `jit`, and 8.25 GiB
without; §C8's arithmetic for sixty-four VMs is right, and this paragraph
disagreed with it by a factor of two. "D165, open" and "D164, open" in
this amendment are of the morning; the paragraph below records both
rulings.)*

**Minor.** M1, the open-questions line: noted. M2, criterion 4's heading:
noted. M3, the direct N× measurement: cited at §C8's D91 note. M4, `WordKind`
is cited by file and type, as Q30 asks for `crates/`; M5, the research's
date: cited. M6, the `!Send` claim's doctest: criterion 9 names it.

**Not done.** `docs/arity.md` still lists the eight words as "not probed:
effectful, out of scope, or terminates". It is generated by `cargo xtask
arity`, which was not re-run for this amendment. The Preservation rows on
the debugger in a host VM and on `bund.exit` are read from the code and
were not run. D164 and D165 are open, and the review's verdict was that
acceptance waits on the document; it waits on them too.

**Rulings, later the same day.** D164: C — §C5 defines none of the three
word kinds, and `WordKind`'s doc says they have no meaning until a reader
exists. D165: B — `Host::with_options`, built and tested. No decision this
RFC rests on is open.

## Amendment, later 2026-10-14 — the fourth and fifth reviews

Answers `docs/rfc/reviews/RFC-0007-review-2026-10-14-2.md` and `-3.md`. The
fourth was written against `25a33da` and not answered before the fifth,
which found its findings unchanged and added one blocker.

**Blocking.**
- **The fifth's B1, `recv`'s third arm.** A decode that fails, reached by a
  JSON value inside a container, which both implementations' `send`
  accepted and neither's `recv` could read; the message was lost. F208
  records the reference's half. **D166, ruled B**: Bund2's codec refuses
  the value at every write, in the walk F118 added for depth. Built and
  measured: `send`, `encode.base64` and `debug.dump` refuse; no golden
  moves (134/146, CEILING 134/146). Two tests, both checked to fail with
  the refusal disabled. Notes at the Current-behaviour paragraph on
  `bus_pull`, at §C2's JSON note and at criterion 1; rows in the
  Preservation table.
- **The fourth's B1 and the fifth's B2, "open".** Notes at the status
  line, the Depends and Supersedes notes, the open question on `Async`, the
  Preservation table's `WordKind` rows and the first amendment.
- **The fourth's B2 and the fifth's B3, two figures.** `HERMETIC.txt` lists
  58 programs; 1,024 VMs are 16.25 GiB under `jit`. Notes where each
  stands.

**Significant.**
- **`bund.exit` in a host VM** (fifth, S1): the row is corrected; the
  `Runtime` is switched off and answers `Ok`, and the code is on
  `exit_requested()`. Not changed: it is the CLI's contract.
- **A host VM's diagnostics** (fourth S2, fifth S2): a row. They go to
  `SilentReporter`; an embedder installs its own. Not changed.
- **The slot before the word table** (fourth S1, fifth S3): fixed.
  `Host::spawn`'s thread drops the `Runtime`, then `Finish`. No test
  observes the order; `live()`'s doc now describes what the code does.
  The `Drop` the row cited is `Finish<T>`'s.
- **Nothing links the host** (fifth, S4): stated at §C8. What the façade
  exposes is RFC-0000's question.
- **"The mechanism above"** (fifth, S5): the second bullet's; noted.

**Rows.** Eleven added, covering every row both reviews listed.

**Assumptions.** The fourth's 1, 2, 5 and the fifth's 1 to 6 and 8 are
rows now. The fourth's 3: `HostOptions` and the threshold are all an
embedder sets; the reporter is set inside the closure, and the stack size
and region are the host's. The fourth's 4: polling a `Vm<T>` again after
it resolved answers "a VM thread ended by panicking", which is wrong for
that case; an executor that does it breaks the `Future` contract, and the
text is not changed here. The fifth's 7: criterion 13's two VMs wait for
each other by 10,000 `recv` polls, an iteration bound rather than a time
bound; it has not failed, and is stated rather than changed. The fifth's
9: no test asserts `size_of::<Slot>()`; §C1's figure is D85's.

**Minor.** Each is a note where it stands: `value_class.rs:83-125`;
`json_wrap` at `create_special.rs:241-251`; 1,181 lines; the N× test
counts only with a tier; the Depends note's missing decisions; D40 as an
analogy; `:627-629`; the probe reads `"in"` only. The cast row's "a
failed cast has already consumed the object and the name" is of the four
`send` forms, which are what `:57-72` covers.

**Not done.** `docs/arity.md` still lists the eight words as not probed;
`cargo xtask arity` was not re-run. Whether `debug.dump` should be exempt
from D166 — nothing reads its bytes back — would be a further decision.
No decision this RFC rests on is open.

