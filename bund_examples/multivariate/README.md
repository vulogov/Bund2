# multivariate

RFC-0020's words over one data set, Anderson's irises: fifty flowers of
each of three species, four measurements each. The flowers are reduced to
principal components, clustered with no species given, tested for a
difference between species, classified, and one measurement's density
estimated.

    make            # run demo.bund
    make check      # run it and compare the output with expected.txt

or, without `make`, from this directory:

    bund2 demo.bund

The one draw, k-means's starts, follows `42 rng.seed`, so a run prints the
same lines each time. The numbers are rounded to six decimals before they
are printed.

## The output

`expected.txt` is what a run prints. A test in `crates/bund2-cli/tests`,
`multivariate_demo.rs`, runs the demo and compares the two. It is not a
golden: the reference has none of these words, so there is no oracle to
capture one from. A second test there holds the demo's CSV text to the
`iris` that R 4.6.2 wrote into the committed fixture
`crates/bund2-stdlib/src/biomath/fixtures/lda_r.txt`, cell for cell and
species for species.

| | What it shows | Words | It prints |
|---|---|---|---|
| 0 | The flowers read from CSV text into a frame, the four measurements as a MATRIX, the species as the groups | `frame.csv.parse`, `frame.matrix`, `frame.column` | 150 rows, five columns |
| 1 | Principal components of the scaled measurements, and the first component's loadings | `mva.pca` | sdev 1.708361 … 0.143926; 73.0% and 95.8% of the variance in one and two components |
| 2 | An average-linkage tree cut into three, against the species; k-means from ten seeded starts | `cluster.dist`, `cluster.hclust`, `cluster.cut`, `frame.aggregate`, `cluster.kmeans` | setosa alone, 14 virginica with the versicolor; sizes 38, 62, 50, within-cluster SS 78.851441, 88.4% between |
| 3 | Whether the species differ in their four means together | `stat.test.manova` | Pillai 1.191899, F 53.466489 on 8 and 290; Wilks 0.023439, F 199.145344 on 8 and 288; −log₁₀ p 52.011345 and 111.864865 |
| 4 | Two discriminants, the flowers classified by them, and flower 71 measured as a new one | `mva.lda`, `mva.lda.predict` | svd 48.642644 and 4.579983; 3 of 150 misplaced; flower 71 virginica at 0.746772 |
| 5 | Petal length's two bandwidths, and its density under Sheather and Jones's | `stat.bw`, `stat.density` | nrd0 0.583233, SJ 0.191964; the trough at 2.5 cm, 0.000696 |

Every printed number of sections 1, 3, 4 and 5 is R 4.6.2's, to the six
places printed: `prcomp(x, scale. = TRUE)`, `summary(manova(x ~ Species))`
under Pillai and Wilks, MASS 7.3-66's `lda` and `predict.lda`, `bw.nrd0`,
`bw.SJ` and `density(…, bw = "SJ", from = 1, to = 7, n = 13)`. So are
section 2's tree and its cut (`hclust(dist(x), "average")`, `cutree(…, 3)`).
The k-means starts are Bund2's own generator's, so R's `kmeans` would draw
others and number the clusters in another order. Its best of 25 starts in
R reaches the same within-cluster sum of squares, 78.85144.

## Three idioms

- **A frame is the way in.** `frame.matrix` takes the named numeric
  columns as a MATRIX, a row per observation, which every `mva.`,
  `cluster.` and `stat.test.` word here reads; `frame.column` gives the
  labels as the LIST a grouped word takes. Its `na` option decides
  missing cells, so no word here meets a NONE.
- **A MATRIX is read through `linalg.mulv`.** A MATRIX does not print as a
  LIST does, and has no column word; multiplying it by a unit vector takes
  a column out as a LIST (`column` in the demo). `linalg.t` first takes a
  row.
- **Signs are fixed, not LAPACK's.** Every loading, score and
  discriminant has its largest element positive, so a run's signs never
  depend on the decomposition's internals. R's may differ in sign, never
  in magnitude.
