# RFC-0020: Multivariate analysis and density estimation — the SVD, PCA, MDS, distances, hierarchical clustering, k-means, Hotelling's T², MANOVA, LDA, kernel density, the ECDF and histograms

- Status: **Draft**
- Drafted 2026-11-25 on the owner's "/rfc-draft 0020", in 0.58.0, after
  `docs/research/07-statistical-research.md` (§3.7, §4, Phase F). D260's
  C8 rules one RFC per phase for F, G and H (its note of 2026-11-21
  changed C8 for D and E only), so this RFC is Phase F alone.
- Its owner questions V1–V14 are D277, **RESOLVED 2026-11-25** as
  recommended ("V1–V14: recommendations"); its **[author]** choices are
  D278, **RESOLVED 2026-11-25** as recommended (all fourteen, "rule on
  D278's 14 open author choices: recomendation"). The draft was written against each recommended option
  and marks each dependent passage **[V*n*]**.
- Amended a third time 2026-11-25 after the third review
  (`docs/rfc/reviews/RFC-0020-review-2026-11-25-3.md`, SF1–SF5,
  N1–N6). SF1 follows the owner's ruling that `nstart` with a MATRIX of
  centres is ignored, as R ignores it (D278's note of the same date).
- Amended again 2026-11-25 after the second review
  (`docs/rfc/reviews/RFC-0020-review-2026-11-25-2.md`, B1, SF1–SF4,
  N1–N3), and to state the owner's rule that R is not a dependency
  (§Acceptance criteria).
- Amended 2026-11-25 after the first review
  (`docs/rfc/reviews/RFC-0020-review-2026-11-25.md`, B1–B4, SF1–SF12,
  N1–N9). The amendment restates D278's choices 1 and 6, adds choices
  10 to 14, and records the result in D278's note of the same date.
- Depends on: RFC-0015 (the `biomath` family: operand readers, §S3's
  forms, §S8's generator, §S9's linear algebra, §S11's refusals, §S14's
  distance matrices), RFC-0016 (§S1's options-MAP rules, §S7's sampling
  words), RFC-0017 (the frame, `frame.matrix`, the factor's sort order),
  RFC-0019 (`optim.root`'s port of R's `zeroin`; how an F test answers
  its degrees of freedom, D272), RFC-0004 (effects)
- Decisions consumed: D260 (C4 D, C5 C, C6 C, C8 A), D250 (B1, B3, B4,
  B5, B7, B8, B10, B11, B12), D251 (A8), D252, D253 (P7), D266 (N6, as
  precedent), D272, D275 (3A, as precedent), D21, D36, D37, D38, D39,
  D72; **D277** (V1–V14, as recommended) and **D278** (the fourteen
  [author] choices, as recommended).
- Reference SHA: `reference/rust_multistackvm`
  4605832678d4792c2b8e0564b7757d5aee1539a4; `reference/Bund`
  21b40b0213a76f3f39bd6de0d2bdfdf0a53b0dca; `reference/rust_dynamic`
  ceb27c96fa10c4edb09106f3f904f22f94d63119. Bund2's own code is cited by
  file and symbol, at `development/0.58.0` 1c94b6b. R is cited as
  reasoning, never as a reference this RFC must match (D72): R 4.4.2's
  source, read through the `wch/r-source` mirror at the tag `R-4-4-2`
  (`https://raw.githubusercontent.com/wch/r-source/tags/R-4-4-2/`), as
  RFC-0018 and RFC-0019 read it, each file compared with the tag
  `R-4-6-2` (§What R does says where they differ). Base R's
  `src/library/base/R/scale.R` was read the same way; it is identical at
  both tags. Also read: MASS 7.3-66's
  `R/lda.R` through the `cran/MASS` mirror at the tag `7.3-66`. Paths
  under `src/` below are R's. The owner's machine has R 4.6.2 and MASS
  7.3-66 (D266's note of 2026-11-22), and the behaviour quoted from R was
  run there.
- Supersedes: nothing. Grounds and refines research 07's Phase F (§3.7,
  §4).

## Summary

RFC-0015 to RFC-0019 gave Bund2 the tests, intervals, frames, models,
resampling and power of univariate statistics. Research 07's Phase F is
what a study needs when it has many measurements per subject, or wants
to see the shape of one: reduce the variables, find groups, test a mean
vector, classify, and estimate a density. This RFC adds, as words of
Bund2's own:

- **Decomposition and ordination.** `linalg.svd`; principal components
  (`mva.pca`, R's `prcomp`); classical multidimensional scaling
  (`mva.mds`, R's `cmdscale`).
- **Clustering.** Distance matrices (`cluster.dist`, R's `dist`);
  hierarchical clustering by eight linkages (`cluster.hclust`, R's
  `hclust`) and its cuts (`cluster.cut`, `cluster.cut.h`, R's
  `cutree`); k-means by Hartigan–Wong, Lloyd or MacQueen
  (`cluster.kmeans`, R's `kmeans`), seeded from the VM's generator.
- **Multivariate tests and classification.** Hotelling's one- and
  two-sample T² (`stat.test.hotelling1`, `stat.test.hotelling2`);
  one-way MANOVA with Pillai's, Wilks', Hotelling–Lawley's and Roy's
  statistics (`stat.test.manova`, R's `summary.manova`); linear
  discriminant analysis and its prediction (`mva.lda`,
  `mva.lda.predict`, MASS's `lda`).
- **Density estimation.** Kernel density over seven kernels
  (`stat.density`, R's `density`), its bandwidth rules (`stat.bw`:
  Silverman's, Scott's, Sheather–Jones'), the ECDF (`stat.ecdf`,
  `stat.ecdf.at`) and histograms with R's breaks (`stat.hist`).

**No existing word changes** in operands, effect, answer, refusals or
cost. No data representation changes, no crate is added, and no word
calls a lambda. Twenty names are added, each with its forms (§S1),
under the prefixes V1 rules.

## Motivation

- **No decomposition of a data matrix.** `linalg.` has the symmetric
  eigendecomposition and no SVD (`crates/bund2-stdlib/src/biomath/linalg.rs`,
  `register`; research 07 §2's table, `docs/research/07-statistical-research.md:48`).
  PCA by hand is possible through `linalg.eigen.sym` of a covariance
  matrix, but no word computes a covariance *matrix*: `stat.cov` is the
  covariance of two series (`crates/bund2-stdlib/src/biomath/describe.rs`,
  `stat_cov`). And `linalg.eigen.sym` answers its eigenvalues ascending
  (`linalg_eigen_sym` sorts by `total_cmp`), the reverse of the order a
  component analysis reads.
- **No clustering of observations.** The one clustering word,
  `clusters.detect`, is the reference's DBSCAN over the dynamic-time-warping
  distance between two series
  (`reference/Bund/src/stdlib/functions/forecast/clustering.rs:57-58`,
  `reference/Bund/src/stdlib/functions/math/clusters.rs:9-28`). It
  answers a label per *series*, takes no data matrix and no k, and its
  `min_cluster_size` is fixed at 2 (`clusters.rs:16`). `phylo.upgma` is
  average linkage, but answers a phylogenetic tree with branch lengths
  at half the joining distance, not R's dendrogram, and has no other
  linkage (`crates/bund2-stdlib/src/biomath/phylo.rs`, `phylo_upgma`).
- **No test of a mean vector.** Every `stat.test.` word tests one
  variable (`bund2 words`, run at 1c94b6b: `stat.test.t1`, `t2`,
  `welch`, `anova1` and the rest). Several correlated outcomes tested one
  at a time inflate the error rate and ignore their correlation.
- **No classifier.** `fit.logistic` and `fit.glm` model two classes; nothing
  separates k groups.
- **No estimate of a distribution's shape.** `stat.quantile` answers one
  quantile (`describe.rs`, the module doc), and nothing answers a density
  estimate, an ECDF or a histogram's counts. A program that plots or
  summarises a sample computes these by hand.

## Current behaviour

### What the reference does

- **The reference has none of these words.** Its registry, read from
  every literal `register_inline("…")` and `register_alias("…")` in
  `reference/rust_multistackvm/src` and `reference/Bund/src`, has no
  name beginning `mva`, `cluster.` (singular), `pca`, `mds`, `lda`,
  `dens`, `kde`, `hist` or `ecdf`, and no `stat.` name but the
  `stat.count` and `stat.mean.*` families
  (`reference/Bund/src/stdlib/functions/statistics/count.rs:52-55`,
  `reference/Bund/src/stdlib/functions/statistics/statistics.rs:185-215`).
- **Two names lie close to this RFC's prefixes:**
  - `clusters.detect` and `clusters.detect.`
    (`reference/Bund/src/stdlib/functions/forecast/clustering.rs:57-58`).
    The word pulls `epsilon`, then two series by `get_data`
    (`clustering.rs:13-31`), and answers DBSCAN's label per series, or
    `"NOISE"` (`reference/Bund/src/stdlib/functions/math/clusters.rs:18-27`).
    Bund2 registers it (`crates/bund2-stdlib/src/forecast.rs`).
  - `mv`, an alias of `fs.mv`
    (`reference/Bund/src/stdlib/functions/create_aliases.rs:15`), which
    Bund2 registers too (`crates/bund2-stdlib/src/host.rs`, `register_alias("mv", "fs.mv")`).
  V1 chooses prefixes with both in view.

### What Bund2's own family does

- **MATRIX operands** are read by `infer.rs`'s `table`: anything but a
  MATRIX is E101, a cell that does not convert to FLOAT or a ragged row
  E103, a non-finite cell E101 naming its position
  (`crates/bund2-stdlib/src/biomath/infer.rs`, `table`). `linalg.rs`'s
  `matrix` adds E101 for an empty MATRIX (`linalg.rs`, `matrix`).
- **`linalg.eigen.sym`** refuses an asymmetry above 10⁻¹² of the largest
  |cell| (E107), runs `nalgebra`'s `SymmetricEigen::try_new` with
  `max_niter` 1000·n (E110 at the cap), and answers `values` ascending
  and `vectors` as columns, with whatever signs `nalgebra` gives
  (`linalg.rs`, `linalg_eigen_sym`, `niter`). No word normalises a sign.
  **`linalg.eigenvalues`** refuses a complex eigenvalue (E108);
  `linalg.eigenvalues.complex` answers every one (`linalg.rs`,
  `linalg_eigenvalues`, `linalg_eigenvalues_complex`).
- **Factors.** `frame.factor` makes a STRING column a factor, its levels
  the distinct STRINGs sorted by bytes. It refuses a numeric or logical
  column (`frame/reshape.rs`, `factorable`, `make_factor`).
  `frame.factor.levels` sets the levels in a given order
  (`frame_factor_levels`). A factor's levels are always STRINGs.
- **`nalgebra` 0.33.3 has the SVD.** `SVD::try_new(matrix, compute_u,
  compute_v, eps, max_niter)` sorts the singular values descending and
  answers `None` past `max_niter`
  (`nalgebra-0.33.3/src/linalg/svd.rs:688-715`). No Bund2 word calls it.
- **Distance matrices** reach `phylo.upgma` and `phylo.nj` through
  `taxa`, which reads two operands, the MATRIX and a LIST of names
  (`v[0]`, `v[1]`). It requires a square MATRIX (E102, `Refusal::Disagree`),
  at least two taxa, a zero diagonal and non-negative cells (E101), and
  exact symmetry (E107). n is at most `TAXA_CAP` = 2000 (E111)
  (`phylo.rs`, `taxa`, `TAXA_CAP`; `refuse.rs`, `Refusal::code`). `phylo.upgma` joins the least pair,
  "the first in active order on a tie" (`phylo.rs`, `least`), and
  updates distances by size-weighted averages, which is UPGMA.
- **One-series statistics** are registered in four forms by
  `four_forms!`, a series and a scalar in two by `scalar_forms!` (plain
  `opaque(2)`, `.` `opaque(1)`), as `stat.quantile` is
  (`crates/bund2-stdlib/src/biomath/operand.rs`, `four_forms`,
  `scalar_forms`; `describe.rs`, `register`).
- **`stat.quantile`** is Hyndman–Fan type 7, R's default, and `stat.var`
  and `stat.sd` divide by n − 1 (`describe.rs`, the module doc, `mean`,
  `variance`, `quantile`). `mean` and `variance` are crate functions over
  `&[f64]`. `quantile` is private and answers a `BundValue`, so §S9 needs
  its arithmetic as an f64 function (§Preservation analysis).
- **Grouped tests take a LIST of LISTs.** `stat.test.anova1` reads its
  groups as one operand (`crates/bund2-stdlib/src/biomath/infer/further/anova.rs`,
  `stat_test_anova1`), and an F test answers `df1` and `df2` (D272).
- **Frames** are MAPs of `columns`, `data` and `levels` (D252), and
  `frame.matrix` answers named numeric columns as a MATRIX of FLOATs,
  refusing or dropping rows with a NONE by its `na` option
  (`crates/bund2-stdlib/src/biomath/frame/summary.rs`, `frame_matrix`).
  A factor's levels sort by bytes (D253 P7).
- **The generator.** `with_rng` runs a closure over the VM's `StdRng`;
  `rng.choice` draws k of n without replacement by the first k steps of
  Fisher–Yates, `gen_range(0..=i)` for i from n − 1 down
  (`crates/bund2-stdlib/src/biomath/rng.rs`, `with_rng`;
  `crates/bund2-stdlib/src/biomath/sample.rs`, the module doc).
- **`optim.root`** is R's `zeroin`, ported (RFC-0019 §S6;
  `crates/bund2-stdlib/src/biomath/optim.rs`).
- **Refusals** are RFC-0015 §S11's E101 to E115 (`refuse.rs`, `Refusal`).
- **The unvaried-operand audit** lists 89 fixed natives
  (`crates/bund2-stdlib/src/lib.rs`, `UNVARIED`).
- **`rustfft` 6.4.1 is in the lock**, through `welch-sde` and
  `augurs-seasons` (`cargo tree -i rustfft`), not as a dependency of
  `bund2-stdlib`'s own.

### What R does, as reasoning

None of this is a reference Bund2 must match (D72). It is where the
algorithms and defaults below come from. Line numbers are R 4.4.2's.
Every cited file was compared with R 4.6.2:
- Where a file differs, the difference is stated below.
- `stem.c` and `calc.R` differ outside the lines cited:
  - `stem.c` changes `stem()`, and `C_bincount` gains an `if (nb1)` guard
    before its `memset`;
  - `calc.R` changes the run detection at `:134-139`.
- Every other file is byte-identical.

**Decomposition and ordination.**
- **`prcomp`** (`src/library/stats/R/prcomp.R:21-58`) calls `scale(x,
  center, scale.)` (`:27`) and stops if any scale is 0 (`:30-31`). Base
  R's `scale.default` divides each column by √(Σv²/max(1, n − 1)) of
  the column *as it then is* (`src/library/base/R/scale.R:38-46`):
  - centred, that is the sd on n − 1;
  - uncentred, it is the root mean square on n − 1.

  So with `center = FALSE`, only an all-zero column stops R, not any
  constant column. It takes `svd(x, nu = 0, nv = k)` and
  divides the singular values by √max(1, n − 1) (`:40-42`). `tol` keeps
  the components whose sdev is above `tol` × the first, and `rank.`
  caps k; both cut the rotation only, never `sdev` (`:34-50`).
  `summary.prcomp` takes the proportions over every sdev and stores them
  rounded to five places (`:107-120`). At 4.6.2 only the helper
  `.check_vars_numeric` differs (`sapply` became `vapply`).
- **Signs are LAPACK's.** `svd` calls `dgesdd` (`src/modules/lapack/Lapack.c:139`,
  `:146`), symmetric `eigen` calls `dsyevr` and reverses its ascending
  order (`src/library/base/R/eigen.R:59-61`), and neither normalises a
  sign. `prcomp`, `cmdscale` and `lda` take the signs as they come. Only
  `princomp` makes each loading's first element non-negative
  (`src/library/stats/R/princomp.R:104-109`). On `USArrests`, `eigen` of
  the correlation matrix and `prcomp`'s SVD give opposite signs on three
  of four components (run on R 4.6.2). A port that does not run the same
  LAPACK routines cannot reproduce R's signs (V3). At 4.6.2, `eigen.R`'s
  `isSymmetric.matrix` gains a `trans` argument for complex matrices, and
  `Lapack.c` changes initialisation and `La_chol`'s pivoted path; neither
  touches the SVD or the symmetric eigendecomposition.
- **`cmdscale`** (`src/library/stats/R/cmdscale.R:19-88`) squares the
  distances, double-centres them row then column
  (`src/library/stats/src/dblcen.c:25-44`), and takes `eigen(-x/2,
  symmetric = TRUE)` (`:53`, `:70`). It keeps an eigenvalue if it is
  strictly above 0, with no tolerance, warning "only %d of the first %d
  eigenvalues are > 0" and dropping the rest (`:73-79`). Points are the
  eigenvectors scaled by √λ (`:80`), and GOF is Σλₖ over Σ|λ| and over
  Σmax(λ, 0) (`:82-86`).
- **`add = TRUE`** is Cailliez's constant, `max(Re(e))` over *every*
  eigenvalue of a 2n × 2n non-symmetric matrix (`:55-69`, `:62-63`).
  Those eigenvalues are often complex: on a 5-point Manhattan matrix
  they include −3.28 ± 0.52i (§Fixture inputs gives the points; run on
  R 4.6.2).
- **Refusals.** It refuses k outside 1 … n − 1 (`:50-51`) and n above
  46340 (`:48-49`). It does not check a matrix operand for symmetry or a
  zero diagonal (`:28-34`).

**Clustering.**
- **`dist`** (`src/library/stats/R/dist.R:19-43`;
  `src/library/stats/src/distance.c:42-192`) has the Euclidean, maximum,
  Manhattan, Canberra, binary and Minkowski distances, and answers the
  lower triangle by columns (`distance.c:263-267`).
  - **Canberra** skips a coordinate where both values are zero
    (`:122`). It then rescales the sum by nc/count, the same rescaling
    it applies to missing values (`:135-136`). So the rescaling applies
    with no missing value at all: `dist(rbind(c(0,1), c(0,3)),
    "canberra")` is 1, not 0.5. Two all-zero rows answer NA (`:135`).
  - **Binary** counts the coordinates where exactly one value is
    non-zero, over those where at least one is (`:140-169`). Two
    all-zero rows answer **0**, not NA (`:166-167`); NA needs every
    coordinate missing (run on R 4.6.2).
  - **Minkowski** needs a finite p > 0 (`:223-225`).
  - **Overflow.** Every distance is a plain double sum, so finite cells
    can give +∞: the Euclidean distance between ±10²⁰⁰ overflows.
  At 4.6.2 `distance.c` adds a threads define, moving later lines by one,
  and `dist.R` changes `as.matrix.dist` only.
- **`hclust`** (`src/library/stats/R/hclust.R:47-113`;
  `src/library/stats/src/hclust.f:34-215`) is Murtagh's
  nearest-neighbour-list algorithm with eight methods: `ward.D`,
  `single`, `complete`, `average`, `mcquitty`, `median`, `centroid` and
  `ward.D2` (`hclust.R:50-52`). Each update is a Lance–Williams formula
  (`hclust.f:133-164`). `ward.D2` squares the distances first and
  reports the square root of each height (`:65-69`, `:111-112`);
  `median` and `centroid` use the distances as given. The pair merged is
  the smallest active index I whose current nearest-neighbour distance is
  least, strictly (`:93-100`). Nearest neighbours are kept to the right
  only, the smallest index on a tie (`:76-87`, `:190-207`), and an update
  that only equals a neighbour's distance leaves it unchanged (`:172-180`).
  So R's tie-breaking is not "the lexicographically least pair". With
  single linkage on 0, 1, 2, 3, 10, 11, R merges (−1, −2), then (−3, 1),
  then (−4, 2) (run on R 4.6.2).
- **The sentinel.** The Fortran's "infinity" is `INF = 1.D+300`
  (`hclust.f:47`), and `JM` starts at 0 (`:53`). A nearest neighbour is
  taken only on a strict `DMIN .GT. DISS(IND)` (`:80`). So a row whose
  distances are all ≥ 10³⁰⁰ keeps `NN(I) = 0`, an index outside 1 … n.
- **`hcass2`** (`hclust.f:258-333`) writes R's `merge`:
  - a singleton as −i, a cluster as the step that formed it;
  - the singleton first;
  - two singletons with the smaller index first, (−1, −2);
  - two clusters ascending.

  It also writes `order`, the leaves read left to right from the last
  merge.
  `hclust` refuses n above 65536 (`hclust.R:64-69`). It neither sorts
  nor warns when `centroid` or `median` gives a height lower than the
  one before (an inversion).
- **`cutree`** (`src/library/stats/R/cutree.R:19-50`;
  `src/library/stats/src/hclust-utils.c:25-113`) numbers the clusters by
  their first member in observation order (`hclust-utils.c:87-95`). A cut
  at height h performs every merge at a height ≤ h (`cutree.R:32`), and
  refuses heights that are not sorted (`:27-28`). A cut into k groups
  refuses k outside 1 … n (`:37-39`) and performs the first n − k merges,
  without checking that the heights are sorted (`:35-40`). At
  4.6.2 `hclust-utils.c` changes `Rboolean` to `bool` only.
- **`kmeans`** (`src/library/stats/R/kmeans.R:19-145`) defaults to
  Hartigan–Wong, AS 136 (`src/library/stats/src/kmns.f:11-447`), with
  Lloyd and MacQueen in C (`src/library/stats/src/kmeans.c:22-145`), and
  `iter.max = 10`, `nstart = 1` (`kmeans.R:19-22`).
  - **Starts.** A number k draws k rows by `sample.int` (`:95-106`).
    With `nstart = 1` it draws again, from the unique rows, only if the
    first draw holds a duplicate. Each further start draws from the
    unique rows. The best total within-cluster sum of squares wins, the
    earliest on a tie (`:126-134`).
  - **Labels and ties.** Cluster j is the one that started at row j of
    the centres; nothing relabels (`:135-139`). Every assignment breaks a
    tie toward the lower centre index (`kmeans.c:36-46`; `kmns.f:41-72`).
    Hartigan–Wong moves a point only for a strict gain (`kmns.f:285-287`,
    `:403-409`).
  - **Odd cases.** k = 1 is forced to MacQueen (`kmeans.R:118`).
    Hartigan–Wong refuses k = nrow(x) (`kmns.f:34-35`, `kmeans.R:43-45`).
  - **Warnings.** An empty cluster stops Hartigan–Wong (`kmeans.R:37-50`)
    but only warns under Lloyd and MacQueen, whose empty centre is then
    NaN (`kmeans.c:60`). Non-convergence and an exceeded quick-transfer
    cap (50·m steps, `kmeans.R:28-29`) warn (`:37-50`, `:70-75`).
  - **Iteration counts.** Lloyd counts a first pass that only assigns,
    so `iter.max = 1` always reports non-convergence (`kmeans.c:31-63`).
  - At 4.6.2 `kmeans.c` changes `Rboolean` to `bool`, and `kmns.f`
    rewrites one loop's labels with the same jumps.

**Multivariate tests and classification.**
- **Base R has no Hotelling's T².** No function in the `stats` namespace
  has "hotel" in its name; "Hotelling" appears only as `summary.manova`'s
  test name (searched on R 4.6.2). For two groups, `summary.manova`'s
  Hotelling–Lawley F equals the two-sample T² transform exactly: on
  versicolor against virginica in `iris`, T² = 355.472145199 and F =
  86.147586209 on 4 and 95, by both routes (run on R 4.6.2).
- **`summary.manova`** (`src/library/stats/R/manova.R:38-128`) defaults
  to Pillai's test and drops the intercept (`:38-47`, `:78-84`).
  - **Scaling.** It scales E by sc = √diag(E). For each variable j it
    sets sc_j to 1 when sc_j < 10⁻⁶·√(E_jj + Σ H_jj), the sum over the
    tested terms (`:98-102`).
  - **The rank test.** It takes `qr(D E D, tol = 1e-7)`, LINPACK's
    `dqrdc2`, whose tolerance compares column *norms*, and refuses a rank
    below the number of responses: "residuals have rank %d < %d"
    (`:103-106`). The eigenvalues of E⁻¹H are `eigen(…, symmetric
  = FALSE)`'s real parts (`:109-110`). The four statistics and their F
  approximations are `src/library/stats/R/mlm.R:363-406`: Pillai's V =
  Σλ/(1 + λ), Wilks' Λ = Π1/(1 + λ) with Rao's F, Hotelling–Lawley's T =
  Σλ, and Roy's θ = max λ, whose F is an upper bound.
  - **Degrees of freedom need not be positive.** Hotelling–Lawley's
    df2 is 2(s·n + 1), with n = ½(N − g − p − 1) (`mlm.R:392-396`). At
    N − g = p this is 2 − s, so 0 for s = 2.
  - **What R answers then.** `summary.manova` answers F 0 on 4 and 0,
    with `Pr(>F)` NA, since its `ok` test fails (`manova.R:117-119`).
    §Fixture inputs gives the case, run on R 4.6.2.
  - **Pillai's F** divides by s − V (`mlm.R:372`).
- **MASS's `lda`** (`R/lda.R:73-211`, MASS 7.3-66) defaults to the
  sample proportions as the prior and the "moment" method (`:73-76`,
  `:104`, `:107`). A prior must be non-negative with `round(sum(prior),
  5) == 1` (`:90`). It answers `prior`, `counts`, `means`, `scaling`,
  `lev`, `svd` and `N` (`:207-209`). It refuses a variable whose within-group sd is below
  `tol` = 10⁻⁴ (`:113-121`). The within-group SVD whitens W (divisor n −
  g) and warns "variables are collinear" below full rank (`:153-161`).
  The between-group SVD weights each group by √(n·priorₖ/(g − 1)) about
  the prior-weighted mean, and keeps the singular values above 10⁻⁴ of
  the first (`:192-198`). `svd`² are the eigenvalues of W⁻¹B (checked on
  `iris`).
- **`predict.lda`.**
  - Its "plug-in" posterior is exp(−(½‖dₖ‖² − log priorₖ − x·dₖ)),
    normalised (`:255-286`).
  - It centres both x and the means on the prior-weighted mean, under
    the prediction's prior (`:255-258`), so the scores move when `prior`
    is given.
  - A prior of 0 enters as log 0 = −∞ (`:264`).
  - `dimen` is clamped silently, `min(dimen, length(object$svd))`
    (`:260`).
  - The class
  is `max.col`'s, which breaks an exact tie at random (its documented
  default) [UNGROUNDED: `max.col`'s source was not read; Q68].

**Density estimation.**
- **`density`** (`src/library/stats/R/density.R:21-184`) is not an exact
  kernel sum. It rounds the grid up to 512 points or the next power of
  two (`:99-101`), extends it four bandwidths past `from` and `to`
  (`ext = 4`, `:146-147`), bins the data linearly
  (`src/library/stats/src/massdist.c:34-68`), and convolves with the
  kernel by FFT (`density.R:151-177`). It then clamps at 0 (`:177`) and
  interpolates linearly onto the output grid (`:178-180`). `from` and
  `to` default to the extremes ∓ `cut`·bw, `cut = 3` (`:140-143`).
  `bw` is the kernel's standard deviation for all seven kernels
  (`:153-175`). A bandwidth *rule* needs two points (`:121-123`); a
  numeric `bw` does not.
- **Against the exact sum of kernels,** on the 10-point sample x₁₀
  (§Acceptance criteria), R's answer differs by:
  - about 1.25·10⁻⁵ absolute;
  - up to 1.6·10⁻³ relative where y > 10⁻³;
  - up to 2.3·10⁻³ relative over the whole 512-point output. R has no exact method; `old.coords = TRUE` restores
  the grid used before R 4.4.0, which is worse.
- **Bandwidths** (`src/library/stats/R/bandwidths.R`).
  - **`bw.nrd0`**, Silverman's rule, is 0.9·min(sd, IQR/1.34)·n^(−1/5).
    When that minimum is 0 it falls back to sd, then |x₁|, the first
    value in the order given, then 1 (`:23-30`).
  - **`bw.nrd`**, Scott's, is 1.06·min(sd, IQR/1.34)·n^(−1/5), with no
    fallback (`:32-38`).
  - **`bw.SJ`**, Sheather and Jones', uses IQR/1.349, not 1.34
    (`:73`). It counts pairs into `nb = 1000` bins, truncating toward
    zero, by an O(n²) loop for n ≤ 500 and by binned counts above
    (`:42-53`; `src/library/stats/src/bandwidths.c:118-171`). It solves
    "ste" by `uniroot` over [0.1·hmax, hmax], widening the interval by
    1.2 up to 99 times (`:83-101`), with `tol = 0.1*lower`. That default
    is evaluated lazily, at the `uniroot` call, after any widening
    (`:56`, `:101`; R's default arguments are promises: `f <-
    function(a, t = 0.1*a) { a <- a/2; t }; f(1)` prints 0.05 on R
    4.6.2). "dpi" is a closed form (`:80-81`).
  - The tolerance is loose. On x₁₀, bw.SJ answers
    1.6638128970574513 at R's default and 1.6620992508857797 at
    `tol = 1e-13` (run on R 4.6.2), so the root differs from R's answer
    at the third digit (V10).
  - **At 4.6.2 `bandwidths.c` changes behaviour** (R 4.5.3's NEWS): the
    unbinned counter stops on constant data, "data are constant in
    bandwidth calculation", and bins by (x − min)/dd when x/dd would
    overflow an int. The binned path is unchanged.
- **`ecdf`** (`src/library/stats/R/ecdf.R:23-36`) is #{xᵢ ≤ t}/n,
  right-continuous, over the sorted distinct values. It drops NA by
  `sort` (`:25`).
- **Histogram breaks.** `nclass.Sturges` is ⌈log₂ n + 1⌉, `nclass.scott`
  ⌈range/(3.5·sd·n^(−1/3))⌉, and `nclass.FD` is from 2·IQR of the data
  rounded to five significant digits, with a chain of fallbacks
  (`src/library/grDevices/R/calc.R:94-114`). `hist` turns a class count
  into breaks by `pretty(range(x), n, min.n = 1)`
  (`src/library/graphics/R/hist.R:82`). Before that it applies four
  rules:
  - it drops non-finite x silently (`:36`);
  - it sorts a vector of breaks (`:48`);
  - with a rule or a count, it forces `include.lowest` to true, with a
    warning (`:50-53`);
  - it clamps a count above 10⁶ to 10⁶, with a warning (`:77-81`).

  It counts into right-closed bins with the lowest closed
  (`include.lowest`) (`:117-128`; `src/library/graphics/src/stem.c:165-194`).
  First it nudges every break by 10⁻⁷ of a width:
  - the median width when there are more than five breaks;
  - the range of x when there are at most three;
  - otherwise the least positive width (`:118-120`).

  It answers `breaks`, `counts`, `density`, `mids` and `equidist`
  (`:133-138`). `pretty`'s C
  (`src/appl/pretty.c:72-257`) is about 70 lines of live arithmetic with
  four bounded loops. Its own comment says it relies on an exact
  `pow(10, ⌊log₁₀ cell⌋)` (`:153-155`). At 4.6.2 `pretty.c` rewrites the
  `eps.correct` branch, which `hist` does not use (it passes 0), and
  `hist.R` adds a plotting argument; the arithmetic `hist` uses is
  unchanged.

## Design

### S1. Shape

- **Bund2's own (D72)**, in the default build, adding no crate (D251
  A8, as a default), under the prefixes V1 rules. **V1's recommended
  option A** is used throughout this draft:
  - `linalg.` for the SVD, beside the other decompositions;
  - `mva.` for ordination and discrimination, a new kind of operand (a
    data matrix read as observations by variables): `mva.pca`,
    `mva.mds`, `mva.lda`, `mva.lda.predict`. Not `mv.`, which reads as
    the reference's `mv` (`fs.mv`);
  - `cluster.` for distances and clustering, whose dendrogram is a new
    kind of thing, and the operand of `cluster.cut` and `cluster.cut.h`
    (D260 C5's test is the operand): `cluster.dist`, `cluster.hclust`,
    `cluster.cut`, `cluster.cut.h`, `cluster.kmeans`. No name begins
    `clusters.`, the reference's family;
  - `stat.test.` for the three tests, as RFC-0016 and RFC-0019 extended
    it, and `stat.` for the density words, statistics of one series
    (D260 C5: "`stat.` or `fit.` for another test or fit").
- **Additive, strictly.** No existing word changes; §Preservation
  analysis lists what could.
- **Refusals** are RFC-0015 §S11's codes; no new code and no new text.
  B12 holds: no word answers NaN or ±∞ from finite operands. Where R
  stops, Bund2 refuses with the matching code. Where R only warns about
  a failure, Bund2 refuses too (V6), as RFC-0019 did: an iterative
  method at its cap is E110 (N6), and a degenerate answer (an empty
  cluster) is E101. A work cap reached is E111 before the work (D39).
  Every E101 is `Refusal::Domain`, `<operand> is outside <word>'s domain:
  <value>`, as RFC-0019 §S1 states.
- **No lambdas.** No word of this RFC calls one, so every word is fixed
  or opaque only by its operands, and the re-entry audits are untouched
  (§S12).
- **Forms** follow RFC-0015 §S3 (B10). `stat.ecdf` is a one-series word,
  in four forms. `stat.ecdf.at` and `stat.bw` take a series and a
  scalar, in `scalar_forms!`'s two, as `stat.quantile`. Every other word
  is a consumer, plain and `.`.
- **Options MAPs** follow RFC-0016 §S1. Keys are STRINGs. An unknown key,
  a key the word does not take, or a value of the wrong type or range is
  E101 naming the key. NONE as a value is a value of the wrong type. Each
  module reads its own keys, as RFC-0019's did.
- **Data matrices** are MATRIXes of numbers read by `table`, rows the
  observations and columns the variables, with its refusals (E101,
  E103). `table` accepts an empty MATRIX and rows of no cells. Every word
  here refuses those itself: no rows, or no columns, is E101 naming the
  operand. A frame reaches them through `frame.matrix`, whose `na` option
  decides missing values (RFC-0017); no word here takes a frame or a
  NONE cell.
- **Series** (`stat.density`, `stat.bw`, `stat.ecdf`, `stat.ecdf.at`,
  `stat.hist`) are read as the family's one-series words read them. A
  non-finite value is E101 (B12) **[author]**. R is not uniform here:
  - `hist` drops non-finite values (`hist.R:36`);
  - `density` stops on NA, "'x' contains missing values", unless
    `na.rm = TRUE` (`density.R:57-69`), and drops ±∞, keeping the mass
    nx/N, the sub-density (`:72-82`);
  - `ecdf` drops NA only and keeps ±∞ as a knot (`ecdf.R:25`);
    `knots(ecdf(c(1, 2, Inf)))` is 1, 2, Inf;
  - `bw.SJ` stops on one, "non-finite x[4] in bandwidth calculation"
    (`bandwidths.c:125-126`), and `bw.nrd0` stops on a missing value in
    its test;
  (each run on R 4.6.2).
- **Overflow.** A computation that overflows a FLOAT from finite operands
  is E114 (`Refusal::Overflow`), naming the expression.
- **Grouped data [V2]** is a MATRIX and a LIST of labels, one per row,
  as R's `lda(x, grouping)`. A label is an INTEGER or a STRING, all of
  one type, else E101 naming `groups`. A length other than the MATRIX's
  rows is E102.
- **Group order** **[author]**:
  - **STRING labels** sort by bytes, as `frame.factor` sorts a STRING
    column's levels (`make_factor`; D253 P7).
  - **INTEGER labels** sort ascending. This rule is this RFC's own:
    `frame.factor` takes no numeric column (`factorable`).
  - **A declared order** from `frame.factor.levels` does not survive as
    a LIST of labels, so `mva.lda` takes it as an option. `levels` is a
    LIST naming every label once, in the order wanted; anything else is
    E101 naming `levels`.
  - Group order changes no statistic. It decides the order of
    `mva.lda`'s answers, of a `prior` (§S8) and of `mva.lda.predict`'s
    posterior columns, and the class only on an exact posterior tie
    (§S8).
- **Signs [V3].** Every word that answers singular vectors, eigenvectors
  or directions makes each vector's element of the largest absolute
  value positive, the first such on an exact tie. A paired vector (an
  SVD's u with its v; a component's scores with its loadings) is flipped
  with it, so every product the decomposition defines is unchanged.
  R's signs are LAPACK's and are not reproduced (§What R does). The
  fixtures compare each vector up to sign.
- **Names.** §S2 to §S11 list every name. Each fixed word has its `.`
  form.

### S2. The SVD

`linalg.svd` (`A -- MAP`) answers the thin SVD of an n × p MATRIX: `d`,
the r = min(n, p) singular values descending, and `u` (n × r) and `v`
(p × r) as MATRIXes, under §S1's sign rule applied to v's columns. It is
`nalgebra`'s `SVD::try_new` with `eps` = ε and `max_niter` 1000·max(n, p),
as `linalg.eigen.sym`'s cap is set; `None` is E110. Fixed, plain and `.`.

### S3. Principal components

`mva.pca` (`X opts -- MAP`) is R's `prcomp.default`:
- **opts:** `center` (BOOL, default true), `scale` (BOOL, default
  false), `rank` (INTEGER ≥ 1) and `tol` (FLOAT ≥ 0), each meaning what
  R's `center`, `scale.`, `rank.` and `tol` do. Under `scale`, a column
  whose scale is 0 is E101 naming it, as R stops (below).
- **Computation:** centre when `center` is true, then, when `scale` is
  true, divide each column by √(Σv²/max(1, n − 1)) of the column as it
  then is, as base R's `scale` does (`scale.R:38-46`):
  - centred, that is the sd on n − 1;
  - uncentred, it is the root mean square.

  A scale of 0 is E101 naming the column, as R stops (`prcomp.R:30-31`):
  any constant column when centred, only an all-zero column when not.
  Then §S2's SVD, with `sdev` = d/√max(1, n − 1). `rank` and `tol` cut
  the components kept, never `sdev` (`prcomp.R:34-50`).
- **Answers:** `sdev` (all r); `rotation`, p × k; `x`, the scores, n × k;
  `center` and `scale`, LISTs, or false when not applied; and `prop` and
  `cumprop`, the proportions of variance over all r. These are not
  rounded, where `summary.prcomp` stores them to five places
  **[author]**.
- n < 2 is accepted (R divides by max(1, n − 1)). Fixed, plain and `.`.

### S4. Classical multidimensional scaling

`mva.mds` (`D opts -- MAP`) is R's `cmdscale`:
- **D** is checked by the family's distance checks, `taxa`'s without the
  names operand (square, E102; at least two objects, zero diagonal,
  non-negative, E101; exactly symmetric, E107). That is stricter than
  R's matrix path (§What R does). Its cap is §S11's.
- **opts:** `k` (INTEGER in 1 … n − 1, default 2, else E101) and `add`
  (BOOL, default false).
  - `add` is Cailliez's constant: R's `max(Re(·))` over every
    eigenvalue of R's 2n × 2n matrix (`cmdscale.R:55-69`), complex ones
    included.
  - It is computed by `linalg.eigenvalues.complex`'s function, since
    `linalg.eigenvalues` refuses a complex eigenvalue.
- **Computation:** D² double-centred row then column, as `dblcen.c`; the
  eigendecomposition of −½ of it by `linalg.eigen.sym`'s function,
  reordered descending.
- **Answers:** `points`, n × k′, column j the jth eigenvector times √λⱼ
  under §S1's sign rule; `eig`, all n eigenvalues; `gof`, R's two
  goodness-of-fit ratios; `ac`, the additive constant (0.0 without
  `add`); and `k`, the columns kept.
- **Fewer than k positive eigenvalues** keep only the positive ones, so
  k′ < k, as R does, with `k` saying so and no warning **[author]**.
  "Positive" is strictly above 0, R's rule, with no tolerance.
- Fixed, plain and `.`.

### S5. Distances and hierarchical clustering [V7]

| Word | Operands | Answers |
|---|---|---|
| `cluster.dist` | X opts | a symmetric n × n MATRIX of distances, zero diagonal |
| `cluster.hclust` | D opts | `merge`, `height`, `order`, `method` |
| `cluster.cut` | tree k | a LIST of n cluster numbers |
| `cluster.cut.h` | tree h | a LIST of n cluster numbers |

- **`cluster.dist`** is R's `dist` over X's rows. Its `method` is
  `"euclidean"` (the default), `"maximum"`, `"manhattan"`,
  `"canberra"`, `"binary"` or `"minkowski"`, with `p` (FLOAT, finite,
  > 0, default 2) for the last. It answers the full symmetric MATRIX that
  `phylo.` and `cluster.hclust` read, not R's lower triangle
  **[author]**. Each method is R's arithmetic (`distance.c:42-192`):
  - **Canberra** skips a coordinate where both values are zero, and
    rescales the sum by nc/count, as R does (`distance.c:122`,
    `:135-136`). Two all-zero rows, where R answers NA, are E101 naming
    the pair **[author]**.
  - **Binary** answers 0 for two all-zero rows, as R does
    (`distance.c:166-167`).
  - A distance that overflows to ∞ is E114.

  Missing values cannot occur, so R's rescaling for them does not apply.
- **`cluster.hclust`** is R's `hclust.f` and `hcass2`, ported line for
  line. That keeps R's nearest-neighbour bookkeeping, its strict
  comparisons and so its merge order on ties (V7). Its `method` is any
  of R's eight names, default `"complete"`. `members` is a LIST of
  positive FLOATs, one per object, default all 1.
  - **D's checks** are the distance checks stated in §S4.
  - **The sentinel.** Every cell must be below R's sentinel 10³⁰⁰
    (`hclust.f:47`), else E101 naming the cell.
    - Under `ward.D2`, which squares every distance before the search
      (`:65-69`), the bound is on the square: a cell at or above 10¹⁵⁰
      is E101.
    - So the port's nearest-neighbour search always finds an index, and
      `NN(I) = 0` (`:53`, `:80`) cannot occur.
    - R does not check. On R 4.6.2,
      `hclust(as.dist(matrix(c(0,1e160,2e160, 1e160,0,3e160,
      2e160,3e160,0),3)), "ward.D2")` answers `merge` (0, 0), (1, 1),
      which is not a tree.
  - **Overflow.** An update that overflows (Ward's, `hclust.f:134-136`),
    or reaches the sentinel, is E114.

  Its answers are R's:
  - `merge`, an (n − 1) × 2 MATRIX of INTEGERs under R's sign convention;
  - `height`, the n − 1 heights, square-rooted under `ward.D2`;
  - `order`, the leaves 1 … n left to right;
  - `method`, the name.

  Inversions under `centroid` and `median` are answered as R answers
  them. n < 2 is E101.
- **`cluster.cut`** performs the first n − k merges, k in 1 … n (E101
  otherwise). **`cluster.cut.h`** performs every merge at height ≤ h,
  and refuses an unsorted `height` (an inversion) with E101 naming
  `tree`, as R stops. Both number clusters by their first member in
  observation order (`hclust-utils.c:87-95`). The tree operand is checked
  on entry: a MAP with `merge` and `height` of consistent shape, each
  merge naming a singleton or an earlier step once, else E101 naming
  `tree`.
- A dendrogram is not converted to `phylo.`'s tree here (§Alternatives).
- All four are fixed, plain and `.`.

### S6. k-means [V4] [V5]

`cluster.kmeans` (`X centers opts -- MAP`) is R's `kmeans`:
- **centers** is an INTEGER k (1 ≤ k ≤ the number of distinct rows, else
  E101) or a k × p MATRIX of starting centres. A MATRIX with duplicate
  rows, more rows than X, or another column count is refused with R's
  conditions (E101, E102). Unlike R, a 1 × 1 MATRIX is a centre, not k
  (§What R does).
- **opts:** `algorithm`, `"hartigan-wong"` (the default), `"lloyd"`
  (R's `"Forgy"` is the same code) or `"macqueen"`; `iter.max` (INTEGER
  ≥ 1, default 10); `nstart` (INTEGER ≥ 1, default 1). With a MATRIX of
  centres `nstart` is checked and then ignored, as R ignores it: R runs
  further starts only when it drew the centres (`kmeans.R:111`, `:126`),
  and on R 4.6.2 `kmeans(x, x[c(1, 3), ], nstart = 5)` answers as
  `nstart = 1` with no warning (owner's ruling, 2026-11-25; D278).
- **The algorithms** are AS 136's `kmns.f` and `kmeans.c`'s Lloyd and
  MacQueen, ported line for line, ties included (§What R does).
  Hartigan–Wong's `BIG` is R's single-precision 10³⁰: `1.E30` widened
  to double, 1.0000000150474662·10³⁰ (`kmns.f:30`). Its quick-transfer
  cap is R's 50·n steps. k = 1 answers the one cluster directly, as
  every algorithm reaches it, with `iter` 1, which is also what R's
  forced MacQueen answers (run on R 4.6.2) **[author]**.
  Hartigan–Wong with k equal to the number of rows is E101, as R stops.
- **Starts [V5].** For an INTEGER k, each of the `nstart` starts draws k
  of the distinct rows, taken in first-occurrence order, by `rng.choice`'s
  partial Fisher–Yates over the VM's generator. The run with the least
  total within-cluster sum of squares wins, the earliest on a tie. After
  the same `rng.seed` the answer repeats. R's streams are not reproduced,
  so the fixtures pass centres as a MATRIX.
- **Refusals where R warns [V6]:** an empty cluster is E101 under every
  algorithm. Not converging within `iter.max`, or Hartigan–Wong's
  quick-transfer cap, is E110. Lloyd's first pass, which only assigns,
  is counted as R counts it, so `iter.max = 1` under Lloyd is always
  E110, as R always warns.
- **Answers,** under R's key names (as D275's 3A kept R's): `cluster`,
  a LIST of INTEGERs 1 … k, cluster j the one started at centre j;
  `centers`, k × p; `totss`, `withinss`, `tot.withinss`, `betweenss` =
  totss − tot.withinss, `size` and `iter`.
- **Work (D39):** n·k·p·`iter.max`·`nstart` above 10¹⁰ is E111, before
  any draw, `nstart` counting as 1 with a MATRIX of centres
  **[author]**.
- Fixed (three operands), plain and `.`.

### S7. Hotelling's T² and MANOVA [V2] [V12]

| Word | Operands | Answers |
|---|---|---|
| `stat.test.hotelling1` | X mu | `statistic` (T²), `F`, `df1`, `df2`, `p`, `method` |
| `stat.test.hotelling2` | X Y | the same |
| `stat.test.manova` | X groups opts | `statistic`, `F`, `df1`, `df2`, `p`, `test`, `eig`, `method` |

- **`stat.test.hotelling1`:** T² = n(x̄ − μ)ᵀS⁻¹(x̄ − μ), S on n − 1, and
  F = (n − p)/(p(n − 1))·T² on p and n − p. `mu` is a LIST of p numbers
  (E102 otherwise). n − p < 1 is E101.
- **`stat.test.hotelling2`:** T² = n₁n₂/(n₁ + n₂)·(x̄₁ − x̄₂)ᵀS_p⁻¹(x̄₁ − x̄₂),
  S_p pooled on n₁ + n₂ − 2, and F = (n₁ + n₂ − p − 1)/(p(n₁ + n₂ − 2))·T²
  on p and n₁ + n₂ − p − 1. The column counts must agree (E102), and n₁ +
  n₂ − p − 1 < 1 is E101.
- **The covariance is solved by Cholesky.** A pivot at most 10⁻⁸ of the
  largest diagonal entry is E106 with `fit.mle`'s text (RFC-0019 §S11),
  naming the pivot. The pivot is d_j = L_jj², before the square root,
  as `fit.mle` tests it (`fit/mle.rs`, `inverse`, `PIVOT_FLOOR`).
- **`stat.test.manova`** is one-way: the groups are the only term (V12).
  - H is the between-groups and E the within-groups SSCP. The eigenvalues
    of E⁻¹H are those of the symmetric L⁻¹HL⁻ᵀ, E = LLᵀ. That gives the
    same values as R's real parts of a non-symmetric eigenproblem, with no
    complex parts to discard.
  - **Scaling.** E is first scaled as R scales it: sc = √diag(E), with
    sc_j set to 1 when sc_j < 10⁻⁶·√(E_jj + Σ H_jj) (`manova.R:98-102`).
  - **The rank test** is Cholesky's of the scaled E. A pivot
    d_j = L_jj² at most 10⁻¹⁴ of the largest diagonal entry of the
    scaled E is E106, naming the pivot **[author]**.
    - 10⁻¹⁴ is the square of R's 10⁻⁷, because `dqrdc2` compares column
      norms and d_j is a squared length.
    - It is not R's test. `dqrdc2` pivots by column norm with limits of
      its own, so near the threshold the two can disagree.
    - It tests the same d_j as the Hotelling words but at a smaller
      floor. Hotelling's 10⁻⁸ follows `fit.mle`'s precedent, where R's
      `solve` has no comparable test; MANOVA's follows R's own rank
      test, so an E that R accepts is not refused here.
  - `opts` takes `test`: `"pillai"` (the default, as R's), `"wilks"`,
    `"hotelling-lawley"` or `"roy"`. Each statistic and its F is
    `mlm.R:363-406`'s, line for line.
  - **Refusals before the statistic:** fewer than two groups, or
    N − g < p, is E101.
  - **Refusals after it:** where the chosen statistic's df1 or df2 is
    not positive, as Hotelling–Lawley's is at N − g = p with s ≥ 2
    (§What R does), or its F is not finite, the word refuses with E101
    naming `test`, where R answers NA **[author]**.
- All are fixed, plain and `.`. An F test answers `df1` and `df2`
  (D272).

### S8. Linear discriminant analysis [V13]

| Word | Operands | Answers |
|---|---|---|
| `mva.lda` | X groups opts | `prior`, `counts`, `means`, `scaling`, `lev`, `svd`, `N` |
| `mva.lda.predict` | fit X opts | `class`, `posterior`, `x` |

- **`mva.lda`** is MASS's `lda.default` under `method = "moment"`, line
  for line: the two SVDs by §S2's function, R's `tol` = 10⁻⁴ used
  absolutely and relatively where R uses it, and R's refusals.
  - A variable constant within groups is E101 naming it.
  - Rank 0 and identical group means are each E101.
  - R's "variables are collinear" warning is answered by fewer
    discriminants (`svd` shorter than p), with no warning **[author]**.
  - `opts` takes:
    - `prior`, a LIST of g non-negative numbers whose sum rounds to 1
      at five places, as R checks (`lda.R:90`); default the sample
      proportions. Entry k is the prior of the k-th group in `lev`'s
      order (§S1's, or `levels` when given), as R matches a prior to
      `levels(grouping)` by position (`lda.R:89-92`);
    - `tol`;
    - `levels` (§S1).
  - **Answers** use MASS's names (`lda.R:207-209`), as D275's 3A kept
    R's. `scaling` is p × r, §S1's sign rule applied to each column.
    `lev` holds the group labels in §S1's order, and `N` the number of
    rows.
- **`mva.lda.predict`** is `predict.lda`'s "plug-in" method.
  - `fit` is a MAP `mva.lda` answered, checked for its keys and shapes
    (E101 naming `fit`).
  - X has the fit's p columns (E102).
  - `opts` takes `prior` (default the fit's, in the fit's `lev` order,
    with `mva.lda`'s checks) and `dimen`, an INTEGER ≥ 1,
    E101 below 1. `dimen` is clamped to r as R clamps it (`lda.R:260`).
  - It answers `class` (a LIST of labels), `posterior` (an n × g MATRIX),
    and `x`, the discriminant scores.
  - The scores are centred on the prior-weighted mean under the prior
    in use, as R does (`lda.R:255-258`), so they move with `prior`.
  - A group whose prior is 0 gets posterior 0 directly, never through
    log 0 (`lda.R:264`).
  - An exact tie in the posterior takes the first group in §S1's order,
    where R's `max.col` breaks it at random **[author]**.
- Fixed (three operands), plain and `.`.

### S9. Kernel density and bandwidths [V8] [V9] [V10]

- **`stat.density`** (`x opts -- MAP`) is R's `density.default`, ported
  (V8): R's grid rounding, `ext` = 4, linear binning as `massdist.c`,
  the circular convolution by FFT, the clamp at 0, and linear
  interpolation onto `n` points from `from` to `to`.
  - **The FFT** is Bund2's own iterative radix-2 transform in f64. The
    grid is 2·max(512, 2^⌈log₂ n⌉) points, a power of two, so no
    mixed-radix code is needed, and no crate is added. It agrees with R's
    `fft` to rounding, not bit for bit
    [UNGROUNDED: R's `src/appl/fft.c` was not read; Q68], so criterion 9
    compares at 10⁻¹² absolute.
  - **opts:** `bw` (a FLOAT > 0 or one of §`stat.bw`'s rule names,
    default `"nrd0"`), `adjust` (FLOAT > 0, default 1), `kernel` (R's
    seven names, default `"gaussian"`), `n` (INTEGER 1 … 2²⁰, default
    512), `from` and `to` (finite FLOATs, from < to), and `cut` (FLOAT
    ≥ 0, default 3).
  - **What is left out:** weights and R's sub-density for ±∞ values are
    not taken; a non-finite x is E101, the family's rule (B12)
    **[author]**.
  - **Answers:** `x` and `y`, LISTs of n FLOATs; `bw`, after `adjust`;
    and `n`, the sample size.
  - **Sample size.** An empty x is E101. One value is accepted with a
    numeric `bw`. A rule needs two values, else E101, as R stops
    (`density.R:121-123`).
  - Fixed, plain and `.`.
- **`stat.bw`** (`x rule -- FLOAT`), with the rules (V9):
  - `"nrd0"`, `bw.nrd0` with its fallbacks;
  - `"nrd"`, `bw.nrd`, which is E101 where it is 0;
  - `"sj"` and `"sj-ste"`, `bw.SJ`'s "ste";
  - `"sj-dpi"`, `bw.SJ`'s "dpi".

  Its arithmetic is `bandwidths.R` and `bandwidths.c` at R 4.6.2: the
  pair counts, `nb` = 1000, the n > 500 switch to binned counts, and the
  constant-data refusal (E101). The binned path's constant data, which R
  stops as "too sparse", are E101 too. IQR is `stat.quantile`'s type-7
  arithmetic, and sd `stat.sd`'s. `nrd0`'s |x₁| is the operand's first
  value as given. "ste" solves by `optim.root`'s `zeroin`
  with R's interval rule and R's tolerance: 0.1 × `lower`, taken after
  any widening, as R's lazy default evaluates (V10). So its answer equals
  R's, not the exact root. R's two "sample is too sparse" stops and "no
  solution in the specified range" are E101. Fewer than two values is
  E101. Plain and `.`, `scalar_forms!`'s shape.

### S10. The ECDF and histograms [V11]

- **`stat.ecdf`** (`x -- MAP`) answers `x`, the sorted distinct values,
  and `y`, #{xᵢ ≤ xⱼ}/n at each. Four forms.
- **`stat.ecdf.at`** (`x t -- FLOAT`) answers #{xᵢ ≤ t}/n. Two forms, as
  `stat.quantile`.
- An empty x is E101 for both, as R stops: "'x' must have 1 or more
  non-missing values" (run on R 4.6.2). A NaN `t` is E101 (B12), where R
  answers NaN. A `t` of −∞ or +∞ answers 0 or 1, as R does, since the
  count is defined for any ordered `t`.
- **`stat.hist`** (`x opts -- MAP`) is R's `hist.default` without the
  plot.
  - `breaks` is `"sturges"` (the default), `"scott"`, `"fd"`, an
    INTEGER class count, or a LIST of at least two breaks (E101
    otherwise).
    - A LIST must be strictly increasing, else E101. R sorts the breaks
      and accepts duplicates (`hist.R:48`), and a duplicate gives a
      zero-width bin whose density is NaN **[author]**.
    - A class count above 10⁶ is E111. R clamps it to 10⁶ with a
      warning (`hist.R:77-81`) **[author]**.
  - A rule or a count becomes breaks by R's `pretty(range(x), n, min.n =
    1)`, ported line for line from `pretty.c` with R's defaults (V11).
    The port computes `pretty.c:155`'s power of ten with the platform's
    `pow`, Rust's `powf`, never `powi`, because R relies on `pow` being
    exact there (`:153-154`).
  - `right` and `include.lowest` are BOOLs, default true.
    `include.lowest` false with a rule or a count is E101 naming it. R
    ignores it there with a warning (`hist.R:50-53`) **[author]**.
  - Counting uses R's fuzz, 10⁻⁷ of the median, range or least width
    (`hist.R:117-125`), and `BinCount`'s rules.
  - Answers: `breaks`, `counts` (INTEGERs), `density` = counts/(n·width),
    `mids` and `equidist`.
  - A value outside a LIST of breaks is E101, R's "some 'x' not counted".
    Scott's and FD's rules on one value are E101, where R fails on an NA
    variance.
  - Fixed, plain and `.`.

### S11. Work caps

Each cap is reached before the work, E111 (D39) **[author]**:
- `cluster.dist`, `cluster.hclust` and `mva.mds` read at most 5000
  objects, since each holds an n × n MATRIX and `hclust` is O(n³) at
  worst. `phylo.`'s cap is 2000 for its O(n³) joins (`TAXA_CAP`).
- `cluster.dist` also caps n²·p at 10¹⁰.
- `cluster.kmeans`: §S6.
- `mva.pca`, `linalg.svd`, `mva.lda` and `stat.test.manova` cap
  n·p·min(n, p) at 10¹⁰.
- `stat.density`: the grid as §S9, and n·`n` interpolations are linear.
  `stat.bw`'s unbinned counts are n ≤ 500 by R's own switch.
- `stat.hist`: a class count above 10⁶ (§S10), as R's own limit, and a
  LIST of more than 10⁶ breaks.

### S12. The tiers, the audits and the generated files

As RFC-0016 §S8 and RFC-0019 §S13:
- **Effects as declared,** the palette audit deciding what
  `PROMOTABLE.txt` lists.
- **Regenerated in each milestone:** `PROMOTABLE.txt`, `bund2 effects`
  and `docs/arity.md`.
- **No golden is added** (D21).
- **Licence.** The ports this RFC calls line for line (`hclust.f`,
  `kmns.f`, `kmeans.c`, `pretty.c`, `density.R`, `bandwidths.c`,
  `mlm.R`, MASS's `lda.R`) derive from GPL-2-or-later code (MASS is
  GPL-2 or GPL-3). D216 already allows GPL code in the build ("a binary
  distributed from this workspace carries GPL-3.0-or-later"), and
  RFC-0019's ports of `zeroin`, `nmmin` and `vmmin` took the same path,
  so this adds no new obligation.
- **`UNVARIED`** gains each fixed native that consumes three or more
  operands on the stack, or whose `.` form consumes two or more
  (`lib.rs`, `varied_of`). That is `cluster.kmeans`, `stat.test.manova`,
  `mva.lda` and `mva.lda.predict`, with their `.` forms. Each milestone
  states its additions against the audit's own selection.
- **The registration tests:**
  - `linalg.svd` joins `linalg.rs`'s `WORDS`;
  - the `stat.test.` names join a module's `WORDS` with the exclusion
    in `infer`'s counting test, as RFC-0019's did;
  - `stat.density`, `stat.bw`, `stat.ecdf`, `stat.ecdf.at` and
    `stat.hist` are listed or excluded in `describe/tests.rs`'s
    `every_word_and_form_is_registered`;
  - `mva.` and `cluster.` are modules of their own, each with a counting
    test.
- **The re-entry audits** (`lib.rs`,
  `every_native_that_changes_the_current_stack_is_named` and
  `every_reentering_function_is_named`) see no new name: nothing here
  calls a lambda or switches stacks.
- **Shared helpers.** Each helper is extracted without changing the word
  it came from (§Preservation analysis):
  - `taxa`'s checks, without the names operand and with the cap a
    parameter;
  - `linalg_eigen_sym`'s and `linalg_eigenvalues_complex`'s
    computations;
  - `quantile`'s type-7 arithmetic as an f64 function;
  - `optim.rs`'s `zeroin`.
- **ERRATA.** At acceptance, `docs/research/ERRATA.md` records where this
  RFC goes beyond research 07 §3.7 (`07-statistical-research.md:186-193`):
  - eight linkages, average linkage a port of its own rather than
    `phylo.upgma`;
  - Scott's bandwidth beside Silverman's and Sheather–Jones';
  - MDS's additive constant by a non-symmetric eigensolver.

### S13. Assumptions the implementation checks first

1. **Rounding.** The exact equalities of criteria 4, 5 and 8 assume
   that R's compiled Fortran and C round as an unfused port does.
   GCC's default `-ffp-contract=fast` can fuse a·b + c on arm64, and Rust
   never does. **[UNGROUNDED:** the build flags of the owner's R were
   not read; Q69.] If a fixture disagrees in the last place on a tie,
   that is the cause to look for first, and the case is taken to a
   decision, not loosened silently.
2. **`uniroot`.** It evaluates f at both ends, then calls `R_zeroin2`
   with `maxiter` 1000 (`nlm.R:55-60`). §S9's call through `optim.rs`'s
   `zeroin` reproduces that sequence, not only its arithmetic.
3. **nalgebra's wide SVD.** `SVD::try_new` on a wide matrix (n < p)
   answers u n × r and vᵀ r × p. Only `try_new` (`svd.rs:688-715`) was
   read; M1's tests confirm the shapes.
4. **Hosts.** Fixtures come from the owner's arm64 macOS; CI runs Linux.
   The tolerances of criteria 3, 6, 7 and 9 (10⁻⁹ and 10⁻¹² relative,
   and criterion 9's 10⁻¹² absolute on `y`)
   are far above the platform library's few ulp. The exact comparisons
   of criteria 4, 5 and 8 are of integers, merge orders and `pretty`'s
   breaks.
   - Merge orders depend on ties decided before any library call.
   - `pretty`'s breaks depend on `log10` and `pow`.
   - If CI disagrees with the macOS answer, D276's per-host fixture is
     the precedent, raised as a decision.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| Every existing `linalg.`, `stat.`, `phylo.`, `frame.`, `rng.` word: operands, effects, forms, answers, refusals, cost | **Preserved exactly.** Every name here is new |
| `linalg.eigen.sym`'s ascending order and unnormalised signs | **Preserved exactly.** §S1's sign rule and the descending order apply to the new words only; `mva.mds` reorders its own copy |
| `phylo.upgma`'s tree and tie rule | **Preserved exactly.** `cluster.hclust`'s `"average"` is a separate port with R's tie rule; the two can merge tied pairs in different orders, and say so in their docs |
| The reference's `clusters.detect` and `clusters.detect.` | **Preserved exactly:** no name proposed begins `clusters.` |
| The alias `mv` (`fs.mv`) | **Preserved exactly:** no name proposed begins `mv` (V1) |
| The VM's generator and every seeded stream | **Preserved exactly:** only `cluster.kmeans` with an INTEGER k draws, and only when called |
| A program that defines one of the new names itself | **Preserved:** a program's word is found before a native |
| A program that asks whether one of the new names exists (`?stdlib`, `?word`) | **Changed by design:** the answer becomes true. No such program is known |
| Registration counting tests (`linalg`'s, `infer`'s, `describe/tests.rs`'s `every_word_and_form_is_registered`) | **Amended deliberately**, each milestone stating its additions (§S12) |
| RFC-0015 §S11's refusals, codes and texts | **Preserved exactly:** no variant or text is added |
| `phylo.upgma`, `phylo.nj` (`taxa`), `linalg.eigen.sym`, `linalg.eigenvalues.complex`, `stat.quantile`, `optim.root`, whose internals §S12 extracts as shared functions | **Preserved exactly:** operands, refusal order and texts, answers and caps unchanged; `taxa` keeps its names operand and `TAXA_CAP`; each word's existing tests pass unchanged |
| The re-entry audits | **Preserved exactly:** no new word calls a lambda |
| `UNVARIED` | **Extended** by §S12's names, each with its reason |
| `PROMOTABLE.txt`, `bund2 effects`, `docs/arity.md` | **Regenerated**, existing name lines not moving |
| `conform` 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602 | **Preserved exactly** (D72): no reference word is touched |

## Alternatives considered

- **`mv.` or `stat.` for everything** (V1, options B and D). `mv.` is
  what research and R users would type, but the reference's `mv` moves
  files, and a program reading `mv.pca` beside `mv` would see one family.
  Putting everything under `stat.` loses D260 C5's rule of a prefix per
  new kind of operand.
- **`clust.` in place of `cluster.`** (V1, option C). It is further from
  the reference's `clusters.`, at the cost of an abbreviation no other
  Bund2 prefix uses.
- **A LIST of MATRIXes, one per group** (V2, option B), as
  `stat.test.anova1`'s LIST of LISTs. It needs no label order, but
  `mva.lda.predict` must answer labels, and a frame's factor column is a
  LIST of labels already.
- **R's signs** (V3). They are LAPACK's `dgesdd` and `dsyevr`'s, and
  porting those routines to reproduce a sign is out of proportion.
  `princomp`'s rule (first element non-negative) is R's only
  normalisation, but flips with noise when that element is near 0. The
  largest element is stable.
- **Lloyd only** (V4, option B). It is the textbook algorithm, but R's
  default is Hartigan–Wong, whose answers usually have a lower
  within-cluster sum, so a program checked against R would disagree.
- **R's two-step start rule** (V5, option B). R's draws are its
  Mersenne-Twister's and cannot be reproduced. Keeping its "draw again
  from the unique rows only on a duplicate" would complicate the stream
  for nothing, so every start draws from the distinct rows.
- **A naive Lance–Williams `hclust`** (V7, option B), reusing
  `phylo.rs`'s `least`. It is simpler and O(n³), but merges tied pairs
  in another order than R's, so `merge`, `order` and every cut would
  differ from R's on tied data, which integer or rounded data often is.
- **An exact kernel sum** (V8, options B and C). It is the definition,
  checkable at 50 digits, but differs from R by about 10⁻⁵ absolute, so a
  density plotted in Bund2 would not overlay R's. Option C would offer
  both.
- **`ucv` and `bcv`** (V9, option B). They need Brent's `fmin`, which is
  not yet ported, and on small samples both stop at the end of their
  range with a warning (both do on x₁₀, run on R 4.6.2).
- **A tight tolerance for `bw.SJ`** (V10, option B), as N7 chose for
  `optim.root`. That gives the exact root, which differs from R's answer
  in the third digit, so a bandwidth "by Sheather–Jones" would not be R's.
- **Equal-width breaks** (V11, option B). They are simpler than `pretty`,
  but counts would differ from R's `hist` on almost every input.
- **MANOVA over a model design** (V12, option B). RFC-0018's
  `model.matrix` takes one response (`model/code.rs`, `model.matrix`
  (`frame response terms opts -- design`)), so several responses would
  amend it. One-way MANOVA covers research 07's need, and a later RFC can
  take designs.
- **Fisher's directions only** (V13, option B). It is less work, but
  without prediction LDA cannot classify.
- **A dendrogram as a `phylo.` tree.** `phylo.newick` could then print it.
  Heights are merge heights, not branch lengths, and a conversion word
  is a later RFC's, if wanted.
- **`augurs-clustering`'s DBSCAN** for a density-based word. It is
  linked (`Cargo.lock`), but DBSCAN is not in research 07's Phase F, and
  its labels and noise convention are the reference's `clusters.detect`'s.

## Acceptance criteria

Reference values follow D260 C4's option D, as V14 rules which words need
R. By default they come from definitions, computed with mpmath at 50
digits by fixture scripts under `crates/bund2-stdlib/src/biomath/fixtures/`.
For the words V14 puts under definitions (`linalg.svd`, `mva.pca`,
`mva.mds`, `cluster.dist`, Hotelling's, the ECDF), the definitional
fixture is the check and R is a second one on the same input.
Where an answer depends on R's algorithm (V14's list), they come from R
at the version the committed output records, 4.6.2 with MASS 7.3-66 on
the owner's machine, by committed `.R` scripts.

**R is not a dependency** of the finished product, on the owner's word
of 2026-11-25:
- no build, test, CI step or runtime path runs or links R;
- every R answer is computed once, by hand, and committed as a text
  fixture beside its script, as RFC-0019's `shapiro.R` and `power_r.R`
  are (`fixtures/shapiro.R`, header);
- the tests read only the committed text;
- every input a test takes from R is committed with it: x₁₀₀₀, and each
  dataset (`USArrests`, `eurodist`, `iris`, and whichever criterion 10's
  example uses) is written by its `.R` script into the committed
  fixture, with the R version that wrote it;
- an `.R` script is documentation of where a value came from, and needs R
  only to regenerate it;
- the ported algorithms are Bund2's own Rust, and no R code or library
  is called.

Vectors are compared up
to sign (§S1). Tolerances are |a − b| ≤ 10⁻⁹|b| + 10⁻¹² unless a
criterion says otherwise, and a p below 10⁻⁶ is compared absolutely at
10⁻¹². §S13 says which comparisons are exact and why.

**Fixture inputs.** Each is written into its committed script. Where a
criterion leaves its input to the implementer (criteria 2, 4 and 5 in
part), the milestone commits the input it chose, beside its answer:
- **x₁₀** = 1.2, 2.5, 2.5, 3.1, 4.8, 5.0, 7.3, 8.8, 9.1, 10.4.
- **x₁₀₀₀** = R's `set.seed(1); rnorm(1000)`. The `.R` script writes the
  values into the fixture, since Bund2's generator is not R's.
- **The `iris` subsets:**
  - versicolor is rows 51–100, virginica rows 101–150, each
    `iris[, 1:4]`;
  - one-sample Hotelling is versicolor against μ = (6, 2.8, 4.2, 1.3);
  - two-sample Hotelling is versicolor against virginica;
  - the three-group MANOVA is all 150 rows by `Species`.
- **MANOVA's edge cases:**
  - a rank-deficient E: the three-group input with column 4 replaced by
    column 1 + column 2;
  - zero df: rows (1, 2), (2, 1), (3, 5), (4, 3), (6, 2), with groups
    1, 1, 2, 2, 3. Under `"hotelling-lawley"` R answers F 0 on 4 and 0,
    with p NA. Under `"wilks"` it answers Λ 0.00186289120715 and F
    11.0844723661 on 4 and 2, p 0.0844595556 (run on R 4.6.2).
- **MDS with `add`:** the Manhattan distances between the five points
  (0, 0), (1, 0), (0, 1), (1, 1) and (3, 2). Cailliez's matrix has the
  eigenvalues −3.281650 ± 0.517931i among others, and R's `ac` is
  1.424602 (run on R 4.6.2).
- **`stat.hist`'s cases:**
  - x₁₀ under each rule;
  - the tied input 1, 2, 2, 3, 3, 3, 4, with `right` true and false;
  - the constant input 5, 5, 5, and the single value 7;
  - the fuzz case: x = 0.05, 0.08, 0.15, 0.30000000000000004, 0.5 with
    breaks 0, 0.1, 0.2, 0.3, 1. R counts 2, 1, 1, 1 (run on R 4.6.2).
- **`pretty`'s cases**, with R's answers on R 4.6.2:
  - `pretty(c(0.13, 9.71))` → 0, 2, …, 10; with n = 10, 0 … 10;
  - `pretty(c(1, 1))` → 0, 1;
  - `pretty(c(0, 0))` → −1, 0;
  - `pretty(c(-3.3, 1e5), n = 4)` → −20000, 0, …, 100000;
  - `pretty(c(1e-300, 2e-300))` → 1.0e-300 … 2.0e-300 by 0.2e-300;
  - `pretty(c(123456789, 123456789.5))` → 123456789.0, 123456789.1, …,
    123456789.5, six values.

1. **Registration.** Every name of §S2 to §S10 is registered with its
   declared effect and forms. The counting tests pass as §S12 amends
   them, and nothing else is added.
2. **SVD.** `linalg.svd` reconstructs A = U·diag(d)·Vᵀ to 10⁻¹²·‖A‖ on
   square, tall, wide and rank-deficient inputs. d matches the fixture,
   and each column obeys §S1's sign rule.
3. **PCA and MDS.**
   - `mva.pca` on `USArrests`, with `scale` false, with `scale` true, and
     with `center` false and `scale` true, matches the definitional
     fixture (mpmath's SVD of the centred or scaled matrix). Its `sdev`
     equals R's, and its `rotation` and `x` match R's up to each column's
     sign. `rank` and `tol` cut as R's do.
   - `mva.mds` on `eurodist` matches the definitional fixture (mpmath's
     eigendecomposition of −½ the double-centred D²) and R's `points` (up
     to sign), `eig` and `gof`.
   - `mva.mds` with `add` on §Fixture inputs' Manhattan matrix, whose
     Cailliez matrix has complex eigenvalues, gives R's `ac` and
     `points`.
   - A case with fewer than k positive eigenvalues answers R's k′.
4. **Hierarchical clustering.**
   - For each of the eight methods, on a tied input (0, 1, 2, 3, 10, 11),
     an untied one and an inversion (centroid), `merge`, `height` and
     `order` equal R's exactly, heights at the tolerance.
   - `members` matches R's on one input.
   - `cluster.cut` and `cluster.cut.h` equal R's `cutree` for every k and
     for heights at, between and beyond the merges; an inversion under
     `cluster.cut.h` is E101.
   - `cluster.dist` matches the definition and R's `dist` for each
     method, on an input with zero coordinates. It includes Canberra's
     rescaled pair (0, 1), (0, 3), which gives 1, binary's all-zero pair,
     which gives 0, and Canberra's all-zero pair, which is E101.
   - `cluster.hclust` refuses a cell at or above 10³⁰⁰ (E101), and
     under `ward.D2` the three-object input of §S5 with cells 10¹⁶⁰ to
     3·10¹⁶⁰ (E101); and `cluster.dist` refuses an overflowing
     distance (E114).
5. **k-means.**
   - Given centres as a MATRIX, each algorithm's `cluster`, `centers`,
     `withinss` and `iter` equal R's on inputs with ties and with more
     than one transfer pass.
   - With an INTEGER k, the answer repeats after `rng.seed`, and its
     starts equal `rng.choice` of the distinct rows' indices.
   - An empty cluster under Lloyd is E101, and Lloyd at `iter.max` 1 is
     E110.
   - Given centres as a MATRIX, `nstart` 5 answers as `nstart` 1, as R
     does; `nstart` 0 is still E101.
6. **Hotelling and MANOVA.**
   - `stat.test.hotelling1` and `stat.test.hotelling2` match the
     fixture's definition and R's `summary.manova` Hotelling–Lawley route
     on `iris` (F 1.8347893795 on 4 and 46; F 86.147586209 on 4 and 95).
   - `stat.test.manova`'s four statistics, F, df and p match R's on the
     three-group input.
   - The rank-deficient E is E106.
   - The zero-df case under `"hotelling-lawley"` is E101, and under
     `"wilks"` answers R's Λ, F, df and p.
7. **LDA.**
   - `mva.lda` on `iris` gives R's `svd`, `means`, `prior` and `counts`,
     and `scaling` up to each column's sign.
   - With `levels` reversed, `lev`, `prior`, `counts`, `means` and the
     posterior columns are reversed. `svd`, `scaling`, `class` and `x`
     do not change. A non-uniform `prior` given in `lev`'s order, and
     reversed with `levels`, gives the same posteriors, columns
     reversed.
   - `mva.lda.predict` gives R's `class` and `posterior` on R's four rows
     (1, 51, 101, 71), and the scores up to sign.
   - A constant variable within groups is E101 with its name.
8. **ECDF and histograms.**
   - `stat.ecdf` and `stat.ecdf.at` match the definition with ties.
   - `stat.hist` equals R's `hist(…, plot = FALSE)` on §Fixture inputs'
     cases: breaks, counts and mids exactly, density at the tolerance.
   - A duplicated break, `include.lowest` false with a rule, and a
     class count above 10⁶ each refuse as §S10 says.
   - The ported `pretty` equals R's on §Fixture inputs' cases.
9. **Density.**
   - `stat.bw` equals R's `bw.nrd0`, `bw.nrd` and `bw.SJ` (both methods)
     at 10⁻¹² relative, on x₁₀ and x₁₀₀₀, one side of the n > 500
     switch each.
   - `stat.density`'s `y` equals R's at 10⁻¹² absolute for each kernel,
     on both samples, with default and given `n`, `from`, `to`.
10. **Example.** `bund_examples/multivariate/` reduces, clusters, tests,
    classifies and estimates a density on one data set, prints
    `expected.txt`, and a test in `crates/bund2-cli/tests` compares the
    two, as `bund_examples/inference/` does.
11. **No abort.** Every name and form answers or refuses with an E1nn or
    a shallow-stack error, on a palette of:
    - NaN and ±∞;
    - wrong types;
    - empty, ragged and NONE-filled operands;
    - large finite values (10¹⁶⁰ to 10³⁰⁷), which reach §S1's overflow
      rule (E114).
12. **Health.**
    - `cargo xtask conform` prints 143/159 and CEILING 143/159, and
      `coverage` prints IMPLEMENTED 597/602 and COVERAGE 577/602.
    - clippy, `cargo fmt --check` and `cargo deny` (after `cargo deny
      fetch`) are clean.
    - `PROMOTABLE.txt` is regenerated with no existing name line moved.
    - The `UNVARIED` audit passes with §S12's names.
    - RFC-0015 to RFC-0019's tests pass unchanged, except the counting
      tests §S12 amends.

## Implementation plan

| Milestone | Scope | Criteria |
|---|---|---|
| M1 | §S2, §S3, §S4: `linalg.svd`, `mva.pca`, `mva.mds` | 2, 3 |
| M2 | §S5: `cluster.dist`, `cluster.hclust`, `cluster.cut`, `cluster.cut.h` | 4 |
| M3 | §S6: `cluster.kmeans` | 5 |
| M4 | §S7: Hotelling's T², MANOVA | 6 |
| M5 | §S8: `mva.lda`, `mva.lda.predict` | 7 |
| M6 | §S10: the ECDF and `stat.hist` with `pretty` | 8 |
| M7 | §S9: `stat.bw`, `stat.density` | 9 |
| M8 | the example | 10 |

Each milestone ends with criteria 1, 11 and 12, and with CI checked
after its push. M1 first: every later word that decomposes a matrix uses
§S2's function and §S1's sign rule.

## Open questions

**Ruled 2026-11-25 (D277):** the owner accepted every recommendation,
"V1–V14: recommendations". The "Recommended" column below is what was
ruled. The **[author]** choices that follow were not part of that
ruling.

| | Question | Options | Recommended |
|---|---|---|---|
| **V1** | Names | **A** `linalg.svd`; `mva.` (pca, mds, lda, lda.predict); `cluster.` (dist, hclust, cut, cut.h, kmeans); `stat.test.hotelling1`, `hotelling2`, `manova`; `stat.density`, `stat.bw`, `stat.ecdf`, `stat.ecdf.at`, `stat.hist`. **B** as A with `mv.` for `mva.`. **C** as A with `clust.` for `cluster.`. **D** everything under `stat.` | **A**: a prefix per new kind of operand (D260 C5), clear of `mv` and `clusters.` |
| **V2** | Grouped data | **A** a MATRIX and a LIST of labels, as R's `lda(x, grouping)`. **B** a LIST of MATRIXes, one per group, as `stat.test.anova1`. **C** a frame, column names and a group column | **A**: prediction answers labels, and a factor column is one |
| **V3** | Signs of vectors | **A** each vector's largest-magnitude element positive, its pair flipped with it. **B** `princomp`'s: the first element non-negative. **C** `nalgebra`'s as computed, unspecified | **A**: stable and deterministic. R's signs are LAPACK's and cannot be matched (§What R does) |
| **V4** | k-means algorithms | **A** R's three, Hartigan–Wong the default, ported line for line. **B** Lloyd only. **C** Hartigan–Wong only | **A** |
| **V5** | k-means starts | **A** every start draws k of the distinct rows by `rng.choice`. **B** R's rule: rows first, distinct rows only after a duplicate | **A**: R's stream cannot be matched anyway |
| **V6** | Where R warns of a failure (k-means' empty cluster, non-convergence, the quick-transfer cap) | **A** refuse: E101, E110 (as N6). **B** answer, with a flag | **A** |
| **V7** | Hierarchical clustering | **A** R's `hclust.f` and `hcass2`, ported, with R's eight methods, tie rules and `merge`/`height`/`order`. **B** Bund2's own Lance–Williams with `phylo.upgma`'s tie rule | **A**: ties are common, and B's merge order differs from R's on them |
| **V8** | Kernel density | **A** R's binned FFT, ported. **B** the exact sum of kernels. **C** A, with B under `exact` true | **A**: R's answer is the one a reader compares |
| **V9** | Bandwidth rules | **A** `nrd0`, `nrd`, `sj-ste`, `sj-dpi`. **B** also `ucv` and `bcv`, porting Brent's `fmin` | **A**: research 07 names Silverman's and Sheather–Jones' |
| **V10** | `bw.SJ`'s tolerance | **A** R's, 0.1 × `lower` as R evaluates it, so the answer is R's. **B** a tight one (N7's), the exact root | **A**: the rule is R's algorithm, and B differs from R in the third digit |
| **V11** | Histogram breaks | **A** R's `pretty` and `hist` rules, ported. **B** equal widths over the range | **A** |
| **V12** | MANOVA's scope | **A** one-way over groups, the four statistics, Pillai's the default; one- and two-sample Hotelling. **B** also MANOVA over RFC-0018's designs, amending `model.matrix` for several responses | **A** |
| **V13** | LDA | **A** MASS's "moment" `lda` and the plug-in prediction. **B** Fisher's directions only | **A** |
| **V14** | Which words take R fixtures (D260 C4's option D says R where an algorithm's choices decide) | **A** R 4.6.2 with MASS 7.3-66 for `cluster.hclust`, `cluster.cut*`, `cluster.kmeans`, `stat.density`, `stat.bw`, `stat.hist`, `mva.lda*` and `stat.test.manova`; definitions for `linalg.svd`, `mva.pca`, `mva.mds`, `cluster.dist`, Hotelling's and the ECDF, with R as a second check. **B** R for every word. **C** definitions only | **A** |

**[author]** choices, D278, **RESOLVED 2026-11-25** as written below.
Choices 1 and 6 were restated and 10 to 14 added on 2026-11-25, after
the first review:
1. **Group order (§S1).** STRING labels sort by bytes, as `frame.factor`
   sorts. INTEGER labels sort ascending, this RFC's own rule. `mva.lda`'s
   `levels` option gives an explicit order.
2. `mva.pca`'s proportions are not rounded (§S3).
3. `mva.mds` keeps only positive eigenvalues, with no warning, `k` saying
   how many (§S4).
4. `cluster.dist` answers the full symmetric MATRIX (§S5).
5. k-means with k = 1 answers directly, `iter` 1; a 1 × 1 MATRIX is a
   centre (§S6).
6. **MANOVA's rank test (§S7).** A Cholesky pivot L_jj² at most 10⁻¹⁴
   of the scaled E's largest diagonal entry is E106. The threshold is
   the square of R's 10⁻⁷, but this is not `dqrdc2`'s test.
7. LDA's collinearity answers fewer discriminants with no warning, and
   a posterior tie takes the first group (§S8).
8. `stat.density` takes no weights, and refuses ±∞ (§S9).
9. The work caps of §S11, now including `stat.hist`'s.
10. **Canberra's all-zero pair** is E101, where R answers NA (§S5).
11. **Non-finite values** in a series given to `stat.density`,
    `stat.bw`, `stat.ecdf`, `stat.ecdf.at` or `stat.hist` are E101,
    where R drops them (`hist`, `density`), keeps ±∞ (`ecdf`) or stops
    (`bw.SJ`, `bw.nrd0`) (§S1).
12. **`stat.hist`'s breaks (§S10).** A LIST of breaks that is not
    strictly increasing is E101, where R sorts it and accepts
    duplicates. `include.lowest` false with a rule or count is E101,
    where R ignores it with a warning. A count above 10⁶ is E111, where
    R clamps it with a warning.
13. **MANOVA's undefined F (§S7).** A statistic whose df1 or df2 is not
    positive, or whose F is not finite, is E101, where R answers NA.
14. **`cluster.hclust`'s sentinel (§S5).** A distance at or above R's
    sentinel 10³⁰⁰ is E101, or at or above 10¹⁵⁰ under `ward.D2`, whose
    squares the sentinel bounds. An overflowing update, or a distance that
    overflows in `cluster.dist`, is E114.

**[UNGROUNDED]**, Q68: R's `fft` (`src/appl/fft.c`) and `max.col` were
not read, so neither the claim that §S9's radix-2 FFT agrees with R's to
rounding nor R's random tie-breaking in `max.col` is grounded in source.

**[UNGROUNDED]**, Q69: the build flags of the owner's R were not read, so
§S13's first assumption, that R rounds as an unfused port, is not
grounded.
