# RFC-0005: Tier 1 — the Cranelift backend

- Status: **Proposed** (2026-09-12, on the owner's authorisation — D59), after
  twenty-one adversarial reviews. Drafted 2026-09-08, revised 2026-09-09,
  2026-09-10, 2026-09-11 and 2026-09-12.
  **The gate is answered on both halves and §S2–§S11 are authorised to be
  built.** D43's `bund2-api` additions — the registration id and the stable
  per-name generation cells — were explicitly held until this status, and are
  now unblocked.

  **Proposed rather than Accepted, for the reason RFC-0001 gives.** Most of the
  thirty acceptance criteria still **cannot run**, because the code they
  describe does not exist — but the reason has narrowed, and this paragraph is
  kept current because it is the first thing a reader checks the RFC against.
  **As of 2026-09-13 `crates/bund2-jit` is no longer a placeholder**: it holds
  a lowering from BundIR to CLIF, §S8's four-piece call boundary, and seventeen
  tests under `--features jit`. What does not exist is a *tier* — no cache, no
  `Interp` integration, no compiled Bund word, no promotion and no meaning
  guard — and that is what the unrunnable criteria are waiting for.

  What the criteria section marks **Met** is **1, 11, 24, 25, 28 and 29**, with
  **19** met at the model level; **2** has run on both tiers and reads the same;
  **16**'s third leg runs, which it could not before a lowering existed; and
  **8** and **15** are recorded as passing today rather than as met, since each
  re-decides on every run. **4** is a special case worth naming here: its
  relocation check cannot run against a `JITModule` at all, because only
  `cranelift-object` exposes relocations, so it belongs to RFC-0006's AOT path.

  RFC-0000's bar for Accepted is a review pass that finds nothing — its own came
  after four, "the fourth is the last that found anything" — and this RFC's
  twenty-first pass still found two blockers. Both are answered since, but **no
  pass has yet found nothing**, so Accepted would be a claim the record does not
  support.
- The gate, and how it was answered:
  `docs/research/00-jit-feasibility.md` §2.2 sets a hard gate — "Project B is
  worth doing only if Project A's measurements show that dispatch and boxing
  are still the bottleneck". When this was drafted the gate did **not** pass:
  the dominant cost was one function in the value layer. **D41 changed that**,
  and §S1's update records it — push/pull 126.9 → 9.8 ns, a 3.8–4.2×
  improvement per program shape.

  **Half the gate is met and half is not, and this RFC does not claim
  otherwise.** The 20 ns prerequisite is met. The second — that dispatch is now
  the dominant term — **cannot be shown with these benchmarks**, because none
  of them separates dispatching a word from the work the word does once
  dispatched (§S1). Criterion 10 makes that an experiment that can fail rather
  than a claim.

  **Corrected 2026-09-12 — the paragraph above is superseded, and kept because
  the reasoning it records is why the second half took four days to answer.**
  What could not be done was separating dispatch from work *by subtracting*
  benchmarks. `crates/bund2-bench/benches/fragment.rs` constructs the
  separation instead, in one harness over one program, and a word decomposes
  **83.8%** dispatch-plus-value-traffic on `Int + Int` and **88.6%** on
  `dup drop`, leaving 11–17% for the addition and the pop themselves — the
  study's condition, over both its terms. `dispatch_isolated`, added the same
  day, measures dispatch outright rather than bounding it: **22.44 ns** a word
  with the work held at exactly zero. Absolutes are one machine's; the shares
  moved under half a point across two runs, and they are what the gate turns
  on (§S1, *Update, 2026-09-12*).

  So §S2–§S11 are authorised to be built. Two correctness problems stood
  between this RFC and Proposed and both have mechanisms. **§S8's frame
  consumption** now has one — a stack floor Bund2 measures in Rust and
  compiled code compares against its own stack pointer (§S8, *How the guard
  reads the stack*), read by the seventh and eighth reviews.
  And **§S6's inlining freezes a name** unless every inlined site re-checks
  what the name means — the fifth review's B1, answered in §S6 by a per-site
  meaning guard that criteria 5 and 17 test, and reviewed since the sixth.

  The sixth review's B1 — **no body's `Rc` reached the point where it starts
  running** — was the owner's, and is decided and built: D42 carries the value
  to every entry point, with dated amendments to RFC-0002 and RFC-0003. D43
  decides the `bund2-api` additions §S6's guards need, to be built when this
  RFC reaches Proposed.

  The seventh review's two blockers are answered. **B1** was the owner's:
  D44 decides that the level at which evaluation reports stack exhaustion is
  not part of a program's meaning, so §S8 now promises that evaluation never
  aborts and that Tier 0 never has less room with the tier on. It no longer
  promises that the level is the same. **B2** was this RFC's: §S5's pre-call
  generation check now applies §S6's rules, so nothing stays promoted across a
  call through an alias. That review's S1 went to the owner too. D45 makes the
  reporter answer `wants_stack` per severity, which lets values be promoted
  across calls under the CLI's default reporter. D45 is built.

  The eighth review's two blockers are answered. **B1** was the owner's: a
  lambda's effect is inferred from slots the pre-call check cannot pin, so
  D46 keeps nothing promoted across a call that resolves to a lambda. **B2**
  was a Bund2 defect: `execute.` declared a fixed effect while running
  whatever it was handed (F87, fixed). Criterion 24 now checks every native
  for that mechanically, and criterion 25 checks that no native reports at
  `Error` severity.

  The ninth review's three blockers are answered. The owner chose each answer
  among options the review set out. **B1**: a call can leave a body for
  the loop to run after it returns. Compiled code now drains that request after
  every call, or hands it to its caller in tail position (§S5, *A call may leave
  a body to run*). **B2**: `object` ran a class's `.init` under a fixed effect,
  and `display` ran a `fmt` runner (F91). Criterion 24 is now an audit of every
  native the corpus calls, and its first run found ten more declared pairs that
  miscount the stack (F92). **B3**: promotion crosses only the natives
  `bund2-stdlib` registered (D47).

  The ninth review's significant items are answered too, again by the owner's
  choice among the review's options:
  - **S1:** callees are classified by `Registry::resolve`, and `effect_of` now
    follows it (F93).
  - **S2:** the tier adds nothing to a Tier 0 level below the Tier 1 floor,
    and Tier 0's part carries a measured margin (§S8).
  - **S3:** `bund2` declares a Tier 1 share, and criterion 2 must see compiled
    bodies.
  - **S4:** the effect audit also catches a native reporting at `Error`.

  The tenth review's two blockers are answered, both by the owner's choice
  among the review's options. **B1**: most declared pairs had been checked only
  on the operands the corpus passes, and `drop_stack`'s was wrong (F94). A
  palette audit now runs every fixed-effect native against fourteen operand
  kinds and the workbench, and promotion crosses only the natives it brought to
  `Ok`, listed in `tests/golden/PROMOTABLE.txt` (D48, criterion 28). **B2**:
  a `NativeFn` cannot sit in a call slot. §S8 now specifies the boundary: a
  context pointer and a status under `CallConv::Tail`, a Rust adapter per
  native, `Tail` thunks and an entry trampoline. A panic in a native is caught
  where the native is called, in both tiers (D49), which fixes F95. Its
  significant items are answered in §S5 and §S8: D48's id set, a margin over
  every re-entering path, and the request protocol's four edges, one of them a
  Tier 0 defect now fixed (F96).

  The eleventh review's blocker is answered, with its significant items.
  **B1**: D52 makes `bund.exit` record a code and return `Ok`, and Tier 0
  stops at its next step; compiled code, which reads three cells after a call
  and none of them an exit, would have run on. A recorded exit now becomes the
  error status at the call that made it, and the entry trampoline refuses to
  start a body (§S5, *A call may end the program*; criterion 30). **S1**:
  §S8's re-entering paths are a derived set rather than a list, and
  `eval_source` is a path of its own; Tier 0's exhaustion message now names
  `bund.eval` and `use`. **S2–S4**: five stale sentences are corrected, every
  figure is re-derived on 2026-09-11 beside its command, criterion 28 states
  the palette's six unrun natives and that its list certifies the default
  registration, and §S7 cites *Q22 (cache)*.

  The twelfth review's blocker is answered, with its significant items.
  **B1**: the exit rule named the adapter and the resolving trampoline, and
  the drain helper, a boundary but not a call, was one step further on: a
  body ending in `bund.exit` returns `Ok` from `run_to`. Now every helper that
  returns to compiled code makes its status through one function that consults
  `exit_requested` first, so a helper added later cannot forget it (§S5,
  *A call may end the program*; criterion 30 gains four cases held below the
  threshold). **S1**: the re-entering set is found by a test,
  `every_reentering_function_is_named`, which adds `!` and `execute` by name,
  `apply`, `?.` and `?MOVE`; Tier 0's message names calls by name. **S2–S4**:
  fifteen operand kinds, criterion 28's list certifies more than a pair since
  D55, line 669's stop count is refreshed, the stops did not move with F111,
  and assumptions 22–23 state the audit's skipped tuples and operand
  positions.

  The thirteenth review's blocker is answered by fixing Tier 0, as the owner
  chose. **B1**: `status_of` handed the error status to the Rust code that
  started a compiled body, where Tier 0 answered `Ok`, so `map` collected and
  `input*` read another line after an exit. That was a Tier 0 defect against
  D52, now F112. `Vm::eval_lambda`, `Interp::apply` and `Vm::scoped_call`
  consult the gate after `run_to`, so both tiers hand a native the same `Err`.
  The owner ruled that the state after an exit is meaning (D52's dated note).
  Criterion 30 gains the mirror cases, and a Preservation row names them.
  **S1**: §S5's three sentences are corrected. **S2**: the re-entry scan now
  reads path-form calls, qualified heads and subdirectories, and assumption
  24 states that `m` covers `bund2-stdlib` only. **S3**: `status_of` clears a
  pending request when it answers an exit. `CollectingReporter` has its file,
  and the named-stack row credits `move_from`.

  The fourteenth review's blocker is answered with one condition. **B1**:
  `status_of` replaced a helper's `Err` with the bare refusal, where Tier 0
  passes a native's wrapped error up. `?try` keeps that text on the final
  stack. Now `status_of` substitutes only for `Ok`, and an error passes
  through unchanged (§S5, assumption 26, a new Preservation row).
  **S1–S2**: a Tier 0 test runs `?try` through `map`, `times`, `bund.eval` and
  `context`. It pins F112's two untested gates, and F112 records the text
  change. Criterion 30's `input*` case names its harness, and
  `input_loop_reads_no_line_after_an_exit` runs it against the binary. The
  criterion's `?try` case says what it compares. **S3**: D52 has a further
  dated note, and the scan's test-module limit is assumption 27.

  The fifteenth review's blocker is answered with one more condition, in the
  same function. **B1**: three of the four helpers reach a native through
  `Interp::dispatch` and so inherit F96's clearing of a failed native's tail
  request. The adapter calls the `NativeFn` directly and does not.
  `execute_value` reached that from `bund2-stdlib` until F113 was fixed; the
  case is an embedder's native again. `status_of` now
  clears the request cell whenever it answers an error, not only an exit (§S5;
  criterion 26 gains the program; the Preservation row names the adapter; F96
  has a dated note). **S1**: `#` catches a refusal and discards it, so the
  "only native that catches" claim is scoped to natives that then do work, in
  all four places. **S2**: criterion 30's drained-body and residual-path cases
  have programs, comparisons and a Tier 0 test, and the `?try` case says what
  its test covers. **S3**: F112's note has a correction, assumption 26 gives
  the reason the texts actually agree, and Tier 0's request overwriting is
  filed as F113, fixed the same day against the oracle, with assumption 29.

  The sixteenth review's blocker is F113's own fix, which did not reach as far
  as the sentences written on the strength of it. **B1**: the first fix tested
  whether the *item* was a lambda, so a lambda held in a dict inside the list
  still filed a tail request, and two such items still lost the first. The fix
  now turns on **reach**: a lambda any container reaches runs at once, as the
  reference does
  (`reference/rust_multistackvm/src/stdlib/execute.rs:93-95`), and a lambda
  the program executes itself is filed, which is a deviation — the reference
  runs that one at once too, having no request mechanism. §S5's "a request
  means *run this before the next value*" is why the two orderings agree, and
  RFC-0003's frame loop (§S4) is why filing costs no Rust frame. §S5,
  assumption 29, criterion 26 and this line are corrected, and both register
  entries carry it.
  **S1–S4**: §S8's prose named 19 of the scan's 21 paths and now names `csv`
  and `sqlite`; it also records that `execute_reached` has two re-entry costs
  (assumption 30); assumption 28 lists every
  arm; and `execute.rs:93-95` is spelled in full, since `cite` cannot see a
  bare filename. Assumptions 31–33 record what the review found unstated.

  The seventeenth review found two blockers, one of them a live abort.
  **B1**: assumption 33 named one writer of the request cell where the code
  had three, so it could not enforce what it was added for, and it left
  undecided whether `status_of`'s clear reaches Tier 0's `pending_tail` or
  only the compiled mirror — mirror-only would leave the fifteenth review's
  blocker standing. The owner's answer is D56: `bund2-api` gains
  `Vm::clear_tail_request`, since `Interp::invoke` is private and a compiled
  call never reaches it. §S6's *Addressing* names every writer, assumption 33
  states the property, and `every_writer_of_the_request_cell_is_named`
  enforces it. **B2**: §S8's reserve invariant was falsified by measurement.
  `list 20000 { drop list push } times !` aborted the process, about 470
  levels past `STACK_RESERVE` at roughly 550 bytes a level. That is F114,
  fixed by driving `execute_reached`'s traversal from a heap worklist, so a
  value's depth costs no stack. The same measurement found two more aborts on
  paths §S8's floors do not reach: F115, dropping a deep value, fixed the same
  way with an iterative `Drop`, and F116, parsing a deep run-time string, fixed
  with a parser bound (`MAX_NESTING`, 1024). Assumptions 34 and 35 record both. **S1–S4**: `execute.rs:93-95` is cited for the reached half only,
  and the filed half is stated as the deviation it is; criterion 11's case is
  a number again; `PROMOTABLE.txt`'s header states its own arithmetic; and
  D55's status records the regeneration.

  The eighteenth review found two blockers. **B1**: §S4 required a compiled
  `$name` call to key on the reference's one-level alias answer. The premise
  about the reference is right, but Bund2 resolves to a fixed point for every
  spelling (`Registry::follow`), which is RFC-0002's approved deviation,
  asserted by its criterion 6a. Built as written, compiled `$a` would have
  reached `b` where Tier 0 reaches `dup_one` — a meaning difference §S2
  forbids, which would have moved conformance and un-met an accepted criterion
  of another RFC under `--features jit`. §S4, the `$name` slot rule and the
  Preservation row now key on the fixed point and record the reference's
  behaviour as the deviation it is. **B2**: assumption 36 called the unbounded
  renderer unmeasured; it aborts at 24,000 levels through
  `debug.display_stack`, a word the golden capture epilogue runs. That is
  F117, fixed with an iterative renderer whose output is byte-identical. The
  set is **six, not four**: the nineteenth review found `display` recursing as
  well (F119, fixed the same way) and the wire codec recursing on the way to
  `save.model` (F118, which aborted at 3,400 levels). F118 is the first of the
  six whose recursion is partly a dependency's, so it took an arena for
  Bund2's half and a 256-level bound on writing for bincode's; D39's dated
  notes state the rule all six share and the third clause F118 added to it. **S1–S4**: F116's status reads FIXED in all three places; assumption
  35 no longer claims `PartialEq`, `Hash` and a wire format recurse, since two
  bottom out at `identity()` and the third does not exist; §S8 and §S5 name
  `execute_one`, which is where the re-entries live, and count its path as two
  frames; and assumption 33 states the three blind spots of its derivation.

  The nineteenth review's answer is folded into the paragraph above, and the
  **twentieth review had no entry here at all** until now — a whole review
  missing from the ledger, which its successor caught (the twenty-first's S1).
  Both are recorded together.

  The twentieth review found two blockers. **B2 is answered here**: §S6 and
  D43 said the generation mirror is written "by the same `touch()`", which
  could not be built — `Slot::touch` took `&mut self` alone, with no `Symbol`
  and no `Registry`, so it could not find the cell. `touch` now lives on
  `Registry` as `touch(&mut self, s: Symbol)`, the one function that bumps a
  generation, with `Slot::bump` private to it, so the mirror write has one home
  beside the bump. That is the structural answer CLAUDE.md prefers, and it
  makes the set one function rather than six. Assumption 37 states the
  property, `every_writer_of_a_slot_generation_is_named` derives it, and D43
  carries a dated note. **B1 is open and is the owner's**: the codec's decode
  side still aborts, and the twenty-first review measured it through a path
  the write-side bound does not reach. Of its significant items, **S2**: a
  `Slot` holds four live bindings, not six — `class` and `method` are declared
  and never written, filed as F121, with class and method resolution living in
  `Registry::classes`/`methods` under their own counters. **S3**: assumption
  33 gains its fourth blind spot, the exact-match head parser. **S4**:
  `follow` follows 65 links, not 64. **Minors**: criterion 11's bound must
  stay under 2 ns rather than inviting reconsideration, and criterion 12's
  re-derivation needs `grep -rl`. **S5** (a resume index per call site) and
  **S6** (the workbench as a promotion source) are answered below, as
  assumptions 38 and 39.

  The twenty-first review found the same two blockers standing. **B1** is
  measured and worse than recorded: `sqlite` decodes every BLOB in any SQLite
  file a program names, through the same `from_binary`, and a 174 KB BLOB
  nested 3,000 deep aborts the process — reproduced here at depths 10, 256 and
  1,000 decoding cleanly and 3,000 aborting. D31 cannot justify that path: it
  governs world files, which have been redb since D27, so a SQLite database is
  never one. The Preservation row's three reasons are also wrong — `run_sqlite`
  *does* re-enter evaluation, `sqlite` is `eff(1, 1)` and not in
  `ACTS_ON_HOST`, and `load.model` is skipped by the palette for being opaque.
  The disposition is the owner's and the row is not rewritten until it is
  taken. **Figures**: criterion 28 now reads 222 promotable of 230 reached of
  267; §S6's loop re-derives 137 sites over **190** programs; the Status date
  line and the assumptions preamble are current.

  **B1 is answered**, by the owner's disposition rather than by this RFC:
  D58 records a **stated exception to D37** for bytes read from a file Bund2
  did not write — bincode builds the nested value before any depth check could
  run, so there is no point at which one could refuse — and `sqlite` refuses a
  BLOB over `MAX_BLOB_BYTES`, 16 KiB, which is the write-side bound read
  through size at a measured 58 bytes a level. The Preservation row is split in
  two: the codec on a value Bund2 wrote, bounded at write, and the codec on
  bytes it did not, bounded by length. Its three wrong reasons are corrected —
  `run_sqlite` does re-enter evaluation, `sqlite` is `eff(1, 1)` and not in
  `ACTS_ON_HOST`, and `load.model` is skipped by the palette for being opaque.
  Assumption 35 now separates the encode check from the decode sites.

  **Criterion 10 has a first measurement**, from a throwaway lowering outside
  this RFC's gate (2026-09-11, branch `spike/lowering-1`). With §S8's call
  boundary on every word and nothing inlined, `1 2 + drop` compiled runs
  **1.93×** faster than Tier 0, against the criterion's floor of 1.2×. The
  question this Status line opened with — whether the tier can earn its keep —
  now has evidence on the side of yes. The criterion itself stays open until
  the tier as shipped is measured.

  **The gate's second prerequisite is answered, 2026-09-12** — the half this
  Status line has called unsettled since 2026-09-08. §S1 said the split
  between dispatch and a word's own work could not be made by these
  benchmarks, and that is true of *subtracting* them; `fragment.rs`
  constructs it instead, in one harness over one program, and says so in its
  preamble. Decomposed over two runs, a word is **83.8%**
  dispatch-plus-value-traffic on `Int + Int` and **88.6%** on `dup drop`
  (83.5% and 88.2% on the first run), leaving 11–17% for the addition and the
  pop themselves — which is the study's condition, over both terms as it
  states it. The second run was 5–8% faster on every arm and moved those
  shares by under half a point, which is the case for quoting shares and not
  absolutes. **A new benchmark measures dispatch outright**: `dispatch_isolated`
  runs an empty native body 1000 times through the eval loop and then through
  its own pointer, giving **22.44 ns** of loop-plus-resolution-plus-dispatch
  per word with the work held at zero — conservative, since `black_box`
  inflates the floor it is measured against. What carries the gate is still
  the measured floor rather than the constructed shares: `promoted` is
  8.7–9.5 ns, within noise of one push and pull. Prerequisite 1 re-confirmed
  at 9.53 ns against its 20 ns bar. The crossing noted on criterion 10 —
  arithmetic's inlining ceiling reading 1.77× and 1.81× here against 2.03×
  before — is machine-dependent and recorded as such.

  **The seam is filled and a hot body now runs compiled, 2026-09-13.**
  `bund2-runtime` owns the `Interp` and installs a `JitTier` holding the
  `Tiering`; a body past the threshold is compiled, filed, and run compiled on
  the next entry. It is the only crate naming both tiers, which is what keeps
  Tier 1 optional by structure. **conform is 106/114 on both tiers with a tier
  actually installed** — the first run where compiled bodies could execute
  during the corpus, and §S2's one invariant says that number must not move.
  The recursion guard is the part worth stating: the seam takes the tier out
  while it runs, so a body entered during compiled execution interprets, and a
  self-recursive word therefore spends no Rust frame per Bund level. A test
  recurses 2,000 levels with the tier installed. **Criterion 23 remains
  unmet** — the cache is per-`Interp` now, but each lowering still builds its
  own `JITModule`. Four runtime tests; thirty-five in `bund2-jit`.

  **The compiled cache and the promotion counter are built, 2026-09-13, and
  criteria 3 and 6 run.** `Tiering` (`crates/bund2-jit/src/cache.rs`) holds two
  maps over the same bodies, keyed on `payload_key` and each holding a
  `payload_weak` — so a dead entry answers for nothing, which is the false hit
  §S3 calls the worst failure available here, and neither map pins a body. All
  four of §S7's knobs are configurable, as that section requires and criterion 6
  depends on: it sets the function cap to 4, the recompile cap to 2 and the
  counter cap to 8 rather than exercising a thousand bodies. The caps behave
  oppositely on purpose — the function cap **refuses**, since code memory is
  never reclaimed, while the counter cap **evicts** coldest-first — and the
  recompile cap counts per slot while demoting the body live at the time, on the
  owner's decision. §S7 gains a dated note for the one thing it never said: what
  calls the sweep. Still absent, and the reason this is not yet a tier: nothing
  consults the cache when a word runs, and **criterion 23 is not met** — it
  wants one cache and one `JITModule` per `Interp`, where each lowering builds
  its own module. Thirty-five tests under `--features jit`; conform unmoved at
  106/114.

  **Dated note, 2026-09-13 — criterion 23 is met but for its cells.** The seam
  consults the cache at `Interp::push_frame`, and `lower`'s `Compiler` holds
  one `JITModule` per tier, hence per `Interp`; the cache holds handles into it
  rather than code. Two tests run the criterion: two tiers over one body share
  nothing, and a dropped `Runtime` leaves another still giving Tier 0's result.
  §S6's cells remain unbuilt, so that noun of the criterion is absent rather
  than per-`Interp`. Thirty-six tests in `bund2-jit` and six in `bund2-runtime`
  under `--features jit`; conform unmoved at 106/114.

  **A compiled body is a real Bund word, 2026-09-13 — and it is a shape, not a
  speedup.** `compile_word_body` lowers a body's values, each through its own
  `Tail` thunk called indirectly through the slot table, and a differential
  asserts the compiled word leaves the same stack as `Interp::eval` over the
  same body — `{ 1 2 + }`, `4 4 + 2 *`, a lambda call that files and drains a
  tail request, a failure that stops the body. Every value goes through
  `Vm::apply`, which *is* Tier 0's path, so the body reproduces Tier 0 exactly
  rather than approximately. That is forced, not chosen: **§S4's step 3** says a
  `CALL` under `autoadd` is not a call at all, guarding it wants §S6's `autoadd`
  cell, and `autoadd` is not readable through the `Vm` trait — so a test pins
  that the compiled word and Tier 0 agree under `autoadd` instead. The cost is
  that this is slower than interpreting; what it buys is the structure §S6's
  fragments are inlined into, and **criterion 10 measures the tier as shipped,
  not this**. Still absent: the cache, `Interp` integration, promotion, inlining
  and the meaning guard — nothing consults a compiled body when a word runs, and
  `compile_word_body` has no caller outside tests. Twenty-seven tests under
  `--features jit`; conform unmoved at 106/114 on both tiers.

  **F96's parity gap is closed, 2026-09-13, and the request cell is not what
  closed it.** The adapter now calls `Vm::clear_tail_request` when the native it
  ran answers an error, which is what D56 put on the trait for a caller that
  cannot reach the private `Interp::invoke` — its first consumer. Two tests:
  Tier 0's F96 test through compiled code, and a **positive control** showing a
  *succeeding* native's filed body still runs, because an adapter that cleared
  unconditionally would pass the first while discarding every tail request a
  compiled call ever filed. **§S5's request cell is still not built**, and this
  entry says so rather than letting the closed gap imply otherwise: the cell is
  the mirror compiled code loads after every call to decide whether to drain,
  and nothing reads it — no drain helper, no `status_of`, no epoch or `autoadd`
  cell, no body that continues past a call. §S5 also assigns this clearing to
  `status_of`; until that helper exists it sits in the adapter. Nineteen tests
  under `--features jit`; conform unmoved at 106/114 on both tiers.

  **Dated note, 2026-09-13 (2) — `status_of` exists, and the request cell has
  its first reader.** The clearing has moved out of both adapters into one
  helper, as this entry said it would. `status_of` consults
  `Vm::exit_requested` first, substitutes `Error::exited` after an `Ok`, passes
  an `Err` through unchanged, and clears the tail request on every error it
  answers — through `Vm::clear_tail_request`, which is one of the four writers
  that write §S6's mirror beside `pending_tail`. So the sentence above is
  superseded in one respect: the cell is now read and written on the compiled
  path, rather than only mirrored. **The other half stands** — no compiled body
  continues past a call, so nothing *loads* the cell to decide whether to
  drain. Forty-one tests under `--features jit`; conform unmoved at 106/114.

  **§S8's boundary is complete, and criterion 29 is Met, 2026-09-13.** The
  per-native adapter (piece 2) and a `Tail` thunk per native (piece 3) now have
  a call site: a compiled body that loads each thunk's address from a slot table
  and calls through it, indirectly, with `return_call_indirect` for a last call
  in tail position. A panicking native called from that body gives **Tier 0's
  exact text** in both tail and non-tail positions, which is criterion 29's
  compiled half and D49's claim that the two tiers agree. Seventeen tests under
  `--features jit`; conform unmoved at 106/114 on both tiers. **Two things are
  recorded rather than claimed**: criterion 4's relocation check cannot run
  against a `JITModule` at all — only `cranelift-object` exposes relocations, so
  it is RFC-0006's — and the adapter does not yet clear a tail request on
  failure as `Interp::invoke` does (F96), because §S5's request cell does not
  exist. That is the boundary's outstanding debt.

  **§S8's boundary has its first two pieces, 2026-09-13.** The arm now carries
  `CallConv::Tail` and Rust enters compiled code only through a JIT-emitted
  entry trampoline under the platform's convention — pieces 1 and 4. The
  C-convention entry the previous day's commit used as a stand-in is gone, and
  the transmute points at the trampoline rather than the arm, since calling a
  `Tail` function from Rust is the one thing §S8 rules out. Pieces 2 and 3, the
  per-native adapter and its `Tail` thunks, wait on a compiled body with a call
  site, and criterion 29 waits with them. **A correction came out of building
  it**: §S8's s390x degradation has no trigger at the pinned Cranelift — every
  backend lowers `return_call` at 0.135.0, and `supports_tail_calls` is a
  property of the convention rather than the target — so the lowering emits
  `Tail` unconditionally instead of carrying a branch no target can take. The
  first draft did carry one, on a constant `true`. conform holds 106/114 on both
  tiers.

  **The first lowering is built, 2026-09-12, and criterion 16's third leg now
  runs.** `crates/bund2-jit/src/lower.rs` compiles one fragment's ops to
  machine code and runs it against a real `dyn Vm`, and its tests compare that
  code against `frag::run` over the criterion's own boundaries — both operand
  orders, the `i64::MAX` wrap, `dup`'s fresh identity, a declined guard
  touching nothing. The leg the criterion called owed and unwritable "before
  one exists" is therefore closed, with a `norm` property test and a mutation
  check standing behind it so the agreement is not vacuous. Three of §S6's
  claims now have running code rather than a design: the fragment
  representation has a consumer, the register file costs nothing at run time,
  and an error travels in the context. **It is not §S8's boundary** — the entry
  is C-convention, not `Tail` — and §S6 says so where the lowering is
  described. conform holds 106/114 on both tiers, which is the invariant a tier
  must not move.

  **RFC-0005 is Proposed as of 2026-09-12, and §S2–§S11 are authorised to be
  built** — the owner's decision on the gate's answer, recorded as D59. That
  unblocks D43's `bund2-api` additions, which were held until this status, and
  it ends the sequencing question this Status line opened with on 2026-09-08.
  It does not pre-empt criterion 10: the stop rule still governs the lowering
  as shipped, and the note on that criterion records that arithmetic's
  inlining ceiling reads under 2× on this machine. Accepted remains out of
  reach until a review pass finds nothing and the criteria that need a
  `bund2-jit` can run.

  **The twentieth review's S5 and S6 are answered, 2026-09-12.** S5: the
  residual path's obligation on the lowering is stated where the path is
  described and as assumption 38 — a compiled body carries a map from each
  call site to the source-body index at which interpretation resumes, and no
  folded or inlined region spans a call site — with criterion 21 asserting the
  side table rather than only the stacks. S6: §S6 now says the workbench is
  never a promotion source, and assumption 39 states why it cannot be — a
  workbench operand was never in a `Variable`, and the guard reads the current
  stack alone (`crates/bund2-interp/src/frag.rs`, `frag::run`). **One of S6's
  figures does not reproduce.** It counts 59 `.`-suffixed natives on
  `PROMOTABLE.txt`; the file today holds 222 non-comment entries of which
  **52** end in `.`, and none end in `,`, the palette's other spelling for a
  workbench form (`crates/bund2-stdlib/src/lib.rs`,
  `every_fixed_effect_native_keeps_its_pair_over_the_promotable_palette`).
  The list has been both larger and smaller — criterion 28 records a
  227-entry reading on 2026-09-11 — so whether 59 was right when written is
  not decidable from here. 52 is what the file says, and no part of the
  design rests on the count.
- Depends on: RFC-0001 (the value, whose representation §S1 indicts),
  RFC-0002 (`StackEffect`, the word slot table, and the open world that forces
  indirect calls), RFC-0003 (BundIR as a cache over a body, and the frame
  loop), RFC-0004 (declared and inferred effects, which order guards)
- Decisions consumed: **D3** (eval'd code is structurally unable to hit the
  cache, so no tier rule is needed), **D5** (lambda bodies are write-once, so
  no invalidation), **D9** as amended 2026-09-09 (**no Cranelift type may
  appear in `bund2-stdlib` or `bund2-api`**; the original sentence governs the
  stable ABI, not where Bund2's own lowerings live), **D10** (a C toolchain is permitted to `bund2 build`, not below
  it), **D12** (the `*` fold family is a permanent optimisation barrier),
  **D16** (dispatch by computed name — the open world), **D20**
  (materialisation points), **D32** as amended 2026-09-10 on Q35's answer (`q` is
  kept on every value and **not** averaged by arithmetic; §S6's constraint 2
  rests on it),
  **D33** (**RESOLVED** 2026-09-11 on option 2 — ordering across int and float
  is exact, which §S6 consumes), **D35** (the cache keys on the body's `Rc`
  pointer; as amended 2026-09-10 on Q32, it holds a `Weak`), **D42** (a body's
  `Rc` reaches the point where it starts running), **D43** (the registration
  id and stable generation cells §S6's guards need), **D44** (the level at
  which stack exhaustion is reported is not meaning), **D45** (a reporter
  wants a stack snapshot per severity), **D46** (nothing stays promoted
  across a call that resolves to a lambda), **D47** (promotion crosses only
  the natives `bund2-stdlib` registered), **D48** (and only those a palette
  audit has brought to `Ok`), **D52** (`bund.exit` is a request the
  embedder honours; compiled code must see it at the call that made it — §S5,
  *A call may end the program*), **D55** (the words that read beyond their
  arity are found by audit and kept off `PROMOTABLE.txt`, so they are
  barriers — Q34's answer; criterion 14), **D49** (a panic in a native is caught where the
  native is called, in both tiers), **D37** (no panic), **D39** (an
  internal loop must be bounded)
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: `00-jit-feasibility.md` §2.1's expectation table, whose rows are
  replaced by measurement in §S1. The document's own framing anticipated this —
  "order-of-magnitude reasoning, to be replaced by measurement in Phase 0" — so
  this is the substitution it asked for, not a contradiction. Recorded in
  `docs/research/ERRATA.md`.

## Summary

Tier 1 compiles a lambda body to native code through Cranelift, behind a
`cargo` feature, keyed on the body's `Rc` pointer, entered only through
runtime-owned indirect slots, and specialised by guard-and-branch with no
deoptimisation. It changes speed and not meaning: `cargo xtask conform` must
move by **exactly zero**.

That is the design. §S1's measurement no longer says "not yet" on performance
grounds — D41 removed that objection. Its two correctness mechanisms, §S8's
stack floor and §S6's meaning guard, are designed and have been reviewed: the
meaning guard since the sixth review, the floor since the seventh. Only Tier
0's half of the floor is built. Each review has found the next level of one
question, what a promoted value's callee can do, and the RFC stays Draft until
a review finds none.

## Motivation

Bund2 is already **5.5× faster than the reference** over the corpus — 153.9 ms
against 845.2 ms for 57 programs, measured through `cargo xtask bench` with
both targets in release (`docs/registers/open-questions.md`, Q14). That factor is startup: the fastest program fell from
13.7 ms to 2.3 ms, which is what D28's dependency cut bought.

So the motivation for a JIT is not "Bund2 is slow relative to the oracle". It
is that interpretation itself is slow in absolute terms, and the question this
RFC has to answer first is *which part* of it.

## Current behaviour

There is no Tier 1 *code*: `crates/bund2-jit/src/lib.rs` is a doc comment and
lint configuration — twelve lines — and `crates/bund2-ir/src/lib.rs` now
declares `pub mod fragment` and re-exports `Fragment`, `Guard` and `Op`: the
representation §S6 describes, and no lowering.

**The wiring, however, already exists**, and an earlier draft said it did not.
Cranelift is pinned at `=0.135.0` in the workspace and declared as optional
dependencies of `bund2-jit` behind `jit` and `aot` features
— the `jit` and `aot` features in `crates/bund2-jit/Cargo.toml`, which
`crates/bund2-runtime/Cargo.toml` re-exports. So the feature gate §S10 requires
is built; what is missing is everything it would gate.

Tier 0 is what runs: `Interp::eval` walks a `Vec<BundValue>` and dispatches
each `CALL` through the slot table.

---

# S1. The gate, measured — half met

`00-jit-feasibility.md` §2.2 divides the work in two and puts a decision gate
between them:

> Project A — representation. […] Delivers the majority of the achievable
> performance. […] Project B — Cranelift tier. […] Project B is worth doing
> only if Project A's measurements show that dispatch and boxing are still the
> bottleneck.

and warns what happens if the gate is skipped:

> A JIT that emits `call stdlib_add_inline` in sequence is a slower, more
> fragile version of the interpreter.

`crates/bund2-bench` now measures this in process. Medians, this machine,
release with debug info:

| benchmark | median | **per group** |
|---|---|---|
| `dispatch/literal_push/w2000` — `1 drop` ×1000 | 188 µs | **188 ns** |
| `dispatch/dup_drop/w3000` — `1 dup drop` ×1000 | 366 µs | **366 ns** |
| `dispatch/native_call/w4000` — `1 2 + drop` ×1000 | 446 µs | **446 ns** |

**Per *group*, not per word.** An earlier draft divided by word count and then
compared the result against `value/push_pull/balanced`, which produced an
impossibility: a whole average word read 112 ns while a single push/pull round
trip read 126.9. A word is not one push/pull — `1 2 + drop` is four words but
three pushes and three pulls — and the two benchmarks do not share a harness,
so the units were never commensurable. **Only differences within the
`dispatch/*` family are meaningful**, and this table is restated in the unit
that supports them.

What the value-layer benchmarks below do establish is the cost of the
operations themselves, which is a claim about `with_tag` and not about any
split between dispatch and the value layer:

| benchmark | median |
|---|---|
| `value/clone/scalar` — clone a `BundValue::Int` | **4.1 ns** |
| `value/with_tag/scalar`\* — one `with_tag("stack", …)` | **73.1 ns** |
| `value/push_pull/balanced` — one push and one pull through `Vm` | **126.9 ns** |

\* **No benchmark of that name exists today.** `crates/bund2-bench` has
`value/with_tag/scalar_unique` — the production case, a freshly boxed scalar
with one holder — and `value/with_tag/heap_shared`. Whether 73.1 ns came from
one of them under an earlier name cannot now be established: the bench crate is
not yet under version control, so it has no history to consult. The figure is
kept because this subsection records the state that motivated the gate; it is
not a figure anything later rests on.
**Corrected 2026-09-12:** the bench crate *is* under version control — seven
tracked files, `benches/fragment.rs` among them. What remains true is that it
was not tracked when 73.1 ns was recorded, so there is still no history from
that date to consult, and the figure stays unattributable rather than wrong.

**`with_tag` was 58% of a push/pull round trip and roughly 60–75% of an average
word.** It was not incidental. `Stack::push_as` did not store the value it was
handed, it stored `v.with_tag("stack", name)` — and `with_tag`, *as it stood
then*, boxed the scalar, materialised its identity, cloned the entire
`HeapValue` including its `BTreeMap` of tags, inserted two freshly allocated
`String`s, and wrapped the result in a new `Rc`. Four allocations and a map
clone, per value, per push. Cloning the same value cost 4.1 ns; tagging it cost
73.1.

**No line is cited for that, deliberately: the code no longer exists.** D41 and
RFC-0001's Q25 amendment rewrote it, and `crates/bund2-value/src/lib.rs`
today takes `Rc<str>` parameters — no `String` allocation — and mints and clones
only on the shared arm, which the production path does not take. An earlier
draft cited that line for the sentence above, which `cargo xtask cite` passed
because the line resolves; it checks that a citation points somewhere, not that
the prose describes what is there. **This whole subsection is a record of the
state that motivated the gate**, and it is kept in the past tense for that
reason. The current numbers are in the update below.

That tag is not decoration — a value's `tags` carry `stack: <name>` and the
reference's own values do too, which is why goldens capture it. The cost is in
*how* it is written, not *that* it is written.

### What this means for this RFC

A Cranelift tier lowers the interpreter's dispatch, and the question is how
much of a run that is.

**An earlier draft answered "at most a quarter" and derived an Amdahl bound of
about 1.3×. That figure is withdrawn.** It rested on the unit error above, and
its inputs were superseded by D41 the same week. It is not replaced with a
corrected number, because **these benchmarks cannot separate dispatch from the
work a word does once dispatched** — removing dispatch would not remove `+`'s
addition or `drop`'s pop, and nothing here measures that split. Criterion 10
makes it an experiment rather than an estimate.

What survives without arithmetic: `with_tag` cost 73.1 ns against a 4.1 ns
clone, on a path that runs for every value the interpreter touches. That is a
representation cost, it is large, and it is not something a code generator
addresses. The study's own precondition — that the representation work be done
first — was therefore not met.

**So the gate's answer is: not yet.** Concretely, the following must land
before §S2–§S11 are worth implementing:

1. **`push` must stop reallocating.** The stack tag is a property of *where a
   value is*, and it is being stored *in* the value. Candidate fixes — a tag
   written only when observed, a stack-name interning so the insert is a `u32`,
   or moving the tag to the stack's own bookkeeping — are RFC-0001's to weigh,
   not this RFC's. The requirement here is a number: `value/push_pull/balanced`
   under **20 ns**.
2. **Re-run the gate.** With push cheap, `dispatch/*` re-measured tells us
   whether dispatch has become the bottleneck. If an average word is then
   dominated by the dispatch loop, Project B is justified and this RFC's design
   sections apply unchanged.

This is not a rejection of Tier 1. It is the sequencing the study asked for,
with the measurement it said to take.

### Update, 2026-09-08 — prerequisite 1 met, prerequisite 2 not settled

Prerequisite 1 is satisfied; prerequisite 2 is not settled (below). **D41** moved the stack tag into the
value's existing padding as an interned symbol, and RFC-0001's Q25 amendment
records the chain:

Per group, the unit the `dispatch/*` family supports:

| program | baseline | **now** | |
|---|---|---|---|
| `1 drop` | 188 ns | **46.9 ns** | 4.0× |
| `1 dup drop` | 366 ns | **95.8 ns** | 3.8× |
| `1 2 + drop` | 446 ns | **105.2 ns** | 4.2× |
| `value/push_pull/balanced` | 126.9 ns | **9.8 ns** | 12.9× |

**Prerequisite 1 is met**: it asked for under 20 ns and `push_pull/balanced` is
9.8.

**Prerequisite 2 is not settled, and this update no longer claims it is.** It
asked whether dispatch has become the bottleneck. Differencing *within* the
family — the only subtraction these numbers support — gives:

| | cost |
|---|---|
| a literal push, `dispatch/literal_only/w1000` — **no dispatch at all** | **13.2 ns** |
| `drop` = `1 drop` − `1` | 33.7 ns |
| `dup` = `1 dup drop` − `1 drop` | 48.9 ns |
| `+` = `1 2 + drop` − `1 drop` − `1` | 45.1 ns |

A literal push runs no word and costs 13.2 ns; a `drop` adds 33.7 for a
dispatch plus a `VecDeque::pop_back`. So dispatch is **at most** 33.7 ns and
plainly the larger term in a word — but "at most" is the honest quantifier,
because nothing here separates the dispatch from the pop, or from `+`'s
addition. That separation is criterion 10.

`dispatch/literal_only/w1000` exists because of this: it is the non-dispatch path
exactly, and it replaced a `nl`-based benchmark that was meant to isolate
dispatch and in fact measured stdout at ~450 ns per word.

Conformance did not move across D41 — 73/86, ceiling 79/86, before and after
— and `BundValue` is still 16 bytes (a test in `crates/bund2-value/src/lib.rs`
asserts `size_of::<BundValue>() == 16`). Conformance has since reached its ceiling,
**105/113** on 2026-09-11, through Tier 0 work unrelated to this RFC
(criterion 2). It read 79/86 until three probes were added on 2026-09-10,
82/89 until a fourth, an approved deviation (D50), and 82/90 until that day's
words and probes.

So §S2–§S11 are worth designing — and, since 2026-09-12, worth building: the
owner authorised them on this section's answer to the gate, and the RFC is
**Proposed** (D59). The Status line says what Proposed does and does not
claim.

### Update, 2026-09-12 — prerequisite 2 is answered, by construction

**The separation this section calls impossible is impossible *by subtraction*,
and `crates/bund2-bench/benches/fragment.rs` does not subtract.** Its own
preamble says so — "§S1 says dispatch and work cannot be separated by
subtracting benchmarks; here they are not subtracted but constructed, in one
harness against one program" — and its four arms are one program with
successive costs removed rather than two programs differenced. That harness
postdates the paragraph above, which is why the paragraph was not wrong when
written and is superseded now.

**Absolutes are this machine's; the gate turns on the shares.** Two runs of
`cargo bench -p bund2-bench --bench fragment` on 2026-09-12, medians from
Criterion's own `estimates.json` for 1000 operations, divided by 1000. The
second run came in 5–8% faster across every arm — Criterion reported
"Performance has improved" against its own baseline, which is machine state
and not a change in Bund2 — and **the shares below moved by less than half a
percentage point**. That is the whole reason a share is quotable here and an
absolute is not:

| | `tier0` | `inlined` | `lowered` | `promoted` |
|---|---|---|---|---|
| `Int + Int` | 54.81 ns | 37.61 ns | 30.28 ns | 8.90 ns |
| `dup drop` | 76.64 ns | 27.83 ns | 21.45 ns | 8.74 ns |

The first run read 56.57 / 38.94 / 32.02 / 9.33 and 80.05 / 28.99 / 22.89 /
9.46 for the same eight cells.

Reading the columns as a decomposition of one word — dispatch is what
`tier0 → lowered` removes, value traffic is what `lowered → promoted` removes,
and what remains is the work itself:

| | dispatch | value traffic | the work | dispatch + value |
|---|---|---|---|---|
| `Int + Int` | 44.8% | 39.0% | **16.2%** | **83.8%** |
| `dup drop` | 72.0% | 16.6% | **11.4%** | **88.6%** |

The first run put the last column at 83.5% and 88.2%.

**That is prerequisite 2's answer.** The study asked whether "dispatch and
boxing are still the bottleneck" — one condition over both, not two
conditions — and on both shapes they are 83–89% of a word, with the actual
addition and the actual stack pop accounting for 11–17%.

**Dispatch also has a measured figure now, not only a share.**
`dispatch_isolated` was added on 2026-09-12 for exactly this: it registers a
native whose body is empty and runs it 1000 times two ways, so the two arms
differ in their dispatch and in nothing else.

| arm | per word | what it is |
|---|---|---|
| `resolved/w1000` | 31.33 ns | the eval loop walks the stream, resolves each name through the registry, dispatches |
| `direct/w1000` | 8.89 ns | the same `NativeFn` through its pointer — the body alone |
| difference | **22.44 ns** | loop step + name resolution + dispatch, **work held at zero** |

This is what the `dispatch/*` family could only bound. It is a *measurement*
of dispatch rather than a ceiling, and it is smaller than the bound — 22.4
against ~27.9 — because the bound had a `VecDeque::pop_back` folded into it.
**One caveat, stated because it cuts against the number:** `direct` wraps the
call in `black_box`, which forces a pointer reload per iteration, so 8.89 ns
overstates the floor and the 22.44 ns difference is therefore conservative —
real dispatch is that or more, not that or less.

**What carries the claim is the residual, not the shares.** `lowered` and
`promoted` are hand-written Rust standing in for perfect lowerings, so each
share is an *upper* bound on what a real lowering could recover — the wrong
direction for "dispatch dominates" on its own. The measured floor is
`promoted`: whatever Tier 1 does, 8.7–9.5 ns of these words survives it across
the two runs. That floor is small, it is measured rather than constructed, and
it is what makes the decomposition load-bearing. Note too that it sits within
noise of `value/push_pull/balanced` at 9.53 ns, which is the same claim from
the other side: a promoted word costs about one push and pull.

The `dispatch/*` family, re-run the same day, agrees on the bound it can
support: a literal alone is 15.9 ns, `1 drop` is 43.8, `1 dup drop` 85.1 and
`1 2 + drop` 95.5, so a `drop`'s dispatch-and-pop is ~27.9 ns. That is still
"at most", and still coarser than the two tables above.

**Prerequisite 1 re-confirmed**: `value/push_pull/balanced` reads **9.53 ns**
against the 20 ns requirement. (It has read 9.47, 9.53, 9.8 and 10.1 across
runs and machines; the requirement is met by a wide enough margin that the
spread does not matter.)

**One caution for anyone re-running this.** `target/criterion/` keeps
directories for benchmarks that no longer exist — `dispatch/dup_drop_1000`,
`dispatch/native_call_1000` and `dispatch/nop_nl_w1000`, the last being the
`nl` benchmark that measured stdout. They are stale artefacts of removed
benchmarks, not results; the live names are the four in `dispatch()`.

# S2. What Tier 1 is, and the one invariant

Tier 1 compiles a **lambda body** to native code. It is never required: Tier 0
interprets every body and must, because a body built and run once can never
repay compilation (RFC-0003 §S3).

**The invariant is that conformance moves by exactly zero.** CLAUDE.md states
it for this milestone specifically — "the JIT and AOT milestones must move it
by exactly zero: they change speed, not meaning, so any movement is a bug."
This RFC adds no word, changes no word's behaviour, and adds nothing to the
conformance denominator.

# S3. The compilation unit, and the cache

**D35 resolves the key: the body's `Rc` pointer.** RFC-0003 §S3 states the
shape, and D42 carries the key to every point where a stored body starts
running. **`Vm::scoped_call` is the exception.** D42 left it taking a `Vec`,
which it wraps as a LIST built on each call (`Interp::scoped_call`,
`crates/bund2-interp/src/lib.rs`), so a `context` body has no key and never
reaches the tier. That is not a meaning risk, since such a body runs at Tier 0
as it does today. It is a body this RFC does not compile.
Two consequences this RFC owns:

- **A freed body's address may be reused**, so an entry that outlives its body
  would become a *false hit* — wrong code executed, the worst failure
  available here. **A `Weak` prevents it**: an `Rc`'s allocation is freed only
  when its strong and weak counts both reach zero, so a live entry keeps the
  address out of reuse. D35 first required a strong reference for this; its
  amendment (Q32) moves the cache to a `Weak`, since the strong reference was
  buying liveness rather than safety, and D42's frames now supply the
  liveness. D35's "an invariant to test, not merely to document" stands, and
  criterion 3 tests it.
- **The cache does not pin bodies.** D35 first said it did, and that the cap
  was therefore load-bearing for heap; with a `Weak` it pins nothing, and the
  cap bounds code memory alone (§S7).

**An eval'd token stream is never a compilation unit.** It is parsed and each
token applied straight into the VM, retaining nothing (D3), so there is no body
and no `Rc` for the cache to key — under pointer keying as under identity
keying. That, not a threshold, is what D3's amended resolution rests on.

**A lambda *inside* eval'd code is an ordinary body.** `1000 { … } times`
evaluated from a string runs its inner body 1000 times under one `Rc` — D42
carries the key to the entry point — and compiles like any other. Each
re-evaluation of the string mints a new body and compiles it again, orphaning
the previous code, which §S4 says is never reclaimed. **Only the 1024-body cap
bounds that**, and a REPL re-evaluating such lines reaches it: after 1024,
Tier 1 is off for the rest of the process. That is Q27's REPL profile arriving
through D3's door. This RFC accepts it for v1 — bounding it is what the cap is
for — and names the eventual answer: content-hash keying, which D35 calls "a
strict upgrade" and defers, would let a re-evaluated line find its earlier
code.

# S4. Every inter-word call is indirect, through a runtime-owned slot

**D16 makes the world permanently open**: a call target may be a name assembled
at run time. `!` is the corpus's spelling of `execute`, and for `PTR | STRING |
CALL` it hands the name to `vm.call`
(`reference/rust_multistackvm/src/stdlib/execute.rs:26-30`).

## The chain, followed all the way

An earlier draft of this section cited `i`/`i_direct` and stopped there. That
was one call short, and it is the mistake CLAUDE.md names: `vm.call` does not
reach `i` directly — it wraps the name and applies it,
`self.apply(Value::call(name.clone(), Vec::new()))`
(`reference/rust_multistackvm/src/multistackvm_call.rs:8`). Everything below
happens *before* the inline table is consulted, and all of it is contract:

| # | step | source |
|---|---|---|
| 1 | an empty name bails | `reference/rust_multistackvm/src/multistackvm_apply.rs:13-14` |
| 2 | **`is_command` → `c(name)`**, ahead of everything else | `reference/rust_multistackvm/src/multistackvm_apply.rs:16-17` |
| 3 | **`autoadd`**, which does not call at all — see below | `reference/rust_multistackvm/src/multistackvm_apply.rs:19-27` |
| 4 | a leading `$` → `call_internal_word` | `reference/rust_multistackvm/src/multistackvm_apply.rs:33-34` |
| 5 | alias resolution | `reference/rust_multistackvm/src/multistackvm_apply.rs:39-40` |
| 6 | `is_lambda` → `lambda_eval` | `reference/rust_multistackvm/src/multistackvm_apply.rs:46-49` |
| 7 | otherwise `i(real_name)` | `reference/rust_multistackvm/src/multistackvm_apply.rs:59` |

Only at step 7 does the chain reach the inline tables, where `i` resolves
aliases **again** (`reference/rust_multistackvm/src/multistackvm_inline.rs:69-75`).
Then `i_direct` tries the VM's own table and falls through to the stack
layer's (`reference/rust_multistackvm/src/multistackvm_inline.rs:41-67`). D16 declares that order
contract.

*(Every path in this table is spelled in full because `cargo xtask cite` cannot
seed a scope from a bare filename, and a table after a blank line has none in
scope; written as `:16-17`, these were thirteen citations the tool could not
check. The fifth review found that, and opened all thirteen by hand.)*

For lowering, steps 2, 4, 5 and 6 mean a compiled call site cannot assume its
target is a native: the same name may be a command, a `$`-forced internal, an
alias, or a lambda, and which one is a run-time property.

**And "the resolved target" is ambiguous, because the two paths resolve to
different depths.** A plain name is resolved twice — once by `apply`
(`reference/rust_multistackvm/src/multistackvm_apply.rs:39-40`) and again by `i`
(`reference/rust_multistackvm/src/multistackvm_inline.rs:69-75`). A `$`-prefixed name is resolved **once**:
`call_internal_word` strips the sigil and calls `i` directly
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`),
skipping `apply`'s resolution.

On a one-deep alias chain the two agree, which is why this has not surfaced. On
a two-deep chain `a → b → c` they do not: `a` reaches `c`, `$a` reaches `b` —
because `get_alias` answers with one `name_mapping` lookup and never follows the
chain (`reference/rust_multistackvm/src/multistackvm_alias.rs:31-39`).
F26 already records that `$` does **not** bypass alias resolution — it skips
the lambda check only — and this is the finer consequence: it skips one *level*
of it.

**Bund2 does not reproduce that, by decision, and Tier 1 follows Bund2.**
`Registry::follow` walks the chain until it runs out of links — **or until 65
of them**, D39's bound against a cycle `alias` can build: the guard increments
after taking a link and breaks on `guard > 64`, so the sixty-fifth link is
followed and the sixty-sixth is not, after which it answers the last link it
reached (the twentieth review's S4; D39's note names the threshold as 64, which
is the constant rather than the count) — and `Registry::resolve` calls it for both
spellings, the sigil selecting only whether `lambda` is consulted
(`crates/bund2-api/src/lib.rs`, `follow` and `resolve`). A chain past 64 links
therefore has no fixed point, and what Tier 0 answers there is part of its
meaning. Tier 1 agrees regardless, because a name reached through an alias
points at the resolving trampoline and runs the same `dispatch`, and so the
same `follow`. So on
`a → b → dup_one` the oracle's `$a` fails where its `a` duplicates, and Bund2's
`$a` duplicates like its `a` (both run 2026-09-11). That is RFC-0002's approved
deviation, asserted by its criterion 6a and recorded in its Preservation table
and in F26's consequence paragraph. **A compiled `$name` call therefore keys on
the fixed point, exactly as every other call does**, because criterion 2's
invariant is agreement with Tier 0, not with the reference. Keying on the
one-level answer — which an earlier revision of this paragraph required — would
make compiled `$a` reach `b` where Tier 0 reaches `dup_one`, move conformance,
and un-meet an accepted criterion of another RFC under `--features jit` (the
eighteenth review's B1). Changing that is a change to Tier 0 and to RFC-0002,
and belongs in `docs/registers/decisions.md` before either is edited.

## Step 3 is the one that changes what a call means

**Under `autoadd`, a CALL is not a call.** `apply` pulls the value beneath and
appends the CALL to it — `self.stack.push(val.push(value))` — and never
dispatches (`reference/rust_multistackvm/src/multistackvm_apply.rs:19-27`); an empty stack is an error, not a
no-op. The flag is VM-wide and mutable, toggled by two words registered as
**commands**:

```rust reference/rust_multistackvm/src/stdlib/autoadd.rs:28
    let _ = vm.register_command(":".to_string(), stdlib_autoadd_enable_inline);
```

with `;` disabling it at `reference/rust_multistackvm/src/stdlib/autoadd.rs:29`. `autoadd.rs` gives no reason
for registering them as commands. **The inference** is that they have to be:
step 2 outranks step 3, and without that `;` could never turn the mode off
again, because under autoadd it would be collected rather than run.

Two consequences this RFC owns, and an earlier draft had neither:

- **Compiled code must guard on `autoadd` at entry** and decline to run when it
  is set, and **re-read it after every call it makes**, because a callee can
  turn it on. The mode changes what the reference does with **every** value
  it applies, not only calls. A CALL and a literal are appended into the value
  beneath, `self.stack.push(val.push(value))`
  (`reference/rust_multistackvm/src/multistackvm_apply.rs:22`, `reference/rust_multistackvm/src/multistackvm_apply.rs:92`). A CONTEXT value is
  **pushed as a value of its own** instead of switching stacks,
  `self.stack.push(value)` (`reference/rust_multistackvm/src/multistackvm_apply.rs:73`). It is not collected;
  F84 has this right, and an earlier revision said "collected". So a check at
  calls alone cannot honour the mode. §S5's residual path does: once the flag is seen set, every remaining
  value goes through `apply`. No OSR is involved; the residual path is part of
  the compiled function.
- **`:` and `;` are opaque sites** in the sense §S5 defines: after either, the
  meaning of every following call has changed, so promotion stops there. They
  are reachable by computed name through `!`, since step 2 tests `is_command`
  before any of the machinery D16 makes dynamic — so this cannot be decided
  statically.

**No corpus program uses `:` or `;` as a word.** An earlier revision counted
one, from a `grep` for either character standing alone. That grep matches two
files today, and neither is a use: every match in
`reference/Bund/examples/code_snippets/textexpression_demo.bund` is inside a
string literal, and the one in `tests/probes/workbench-variants.bund` is in a
comment. A `grep` cannot tell a word from a string or a comment, so it is not
evidence for a word count. `bund2 check`'s tokeniser could be. This agrees with
Bund2 binding neither word while `conform` sits at its ceiling. D16 still means
one may appear at run time.

Cranelift compounds this. `JITModule` has no per-function redefinition or
deallocation; `get_finalized_function`'s pointer is valid until
`free_memory(self)` consumes the whole module
(`00-jit-feasibility.md` §3.2a). Bund's `register`, `unregister` and `alias`
mutate the word table at run time, and Bund2 implements all three
(`register` and `unregister` in `crates/bund2-stdlib/src/values.rs`, `alias` in
`crates/bund2-stdlib/src/singles.rs`).

Therefore:

- **No compiled call is a direct relocation to a `FuncId`.** Every inter-word
  call loads a pointer from a runtime-owned slot and calls it indirectly.
- **A call is not complete when the callee returns.** A native, or the
  dispatch of a lambda, may leave a body for the loop to run afterwards. So
  every call is followed by what §S5, *A call may leave a body to run*,
  describes.
- **Redefining a word writes a new pointer into its slot.** The old code is
  orphaned; its pages are never reclaimed.
- **Unregistering rewrites the call slot to whatever the name now resolves
  to**, and writes a failing stub only when nothing does. Bund2's registry
  `Slot` declares six fields and holds **four live bindings** —
  `command`, `alias`, `lambda`, `native` (`crates/bund2-api/src/lib.rs`,
  `Slot`). `class` and `method` are declared and never written: classes live
  in `Registry::classes` and methods in `Registry::methods`, each with its own
  counter read through `oop_generation`, so class and method resolution does
  not pass through a `Slot` at all (F121; the twentieth review's S2). §S6's
  meaning guard therefore covers exactly the four, which is what §S4's chain
  consults. The `unregister` word clears the lambda alone
  (`crates/bund2-stdlib/src/values.rs`, `unregister`). So unregistering a
  lambda that shadowed a native **reveals the native**, and Tier 0 runs it; a
  stub there would diverge. The call slot itself is never freed, because
  compiled code holds its address.

**Two structures, one name each.** This RFC says *call slot* for the
runtime-owned cell a compiled call site loads its target from, and *registry
`Slot`* for `bund2-api`'s record of a name's six bindings and their generation.
There is one call slot per name, rewritten whenever that name's registry
`Slot` is touched. A name whose resolution passes through another `Slot` — an
alias — has its call slot point at a resolving trampoline that runs the full
chain through `dispatch`, so an alias is never cached and never fanned out.
When that chain ends at a lambda, `dispatch` files a request rather than
running it, and the request is drained like any call's (§S5).
That is the same rule §S6 applies when it refuses to inline through an alias,
so the two sections now describe one model.
- **Code memory grows monotonically for the process lifetime**, which is what
  §S7's caps exist to bound.
- Module rotation — fresh `JITModule`, re-JIT live words, `free_memory` the old
  one — needs a shadow stack to prove no orphaned frame is live. **Out of scope
  for v1**, as the study directs.

**§3.2d is *not* discharged by the rule above, and an earlier draft claimed it
was.** Calls on x86-64 use 32-bit relocations (±2 GB), and the study's
mitigation has two halves: reserve a contiguous region up front, and "route
**runtime-helper** calls through an indirection table so helper addresses are
never a relocation-range problem".

The slot rule covers **inter-word** calls. Runtime-helper calls are a different
set and a larger one: §S5's second rule syncs promoted values back to the real
stack *through helpers*, so every opaque site emits them, and §S6's promotion
emits them at every boundary. None of that goes through a word slot, because a
helper is not a word.

So this RFC owes §3.2d a second mechanism it does not yet specify: **a helper
table, addressed indirectly, distinct from the word slots.** The allocator half
is settled — `cranelift-jit`'s `ArenaMemoryProvider`, adopted on §3.2d's
recommendation and not on evidence of its own. The helper table is not, and it
is exactly the path §S5 depends on most.

**D16 also forecloses static devirtualisation of `!`.** Speculation behind a
guard with a full-resolution fallback is permitted, because that is speed and
not meaning — but the health metric must still move by zero.

# S5. Guard and branch; never guard and bail

Cranelift is a code generator, not a JIT runtime: **no deoptimisation, no
bailout metadata, no on-stack replacement** (§3.2b). For a dynamically typed
language this is *the* defining constraint, and it fixes the specialisation
strategy:

> Guard and branch to a compiled generic path. Never guard and bail out.

Every type-specialised region carries its generic counterpart in the same
function, reachable by a conditional branch. This costs code size and
forecloses V8-style speculation; it has no runtime metadata cost and cannot go
wrong at run time, which under D37 is the property that matters.

The absence of OSR has a second consequence: **do not plan to enter compiled
code mid-loop.** Compile at word granularity and rely on loop bodies being
separately compilable lambdas — which in Bund they are: `times`, `loop`, `map`
and `while` each test their operand with `is_type(LAMBDA)` before running it
(`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:13`, `reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:12`,
`reference/rust_multistackvm/src/stdlib/logic/map_fun.rs:12`, `reference/rust_multistackvm/src/stdlib/logic/while_fun.rs:11`), and each
refuses anything else outright — `times` with `TIMES: #1 parameter must be
lambda` (`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:38`), and `loop`, `map` and `while` the
same way (`reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:41`, `reference/rust_multistackvm/src/stdlib/logic/map_fun.rs:99`,
`reference/rust_multistackvm/src/stdlib/logic/while_fun.rs:39`).

**Bund2 now keeps what that needs — D42.** The reference's `times` passes
`lambda_val.clone()` to `lambda_eval` on every iteration
(`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:20`), so the same `Rc` arrives each time.
Bund2's `times` used to copy the body out once per call and run a slice
through a `Vm::eval_body` that took `&[BundValue]`, so no key reached the entry
point (the sixth review's B1). Under D42 `Vm::eval_lambda` takes the value and
the frame holds it, and `100 { drop } times` enters one body under one key a
hundred times, which `times_enters_one_body_under_one_key` asserts
(`crates/bund2-stdlib/src/seq.rs`).

**RFC-0004 orders the guards, and only its declared effects are trusted across
a call.** A compiled body's analysis reads each callee's effect when the body
is compiled. It classifies each callee with `Registry::resolve`, the order
dispatch uses, and reads an effect only when that answers `Native` or
`Command`. The effect is the one declared on that binding, which §S5's
pre-call check pins. `Registry::effect_of` has followed the same order since
F93 (`crates/bund2-api/src/lib.rs`), so it answers `None` for any name that
resolves to a lambda, including a lambda that shadows a native. Until then it
returned the shadowed native's effect, which the ninth review found (S1). A
lambda has no declared effect, and RFC-0004 §S3 infers its effect by
composition from every slot its body calls through. That inferred effect is never trusted across a
call, because nothing stays promoted across a call that resolves to a lambda
(D46; *What a promoted value must not change*, below). An absent effect is
`Opaque` (criterion 13). This is the RFC-0004 dependency doing real work
rather than nominal work.

## `Opaque` is not a type question, and takes the other rule

The rule above is about **types**: *is this operand an Int?* It has a generic
counterpart — the boxed arithmetic the interpreter would have run — so it can
branch to it.

**`Opaque` has no such counterpart.** It is a static claim about *stack depth*:
after `!`, the depth is unknown, and no path recovers it. Calling that "the
generic path" is a category error, and an earlier draft of this section made
it. So opacity takes a second rule:

> **Promotion stops at an opaque site.** Values held in `Variable`s are synced
> back to the real stack before the call, and everything after it runs through
> runtime helpers.

**An opaque site is one whose native says so, so every native that runs a body
must say so.** `execute.` declared `eff(1, 0)` while running whatever it was
handed, so promotion would have run straight across arbitrary code. The eighth
review found it by hand (F87, fixed). The ninth found `object`, which runs a
class's `.init` and which a lambda-only check could not reach, and `display`
(F91). **Criterion 24 is now an audit over every program `conform` runs.**
While a native with a fixed effect runs, it may not start a body, file a tail
request or dispatch a word, and it must move the current stack by its pair. Its
first run found ten more pairs that miscount the stack (F92). It covers the
natives the corpus calls, with the operands the corpus hands them, and claims no
more.

**The sync writes through the path `Stack::push` uses**, not a bare
`push_back`, so a synced value carries its D41 stack symbol. A bare write would
leave `StackSym::NONE` and render `tags: {}` where the oracle renders
`tags: {"stack": "main"}` (criterion 12).

"Everything after it runs through runtime helpers" has one exception, and it is
§S6's: an **inlined fragment** after an opaque site still runs inline, because
its per-site meaning guard re-reads exactly the state an opaque site can
change. Without that guard the exception would be unsound, and an earlier
revision of §S6 claimed it without one.

Control never leaves compiled code, so **no OSR is required** — which is the
same constraint the type rule obeys, applied consistently.

**RFC-0004 §S1 said something different, and is amended.** Its closing sentence
— "a `Fold` or an `Opaque` site bails to Tier 0" — was a forward-looking claim
about this RFC, written before it existed. Read as a run-time bail it needs the
OSR machinery §3.2b denies; read as whole-body exclusion it would refuse every body that branches or
loops — control flow is 26 of the 116 first stops that §S6's *Where promotion
stops* counts. RFC-0004's amendment of 2026-09-09 withdraws it and points here.
**This is not a deviation**: nothing a program does changes, and the health
metric must still move by exactly zero.

**D12 is untouched.** `Fold` remains a barrier that cannot be optimised across;
it simply no longer kills the body it appears in.

## After any call, three things may have changed — and one path answers all three

*Added 2026-09-10, answering the sixth review's B2 and S1.*

A compiled body calls words through call slots, and a called word can change
state the body relied on:

- **which stack is current.** `to_stack` and `to_current` declare `eff(1, 0)`,
  `stacks_left` and `stacks_right` declare `eff(0, 0)` and rotate the stack
  ring whose front *is* the current stack (`crates/bund2-stdlib/src/stack.rs`),
  `endcontext` declares `eff(0, 0)` and switches back
  (`crates/bund2-stdlib/src/conditional.rs`), and a conditional that runs its
  body on another stack does so through `Vm::scoped_call`;
- **what a later name means** — `register`, `unregister`, `alias`;
- **whether values are applied or collected** — `:` sets `autoadd`.

RFC-0004 classifies none of these; its effects count stack depth. Rather than
enumerate every word that can change them — a list that would be wrong the day
a word landed — **compiled code re-reads the state after every call it
makes**, from runtime-owned cells (§S6, *Addressing*): the current stack's
epoch, bumped on every change of current stack, and the `autoadd` flag. Name
meaning is re-read at each inlined site, which is where it matters (§S6), and
a slot call always loads its target fresh.

If either cell has changed, the body takes its **residual path**. It syncs every
promoted value to **the stack it was taken from** — recorded when the value
was promoted, not the stack current now — and then applies the rest of the
body's values one at a time through the runtime's `apply`, exactly as Tier 0
would. That is guard-and-branch, not guard-and-bail: the residual path is
compiled into the same function as the generic counterpart of everything after
the call, and control never leaves compiled code. No OSR.

**"The rest of the body's values" is an index, and the lowering must carry
it.** It is a position in the body's `Vec<BundValue>`, and the compiled ops are
not in bijection with those values: §S6 inlines fragments, promotion elides
pushes, and criterion 10's spike records Cranelift folding `1 2 +` outright and
merging the guards' cell loads between them. So a compiled body carries **a map
from each call site to the index in its source body at which the residual path
resumes**, and **no folded or inlined region spans a call site**. The second
half is what makes the first implementable rather than a wish: residual entry
is only ever *after a call*, and at a call every promoted value below the
callee's arity is either synced or provably live — so a folded region cannot
straddle one, and the index is always a real position in the source body.
Assumption 38 states it and criterion 21 asserts the side table (the twentieth
review's S5).

The sixth review's example is `1 2 "s" to_stack +`. `to_stack` bumps the epoch;
the body syncs `1` and `2` back to `main` and applies `+` through the runtime
on `s`, which fails `Stack is too shallow for inline ADD()` — as Tier 0 and the
reference both do, checked 2026-09-10. Without the check, promotion would add
the two and push `3` onto `s`.

Three rules follow:

- **A CONTEXT literal is a static barrier.** It switches stacks with no call at
  all (`reference/rust_multistackvm/src/multistackvm_apply.rs:69-87`, and `Interp::apply_step`'s CONTEXT
  arm), so the lowering sees it, syncs before it, and applies it through the
  runtime.
- **A lowered op addresses the current stack as of that op**, never a stack
  resolved once at entry. `frag::run` already does, through `Interp::pull`. A
  lowering may cache the current stack only between calls, which is exactly
  the window in which the epoch cannot move.
- **Words that reach another stack by name** — `swap_in`, `rotate_stack_left`,
  `rotate_stack_right` — do not switch the current stack, so the epoch does
  not move. When the name *is* the current stack's, they read or reorder values
  promotion may be holding. That is Q34's shape, a word observing beyond its
  arity. D55's audit keeps such words off `PROMOTABLE.txt`, so compiled code
  syncs before them.

The cost is one load and compare per cell. After every call three cells are
read: the epoch, `autoadd` and the request cell (*A call may leave a body to
run*, below). Before a call across which values stay promoted, a fourth is
read: the callee's generation (*What a promoted value must not change*, below).
Criterion 17 bounds these per-call checks as well as the inlined sites' checks.
Criterion 21 checks the behaviour, and criterion 18 the `autoadd` half.

## A call may end the program

*Added 2026-09-11, answering the eleventh review's B1.*

D52 makes `bund.exit` a request. It records a code through `Vm::request_exit`
and returns `Ok` (`crates/bund2-stdlib/src/host.rs`, `bund_exit`); nothing
ends at that moment. Tier 0 stops at its **next step**: `Interp::exit_gate`
refuses at the top of `Interp::apply_step` and of `Vm::eval_lambda`, the
refusal unwinds whatever is running, and the top level treats it as the end
of the program rather than an error (`crates/bund2-interp/src/lib.rs`,
`exit_gate` and `eval_observed`). Since F112 it also refuses where a
synchronous run returns to Rust: `Vm::eval_lambda`, `Interp::apply` and
`Vm::scoped_call` consult the gate after `run_to`, whose pop of a finished
frame does not. Before F112 a body ending in `bund.exit` returned `Ok` to the
native that ran it, and that native went on in Rust until its next step:
`map` collected, and `input*` read another line (the thirteenth review's B1).
Now the native gets the refusal at once, and in `bund2-stdlib` one error path
does work afterwards: `?try`'s. `run_tryexcept`
(`crates/bund2-stdlib/src/conditional.rs`) catches the result and runs its
handler, as assumption 25 says. `#` and `#.` also catch one —
`let _ = crate::values::execute_top(vm)`, whose result they discard by design
(`object_execute_base`, `crates/bund2-stdlib/src/oop.rs`) — but they do
nothing afterwards and return `Ok`, so that is the `Ok` path assumption 26
covers. Every other native passes the refusal up, wrapped in its own context.
The claim covers `bund2-stdlib` alone.

Compiled code has no such step. The three cells it reads after a call record
no exit, `bund_exit`'s `Ok` becomes the success status, and literal pushes and
inlined fragments never pass `apply_step`. So a compiled body would run on past
`exit`: `tests/probes/bund-exit.bund`, whose golden stops at `inside, before
exit` with exit code 3, would print `inside, after exit` under
`--jit-threshold 1`, and criterion 2 would move. A `bund.eval` or `use` whose
source calls `bund.exit` gets the refusal from the `Vm::apply` that ran the
call, even when it is the source's last value, since F112 (`eval_source`,
`crates/bund2-stdlib/src/singles.rs`, applies each value through `Vm::apply`).
`status_of` at the adapter that wraps it passes that error through unchanged,
as Tier 0 does. A direct call from compiled code gets no refusal at all, and
needs the rule below.

**The rule adds no cell. A recorded exit becomes the error status at the
helper that returns to compiled code.** Every Rust function that runs Bund
code on compiled code's behalf and then returns to it turns its `Result` into
the status through **one function**, `status_of`, which consults
`Vm::exit_requested` before anything else. On `Some` it answers the error
status and clears the request cell. **What it parks in the error slot depends
on the helper's own `Result`** (the fourteenth review's B1):
- after `Ok`, `exit_gate`'s refusal, the error Tier 0 would make at its next
  step;
- after `Err`, that error unchanged, as Tier 0 passes a native's error up.

The helpers are:
- the per-native adapter (§S8);
- the resolving trampoline, after `dispatch`, which does not pass the gate;
- §S5's drain helper, after `take_pending` and `run_to` (*A call may leave a
  body to run*, below);
- the residual path's `apply` (*What a promoted value must not change*,
  below).

And where Rust enters compiled code, the entry trampoline refuses to start a
body once an exit is recorded, as `Vm::eval_lambda` does before its cache
lookup. The frame loop enters through it too (§S8, the entry trampoline), so
it needs no gate of its own. (Until the thirteenth review a second bullet put
the frame loop's compiled entry behind `apply_step`'s gate, which a compiled
entry does not pass; S1.)

**One function, not a list, is what closes the set** (the twelfth review's
B1). The eleventh review's answer named the adapter and the resolving
trampoline. The drain helper, a boundary but not a call, was one step further
on. Tier 0's `Interp::run_to` pops a finished frame with no gate, so when
`bund.exit` is the last word of a body, `run_to` returns `Ok`; Tier 0 stops
only because its next action is an `apply_step`
(`crates/bund2-interp/src/lib.rs`, `run_to`). Compiled code's next action
after a drain is its own next op. In
`3 { "tick" println true { "bye" println 7 exit } if "after if" println } times`,
with the loop body compiled and the branch still interpreted, a drain that
returned success would print `after if` where Tier 0 exits 7. `bund2-jit`
defines `status_of` once and gives the helpers no other way to make a status,
so a helper added later cannot forget the check.

**Built 2026-09-13.** `status_of` is in `crates/bund2-jit/src/lower.rs`, and
the per-native adapter and the body's `apply` adapter both return through it;
neither makes a status of its own. **One of the four helpers this section names
exists**, so "gives the helpers no other way" is enforced by there being no
other way to write one rather than by a scan: the resolving trampoline, the
drain helper and the residual path's `apply` are unbuilt, and each will route
here when it lands. Three tests cover the arms — a recorded exit after `Ok`
becomes `Error::exited`, an `Err` is passed through unchanged rather than
replaced, and a run with neither reports success — and `Error::exited` is the
single constructor `Interp::exit_gate` also uses, so the two tiers cannot
spell the refusal differently (D63).

**The Rust side of the boundary gets what Tier 0 gives it** (the thirteenth
review's B1). In tail position, the error status `status_of` makes travels to
whatever Rust code started the compiled body. Before F112 Tier 0 answered `Ok`
at that point, and the native that ran the body acted on the difference.
`[1] { 7 exit } map` left `[1]` at Tier 0, and would have left `1` compiled.
Since F112 Tier 0 answers the refusal there too, so the status agrees.

**So does the error value** (the fourteenth review's B1). Under `?try` the
error's text becomes a value on the final stack: `run_tryexcept` stores `e.0`
in its `error` CONDITIONAL's `context` slot. Tier 0 never replaces an error.
A native whose body was refused passes the refusal up wrapped in its own
context, as `map` does with `MAP: lambda execution returns error: …`. So
`status_of` leaves an `Err` as it is, and substitutes the refusal only for
`Ok`. On the `Ok` path, Tier 0 makes its refusal at its next step. The frame
loop is flat, so that step runs inside the same `run_to`, under the same
`Vm::eval_lambda` wrapper as the compiled refusal (assumption 26). Until the
fourteenth review `status_of` substituted after `Err` too. A `try` body that
reached `exit` through `map` would then have kept only the short form.

The tail hand-back agrees with the direct tail call for the
same reason: the entry drains a handed-back body ending in `exit`, and
`Vm::eval_lambda` consults the gate after the drain. **The state left after an
exit is meaning** (D52's dated note of 2026-09-11). An embedder such as a TUI
may show the stacks, so criterion 30 compares them.

Compiled code's existing error path does the rest. It syncs every promoted
value and returns (*What a promoted value must not change*, below), and in
tail position the status travels with the return, so no compiled op runs after
the helper that saw the exit returns. At the top the embedder reads
`exit_requested` and ends the program as Tier 0's does. The cost is one load
and compare in `status_of`, which is Rust already, and nothing in compiled
code. The alternative the twelfth review also offered, an exit cell read after
every call, needs no enumeration. But it is a fourth per-call check under
criterion 17, and in tail position the entry would have to read it instead of
the body. One status-making function closes the set as surely, at no cost in
compiled code.

A fixed-effect native may not request an exit: the effect audit records one
that does, as it records a tail request (`Interp::request_exit`). Only
`bund.exit` asks, and it is opaque. Criterion 30 checks the rule. **Is
`exit_gate` the only state Tier 0 gates per step rather than returns per
call?** Today it is: it is the only gate in `apply_step`. A second would need
the same treatment, and assumption 19 says so.

## A call may leave a body to run

*Added 2026-09-10, answering the ninth review's B1. The repository owner chose
this among the options the review set out.*

Tier 0 does not always run a body when it is called. `Vm::tail_lambda` is
`Interp::request_tail`, which files the body and returns: "Ask the loop to run
`body` **after the current native returns**" (`Interp::request_tail`,
`crates/bund2-interp/src/lib.rs`). Only `Interp::take_pending` pushes its
frame, and Tier 0 calls it straight after each value it applies
(`Interp::run_to`, `Interp::apply`). Five kinds of call file a request:
- `!` on a lambda the program executed itself, `Reach::Top` (`execute_one`,
  `crates/bund2-stdlib/src/values.rs`; a lambda a container reached runs at
  once instead, F113);
- `if.stack`, when the stack it names is the current one (`if_stack`,
  `crates/bund2-stdlib/src/control.rs`; it declares `opaque(2)`, and the
  eleventh review found it missing here);
- `if` and `if.false`;
- `ifthenelse` (`crates/bund2-stdlib/src/control.rs`);
- every call to a lambda by name (`Interp::dispatch`, its `Resolved::Lambda`
  arm).

So a request means *run this before the next value*.
`{ 10 } ! 20`, `true { 10 } if 20` and `:f { 10 } register f 20` each leave
`10` beneath `20` (checked 2026-09-10).

A compiled call through a slot gets control back before the body has run. If
the compiled code went on, the body would run after the next value. And since
`request_tail` assigns, a later request could overwrite it, and it would never
run. So:

- **Non-tail: the request is state, read after every call.** A **request
  cell** (§S6, *Addressing*) is set by `request_tail` and cleared by
  `take_pending`. After every call, compiled code loads it beside the epoch and
  `autoadd`. If it is set, compiled code calls a drain helper that does what
  `Interp::apply` does after `apply_step`: `take_pending`, then `run_to` down to
  the frame count it found. That happens before the next op. Nothing decides in
  advance which words file requests, which D16 would leave open anyway.
- **Tail: the request is handed to the caller.** A body's last call does not
  drain. The compiled function returns with the request pending, and each entry
  takes it at once: the frame loop before its next `apply_step`, and
  `Vm::eval_lambda` before it returns. So a self-recursive word whose last word
  is a lambda call goes back to the loop, and recurses on the heap as RFC-0003's
  frame loop does.
- **A request is never lost at a compiled call site.** A slot call that starts
  while the request cell is set returns `Error::internal`, naming the
  invariant: some earlier call's request was neither drained nor handed back.
  Tier 0 does not leave one behind on a failure, since F96: `Interp::invoke`
  clears a request when the native that filed it fails. Before F96 a native
  could leave one, and Tier 0 ran it late (the tenth review's S3.2).

  **Compiled code does not inherit that fix, and `status_of` supplies it**
  (the fifteenth review's B1). Three helpers reach a native through
  `Interp::dispatch`, and so through `call_native` and `invoke`: the resolving
  trampoline, the residual path's `apply`, and the drain helper. **The adapter
  does not.** §S8 has it call the native's `NativeFn` directly, and `invoke`
  is private to `Interp`. So a native that files a request through
  `Vm::tail_lambda` and then fails would keep it, and `?try` would run that
  body inside its handler. When the fifteenth review found this,
  `execute_value`'s LIST arm reached it from `bund2-stdlib`. F113's first fix
  tested the item's own type, which left a lambda held in a dict inside the
  list still filing (the sixteenth review's B1). F113 as completed runs a
  lambda at once however a container reaches it, so the case is an embedder's
  native again, as F96 has it. The adapter's gap is the same either way, and
  criterion 26 gives the shape. **`status_of` therefore
  clears the request cell whenever it answers an error**, which is what
  `invoke` does for a native that fails. The same argument as §S5's: one
  status-maker, so a helper added later cannot forget it.

  **Built 2026-09-13, in the adapter rather than in `status_of`.** `status_of`
  does not exist; the adapter does, and it calls `Vm::clear_tail_request` (D56)
  on any error from the native it ran, so F96's rule now holds through a
  compiled call. When `status_of` arrives the clearing moves there, for the
  reason this passage gives — one status-maker — and the adapter's call becomes
  that helper's. Until then the obligation is discharged where the error is
  answered. **What "clears the request cell" cannot yet mean** is the mirror:
  §S5's cell has no reader — no drain helper, no epoch or `autoadd` cell, no
  compiled body that continues past a call — so today this clears Tier 0's
  `pending_tail` alone, which is the half the defect lives in. D56 rejected
  clearing *only* the mirror; clearing only Tier 0's is the opposite and is
  correct while the mirror does not exist.

  **Built 2026-09-13 (2) — `status_of` has arrived, and the clearing moved
  there.** Both adapters now return `status_of(c, r)` and neither clears on its
  own, which is what this passage promised. The mirror exists too, so
  `Vm::clear_tail_request` writes Tier 0's `pending_tail` *and* §S6's request
  cell, in the one function that writes both (assumption 33's pairing). A test
  asserts the cell is clear after the adapter answers an error, and another
  that a succeeding native's filed body still runs — the positive control,
  without which a helper that cleared unconditionally would pass every other
  case while discarding every tail request a compiled call ever filed.

  This rule is about compiled call sites. It is not a claim that Tier 0 never
  loses a request: `Interp::request_tail` assigns, so a native that files two
  loses the first. The one native that did — `execute_value`'s LIST arm, where
  `[ { 10 } { 20 } ] !` ran only `{ 20 }` — was fixed by F113. An embedder's
  native still can (assumption 29).

The drain helper runs a body synchronously, so it is a third place a body
starts, beside the frame loop and `Vm::eval_lambda` (§S8, *A decline is a
return*). It checks the Tier 0 floor before it re-enters, as `Vm::eval_lambda`
does. A body it starts is entered like any other: compiled if the cache holds it
and the Tier 1 floor allows, and declined otherwise.

**Built 2026-09-13 (D64).** The helper is `Vm::drain_tail_request`, because
`Interp::take_pending`, `run_to` and `unwind_to` are private and `bund2-jit`
holds only `&mut dyn Vm`. `Interp`'s implementation is `Interp::apply`'s tail
without the value and without the gate: the floor check, the recorded frame
count, `take_pending`, the unwind on error, `run_to`. It writes no
`pending_tail` itself — `take_pending` and `clear_tail_request` do — so
assumption 33's named set stays at four and the mirror stays paired.

**It does not consult the exit gate**, deliberately: `status_of` already
substitutes the refusal after an `Ok`, and doing both would substitute twice. A
drained body that ends the program therefore drains successfully and the
refusal is the status-maker's to make, which a Tier 0 test pins.

**Position reaches the adapter through the thunk.** A non-tail call drains; a
body's last call under `LastCall::Tail` hands the request back, because draining
there would spend a Rust frame per level and break RFC-0003's criterion 2. The
adapter cannot see where it was called from, so there are two adapter symbols —
`jit_call_native`/`jit_apply` and `jit_call_native_tail`/`jit_apply_tail` — and
`emit_into` gives the last thunk the tail symbol when the body claims a tail
call. Both are bound on the module; an import nothing calls costs nothing.

**Built 2026-09-13 (2) — the branch moved into the emitted code (D65).** The
paragraph above described the shape while the decision was Rust's. It is now
CLIF's: after a non-tail call the body loads the request cell through the
address embedded at compile time and `brif`s to a drain block, which calls
`jit_drain` **through its own slot** — one extra slot table entry beyond the
calls, so criterion 4's rule is untouched and the only relocations remain "a
thunk calling its adapter" and "the trampoline calling the body". A tail call
emits no load at all, which is how position is expressed now. The two `_tail`
adapter symbols are gone: one drain adapter serves both lowerings, because a
drain does not depend on what the call was.

**What the branch costs, and what is still Rust's.** The check is one load and
one compare, as this section claims throughout, and it is taken only when a
request is actually filed — where the earlier shape called into Rust after
every non-tail call whether or not anything was pending. The drain *helper*
remains Rust (`Vm::drain_tail_request`), which is right: it runs Bund code.

**Three edges, settled** for the tenth review's S3:
- **The epoch and `autoadd` are read after the drain.** A drained body can
  switch the current stack, set `autoadd` or `register` a name, so the loads
  that decide the residual path come after it, not beside the request cell.
- **A refused drain clears the cell.** If the Tier 0 floor refuses the drain,
  the helper clears the request before it returns the exhaustion error, as
  `Interp::invoke` clears one a failing native filed (F96). No stale request
  outlives an error.
- **So does any error `status_of` answers.** A native that files a request and
  then fails keeps it, because the adapter does not pass through
  `Interp::invoke` (above). So `status_of` clears the cell whenever it answers
  an error, whether it made that error from a recorded exit or is passing the
  helper's own `Err` through. That covers both the thirteenth review's S3 (an
  exit, where nothing could run the request anyway, since every later start is
  refused) and the fifteenth's B1 (a plain failure, where `?try` would run it).
- **Where the frame loop enters compiled code.** `push_frame` stays the one
  place a body starts. It records the body's key in `entry_log` and in §S7's
  counter, and the frame carries its exit action. The loop calls the compiled
  function when it takes that frame's first step, and pops the frame, running
  its exit action, when the function returns, whichever tier ran the body. So a
  compiled entry is counted and observed exactly as an interpreted one.

This is independent of promotion. D46 syncs before a lambda call, and §S5 syncs
before `!`. After the sync, the call still has to run its body in order, and
this is what makes it do so. Criterion 26 checks it.

## What a promoted value must not change

*Added 2026-09-10, clearing items reviews had carried since the fourth: the
stack a report shows, identity and timestamps, diagnostics from compiled code,
and a fold word that arrives at run time.*

**An error's report shows the stack Tier 0 would have shown.** An uncaught
error is reported after it has returned to the top: the report's stack
snapshot is taken when `Vm::report` is called (`Interp::report`), and the
`[BUND]  Content of the stack` dump is printed after it
(`crates/bund2-stdlib/src/report.rs`). So a compiled body **syncs every
promoted value, each to the stack it came from, before it returns an error** —
whether the error is its own or a callee's — exactly as the residual path does.
By the time anything reads the stack, nothing is left in a register.

**A native that reports mid-body may read the whole stack.** `Interp::report`
takes a snapshot when the reporter wants one for the diagnostic's severity, so
a native that calls `Vm::report` while values below its arity are held in
registers would show a short stack. That is Q34's shape, a word observing
beyond its arity, and the rule is structural: **while the reporter wants a
snapshot for a severity natives report mid-body, `Warning` or `Notice`, no
value stays promoted across a call.** Natives return errors rather than
reporting them. That was a convention (D45) until criterion 25 made it a
test: no shipped code in `bund2-stdlib` reports at `Error` severity. Promotion
crosses no other crate's natives (D47), so a native from outside
`bund2-stdlib` that reports at `Error` never holds a value it cannot see. The
embedder's fatal report comes after evaluation has returned, by which time
every error path has synced.

**The reporter is read at each compiled body's entry.** It is not fixed when
the `Interp` is built, which an earlier revision claimed. The CLI replaces it
afterwards (`run`, `crates/bund2-cli/src/main.rs`), and the field is public.
What does hold is narrower: no `Vm` method reaches the reporter, so it cannot
change while a compiled body runs.

**In the default configuration the rule withholds nothing.** D45 made
`wants_stack` take the severity, and the CLI's `TextReporter` wants a snapshot
only for a fatal report, the only kind under which it renders one
(`TextReporter::wants_stack`, `crates/bund2-stdlib/src/report.rs`). So under
`bund2 script`, with or without `--no-dump-stack`, values are promoted across
calls. A reporter that wants mid-body snapshots gets exact ones and no
promotion across calls: `CollectingReporter`
(`crates/bund2-api/src/diag.rs`) with `wants_stack` set, or a TUI
that shows the stack beside a warning. The seventh review found that the
earlier rule, which read `wants_stack` without a severity, meant no promotion
across any call in any `bund2 script` run, and §S6's promotion ceilings were
figures for a configuration nobody runs by default.

**`.id` and `.timestamp` need no rule of their own.** Both are lazy. An
identity is minted on first *need* — `.id`, equality, ordering, hashing or
serialisation (D1) — and a stamp is sampled when it is first observed, not at
construction (D2). A promoted value is observed only after it has been synced,
so its stamp is sampled at the same observation as in Tier 0.

**Its identity may be minted in a different order, and that is not meaning.**
Tier 0 also mints where nothing observes the result. `Stack::push_as` tags a
shared heap value through `with_tag`, and `with_tag`'s shared arm materialises
identity before it splits (`BundValue::with_tag`,
`crates/bund2-value/src/lib.rs`). `mint()` is a process-wide counter in the
same file. A promoted heap value skips that push, so it would be minted later,
and in a different order relative to other values. An earlier revision said
it is "minted at the same program point", which holds for scalars only. The
rule that survives a wider guard is that ids are opaque, and F14 normalises
them in every golden, so mint order is not meaning. Today the case cannot
arise, because `Guard::TopAreInt` admits unboxed scalars only.

The obligation falls on fragments instead: an arm that compares, orders or hashes
must materialise identity wherever its word does. Today's fragments compare
nothing, and scalars compare by content (D30). Criterion 16's differential
test, which already asserts `dup`'s fresh identity, is where a wider arm has to
show it.

**Compiled code emits no diagnostic of its own.** Errors are returned values
(§S11). Warnings and notices come from natives, which compiled code reaches
through their call slots, so they are emitted exactly as in Tier 0. And a
fragment *cannot* emit one: `Op` has no reporting operation. So an arm that
reports — `while`'s warning at ten million iterations, say — is not fragment
material and stays a call.

**A fold, or any change of effect, arriving at run time.** Promotion across a
call keeps the values below the callee's arity in registers, and it trusts the
callee's effect *as it was at compile time*. A `register` or `alias` can later
rebind that name to a word with a different effect — a fold that consumes the
whole stack, or simply one that consumes more — and then the promoted values
would be invisible to it. So before every call **across which any value stays
promoted**, compiled code compares the callee's generation cell (§S6,
*Addressing*) against the one it was compiled with. If it has changed, the body
syncs everything and takes the residual path from that call onward. That is one
load and compare per such call, and it is what "detected at run time" means for
D12.

**That check reads one slot, so it holds only for a call that resolves through
one.** This is the seventh review's B2, answered by carrying over §S6's three
inlining rules:

- **A direct resolution only.** A name that is an alias resolves through two
  registry `Slot`s, and each writer touches only the slot it writes
  (`Registry::register_alias`, `Registry::register_lambda`,
  `crates/bund2-api/src/lib.rs`). The stdlib's own aliases show the failure.
  `<-` and `←` alias `stacks_left`, `->` and `→` alias `stacks_right`
  (`stacks_left` in `crates/bund2-stdlib/src/stack.rs`), and `?` aliases
  `conditional` (`crates/bund2-stdlib/src/conditional.rs`). If `:stacks_left { … } register`
  binds a lambda that consumes two values, a call to `<-` reaches that lambda,
  and `<-`'s own generation has not moved. **So nothing stays promoted across a
  call through an alias.** The body syncs before it, as at an opaque site. A
  name that is direct at compile time and made an alias later is caught,
  because `register_alias` touches that name's own slot and the check fails.
- **`$name` reads its own name's slot, under the same rule.** `$` skips the
  lambda check; in Bund2 it resolves the alias chain to a fixed point like any
  other spelling (§S4, *The chain*, and RFC-0002's criterion 6a). On a direct
  name that fixed point is the name itself, so the check reads that name's
  slot. On an alias, the body syncs before the call. The effect trusted is the
  **native** binding's, since `$` skips the lambda. A lambda registered on the
  same name bumps that slot's generation, so the check still fails when one
  arrives.
- **Never against a saturated slot.** `Slot::touch` stops at `u32::MAX`
  (`crates/bund2-api/src/lib.rs`), so a check compiled against a saturated
  generation can never fail. A callee whose slot is saturated at compile time
  is treated as an alias is: the body syncs before the call.
- **The third inlining rule, that the registration is recognised, belongs to
  inlining alone.** It asks whether a fragment belongs to the binding in the
  slot, and D43's registration id answers it. The pre-call check trusts only
  an effect. For a native, the generation pins the binding that effect was read
  from, because a native's effect is declared in its own slot. That is true of
  natives only, and the next rule is why.
- **Never across a call that resolves to a lambda (D46).** A lambda's effect
  is inferred from every slot its body calls through, so pinning the lambda's
  own slot pins nothing. The eighth review gave two programs. With
  `:g { drop } register  :f { g } register`, a body `1 2 3 f` promotes `1` and
  `2` across `f`, which infers as `(1, 0)`; then `:g { drop drop drop }
  register` leaves `f`'s slot untouched, and `g` pulls three values from a
  stack holding one. And `:f { :g { drop drop drop } register g } register`
  rebinds `g` during the call, where no check made before it can look. So the
  body syncs before any call whose name resolves to a lambda at compile time,
  as at an opaque site. A name that resolves to a native at compile time and
  gets a lambda later is caught by the generation, because `register_lambda`
  touches that name's slot.
- **Only across the natives an audit has brought to `Ok` (D47, D48).**
  Promotion crosses a call only when the callee's registration id (D43) is one
  `bund2-stdlib`'s `register_all` minted, *and* the native is listed in
  `tests/golden/PROMOTABLE.txt`, which criterion 28's palette keeps.
  `register_all` records the ids it mints in the `Registry`, a replay (F32)
  re-mints and re-records them, and `bund2-jit` reads that set when it
  compiles, as it reads the fragment table. An earlier revision called the set
  "the same set its fragments are keyed by", but the fragment table holds three
  ids (the tenth review's S1). A command carries no D43 id, so no command is
  ever crossed; `bund2-stdlib` registers none. Every other native is synced
  before, as at an opaque site: an embedder's, an external package's under D9,
  or a `bund2-stdlib` native no audit brought to `Ok`. That costs speed, never
  meaning. F87, F91, F92 and F94 showed that each check was needed. Criteria 27
  and 28 check it.

This is CLAUDE.md's "follow the call one level further". The check read the
call slot and stopped there, while dispatch went on to the target. The eighth
review took the same question one level further again, from the target's slot
into the target's body. Criterion 22 checks both, the alias case and the
lambda case.

# S6. Stack-slot promotion — the actual win, and what withholds it

§2.1's only row **that needs Cranelift** and promises more than ~1.5× is
"stack-slot promotion + type guards — ~2–5×". The other three rows above that
figure — two "large multiple"s and the ~2× threaded interpreter — are all in
the "Needs Cranelift? **no**" column, which is §2.2's whole argument and the
reason the qualification matters. An earlier draft dropped it and claimed this
was the *only* such row.

The idea: a compiled body's intermediate values live in Cranelift `Variable`s
rather than round-tripping through the `Stack`, so a sequence like `1 2 +`
never materialises a `BundValue` at all.

Promotion is valuable *precisely because* touching the stack is expensive — and
**D41 made it much less expensive**, from 126.9 ns per push/pull to 9.8. So
promotion's headroom shrank with the thing that motivated it, and the ~2–5×
figure was written against the old cost. **The two are not additive and must
not be claimed as such**; criterion 10 measures what is left.

## Promotion needs inlining, and §S8 is what makes that necessary

§S8's escape from §3.2f is that a compiled word's signature is uniform —
`fn(ctx: i64) -> i32` under `CallConv::Tail`, §S8's call boundary — **because
operands travel on the shared VM stack rather than in the call**. That answer is correct and it has a consequence this section
originally did not face: *a value held in a Cranelift `Variable` is invisible to
every word.*

So `1 2 +` skips materialising a `BundValue` only if `+` is **inlined** into the
compiled body. If `+` is *called* — through a slot, with the uniform signature —
it reads its operands off the real stack, and they have to be there. **Promotion
across a word therefore requires a lowering for that word, not a call to it.**

**And the workbench is never a promotion source.** A `Variable` holds only what
the body would have on the current stack. A workbench operand is on the
workbench when the native runs — it was never promoted, so there is nothing to
sync and no epoch to guard. `PROMOTABLE.txt`'s 52 `.`-suffixed natives are
crossed on exactly that understanding, and assumption 39 states it, because
§S5's "the stack it was taken from" otherwise reads as though the workbench
were one more stack promotion draws from.

### The apparent trap, and why it is not one

`Intrinsic` is named by D9 and appears nowhere in `crates/`. D9 concludes that
it "stays internal to `bund2-stdlib`", and RFC-0000 criterion **B3** requires
that `cargo tree -p bund2-stdlib` not list `bund2-jit`. Read together those
seem to put the lowering where the code generator cannot reach it.

Two readings are wrong there, and D9's amendment of 2026-09-09 records both.
**The forbidden direction is `stdlib → jit`; `jit → stdlib` is permitted** and
nothing in RFC-0000 says otherwise. And D9's subject is the *stable ABI* —
whether external packages may ship CLIF — not where Bund2's own lowerings live.
The constraint that actually binds is narrower: **no Cranelift type may appear
in `bund2-stdlib` or `bund2-api`.**

### The mechanism: a word publishes BundIR, not CLIF

A word may carry an optional **BundIR fragment**. `bund2-jit` lowers BundIR to
CLIF on its own side of the boundary; `bund2-stdlib` mentions no Cranelift type
and gains no Cranelift dependency, so B3 and D9 stand exactly as written.
`bund2-ir` is already a dependency of `bund2-jit`.

Three constraints, and the second is what makes this affordable:

1. **The fragment is BundIR, never CLIF.** The permitted dependency direction
   is `bund2-jit → bund2-stdlib`, and criterion 15 asserts it so a later change
   cannot invert it quietly.

2. **A fragment specialises one arm and always has the word as its generic
   branch.** It is not an alternative implementation of the word.

   This is what keeps fragments small enough to be worth writing. A fragment
   for `+` cannot restate `numeric_op` (`crates/bund2-stdlib/src/math.rs`),
   which handles int, float, mixed kinds, **LIST append**, string
   concatenation and division by zero. It does not have to: under §S5's
   existing guard-and-branch rule the fragment covers `Int + Int → Int` — four
   IR operations — and every other shape branches to the call.

   **What makes that arm free of `q` is the guard.** `Guard::TopAreInt` admits
   only *unboxed* scalars, whose `q` is the 100.0 every constructor writes, and
   the word's result is a fresh value at 100.0 too
   (`crates/bund2-stdlib/src/math.rs`'s header). An earlier revision said the
   fragment carried "D32's `q` average". It carries none: the reference's `+`
   calls `numeric_op` directly and never reaches the code that averages, and
   **D32, amended on Q35's answer, says Bund2 keeps `q` but does not average
   it.** So an arithmetic result is a fresh value at 100.0 whatever its
   operands carried, and `PushInt` produces exactly that. Widening a guard to
   boxed values therefore owes no `q` arithmetic; it owes the differential test
   an operand whose `q` is not 100.0, if one can be built, or the `q` assertion
   stays unable to fail.

3. **Every fragment carries a differential test** running the specialised arm
   and the generic arm on the same inputs and asserting equality — value,
   `dt`, `q`, D41's stack tag and, for `dup`, F13's fresh identity.
   Criterion 16. On today's integer domain the `q` assertion cannot fail
   (constraint 2); it is kept so that it can once a guard widens.

Because the generic path *is* the word, a fragment can only be wrong on the arm
it claims — which is what bounds the divergence risk that would otherwise make
this a second implementation of the language.

### Inlining freezes a name — unless every inlined site asks

*Added 2026-09-10, answering the fifth review's B1. An earlier revision called
inlining the half that carried no risk. It carries one.*

§S4 builds the call architecture on a single rule: no compiled call binds a
name at compile time; every call loads its target from a slot, so a redefined
word is seen by every caller. **An inlined fragment breaks that rule by a
different route.** Its ops *are* the body, and no slot is on the path. Once
`+`'s `Int + Int` arm is inlined, the compiled body adds two integers for as
long as it lives, whatever `+` has since become — and §S4's chain lists what it
can become: a lambda (`reference/rust_multistackvm/src/multistackvm_apply.rs:46-49`), an alias to something
else (`reference/rust_multistackvm/src/multistackvm_apply.rs:39-40`), a command
(`reference/rust_multistackvm/src/multistackvm_apply.rs:16-17`), or, under `autoadd`, not a call at all
(`reference/rust_multistackvm/src/multistackvm_apply.rs:19-27`).

**It is not confined to opaque sites.** `register`, `unregister` and `alias`
declare fixed effects; they are not `Opaque`. So under §S5 a compiled body runs
straight through one and then reaches an inlined `+` whose meaning the word
before it just changed. RFC-0004's `Opaque` classifies stack depth, and nothing
classifies name meaning. Q34 is the stack-observation case of a fixed effect
understating what a word does; this is the name-table case.

**The mechanism is a meaning guard at every inlined site.** After the type
guard admits and before the first op, compiled code checks two things against
live runtime state:

1. **The site's slot generation still equals the one captured when the
   fragment was inlined.** The registry `Slot` already carries this counter —
   documented for inline caches, none of which exists yet, so the meaning
   guard would be its first reader — and each writer of a binding §S4's chain
   consults bumps it:
   `register_native`, `register_lambda`, `register_alias`, `unregister_alias`,
   `register_command` and `unregister_lambda` all call `touch()`
   (`crates/bund2-api/src/lib.rs`). A lambda shadowing `+`, an alias
   retargeting it, a command registered under it and an unregister all change
   the generation of `+`'s slot.
2. **`autoadd` is clear.**

If either check fails, the site branches to the slot call — the generic path,
in exactly §S5's shape, with no bail and no OSR. Both checks are loads at the
site, not facts fixed at compile time, so the guard holds after an opaque site
and after a table mutator earlier in the same body. Mutators need no
classification at all.

Three rules make this sound rather than nearly sound:

- **Inline only on a direct resolution.** A fragment is inlined only when the
  site's name resolves through its *own* slot — no alias on the path, and no
  lambda or command binding in the slot — so there is one generation to guard.
  A name reached through an alias resolves through a second slot, whose
  rewrite would not touch the first. Such a site is called, not inlined.
  Aliases are rare on arithmetic, and this keeps the guard to one compare.
- **Never inline against a saturated slot.** `touch()` saturates at
  `u32::MAX` so that a stale inline cache cannot match a wrapped counter; a
  saturated slot "stops caching instead". The same rule applies here: a slot
  at `u32::MAX` keeps that value through every later rewrite, so a fragment
  inlined against it would never see one.
- **Recognise the registration — not the name, and not the address.** At
  compile time the JIT must confirm the slot holds *`bund2-stdlib`'s own* `+`,
  not another native registered under the same name, and a name cannot show
  that. A function address cannot either: Rust guarantees neither that two
  distinct functions have distinct addresses nor that one function has only
  one — `std::ptr::fn_addr_eq`'s documentation says so. So a `Native` needs an
  identity of its own — a **registration id**
  assigned by `Registry::register_native` — which `Native` does not carry
  today, and `bund2-stdlib` publishes its fragments keyed by the ids of the
  registrations it made. Ids are per `Registry` — a fresh `Interp` mints new
  ones, and F32's replay gives a re-registration a fresh id — so the table is
  built per registry, at registration time, never as a static.

**Where the association lives: `bund2-jit`.** `bund2-stdlib` publishes
`(registration id, Fragment)` pairs from `crate::fragments`, and `bund2-jit` —
which may depend on `bund2-stdlib` (D9 amended) — reads them when it compiles a
site. **`bund2-api` carries no `Fragment` type.** Its only addition is the
opaque registration id on `Native`, which names no code generator and no IR.
External packages therefore cannot publish fragments: D9's amendment gives them
`Native` with a declared effect and no more, and this keeps it so.

**A lowering exists, 2026-09-12, and it is not this section's whole mechanism.**
`crates/bund2-jit/src/lower.rs` emits machine code for one fragment's ops: the
register file is a **compile-time renaming** over CLIF `Variable`s, so
`Op::PopInt`'s "shift the file and write slot 0" costs nothing at run time and
`Op::AddInt` becomes a single `iadd` between SSA values. Stack traffic — the
pops, the pushes, `DupTop`'s `.dup()` and `DropTop` — goes through four
`extern "C"` helpers over a `bund2-jit`-owned context holding the `&mut dyn Vm`
and an error slot, so an error travels in the context and never as an unwind
(§S11). The guard is asked before entry, exactly as `frag::run` asks it, so the
emitted code may assume admission and every helper's non-zero status branches to
a single failure block.

**Two of §S8's four boundary pieces are built, 2026-09-13.** The arm is emitted
under **`CallConv::Tail`** (§S8's first piece), and Rust enters it only through
a JIT-emitted **entry trampoline** under the platform's own convention (§S8's
fourth), which takes the context, makes an ordinary `call` into the arm, and
hands the status back. The trampoline is not a convenience: Rust can neither
define nor call a `Tail` function, so it is the only seam available. That a
plain `call` may cross conventions is what makes it work — the verifier has no
`typecheck_call` at all, and only `typecheck_tail_call` requires caller and
callee to agree (`src/verifier/mod.rs`). The direct call it makes is the second
of the two relocations criterion 4 permits.

An earlier revision of this passage said the entry was C-convention *in place
of* the boundary. That stand-in is gone; the entry is the boundary's trampoline
now, and the transmute points at it rather than at the arm — pointing it at the
arm would have Rust calling a `Tail` function directly, the one thing §S8
establishes is impossible.

**All four boundary pieces are built, 2026-09-13.** The **second** is
`jit_call_native`, an `extern "C" fn(*mut Ctx, usize) -> i32` that rebuilds the
`&mut dyn Vm` from the context, runs the native through `bund2_api::catch_panic`
and parks any `Err` in the error slot — reaching `bund2_api::panicked` exactly
as `Interp::invoke` does, so a caught panic is worded identically in both tiers
rather than merely being internal in both (D49). The **third** is a `Tail` thunk
per native, which makes that ordinary call to the adapter — the other of the two
relocations criterion 4 permits.

**The call site is a compiled body, and its calls are indirect by
construction.** The body loads each thunk's address from a **slot table**
allocated before compilation, whose base is embedded as an immediate (§S6,
*Addressing*), and calls through it with `call_indirect` — or
`return_call_indirect` for a last call in tail position, which is §S8's claimed
tail call. No call between compiled functions names a `FuncId`, which is what
criterion 4 is about. The table is filled from `get_finalized_function` once the
thunks exist; nothing in Rust ever reads it, and it is held only to keep the
allocation alive.

**F96's parity is closed, 2026-09-13.** The debt named here yesterday is paid:
the adapter calls `Vm::clear_tail_request` when the native it ran answers an
error, so a native that files a body and then fails leaves nothing for the next
`take_pending` to run. D56 put that method on the trait for precisely this
caller — "RFC-0005's compiled tier calls a native's `NativeFn` directly and
never reaches that place" — and this is its first consumer.

**Two tests, because one would have been enough to pass and not enough to
mean anything.** `a_failing_native_leaves_no_tail_request_behind_compiled_code`
is Tier 0's `a_failed_native_leaves_no_tail_request` through compiled code, and
`a_succeeding_native_keeps_the_tail_request_it_filed` is the positive control: a
native that files a request and *succeeds* must still leave the body to run. An
adapter that cleared unconditionally would satisfy the first while discarding
every tail request a compiled call ever filed — a worse defect than F96, in the
same place.

**§S5 assigns this clearing to `status_of`, which does not exist**, so it sits
in the adapter meanwhile. The obligation is the adapter's either way, and a gap
held open for a future helper is still a gap.

**The seam is filled, 2026-09-13** (`crates/bund2-runtime/src/tier.rs`).
`bund2-runtime` owns the `Interp` and installs a `JitTier` that holds the
`Tiering` — inside the tier, not beside the interpreter, because `Interp` owns
the `Box<dyn Tier>` and a cache held as a sibling would be unreachable while it
was installed. A body past the threshold is compiled and filed; the next entry
runs compiled code. It is the only crate that names both tiers: `bund2-interp`
knows a tier might exist and nothing about what one is, `bund2-jit` knows how to
compile and nothing about when.

**A compiling entry still interprets.** `Decision::Compile` files the code and
returns `None`, so the body runs interpreted that time and the next entry hits
the cache. The threshold is a tuning knob (§S7), not a semantic boundary, and
the fewer behaviours that hang off it the better.

**The recursion guard is a correctness property, not a lost optimisation.** The
seam takes the tier out of the `Interp` while `enter` runs, so a body entered
*during* compiled execution finds `None` and is interpreted. Without that, a
self-recursive word would enter compiled code, whose `jit_apply` calls
`Vm::apply`, which reaches `push_frame`, which would enter compiled code again —
one Rust frame per Bund level, breaking RFC-0003's criterion 2 and making this
exactly the "conformance change, not an optimisation" §S8 warns of. At most one
compiled body runs at a time. A test recurses 2,000 levels with the tier
installed to say so.

**Criterion 23 is still not met**, and the wiring does not change that: it wants
one cache, one `JITModule` and one set of cells *per `Interp`*, and while the
cache is now per-`Interp`, each `compile_word_body` still builds a module of its
own. Sharing one module across compilations is a `lower.rs` refactor and is not
done here.

**The cache and the counter exist, 2026-09-13**
(`crates/bund2-jit/src/cache.rs`).
Two maps over the same bodies, keyed on `payload_key` (D35) and each holding a
`payload_weak` (D35 as amended by Q32, and RFC-0001's amendment of the same day,
which added the accessor because the `Weak` those structures are specified to
hold was not obtainable). **Criterion 3 and criterion 6 both run**: a dead entry
answers for nothing and is gone after a sweep; the function cap leaves the body
past it interpreted; a third redefinition demotes the body live at the time;
nine bodies leave a counter capped at eight; and the counter does not pin what
it counts. A `dup`'d body finds its original's entry, which is D35's argument
for pointer keying made into a property rather than a paragraph.

**Not `Interp` integration, and not criterion 23.** Nothing consults `Tiering`
when a word runs — its caller today is its tests. And criterion 23 wants one
cache, one `JITModule` and one set of cells *per `Interp`*, where `lower`'s
entry points each build a module of their own; that stays open and is named
here rather than implied away.

**Dated note, 2026-09-13.** Both halves of that paragraph are now closed except
the cells. `Tiering` is consulted when a word runs, through the `Tier` seam at
`Interp::push_frame`; and `lower`'s `Compiler` holds one `JITModule` for every
word a tier compiles, so it is one module per `Interp`. The cache stores
`WordHandle`s into that compiler rather than code. **The cells are still
absent**, so criterion 23's third noun is unbuilt rather than shared. The two
test-only entry points, `compile` and `compile_body`, still build a module
each: neither is on an `Interp`'s path, and both exist to give the fragment
lowering and §S8's boundary a call site.

**A body that is a real Bund word, 2026-09-13.** `compile_word_body` lowers a
body's *values* — literals, `CALL`s, `CONTEXT`s — each through its own `Tail`
thunk, called indirectly through the slot table. `{ 1 2 + }` compiles and runs,
and a differential asserts the compiled word leaves the same stack as
`Interp::eval` over the same body.

**Every value goes through `Vm::apply`, and that is the design rather than a
shortcut.** `Interp::apply` is the Tier 0 floor check, `apply_step`,
`take_pending`, `run_to` and the exit gate, so a compiled body reproduces Tier 0
*exactly* — which it must, because the semantics a body owes are not yet
expressible in compiled code. **§S4's step 3 is the reason**: under `autoadd` a
`CALL` is not a call, the flag is VM-wide and toggled by `:` and `;` as
commands, and it changes what happens to *every* value applied. §S4 requires a
compiled body to guard on it at entry and re-read it after every call; that
guard wants §S6's `autoadd` cell, which does not exist, and **`autoadd` is not
readable through the `Vm` trait at all**. A test pins the consequence: with
`autoadd` set, the compiled word and Tier 0 agree that the `CALL` was appended
rather than run.

**This is a shape, not a speedup, and the RFC should not be read as claiming
otherwise.** It wraps an entry trampoline, a slot table and a status protocol
around work the interpreter already does, and is slower than Tier 0 for it.
What it buys is the structure §S6's fragments are inlined *into*. **Criterion 10
measures the tier as shipped and this is not that measurement.**

**What is still not built — and the request *cell* is part of it.** §S5's cell
is the mirror compiled code loads *after every call*, beside the epoch and
`autoadd`, to decide whether to drain. **Nothing reads it yet**: there is no
drain helper, no `status_of`, and no epoch or `autoadd` cell — this body drains
inside `apply`, where Tier 0 drains, rather than deciding for itself. Adding the
cell now would be surface with no consumer, which is the argument that deferred
D43's id set, so it waits for the reader that gives it meaning. Nor is this a
tier: **no cache and no `Interp` integration** — nothing consults a compiled
body when a word runs, and `compile_word_body` is called by tests alone — no
promotion, no inlining, no meaning guard. `bund2-jit` gains `bund2-api` as a
dependency for `dyn Vm` and `bund2-stdlib` as a *dev*-dependency so the
differential has real words to run; criterion 15's direction still holds,
checked default, with the feature, and over dev edges.

**Built 2026-09-12**: `fragments::published(&Registry)` returns the three pairs,
keyed by the ids the registry's slots hold when it is called — "at registration
time, never as a static", as above. It returns a `Result`, because each arm
validates and a malformed one is a Bund2 defect rather than a fact about the
program, and shipped code may not `expect` (D37).

**The arm for `dup` is keyed by `dup_one`.** `dup` is an *alias*
(`crates/bund2-stdlib/src/stack.rs`, `register`), so it holds no `Native` and
therefore no registration id; keying by the name a program writes would have
put an entry in the table that no callee could ever match, and nothing else in
the build would have complained. `+` and `drop` are registered under their own
names. Criterion 16's differential test still dispatches by the name `dup`,
because dispatch resolves the alias — the two spellings are right for different
jobs and `fragments.rs` says so beside each.

**What it costs**: two loads and two compares per inlined site, both
predictable — addressed as *Addressing* below describes. Criterion 17 checks that every inlined region carries the guard
and measures what it costs. Criterion 5, extended, checks that it works — for
each way §S4's chain lets a name change meaning, before the caller runs and
mid-body.

**Rejected: one VM-wide epoch**, bumped on any table write. It needs one
compare rather than two, but in a REPL, where `register` is routine, a single
unrelated definition would demote every inlined site in every compiled body
until each was recompiled.

### Addressing: where the guards' loads point

*Added 2026-09-10, answering the sixth review's S2. The costing above said "two
loads" without saying of what.*

Compiled code receives a context pointer that holds the `&mut dyn Vm` (§S8),
and `Vm` exposes no registry, no
generation and no `autoadd`. Nor does a generation have a stable address:
`Registry` keeps its slots in a `Vec<Slot>` that `slot_mut` grows with
`resize_with` whenever a new name is registered (`crates/bund2-api/src/lib.rs`,
`Registry::slot_mut`), so a pointer into it dangles after the next `register`.

So the guards read **runtime-owned cells at stable addresses**, not the
registry:

- a **generation cell per name**, mirrored from the registry `Slot`'s counter
  by `Registry::touch`, the one function that bumps it, and allocated in
  fixed-size chunks that never move when more are added. **The mechanism is
  the name.** An earlier revision said "the same `touch()`", which could not
  be built: `Slot::touch` took `&mut self` alone, so it had no `Symbol` and no
  `Registry` and could not find the cell (the twentieth review's B2). `touch`
  now lives on `Registry` and takes the symbol, so one function bumps the
  counter and writes the mirror, and assumption 37 states what that buys;
- one **`autoadd` cell** and one **current-stack epoch cell** (§S5), owned by
  the runtime in a single allocation that lives as long as the `Interp`;
- one **request cell** (§S5, *A call may leave a body to run*), mirroring
  whether a tail request is pending, and in the same allocation as the two
  above. **The cell and `Interp::pending_tail` are written together, by one
  set of writers** (the seventeenth review's B1): `request_tail` sets both,
  `take_pending` clears both, `Interp::invoke` clears both when a native fails
  (F96), the drain helper clears both when the floor refuses it, and
  `status_of` clears both whenever it answers an error. The last two reach
  `pending_tail` through `Vm::clear_tail_request` (D56), since `Interp::invoke`
  is private to `Interp` and a compiled call never passes through it. A clear
  that reached the mirror alone would leave Tier 0 holding a body its next
  `take_pending` would run, which is the fifteenth review's B1 unfixed;
**Dated note, 2026-09-13 — three of the four cell families are built, and
nothing reads them yet.** `bund2_api::Cells` is the single allocation this
section describes: `autoadd`, the current-stack `epoch` and the `request`
mirror, `Cell`s written by safe Rust, owned by `Interp` in a `Box` so the
address is stable for the interpreter's life, and reached by a tier through
`Vm::cells`. D43's generation cells already existed; the **stack-floor cell is
still unbuilt**.

**Each mirror is written where its truth is written, not beside it**, because a
mirror that can drift is worse than no mirror — it would let a guard admit a
body whose meaning had changed:

- `autoadd` is now a **private** field with one writer, `Interp::set_autoadd`,
  which writes the mode and the cell together. It was `pub`, and a public field
  cannot be mirrored: any writer would bypass the cell. Privatising it cost five
  call sites, all tests, because `:` and `;` are still unbound — cheap now and
  expensive once they land, which is why it was done now;
- the **epoch** is published from a counter inside `Stacks`, bumped at every
  site that can change which stack is current, and **exact rather than
  conservative**: a `to_stack` to the stack already current, a rotation of a
  one-stack ring, and a drop of a stack that is not current all leave it alone,
  so a guard cannot fire on a program that never switched. Assumption 40 records
  what makes it a property;
- the **request mirror** is written in the four writers this section names, and
  `every_writer_of_the_request_cell_is_named` now asserts the mirrored set
  *equals* the named set, so a half-write in either direction fails the scan
  rather than shipping. `take_pending` clears the mirror **before** it pushes
  the frame, since the body it runs may file a request of its own.

**What this does not yet do.** No compiled code reads any of it: there is no
residual path, no drain helper and no `status_of`. The value of the cells today
is that the mirrors provably track the truth, which is the part a later lowering
cannot check for itself, and the part that is testable now.

**Dated note, 2026-09-13 (3) — the request cell has a reader.** `status_of`
(§S5, *A call may end the program*) clears the request on every error it
answers, through `Vm::clear_tail_request`, which writes the mirror beside
`pending_tail`. That is the first code on the compiled path to touch a cell
rather than merely keep it in step. **The other three are still unread**:
`autoadd`, the epoch and the floor are written and never loaded, because no
lowering emits a guard, a residual path or an entry check. The paragraph above
holds for them.

**Dated note, 2026-09-13 (4) — the request cell is now acted on, not only
cleared.** §S5's drain helper is built: `Vm::drain_tail_request` on the trait,
`Interp`'s implementation, and a `drained` step in the compiled boundary that
runs a filed body after every **non-tail** call, before the next value. So the
cell's whole contract is exercised — filed, drained, cleared — rather than
mirrored and discarded. **The load is still Rust's, not CLIF's**: compiled code
calls the adapter unconditionally and the adapter asks the `Vm`, where §S6 has
compiled code load the cell itself and branch. That is the remaining half, and
it arrives with the guards. The other three cells are unchanged: written, never
read (D64).

- one **stack-floor cell** per `Interp` (§S8), holding the floor that `Interp`
  took from its thread's declared region, which the check at every compiled
  body's entry compares `get_stack_pointer` against.

**Dated note, 2026-09-13 (2) — the floor cell is built, and §S6's four cell
families are complete.** `Cells::floor` holds the **Tier 1** floor,
`top − share + STACK_RESERVE`, set once in `Interp::new` from `tier1_floor()`.
It is the only one of the four with no writer discipline, and needs none: the
floor is a property of the thread the `Interp` was built on and never moves, so
there is nothing for it to drift from.

**It is not the Tier 0 floor**, which is a different and lower address —
`top − size + STACK_RESERVE`, one reserve above the stack's *end*. Mirroring
that one into the cell would admit compiled frames into Tier 0's part, against
D44's "never less"; a test asserts the Tier 1 floor is the higher of the two.

**A thread with no share gets a floor above its own top**, since the share is
zero and the reserve is added to the top. Every stack pointer is then beneath
it, every compiled body declines, and the tier is inert on that thread — §S8's
"compiled code does not run there" as arithmetic rather than a flag. An
undeclared thread reaches the same answer through `stack_marker`.

**Still no reader.** No lowering emits the entry comparison yet, so the cell
records the floor without anything consulting it. D62.

**Dated note, 2026-09-13 (3) — this section is built for the request cell.**
A lowering now takes the cells' base as a parameter, embeds it with `iconst`,
and emits `uload32` of the request cell followed by `brif` after every non-tail
call — the load-and-compare this section describes, in machine code rather than
in Rust. Two things made it sound, both D65: `Cells` is `#[repr(C)]`, because a
default-repr struct may be reordered and a compiled base-plus-offset load
against it is undefined; and the offsets come from `std::mem::offset_of!`, so a
lowering cannot disagree with the layout it reads. A zero base is refused, since
a body lowered without cells would read address zero from emitted code.

**The other three cells are still not loaded, and that is deliberate.** The
`autoadd` and generation guards protect an *inlined region*, which criterion 17
requires to exist, record in a side table, and check for dominance — and the
body lowering inlines nothing, so every site already is the generic slot call a
failing guard would branch to. The epoch's second path is §S5's residual path,
which needs promotion and assumption 38's resume index. Emitting those guards
now would produce two branches to identical code and let criterion 17 pass
vacuously. They arrive with fragment inlining.

At compile time the JIT embeds each cell's address as an immediate. That is
safe to do because the cells outlive every compiled function: both die with the
runtime. A check is then one load and one compare per cell. That makes **two
per inlined site** (generation, `autoadd`) and **three after every call**
(epoch, `autoadd`, request). **A third comes before a call across which values stay promoted**:
the callee's generation (§S5), so generation cells are read at call sites as
well as at inlined sites. Each compiled body entry adds **one** (the floor).
Criterion 17 bounds the per-site and per-call costs alike. AOT cannot embed an
address; it would reach the cells through a relocated data symbol or a helper,
and AOT's lowering is RFC-0006's.

**"The runtime" is the `Interp`, and there is one of each per `Interp`**: one
compiled cache, one `JITModule`, one set of cells and one fragment table. The
floor cell's value is the one each `Interp` takes from its thread's declared
region when it is built. Values are `Rc` and `!Send`, but two `Interp`s on one
thread share them freely, as the tests and embedders do. A cache shared by the
thread would find code compiled against another `Interp`'s cells, or against
freed cells once that `Interp` dropped. With a cache per `Interp`, a body
another `Interp` compiled is a miss, and runs at Tier 0 or is compiled again
against this `Interp`'s cells. Registration ids are per `Registry` (D43), so
the fragment table could not be shared in any case. Criterion 23 checks this.

**The cells add no `unsafe`.** They are `Cell`s written by safe Rust, and
compiled code reads them through addresses it was handed as integers. The tier
needs `unsafe` in two places, both in `bund2-jit`: calling JIT-emitted code at
all, and the native adapter's dereference of its context pointer (§S8's call
boundary). An earlier revision said the first was the only one. That was
before the boundary was specified (the tenth review's B2).

**What this changes outside this RFC.** The generation mirror belongs to
`Registry`, which is `bund2-api`'s, so `Registry` grows an accessor that hands
out a name's cell. With the registration id above, that is an addition to
RFC-0002's surface — **decided as D43** (Q37), and built when this RFC reaches
Proposed. **Built 2026-09-12**, on D59's authorisation: `Native::id` carries an
opaque `RegistrationId` minted per `Registry` by `register_native` (and left
`None` by `register_command`, since no command is crossed), and
`Registry::generation_cell` hands out one `Cell<u32>` per symbol from boxed
chunks of 256, written by `touch` beside the bump. RFC-0002's amendment records
the surface. The id *set* §S5 wants `register_all` to record is not built:
`bund2-jit` is still a placeholder, and it is that consumer's to shape.

**So both halves of this design carry risk.** Inlining carries the
*redefinition* risk, closed by this guard. Promotion carries the
*stack-visibility* risk, closed by Q34 and criteria 9, 12 and 14. The staging
below weighs both.

### Measured — three times, and the third reverses the second

Fragments were prototyped and measured before the rest were written, as this
section required. `crates/bund2-bench/benches/fragment.rs`, run with

    cargo bench -p bund2-bench --bench fragment

on 2026-09-10 — one run, this machine, release. There is no Cranelift lowering
to time, so it measures **ceilings**, and each column is constructed rather than
subtracted:

| column | what it is |
|---|---|
| `tier0` | the program interpreted, every word dispatched |
| `inlined` | the fragment executed by `frag::run` — the model, as it runs |
| `lowered` | the fragment's own ops written out in Rust: literal pushed and pulled, guard asked, nothing folded — **the ceiling for inlining** |
| `promoted` | intermediates held in a register — **the ceiling for inlining plus promotion** |

Per operation — Criterion's point estimate for 1000 operations, divided by
1000:

| | `tier0` | `inlined` | `lowered` | `promoted` |
|---|---|---|---|---|
| `Int + Int` | 59.2 ns | 44.7 ns | **29.2 ns** | **6.7 ns** |
| `dup drop` | 88.8 ns | 44.7 ns | **29.7 ns** | **6.9 ns** |

| | inlining alone, `tier0`/`lowered` | promotion on top, `lowered`/`promoted` | together |
|---|---|---|---|
| `Int + Int` | **2.0×** | **4.4×** | 8.8× |
| `dup drop` | **3.0×** | **4.3×** | 12.9× |

Run-to-run spread is about 2%: `int_add/tier0` read 57.9 and 59.2 ns in two
runs the same day, and the sixth review's re-run agreed with every ratio to
within 3%. The two `inlined` cells are both 44.7 ns, and equal in that re-run
too (46.3 ns). That is `frag::run`'s fixed cost — guard, op loop, register
file — dominating two arms that each do about one push's work; it is not a
copying error.

**The inlining ceiling on arithmetic is 2.0×, and promotion is the larger
multiplier.** Once an inlined arm's operands live on the real stack, the stack
traffic — the literal's push, then two pulls and a push per `+` — is most of
what is left, and removing that traffic is exactly what promotion does. The
prize is promotion. Inlining is what makes it possible (*Promotion needs
inlining*, above).

**The two earlier versions of this table were each wrong, in opposite
directions**, and both are recorded because both errors are easy to repeat:

- **2026-09-09: inlining 3.8–5.4×.** Its `inlined` column was hand-written as
  one pull and one push, with the literal `1` folded into the arm as a constant
  and no guard. No fragment can express that: `Op` has no immediate operand,
  and `fragments::int_add()` pops two values behind `Guard::TopAreInt(2)`. Its
  `dup drop` half also measured `push(clone)` rather than `dup`, which is not
  what the word does (F13); correcting that alone moved 5.4× to 4.8×. The fifth
  review found the folded constant (B2).
- **2026-09-10, first re-run: inlining 0.9× — slower than the word.** That
  column ran the real fragment through `frag::run`, which at the time
  collected the top of the stack into a `Vec` and heap-allocated its register
  file on every entry: `int_add/inlined` read 65.0 ns against `tier0`'s 57.9.
  Removing both allocations brought it to 44.7. That gap is the model's
  overhead, not inlining's — which is why `lowered` is a separate column.

The earlier revision drew its staging from the first of these: "inline first,
promotion second", because inlining was the larger win and every risk belonged
to promotion. **Both halves of that are withdrawn.** Inlining is the smaller
win, and it carries the redefinition risk the meaning guard above closes.

**So the staging is:** inlining is built first because promotion cannot exist
without it, not because it pays on its own. On operand-free arms — `dup drop`,
3.0× — it clears criterion 10's threshold by itself. On arithmetic — 2.03× —
it clears it by a hair before compiled code pays anything, and criterion 10
expects the lowering to fall under it. Promotion follows,
and criterion 10 is where it has to earn its machinery.

**Re-measured 2026-09-12, twice, and two ratios moved enough to record.** Same
command, same machine, two consecutive runs; medians from Criterion's
`estimates.json`. The second was 5–8% faster on every arm, which is machine
state rather than a change in Bund2:

| | `tier0` | `inlined` | `lowered` | `promoted` |
|---|---|---|---|---|
| `Int + Int`, run 1 | 56.57 ns | 38.94 ns | 32.02 ns | 9.33 ns |
| `Int + Int`, run 2 | 54.81 ns | 37.61 ns | 30.28 ns | 8.90 ns |
| `dup drop`, run 1 | 80.05 ns | 28.99 ns | 22.89 ns | 9.46 ns |
| `dup drop`, run 2 | 76.64 ns | 27.83 ns | 21.45 ns | 8.74 ns |

| | inlining alone | promotion on top | together |
|---|---|---|---|
| `Int + Int` | **1.77× / 1.81×** | 3.43× / 3.40× | 6.06× / 6.16× |
| `dup drop` | **3.50× / 3.57×** | 2.42× / 2.45× | 8.46× / 8.77× |

Arithmetic inlining's ceiling fell from 2.03× to **1.77× and 1.81×** — under
criterion 10's stop rule rather than over it, on both runs, so it is
reproducible here and not run-to-run noise. The dated note on that criterion
says what that does and does not settle. Promotion's own multiplier on
`dup drop` also fell, 4.3× to ~2.4×, because `lowered` improved more than
`promoted` did. And the two `inlined` cells are no longer equal: 38.94 and
37.61 against 28.99 and 27.83, where both read 44.7 before — so the earlier
observation that `frag::run`'s fixed cost dominated both arms equally no
longer holds, and the arms now separate by about 10 ns.

The table above is not replaced, because the conclusion it draws is unchanged
and stronger — the prize is promotion, inlining is its prerequisite — and
because two readings of the same ceiling on either side of a threshold is
exactly the kind of thing a later reader needs to see rather than have tidied
away. **Absolutes here are one machine's; the shares in §S1's update are what
the gate turns on, and they moved by under half a point across the two runs.**

### What has been built, 2026-09-09 and 2026-09-10

The representation and its first consumer, which is everything on this side of
a code generator:

| | where | what it is |
|---|---|---|
| `Fragment`, `Guard`, `Op` | `crates/bund2-ir/src/fragment.rs` | the arm, naming no code generator |
| `frag::run` | `crates/bund2-interp/src/frag.rs` | executes one against the real stack, allocating nothing; `Ok(false)` only when the guard declines |
| `Fragment::new` | `crates/bund2-ir/src/fragment.rs` | the only constructor outside the `unchecked` feature, which only `bund2-interp`'s `[dev-dependencies]` enables; refuses a fragment whose ops, walked typed against its guard, could fail after it admits |
| `Vm::peek_at` | `crates/bund2-api/src/lib.rs` | the top *n* without copying the stack — what a guard asks |
| `int_add`, `dup`, `drop_top` | `crates/bund2-stdlib/src/fragments.rs` | the two measured arms |
| criterion 16's differential test | same file | the arm against the word, over the arm's boundaries |

**Criterion 15 holds**: `cargo tree -p bund2-stdlib` lists neither `bund2-jit`
nor any `cranelift-*` crate, with `bund2-stdlib` now depending on `bund2-ir`.

`frag::run` is **not Tier 1** — it generates no code and caches nothing. It
exists so the representation is exercised rather than assumed, and so
criterion 16 can run before a lowering is written. **The remaining half is the
Cranelift consumer**, which is what `bund2-jit` is for and what this RFC still
has to be Accepted before anyone writes.

Four properties are asserted by test rather than by prose, because each fails
silently:

- **A declined guard leaves the stack untouched.** If it did not, the word that
  runs instead would see operands the program never pushed.
- **A fragment that could fail after its guard admits cannot be built**, and
  one that escapes anyway is an internal error, never a decline and never a
  silent success: by then the operands are gone (criterion 19).
- **The arm agrees with the word on value, `dt`, `q` and D41's stack tag**,
  none of which any golden prints on this path. On today's domain the `q` half
  cannot fail, and constraint 2 says why.
- **`dup`'s copy has its own identity**, in the arm and in the word (F13). The
  render comparison normalises identities away and could not see this; an
  earlier version of the test relied on it alone.

**What the ceilings do not include.** `lowered` and `promoted` call `Interp`
directly rather than through `&mut dyn Vm` or §S4's runtime helper table, and
compiled code pays entry, exit, a type guard and — per inlined site — a meaning
guard. Both columns are optimistic, `promoted` more so: its `dup drop` case is a
value kept alive in a register, which is what "compiles to nothing" looks like
in Rust. Criterion 10 measures the real thing; these say whether it can be worth
measuring. For promotion the answer is yes. For inlining alone on arithmetic it
is almost certainly not, and criterion 10's measurement decides.

### Where promotion stops — the corpus, counted

*Restored 2026-09-10. An earlier revision carried this table, lost it in an
edit, and kept five references to it; the fifth review found them pointing at
nothing — one of them in RFC-0004's accepted amendment.*

`bund2 check` reports where RFC-0004's analysis stops: the first stop on each
analysed path, not every opaque site (the re-derivations below say why).
Re-derive with:

    for f in $(find reference/Bund/examples reference/Bund/tests tests/probes \
                    -name '*.bund' -not -path '*/features/*'); do
      ./target/debug/bund2 check --file "$PWD/$f" | grep -E '^ +[0-9]+ +`'
    done

Re-derived on 2026-09-12, after the MATRIX family's probe, across **190**
programs, it prints **137** sites (on 2026-09-11 the same 137 over 189, and
that morning 124 over 165):

| why analysis stops | sites |
|---|---|
| the word's effect is not a fixed pair — `Opaque` or `Fold` | **116** |
| the name has no binding when analysed | **21** |

The 116, by word:
- `object` 25, `!` 17, `format` 14, `loop` 11, `times` 9, `if` 6, `graph!` 5,
  `clear` 5;
- two each of `math.interpolation`, `load.model`, `input` and `?true*`;
- one each of `unfold`, `**.`, `display`, `map`, `*+`, `?.`, `fold`, `pull.`,
  `notifthenelse`, `bund.exit`, `bund.eval`, `if.in_workbench`, `execute` and
  `*loop`;
- two user words, `HelloWorld` and `f`, whose inferred effects are opaque.

The probes `if-workbench-variants` and `stack-words-by-name` added the first
stops at `if.in_workbench` and `execute`. F94 added none, since no program
calls `drop_stack`, and neither did the `sysinfo-version` probe.

**Twenty-six are ordinary control flow**: `loop` 11, `times` 9 and `if` 6. That is
why whole-body exclusion, RFC-0004 §S1's original reading, would refuse nearly
every body that branches or loops.

**All 21 are names Bund2 does not register at all**: `classifier` 4,
`sample.analysis`, `internaldb.execute`, `global` and `console.spinner` 2 each,
and nine singletons from the postponed console, database and AI vocabulary.
`generator` and `cwd`, 24 of the morning's 48, are registered now. Re-derived
again after F111 made six named-stack words opaque, every figure here is
unchanged: no program reaches the six. "No effect" there means *no binding at analysis time*, and D16 says
a binding may still arrive at run time; that is why criterion 13 treats an
absent effect as `Opaque` rather than as zero.

**What promotion can reach is the straight-line run before the first of these
sites in a body.** Inlining, behind the meaning guard above, is not bounded by
them.

The figures move as words land and effects are declared. The fourth review
counted 116, 61 and 55; since then `if` has gained a site and `#` has appeared.
That is why the command sits beside the numbers: a count with no derivation
beside it rots.

**Re-derived after F87, 2026-09-10: still 113, 63 and 50.** Making `execute.`
opaque did not move the count, although
`tests/probes/remaining-vocabulary.bund:52` and `:55` call `execute.` and
`!.`. `bund2 check` abandons that program at an earlier stack switch, and it
counts only where analysis *stops*, once per program path. A site after the
first stop is never reached. Stops at a stack switch also fall outside the
`grep` above, because that report names no word. So the table counts the first
word-named stop on each analysed path, not every opaque site in the corpus.
It is a lower bound, and the eighth review's "moves by at least two" assumed
the probe's calls were reached.

**Re-derived after F91 and F92, 2026-09-10: 122, 74 and 48**, from 113, 63
and 50. Making `object`, `format`, `clear`, `fold` and `display` opaque moved
first stops earlier on many paths:
- `object` became the commonest first stop, because construction usually comes
  before the first `!`;
- `!` fell from 32 to 15, and `if`, `loop` and `graph!` fell with it;
- two unbound names were no longer the first stop on their paths.

The shape is the one the paragraph above describes: the table counts first
stops, not every opaque site.

Three things bound promotion:

- **D12 — the `*` fold family is a permanent optimisation barrier.** `*+`, `**`
  and friends consume the whole stack, so the stack depth is not statically
  known across them. ERRATA records that the corpus uses none of them, so the
  barrier costs nothing measurable; it still has to be *represented*, because
  D16 means one may appear at run time. Bund2 registered none of the ten until
  2026-09-11 (`*.` is ordinary multiplication's workbench variant, not a fold).
  The eight arithmetic folds, with `Σ` and `Σ.`, and `*loop` and `*loop.` are
  now registered, and each declares `StackEffect::opaque`, since
  `StackEffect` has no separate fold kind. So a call to one stops promotion
  as an opaque site does. D12's "bails to Tier 0" is read as §S5's promotion stop (D12's dated
  note), and a fold bound at run time to a name a body was compiled against is
  caught by §S5's pre-call check.
- **`Opaque` effects**, counted in *Where promotion stops* above. Promotion
  stops; compilation does not.
- **D33 is RESOLVED, and the lowering is available.** Ordering across int and
  float answered **true to all four of `<`, `>`, `<=`, `>=` at once** (F47),
  which is not a machine representable order, so while D33 stood this RFC
  forbade lowering a mixed-kind comparison and required the generic path. D33
  took option 2 on 2026-09-11: the two order by the mathematical values they
  denote, exactly one of `<`, `==`, `>` holding. A mixed-kind comparison may
  now be lowered, **but not as a single machine compare**.
  `exact_int_float_ord` (`crates/bund2-stdlib/src/logic.rs`) is a widening
  compare only where the integer converts exactly; NaN answers false to all
  four, ±∞ are special-cased, and above 2^53 the comparison goes through the
  float's floor with a fractional tie-break. So this is a lowerable *fragment*
  under §S6's constraint 2 — the fast path guarded, the generic branch still
  the word — and not an `fcmp`.

# S7. Tiering policy: threshold, cap, demotion — Q22 (cache), answered here

Q22 (cache) asks for the compiled-cache promotion threshold and cap and says RFC-0005
"must state them as load-bearing for D3, not as tuning". D35 first added
that the cap was load-bearing for heap as well, because the cache held strong
references to bodies; its amendment (Q32) moves the cache to a `Weak`, and the
cap now bounds code memory alone.

Stated once, for both:

| knob | value | why it is load-bearing |
|---|---|---|
| **promotion threshold** | 64 evaluations of one body | Below it, a body is interpreted. It is **not** what makes D3 true — an eval'd token stream is never a body at all (§S3). A lambda inside eval'd code *is* a body, can cross 64 within one evaluation, and is bounded only by the cap. An earlier revision said the threshold made D3 true by construction, and it does not |
| **compiled-function cap** | 1024 bodies | Code memory is never reclaimed (§S4). This is the only bound on it |
| **recompile cap** | 4 per slot | A word redefined in a REPL loop would otherwise orphan a function per redefinition |
| **demotion** | permanent, per body | A body that exceeds the recompile cap returns to Tier 0 and is never promoted again |
| **counter cap** | 4096 bodies | See below. The counter is a second structure and needs its own bound |
| **heap consequence** | neither structure pins a body: the cache and the counter both hold a `Weak` | D35 first required the cache's reference to be strong; its amendment (Q32) withdrew that |

These are **defaults with a stated basis**. The cap exists because
`free_memory` is all-or-nothing, and changing it changes a correctness argument,
not a benchmark. The threshold is the tuning knob of the two: it decides when a
body has earned compilation, and no correctness argument rests on it. Both are
configurable, and the configuration is recorded rather than silent.

## The counter's key and lifetime

An earlier draft named a threshold of "64 evaluations of one body" and said
nothing about what counts them. That is not a detail: the counter is a second
map over the same bodies as the cache, and the two obvious designs are both
wrong.

- **Key on the pointer, hold a strong `Rc`** — safe from address reuse, and it
  pins *every body ever evaluated*, not the 1024 that were compiled. The heap
  claim in the table above would be false and the growth unbounded.
- **Key on the pointer, hold nothing** — no pinning, and it inherits exactly
  the hazard §S3 guards against: a freed body's address is reused, the new body
  inherits a hot count, and it is compiled on its first evaluation.

**The counter holds a `Weak`.** That is not a compromise between the two; it
removes the dilemma, because of a property of `Rc` this RFC now depends on:
**the backing allocation is freed only when the strong *and* weak counts reach
zero** (the `std::rc` module's documentation of `Weak`). So a live `Weak` keeps the allocation — and therefore the address
`Rc::as_ptr` returns — out of circulation, while the body's *contents* are
dropped on the last strong reference. No pinning, and no reuse.

Their stale entries would fail differently, which is why the question of
strength was asked separately for each:

| | strength | a stale entry means |
|---|---|---|
| compiled cache | `Weak` (D35 as amended) | a dead entry that no longer upgrades — swept, and never a false hit, because its address cannot be reused while it lives |
| promotion counter | `Weak` | at worst a body compiled earlier than it earned. A performance mistake, never a wrong answer |

Both now hold a `Weak`, for the same reason: a `Weak` is enough to keep an
address out of reuse, and a strong reference would pin what nothing else
needs.

**And that argument cuts at D35, which this RFC should say rather than let a
reader notice.** D35's stated reason for the cache holding a *strong* reference
is that "if an entry outlives its body, the allocator may reuse the address and
a stale entry becomes a false hit". The paragraph above establishes that a
`Weak` prevents address reuse too — the allocation is not freed while one
lives. So **address safety is not what the strong reference buys**, and D35's
rationale as written is weaker than it appears.

What a strong reference does buy is separate and this RFC does not have the
standing to decide it: a demoted or evicted entry needs the *body* to fall back
to, and if the cache were the last holder, dropping the entry would drop the
lambda. Whether that is reachable — whether a body can be live for the cache
and dead for everything else — depends on how bodies are held elsewhere, which
is RFC-0003's territory.

**Settled 2026-09-10 (Q32, option A).** The owner amended D35: the cache
holds a `Weak`. The case above is why — address safety never needed a strong
reference — and D42 removes the other reason: every running frame now holds its
own clone of its body's `Rc`, so a body is alive whenever compiled code for it
can run. The key is unchanged.

**Lifetime.** An entry whose `Weak` no longer upgrades is dead and is evicted
on the next sweep. The map is capped at 4096; at the cap, the coldest entries
go first. A dead entry costs one `RcBox` header until it is swept — not the
body.

**D39 applies**: the sweep is an internal loop and is bounded on data already
taken. It runs over the map as it stands, never until a condition holds.

**Dated note, 2026-09-13 — what calls the sweep, which this section never
said.** Four passages describe what the sweep does and none names a trigger,
while criteria 3 and 6 both assert state "after the sweep" and therefore need it
callable. Built as: `Tiering::sweep` is **public**, so a caller — and a test —
can ask for it; and `Tiering::observe` calls it when the counter reaches its
cap, *before* evicting a live entry, because dropping a dead entry is free where
evicting a live one costs a recount. Chosen rather than specified. It changes no
observable behaviour — only when memory is given back — which is why it is
recorded here as implementation policy and not taken to the decision register.

**The caps are built, 2026-09-13** (`crates/bund2-jit/src/cache.rs`,
`Tiering`), with this section's four knobs configurable as it requires, and each
row of the table above asserted by a test at small values, as criterion 6 asks.
Two behaviours worth stating because they are opposite and easy to transpose:
the **function cap refuses** — a body past it stays interpreted, since code
memory is never reclaimed and evicting would orphan a function — while the
**counter cap evicts**, coldest first, because an evicted count costs only a
recount. The **recompile cap counts per slot and demotes the body live at the
time** (the owner's decision, 2026-09-13): the two keys differ because a
redefinition replaces one body with another, so the slot is what persists across
it and the body is what a demotion can name.

# S8. Tail calls, and why §3.2f does not take them away

`CallConv::Tail` with `return_call` / `return_call_indirect` is supported on
x86-64, aarch64 and riscv64 (§3.1); **s390x historically lacked it**, so the
lowering must degrade to an ordinary call there rather than assume it.

**Dated note, 2026-09-13 — the degradation has no trigger at the pinned
version, and the lowering therefore has no branch for it.** At
`cranelift-codegen` 0.135.0 every backend carries lowering rules for
`return_call` and `return_call_indirect`, s390x included: the section "Rules for
`return_call` and `return_call_indirect`" appears in `src/isa/s390x/lower.isle`
and in `aarch64`, `x64` and `riscv64` alike. Nor is there a per-target
capability to consult — `supports_tail_calls` is a property of the *convention*,
answering `true` for `CallConv::Tail` and nothing else on every target
(`src/isa/call_conv.rs`). So "historically lacked it" is now history, and the
first lowering emits `Tail` unconditionally and says why.

The first draft of that lowering did branch, on
`CallConv::Tail.supports_tail_calls()` — a constant `true`, so a check that read
like a capability test and tested nothing. It was removed rather than repaired:
a branch no target can take is worse than none, because it looks covered. If a
target without tail calls is ever added, this is the sentence to come back to,
and **Q28** is where the question of whether s390x is a target belongs.

## The objection this section has to answer

§3.2f says dynamically typed languages want callee-side argument-count checks
and array-style access, and that the workaround — "passing `argc: usize,
argv: *mut Value`" — **defeats tail calls**. An earlier draft of this section
did not mention it, which left §S8 premised on something the study appears to
withdraw two pages later.

**The argument count does not apply to Bund, and the reason is structural.**
§3.2f is about passing *arguments*. A concatenative language passes none: a
word's operands are already on the VM's stack, which the callee shares. So no
signature here is variadic, and none becomes `argc/argv`.

**But a native's own signature is not a boundary compiled code can use, and an
earlier revision said it was** (the tenth review's B2):

```rust crates/bund2-api/src/lib.rs:61
pub type NativeFn = fn(&mut dyn Vm) -> Result<(), Error>;
```

That is Rust's unspecified ABI. `&mut dyn Vm` is two words whose layout is not
stable, and `Error` wraps a `String` (`crates/bund2-api/src/lib.rs`, `Error`),
so the return is not a status Cranelift can express. `cranelift-codegen`
0.135.0's `CallConv` has no Rust ABI (`src/isa/call_conv.rs`, `CallConv`).
`return_call` constrains the callee against the caller: the verifier requires
both to share a calling convention that supports tail calls, and to return the
same types. Parameters need not match (`src/verifier/mod.rs`,
`typecheck_tail_call`). Only `CallConv::Tail` supports tail calls
(`src/isa/call_conv.rs`, `supports_tail_calls`), and Rust cannot define a
`Tail` function. So the boundary is four pieces:

*Built 2026-09-13 — all four, in `crates/bund2-jit/src/lower.rs`. The context
holds the `&mut dyn Vm`, an error slot and the table of natives the body may
call; it does **not** yet hold §S6's cells or §S5's request mirror, neither of
which exists. The natives table carries each native's **name** beside its
function, because D49's parity claim is about the message and Tier 0's names the
word.*

1. **One JIT signature**: `fn(ctx: i64) -> i32` under `CallConv::Tail`, a thin
   pointer to a per-call context and an integer status, `0` for success and `1`
   for an error. The context is a `bund2-jit` struct. It holds the
   `&mut dyn Vm` as a stored fat pointer, an error slot, and the cells of §S6's
   *Addressing*.
2. **A per-native adapter in Rust**: `extern "C" fn(ctx: *mut Ctx, native:
   usize) -> i32`. It rebuilds `&mut dyn Vm` from the context and calls the
   native's `NativeFn` through `bund2_api::catch_panic` (D49). It parks an
   `Err` in the error slot and returns the status. No panic unwinds out of it,
   so none reaches a compiled frame.
3. **A JIT-emitted `Tail` thunk for each native a call slot can hold.** The
   thunk makes an ordinary call to the adapter. A compiled body is already
   `Tail` and sits in its slot directly. So every slot target is `Tail`, and
   `return_call_indirect` is legal from any tail position: body to body, and
   body to a native's thunk.
4. **An entry trampoline**, JIT-emitted with the platform's C convention,
   through which Rust enters a compiled body: from the frame loop, from
   `Vm::eval_lambda` and from §S5's drain helper. It builds the context, calls
   the `Tail` body, and turns the status back into a `Result`.

The adapter's dereference of the context pointer is the second `unsafe` this
tier needs, beside calling JIT-emitted code at all (§S6, *Addressing*). Both
live in `bund2-jit`. The signature is uniform because operands do not travel
in it, which is what satisfies the verifier's rule. §S11's return-value
protocol is the other half: an error travels in the context, never as an
unwind. Criteria 4 and 29 check the boundary.

**What §3.2f does cost Bund is real, but it is not this.** "Force everything
onto the stack" lands here as §S5's rule — promoted values must be synced back
to the VM stack before an inter-word call, because the callee reads them there.
That is a promotion cost, measured by criterion 9, and it would exist whatever
calling convention Cranelift offered.

## What this RFC claims, and what it does not

Two different uses of tail calls sit behind §3.1's sentence "a word body can be
compiled as a chain of tail calls", and only one is claimed here:

- **Claimed: the last word of a body is in tail position.** Lowering it as
  `return_call_indirect` saves a frame per body call, and matters most for
  self-recursive words. RFC-0003's frame loop already makes Bund-level call
  depth cost heap rather than Rust stack, and `Vm::tail_lambda` exists for
  exactly these positions; a compiled body must preserve that property rather
  than reintroduce stack growth Tier 0 does not have. A request the last
  callee files goes back with the return, and the entry takes it at once
  (§S5, *A call may leave a body to run*).
- **Not claimed: threaded code**, in which every word tail-calls the next. That
  is a different compilation strategy with its own register-allocation and
  debugging consequences, and nothing in this RFC depends on it. §3.1's phrase
  describes it; this RFC does not adopt it.

The distinction matters because the second is what makes tail calls
load-bearing in other designs.

## And that is a correctness problem, not an optimisation

An earlier draft ended here with "if `CallConv::Tail` were withdrawn tomorrow
the design would lose a frame per call and nothing else". **That is wrong, and
it is the most serious defect this RFC has had.**

If only the last word of a body is a tail call, **every other inter-word call
consumes a machine frame** — and RFC-0003's flat frame loop consumes none.
That RFC's criterion 2 is not decorative: *"Bund call depth is bounded by heap,
not by the Rust stack"*, **Met** at a call depth of **100,000**, where before
the frame loop the same program aborted between 5,000 and 20,000 (RFC-0003,
acceptance criterion 2). It is cited by number because RFC-0003's amendments
append and move its lines. An earlier revision cited a line range, and after an
amendment moved the text that range pointed into another criterion.
`cite` could not tell, because the lines still existed.

So a self-recursive word that completes at Tier 0 **overflows the machine stack
once promoted**. Three things follow, and none of them is about speed:

- **It is a conformance change.** The program's observable behaviour differs
  between tiers, and CLAUDE.md requires this milestone to move the health
  metric by exactly zero.
- **It is an abort, which D37 forbids** outright. A stack overflow is precisely
  the failure D37 exists to prevent: the process dies, the user's stacks and
  word table die with it, and the trace names machine frames rather than a Bund
  word.
- **It silently un-meets an accepted criterion of another RFC.** RFC-0003's
  criterion 2 would fail with the `jit` feature on, and nothing in this RFC
  noticed.

### The guard

**Every compiled body checks machine-stack headroom on entry and declines if it
is low**, falling back to the interpreter for that call.

This is an *entry* guard, the same shape as §S4's `autoadd` guard and for the
same reason: there is no OSR to bail with mid-body (§3.2b), so the only safe
place to refuse is before the frame is taken. Declining recovers the RFC-0003
guarantee **for direct calls**, which Tier 0's frame loop runs flat: one
compiled frame is spent, and the recursion continues on the heap. It does not
for **native-mediated nesting**. `Vm::eval_lambda`, which `times`, `loop`,
`map`, the conditionals and the method paths use, pushes a frame and calls
`run_to` from inside the native — one Rust frame per nesting — so a compiled
body calling such a word, whose lambda calls a compiled body, alternates Rust
frames. The headroom threshold has to cover that, and criterion 11 gains a case
for it.

Tail calls reduce how often the guard fires; they do not replace it, because
they cover one call site per body. **Q28 sharpens accordingly**: on a target
where `CallConv::Tail` degrades, the guard fires sooner and more often, but
correctness does not depend on the platform.

### How the guard reads the stack — one floor, measured in Rust, compared in CLIF

*Added 2026-09-10. Earlier revisions required this check and gave it no
mechanism.*

Four ways of doing it were ruled out first:

- **Cranelift's own stack limit.** `Function::stack_limit` makes the prologue
  compare the stack pointer against a limit and **trap** on overflow — on x64,
  a `cmpq` against `rsp` followed by `TrapIf` with `TrapCode::STACK_OVERFLOW`
  (`cranelift-codegen` 0.135.0, `src/isa/x64/abi.rs`,
  `gen_stack_lower_bound_trap`). A trap is a hardware fault: with no signal
  handler it ends the process, which D37 forbids, and it is guard-and-bail in
  its hardest form (§S5). It takes its limit from a `VMContext` parameter
  (`src/machinst/abi.rs`, `generate_gv`). The JIT signature's context pointer
  could be declared as one, so that is no obstacle: the trap alone rules it
  out.
- **The `stacker` crate.** It measures remaining stack through `psm`, whose
  build script compiles assembly with a C toolchain. D10 forbids a C toolchain
  anywhere below `bund2 build`.
- **Asking the platform** for the current thread's stack bounds —
  `pthread_get_stackaddr_np` on macOS, `pthread_getattr_np` on Linux — is
  per-platform `unsafe` FFI for a number Bund2 can know without asking.
- **Counting frames instead of bytes.** A counter bounds calls, not bytes.
  Compiled frames differ in size from body to body, and the Rust frames between
  two compiled bodies differ from native to native, so a sound count needs a
  byte bound per frame that nothing provides.

**The mechanism: Bund2 owns the stack it runs on, so it knows where that stack
ends.**

1. **Evaluation runs on a thread Bund2 spawns, with a stack size it chooses**
   — `std::thread::Builder::stack_size`: the standard library, no crate, no
   `unsafe`, no C toolchain. `bund2` does this for every subcommand (`main`,
   `crates/bund2-cli/src/main.rs`). There is no REPL yet, and one would do the
   same. Nothing observable changes: standard output, standard error and the
   exit code pass straight through.
2. **At that thread's entry the runtime records the stack's top**: the address
   of a local in the entry function, taken with `std::ptr::addr_of!` and cast to
   an integer, which is safe Rust. Stacks grow downward on all four targets
   Cranelift supports. Each target's `gen_stack_lower_bound_trap` traps when
   the stack pointer falls *below* its limit (`cranelift-codegen` 0.135.0,
   `src/isa/x64/abi.rs`, `src/isa/aarch64/abi.rs`, `src/isa/riscv64/abi.rs`
   and `src/isa/s390x/abi.rs`), which is only a bound if the stack grows down.
   So the stack's end is the top minus the size.
3. **From those two numbers it computes two floors**, described next.
4. **A check is one comparison against a floor, and it branches — it never
   traps.** Tier 0 compares the address of a local in the function doing the
   check. Tier 1 compares CLIF's `get_stack_pointer`, which is lowered on x64,
   aarch64, s390x and riscv64 (`src/isa/*/lower.isle`, and riscv64's
   `inst.isle`), against the floor loaded from a runtime-owned cell (§S6,
   *Addressing*).

**Two floors, and what they promise (D44).**

- **The Tier 1 floor** sits one reserve above the bottom of Tier 1's share:
  `top − share + STACK_RESERVE`. A compiled body whose entry finds the stack
  pointer below it declines. The floor says where
  compiled frames may *start*, and it reserves no region for them. The stack
  is LIFO, and its outermost frames are always Tier 0's (`run_cli`,
  `Interp::eval`, the frame loop). So Tier 0 frames also sit above this floor,
  whenever a compiled body calls a native that re-enters evaluation.
- **The Tier 0 floor** sits one reserve above the stack's end. Below it, a
  native that would run a body synchronously reports a Bund-level error
  instead of nesting further.

The thread is sized at Tier 0's part plus Tier 1's share. Tier 0's part is
8 MiB — `EVAL_STACK` is that plus `bund2_interp::STACK_RESERVE`, which is the
reserve below the floor rather than part of Tier 0's capacity
(`crates/bund2-cli/src/main.rs`). The main thread's stack
on this machine, 8176 KiB by `ulimit -s`, is the reason for that default,
since it is roughly what Tier 0 ran on before F85's fix. It is not the value.
Compiled frames start only above the Tier 1 floor. The reserve beneath that
floor holds the last compiled body entered and whatever it calls without
re-entering evaluation, so compiled frames never reach Tier 0's 8 MiB. **Tier 0's capacity with the tier on is
therefore never less than with it off**, and it is more whenever compiled
frames are not using the top part. A program whose native nesting fits today
still fits.

**Never less, and not the same.** How much more room Tier 0 gets depends on
what runs, so the level at which a program reports `machine stack exhausted`
can differ with the tier on. An earlier revision also required that level to
be *the same*, and no fixed floor gives both (the seventh review's B1). The
owner decided that the level is not meaning (D44). It already differs between
Bund2's own build profiles: the `loop` axis reports at level 10,923 in release
(`cargo xtask depth`) and 2,371 in dev (`cargo xtask depth --dev`). And the oracle aborts at every
depth (F85). What is meaning is that evaluation nesting never aborts, and that
the level with the tier on is never lower. Criterion 11 checks both.

**The reserve is for the frames between one check and the next.** It is not
for reporting the error, which an earlier revision said: by the time the
error is reported, the stack has unwound (`STACK_RESERVE`,
`crates/bund2-interp/src/lib.rs`). Each floor sits one reserve above the bottom
of its part. The Tier 0 floor is at `top − size + STACK_RESERVE`
(`tier0_floor`, same file). The Tier 1 floor is one reserve above the bottom of
Tier 1's share, so a compiled body entered just above it, and anything it calls
without re-entering evaluation, stays inside that share **provided nothing on
that path recurses on data the program controls**. That proviso was missing
until the seventeenth review measured it: `execute_reached` recursed on a
value's depth at about 550 bytes a level, so the 256 KiB reserve carried about
470 levels and a 20,000-deep list aborted the process (F114, fixed by a heap
worklist). Two more paths outside this design recursed the same way, and
§S8's floors reached neither: dropping a deep value (F115, since fixed the same
way) and parsing a deep run-time string (F116, open), assumptions 34 and 35. Two things come out of
the reserve and are not measured, though both are small against 256 KiB:

- the thread-entry frames, because the declared top is `stack_marker()` called
  inside the spawned closure, below the true top;
- a guard page, on platforms where the size requested includes one.

The reserve rests on an assumption, stated under *What this design assumes*:
everything a leaf native does between two checks fits in it. A native that
recurses in Rust on the depth of its *data*, such as one rendering a deeply
nested value, is not bounded by a floor on evaluation, and this RFC claims
nothing about it.

Proposed defaults: 8 MiB for each part and a 256 KiB reserve. Under `jit`,
Tier 0's part also carries the margin `m` described below, measured by
criterion 11. Like §S7's knobs, these are defaults with a stated basis, and they
change only with a measurement behind the change.

**A decline is a return, not a call.** A compiled body is entered from the
three places a body starts:
- the frame loop, for a body a tail position handed back (`Vm::tail_lambda`);
- `Vm::eval_lambda`, for a native running one synchronously;
- since the ninth review, the drain helper that runs a body a compiled call
  asked for (§S5, *A call may leave a body to run*). The drain helper acts on a
  decline as `eval_lambda` does. Its entry check runs first. Below the Tier 1 floor the function
returns a *declined* status at once, before it touches the stack or holds any
promoted value, so its frame is gone by the time the caller acts on the
status. The frame loop then pushes the body as an interpreted frame and runs
it on the heap. `Vm::eval_lambda` runs it in the frame `eval_lambda` already
holds, exactly as it does with the tier off. A level beneath the floor
therefore costs what it costs with the tier off, plus whatever the feature
adds to `eval_lambda`'s own frame (a cache lookup). It never keeps a compiled
frame alive. Every compiled body entered after that point finds the stack
pointer still below the Tier 1 floor and declines in turn, so beneath the
floor nothing compiled runs, and the recursion continues on the heap. That
recovers RFC-0003's guarantee, and it never needs OSR.

**Room is bytes, and criterion 11 counts levels.** D44's second requirement is
about room: with the tier on, Tier 0 has at least its own part and at most
both parts. Criterion 11 measures levels, which are room divided by the bytes
one level spends. The two agree as long as a level costs no more than
(Tier 0's part + Tier 1's share) / Tier 0's part times what it costs with the
tier off, which is twice at the proposed 8 MiB each. A decline that returns
keeps the per-level cost almost unchanged, so the inequality has close to a
factor of two to spare. A decline that *called* `eval_lambda` would have added
a compiled prologue and a second `eval_lambda` frame to every level beneath
the floor, and the argument would then depend on frame sizes. That was the
eighth review's S2. Criterion 11 measures the result either way.

**When compiled frames already hold the share.** This is the ninth review's
S2, and the owner chose among its options. Take a program that recurses
directly through compiled code until it reaches the Tier 1 floor, and only
then nests through a native.
- **With the tier off**, the direct recursion is flat on the heap, and the
  nesting has Tier 0's whole part.
- **With the tier on**, the share is held by compiled frames, and the nesting
  has the same part.

So the factor of two above is not available to that nesting. Any byte the tier
adds to a Tier 0 level lowers its level below the tier-off one, and D44's "a
program whose native nesting fits … fits under the `jit` binary" would fail near
the limit. Two rules close it:

- **The tier adds nothing to a Tier 0 level below the floor.** The cache
  lookup at `Vm::eval_lambda` is a separate `#[inline(never)]` helper that
  returns before `run_to` is called. So no state the tier adds is live across
  the nesting, and `eval_lambda`'s own frame is the tier-off frame. The same
  holds for the compiled-entry test the loop makes at a frame's first step
  (§S5): it is a helper that returns before the loop goes on, so `run_to`'s own
  frame is the tier-off frame too. The Rust compiler still owns frame sizes,
  so this is a design rule and not a guarantee.
- **So Tier 0's part carries a margin under `jit`, over every re-entering
  path.** Let `c_p` be the bytes one level of path `p` spends with the tier
  off, and `δ_p` what it spends more beneath the floor with the tier on. The
  paths are every way a native re-enters evaluation, and **the set is derived,
  not listed**, because a list went stale within a day (the eleventh review's
  S1), and its prose replacement missed four more (the twelfth's S1). The set
  is every `bund2-stdlib` function that calls `Vm::eval_lambda`, `Vm::apply`
  or `Vm::scoped_call`. The test `every_reentering_function_is_named`
  (`crates/bund2-stdlib/src/lib.rs`) finds it by a source scan, as criterion
  25's finds `Error` reports, and fails until a new re-entering function is
  named. It matches each call in its method form and its path form
  (`vm.apply(` and `Vm::apply(`), reads a function head whatever its
  qualifiers (`pub(super)`, `const`, `unsafe`, `async`), and descends into
  subdirectories. Until the thirteenth review it did none of the three (S2).
  None of the three hid a call. **What it cannot see:**
  - a call made through a function pointer or a macro;
  - shipped code placed after a file's inline test module, since each file is
    cut at its first `#[cfg(test)]` module (assumption 27);
  - anything outside `bund2-stdlib`, which assumption 24 takes up.

  Within those limits the set cannot go stale. Add
  §S5's drain helper. On 2026-09-11 the
  scan finds 21:
  - through `Vm::eval_lambda`: the loop words (`times`, `loop`, `map`,
    `while`, `for`, `*loop`, `input*`), the conditional runners, the method
    paths, `?.` and `?MOVE` (`conditional_move`), and **`csv` and `sqlite`**
    (`run_csv` and `run_sqlite`, `crates/bund2-stdlib/src/data.rs`), which the
    prose omitted until the sixteenth review counted it against the test's
    21;
  - through `Vm::scoped_call`: `context`;
  - through `Vm::apply`: **`eval_source`** (`bund.eval`, `!!`, `use`),
    **`execute_one`** (the arms of `!` and `execute`, handed a name; the
    derived set names this, not its callers `execute_value` and
    `execute_reached`), **`apply`**, and `text`, which applies a TEXTBUFFER
    and so cannot recurse.

  `eval_source` and `execute_one` are paths of their own. Each adds its own
  frame, and `eval_source` a parse and a loop, to `Vm::apply`'s, so their
  `c_p` is not `Vm::apply`'s alone. `execute_one` adds two: the worklist in
  `execute_reached` calls it per value, so a re-entry through `!` spends both
  frames. Criterion 11 reports `c_p` and `δ_p` for each. **`execute_one` has
  two costs, not one**, since F113: a name goes through `Vm::apply`, and a
  lambda a list or dict reached goes through `Vm::eval_lambda`. Its LIST and MAP arms drive a heap worklist rather than
  recursing, since F114, so a value's depth costs no Rust stack and does not
  enter `c_p` at all. Criterion 11 measures its two evaluation re-entries, as
  it does every other path. Then Tier 0's part is
  `8 MiB + m`, with `m ≥ 8 MiB × max_p(δ_p / c_p)`, and the level with the tier
  on cannot be lower whatever holds the share. An earlier revision took `c` and
  `δ` from `loop` alone, and the native with the smallest `c` need not be
  `loop` (the tenth review's S2). `m` is a measurement taken when the
  tier exists, not a number guessed now. It costs address space rather than
  memory, because a thread's stack is committed as it is touched.

Criterion 11 has a case for it.

**§S5's residual path is bounded the same way.** It applies the rest of a body
through `Vm::apply`, which is synchronous. Each value there that runs a body
costs a Rust frame, as any native re-entering evaluation does. So recursion
that passes through residual paths spends machine stack. It stops spending it
at the Tier 1 floor, below which every compiled body declines and the
recursion continues on the heap. `Vm::apply` also checks the Tier 0 floor
before it re-enters (`Interp::apply`, `crates/bund2-interp/src/lib.rs`). The
worst case is therefore a Bund-level error, never an abort.

**Embedders.** A program that runs `Interp` on a thread Bund2 did not spawn
declares that thread's stack with `bund2_interp::set_stack_region` before
building the `Interp`. If it does not, the `Interp` assumes 1 MiB below the
point where it was built, and Tier 1 gets no share there, so compiled code does
not run on that thread. 1 MiB is half of Rust's default for a spawned thread,
since a constructor is rarely at the very top. `std::thread`'s module
documentation, *Stack size*, says "Currently, it is 2 MiB on all Tier-1
platforms". That was read in the 1.94.1 `rust-docs` HTML, and in the `stable`
toolchain's `rust-src` at `library/std/src/thread/mod.rs`, line 129. The
pinned 1.95.0 has neither component. The same section names the
`RUST_MIN_STACK` environment variable, at line 134, as changing that default.

**The arithmetic, for a thread that declares nothing.** The floor is
`marker − ASSUMED_BUDGET + STACK_RESERVE`, 768 KiB below the point where the
`Interp` is built (`tier0_floor`, `crates/bund2-interp/src/lib.rs`). The frames
between one check and the next may run up to one reserve past it, which holds
only while no frame on that path recurses on data a program chooses — an
embedder has less room here than `bund2` does, not more, so F114's measurement
applies with a smaller margin. So the
undeclared case is safe only while at least 1 MiB of stack lies below the
constructor. On a default 2 MiB thread that leaves 1 MiB for whatever sits
above the constructor. An embedder that sets `RUST_MIN_STACK` below 1 MiB plus
that depth, or builds the `Interp` deep in its own call stack, must declare its
region.

**A region declared with `set_stack_region` is Tier 0's alone.** The function
takes a top and a size, and nothing in it says how much of that is Tier 1's.
So under `jit`, such a region gets no Tier 1 share, as an undeclared thread
gets none, and compiled code does not run there.

**A share is declared, never inferred.** This is the ninth review's S3, and the
owner chose among its options. `bund2-interp` gains
`set_stack_region_with_share(top, size, share)`, which names the part of the
region that is Tier 1's. `bund2`'s own thread declares through it under `jit`,
so the tier runs in the CLI. As written before, `main`'s `set_stack_region`
call would have kept compiled code out of `bund2` itself, and criterion 2 would
have passed with nothing compiled. An embedder opts in the same way, and one
that does not keeps all of its region for Tier 0. A default split was rejected:
it would take room from every embedder's Tier 0 without asking, against D44's
"never less". The function is added with the tier.

**Built 2026-09-13 (D62).** `bund2_interp::set_stack_region_with_share(top,
size, share)` exists, the thread-local region carries the share as a third
number, and `bund2`'s evaluation thread declares through it under `jit` with
§S8's proposed default of 8 MiB. The share is added *above* Tier 0's part
rather than carved out of it — `EVAL_STACK` is `8 MiB + 8 MiB + STACK_RESERVE`
under `jit`, and unchanged without it — which is what keeps D44's second
requirement. Plain `set_stack_region` sets a share of zero, so an embedder that
does not opt in keeps its whole region for Tier 0, and three tests pin the
three cases: a declared share, a declared region without one, and an undeclared
thread.

**Tier 0's part is still 8 MiB, not `8 MiB + m`.** The margin is a measurement
criterion 11 takes once the tier exists, and guessing it here is what this
section forbids. The gap is recorded in D62 rather than filled.

**What it costs.** Tier 1 pays a stack-pointer read, a load and a compare per
compiled body entry, under criterion 11's 2 ns bound. Tier 0 pays an address, a
load and a compare each time a native re-enters evaluation — `Vm::eval_lambda`,
`Vm::apply` and `Vm::scoped_call` — calls that already push a frame and run a
loop.

### Tier 0 needs the same floor, and needs it now — F85

`:f { 1 { f } times } register` followed by `f` aborts Bund2 today, and the
oracle with it: exit 134, `thread 'main' has overflowed its stack` (F85).
RFC-0003's frame loop makes *direct* recursion cost heap, but recursion through
a native that runs its body synchronously spends a Rust frame per level (*The
guard*, above). That is a D37 violation with no tier involved at all.

The Tier 0 floor is its fix. `Vm::eval_lambda`, `Vm::apply` and
`Vm::scoped_call` check it before re-entering evaluation, and below it they
return a Bund-level error — reported through `Vm::report` like any other, and
catchable by `?try` — that names the cause: recursion through a word that
runs a lambda, evaluates source or calls a word by name — `times`, `loop`,
`map`, `while`, `for`, `*loop`, a conditional, `?try`, a method, `!` or
`execute` handed a name, `apply`, `bund.eval` or `use` — runs on the machine
stack, and recursing directly runs on the heap instead. (The message was
widened twice: after the eleventh review for `bund.eval`, and after the
twelfth for `!` by name; `Error::stack_exhausted`,
`crates/bund2-api/src/lib.rs`.) **This half does not depend on Tier 1, and
it landed on 2026-09-10**, ahead of this RFC's acceptance. `bund2` spawns its
evaluation thread at 8 MiB plus the 256 KiB reserve, every `Interp::new` takes
its floor from the declared region, and `Error::context` passes the exhaustion
through each body wrapper unchanged, so it is reported once rather than
re-wrapped at every level. `cargo xtask depth`'s new `loop` axis now reports
instead of aborting, and prints the level at which the floor fired: 10,923 in
release on 2026-09-10, and 2,371 under `cargo xtask depth --dev`. Tier 1's share of the stack, and its floor, arrive with
the tier.

## What this design assumes

*Added 2026-09-10. The seventh review listed six assumptions the text relied on
without stating them, the eighth review five more, and the tenth seven more,
answered in 7 and 14–18. The eleventh named three more, answered in 19–21,
and D55 and the twelfth review two more, 22 and 23. The thirteenth named two
more, 24 and 25, the fourteenth two, 26 and 27, and the fifteenth two, 28 and
29. The sixteenth added 30 and 31, the seventeenth 32 and 33, F116 and F115
added 34 and 35, F117 added 36, the twentieth review's B2 added 37, and its S5
and S6 added 38 and 39. Building §S6's cells added 40.* Each
is stated here, with the place that enforces or decides it.

1. **One compiled cache, one `JITModule`, one set of cells and one fragment
   table per `Interp`.** §S6, *Addressing*, and criterion 23.
2. **The reporter's `wants_stack` does not change while a compiled body
   runs.** It can change between runs, since the CLI replaces the reporter
   after construction, so it is read at each entry. No `Vm` method reaches
   it, so it cannot change within one. It is asked per severity (D45, §S5).
3. **The level at which evaluation reports stack exhaustion is not part of a
   program's meaning.** D44, §S8, criterion 11.
4. **Everything a leaf native does between two floor checks fits in the
   256 KiB reserve.** This is not measured. A native that recurses in Rust on
   the depth of its data is not bounded by a floor on evaluation, and this RFC
   claims nothing for it (§S8).
5. **The residual path's `apply` is `Vm::apply`, which is synchronous.**
   Recursion through residual paths spends machine stack until the Tier 1
   floor, and is bounded by §S8's floors rather than by the heap (§S8,
   *§S5's residual path is bounded the same way*). **Where it resumes** is
   assumption 38's.
6. **Every call crossed by promotion resolves through one registry `Slot`.**
   This one is enforced rather than assumed. A call through an alias, or
   against a saturated slot, is synced before (§S5).
7. **Every native promotion crosses has an honest declared effect.** It runs
   no body and moves the current stack by its pair. Criterion 24 checks this
   over the corpus, and criterion 28's palette over fifteen operand kinds and
   the workbench. Promotion crosses only the natives that palette brought to
   `Ok` (D48). Natives from any other crate are not trusted at all (D47).
8. **No `bund2-stdlib` native reports at `Error` severity mid-body.**
   Criterion 25. Other crates' natives are not promoted across (D47).
9. **A callee's effect is trusted across a call only when it is declared.**
   Enforced by D46: nothing stays promoted across a call that resolves to a
   lambda (§S5).
10. **A declined compiled body keeps no frame.** A decline is a return (§S8,
    *A decline is a return*).
11. **The Tier 1 floor sits one reserve above the bottom of Tier 1's share,
    and an embedder's declared region has no Tier 1 share** (§S8).
12. **A call is complete only when any body it asked the loop to run has
    run.** Compiled code drains the request after a non-tail call, and hands it
    to the entry after a tail call (§S5, *A call may leave a body to run*);
    criterion 26.
13. **The tier adds nothing to a Tier 0 level below the Tier 1 floor, and
    Tier 0's part under `jit` carries a margin for what it adds anyway.** A
    design rule and a measurement, not a guarantee, because frame sizes are
    the compiler's (§S8, *When compiled frames already hold the share*);
    criterion 11.
14. **No panic escapes a native.** It is caught where the native is called,
    in both tiers, and becomes `Error::internal` (D49); criterion 29.
15. **`bund2-jit` can obtain the set of registrations `bund2-stdlib` made.**
    `register_all` records the ids it mints, and a replay re-records them
    (§S5, D48).
16. **The margin covers every re-entering path.** `m` comes from the largest
    `δ_p / c_p` (§S8); criterion 11.
17. **The epoch and `autoadd` are read after the drain** (§S5, *A call may
    leave a body to run*).
18. **No native leaves a tail request behind when it fails.** Enforced by
    F96's fix. A refused drain clears the cell too, and so does `status_of`
    when it answers an exit (§S5).
19. **Every way a program stops is visible to compiled code at the helper that
    returns to it, and to Rust where compiled code returns.** A failed
    `Result` is visible through the status. `bund.exit` is not visible by
    itself, since it returns `Ok`. So every helper that runs Bund code for
    compiled code makes its status through one function that consults
    `exit_requested` first: the adapter, the resolving trampoline, the drain
    helper and the residual path's `apply` (§S5, *A call may end the
    program*). On the Rust side, Tier 0 refuses at the top of `apply_step` and
    `Vm::eval_lambda`. Since F112 it also refuses after `run_to` in
    `Vm::eval_lambda`, `Interp::apply` and `Vm::scoped_call`, so a native gets
    the same `Err` from either tier; criterion 30. Today `exit_gate` is the
    only state Tier 0 gates rather than returns, and a second would need the
    same treatment.
20. **`PROMOTABLE.txt` certifies the default registration.** It names
    natives, and `--noio` and `--noeval` register failing stubs under the same
    names. D47's id set comes from the registration actually made, and a stub
    that is crossed fails, so it takes the error path (criterion 28).
21. **The natives the palette leaves unrun are the ones that act on the
    host, and the list of them is kept by hand** (`ACTS_ON_HOST`; D48's
    dated notes). An unrun native is not listed and so not crossed, which is
    the safe side; a new host-acting native runs for real under `cargo test`
    until it is added (criterion 28).
22. **A native that reads beyond its operands shows it to D55's audit**,
    either by returning something different when the values beneath it
    change, or by reading the whole stack or workbench, or a stack's depth by
    name. A native that reads `depth()` and only prints it does neither, and
    is not caught; none is known (criterion 14). The comparison also skips any
    operand tuple holding a lambda or an object, whose display carries an id
    and a stamp (F14); the breach, observation and padding checks still run on
    those. So a native that observes only when handed a lambda, as `curry`
    takes one, is compared on its other tuples alone.
23. **A stack-name operand is among the top two.** The palette varies only
    the top two operands, and deeper ones are always `7`, so a name in third
    place or lower is never `"main"`. That holds for every fixed-effect native
    that takes a name today, all of which consume at most two operands.
    Nothing enforces it for the next one (criterion 28).
24. **Criterion 11's margin covers only `bund2-stdlib`'s re-entering paths.**
    The scan reads that crate alone (§S8). An embedder's native, or an
    embedder's method native reached through `run_init` or `dispatch_method`
    (`crates/bund2-stdlib/src/oop.rs`), may call `Vm::eval_lambda`, and its
    `c_p` is not in `m`. D47 keeps promotion off such natives, but `m` is a
    property of the stack. An embedder that declares a share through
    `set_stack_region_with_share` takes that on (the thirteenth review's S2).
25. **A native that catches an error still runs its handler after an exit.**
    Since F112 every native gets the refusal where the body returns, and most
    pass it up. `?try` first pushes its `error` CONDITIONAL, and only then is
    its `except` body refused (`run_tryexcept`,
    `crates/bund2-stdlib/src/conditional.rs`). Both tiers hand it the same
    `Err`, so both leave the same stack (criterion 30). In `bund2-stdlib`,
    `?try` is the only native that catches the refusal **and then does work**.
    `#` and `#.` catch one and discard it (`object_execute_base`,
    `crates/bund2-stdlib/src/oop.rs`), doing nothing afterwards, which is the
    `Ok` path assumption 26 covers. An embedder's native that catches the
    refusal and then does host work does that work in both tiers. D52's
    "nothing more" holds only for natives that pass the error up.
26. **On the `Ok` path, Tier 0's deferred refusal is made under the same
    wrappers as the compiled one.** Where a next step follows, the frame loop
    is flat, so that step runs inside the same `run_to`. Where the `Ok` was a
    body's last word there is no next step: `run_to` pops the frame and
    returns `Ok`, and the refusal comes from the gate **after** `run_to`. The
    texts agree because `Vm::eval_lambda` applies one `map_err` to both —
    `self.run_to(floor).and_then(|()| self.exit_gate()).map_err(…)`
    (`crates/bund2-interp/src/lib.rs`, `eval_lambda`). Criterion 30's `map`
    rows and its `?try` control row rest on that chain. If the post-`run_to`
    gate ever moved outside the `map_err`, the tiers would part while every
    other stated reason still held. That is why `status_of` substitutes the
    refusal for `Ok` and leaves an `Err` alone (§S5, *A call may end the
    program*).
27. **Every `bund2-stdlib` source file's inline test module comes last.** The
    re-entry scan cuts each file at its first `#[cfg(test)]` module, so
    shipped code after it would go unscanned (§S8). This holds in every file
    on 2026-09-11, and nothing enforces it.
37. **No code bumps a `Slot`'s generation without writing that name's mirror
    cell.** §S6 gives compiled code a cell per name, and a write that reaches
    the `Slot` alone leaves an inlined fragment running a meaning the name no
    longer has — which moves `conform` under `--features jit`, where a stale
    request cell only ran a body late. The property is **structural rather
    than enumerated**: `Registry::touch(&mut self, s: Symbol)` is the only
    function that bumps a generation, so the mirror write has one home beside
    the bump, and `Slot::bump` is private to it.
    `every_writer_of_a_slot_generation_is_named`
    (`crates/bund2-api/src/lib.rs`) holds that set to one function and fails
    when a second appears. **What the scan cannot see**, as assumption 33's
    does: it reads `crates/bund2-api/src/lib.rs` alone, cuts at the first
    `#[cfg(test)]` module, and matches the spellings `.bump()` and
    `self.generation =`. The seventeenth review's B1 is why this is derived and
    not asserted: an enumerated mirror-writer set went stale within a day.

    **The boundary, stated.** This is a `Slot`'s generation and nothing else.
    `Registry` carries a second pair, `class_generation` and
    `method_generation` (`crates/bund2-api/src/lib.rs`), written by
    `register_class`, `unregister_class` and `register_method` and read through
    `oop_generation`. §S6 mirrors the per-name cell alone, so those two are
    outside this assumption — and a lowering that ever inlines a method
    dispatch owes them the same mirror argument before it does.
28. **A list literal does not evaluate its items.** `[ 1 2 + ]` keeps `+` as a
    CALL value (run 2026-09-11), so `execute_one`'s arms that do Rust work
    before re-entering evaluation — CLASS, OBJECT, CONDITIONAL — are not
    reachable from a literal, and neither is its MAP arm. Its other arms are
    LIST, LAMBDA and the name arm. (`execute_value` delegates to
    `execute_reached`, whose worklist calls `execute_one`; the arms are all in
    the last.) F112's "no golden moves" rests on this.
    **A literal is not the only way a program gets a LIST**, and the first
    word that returns a list of dicts makes the MAP arm reachable from source
    without touching `values.rs` (the sixteenth review's B1).
29. **A native that files two tail requests loses the first.**
    `Interp::request_tail` assigns. No `bund2-stdlib` native does that now:
    the one that did, `execute_one`, files only when the program executed a
    lambda itself — its `Reach::Top` arm — and runs one at once when a list or
    a dict reached it
    (F113, completed after the sixteenth review's B1). The reached half is the
    reference's — its one LAMBDA arm runs the body at once, whichever arm
    reached it
    (`reference/rust_multistackvm/src/stdlib/execute.rs:93-95`). The filed
    half is a deviation from it, taken because the frame loop makes a tail
    position cost no Rust frame, and unobservable in ordering because a
    request runs before the next value (§S5). The other
    request sites file once and return. An embedder's native can still file
    twice. The effect audit records a tail request filed by a native that
    declares a *fixed* effect (`Interp::request_tail`'s `audit_breach`), so
    the case nothing detects is an opaque native filing twice. §S5's "a
    request is never lost" is a rule about compiled call sites, not a property
    of Tier 0.
30. **A native may recurse in Rust between two floor checks, and the reserve
    does not carry an unbounded one.** Assumption 4 scopes its claim to a leaf
    native. `execute_reached` was not one: its LIST and MAP arms recursed into
    themselves on the *depth of the value*, with `stack_ok` consulted only
    where an arm reached `Vm::apply` or `Vm::eval_lambda`. An earlier revision
    of this assumption said `STACK_RESERVE` carried those frames. **It does
    not**: the seventeenth review measured about 550 bytes a level, so the
    256 KiB reserve carries about 470 levels, and
    `list 20000 { drop list push } times !` aborted the process — a D37
    violation, filed as F114 and fixed by driving the traversal from a heap
    worklist. Data depth now costs no stack on that path. The general claim
    stands: a native that recurses in Rust between floor checks is bounded by
    the reserve and by nothing else, so a new one must not recurse on data a
    program controls. **The re-entering frames on this path are
    `execute_reached`'s and `execute_one`'s**, since the worklist calls the
    latter per value, so `c_p` counts two (the eighteenth review's S3).
31. **The entry trampoline makes no status of its own.** §S5 gives four
    helpers `status_of` and treats the entry separately, because the entry
    only converts a body's status rather than making one. An entry path that
    grew a step after the body returned would bring the twelfth review's B1
    back at the entry.
32. **`#` and `#.` are the whole of the discarding family.** Assumption 25
    names them as the `Ok` path. The spelling is `let _ =` around a
    re-entering call, and no scan or audit looks for a third
    (the sixteenth review's §4.3).
33. **No code writes `pending_tail` outside the named set.** The set is
    `request_tail`, `take_pending`, `Interp::invoke` (F96) and
    `Vm::clear_tail_request` (D56), and §S6's *Addressing* requires the
    compiled mirror to be written wherever `pending_tail` is. A write from
    anywhere else would leave the mirror behind Tier 0 with nothing to
    notice. This is derived rather than asserted:
    `every_writer_of_the_request_cell_is_named`
    (`crates/bund2-interp/src/lib.rs`) scans for writes and fails when a fifth
    appears. An earlier revision of this assumption named one writer where the
    code had three, and so could not have caught what it was added for (the
    seventeenth review's B1). **What the derivation cannot see**, as every
    sibling scan in this RFC states for itself: it reads
    `crates/bund2-interp/src/lib.rs` alone, so a write from that crate's
    `frag.rs` passes; it cuts at the first `#[cfg(test)]` module, as
    assumption 27 says for the other scan; and it matches two spellings,
    `pending_tail =` and `pending_tail.take()`, so `replace`,
    `get_or_insert_with`, a `mem::swap` or a `&mut` handed to a helper would
    not be seen. **A fourth:** its head parser matches qualifiers by exact
    equality — `["pub", "const", "unsafe", "async", "extern"].contains(&w)` —
    so a write inside a `pub(crate) fn` or `pub(super) fn` is attributed to
    whichever function was parsed last, silently rather than as a failure.
    §S8's re-entry scan and assumption 37's use `starts_with` and do not have
    this gap, which makes it easy to carry the wrong property across (the
    twentieth review's S3). `crates/bund2-interp/src/lib.rs` holds only `fn `
    and `pub fn ` heads today, and nothing enforces that. The half no scan can ever cover is `bund2-jit`'s: `status_of`
    and the drain helper are given `Vm::clear_tail_request` (D56) and nothing
    derives that they call it, because the crate does not exist yet (the
    eighteenth review's S4).
34. **§S8's floors are silent about parse depth, and a bound stands in for
    them.** They bound evaluation nesting and compiled entries. The parse of a
    *run-time string* handed to `bund.eval`, `!!` or `use` recurses on that
    string's depth with no floor between, and a 16,000-deep literal aborted
    the process. F116 bounds the parser at `MAX_NESTING`, 1024 blocks, refused
    before the frame is spent, which is 512× the deepest nesting anywhere in
    the corpus. RFC-0003 excludes the parser's recursion over a source file;
    this was never that, because the string is a value the program built.
35. **§S8's floors are silent about a value's teardown, and nothing on that
    path may recurse.** No floor can be put there: a drop runs wherever the
    value dies, long after any check. Dropping a deeply nested value used to
    recurse on its depth, and a 30,000-deep list aborted after the program's
    own work had finished and its output had printed (F115, fixed by an
    iterative `Drop` for `HeapValue`). **The walks over a value's members are
    seven**, and the nineteenth review found the previous revision of this
    sentence wrong in both directions, so they are listed:
    `Drop` (F115, iterative), `render_into` (F117, iterative), `display`
    (F119, iterative), the **wire codec** (`crates/bund2-value/src/wire.rs` —
    F118: an arena for the half Bund2 owns, and a bound for bincode's, which
    cannot be checked on read because the nested value is built before any
    Bund2 code runs. **Encoding** refuses past `MAX_WIRE_DEPTH`, 256, in
    `to_binary`'s own `depth_of` walk, so `save.model` reports at 257 and never
    reaches bincode's encode recursion — the arena's ~39,000 is what the
    encoder could take, not what a program can reach. **Decoding** has no depth
    check: `load.model` reads a world file Bund2 wrote and therefore bounded,
    while `sql_cell` reads a BLOB from a file Bund2 did not write and refuses
    one over `MAX_BLOB_BYTES`, 16 KiB, with D58 recording the residue as a
    stated exception to D37. **`depth_of` undercounts a JSON payload, so the
    walk it measures is not the walk bincode makes.** `children_of` gives
    `Payload::Json` no members, so a JSON value counts as **one** level
    however deeply its own structure nests, while bincode's encode descends
    `serde_json::Value` in full — the reference wraps only a *top-level* JSON
    value as `JSON_WRAPPED` text, and one nested in a list "is written as
    JSON", as `to_binary`'s own comment records. What bounds that path is
    therefore not `MAX_WIRE_DEPTH` but **serde_json's parser, which refuses
    past 127 levels**: `json` on a string of 128 nested arrays reports
    `recursion limit exceeded at line 1 column 128`, where 127 parses and
    prints, measured 2026-09-12. The bound holds, but it is a different
    bound than the one this assumption names, and only the shallower walk is
    checked (the twenty-first review's S3)),
    `summarise` (bounded at depth 2 by design, D36), and `PartialEq` and
    `Hash`, which compare and hash a container by `identity()` and never
    descend (`crates/bund2-value/src/lib.rs`). Two of the seven recursed when
    the eighteenth review's answer claimed one did.
36. **Rendering a value walks its depth, and that walk is on the conformance
    path.** `debug.display_stack` and `--raw-values` render in full, where a
    report's values go through `BundValue::summary`, bounded at depth 2 by
    design (D36). The seventeenth review called the unbounded half unmeasured;
    the eighteenth measured it, and it aborted at 24,000 levels where 20,000
    exited 0 — F117, fixed by rendering from a worklist so the output is
    byte-identical at every depth. The golden capture epilogue calls
    `debug.display_stack`, so this was never an embedder-only corner.
38. **A compiled body carries a resume index per call site.** §S5's residual
    path applies "the rest of the body's values", which is a position in the
    body's `Vec<BundValue>` — and the compiled ops are not in bijection with
    those values, since §S6 inlines fragments, promotion elides pushes, and
    criterion 10's spike records Cranelift folding `1 2 +` outright. So the
    lowering carries, for every point at which the residual path can be
    entered, the index into the source body at which interpretation resumes,
    **and no folded or inlined region spans a call site**. The second half is
    what makes the first implementable: residual entry is only ever after a
    call, and at a call every promoted value below the callee's arity is
    either synced or provably live, so a folded region cannot straddle one.
    Criterion 21 asserts the side table and not only the stacks; without that
    it would pass a lowering that resumed at the wrong index on a seventh
    program (the twentieth review's S5).
39. **The workbench is never a promotion source.** Promotion is described
    throughout in terms of stacks — "the stack it was taken from", the
    current-stack epoch, `Stack::push`'s path for the tag — and the workbench
    appears only as something *audited*. That silence is now a statement: a
    compiled body promotes only values it would have on the **current stack**,
    so a `.`-family native's workbench operand is on the workbench when it
    runs, always, because it was never in a `Variable` and so has nothing to
    sync. The guard cannot reach it either — `Fragment::admits_with` is asked
    through `i.depth()` and `i.peek_at(n)`, both the current stack
    (`crates/bund2-interp/src/frag.rs`, `frag::run`), and no op on that path
    touches the workbench. This matters because "the stack it was taken
    from" reads naturally as including the workbench, and a reader
    implementing §S5 from that phrase could promote a value destined for the
    workbench and find no epoch, no sync rule and no barrier for it.
    `tests/golden/PROMOTABLE.txt` lists **52** `.`-suffixed natives among its
    222 entries, every one of which takes or leaves a workbench operand; `+.`
    declares `eff(1, 0)`
    (`crates/bund2-stdlib/src/math.rs`, `add_wb`'s registration), honest
    about the current stack and silent about the workbench, which is the
    convention the audits check. Nothing derives this — it is a constraint
    on a lowering that does not exist yet, and criterion 22 is where a
    breach would surface (the twentieth review's S6).
40. **No code changes which stack is current without moving the epoch.**
    §S6's guards re-read the epoch after every call, and a switch that did not
    move it would leave a compiled body holding values promoted from a stack
    that is no longer in force — the sixth review's `1 2 "s" to_stack +` with
    the check defeated. The property is **structural rather than enumerated**,
    as assumption 37 is: `Stacks::order` is private to `Stacks`, and every
    mutator that can move its front bumps the epoch, so a switch that does not
    bump cannot be written.

    It had to be made structural rather than asserted, because it was already
    violated in the direction that matters: `drop_stack`, `rotate_stacks_left`
    and `rotate_stacks_right` on `Interp`'s `Vm` impl reached *past* `Stacks`
    into `order` directly. Instrumenting only `Stacks`'s own methods would have
    left three silent switches. They now call sealed mutators.

    **The bump is exact.** `to_stack` to the stack already current, a rotation
    of a one-stack ring, and a drop of a stack that is not current change no
    front and do not bump; a compiled body must not take a guard failure from a
    program that never switched. `every_way_the_current_stack_changes_moves_the_epoch`
    (`crates/bund2-interp/src/lib.rs`) asserts both halves.

# S9. Tier pinning

Two later RFCs need the same mechanism: RFC-0007 must pin a word to Tier 0
across an await point, and RFC-0008 must pin one to Tier 0 while a breakpoint
is set in it. The roadmap assigns it jointly to RFC-5, RFC-7 and RFC-8 as "one
mechanism" (`docs/research/05-rfc-roadmap.md:243`).

This RFC specifies **the mechanism and not its policy**: a body may be marked
`pinned`, which makes it ineligible for promotion and demotes it if already
promoted. What causes a pin for async is **D6 and D7's**, both OPEN and both
declaring `Blocks: RFC-0007`; this RFC does not anticipate them.

# S10. The feature gate, and portability

Cranelift targets x86-64, aarch64, s390x and riscv64 — no 32-bit x86, no 32-bit
ARM (§3.2c). **The interpreter is therefore the portability story**, and Tier 1
must be a `cargo` feature that compiles out cleanly, exactly as `string.grok`
is (D40).

D10's resolution is the governing rule and it cuts both ways here: `bund2 build
--emit=native` may require a C toolchain, and **nothing below `bund2 build` may
require one**. Cranelift is pure Rust and needs no `cc`, so the JIT feature
does not violate D10 — but it must not drag in anything that does.

Exact version pins are already in place (`=0.135.0`) and §3.2e's instruction —
"pin exact versions; budget for periodic migration work" — is honoured by the
workspace as it stands.

**The migration was owed and has now been paid — F80.**
`cranelift-codegen@0.135.0` requires rustc **1.95.0** while
`rust-toolchain.toml` pinned **1.90.0**, so `--features jit` failed before
compiling any Bund2 code, in every crate gating on it. The toolchain pin is now
**1.95.0** (Q31, decided by the repository owner), which was the only option of
three that keeps Tier 1 the same build as Tier 0 rather than a second one.

`--features jit` and `--features aot` are declared in five crates —
`bund2-jit`, `bund2-runtime`, the umbrella `bund2`, `bund2-cli` and
`bund2-bench`. `conform --features jit` builds `bund2-cli` with the feature,
and **criterion 2 has run**: 105/113 with the feature on and 105/113 with it
off, ceiling 105/113 in both, re-run on 2026-09-11 after that day's probes.
It read 79/86 before three probes were added on 2026-09-10, 82/89 before a
fourth, an approved deviation (D50), and 82/90 on the morning of 2026-09-11.

**D9, as amended 2026-09-09, applies**: no Cranelift type may appear in
`bund2-stdlib` or `bund2-api`, which is what §S6's fragments are shaped around.
D9's original sentence — no third-party CLIF lowerings — is about the stable
ABI and still holds: an external package gets `Native` with a declared effect
and no more, and cannot publish a fragment.

**AOT conformance is RFC-0006's**, and it inherits this RFC's invariant
unchanged: the output of `bund2 build --emit=native` must read the same N/M
and CEILING as Tier 0, because AOT changes speed and not meaning exactly as the
JIT does. Criterion 2 here covers the JIT; RFC-0006 states the AOT
counterpart.

# S11. Errors stay a return-value protocol

`try_call` / `try_call_indirect` exist but the unwinder story is incomplete
(§3.2g). Bund2's errors are already `Result`-shaped and route through `Vm::report`
as `Diagnostic`s (D36); compiled code keeps that convention and does not adopt
native unwinding. Under D37 this is also the only option that cannot abort.
A panic in a native is caught where the native is called, in both tiers
(D49), so nothing unwinds into or through a compiled frame. The error travels
in the call context (§S8).

## Preservation analysis

Tier 1 preserves everything by construction, because it adds no word and
changes no word's behaviour. The risks are all of the form "compiled code
disagrees with interpreted code", and each has a named guard:

| risk | guard |
|---|---|
| stale cache entry after a body is freed | the cache's `Weak` keeps the address out of reuse, and a dead entry is swept (D35 as amended); criterion 3 |
| a redefined word still calling old code | all calls indirect through a slot (§S4) |
| **an inlined fragment after its name changes meaning** — re-registered, aliased, shadowed by a lambda, made a command or unregistered, including by a `register`, `unregister` or `alias` earlier in the same compiled body | per-site meaning guard: the slot's generation and `autoadd`, re-read before the first op (§S6); criteria 5 and 17 |
| a **type** specialisation taking a path the interpreter would not | guard-and-branch, generic counterpart in the same function (§S5) |
| a compiled call running under `autoadd`, where the reference would collect the name instead | entry guard on the flag, and a per-site check at every inlined fragment; `:` and `;` are opaque sites (§S4, §S6); criterion 18 |
| **a promoted recursion overflowing the machine stack where Tier 0 runs it on the heap** | a stack floor Bund2 measures on a thread it spawns, compared against `get_stack_pointer` at every compiled body's entry; below it the body declines along the path an interpreted body takes (§S8); `cargo xtask depth` at 100,000 with the feature on, criterion 11 |
| **native-mediated recursion overflowing the machine stack in Tier 0 itself** — through a word that runs a lambda, evaluates source or calls a word by name, the set `every_reentering_function_is_named` finds (§S8) — which aborted until 2026-09-10 (F85) | the Tier 0 floor, built, and checked wherever a native re-enters evaluation; below it a Bund-level error, never an abort from evaluation nesting (§S8). A native recursing in Rust on the depth of its data is outside that guarantee (*What this design assumes*, 4); criterion 11's `loop` axis |
| the tier taking stack that Tier 0's native nesting would have had | the thread is sized at Tier 0's part plus Tier 1's share, and compiled frames start only above the Tier 1 floor, so Tier 0 never has less room with the tier on (§S8); criterion 11 checks that the `loop` level is no lower with the feature on |
| the level at which `machine stack exhausted` is reported, which moves with the build profile and with the feature | not meaning (D44): never an abort, and never lower with the tier on; criterion 11 |
| a value synced back from a `Variable` losing its D41 stack symbol, so a golden renders `tags: {}` | the sync writes through the same path `Stack::push` uses, not a bare `push_back` (§S5); criterion 12 |
| a slot naming a spelling rather than the resolved target, when `i` resolves aliases twice | slots key on the resolved name (§S4), **for every spelling including `$`**: `Registry::follow` walks to a fixed point and `Registry::resolve` calls it either way, so Tier 1 keys on what Tier 0 dispatches to. The reference resolves `$name` one level shallower; not reproducing that is RFC-0002's approved deviation, asserted by its criterion 6a (§S4, *The chain*) |
| `unregister` against a name a compiled body still calls | the call slot is rewritten to what the name now resolves to — the native a lambda shadowed, if any — and to a failing stub only when nothing resolves; never freed (§S4) |
| `alias` retargeted after a caller was compiled | a name reached through an alias is never cached: its call slot points at the resolving trampoline (§S4, *Two structures*), so a retarget is seen on the next call and there is nothing to fan out |
| a diagnostic raised in compiled code carrying no Bund source location | D36 requires a `Diagnostic` with a Bund location; compiled frames have none unless the lowering carries spans, which §S11's return-value protocol must thread |
| an **opaque** site leaving a stale promoted value behind | promotion stops and syncs to the real stack before the call (§S5); cost checked by criterion 9 |
| mixed-kind comparison lowered to a machine compare | permitted since D33 resolved to exact ordering, but only as a guarded **fragment**, never a bare `fcmp`: `exact_int_float_ord`'s shape is a widening compare where the integer converts exactly, the float's floor above 2^53, ±∞ special-cased, and false to all four operators for NaN (§S6) |
| a `*`-family word crossing a promoted region — including one bound at run time to a name the body was compiled against | an `opaque` effect (D12; the ten folds are registered, each opaque, since 2026-09-11), and the pre-call slot-generation check, which syncs before any call whose binding changed (§S5, *What a promoted value must not change*); criterion 22 |
| **a callee reached through an alias or `$name` whose target is rebound while values are promoted across the call**, such as `<-` → `stacks_left` with `stacks_left` rebound to a lambda that consumes two | nothing stays promoted across a call that does not resolve through its own registry `Slot`, or whose slot's generation is saturated: the body syncs before it (§S5); criteria 5 and 22 |
| an error returned from compiled code while values are promoted, whose report or `[BUND]` stack dump would show a short stack | every error return syncs first (§S5); criterion 22 |
| a native that reports with a stack snapshot while values below its arity are promoted | the reporter says per severity whether it wants a snapshot (D45); while it wants one for `Warning` or `Notice`, nothing stays promoted across a call, and this is read at each entry (§S5); Q34; criterion 22 |
| a promoted value's `.id` or `.timestamp` | both are lazy (D1, D2), and a stamp is observed only after a sync. Once a guard admits heap values, a heap value's mint order may differ from Tier 0's; that is not meaning, because ids are opaque and F14 normalises them (§S5). A fragment that compares or hashes must materialise identity where its word does — criterion 16 |
| a warning or notice from compiled code | compiled code emits none of its own; a fragment has no reporting op, so an arm that reports stays a call (§S5) |
| unbounded code memory | caps and permanent demotion (§S7) |
| a fragment disagreeing with its word | differential test per fragment over the arm's boundaries, identity included (§S6); criterion 16 |
| an op failing after its guard admitted, with operands already pulled | `Fragment::new`, the only constructor, refuses such a fragment; anything that escapes is `Error::internal`, never a fall-through and never silent success (§S6); criterion 19 |
| a word reading beyond its declared arity while values are promoted | a promotion barrier. D55's audit, a four-run differential and an observation audit inside criterion 28's palette, keeps every such native off `PROMOTABLE.txt`, and promotion syncs before any native not listed; criterion 14 |
| the lowering and `frag::run` disagreeing about what a fragment means | criterion 16's third leg, required before a lowering ships |
| a body entered through a loop word, conditional or method path never reaching the tier, because no `Rc` survived to the entry | D42: `Vm::eval_lambda` and `Vm::tail_lambda` take the value, and the frame holds it; criterion 20. `Vm::scoped_call` is the exception: its body is a LIST built per call, so a `context` body has no key and stays at Tier 0 (§S3) |
| a **current-stack switch** mid-body — `to_stack`, `to_current`, `stacks_left`, `stacks_right`, `endcontext`, a scoped conditional, a CONTEXT literal — while values are promoted | the current-stack epoch, re-read after every call, and a static barrier at a CONTEXT literal; the residual path syncs each value to the stack it came from (§S5); criterion 21 |
| a named-stack word — `swap_in`, `move_from`, `rotate_stack_*` — reaching the current stack by name while promotion holds its values | a promotion barrier. The palette includes the current stack's name, so D55's differential sees such a word change what it leaves when the values beneath change, and the observation audit records `depth_of`; D55 keeps these natives off `PROMOTABLE.txt` (its first run found `swap_in`, `move_from`, `rotate_stack_left` and `rotate_stack_right`, and F111's six miscounted pairs); criterion 14 |
| `autoadd` turned on mid-body, when the reference collects literals as well as calls, and pushes a CONTEXT value rather than switching to it | re-read after every call; the residual path applies the rest through `apply` (§S5); criterion 18, against a reference-captured probe |
| unregistering a lambda that shadowed a native | the call slot is rewritten to the revealed native, not stubbed (§S4) |
| a compiled body substituted at `eval_lambda`, whose errors Tier 0 wraps as `Lambda content evaluation returned error: …` and `times` wraps again as `TIMES: lambda execution returns error: …` | the compiled body returns its error unwrapped and the entry wraps it, so both prefixes come from the same code whichever tier ran (`Vm::eval_lambda`; `times_base` in `crates/bund2-stdlib/src/seq.rs`) |
| an eval'd string's inner lambda recompiled on every evaluation | the 1024-body cap (§S7); content-hash keying is the eventual answer (§S3) |
| a guard widened to boxed values, whose operands might carry a `q` other than 100.0 | no arithmetic word averages `q` (D32 as amended, Q35), so the result is a fresh 100.0 either way; criterion 16 must then include such an operand if one can be built (§S6, constraint 2) |
| a body compiled under one `Interp` and run by another on the same thread | one cache, `JITModule`, set of cells and fragment table per `Interp`, so another `Interp`'s code is never found (§S6, *Addressing*); criterion 23 |
| **a lambda callee whose inferred effect changes**, because a word its body calls is rebound before the call or by the callee during it | nothing stays promoted across a call that resolves to a lambda (D46, §S5); criteria 5 and 22 |
| **a native whose declared effect hides a body it runs** — `execute.` until F87, `object` and `display` until F91 | every native that runs a body declares `StackEffect::opaque`, checked by criterion 24's audit over the corpus, and by criterion 28's palette for the words and arms the corpus never reaches |
| **a native whose declared pair miscounts the current stack** — `clear`, `fold`, `move`, `dup_many`, `format`, `pull`, `fold_stack`, `swap_in`, `print.` and `println.` until F92, `drop_stack` until F94 | criterion 24's audit compares each fixed-effect native's depth change with its pair, as RFC-0004 §S1 reads it, and criterion 28's palette does the same over fifteen operand kinds and the workbench; promotion crosses only the natives the palette brought to `Ok` (D48) |
| **a native reporting at `Error` severity mid-body**, under a reporter that wants fatal snapshots | no shipped `bund2-stdlib` code reports at `Error`, checked by a source scan (criterion 25); no other crate's native is promoted across (D47) |
| **a native `bund2-stdlib` did not register** — an embedder's, or an external package's under D9 — whose declared effect nothing checks | promotion crosses only natives whose registration id `bund2-stdlib` made (D47, §S5); criterion 27 |
| **a body a call asked the loop to run** — `!` on a lambda, `if`, `ifthenelse`, a lambda call — running after the next value, or overwritten by a later request | the request cell, read after every call, with a drain for a non-tail call, a hand-back for a tail call, and an internal error if a request is ever pending when a call starts (§S5); criterion 26 |
| the per-level stack cost of a declined compiled body below the Tier 1 floor | a decline is a return, so no compiled frame is kept (§S8); criterion 11 |
| **native nesting that starts beneath compiled frames holding Tier 1's share**, where any byte the tier adds to a Tier 0 level lowers the level at which exhaustion is reported | the cache lookup returns before `run_to`, and Tier 0's part under `jit` carries a measured margin `m ≥ 8 MiB × max_p(δ_p / c_p)` over every re-entering path (§S8); criterion 11's share-held case |
| **a lambda that shadows a native**, whose name `effect_of` answered with the native's effect until F93 | callees are classified by `Registry::resolve`, and `effect_of` follows the same order (§S5, F93); criterion 22 |
| **the tier never running in `bund2` itself**, because its thread's declared region had no share, so that the meaning criteria pass vacuously | `bund2` declares its share with `set_stack_region_with_share` (§S8); criterion 2 requires compiled bodies, and runs again at threshold 1 |
| **a native reporting at `Error` severity through a spelling the source scan misses** | the effect audit records any `Error` report made while a native runs (criteria 24 and 25) |
| **a native panicking inside a dependency** — `jarowinkler` in `natural` (F95) — reached from compiled code | caught where the native is called, in both tiers, and reported as `Error::internal` (D49); no panic unwinds through a compiled frame (§S8); criterion 29 |
| **the call boundary itself**: a `NativeFn` whose ABI and `Result` CLIF cannot carry | a context pointer and an integer status under `CallConv::Tail`, a Rust adapter per native, `Tail` thunks in the slots, and a C-convention entry trampoline (§S8); criteria 4 and 29 |
| **a lambda a container reaches, now run inside `execute_one`'s `Reach::Nested` arm where it used to be filed** (F113) | the answer is the same, since a request runs before the next value (§S5), and the cost is one Rust frame per lambda rather than none. Its traversal spends no stack on the value's depth (F114), and `Vm::eval_lambda` holds the floor; criterion 11 |
| **a deep value rendered by a word on the conformance path** — `debug.display_stack`, which the golden capture epilogue calls, and `--raw-values` | `render_into` and `render_payload` drive a worklist rather than recursing, so the depth costs heap and the text is byte-identical at every depth (F117). A diagnostic's values are bounded at depth 2 by `summary` instead (D36); criterion 14 |
| **the wire codec walking a value's depth on a value Bund2 wrote** — `save.model` encoding, `load.model` decoding a redb world file | an arena for both directions, and `MAX_WIRE_DEPTH` refusing to *write* past 256 levels, so a world file never holds a value deeper than the decoder takes (F118). Neither word is audited: `save.model` is in `ACTS_ON_HOST` and `load.model` is `opaque(2)`, which the palette filters |
| **the same codec on bytes Bund2 did not write** — `sqlite` decoding a BLOB from any SQLite file a program names (`sql_cell`) | no write side exists to bound, and bincode's derived `Deserialize` builds the nested value before a depth check could run: a 174 KB BLOB nested 3,000 deep aborts (measured). `sqlite` refuses a BLOB over `MAX_BLOB_BYTES`, 16 KiB — the write-side bound read through size, at 58 bytes a level — and D58 records the residue as a stated exception to D37. `sqlite` is `eff(1, 1)` and `run_sqlite` *does* re-enter evaluation, so this path is inside §S8's re-entering set and still outside every audit that would see the decode |
| **a value's teardown recursing on its depth** — a program that printed its output aborts while its stacks are dropped | nothing in this design: §S8's floors are on evaluation and on compiled entry, and a drop runs where the value dies. `Drop for HeapValue` walks the levels through a worklist instead (F115); assumption 35 |
| **the parse inside `bund.eval`, `!!` and `use` recursing on the depth of a run-time string** | the floor `eval_source` checks comes after the parse, so the parser bounds itself instead: `MAX_NESTING` refuses past 1024 blocks before the frame is spent, and reports (F116; assumption 34) |
| **a stale tail request** left by a native that failed after filing it | at Tier 0 `Interp::invoke` clears it (F96). A compiled call does not reach `invoke`, since the adapter calls the `NativeFn` directly, so `status_of` clears the cell whenever it answers an error, and a refused drain clears it too (§S5); criterion 26 |
| **native nesting through a word other than `loop`**, whose per-level cost makes the margin too small | `m` is set from the largest `δ_p / c_p` over every re-entering path, a set derived by source scan rather than listed (§S8); criterion 11 |
| **the error value a Rust caller receives after an exit, when the helper that saw it returned `Err`** — `?try` keeps its text in the `error` CONDITIONAL's `context` slot, on a final stack that is meaning | `status_of` parks the helper's own `Err` unchanged, and substitutes `exit_gate`'s refusal only for `Ok`. So the wrappers of the natives inside a compiled body (`MAP:`, `TIMES:`, `Attempt to evaluate value …`) survive as they do at Tier 0; criterion 30's `?try` cases compare the `context` slot |
| **a compiled body entered from Rust whose last action records an exit** — a native such as `map`, `?try` or `input*` ran it, and acts on the `Result` it gets back | the entry returns the error status `status_of` made, and since F112 Tier 0 returns the same refusal where a synchronous run returns to Rust; a handed-back body is drained by the entry and then gated the same way, so the tier agrees with itself; the state left after the exit is meaning (D52's dated note); criterion 30's mirror cases |
| **a program ended by `bund.exit` (D52) while a compiled body runs** — the native returns `Ok`, and Tier 0 stops only at its next step, which compiled code does not take | every helper that returns to compiled code — the adapter, the resolving trampoline, the drain helper and the residual path's `apply` — makes its status through one function that turns a recorded exit into the error status, the entry trampoline refuses to start a body, and compiled code's error path syncs and returns (§S5, *A call may end the program*); the effect audit records a fixed-effect native that requests an exit; criterion 30 |

## Alternatives considered

**Build Tier 1 before the representation work.** This was the live question when
this RFC was drafted, and it is now moot: D41 did the representation work, and
§S1's update records a 3.8–4.2× improvement per program shape from that alone.

It is kept as an alternative because the *reasoning* still applies to whatever
comes next. A compiled lowering of `push` written before D41 would have encoded
the expensive shape into generated code, making the representation fix harder
rather than easier — and the cheapest thing a JIT can do is not be asked to
compensate for a representation that has slack left in it. Whether slack
remains is criterion 10.

An earlier version of this entry rejected the alternative on an Amdahl bound of
"~1.3× at best". That figure is withdrawn (§S1) and this entry no longer rests
on it.

**A threaded interpreter instead of a JIT.** §2.1 rates this ~2× and it needs
no code generator, no platform restriction and no code-memory policy. It is a
genuine competitor to this RFC and it is **not** ruled out here — but it is
RFC-0003's territory (Tier 0's implementation), not this one's. §S1's
prerequisite may make it the better next step, and this RFC should not
pre-empt that.

**Content-hashed cache keys instead of pointer keys.** D35 considered and
rejected this for now; it remains "a strict upgrade" whose only cost is the key
function. Not reopened here.

**Module rotation to reclaim code memory.** Needs a shadow stack to prove no
orphaned frame is live. Out of scope for v1, per §3.2a.

## Acceptance criteria

Each names the tool that decides it **and either a threshold or a boolean
outcome**. Criteria 1, 7, 9, 10, 11 and 17 carry a number; the rest are boolean by
nature — a redefined word is observed or it is not, a cap holds or it does not,
`cite` exits 0 or it does not.

An adversarial review found 4, 6 and 7 naming neither, which is how a criterion
goes vacuous: 7 read "shows improvement", and nothing fails that. **Criteria 1
and 2 gate the RFC's own premise**; the rest are for the implementation.

**The `--features jit` criteria became runnable on 2026-09-09**, when the
toolchain pin was raised to 1.95.0 to match the pinned Cranelift (F80, Q31).
Before that none of 2, 4, 5, 6, 7, 9, 10, 11 or 12 could be executed at all.

Criterion 2 has since **run and passed** — 105/113 with the feature on and off, ceiling 105/113, re-run on 2026-09-11 after that day's probes —
and is **vacuous until a tier exists**, since the feature gates no code. That
is worth stating rather than counting: a criterion that cannot fail yet is not
evidence, and this one is listed as runnable rather than as met.

1. **The gate is passed before implementation begins.**
   `cargo bench -p bund2-bench -- value/push_pull` reports **under 20 ns**.
   **Met**: 9.8 ns, after D41; re-run at **9.47 ns** on 2026-09-11, after
   F115 put a `Drop` impl on `HeapValue`, which is the hottest type this
   criterion guards. The change was an improvement of 3.4%, not a cost.

   An earlier version also required "`dispatch/*` re-measured shows the
   dispatch loop, not the value layer, as the dominant term". **That half is
   withdrawn**, because §S1's update establishes it cannot be shown with these
   benchmarks — nothing separates dispatching a word from the work the word
   does once dispatched, and the difference `1 drop` − `dispatch/literal_only/w1000`
   bounds dispatch only from above. A criterion that cannot be decided is worse than none; the question
   it was reaching for is criterion 10, which can fail.

2. **Conformance moves by exactly zero.**

        cargo xtask conform
        cargo xtask conform --features jit
        cargo xtask conform --features jit --jit-threshold 1

   The third cannot run yet: `xtask conform` answers `unknown argument
   --jit-threshold` (2026-09-11), since the flag arrives with the tier. All
   three must read the same N/M **and the same CEILING**. Any movement is a bug,
   per CLAUDE.md — which also requires the ceiling beside the number, since an
   approved deviation can never enter the numerator and N/M alone overstates
   the remaining work. Today: **105/113, ceiling 105/113**, with the feature
   off and on, re-measured 2026-09-11 after that day's probes (82/90 that
   morning); eight approved deviations, D50's among them. This criterion measures the **dev** profile and criterion 11
   measures **release**; the split is deliberate, and stated in both places.

   **The two `jit` runs must compile something.** This is the ninth review's
   S3, and the owner chose among its options. Two things could make them
   vacuous.
   - `bund2`'s thread had no Tier 1 share, which §S8's
     `set_stack_region_with_share` now fixes.
   - §S7's threshold is 64 evaluations of one body, and most corpus programs
     are short.

   So `bund2` reports how many bodies it compiled when asked, through a
   statistics flag, and `conform` prints the total beside its `measured:`
   line. **A `jit` run that compiles no body over the corpus fails.** The
   second `jit` run sets §S7's threshold to 1, so every body compiles on its
   first evaluation. That is the strongest test of meaning the corpus can give,
   and it is why §S7 makes the threshold configurable and recorded.

   **Dated note, 2026-09-14 — the flag is built, and it found what it was for.**
   `bund2 --stats` reports compiled bodies and inlined sites on stderr. Its
   first use found **F124**: the CLI constructed an `Interp` directly and never
   reached `bund2-runtime`, the only crate that installs a tier — so
   `--features jit` had been enabling a feature on code that never ran, and this
   criterion's comparison had been running Tier 0 against itself. Fixed the same
   day; `conform --features jit` now reads 106/114, ceiling 106/114, with a tier
   actually installed.

   **The second `jit` run is still not runnable.** `--jit-threshold` is
   specified here and in §S5, and built nowhere (**F125**), so the corpus is
   only exercised at the default threshold of 64 — where most programs are too
   short to compile anything. Measured: a 200-iteration loop compiles no body;
   1000 compiles two. Until the flag exists this criterion's stronger half is
   unavailable, and that is a gap in the evidence rather than a passing test.

   **Once a tier exists, this criterion exercises promotion across calls
   under the CLI's default reporter.** D45 made `TextReporter` want a snapshot
   only for a fatal report, which is made after evaluation returns. Before
   D45 the default reporter kept every value from being promoted across a
   call, so this criterion would have run promotion only between calls.

   **`--features` did not exist on `conform` when this criterion first claimed
   to have run.** `conform` rebuilt `bund2-cli` unconditionally and without
   features, so a jit-built binary was overwritten and the non-jit one
   measured — both numbers came from the same build and nothing said so. All
   five subcommands that measure the binary now share one builder
   (`xtask/src/buildcli/mod.rs`) — `depth` last, on 2026-09-10, after the fifth
   review found it still carrying a private one — and `conform` prints the line
   `measured: bund2-cli, dev profile, features: jit` above its result.

3. **A cache entry cannot answer for a different body.** D35 requires this as
   a test, not a comment, and names the failure: an entry whose body was freed
   can be answered for a *different* body at the reused address — wrong code
   executed.

   With D35 as amended (Q32) **the cache holds a `Weak`**, which keeps the
   address out of reuse while the entry lives. The test: drop every strong
   reference to a compiled body and assert its contents are dropped, that its
   entry no longer upgrades and answers for nothing, and that after the sweep
   the entry is gone. An earlier version tested that a strong reference kept
   the body alive, which D35 no longer requires.

   §S7's counter is criterion 6, and it now holds the same kind of reference:
   both must drop a body's contents with its last strong reference.

4. **Every inter-word call in compiled code is indirect**, checked
   mechanically rather than by reading.

   "Inspecting the emitted CLIF" was the earlier wording and it names no tool:
   inspection is a person, and a person will stop looking once the design feels
   settled. The property §S4 actually requires is about *relocations* — a
   direct call emits a relocation naming a `FuncId`, and that relocation is
   exactly what would still point at orphaned code after a word is redefined.

   The check: compile a body that calls another word, and assert the module's
   relocation records contain **no entry targeting a function**. Corroborated,
   not replaced, by asserting the CLIF text contains `call_indirect` and no
   bare `call fn`, which is readable but formatting-dependent.

   It fails if any lowering path emits a direct call, including one added later
   for a word that looks safely static — and D16 means none is.

   **The relocation check is not reachable from a `JITModule`, 2026-09-13.**
   `cranelift-jit` consumes relocations internally when it finalises a
   definition (`src/backend.rs`, `perform_relocations`) and exposes no accessor;
   `cranelift-object` is the one that keeps them, behind `relocs()`. So **this
   criterion belongs to the AOT path** — RFC-0006's — and cannot be run against
   the JIT as it stands. Saying so is better than quietly reading the CLIF
   instead, which is the inspection this criterion was rewritten to avoid.

   What *is* established meanwhile, by construction rather than by assertion: a
   compiled body reaches a native only by loading a thunk's address from a slot
   table and calling through it (§S6), so no inter-function call in emitted code
   names a `FuncId`. The two permitted relocations are the only direct calls
   emitted — thunk to adapter, trampoline to body.

   **Two kinds of relocation are allowed**, both from §S8's call boundary: a
   native's `Tail` thunk calling its Rust adapter, and the entry trampoline
   calling the body it enters. Both name an external symbol or the trampoline's
   own target, never a compiled body's `FuncId` from inside another body. The
   check also asserts that every call slot's target is a `Tail` thunk or a
   `Tail` body.

5. **A redefined word is observed by compiled callers — called *and*
   inlined.** `register` a word, force promotion of a caller, `register` it
   again with different behaviour, and assert the caller's next result
   changes.

   **As first written this passed vacuously for every inlined word**, because
   an inlined fragment goes through no slot (the fifth review's B1). So it runs
   a second time against a caller that inlines `+`'s fragment, changing what
   `+` means in each way §S4's chain allows — re-registered as a lambda,
   aliased to another word, unregistered, and registered as a command — once
   before the caller runs and once **mid-body**, by a `register`, `alias` or
   `unregister` earlier in the same compiled body. Each must change the
   caller's result exactly as it changes Tier 0's.

   **A third time through an alias.** A caller that calls `<-`, with
   `stacks_left` rebound to a lambda of a different effect, once before the
   caller runs and once mid-body. `<-`'s own generation does not move, which
   is the case §S5's direct-resolution rule exists for.

   **A fourth time through a lambda callee.** A caller that calls `f`, where
   `f` is `{ g }`, with `g` rebound to a word of a different effect, once
   before the caller runs and once by `f` itself mid-call. Nothing was
   promoted across `f` (D46), so the result must match Tier 0's.

6. **The caps hold**, checked by a test per row of §S7's table rather than by
   inspection. With the compiled-function cap set to 4, compiling five distinct
   bodies leaves the fifth interpreted. With the recompile cap set to 2, a
   third redefinition demotes the body permanently. With the counter cap set to
   8, evaluating nine distinct bodies leaves the map at 8.

   **And the counter does not pin.** Evaluate a body below the threshold, drop
   every strong reference to it, and assert the body's contents are dropped
   while the counter still holds its entry — then that the entry is gone after
   a sweep. This is §S7's claim that the *cache* pins ≤1024 bodies and the
   *counter* pins none; without it, "the counter holds a `Weak`" is a comment
   rather than a property. It is the counterpart of criterion 3, and it fails
   the same way: silently, and only under memory pressure.

7. **The feature does not move what it must not**, with a stated tolerance.

   This is the regression half; the *win* is criterion 10, and an earlier
   version conflated them into "shows improvement", which nothing can fail —
   0.1% is an improvement.

       cargo bench -p bund2-bench -- --save-baseline off      # feature off
       cargo bench -p bund2-bench --features jit -- --baseline off

   | group | requirement |
   |---|---|
   | `startup` | **no change**: Criterion reports no statistically significant difference, and the point estimate moves by **< 5%** |
   | `value` | no change, same tolerance — D41's work is below the tier and the tier must not disturb it |
   | `dispatch`, `arith`, `corpus` — the `bund2-bench` group, not `cargo xtask corpus` | free to improve; a **regression beyond 5%** on any of them fails |

   The 5% band is not arbitrary. Run-to-run spread on one machine is already a
   few percent — `fragment/int_add/tier0` read 57.9 and 59.2 ns in two runs on
   2026-09-10, and the sixth review's re-run of that table agreed to within 3%
   — and a threshold inside the noise would fail on weather. (An earlier
   wording quoted a range for `startup/registry/register_all` that no recorded
   run supports.)

   **A corpus wall-clock figure is not an acceptance criterion.** Q14
   establishes that the subprocess harness cannot resolve interpretation — an
   ordinary program evaluates in ~9 µs against a 2.3 ms process floor — so a
   percentage taken there measures process spawn. `cargo xtask bench` keeps
   only its regression role: catching a startup collapse.

8. **`cite` and `lint` clean**, with `cargo xtask cite` resolving every
   `path:line` in this document. **Note what this now checks and did not
   before:** citations into `crates/` were invisible to the extractor until
   F79, which is how two of this document's own citations were stale on the day
   they were written. It is **still partly blind**: the extractor cannot seed a
   scope from an unprefixed filename, so §S4's thirteen `multistackvm_apply.rs`
   and `multistackvm_inline.rs` citations were invisible to it until they were
   spelled with their `reference/rust_multistackvm/src/` prefix on 2026-09-10.
   A citation `cite` cannot see is one it cannot fail.

9. **The sync is worth its cost.** §S5's second rule requires promoted values
   to be written back — each to the stack it was taken from — before an opaque
   call. **It is not
   established that this pays.** For a short straight-line run the sync may
   cost more than the promotion saved, which would collapse the stop-at-the-site
   rule back into whole-body exclusion for those bodies.

   The criterion: a benchmark in `crates/bund2-bench` comparing a body with an
   opaque site in the middle against the same body interpreted, at straight-line
   run lengths of **1, 4, 16 and 64 words before the site**. At each length the
   compiled form must be **no more than 5% slower** — the same band as
   criterion 7, and for the same reason: run-to-run spread is already ~±2.5%.

   **It runs under the CLI's default reporter**, `TextReporter` with its stack
   dump on, which is the configuration `bund2 script` uses and which permits
   promotion across calls under D45. A figure taken under `--no-dump-stack` or
   a silent embedder must say so.

   **The crossover length is reported, not optional.** An earlier wording said
   that if the compiled form is slower below some length, "this RFC must state
   that length" — a criterion that lets its own failure be renamed as a
   parameter. It is now: the shortest length passing the 5% band is recorded,
   and promotion is skipped beneath it. **If no length passes, §S5's second
   rule is wrong** and the choice is between whole-body exclusion (RFC-0004's
   original reading; §S6's *Where promotion stops* counts what that costs) and abandoning promotion
   across opaque sites entirely.

   Recorded as a criterion rather than an assumption because it is the one place
   §S5's rule could be wrong in a way that no correctness test would catch.

   **Measured 2026-09-14 — the rule survives, and the crossover is 4 words.**
   `crates/bund2-bench/benches/fragment.rs`, the `sync` group, under
   `TextReporter` with the dump on as this criterion requires:

   | words before the site | Tier 0 | promoted + synced | | verdict |
   |---|---|---|---|---|
   | 1 | 104.6 µs | 112.4 µs | **0.93×** | **fails** the band, +7.4% |
   | 4 | 265.3 µs | 112.8 µs | **2.35×** | passes |
   | 16 | 838.1 µs | 116.9 µs | **7.17×** | passes |
   | 64 | 3117.6 µs | 127.8 µs | **24.4×** | passes |

   Re-run for the decisive pair: length 1 at +8.8%, length 4 at 2.32×. The
   failure at 1 reproduces well clear of the ±2.5% spread, so it is a result
   rather than noise. **The crossover is 4**, and promotion is skipped beneath
   it.

   The shape is the explanation: `promoted` is nearly flat — 112 to 128 µs
   across a 64× increase in work — because register adds cost almost nothing,
   while Tier 0 scales with dispatches. The sync is a **fixed cost per body**,
   so it loses only where there is no straight-line run to amortise it over.

   **This is a veto, not a certificate.** The `promoted` column is hand-written
   Rust standing for a lowering that does not exist: no entry, no exit, no
   guard per site, no helper call per sync. A real lowering sits above it, so a
   length failing here could not pass there — while one passing here may still
   fail. What the measurement settles is the question this criterion was
   written to ask: whether §S5's second rule is wrong. It is not. Three lengths
   pass, the fallback to whole-body exclusion is not reached, and the rule
   stands as written with a recorded crossover.

   **A first version of this benchmark reported a crossover of 1, and was
   wrong.** It built an `Interp` per iteration inside `iter_batched`, and
   `startup/registry` puts `register_all` at 5.08 µs — so both columns sat on a
   ~9 µs pedestal that wobbled 18.6% across runs, against a 5% band, with the
   signal at length 1 being a single addition. Repeating each body 1,000 times
   inside one timed iteration puts the signal at 84% of the span in the worst
   case. The correction is recorded because the artefact was in the direction
   that flattered the rule.

10. **The dispatch share is measured, not estimated.** §S1 withdraws an Amdahl
    bound rather than correcting it, because no benchmark here separates
    dispatching a word from the work the word does once dispatched:
    the difference `1 drop` − `dispatch/literal_only/w1000` bounds dispatch at
    **≤ 33.7 ns** and cannot go finer. (An earlier wording attributed the bound
    to `dispatch/literal_only/w1000` itself, which reads 13.2.)

    The criterion: before any lowering is optimised, an A/B on one compiled
    body against the same body interpreted, reported per program shape as in
    §S1's update. **A speedup below 1.2× on `1 2 + drop` means the tier is not
    earning its keep** and §S1's gate should be reopened rather than the number
    explained. This is the criterion that decides whether Tier 1 was worth
    building, and it is deliberately the one that can fail.

    **The first compiled body is not this measurement, 2026-09-13.**
    `compile_word_body` (§S6) applies each of a body's values through
    `Vm::apply`, which is Tier 0's own path, so it is *slower* than interpreting
    and measuring it would answer a question this criterion does not ask. It is
    the structure fragments are inlined into; the A/B this criterion requires
    waits for a lowering that inlines them.

    §S6's prototype puts ceilings on what this can report (*Measured*,
    2026-09-10): **inlining alone at most 2.0× on `Int + Int` and 3.0× on
    `dup drop`; inlining with promotion at most 8.8× and 12.9×** — in Rust,
    with no compiled-code overhead. Compiled code pays entry, exit, a type
    guard and a meaning guard per inlined site, and a helper call per sync, so
    the real figures sit below those.

    **The stop rule governs the measured lowering, not the ceiling.** If
    inlining alone, *compiled*, comes in under 2× on a shape, that shape's
    inlining is not worth its machinery on its own, and the constraint is the
    lowering rather than the ceiling — a reason to stop, not to tune. The
    ceiling cannot trigger the rule: on `Int + Int` it is **2.03×**
    (59.2 / 29.2), and the sixth review's re-run read 2.04×. What the ceiling
    does say is that there is almost no room. A lowering that adds more than
    about 0.4 ns per `+` to the 29.2 ns `lowered` measures will land under 2×.
    **Dated note, 2026-09-12: on this machine the ceiling triggers the rule by
    itself, across two runs.** `Int + Int` reads `tier0` / `lowered` of
    56.57 / 32.02 and then 54.81 / 30.28, ceilings of **1.77×** and
    **1.81×** — both *below* the 2× stop rule, where the 2.03× above sits just
    over it. Two runs on the same side make this reproducible here rather than
    run-to-run noise, so the sentence "the ceiling cannot trigger the rule" is
    false on this hardware and true on the hardware that produced 2.03×: the
    crossing is machine-dependent, and the threshold sits inside the spread
    between machines. This does not change the criterion, and it strengthens
    rather than settles the RFC's own prediction that arithmetic inlining
    alone will not clear the rule — the prediction now has the ceiling on its
    side on at least one machine. The operand-free arm still has room —
    `dup drop` reads 3.50× and 3.57× — and the decision stays this criterion's
    measurement of a real lowering, not the ceiling's (§S1, *Update,
    2026-09-12*).
    So this RFC **predicts** arithmetic inlining alone will not clear the rule,
    does not propose it as a performance feature on the strength of that
    prediction, and leaves the decision to this criterion's measurement. An
    earlier revision said the rule had already been applied; it cannot have
    been, before a lowering exists. Inlining stays necessary as the mechanism
    promotion rests on, and operand-free arms like `dup drop` have a 3.0×
    ceiling and room to spare.

    Reported per shape, then: inlining alone, and inlining with promotion. The
    1.2× floor above applies to the tier as shipped. It is measured under the
    CLI's default reporter, as criterion 9 is.

    **Measured by a throwaway lowering, 2026-09-11 — outside this RFC's gate.**
    A spike on the scratch branch `spike/lowering-1` (commit `3138d3c`,
    `bund2_jit::spike` and `bund2-bench`'s `spike` benchmark under
    `--features jit`) compiles `1 2 + drop` through Cranelift with §S8's call
    boundary: a `CallConv::Tail` body taking a context pointer and returning a
    status, a Rust adapter calling each native under `catch_panic` (D49), an
    entry trampoline, §S8's entry checks and §S5's post-call loads. It is not
    the tier and is not proposed for merging. Release build, Apple silicon,
    Criterion medians, against `Vm::eval_lambda` on the same body:

    | body | Tier 0 | every word a call | literals promoted, `+` and `drop` inline |
    |---|---|---|---|
    | `{ 1 2 + drop }` | 111.8 ns | 58.0 ns, **1.93×** | 1.86 ns, 60× |
    | the four words ×100 in one body | 10.55 µs | 5.68 µs, **1.86×** | 19.6 ns, ≈540× |

    What it shows and what it does not:
    - **The floor is cleared by removing dispatch alone.** The "every word a
      call" lowering inlines nothing and keeps §S8's full boundary on every
      word. It reaches 1.9×, close to the 2.03× inlining ceiling above,
      although the two are measured on different bases: this one per body
      entered through `eval_lambda`, that one per `+` in a chain.
    - **The promoted column is a ceiling for bodies made only of literals.**
      Cranelift folds `1 2 +`, and merges the meaning guards' cell loads,
      because no call lies between two guards, so no cell can change: the ×100
      body is 1,744 bytes, about 16 per group, where the call lowering grows by
      about 480 per group. That merge is sound, and it means criterion 17's
      per-site cost is near zero in straight-line code; the cost sits after
      calls. A body whose operands arrive on the stack pays pulls, type guards
      and a sync, for which §S6's fragment `promoted` column, about 6.7 ns per
      `+`, is the better estimate.
    - **The spike leaves out things that make the tier slower**: call slots and
      `Tail` thunks, the cache lookup at entry, §S7's counter, the residual
      path, type guards, and the exit check after each native (§S5, *A call
      may end the program*). None of them is plausibly worth the 0.9× of
      headroom above the floor, but this criterion is met only by the tier as
      shipped, and stays open until then.

    **Measured 2026-09-14, on the shipped lowering, and it is below the stop
    rule.** §S6's inlining join exists: a body's `CALL`s are planned against
    the published table, and an admitted site runs the arm inline behind the
    type, generation and `autoadd` guards. The A/B this criterion asks for can
    now be run, and the figures are **attributed** — `bund2 --stats` reports
    compiled bodies *and* inlined sites, so a timing can no longer be confused
    with a body that compiled and inlined nothing.

    | program | sites inlined | Tier 0 | with the tier | |
    |---|---|---|---|---|
    | `1 2 + drop` ×10⁶ (this criterion's own shape) | 3 | 0.31 s | 0.31 s | **1.00×** |
    | a fixed-point Julia step ×2×10⁵ | 7 | 0.49 s | 0.465 s | **≈1.04×** |

    Release profile, ten alternating pairs for the first and six for the
    second, medians quoted; alternating because a straight A-then-B run on this
    machine drifts by more than the difference being measured. Both programs
    are `register`ed words called in a `for` loop, so §S7's threshold is passed
    early and the body is compiled for nearly the whole run.

    **So the stop rule fires.** "A speedup below 1.2× on `1 2 + drop` means the
    tier is not earning its keep and §S1's gate should be reopened rather than
    the number explained." It reads 1.00×. This entry does not explain it away.

    **What the number is not.** It is not evidence that the *machinery* is
    wrong: criterion 17's dominance holds, the guards are taken, and
    conformance is unmoved at 106/114 with the feature on. It is evidence about
    **inlining without promotion**, which is what exists today — and §S6
    predicted it in terms: "For inlining alone on arithmetic it is almost
    certainly not [worth it], and criterion 10's measurement decides." D59
    recorded the same conclusion from the other side, measuring this machine's
    `Int + Int` ceiling at **1.77× and 1.81×**, already below the 2× rule
    before any compiled code existed. The measurement agrees with the RFC's own
    prediction rather than upsetting it.

    **Why a compiled body still costs what it does.** Every value that is not
    an inlined site goes through `Vm::apply` — Tier 0's own path — plus the
    boundary: an entry trampoline, a slot load and an indirect call per value,
    the request-cell load after each, and three guards per site. Three inlined
    sites in `1 2 + drop` remove three dispatches and add all of that. The
    remaining headroom is promotion, which §S6 calls "the prize" and which
    needs the residual path and assumption 38's resume index.

    **Re-measured against promotion's first stage, 2026-09-14 (D66).** An int
    literal now becomes an `iconst` in a `Variable` and never reaches the stack,
    and an inlined site whose operands are all promoted reads them from
    registers rather than calling the pop helper, so `1 2 +` lowers to two
    `iconst`s and an `iadd`. `bund2 --stats` gained a **third** figure for this
    measurement — promoted values — because a body that compiled *and* inlined
    may still have promoted nothing, and a timing of that answers a different
    question. The same argument that added `inlined_sites` one stage earlier.

    | program | bodies / sites / promoted | Tier 0 | with the tier | |
    |---|---|---|---|---|
    | `1 2 + drop` ×10⁶ (this criterion's own shape), batch 1 | 2 / 3 / 4 | 0.3160 s | 0.2992 s | **1.057×** |
    | the same, batch 2 | 2 / 3 / 4 | 0.3177 s | 0.2978 s | **1.067×** |
    | a fixed-point Julia step ×2×10⁵ | 4 / 7 / 9 | 0.5049 s | 0.4558 s | **1.108×** |

    **Re-measured again after the residual path and chaining (D67), same day.**
    A site's result now stays in a register and feeds the next site, so
    `1 2 + 3 +` never round-trips its first sum through the stack:

    | program | bodies / sites / promoted | Tier 0 | with the tier | |
    |---|---|---|---|---|
    | `1 2 + drop` ×10⁶, batch 1 | 2 / 3 / 4 | 0.3106 s | 0.2895 s | **1.073×** |
    | the same, batch 2 | 2 / 3 / 4 | 0.3077 s | 0.2918 s | **1.055×** |
    | a fixed-point Julia step ×2×10⁵ | 4 / 7 / 9 | 0.5005 s | 0.4565 s | **1.097×** |

    **Chaining bought nothing measurable.** The two batches after bracket the
    two before — 1.055× and 1.073× against 1.057× and 1.067× — so they are one
    population, and the Julia step moved the wrong way. The change is correct
    (62 tests, conformance unmoved, output byte-identical with the feature and
    without) and it removed exactly the traffic it was built to remove. The
    figure did not follow.

    So the remaining cost is **not** value traffic between inlined sites. It is
    the boundary — entry trampoline, a slot load and an indirect call per value,
    the request-cell load after each — plus `Vm::apply` for every value that is
    not a site. In `1 2 + drop`, three of four values are sites and the fourth
    is a literal in a register, so almost nothing is left for promotion to take:
    what is left is the machinery around it. A future attempt at the 1.2× floor
    has to attack that, not the operands.

    Release profile, ten alternating pairs per row, medians quoted, under the
    CLI's default reporter as this criterion requires. Output is byte-identical
    with the feature and without, on both programs. **All thirty pairs favour
    the tier** — the columns do not overlap on the first batch — so the
    direction is a result rather than drift, which the 1.00× reading could not
    claim.

    **The stop rule still fires, and this entry does not explain it away.**
    1.06× is not 1.2×. Promotion moved the figure off 1.00×, which says the
    mechanism works and that `Vm::apply` dispatch was indeed part of what a
    compiled body was paying — but it did not move it far. The remaining cost
    is what this stage deliberately left standing: **the result of every
    inlined site is synced** rather than kept in a register, and **nothing stays
    promoted across a call**, so `1 2 + drop`'s sum makes a round trip through
    the stack between `+` and `drop`, and each of the four literals is still
    pushed once. The full residual — values promoted across a call, behind the
    callee's generation check, with assumption 38's resume index — is what the
    1.2× judgement should be taken against. Until then §S1's gate stays open on
    the strength of this number, not closed by it.

    **What this criterion still cannot report**: the corpus-wide figure
    criterion 2 wants at threshold 1, because `--jit-threshold` is specified
    and unbuilt (F125).

11. **A promoted recursion does not overflow the machine stack.** §S8's
    correctness problem, and the criterion is one that already exists:

        cargo xtask depth                       # feature off — Met at 100,000
        cargo xtask depth --features jit        # loop level no lower (D44)

    **`--features` did not exist when this criterion was written**, and
    `xtask depth` shelled out to whichever `bund2` happened to be sitting in
    `target/`, so the command could not have measured a tier even in
    principle. `depth` now *builds* the binary it measures, with the features
    asked for — F80's lesson applied to the one check that stands between §S8
    and an abort.

    RFC-0003's criterion 2 is **Met** at a call depth of 100,000 (RFC-0003,
    acceptance criterion 2, cited by number as in §S8). It must stay met with
    the tier on. Before the frame loop the same program aborted between 5,000
    and 20,000, so this criterion has a demonstrated failure mode and is not
    hypothetical.

    Reported with the result: **both floors**, **the depth at which compiled
    bodies begin declining**, and the per-entry cost of the Tier 1 check
    measured in `crates/bund2-bench`. The check **must stay under 2 ns per
    body entry**, as criterion 17's identical bound does; above it the design
    fails this criterion and is reconsidered against simply not promoting
    bodies that can
    recurse — which D16 makes undecidable, so the check is the expected answer
    and the number is what says whether it is affordable.

    It also runs a recursion **through a loop word** — a body that calls itself
    from inside `times` — because `eval_lambda` spends a Rust frame per nesting
    (§S8, *The guard*), and the self-recursive word alone never exercises that.
    That axis must **report, not abort**, in both builds. It aborted in both
    until §S8's Tier 0 floor landed (F85). `cargo xtask depth`'s `loop` axis
    now runs it at 100,000 levels, reports a Bund-level error, and prints the
    level at which Tier 0's floor fired: **10,923** in release on 2026-09-10.

    **That level is not meaning (D44).** It is 2,371 in the dev profile on the
    same source (`cargo xtask depth --dev`), and the oracle aborts at every
    depth. In dev the `nesting` axis also aborts, in the parser's recursion,
    which RFC-0003 excludes. That is why this criterion runs release. So the criterion is that
    the axis reports rather than aborts in both builds, and that **its level
    with the feature on is no lower than with it off**. An earlier revision
    required the two levels to be equal, and no fixed floor can give that and
    "never less" together (the seventh review's B1; §S8, *Two floors*).

    **A case with the share held**, added for the ninth review's S2. A word
    recurses directly until compiled frames pass the Tier 1 floor, and then
    runs the `loop` axis from there. Its level with the feature on must be no
    lower than with it off, and the run reports `c` and `δ`, from which §S8's
    margin `m` is set. The `loop` axis alone starts at the top of the stack,
    with nothing compiled above it, so it cannot show this.

    **Every re-entering path, not `loop` alone**, for the tenth review's S2.
    The run reports `c_p` and `δ_p` for each path in the set
    `every_reentering_function_is_named` finds (§S8), plus the drain helper,
    and `m` is set from the largest ratio.

12. **A synced value keeps its stack tag.** §S5's rule writes promoted values
    back to the real stack; **D41 put the stack tag inside the value for
    scalars**, so a sync that pushes a bare `BundValue` leaves `StackSym::NONE`
    and the value renders `tags: {}` where the oracle renders
    `tags: {"stack": "main"}`.

    42 of 113 goldens carry exactly that text, and 46 carry some `"stack":`
    tag (2026-09-11, `grep -rl` over `tests/golden` — recursively, or the
    count reads 29 and 33 across four subdirectories), so the failure is loud — but only if a
    golden exercises a compiled body with an opaque site in it, which none does
    today. The criterion: a probe that pushes, promotes, syncs and dumps, with
    `cargo xtask conform` green. **This row exists because a reviewer found it
    and the Preservation table did not**; the tag moved into the value one day
    and this RFC was written the next, without the two being connected.

13. **An absent effect is treated as `Opaque`, never as zero.** §S6's *Where
    promotion stops* counts **21 sites** (2026-09-11) whose word declares no effect at all. A defaulted
    `StackEffect` would read `Fixed(0, 0)` — "consumes nothing, produces
    nothing" — and a compiled body would keep promoting straight through a call
    that may do anything to the stack.

    The check: register nothing for a name, compile a body that calls it, and
    assert the lowering stops promoting at that site exactly as it does for
    `!`. It fails silently otherwise, which is why it is a criterion and not a
    remark.

14. **A word that reads beyond its arity is a promotion barrier.** Q34. `debug.display_stack` declares `eff(0, 0)` and calls `vm.snapshot()`;
    it runs in the golden capture epilogue, so it is on the conformance path.

    The check: compile a body that promotes, then calls a word which inspects
    the stack beyond its declared arity, and assert the observed depth and
    contents match Tier 0's. **D55 identifies the set.** Criterion 28's
    palette runs each fixed-effect native four ways: padded twice, with
    nothing beneath its operands, and with different values beneath. It also
    records any native that reads the whole stack or workbench, or a stack's
    depth by name, while audited. A native either check flags is left
    off `PROMOTABLE.txt`, so compiled code syncs before it. The Tier 0 half
    runs today, inside
    `every_fixed_effect_native_keeps_its_pair_over_the_promotable_palette`
    (`crates/bund2-stdlib/src/lib.rs`), and the flagged natives are listed in
    `PROMOTABLE.txt` as comments. Its first stable run, on 2026-09-11, flags
    eight. `debug.display_stack` and `debug.display_workbench` read the whole
    stack or workbench. `move_from`, `rotate_current_left`,
    `rotate_current_right`, `rotate_stack_left` and `rotate_stack_right`
    change the values beneath their operands. `swap_in` reads a stack's depth
    by name. The same run found six
    named-stack words whose declared pair was wrong on the current stack
    (F111, now opaque). The compiled half needs a tier.

15. **The dependency direction is not inverted.** §S6's mechanism rests on
    `bund2-jit → bund2-stdlib` being permitted while the reverse is not.
    RFC-0000's B3 already checks one half:

        cargo tree -p bund2-stdlib      # must not list bund2-jit
        cargo tree -p bund2-stdlib      # must not list any cranelift-* crate

    The second line is new and is the one D9's amendment turns on: a fragment
    that reached for a Cranelift type would pull the optional subsystem into
    the mandatory crate, and B3 as written would not catch it because the
    dependency would be on `cranelift-frontend` rather than on `bund2-jit`.

16. **Every BundIR fragment agrees with the word it specialises.** A
    differential test per fragment runs the arm and the word on **the same
    constructed values** — not the same source text, which would route one side
    through the lexer — and asserts equality of value, `dt`, `q` and D41's
    stack tag. For `dup` it also asserts F13's property **directly**: the two
    values left behind have different identities. The render comparison cannot
    see that, because identities differ per run and are normalised away.

    **The inputs are the arm's boundaries, chosen by hand**, and the test says
    so; an earlier version called them generated. For `Int + Int` they include
    `i64::MAX`, `i64::MIN` and their neighbours, and a separate assertion pins
    `i64::MAX 1 +` to `i64::MIN` — wrap-around is where a lowering using a
    trapping or checked add would part company with the word.

    **On today's domain the `q` assertion cannot fail** (§S6, constraint 2),
    and this criterion does not count it as evidence; it is kept so that a
    widened guard can make it fail.

    **It certifies the model, not the compiled code**, until it has a third
    leg: the lowered code against `frag::run` over the same boundaries. That
    leg is required before any lowering ships, and cannot be written before one
    exists.

    **The third leg exists and runs, 2026-09-12.**
    `crates/bund2-jit/src/lower.rs` lowers a fragment to machine code and its
    tests compare the lowered arm against `frag::run` over the same
    hand-chosen boundaries: `Int + Int` across
    `i64::MAX`, `i64::MIN` and their neighbours in **both operand orders** —
    addition commuting is what would hide a reversed register file, and
    `Op::PopInt`'s numbering is the whole difficulty — the `i64::MAX 1 +` wrap
    pinned on the lowered path as well as the model's, `dup` and `drop` over
    four payload shapes, F13's fresh identity asserted directly, and a declined
    guard consuming nothing on either path.

    **Two guards keep the leg from being vacuous**, because the comparison is a
    normalised render and a `norm` that collapsed distinct values would make
    every assertion above pass while checking nothing. `norm` is asserted as a
    property — two independently built `int(1)`s normalise *equal*, `int(1)` and
    `int(2)` do not — and the comparison is **mutation-checked**: an arm that
    subtracts where the model adds gives 7 against 13, and the test asserts the
    renders differ.

    Runs today: `cargo test -p bund2-stdlib fragments` for the first two legs,
    and `cargo test -p bund2-jit --features jit` for the third.

17. **Every inlined site carries its meaning guard.** §S6: an inlined
    fragment is entered only while the site's slot still holds the binding it
    was inlined against, and `autoadd` is clear. Checked mechanically, as
    criterion 4 is: compile a body that inlines a fragment and assert every
    inlined region in the emitted code is preceded, on every path into it, by a
    load and compare of the name's generation cell and of the `autoadd` cell
    (§S6, *Addressing*). That is a dominance property, and it is checkable
    because the lowering records each inlined region's entry block in a side
    table and `cranelift-codegen`'s `DominatorTree` answers whether the guard's
    block dominates it.

    Its cost per site is measured in `crates/bund2-bench` and **must stay under
    2 ns** — the bound criterion 11 sets for the headroom check, and about 7% of
    the 29.2 ns `lowered` `+` it guards. Above that, the guard's design is
    reconsidered before inlining ships.

    **The per-call checks have the same bound.** The three loads after every
    call (epoch, `autoadd`, request), together with the callee's generation before a call
    across which values stay promoted (§S5), must cost under 2 ns per call.
    In a body that promotes, calls are far more common than inlined sites, and
    until the seventh review nothing bounded their checks.

18. **Compiled code honours `autoadd`** — at entry, after every call, and for
    every kind of value the mode affects. Bind `:` and `;`, compile a body that
    turns the mode on mid-body through `!`, and assert two things. The calls
    and **literals** after it must be appended into the value beneath rather
    than dispatched or pushed
    (`reference/rust_multistackvm/src/multistackvm_apply.rs:19-27`, `reference/rust_multistackvm/src/multistackvm_apply.rs:89-97`).
    A CONTEXT value must be **pushed as a value of its own** rather than
    switched to (`reference/rust_multistackvm/src/multistackvm_apply.rs:72-73`).
    An earlier wording had CONTEXT "collected", which contradicted F84; a probe
    written to it would have asserted the wrong shape.

    **The oracle is the reference, not Tier 0.** Tier 0's `autoadd` arm pushes
    the name *beside* the value where the reference appends it *into* the
    value, and it collects no literals or CONTEXT values at all — F84, whose
    unit test asserts the divergence. So the criterion runs against a probe
    golden captured from the oracle, like every other probe.

    **Not runnable today, and stated so the guard is not built blind.** Bund2
    binds neither `:` nor `;` (`bund2 words`), although `Interp` carries the
    flag and the parser recognises `:` as a command term. The probe is
    captured when they are bound — capturing it now would add a golden Tier 0
    cannot pass — and F84 is fixed with them.

19. **A fragment that could fail after its guard admits cannot be built.**
    By then operands have been pulled, so neither declining nor running the
    word next is safe. `Fragment`'s fields are private and `Fragment::new` is
    its only constructor outside the `unchecked` feature, which only
    `bund2-interp`'s `[dev-dependencies]` enables. It walks the ops,
    typed, against what the guard promises, so every consuming op counts —
    `DropTop` and `DupTop` as well as `PopInt` — and every register is written
    before it is read. `frag::run` reports anything that escapes as
    `Error::internal`, `DropTop` on an empty stack included, never as success.

    **Met at the model level** by
    `every_consuming_op_counts_and_registers_are_written_before_read`
    (`crates/bund2-ir/src/fragment.rs`), and by
    `new_refuses_the_fragments_that_could_fail_after_the_guard` and
    `a_post_guard_failure_is_an_internal_error`
    (`crates/bund2-interp/src/frag.rs`). An earlier revision claimed this with
    `validate` optional and counting only `PopInt`; the sixth review found two
    fragments that passed it and then succeeded silently. A lowering must keep
    it: no path in the emitted code leads from a failed op back to the slot
    call.

20. **A body run by a loop word reaches the counter under one key.** Run a
    lambda through `times` 100 times and assert §S7's counter holds one entry
    for it, at 100. **Its precondition is met**: under D42 the key reaches the
    entry point, and `times_enters_one_body_under_one_key`
    (`crates/bund2-stdlib/src/seq.rs`) shows one key on all 100 entries through
    Tier 0's `entry_log` seam. The criterion itself needs the counter, and runs
    when one exists.

21. **A current-stack switch mid-body gives Tier 0's result.** Compile, with
    promotion on, `1 2 "s" to_stack +`; the same with `to_current`,
    `stacks_left`, `endcontext` and a CONTEXT literal in place of `to_stack`;
    and a conditional that runs its body on another stack. Assert each
    program's stacks and diagnostics match Tier 0's. It fails on any lowering
    that resolves the current stack once, or that syncs to the stack current
    at the sync rather than the one each value came from.
    It also asserts the **resume index** (assumption 38): that the compiled
    body's side table maps every call site to the source-body index at which
    the residual path resumes, and that no inlined or folded region spans a
    call site. Stacks alone would pass a lowering that resumed at the wrong
    index on a seventh program, which is why the side table is asserted
    directly rather than through its effects.

    **Met, 2026-09-14 (D67), in both halves — and they are met by different
    means, which is worth keeping distinct.**

    *The side table* is asserted **directly**, as this criterion demands.
    *The six programs* are a **differential** against Tier 0: they cannot prove
    the index is right, only that the observable result is, which is exactly
    why the criterion asks for both.

    *Met*: the **resume index** exists and is asserted directly.
    `Compiler::resume_table` and `Compiler::resume_index` expose assumption 38's
    map, and `every_inlined_site_records_where_its_residual_resumes`
    (`crates/bund2-jit/src/lower.rs`) checks that `1 2 + 3 +` records one entry
    per inlined site, at the site's own body position, and that every recorded
    index is a real position in the source body. The index is the site's own
    position rather than the next: a guard refuses *before* its value has run,
    so the value is still owed and the residual re-applies it. Two other tests
    reach the residual for real — `autoadd_sends_an_inlined_site_to_the_residual`
    through guard three, and `a_rebound_name_syncs_the_operands_it_promoted`
    through the generation guard — each asserting its site count first, so
    neither can pass vacuously (F127).

    **The six stack-switch programs are written, 2026-09-14.**
    `a_current_stack_switch_mid_body_gives_tier_zeros_result`
    (`crates/bund2-runtime/src/tier.rs`) runs each as a registered word, called
    until the tier has compiled it, against the same source with no tier at
    all:

    | program | what Tier 0 does |
    |---|---|
    | `1 2 :s to_stack +` | **fails** `Stack is too shallow for inline ADD()`; `1 2` stranded on `main`, `s` empty |
    | `1 2 :s to_current +` | the same failure, by the other switching word |
    | `1 2 stacks_left +` | the same failure, by rotation rather than by name |
    | `1 2 @s 9 endcontext` | current back to `main`, `1 2` on `main`, `s` dropped, `9` on the **workbench** tagged `s` |
    | `1 2 @s +` | the same failure — the CONTEXT literal is §S5's static barrier |
    | `1 2 true { 7 } { @s 9 } ifthenelse` | current `s` holding `9`, `1 2` left on `main` |

    Each answer was taken from a run before the test was written, so the
    assertions are against observed behaviour rather than against what the
    words are assumed to do. The last one runs the lambda **on top** (F97), so
    the switch is in the branch that executes.

    **Four of the six fail, and that is the point.** The two literals are
    pushed on `main`; the switch makes an *empty* stack current; `+` finds
    nothing there. This is §S5's own sixth-review example — `1 2 "s" to_stack +`
    failing on `s` — and it is precisely what a wrong lowering would hide: one
    that synced promoted values to the stack current *at the sync* rather than
    to the stack each came from would add `1` and `2` and push `3`, turning a
    failure into an answer. A criterion whose programs all succeeded could not
    catch that.

    **The first version of these tests passed while testing nothing**, and the
    reason is worth recording. The fixture's setup line was `:s ensure_stack`,
    and `Interp::ensure_stack` calls `add_as_current` for a name not yet in the
    ring — so `s` was already current before the body ran, and the switch
    inside each body was a switch to the stack already current: a no-op. All six
    passed, four with answers (`3` on `s`) that the corrected setup shows to be
    failures. The fixture now returns to `main` after the setup, and asserts
    that it did. This is the third vacuous-fixture bug in this area (F124's
    tier that was never installed, F127's empty fragment table), and all three
    were found by probing what the test actually reached rather than by reading
    it.

    **The comparison is every stack, the workbench and the diagnostics** — not
    just the current stack, which is the whole point: a body that switches away
    can leave values behind on the stack it left, and a comparison reading only
    what is current would miss exactly that. It also asserts the tier compiled
    something first, so a program cannot pass by never reaching compiled code
    (F127's lesson).

    **Error text must be normalised before it is compared.** The first run
    failed on `stacks_left` with two strings differing only in a `stamp` one
    millisecond apart: `Interp::eval`'s message renders the offending value,
    which carries `id` and `stamp`. That is **F14**, reproduced from the
    reference, and the golden capture normalises the same two fields. A
    CLI-level `diff` of the `to_stack` program was briefly mistaken for a tier
    defect for the same reason before the normalisation was applied.

22. **What a promoted value must not change, doesn't.** Six parts, each
    asserted against Tier 0's result:

    - **An error with values promoted.** Compile `1 2 true +` with `1`
      promoted. `+`'s type guard declines, and its generic counterpart, the
      word called through its slot, returns `ADD returns error: Incompartible
      Y argument for the math operations`, as Tier 0 does (checked
      2026-09-10). The report and the `[BUND]  Content of the stack` dump must
      match. Then do the same with a callee that fails. A fragment op cannot
      be the one that fails, because criterion 19 makes a failure after the
      guard impossible to construct. An earlier wording used `1 2 "a" +`,
      which succeeds, because `+` joins an `Int` and a string (F64's
      pass-through family). It constructed no error.
    - **An effect changed at run time.** Promote across a call, and rebind that
      call's name to a word with a different effect, once before the body runs
      and once mid-body through `register`. The stacks must match.
    - **An alias whose target is rebound.** Promote across a call to `<-`, and
      rebind `stacks_left` to a lambda that consumes two, before the body runs
      and mid-body. The stacks must match. Repeat with `$stacks_left`, and
      with a callee whose slot generation is saturated.
    - **A lambda callee whose callee is rebound (D46).** With
      `:g { drop } register  :f { g } register`, run a body `1 2 3 f`, and
      rebind `g` to `{ drop drop drop }` before the body runs. Then use
      `:f { :g { drop drop drop } register g } register`, which rebinds `g`
      during the call. The stacks and diagnostics must match.
    - **A lambda that shadows a native (F93).** After
      `:drop { drop drop } register`, run a promoted body `1 2 3 drop`.
      `drop` resolves to the lambda, so nothing stays promoted across it,
      although the slot still holds the native's `(1, 0)`. The stacks must
      match Tier 0's.
    - **The reporter, observed through the diagnostic.** Under
      `CollectingReporter` (`crates/bund2-api/src/diag.rs`) with
      `wants_stack` set, run a promoted body
      `1 2 :x :x alias`. `alias` is `eff(2, 0)` and warns mid-body that `x`
      resolves back to itself (`alias`, `crates/bund2-stdlib/src/singles.rs`),
      with `1` and `2` promoted below its arity. The warning's snapshot in
      `CollectingReporter::seen` must equal Tier 0's, `1` and `2` included.
      That snapshot is the seam that shows whether anything was held across
      the call. `alias` is the only fixed-effect native that reports mid-body
      today; `execute.`, the other, is opaque since F87. An earlier wording
      used `?error`, which only pushes a CONDITIONAL. Its notice is reported
      when `!` runs that value, and `!` is opaque, so promotion had already
      stopped and the part could not fail. Then, under
      `TextReporter::new(true)`, the lowering's side table (criterion 17) must
      record at least one call crossed by promotion in the same body. Under the
      default reporter, promotion across calls must not be zero (D45).

    Needs a tier.

23. **A body compiled for one `Interp` is never run by another.** On one
    thread, build two `Interp`s, evaluate a body under the first until it is
    compiled, and evaluate the same `Rc` under the second. The second's cache
    must miss, so the body is interpreted or compiled again against the second
    `Interp`'s cells. After the first `Interp` is dropped, the second must still
    give Tier 0's result. This fails for any cache, module or cell shared
    across `Interp`s (§S6, *Addressing*). Needs a tier.

    **Dated note, 2026-09-13 — a tier exists, and this is still not met.** The
    *cache* half is now satisfiable: `bund2-runtime` installs a `JitTier` per
    `Runtime`, each holding its own `Tiering`, so two `Interp`s share no cache.
    The *module* half is not: each `compile_word_body` builds a `JITModule` of
    its own, so there is one module per compiled body rather than one per
    `Interp`. That is stricter than the criterion asks in the direction that
    matters — no module is shared *across* `Interp`s, so the failure it names
    cannot occur — but it is not what §S6's *Addressing* describes, and the
    cells it wants one set of do not exist at all. Sharing a module across
    compilations is a `lower.rs` refactor: the module must outlive individual
    compilations and hand out finalised pointers as each is defined. The
    criterion runs when that lands.

    **Dated note, 2026-09-13 (2) — it landed, and the criterion runs.** `lower`
    gained a `Compiler`: one `JITModule`, every word emitted into it, and a
    `WordHandle` handed back in place of a self-contained compiled value. A
    `JitTier` owns one, so it is one module per tier and one tier per `Interp`.
    The cache stores handles rather than code, which is what makes the two
    halves inseparable — a handle is meaningless without the compiler beside it
    — and no compiled word can outlive the module that emitted it.

    Two Cranelift facts carry the sharing, both read rather than assumed:
    `finalize_definitions` takes its pending list with `mem::take`, so calling
    it once per compilation finalises only that compilation's functions and
    leaves earlier code executable; and `declare_function` **merges** a
    duplicate name into the existing `FuncId` instead of failing, which is why
    each body's functions carry a sequence suffix. Without it a second word
    would silently redefine the first and the symptom would be a wrong answer,
    not an error — so a test compiles two words into one module, runs both, and
    runs the first again after the second was emitted.

    Two tests in `crates/bund2-runtime/src/tier.rs` run the criterion itself:
    the same body driven past one tier's threshold leaves the other tier with
    no compiled word and a missing cache entry, and a `Runtime` dropped after
    compiling the shared body leaves a second `Runtime` still giving Tier 0's
    result. **The cells remain absent**, so the criterion's "one set of cells"
    is still nothing rather than something shared; that half is §S6's work and
    is not claimed here.

    **Dated note, 2026-09-13 (3) — the cells exist, one set per `Interp`.**
    `bund2_api::Cells` holds `autoadd`, the current-stack epoch and the request
    mirror in one allocation, owned by `Interp` in a `Box` and reached through
    `Vm::cells`. All three of this criterion's nouns — cache, module, cells —
    are now per-`Interp`, so the paragraph above is superseded except for the
    **stack-floor cell**, which §S6 also lists and which is still unbuilt.

    **Dated note, 2026-09-13 (4) — the floor cell landed too, so every noun of
    this criterion is per-`Interp`.** `Cells::floor` holds the Tier 1 floor,
    set once in `Interp::new` from the thread's declared share (D62). §S6's
    four cell families are complete: generation cells (D43), `autoadd`, the
    epoch, the request mirror, and the floor. Two `Interp`s on one thread get
    their own `Cells`, so neither can be compiled against the other's floor.

    **This does not change the criterion's behaviour**, and the two tests above
    still carry it: nothing in compiled code loads a cell yet, because there is
    no residual path, no drain helper and no `status_of`. What changed is that a
    second `Interp` now has cells of its own to be compiled against rather than
    none at all — which is the condition the criterion was written to protect.

24. **Every `bund2-stdlib` native with a fixed effect keeps it.** Promotion
    stops at an opaque site (§S5), and after any other call it models the
    depth from the callee's pair. So a native that runs a body, dispatches a
    word, or moves the stack by something other than its pair breaks promotion
    silently.

    The check is an audit over every program `conform` runs: each line of
    `tests/golden/HERMETIC.txt`, and each probe with a golden. Every program
    runs in process with `Interp::effect_audit` on
    (`crates/bund2-interp/src/lib.rs`). While a fixed-effect native runs,
    three things are breaches: starting a body, filing a tail request, and
    dispatching a word. So is a change of the current stack's depth that
    differs from its pair, read as RFC-0004 §S1 reads it: a floor and a net,
    on the main stack. The test is
    `every_fixed_effect_native_keeps_its_effect_over_the_corpus`
    (`crates/bund2-stdlib/src/lib.rs`). **Met**, 2026-09-10.

    **It has failed on what it exists for, twice.**
    - At `961028c`, the lambda-only test beside it named `execute.` (F87).
    - Before F91, the corpus audit named `object`: a body started and three
      words dispatched in `class_constructors_demo.bund`. The same first run
      also found ten miscounted pairs (F92).

    It reaches the natives the corpus calls, with the operands the corpus hands
    them. A word the corpus never calls, or an arm the corpus never reaches, is
    criterion 28's: its palette checks the pair as well as body starts.
    `no_fixed_effect_native_runs_a_body`, beside it, checks body starts only.
    A dispatch the audit records is one through `Interp::call_native`. A method
    native or conditional runner that `bund2-stdlib` calls directly
    (`oop.rs`, `conditional.rs`) is invisible to it, and today only opaque
    words make such calls. A named-stack word
    handed the current stack's name is Q34's case, and F92 lists the words.
    Runs today: `cargo test -p bund2-stdlib honesty`.

25. **No native reports at `Error` severity.** §S5's reporter rule reads
    `Warning` and `Notice` because natives return errors rather than
    reporting them. `no_native_reports_at_error_severity`
    (`crates/bund2-stdlib/src/lib.rs`) scans the crate's shipped source for
    `Diagnostic::error` and `Severity::Error`. **Met**, 2026-09-10. A native
    that must report an error mid-body fails it, and §S5's rule then has to
    read `Error` too. It covers `bund2-stdlib` only, and so does the rule it
    supports: promotion crosses no other crate's native (D47, criterion 27).

    **The scan checks spellings, and knows it.** `Diagnostic`'s fields are
    public, so a native could build a warning and then assign its `severity`
    from a variable. So criterion 24's audit has a run-time half, added for
    the ninth review's S4 at the owner's choice. While any native runs, a
    `Vm::report` at `Error` severity is a breach, whatever spelling built it.
    The scan covers code the corpus never reaches. The audit covers every
    spelling on the paths it does reach.

26. **A body a call asks for runs before the next value.** Compile each of
    these twice, once with the call in a non-tail position and once in a tail
    position: `{ 10 } ! 20`, `true { 10 } if 20` and
    `:f { 10 } register f 20`. Each must leave `10` beneath `20`, as Tier 0
    does. Then compile `{ 1 } ! { 2 } !`, where a second request follows the
    first: both bodies must run, in order. The criterion fails for a lowering
    that ignores the request cell, one that drains in tail position instead of
    handing the request back, and one whose entry takes a returned request
    late (§S5, *A call may leave a body to run*). Needs a tier.

    Two more cases, for the tenth review's S3. A request filed inside a body
    the drain started must itself be drained before the drain returns. And a
    native that files a request and then fails, under `?try`, must leave no
    body to run afterwards (F96). The second case's Tier 0 half runs today:
    `a_failed_native_leaves_no_tail_request`
    (`crates/bund2-interp/src/lib.rs`).

    **The second case's shape**, from the fifteenth review's B1. Register a
    native that files a body through `Vm::tail_lambda` and then returns `Err`,
    as `a_failed_native_leaves_no_tail_request` does
    (`crates/bund2-interp/src/lib.rs`). Call it from a compiled body under
    `?try`, at threshold 1. The final stack must hold `?try`'s CONDITIONAL and
    nothing the filed body would have left, which is what Tier 0 gives
    through `Interp::invoke`. It fails for an adapter whose `status_of` clears
    the request cell only for an exit.

    The review's own program was `?try` over `[ { 10 } 5 ] !`, when
    `execute_value` filed each lambda a list reached. F113 runs one at once
    instead, so the arm no longer files and the case is an embedder's native
    again. F113 took two passes: the first tested the item's own type, and a
    lambda held in a dict inside the list still filed (the sixteenth review's
    B1). What those programs do now is
    `a_list_execute_that_fails_keeps_what_earlier_items_left` and
    `a_lambda_reached_through_a_dict_in_a_list_runs_at_once`
    (`crates/bund2-stdlib/src/host.rs` and
    `crates/bund2-stdlib/src/values.rs`).

27. **Promotion does not cross a native `bund2-stdlib` did not register
    (D47).** From the test, register a native declaring `eff(1, 1)` that
    replaces its operand with the current depth. Its pair is honest, and it
    still observes beyond its operand. Compile a body that promotes, calls it,
    and continues. The result must match Tier 0's, and the lowering's side
    table (criterion 17) must record the call as synced, not crossed. Needs a
    tier.

28. **Promotion crosses only natives a palette has checked (D48).** Criterion
    24 checks the arms the corpus reaches, and `drop_stack`, which no program
    calls, declared `1 -> 0` while removing the whole current stack (F94). So
    `every_fixed_effect_native_keeps_its_pair_over_the_promotable_palette`
    (`crates/bund2-stdlib/src/lib.rs`) runs every fixed-effect native under
    the effect audit against fifteen operand kinds, the current stack's name
    among them (D55). The top two operands take
    every pair of kinds. A workbench form also runs with every kind on the
    workbench. The test asserts two things: no run breaches its declared
    effect, and the natives some run brought to `Ok` are exactly those
    `tests/golden/PROMOTABLE.txt` lists. §S5's promotion crosses only those.
    **Met**, 2026-09-10, when 178 of 205 fixed-effect natives were listed;
    222 promotable of 230 reached, of 267 fixed-effect natives, after D55,
    F111 and the MATRIX family's four convert words. The list's header line said "218 of 263 … 8 of them observe" until
    the seventeenth review's S3 found that arithmetic mis-stated — the 8
    observers are the difference between reached and promotable, not part of
    the 218 — so the generator now writes the three figures separately, and
    the file carries them from its next regeneration
    (`BUND2_UPDATE_PROMOTABLE=1 cargo test -p bund2-stdlib promotable`, the
    repository owner's to run). The comparison ignores comment lines either
    way, which read
    227 of 269 earlier on 2026-09-11.
    **Mutation-checked**: before F94's fix it named `drop_stack` and nothing
    else, "declares (1, 0) and moved `main` from 4 to 0". Since D55 the list
    certifies both a native's pair and that D55's audit saw it read nothing
    beyond its operands (criterion 14). Runs today: `cargo test -p bund2-stdlib promotable`.

    **Two limits, stated for the eleventh review's S4.** First, six natives
    that act on the host are never run: `fs.rm`, `sleep.seconds`,
    `system.setproctitle`, `system.setproctitle.`, `password` and
    `save.model` (`ACTS_ON_HOST`, `crates/bund2-stdlib/src/lib.rs`). So
    "every fixed-effect native" means every one but those. That is the
    conservative side, since an unrun native is not listed and not crossed.
    But the exclusions are kept by hand, so a new host-acting native runs for
    real under `cargo test` until it is added. Second, the list names natives,
    so it certifies the **default** registration. `--noio` and `--noeval`
    register failing stubs under the same names (`register_all_with`), and
    D47's id set is taken from the registration actually made. A listed stub
    is therefore crossed under those options, and since every stub fails, the
    crossing takes the error path, which syncs first.

29. **A native that panics gives Tier 0's result from compiled code (D49).**
    Register a native that panics. Call it from a compiled body in a non-tail
    and a tail position, and assert the same observable result Tier 0 gives:
    `Error::internal` naming the native, reported through the reporter, with
    nothing on stderr and no abort. It fails for an adapter that lets the
    panic unwind, which would abort through `extern "C"` or unwind through
    frames with no unwind tables (§S8). **The call must not pass through a
    native that discards its callee's error** — `#` and `#.` do
    (assumption 32) — or the criterion passes without exercising the adapter.
    The Tier 0 half is **Met**:
    `a_panicking_native_is_an_internal_error_not_an_unwind`
    (`crates/bund2-interp/src/lib.rs`).
    F95's `jarowinkler` program reports, and exits 0.

    **The compiled half is Met, 2026-09-13.** A compiled body calls a panicking
    native through its `Tail` thunk and the Rust adapter, in a **non-tail**
    position and in a **tail** position, and both give Tier 0's result:
    `a_panicking_native_in_a_non_tail_position_matches_tier_zero` and
    `a_panicking_native_in_a_tail_position_matches_tier_zero`
    (`crates/bund2-jit/src/lower.rs`). The assertion is against Tier 0's
    **text** — `internal error: native `boom` panicked: boom from a
    dependency` — and not merely against the error being internal, because
    D49's claim is that the two tiers word one failure the same way. Nothing
    reaches stderr and the process survives: the test runs a second compiled
    body afterwards and it works.

    **The caveat the criterion names does not apply here.** "The call must not
    pass through a native that discards its callee's error" — the body calls the
    native directly through its thunk, with no `#` or `#.` between them.

    Runs today: `cargo test -p bund2-jit --features jit`.

30. **A program ended by `bund.exit` ends at the same point in compiled code
    (D52).** Run `tests/probes/bund-exit.bund` and
    `tests/probes/bund-exit-word.bund` under criterion 2's three
    configurations. Then compile, with promotion on, bodies that call
    `bund.exit` directly, through the alias `exit`, and in tail position, each
    followed by a call and by an inlined `+`. Output, exit code and final
    stacks must match Tier 0's. It fails for an adapter that returns success
    after a recorded exit (§S5, *A call may end the program*).

    **Four more cases, for the twelfth review's B1.** Each is followed by a
    `println`, and each runs with the callee held **below the compile
    threshold**, since threshold 1 compiles the callee and hides the defect:
    - a compiled caller of a cold lambda whose last word is `exit`;
    - the same reached as an `if` branch, as in
      `3 { "tick" println true { "bye" println 7 exit } if "after if" println } times`;
    - a body on the residual path whose last value is `exit`;
    - a callee that declines below the Tier 1 floor.

    Each fails for a helper that makes its status other than through
    `status_of`.

    **The mirror cases, for the thirteenth review's B1**: a compiled body
    whose caller is a native. Each runs at threshold 1 with the body hot:
    - `[1] { 7 exit } map` and `[1 2] { 7 exit } map`, each leaving `1`;
    - `?try` over `try` bodies that reach `exit` directly, through `map`,
      through `times`, through `bund.eval` and through a `context` body.
      These compare the final stack, with the `error` CONDITIONAL's id and
      stamp normalised (F14) but its `context` slot compared as text;
    - a drained body that reaches `exit` through a native:
      `?try :try { { [ 1 ] { 7 exit } map } ! } set :except { "EXCEPT" println } set :recovery { "RECOVERY" println } set !`,
      where `!` files the outer body for the drain;
    - a residual-path value that reaches `exit` through a native, the same
      program with `:scratch to_stack` before the `!`, so the stack switch
      puts what follows on the residual path.

      Both compare the final stack the same way as the `?try` bullet above:
      the `error` CONDITIONAL's `context` slot as text, its id and stamp
      normalised (F14). Tier 0 leaves `MAP: lambda execution returns error: …`
      inside that text, so a helper whose `status_of` replaced the native's
      error would fail them;
    - `"p> " { println 7 exit } input*`. **The harness:** the test writes one
      line to the process's stdin and holds the pipe open. It asserts that
      the process exits with code 7 before a second line is written.

    Output, exit code and final stack must match Tier 0's. Afterwards the
    request cell must be clear (§S5, *A call may leave a body to run*).
    **Tier 0's halves are Met:**
    - `map`, by `an_exit_ending_a_synchronous_body_stops_the_native_that_ran_it`
      (`crates/bund2-stdlib/src/host.rs`);
    - `?try`, by `try_keeps_the_error_its_body_returned_after_an_exit`
      (same file). It also pins F112's `Interp::apply` and `Vm::scoped_call`
      gates, whose only visible effect is the text. **What it covers:** that
      the `context` slot ends with the refusal and carries each native's
      wrapper. It does not compare whole final stacks, which the compiled half
      must;
    - the drained-body and residual-path cases, by
      `a_drained_body_that_exits_through_a_native_keeps_its_error` (same
      file);
    - `input*`, by `input_loop_reads_no_line_after_an_exit`
      (`crates/bund2-cli/tests/input_exit.rs`).

    The audit's
    half runs today: a fixed-effect native that requests an exit is a breach,
    `an_exit_requested_under_a_fixed_effect_is_a_breach`
    (`crates/bund2-interp/src/lib.rs`). The compiled half needs a tier.

## Open questions

- **Q25 — what replaces the stack tag on `push`?** §S1 requires
  `value/push_pull/balanced` under 20 ns and deliberately does not say how.
  The tag is observable (goldens capture `tags: {"stack": "main"}`), so the
  options — write-on-observe, interned stack names, or moving the tag to the
  stack's bookkeeping — differ in what they preserve. **Answered by D41**
  (interned stack names, in the value's padding), which §S1's update records;
  this RFC is no longer blocked on it.
- **Q26 — resolved while drafting RFC-0001's amendment, not open.** The
  `identity()` call on every push is **D13's policy**: RFC-0001's "One policy
  for the `Rc` and the identity slot" requires a CoW split to materialise
  identity before copying, and names `set_tag` on push as the case that fires
  it. What is over-applied is that it is unconditional; that is option 1 of the
  amendment, not a separate question.
- **Q27 — which target profile is Bund2 optimising for?** §3.2a's code-memory
  constraint "bites hard" on a REPL and "barely at all" on long-running batch,
  and §S7's caps are chosen for the former. The study raised this as its own
  open question 7 and it is still unanswered.
- **Q28 — does `s390x` matter?** §S8 degrades tail calls there. If s390x is
  not a target, the degradation path is dead code that will not be tested.
  **Narrowed 2026-09-13:** the question is now about *targets*, not tail calls.
  At `cranelift-codegen` 0.135.0 s390x lowers `return_call` like every other
  backend, so there is no degradation path to test and the first lowering
  carries no branch for one (§S8, dated note). What remains of Q28 is whether
  s390x is a platform Bund2 ships for at all, which §S10's portability story
  answers for the interpreter and RFC-0006 will answer for AOT.
- **Q34 — answered 2026-09-11 by the owner: D55.** The set is derived by
  criterion 28's palette, through a four-run differential and an observation
  audit, and kept off `PROMOTABLE.txt`. Criterion 14 is satisfiable.
- **Q35 — answered 2026-09-10 by the owner: `q` is kept but not averaged.**
  D32 is amended to match, and §S6's constraint 2 now rests on it.
- **Q36 — answered 2026-09-10: D42.** The value reaches every entry point;
  built.
- **Q37 — answered 2026-09-10: D43.** The registration id and the stable
  generation cells, built when this RFC reaches Proposed.
