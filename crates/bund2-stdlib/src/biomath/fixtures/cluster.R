# RFC-0020 criterion 4's R fixture: dist, hclust and cutree on the inputs
# of §Fixture inputs and criterion 4, with the inputs themselves, so that
# no test runs R (RFC-0020, "R is not a dependency").
#
#     Rscript cluster.R > cluster_r.txt
#
# Written on R 4.6.2. Each line is a label and a bracketed list, row-major
# for a matrix, at 17 significant digits; a matrix's line is preceded by
# nothing else, its column count is the test's to know.

out <- function(label, x) {
    if (is.matrix(x)) x <- as.vector(t(x))
    cat(label, " [", paste(sprintf("%.17g", x), collapse = " "), "]\n", sep = "")
}

cat("# R", paste(R.version$major, R.version$minor, sep = "."), "\n")

METHODS <- c("ward.D", "single", "complete", "average", "mcquitty",
             "median", "centroid", "ward.D2")

# Three inputs: tied (criterion 4's 0, 1, 2, 3, 10, 11), untied (the
# first twelve rows of USArrests) and an inversion under centroid (the
# squared Euclidean distances of (0, 0), (2, 0), (1, 1.8), and a fourth
# point (5, 5) so that the inversion is not the last merge).
inputs <- list(
    tied = as.matrix(dist(c(0, 1, 2, 3, 10, 11))),
    untied = as.matrix(dist(as.matrix(USArrests)[1:12, ])),
    inversion = as.matrix(dist(rbind(c(0, 0), c(2, 0), c(1, 1.8), c(5, 5))))^2
)
for (name in names(inputs)) {
    d <- inputs[[name]]
    n <- nrow(d)
    out(name, d)
    for (m in METHODS) {
        h <- hclust(as.dist(d), m)
        out(paste0("hc.", name, ".", m, ".merge"), h$merge)
        out(paste0("hc.", name, ".", m, ".height"), h$height)
        out(paste0("hc.", name, ".", m, ".order"), h$order)
        out(paste0("cut.", name, ".", m, ".k"), cutree(h, k = 1:n))
        if (!is.unsorted(h$height)) {
            hs <- sort(unique(c(h$height, (head(h$height, -1) + tail(h$height, -1)) / 2,
                                min(h$height) - 1, max(h$height) + 1)))
            out(paste0("cut.", name, ".", m, ".hs"), hs)
            out(paste0("cut.", name, ".", m, ".h"), cutree(h, h = hs))
        }
    }
}

# members, on the untied input.
members <- c(1, 2, 3, 1, 1, 2, 4, 1, 1, 2, 1, 3)
out("members", members)
for (m in METHODS) {
    h <- hclust(as.dist(inputs$untied), m, members = members)
    out(paste0("hc.members.", m, ".merge"), h$merge)
    out(paste0("hc.members.", m, ".height"), h$height)
    out(paste0("hc.members.", m, ".order"), h$order)
}

# dist on an input with zero coordinates: Canberra's rescaled pair
# (0, 1), (0, 3) as rows 1 and 2, binary's all-zero pair as rows 3 and 4
# (Canberra's all-zero pair is the test's refusal, and is not here).
x <- rbind(c(0, 1, 0), c(0, 3, 0), c(0, 0, 0), c(0, 0, 0), c(1.5, -2, 4), c(-1, 0, 2.5))
out("x", x)
for (m in c("euclidean", "maximum", "manhattan", "binary")) {
    out(paste0("dist.", m), as.matrix(dist(x, m)))
}
out("dist.minkowski.3", as.matrix(dist(x, "minkowski", p = 3)))
out("dist.minkowski.0.5", as.matrix(dist(x, "minkowski", p = 0.5)))
# Canberra without the all-zero pair: rows 1, 2, 3, 5, 6.
xc <- x[c(1, 2, 3, 5, 6), ]
out("xc", xc)
out("dist.canberra", as.matrix(dist(xc, "canberra")))
out("canberra.pair", dist(rbind(c(0, 1), c(0, 3)), "canberra"))
