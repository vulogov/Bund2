# RFC-0015 criterion 10's fixture for the univariate families of §S7,
# computed with mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I dist.py > dist.txt
#
# Each line is a Bund program, `x <parameters> dist.<family> dist.<word>`,
# and its answer to 17 significant digits; the test runs every program and
# compares. The parameterisations are R's (dnorm, dlnorm, dunif, dexp, dgamma
# with a rate, dbeta, dchisq, dt, df, dbinom, dpois, dnbinom, dhyper with
# m = K, n = N - K, k = n, dgeom counting failures), so each answer is what R's
# d/p/q function would give, to far more digits than R's double precision.
# A parameter is taken at the f64 the Bund literal reads as, so the fixture
# answers the question Bund2 is asked, not a decimal neighbour of it.
import mpmath
from mpmath import mpf

mpmath.mp.dps = 50


def show(x):
    return mpmath.nstr(x, 17, min_fixed=-1, max_fixed=-1)


def f(s):
    return mpf(float(s))


def reg_beta(x, a, b):
    return mpmath.betainc(a, b, 0, x, regularized=True)


def reg_gamma(x, a):
    return mpmath.gammainc(a, 0, x, regularized=True)


# Continuous families: (name, parameters, pdf, cdf, support's lower end).


def normal(m, s):
    return (lambda x: mpmath.npdf(x, m, s), lambda x: mpmath.ncdf(x, m, s))


def lognormal(m, s):
    return (
        lambda x: mpmath.npdf(mpmath.log(x), m, s) / x if x > 0 else mpf(0),
        lambda x: mpmath.ncdf(mpmath.log(x), m, s) if x > 0 else mpf(0),
    )


def uniform(a, b):
    return (
        lambda x: 1 / (b - a) if a <= x <= b else mpf(0),
        lambda x: min(max((x - a) / (b - a), mpf(0)), mpf(1)),
    )


def exponential(r):
    return (
        lambda x: r * mpmath.exp(-r * x) if x >= 0 else mpf(0),
        lambda x: 1 - mpmath.exp(-r * x) if x >= 0 else mpf(0),
    )


def gamma(k, r):
    return (
        lambda x: r**k * x ** (k - 1) * mpmath.exp(-r * x) / mpmath.gamma(k) if x > 0 else mpf(0),
        lambda x: reg_gamma(r * x, k) if x > 0 else mpf(0),
    )


def beta(a, b):
    return (
        lambda x: x ** (a - 1) * (1 - x) ** (b - 1) / mpmath.beta(a, b) if 0 < x < 1 else mpf(0),
        lambda x: reg_beta(min(max(x, mpf(0)), mpf(1)), a, b),
    )


def chisq(k):
    return gamma(k / 2, mpf(1) / 2)


def t(v):
    def cdf(x):
        tail = reg_beta(v / (v + x * x), v / 2, mpf(1) / 2) / 2
        return tail if x < 0 else 1 - tail

    return (
        lambda x: mpmath.gamma((v + 1) / 2)
        / (mpmath.sqrt(v * mpmath.pi) * mpmath.gamma(v / 2))
        * (1 + x * x / v) ** (-(v + 1) / 2),
        cdf,
    )


def fdist(d1, d2):
    return (
        lambda x: (d1 / d2) ** (d1 / 2)
        * x ** (d1 / 2 - 1)
        * (1 + d1 * x / d2) ** (-(d1 + d2) / 2)
        / mpmath.beta(d1 / 2, d2 / 2)
        if x > 0
        else mpf(0),
        lambda x: reg_beta(d1 * x / (d1 * x + d2), d1 / 2, d2 / 2) if x > 0 else mpf(0),
    )


CONTINUOUS = [
    ("normal", ["1.0", "2.0"], normal, ["-1.5", "0.5", "4.0"]),
    ("lognormal", ["0.5", "0.75"], lognormal, ["0.5", "1.5", "5.0"]),
    ("uniform", ["-1.0", "3.0"], uniform, ["-0.5", "1.0", "2.5"]),
    ("exponential", ["1.5"], exponential, ["0.1", "1.0", "3.0"]),
    ("gamma", ["2.5", "1.5"], gamma, ["0.3", "1.5", "4.0"]),
    ("beta", ["2.0", "3.5"], beta, ["0.1", "0.35", "0.8"]),
    ("chisq", ["4.5"], chisq, ["0.5", "3.0", "12.0"]),
    ("t", ["5.5"], t, ["-2.5", "0.5", "3.0"]),
    ("f", ["4.0", "7.5"], fdist, ["0.2", "1.0", "4.0"]),
]

# Discrete families: (name, parameters, pmf, a bound past which the mass is
# negligible to 50 digits).


def binomial(n, p):
    return lambda k: mpmath.binomial(n, k) * p**k * (1 - p) ** (n - k) if 0 <= k <= n else mpf(0)


def poisson(lam):
    return lambda k: lam**k * mpmath.exp(-lam) / mpmath.factorial(k)


def negbinomial(r, p):
    return lambda k: mpmath.gamma(k + r) / (mpmath.gamma(r) * mpmath.factorial(k)) * p**r * (1 - p) ** k


def hypergeometric(N, K, n):
    return (
        lambda x: mpmath.binomial(K, x) * mpmath.binomial(N - K, n - x) / mpmath.binomial(N, n)
        if max(0, n - (N - K)) <= x <= min(n, K)
        else mpf(0)
    )


def geometric(p):
    return lambda k: p * (1 - p) ** k


def bernoulli(p):
    return lambda k: [1 - p, p][int(k)] if k in (0, 1) else mpf(0)


DISCRETE = [
    ("binomial", ["20", "0.3"], lambda n, p: binomial(int(n), p), ["2", "6", "11"], 20),
    ("poisson", ["4.2"], poisson, ["0", "4", "9"], 200),
    ("negbinomial", ["3.5", "0.4"], negbinomial, ["0", "4", "12"], 600),
    ("hypergeometric", ["50", "12", "10"], lambda N, K, n: hypergeometric(int(N), int(K), int(n)), ["0", "2", "5"], 10),
    ("geometric", ["0.2"], geometric, ["0", "3", "10"], 1200),
    ("bernoulli", ["0.3"], bernoulli, ["0", "1"], 1),
]

PS = ["0.001", "0.5", "0.999"]


def line(program, value):
    print(program, show(value))


for name, params, family, xs in CONTINUOUS:
    pdf, cdf = family(*[f(p) for p in params])
    head = " ".join(params) + f" dist.{name}"
    for x in xs:
        line(f"{x} {head} dist.pdf", pdf(f(x)))
        line(f"{x} {head} dist.cdf", cdf(f(x)))
        line(f"{x} {head} dist.sf", 1 - cdf(f(x)))
    for p in PS:
        target = f(p)
        lo, hi = mpf(-1), mpf(1)
        while cdf(lo) > target:
            lo *= 2
        while cdf(hi) < target:
            hi *= 2
        # Bisection, 200 halvings: the bracket's width falls below 10^-55 of
        # its start, past the 50 digits the arithmetic carries.
        for _ in range(200):
            mid = (lo + hi) / 2
            if cdf(mid) < target:
                lo = mid
            else:
                hi = mid
        q = (lo + hi) / 2
        # The t's median is 0; near it the cdf's 1 - v / (v + x^2) loses to
        # the working precision what the bisection would resolve.
        if abs(q) < mpf(10) ** -20:
            q = mpf(0)
        assert abs(cdf(q) - target) < mpf(10) ** -40, (name, p)
        line(f"{p} {head} dist.quantile", q)

for name, params, family, xs, bound in DISCRETE:
    pmf = family(*[f(p) for p in params])
    head = " ".join(params) + f" dist.{name}"
    masses = [pmf(mpf(k)) for k in range(bound + 1)]
    for x in xs:
        k = int(x)
        line(f"{x} {head} dist.pdf", masses[k])
        line(f"{x} {head} dist.cdf", mpmath.fsum(masses[: k + 1]))
        line(f"{x} {head} dist.sf", 1 - mpmath.fsum(masses[: k + 1]))
    for p in PS:
        target, total, k = f(p), mpf(0), 0
        while True:
            total += masses[k]
            if total >= target:
                break
            k += 1
        line(f"{p} {head} dist.quantile", mpf(k))

# The multivariate densities: R's dmultinom, and the Dirichlet's closed form.
x, size, prob = [1, 2, 3], 6, [f("0.2"), f("0.3"), f("0.5")]
v = mpmath.factorial(size) / mpmath.fprod(mpmath.factorial(k) for k in x)
v *= mpmath.fprod(p**k for p, k in zip(prob, x))
print("multinomial 6 [0.2 0.3 0.5] at [1 2 3]", show(v))
alpha, at = [f("2.0"), f("3.0"), f("4.0")], [f("0.2"), f("0.3"), f("0.5")]
v = mpmath.gamma(sum(alpha)) / mpmath.fprod(mpmath.gamma(a) for a in alpha)
v *= mpmath.fprod(y ** (a - 1) for y, a in zip(at, alpha))
print("dirichlet [2.0 3.0 4.0] at [0.2 0.3 0.5]", show(v))
