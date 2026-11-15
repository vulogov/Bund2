# RFC-0015 criterion 12's fixture for S9: exact answers on 3x3 matrices,
# by rational arithmetic, and worked example 5's eigenvalues by mpmath 1.3.0
# at 50 significant digits.
#
#     python3 -I linalg.py > linalg.txt
#
# Each line is a label and the answer, a number or a bracketed row-major
# list, to 17 significant digits. The test holds the Bund program for each
# label.
from fractions import Fraction as Q

import mpmath
from mpmath import mpf

mpmath.mp.dps = 50


def show(x):
    if isinstance(x, Q):
        x = mpf(x.numerator) / x.denominator
    return mpmath.nstr(mpf(x), 17, min_fixed=-1, max_fixed=-1)


def out(label, xs):
    if isinstance(xs, list):
        flat = [v for row in xs for v in row] if isinstance(xs[0], list) else xs
        print(label, "[" + " ".join(show(v) for v in flat) + "]")
    else:
        print(label, show(xs))


A = [[Q(2), Q(-1), Q(0)], [Q(1), Q(3), Q(2)], [Q(0), Q(1), Q(4)]]
B = [[Q(1), Q(2), Q(0)], [Q(0), Q(1), Q(-1)], [Q(3), Q(0), Q(2)]]


def mul(x, y):
    return [[sum(x[i][k] * y[k][j] for k in range(len(y))) for j in range(len(y[0]))] for i in range(len(x))]


def det(m):
    a, b, c = m[0]
    d, e, f = m[1]
    g, h, i = m[2]
    return a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)


def inv(m):
    d = det(m)
    cof = [
        [
            (-1) ** (i + j)
            * det2([[m[r][c] for c in range(3) if c != j] for r in range(3) if r != i])
            for j in range(3)
        ]
        for i in range(3)
    ]
    return [[cof[j][i] / d for j in range(3)] for i in range(3)]


def det2(m):
    return m[0][0] * m[1][1] - m[0][1] * m[1][0]


out("mul", mul(A, B))
out("t", [[A[j][i] for j in range(3)] for i in range(3)])
out("det", det(A))
out("inv", inv(A))
b = [Q(1), Q(2), Q(3)]
Ai = inv(A)
out("solve", [sum(Ai[i][k] * b[k] for k in range(3)) for i in range(3)])
P = [[Q(int(i == j)) for j in range(3)] for i in range(3)]
for _ in range(5):
    P = mul(P, A)
out("power", P)
v = [Q(1), Q(-1), Q(2)]
out("mulv", [sum(A[i][k] * v[k] for k in range(3)) for i in range(3)])

# Worked example 5: the characteristic polynomial of
# [[0 1.5 0.8] [0.5 0 0] [0 0.7 0]] is l^3 - f2 s1 l - f3 s1 s2, with the
# cells taken at the f64s the Bund literals read as.
f2, f3, s1, s2 = (mpf(float(x)) for x in ["1.5", "0.8", "0.5", "0.7"])
roots = mpmath.polyroots([1, 0, -f2 * s1, -f3 * s1 * s2], maxsteps=200, extraprec=200)
roots = sorted(roots, key=lambda z: (-abs(z), -mpmath.re(z), -mpmath.im(z)))
lam = mpmath.re(roots[0])
out("leslie.value", lam)
w = [mpf(1), s1 / lam, s1 * s2 / lam**2]
total = mpmath.fsum(w)
out("leslie.vector", [x / total for x in w])
print("leslie.complex [" + " ".join(f"{show(mpmath.re(z))} {show(mpmath.im(z))}" for z in roots) + "]")
