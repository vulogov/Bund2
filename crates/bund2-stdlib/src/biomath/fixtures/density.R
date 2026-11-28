# RFC-0020 criterion 9's R fixture: bw.nrd0, bw.nrd, bw.SJ (both methods)
# and density on the inputs of §Fixture inputs, with the inputs, so that no
# test runs R (RFC-0020, "R is not a dependency").
#
#     Rscript density.R > density_r.txt
#
# Written on R 4.6.2. Each line is a label and a bracketed list at 17
# significant digits. A case where R stops writes its message under
# `.stop`.

out <- function(label, x) {
    cat(label, " [", paste(sprintf("%.17g", as.numeric(x)), collapse = " "), "]\n", sep = "")
}

try_out <- function(label, expr) {
    tryCatch(out(label, expr),
             error = function(e) cat(label, ".stop ", conditionMessage(e), "\n", sep = ""))
}

# The grid x depends on bw, n, from and to, not the kernel: written once.
d_out <- function(label, x, ..., grid = TRUE) {
    tryCatch({
        d <- density(x, ...)
        if (grid) out(paste0(label, ".x"), d$x)
        out(paste0(label, ".y"), d$y)
        out(paste0(label, ".bw"), d$bw)
        out(paste0(label, ".n"), d$n)
    }, error = function(e) cat(label, ".stop ", conditionMessage(e), "\n", sep = ""))
}

cat("# R", paste(R.version$major, R.version$minor, sep = "."), "\n")

x10 <- c(1.2, 2.5, 2.5, 3.1, 4.8, 5.0, 7.3, 8.8, 9.1, 10.4)
set.seed(1)
x1000 <- rnorm(1000)
# Either side of bw.SJ's n > nb/2 switch to binned counts.
x500 <- x1000[1:500]
x501 <- x1000[1:501]
# IQR 0 with sd > 0: nrd0 falls back to sd, nrd answers 0.
spike <- c(rep(5, 12), 1, 9, 20)
const <- c(5, 5, 5)
zeros <- c(0, 0)
# A first value of 0 with sd 0 falls through to 1 (`bandwidths.R:28`).
out("x10", x10)
out("x1000", x1000)
out("spike", spike)
out("const", const)
out("zeros", zeros)

for (s in c("x10", "x1000", "x500", "x501", "spike")) {
    x <- get(s)
    try_out(paste0("bw.nrd0.", s), bw.nrd0(x))
    try_out(paste0("bw.nrd.", s), bw.nrd(x))
    try_out(paste0("bw.ste.", s), bw.SJ(x, method = "ste"))
    try_out(paste0("bw.dpi.", s), bw.SJ(x, method = "dpi"))
}
try_out("bw.nrd0.const", bw.nrd0(const))
try_out("bw.nrd0.zeros", bw.nrd0(zeros))
try_out("bw.ste.const", bw.SJ(const, method = "ste"))
try_out("bw.dpi.const", bw.SJ(c(rep(5, 600)), method = "dpi"))
try_out("bw.nrd0.one", bw.nrd0(7))

kernels <- c("gaussian", "epanechnikov", "rectangular", "triangular",
             "biweight", "cosine", "optcosine")
for (k in kernels) {
    g <- k == "gaussian"
    d_out(paste0("d.x10.", k), x10, kernel = k, grid = g)
    d_out(paste0("d.x1000.", k), x1000, kernel = k, grid = g)
    d_out(paste0("d.x10.", k, ".given"), x10, kernel = k, n = 60, from = 0, to = 12, grid = g)
    d_out(paste0("d.x1000.", k, ".given"), x1000, kernel = k, n = 40, from = -3, to = 3, grid = g)
}
d_out("d.x10.sj", x10, bw = "SJ", n = 50)
d_out("d.x10.dpi", x10, bw = "sj-dpi", n = 50)
d_out("d.x10.nrd", x10, bw = "nrd", n = 50)
d_out("d.x10.adjust", x10, bw = 0.7, adjust = 2, cut = 1, n = 50)
d_out("d.x10.n1", x10, n = 1)
d_out("d.one", 7, bw = 0.5, n = 50)
d_out("d.one.rule", 7)
