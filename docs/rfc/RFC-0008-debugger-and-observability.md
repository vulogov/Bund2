# RFC-0008: The debugger and observability

- Status: **Draft** (2026-09-30). Not proposed: §D2 and §D6 each name a change
  this RFC does not yet know the cost of, and the open questions list what is
  genuinely undecided rather than leaving it in the prose.
- Depends on: RFC-0003 (the flat frame loop, spans), RFC-0005 (the tier, whose
  decline machinery §D6 uses)
- Decisions consumed: D36, D45, D52
- Touched but not consumed: D16, D74
- Reference SHA: `reference/Bund` at `21b40b0`, per `reference/PINNED.txt`
- Supersedes: nothing. `docs/research/03-metaprogramming-oop-debugger.md` §3 is
  the reasoning trail and is **followed**, including its own fallback in (f);
  where this RFC departs, `docs/research/ERRATA.md` records it. It departs
  nowhere today.

## Summary

Bund2 gains a debugger that debugs **the running program** rather than a
re-parsed string, and the observability words the reference has.

Three prerequisites are already built, which is why this RFC is small relative
to its scope. `Interp::eval_observed` is the per-word observer F52's
disposition called for — "the single evaluator takes a per-word observer" — and
its own doc says "the debugger's copy is this with a printing, stepping
observer". The flat frame stack that the research calls "the expensive
prerequisite" exists. And the `Diagnostic`/`Reporter` split was built for this:
CLAUDE.md states that seam exists because "a TUI will implement" it.

What remains is a **driven** step rather than a callback, a frame that knows
which word it is running, ten unimplemented words, and the Bund-specific
feature the research identifies: watchpoints on named stacks.

## Motivation

**The reference's debugger cannot debug a program.** It pulls a *string*,
re-parses it, and steps that (`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:52-95`).
The research states six limits (§3.2); four survive into Bund2 and two do not:

| limit | in Bund2 |
|---|---|
| It debugs a string, not the running program | **survives** |
| Granularity is one top-level term; no step-into | **survives** |
| No call stack to show — state lives in Rust frames | **gone.** RFC-0003's flat loop materialised `frames` |
| No breakpoints, watchpoints or conditional stops | **survives** |
| JIT frames would be opaque | survives, and §D6 answers it |
| History written to the working directory | **survives** — F10, disposition FIX |

**And the debugger is why the evaluator existed three times.** F52 records that
the third copy "prints each word before applying it and then runs a readline
loop after every word", and draws the conclusion this RFC rests on: "The
debugger exists as a duplicated interpreter *because* the interpreter has no
steppable state." Bund2 has one loop with an observer, so the duplication is
already gone — what is missing is the debugger that was meant to use it.

## Current behaviour

The preservation contract. Every word below is cited at its registration, and
**Bund2 has three of the thirteen**.

| word | behaviour | in Bund2 |
|---|---|---|
| `debug` | Pull a STRING, `bund_parse` it, and per top-level term: print a table, `vm.apply` it, then a `rustyline` loop where an empty line advances and any other line is evaluated **in the same VM** (`debug_debug.rs:52-95`) | no |
| `debug.shell` | An unconditional REPL with history (`debug_shell.rs:67`) | no |
| `debug.display_stack` | Table dump of the current stack (`debug_display_stack.rs:44`) | **yes** |
| `debug.display_workbench` | Table dump of the workbench (`debug_display_workbench.rs:38`) | **yes** |
| `debug.dump` | Hexdump of the top value's raw bytes (`debug_dump.rs:74`) | no |
| `debug.display_hostinfo` | Environment table, colour or not (`debug_display_hostinfo.rs:157,159`) | **yes** |
| `debug.display_memstat` | Memory table, colour or not (`debug_display_memstats.rs:70`) | no |
| `debug.display_distributed_info` | The bus/zenoh layer's state (`debug_display_distributed_info.rs:132,134`) | no |
| `log.info` `log.warning` `log.error` `log.debug` `log.trace` | Pull a STRING and emit it through the `log` crate at that level (`debug_trace.rs:17-33,61-65`) | no |

**Three facts about these that decide what can be checked.**

`log.*` pull one value, `cast_string` it, and emit through the `log` crate —
failing with `{PREFIX}: NO DATA #1` on an empty stack and
`{PREFIX}: Error casting tracing message: {err}` otherwise
(`debug_trace.rs:17-33`). **Where that output goes is the logger's**, not the
word's, so what a golden captures depends on how the binary configures logging.

`debug.dump` hexdumps `bytemuck::bytes_of` of the *scalar* for INT, FLOAT and
BOOL — so eight little-endian bytes, reproducible — and `to_binary()` for
everything else (`debug_dump.rs:20-55`). **That second path is not
reproducible**: the encoding carries the id and the stamp, which F14 normalises
in a golden's error text and cannot normalise inside a hexdump.

`debug` and `debug.shell` **read the terminal**. They belong with `password`
and `bund.prompt` in `ACTS_ON_HOST`, and no hermetic golden can run them.

## Design

### §D1 — Debugging is an execution mode of Tier 0, not a second parser

`Interp::step()` executes one value of the current frame and returns. The
debugger drives it; `step`, `next` and `finish` are three comparisons of frame
depth before and after, which is gdb's model and falls out of `frames` being
plain data (research §3.3(a)).

**`eval_observed` is not this, and the difference is the whole of §D1.** It
takes `&mut dyn FnMut(&BundValue)` and runs to completion, so the observer can
*watch* but cannot *stop*: control never returns to the caller between values.
A debugger needs the inverse — the caller resumes. Both survive: the observer
is what the trace stream (§D7) and the profiler use, because neither wants to
stop.

**Step-into works for words, lambdas and `bund.eval`'d code alike**, because
all three are bodies in frames. That is the property the reference cannot have,
and it is not new work here — it is what RFC-0003's loop already did.

### §D2 — A backtrace needs a frame to know what it is running

`Frame` holds `body: BundValue`, `ip: usize` and an exit action
(`crates/bund2-interp/src/lib.rs`, `Frame`). That is enough to *resume* a body
and not enough to *name* it: `bt` would print "a lambda at 3".

So a frame gains **the symbol it was pushed for** and **a span**, and this is
the one place this RFC adds a field to a hot structure. Two things follow, and
both are why §D2 is a reason this document is a Draft:

- **Cost.** A frame is pushed per call; RFC-0005's criterion 7 protects the
  `dispatch` group within 5%. Adding two words to `Frame` must be measured
  against it, not assumed free.
- **Spans must reach a frame.** `lower_with_spans` produces `Lowered` with a
  span per value, and a body is an `Rc<[BundValue]>` with no span table
  attached. Threading one through is the substance of §D2.

**A body assembled at run time has no span at all** — D16's consequence, and
`bund.eval`'d text has spans only into a string that no file holds. `bt` says
so rather than inventing a location.

### §D3 — Breakpoints, in three forms

Research §3.3(c), unchanged:

- **By word symbol** — break when a frame for that symbol is pushed. The frame
  push is one place, so this is one comparison.
- **By source location** — file and line, resolved through §D2's span table to
  a value index. Unavailable for a body with no spans, per §D2.
- **Conditional** — the condition is **a Bund lambda**, evaluated at the
  breakpoint. This is natural here and awkward elsewhere: lambdas are already
  first-class and `bund.eval` already exists.

**A condition that fails is not a stop.** A lambda that errors, exits, or
leaves no value cannot be allowed to end the program being debugged — the
debugger reports the condition's failure and treats it as "do not stop", which
is the only choice that keeps a broken breakpoint from destroying a session.

### §D4 — Watchpoints on named stacks

The Bund-specific feature, and the one the research argues is where a Bund2
debugger is *better* than a port of gdb's model rather than a weaker one
(§3.3(d)): `watch @errors` stops when anything is pushed to that stack,
`watch depth > 100` on the current one, `watch workbench`.

**Every push already writes the stack tag** (D41), so the push path is the one
place this hooks, as `@name`-keyed observation. A watchpoint is therefore the
same shape as a breakpoint and not a second mechanism.

### §D5 — Inspection words

**Keep the three Bund2 has.** The research says `display_stack`,
`display_workbench` and `dump` are "worth keeping essentially unchanged"; two
are built and `dump` is §D8's work.

**Implement the rest as the reference has them**, with the hermeticity each
allows: `debug.dump`'s scalar path is checkable and its `to_binary` path is
not; `display_memstat` and `display_distributed_info` report the environment
and cannot be pinned by a golden at all; `debug` and `debug.shell` read the
terminal.

**Add what the multi-stack model makes natural** (§3.3(e)): `stacks` for every
stack at once, `words` for the slot table with tier, generation and call count,
`classes` for registered classes. `words` already exists as a **subcommand**
for `xtask coverage`'s join key; the debugger's is a different thing — a view,
not a list — and the RFC keeps them apart rather than overloading one name.

### §D6 — The tier: disabled globally under the debugger

Research §3.3(f) offers two mechanisms and this RFC takes **the second**: "`bund2
run --debug` simply disables tiering globally."

**This needs nothing new.** `Runtime::with_options_and_threshold` installs the
tier in one place, so a debug session declines to install it, and every body is
Tier 0 — steppable, with no opaque frames. The first mechanism — pinning a
breakpointed word to Tier 0 by "reusing the async coloring machinery" — is a
refinement, and that machinery **does not exist**: async is RFC-7, blocked on
D6 and D7. Taking the global disable is therefore not a departure from the
research; it is the research's own fallback, and it removes this RFC's only
dependency on an unopened one.

**What it costs, stated:** a debugged program runs at Tier 0 throughout, so a
session cannot observe the tier's behaviour. That is the right trade — a
debugger that changes what it observes is worse than a slow one — and F139
measured that no corpus program compiles a body at the default threshold
anyway.

**What would reopen it:** a program slow enough at Tier 0 to be undebuggable.
Then per-word pinning, with its own decision, using the decline machinery
RFC-0005 already has — §S8's floor and criterion 18's `autoadd` entry check are
both existing examples of a compiled body refusing at entry.

### §D7 — Tracing is not logging

**`log.*` stay exactly as they are** (§3.3(h)). They are user-level logging,
the program's own output, and nothing here touches them.

**The execution trace is a separate stream** behind a flag, emitting
`(depth, symbol, value, stack effect)` per step. It is `eval_observed`'s
consumer — the callback shape is right for it, precisely because a trace never
wants to stop.

**The profiler comes from the counters that already exist**, not from new
instrumentation. The research notes the trace "replaces the
`time_graph::instrument` attributes currently scattered through the hot path —
the tier-up call counters give you a profiler for free". Bund2 has those
counters: the per-word entry count §S7's threshold compares, and
`compiled_bodies`, `compiled_entries` and `crossed_calls` on the tier.
**Nothing in Bund2 carries a `time_graph`-style attribute**, so there is
nothing to remove.

### §D8 — History goes to a config directory

F10, disposition FIX: the reference writes `bund_debug_debugger_history.txt`
into the working directory (`debug_debug.rs:48`). Bund2 writes to the
platform's config directory. **A deviation, and a deliberate one** — the file
is the debugger's state, not the program's output, and a program's directory is
not the debugger's to write in.

### §D9 — Not promised

DAP and time travel are consequences of the flat frame stack rather than
features planned here (§3.3(j)). The one obligation this RFC takes from that:
**keep the frame state serialisable**, so neither is foreclosed.

## Preservation analysis

| behaviour | disposition |
|---|---|
| `debug.display_stack`, `debug.display_workbench` | **Preserved**, and already built. |
| `debug.display_hostinfo` | **Preserved**, built. Environment-dependent, so no golden pins its content. |
| `log.*`, five words | **Preserved exactly** — one STRING, five levels, the two error texts. Where the output goes is the logger's. |
| `debug.dump`, scalar path | **Preserved** — eight little-endian bytes via `bytes_of`. |
| `debug.dump`, `to_binary` path | **Preserved in behaviour, uncheckable by a golden**: the encoding carries the id and stamp, which cannot be normalised inside a hexdump. |
| `debug.display_memstat`, `debug.display_distributed_info` | **Preserved in shape.** The second reports the bus/zenoh layer, which Bund2 does not have — so it reports its absence rather than inventing state. |
| `debug`, `debug.shell` | **Preserved, and extended.** `"…" debug` keeps working as a thin wrapper: evaluate the string with the debugger attached, breakpoint on entry (§3.3(g)). It then steps *into* things, which the reference cannot. Both act on the host. |
| `debug`'s history file location | **Deliberately changed** — §D8, F10. |
| The program being debugged | **Unchanged, which is the point.** §D6 disables the tier, so a session observes Tier 0; it does not change what Tier 0 does. |
| `Frame`'s size | **Changed**, and the cost is unmeasured — §D2. |

## Alternatives considered

- **A second, stepping evaluator**, as the reference has. Rejected: F52 records
  that this is *why* the reference has three loops, and its disposition is the
  observer this RFC uses.
- **Per-word Tier-0 pinning now**, research §3.3(f)'s first mechanism.
  Deferred, not rejected: the machinery it names does not exist, and §D6's
  trigger says when to revisit.
- **Making `eval_observed` the debugger.** Rejected on §D1: an observer that
  cannot stop is a trace, not a debugger. Both are kept, for their own
  consumers.
- **Extending `bund2 words` into the debugger's view.** Rejected: one is a join
  key for `xtask coverage` whose output a tool parses, the other is for a
  person. Overloading it would make a tool's input depend on a view's layout.

## Acceptance criteria

Each names what checks it. **Criteria 1–4 are checkable by a golden; 5–9 are
not, and say what stands in for one** — an interactive word cannot have a
golden, and pretending otherwise is how a criterion goes vacuous.

1. **`log.*` preserve their two error texts.** A probe running each of the five
   on an empty stack, and on a value that will not cast, against the oracle.
2. **`debug.dump`'s scalar path is byte-identical to the oracle's** for an
   INT, a FLOAT and a BOOL.
3. **Conformance moves by exactly zero.** As RFC-0005's criterion 2 and
   RFC-0006's criterion 13: this RFC adds words and a mode, not meaning.
   107/116, ceiling 107/116.
4. **Coverage moves by exactly ten**, or the difference is explained. The ten
   unimplemented words of §3.1 are in the in-scope set; a word implemented
   without a probe moves IMPLEMENTED and not COVERAGE, and the gap between them
   is the claim.
5. **`step`, `next` and `finish` agree with an uninterrupted run.** A program
   stepped to completion leaves the same final state as the same program run
   normally, asserted over every golden's program in the suite — a differential
   rather than a golden, and the strongest statement available.
6. **A breakpoint on a word stops before its body runs**, checked by frame
   depth and by the stack being what it was at the call.
7. **A failing conditional breakpoint does not stop and does not end the
   session** — §D3. The condition errors, and the program continues.
8. **A watchpoint on `@name` stops on a push to that stack and on no other.**
   The "no other" half is what makes it a watchpoint rather than a stop after
   every push.
9. **A debugged session runs no compiled code.** `compiled_entries()` reads
   `Some(0)` or `None` throughout — §D6, and the figure RFC-0005 added for
   exactly this kind of claim.
10. **`Frame`'s growth is measured, not assumed.** `cargo bench -p bund2-bench
    -- dispatch` within RFC-0005 criterion 7's 5% band, before §D2 is built on.

## Open questions

- **Where `log.*` output goes.** The words emit through the `log` crate, so the
  destination is the binary's logging configuration and not the word's. Until
  that is decided, criterion 1 can check the *error* texts and not the emitted
  line. Not a language question, which is why it is here and not a decision.
- **What `debug.display_distributed_info` reports** when there is no
  distributed layer. Reporting absence is this RFC's proposal; reproducing the
  reference's table with empty fields is the alternative, and the research flags
  the bus/zenoh layer as something "no document has accounted for".
- **§D2's cost.** Two words on `Frame`, against criterion 7's band. Criterion 10
  is the gate, and this RFC is a Draft until it is answered.
- **Whether the debugger is a word, a flag, or both.** The reference has the
  word `debug`; the research assumes `bund2 run --debug`. §D6 needs the flag;
  §3.3(g) keeps the word. Both is the proposal, and it is the one scoping
  question with no cost attached either way.
