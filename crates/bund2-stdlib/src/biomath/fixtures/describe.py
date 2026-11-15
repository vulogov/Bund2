# RFC-0015 criteria 8 and 9's fixture for M3: S5's vec. words and S6's
# descriptive stat. words, computed from their definitions in exact rational
# arithmetic (fractions), with mpmath 1.3.0 at 50 digits for the roots and
# logarithms.
#
#     python3 -I describe.py > describe.txt
#
# Each line is a word, a fixture name, its operand where it takes one, and
# the answer: one number to 17 significant digits, or a LIST of them.
from fractions import Fraction as F

import mpmath

mpmath.mp.dps = 50

A = [F(1), F(2), F(3), F(4)]
B = [F(1, 2), F(-1), F(2), F(8)]
X = [F(v) for v in [2, 4, 4, 4, 5, 5, 7, 9]]
Y = [F(v) for v in [1, 3, 2, 5, 4, 6, 8, 7]]


def mp(v):
    return mpmath.mpf(v.numerator) / v.denominator if isinstance(v, F) else v


def show(v):
    if isinstance(v, list):
        return "[" + " ".join(show(e) for e in v) + "]"
    return mpmath.nstr(mp(v), 17)


def out(word, name, value, operand=None):
    print(word, name, *([] if operand is None else [operand]), show(value))


def mean(s):
    return sum(s) / len(s)


def var(s):
    m = mean(s)
    return sum((v - m) ** 2 for v in s) / (len(s) - 1)


def moment(s, k):
    m = mean(s)
    return sum((v - m) ** k for v in s) / len(s)


def ranks(s):
    order = sorted(range(len(s)), key=lambda i: s[i])
    r = [F(0)] * len(s)
    i = 0
    while i < len(s):
        j = i
        while j + 1 < len(s) and s[order[j + 1]] == s[order[i]]:
            j += 1
        for t in range(i, j + 1):
            r[order[t]] = F(i + j + 2, 2)
        i = j + 1
    return r


def cov(a, b):
    ma, mb = mean(a), mean(b)
    return sum((p - ma) * (q - mb) for p, q in zip(a, b)) / (len(a) - 1)


def pearson(a, b):
    return mp(cov(a, b)) / mpmath.sqrt(mp(var(a) * var(b)))


def kendall_b(a, b):
    n = len(a)
    nc = nd = ta = tb = 0
    for i in range(n):
        for j in range(i + 1, n):
            da, db = a[i] - a[j], b[i] - b[j]
            if da == 0:
                ta += 1
            if db == 0:
                tb += 1
            if da != 0 and db != 0:
                if (da > 0) == (db > 0):
                    nc += 1
                else:
                    nd += 1
    n0 = n * (n - 1) // 2
    return (nc - nd) / mpmath.sqrt(mpmath.mpf((n0 - ta) * (n0 - tb)))


def quantile7(s, p):
    s = sorted(s)
    h = (len(s) - 1) * p
    lo = int(h)
    hi = min(lo + 1, len(s) - 1)
    return s[lo] + (h - lo) * (s[hi] - s[lo])


# S5, criterion 8.
out("vec.add", "A B", [p + q for p, q in zip(A, B)])
out("vec.sub", "A B", [p - q for p, q in zip(A, B)])
out("vec.mul", "A B", [p * q for p, q in zip(A, B)])
out("vec.div", "A B", [p / q for p, q in zip(A, B)])
out("vec.scale", "A", [p * F(5, 2) for p in A], "2.5")
out("vec.dot", "A B", sum(p * q for p, q in zip(A, B)))
for name, s in [("A", A), ("B", B)]:
    out("vec.sum", name, sum(s))
    prod = F(1)
    for v in s:
        prod *= v
    out("vec.prod", name, prod)
    out("vec.cumsum", name, [sum(s[: i + 1]) for i in range(len(s))])
    cp, acc = [], F(1)
    for v in s:
        acc *= v
        cp.append(acc)
    out("vec.cumprod", name, cp)
    out("vec.diff", name, [s[i + 1] - s[i] for i in range(len(s) - 1)])
    out("vec.logsumexp", name, mpmath.log(sum(mpmath.exp(mp(v)) for v in s)))
    out("vec.normalize.sum", name, [v / sum(s) for v in s])
out("vec.logsumexp", "BIG", 1000 + mpmath.log(2))

# S6, criterion 9: X by hand is mean 5, variance 32/7, median 4.5, mode 4.
out("stat.var", "X", var(X))
out("stat.sd", "X", mpmath.sqrt(mp(var(X))))
out("stat.se", "X", mpmath.sqrt(mp(var(X) / len(X))))
out("stat.median", "X", F(9, 2))
out("stat.mode", "X", F(4))
out("stat.skewness", "X", mp(moment(X, 3)) / mp(moment(X, 2)) ** mpmath.mpf(1.5))
out("stat.kurtosis", "X", moment(X, 4) / moment(X, 2) ** 2 - 3)
out("stat.zscore", "X", [mp(v - mean(X)) / mpmath.sqrt(mp(var(X))) for v in X])
for p in ["0", "0.1", "0.25", "0.5", "0.9", "1"]:
    out("stat.quantile", "X", quantile7(X, F(p)), p)
out("stat.cov", "X Y", cov(X, Y))
out("stat.cor.pearson", "X Y", pearson(X, Y))
out("stat.cor.spearman", "X Y", pearson(ranks(X), ranks(Y)))
out("stat.cor.kendall", "X Y", kendall_b(X, Y))
