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


# Criteria 5's second part and 6 (M3): Fisher's conditional MLE and exact
# interval, the KS test's one-sided statistics and p, and Mann-Whitney's
# one-sided and shifted p. Fisher's roots are found by bisection on log psi
# to 1e-40; the masses are exact binomials times psi^x.
def fisher_opts(label, t, alts):
    (a_, b_), (c_, d_) = t
    m, n_, k, x = a_ + c_, b_ + d_, a_ + b_, a_
    lo, hi = max(0, k - n_), min(k, m)
    base = {i: mpf(comb(m, i) * comb(n_, k - i)) for i in range(lo, hi + 1)}

    def masses(lpsi):
        return {i: base[i] * mpmath.exp(i * lpsi) for i in base}

    def mean_at(lpsi):
        w = masses(lpsi)
        return mpmath.fsum(i * v for i, v in w.items()) / mpmath.fsum(w.values())

    def ge(lpsi):
        w = masses(lpsi)
        return mpmath.fsum(v for i, v in w.items() if i >= x) / mpmath.fsum(w.values())

    def le(lpsi):
        w = masses(lpsi)
        return mpmath.fsum(v for i, v in w.items() if i <= x) / mpmath.fsum(w.values())

    def root(f, target, rising):
        lo_, hi_ = mpf(-60), mpf(60)
        while hi_ - lo_ > mpf("1e-40"):
            mid = (lo_ + hi_) / 2
            if (f(mid) < target) == rising:
                lo_ = mid
            else:
                hi_ = mid
        return mpmath.exp((lo_ + hi_) / 2)

    total = mpmath.fsum(base.values())
    for alt in alts:
        conf = mpf(0.95)
        alpha = (1 - conf) / 2 if alt == "two.sided" else 1 - conf
        tag = label + "." + alt
        if alt == "two.sided":
            if lo < x < hi:
                num(tag, "estimate", root(mean_at, x, True))
        if alt in ("two.sided", "greater") and x > lo:
            num(tag, "lower", root(ge, alpha, True))
        if alt in ("two.sided", "less") and x < hi:
            num(tag, "upper", root(le, alpha, False))
        if alt == "greater":
            num(tag, "p", mpmath.fsum(v for i, v in base.items() if i >= x) / total)
        if alt == "less":
            num(tag, "p", mpmath.fsum(v for i, v in base.items() if i <= x) / total)


fisher_opts("fisher.tea", [[3, 1], [1, 3]], ["two.sided", "greater", "less"])
fisher_opts("fisher.zero", [[3, 0], [0, 3]], ["two.sided"])


# KS on sleep: D+ = max(F_a - F_b), D- = max(F_b - F_a), over the pooled
# values, exactly; the one-sided p is exp(-2 nm/(n+m) D^2).
QA = [Q(s) for s in ["0.7", "-1.6", "-0.2", "-1.2", "-0.1", "3.4", "3.7", "0.8", "0.0", "2.0"]]
QB = [Q(s) for s in ["1.9", "0.8", "1.1", "0.1", "-0.1", "4.4", "5.5", "1.6", "4.6", "3.4"]]
dp = max(Q(sum(1 for a in QA if a <= v), 10) - Q(sum(1 for b in QB if b <= v), 10) for v in QA + QB)
dm = max(Q(sum(1 for b in QB if b <= v), 10) - Q(sum(1 for a in QA if a <= v), 10) for v in QA + QB)
for alt, d in [("greater", dp), ("less", dm)]:
    num("ks2." + alt, "statistic", mpf(d.numerator) / d.denominator)
    num("ks2." + alt, "p", mpmath.exp(-2 * mpf(100) / 20 * (mpf(d.numerator) / d.denominator) ** 2))


# Mann-Whitney on criterion 6's exact case: W of a - mu against b, each
# value the f64 the program writes, taken exactly; sigma with its tie term;
# the one-sided p with c = +1/2 ("greater") or -1/2 ("less"), the
# two-sided with sign(z)/2.
def mw(a, b, mu, alt):
    a = [Q(x) - Q(mu) for x in a]  # a - mu rounds in f64 in Bund2; exact here
    b = [Q(x) for x in b]
    allv = a + b
    n1, n2 = len(a), len(b)
    srt = sorted(allv)
    rank = {}
    i = 0
    while i < len(srt):
        j = i
        while j + 1 < len(srt) and srt[j + 1] == srt[i]:
            j += 1
        rank[srt[i]] = Q(i + j + 2, 2)
        i = j + 1
    W = sum(rank[v] for v in a) - Q(n1 * (n1 + 1), 2)
    ties = 0
    for v in set(allv):
        t = allv.count(v)
        ties += t**3 - t
    N = n1 + n2
    sigma = mpmath.sqrt(mpf(n1 * n2) / 12 * ((N + 1) - mpf(ties) / (N * (N - 1))))
    z0 = mpf(W.numerator) / W.denominator - mpf(n1 * n2) / 2
    if alt == "two.sided":
        c = 0 if z0 == 0 else mpmath.sign(z0) / 2
        z = (z0 - c) / sigma
        return W, 2 * min(mpmath.ncdf(z), 1 - mpmath.ncdf(z))
    if alt == "greater":
        return W, 1 - mpmath.ncdf((z0 - mpf(1) / 2) / sigma)
    return W, mpmath.ncdf((z0 + mpf(1) / 2) / sigma)


HA = [1.1, 2.2, 3.3, 4.4]
HB = [0.5, 1.5, 2.0, 2.5, 3.0]
for label, mu, alt in [("mw.greater", 0.0, "greater"), ("mw.less", 0.0, "less"),
                       ("mw.mu", 0.5, "two.sided")]:
    W, p = mw(HA, HB, mu, alt)
    num(label, "statistic", mpf(W.numerator) / W.denominator)
    num(label, "p", p)


# Criterion 7 (M4): the correlation tests. Pearson's r from the f64 values
# at 50 digits, its t on n - 2 df and Fisher's z interval; Spearman's rho
# from exact ranks, S and the t approximation's p; Kendall's exact T and p
# by the integer recurrence I(n, t) = sum_j I(n - 1, t - j) over n!, and
# with ties its z by S3's variance.
def pearson_r(a, b):
    a, b = fl(a), fl(b)
    ma, mb = mean(a), mean(b)
    sab = mpmath.fsum((x - ma) * (y - mb) for x, y in zip(a, b))
    saa = mpmath.fsum((x - ma) ** 2 for x in a)
    sbb = mpmath.fsum((y - mb) ** 2 for y in b)
    return sab / mpmath.sqrt(saa * sbb)


def zq(p):
    return mpmath.sqrt(2) * mpmath.erfinv(2 * p - 1)


def cor_t(r, df, alt):
    t = r * mpmath.sqrt(df / (1 - r * r))
    if alt == "two.sided":
        return t, 2 * t_cdf(-abs(t), df)
    if alt == "greater":
        return t, 1 - t_cdf(t, df)
    return t, t_cdf(t, df)


def pearson_case(label, a, b):
    r = pearson_r(a, b)
    n = len(a)
    num(label, "estimate", r)
    for alt in ["two.sided", "greater", "less"]:
        t, p = cor_t(r, mpf(n - 2), alt)
        num(label + "." + alt, "statistic", t)
        num(label + "." + alt, "p", p)
        if n >= 4:
            se = 1 / mpmath.sqrt(n - 3)
            alpha = 1 - mpf(0.95)
            if alt == "two.sided":
                q = zq(1 - alpha / 2)
                num(label + "." + alt, "lower", mpmath.tanh(mpmath.atanh(r) - q * se))
                num(label + "." + alt, "upper", mpmath.tanh(mpmath.atanh(r) + q * se))
            elif alt == "greater":
                num(label + "." + alt, "lower", mpmath.tanh(mpmath.atanh(r) - zq(1 - alpha) * se))
            else:
                num(label + "." + alt, "upper", mpmath.tanh(mpmath.atanh(r) + zq(1 - alpha) * se))


def exact_ranks(xs):
    xs = [Q(x) for x in xs]
    srt = sorted(xs)
    out = []
    for x in xs:
        first = srt.index(x)
        last = len(srt) - 1 - srt[::-1].index(x)
        out.append(Q(first + last + 2, 2))
    return out


def spearman_case(label, a, b):
    ra, rb = exact_ranks(a), exact_ranks(b)
    n = len(a)
    ma, mb = sum(ra) / n, sum(rb) / n
    sab = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    saa = sum((x - ma) ** 2 for x in ra)
    sbb = sum((y - mb) ** 2 for y in rb)
    rho = mpf(sab.numerator) / sab.denominator / mpmath.sqrt(
        mpf((saa * sbb).numerator) / (saa * sbb).denominator)
    num(label, "estimate", rho)
    num(label, "statistic", (mpf(n) ** 3 - n) * (1 - rho) / 6)
    for alt in ["two.sided", "greater", "less"]:
        num(label + "." + alt, "p", cor_t(rho, mpf(n - 2), alt)[1])


def kendall_counts(a, b):
    a, b = [Q(x) for x in a], [Q(x) for x in b]
    nc = nd = 0
    for i in range(len(a)):
        for j in range(i + 1, len(a)):
            s = (a[i] - a[j]) * (b[i] - b[j])
            if s > 0:
                nc += 1
            elif s < 0:
                nd += 1
    return nc, nd


def kendall_exact(label, a, b):
    n = len(a)
    nc, nd = kendall_counts(a, b)
    I = [1]
    for k in range(2, n + 1):
        nxt = [0] * (k * (k - 1) // 2 + 1)
        for t in range(len(nxt)):
            nxt[t] = sum(I[u] for u in range(max(0, t - k + 1), min(t, len(I) - 1) + 1))
        I = nxt
    fact = mpmath.factorial(n)
    le = mpf(sum(I[: nc + 1])) / fact
    ge = mpf(sum(I[nc:])) / fact
    N = n * (n - 1) // 2
    two = min(1, 2 * (ge if 4 * nc > n * (n - 1) else le))
    num(label, "statistic", nc)
    num(label + ".two.sided", "p", two)
    num(label + ".greater", "p", ge)
    num(label + ".less", "p", le)


def kendall_ties(label, a, b):
    n = len(a)
    nc, nd = kendall_counts(a, b)

    def groups(xs):
        return [xs.count(v) for v in set(xs)]

    ta, ub = groups([Q(x) for x in a]), groups([Q(x) for x in b])
    v0 = n * (n - 1) * (2 * n + 5)
    vt = sum(t * (t - 1) * (2 * t + 5) for t in ta)
    vu = sum(u * (u - 1) * (2 * u + 5) for u in ub)
    v1 = sum(t * (t - 1) for t in ta) * sum(u * (u - 1) for u in ub)
    v2 = sum(t * (t - 1) * (t - 2) for t in ta) * sum(u * (u - 1) * (u - 2) for u in ub)
    v = Q(v0 - vt - vu, 18) + Q(v1, 2 * n * (n - 1)) + Q(v2, 9 * n * (n - 1) * (n - 2))
    z = (nc - nd) / mpmath.sqrt(mpf(v.numerator) / v.denominator)
    num(label, "statistic", z)
    num(label + ".two.sided", "p", 2 * min(mpmath.ncdf(z), 1 - mpmath.ncdf(z)))
    num(label + ".greater", "p", 1 - mpmath.ncdf(z))
    num(label + ".less", "p", mpmath.ncdf(z))


SLEEP_A = ["0.7", "-1.6", "-0.2", "-1.2", "-0.1", "3.4", "3.7", "0.8", "0.0", "2.0"]
SLEEP_B = ["1.9", "0.8", "1.1", "0.1", "-0.1", "4.4", "5.5", "1.6", "4.6", "3.4"]
pearson_case("cor.pearson", SLEEP_A, SLEEP_B)
pearson_case("cor.pearson3", ["1.0", "2.0", "4.0"], ["1.5", "2.0", "5.0"])
spearman_case("cor.spearman", SLEEP_A, SLEEP_B)
kendall_exact("cor.kendall10", SLEEP_A, SLEEP_B)
K45_B = list(range(1, 46))
for i in [0, 9, 19, 29, 39]:
    K45_B[i], K45_B[i + 1] = K45_B[i + 1], K45_B[i]
kendall_exact("cor.kendall45", list(range(1, 46)), K45_B)
kendall_ties("cor.kendall.ties", ["1", "2", "2", "3", "4", "4", "5", "6", "7", "8"],
             ["1.9", "0.8", "1.1", "0.1", "0.1", "4.4", "5.5", "1.6", "4.6", "4.4"])
