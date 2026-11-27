# RFC-0020 criterion 7's R fixture: MASS's lda (method "moment") and
# predict.lda's plug-in method on iris, with the inputs, so that no test
# runs R (RFC-0020, "R is not a dependency").
#
#     Rscript lda.R > lda_r.txt
#
# Written on R 4.6.2 with MASS 7.3-66. Each line is a label and a bracketed
# list, row-major for a matrix, at 17 significant digits; a matrix's column
# count is the test's to know. A case where R stops writes its message under
# `.stop`.

library(MASS)

out <- function(label, x) {
    if (is.matrix(x)) x <- as.vector(t(x))
    cat(label, " [", paste(sprintf("%.17g", x), collapse = " "), "]\n", sep = "")
}

fit_out <- function(label, f) {
    out(paste0(label, ".prior"), unname(f$prior))
    out(paste0(label, ".counts"), unname(f$counts))
    out(paste0(label, ".means"), unname(f$means))
    out(paste0(label, ".scaling"), unname(f$scaling))
    out(paste0(label, ".svd"), f$svd)
    out(paste0(label, ".lev"), as.integer(f$lev))
}

pred_out <- function(label, p) {
    out(paste0(label, ".class"), as.integer(as.character(p$class)))
    out(paste0(label, ".posterior"), unname(p$posterior))
    out(paste0(label, ".x"), unname(p$x))
}

cat("# R", paste(R.version$major, R.version$minor, sep = "."),
    "MASS", as.character(packageVersion("MASS")), "\n")

x <- as.matrix(iris[, 1:4])
g <- as.integer(iris$Species)
out("iris", x)
out("iris.groups", g)
rows <- c(1, 51, 101, 71)
out("rows", rows)

f <- lda(x, g)
fit_out("fit", f)
pred_out("pred", predict(f, x[rows, ]))
pred_out("pred.dimen1", predict(f, x[rows, ], dimen = 1))
pred_out("pred.prior0", predict(f, x[rows, ], prior = c(0, 0.5, 0.5)))

# A non-uniform prior, then the same prior under reversed levels.
pri <- c(0.5, 0.3, 0.2)
fp <- lda(x, g, prior = pri)
fit_out("fitp", fp)
pred_out("predp", predict(fp, x[rows, ]))
fr <- lda(x, factor(g, levels = 3:1), prior = rev(pri))
fit_out("fitr", fr)
pred_out("predr", predict(fr, x[rows, ]))

# A variable constant within groups: column 3 is its group's number.
xc <- x
xc[, 3] <- g
r <- tryCatch(lda(xc, g), error = function(e) conditionMessage(e))
cat("constant.stop ", r, "\n", sep = "")

# Collinear: column 4 is column 1 + column 2, so three discriminants of
# full rank become fewer; R warns "variables are collinear".
xl <- x
xl[, 4] <- xl[, 1] + xl[, 2]
fl <- suppressWarnings(lda(xl, g))
fit_out("collinear", fl)
pred_out("collinear.pred", predict(fl, xl[rows, ]))
