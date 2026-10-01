# RFC-0007: Concurrency, and the two buses

- Status: **Draft** (2026-09-30). Not proposed: §C5 and §C6 each name something
  this document does not yet decide, and the open questions say which.
- Depends on: RFC-0002 (the word kinds this defines), RFC-0003 (the flat frame
  loop)
- Decisions consumed: D10, D20, D40, D44, D84, D85, D86
- Touched but not consumed: D16, D27, D31
- Reference SHA: `reference/Bund` at `21b40b0`, per `reference/PINNED.txt`
- Supersedes: nothing. `docs/research/01-extensibility-async.md` §2 is the
  reasoning trail and is **followed**; where this RFC departs,
  `docs/research/ERRATA.md` records it. It departs nowhere today.

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
| `recv` / `recv.` | Name; pushes the decoded value. **Errors on an empty channel rather than blocking** |
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

**`recv` does not block**, so a golden cannot hang on it: an empty channel is
an immediate error (`bus/crossbus.rs:144-147`). Combined with the capture
normalising id and stamp, **the local bus is goldenable** — which no
zenoh-backed word is.

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

**A sent value is not the value that arrives.** `to_binary` materialises the
identity and the stamp (D20), so `recv` yields an *equal* value with its own
identity — the property that sank the encoded-stream bundle in D77. A program
comparing `.id` across the bus sees two values.

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

## Preservation analysis

| behaviour | disposition |
|---|---|
| `send`, `send.`, `send.quick`, `send.quick.` | **Preserved**, including the bool/no-bool split and the operand order. |
| `recv`, `recv.` | **Preserved**, including the immediate error on an empty channel. |
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

1. **The eight bus words match the oracle**, by probe: operand order, the
   bool from `send` and its absence from `send.quick`, `recv`'s immediate error
   on an empty channel, and `bus.data` returning false *after* creating a
   channel that did not exist.
2. **A `send`/`recv` round trip is goldenable**, which is the claim §C2 rests
   on: `"chan" 42 send "chan" recv` captured from the oracle and matched, with
   the id and stamp normalised as every golden's are.
3. **A received value is equal and not identical.** Its `.id` differs from the
   sent value's, asserted rather than assumed, because D20 makes it so and a
   reader will expect otherwise.
4. **A program too deep to serialise is refused at `send`**, not silently
   truncated: `MAX_WIRE_DEPTH`'s 256 levels, with the depth and the cap named.
5. **Coverage moves by eight**, or the difference is named word by word.
6. **Conformance moves by exactly zero.** As RFC-0005's criterion 2 and
   RFC-0006's criterion 13.
7. **Nothing reads `WordKind` yet, and that stays true until something needs
   to.** A check, not a measurement: if §C5's `Blocking` acquires a reader, the
   RFC that gives it one states what changes.

## Open questions

- **Does a Bund program create tasks?** §C6. This RFC says no, following the
  reference by omission, which makes concurrency an embedding feature with a
  message vocabulary. The alternative — a `spawn` word — is a language
  decision and not this document's to take silently.
- **What `Async` means**, if anything, now that D84 has removed the mechanism
  it would have used. §C5 declines to invent it.
- **Zenoh's scope**: the globals semantics, `--distributed` for a Bund2 node,
  and whether the dependency is default. §C4, and D40 is the precedent that
  makes it a decision rather than a detail.
- **Whether the audit's classification of `send`/`recv` should change.** The
  path audit calls them effectful because "zenoh is reached through
  `helpers/zenoh`" (`docs/registers/open-questions.md:624-626`) — the wrong
  mechanism, since they use crossbeam. The classification is probably still
  right, because a process-global `PIPES` map is a side effect, but the
  recorded reason is not.
