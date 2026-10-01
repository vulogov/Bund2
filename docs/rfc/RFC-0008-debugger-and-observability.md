# RFC-0008: The debugger and observability

- Status: **Draft**, revised 2026-09-30 after the first adversarial review
  (`docs/rfc/reviews/RFC-0008-review-2026-09-30.md`). §D1 was blocked on **D6**,
  which the first draft treated as settled — the failure CLAUDE.md names in
  terms, and neither of the two reasons that draft gave for staying a Draft.
- **No longer blocked on a decision — D84 closes D6** (VM-per-task; the
  debugger stops a thread rather than suspending a word). RFC-0003 had placed
  this RFC's subject in D6's scope — "The `--debugger` path is now in D6's
  scope… the observer's interface is not designed here" (RFC-0003:944-946) —
  and D84 answers it. What remains before proposing is **measurement, not
  decision**: §D1's safepoint cost and §D2's frame growth, criteria 11 and 13.
- Depends on: RFC-0003 (the flat frame loop), RFC-0005 (the tier, whose decline
  machinery §D6 uses)
- Decisions consumed: D36, D44, D45, D52, D53, D84
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
2. **`log.error`'s emitted line matches the oracle's** at the default level, and
   the other four emit nothing. `env_logger` writes to stderr, a capture appends
   stderr, and the default filter is `error`
   (`reference/Bund/src/cmd/setloglevel.rs:7`) — so this is checkable, which the
   first draft denied.
3. **`debug.dump` is byte-identical to the oracle** for an INT, a FLOAT, a BOOL
   and a STRING — eight bytes, eight, **one**, and the string's own bytes — and
   all seven `DUMP: error CASTING to …` texts match.
4. **Conformance does not regress**, measured *after* the new probes are
   captured. **Not "stays at 107/116"**: criteria 1–3 add probes, so the
   denominator grows, and the first draft's criterion 3 contradicted its own
   criteria 1, 2 and 4 by demanding a fixed ratio. The claim is that no golden
   that passed before fails after, per golden, and that CEILING moves only by
   deviations this RFC records.
5. **Coverage moves by ten, or the difference is named word by word.** The first
   draft said "moves by exactly ten, or the difference is explained", which
   cannot fail — any outcome is explicable. This version requires the ten words
   to be listed with their state: implemented and probed, implemented and
   unprobed, or not implemented.
6. **`step`, `next` and `finish` agree with an uninterrupted run**, over every
   suite program: stepped to completion leaves the same final state as run
   normally. **Only over the reach D6 permits** — top-level values and bodies
   called directly from them — and the criterion says which programs that
   excludes rather than quietly passing on the ones it can do.
7. **A breakpoint on a word stops before its body runs**, checked by frame depth
   and by the stack being what it was at the call.
8. **A conditional breakpoint cannot change the program it watches.** The
   condition runs in a child VM: a lambda that pushes, switches stack, rebinds a
   word **or calls `bund.exit`** leaves the debugged program untouched and does
   not stop. The exit half is the one that needs the child VM — `exit_code` is
   `get_or_insert` and cannot be cleared.
9. **A watchpoint on `@name` stops on a push to that stack and on no other**,
   and `watch workbench` stops on a workbench push — **two hooks, checked
   separately**, because the workbench path writes no stack tag.
10. **A debugged session runs no compiled code.** Checked by *building with
    `jit`* and asserting `compiled_entries()` reads `Some(0)` after a debugged
    run and `> 0` after the same program run without `--debugger`. The first
    draft asserted only the first half, which §D6 makes true by construction
    and therefore unable to fail.
11. **The safepoint costs nothing when no debugger is attached.**
    D84 traded suspension for a per-step check, and this is the price of that
    trade made checkable: with the check in place, inside RFC-0005 criterion 7's
    5% band, against a baseline taken **before** it exists.

    **`dispatch` is the group that decides this and `startup` is the control.**
    The check runs per step, so it compounds where steps do; `startup` carries
    `registry/register_all` at ~41.8 µs and `parse/mixed` at ~4.3 µs, neither of
    which a per-step cost can reach. A reading that moved `startup` and not
    `dispatch` would be measuring the machine.

12. **§D2's growth is measured against a baseline taken first.**
    `Frame` gains a symbol and a span, on a structure pushed per call, so
    `dispatch` **before** the field is added and then after, inside the same 5%
    band. The first draft named the band and no baseline, leaving nothing to
    compare against.

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
