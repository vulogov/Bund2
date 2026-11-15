# RFC-0015: Biomathematics and genetics — numerical, statistical and sequence words

- Status: **Accepted** (2026-11-14) by the repository owner ("accepting
  RFC-0015"), at revision 2, after D250's B1–B16 were ruled "all as
  recommended". Revision 2 answers
  `docs/rfc/reviews/RFC-0015-review-2026-11-14.md`; §Revision 2 maps each
  finding to the change it made. Accepting means what "Which points are
  rulings" lists:
  - D250's B1–B16 as ruled;
  - S1–S18 as specified;
  - the four phases built in order, each gated by its criteria;
  - the family as Bund2's own, outside D14's in-scope set (D72).
- Depends on: RFC-0003 (natives, `Vm::scoped_call_from`), RFC-0004
  (effects), RFC-0007 amended by D249 (`vm.spawn`, for replicates),
  RFC-0013 (`kb.*`'s bridges, §S18 only)
- Decisions consumed: D14, D18, D21, D24, D28, D36, D37, D38, D39, D52,
  D57, D72, D111, D171, D175, D213, D215, D226 (L2, L11, L13), D243, D249;
  F85, F112, F123. **Rulings: D250, B1–B16, RESOLVED 2026-11-14**, "all
  as recommended": B7 is C, every other point A. Each passage that rests
  on one is marked **[B*n*]**.
- Reference SHA: `reference/Bund` 21b40b0213a76f3f39bd6de0d2bdfdf0a53b0dca;
  `reference/rust_multistackvm` 4605832678d4792c2b8e0564b7757d5aee1539a4;
  `reference/rust_dynamic` ceb27c96fa10c4edb09106f3f904f22f94d63119.
  Third-party crates are cited at the versions `Cargo.lock` resolves,
  under `~/.cargo/registry/src/index.crates.io-*/`, written `<crate>-<ver>/…`.
- Supersedes: nothing. Grounds and refines
  `docs/research/06-biomathematics-and-genetics.md`. It contradicts that
  document's §2 on `argmin`, recorded in `docs/research/ERRATA.md`.

## Summary

A family of words of Bund2's own (D72) to make Bund usable for the everyday
computations of mathematical biology and genetics. It comes in four phases,
each usable on its own and each gated by its own criteria:

1. **Foundation:** special functions, element-wise vectors, probability
   distributions, descriptive statistics, hypothesis tests, multiple-testing
   correction, linear algebra and seeded randomness.
2. **Sequences:** nucleotide and protein alphabets, complement, translation,
   k-mers, motifs, and global and local alignment.
3. **Population genetics and phylogenetics.**
4. **Dynamics and fitting:** ODE integration, stochastic simulation and
   non-linear least squares, with the model given as a Bund lambda.

§S18 says how the family meets RFC-0013's logic programming: through the
bridges that already exist, with no new goal.

No existing word, value or golden changes, so conform moves by exactly
zero.

**No crate is added to the default build.** `statrs`, `nalgebra`,
`rand` and `rand_distr` are compiled in it today (§Current behaviour,
"What is linked"). Two of them become direct dependencies of
`bund2-stdlib`, at the versions the lock already holds. Phase 2's
algorithms, Phase 4's integrators and its least-squares solver are
Bund2's own code. `argmin`, which the first draft named, is not used:
its Gauss–Newton cannot be built from the `argmin-math` the lock compiles
(§Current behaviour).

## Motivation

A biologist using Bund today finds scalar functions and means, and little
else:

- **Scalar functions:** the unary functions of `float_math.rs`
  (`reference/rust_multistackvm/src/stdlib/math/float_math.rs:170-188`) and
  eight of `mathlab`'s (`reference/Bund/src/stdlib/functions/math/math.rs:7,153-160`).
  There is no `log2`, no gamma or beta function, no `erf` and no binomial
  coefficient.
- **Statistics:** `stat.count`, six means and one spread from `rstats`
  (`reference/Bund/src/stdlib/functions/statistics/statistics.rs:23-51`;
  the spread is `stat.mean.harmonicspread`, `hmad`, `:33`). There is no
  variance, median, quantile, correlation or test.
- **Distributions:** three, Normal, Uniform and LogNormal, offered only as
  named samplers (`reference/Bund/src/stdlib/functions/generators/mod.rs:21-42`).
  None has a density, cdf or quantile.
- **Randomness:** none of it is reproducible.
  - `math.random.int` draws from a Mersenne Twister in a lazy static
    (`reference/Bund/src/stdlib/functions/math/rand.rs:14-19,29-35`),
    seeded by `fastrand` when the library starts, because `init_stdlib`
    locks it (`rand.rs:54`).
  - `generator.sample` draws from `rand::thread_rng()`
    (`reference/Bund/src/stdlib/functions/generators/normal.rs:49-50`).
  - Neither takes a seed.
- **Matrices:** a MATRIX is a container whose arithmetic is element-wise
  only. A shape mismatch silently answers the left operand
  (`reference/rust_dynamic/src/math.rs:36-65`, D57).
- **Lists:** `+` on two LISTs concatenates them
  (`reference/rust_dynamic/src/math.rs:296-309`), `+` of a LIST and
  anything else appends it (`:315-319`), and every other operator on a
  LIST is refused (`:310-312`).

So the statistics of a genetics paper cannot be written in Bund: a
Hardy–Weinberg test, an association test with FDR correction, a growth
rate from a Leslie matrix. Neither can a Wright–Fisher simulation anyone
can repeat, or the reverse complement of a primer.

## Current behaviour

This is what the RFC must leave unchanged, and what its words must sit
beside.

### The series reader

The `stat.*` words, `math.normalize`, `math.max` and `math.min`
(`reference/Bund/src/stdlib/functions/statistics/minmax.rs:17`),
`math.smoothing`, `math.interpolation` and the `forecast` words take
their data through one reader:

- **Dispatch on the top value** (`reference/Bund/src/stdlib/functions/statistics/get_data.rs:184-223`):
  - a LIST is taken whole;
  - a METRICS yields each sample's `data`;
  - a NODATA on top is refused, "END OF DATA" (`:212`);
  - anything else drains the stack, value by value, converting each to
    FLOAT, until the stack empties or a NODATA is met (`:8-65`, the NODATA
    test at `:31-32` and `:51-52`).
- **The four forms.** Each single-series word has four:
  - plain: main stack, consume;
  - `.`: workbench, consume;
  - `,`: main stack, keep;
  - `.,`: workbench, keep

  (`reference/Bund/src/stdlib/functions/statistics/statistics.rs:53-67,185-188`).
  "Keep" pushes the LIST or METRICS back after reading it
  (`get_data.rs:105-113,170-178`). The drain path is never called with the
  mode (`:215`), so a drained series is never kept, whatever the form.
- **The result** is one FLOAT, pushed where the operand came from
  (`statistics.rs:41-44`). An `rstats` error becomes the word's error
  (`:35-39`).
- **Two series.** `math.interpolation` and the outlier words call the
  reader twice, and the first call takes the **top** series: `x` from the
  top, `y` from beneath (`reference/Bund/src/stdlib/functions/math/interp.rs:15-26`;
  `reference/Bund/src/stdlib/functions/forecast/outliers.rs:24-31`).

Bund2 builds the same reader (`crates/bund2-stdlib/src/series.rs`,
`get_data`) and the same four forms (D175, D213). Every series word is
declared `StackEffect::opaque`, "for the drain's sake" (the same file, the
comment above its registrations), so `bund2 effects` prints
`stat.count 1 0 opaque`.

### Scalar functions

- **The unary functions** pull one value, `cast_float` it, and push a
  FLOAT. `cast_float` accepts `Val::F64` alone
  (`reference/rust_dynamic/src/cast.rs:9-16`), so an INTEGER is refused,
  "FLOAT_OP returns error" (`float_math.rs:97-100,155-162`, and
  `math.rs:57-60,130-137` for `mathlab`'s eight). In Bund2,
  `4 math.sqrt` answers "FLOAT_OP returns error: This Dynamic type is not
  float: 2".
- **Out of domain they answer NaN.** In Bund2, `-1.0 math.ln` and
  `-1.0 math.sqrt` answer `NaN`.
- **Operand order is not uniform.** `math.nroot` passes the top operand
  as the radicand, `math::nrt(fvalue, nvalue)` with `fvalue` pulled first
  (`math.rs:57-82`), so `8.0 3.0 math.nroot` answers 3^(1/8) = 1.1472.
  `math.perimeter` also takes the top first (`math.rs:94-99`).
  `math.power` takes the deepest as the base, `math::pow(xvalue, fvalue)`
  (`math.rs:111-116`). `-` and `/` take the deepest as the right operand.
- **`math.factorial`** truncates its operand to `u64` before `mathlab`'s
  `fact` (`math.rs:68-69`).
- **`stat.mean` and `stat.mean.`** are aliases of `stat.mean.arithmetic`
  and its `.` form (`reference/Bund/src/stdlib/functions/create_aliases.rs:24-25`).

### Distributions and randomness

- **Generators** live in one process-wide map from a name to an entry
  (`generators/mod.rs:45-56`). The Normal reads `Mean` and `Deviation` from
  a configuration, defaulting to 0 and 1 (`normal.rs:24-25`). Bund2 keeps
  the same process-wide map (`crates/bund2-stdlib/src/random.rs`,
  `GENERATORS`) over `statrs` 0.18's Normal, Uniform and LogNormal (the
  same file, `Gen`).
- **The secure generator.** `math.securerandom.int` draws from a ChaCha20
  seeded from the operating system (`rand.rs:21-26,38-44`). Neither
  generator can be seeded by a program.
- **Promotion crosses them.** `math.random.int` and
  `math.securerandom.int` are in `tests/golden/PROMOTABLE.txt`, which the
  palette audit writes (`crates/bund2-stdlib/src/lib.rs`, the comment
  above `KINDS`). The hand-kept list of natives the audit never runs is
  `ACTS_ON_HOST` (the same file), and holds words that act on the host
  (files, the shell, the terminal, a model).

### MATRIX and LIST arithmetic

- **`numeric_op_matrix`** combines cell by cell. It returns the left matrix
  unchanged when the row counts differ (`rust_dynamic/src/math.rs:37-39`),
  a row's width differs (`:46-48`) or a cell pair will not combine
  (`:51-55`). Bund2 reproduces this (`crates/bund2-stdlib/src/math.rs`,
  `matrix_op`), and D57 records it.
- **A MATRIX holds BundValues** (`reference/rust_dynamic/src/types.rs:75`),
  and so does Bund2's (D57). It converts only to a LIST (D57).
- **The reference has a packed vector,** `Val::Embedding(Vec<f32>)`
  (`reference/rust_dynamic/src/types.rs:86`). Bund2 carries it only in
  the wire codec, and refuses it there (`crates/bund2-value/src/wire.rs`).

**`*` therefore cannot become a matrix product, and `+` cannot become
vector addition.** Both are preserved behaviours with goldens behind them.
Every operation this RFC adds is a word of its own.

### What is linked

The default build already compiles:

| Crate | Why it is there | What this RFC uses |
|---|---|---|
| `statrs` 0.18.0, without its default features, with `rand` | `generator` (`crates/bund2-stdlib/Cargo.toml`, the `statrs` line and its comment) | <ul><li>`ln_gamma`, `gamma`, `digamma` (`statrs-0.18.0/src/function/gamma.rs:54,84,373`)</li><li>`checked_ln_beta`, `checked_beta`, `checked_beta_reg` (`beta.rs:57,87,137`)</li><li>`checked_gamma_lr` (`gamma.rs:280`)</li><li>`erf`, `erfc`, `erf_inv` (`erf.rs:8,40,24`)</li><li>`ln_factorial`, `ln_binomial` (`factorial.rs:29,56`)</li><li>the `Continuous` and `Discrete` densities, and `ContinuousCDF` and `DiscreteCDF` (`distribution/mod.rs:96,135,167,204,232,270`)</li></ul> |
| `nalgebra` 0.33.3 | through `augurs-ets` (`cargo tree -i`) | LU, QR, determinant and inverse (`nalgebra-0.33.3/src/linalg/decomposition.rs:48,57`; `full_piv_lu.rs:219,249`); `try_symmetric_eigen` and `try_schur` (`decomposition.rs:341,307`); a Schur form's `eigenvalues` and `complex_eigenvalues` (`schur.rs:393,404`) |
| `rand` 0.8.7 | the random words | `StdRng`, seedable; `RngCore`; what `statrs` samples with |
| `rand_distr` 0.4.3 | through `algos` and `augurs-ets` (`cargo tree -i rand_distr@0.4.3`) | `Binomial`, which samples in O(1) (BINV below n·p = 10, BTPE above, `rand_distr-0.4.3/src/binomial.rs:102-128`) |
| `rstats` 2.1.15 | `stat.mean.*` (D213) | nothing: see S6 |

**What must be avoided in them.** Several of their entry points panic or
run without a bound, which D37 and D39 forbid. S4, S7 and S9 name the
checked forms, and S11 says what Bund2 checks before each call:

- `beta`, `ln_beta` and `beta_reg` are `checked_*(…).unwrap()`
  (`statrs-0.18.0/src/function/beta.rs:44-45,75-76,124-125`), and
  `gamma_lr` is `checked_gamma_lr(a, x).unwrap()` (`gamma.rs:264-265`).
- `checked_gamma_lr` refuses x ≤ 0, though P(a, 0) = 0, and answers NaN
  for a NaN operand (`gamma.rs:280-289`).
- `ln_gamma` below 0.5 takes `ln(sin(πx))` (`gamma.rs:54-66`), which is
  +∞ at 0 and NaN wherever sin(πx) < 0.
- `erf_inv` answers ±∞ at ±1 (`erf.rs:24-31`).
- `binomial` takes `exp` of a log difference and floors it
  (`factorial.rs:42-48`), so it is inexact for large n; `ln_binomial`
  answers −∞ for k > n (`:56-62`).
- The default `DiscreteCDF::inverse_cdf` panics for p outside [0, 1], and
  a NaN p reaches the panic (`distribution/mod.rs:198-210`). `Gamma`'s own
  panics the same way (`gamma.rs:184-187`), and its answer is eight
  bisection steps and at most four Newton steps (`gamma.rs:194-222`).
- `symmetric_eigen` and `schur` call the iteration with `max_niter` 0,
  which "continues indefinitely until convergence", and unwrap
  (`decomposition.rs:285,300-306,321`; `schur.rs:59`). The `try_` forms
  answer `None` at the cap (`decomposition.rs:307,341`).
- `statrs`'s `Binomial::sample` draws one uniform per trial, O(n)
  (`statrs-0.18.0/src/distribution/binomial.rs:113-124`).
- `statrs`'s Geometric counts trials, with support from 1
  (`geometric.rs:166-168`); R's `dgeom` counts failures, from 0.

**`argmin` is not usable.** It is linked through `augurs-forecaster`, but
its Gauss–Newton requires `ArgminInv` of the Jacobian, which
`argmin-math` 0.4.0 implements only for its nalgebra and ndarray backends.
The lock compiles neither (`Cargo.lock`, `argmin-math`'s dependencies),
and its newest nalgebra backend is 0.32, not 0.33. The first draft's
claim to the contrary, and research 06's, is withdrawn (ERRATA).

`statrs`'s Multinomial, Dirichlet and multivariate normal compile only
under its `nalgebra` feature (`statrs-0.18.0/src/distribution/mod.rs:16-17,30-35`).
That feature takes `nalgebra` 0.33 (`statrs-0.18.0/Cargo.toml:61-65`),
the version already linked. It also enables `nalgebra?/rand`, which
`augurs-ets` already enables: the lock shows `nalgebra` 0.33.3 depending
on `rand` and `rand_distr`. So the feature adds no crate. It does change
`bund2-stdlib`'s manifest, which today drops it because "no generator
uses" it.

### Calling Bund from a native

- **`Vm::eval_lambda`** runs a LAMBDA's body on the current stack to
  completion and returns, leaving its effect there (`crates/bund2-api/src/lib.rs`,
  the trait method's doc). Once an exit is recorded it returns an error,
  so the native runs nothing more (D52, F112). Each call costs one Rust
  frame per native, and below the stack floor it refuses with
  stack-exhausted rather than nesting (`crates/bund2-interp/src/lib.rs`,
  `eval_lambda`, "F85's fix"; RFC-0005 §S8).
- **RFC-0013's bridges do not use it.** `bridge::run`
  (`crates/bund2-stdlib/src/kb/bridge.rs`, `run`) runs the body with
  `Vm::scoped_call_from` on a scratch stack named per nesting depth,
  pulls the one value it left, returns to the caller's stack and drops
  the scratch stack. The body sees only what the bridge put there.
  `scoped_call_from` returns to the caller's stack however the body
  leaves (`crates/bund2-api/src/lib.rs`, `scoped_call`'s doc), and
  `scoped_frame` refuses below the stack floor
  (`crates/bund2-interp/src/lib.rs`, `scoped_frame`). The first draft
  said the bridges call `eval_lambda`; they do not.

S15 and S16 adopt the bridge's discipline (§S15, "The model").

### Rules of the house

- **D37:** no panic.
- **D36:** errors are `Diagnostic`s through `Vm::report`.
- **D39:**
  - an internal loop is bounded;
  - no Rust recursion is driven by a value a program controls;
  - a program's loop is warned at 10,000,000 iterations and never stopped.
- **D38:** a crate is pinned exactly.
- **D21:** goldens are captured from the oracle, so none can hold a word
  the oracle lacks.
- **D171:** a program that drives the host out of memory fails with a
  run-time error.

## Design

### S1. Shape of the family

- **Bund2's own (D72).** These words sit outside D14's in-scope set, as
  RFC-0013's do. Neither health number counts them. They are listed by
  `bund2 words` and audited like every other native.
- **Additive.** No existing word changes. In particular:
  - `+`, `-`, `*` and `/` keep their LIST and MATRIX meanings;
  - the `generator`, `math.random.int` and `math.securerandom.int` sources
    stay unseeded;
  - a program that defines a word of the same name still calls its own,
    since a slot's lambda resolves first (`crates/bund2-api/src/lib.rs`,
    `resolve_uncached`, as D249 cites it).
- **The build [B1].** In the default build, adding no crate (§Current
  behaviour, "What is linked").
- **Names [B5]:**

  | Prefix | Holds |
  |---|---|
  | `math.` | scalar special functions, beside the reference's |
  | `stat.` | descriptive statistics, tests and corrections, beside `stat.mean.*` |
  | `vec.` | element-wise series arithmetic |
  | `linalg.` | MATRIX algebra |
  | `dist.` | distributions |
  | `rng.` | seeded randomness |
  | `bio.` | sequences |
  | `popgen.` | population genetics |
  | `phylo.` | phylogenetics |
  | `ode.` | integration |
  | `sim.` | stochastic simulation |
  | `fit.` | estimation |

  `seq` is not available: it is the reference's numeric sequence word
  (`reference/Bund/src/stdlib/functions/math/seq.rs:155`). Among `stat.`
  names the reference registers only `stat.count`, the `stat.mean.*`
  family and the aliases `stat.mean` and `stat.mean.`
  (`statistics.rs:185-221`; `count.rs:52-55`; `create_aliases.rs:24-25`),
  so no new `stat.` name collides.

### S2. Values [B3]

- **A series** is what `get_data` reads: a LIST, a METRICS, or the stack
  down to NODATA. Results that are series are LISTs of FLOATs.
- **A matrix** is a MATRIX whose cells convert to FLOAT. `linalg.` words
  refuse a ragged one rather than returning an operand (S11, E103), so
  D57's silent arm is not copied.
- **A distribution** is a MAP **[B2]** with a `dist` key naming the
  family and one key per parameter. A program builds one with the
  constructor words of S7, or by hand as any MAP is built:
  `dict "dist" "normal" set "mean" 0.0 set "sd" 1.0 set`, which prints
  `{ dist=normal ::  mean=0.0 :: … }`. A MAP is chosen so that a program
  can build, print, save and send one with words that already exist.
- **A test's answer** is a MAP holding `statistic`, `p`, `df` where it
  applies, and `method` (S6).
- **A sequence** is a STRING (Phase 2). Alignments are MAPs (S12); a tree
  is a MAP of flat LISTs (S14).

No new value type is proposed. A packed FLOAT vector is the alternative
B3 weighs; it touches `bund2-value`, the wire format (`bund2_value::wire`)
and Tier 1. The reference's `Embedding` is f32, which loses the
precision S6's tests need, so it is not that vector.

### S3. Operands and forms [B10]

**Operand order.** For every word of this RFC the deepest operand is the
first parameter of the formula, so `10 3 math.choose` is C(10, 3), and
`a b stat.test.t2` tests a − b. This is a choice. The reference has no
single order: `math.power` agrees with it, while `math.nroot`,
`math.perimeter`, `math.interpolation` and the outlier words take the top
first (§Current behaviour).

**How operands are pulled.** A word's scalar and MAP operands sit above
its series, and are pulled first. Then:

- **One series** is read by `get_data`, so it may be a LIST, a METRICS
  or a drain.
- **Two series** (`vec.add` and the other binary `vec.` words,
  `stat.cov`, `stat.cor.*`, the two-sample tests, `bio.hamming`) must
  each be a LIST or a METRICS. The top is the second parameter. A drain
  is refused, E115, because a drained first series would consume the
  second.

**Forms.**

- **One series and nothing else** (`stat.var`, `vec.sum`, …): the four
  forms of their `stat.` neighbours, plain, `.`, `,` and `.,`.
- **Every other word that consumes operands** has a plain form and a `.`
  form. Under D111's family table, the `.` form takes from the workbench
  "the operand the plain word has on top" (`docs/registers/decisions.md`,
  D111, the row "anything else"). So `stat.quantile.` takes `p` from the
  workbench and the series from the stack.
- **Producers have no `.` form** (D24): `rng.uniform`.
- **Lambda-calling words** (`ode.*`, `sim.gillespie`, `fit.nls`) have the
  plain form only. A `.` form would split a model from its data across
  two stacks for no reader's benefit.

**Effects.** A word is declared `StackEffect::opaque` exactly when it
reads a series that may be a drain, or calls a lambda, as Bund2's own
series words are today. Every other word declares its fixed effect. The
palette audit then runs them, and `PROMOTABLE.txt` is regenerated (F123).

### S4. Scalar special functions (Phase 1)

Each pushes one FLOAT. **An INTEGER operand converts to FLOAT [B15]**,
unlike the reference's unary `math.*` words.

| Word | Effect | Answer | Computed by |
|---|---|---|---|
| `math.log2`, `math.log1p`, `math.expm1` | `x -- y` | log₂ x, ln(1+x), eˣ−1 | `f64` |
| `math.gamma` | `x -- y` | Γ(x) | `statrs` `gamma` |
| `math.lgamma` | `x -- y` | ln \|Γ(x)\|, as R's `lgamma` | `ln_gamma` for x ≥ 0.5; below it Bund2's own reflection, ln π − ln\|sin πx\| − ln\|Γ(1−x)\| |
| `math.digamma` | `x -- y` | ψ(x) | `digamma` |
| `math.beta`, `math.lbeta` | `a b -- y` | B(a,b), ln B(a,b) | `checked_beta`, `checked_ln_beta` |
| `math.betainc` | `x a b -- y` | the regularised incomplete beta I_x(a,b) | `checked_beta_reg` |
| `math.gammainc` | `x a -- y` | the regularised lower incomplete gamma P(a,x) | 0 at x = 0; otherwise `checked_gamma_lr` |
| `math.erf`, `math.erfc` | `x -- y` | | `erf`, `erfc` |
| `math.erfinv` | `x -- y` | erf⁻¹ x, for −1 < x < 1 | `erf_inv` |
| `math.choose`, `math.lchoose` | `n k -- y` | C(n,k), ln C(n,k), both INTEGER operands | `ln_binomial`; `math.choose` computes C(n,k) exactly in u128 while it fits, and `exp(ln_binomial)` above |
| `math.lfactorial` | `n -- y` | ln n! | `ln_factorial` |

**Domains [B12].** Each word checks its operands before the call, so no
`statrs` panic path is reachable, and refuses:

| Case | Disposition |
|---|---|
| a NaN or ±∞ operand | E101 |
| Γ or ln\|Γ\| at 0 or a negative integer | E101 |
| `math.lgamma` at a negative non-integer | **answered**, as R's `lgamma(-0.5)` = 1.2655… |
| `math.erfinv` at x ≤ −1 or x ≥ 1 | E101 |
| `math.choose` with k > n, or either negative | E101 |
| `math.beta`, `math.betainc` with a or b ≤ 0, or x outside [0, 1] | E101 |
| `math.gammainc` with a ≤ 0 or x < 0 | E101 |
| `math.log2`, `math.log1p` at or below their domain's edge | E101 |
| a finite operand whose answer overflows f64: Γ(172), `math.expm1` at 710 | E114 |

`math.gamma` at 171.5 is 9.483…×10³⁰⁷ and is answered; at 172 it is E114.

**Its neighbours differ, and B12 and B15 accept that.** In one program,
`-1.0 math.ln` answers NaN where `0.0 math.log2` refuses, and `4 math.sqrt`
refuses an INTEGER that `4 math.lgamma` takes. The alternative,
**B15 B**, makes the new words match the old in both respects.

*Note 2026-11-14, M2:* three "Computed by" entries changed when built,
each because the named routine broke a rule of the house or B11's
tolerance. B12's dispositions and every answer are unchanged.

- **`math.erf`, `math.erfc`** are `libm`'s (fdlibm's, within an ulp).
  `statrs` 0.18's are off by about 1e-10: `erf(0.5)` answers
  0.52049987776 where the value is 0.52049987781, past B11's 1e-12 for a
  closed form. `libm` 0.2.16 was compiled already, through `humansize`,
  and is now direct, pinned; the lock gains no package. **M4 must check
  the same for S7:** `statrs`'s normal cdf goes through its `erfc`.
- **`math.betainc`, `math.gammainc`** are Bund2's own, the algorithms
  `checked_beta_reg` and `checked_gamma_lr` use (Numerical Recipes'
  continued fraction, and series below x = a + 1), with an iteration cap
  and **E110** at it: 100,000 terms for the beta, 1,000,000 for the
  gamma. `checked_gamma_lr`'s series is an unbounded `loop` that never
  ends once `a + k` stops changing in f64 (a ≳ 10¹⁶), which D39 forbids;
  `checked_beta_reg` stops after 140 terms and answers what it has,
  converged or not. So E110 joins S4's dispositions, for a and b far past
  the fixture's (`1e17 1e17 math.gammainc`).
- **`math.gamma`** answers exp(ln Γ) where `statrs`'s `gamma` overflows
  its intermediate power, from x ≈ 170, below where Γ itself overflows
  (171.62).

Two dispositions the table leaves implicit: ψ's poles are Γ's, E101; and a
NaN the arithmetic makes from operands that passed the checks, which only
an intermediate overflow can cause (`math.lbeta` at 10³⁰⁸ 10³⁰⁸), is
E114, so that no word answers NaN [B12]. A non-number operand is E101
naming the value, and `math.choose`, `math.lchoose` and `math.lfactorial`
refuse a FLOAT, as the table's "INTEGER operands" says.

### S5. Element-wise vectors (Phase 1)

| Word | Effect | Answer |
|---|---|---|
| `vec.add`, `vec.sub`, `vec.mul`, `vec.div` | `a b -- c` | element-wise a∘b; equal lengths, or E102; a zero divisor is E101 |
| `vec.scale` | `a k -- c` | each element times k |
| `vec.dot` | `a b -- x` | Σ aᵢbᵢ |
| `vec.sum`, `vec.prod` | series `-- x` | |
| `vec.cumsum`, `vec.cumprod`, `vec.diff` | series `-- list` | |
| `vec.logsumexp` | series `-- x` | ln Σ e^xᵢ, computed stably |
| `vec.normalize.sum` | series `-- list` | divided by the sum, so a count vector becomes frequencies; a zero sum is E101 |

`a` and `b` follow S3's two-series rule. A sum, product or element that
overflows is E114.

### S6. Descriptive statistics, tests, corrections (Phase 1)

Bund2 computes these itself, in f64, rather than through `rstats`. `rstats`
offers a mean-and-deviation, Kendall's τ and a covariance
(`rstats-2.1.15/src/lib.rs:118,221,355`) but none of the tests, and its
answers to the descriptive words would then come from two crates. A
consequence: `stat.var`'s mean and `stat.mean.arithmetic`'s may differ
in the last bit. Criterion 8 compares with a tolerance, not bit for bit.

**Series words, in four forms:**
- `stat.var`, `stat.sd`, `stat.se`, with n − 1; fewer than two values is
  E101;
- `stat.median`;
- `stat.mode`: the most frequent value, the smallest among ties;
- `stat.skewness` (g₁ = m₃/m₂^{3/2}) and `stat.kurtosis` (excess,
  g₂ = m₄/m₂² − 3), where m_k are the population central moments;
- `stat.zscore` (`series -- list`).

**A series and a scalar:** `stat.quantile` (`series p -- x`,
Hyndman–Fan type 7, R's default).

**Two series:**
- `stat.cov`;
- `stat.cor.pearson`, `stat.cor.spearman` (ties averaged) and
  `stat.cor.kendall` (τ-b).

**Tests.** Each answers a MAP as in S2. Each is two-sided, and the table
states the convention wherever R's default differs or is a choice:

| Word | Operands | Convention |
|---|---|---|
| `stat.test.t1` | series μ₀ | one-sample t |
| `stat.test.t2` | a b | Student, pooled variance; tests mean(a) − mean(b) |
| `stat.test.welch` | a b | Welch–Satterthwaite df |
| `stat.test.paired` | a b | equal lengths; tests a − b |
| `stat.test.chisq` | observed expected | goodness of fit. `expected` holds probabilities when it sums to 1 within 1e-9, and counts otherwise |
| `stat.test.chisq.table` | MATRIX | contingency, df (r−1)(c−1), **no Yates correction** |
| `stat.test.fisher` | MATRIX 2×2 | exact; the two-sided p sums the tables whose probability is at most the observed one's × (1 + 10⁻⁷), as R's `fisher.test` does |
| `stat.test.binom` | k n p | exact, with the same two-sided rule as R's `binom.test` |
| `stat.test.mannwhitney` | a b | the normal approximation with tie and continuity correction; the statistic is W for a |
| `stat.test.ks2` | a b | two-sample Kolmogorov–Smirnov; the asymptotic p, never the exact |

**Corrections and models:**
- `stat.padjust.bonferroni`, `stat.padjust.holm` and `stat.padjust.bh`
  (`p-list -- p-list`, in input order).
- `stat.lm`: OLS by QR (`X y -- MAP`), answering `coef`, `se`, `t`, `p`,
  `r2` and `df`. `X` is a MATRIX of predictors without the intercept,
  which the word adds.

*Note 2026-11-15, M5:* built with these choices.

- **The answer.** A test's MAP holds `statistic`, `p`, `df` (a FLOAT,
  for the t and χ² tests only) and `method`, R's name for the test (R's
  `" Two Sample t-test"` without its leading space).
- **Where R has no statistic.** Fisher's test answers the count in the
  table's first cell, the hypergeometric variable it conditions on; R
  reports the conditional MLE of the odds ratio, which is infinite at the
  support's ends (B12). The binomial test answers k.
- **Bounds (D39).** Fisher's test sums its support, and refuses with E111
  past 10⁸ tables. The binomial test counts the outcomes as probable as k
  or less by a binary search, since the mass is monotone on the far side
  of the mean, so it is O(log n) where R's `sum` is O(n).
- **Refusals.** Each sample of a t test needs two values and a sample of
  a rank test one (E101); a series with no spread is E101 (`a constant
  series`). χ²'s observed counts must be non-negative and its expected
  values positive, a table at least 2×2 with no row or column summing to 0
  (E101). Fisher's cells must be whole numbers (E101) in a 2×2 table
  (E102). `stat.test.binom`'s k and n are non-negative INTEGERs with
  k ≤ n, p in [0, 1] (E101). A correction refuses a p outside [0, 1]
  (E101) and answers an empty LIST for an empty one.
- **`stat.lm`.** A rank-deficient X is E106, where R drops the aliased
  columns; n no greater than the number of coefficients is E101, as are a
  constant y and a perfect fit, whose t would be infinite. `df` is a
  FLOAT.
- **The KS p** is the Kolmogorov distribution's upper tail summed to
  f64's precision, by the alternating series above 1 and the Jacobi theta
  form below it.
- **Forms.** `stat.test.t1` is a series and a scalar, so opaque in its
  plain and `.` forms; the corrections are words of one series, so take
  the four forms; the rest have a fixed effect and a `.` form.

### S7. Distributions (Phase 1) [B2]

**Constructors.** Each takes the family's parameters, deepest first, and
pushes a distribution MAP:

| Group | Words |
|---|---|
| Continuous | `dist.normal` (mean sd), `dist.lognormal` (μ σ), `dist.uniform` (a b), `dist.exponential` (rate), `dist.gamma` (shape rate), `dist.beta` (α β), `dist.chisq` (k), `dist.t` (ν), `dist.f` (d₁ d₂) |
| Discrete | `dist.binomial` (n p), `dist.poisson` (λ), `dist.negbinomial` (r p), `dist.hypergeometric` (N K n), `dist.geometric` (p), `dist.bernoulli` (p) |
| Multivariate | `dist.multinomial` (n p-list), `dist.dirichlet` (α-list), behind `statrs`'s `nalgebra` (§Current behaviour) |

**The parameterisations are R's.** Normal, LogNormal, Uniform and Gamma
(shape, rate) already agree with `statrs`. Two do not, and Bund2 maps
them:

- `dist.geometric` counts **failures** before the first success, support
  0, 1, 2, … as R's `dgeom`. Bund2 calls `statrs` at x + 1.
- `dist.hypergeometric (N K n)` is a population of N holding K successes,
  n drawn. It is R's `dhyper(x, m = K, n = N − K, k = n)`.

Invalid parameters refuse with `statrs`'s own message as the reason
(E104).

**Generic words.** Each takes the distribution on top:

| Word | Effect | Answer |
|---|---|---|
| `dist.pdf` | `x d -- y` | the density, or the mass for a discrete family |
| `dist.lpdf` | `x d -- y` | its natural log; −∞ outside the support is E101 |
| `dist.cdf`, `dist.sf` | `x d -- y` | P(X ≤ x), P(X > x) |
| `dist.quantile` | `p d -- x` | the inverse cdf; p outside [0,1], or NaN, is E101 |
| `dist.mean`, `dist.var` | `d -- y` | E105 where the moment does not exist (t with ν ≤ 2, F) |
| `dist.sample` | `d -- x` | one draw, from the VM's generator (S8) |
| `dist.sample*` | `d n -- list` | n draws; n above 10⁸ is E111 |

**Guards.** Every call into `statrs` is preceded by Bund2's own check
that p ∈ [0, 1], that x is not NaN, and that a density over or under
flows to neither ±∞ nor NaN (E114). No `inverse_cdf` is reached with a p
it panics on.

**Quantiles are polished.** For the continuous families whose
`inverse_cdf` is the bounded bisection-and-Newton of `Gamma`
(Gamma, χ²) or the trait default (Beta, F, t), Bund2 refines `statrs`'s
answer by bracketed Newton until |cdf(x) − p| ≤ 10⁻¹² or 100 steps, E110
at the cap. The discrete families answer the smallest x with cdf(x) ≥ p.

**Sampling.** `dist.sample*` uses `statrs`'s sampler for every family but
the binomial, which is `rand_distr`'s `Binomial` (O(1) per draw), so n
in the millions costs what n = 10 costs.

The multivariate families answer `dist.pdf` (with a LIST for x),
`dist.sample` and `dist.sample*`. They refuse `dist.cdf`, `dist.sf`,
`dist.quantile`, `dist.mean` and `dist.var` with E105.

*Note 2026-11-15, M4:* built with these changes and choices.

- **Keys.** A distribution MAP's keys are R's argument names: `mean` and
  `sd`, `meanlog` and `sdlog`, `min` and `max`, `rate`, `shape` and
  `rate`, `shape1` and `shape2`, `df`, `df1` and `df2`, `size` and
  `prob`, `lambda`, `prob`, `alpha`. The hypergeometric's are this
  section's own `N`, `K` and `n`, since R's `m`, `n` and `k` name
  different things. A MAP built by hand that lacks a key, or holds the
  wrong kind of value under one, is E104 with Bund2's text (`sd is
  missing`); one that names no family is E101 on the operand `d`.
- **Computed by.** The densities, tails, quantiles and moments are Bund2's
  own, over §S4's `lgamma`, its incomplete beta and gamma (each now
  answering the pair P and 1 − P, the one its series or continued fraction
  gives directly computed as such) and `libm`'s `erfc`. `statrs`'s cdfs go
  through the `beta_reg`, `gamma_lr` and `erfc` that §S4's note of
  2026-11-14 replaced. `statrs` checks every family's parameters, and its
  message is E104's text, as above; it draws for the continuous families,
  the Poisson, the geometric and the Bernoulli.
- **Quantiles.** No `statrs` `inverse_cdf` is called, so the polish starts
  from Bund2's own first guess (Wilson–Hilferty for the gamma and χ²,
  normal approximations otherwise) rather than from `statrs`'s answer. It
  takes Newton steps until a step no longer moves x, and refuses with E110
  only if |cdf(x) − p| > 10⁻¹² after 100 steps. A discrete quantile takes R's
  fuzz, the smallest x with cdf(x) ≥ p·(1 − 64ε). A quantile at an
  unbounded end of the support (the normal's at 0 or 1, say) is E114.
- **Sampling.** The hypergeometric draws with `rand_distr`'s
  `Hypergeometric`, since `statrs`'s draws one unit at a time, O(n) per
  draw (D39). The multinomial draws a chain of `rand_distr` binomials,
  O(cells) where `statrs`'s is O(size). The negative binomial is drawn here
  as its gamma–Poisson mixture so that the Poisson's mean can be capped.
  The Dirichlet is drawn here too: `statrs` 0.18's Dirichlet sampler
  returns its gamma draws without dividing by their sum. The geometric is
  `statrs`'s draw less 1, as above.
- **Further refusals.** A negative binomial with `prob` 0 is E104 (`p is 0,
  outside R's (0, 1]`): `statrs` accepts [0, 1], but at 0 there is no
  distribution. A Poisson draw at a mean above 2⁵³ is E111, since its draw
  could not be held exactly. A hypergeometric tail that would sum more
  than 10⁸ terms is E111.
- **Kinds.** The counts (the binomial's and multinomial's `size`, the
  hypergeometric's `N`, `K` and `n`) are non-negative INTEGERs, E101
  otherwise, as `math.choose`'s are. Every answer is a FLOAT, a discrete
  family's draws and quantiles included, since a series is a LIST of FLOATs
  (§S2). A multivariate `dist.sample` answers a LIST, and its
  `dist.sample*` a LIST of LISTs.
- **Moments.** E105 is answered only where the moment does not exist: the
  t's mean for ν ≤ 1 and variance for ν ≤ 2, the F's mean for d₂ ≤ 2 and
  variance for d₂ ≤ 4. The text names the deciding parameter (`t with df
  = 2 has no variance`).
- **The multivariate densities.** A multinomial count whose cells do not
  sum to `size` has mass 0, where R's `dmultinom` stops with an error; a
  Dirichlet point off the simplex has density 0. `statrs` requires
  `min < max` of a uniform, where R accepts `min = max`.

### S8. Seeded randomness (Phase 1) [B4]

- **One generator per VM.** A new method on the `Vm` trait,
  `Vm::rng(&mut self) -> Option<&mut dyn rand_core::RngCore>`, answers
  the VM's own `StdRng`, which `Interp` holds and seeds from the
  operating system on first use. Defaulted to `None`, so a test double
  owes nothing; a `Vm` that answers `None` gets a thread-local `StdRng`
  instead. `rand_core` 0.6 is in the lock under `rand` 0.8, so
  `bund2-api` gains a dependency and the build no crate.

  *Note 2026-11-14, M1:* built as `Vm::rng(&mut self) ->
  Option<&mut rand::rngs::StdRng>`, the concrete type rather than
  `dyn RngCore`. `rng.seed` replaces the generator, and a `dyn RngCore`
  cannot be reseeded through. So `bund2-api` depends on `rand` 0.8.7,
  with only its `std_rng` feature, rather than on `rand_core`; both were
  in the lock, and the lock still gains no package. B4's ruling, a
  generator per VM through a defaulted `Vm::rng`, is unchanged.
- **Why not per thread.** D249 and `Host` give each VM a thread of its
  own (`crates/bund2-runtime/src/vms.rs`, the module doc), but an
  embedder may build several `Interp`s on one thread, as the stdlib's own
  palette audit does (`crates/bund2-stdlib/src/lib.rs`, its repeated
  `Interp::new()`). Per thread, `rng.seed` in one would change another's
  draws.
- **`rng.seed`** (`n --`) reseeds the calling VM's generator with
  `StdRng::seed_from_u64(n as u64)`, so a negative INTEGER is its two's
  complement and every INTEGER is a seed. After it, every word of this
  RFC that draws (`dist.sample`, `dist.sample*`, `rng.uniform`,
  `popgen.wf*`, `sim.gillespie`) repeats exactly, **within one Bund2
  version**. `rand`, `rand_distr` and `statrs` are pinned (D38), so their
  algorithms are that version's; a change of any of them may change every
  seeded stream, and its release notes say so.
- **`rng.uniform`** (`-- x`) answers one draw in [0,1).
- **Existing sources are untouched.** `generator.sample`,
  `math.random.int` and `math.securerandom.int` keep their own unseeded
  sources, and draw nothing from the VM's generator.
- **A child VM** started with `vm.spawn` has its own generator,
  unseeded. A program that wants reproducible replicates passes each
  child a seed and calls `rng.seed` in it, as worked example 3 does.
- **Promotion.** `rng.seed` and the drawing words change only VM state,
  as `math.random.int` does, so none is put in `ACTS_ON_HOST`. The
  palette audit decides whether each is promotable, as it decides for
  every native.

### S9. Linear algebra (Phase 1)

Operands are MATRIXes of numbers (S2). Every word answers a MATRIX,
except where the table says otherwise. Every iterative decomposition is
the `try_` form with `max_niter` = 1000·n and the default epsilon; `None`
is E110.

| Word | Effect | Answer |
|---|---|---|
| `linalg.mul` | `A B -- C` | the matrix product AB; inner dimensions must agree, or E102 |
| `linalg.t` | `A -- Aᵀ` | |
| `linalg.identity` | `n -- I` | n from 1 to 10⁴, or E101 |
| `linalg.det` | `A -- x` | square, or E102 |
| `linalg.inv` | `A -- A⁻¹` | singular is E106 |
| `linalg.solve` | `A b -- x` | x with Ax = b; b and x are LISTs; singular is E106 |
| `linalg.eigen.sym` | `A -- MAP` | `values` (a LIST, ascending) and `vectors` (a MATRIX, column i for value i); E107 unless \|Aᵢⱼ − Aⱼᵢ\| ≤ 10⁻¹²·max\|A\| |
| `linalg.eigenvalues` | `A -- list` | the real eigenvalues; E108 if any imaginary part exceeds 10⁻¹²·‖A‖ |
| `linalg.eigenvalues.complex` | `A -- list` | every eigenvalue, as a LIST of `[ re im ]` LISTs, by descending modulus |
| `linalg.dominant` | `A -- MAP` | `value`, the eigenvalue of largest modulus, and `vector`, its right eigenvector scaled to sum 1. E108 if that eigenvalue is not real, or if another of equal modulus exists (an imprimitive matrix) |
| `linalg.power` | `A k -- Aᵏ` | k ≥ 0, by repeated squaring; a cell that overflows is E114 |
| `linalg.mulv` | `A v -- w` | matrix times a LIST, for projecting a population |

`linalg.dominant` is what a biologist wants from a projection matrix: the
asymptotic growth rate and the stable stage distribution. A Leslie matrix
generically has complex subdominant eigenvalues (worked example 5 has
−0.5066 ± 0.1407i), so `linalg.eigenvalues` refuses it, and that refusal
is right. The vector is found by inverse iteration on A − λI, capped at
100 steps (E110).

*Note 2026-11-15, M6:* built with these choices.

- **Operands.** A MATRIX is read as `stat.test.chisq.table` reads one
  (§S6): a cell that will not convert to FLOAT, or a ragged row, is E103;
  a NaN or ±∞ cell is E101, as are an empty MATRIX and an operand that is
  not a MATRIX. Every shape `nalgebra` would assert is checked first, E102.
- **Singularity.** `linalg.inv` and `linalg.solve` are E106 when the LU
  factorisation meets an exactly zero pivot, as `[ [ 1 2 ] [ 2 4 ] ]`
  does; a nearly singular matrix answers its large inverse, and E114 if a
  cell overflows. `linalg.det` of a singular matrix answers 0.
- **Order and tolerance.** `linalg.eigenvalues` answers in the order
  `linalg.eigenvalues.complex` does, descending modulus, then descending
  real and imaginary part. ‖A‖ is the Frobenius norm.
- **`linalg.dominant`.** Two eigenvalues share the largest modulus when
  their moduli differ by at most 10⁻¹²·max(‖A‖, |λ|); the swap matrix
  `[ [ 0 1 ] [ 1 0 ] ]` is E108 so. The inverse iteration's shift is
  λ + 10⁻¹⁰·max(‖A‖, 1), so A − σI is never exactly singular, and it stops
  when an iterate moves by at most 10⁻¹⁴. A dominant eigenvector summing to
  0 cannot be scaled to sum 1, and is E101.
- **Forms.** `linalg.identity` consumes its n, so it has a `.` form too;
  every word of §S9 does.

### S10. Sequences (Phase 2)

| Word | Effect | Answer |
|---|---|---|
| `bio.valid?` | `s alphabet -- bool` | alphabet `"dna"` (ACGT), `"rna"` (ACGU), `"protein"` (the 20 standard letters and `*`), `"iupac"` (ACGT and RYSWKMBDHVN); upper or lower case |
| `bio.complement`, `bio.revcomp` | `s -- s'` | DNA and IUPAC codes; case kept; any other character is E109 |
| `bio.transcribe`, `bio.backtranscribe` | `s -- s'` | T↔U, case kept |
| `bio.translate` | `s table -- p` | `table` an NCBI table number; `*` for stop; a trailing partial codon dropped; a codon with an ambiguous base is `X` |
| `bio.orfs` | `s min -- list` | MAPs of `start`, `end` (0-based, end exclusive, on the forward strand's coordinates), `strand` (`"+"` or `"-"`) and `protein`, for each ATG…stop of at least `min` codons, stop excluded, under table 1 |
| `bio.gc` | `s -- x` | (G+C)/(A+C+G+T) over unambiguous bases; no unambiguous base is E101 |
| `bio.kmers` | `s k -- dict` | a k-mer count MAP; k from 1 to 32, or E101 |
| `bio.count` | `s -- dict` | symbol counts |
| `bio.hamming` | `a b -- n` | as `string.distance.hamming` (`reference/Bund/src/stdlib/functions/string/distance.rs:143-144`), refusing unequal lengths (E102) |
| `bio.consensus` | `list -- s` | aligned sequences of equal length (E102); ties broken alphabetically; a column of gaps is `-` |
| `bio.pwm` | `list -- matrix` | log₂-odds against a uniform background, rows A C G T, from aligned DNA sites with a pseudocount of 0.5 per symbol |
| `bio.pwm.scan` | `s pwm threshold -- list` | MAPs of `pos` and `score` for each window scoring at least `threshold`, forward strand |

**The genetic code tables** are data in the source, transcribed from
NCBI's `gc.prt`, which is committed as a fixture with its release named in
its header. `bio.translate` answers every table that file lists; any
other number is E101. Criterion 13 compares all 64 codons of each.

### S11. Refusals (all phases)

Codes are this RFC's own, E101 to E115, rendered as RFC-0013's are:
`<WORD>: E1nn <text>`, the word upper-cased
(`crates/bund2-stdlib/src/kb/words.rs`, `refused`), through `Vm::report`
(D36).

| Code | Text (template) |
|---|---|
| E101 | `<operand> is outside <function>'s domain: <value>` |
| E102 | `<what> do not agree: <n> and <m>` |
| E103 | `<operand> is not a MATRIX (or LIST) of numbers: <cell or row>` |
| E104 | `invalid <family> parameters: <statrs's message>` |
| E105 | `<family> has no <operation>` |
| E106 | `the matrix is singular` |
| E107 | `the matrix is not symmetric: [<i>,<j>] and [<j>,<i>] differ by <d>` |
| E108 | `an eigenvalue is complex: <re> ± <im>i`, or `the dominant eigenvalue is not unique: <λ₁>, <λ₂>` |
| E109 | `<char> at <pos> is not in the <alphabet> alphabet` |
| E110 | `<method> did not converge within <cap> iterations` |
| E111 | `<what> exceeds its cap of <cap>` |
| E112 | `the model lambda left <what>, where <expected> was required` |
| E113 | `malformed Newick at <pos>: <why>` |
| E114 | `the answer overflows a FLOAT: <expression>` |
| E115 | `<operand> must be a LIST or METRICS; a drained series is not allowed here` |

E110 is for an iterative method that stops short: a decomposition, an
inverse cdf's polish, inverse iteration, IRLS, Levenberg–Marquardt, an
ODE step size underflowing. E111 is for a cap on work a program's
operands choose: steps, events, cells, generations, draws.

No word answers NaN or ±∞ from finite operands **[B12]**. A NaN or ±∞
operand is E101.

### S12. Alignment (Phase 2) [B9]

`bio.align` (`a b opts -- MAP`).
- **`opts`** is a MAP:
  - `mode`: `"global"` (Needleman–Wunsch), `"local"` (Smith–Waterman) or
    `"semiglobal"` (end gaps free in both sequences);
  - `matrix`: `"blosum62"`, `"dna"` (+5/−4), or a MAP from two-letter
    STRING pairs (`"AC"`) to scores, symmetric;
  - `gap.open` and `gap.extend`, positive numbers, for Gotoh's affine
    gaps. **A gap of length L costs `gap.open` + (L − 1)·`gap.extend`**,
    EMBOSS's convention.
  - **End gaps** are charged as interior gaps in `global` mode, and free
    in `semiglobal`.
- **The answer** holds `score`, `a`, `b` (the gapped strings), `start` and
  `end` for each sequence (0-based, end exclusive), and `identity`
  (identical columns over alignment length).
- **Memory.** Gotoh's three matrices take 12 bytes a cell, so 10⁸ cells
  are about 1.2 GB. A product n·m above 10⁸ is refused with E111 before
  any allocation (D39). Below the cap the allocation is fallible
  (`try_reserve`), and a failure is the same run-time error D171's guard
  reports, not an abort.

`bio.align.score` answers the score alone, in O(min(n, m)) memory, under
the same cap on n·m, which here bounds time rather than memory.

### S13. Population genetics (Phase 3) [B13]

**Inputs.**

- **A genotype** is a STRING of two characters, one per allele copy,
  each character an allele: `"AA"`, `"Ag"`, `"13"`. Order within it is
  ignored. Any number of alleles is allowed; the words that need two say
  so.
- **Genotype data** is a LIST of genotypes.
- **Counts** are INTEGERs, never negative (E101).

| Word | Effect | Answer |
|---|---|---|
| `popgen.freq` | `genotypes -- dict` | each allele's frequency |
| `popgen.hwe` | `nAA nAa naa -- MAP` | `expected` (a LIST of three), `chisq`, `p.chisq` (1 df) and `p.exact`, the exact test of Wigginton, Cutler & Abecasis (2005) |
| `popgen.het` | `genotypes -- MAP` | `observed`, and `expected` = 1 − Σpᵢ² |
| `popgen.fst` | `n1 k1 n2 k2 -- x` | Hudson's F_ST for one biallelic locus, two populations; nᵢ allele copies sampled, kᵢ of them one allele. The estimator is Bhatia et al. (2013) eq. 10: N̂/D̂, with N̂ = (p₁−p₂)² − p₁(1−p₁)/(n₁−1) − p₂(1−p₂)/(n₂−1) and D̂ = p₁(1−p₂) + p₂(1−p₁). D̂ = 0 is E101 |
| `popgen.ld` | `nAB nAb naB nab -- MAP` | `D`, `D'` and `r2` from the four two-locus haplotype counts, in that order |
| `popgen.pi`, `popgen.theta.w`, `popgen.tajima` | `alignment -- x` | over a LIST of aligned STRINGs of equal length (E102); a site with a gap or an ambiguous base is skipped |
| `popgen.wf` | `N p generations -- list` | one Wright–Fisher trajectory of an allele frequency, generations + 1 values from p, by binomial sampling of 2N copies from the VM's generator (S8, `rand_distr`'s `Binomial`) |
| `popgen.wf.selection` | `N p s h generations -- list` | with genic selection s and dominance h |

`popgen.wf*` cap N at 10¹² and `generations` at 10⁷ (E111). With an O(1)
sampler the work is the number of generations, whatever N is.

### S14. Phylogenetics (Phase 3) [B7]

- **A tree [B7]** is a MAP of three LISTs of equal length, one entry a
  node: `names` (STRINGs, verbatim, `""` for an unnamed inner node),
  `parent` (the INTEGER index of each node's parent, −1 for the root) and
  `length` (each node's branch length to its parent, 0.0 at the root).
  It is flat, so no word recurses on its depth (D39), and the names are
  kept exactly as given.
- **Distances:** `phylo.dist` (`alignment model -- matrix`), with model
  `"p"`, `"jc69"` or `"k2p"`. A saturated pair is E101 (log of a
  non-positive), not ∞.
- **Building a tree:** `phylo.upgma` and `phylo.nj` (`matrix names --
  tree`). NJ is rooted at its last join.
- **Newick:** `phylo.newick` (`tree -- s`) and `phylo.newick!` (`s --
  tree`). Parsing is iterative, with no Rust recursion on the input's
  depth (D39), and a malformed string is E113.
- **`phylo.graph`** (`tree -- graph`) builds a GRAPH as RFC-0012 does, for
  a program that wants `graph.*` over a tree. **Its names do not survive.**
  `graph.*` Title-cases node names before anything looks them up
  (`crates/bund2-stdlib/src/graph.rs`, `node_name`), so `root` reads as `Root`,
  `seq1` as `Seq 1`, and `AB` and `Ab` become one node. The word refuses a
  tree two of whose names Title-case alike (E101), so no node is merged
  silently.

### S15. Dynamics (Phase 4) [B6] [B8]

`ode.rk4` (`f y0 t0 t1 h -- MAP`) and `ode.rk45` (`f y0 t0 t1 opts --
MAP`).
- **The model.** `f` is a LAMBDA with the effect `t y -- dy`. It runs as
  RFC-0013's bridge runs (§Current behaviour, "Calling Bund from a
  native"): on a scratch stack named per nesting depth, by
  `Vm::scoped_call_from`, with `t` and `y` the only values on it. One
  value is pulled from the top when it returns; it must be a LIST of
  FLOATs (an INTEGER converts) as long as `y`, else E112. Whatever else
  it left is dropped with the scratch stack. The caller's stack is never
  visible to `f` and is unchanged by it.
- **On a refusal mid-integration** the word's operands are consumed and
  nothing is pushed: the refusal is the result (D36).
- **Nesting.** An `f` that calls `ode.rk45` costs Rust stack per level,
  and `scoped_frame`'s stack floor refuses below it with stack-exhausted
  (F85, RFC-0005 §S8). That floor is the D39 answer to nesting.
- **`opts`:** `rtol` (default 1e-6), `atol` (1e-9), `h0`, `hmax`.
- **The answer** holds `t` (a LIST) and `y` (a LIST of LISTs).
- **Integrators [B8]:** Bund2's own. RK4 is classical. RK45 is
  Dormand–Prince with its embedded error estimate and standard step
  control. Steps are capped at 10⁶; reaching the cap is E111, and a step
  below 10⁻¹²·|t1 − t0| is E110.

`sim.gillespie` (`x0 propensities stoich tmax -- MAP`) runs the direct
method:
- **`propensities`** is a LAMBDA, `x -- list`, run as `f` is.
- **`stoich`** is a MATRIX, one row per reaction.
- **Draws** come from the VM's generator (S8). Events are capped at 10⁶
  (E111).
- **The answer** holds `t` and `x` as `ode.*`'s does.

At Tier 0 each call into a model is interpretation per step. B6 asks
whether a model may also be given as data, a MAP of rates and
stoichiometry with no lambda.

### S16. Fitting (Phase 4) [B8]

`fit.nls` (`f x y p0 -- MAP`) fits by Levenberg–Marquardt, Bund2's own.
- **`f`** is a LAMBDA, `x p -- y`, run as S15's `f` is, answering a LIST
  as long as `x`.
- **The Jacobian** is by forward differences, step
  hⱼ = √ε·max(|pⱼ|, 1), ε the f64 machine epsilon: one call of `f` per
  parameter per iteration.
- **The damping** starts at λ = 10⁻³·max diag(JᵀJ), divided by 10 on an
  accepted step and multiplied by 10 on a rejected one. The linear solve
  is `nalgebra`'s, so a singular normal matrix is damped rather than
  refused. Convergence is a relative change in the residual sum of squares
  below 10⁻¹⁰, or in every parameter below 10⁻⁸; 200 iterations is E110.
- **`p0`** is a LIST of initial parameters.
- **The answer** holds `params`, `se` (from s²(JᵀJ)⁻¹ at the solution),
  `rss`, `df`, `iterations` and `converged` (always `true`, since a
  failure is E110).

`fit.logistic` (`X y -- MAP`) fits logistic regression by IRLS, capped at
100 iterations (E110). It answers `coef`, `se`, `z`, `p`, `deviance` and
`aic`.

### S17. The tiers and the audits

- **Effects** are declared as S3 says. Tier 1 crosses a native only if
  `PROMOTABLE.txt` lists it, and the palette audit, not this RFC, decides
  which are listed (F123). A declared-opaque word is never run by the
  audit, so the lambda-calling words are never promoted.
- **The palette** runs every fixed-effect native over
  `7 2.5 true "zz_nofile" …` (`crates/bund2-stdlib/src/lib.rs`, `KINDS`).
  For these words that is harmless: the palette's INTEGER is 7, so no
  operand reaches a cap, and none acts on the host.
- **Generated files move.** `PROMOTABLE.txt`, `bund2 effects`' output and
  `docs/arity.md` (`cargo xtask arity`) gain the new words, regenerated
  by their own commands in each milestone.
- **No golden is added.** The oracle has none of these words (D21). The
  pins are the unit tests of §Acceptance criteria.

### S18. Logic programming (all phases) [B16]

RFC-0013's engine can use this family today, with no new goal, through
the bridges it already has.

**What a rule can already do:**

- **Numbers are terms.** An INTEGER is an integer cell and a FLOAT a
  float cell (`crates/bund2-stdlib/src/kb/term.rs`, `Cell::Int` and `Cell::Float`). `1` and `1.0`
  do not unify, and floats compare by key equality (D226 L2).
- **A rule body computes through a bridge.** `[ is X λ ]`, `[ test λ ]`
  and `[ in X λ ]` are reserved goals (`crates/bund2-stdlib/src/kb/store.rs`, `RESERVED`;
  `solve.rs`, `Solver::bridge`). The bridge substitutes the clause's
  bound variables into λ and runs it as §Current behaviour describes. λ
  may call any word, so every word of this RFC is callable from a rule
  as soon as it exists.
- **Answers come back** as a LIST of MAPs from `kb.query`, one per
  answer, in depth-first clause order (RFC-0013).

So, for example, with a fact base of p-values and genotypes:

    [ [ significant S ] ← [ pvalue S P ] [ test { 0.05 P < } ] ]
    [ [ carrier X ] ← [ genotype X G ] [ test { G "Aa" == } ] ]
    [ [ hwe.p L P ] ← [ counts L AA Aa AAa ] [ is P { AA Aa AAa popgen.hwe "p.exact" get } ] ]

`{ 0.05 P < }` means P < 0.05: `<` takes its operands as `-` does
(`bund_examples/logic_demo/demo.bund`: "`A 18 <=` asks 18 <= A").

**A genotype in a fact is a quoted STRING**, `[ genotype ann "Aa" ]`. A
bare `Aa` or `AA` begins with a capital, so the engine reads it as a
variable, and the `carrier` rule then fails with an instantiation error.
The first two rules, and the third with `AA Aa AAa + +` in place of
`popgen.hwe`, were run on `target/debug/bund2` over facts
`[ pvalue rs1 0.001 ] [ pvalue rs2 0.2 ] [ pvalue rs3 0.04 ]`,
`[ genotype ann "Aa" ] [ genotype bob "AA" ]` and
`[ counts l1 298 489 213 ]`, and answered `[ rs1 rs3 ]`, `[ ann ]` and
`{ T=1000 }`.

**What it cannot do, and this RFC does not add:**

- **No native float comparison or arithmetic goal.** Each check is a
  bridge, so a stack switch and an interpretation. CLP(FD) refuses a
  FLOAT (E64, `crates/bund2-stdlib/src/kb/fd/post.rs`, `Bad`), and there is
  no CLP(R).
- **No aggregation but `findall/3`** (`solve.rs`, the `findall` arm).
  A count, sum or maximum is `findall` and then a bridge or a series word
  over the LIST, `stat.padjust.bh` included.
- **A distribution MAP in a fact is opaque.** It unifies by identity
  (`crates/bund2-stdlib/src/kb/unify.rs`, the `Cell::Opaque` arm of `unify_pairs`, D226 L13), so a rule
  can carry one to a bridge but cannot match on its family or
  parameters.
- **No uncertain inference.** Certainty factors are deferred (D226 L11).
  A rule that weighs evidence by probability would reopen that ruling.
- **A FLOAT first argument has no index key** (D243), so a fact base
  keyed by FLOAT scans.

**B16** asks whether that is enough. **A**: this RFC adds nothing to
`kb.*`, and M13's examples include one rule base over Phase 1 and 3
words. **B**: add float comparison and arithmetic goals, and `sum`,
`count` and `max` aggregates, as an amendment to RFC-0013 under its own
ruling. **C**: words that emit facts, as `smysl.kb` turns a corpus into
facts (`crates/bund2-stdlib/src/smysl/facts.rs`), for example a
`popgen.facts` writing `genotype` and `freq` facts. A is recommended; B
is a separate RFC if a measurement shows bridges dominate a real rule
base's time.

## Worked examples

Every line below that uses only existing words was run on
`target/debug/bund2` at 948bd78, and the idioms it relies on are noted.
The new words are not yet built; criterion 20 runs each example whole.

### 1. Hardy–Weinberg, and an association test with FDR

    // Counts at one SNP: 298 AA, 489 Aa, 213 aa
    298 489 213 popgen.hwe "p.exact" get println     // 0.65566…
    // SNP p-values, corrected
    :ps [ 0.001 0.02 0.04 0.3 0.5 ] var
    :ps var? stat.padjust.bh println

`dict … "k" get` reads a MAP's key (run).

### 2. A primer's reverse complement, and its protein

    "ATGGCCATTGTAATGGGCCGCTGA" bio.revcomp println   // TCAGCGGCCCATTACAATGGCCAT
    "ATGGCCATTGTAATGGGCCGCTGA" 1 bio.translate println   // MAIVMGR*

### 3. Wright–Fisher replicates on threads, reproducibly

    :rep { rng.seed 100 0.5 200 popgen.wf } var
    :hs list var
    [ 1 2 3 4 5 6 7 8 ] {
      list + :rep var? swap vm.spawn      // [ seed ] as the child's stack
      :hs var? + :hs swap var
    } loop
    :hs var? { vm.join println } loop

The program is the LAMBDA itself, since a child cannot call the parent's
words (`crates/bund2-runtime/src/vms.rs`, `spawn`'s doc). `seed list +`
makes `[ seed ]`, because `+` of a LIST and a non-LIST appends. The loop,
the spawn and the joins were run with `{ 100 + }` in place of the body,
and answered `[ 101 ]`, `[ 102 ]`, `[ 103 ]`.

### 4. An SIR epidemic

    :sir {                                   // t [ S I R ] -- [ dS dI dR ]
      unfold :R swap var :I swap var :S swap var drop
      :S var? :I var? * 0.5 * -1.0 * list +                      // −βSI
      :S var? :I var? * 0.5 * :I var? 0.1 * swap - swap +        // βSI − γI
      :I var? 0.1 * swap +                                       // γI
    } var
    :sir var? [ 0.99 0.01 0.0 ] 0.0 100.0 dict ode.rk45 "y" get println

β = 0.5 and γ = 0.1. `0.0 [ 0.99 0.01 0.0 ] :sir var? execute` was run
alone and answered `[ -0.00495 0.00395 0.001 ]`, the derivatives at t = 0.
`a b -` is b − a, so `swap -` subtracts the top from the value beneath.

### 5. A Leslie matrix's growth rate and stable stage distribution

    [ [ 0.0 1.5 0.8 ] [ 0.5 0.0 0.0 ] [ 0.0 0.7 0.0 ] ] matrix
    linalg.dominant dup "value" get println        // 1.0131034…
    "vector" get println                           // [ 0.54510 0.26902 0.18588 ]

The matrix literal was run and is a MATRIX. Its characteristic equation is
λ³ = f₂s₁λ + f₃s₁s₂ = 0.75λ + 0.28, whose real root is
λ = 1.013103401237989; the stable distribution is proportional to
(1, s₁/λ, s₁s₂/λ²).

## Implementation plan

| M | Delivers | Phase |
|---|---|---|
| M1 | the module tree under `crates/bund2-stdlib/src/`; S11's codes; `statrs`'s `nalgebra` feature on; `nalgebra` and `rand_distr` direct, pinned `=` to the lockfile's versions (D38); `Vm::rng`. `Cargo.lock` gains no package | 1 |
| M2 | S4 | 1 |
| M3 | S5, S6's descriptive words | 1 |
| M4 | S7, S8 | 1 |
| M5 | S6's tests, corrections, `stat.lm` | 1 |
| M6 | S9 | 1 |
| M7 | S10 | 2 |
| M8 | S12 | 2 |
| M9 | S13 | 3 |
| M10 | S14 | 3 |
| M11 | S15 | 4 |
| M12 | S16 | 4 |
| M13 | `bund_examples/biomath/`: the worked examples, and S18's rule base | all |

**Progress.** M1 landed 2026-11-14: `crates/bund2-stdlib/src/biomath/`
with `refuse` (S11's E101–E115, each text tested) and `rng`
(`with_rng`, `reseed`, over `Vm::rng`); `statrs`'s `nalgebra` feature;
`nalgebra` =0.33.3 and `rand_distr` =0.4.3 direct; no word registered
yet, so `bund2 words`, `PROMOTABLE.txt` and `docs/arity.md` are unchanged.
M2 landed 2026-11-14: `biomath::special`, S4's sixteen words, each with
its `.` form, and their criteria 2, 3 (E101, E110, E114), 5, 6 and 7, with
the changes S4's M2 note records. `PROMOTABLE.txt` lists fourteen words
and their forms, and names `math.betainc` and `math.erfinv`, which the
palette's operands do not reach, as not reached. `docs/arity.md` is
generated from the reference's registry, so the family never enters it;
it is unchanged.
M3 landed 2026-11-14: `biomath::vector` (S5's thirteen `vec.` words) and
`biomath::describe` (S6's thirteen descriptive words), over
`biomath::operand`'s readers, with criteria 2, 3 (E101, E102, E103,
E114, E115), 5, 8 and 9's descriptive half. Each word has the forms S3
gives it: four for a word of one series, plain and `.` for the rest, 82
names in all. `PROMOTABLE.txt` gains the nine two-series words and their
`.` forms; the rest read a series that may be a drain, so are opaque and
never audited (S17). Their expected values are `fixtures/describe.py`'s, exact
rationals and mpmath, beside hand-worked values in the tests' comments;
S6's tests (M5) are where R's are needed. As M3 builds them:

- a one-series word's reader is `get_data`, so a cell that will not
  convert keeps the house text, `<WORD> returned: …`; a two-series word
  reads its LISTs itself, and such a cell is E103;
- a NaN or ±∞ element is E101 naming its position, 1-based as `at`
  counts, and a too-short or constant series is E101 naming it;
- `stat.cor.kendall` compares every pair, n(n − 1)/2 comparisons, which a
  program's series bounds (D39).

M4 landed 2026-11-15: `biomath::dist` (§S7's seventeen constructors and
nine generic words) and `biomath::rng`'s `rng.seed` and `rng.uniform`
(§S8), with criteria 2, 3 (E101, E102, E104, E105, E110, E111, E114), 5, 6,
10 and 11, with the changes §S7's M4 note records. Each word has its `.`
form but `rng.uniform`, a producer: 55 names. Every fixture point agrees
with mpmath to within 2e-13. Criterion 11's `vm.spawn` bullet is a test of
the binary (`crates/bund2-cli/tests/biomath_rng.rs`), since the runtime
registers `vm.spawn`. `PROMOTABLE.txt` gains thirteen constructors, their
`.` forms, `rng.seed`, its form and `rng.uniform`. The other four
constructors (the Bernoulli, binomial, geometric and negative binomial)
and the generic words, which take a distribution MAP the palette does not
hold, are listed as not reached. `dist.hypergeometric`
and its form join the unvaried-operand audit's list, their first operand
being a count.

M5 landed 2026-11-15: `biomath::infer`, §S6's ten tests, three
corrections and `stat.lm`, with criteria 2, 3 (E101, E102, E103, E106,
E115), 5, 6 and 9, and the choices §S6's M5 note records: 34 names with
their forms. `PROMOTABLE.txt` gains the five tests of two series the
palette reaches, and their forms; `stat.test.binom` and
its form join the unvaried-operand audit's list, their first operand being
a count.

M6 landed 2026-11-15: `biomath::linalg`, §S9's twelve words and their `.`
forms, with criteria 2, 3 (E101, E102, E103, E106, E107, E108, E114), 5, 6
and 12, and the choices §S9's M6 note records. Worked example 5 answers
λ = 1.013103401237989 and the stable distribution to within 10⁻¹², against
`fixtures/linalg.py`, which also holds the 3×3 fixtures' exact answers. `PROMOTABLE.txt` gains
`linalg.identity` and its form; the other eleven take a MATRIX of numbers,
which the palette does not hold, so are listed as not reached.

Each milestone ends with conform and coverage unchanged (D72), clippy
clean, `cargo deny check` clean, and `PROMOTABLE.txt`, `bund2 effects`
and `docs/arity.md` regenerated.

## Which points are rulings

Accepting this RFC would mean:
- D250's points as ruled;
- S1–S18 as specified;
- the four phases built in order, each gated by its criteria;
- the family as Bund2's own, outside D14's in-scope set (D72).

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| `get_data`'s dispatch, drain and four forms | **preserved**, and reused by S5 and S6 as is |
| The reference's top-first order for `math.nroot`, `math.perimeter`, `math.interpolation` and the outlier words | **preserved**; S3's deepest-first order applies to new words only |
| `+ - * /` on LIST and MATRIX (`rust_dynamic/src/math.rs:36-65,296-319`), LIST `+` non-LIST appending among them | **preserved**; S5 and S9 are new words |
| D57's silent shape arm | **preserved** for the operators; `linalg.*` refuses instead (E102, E103), as new words |
| `math.*` and `stat.*` reference words, INTEGER refusal and NaN answers included | **preserved**; S4 and S6 only add names |
| `generator`, `math.random.int`, `math.securerandom.int` unseeded | **preserved**; S8's generator is separate |
| The process-wide generator map | **preserved** |
| `graph.*`'s Title-casing | **preserved**; `phylo.graph` refuses a tree it would merge |
| `kb.*`'s terms, bridges and goals | **preserved**; S18 adds nothing to the engine |
| The `Vm` trait | **extended** by a defaulted `Vm::rng`; no existing method changes |
| `PROMOTABLE.txt`, `bund2 effects`, `docs/arity.md` | **grow** by the new words, regenerated; no existing line changes |
| Every golden | **unchanged**; conform 143/159, CEILING 143/159 before and after |

No deviation is proposed.

## Alternatives considered

- **Redefine `*` on MATRIX as the product, and `+` on LISTs as vector
  addition.** Rejected: both are preserved reference behaviours with
  goldens (§Current behaviour).
- **`argmin` for fitting.** Its Gauss–Newton needs an `argmin-math`
  backend the lock does not compile, at nalgebra 0.32 beside the 0.33 used
  everywhere else. Implementing its traits for Bund2's own types would
  make `argmin-math` a direct dependency for one word. Levenberg–Marquardt
  is about 150 lines over `nalgebra`'s solve **[B8]**.
- **`rust-bio` for Phase 2.** It offers alignment, FASTA/FASTQ and pattern
  matching. Its dependency set has not been measured, and D28 requires
  that before anything joins the default build **[B9]**.
- **`ndarray` for vectors and matrices.** It is linked through `algos`,
  but `nalgebra` has the decompositions, and a second matrix library adds
  conversions and nothing else.
- **Per-distribution words** (`dist.normal.cdf` and so on): about 90
  names against 25 **[B2]**.
- **A packed FLOAT vector value [B3]:** faster at a million elements, and
  a change to `bund2-value`, the wire format and Tier 1. It is deferred
  until a measurement shows LISTs are the bottleneck. The reference's
  `Embedding` is f32 and not a candidate.
- **A generator per thread [B4 B]:** no `Vm` change, but two `Interp`s on
  one thread share it (S8).
- **A process-wide seed [B4 C]:** two VMs on two threads would interleave
  draws, so a seeded run would not repeat.
- **A tree as a GRAPH [B7 A]:** `graph.*` would walk it, but every name
  is Title-cased on reading, so taxon names change and can merge (S14).
- **A tree as nested LISTs or MAPs [B7 B]:** a deep tree would be deep
  values, and printing, comparing and dropping them recurse on depth.
- **`statrs`'s binomial sampler:** O(n) per draw, so a Wright–Fisher
  generation at N = 10⁹ would be 2·10⁹ uniforms in one native call.
- **Maximum-likelihood and Bayesian phylogenetics:** out of scope. They
  are what dedicated tools are for.
- **`bio.mw` and the PAM250 matrix,** in the first draft, are dropped:
  neither had a named reference table to test against.

## Acceptance criteria

Each criterion is a named test in `crates/bund2-stdlib`, unless placed
otherwise. Numerical expectations are stated values, to the tolerance
stated **[B11]**: relative error 1e-12 for closed forms, 1e-8 for series
and iterative answers, and 1e-6 for integrators, unless a criterion says
otherwise. "R" means R 4.4.1; each fixture computed with R is committed
with the R script that made it and the script's output.

*Note 2026-11-14, M2:* R is not installed where M2 was built, so
criterion 7's fixture for `math.betainc`, `math.gammainc` and
`1000 500 math.choose` is mpmath 1.3.0's at 50 digits, committed as
`crates/bund2-stdlib/src/biomath/fixtures/special.py` with its output
`special.txt`, which the test reads. mpmath is an arbitrary-precision
reference, so the fixture is stricter than R's double-precision answers,
not looser. Later milestones that name R use it where it is available.
M5's criterion 9 fixture is computed from the definitions R's help
pages give (`fixtures/infer.py`, mpmath and exact rationals), on R's
`sleep` data and the examples of `?chisq.test`, `?fisher.test` and
`?binom.test`. R's printed answers for those examples, at R's printed
precision, are checked beside it; they agree to every printed digit. The
criterion names `cor.test`, but §S6 has no correlation test word: the
estimate `cor.test` reports is `stat.cor.*`'s, which M3 checked.
M4's criterion 10 fixture is mpmath's for the same reason:
`fixtures/dist.py`, with its output `dist.txt`, which the test runs
program by program.

### Every phase

1. conform 143/159 with CEILING 143/159, and IMPLEMENTED and COVERAGE
   unchanged, before and after each milestone.
2. `bund2 words` lists every S-table word and the forms S3 gives it, and
   nothing else is added; the test holds the list.
3. Every code in S11 has a test that provokes it and compares the
   reported text with S11's template filled in.
4. Clippy clean, `cargo deny check` clean, and `Cargo.lock` gains no
   package.
5. **No abort.** For each S4 word, each S7 family and each S9 word, a
   test feeds NaN, ±∞, p = −0.1, p = 1.1 and the domain edges S4 lists,
   and requires a refusal or the stated answer. The test running to
   completion is the check that no panic was reached.
6. A program defining a word of a family's name still calls its own.

### Phase 1

7. **S4:**
   - Γ(0.5)² = π; Γ(171.5) = 9.48337…×10³⁰⁷ is answered; Γ(172) is E114;
   - ln Γ(171.5) = 709.143163030928; `-0.5 math.lgamma` = 1.26551212348464;
     `0 math.lgamma` is E101;
   - `10 3 math.choose` = 120; `1000 500 math.choose` against
     `exp(lchoose(1000, 500))` from R;
   - `math.erf` at 0.5 = 0.5204998778 against Abramowitz & Stegun
     table 7.1, to the table's 10 digits;
   - `math.betainc` and `math.gammainc` against R's `pbeta` and `pgamma`
     at the points in the committed fixture; `0 2 math.gammainc` = 0.
8. **S5:** each word on the fixture `[ 1 2 3 4 ]`, `[ 0.5 -1 2 8 ]`; E102
   on unequal lengths; E115 on a drain where two series are required;
   `vec.logsumexp` over `[ 1000 1000 ]` is 1000 + ln 2.
9. **S6:**
   - each descriptive word on `[ 2 4 4 4 5 5 7 9 ]` against values worked
     by hand in the test's comment (variance 32/7, median 4.5, mode 4);
     skewness and kurtosis by S6's formulas;
   - each test's statistic and p against R's `t.test`, `chisq.test`
     (`correct = FALSE`), `fisher.test`, `binom.test`, `wilcox.test`
     (`exact = FALSE, correct = TRUE`), `ks.test` (`exact = FALSE`),
     `cor.test`, `p.adjust` and `lm`, on fixtures committed with R's
     output;
   - all four forms of each single-series word, and both forms of
     `stat.quantile`.
10. **S7:**
    - each family's `pdf`, `cdf` and `quantile` against R's `d`/`p`/`q`
      at the three points per family in the committed fixture, the
      geometric and hypergeometric through S7's mappings;
    - `dist.mean` and `dist.var` against the closed forms;
    - `dist.cdf` of `dist.quantile` returns p within 1e-10, for p in
      {0.001, 0.5, 0.999} and every continuous family;
    - the multivariate `dist.pdf` against R's `dmultinom` and a closed
      form for the Dirichlet; E104 and E105.
11. **S8:**
    - the same seed gives the same `dist.sample*` LIST, twice in one VM;
    - two `vm.spawn` children seeded alike answer the same LIST;
    - a seeded `dist.sample*` stream is unchanged when `math.random.int`
      and `generator.sample` calls are interleaved with it;
    - two `Interp`s on one thread, one seeded, do not share draws;
    - `rand_distr`'s binomial at n = 10⁹ answers in under a millisecond.
12. **S9:**
    - `linalg.mul`, `t`, `identity`, `det`, `inv`, `solve`, `power` and
      `mulv` on 3×3 fixtures with their exact answers;
    - E106 on `[ [ 1 2 ] [ 2 4 ] ]`;
    - `linalg.eigen.sym` on a 3×3 symmetric fixture, checked by
      A·v = λ·v to 1e-10;
    - `linalg.eigenvalues` on worked example 5 is E108;
    - `linalg.eigenvalues.complex` there answers 1.013103401237989 and
      −0.50655 ± 0.14066i;
    - `linalg.dominant` there answers λ = 1.013103401237989 and the vector
      [0.545096414296, 0.269023089662, 0.185880496042], the closed forms
      of example 5.

### Phase 2

13. **S10:**
    - `bio.revcomp` of every IUPAC code, and of example 2's primer;
    - `bio.translate` of example 2's primer under table 1 is `MAIVMGR*`;
    - `bio.valid?`, `complement`, `transcribe`, `backtranscribe`, `gc`,
      `kmers`, `count`, `hamming` and `consensus` on fixtures with
      hand-worked answers in the test;
    - `bio.orfs` on a synthetic 120-base fixture holding two ORFs on each
      strand, at positions written in the test;
    - `bio.pwm` and `bio.pwm.scan` on five 6-base sites, against values
      worked by hand.
14. **The tables:** every table in the committed `gc.prt`, all 64 codons,
    against the file.
15. **S12:**
    - global, local and semiglobal scores and alignments against EMBOSS
      `needle -endweight Y -endopen <open> -endextend <extend>` and
      `water` output under BLOSUM62 and the `dna` matrix, on committed
      fixtures; semiglobal against hand-worked cases;
    - `bio.align.score` equals `bio.align`'s `score` on every fixture;
    - the cap refuses a 10⁴ × 10⁴ + 1 alignment before it allocates.

### Phase 3

16. **S13:**
    - `298 489 213 popgen.hwe` answers `chisq` 0.221489587647, `p.chisq`
      0.637907263782 and `p.exact` 0.655663488562; 57 heterozygotes with
      14 and 50 homozygotes answer `p.exact` 0.842279756571. The values
      are the published algorithm (Wigginton et al. 2005, its appendix)
      run in a committed Python script;
    - `popgen.freq`, `het`, `fst` and `ld` on fixtures with hand-worked
      answers in the test;
    - π, θ_W and Tajima's D on a committed six-sequence fixture, against
      values worked by hand;
    - `popgen.wf`'s final frequency, over 1000 seeded replicates, has a
      mean within 3 SE of p; `popgen.wf.selection` with s = 0 matches
      `popgen.wf` under the same seed.
17. **S14:**
    - JC69 and K2P distances against their closed forms;
    - on the five-taxon matrix a–e,
      `[0 5 9 9 8; 5 0 10 10 9; 9 10 0 8 7; 9 10 8 0 3; 8 9 7 3 0]`,
      `phylo.nj` gives branch lengths a 2, b 3, c 4, d 2, e 1, and inner
      branches 3 and 2; `phylo.upgma` gives
      `((a:2.5,b:2.5):2.08333,(c:3.75,(d:1.5,e:1.5):2.25):0.83333);`;
    - Newick round trip, names verbatim;
    - a 10⁵-deep nested Newick parses without overflow;
    - `phylo.graph` on a tree with names `seq1` and `Seq1` is E101, and
      one `graph.*` word runs over a tree with distinct mixed-case names.

### Phase 4

18. **S15:**
    - `ode.rk4` and `ode.rk45` on logistic growth against its closed
      form;
    - Lotka–Volterra's first integral conserved to 1e-6 over 10 periods
      under RK45;
    - a model leaving an INTEGER is E112, and the caller's stack is
      unchanged by a model that pushes extra values;
    - E111 at the cap.
19. **`sim.gillespie`:** a birth–death process's mean at t against its
    analytic mean, over 1000 seeded replicates, within 3 SE.
20. **S16:**
    - `fit.nls` on R's `Puromycin` data, treated state, with
      `rate = Vm·conc/(K + conc)` from p0 = (200, 0.05), gives
      Vm = 212.68, K = 0.06412, se 6.947 and 0.008281, rss 1195.45,
      to 1e-4 relative, as R's `nls` does;
    - `fit.logistic` against R's `glm(family = binomial)` on a committed
      fixture.

### Phases together

21. `bund_examples/biomath/` runs each worked example and S18's rule base;
    its README states the outputs, and a CLI test compares them.

## Rulings — D250 (RESOLVED 2026-11-14, as recommended)

| | Question | Options | Recommended |
|---|---|---|---|
| **B1** | Which build has the family | **A** the default build. **B** a feature, off by default (D28). **C** Phase 1 default, Phases 2–4 a feature | **A**: no crate is added; Phases 2 and 4 are Bund2's own code |
| **B2** | The distributions' shape | **A** a distribution MAP and generic words (about 25 names). **B** a word per family and function (about 90) | **A** |
| **B3** | Numeric vectors | **A** LISTs of FLOATs, read by `get_data`. **B** a packed FLOAT vector type | **A**, and B only after a measurement |
| **B4** | Seeded randomness | **A** a generator per VM, through a defaulted `Vm::rng`, and `rng.seed`. **B** a generator per thread. **C** one process-wide seed | **A** |
| **B5** | Names | **A** S1's prefixes. **B** everything under `bio.` | **A** |
| **B6** | Models for dynamics | **A** a LAMBDA only. **B** a LAMBDA, or a MAP of rates and stoichiometry | **A**, with B a later amendment if Tier 0's cost is measured too high |
| **B7** | A phylogenetic tree | **A** a GRAPH (RFC-0012). **B** nested LISTs. **C** a MAP of flat LISTs, with `phylo.graph` converting | **C**: names verbatim and no recursion on depth |
| **B8** | Integrators and the least-squares solver | **A** Bund2's own: RK4, Dormand–Prince, Levenberg–Marquardt. **B** a pinned crate | **A**: no linked crate offers them buildable as linked |
| **B9** | Alignment | **A** Bund2's own. **B** `rust-bio` behind a feature | **A** |
| **B10** | Forms | **A** S3: four for one-series words, plain and `.` for other consumers, none for producers and lambda-callers. **B** plain only | **A** |
| **B11** | Test tolerance and reference values | **A** stated values, R 4.4.1 fixtures and hand-worked answers, at §Acceptance's tolerances. **B** Bund2's own earlier output | **A** |
| **B12** | Domain errors | **A** refuse (E101, E114), never answer NaN or ±∞ from finite operands. **B** IEEE: answer NaN and ±∞, as the reference's `math.*` words do | **A** |
| **B13** | Genotype input, and F_ST's estimator | **A** two-character genotype STRINGs and counts; Hudson's F_ST (Bhatia et al. 2013). **B** Weir & Cockerham's F_ST as well, by option | **A**, with B as a later addition |
| **B14** | What acceptance covers | **A** every phase, built in order, each gated by its criteria (as L14). **B** Phase 1 only | **A** |
| **B15** | INTEGER operands to the new `math.*` words | **A** convert to FLOAT. **B** refuse, as the reference's unary `math.*` words do | **A**: `10 3 math.choose` needs it, and refusing would be a trap kept for consistency's sake |
| **B16** | Logic programming | **A** S18: bridges only, plus an example rule base. **B** float goals and aggregates in `kb.*`, by an RFC-0013 amendment. **C** fact-emitting words | **A**; B only after a measurement |

## Open questions

- None is ungrounded. Every claim about current behaviour cites the
  reference at the SHA above or a pinned crate. Every claim about Bund2
  cites a symbol in live code.
- Rulings B1–B16 are D250's, RESOLVED 2026-11-14 as recommended.
- Phase 4's Tier 0 cost per step is unmeasured. M11 records the time for
  `ode.rk45` on worked example 4 in `docs/measurements/`, and that number
  informs B6's later amendment, if any. M13 records the time S18's rule
  base spends in bridges, which informs B16 B.

## Revision 2

Each finding of `docs/rfc/reviews/RFC-0015-review-2026-11-14.md`, and
what changed:

| Finding | Change |
|---|---|
| B1 `argmin` not buildable | `argmin` dropped; S16 is Bund2's own Levenberg–Marquardt with a stated Jacobian; "What is linked" says why; B1, B8 and D250 re-derived; ERRATA entry for research 06 |
| B2 panics and unbounded loops | "What must be avoided"; S4 names the `checked_*` forms and its guards; S7's guards and polish; S9's `try_` forms with a cap; E110 widened; criterion 5 |
| B3 Leslie refused by E108 | `linalg.dominant` and `linalg.eigenvalues.complex`; example 5 and criterion 12 with the closed-form values |
| S1 overflow and domains | E114; S4's domain table; `lgamma` as R's; `gammainc` at 0; criterion 7 pins Γ(172), not Γ(171.5), which is finite |
| S2 neighbours | §Current behaviour, "Scalar functions"; S4's last paragraph; B15 |
| S3 multi-operand words | S3's operand and form rules; E115 |
| S4 "opaque" | S3's "Effects" and S17 use the house meaning; `rng.seed` and promotion in S8; producers exempt (D24) |
| S5 Title-casing | B7 now recommends a flat tree; `phylo.graph` refuses a merge; criterion 17 |
| S6 examples | every example rewritten and its existing-word parts run |
| S7 O(n) binomial | `rand_distr`'s `Binomial`; caps on N and generations; S8's "within one version" names the crates |
| S8 criteria | R 4.4.1; R's defaults stated in S6; geometric and hypergeometric mappings; skewness and kurtosis defined; named values throughout; criterion 3 by template; criterion 11's converse form |
| S9 words without criteria | criteria 8–20 name every word; `bio.mw` and PAM250 dropped |
| S10 per thread | `Vm::rng`; B4 restated |
| S11 model lambda | S15's "The model", "On a refusal", "Nesting" |
| S12 inputs | S12's gap convention and end gaps; S13's inputs, `fst` and `ld` operands; S10's tables by fixture |
| N1–N14 | taken: the reader's callers, "six means and a spread", `rand.rs:54`, `math.rs:315-319`, the uncited claims, E115, S13/S15, D250's dependencies, `rstats`, `Embedding`, `fit.logistic`'s answer, the thresholds, alignment memory beside D171 |

The bridge mechanism was misstated in the first draft (`eval_lambda`);
§Current behaviour corrects it, and §S18 is new, at the owner's request
to review integration with logic programming.
