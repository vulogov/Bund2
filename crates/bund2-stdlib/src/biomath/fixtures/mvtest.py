# RFC-0020 criterion 6's definitional fixture: Hotelling's one- and
# two-sample T², F and p by their definitions, with mpmath at 50 digits, on
# the iris subsets that mvtest.R writes into mvtest_r.txt.
#
#     python3 mvtest.py > mvtest.txt
#
# T² = n (x̄ − μ)ᵀ S⁻¹ (x̄ − μ), S on n − 1, F = (n − p)/(p(n − 1)) T² on p
# and n − p; T² = n₁n₂/(n₁ + n₂) (x̄₁ − x̄₂)ᵀ S_p⁻¹ (x̄₁ − x̄₂), S_p pooled on
# n₁ + n₂ − 2, F = (n₁ + n₂ − p − 1)/(p(n₁ + n₂ − 2)) T² on p and
# n₁ + n₂ − p − 1. p is F's upper tail, the regularised incomplete beta.

import os
import mpmath as mp

mp.mp.dps = 50
HERE = os.path.dirname(os.path.abspath(__file__))


def fixture(label):
    with open(os.path.join(HERE, "mvtest_r.txt")) as f:
        for line in f:
            if line.startswith(label + " ["):
                body = line[line.index("[") + 1:line.rindex("]")]
                return [mp.mpf(v) for v in body.split()]
    raise KeyError(label)


def rows(xs, p):
    return [xs[i:i + p] for i in range(0, len(xs), p)]


def mean(x):
    n = len(x)
    return [mp.fsum(r[j] for r in x) / n for j in range(len(x[0]))]


def sscp(x):
    m = mean(x)
    p = len(m)
    return mp.matrix([[mp.fsum((r[i] - m[i]) * (r[j] - m[j]) for r in x)
                       for j in range(p)] for i in range(p)])


def f_upper(f, d1, d2):
    return mp.betainc(d2 / 2, d1 / 2, 0, d2 / (d2 + d1 * f), regularized=True)


def out(label, xs):
    print(label, "[" + " ".join(mp.nstr(v, 25) for v in xs) + "]")


p = 4
ver = rows(fixture("versicolor"), p)
vir = rows(fixture("virginica"), p)
mu = fixture("mu")

n = len(ver)
d = mp.matrix([mean(ver)[j] - mu[j] for j in range(p)])
s = sscp(ver) / (n - 1)
t2 = n * (d.T * mp.lu_solve(s, d))[0]
f = (n - p) / mp.mpf(p * (n - 1)) * t2
out("hotelling1", [t2, f, p, n - p, f_upper(f, p, n - p)])

n1, n2 = len(ver), len(vir)
d = mp.matrix([mean(ver)[j] - mean(vir)[j] for j in range(p)])
sp = (sscp(ver) + sscp(vir)) / (n1 + n2 - 2)
t2 = mp.mpf(n1 * n2) / (n1 + n2) * (d.T * mp.lu_solve(sp, d))[0]
df2 = n1 + n2 - p - 1
f = df2 / mp.mpf(p * (n1 + n2 - 2)) * t2
out("hotelling2", [t2, f, p, df2, f_upper(f, p, df2)])
