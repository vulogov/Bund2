# RFC-0019 criterion 12's fixture for §S11: fit.mle's profile intervals,
# by the chi-squared inversion of the likelihood ratio, with mpmath 1.3.0
# at 50 digits.
#
#     python3 -I mle.py > mle.txt
#
# chi2_1(conf) = qnorm((1 + conf)/2)^2. Each end solves
# 2 (l_hat - l_p(theta)) = chi2_1(conf), found by mpmath's findroot from
# either side of the estimate.
#
# - "e": an exponential sample's rate, l(r) = n ln r - r S, the estimate
#   n/S; one parameter, so l_p is l.
# - "m": a normal sample's mean with sigma profiled out:
#   l_p(mu) = -n/2 ln(s2(mu)), s2(mu) = sum (x - mu)^2 / n, so
#   2 (l_hat - l_p(mu)) = n ln(s2(mu)/s2_hat) and the ends are
#   xbar -+ sqrt(s2_hat (exp(chi2/n) - 1)); the closed form is printed
#   beside the root as a check.
#
# Each line is `case key value`, 17 digits; data are written as given and
# read as doubles.

from mpmath import mp, mpf, sqrt, erfinv, log, exp, findroot

mp.dps = 50

EXP = ["0.8", "1.9", "0.4", "2.7", "1.1", "0.3", "1.6", "0.9", "2.2", "0.6"]
NORM = ["4.1", "5.3", "3.8", "6.0", "4.9", "5.5", "4.4", "5.1", "3.6", "5.8", "4.7", "5.0"]


def x(s):
    return mpf(float(s))


def chi2(conf):
    z = sqrt(2) * erfinv(conf)
    return z * z


def out(case, key, v):
    print(f"{case} {key} {mp.nstr(v, 17, min_fixed=-30, max_fixed=30)}")


for conf_s in ["0.95", "0.9"]:
    conf = x(conf_s)
    c = chi2(conf)

    xs = [x(v) for v in EXP]
    n, s = len(xs), sum(xs)
    rate = n / s
    l = lambda r: n * log(r) - r * s
    g = lambda r: 2 * (l(rate) - l(r)) - c
    case = f"e@{conf_s}"
    if conf_s == "0.95":
        print(f"e data [ {' '.join(EXP)} ]")
        out("e", "rate", rate)
    out(case, "lower", findroot(g, rate * mpf("0.5")))
    out(case, "upper", findroot(g, rate * mpf("1.5")))

    xs = [x(v) for v in NORM]
    n = len(xs)
    xbar = sum(xs) / n
    s2 = lambda mu: sum((v - mu) ** 2 for v in xs) / n
    gm = lambda mu: n * log(s2(mu) / s2(xbar)) - c
    case = f"m@{conf_s}"
    if conf_s == "0.95":
        print(f"m data [ {' '.join(NORM)} ]")
        out("m", "mean", xbar)
        out("m", "sigma", sqrt(s2(xbar)))
    half = sqrt(s2(xbar) * (exp(c / n) - 1))
    lo, hi = findroot(gm, xbar - 1), findroot(gm, xbar + 1)
    assert abs(lo - (xbar - half)) < mpf("1e-40") and abs(hi - (xbar + half)) < mpf("1e-40")
    out(case, "lower", lo)
    out(case, "upper", hi)
