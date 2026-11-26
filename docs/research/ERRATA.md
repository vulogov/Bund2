# Errata

Supersessions of `docs/research/`. One line each. Never edit the originals.

Format: `<document> §<section> -> superseded by <RFC> §<section> (<reason>)`

---

- `00-jit-feasibility.md` §4.1 -> superseded by the OOP finding in
  `03-metaprogramming-oop-debugger.md` §2.2: `.id` and `.timestamp` are public
  API on the base `Object` class, so the value cannot shrink to 16 bytes.

- `02-native-binaries.md` §4 -> superseded by `03-metaprogramming-oop-debugger.md`
  §0.2: `bund.eval` exists, so closed-world analysis rarely fires and the front
  end is mandatory in every binary.

- `00-jit-feasibility.md` §"Variadic words" (line 485) and the risk row at
  line 587 -> corrected by `cargo xtask corpus`: the survey the risk row asks
  for ("survey real Bund programs before committing") has been done, and the
  arithmetic fold-family is **entirely unused**. `*+`, `*+.`, `*-`, `*-.`,
  `**`, `**.`, `*/`, `*/.`, `*loop`, `*loop.` and the Unicode aliases `Σ`,
  `Σ.` have zero uses across all 132 programs. The mitigation "be prepared to
  restrict the `*`-family in JIT-able positions" is therefore unnecessary, and
  D12 accepts the barrier because it costs nothing measurable. Note the open
  question at line 650 — "how variadic is real Bund code?" — is answered: not
  at all, in the corpus. `lambda*` is the exception, at 2 uses, and it is not
  an arithmetic fold.

- `00-jit-feasibility.md` §5 (line 128) and `02-native-binaries.md` line 151
  -> corrected by `cargo xtask corpus`: both discuss `execute` /
  `stdlib_execute_base_inline` by its registered name, but **no program spells
  it `execute`**. It is spelled `!`, an alias registered at
  `reference/rust_multistackvm/src/stdlib/create_aliases.rs:5`, and that
  spelling is one of the five most-used words in the corpus — 69 invocations
  across 39 of 132 programs, 13 of them inside a lambda body. A survey keyed
  on the registered name returns zero and reads as "no dynamic dispatch",
  which is the opposite of the truth. `execute` is also polymorphic over eight
  dispatch arms, of which the corpus reaches four
  (`reference/rust_multistackvm/src/stdlib/execute.rs:26-97`).

- `00-jit-feasibility.md` §2 (lines 48-55) and
  `03-metaprogramming-oop-debugger.md` §1 (line 19) -> corrected by
  `cargo xtask corpus`: both describe word registration as coming from
  `rust_multistackvm` (156 words) plus the CLI (357 total), which is **two
  tiers**. There are **three**. `i_direct` tries the VM's own inline table
  (`reference/rust_multistackvm/src/multistackvm_inline.rs:42`) and on a miss
  falls through to the stack layer's own inline table (`:52`), which is
  `reference/rust_multistack` — and that third tier holds the core stack
  words: `take` (`rust_multistack/src/stdlib/workbench.rs:81`, used by 29
  programs), `drop` (`stdlib/drop.rs:71`, 16 programs), `dup_one`, `swap_one`,
  `return`. A scan of only the two documented crates reports those as
  unregistered. RFC-0002's slot table has to absorb all three.

- `04-consolidated-architecture.md` §2-§3 -> superseded by
  `docs/rfc/RFC-0000-architecture.md` as the statement of record for crate
  layout, boundaries and the tier model. §1.1 is deliberately not superseded:
  it mixes layout with value, symbol and lambda claims belonging to RFC-0001,
  RFC-0002 and RFC-0003, and RFC-0000 disclaims that scope.

- `00-jit-feasibility.md` §"leading `$`" (line 119) and §"`$name`" (line 556)
  -> corrected by F26: `$` does **not** bypass alias resolution. It skips the
  lambda check only. `call_internal_word` strips the sigil and calls `i()`
  (`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`),
  and `i()` resolves aliases first
  (`reference/rust_multistackvm/src/multistackvm_inline.rs:71-72`). Verified
  against the oracle: `1 "$dup" ptr !` resolves through the alias table.
  Line 556 proposes a distinct IR opcode premised on full bypass; that premise
  is false.

- `05-rfc-roadmap.md` §6, the row "24-byte `{tag, payload, birth}`; lazy
  `.id`; sampled `.timestamp`" -> superseded by RFC-0001: the measured value
  is **16 bytes**, and the tag is not in it. `cargo xtask layout` puts
  candidate A (identity in the heap header) at 16 and candidate D (the same
  plus the reference's `dt`) also at 16, because the tag is only ambiguous for
  heap types and rides in the header. Lazy `.id` and sampled `.timestamp` are
  unchanged and are D1 and D2. The 24-byte figure survives only as candidate
  C, the cheapest *inline*-identity shape, which the scan rules out.

- `00-jit-feasibility.md` §4.1's supersession above — "`.id` and `.timestamp`
  are public API on the base `Object` class, so the value cannot shrink to 16
  bytes" -> **retired by RFC-0001**. The premise is right and the conclusion
  does not follow. `.id` and `.timestamp` are public
  (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:91-92`) and `.id`
  returns a string (`:16`), but identity moves to the *heap header*, not off
  the value entirely, so both stay answerable while the value measures 16
  bytes — `cargo xtask layout`, candidate D. What the entry correctly rules
  out is candidate B, identity carried inline, which measures 32.

- `00-jit-feasibility.md` §2.1's expectation table -> superseded by RFC-0005
  §S1, which replaces its rows with measurement. The document asked for exactly
  this — "order-of-magnitude reasoning, to be replaced by measurement in
  Phase 0" — so this is the substitution it invited. The rows that survive:
  "Cranelift tier: naive lowering ~1.2–1.5×" is corroborated, since dispatch is
  at most a quarter of an average word's cost and Amdahl bounds a perfect
  Tier 1 near 1.3×. **That corroboration is itself withdrawn, 2026-09-09**: the
  "at most a quarter" rested on comparing a per-word figure against a
  per-push/pull one, which are not commensurable, and RFC-0005 §S1 now declines
  to replace the bound because no benchmark separates dispatch from the work a
  word does once dispatched. §2.1's row is neither corroborated nor refuted
  here; RFC-0005 criterion 10 makes it an experiment. The row that does not
  survive as written is the framing that value-representation work was already
  behind us: `value/with_tag/scalar`
  measures **73.1 ns** against a **4.1 ns** clone and a **126.9 ns** push/pull
  round trip, so §2.2's gate — "Project B is worth doing only if Project A's
  measurements show that dispatch and boxing are still the bottleneck" — does
  not pass **when RFC-0005 was drafted**. Boxing was the bottleneck; dispatch
  was not.

  **That is no longer the state, 2026-09-09.** D41 moved the stack tag into the
  value's padding: `value/push_pull/balanced` went 126.9 → 9.8 ns and the
  `dispatch/*` programs improved 3.8–4.2×. RFC-0005 §S1's update carries the
  numbers. Half the gate is now met — the 20 ns prerequisite — and the other
  half, that dispatch has become the dominant term, **cannot be shown with
  these benchmarks** and is RFC-0005 criterion 10's experiment. This paragraph
  is kept because it is what the gate turned on, not because it is current.

  **A benchmark name above, corrected 2026-09-10.** `value/with_tag/scalar` is
  not a benchmark that exists: `crates/bund2-bench` has
  `value/with_tag/scalar_unique` and `value/with_tag/heap_shared`, and RFC-0005
  §S1's footnote records that which of them produced the 73.1 ns cannot now be
  established. The figure stands as a record of that day's measurement.

- `02-native-binaries.md` §1 and §9 -> superseded by RFC-0006 §B1 and §B5: the
  `include_bytes!`-and-link construction it describes needs `cc` on Linux and
  macOS, which D10 forbids below `bund2 build`; and its "point the same
  lowering at `ObjectModule`" ordering is moot, the lowering having been built
  JIT-first under RFC-0005 and baking host heap addresses as immediates.

- Correction to the entry above, 2026-10-09 (RFC-0006's eleventh review, S5):
  "needs `cc` on Linux and macOS" states for Linux what was measured on macOS
  only. RFC-0006 §B1 has said since 2026-10-08 that Linux was not measured.
  The supersession stands; the Linux half of its reason is unmeasured.

- `02-native-binaries.md` §1 -> superseded by RFC-0006 criterion 1 (measured
  2026-09-30): "`cranelift-codegen` with its ISLE-generated instruction
  selection tables is the single largest code contributor to any binary that
  embeds the JIT" is false for Bund2. Release builds with and without `jit`
  differ by 2,259,088 bytes, 12.45%; Cranelift and `regalloc2` own 8.9% of
  `__text`, where `graphitesql` alone owns 13.0% and `sqlparser`, `prqlc` and
  `redb` together own 17.2%. Product B's stated strongest argument — shedding
  the code generator being "a bigger practical win than the arithmetic
  speedup" — does not hold at that share.

- `03-metaprogramming-oop-debugger.md` §3.3(h) -> superseded by RFC-0008 §D7
  (2026-09-30): "the tier-up call counters give you a profiler for free" does not
  hold in Bund2. Its counters — `compiled_bodies`, `compiled_entries`,
  `crossed_calls` — are totals on the tier rather than per-word times, they exist
  only in `jit` builds, and §D6 removes the tier for a debugged session, so a
  profile taken under the debugger reads zero by construction. A profiler needs
  its own counters on the frame loop.

- `03-metaprogramming-oop-debugger.md` §3.3(g) -> superseded by D113's ruling
  of 2026-10-07, recorded here on 2026-10-13: "`"…" debug` becomes a thin
  wrapper: eval the string with the debugger attached" was not done. `debug`
  stays the reference's word, a loop over a string's top-level terms, and a
  new word, `debug.run`, is the wrapper that stops at each term and steps
  into what it calls. And §3.3(a)'s `Interp::step()` does not exist: the
  debugger is a safepoint in the frame loop (D84, RFC-0008 §D1).

- `01-extensibility-async.md` §2.3 -> superseded by RFC-0007 §C3 (2026-10-01):
  "`Value::wrap()` produces an `ENVELOPE`… the message-passing boundary between
  VMs is a mechanism the language already has a word for" does not hold. The
  reference bus calls `to_binary` directly (`bus/mod.rs:88`) and constructs no
  envelope, and the Bund words `wrap`/`unwrap` are OOP `.data` words rather than
  a serialisation boundary. D11 carries the same error.

- `01-extensibility-async.md` §2.6 and §2.7 -> **not** superseded, but in
  conflict with D60 and RFC-0005 criterion 23, recorded by RFC-0007 §C8
  (2026-10-01). The research calls a per-VM `JITModule` "Bad" under concurrency
  and lists one shared compile service as "required under concurrency"; Bund2
  built the per-`Interp` form, which D60 derives from a lifetime and criterion
  23 asserts. Tolerable while D85 bounds the count at tens and F139 shows no
  corpus program compiles a body at the shipped threshold. The trigger for
  revisiting, and the collision with criterion 23 it would cause, are in §C8.

- `01-extensibility-async.md` §2.1(c), §2.4 and §2.7 -> superseded by D84
  (2026-09-30), recorded here on 2026-10-14 at RFC-0007's review of that
  date (B2): "(c) A Bund word performs I/O and yields mid-execution …
  achievable at Tier 0" (`:230`), §2.4's await that "returns control to the
  async driver with the pending future stored in the frame" (`:298-299`),
  and §2.7's "Await inside a Tier-0 word: yes" (`:389`) do not hold. D84
  made a task a VM with its own thread and removed suspension inside a
  word; nothing in Bund2 awaits inside one.

- `01-extensibility-async.md` §2.4's `NativeKind` (`:302-307`) and §2.7's
  "`Sync` / `Blocking` / `Async` native word kinds: required for any async"
  (`:388`) -> departed from by RFC-0007 §C5, recorded here on 2026-10-14
  (B2). Bund2's `WordKind` has the three variants and no reader; `Blocking`
  is not dispatched to `spawn_blocking` (there is no executor, D92) and
  `Async` carries no future. What the variants mean is D164's question.

- `05-rfc-roadmap.md:96` -> RFC-7's "tier pinning" is moot, recorded here
  on 2026-10-14 (RFC-0007's review, S5). It was the research's coloring
  rule, pinning a word that awaits to Tier 0 (`01-extensibility-async.md:390`);
  with no await inside a word (D84) there is nothing to pin, and RFC-0007
  specifies none.

- `01-extensibility-async.md` §2.6 and §2.7, the entry above on the shared
  compile service -> D91 resolved it on 2026-10-03, recorded here on
  2026-10-14: option 1, keep per-`Interp` modules. The entry's "in conflict"
  stands as a description; the conflict is now decided rather than open.

- `01-extensibility-async.md` §2.3, the 2026-10-01 entry above on
  `Value::wrap()` -> corrected, 2026-10-14 (RFC-0007's seventh review, S2).
  That entry said the research's sentence "does not hold" and corrected the
  wrong half of it. `Value::wrap()` exists and produces an `ENVELOPE` holding
  the bincode serialisation, and `unwrap()` reconstructs it
  (`reference/rust_dynamic/src/bincode.rs:79-103`), as the research says.
  What does not hold is "a mechanism the language already has a word for":
  no Bund word calls either method; the words `wrap` and `unwrap` are OOP
  `.data` words. The reference bus uses neither (`bus/mod.rs:88`).

- `02-native-binaries.md:183-184` -> the spelling `--profile` is taken by
  another feature, recorded here on 2026-10-17 (RFC-0011, D185). The
  research's `bund2 run --profile prog.prof` records observed type tags for
  profile-guided AOT; D83 withdrew the AOT and nothing records type tags.
  D185 gives `--profile <base>` to RFC-0011's profiler of words, which
  writes a markdown report and a Graphviz file. If profile-guided guard
  ordering ever returns it needs another name; the research's sentence is
  left as the design it was.

- `06-biomathematics-and-genetics.md:70,74,116,217` -> `argmin`'s
  Gauss–Newton is not usable as linked, and so "Phases 1 and 4 … add no new
  crate" rested on a solver that cannot be built; recorded here on
  2026-11-14 (RFC-0015's review, B1). `GaussNewton` needs `ArgminInv` of
  its Jacobian, which `argmin-math` 0.4.0 implements only in its nalgebra
  and ndarray backends; the lock compiles neither, and the newest nalgebra
  backend is 0.32. RFC-0015 revision 2 drops `argmin` and specifies a
  Levenberg–Marquardt of Bund2's own over `nalgebra`, so the conclusion
  "no new crate" holds by another route. The research's §3.4 ChaCha
  generator value is refined, not contradicted: RFC-0015 chooses a
  `StdRng` per VM (D250 B4).

- `07-statistical-research.md:133-140,216` -> RFC-0016 departs from §3.4
  and from §4's Phase A row, recorded here on 2026-11-16 (RFC-0016's
  second review, S6). §3.4 says "Every test answers its interval and
  estimate … adding keys breaks no program that reads `p`", and Phase A
  puts "intervals, estimates and alternatives on RFC-0015's tests". After
  the first review's B1, D251's A2 was re-ruled C: RFC-0015's tests answer
  exactly what they answered, and estimates, intervals and alternatives
  come only from new `<test>.opts` words. Adding keys would have broken
  more than §3.4 counted: the plain words would gain refusals (a quantile's
  polish, a bisection, the Hodges–Lehmann cap), costs (O(n·m) for
  Mann–Whitney, up to 798 support sums for Fisher), and a changed printed
  key order, since a MAP's keys sort. In the same section, "Spearman's and
  Kendall's exact and asymptotic p" is narrowed by D251 A5: Spearman's p
  is the t approximation only; Kendall's keeps both.

- `07-statistical-research.md:91-92` -> departed from by RFC-0017,
  recorded here on 2026-11-17 (RFC-0017's second review, N13). §3.1 calls
  "a MAP of equal-length LISTs … (`"dose" get`)" the natural convention
  for a table. D252 ruled C1 B, a MAP with reserved keys (`columns`,
  `data`, `levels`), so a column is `"data" get "dose" get`, and `"dose"
  get` on a frame is `get`'s missing-key error. §3.1's "reading a whole
  file into such a MAP" is likewise refined: RFC-0017 parses the STRING
  that `file` reads (`frame.csv.parse`) rather than adding a word that
  opens a file. §5 posed the shape as question C1, so the research
  reached no conclusion this contradicts; the entry records the
  departure from its stated default.

- `07-statistical-research.md:109-127,218` -> narrowed and refined by
  RFC-0018, recorded here on 2026-11-20 at its acceptance (D261's K1,
  ruled A). Phase C (`:218`) is built as drawn: `model.matrix`, `fit.lm`,
  `fit.glm` with weights and offsets, residuals, influence, prediction, F
  and likelihood-ratio tests, and the type I `anova`, with BIC beside AIC.
  §3.3's table (`:118-127`) is narrowed in six places, each left to a
  later RFC as a self-contained addition:
  - variance inflation, in the diagnostics row (`:125`), is not built;
  - robust HC0–HC3 standard errors (`:126`) are not built;
  - `anova` types II and III (`:124`) are not built; only type I is;
  - the quasi families (`:123`) are not built (K9);
  - the negative binomial (`:123`) is not built (K9);
  - `fit.logistic` does not "become one case" of `fit.glm` (`:122`). It
    is kept, its text and answers unchanged, and `fit.glm` under the
    binomial `logit` agrees with it (K15).
  §3.2 is refined in two places. `model.matrix` answers a MAP holding the
  design MATRIX, with its names, term assignment, response, weights,
  offset and spec, rather than "a design MATRIX" (`:111`). "Weights and
  offsets for every model word" (`:116`) holds for RFC-0018's words;
  `stat.lm` and `fit.logistic` take neither, unchanged under K15.

- `07-statistical-research.md:149,155-160,171` -> narrowed and renamed
  by RFC-0019, recorded here on 2026-11-22 at its acceptance (D266's N1,
  N9 and N11, ruled A). Phases D and E (`:219-220`) are built as drawn:
  the bootstrap and the jackknife, the permutation test, `optim`'s
  minimisers, `fit.mle`, root finding and quadrature; the noncentral
  distributions, power, the studentized range with Tukey's HSD, and the
  further tests. Four places depart from §3.4–§3.6:
  - §3.5's `stat.bootstrap` (`data statistic B -- MAP`, `:155`) is
    `boot.sample` (`data statistic R opts -- MAP`), under N1's `boot.`
    prefix. The replicate count is R, as `boot` names it, and the
    interval types are asked for in `opts`. No name proposed is
    `bootstrap` or begins with it, which keeps the reference's
    `bootstrap` word apart;
  - §3.5's permutation tests "for two samples and for a statistic over
    a frame" (`:158-160`) are narrowed to two samples.
    `stat.test.permutation` takes `a b statistic opts`; a test over a
    frame is left to a later RFC;
  - §3.6's "bounds by transformation" (`:171`) are not built (N11). The
    minimisers are unconstrained, and a later RFC can add `lower` and
    `upper` keys;
  - §3.4's Dunnett test (`:149`) is not built (N9). Its multivariate t
    cdf is Genz's randomised quasi-Monte Carlo in R's `multcomp`, so its
    answer varies between runs.
  MCMC, which §3.6 lists, is not a departure: research 07's own §4 puts
  it in Phase H (`:223`).

- `07-statistical-research.md:186-193,221` -> refined and narrowed by
  RFC-0020, recorded here on 2026-11-25 at its acceptance (D277, D278).
  Phase F (`:221`) is built as drawn: PCA by SVD, classical MDS,
  k-means with seeded starts, hierarchical clustering, Hotelling's T²,
  MANOVA, LDA, kernel density, the ECDF and histograms. Four places
  depart from §3.7:
  - §3.7's linkages, "single, complete, average (UPGMA, already in
    `phylo.`) and Ward" (`:188-189`), are R's eight (V7), and average
    linkage is a port of R's `hclust.f`, not `phylo.upgma`, whose tie
    rule differs;
  - §3.7's "Silverman or Sheather–Jones bandwidth" (`:191-192`) gains
    Scott's (`nrd`), as R's `bw.nrd`, beside `nrd0` and both
    Sheather–Jones methods (V9);
  - §3.7's "classical MDS from `linalg.eigen.sym`" (`:187`) holds
    for the solution; the additive constant (`add`) needs Cailliez's
    non-symmetric matrix and takes `linalg.eigenvalues.complex`'s
    solver (§S4);
  - §3.7's "MANOVA" (`:190`) is narrowed to one-way MANOVA over groups
    (V12); MANOVA over RFC-0018's designs is left to a later RFC.

