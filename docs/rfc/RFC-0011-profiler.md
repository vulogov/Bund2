# RFC-0011: A profiler of words

- Status: **Accepted** (2026-10-17) by the repository owner ("accept
  RFC-0011"), with all eight criteria met and each naming the test or the
  reading that decided it, and with D188's amendment built and read
  against criteria 1, 3 and 4 the same day. Accepting means: the unit is
  the word and a body no word names has no node (§S1); the tier is
  declined for a profiled run (D186), and profiling under it stays §S8's
  first item; the two files, their order and the command line's voice are
  D187's; and the report says what its clock cannot show and counts steps
  beside the time (D188).

  Before that: **Proposed** (2026-10-17), by the repository owner's three
  rulings of that date: **D185** (the flag and its operand), **D186** (a
  profiled run declines the tier) and **D187** (the files, the voice and
  the exit code). *(Later the same day, on the owner's "Build RFC-0011":
  **built**, and every criterion has its reading under "Built" below. Two
  things building taught, each recorded where it belongs: the `via` mark
  is the native that filed the body, not only `!` (§S1's note), and the
  `--stats` no-tier line was wrong for a declined tier (F213).)*

  Before that: **Draft** (2026-10-16), revised 2026-10-17 after its first
  adversarial review (`docs/rfc/reviews/RFC-0011-review-2026-10-17.md`),
  whose verdict was "not ready for Proposed": the idea survived every
  finding and the statement of what Bund2 has today did not. Every finding
  is answered in place below and listed in §S10. Written on the repository
  owner's request ("Feature research: Adding profiling into bund2").
- Depends on: RFC-0002 (the word table and the `Vm` seam this document
  widens by two functions), RFC-0003 (the flat frame loop, where a word's
  body runs), RFC-0004 (§S3: a Bund word's effect is inferred, which is
  where §S4's effect column comes from), RFC-0005 (the tier, and the
  decline it already has), RFC-0006 (§B3: a bundle owns all of argv),
  RFC-0008 (§D2 names what a frame carries and what it cannot; §D7 names
  the hook a profiler needs and records that the tier's counters are not
  one; its flag table carries D157 and D185)
- Decisions produced: **D185** (the flag and its required operand),
  **D186** (the tier is declined), **D187** (the two files, `ui::info!`,
  and exit 1 when a successful program's profile cannot be written);
  **D188** (2026-10-17, after the build: the report says what its clock
  cannot show, and counts steps beside the time)
- Decisions consumed: D36 (what is reported goes through the reporter, by
  severity), D49 (a native call is caught where it is made, at three kinds
  of site), D117 (the hot path's cost is read in instructions retired),
  D139 (a compiled body has a frame while it runs, pushed by the tier),
  D145 and D146 (how the tier is offered a body and told not to be), D157
  and D185 (the spelling), D170 (what sampling has already been used for,
  and the only measured figures this host has), D171 (every interpreted
  body starts at the frame push), D173 (the command line's two voices)
- Touched but not consumed: D16 (the open world, which is why §S1's `!`
  node matters), D115 (a bundle is never given a debugger), D151 (D117's
  Linux reading)
- Reference SHA: `reference/Bund` at `21b40b0`, per `reference/PINNED.txt`
- Supersedes: nothing. `docs/research/03-metaprogramming-oop-debugger.md`
  §3.3 says "the tier-up call counters give you a profiler for free";
  RFC-0008 §D7 already records why they do not, and this document builds
  on §D7's correction. `docs/research/02-native-binaries.md:183-184`
  describes a *different* `--profile`, a record of observed type tags for
  profile-guided AOT; D83 withdrew the AOT, D185 takes the spelling for
  this feature, and `docs/research/ERRATA.md` records that since
  2026-10-17.

## Summary

`bund2 script --profile <base>` records, for one run, how long every word
took and which word called which, and writes two files when the program
ends: a markdown report written for a reader who did not see the run,
human or model, and a Graphviz `dot` file of the call graph. The operand
is required (D185). The flag is off by default, costs one branch per call
when off, writes nothing to the program's output, and declines the compiled
tier for the run so that what it measures is one thing.

The nodes of the graph are **words**: the natives Bund2 registers in Rust,
the method natives and conditional runners it calls directly, and the
words a program registers in Bund, under one shape. That is the unit a
Bund programmer thinks in, and it is not what the reference's profiler
counts (§S1).

## Motivation

A Bund program's cost is invisible from outside. `--stats` reports what the
tier did, in totals for the whole run, and on `stderr` for the reason
`print_stats` in `crates/bund2-cli/src/main.rs` gives: every golden captures
`stdout`. The debugger stops and inspects but keeps no clock. D170 is the
one profile the project has taken, macOS `sample` over the VM's Rust
frames on `bund_examples/fibonacci`, and it answers what the *interpreter*
spends, not what the *program* spends. So the only way to learn that a
program spends its time in `format` rather than in the loop around it is to
guess, change the program, and time it again.

RFC-0008 §D7 looked at this and found that a profiler "needs its own
counters on the frame loop, which is the same hook the trace needs", and
that this "is more work than 'for free'". It put `--profile` in the flag
contract and left it unimplemented. D157 then ruled the *reference's*
`--profile` out of scope, on the ground that the reference's table "names
its own Rust functions, so no Bund2 output could match it". D185 takes the
spelling back for a different feature, with a required operand as the line
between the two. This document is the design §D7 said was needed.

## Current behaviour

**The reference.** The flag is `--profile`, a boolean "Execute internal
profiler" (`reference/Bund/src/cmd/mod.rs:136-137`). Before the command
runs it switches on the `time-graph` crate's collection
(`reference/Bund/src/cmd/mod.rs:72-74`), and after the command returns it
prints the collected graph as a table on standard output
(`reference/Bund/src/cmd/mod.rs:106-109`). The crate is
`time-graph = { version="0.3.2", features=["table"] }`
(`reference/Bund/Cargo.toml:70`). Its unit is the Rust function marked
`#[time_graph::instrument]`, which 114 source files under
`reference/Bund/src` and `reference/rust_multistackvm/src` carry. **Most
natives are invisible in it, because the dispatcher is instrumented and
they are not**: `dup` is time inside `i` or `i_direct`. **A handful of
words do appear, under a Rust spelling**: F7 lists `apply`, `i`,
`i_direct`, `call`, `lambda_eval`, `stdlib_execute_base_inline`,
`stdlib_logic_if_base` and `stdlib_logic_times`, and the review found
`format`'s implementing function among the instrumented ones as well. So
the reference's table has a row for `format` and none for `dup`, and the
row it has is named for the Rust function, not the word.

**Bund2 today.** The CLI refuses `--profile` as an unknown argument, and
refuses `--profile=x` the same way, which is D157's option A; RFC-0008's
flag table says so and carries D185's note. Nothing in Bund2 is
instrumented. The pieces a profiler would stand on exist for other
reasons, and the first review corrected three of this section's claims
about them:

- **A registered native is called in one place**, `Interp::invoke` in
  `crates/bund2-interp/src/lib.rs`, reached through `call_native`, which
  already branches once on an `Option` for the effect audit and takes the
  plain path when it is `None`. **Two other kinds of native are called
  outside it** — D49 names them: a method native, which `bund2-stdlib`
  calls directly in `crates/bund2-stdlib/src/oop.rs` at two sites after
  `vm.method(name)` hands it the function; and a conditional runner, called
  directly in `crates/bund2-stdlib/src/conditional.rs` after
  `vm.conditional(ty)`. Each site wraps the call in `catch_panic` itself.
  So there are three kinds of native call site, not one.
- **A Bund word is not called at any of them.** `dispatch`'s `Lambda` arm
  files a tail request and returns; `take_pending` then pushes a frame
  (`push_frame`) carrying `who: Option<Symbol>`, the name RFC-0008 §D2
  added so that a backtrace can say `#1  w at 3`. The frame is popped at
  **three** sites: `run_to`'s arm for a body that ran out of values, on
  success; `unwind_to`, on failure, which pops *every* frame down to a
  floor in one loop and is called from `run_to`'s error arm and from other
  callers; and `push_frame`'s own `truncate` after a tier shadow.
- **`who` is `None` for a body no word names**: a `( … )` context, a LIST
  assembled at run time, a lambda a native handed to `eval_lambda`, and the
  body `tail_lambda` files for `{ … } !` at top reach. RFC-0008 §D2 is where
  that limit is stated, and it is §D2's reading of RFC-0003 §S5 that says a
  source position for such a body needs an IR that does not exist.
- **`!` is the native `execute`**, resolved from the alias at dispatch, and
  it runs a lambda on one of two paths (`crates/bund2-stdlib/src/values.rs`,
  the `LAMBDA` arm): at top reach it files a tail request through
  `tail_lambda`, so the body runs *after* the native has returned, as a
  sibling; nested, it runs the body through `eval_lambda` *inside* the
  native's call, as a child. `times`, `map`, `?try` and every other native
  that runs a body synchronously go through `eval_lambda`.
- **The tier is offered at the frame push.** `push_frame` offers a body
  with a key to the tier (D145) unless the tier has declined it for good
  (D146). **A compiled body's frame is pushed by the tier**, through
  `Vm::tier_frame`, and removed by `push_frame`'s `truncate` when `enter`
  answers (D139); it never passes `push_frame`'s own push. Natives called
  from compiled code go through `jit_call_native` in
  `crates/bund2-jit/src/lower.rs`, not through `invoke`; `dup`, `drop`,
  `swap` and integer pushes are inlined by the lowering and call nothing.
- **The debugger declines the tier** in one place, the `Runtime`
  constructor `script` chooses by `args.debugger` in
  `crates/bund2-cli/src/main.rs`, because "a compiled body runs without
  entering the loop that carries the safepoint" (RFC-0008 §D6). A
  `debug.*` word that arms a console mid-run sets `tier_off` on the
  interpreter instead (D113.6).
- **A bundle owns all of argv** (RFC-0006 §B3): `run_carried` in
  `crates/bund2-cli/src/main.rs` builds its `Args` from the environment,
  `stats: env_set("BUND2_STATS")` among them, and no flag reaches it.
  `script` reads no `BUND2_STATS`; the variable is bundle-only.
- **A run reports on `stderr` or through the reporter, never on `stdout`.**
  `print_stats` says why, and `conform` strips the two `--stats` lines by
  text from what it compares; a third such line "would read as 145
  failures until it too is removed" (RFC-0005's preservation table).
- **The command line has two voices** (D173): a `Diagnostic` through
  `Vm::report`, which is the program's, and `ui::info!`, `ui::warning!`
  and the rest in `crates/bund2-cli/src/ui.rs`, which are the command
  line's own, levelled by `-v` and `-q`. "Running the included …" is an
  `info`, hidden by default.
- **The registry holds no effect for a Bund word and no origin for any
  word.** `Slot` is generation, command, alias, lambda and native; only
  `Native` carries a `StackEffect`, and neither carries a file, a line or a
  package (`crates/bund2-api/src/lib.rs`). A Bund word's effect is
  *inferred* by RFC-0004 §S3's composition, in
  `crates/bund2-stdlib/src/check.rs`.
- **`bund2 build` writes its output staged and renamed, never in place**,
  one file per `place`, after F181.
- **The hot path's cost is a count, not a clock** (D117): instructions
  retired, read with `/usr/bin/time -l` on two release binaries, five
  runs, the median, the empty program subtracted, a 5% band; callgrind's
  count on Linux (D151).

## Design

### §S1 — The unit is the word, and every kind of native is one node

A node is a word as the program names it: a registered native, a method
native, a conditional runner, or a word the program or a loaded file
registered in Bund. Each is recorded under the name dispatch resolved,
with its kind beside it, **and with the spellings the program used**, so
that a program that wrote `!` sixty-nine times reads a report that says
`execute`, spelled `!`. ERRATA records why the resolved name alone
misleads: a survey keyed on `execute` finds nothing in a corpus that never
spells it. The alias table has no multi-hop chains (RFC-0003 §S8), so one
column holds it. A word registered twice under one name is one node, and
the report says a redefinition happened (§S4).

**A body no word names has no node of its own.** That is RFC-0008 §D2's
limit, not a choice here: the frame cannot name it, so neither can the
profile. Its time is charged to the nearest named node open above it. The
one exception is the root: the program's top-level body, which the report
names `⟨program⟩` with the file beside it, because every run has exactly
one and a tree needs a root.

**Where a lambda's callees hang is decided by the path `!` takes**, and the
report says which:

- `{ … } !` at top reach files a tail request. `execute`'s span closes, the
  body's frame is pushed with `who = None`, and its callees are recorded as
  **siblings** of `execute` under the enclosing named node, with the edge
  from that node marked `via !`.
- `[ { … } ] !`, `times`, `map`, `?try`, `if` and every native that runs a
  body through `eval_lambda` run it inside their own call, so the callees
  are **children** of that native and its inclusive time contains them.

Two spellings of one program can draw two graphs; the `via` mark is how a
reader tells them apart.

*(2026-10-17, built: **the mark is `via <native>`, for whichever native
filed the body as a tail request**, not `via !` alone. The first profile
of `fibonacci.bund` drew `via !` on a program with no `!` in it: `if`,
`?true`, `if.stack` and the inline `ifthenelse` file their chosen body
through `tail_lambda` as `!` does — RFC-0003 §S4a's tail position — so
`{ … } if` at top reach hangs its body's calls under the enclosing word
`via if`, and a self-recursive word through `if` carries a self edge
`via if`. `times`, `map`, `?try` and the rest run bodies through
`eval_lambda`, inside their own span. The record keys an edge by
`(caller, callee, via)`, so the two roads to one callee are two lines.)*

### §S2 — Where the clock goes

Three hooks, all behind one `Option` on the interpreter, each taking the
plain path in one branch when it is `None`:

1. **Around a registered native**, in `invoke`. The native's time is the
   duration of its function.
2. **Around a method native and a conditional runner.** Today
   `bund2-stdlib` calls both directly, with its own `catch_panic`. The
   design moves each call behind one function on the `Vm` seam,
   `call_method` and `run_conditional`, which the interpreter implements
   and which carry D49's catch, the effect audit's bookkeeping and this
   hook in one place per kind. That is two functions added to RFC-0002's
   `Vm` trait, and the two call sites in `oop.rs` and the one in
   `conditional.rs` become calls to them. D49's rule, "caught where the
   native is called", is kept; where it is called moves one function down.
   *(Built as written: `Vm::call_method(name) -> Option<Result>` and
   `Vm::run_conditional(ty, c) -> Option<Result>`, `None` when nothing is
   bound to the name, which the three call sites turn into the errors
   they answered before.)*
3. **At the frame push and at every pop.** A Bund word's time is the
   lifetime of the frame pushed for it with `who = Some(w)`. The profiler
   keeps a mirror stack of open nodes, each stamped with the frame depth it
   was opened at, and the interpreter closes it **by depth, not by one**:
   every pop site — `run_to`'s success arm, `unwind_to`, and `push_frame`'s
   truncate — ends with one call that closes every mirror entry deeper than
   `frames.len()`. A mirror that drifted after an `unwind_to` would charge
   the rest of the run to the wrong word, so this is the one invariant the
   implementation owes: every path that changes `frames.len()` calls the
   close. `evaluate_typed`, which runs a console line at a stop with the
   tail request set aside, changes no frame depth it does not restore and
   needs nothing of its own.

Natives are leaves on the same mirror, opened before the call and closed
after it, on success and on failure alike.

Nothing is sampled. The graph needs every call, and a sampling profiler
cannot draw an edge it did not see; D170 shows what sampling is for here,
the interpreter's own frames.

**The arithmetic.** Every activation of a node records one span. A node's
**inclusive** time is the sum of the spans of its activations that are not
nested inside another activation of the same node; its **exclusive** time
is the sum over all its activations of the span less the spans of its
direct children. Exclusive times therefore sum to the root's span, and for
a word `w` whose only child is `c`, `exclusive(w) + inclusive(c)` equals
`inclusive(w)` up to the clock's own cost. An edge carries a call count and
the sum of the callee's spans along it; **a self edge carries the count
alone**, since its spans are the nested activations the inclusive rule
leaves out.

The clock is `std::time::Instant`, monotonic. Its cost per reading on this
host is not known and is not claimed; D170's figures are percentages of a
sampled run and say nothing about it. Criterion 3 measures the cost and the
report prints it beside every table, so a reader can discount the cheapest
words by it. The header also prints each native's `WordKind`, because a
`Blocking` word that waits on input or a bus reads as hot on a wall clock
and is not.

*(2026-10-17, D188: the calibration measures the clock's **resolution**
beside its cost — the smallest non-zero step between consecutive readings,
41 ns on this host against a 16 ns cost — and the report prints both, the
readings taken, what they come to against the wall, and a per-call floor of
two readings and one tick. A per-call figure under the floor is the
clock's, and the report marks it. Beside the clock, each node counts its
**steps**, the values dispatched while it was innermost, under this same
arithmetic and with no clock at all; see the amendment under "Built".)*

### §S3 — Profiling declines the tier

A profiled run installs no tier, through the `Runtime` constructor
`for_debugging` already uses, and for the same shape of reason. A compiled
body has a frame, but it is the tier's frame, pushed by `tier_frame` and
removed by the truncate, and the words inside the body never reach hook 3;
the natives it calls go through `jit_call_native` and never reach hook 1;
and the words the lowering inlines call nothing at all. A profile taken
under the tier would show a hot body as one opaque span with some natives
missing and some present, depending on which the tier chose to inline.
That is a measurement of the tier, which `--stats` already is, and not of
the program.

So the profile is Tier 0's — **D186** — and the report header says
`tier: declined (profiling)`. **Two consequences are stated rather than
discovered**:

- A `debug.*` word that arms a console sets `tier_off` as well, so the
  header's reason is one of two the tier may be off for, and a stop's wall
  time lands in whichever node is open. The report marks a run in which a
  console was attached.
- Where deep recursion fails moves. D145's stated consequence is that
  under the tier the levels of a compiled recursion above the Tier 1 floor
  sit on the Rust stack, and at Tier 0 they sit on the heap. A program near
  either limit can fail under one and not the other. RFC-0008 §D6 accepts
  the same caveat for the debugger, and so does this.

`--jit-threshold` beside `--profile` is accepted and ignored, as it is in a
build without the feature, and the header says `jit-threshold: ignored`.

### §S4 — What is recorded

Per node: the resolved name; the spellings used; the kind (`native`,
`method`, `conditional` or `word`); the `WordKind` for a native; the
effect — **declared** for a native, from its `Native`, and **inferred** for
a Bund word at report time by RFC-0004 §S3's composition over its
registered body, printed as a pair or as `opaque` where the inference
saturates; the call count; inclusive and exclusive time; the minimum and
maximum of one span; and a flag when the name was registered more than
once during the run. Per edge: caller, callee, call count, the callee's
time along the edge, and the `via !` mark.

*(2026-10-17, D188: three more things per node — the number of
**outermost** activations, which is what the inclusive time, the minimum
and the maximum are over, so that a recursive word's nested spans are
counted in `calls` and nowhere else (F214); the **steps**, the values the
interpreter dispatched while the node was the innermost named node open,
exclusive and inclusive under the same arithmetic as the time, one
increment per step and no clock; and, for the run, the node that was
innermost when the last failure was raised. Per edge, the callee's steps
along it. The clock's resolution joins its cost on the record.)*

**Not recorded: where a word was registered.** No slot and no native holds
a file, a line or a package today, and a word's definition site needs the
spans RFC-0008 §D2 says do not exist. Adding a package field to `Native` is
a change to RFC-0002's ABI and is §S8's, not a hedge here.

The record is bounded by the registry plus the program's own words for
nodes and by their pairs for edges. The minimum, the maximum and the
redefinition flag are kept on the hot path, and criterion 3 measures the
on-path cost as one number that includes them.

The record lives in the interpreter crate behind the `Option`; the two
renderers live in the CLI and run on the thread the interpreter lives on,
after the program has ended and before the process exits. For `script` that
is the main thread. The interpreter never formats.

### §S5 — Two renderings of one record

**The markdown** is written for a reader who did not see the run, which is
what makes it usable by a model:

```
# bund2 profile

| program | tier | console | wall | calls | clock cost / reading |
|---|---|---|---|---|---|
| bund_examples/fibonacci/fibonacci.bund | declined (profiling) | none | 1.284 s | 2 104 331 | 23 ns |

## Hottest words, by exclusive time

| word | spelled | kind | effect | calls | exclusive | % of wall | inclusive | % of wall |
|---|---|---|---|---|---|---|---|---|
| `+` | | native | (2, 1) | 832 040 | 0.412 s | 32.1 | 0.412 s | 32.1 |
| `fib` | | word | (1, 1) | 832 040 | 0.301 s | 23.4 | 1.280 s | 99.7 |
| `execute` | `!` | native | opaque | 1 | 0.000 s | 0.0 | 0.000 s | 0.0 |
…

## Call tree (depth ≤ 6, repeated siblings collapsed)

- ⟨program⟩ 1.284 s
  - `fib` ×1 1.280 s
    - `fib` ×2 (recursive)
    - `+` ×1 0.412 s
    - `times` ×1 0.019 s
      - `println` ×35 0.018 s
…

## Anomalies

- `fib` is recursive to depth 31.
- `println`: one call holds 9.8 % of exclusive time (max 0.126 s, mean 0.001 s).
```

*(2026-10-17, D188: the header is two tables. The first adds `ended` —
`normally`, `` `bund.exit` 7 ``, or `error in `+`: reason at file:line:col`
— and the run's `steps`. The second carries the bund2 version, the date,
the clock's cost and resolution, the readings taken, what they come to
and their share of the wall, and the per-call floor, `2 × cost + one
tick`. The hottest table adds `steps`, `% of steps` and `per call`, marks
a per-call figure under the floor *(floor)*, and its kind column names a
native's `WordKind` (F215). The paragraph under the header says in words
that the clock's time is inside the figures and that only calls and steps
are exact.)*

Percentages are of wall time, not of the root's inclusive, and the two
differ by start-up and by time outside any frame; the header gives both so
the difference is visible. Every table has a stable order and every figure
has its percentage beside it, so two reports diff cleanly. The tree is
capped by depth and collapses repeated siblings into a count, both stated
in its heading. The anomalies section is derived, not judged:
self-recursion, a maximum far above the mean, a word called once that
holds most of the time, a word registered more than once, a console
attached.

**The dot file** is the aggregated call graph: one node per word, one edge
per caller-callee pair, cycles drawn as they are.

```
digraph profile {
  rankdir=LR;
  "<program>" [shape=box, label="⟨program⟩\n1.284 s"];
  subgraph cluster_native { label="natives"; "+" [label="+\n832 040 calls\n0.412 s self", fillcolor="#ffb3b3", style=filled]; }
  subgraph cluster_word   { label="words";   "fib" [label="fib\n832 040 calls\n0.301 s self"]; }
  "<program>" -> "fib" [label="1 · 1.280 s"];
  "fib" -> "fib" [label="832 039"];
  "fib" -> "+"   [label="832 040 · 0.412 s"];
}
```

Node fill scales with exclusive share; an edge label is count and time, or
count alone on a self edge. `dot -Tsvg` must accept the file with no
preprocessing.

*(2026-10-17, on the owner's "then items 1 and 2" after D188. **The
markdown is pruned.** Past the first twelve rows, a word under 0.05 % of
the wall, exclusive and inclusive, and of the steps is one entry of a line
after the table — name and calls — so a reader scanning for the hot words
is not made to scan the cold ones and a reader looking for one word still
finds its count; and the callers-and-callees section is only for words
with more than one caller or any callee, since the tree already says
everything about a single-caller leaf. **The dot file is redrawn.** The
two clusters are gone, which let `dot` lay every native in one column;
kind is shape instead — double box the program, rounded box a word,
ellipse a native, octagon a method, diamond a conditional runner — so the
layout follows the calls. A node is labelled with the spelling the program
used and the resolved name beneath, so a program that wrote `dup` finds
`dup`, not `dup_one`. Edge width scales with the callee's time along it.
In a graph of more than twenty nodes, words under 0.5 % of the wall,
inclusive and exclusive, fold into one note node with their edges summed,
so the hot ones can be read. A plaintext legend says all of this in the
file. The sample above is the shape before this note; the code is the
record of the shape after.)*

### §S6 — The flag, the files, and what reaches the terminal

`bund2 script --profile <base> --file …`, or `--profile=<base>`. Off by
default. **The operand is required** — D185: a bare `--profile` is refused
with the message `parse_args` gives any flag missing its operand,
"`--profile` needs a path for the two files; see `bund2 help script`". That
is the reference's shape, a bare boolean, so a command line carried over
from the reference fails as it does today and the refusal says what to type
instead; and nothing is ever written at a place the user did not name. The
two files are `<base>.profile.md` and `<base>.profile.dot`.

*(Before D185 this section gave `<base>` a default, the program's path
with its extension removed. The default is withdrawn: with it, the
reference's bare `--profile` would have written two files nobody asked
for, which is D157's risk turned around.)*

**A bundle reads `BUND2_PROFILE`, holding a base path, and nothing else.**
A bundle owns all of argv (RFC-0006 §B3), so there is no flag to pass
through, and `script` does not read the variable, as it does not read
`BUND2_STATS`. The base is resolved against the working directory of the
run, not against the build-time path `carried.name` holds, which the
deploy host may not have. A bundle's debugger words do nothing (D115); its
profiler does what the variable says, since a profile is a file and not a
session.

**Two files, written in a fixed order — D187.** Each is written as `bund2 build`
writes its artefact, staged beside the target and renamed over it, one
`place` per file; the `dot` file first and the markdown last, so that the
markdown's presence means both are there, and a failure between them
leaves the `dot` file and no markdown. A reader who finds one file knows
which half failed.

**What reaches the terminal.** Writing the two files is the command line
reporting on its own work, so it speaks in D173's second voice: one
`ui::info!` line naming both paths, hidden by default and shown at `-v`.
It is not a `Diagnostic`: the program raised nothing. A failure to write is
`ui::error!`, shown at every level including `-q`. **Nothing goes to
`stdout`**, and nothing goes to `stderr` at the default level on a
successful run, so a profiled run's `stdout` and `stderr` both equal an
unprofiled run's, which is what criterion 4 checks. **The exit code after
a write failure is D187's**: when the program itself succeeded and its
profile could not be written, the process exits 1, the `ExitCode::FAILURE`
that `bund2 build` answers when `place` cannot write; when the program
failed, it keeps its own code, since that is the more important fact and
D36 says a program's failure exits 0. The `ui::error!` line is what says
the report is missing.

**The spelling.** `--profile` is the natural word and it is the reference's
flag, which D157 ruled out of scope on the ground that the reference's
table could never be matched. This flag matches nothing of the reference's
by design: different unit, different output, different destination.
**D185** takes the spelling and supersedes D157 for it alone, with the
required operand as the line between the two. The research's
`--profile prog.prof` for guard ordering is a third meaning, withdrawn with
the AOT; ERRATA says so, and if it ever returns it needs another name.

`script` is the only command that takes the flag. `bund2 build --check`
runs no program; `build` and `check` have nothing to profile.

### §S7 — What this does not do

- It does not name a body no word names, except the root (§S1, RFC-0008
  §D2).
- It does not profile a compiled run (§S3). The tier is declined.
- It does not profile a thread other than the main VM's. The interpreter is
  not `Send` (RFC-0007 §C8), so a bus or async host runs its own VM, and
  that VM's record is its own. Merging is §S8's.
- It does not trace. RFC-0008 §D7's trace is a stream per step; this is an
  aggregate per word. They share the frame-loop hook and nothing else.
- It does not change what `script` does. A program that is profiled runs,
  and fails, as it does without the flag, with the tier's absence the one
  difference, observable in `--stats` and, for a program near a recursion
  limit, in where it fails (§S3).

### §S8 — Later, named so it is not discovered later

- Hook `jit_call_native` and `tier_frame`, so a compiled run can be
  profiled at the native and body level, with the inlined words still
  absent and the header saying so.
- A package field on `Native` and a span on a registered word, so §S4 can
  say where a word came from. The first is RFC-0002's ABI; the second is
  RFC-0008 §D2's missing IR.
- Merge worker VMs' records into the main report, one section per VM.
- A `?profile` or `debug.profile` word that writes the record mid-run.
- A per-site record, word *at* a call site, once per-value spans exist.
- Read the clock less: time a native's call every Nth time once it has
  been seen K times, and scale, with the figure marked estimated and the
  count and the steps still exact. D188's option C; **D189**, OPEN, puts
  four options for a ruling and recommends sampling only the natives
  whose measured cost is already under the clock's floor.

### §S9 — Open questions

See the section of that name at the end; §S9 is kept as the pointer the
first draft's text refers to.

### §S10 — The first review, answered

| Finding | Answer |
|---|---|
| B1: the RFC contradicted D185 | It did, on the review's reading of the file before the amendment of the same day; the header, §S6, the preservation table and question 1 carry D185. |
| B2: natives are called at three kinds of site | §S2's hook 2 and the `Vm` seam; "Current behaviour" says three. |
| B3: the registry holds no effect for a Bund word and no origin | §S4: declared for a native, inferred at report time for a word; origin not recorded, §S8. |
| S1: "no Bund word is a row" | Reworded with F7's list and `format`. |
| S2: `!` is a node; where its lambda's callees hang varies | §S1, per path, with the `via !` mark. |
| S3: the resolved name alone misleads | §S1 and §S4: the spellings used are recorded beside it. |
| S4: a bundle owns argv | §S6: `BUND2_PROFILE` only, resolved against the run's directory. |
| S5: the failure pop is `unwind_to`, and there are three pop sites | §S2: the mirror closes by depth at every site. |
| S6: a compiled body's frame is the tier's | §S3 and "Current behaviour" corrected; the conclusion stands. |
| S7: the recursion rule left the sample undefined | §S2's arithmetic; the self edge carries a count; the root is `⟨program⟩`. |
| S8: D16 misattributed | RFC-0008 §D2 cited; D16 moved to "touched". |
| S9: the Notice took one of D173's voices silently | §S6: `ui::info!`, hidden by default; `ui::error!` on failure. |
| Criteria 1 to 8 | Reworded below with the method, the tool and the path. |
| Preservation rows the table did not enumerate | Added. |
| ERRATA for research 02 | Recorded 2026-10-17. |

## Preservation analysis

| Current behaviour | Disposition |
|---|---|
| The reference's `--profile` prints a table of its own Rust functions with `println!` (`reference/Bund/src/cmd/mod.rs:106-109`) | **Deliberately changed — D185.** Different unit, two files, nothing on standard output. |
| The CLI refuses `--profile` as an unknown argument (D157) | **Preserved for the reference's shape, changed for Bund2's — D185.** A bare `--profile` is still refused, naming the operand; `--profile <base>` runs the profiler. |
| `script` writes only the program's output to `stdout`; the tier's own lines go to `stderr` or the reporter | **Preserved.** The profiler writes files and one `info` line hidden by default. Criterion 4. |
| A registered native is called in `Interp::invoke`, under D49's catch | **Preserved.** Hook 1 brackets the call and does not move it. |
| A method native and a conditional runner are called directly from `bund2-stdlib`, each under its own `catch_panic` (D49) | **Deliberately changed in place, not in meaning.** The calls move behind `Vm::call_method` and `Vm::run_conditional`; the catch, the message and the error are the same. RFC-0002's `Vm` trait gains two functions. |
| A frame is pushed per call and carries `who` (RFC-0008 §D2) | **Preserved.** The profiler reads `who` and adds no field to the frame. Criterion 3 measures the branch. |
| A frame is popped at three sites, `unwind_to` popping to a floor | **Preserved, and followed.** Each site closes the mirror by depth; a mirror that drifted would charge the rest of the run to the wrong word, and criterion 8 exercises the `unwind_to` path. |
| The tier is offered at the frame push (D145) and a debugged run installs none (RFC-0008 §D6) | **Preserved, and reused.** A profiled run installs none the same way — D186. |
| Under the tier, deep compiled recursion sits on the Rust stack; at Tier 0 on the heap (D145) | **Preserved**, with its consequence stated: a profiled program near a limit may fail where an unprofiled one does not (§S3). |
| A `debug.*` word that arms a console sets `tier_off` (D113.6) | **Preserved.** The header marks a console; the tier is off for two reasons at once. |
| `--jit-threshold` is accepted and ignored where there is no tier | **Preserved**, and the same beside `--profile`. |
| A program that spelled `!` dispatched `execute` | **Preserved**, and the report says both names. |
| A bundle owns all of argv and reads `BUND2_*` switches (RFC-0006 §B3) | **Preserved.** `BUND2_PROFILE` joins them, holding a base path. |
| `bund2 build` writes one file staged and renamed, never in place | **Preserved**, applied twice in a fixed order (D187). |
| A failing program exits 0, as the reference's does; `bund.exit` and a failure to start set a code (D36, `bund2 help script`) | **Preserved.** A write failure after a *successful* program exits 1, `build`'s code for the same failure; a failed program keeps its code — D187. |
| The command line's two voices (D173): `-q` drops `Warning` and nothing else of the reporter's | **Preserved.** The profiler's line is `ui::info!`, not a `Diagnostic`, so `-q` and the default level never show it. |
| The hot path's cost is a count (D117) | **Preserved.** Criterion 3 is read that way. |
| A bundle is never given a debugger (D115) | **Preserved.** A profile is not a session. |

## Alternatives considered

- **Use the `time-graph` crate as the reference does.** Rejected: its unit
  is a Rust function marked by attribute, so a word would be a row only if
  every native were marked, and a Bund word could never be one. It would
  also print a table that invites comparison with the reference's and could
  never match it, which is what D157 objected to.
- **Derive the profile from the tier's counters.** Rejected by RFC-0008 §D7
  already: they are totals on the tier, exist only in `jit` builds, and read
  zero whenever the tier is declined.
- **Sample instead of instrument.** Rejected: a sampled run cannot draw the
  call graph the request asks for, and Bund's frames are heap frames the
  host's sampler sees as the interpreter's loop. D170 is what sampling is
  used for here, and its numbers are about the VM, not the program.
- **Leave method natives and conditional runners where they are and hook
  them from `bund2-stdlib`.** Rejected: the profiler's mechanism would leak
  into every direct call site, and a fourth site added later would miss it.
  One function per kind on the `Vm` seam is where D49's catch already
  wanted to be.
- **Profile under the tier and mark the gaps.** Rejected for version one
  (§S3): the gaps depend on what the tier chose to inline, so two runs of
  one program would show different natives. Kept as §S8's first item.
- **One file, markdown with the dot embedded.** Rejected: a reviewer pipes
  the dot file to `dot` and pastes the markdown to a model, and both are
  easier when each is its own file.
- **Print the report on `stderr`.** Rejected: a report of hundreds of lines
  on `stderr` is unusable at a terminal and `conform` would have to strip
  it by text, which the preservation table of RFC-0005 already warns about.
- **A `Notice` for "wrote two files".** Rejected after the first review: it
  would print under `-q`, which drops only `Warning`, and it would reach a
  TUI's reporter as if the program had said it. The command line's own
  voice is `ui::info!`.

## Acceptance criteria

1. **Off is off.** With the flag absent, `cargo xtask conform` reads the
   same N/M and CEILING it reads today in the three configurations the
   verify script runs: default, `--features jit`, and `--features jit
   --jit-threshold 1`.
2. **The files exist and parse.** A profiled run of
   `bund_examples/fibonacci/fibonacci.bund` writes `<base>.profile.md` and
   `<base>.profile.dot`. A unit test in `bund2-cli` checks the markdown
   opens with §S5's header table and that the dot file is one `digraph`
   with balanced braces and every edge naming two declared nodes; on a host
   that has `dot`, the same test also runs `dot -Tsvg` on it and requires
   exit 0, and says it skipped that half otherwise.
3. **The off path stays inside D117's band.** Instructions retired, read as
   D117 says — `/usr/bin/time -l` on two release binaries, five runs each,
   the median, the empty program subtracted — for the four programs D117's
   table names, with the profiler compiled in and the flag absent, within
   5% of the binary before this change. Recorded under D186 when it is
   taken. The on-path cost per reading is measured by the
   same method with the flag present, divided by the call count the report
   gives, and printed in the header; it is a number, not a bound.
4. **`stdout` and `stderr` are unchanged.** `cargo xtask conform` gains a
   switch that adds `--profile <scratch>` to every run it makes, and with
   it N/M and CEILING equal criterion 1's. `conform` already compares
   `stdout` with `stderr` appended, so an unexpected line in either is a
   failure, and the programs it runs are the ones whose output reproduces,
   `tests/golden/UNSTABLE.txt` excluded.
5. **The graph is the graph.** For `:inc { 1 + } register 5 inc inc` the
   record has exactly one node of kind `word`, `inc`; nodes of kind
   `native` for `+` and `register` and no other natives on the edge from
   `inc`; the edge `inc → +` with count 2; and `inc`'s inclusive time not
   less than `+`'s. A unit test over the record, before rendering.
6. **Recursion adds up.** For a word that calls itself to depth 20 and
   does one `+` per level, the sum of every node's exclusive time is not
   greater than wall time, and `exclusive(w) + inclusive(+)` equals
   `inclusive(w)` within twenty-one readings' worth of the measured clock
   cost. A unit test over the record.
7. **The tier is declined.** Under `--features jit`, a profiled run's
   `--stats` reports no compiled bodies and no inlined sites, and the
   report header says `tier: declined (profiling)`.
8. **A failing program still writes, and the mirror did not drift.** A
   program that fails forty words deep has its profile written, with the
   failing word's node present, every node closed, and the exclusive times
   summing to not more than wall. The same for `bund.exit` (D52) and for
   D171's OOM trip, both of which end the run through the error path; a
   `SIGINT` writes nothing, since the process ends before the renderers
   run.

### Built, 2026-10-17

The record is `crates/bund2-interp/src/profile.rs`, the hooks are in
`crates/bund2-interp/src/lib.rs` (`call_native`, `push_frame`,
`profile_frames_popped` at the three pop sites, `tail_lambda`,
`call_method`, `run_conditional`, `register_lambda`, the two console
attachments), the seam is in `crates/bund2-api/src/lib.rs`,
`crates/bund2-cli/src/profile.rs` holds the renderers and `write_both`,
`crates/bund2-cli/src/main.rs` holds the flag and `finish_profile`,
and `cargo xtask conform --profile` is the switch criterion 4 names. The
reading of each criterion:

1. **Met.** With the flag absent the verify script of 2026-10-17 read
   `CONFORMANCE 135/147 (+12 approved deviations)`, `CEILING 135/147`, in
   all three configurations: default, `--features jit`, and
   `--features jit --jit-threshold 1`. The same numbers it read before
   the change.
2. **Met.** `profile_needs_a_base_and_then_writes_two_files_and_nothing_else`
   in `crates/bund2-cli/tests/cli_ux.rs` checks the header table, the
   `digraph`, balanced braces and that every edge names a declared node,
   and runs `dot -Tsvg` where the host has it; this host has it, and it
   accepted the file for `fibonacci.bund` and for the test's program.
3. **Met.** D186's "Built" has the table: +1.66% to +2.11% on the five
   programs, flag absent, inside the band; 893 to 1,116 instructions a
   call on the on path; the clock at 15 to 21 ns a reading, printed.
4. **Met.** `cargo xtask conform --profile` adds `--profile <scratch>` to
   every source run and `BUND2_PROFILE` to every bundle run, and read
   `CONFORMANCE 135/147 (+12 approved deviations)`, `CEILING 135/147` on
   the default build and on `--features jit`; under `jit` it asks for no
   `--stats` line and takes no tier total, since D186 makes the run Tier 0
   by design, and says so. `conform` compares `stdout` with `stderr`
   appended, so the equal numbers are the check that the profiler adds a
   line to neither.
5. **Met.** `the_graph_is_the_graph` in `crates/bund2-stdlib/tests/profile.rs`,
   over the record: one node of kind `word`, the edge `inc → +` with count
   2, `register` reached from the root and not from `inc`, and §S2's
   identity `exclusive(inc) + inclusive(+) = inclusive(inc)` to the
   microsecond.
6. **Met.** `recursion_adds_up` and `a_self_edge_carries_the_count_alone`
   in the same file: exclusive times sum to the wall to the microsecond,
   a word recursive to depth 21 has inclusive time at most the wall and
   `max_depth` 21, its recursion is a self edge `via if` with a count and
   no time, and direct recursion is a self edge with no `via`.
7. **Met.** D186's "Built": on a `jit` release binary the profiled run's
   `--stats` reads `no tier (declined: profiling)`, with nothing
   compiled and nothing inlined, and the header says so.
8. **Met.** `a_failing_program_leaves_a_closed_record` fails forty-one
   words deep and finds the mirror empty, the failing word's node present
   with 41 calls, every inclusive time at most the wall and the exclusive
   times summing to it; `a_self_edge_carries_the_count_alone` ends through
   `bund.exit` with the same checks; a `SIGINT` is not tested, the process
   being gone.

### Amended 2026-10-17 — the effect column says why a word is opaque

On the owner's "item 3": `bund2_stdlib::check::infer_explained` answers
the effect `infer` answers and, for an opaque body, the reason the walk
gave up *in that body*, in the words `bund2 check` reports. It shares
`Walker::infer_in_why` with `infer`, so the two cannot disagree; before
this, `infer_in` dropped the reason when it restored the caller's. The
effect column prints `opaque: <reason>` for a word, `opaque: recursive,
<reason>` when the blocking term is the word itself, and `opaque,
declared` for a native whose registration says so. On fibonacci, `fib`
reads `opaque: recursive, `fib`, whose effect is not a fixed pair`.
`an_opaque_body_says_why` in `crates/bund2-stdlib/src/check.rs` and the
process test check the three forms. Renderer and inference only; the
record and the hot path are untouched.

### Amended 2026-10-17 — D188: the clock's honesty, and steps beside the time

Two profiles taken on the build above showed what the report could not
say. On this host `fibonacci.bund` runs in 0.48 s unprofiled and 1.06 s
profiled; its record took 43,080,600 readings at 12 to 16 ns each, which
is the whole difference, and every native in `fib` measured 20 to 25 ns
a call, under what two readings cost. The report printed the cost per
reading and left the reader to multiply. Every small span was also a
multiple of 41.7 ns, the 24 MHz tick on Apple Silicon, which the
throughput calibration does not see. The owner's ruling, D188, takes
option A there:

- **The clock is in the figures, and the report says so.** `calibrate`
  answers a `Clock` of cost and resolution, the resolution being the
  smallest non-zero step between consecutive readings. The header prints
  both, the readings taken, their product against the wall, and a floor
  of two readings and one tick; the hottest table prints a per-call
  figure and marks one under the floor; an anomaly line names each word
  under the floor that still holds 1 % of the wall, and says the share is
  the clock's. Nothing is subtracted: a corrected figure would read as
  measured when it is modelled (D188's option B).
- **Steps are exact.** `run_to` and the top-level loop count one step
  before each `apply_step` when a record is kept; the record charges it
  to the innermost named node and carries exclusive and inclusive steps
  per node and steps per edge under §S2's arithmetic. A native's steps
  are zero, since it dispatches nothing; the step that named it is its
  caller's. The same program counts the same on every host, which no time
  in the report can claim, and `steps_are_exact_and_add_up` asserts the
  counts to the step where the time tests assert bounds.
- **F214 and F215.** `min`, `max` and the max-against-mean anomaly are
  over outermost activations, counted in `Node::outer`; the kind column
  carries a native's `WordKind`.
- **`ended`.** The first header cell after the program says how the run
  ended — normally, by `bund.exit` with its code, by an uncaught error,
  or by a parse error that ran nothing — and, for a failure, names the
  word that was innermost when it was raised: the native that answered
  the error, or the word whose body named something that could not be
  dispatched. The record marks the failure at the three native call
  sites, in `run_to`'s error arm and at the top level, keeping the inner
  mark of one failure seen at several levels and replacing it on a later
  failure, the rule D173's traceback uses; so a failure `?try` caught and
  a later one that ended the run leave the later one named.
- **Version and date** in the second header table, so a report read
  apart from its run says which bund2 wrote it and when.

Criterion 3 was read again under D188: every program inside D117's band,
+0.05 % to +1.01 % on the off path, and 695 to 709 instructions a call on
the on path with the step counter in. Criteria 1 and 4: `conform` and
`conform --profile` read the same numbers as before.

**Later the same day, on the owner's "then items 1 and 2"**: the markdown
pruning and the dot redraw described under §S5's note. Renderer only; the
record, the hooks and the hot path are untouched, so criterion 3 was not
read again. The process test checks the pruned section's heading, that a
single-caller leaf has no section, and the legend in the graph; `dot`
itself was not on the host that built this, so the test's `dot -Tsvg`
half skipped there, as it says it does. *(Graphviz was then found at
`/opt/homebrew/bin/dot`, and rendering the redrawn file showed a
spelled node's label printing a literal `\n`, `dup\n(dup_one)`: the name
was escaped after the line break went in, so the break's own backslash
was escaped. Names are now escaped first; one call reads `1 call`; the
process test's program spells `dup` and asserts the break, and its
`dot -Tsvg` half runs on this host and passes.
`bund_examples/fibonacci/fib.profile.png` is re-rendered from the new
file.)*
*(Then, on the owner's "pin the legend to a corner": the legend is no
longer a node, which ranking placed wherever it fitted, but the graph's
own label with `labelloc="b"` and `labeljust="l"`, which `dot` puts in
the bottom-left corner whatever the layout. The example graph is
re-rendered again.)*

## Open questions

1. **D157's status.** *Ruled 2026-10-17 — **D185**, option C.* Its
   ruling was "a reference flag Bund2 does not have", and this RFC reuses
   the spelling for a different feature. Three options were put: keep D157
   and spell the flag differently; supersede it and take `--profile` with a
   default base; supersede it and take `--profile` with a required operand.
   The third was recommended and taken: the reference's bare `--profile` is
   still refused, so its risk does not become a silent write.
2. **Profiling declines the tier (§S3).** *Ruled 2026-10-17 — **D186**,
   option A.* Three options were put: decline the tier for the run;
   profile under it and mark the gaps; run twice under one flag. The first
   was recommended and taken; the second is §S8's first item.
3. **Files and the command line's voice (§S6).** *Ruled 2026-10-17 —
   **D187**.* The extension pair `.profile.md` and `.profile.dot`; the
   `dot` file first; `ui::info!` on success and `ui::error!` on failure;
   and, for the exit code after a write failure, the third of three
   options: exit 1 only when the program itself succeeded, the program's
   own code otherwise.

All three questions are ruled, each by its own entry, as D96 did for
RFC-0010.
