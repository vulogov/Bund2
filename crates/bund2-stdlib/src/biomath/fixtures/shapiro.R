# RFC-0019 criterion 6: Shapiro-Wilk's W and p, as R's shapiro.test answers
# them (N10, as amended: R >= 4.4; D266's note finds swilk.c, shapiro.test.R
# and qnorm.c byte-identical at R 4.4.2 and 4.6.2). The first line records
# R's version.
#
#     Rscript shapiro.R > shapiro.txt
#
# Each sample is written in the Bund program with six decimals, or in
# scientific form for the one whose range is below 1e-10 (shapiro.test.R:29
# rescales it), and R reads the same digits, so both see the same doubles.
# Each line is the program, " | ", the key and R's value to 17 digits.
cat("# ", R.version.string, "\n", sep = "")
show <- function(x, text = sprintf("%.6f", x)) {
  x <- as.numeric(text)
  program <- paste("[", paste(text, collapse = " "), "] stat.test.shapiro")
  r <- shapiro.test(x)
  cat(program, " | statistic ", sprintf("%.17g", r$statistic), "\n", sep = "")
  cat(program, " | p ", sprintf("%.17g", r$p.value), "\n", sep = "")
}
show(c(1, 2, 4))                          # n = 3, the exact p
show(c(2.1, 3.5, 1.2, 7.8))               # n = 4
show(c(5.1, 4.9, 6.2, 5.8, 6.0))          # n = 5
show(c(3.2, 1.1, 4.4, 2.0, 9.7, 3.3, 2.8, 3.0))         # n = 8
show(c(1, 1, 2, 2, 2, 3, 4, 8, 9, 15, 30))              # n = 11, ties
show(c(148, 154, 158, 160, 161, 162, 166, 170, 182, 195, 236, 240))  # n = 12
set.seed(20261123)
show(rnorm(20, 10, 2))
show(rexp(50))
show(runif(100))
show(rnorm(1000))
show(rt(5000, 3))                         # n = 5000, the largest
show(c(1, 2, 4.5, 3, 7) * 1e-12, sprintf("%.1fe-12", c(1, 2, 4.5, 3, 7)))
