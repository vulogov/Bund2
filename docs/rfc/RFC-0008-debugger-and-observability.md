# RFC-0008: The debugger and observability

- Status: **Proposed** (2026-10-04, on the owner's authorisation — **D94**),
  after one adversarial review of the document and the implementation of §D1
  through §D6. **All thirteen criteria are discharged**: eleven met, criterion 4
  met on each capture, and one withdrawn for naming no check.

  **What is specified and not built, named here because no criterion covers
  it.** §D7's execution trace — a stream behind a flag, `(depth, symbol, value,
  stack effect)` per step — has no flag; §D1's safepoint is the hook it would
  use. §D8's history is written nowhere: the console reads plain lines and keeps
  none. *(Since 2026-10-06 the `debug` and `debug.shell` words keep history —
  see §D8. The `--debugger` console still keeps none.)* So a debugged session steps, breaks, watches and inspects, and does not
  trace or remember. That is the whole gap.

  **Two things the criteria record as owed rather than met.** Criterion 6 says
  "every suite program" and four were used — the sweep belongs in `xtask
  conform`. And §D2's span half is withdrawn from criterion 12: a frame carries
  the symbol it was pushed for and no source position, because
  `lower_with_spans` is top level only and per-value spans are RFC-0003 §S5's
  IR, which does not exist.

  **The previous status said what remained was "measurement, not decision" and
  named criteria 11 and 13.** It was wrong about which: the measurements were 11
  and **12**, and criterion 13 was a statement about `debug` that needed no
  measurement at all. Both readings were corrected by taking them.

  **Four criteria were wrong as written, and building caught every one** —
  criterion 2 asked for a line carrying a wall-clock timestamp and the
  reference's own Rust module path, 3 asked for seven texts of which five are
  vacuous, 5 predicted ten words and five moved, and 10 asked for `Some(0)`
  where `None` is the truth. D94 tabulates them. The pattern is the lesson: the
  criteria were written from the design, and the design was right about
  mechanism and wrong about measurement four times over.

  The first adversarial review
  (`docs/rfc/reviews/RFC-0008-review-2026-09-30.md`) found §D1 blocked on
  **D6**, which the first draft treated as settled — the failure CLAUDE.md names
  in terms. **D84 closed D6** (VM-per-task; the debugger stops a thread rather
  than suspending a word), answering what RFC-0003 had placed in D6's scope.
- Depends on: RFC-0003 (the flat frame loop), RFC-0005 (the tier, whose decline
  machinery §D6 uses)
- Decisions consumed: D36, D44, D45, D52, D53, D84, D90, D94
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

**The reference also has four CLI flags, and the first draft's contract omitted
all four** (`reference/Bund/src/cmd/mod.rs:121-137`,
`reference/Bund/src/cmd/setloglevel.rs:6-33`):

| flag | behaviour |
|---|---|
| `--debugger` | Run the program inside the debugger. F52: "`--debugger` uses the third [loop]" |
| `--debug-shell` | Drop to the debug shell **if an error occurs** |
| `-d` / `--debug` | A **count**, not a boolean: 0 takes the environment, 1 `bund=info`, 2 `bund=debug`, 3+ `bund=trace` |
| `--profile` | Run the internal profiler |

**So this RFC may not spell its flag `--debug`.** The first draft proposed
`bund2 run --debug`, following the research; that name is taken by the log-level
count, and Bund2 has no `run` subcommand for it to hang off either. **The flag
is `--debugger`**, as the reference spells it, and §D6's global tier disable
attaches to that.

**Three facts about these that decide what can be checked.**

`log.*` pull one value, `cast_string` it, and emit through the `log` crate —
failing with `{PREFIX}: NO DATA #1` on an empty stack and
`{PREFIX}: Error casting tracing message: {err}` otherwise
(`debug_trace.rs:17-33`).

**Where the output goes is not an open question, and the first draft filed it as
one.** The reference initialises `env_logger` with
`Env::default().filter_or("BUND_LOG_LEVEL", "error")`, and `-d` repetition
raises it to `bund=info`, `bund=debug`, then `bund=trace`
(`reference/Bund/src/cmd/setloglevel.rs:6-33`). `env_logger` writes to
**stderr**, and a capture appends stderr to stdout — so **`log.error`'s line
lands in a golden and the other four do not**, at the default level. That is
checkable, and criterion 1 now checks it.

`debug.dump` **peeks** rather than pulls (`debug_dump.rs:14`) and hexdumps
`bytemuck::bytes_of` of the scalar: **eight** little-endian bytes for INT and
FLOAT, **one** for BOOL, and the bytes of the string for STRING — all
reproducible. `to_binary()` covers everything else (`debug_dump.rs:20-72`).
**That last path is not reproducible**: the encoding carries the id and the
stamp, which F14 normalises in a golden's error text and cannot normalise inside
a hexdump.

The first draft got three of these wrong — it described the word from the
research rather than the code, gave BOOL eight bytes, missed the STRING arm, and
said "pull". It also omitted the **seven** `DUMP: error CASTING to …` texts,
which are part of the contract. **And the output format belongs to the `hexdump`
crate**, which lives outside `reference/` and therefore cannot be cited under
the grounding rule — so the format is reproduced by matching the oracle's bytes,
not by citing a spec.

`debug` and `debug.shell` **read the terminal**. They belong with `password`
and `bund.prompt` in `ACTS_ON_HOST`, and no hermetic golden can run them.

## Design

### §D1 — The debuggee stops on its own thread; it is not suspended

`Interp::step()` executes one value of the current frame and returns. The
debugger drives it; `step`, `next` and `finish` are three comparisons of frame
depth before and after, which is gdb's model (research §3.3(a)).

**Step-into does not come from bodies being frames, and the first draft claimed
it did.** It said "step-into works for words, lambdas and `bund.eval`'d code
alike, because all three are bodies in frames". They are, and that is not
sufficient: `Interp::eval_lambda` records `floor = frames.len()`, pushes, then
calls **`run_to(floor)`** — a nested loop *inside the native's own Rust frame*
(`crates/bund2-interp/src/lib.rs`, `eval_lambda`), whose comment says "every
native that runs a body synchronously comes through here". There are **27 such
call sites across nine files**, and several hold Rust state across the body:
`map_base` keeps a `Vec<BundValue>` and a loop position.

**D84 answers it without suspension.** Stepping needs a program to *stop*, not
to suspend. The debuggee runs on its own thread — the shape `bund2-cli`'s `main`
already uses under D44 — and blocks at a safepoint each step. A thread blocked
inside `map`'s Rust frame has its state intact on its own stack, so **step-into
works**: the debuggee stops *inside* the nested loop rather than needing to
escape it.

**The debuggee inspects itself, because `Interp` is not `Send`.** It holds
`Box<dyn Reporter>` and, through every value, `Rc<HeapValue>` — two independent
reasons a debugger thread may not touch it. So at a safepoint the debuggee
receives a **command**, executes it against its own `Interp`, and returns
**rendered text**; only commands and text cross the boundary. That is the
`Reporter` seam's shape (D36, D45), and it is what the reference already does —
`debug`'s readline loop hands each line to `bund_compile_and_eval` **in the same
VM** (`debug_debug.rs:81-95`).

**What is unavailable**: unwinding or restarting a native mid-call, so "step out
of a native" and time travel stay off the table — §3.3(j) already declined the
second.

**What it costs**: a safepoint check per step, which criterion 13 measures
against RFC-0005 criterion 7's band rather than assuming.

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
- **Spans do not exist for anything a frame runs.** `lower_with_spans` is
  **top level only**, and says so at its definition: "A value inside a lambda
  body carries no span, so a failure three calls deep is reported at the
  top-level term that started it… positions for nested values need the IR
  RFC-0003 §S5 describes, **which does not exist yet**"
  (`crates/bund2-syntax/src/lib.rs`, `lower_with_spans`).

  So §D2 has spans to **produce**, not to thread through, and producing them is
  RFC-0003 §S5's IR — a dependency the first draft did not name. A backtrace
  can carry a word symbol and a value index today; a *source location* needs
  that IR.

**A body assembled at run time has no span at all** — D16's consequence, and
`bund.eval`'d text has spans only into a string that no file holds. `bt` says
so rather than inventing a location.

### §D3 — Breakpoints, in three forms

Research §3.3(c), unchanged:

- **By word symbol** — break when a frame for that symbol is pushed. The frame
  push is one place, so this is one comparison.
- **By source location** — file and line, resolved through §D2's span table to
  a value index. Unavailable for a body with no spans, per §D2.
- **Conditional** — the condition is **a Bund lambda, evaluated in a child VM
  context** at the breakpoint (research §3.3(c)). This is natural here and
  awkward elsewhere: lambdas are already first-class and `bund.eval` exists.

  **The first draft dropped "in a child VM context", which is the clause that
  makes it sound.** A condition evaluated in the program's own VM changes the
  program it is watching: it can push, switch stacks, rebind a word — and it can
  **exit irreversibly**. `Interp::request_exit` is
  `self.exit_code.get_or_insert(code)`, with the comment "The first request
  stands; a second, made while unwinding, does not change the code"
  (`crates/bund2-interp/src/lib.rs`, `request_exit`). So a breakpoint condition
  that calls `bund.exit` ends the debugged program and **nothing can clear it**.
  A child VM is not a refinement here; it is the requirement.

**A condition that fails is not a stop**, and with a child VM that is
achievable: a lambda that errors, exits, or leaves no value is reported and
treated as "do not stop". Without one it is not achievable at all, per the exit
cell above.

### §D4 — Watchpoints on named stacks

The Bund-specific feature, and the one the research argues is where a Bund2
debugger is *better* than a port of gdb's model rather than a weaker one
(§3.3(d)): `watch @errors` stops when anything is pushed to that stack,
`watch depth > 100` on the current one, `watch workbench`.

**The stack push writes the stack tag** (D41), so a `@name`-keyed watchpoint
hooks there. **But that is not "one place", as the first draft claimed**: the
workbench is a separate path and does **not** write a stack tag — it has no
stack to name — so `watch workbench` is a second hook, not the same one. Two
hooks, named as two.

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
`(depth, symbol, value, stack effect)` per step.

**`eval_observed` cannot drive it, and the first draft said it could.** The
observer is called from the top-level loop only, so it sees top-level values and
nothing a frame runs — the same limit as the spans in §D2, and for the same
reason. A per-step trace needs the hook where a *frame* advances, which is
`run_to`'s loop, not `eval_observed`'s. RFC-0003 also carries an accepted
Preservation row about the observer's reach, and widening it is a change to that
RFC rather than a detail here.

So `eval_observed` stays what it is — the printing observer F52's disposition
called for — and the trace is a second hook, one level down.

**The profiler does not come for free from the tier's counters, and the first
draft claimed it did.** The research says "the tier-up call counters give you a
profiler for free", and Bund2's counters are the wrong shape for it three times
over: `compiled_bodies`, `compiled_entries` and `crossed_calls` are **totals on
the tier**, not per-word times; they exist **only in `jit` builds**; and **§D6
removes the tier for a debug session**, so a profile taken under the debugger
would read zero by construction.

A profiler therefore needs its own counters on the frame loop, which is the same
hook the trace needs. That is more work than "for free", and it is recorded here
rather than discovered later. **`--profile` is in the contract** (§the flags
above) and unimplemented; nothing in Bund2 carries a `time_graph`-style
attribute, so there is nothing to remove.

### §D8 — History goes to a config directory

F10, disposition FIX: the reference writes `bund_debug_debugger_history.txt`
into the working directory (`debug_debug.rs:48`). Bund2 writes to the
platform's config directory. **A deviation, and a deliberate one** — the file
is the debugger's state, not the program's output, and a program's directory is
not the debugger's to write in.

**Built 2026-10-06 (D102), for the two words that read lines.** `debug` keeps
`bund_debug_debugger_history.txt` and `debug.shell` keeps
`bund_debug_shell_history.txt` — the reference's two file names — under
`<config>/bund2/`, where `<config>` is `$XDG_CONFIG_HOME`, else
`$HOME/.config`, else `%APPDATA%` (`crates/bund2-stdlib/src/terminal.rs`,
`config_home` and `history_path`). The file is written when the session ends
and only if a line was entered, so a session that meets end-of-input at once
leaves nothing behind. Measured beside the oracle: it writes into the working
directory, Bund2 does not.

**Not covered:** the `--debugger` console (`crates/bund2-cli/src/debugger.rs`)
still reads plain lines and keeps none. It has no line editor to keep them
for; that arrives with the input seam D101 defers.

### §D9 — Not promised

DAP and time travel are consequences of the flat frame stack rather than
features planned here (§3.3(j)). The one obligation this RFC takes from that:
**keep the frame state serialisable**, so neither is foreclosed.

## Preservation analysis

| behaviour | disposition |
|---|---|
| `debug.display_stack`, `debug.display_workbench` | **Preserved**, and already built. |
| `debug.display_hostinfo` | **Approved deviation — D53**, not preserved. It reports *Bund2's* crates where the reference reports its own six, and Bund2 has none of them. The first draft said "Preserved", contradicting a decision already taken. |
| `log.*`, five words | **Preserved exactly** — one STRING, five levels, the two error texts. Where the output goes is the logger's. |
| `debug.dump`, scalar path | **Preserved** — eight little-endian bytes via `bytes_of`. |
| `debug.dump`, `to_binary` path | **Preserved in behaviour, uncheckable by a golden**: the encoding carries the id and stamp, which cannot be normalised inside a hexdump. |
| `debug.display_memstat` | **Preserved in shape.** Reports the environment, so no golden pins its content. |
| `debug.display_distributed_info` | **Preserved, including its absence behaviour, which already exists.** Without `--distributed` the reference logs `BUND must be in distributed mode. You shall pass --distributed to CLI` and **prints nothing** (`debug_display_distributed_info.rs:23`). Bund2 has no distributed layer at all, so that is its only path. Neither option in the first draft's open question was this. |
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

Each names what checks it. **The split is not "golden versus not" — that was the
first draft's framing and it was too coarse.** A word can be runnable under a
capture and still unpinnable, which is `debug`'s case.

1. **`log.*` preserve their two error texts**, checked against the oracle: each
   of the five on an empty stack, and on a value that will not cast.

   **Met, 2026-10-03.** `the_two_error_texts_are_the_references_own`, with the
   reference's own prefixes — the word's name upper-cased, so `LOG.WARNING` for
   `log.warning`. Unit tests rather than a golden: an error renders through the
   diagnostic table, which F66 records as text no second machine reproduces.

   **A third behaviour was found while checking the second** and is preserved:
   the pull and the cast happen **before** the level is consulted, so
   `7 log.info` fails with a casting error at the default level *even though
   `log.info` emits nothing there*. Measured on the oracle, which reports the
   same text for the same program.
2. **`log.error`'s emitted line matches the oracle's** at the default level, and
   the other four emit nothing. `env_logger` writes to stderr, a capture appends
   stderr, and the default filter is `error`
   (`reference/Bund/src/cmd/setloglevel.rs:7`) — so this is checkable, which the
   first draft denied.

   **Half of this is confirmed and half is not achievable — D90, measured
   2026-10-01.** At the default level the oracle emits exactly one line for a
   program that calls all five words, so *which* words are silent is checkable
   and the first draft was indeed wrong to deny it. **The line is not.** It reads

       [2026-10-01T15:35:51Z ERROR bund::stdlib::functions::debug_fun::debug_trace] hello from error

   and two of its fields cannot be reproduced: the timestamp is wall-clock, which
   no normaliser handles today, and
   `bund::stdlib::functions::debug_fun::debug_trace` is **a Rust module path in
   the reference's source tree** — `env_logger` prints the calling module.
   Bund2's equivalent code is `bund2_stdlib::console`, so emitting the
   reference's path verbatim would be printing a false statement about where the
   code is.

   So the criterion is restated: **what is checkable is which of the five emit at
   the default level and which are silent, not what the line says.**

   **Met as restated, 2026-10-03, under D90.** Four probes' worth of silence is
   pinned by `tests/probes/log-words-quiet.bund`, and
   `only_log_error_speaks_at_the_default_level` asserts the same thing from
   inside. D90 chose `Vm::report`, so a line reads `Warning: log.error: …` —
   Bund2's own, with no timestamp and no Rust module path.

   **The filter is a requirement rather than a nicety, and the probe is why.**
   It asserts that four words say *nothing*; a Bund2 without `setloglevel`'s
   filter would emit four lines and fail it. Two narrowings are recorded in
   D90: Bund2's CLI has no `--debug` count, so only `BUND_LOG_LEVEL` raises the
   level, and a directive is read by taking the level after its last `=` —
   enough for the `bund=info` shape the reference's own `--debug` produces,
   without reimplementing `env_logger`'s filter grammar.
3. **`debug.dump` is byte-identical to the oracle** for an INT, a FLOAT, a BOOL
   and a STRING — eight bytes, eight, **one**, and the string's own bytes — and
   all seven `DUMP: error CASTING to …` texts match.

   **Met for the four tags, 2026-10-01**, and the four-tag restriction was
   right for a reason the criterion does not give: everything else falls to the
   reference's `_` arm, which dumps `to_binary`, and **that encodes the value's
   id and stamp — so the oracle's own two runs of `[ 1 2 ] debug.dump` differ
   from each other.** It is F14's class reached through a hexdump rather than a
   `Debug` line, and no normaliser can reconcile it because the bytes are the
   subject rather than the frame around them.

   `tests/probes/debug-dump.bund` covers eleven cases across the four tags —
   both BOOLs, a string of exactly sixteen bytes and one that spills to a second
   line, since the padding differs in all three, and the empty string, whose
   dump is the summary line alone — and diffs byte-identical. The word **peeks**,
   so its effect is `1 -> 1`, the shape F92 corrected for `println.`; the probe
   drops after every dump and the final stack dump is what proves the value was
   there to drop.

   **"All seven texts" is wrong: five of them are vacuous.** Each casting arm
   sits behind the tag check that guarantees its cast, in the reference as in
   Bund2, so no run of either can produce them. Reproducing their text would
   mean inventing the `{err}` half of a message nothing emits, so Bund2 names the
   broken invariant instead — D37's third way out, since a FLOAT whose payload is
   not a float is a Bund2 defect and not a program error. The two that a program
   can reach are `DUMP: NO DATA #1`, which matches, and `DUMP: error converting
   to binary`, which Bund2 can reach above `MAX_WIRE_DEPTH` where the oracle's
   encoder has no bound at all — RFC-0007 criterion 4's deferred question, now
   with a second word behind it.
4. **Conformance does not regress**, measured *after* the new probes are
   captured. **Not "stays at 107/116"**: criteria 1–3 add probes, so the
   denominator grows, and the first draft's criterion 3 contradicted its own
   criteria 1, 2 and 4 by demanding a fixed ratio. The claim is that no golden
   that passed before fails after, per golden, and that CEILING moves only by
   deviations this RFC records.

   **Met, 2026-10-01 and again 2026-10-03**: 109/118, then 110/119 after
   criterion 3's probe, then **111/120 after criterion 2's**, each with the
   ceiling equal, in the default build and under `--features jit`. The
   denominator grew by exactly the probes this RFC added, the numerator with
   them, and
   the nine approved deviations are the same nine — **this RFC has recorded
   none**, so the ceiling moved only with the denominator. COVERAGE 395 → 396
   and IMPLEMENTED 399 → 400, which is criterion 5's table read back from the
   binary rather than predicted.

   Also unmoved by §D1's safepoint and §D6's declined tier, each measured when
   it landed: a branch nothing takes and a tier that is not installed must not
   change what a program means, and 109/118 held across both.
5. **Coverage moves by ten, or the difference is named word by word.** The first
   draft said "moves by exactly ten, or the difference is explained", which
   cannot fail — any outcome is explicable. This version requires the ten words
   to be listed with their state: implemented and probed, implemented and
   unprobed, or not implemented.

   **The ten, named, as of 2026-10-01.**

   | word | state |
   |---|---|
   | `debug.dump` | implemented and probed — criterion 3 |
   | `log.info`, `log.warning`, `log.debug`, `log.trace` | implemented and probed — D90, criteria 1 and 2 |
   | `log.error` | implemented; **no golden can hold it** — it is the only one that emits at the default level, and its line carries a wall-clock timestamp, so two oracle runs differ and the capture refuses it. Named unreachable beside `debug`, `debug.shell` and `password` |
   | `debug` | not implemented — criterion 13 says a golden cannot pin it |
   | `debug.shell` | not implemented — same |
   | `debug.display_memstat` | not implemented, not yet grounded |
   | `debug.display_distributed_info` | not implemented; it reports a zenoh session, which D87 left deferred |

   **Measured 2026-10-03, after D90's capture: COVERAGE 395 → 400/505,
   IMPLEMENTED 399 → 405/505.** Implemented moved by **six** — `debug.dump` and
   all five `log.*` — and coverage by **five**, because `log.error` is
   implemented and no golden can run it. It now sits in the report's own
   `implemented but run by no golden` list beside `$`, `<-`, `password` and
   `←`, which is `coverage` saying so rather than this document claiming it.
   The other five of the ten are accounted for word by word:
   `debug`/`debug.shell` are goldenable by no capture,
   `debug.display_memstat` is ungrounded, and `debug.display_distributed_info`
   reports a zenoh session, which D87 left deferred. That is the criterion
   working as restated; the first draft's "moves by exactly ten, or the
   difference is explained" would have passed by explaining.
6. **`step`, `next` and `finish` agree with an uninterrupted run**, over every
   suite program: stepped to completion leaves the same final state as run
   normally. **Only over the reach D6 permits** — top-level values and bodies
   called directly from them — and the criterion says which programs that
   excludes rather than quietly passing on the ones it can do.

   **Met over the suite, 2026-10-06 — 134 of its 136 programs, with the two
   exclusions named.** `stepping_agrees_with_an_uninterrupted_run_over_the_whole_suite`
   (`crates/bund2-cli/tests/debugger_step.rs`) runs every program `conform`
   runs — each `HERMETIC.txt` entry and each probe with a golden — plainly and
   then under `s`, `n` and `f` driven to completion, and compares stdout and
   exit code: 402 stepped runs, all agreeing.

   **The two it excludes are excluded for one reason, and that reason is a
   defect rather than a limit of stepping** (F165): `terminal-words` and
   `debug-repl-words` read standard input, and under `--debugger` standard
   input is the session's command channel. The program takes the commands as
   its own input and the run does not finish. So the criterion is met for
   every program that leaves standard input alone, and a program that reads it
   cannot be debugged at all — which is a statement about the console's
   transport (§D1), not about `step`.

   The excluded set is derived from each program's source, so it cannot go
   stale, and then compared against those two names, so a third is noticed.

   *As first met, 2026-10-04:* **at process level, over four programs rather
   than the suite.** `crates/bund2-cli/tests/debugger_step.rs` spawns the binary — the
   claim is about a program's output and nothing in process captures stdout —
   and drives `s`, `n` and `f` each to completion against a plain run, matching
   stdout and exit code. The four cover what §D1's safepoint has to reach:
   top-level values, a word's body, a lambda run by a native, and `times`,
   which re-enters through `eval_lambda` and is the nested-loop shape §D1 says
   stepping could not have escaped.

   **Stepping is driven to the end, not for a fixed count.** A writer thread
   feeds the command until the child exits, because a finite script would hit
   EOF, detach, and let the rest run uninterrupted — the thing under test
   passing by not happening.

   **With a control, because the differential alone is a tautology.** A
   `--debugger` that did nothing would match a plain run perfectly;
   `the_debugger_stops_and_says_where` asserts the session reported a top-level
   position, stopped *inside* the word with §D2's symbol in the line, and
   leaked nothing into the program's stdout. That last is what the whole
   comparison rests on, and it is why the session writes to stderr.

   **What remains is the word "every".** Four programs are not the suite, and
   the sweep belongs in `xtask conform` beside the capture that already
   enumerates them. Recorded as owed rather than claimed.
7. **A breakpoint on a word stops before its body runs**, checked by frame depth
   and by the stack being what it was at the call.

   **Met, 2026-10-04.** The check sits in `push_frame`, *before* the push, so
   the stop is at the call: the position line is the caller's and the stack
   holds what the caller left. §D2's symbol is what makes it possible — a frame
   knew no name before it.

   **`continue` must not suppress it, and that is asserted.** A breakpoint
   stops whatever the stepping mode says, because `continue` sets `Mode::Run`
   and a breakpoint `Mode::Run` swallowed would be no breakpoint at all. The
   test sets one, continues, and requires both calls of the word to stop.

   **Two guards on the hot path**: one branch when nothing is attached, and a
   second when a debugger is attached but watches no word — so a session that
   only steps pays no lookup per call.
8. **A conditional breakpoint cannot change the program it watches.** The
   condition runs in a child VM: a lambda that pushes, switches stack, rebinds a
   word **or calls `bund.exit`** leaves the debugged program untouched and does
   not stop. The exit half is the one that needs the child VM — `exit_code` is
   `get_or_insert` and cannot be cleared.

   **Met, 2026-10-04, with the exit case demonstrated rather than argued.**
   `break w if { 9 bund.exit }` reports "the condition did not stop", both
   calls of `w` run to completion, and the **process exits 0** — the code the
   condition asked for never reaches the debugged program.

   **`bund2-interp` builds no child VM, and cannot.** A usable child needs the
   standard vocabulary, and `bund2-stdlib` depends on the interpreter rather
   than the reverse — so `Console::evaluate_condition` is the seam and the host
   supplies the child, as it supplies the transport and the `Reporter`. **The
   default declines, and declining is "do not stop"**: a host with no child VM
   gets a breakpoint that never fires rather than one that always does.

   **A fresh `Runtime` per evaluation, and the exit cell is the reason.** A
   reused child that once called `bund.exit` could not be cleared, so every
   later evaluation would inherit the exit — the same `get_or_insert` that
   forces the child VM, one level down.

   **The condition is run, not merely constructed.** §D3 says the condition
   *is* a lambda, so `{ 1 1 == }` is evaluated and then executed; without that
   every lambda condition would answer "left lambda/3 rather than a BOOL". A
   bare `1 1 ==` leaves its answer directly, so both spellings work. What
   counts as true is a `BOOL` on top and nothing else — guessing a truthiness
   for a LIST would invent a rule the reference does not have.
9. **A watchpoint on `@name` stops on a push to that stack and on no other**,
   and `watch workbench` stops on a workbench push — **two hooks, checked
   separately**, because the workbench path writes no stack tag.

   **Met, 2026-10-04, and the separateness is what the tests assert.** A named
   watch on every stack a program uses never sees a workbench push, and the
   workbench hook does; both halves in one test, because the claim is that they
   are *not* the same place. In process:
   `a_stack_watchpoint_stops_only_on_its_own_stack` and
   `the_workbench_is_a_second_hook_a_named_watch_cannot_see`; at process level,
   `a_watchpoint_fires_on_its_own_stack_and_the_workbench_is_separate`.

   **`watch @workbench` is refused rather than taken as either.** §D4's whole
   point is that the workbench is not a named stack, so accepting `@workbench`
   as a stack name — which would silently never fire — would contradict the
   section in the command line. The spelling is `watch workbench`.

   **One thing the tests had to learn.** A watchpoint is set *at* a safepoint,
   so one has to happen before a push can be watched; the first version of the
   in-process tests pushed directly and never read their own script, so nothing
   was being watched at all. They now arm through a `nop` word, which reaches a
   safepoint and pushes nothing.
10. **A debugged session runs no compiled code.** Checked by *building with
    `jit`* and asserting `compiled_entries()` reads `Some(0)` after a debugged
    run and `> 0` after the same program run without `--debugger`. The first
    draft asserted only the first half, which §D6 makes true by construction
    and therefore unable to fail.

    **Met, 2026-10-01, and `Some(0)` was the wrong assertion.** §D6 declines to
    *install* the tier, so `compiled_entries()` reads through a tier that is not
    there and answers **`None`**. That is the stronger statement — not "a tier
    that compiled nothing" but "no tier at all" — and writing the test is what
    found it. `Runtime::for_debugging` is the one `if`, in the one place a tier
    is installed, exactly as §D6 says it needs nothing new.

    The criterion's own warning stands and the test is built around it: the
    `None` half cannot fail, so the load-bearing assertion is the **control** —
    that this same program at this same threshold does compile and does enter a
    compiled body with no debugger attached. Without that, the first assertion
    is a tautology dressed as a measurement. A third assertion compares the two
    final states, because a debugger that changes what it observes is worse than
    a slow one.

    `a_debugged_session_installs_no_tier_and_the_control_compiles`, in
    `crates/bund2-runtime/src/tier.rs` — a module the `jit` feature gates, which
    is how "building with `jit`" is enforced rather than remembered.
11. **The safepoint costs nothing when no debugger is attached.**
    D84 traded suspension for a per-step check, and this is the price of that
    trade made checkable: with the check in place, inside RFC-0005 criterion 7's
    5% band, against the baseline below — **which is taken, as of 2026-09-30**.
    What remains is the second half, after the check exists.

    **Met, 2026-10-03.** Three CLEAN windows against the 2026-09-30 baseline,
    with §D1's safepoint in place and §D2's field not yet added — so this one
    reading is criterion 11's *after* and criterion 12's *before*.

    | row | A | B | C | mean | vs baseline | spread |
    |---|---|---|---|---|---|---|
    | `startup/parse/mixed` | 4.344 µs | 4.200 µs | 4.174 µs | 4.239 µs | **−1.29%** | 4.1% |
    | `dispatch/dup_drop/w3000` | 84.060 µs | 83.525 µs | 86.739 µs | 84.775 µs | **+1.41%** | 3.8% |
    | `dispatch/native_call/w4000` | 94.130 µs | 95.511 µs | 91.863 µs | 93.835 µs | **−0.49%** | 4.0% |
    | `dispatch/literal_only/w1000` | 15.359 µs | 15.576 µs | 15.175 µs | 15.370 µs | **+2.49%** | 2.6% |
    | `startup/registry/register_all` | 44.084 µs | 45.117 µs | 42.330 µs | 43.844 µs | +0.92% | 6.6% |
    | `dispatch/literal_push/w2000` | 44.543 µs | 47.289 µs | 46.257 µs | 46.030 µs | **+5.77%** | 6.2% |

    **The four rows that can carry a claim are all inside the band**, the
    largest being `literal_only` at +2.49%. Guard verdicts: `peak=12.3%
    (Brave)`, `peak=5.3% (claude)`, `peak=8.6% (launchd)`, means 2.8–2.9%.

    **`literal_push` reads +5.77%, outside the band, and it is one of the two
    rows this document already disqualified** — "a 4% move there is
    indistinguishable from another Tuesday", and neither criterion "may rest a
    verdict on those two rows". Its spread between clean runs was 4.8% at
    baseline and is 6.2% now, so the move is smaller than the row's own
    variation. It is reported rather than omitted, because a disqualified row
    that is quietly dropped the one time it reads badly is how a band gets
    adjusted until a verdict lands.

    **What the reading does *not* support is "costs nothing".** `literal_only`
    moved +2.49% with a 2.6% spread of its own — at baseline that row had a
    0.5% spread and was the sharpest instrument here; it is blunter today. So
    the honest claim is the one the criterion makes and no more: **the per-step
    branch is inside the 5% band on every row that can carry a verdict.**
    Resolving it to zero would need a sharper instrument than this host has.

    **What it cost, for the next attempt.** Eighteen windows were skipped
    before the first was opened — a second agent session on this machine
    building and then running an unrelated project, at 100% for six consecutive
    probes and above 1300% for eight more. Once the host freed, the first three
    attempts were all CLEAN. **That is the opposite of 2026-09-30's three CLEAN
    out of seventeen**, and the difference was not patience within a window but
    waiting for the machine: the contaminant was one process, and nothing about
    the protocol mattered until it exited.

    **`dispatch` is the group that decides this and `startup` is the control.**
    The check runs per step, so it compounds where steps do; `startup` carries
    `registry/register_all` at ~41.8 µs and `parse/mixed` at ~4.3 µs, neither of
    which a per-step cost can reach. A reading that moved `startup` and not
    `dispatch` would be measuring the machine.

    **Read again on 2026-10-08, by count — D117.** D113 put three more checks
    on this path after the reading above: one where a frame is pushed, one
    before every native, one at dispatch. The acceptance review of that date
    raised that nothing had measured them. The owner ruled that the cost is
    read in instructions retired, since criterion 12 records that this host
    can no longer resolve the band by timing. The commit before D113 against
    the tree of 2026-10-08, the `dispatch` shapes run 300,000 times: **Tier 0
    costs 1.6% to 3.5% more instructions; with a tier, −0.4% to +1.9%.** Every
    row is inside the 5% band. D117 carries the table and what a count does
    not say — chiefly that it is not time.

12. **§D2's growth is measured against a baseline taken first.**
    `Frame` gains a symbol and a span, on a structure pushed per call, so
    `dispatch` **before** the field is added and then after, inside the same 5%
    band. The first draft named the band and no baseline, leaving nothing to
    compare against; **the baseline is now taken** and the four usable rows are
    named below.

    **Met, 2026-10-03, for the symbol — and the span half is withdrawn from
    this criterion.** `Frame` gained `who: Option<Symbol>` and nothing else,
    because §D2 says why a span cannot be added yet: `lower_with_spans` is top
    level only, so no value inside a body carries one, and producing them is
    the IR RFC-0003 §S5 describes and that does not exist. A field with no
    possible writer would have made this criterion measure a permanent `None`
    and need re-measuring the day the IR landed and the field began to be
    written. **So this is one word, not two**, and the span's cost is owed to
    whichever work item produces spans.

    `Frame` is **48 → 56 bytes**, measured on the tree either side of the
    change and pinned in `a_frame_is_the_size_it_was_measured_at`, so a later
    field cannot arrive unnoticed between two benchmark runs. `Symbol` is a
    `u32` with no niche, so `Option<Symbol>` costs eight bytes rather than
    four; a `NonZeroU32` would halve it inside existing padding and is **not**
    done, because the reading below did not ask for it and shrinking a field to
    fit a measurement nobody needed is the wrong order.

    **Three CLEAN windows, against criterion 11's reading as the *before*.**
    One attempt was discarded at `peak=128.4% (PerfPowerServices)`; the three
    kept read `peak=6.9% (dasd)`, `peak=9.1% (Tailscale)`, `peak=5.6%
    (macmon)`, means 2.7–2.9%.

    | row | A | B | C | mean | vs criterion 11 | spread |
    |---|---|---|---|---|---|---|
    | `startup/parse/mixed` | 4.236 µs | 4.251 µs | 4.196 µs | 4.228 µs | **−0.26%** | 1.3% |
    | `dispatch/dup_drop/w3000` | 80.031 µs | 84.005 µs | 80.469 µs | 81.502 µs | **−3.86%** | 5.0% |
    | `dispatch/native_call/w4000` | 93.836 µs | 90.365 µs | 93.611 µs | 92.604 µs | **−1.31%** | 3.8% |
    | `dispatch/literal_only/w1000` | 15.671 µs | 15.775 µs | 14.552 µs | 15.333 µs | **−0.24%** | 8.4% |
    | `startup/registry/register_all` | 42.203 µs | 42.762 µs | 41.567 µs | 42.177 µs | −3.80% | 2.9% |
    | `dispatch/literal_push/w2000` | 42.471 µs | 45.321 µs | 43.203 µs | 43.665 µs | **−5.14%** | 6.7% |

    All four rows that can carry a claim are inside the band. `literal_push` is
    again outside it, at −5.14%, and is again one of the two rows this document
    disqualified — this time in the *fast* direction, which is the clearest
    possible demonstration that the row measures the host rather than the code.

    **Every usable row moved negative**, after a change that can only add work:
    eight more bytes written per frame push. A field that made the interpreter
    faster is not a finding, it is drift — two readings twenty minutes apart on
    the same host, with the slower one taken first. So the verdict this
    criterion can state is **no detectable cost**, not a percentage.

    **The instrument is degrading, and that is the reading worth keeping.**
    `literal_only`'s spread between clean runs has gone 0.5% → 2.6% → 8.4%
    across the three sessions. On 2026-09-30 this document called it "the
    sharpest instrument here"; today it is the noisiest of the four, and a
    change costing 4% would be invisible in it. Measured end to end — 2026-09-30
    with neither change against today with both — the four usable rows read
    −1.55%, −2.51%, −1.80% and +2.24%, every one of them smaller than the
    spread of some window that produced it.

    **What follows for the next criterion that wants a 5% band on this host.**
    It cannot have one. Two of six rows were at or below the noise floor when
    the baseline was taken, and a third has joined them since. A future
    measurement needs either a quieter machine, more windows per reading, or a
    different instrument — and deciding which is a decision, not a number to be
    produced by running the same script again.

    ### The protocol both criteria use — written before the numbers, 2026-09-30

    Not a style note. RFC-0005's criterion 7 recorded a `dispatch` failure at
    `+9.96%`, `+11.94%` and `+12.62%` that repetition withdrew at `+2–3%`, and
    F135 exists because readings taken on a busy host were believed for a day
    before being discarded. So:

    - **Three runs, each CLEAN**, through
      `crates/bund2-bench/scripts/guarded_bench.sh`, which samples the busiest
      non-benchmark process every three seconds for the whole window and prints
      a verdict beside the numbers.
    - **All three recorded, not a median.** Criterion 7's own table prints every
      run, which is what made its withdrawn failure visible.
    - **A discarded run is recorded with its verdict and the named
      contaminant.** "Discarded on its window" without the process name tells a
      later reader nothing about whether the host or the code was at fault.
    - **The baseline is taken before the code**, not reconstructed after. Both
      criteria say so because neither can be checked otherwise.
    - **The 15% threshold is not adjustable to make a verdict land.** Criterion
      7 says deciding what a measurement accepts is "worth doing deliberately
      rather than by adjusting a fixture until the verdict lands". If 15% is
      wrong for a machine, that is a decision taken on its own merits and
      recorded — never a route to a number.

    **The agent driving the run can be its own contaminant, which F135's
    protocol does not say and should.** On 2026-09-30 a run driven by this
    session was discarded at `peak=22.4% (claude)`; prebuilding the benchmark so
    the window held only the measurement removed it as the peak. So a
    measurement taken by an agent needs the build done first and the agent idle
    through the window — and may still be unable to produce a CLEAN run, which
    makes these two baselines a human's to take.

    ### The baseline — three CLEAN runs, 2026-09-30

    Taken before either change exists, which is what both criteria require.
    `guarded_bench.sh` over `^(startup|dispatch)/`, three windows that the guard
    passed; every figure is Criterion's median.

    | row | A | B | C | spread |
    |---|---|---|---|---|
    | `startup/registry/register_all` | 42.915 µs | 42.708 µs | 44.715 µs | **4.7%** |
    | `startup/parse/mixed` | 4.286 µs | 4.265 µs | 4.332 µs | 1.6% |
    | `dispatch/dup_drop/w3000` | 82.835 µs | 83.226 µs | 84.727 µs | 2.3% |
    | `dispatch/native_call/w4000` | 93.526 µs | 94.441 µs | 94.933 µs | 1.5% |
    | `dispatch/literal_only/w1000` | 15.042 µs | 14.988 µs | 14.960 µs | 0.5% |
    | `dispatch/literal_push/w2000` | 44.239 µs | 42.200 µs | 44.120 µs | **4.8%** |

    **Two of six rows vary by nearly 5% between clean runs with nothing
    changed**, and that is the finding rather than the medians. RFC-0005
    criterion 7's band is 5%, so on `register_all` and `literal_push` the band
    sits **at or below this host's noise floor**: a 4% move there is
    indistinguishable from another Tuesday. Neither criterion 11 nor 12 may
    rest a verdict on those two rows.

    **The four rows that can carry a claim** are `parse/mixed`,
    `dup_drop/w3000`, `native_call/w4000` and `literal_only/w1000`, at 0.5–2.3%.
    `literal_only` at 0.5% is the sharpest instrument here, and it is a
    dispatch row, which is where §D1's per-step check would show.

    **This is why the protocol demands three runs recorded individually.** One
    run would have produced a baseline with no variance attached, and a median
    of three would have hidden a 4.8% spread inside a single number — then a
    later 4% regression would have read as real. The spread *is* the
    measurement.

    ### What it cost, recorded because it bounds who can repeat it

    **Three CLEAN windows out of seventeen attempts**, across an evening,
    naming eleven distinct contaminants: `mediaanalysisd` (218%),
    `PhotoAnalysis` (99.6%), Webex (74.2%), Slack (33–74%, three consecutive
    windows), `spotlightknowledged.updater` (45.2%), `sysmond` (51.1%),
    CoreServices (57.9%), Brave (38.7%), `launchd` (16.3–31.1%),
    `MenuBarAgent` (19.9%), `NotificationCenter` (**exactly 15.0%** — rejected
    by the narrowest possible margin, with an otherwise exemplary 3.6% mean),
    and **this session itself** at 21.0% and 30.6%.

    **Quitting applications was not what fixed it.** Slack going changed
    nothing; Brave, Spotlight, `launchd` and `NotificationCenter` followed. What
    worked was patience — a watcher that waited 800 s through the photo indexer
    — plus running batches detached and taking the first clean window.

    **The agent must run the measurement detached, and prebuilding is not
    enough.** A foreground tool call spikes the driving process *inside its own
    window*: two runs were lost to `claude` at 21.0% and 30.6% **after** the
    benchmark was already built. A 25-second lead-in before the first run of a
    detached batch removed it, and `claude` was never the peak again. F135's
    protocol does not say this, and it should: it is the difference between a
    measurement an agent can take and one it cannot.

    **A second agent session on the same machine is a contaminant this session
    can neither see nor stop — and the guard could not see it either (F145).**
    On 2026-10-01 criterion 11's reading was abandoned after twenty-two minutes
    of waiting: a session working on an unrelated project held one `rustc`
    between 100% and 498% of CPU throughout. The guard excluded `cargo` and
    `rustc` **by name**, so that the benchmark's own build would not read as
    interference, which meant it would have called that window CLEAN with the
    machine saturated. `guarded_bench.sh` now excludes by **ancestry** — a
    process is the measurement's own exactly when it descends from the guard
    invocation — and the foreign compile is named as the peak.

    So the protocol gains a precondition it did not have: **the host is not
    quiet while another agent session is building.** Waiting is the only
    remedy, and it is not this session's to apply.

    **What a non-quiet host looks like, recorded so the next attempt knows what
    to clear.** Five attempts on 2026-09-30 produced five contaminated windows
    from four distinct processes — `claude` at 22.4%, a system framework at
    37.8%, Webex at 74.2%, Tailscale at 23.4%, `MenuBarAgent` at 19.9% — with
    load averages of 1.0–1.4 and means falling 12.7% → 10.2% → 8.1%. **It was
    not load**: each offender was idle when checked beforehand and spiked inside
    the sixty-second window. Quiet here means those agents quit, not merely
    unoccupied.
13. **`debug` and `debug.shell` run under a capture, and are refused as
    unstable.** Measured, correcting an `[UNGROUNDED]` claim: with stdin closed
    the oracle's `debug` runs to completion — readline gets EOF and advances —
    so "no golden can run them" was wrong. What stops a golden is that the table
    embeds a fresh id and stamp per run, so two captures differ and the capture
    refuses them. That is the F14 class and belongs in `UNSTABLE.txt`, not in a
    claim that the words are unreachable.

    **The second half is wrong too, and a golden holds both words —
    corrected 2026-10-08.** `tests/golden/probes/debug-repl-words.golden` runs
    `debug.shell` and `debug`, was captured from the oracle with two runs
    agreeing, and passes. The capture normalises the id and the stamp (F14),
    so the table is stable once they are. This criterion is therefore **met
    by a golden**, not by a refusal. The acceptance review of this date found
    it, and found that the claim had been repeated as advice: the owner was
    told that no golden could hold `debug` while choosing what `debug` should
    become. The choice made kept the word as the reference's, so the golden
    still passes; one of the options offered would have failed it.

## Open questions

- **D6 — answered by D84**, 2026-09-30: VM-per-task, and the debugger stops a
  thread rather than suspending a word. Left listed because the first draft of
  this document adopted its default silently, and a reader tracing D84 should
  find where that happened.
- **Whether the safepoint check is affordable.** D84 replaced a decision with a
  measurement, and criterion 11 is it. If the check moves `dispatch` beyond
  criterion 7's band, the shape has to change — a `cfg`, or a check only under
  `--debugger` — and that is a design question this RFC would answer then.
- **RFC-0003 §S5's IR, for source locations.** §D2 can give a backtrace a word
  symbol and a value index today; a file and line need spans for nested values,
  which `lower_with_spans` does not produce and says it does not.
- **Where the trace and the profiler hook.** Both need the frame loop rather
  than `eval_observed`, and that touches an accepted Preservation row in
  RFC-0003. Whether that is an amendment to RFC-0003 or a section here is a
  scoping question, not a language one.
- **Whether `--profile` is in scope at all.** It is in the contract, and §D7
  shows the "free profiler" is not free. Deferring it would leave one reference
  flag unimplemented and named as such, which may be the right trade.

**Four departures from the research, each now recorded rather than silent.** The
first draft departed from §3.3 in four places without an ERRATA entry, and three
were errors rather than choices: dropping "in a child VM context" from (c),
claiming (a)'s step-into already worked, treating (h)'s profiler as free, and
spelling the flag `--debug` where the reference has that name taken. Only the
last is a deliberate choice, and none is a departure any more — (c) and (a) are
corrected to follow the research, (h) is corrected to contradict it, and the
flag follows the reference. **`docs/research/ERRATA.md` gains one entry**, for
(h): the tier counters do not give a profiler for free in Bund2, because they
are totals, they exist only under `jit`, and §D6 removes the tier.

## Amendment, 2026-10-07 — debugger words (D113)

- Status: **Parts A and B decided and built.** Written
  at the repository owner's request, on the idea that the debugger's commands
  should be **words** — runnable in a script, in `debug.shell`, and at the
  `--debugger` console alike. The owner ruled on D113's six questions the same
  day, each as recommended. The text below is the draft as ruled on; **"What
  building Part A changed"** and **"What building Part B changed"**, at the
  end, are where it and the code differ.
- Registers searched by subject (`debug`, `debugger`, `safepoint`,
  `breakpoint`, `debug.shell`, `stacks`, `classes`, `input`): D36, D52, D84,
  D94, D99, D101, D102, D112, F10, F37, F52, F139, F165, and criterion 6, 10
  and 13 of this document. D55 (the effect audit's palette) and RFC-0002's
  `Input` amendment are consumed below.

### What the reference does, which the first design did not follow

**The reference debugger's command language is Bund.** `debug` applies one
term, then reads lines: an empty line moves on, and any other line goes to
`bund_compile_and_eval` in the same VM
(`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:81-95`).
`debug.shell` is that loop with no stepping
(`reference/Bund/src/stdlib/functions/debug_fun/debug_shell.rs:23-33`). The
callee parses the line and `vm.apply`s each value, skipping `NONE` and
stopping at `EXIT` (`reference/Bund/src/stdlib/helpers/eval.rs:7-37`). So at a
reference stop **every word is a debugger word**, and it has no stepping,
breakpoint or watch command at all. A shell is itself a Bund program there:
`bund.prompt { … bund.eval … } input*`
(`reference/Bund/examples/code_snippets/bund_shell.bund:24`).

**Bund2 has two languages where the reference has one.** The `--debugger`
console parses its own vocabulary and refuses anything else as "not a
command" (`crates/bund2-cli/src/debugger.rs`, `parse`). The words `debug` and
`debug.shell` evaluate Bund in the live VM as the reference does
(`crates/bund2-stdlib/src/terminal.rs`, `debug_word`, `debug_shell`) and know
nothing of frames, breakpoints or watches. Neither can do what the other
does: a console session cannot run a word, and a word cannot arm a
breakpoint.

**And one promise of this document is unkept.** The Preservation row for
`debug` says it "keeps working as a thin wrapper: evaluate the string with the
debugger attached, breakpoint on entry", stepping *into* things. `debug_word`
is the reference's loop over top-level terms. Part B below is what would keep
the promise.

### §W1 — The vocabulary

Thirteen words, all Bund2-only. None is in the reference's registry, so none
moves `COVERAGE` or `IMPLEMENTED`, and none can be held by a golden against
the oracle.

| word | operands | what it does | part |
|---|---|---|---|
| `debug.backtrace` | — | renders the frame stack | A |
| `debug.stacks` | — | renders every stack: name, depth, top (§D5's `stacks`) | A |
| `debug.info` | — | renders what is armed: breakpoints, watches, the stepping mode | A |
| `debug.feed` | STRING | queues one line for the program's next read — §W4 | A |
| `debug.break` | STRING | break when a frame for that word is pushed (§D3) | B |
| `debug.break.if` | STRING, LAMBDA | the same, when the lambda answers true in a child VM (§D3) | B |
| `debug.watch` | STRING | stop on a push to that named stack (§D4) | B |
| `debug.watch.workbench` | — | stop on a push to the workbench (§D4's second hook) | B |
| `debug.delete` | STRING | stop watching that word or stack | B |
| `debug.step` | — | stop before the next value, into any body | B |
| `debug.next` | — | stop before the next value at this depth | B |
| `debug.finish` | — | stop when the current frame has left | B |
| `debug.continue` | — | stop nowhere until something armed fires | B |

`debug.display_stack` and `debug.display_workbench` already cover "render the
current stack" and are not duplicated. §D5's `words` and `classes` are not in
this table: they are views of the slot table and the class table, not of a
debugged run, and stay §D5's.

**Where the inspecting words write is a decision (D113.3).** The session
writes to stderr so the program keeps stdout, which criterion 6's method
depends on. `debug.display_stack` writes to stdout, as the reference's does. A
word typed at the console is still a word, so the two rules meet here.

### §W2 — The console evaluates Bund

At a stop, a line the console does not recognise as one of its own short
forms is **evaluated as Bund in the debuggee's VM**, as the reference does.
The short forms stay, each defined as the word it stands for:

| typed | means |
|---|---|
| `s`, `step` | `debug.step` |
| `n`, `next` | `debug.next` |
| `f`, `finish` | `debug.finish` |
| `c`, `cont`, `continue` | `debug.continue` |
| `bt`, `backtrace`, `where` | `debug.backtrace` |
| `i`, `info` | `debug.info` |
| `break w`, `b w` | `"w" debug.break` |
| `break w if { … }` | `"w" { … } debug.break.if` |
| `watch @s`, `watch workbench`, `delete w` | the three words likewise |

**`stack` and `st` are a collision, and a decision (D113.4).** The console
takes `stack` to render the current stack. The reference binds `stack` as an
alias of `ensure_stack`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:35`) and Bund2
registers it. Once lines are Bund, one meaning must give way at the console.
No other short form is a registered word today; every one becomes
unavailable as a *bare* Bund line at the console, which is the cost of
keeping them.

**A failing line is reported and the debuggee stays stopped**, at `Warning`,
as `debug_shell` already does and for its reason: the program has not stopped
(D36). An empty line is nothing, as now.

**What a typed line may do to the program, stated.** It runs in the program's
own VM, so it can push, switch stacks, rebind a word, and call `bund.exit` —
whose request cannot be withdrawn (§D3 gives the cell). §D3 put *conditions*
in a child VM for exactly this, because a condition runs unasked. A typed line
is asked for; the reference permits it and so do `debug` and `debug.shell`
here. This amendment keeps §D3's rule for conditions and does not extend it to
typed lines.

**What the mechanism has to change, read from the code.**

- `Console::next_command` answers a `Command`, and only commands and text
  cross that seam because `Interp` is not `Send`
  (`crates/bund2-interp/src/debug.rs`, module doc). A line is text, so it
  crosses as `Command::Eval(String)`.
- `bund2-interp` cannot compile source: it depends on `bund2-ir`,
  `bund2-value` and `bund2-api`, and the parser is reached through
  `bund2-stdlib` (`singles.rs`, `eval_source`). The host already supplies the
  child VM for conditions for the same reason (`Console::evaluate_condition`).
  So the host supplies the evaluation too: a function the embedder installs,
  `fn(&mut dyn Vm, &str) -> Result<(), Error>`, which the CLI fills with
  `eval_source`.
- **`safepoint_for` takes the whole `Debug` out of the `Interp` for the
  length of the stop** (`crates/bund2-interp/src/lib.rs`, `safepoint_for`).
  Two consequences. A typed line's own values are not stepped, because the
  nested loop finds no debugger — wanted. But `debug.break` typed at the
  console would find no debugger to arm — not wanted. So `Debug` splits:
  **the console is taken for the stop; what is armed stays on the
  `Interp`.**

`[UNGROUNDED]` — **that evaluating a line is sound at every safepoint.** Four
sites reach `safepoint_for`: the top-level stream, `run_to`'s loop head, the
frame push for a breakpoint, and the two pushes for a watch. The first two sit
between values, where a nested `Interp::apply` records its own floor exactly
as a native's re-entry does. The breakpoint site runs after `take_pending` has
cleared the tail request, which reads as safe. **The watch sites are inside a
native that is mid-push**: a `+` has pulled its operands and not yet pushed
its sum, so a typed line sees a stack the program never showed and may
rearrange it under the native. Memory-safe by the language; whether every
invariant the frame loop keeps holds there has not been read call by call.
Criterion W3 is the check, and Q41 records the gap.

### §W3 — Words reach the debugger through the VM

`Vm` gains one method, defaulted to "nothing is attached", in the shape
`report` and `read_line` have. The request type moves to `bund2-api` so a word
in `bund2-stdlib` can name it — an amendment to RFC-0002's `Vm` surface, to be
written with the build:

```rust
// bund2-api
pub enum Debugging {
    Backtrace, Stacks, Info,
    Break(String), BreakIf(String, BundValue), Watch(String), WatchWorkbench,
    Delete(String),
    Step, Next, Finish, Continue,
}
// on `Vm`
fn debugging(&mut self, _ask: Debugging) -> Result<Option<String>, Error> {
    Ok(None)
}
```

An inspecting request answers text; an arming or moving one answers nothing.
`Command` in `bund2-interp` becomes this type plus `Eval`, so the console and
the words say one thing in one vocabulary.

**The words are opaque**, for the reason `debug.shell` is: what they do to the
run is not a stack effect. That keeps them out of D55's palette and out of
RFC-0005's compiled bodies without a list to maintain.

### §W4 — Typing into a debugged program: `debug.feed`

D112 left this open: a debugged program is given no input, because the
console owns standard input (F165).

**Under `--noeval` the word is a stub — D121, 2026-10-08.** Queued before
`debug.shell`, a line is run by the shell, so a program built `--noeval`
evaluated a string it held with nobody typing it. The flag now refuses
`debug.feed` with `bund DEBUG.FEED functions disabled with --noeval`, which
also means it cannot feed `input` under that flag. This document had not
asked what the flag does to the word.

`"alice" debug.feed` queues one line. **The queue is the interpreter's, not
the input's**: `Vm::read_line` answers from it first and asks the installed
`Input` only when it is empty, and `Vm::read_secret` likewise. So the rule
holds for the terminal, for `NoInput` and for a test's script without any of
them knowing, and `Input` does not change. `Vm` gains `feed_line`, defaulted
to dropping the line.

- Under `--debugger` an empty queue is still the end of input, as D112 made
  it. A session that feeds nothing is unchanged, and so is criterion 6.
- In a plain run a script may answer its own `input` in advance. That is new
  behaviour for a Bund2-only word and touches no reference word.
- A fed line is not remembered in a history unless the reading word says so;
  `remember_line` is untouched.
- A fed secret is visible where it was typed. Stated, not solved.

**The read has to be anticipated**: the line must be queued before the word
reads. With Part B that is `"input" debug.break`, then feed, then continue.
`[UNGROUNDED]` — that a breakpoint on a *native* stops before it runs: §D3
breaks "when a frame for that symbol is pushed" and a native pushes no frame.
Criterion W5 measures it; if it fails, a stop-on-read is needed and is not
designed here.

### §W5 — Part B: arming and moving from a script

Part A needs no debugger attached. Part B does, and a script has no
`--debugger` flag. Two questions follow, and **both are D113's**.

**Where does a script's breakpoint stop (D113.5)?** Nothing is attached, so
there is no console to block on. The candidates: the arming words refuse
without `--debugger`; they are inert with a notice; or the first arming word
attaches a console over the VM's own `Input`, so the stop is a prompt on the
terminal the program already has — which is `debug`'s Preservation row kept,
and makes `debug.step` in a script what `breakpoint()` is elsewhere. Under a
capture the third reads the end of input and detaches, which is what
`Console` already does when its host is gone.

**What of the tier (D113.6)?** §D6 disables it by the flag, in the one place
it is installed (`Runtime::for_debugging`). A script that arms mid-run has a
tier already installed, and a compiled body has no safepoint, so a breakpoint
inside one would not fire. §D6 names what would be needed — "per-word pinning,
with its own decision" — and says the machinery for the research's first
mechanism does not exist. F139 measured that no corpus program compiles a body
at the default threshold, so the gap is narrow; it is still a debugger that
can miss a stop, silently.

**A script that calls a moving word is outside criterion 6**, by
construction: its stepped and plain runs are different programs.

### Preservation

| behaviour | disposition |
|---|---|
| Every reference word | **Unchanged.** No reference word is added, removed or altered. |
| `debug`, `debug.shell` | **Unchanged in Part A.** In Part B `debug` may gain the stepping its row promised; that is D113.5's to say. |
| The console's commands | **Preserved as short forms**, less `stack`/`st` if D113.4 rules so. |
| A line the console does not know | **Changed**: evaluated, where it was refused. |
| A debugged program's input | **Changed only when fed.** |
| `conform`, `COVERAGE`, `IMPLEMENTED` | **Move by zero.** The words are outside the reference's list. |

### Criteria

- **W1. The three inspecting words answer the same text in a script, in
  `debug.shell` and at a console stop**, for one program stopped at one
  place. Checked by a scripted console and a scripted input.
- **W2. A line typed at a stop runs in the debuggee's VM and is not stepped**:
  the stack shows its effect, and `debugger_stops` does not move for its
  values.
- **W3. A line may be typed at each of the four safepoint sites** — top
  level, loop head, breakpoint, and both watch hooks — and the program then
  runs to the end it would have reached had the line been part of it, or the
  site is refused by name. Closes Q41.
- **W4. A session that only steps is byte-identical to before**: criterion
  6's sweep passes unchanged, with no exclusion.
- **W5. `debug.feed` reaches `input`, `input*`, `password` and `debug.shell`**
  through a scripted run, in order, and an empty queue under `--debugger` is
  the end of input. Whether a breakpoint on `input` stops before the read is
  measured here.
- **W6. A failing line and a line that calls `bund.exit`** are each shown to
  do what §W2 says: the first reports and stays stopped, the second ends the
  program with its code.
- **W7 (Part B). Arming from a word and arming from the console are the same
  state**: `debug.info` lists both, `debug.delete` removes either.
- **W8 (Part B). The tier question is answered by a test, not by F139**:
  build with `jit`, lower the threshold until a body compiles, arm a
  breakpoint inside it from the script, and show what D113.6 ruled.
- **W9. No new word is promotable or fixed-effect**: the honesty tests in
  `bund2-stdlib` hold with the thirteen registered.

### What this does not do

§D7's trace stays deferred (D102). §D8's history for the `--debugger` console
stays unbuilt. Source-location breakpoints stay blocked on RFC-0003 §S5's
spans (§D2). `words` and `classes` stay §D5's.

### What building Part A changed

Recorded here rather than by rewriting the sections above, which are what the
owner ruled on.

- **§W1: `debug.info` renders what is armed, and not the stepping mode.**
  With nothing attached it says `no debugger is attached`.
- **§W2: the debugger is not split in two.** The draft said the console would
  be taken for a stop and the armed state left. What was built is smaller: the
  whole of it is put back for the length of a typed line, marked busy, so the
  line can read what is armed and nothing in the line is a stop. Part B, where
  a word arms from outside any stop, needs no more than that.
- **§W2: one guard the draft did not have.** A typed line sets aside the tail
  request of a native it interrupted and restores it. Without it a failing
  word in the line discarded the body that native had filed. This is the
  answer to the section's `[UNGROUNDED]` marker: sound at all four sites,
  given the guard. Criterion W3.
- **§W3: `Vm::debugging` takes the three inspecting requests and answers
  `Option<String>`.** The arming and moving variants join the type with Part
  B; `Command` is not yet that type.
- **§W4: `feed_line` answers whether the line was kept**, so `debug.feed` on
  a `Vm` with no queue fails in words where the draft had it drop the line.
- **§W4's `[UNGROUNDED]` marker is answered, and the answer is no.** A
  breakpoint on a native such as `input` never fires. A read is fed ahead, or
  by stepping to it.
- **§W2: `?` and `help` print the short forms**, since a mistyped command no
  longer does.

Criteria met: W1, W2, W3, W4, W5, W6, W9. W7 and W8 are Part B's.

### What building Part B changed

- **§W1: `debug.break.if` takes STRING, STRING** — the word, then the
  condition as Bund source — where the table says STRING, LAMBDA. §D3 runs a
  condition in a child VM, the child is handed text, and a lambda value has
  no source form.
- **§W3: one type, as designed.** `Debugging` now carries the arming and
  moving requests and `Vm::debugging` answers `Result<Option<String>, Error>`:
  text for a view, nothing for the rest, and an error where no debugger is
  attached and none can be.
- **§W5, D113.5: the console is the embedder's, made on demand.**
  `Interp::console_factory`; the CLI supplies `OverInput`, which has
  `Stdio`'s vocabulary and conditions and reads its lines through
  `Vm::read_line`. `Console` gains `next_command_with`, defaulted to
  `next_command`, which is how a console with no stream of its own is handed
  the stopped VM to read through. The attached debugger starts running, not
  stopped.
- **§W5, D115, 2026-10-08: not in a bundle.** A bundle sets
  `Interp::console_refused` and supplies no factory, and there the arming and
  moving words do nothing: no console, no stop, tier left on. D113.5 had made
  a shipped program stop at a prompt if a breakpoint was left in it. The
  views still print. RFC-0006 §B3a has the other half.
- **§D8, for this console only: it has a history**, because a read through
  `Input` names one. The `--debugger` console still reads plain lines.
- **§W5, D113.6: the tier is switched off, not removed.** It is out of the
  interpreter while it runs, so there is nothing to drop from inside a word.
  The body that was running compiled when the word was called finishes so.
- **§D1: a detach is final.** Stated in D113's note; it applies to
  `--debugger` as well.
- **The Preservation row for `debug` is still unkept**, and this amendment
  no longer says Part B keeps it. `debug.step` is the word that does what the
  row describes; `debug` is the reference's loop.

Criteria met: W7 and W8, the latter in both builds — the stop always, and
that a compiled body was bypassed where there is a tier. With these all nine
are met.

### `debug.run`, and the row for `debug` — D113.5's second ruling

**Ruled by the repository owner, 2026-10-07**, on being shown three options
for `debug`: leave it and withdraw the row's promise; make it the stepping
wrapper the row describes, which is a deviation from the reference's word;
or leave it and add a word. The third.

**The Preservation row for `debug` is withdrawn.** `"…" debug` is the
reference's word and stays it: a table per top-level term, then a prompt
whose every line is Bund
(`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:81-95`). It
does not become a wrapper and does not step into anything.

**`debug.run` is the wrapper.** A fourteenth Bund2-only word: it takes a
STRING of Bund, asks for stepping as `debug.step` does, and evaluates the
string with a stop offered before each of its terms.

| typed at the first stop | what stops |
|---|---|
| `s` | each term of the string, and each value of every body a term enters |
| `n` | each term of the string, and no body |
| `c` | nothing more, until something armed fires |

**Why a word and not `debug.step` before `bund.eval`.** Measured: that pair
stops inside the words the string calls and at none of the string's own
terms. `bund.eval` hands each term to the VM through `apply`
(`reference/Bund/src/stdlib/helpers/eval.rs:7-37` is the shape Bund2 keeps),
and the safepoints are in the VM's two loops — the stream's and a body's —
which a string's terms pass through neither of. `debug.run` offers the
safepoint itself: `Debugging::Term(index, value)`, one per term, which the
VM answers by stopping if stepping says to and by nothing otherwise.

A stop at a term reads `debug.run's string at 2  next: w`. The index is into
the string's own terms; the frame the word was called from is still what
`bt` shows.

**What it shares with `debug.step`.** It attaches a console if none is
(§W5), turns the tier off (D113.6), detaches and runs the string whole when
nobody is there, and is refused in words by a VM given no console. Stepping
is not switched off when the string ends: `s` after its last term stops at
the program's next value, as it would after any word.

**What it does not do.**

- **Typed at a stop, the string runs unstepped.** Nothing in a typed line is
  a stop (§W2). The stepping it asked for applies to the program once the
  line is done.
- **A failing term is the word's failure**, with the term's own message and
  none of `bund.eval`'s `Attempt to evaluate value …` frame.
- **No golden holds it**, for §W5's reason: its stepped and plain runs are
  different programs.

| behaviour | disposition |
|---|---|
| `debug` | **Preserved as the reference has it.** The earlier row is withdrawn. |
| `debug.run` | **New, Bund2-only.** Outside the reference's list, so `conform`, `COVERAGE` and `IMPLEMENTED` move by zero. **Under `--noeval` it is a stub — D120, 2026-10-08**: it evaluates a string as `bund.eval` does, and this document had not asked what that flag does to it. |

Checked by two tests at process level — the terms under `s`, `n`, `c` and no
input, with the `bund.eval` gap asserted beside them; the stop inside a word
and a failing term — and the refusals in the unit test of the arming words.

### The console at a real terminal

Driven through a pseudo-terminal, 2026-10-07, outside the suite: a script's
`debug.step` with no flag gives the `(bund2)` prompt with editing and a
recalled history; the program's own `input` and `password` read the same
terminal after `c`; `--debugger` at a terminal takes `debug.feed`. One thing
was added: **a detach says so** — `bund2: detached; the program runs on.` —
where end-of-input left a running program and no explanation.

**Ctrl-C at the prompt detaches, as Ctrl-D does.** The input seam answers
both the same way (D112), and the console reads through it.

**No test in the suite holds a terminal.** It would need a pseudo-terminal
dependency; the claims above are a measurement and not a criterion.

### A breakpoint on a native, and on an alias — Q41's second half, closed

**Built 2026-10-07, on the owner's instruction.** §D3 breaks "when a frame
for that symbol is pushed". Two kinds of word were armed and never reached:

- **A native pushes no frame.** `break input` listed a breakpoint nothing hit.
- **An alias is called under another name.** A frame and a native are both
  known by the word the alias resolves to, so `break dup` — `dup` is an alias
  — armed a name nothing is ever called under. The same held for an alias of
  a lambda.

Both stop now. **The stop is before the word runs**: for a native, where it
is about to be called, with its operands still on the stack — which is what a
condition is shown, and what `st` shows. A line fed there with `debug.feed`
is the line an `input` reads; that was the case the gap was found on.

| armed | called as | stops |
|---|---|---|
| a native's own name | that name, or any alias of it | once |
| an alias | the alias | once |
| an alias | the word's own name | not at all |
| an alias and the word it resolves to | the alias | twice, once under each name |

The stop is named by what was armed: `breakpoint: dup`, not the word `dup`
resolves to.

**What stays as it was.** A word in a line typed at a stop is not a stop.
A line that exits at a native's stop keeps the native from running. The cost
with nothing attached is one branch at each native call, as it is at each
frame push. Under a tier, arming has already turned the tier off (D113.6), so
no native is called from compiled code past a breakpoint.

Checked at process level: `input` stopped and fed at its own stop; `dup` and
`ensure_stack` armed together; an alias of a lambda; a condition on
`println`; an exit typed at the stop; a typed line that calls the armed word.

### A terminal in the suite

**Built 2026-10-07, on the owner's instruction.** "No test in the suite holds
a terminal" above is no longer so. `crates/bund2-cli/tests/terminal.rs` opens
a pseudo-terminal and types, on Unix:

- a console attached by `debug.step`: the prompt answers, the up arrow
  recalls and re-runs a line, the history file is written, and the program's
  `input` reads the same terminal after `c`;
- Ctrl-D and Ctrl-C at the prompt each detach, say so, and the program ends
  with 0;
- `password` after a session shows a dot a key and none of the keys;
- `--debugger` at a terminal takes a fed line and answers a typo.

It costs one dev-dependency's features: `rustix`, already in the lock file
through the line editor, asked for its pty calls. They are safe functions.
The child's `HOME` is a scratch directory, so the history written is not the
developer's.

**One thing the test had to learn.** `password` writes its prompt and then
turns the echo off, so keys typed on sight of the prompt are echoed by the
terminal before the word has read anything. A person is slower than that. The
test waits for the terminal's mode and not for the prompt's text. The word
is unchanged.
