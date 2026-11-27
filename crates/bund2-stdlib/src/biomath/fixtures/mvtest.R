# RFC-0020 criterion 6's R fixture: Hotelling's T² by summary.manova's
# Hotelling-Lawley route, and one-way MANOVA under each test, on the inputs
# of §Fixture inputs, with the inputs themselves, so that no test runs R
# (RFC-0020, "R is not a dependency").
#
#     Rscript mvtest.R > mvtest_r.txt
#
# Written on R 4.6.2. Each line is a label and a bracketed list, row-major
# for a matrix, at 17 significant digits; a matrix's column count is the
# test's to know. A case where R stops writes its message under `.stop`.

out <- function(label, x) {
    if (is.matrix(x)) x <- as.vector(t(x))
    cat(label, " [", paste(sprintf("%.17g", x), collapse = " "), "]\n", sep = "")
}

cat("# R", paste(R.version$major, R.version$minor, sep = "."), "\n")

TESTS <- c("Pillai", "Wilks", "Hotelling-Lawley", "Roy")

# One row of summary.manova's table: statistic, F, df1, df2, p; and the
# eigenvalues.
manova_row <- function(label, x, g, test, intercept = FALSE) {
    r <- tryCatch({
        s <- if (is.null(g)) summary(manova(x ~ 1), test = test, intercept = TRUE)
             else summary(manova(x ~ g), test = test)
        st <- s$stats[1, ]
        out(paste0(label, ".stat"), unname(st[2:6]))
        out(paste0(label, ".eig"), unname(s$Eigenvalues[1, ]))
    }, error = function(e) cat(label, ".stop ", conditionMessage(e), "\n", sep = ""))
}

versicolor <- as.matrix(iris[51:100, 1:4])
virginica <- as.matrix(iris[101:150, 1:4])
mu <- c(6, 2.8, 4.2, 1.3)
out("versicolor", versicolor)
out("virginica", virginica)
out("mu", mu)

# One-sample: the intercept of x - mu.
manova_row("h1", sweep(versicolor, 2, mu), NULL, "Hotelling-Lawley")
# Two-sample: two groups; T² is (n1 + n2 - 2) times the statistic.
two <- rbind(versicolor, virginica)
manova_row("h2", two, factor(rep(1:2, each = 50)), "Hotelling-Lawley")

# Three groups, all of iris.
x3 <- as.matrix(iris[, 1:4])
g3 <- as.integer(iris$Species)
out("iris", x3)
out("iris.groups", g3)
for (t in TESTS) manova_row(paste0("m3.", t), x3, factor(g3), t)

# Rank-deficient E: column 4 is column 1 + column 2.
xd <- x3
xd[, 4] <- xd[, 1] + xd[, 2]
manova_row("deficient", xd, factor(g3), "Pillai")

# The near-collinear pair: one draw of a and of the noise, scaled by eps.
set.seed(3)
a <- rnorm(20)
e <- rnorm(20)
gn <- rep(1:2, each = 10)
for (eps in c("1e-3", "1e-4")) {
    xn <- cbind(a + gn, a + as.numeric(eps) * e + gn)
    out(paste0("near.", eps), xn)
    for (t in TESTS) manova_row(paste0("near.", eps, ".", t), xn, factor(gn), t)
}

# Zero df: N - g = p.
xz <- rbind(c(1, 2), c(2, 1), c(3, 5), c(4, 3), c(6, 2))
gz <- c(1, 1, 2, 2, 3)
out("zero", xz)
for (t in TESTS) manova_row(paste0("zero.", t), xz, factor(gz), t)
