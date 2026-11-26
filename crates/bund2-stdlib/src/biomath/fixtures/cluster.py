# RFC-0020 criterion 4's definitional fixture: cluster.dist's six
# distances from their definitions, by mpmath 1.3.0 at 50 significant
# digits.
#
#     python3 -I cluster.py > cluster.txt
#
# The inputs x and xc are read from cluster_r.txt, which cluster.R wrote,
# so this script needs no R. Each line is a label and a bracketed list,
# the full n x n matrix row-major, at 17 significant digits.
#
# The definitions, with no missing value (RFC-0020 §S5):
# - euclidean: sqrt(sum (a - b)^2); maximum: max |a - b|;
#   manhattan: sum |a - b|; minkowski: (sum |a - b|^p)^(1/p);
# - canberra: the sum of |a - b| / (|a| + |b|) over the coordinates where
#   a and b are not both 0, times nc / (the count of those coordinates);
# - binary: of the coordinates where a or b is non-zero, the share where
#   exactly one is; 0 if there are none.
import os

import mpmath
from mpmath import mp, mpf

mp.dps = 50

HERE = os.path.dirname(os.path.abspath(__file__))


def show(x):
    return mpmath.nstr(mpf(x), 17, min_fixed=-1, max_fixed=-1)


def out(label, xs):
    print(label, "[" + " ".join(show(v) for v in xs) + "]")


def r_input(label):
    with open(os.path.join(HERE, "cluster_r.txt")) as f:
        for line in f:
            if line.startswith(label + " "):
                return [mpf(v) for v in line[line.index("[") + 1 : line.rindex("]")].split()]
    raise KeyError(label)


def rows(xs, cols):
    return [xs[i : i + cols] for i in range(0, len(xs), cols)]


def euclidean(a, b):
    return mpmath.sqrt(sum((x - y) ** 2 for x, y in zip(a, b)))


def maximum(a, b):
    return max(abs(x - y) for x, y in zip(a, b))


def manhattan(a, b):
    return sum(abs(x - y) for x, y in zip(a, b))


def minkowski(p):
    return lambda a, b: sum(abs(x - y) ** p for x, y in zip(a, b)) ** (1 / p)


def canberra(a, b):
    terms = [abs(x - y) / (abs(x) + abs(y)) for x, y in zip(a, b) if not (x == 0 and y == 0)]
    return sum(terms) * len(a) / len(terms)


def binary(a, b):
    either = [(x, y) for x, y in zip(a, b) if x != 0 or y != 0]
    if not either:
        return mpf(0)
    return mpf(sum(1 for x, y in either if (x == 0) != (y == 0))) / len(either)


def table(f, xs):
    # The diagonal is 0 by definition, as dist's.
    return [mpf(0) if i == j else f(a, b) for i, a in enumerate(xs) for j, b in enumerate(xs)]


x = rows(r_input("x"), 3)
xc = rows(r_input("xc"), 3)
out("dist.euclidean", table(euclidean, x))
out("dist.maximum", table(maximum, x))
out("dist.manhattan", table(manhattan, x))
out("dist.binary", table(binary, x))
out("dist.minkowski.3", table(minkowski(mpf(3)), x))
out("dist.minkowski.0.5", table(minkowski(mpf("0.5")), x))
out("dist.canberra", table(canberra, xc))
