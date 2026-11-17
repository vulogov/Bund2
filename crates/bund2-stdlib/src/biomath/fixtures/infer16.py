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
