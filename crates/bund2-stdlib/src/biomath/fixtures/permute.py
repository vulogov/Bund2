# RFC-0019 criterion 11's fixture for §S9: the exact two-sample permutation
# test's statistic and p by enumeration, for each named statistic and each
# alternative, with mpmath 1.3.0 at 50 digits.
#
#     python3 -I permute.py > permute.txt
#
# The samples are small integers, read exactly. Every split puts n of the
# n + m pooled values (a, then b) in a, each half in the pooled order; the
# splits are counted, not ordered, so the enumeration's order does not
# matter here. mean.diff and median.diff are computed as exact fractions
# and t, Welch's, at 50 digits, so a split tied with the observed value in
# exact arithmetic is tied here too (to within 1e-40 for t, far below the
# 1e-12 relative tolerance §S9 gives the f64 comparison). "As extreme" is
# |T| >= |T0| two-sided, T >= T0 greater, T <= T0 less.
#
# Each line is `case statistic alternative key value`: `t0` (17 digits),
# `hits` and `splits`, so that p = hits/splits.

from fractions import Fraction
from itertools import combinations

from mpmath import mp, mpf, sqrt

mp.dps = 50

CASES = {
    "x": ([12, 15, 9, 20, 17], [8, 11, 6, 14, 10, 7]),
    "y": ([1, 2, 3, 3], [1, 2, 4, 5, 5]),
}


def mean(xs):
    return Fraction(sum(xs), len(xs))


def median(xs):
    s = sorted(xs)
    n = len(s)
    return Fraction(s[n // 2]) if n % 2 else Fraction(s[n // 2 - 1] + s[n // 2], 2)


def var(xs):
    m = mean(xs)
    return sum((v - m) ** 2 for v in xs) / (len(xs) - 1)


def welch(a, b):
    se2 = var(a) / len(a) + var(b) / len(b)
    d = mean(a) - mean(b)
    if se2 == 0:
        return mpf("inf") if d > 0 else -mpf("inf")
    return mpf(d.numerator) / d.denominator / sqrt(mpf(se2.numerator) / se2.denominator)


STAT = {
    "mean.diff": lambda a, b: mean(a) - mean(b),
    "median.diff": lambda a, b: median(a) - median(b),
    "t": welch,
}


def tied(t, t0):
    if isinstance(t0, Fraction):
        return t == t0
    return abs(t - t0) <= mpf("1e-40") * abs(t0)


def extreme(alt, t, t0):
    if alt == "two.sided":
        return abs(t) > abs(t0) or tied(abs(t), abs(t0))
    if alt == "greater":
        return t > t0 or tied(t, t0)
    return t < t0 or tied(t, t0)


def show(v):
    if isinstance(v, Fraction):
        v = mpf(v.numerator) / v.denominator
    return mp.nstr(v, 17, min_fixed=-30, max_fixed=30)


for case, (a, b) in CASES.items():
    pooled = a + b
    n = len(a)
    print(f"{case} a [ {' '.join(map(str, a))} ]")
    print(f"{case} b [ {' '.join(map(str, b))} ]")
    for name, f in STAT.items():
        t0 = f(a, b)
        splits = list(combinations(range(len(pooled)), n))
        values = []
        for left in splits:
            right = [i for i in range(len(pooled)) if i not in left]
            values.append(f([pooled[i] for i in left], [pooled[i] for i in right]))
        for alt in ("two.sided", "greater", "less"):
            hits = sum(1 for t in values if extreme(alt, t, t0))
            print(f"{case} {name} {alt} t0 {show(t0)}")
            print(f"{case} {name} {alt} hits {hits}")
            print(f"{case} {name} {alt} splits {len(splits)}")
