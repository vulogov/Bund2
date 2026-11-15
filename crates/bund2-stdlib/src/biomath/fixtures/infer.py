# RFC-0015 criterion 9's fixture for S6's tests, corrections and stat.lm,
# computed with mpmath 1.3.0 at 50 significant digits and exact rationals.
#
#     python3 -I infer.py > infer.txt
#
# R is not installed where this was written, so each answer is computed
# here from the definition R's function documents: t.test, chisq.test
# (correct = FALSE), fisher.test, binom.test, wilcox.test (exact = FALSE,
# correct = TRUE), ks.test (exact = FALSE), p.adjust and lm. Each line is a
# label, a field and its value to 17 significant digits; the test holds the
# Bund program for each label. The data are R's own `sleep` (extra, by
# group) and the examples in R's help for chisq.test, fisher.test and
# binom.test, so the printed R answers can be checked against the same
# lines.
from fractions import Fraction as Q

import mpmath
from mpmath import mpf

mpmath.mp.dps = 50


def show(x):
    return mpmath.nstr(mpf(x), 17, min_fixed=-1, max_fixed=-1)


def out(label, field, value):
    print(label, field, show(value))


def f(xs):
    return [mpf(float(x)) for x in xs]


def mean(xs):
    return mpmath.fsum(xs) / len(xs)


def var(xs):
    m = mean(xs)
    return mpmath.fsum((x - m) ** 2 for x in xs) / (len(xs) - 1)


def t_p(t, df):
    # Two-sided: 2 P(T <= -|t|) = I_{df/(df+t^2)}(df/2, 1/2).
    return mpmath.betainc(df / 2, mpf(1) / 2, 0, df / (df + t * t), regularized=True)


def chisq_p(x, df):
    return mpmath.gammainc(mpf(df) / 2, x / 2, mpmath.inf, regularized=True)


A = ["0.7", "-1.6", "-0.2", "-1.2", "-0.1", "3.4", "3.7", "0.8", "0.0", "2.0"]
B = ["1.9", "0.8", "1.1", "0.1", "-0.1", "4.4", "5.5", "1.6", "4.6", "3.4"]
a, b = f(A), f(B)

# t tests.
mu = mpf(0.5)
t = (mean(a) - mu) / mpmath.sqrt(var(a) / len(a))
out("t1", "statistic", t)
out("t1", "p", t_p(t, mpf(9)))

df = mpf(18)
pooled = (9 * var(a) + 9 * var(b)) / df
t = (mean(a) - mean(b)) / mpmath.sqrt(pooled * (mpf(1) / 10 + mpf(1) / 10))
out("t2", "statistic", t)
out("t2", "p", t_p(t, df))

v1, v2 = var(a) / 10, var(b) / 10
df = (v1 + v2) ** 2 / (v1**2 / 9 + v2**2 / 9)
t = (mean(a) - mean(b)) / mpmath.sqrt(v1 + v2)
out("welch", "statistic", t)
out("welch", "df", df)
out("welch", "p", t_p(t, df))

d = [x - y for x, y in zip(a, b)]
t = mean(d) / mpmath.sqrt(var(d) / len(d))
out("paired", "statistic", t)
out("paired", "p", t_p(t, mpf(9)))

# Chi-squared, goodness of fit: ?chisq.test's example with probabilities,
# and the same observations against expected counts.
obs = f([89, 37, 30, 28, 2])
prob = f(["0.40", "0.20", "0.20", "0.19", "0.01"])
n = mpmath.fsum(obs)
x2 = mpmath.fsum((o - n * p) ** 2 / (n * p) for o, p in zip(obs, prob))
out("chisq", "statistic", x2)
out("chisq", "p", chisq_p(x2, 4))
counts = f(["74.8", "37.4", "37.4", "35.53", "1.87"])
x2 = mpmath.fsum((o - e) ** 2 / e for o, e in zip(obs, counts))
out("chisq.counts", "statistic", x2)
out("chisq.counts", "p", chisq_p(x2, 4))

# Chi-squared on ?chisq.test's contingency table.
M = [[762, 327, 468], [484, 239, 477]]
rows = [sum(r) for r in M]
cols = [sum(r[j] for r in M) for j in range(3)]
N = sum(rows)
x2 = sum(
    (Q(M[i][j]) - Q(rows[i] * cols[j], N)) ** 2 / Q(rows[i] * cols[j], N)
    for i in range(2)
    for j in range(3)
)
x2 = mpf(x2.numerator) / x2.denominator
out("chisq.table", "statistic", x2)
out("chisq.table", "p", chisq_p(x2, 2))


# Fisher: ?fisher.test's TeaTasting, and a larger table.
def fisher(t):
    (a_, b_), (c_, d_) = t
    m, n_, k, x = a_ + c_, b_ + d_, a_ + b_, a_
    lo, hi = max(0, k - n_), min(k, m)

    ds = {i: Q(int(mpmath.binomial(m, i)) * int(mpmath.binomial(n_, k - i))) for i in range(lo, hi + 1)}
    bar = ds[x] * (1 + Q(1, 10**7))
    p = sum(v for v in ds.values() if v <= bar) / sum(ds.values())
    return x, mpf(p.numerator) / p.denominator


for label, t in [("fisher.tea", [[3, 1], [1, 3]]), ("fisher.big", [[12, 5], [7, 21]])]:
    s, p = fisher(t)
    out(label, "statistic", s)
    out(label, "p", p)


# Binomial: ?binom.test's example, by R's rule, exactly.
def dbinom(i, n, p):
    from math import comb

    return comb(n, i) * p**i * (1 - p) ** (n - i)


def binom(x, n, p):
    rel = 1 + Q(1, 10**7)
    d = dbinom(x, n, p)
    m = n * p
    if x == m:
        return Q(1)
    if x < m:
        import math

        y = sum(1 for i in range(math.ceil(m), n + 1) if dbinom(i, n, p) <= d * rel)
        pv = sum(dbinom(i, n, p) for i in range(0, x + 1)) + sum(
            dbinom(i, n, p) for i in range(n - y + 1, n + 1)
        )
    else:
        import math

        y = sum(1 for i in range(0, math.floor(m) + 1) if dbinom(i, n, p) <= d * rel)
        pv = sum(dbinom(i, n, p) for i in range(0, y)) + sum(dbinom(i, n, p) for i in range(x, n + 1))
    return min(Q(1), pv)


for label, (x, n_, p) in [("binom", (682, 925, Q(3, 4))), ("binom.low", (3, 20, Q(1, 2)))]:
    pv = binom(x, n_, p)
    out(label, "statistic", x)
    out(label, "p", mpf(pv.numerator) / pv.denominator)

# Mann-Whitney on sleep: normal approximation, ties and continuity.
allv = [Q(s) for s in A + B]
order = sorted(range(20), key=lambda i: allv[i])
rank = [Q(0)] * 20
i = 0
while i < 20:
    j = i
    while j + 1 < 20 and allv[order[j + 1]] == allv[order[i]]:
        j += 1
    for k in order[i : j + 1]:
        rank[k] = Q(i + j + 2, 2)
    i = j + 1
W = sum(rank[:10]) - Q(10 * 11, 2)
ties = {}
for v in allv:
    ties[v] = ties.get(v, 0) + 1
nt = sum(t**3 - t for t in ties.values())
sigma = mpmath.sqrt(mpf(100) / 12 * (21 - mpf(nt) / (20 * 19)))
z = mpf(W.numerator) / W.denominator - 50
z = (z - mpmath.sign(z) / 2) / sigma
out("mannwhitney", "statistic", mpf(W.numerator) / W.denominator)
out("mannwhitney", "p", 2 * min(mpmath.ncdf(z), 1 - mpmath.ncdf(z)))

# Kolmogorov-Smirnov on sleep: D exactly, and P(K > sqrt(nm/(n+m)) D).
sa, sb = sorted(Q(s) for s in A), sorted(Q(s) for s in B)
D = max(abs(Q(sum(1 for v in sa if v <= x), 10) - Q(sum(1 for v in sb if v <= x), 10)) for x in sa + sb)
lam = mpmath.sqrt(mpf(100) / 20) * mpf(D.numerator) / D.denominator
q = 2 * mpmath.nsum(lambda k: (-1) ** (k - 1) * mpmath.exp(-2 * k * k * lam * lam), [1, mpmath.inf])
out("ks2", "statistic", mpf(D.numerator) / D.denominator)
out("ks2", "p", q)

# p.adjust, exactly.
P = [Q(s) for s in ["0.01", "0.02", "0.03", "0.04", "0.05", "0.5", "0.001", "0.04"]]
P = [Q(float(p)) for p in P]
n_ = len(P)
o = sorted(range(n_), key=lambda i: P[i])
holm, run = [None] * n_, Q(0)
for r, i in enumerate(o):
    run = max(run, (n_ - r) * P[i])
    holm[i] = min(Q(1), run)
bh, run = [None] * n_, Q(1)
for r in range(n_ - 1, -1, -1):
    i = o[r]
    run = min(run, P[i] * n_ / (r + 1))
    bh[i] = min(Q(1), run)
print("padjust bonferroni [" + " ".join(show(mpf(min(Q(1), p * n_).numerator) / min(Q(1), p * n_).denominator) for p in P) + "]")
print("padjust holm [" + " ".join(show(mpf(h.numerator) / h.denominator) for h in holm) + "]")
print("padjust bh [" + " ".join(show(mpf(h.numerator) / h.denominator) for h in bh) + "]")

# lm: y on x1 and x2, with an intercept, by the normal equations at 50
# digits.
x1 = f(range(1, 9))
x2 = f(["2.5", "1.0", "4.0", "3.5", "6.0", "5.0", "8.5", "7.0"])
y = f(["3.1", "4.9", "7.2", "8.8", "11.1", "13.2", "14.8", "17.1"])
X = mpmath.matrix([[1, u, v] for u, v in zip(x1, x2)])
Y = mpmath.matrix(y)
XtX_inv = (X.T * X) ** -1
coef = XtX_inv * X.T * Y
res = Y - X * coef
rss = sum(r * r for r in res)
df = 8 - 3
s2 = rss / df
se = [mpmath.sqrt(s2 * XtX_inv[i, i]) for i in range(3)]
tt = [coef[i] / se[i] for i in range(3)]
pp = [t_p(t, mpf(df)) for t in tt]
ybar = mean(y)
r2 = 1 - rss / mpmath.fsum((v - ybar) ** 2 for v in y)
print("lm coef [" + " ".join(show(c) for c in coef) + "]")
print("lm se [" + " ".join(show(c) for c in se) + "]")
print("lm t [" + " ".join(show(c) for c in tt) + "]")
print("lm p [" + " ".join(show(c) for c in pp) + "]")
out("lm", "r2", r2)
out("lm", "df", df)
