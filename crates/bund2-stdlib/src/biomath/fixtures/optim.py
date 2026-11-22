# RFC-0019 criterion 8's fixture for optim.root and optim.integrate (§S6).
#
#     python3 -I optim.py > optim.txt
#
# Each line is a label and its values. Three kinds:
#
# - `root <name> <x>`: a root to 17 significant digits, from its closed
#   form or mpmath 1.3.0's findroot at 50 digits;
# - `integral <name> <value>`: a closed form at 50 digits, to 17;
# - `extend <name> <lower> <upper> <steps>`: where R's `uniroot` extension
#   (R 4.4.2, src/library/stats/R/nlm.R, the `doX` block) leaves the
#   interval, replayed here step for step in IEEE doubles, Python's floats
#   being R's: each end moves by delta = 0.01 * max(1e-4, |end|), delta
#   doubling each step; "yes" moves both ends at once while f has one sign
#   at both, "upX" and "downX" first the lower end while sig * f(lower) > 0,
#   then the upper while sig * f(upper) < 0, sig being 1 and -1.
#
# The functions are the ones the test runs as lambdas, named below.
import mpmath
from mpmath import mpf

mpmath.mp.dps = 50


def show(x):
    return mpmath.nstr(mpf(x), 17, min_fixed=-1, max_fixed=-1)


def delta(u):
    return 0.01 * max(1e-4, abs(u))


def extend(f, lower, upper, how, maxiter=1000):
    fl, fu = f(lower), f(upper)
    it = 0
    if how == "yes":
        dl, du = delta(lower), delta(upper)
        while fl * fu > 0:
            it += 1
            assert it <= maxiter
            lower -= dl
            fl = f(lower)
            upper += du
            fu = f(upper)
            dl, du = 2 * dl, 2 * du
    else:
        sig = 1.0 if how == "upX" else -1.0
        d = delta(lower)
        while sig * fl > 0:
            it += 1
            assert it <= maxiter
            lower -= d
            fl = f(lower)
            d *= 2
        d = delta(upper)
        while sig * fu < 0:
            it += 1
            assert it <= maxiter
            upper += d
            fu = f(upper)
            d *= 2
    assert not (fl > 0 and fu > 0) and not (fl < 0 and fu < 0)
    return lower, upper, it


EXTENSIONS = [
    # name, f, lower, upper, extend
    ("yes", lambda x: x - 10.0, 0.0, 1.0, "yes"),
    ("upX.lower", lambda x: x + 4.0, 0.0, 1.0, "upX"),
    ("upX.upper", lambda x: x - 10.0, 0.0, 1.0, "upX"),
    ("downX", lambda x: 7.0 - x, 0.0, 1.0, "downX"),
]


def main():
    print(f"root cubic {show(mpmath.cbrt(2))}")
    print(f"root cos {show(mpmath.findroot(lambda x: mpmath.cos(x) - x, 0.7))}")
    print(f"root yes {show(10)}")
    print(f"root upX.lower {show(-4)}")
    print(f"root upX.upper {show(10)}")
    print(f"root downX {show(7)}")
    for name, f, lo, hi, how in EXTENSIONS:
        a, b, it = extend(f, lo, hi, how)
        print(f"extend {name} {a!r} {b!r} {it}")
    # x^3 - 2x + 1 over [0, 3]: 81/4 - 9 + 3.
    print(f"integral polynomial {show(mpf(81) / 4 - 9 + 3)}")
    print(f"integral gaussian {show(mpmath.sqrt(mpmath.pi))}")
    print(f"integral rsqrt {show(2)}")
    # 1/((x - 0.3)^2 + 1e-4) over [0, 1]: 100 (atan(70) + atan(30)).
    print(f"integral peak {show(100 * (mpmath.atan(70) + mpmath.atan(30)))}")
    print(f"integral exp.upper {show(1)}")
    print(f"integral exp.lower {show(1)}")


main()
