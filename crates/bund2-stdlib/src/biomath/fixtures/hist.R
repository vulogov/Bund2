# RFC-0020 criterion 8's R fixture: hist(..., plot = FALSE), pretty and ecdf
# on the inputs of §Fixture inputs, with the inputs, so that no test runs R
# (RFC-0020, "R is not a dependency").
#
#     Rscript hist.R > hist_r.txt
#
# Written on R 4.6.2. Each line is a label and a bracketed list at 17
# significant digits; a BOOL is 1 or 0. A case where R stops writes its
# message under `.stop`.

out <- function(label, x) {
    cat(label, " [", paste(sprintf("%.17g", as.numeric(x)), collapse = " "), "]\n", sep = "")
}

h_out <- function(label, x, ...) {
    r <- tryCatch({
        h <- suppressWarnings(hist(x, plot = FALSE, ...))
        out(paste0(label, ".breaks"), h$breaks)
        out(paste0(label, ".counts"), h$counts)
        out(paste0(label, ".density"), h$density)
        out(paste0(label, ".mids"), h$mids)
        out(paste0(label, ".equidist"), h$equidist)
    }, error = function(e) cat(label, ".stop ", conditionMessage(e), "\n", sep = ""))
}

cat("# R", paste(R.version$major, R.version$minor, sep = "."), "\n")

x10 <- c(1.2, 2.5, 2.5, 3.1, 4.8, 5.0, 7.3, 8.8, 9.1, 10.4)
set.seed(1)
x1000 <- rnorm(1000)
tied <- c(1, 2, 2, 3, 3, 3, 4)
# FD's IQR is 0 here, so its fallback quantiles decide the width.
spike <- c(rep(5, 12), 1, 9, 20)
out("x10", x10)
out("x1000", x1000)
out("tied", tied)
out("spike", spike)

for (rule in c("Sturges", "Scott", "FD")) {
    lab <- tolower(rule)
    h_out(paste0("x10.", lab), x10, breaks = rule)
    h_out(paste0("x1000.", lab), x1000, breaks = rule)
    h_out(paste0("spike.", lab), spike, breaks = rule)
}
for (k in c(1, 3, 7, 25)) {
    h_out(paste0("x10.count", k), x10, breaks = k)
    h_out(paste0("x1000.count", k), x1000, breaks = k)
}
h_out("x10.list", x10, breaks = c(0, 2.5, 5, 7.5, 11))
h_out("x10.uneven", x10, breaks = c(1, 2.5, 3, 9.1, 10.4))
h_out("x10.uneven.left", x10, breaks = c(1, 2.5, 3, 9.1, 10.4), right = FALSE)
h_out("x10.short", x10, breaks = c(2, 5, 9))

h_out("tied.right", tied)
h_out("tied.left", tied, right = FALSE)
tb <- c(1, 2, 3, 4)
h_out("tied.list.right", tied, breaks = tb)
h_out("tied.list.left", tied, breaks = tb, right = FALSE)
h_out("tied.list.right.open", tied, breaks = tb, include.lowest = FALSE)
h_out("tied.list.left.open", tied, breaks = tb, right = FALSE, include.lowest = FALSE)

h_out("constant", c(5, 5, 5))
h_out("single", 7)
h_out("single.scott", 7, breaks = "Scott")
fz <- c(0.05, 0.08, 0.15, 0.30000000000000004, 0.5)
out("fuzz", fz)
h_out("fuzz", fz, breaks = c(0, 0.1, 0.2, 0.3, 1))

# pretty's cases (§Fixture inputs), each with R's own n and min.n defaults.
p_out <- function(label, x, n = 5L) {
    out(paste0("pretty.", label, ".in"), c(range(x), n, n %/% 3L))
    out(paste0("pretty.", label), pretty(x, n = n))
}
p_out("a", c(0.13, 9.71))
p_out("a10", c(0.13, 9.71), 10L)
p_out("ones", c(1, 1))
p_out("zeros", c(0, 0))
p_out("wide", c(-3.3, 1e5), 4L)
p_out("tiny", c(1e-300, 2e-300))
p_out("close", c(123456789, 123456789.5))
p_out("neg", c(-7.2, -0.4))
p_out("huge", c(-1e308, 1e308))

# ecdf: knots and values, and the step function at points between, on and
# beyond them.
e <- ecdf(tied)
out("ecdf.tied.x", knots(e))
out("ecdf.tied.y", e(knots(e)))
at <- c(0, 1, 1.5, 2, 2.999, 3, 4, 10)
out("ecdf.tied.at", at)
out("ecdf.tied.at.y", e(at))
e10 <- ecdf(x10)
out("ecdf.x10.x", knots(e10))
out("ecdf.x10.y", e10(knots(e10)))
r <- tryCatch(ecdf(numeric()), error = function(e) conditionMessage(e))
cat("ecdf.empty.stop ", r, "\n", sep = "")
