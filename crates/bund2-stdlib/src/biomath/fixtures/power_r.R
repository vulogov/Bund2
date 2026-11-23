# RFC-0019 criterion 4: the examples of R's help pages for power.t.test,
# power.prop.test and power.anova.test (src/library/stats/man/*.Rd, at
# R-4-4-2), each with the value R prints for it (Q67).
#
#     Rscript power_r.R > power_r.txt
#
# print.power.htest formats each value with format(x, digits = 7)
# (src/library/stats/R/power.R:150-159), which is what each line holds,
# after the Bund program for the same call and " | " and the key.
# Two examples have no Bund line: p2 = 1.0 is outside power.prop's domain
# (0, 1), and the search for p2 above 1 with n = 30, p1 = 0.9 is the
# extension §S4 does not make.
cat("# ", R.version.string, "\n", sep = "")
show <- function(program, key, x) cat(program, " | ", key, " ", format(x[[key]], digits = 7), "\n", sep = "")
show('dict "n" 20 set "delta" 1.0 set power.t', "power",
     power.t.test(n = 20, delta = 1))
show('dict "delta" 1.0 set "power" 0.9 set power.t', "n",
     power.t.test(power = .90, delta = 1))
show('dict "delta" 1.0 set "power" 0.9 set "sides" 1 set power.t', "n",
     power.t.test(power = .90, delta = 1, alternative = "one.sided"))
show('dict "n" 50 set "p1" 0.5 set "p2" 0.75 set power.prop', "power",
     power.prop.test(n = 50, p1 = .50, p2 = .75))
show('dict "p1" 0.5 set "p2" 0.75 set "power" 0.9 set power.prop', "n",
     power.prop.test(p1 = .50, p2 = .75, power = .90))
show('dict "n" 50 set "p1" 0.5 set "power" 0.9 set power.prop', "p2",
     power.prop.test(n = 50, p1 = .5, power = .90))
show('dict "n" 50 set "p1" 0.5 set "p2" 0.9 set "power" 0.9 set "sig.level" na set power.prop', "sig.level",
     power.prop.test(n = 50, p1 = .5, p2 = 0.9, power = .90, sig.level = NULL))
show('dict "p1" 0.5 set "p2" 0.501 set "sig.level" 0.001 set "power" 0.9 set power.prop', "n",
     power.prop.test(p1 = .5, p2 = 0.501, sig.level = .001, power = 0.90))
show('dict "groups" 4 set "n" 5 set "between.var" 1.0 set "within.var" 3.0 set power.anova', "power",
     power.anova.test(groups = 4, n = 5, between.var = 1, within.var = 3))
show('dict "groups" 4 set "between.var" 1.0 set "within.var" 3.0 set "power" 0.8 set power.anova', "n",
     power.anova.test(groups = 4, between.var = 1, within.var = 3, power = .80))
show('dict "groups" 4 set "between.var" 166.66666666666666 set "within.var" 500.0 set "power" 0.9 set power.anova', "n",
     power.anova.test(groups = 4, between.var = var(c(120, 130, 140, 150)), within.var = 500, power = .90))
