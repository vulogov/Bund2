# RFC-0019 criterion 2's fixture for the noncentral t, chi-squared and F
# (§S2), computed with mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I noncentral.py > noncentral.txt
#
# Each line is a Bund program and its answer to 17 significant digits, as
# dist.py's are. Nothing here sums the Poisson series Bund2 sums, so the
# fixture is a check on it rather than a copy of it:
#
# - the densities are the closed forms: the chi-squared's by the modified
#   Bessel function, the F's and the t's by Kummer's 1F1;
# - the tails are those densities integrated by quadrature, and the t's tails
#   the definition, P(T <= t) = E[Phi(t sqrt(V / nu) - delta)] over V, a
#   central chi-squared on nu degrees of freedom; the upper tail is
#   integrated directly, not taken as 1 less the lower;
# - the means and variances are integrals of x and x^2 against the density;
# - a quantile is the root of the integrated cdf less p.
#
# Each pair of tails is checked to sum to 1 within 1e-30, which a failed
# quadrature would not.
import mpmath
from mpmath import mpf

mpmath.mp.dps = 50


def show(x):
    return mpmath.nstr(x, 17, min_fixed=-1, max_fixed=-1)


def f(s):
    return mpf(float(s))


def chi2_pdf(u, k):
    if u <= 0:
        return mpf(0)
    return mpmath.exp((k / 2 - 1) * mpmath.log(u) - u / 2 - (k / 2) * mpmath.log(2) - mpmath.loggamma(k / 2))


# Each family: (pdf, cdf, sf, the points quadrature splits at).


def ncchisq(k, lam):
    def pdf(x):
        if x <= 0:
            return mpf(0)
        if lam == 0:
            return chi2_pdf(x, k)
        return (
            mpmath.exp(-(x + lam) / 2) / 2 * (x / lam) ** (k / 4 - mpf(1) / 2) * mpmath.besseli(k / 2 - 1, mpmath.sqrt(lam * x))
        )

    m, s = k + lam, mpmath.sqrt(2 * (k + 2 * lam))
    cuts = sorted({mpf(0), max(m - 4 * s, mpf(0)), max(m - s, mpf(0)), m, m + s, m + 4 * s})

    def from_zero(a):
        # Over [0, a] as s = u^4, which takes the pole x^(k/2 - 1) at 0 off
        # the integrand for k >= 1/2.
        return mpmath.quad(lambda u: pdf(u**4) * 4 * u**3, [0, a ** (mpf(1) / 4)])

    def cdf(x):
        if x <= 0:
            return mpf(0)
        first = min(x, mpf(1))
        return from_zero(first) + (mpmath.quad(pdf, [first] + [c for c in cuts if first < c < x] + [x]) if x > first else 0)

    def sf(x):
        if x <= 0:
            return mpf(1)
        return mpmath.quad(pdf, [x] + [c for c in cuts if c > x] + [mpmath.inf])

    return pdf, cdf, sf, cuts


def ncf(d1, d2, lam):
    def pdf(x):
        if x <= 0:
            return mpf(0)
        central = (
            (d1 / d2) ** (d1 / 2)
            * x ** (d1 / 2 - 1)
            * (1 + d1 * x / d2) ** (-(d1 + d2) / 2)
            / mpmath.beta(d1 / 2, d2 / 2)
        )
        return mpmath.exp(-lam / 2) * central * mpmath.hyp1f1((d1 + d2) / 2, d1 / 2, lam * d1 * x / (2 * (d2 + d1 * x)))

    m = (d1 + lam) / d1
    cuts = [mpf(0), m / 4, m / 2, m, 2 * m, 4 * m, 16 * m]

    def cdf(x):
        if x <= 0:
            return mpf(0)
        return mpmath.quad(pdf, [c for c in cuts if c < x] + [x])

    def sf(x):
        if x <= 0:
            return mpf(1)
        return mpmath.quad(pdf, [x] + [c for c in cuts if c > x] + [mpmath.inf])

    return pdf, cdf, sf, cuts


def nct(v, d):
    def pdf(x):
        r = v + x * x
        z = d * d * x * x / (2 * r)
        front = v ** (v / 2) * mpmath.gamma(v + 1) * mpmath.exp(-d * d / 2) / (2**v * r ** (v / 2) * mpmath.gamma(v / 2))
        odd = mpmath.sqrt(2) * d * x / r * mpmath.hyp1f1(v / 2 + 1, mpf(3) / 2, z) / mpmath.gamma((v + 1) / 2)
        even = mpmath.hyp1f1((v + 1) / 2, mpf(1) / 2, z) / (mpmath.sqrt(r) * mpmath.gamma(v / 2 + 1))
        return front * (odd + even)

    # The chi-squared's mass, for the quadrature's cuts over V.
    cuts_v = [mpf(0), v / 8, v / 2, v, 2 * v, 4 * v, 16 * v, 64 * v]

    def over_v(g, t):
        cuts = list(cuts_v)
        if t != 0 and d / t > 0:
            # Where t sqrt(u / nu) - delta crosses 0.
            cuts.append(v * (d / t) ** 2)
        cuts = sorted(set(cuts)) + [mpmath.inf]
        return mpmath.quad(lambda u: g(u) * chi2_pdf(u, v), cuts)

    def cdf(t):
        return over_v(lambda u: mpmath.ncdf(t * mpmath.sqrt(u / v) - d), t)

    def sf(t):
        return over_v(lambda u: mpmath.ncdf(d - t * mpmath.sqrt(u / v)), t)

    return pdf, cdf, sf, None


FAMILIES = [
    ("nct", ["5.5", "1.5"], nct, ["-1.0", "2.0", "6.0"]),
    ("nct", ["10.0", "-2.0"], nct, ["-4.0", "-1.0", "1.0"]),
    ("nct", ["8.0", "100.0"], nct, ["60.0", "100.0", "160.0"]),
    ("nct", ["4.0", "0.0"], nct, ["-1.0", "0.5", "2.0"]),
    ("ncchisq", ["4.5", "2.5"], ncchisq, ["1.0", "6.0", "15.0"]),
    ("ncchisq", ["3.0", "100.0"], ncchisq, ["70.0", "103.0", "150.0"]),
    ("ncchisq", ["2.0", "0.0"], ncchisq, ["0.5", "2.0", "6.0"]),
    ("ncchisq", ["0.5", "1.0"], ncchisq, ["0.05", "1.0", "5.0"]),
    ("ncf", ["4.0", "7.5", "3.0"], ncf, ["0.3", "1.5", "5.0"]),
    ("ncf", ["5.0", "12.0", "100.0"], ncf, ["10.0", "20.0", "40.0"]),
    ("ncf", ["3.0", "10.0", "0.0"], ncf, ["0.2", "1.0", "4.0"]),
]

PS = ["0.001", "0.5", "0.999"]


def line(program, value):
    print(program, show(value))


def quantile(pdf, cdf, p, start):
    target = f(p)
    # A bracket by doubling outward from the start, 40 halvings of it, then
    # Newton's steps on the closed-form density until a step is below
    # 10^-45 of the root.
    lo, hi = start - 1, start + 1
    while cdf(lo) > target:
        lo = lo - 2 * (hi - lo)
    while cdf(hi) < target:
        hi = hi + 2 * (hi - lo)
    for _ in range(40):
        mid = (lo + hi) / 2
        if cdf(mid) < target:
            lo = mid
        else:
            hi = mid
    q = (lo + hi) / 2
    for _ in range(20):
        step = (cdf(q) - target) / pdf(q)
        q -= step
        if abs(step) <= mpf(10) ** -45 * max(abs(q), mpf(1)):
            break
    assert abs(cdf(q) - target) < mpf(10) ** -40, p
    return q


for name, params, family, xs in FAMILIES:
    ps = [f(p) for p in params]
    pdf, cdf, sf, _ = family(*ps)
    head = " ".join(params) + f" dist.{name}"
    for x in xs:
        lower, upper = cdf(f(x)), sf(f(x))
        assert abs(lower + upper - 1) < mpf(10) ** -30, (name, params, x)
        line(f"{x} {head} dist.pdf", pdf(f(x)))
        line(f"{x} {head} dist.cdf", lower)
        line(f"{x} {head} dist.sf", upper)
    support = mpf(0) if name != "nct" else -mpmath.inf
    m = mpmath.quad(lambda x: x * pdf(x), [support, 0, mpmath.inf] if name == "nct" else [0, mpmath.inf])
    for p in PS:
        start = max(m, mpf(1) / 4) if name != "nct" else m
        q = quantile(pdf, cdf, p, start)
        line(f"{p} {head} dist.quantile", q)
    # The moments by integration, where they exist: the t's mean needs
    # df > 1 and its variance df > 2, the F's df2 > 2 and df2 > 4; every
    # family here has them.
    if name == "nct":
        span = sorted({-mpmath.inf, mpf(0), ps[1] - 10, ps[1], ps[1] + 10, mpmath.inf})
    elif name == "ncchisq":
        span = sorted({mpf(0), ps[0] + ps[1], mpmath.inf})
    else:
        mid = (ps[0] + ps[2]) / ps[0]
        span = sorted({mpf(0), mid, 4 * mid, mpmath.inf})
    mean = mpmath.quad(lambda x: x * pdf(x), span)
    second = mpmath.quad(lambda x: x * x * pdf(x), span)
    line(f"{head} dist.mean", mean)
    line(f"{head} dist.var", second - mean * mean)
