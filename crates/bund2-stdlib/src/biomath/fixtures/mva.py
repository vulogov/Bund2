# RFC-0020 criteria 2 and 3's definitional fixture: the SVD's singular
# values, and principal components and classical MDS from their
# definitions, by mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I mva.py > mva.txt
#
# The inputs USArrests and eurodist are read from mva_r.txt, which
# mva.R wrote, so this script needs no R. Each line is a label and a
# bracketed list, row-major for a matrix, at 17 significant digits. Every
# vector obeys RFC-0020 §S1's sign rule: its element of the largest
# absolute value is positive, the first such on a tie.
import os

import mpmath
from mpmath import mp, mpf

mp.dps = 50

HERE = os.path.dirname(os.path.abspath(__file__))


def show(x):
    return mpmath.nstr(mpf(x), 17, min_fixed=-1, max_fixed=-1)


def out(label, xs):
    print(label, "[" + " ".join(show(v) for v in xs) + "]")


def rowmajor(m):
    return [m[i, j] for i in range(m.rows) for j in range(m.cols)]


def r_input(label):
    with open(os.path.join(HERE, "mva_r.txt")) as f:
        for line in f:
            if line.startswith(label + " "):
                return [mpf(v) for v in line[line.index("[") + 1 : line.rindex("]")].split()]
    raise KeyError(label)


def square(xs):
    n = int(len(xs) ** 0.5)
    return mp.matrix([[xs[i * n + j] for j in range(n)] for i in range(n)])


def signed(m):
    """Flip each column so its largest |element| is positive."""
    m = m.copy()
    for j in range(m.cols):
        best = 0
        for i in range(m.rows):
            if abs(m[i, j]) > abs(m[best, j]):
                best = i
        if m[best, j] < 0:
            for i in range(m.rows):
                m[i, j] = -m[i, j]
    return m


def svd(a):
    """The thin SVD, singular values descending: d, and V's columns."""
    u, s, v = mp.svd_r(a, full_matrices=False)
    r = len(s)
    order = sorted(range(r), key=lambda i: -s[i])
    d = [s[i] for i in order]
    vt = mp.matrix(a.cols, r)
    for c, i in enumerate(order):
        for j in range(a.cols):
            vt[j, c] = v[i, j]
    return d, vt


# Criterion 2: the singular values.
for label, rows in [
    ("svd.square", [[2, -1, 0], [1, 3, 2], [0, 1, 4]]),
    ("svd.tall", [[1, 2], [3, 4], [5, 6]]),
    ("svd.wide", [[1, 2, 3, 4], [2, 0, 1, -1]]),
    ("svd.rankdef", [[1, 2, 3], [2, 4, 6], [1, 0, 1]]),
]:
    d, _ = svd(mp.matrix(rows))
    out(label, d)

# Criterion 3: prcomp's definition on USArrests.
xs = r_input("usarrests")
n, p = len(xs) // 4, 4
x = mp.matrix([[xs[i * p + j] for j in range(p)] for i in range(n)])
for label, center, scale in [("cs", True, False), ("cS", True, True), ("nS", False, True)]:
    y = x.copy()
    if center:
        for j in range(p):
            mean = mpmath.fsum(y[i, j] for i in range(n)) / n
            for i in range(n):
                y[i, j] -= mean
    if scale:
        for j in range(p):
            s = mp.sqrt(mpmath.fsum(y[i, j] ** 2 for i in range(n)) / max(1, n - 1))
            for i in range(n):
                y[i, j] /= s
    d, v = svd(y)
    v = signed(v)
    sdev = [di / mp.sqrt(max(1, n - 1)) for di in d]
    total = mpmath.fsum(s**2 for s in sdev)
    out(f"pca.{label}.sdev", sdev)
    out(f"pca.{label}.rotation", rowmajor(v))
    out(f"pca.{label}.x", rowmajor(y * v))
    prop = [s**2 / total for s in sdev]
    out(f"pca.{label}.prop", prop)
    out(f"pca.{label}.cumprop", [mpmath.fsum(prop[: k + 1]) for k in range(len(prop))])

# Criterion 3: cmdscale's definition on eurodist.
d = square(r_input("eurodist"))
n = d.rows
b = mp.matrix(n, n)
for i in range(n):
    for j in range(n):
        b[i, j] = d[i, j] ** 2
for i in range(n):
    mean = mpmath.fsum(b[i, j] for j in range(n)) / n
    for j in range(n):
        b[i, j] -= mean
for j in range(n):
    mean = mpmath.fsum(b[i, j] for i in range(n)) / n
    for i in range(n):
        b[i, j] -= mean
b = -b / 2
e, q = mp.eigsy(b)
order = sorted(range(n), key=lambda i: -e[i])
eig = [e[i] for i in order]
pts = mp.matrix(n, 2)
for c in range(2):
    for i in range(n):
        pts[i, c] = q[i, order[c]] * mp.sqrt(eig[c])
out("mds.euro.points", rowmajor(signed(pts)))
out("mds.euro.eig", eig)
kept = mpmath.fsum(eig[:2])
out(
    "mds.euro.gof",
    [kept / mpmath.fsum(abs(v) for v in eig), kept / mpmath.fsum(max(v, 0) for v in eig)],
)
