# RFC-0015 criterion 20's fixture for S16's fit.nls and fit.logistic, by
# mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I fit.py > fit.txt
#
# R is not installed where this was written, so each answer is computed here
# from the definition R's function documents, as infer.py's are, and solved
# to far beyond R's own tolerances:
#
# - nls: the least-squares estimate, by Gauss-Newton with the exact Jacobian;
#   se from s^2 (J'J)^-1 at the solution, s^2 = rss / (n - k).
# - glm(family = binomial): the maximum-likelihood estimate, by Newton's
#   method; se from the inverse Fisher information; z = coef / se; p the
#   two-sided normal tail; deviance -2 log L; aic deviance + 2k, for 0/1 y.
#
# The data are R's own. Puromycin's treated state (conc, rate), whose nls
# answer criterion 20 states as R prints it, and mtcars' (mpg, vs), whose
# glm(vs ~ mpg, family = binomial) R prints as coefficients -8.8331 and
# 0.4304, se 3.1623 and 0.1584, residual deviance 25.533, AIC 29.533.
import mpmath
from mpmath import mpf, matrix

mpmath.mp.dps = 50

CONC = [0.02, 0.02, 0.06, 0.06, 0.11, 0.11, 0.22, 0.22, 0.56, 0.56, 1.10, 1.10]
RATE = [76, 47, 97, 107, 123, 139, 159, 152, 191, 201, 207, 200]

MPG = [21.0, 21.0, 22.8, 21.4, 18.7, 18.1, 14.3, 24.4, 22.8, 19.2, 17.8, 16.4,
       17.3, 15.2, 10.4, 10.4, 14.7, 32.4, 30.4, 33.9, 21.5, 15.5, 15.2, 13.3,
       19.2, 27.3, 26.0, 30.4, 15.8, 19.7, 15.0, 21.4]
VS = [0, 0, 1, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0,
      0, 1, 0, 1, 0, 0, 0, 1]


def show(label, xs):
    print(label, "[" + " ".join(mpmath.nstr(x, 17, min_fixed=-30, max_fixed=30)
                                 for x in xs) + "]")


def nls():
    x = [mpf(str(c)) for c in CONC]
    y = [mpf(r) for r in RATE]
    vm, k = mpf(200), mpf("0.05")
    for _ in range(100):
        f = [vm * c / (k + c) for c in x]
        j = matrix([[c / (k + c), -vm * c / (k + c) ** 2] for c in x])
        r = matrix([yi - fi for yi, fi in zip(y, f)])
        step = mpmath.lu_solve(j.T * j, j.T * r)
        vm, k = vm + step[0], k + step[1]
    f = [vm * c / (k + c) for c in x]
    j = matrix([[c / (k + c), -vm * c / (k + c) ** 2] for c in x])
    rss = sum((yi - fi) ** 2 for yi, fi in zip(y, f))
    df = len(x) - 2
    cov = (j.T * j) ** -1 * (rss / df)
    show("nls.params", [vm, k])
    show("nls.se", [mpmath.sqrt(cov[0, 0]), mpmath.sqrt(cov[1, 1])])
    show("nls.rss", [rss])
    show("nls.df", [df])


def logistic():
    x = [mpf(str(m)) for m in MPG]
    y = [mpf(v) for v in VS]
    b = matrix([0, 0])
    for _ in range(100):
        eta = [b[0] + b[1] * xi for xi in x]
        mu = [1 / (1 + mpmath.exp(-e)) for e in eta]
        w = [m * (1 - m) for m in mu]
        info = matrix(2, 2)
        score = matrix(2, 1)
        for xi, yi, mi, wi in zip(x, y, mu, w):
            row = [1, xi]
            for a in range(2):
                score[a] += row[a] * (yi - mi)
                for c in range(2):
                    info[a, c] += row[a] * row[c] * wi
        b = b + mpmath.lu_solve(info, score)
    eta = [b[0] + b[1] * xi for xi in x]
    mu = [1 / (1 + mpmath.exp(-e)) for e in eta]
    info = matrix(2, 2)
    for xi, mi in zip(x, mu):
        row = [1, xi]
        for a in range(2):
            for c in range(2):
                info[a, c] += row[a] * row[c] * mi * (1 - mi)
    cov = info ** -1
    se = [mpmath.sqrt(cov[0, 0]), mpmath.sqrt(cov[1, 1])]
    z = [b[0] / se[0], b[1] / se[1]]
    p = [mpmath.erfc(abs(zi) / mpmath.sqrt(2)) for zi in z]
    dev = -2 * sum(yi * mpmath.log(mi) + (1 - yi) * mpmath.log(1 - mi)
                   for yi, mi in zip(y, mu))
    show("logistic.coef", [b[0], b[1]])
    show("logistic.se", se)
    show("logistic.z", z)
    show("logistic.p", p)
    show("logistic.deviance", [dev])
    show("logistic.aic", [dev + 4])


nls()
logistic()
