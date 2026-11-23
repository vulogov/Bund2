# The fixture for the t family where its cdf and quantile were wrong: a large
# df (150 to 1e30, either side of the switch to the expansion at 1000), where
# ln B(ν/2, ½) as a difference of lgammas cancels and the continued fraction
# at ν/(ν + x²) near 1 loses a factor ν, and p near ½, where the quantile is
# near 0 and 1/(1 + x²/ν) rounded to 1.
# Computed with mpmath 1.3.0 at 120 digits, so that ν/(ν + x²) at ν = 1e30
# keeps 50 digits beyond its rounding.
#
#     python3 -I student.py > student.txt
#
# The cdf is ½ + x·Γ((ν+1)/2)/(√(νπ)·Γ(ν/2))·₂F₁(½, (ν+1)/2; 3/2; −x²/ν),
# which has no ν/(ν + x²) to cancel and is independent of the incomplete
# beta Bund2 uses. Each line is a Bund program and its answer to 17
# significant digits; parameters are taken at the f64 the literal reads as.
import mpmath
from mpmath import mpf

mpmath.mp.dps = 120


def show(x):
    return mpmath.nstr(x, 17, min_fixed=-1, max_fixed=-1)


def f(s):
    return mpf(float(s))


def lc(v):
    return mpmath.loggamma((v + 1) / 2) - mpmath.loggamma(v / 2) - mpmath.log(v * mpmath.pi) / 2


def pdf(v, x):
    return mpmath.exp(lc(v) - (v + 1) / 2 * mpmath.log1p(x * x / v))


def cdf(v, x):
    return mpf(1) / 2 + x * mpmath.exp(lc(v)) * mpmath.hyp2f1(
        mpf(1) / 2, (v + 1) / 2, mpf(3) / 2, -x * x / v
    )


def quantile(v, p):
    # Bisection to 400 halvings from a bracket around the normal's quantile.
    lo, hi = mpf(-64), mpf(64)
    for _ in range(400):
        mid = (lo + hi) / 2
        if cdf(v, mid) < p:
            lo = mid
        else:
            hi = mid
    q = (lo + hi) / 2
    assert abs(cdf(v, q) - p) < mpf(10) ** -100, (v, p)
    return q


DFS = ["150.0", "1000.0", "2.0e6", "1.0e8", "1.0e12", "1.0e30"]
XS = ["-3.0", "1.0e-9", "0.5", "7.0"]
PS = ["0.001", "0.4999999", "0.5000000001", "0.975"]

for d in DFS:
    v = f(d)
    for x in XS:
        print(f"{x} {d} dist.t dist.pdf", show(pdf(v, f(x))))
        print(f"{x} {d} dist.t dist.cdf", show(cdf(v, f(x))))
        print(f"{x} {d} dist.t dist.sf", show(1 - cdf(v, f(x))))
    for p in PS:
        print(f"{p} {d} dist.t dist.quantile", show(quantile(v, f(p))))

# p near ½ at a small df, where the quantile is near 0 too.
for d in ["1.0", "5.5", "30.0"]:
    for p in ["0.4999999", "0.5000000001"]:
        print(f"{p} {d} dist.t dist.quantile", show(quantile(f(d), f(p))))
