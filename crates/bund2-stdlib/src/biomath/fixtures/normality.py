# RFC-0019 criterion 6's fixture for the one-sample Kolmogorov-Smirnov,
# Bartlett and Levene tests, from their definitions with mpmath 1.3.0 at 60
# digits.
#
#     python3 -I normality.py > normality.txt
#
# KS: D = max over the sorted x of F(x_i) - (i-1)/n and i/n - F(x_i). The
# exact p is 1 - P(D < d) by Marsaglia, Tsang and Wang's matrix power (2003),
# computed here in mpmath at 60 digits, without ks.c's 1e140 rescaling; the
# asymptotic p, for n >= 100 or ties, is Kolmogorov's limit
# 2 sum (-1)^(k-1) exp(-2 k^2 n d^2). Bartlett: bartlett.test.R's K^2 on
# k - 1, p by the chi-squared upper tail. Levene: the one-way F on
# |x - centre| (the group's median, or its mean), p by F's upper tail.
# Each line is a Bund program, " | ", a key and its value to 17 digits.
import mpmath
from mpmath import mpf

mpmath.mp.dps = 60


def show(x):
    return mpmath.nstr(x, 17, min_fixed=-1, max_fixed=-1)


def f(s):
    return mpf(float(s))


def line(program, key, value):
    print(f"{program} | {key} {show(value) if not isinstance(value, str) else value}")


def chisq_sf(x, k):
    return mpmath.gammainc(k / 2, x / 2, mpmath.inf, regularized=True)


def f_sf(x, d1, d2):
    return mpmath.betainc(d2 / 2, d1 / 2, 0, d2 / (d2 + d1 * x), regularized=True)


# KS.

def mtw(n, d):
    k = int(n * d) + 1
    m = 2 * k - 1
    h = k - n * d
    H = mpmath.matrix(m, m)
    for i in range(m):
        for j in range(m):
            H[i, j] = 0 if i - j + 1 < 0 else 1
    for i in range(m):
        H[i, 0] -= h ** (i + 1)
        H[m - 1, i] -= h ** (m - i)
    H[m - 1, 0] += (2 * h - 1) ** m if 2 * h - 1 > 0 else 0
    for i in range(m):
        for j in range(m):
            if i - j + 1 > 0:
                H[i, j] /= mpmath.factorial(i - j + 1)
    Q = H ** n
    s = Q[k - 1, k - 1]
    return s * mpmath.factorial(n) / mpf(n) ** n


def kolmogorov(x):
    return 2 * mpmath.nsum(lambda k: (-1) ** (k - 1) * mpmath.exp(-2 * k * k * x * x), [1, mpmath.inf])


def normal_cdf(m, s):
    return lambda x: mpmath.ncdf(x, m, s)


def exp_cdf(r):
    return lambda x: 1 - mpmath.exp(-r * x) if x > 0 else mpf(0)


def unif_cdf(a, b):
    return lambda x: min(max((x - a) / (b - a), mpf(0)), mpf(1))


def ks(texts, dist_src, cdf):
    xs = sorted(f(t) for t in texts)
    n = len(xs)
    d = max(max(cdf(x) - mpf(i) / n, mpf(i + 1) / n - cdf(x)) for i, x in enumerate(xs))
    ties = len(set(xs)) < n
    exact = n < 100 and not ties
    p = 1 - mtw(n, d) if exact else kolmogorov(mpmath.sqrt(n) * d)
    program = f"[ {' '.join(texts)} ] {dist_src} stat.test.ks1"
    line(program, "statistic", d)
    line(program, "p", p)


ks(["0.61", "-1.20", "0.33", "1.95", "-0.44", "0.08", "-2.10", "1.10", "0.72", "-0.05"],
   "0.0 1.0 dist.normal", normal_cdf(0, 1))
ks(["0.12", "0.45", "1.70", "0.03", "2.80", "0.66", "0.91", "0.24", "3.90", "1.15",
    "0.38", "0.07", "1.42", "0.55", "0.19", "2.05", "0.81", "0.31", "1.01", "0.49"],
   "1.5 dist.exponential", exp_cdf(f("1.5")))
ks(["0.9", "0.85", "0.7", "0.95", "0.6"], "0.0 1.0 dist.uniform", unif_cdf(0, 1))
# Ties: Kolmogorov's limit, though n < 100.
ks(["0.1", "0.2", "0.2", "0.5", "0.7", "0.9"], "0.0 1.0 dist.uniform", unif_cdf(0, 1))
# n = 120 >= 100: the limit.
ks([f"{(i + 0.5) / 120 * 1.1:.6f}" for i in range(120)], "0.0 1.0 dist.uniform",
   unif_cdf(0, 1))

# Bartlett and Levene.

GROUPS = [
    [["4.2", "4.8", "5.1", "3.9", "4.4"], ["6.1", "5.2", "7.9", "4.8", "6.6", "5.5"],
     ["3.0", "3.4", "2.9", "3.3"]],
    [["10.0", "12.5", "9.1", "11.3"], ["8.8", "15.2", "4.1", "13.9", "7.7"]],
    [["1", "2", "3", "4", "5", "6"], ["2", "4", "6", "8", "10", "12"],
     ["1", "1", "2", "3", "5", "8"], ["0.5", "0.6", "0.9", "1.0", "1.2", "1.3"]],
]


def literal(groups):
    return "[ " + " ".join("[ " + " ".join(g) + " ]" for g in groups) + " ]"


def var(xs):
    m = sum(xs) / len(xs)
    return sum((x - m) ** 2 for x in xs) / (len(xs) - 1)


def median(xs):
    s = sorted(xs)
    n = len(s)
    return s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2


for gs in GROUPS:
    g = [[f(t) for t in grp] for grp in gs]
    k = len(g)
    n = [len(x) - 1 for x in g]
    v = [var(x) for x in g]
    nt = sum(n)
    vt = sum(ni * vi for ni, vi in zip(n, v)) / nt
    stat = (nt * mpmath.log(vt) - sum(ni * mpmath.log(vi) for ni, vi in zip(n, v))) / (
        1 + (sum(mpf(1) / ni for ni in n) - mpf(1) / nt) / (3 * (k - 1)))
    program = f"{literal(gs)} stat.test.bartlett"
    line(program, "statistic", stat)
    line(program, "p", chisq_sf(stat, k - 1))
    line(program, "df", mpf(k - 1))
    for centre, suffix in [("median", "stat.test.levene"),
                           ("median", 'dict "center" "median" set stat.test.levene.opts'),
                           ("mean", 'dict "center" "mean" set stat.test.levene.opts')]:
        z = [[abs(x - (median(grp) if centre == "median" else sum(grp) / len(grp))) for x in grp]
             for grp in g]
        N = sum(len(zi) for zi in z)
        grand = sum(sum(zi) for zi in z) / N
        means = [sum(zi) / len(zi) for zi in z]
        between = sum(len(zi) * (m - grand) ** 2 for zi, m in zip(z, means))
        within = sum(sum((x - m) ** 2 for x in zi) for zi, m in zip(z, means))
        d1, d2 = k - 1, N - k
        F = (between / d1) / (within / d2)
        program = f"{literal(gs)} {suffix}"
        line(program, "statistic", F)
        line(program, "p", f_sf(F, d1, d2))
        line(program, "df1", mpf(d1))
        line(program, "df2", mpf(d2))
