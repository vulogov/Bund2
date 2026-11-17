# RFC-0016's fixture for its tests and corrections, computed with exact
# rationals and printed with mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I infer16.py > infer16.txt
#
# Each line is a label, a field and its value to 17 significant digits; the
# test holds the Bund program for each label. The p-values are f64s, taken
# exactly (Fraction of a float), as Bund2 holds them.
#
# Criterion 8 (M1): Hochberg, Hommel and Benjamini-Yekutieli on 20
# p-values, in input order. Hommel's is computed by its definition, the
# largest Simes p over every subset containing i, enumerating all 2^20
# subsets; S4's steps are not used here.
from fractions import Fraction as Q
from itertools import combinations

import mpmath
from mpmath import mpf

mpmath.mp.dps = 50


def show(x):
    return mpmath.nstr(mpf(x.numerator) / x.denominator, 17, min_fixed=-1, max_fixed=-1)


def line(label, field, values):
    print(label, field, "[" + " ".join(show(v) for v in values) + "]")


# Twenty p-values, unsorted, with a tie (0.0278) and a 1.
P_F = [0.0201, 0.0001, 0.3240, 0.0278, 0.6528, 0.0019, 1.0, 0.0459, 0.0004,
       0.0298, 0.4262, 0.0278, 0.0150, 0.7590, 0.0344, 0.0095, 0.5719,
       0.0410, 0.0009, 0.2000]
P = [Q(p) for p in P_F]
n = len(P)
order = sorted(range(n), key=lambda i: P[i])  # stable: ties in input order
S = [P[i] for i in order]


def in_input_order(sorted_adj):
    out = [None] * n
    for r, i in enumerate(order):
        out[i] = sorted_adj[r]
    return out


# Hochberg: min over j >= i of min(1, (n - j + 1) p_(j)), 1-based.
hoch, run = [None] * n, Q(1)
for r in range(n - 1, -1, -1):
    run = min(run, (n - r) * S[r])
    hoch[r] = min(Q(1), run)

# Benjamini-Yekutieli: min over j >= i of min(1, c n p_(j) / j).
c = sum(Q(1, k) for k in range(1, n + 1))
by, run = [None] * n, Q(1)
for r in range(n - 1, -1, -1):
    run = min(run, c * n * S[r] / (r + 1))
    by[r] = min(Q(1), run)

# Hommel by definition: for each i, the largest Simes p over subsets I
# containing i, Simes(I) = min_j |I| p_(j)^I / j; then capped at 1. Exact:
# each p is k/2^e, so compare as integers over a common denominator.
D = max(p.denominator for p in S)
N = [int(p * D) for p in S]  # exact: every denominator divides D
best = [Q(0)] * n
for size in range(1, n + 1):
    for I in combinations(range(n), size):  # I is sorted, so p's ascend
        # min over j of size * N[I[j]] / (j + 1), as a fraction num/den
        num, den = size * N[I[0]], 1
        for j in range(1, size):
            a, b = size * N[I[j]], j + 1
            if a * den < num * b:
                num, den = a, b
        v = Q(num, den * D)
        for r in I:
            if v > best[r]:
                best[r] = v
hommel = [min(Q(1), b) for b in best]

line("padjust20", "p", P)
line("padjust20", "hochberg", in_input_order(hoch))
line("padjust20", "hommel", in_input_order(hommel))
line("padjust20", "by", in_input_order(by))


# Criteria 4 and 5's first part (M2): the t tests' and the binomial's
# estimates, intervals and one-sided p, at 50 digits. A quantile is found
# on the regularized incomplete beta, the t's cdf by mpmath's root
# finder and the beta's by bisection. The data are R's `sleep`, as infer.py's, and
# ?binom.test's 682 of 925.
def num(label, field, x):
    print(label, field, mpmath.nstr(mpf(x), 17, min_fixed=-1, max_fixed=-1))


def fl(xs):
    return [mpf(float(x)) for x in xs]


SA = fl(["0.7", "-1.6", "-0.2", "-1.2", "-0.1", "3.4", "3.7", "0.8", "0.0", "2.0"])
SB = fl(["1.9", "0.8", "1.1", "0.1", "-0.1", "4.4", "5.5", "1.6", "4.6", "3.4"])


def mean(xs):
    return mpmath.fsum(xs) / len(xs)


def var(xs):
    m = mean(xs)
    return mpmath.fsum((x - m) ** 2 for x in xs) / (len(xs) - 1)


def t_cdf(t, df):
    # P(T <= t) for t < 0 is I_{df/(df+t^2)}(df/2, 1/2) / 2.
    tail = mpmath.betainc(df / 2, mpf(1) / 2, 0, df / (df + t * t), regularized=True) / 2
    return tail if t < 0 else 1 - tail


def t_q(p, df):
    # The upper quantile, p > 1/2.
    return mpmath.findroot(lambda t: t_cdf(t, df) - p, mpmath.sqrt(2) * mpmath.erfinv(2 * p - 1))


def t_case(label, est, se, df, mu_stat=None):
    t = est / se
    num(label, "statistic", t)
    num(label, "df", df)
    num(label, "p", 2 * t_cdf(-abs(t), df))
    num(label, "estimate", est)
    for conf, tag in [(mpf("0.95"), ""), (mpf("0.99"), "99")]:
        conf = mpf(float(conf))  # the f64 the program writes
        q = t_q(1 - (1 - conf) / 2, df)
        num(label, "lower" + tag, est - q * se)
        num(label, "upper" + tag, est + q * se)
    conf = mpf(0.95)
    num(label, "less.p", t_cdf(t, df))
    num(label, "less.upper", est + t_q(conf, df) * se)
    num(label, "greater.p", 1 - t_cdf(t, df))
    num(label, "greater.lower", est - t_q(conf, df) * se)
    if mu_stat is not None:
        t1 = (est - 1) / se
        num(label, "mu1.statistic", t1)
        num(label, "mu1.p", 2 * t_cdf(-abs(t1), df))


t_case("t1.opts", mean(SA), mpmath.sqrt(var(SA) / 10), mpf(9))
pooled = (9 * var(SA) + 9 * var(SB)) / 18
t_case("t2.opts", mean(SA) - mean(SB), mpmath.sqrt(pooled * mpf(2) / 10), mpf(18), True)
v1, v2 = var(SA) / 10, var(SB) / 10
t_case("welch.opts", mean(SA) - mean(SB), mpmath.sqrt(v1 + v2),
       (v1 + v2) ** 2 / (v1**2 / 9 + v2**2 / 9), True)
D = [x - y for x, y in zip(SA, SB)]
t_case("paired.opts", mean(D), mpmath.sqrt(var(D) / 10), mpf(9), True)


def beta_q(p, a, b):
    # Bisection on [0, 1], where the cdf rises: 200 halvings, below 1e-60.
    lo, hi = mpf(0), mpf(1)
    for _ in range(200):
        mid = (lo + hi) / 2
        if mpmath.betainc(a, b, 0, mid, regularized=True) < p:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def binom_case(label, k, n, p):
    pr = Q(p)
    tot = lambda r: sum(Q(comb(n, i)) * pr**i * (1 - pr) ** (n - i) for i in r)
    num(label, "estimate", mpf(k) / n)
    alpha = 1 - mpf(0.95)
    num(label, "lower", beta_q(alpha / 2, k, n - k + 1))
    num(label, "upper", beta_q(1 - alpha / 2, k + 1, n - k))
    num(label, "greater.lower", beta_q(alpha, k, n - k + 1))
    num(label, "less.upper", beta_q(1 - alpha, k + 1, n - k))
    lp, gp = tot(range(0, k + 1)), tot(range(k, n + 1))
    num(label, "less.p", mpf(lp.numerator) / lp.denominator)
    num(label, "greater.p", mpf(gp.numerator) / gp.denominator)


from math import comb  # noqa: E402

binom_case("binom.opts", 682, 925, 0.75)
