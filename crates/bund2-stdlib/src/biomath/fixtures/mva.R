# RFC-0020 criterion 3's R fixture: prcomp and cmdscale on USArrests,
# eurodist and the small cases of §Fixture inputs, with the inputs
# themselves, so that no test runs R (RFC-0020, "R is not a dependency").
#
#     Rscript mva.R > mva_r.txt
#
# Written on R 4.6.2. Each line is a label and a bracketed list, row-major
# for a matrix, at 17 significant digits. Vectors are R's as computed, with
# LAPACK's signs; the tests compare them up to each column's sign.

out <- function(label, x) {
    if (is.matrix(x)) x <- as.vector(t(x))
    cat(label, " [", paste(sprintf("%.17g", x), collapse = " "), "]\n", sep = "")
}

cat("# R", paste(R.version$major, R.version$minor, sep = "."), "\n")

x <- as.matrix(USArrests)
out("usarrests", x)
for (case in list(list("cs", TRUE, FALSE), list("cS", TRUE, TRUE), list("nS", FALSE, TRUE))) {
    p <- prcomp(x, center = case[[2]], scale. = case[[3]])
    out(paste0("pca.", case[[1]], ".sdev"), p$sdev)
    out(paste0("pca.", case[[1]], ".rotation"), p$rotation)
    out(paste0("pca.", case[[1]], ".x"), p$x)
    if (!isFALSE(p$center)) out(paste0("pca.", case[[1]], ".center"), p$center)
    if (!isFALSE(p$scale)) out(paste0("pca.", case[[1]], ".scale"), p$scale)
}
p <- prcomp(x, scale. = TRUE, rank. = 2)
out("pca.rank2.sdev", p$sdev)
out("pca.rank2.rotation", p$rotation)
p <- prcomp(x, scale. = TRUE, tol = 0.6)
out("pca.tol.sdev", p$sdev)
out("pca.tol.rotation", p$rotation)

d <- as.matrix(eurodist)
out("eurodist", d)
m <- cmdscale(eurodist, k = 2, eig = TRUE)
out("mds.euro.points", m$points)
out("mds.euro.eig", m$eig)
out("mds.euro.gof", m$GOF)

pts <- rbind(c(0, 0), c(1, 0), c(0, 1), c(1, 1), c(3, 2))
d <- as.matrix(dist(pts, "manhattan"))
out("manhattan", d)
m <- cmdscale(d, k = 2, eig = TRUE, add = TRUE)
out("mds.add.points", m$points)
out("mds.add.eig", m$eig)
out("mds.add.ac", m$ac)
out("mds.add.gof", m$GOF)

d <- matrix(c(0, 1, 1, 1, 0, 3, 1, 3, 0), 3)
out("few", d)
m <- suppressWarnings(cmdscale(d, k = 2, eig = TRUE))
out("mds.few.k", ncol(m$points))
out("mds.few.points", m$points)
out("mds.few.eig", m$eig)
