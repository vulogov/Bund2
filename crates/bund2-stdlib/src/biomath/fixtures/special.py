# RFC-0015 criterion 7's fixture for math.betainc, math.gammainc and
# math.choose, computed with mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I special.py > special.txt
#
# Each line is a word, its operands in stack order (deepest first), and the
# answer to 17 significant digits. The test reads special.txt.
import mpmath

mpmath.mp.dps = 50


def show(x):
    return mpmath.nstr(x, 17, min_fixed=-1, max_fixed=-1)


# math.betainc: x a b -- I_x(a, b)
for x, a, b in [
    (0.5, 2, 3),
    (0.1, 0.5, 0.5),
    (0.9, 10, 2),
    (0.3, 50, 60),
    (0.999, 0.1, 7),
    (0.42, 1000, 1400),
]:
    v = mpmath.betainc(a, b, 0, mpmath.mpf(x), regularized=True)
    print("math.betainc", x, a, b, show(v))

# math.gammainc: x a -- P(a, x)
for x, a in [
    (1, 1),
    (0.5, 2.5),
    (10, 3),
    (3, 10),
    (99, 100),
    (1e-3, 0.01),
    (12000, 11900),
]:
    v = mpmath.gammainc(a, 0, mpmath.mpf(x), regularized=True)
    print("math.gammainc", x, a, show(v))

# math.choose at 1000 500, which is past u128 and so exp(ln C).
print("math.choose", 1000, 500, show(mpmath.binomial(1000, 500)))
print("math.lchoose", 1000, 500, show(mpmath.log(mpmath.binomial(1000, 500))))
