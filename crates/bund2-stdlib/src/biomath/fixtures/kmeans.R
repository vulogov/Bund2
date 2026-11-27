# RFC-0020 criterion 5's R fixture: kmeans under each algorithm, given its
# centres as a MATRIX, on an input with ties and on two inputs that take
# more than one transfer pass, with the inputs themselves, so that no test
# runs R (RFC-0020, "R is not a dependency").
#
#     Rscript kmeans.R > kmeans_r.txt
#
# Written on R 4.6.2. Each line is a label and a bracketed list, row-major
# for a matrix, at 17 significant digits; a matrix's column count is the
# test's to know. A run that warns writes its warnings under `.warn`, and
# the fixture is chosen so that none does.

out <- function(label, x) {
    if (is.matrix(x)) x <- as.vector(t(x))
    cat(label, " [", paste(sprintf("%.17g", x), collapse = " "), "]\n", sep = "")
}

cat("# R", paste(R.version$major, R.version$minor, sep = "."), "\n")

# tied: nine points on a line, centres (1, 0), (3, 0), (5, 0), so that
# (2, 0) is equidistant from the first two and (4, 0) from the last two.
tied <- cbind(0:8, 0)
# usarrests: all of USArrests, centres rows 1, 10, 20, 30, 40.
usarrests <- as.matrix(USArrests)
# sim: three groups of 30 in the plane, centres rows 1, 2, 3, all of the
# first group, so that most points move.
set.seed(42)
sim <- rbind(cbind(rnorm(30, 0), rnorm(30, 0)),
             cbind(rnorm(30, 4), rnorm(30, 0)),
             cbind(rnorm(30, 2), rnorm(30, 3)))
inputs <- list(
    tied = list(x = tied, centres = tied[c(2, 4, 6), ]),
    usarrests = list(x = usarrests, centres = usarrests[c(1, 10, 20, 30, 40), ]),
    sim = list(x = sim, centres = sim[1:3, ])
)
ALGORITHMS <- c("Hartigan-Wong", "Lloyd", "MacQueen")
for (name in names(inputs)) {
    x <- inputs[[name]]$x
    c0 <- inputs[[name]]$centres
    out(paste0(name, ".x"), x)
    out(paste0(name, ".centres"), c0)
    for (a in ALGORITHMS) {
        warns <- character()
        z <- withCallingHandlers(
            kmeans(x, c0, algorithm = a),
            warning = function(w) {
                warns <<- c(warns, conditionMessage(w))
                invokeRestart("muffleWarning")
            })
        l <- paste0(name, ".", a)
        out(paste0(l, ".cluster"), z$cluster)
        out(paste0(l, ".centers"), z$centers)
        out(paste0(l, ".totss"), z$totss)
        out(paste0(l, ".withinss"), z$withinss)
        out(paste0(l, ".size"), z$size)
        out(paste0(l, ".iter"), z$iter)
        if (length(warns)) cat(l, ".warn ", paste(warns, collapse = "; "), "\n", sep = "")
    }
}
