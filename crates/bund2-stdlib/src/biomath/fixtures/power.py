# RFC-0019 criterion 4's fixture for power and sample size (§S4), computed
# with mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I power.py > power.txt
#
# Each line is a Bund program, " | ", the key it answers, and that key's
# value to 17 significant digits. Every value is a definition:
#
# - power.t: the noncentral t's upper tail above the central t's upper
#   alpha/sides quantile, nu = (n - 1) tsample and ncp = sqrt(n / tsample)
#   delta / sd, plus the lower tail below minus that quantile when `strict`
#   and two-sided. The noncentral tail is its definition,
#   P(T > t) = E[Phi(delta - t sqrt(V / nu))] over V ~ chi2(nu), by
#   quadrature, as noncentral.py's;
# - power.prop: R's normal approximation, written out;
# - power.anova: the noncentral F's upper tail, its closed-form density
#   (Kummer's 1F1) integrated, above the central F's upper alpha quantile,
#   with df1 = groups - 1, df2 = (n - 1) groups and
#   lambda = (groups - 1) n between.var / within.var;
# - power.chisq: the noncentral chi-squared's upper tail, its Bessel density
#   integrated, above the central chi-squared's upper alpha quantile on df,
#   with lambda = n w^2.
#
# A central quantile is bisection then Newton's steps on its closed-form
# tail (the regularised incomplete beta or gamma). A solved parameter is
# the secant method's root of power less its target, from two starting
# points near it, to 10^-40 of the root; nothing here uses Brent's method
# or an interval of R's. Each solved line is checked by putting the root
# back, to 10^-35.
import multiprocessing

import mpmath
from mpmath import mpf

mpmath.mp.dps = 50


def show(x):
    return mpmath.nstr(x, 17, min_fixed=-1, max_fixed=-1)


def chi2_pdf(u, k):
    if u <= 0:
        return mpf(0)
    return mpmath.exp((k / 2 - 1) * mpmath.log(u) - u / 2 - (k / 2) * mpmath.log(2) - mpmath.loggamma(k / 2))


def upper_quantile(sf, pdf, alpha, start):
    """x with sf(x) = alpha, sf decreasing."""
    lo, hi = start / 2, start * 2
    while sf(lo) < alpha:
        lo = lo / 2
    while sf(hi) > alpha:
        hi = hi * 2
    for _ in range(40):
        mid = (lo + hi) / 2
        if sf(mid) > alpha:
            lo = mid
        else:
            hi = mid
    x = (lo + hi) / 2
    for _ in range(30):
        step = (sf(x) - alpha) / pdf(x)
        x += step
        if abs(step) <= mpf(10) ** -45 * abs(x):
            break
    assert abs(sf(x) - alpha) < mpf(10) ** -42 * alpha, (alpha, x)
    return x


# Central families: upper tails and densities.


def t_sf(x, v):
    # x > 0 only, which is all an upper quantile at alpha < 1/2 needs.
    return mpmath.betainc(v / 2, mpf(1) / 2, 0, v / (v + x * x), regularized=True) / 2


def t_pdf(x, v):
    return mpmath.exp(
        mpmath.loggamma((v + 1) / 2) - mpmath.loggamma(v / 2) - mpmath.log(v * mpmath.pi) / 2
        - (v + 1) / 2 * mpmath.log(1 + x * x / v)
    )


def f_sf(x, d1, d2):
    return mpmath.betainc(d2 / 2, d1 / 2, 0, d2 / (d2 + d1 * x), regularized=True)


def f_pdf(x, d1, d2):
    return mpmath.exp(
        (d1 / 2) * mpmath.log(d1 / d2) + (d1 / 2 - 1) * mpmath.log(x)
        - ((d1 + d2) / 2) * mpmath.log(1 + d1 * x / d2) - mpmath.log(mpmath.beta(d1 / 2, d2 / 2))
    )


def chisq_sf(x, k):
    return mpmath.gammainc(k / 2, x / 2, mpmath.inf, regularized=True)


# Noncentral upper tails, as noncentral.py's.


def nct_sf(t, v, d):
    cuts = [mpf(0), v / 8, v / 2, v, 2 * v, 4 * v, 16 * v, 64 * v]
    if t != 0 and d / t > 0:
        cuts.append(v * (d / t) ** 2)
    cuts = sorted(set(cuts)) + [mpmath.inf]
    return mpmath.quad(lambda u: mpmath.ncdf(d - t * mpmath.sqrt(u / v)) * chi2_pdf(u, v), cuts)


def nct_cdf(t, v, d):
    cuts = [mpf(0), v / 8, v / 2, v, 2 * v, 4 * v, 16 * v, 64 * v]
    if t != 0 and d / t > 0:
        cuts.append(v * (d / t) ** 2)
    cuts = sorted(set(cuts)) + [mpmath.inf]
    return mpmath.quad(lambda u: mpmath.ncdf(t * mpmath.sqrt(u / v) - d) * chi2_pdf(u, v), cuts)


def ncf_sf(x, d1, d2, lam):
    def pdf(y):
        central = (
            (d1 / d2) ** (d1 / 2) * y ** (d1 / 2 - 1) * (1 + d1 * y / d2) ** (-(d1 + d2) / 2)
            / mpmath.beta(d1 / 2, d2 / 2)
        )
        return mpmath.exp(-lam / 2) * central * mpmath.hyp1f1((d1 + d2) / 2, d1 / 2, lam * d1 * y / (2 * (d2 + d1 * y)))

    m = (d1 + lam) / d1
    cuts = [m / 4, m / 2, m, 2 * m, 4 * m, 16 * m]
    return mpmath.quad(pdf, [x] + [c for c in cuts if c > x] + [mpmath.inf])


def ncchisq_sf(x, k, lam):
    def pdf(y):
        if lam == 0:
            return chi2_pdf(y, k)
        return mpmath.exp(-(y + lam) / 2) / 2 * (y / lam) ** (k / 4 - mpf(1) / 2) * mpmath.besseli(k / 2 - 1, mpmath.sqrt(lam * y))

    m, s = k + lam, mpmath.sqrt(2 * (k + 2 * lam))
    cuts = [m - 4 * s, m - s, m, m + s, m + 4 * s]
    return mpmath.quad(pdf, [x] + [c for c in cuts if c > x] + [mpmath.inf])


# The four powers, each over a dict of mpf parameters.


def power_t(p):
    tsample = {"two.sample": 2, "one.sample": 1, "paired": 1}[p["type"]]
    sides = p["sides"]
    delta = abs(p["delta"]) if sides == 2 else p["delta"]
    v = max(mpf("1e-7"), p["n"] - 1) * tsample
    alpha = p["sig.level"] / sides
    qu = upper_quantile(lambda x: t_sf(x, v), lambda x: t_pdf(x, v), alpha, mpf(2))
    ncp = mpmath.sqrt(p["n"] / tsample) * delta / p["sd"]
    power = nct_sf(qu, v, ncp)
    if p["strict"] and sides == 2:
        power += nct_cdf(-qu, v, ncp)
    return power


def z_upper(alpha):
    return mpmath.sqrt(2) * mpmath.erfinv(1 - 2 * alpha)


def power_prop(p):
    p1, p2, n, sides = p["p1"], p["p2"], p["n"], p["sides"]
    qu = z_upper(p["sig.level"] / sides)
    d = abs(p1 - p2)
    v1, v2 = p1 * (1 - p1), p2 * (1 - p2)
    pbar = (p1 + p2) / 2
    vbar = pbar * (1 - pbar)
    if p["strict"] and sides == 2:
        return mpmath.ncdf((mpmath.sqrt(n) * d - qu * mpmath.sqrt(2 * vbar)) / mpmath.sqrt(v1 + v2)) + mpmath.ncdf(
            -(mpmath.sqrt(n) * d + qu * mpmath.sqrt(2 * vbar)) / mpmath.sqrt(v1 + v2)
        )
    return mpmath.ncdf((mpmath.sqrt(n) * d - qu * mpmath.sqrt(2 * vbar)) / mpmath.sqrt(v1 + v2))


def power_anova(p):
    g, n = p["groups"], p["n"]
    d1, d2 = g - 1, (n - 1) * g
    lam = (g - 1) * n * p["between.var"] / p["within.var"]
    crit = upper_quantile(lambda x: f_sf(x, d1, d2), lambda x: f_pdf(x, d1, d2), p["sig.level"], mpf(2))
    return ncf_sf(crit, d1, d2, lam)


def power_chisq(p):
    k = p["df"]
    lam = p["n"] * p["w"] ** 2
    crit = upper_quantile(lambda x: chisq_sf(x, k), lambda x: chi2_pdf(x, k), p["sig.level"], k)
    return ncchisq_sf(crit, k, lam)


POWER = {"power.t": power_t, "power.prop": power_prop, "power.anova": power_anova, "power.chisq": power_chisq}
DEFAULTS = {
    "power.t": {"sd": "1.0", "sig.level": "0.05", "type": "two.sample", "sides": 2, "strict": False},
    "power.prop": {"sig.level": "0.05", "sides": 2, "strict": False},
    "power.anova": {"sig.level": "0.05"},
    "power.chisq": {"sig.level": "0.05"},
}

# (word, the parameters given, the one solved for, two starting points).
CASES = [
    ("power.t", {"n": "20", "delta": "1.0"}, "power", None),
    ("power.t", {"delta": "1.0", "power": "0.9"}, "n", ("22", "22.1")),
    ("power.t", {"delta": "1.0", "power": "0.9", "sides": 1}, "n", ("17.8", "17.9")),
    ("power.t", {"n": "15", "delta": "0.8", "sd": "1.5", "type": "one.sample", "strict": True}, "power", None),
    ("power.t", {"n": "20", "delta": "-0.5", "sides": 1, "type": "paired"}, "power", None),
    ("power.t", {"n": "12", "power": "0.8", "type": "paired"}, "delta", ("0.9", "0.95")),
    ("power.t", {"n": "30", "delta": "1.0", "sd": None, "power": "0.85"}, "sd", ("1.3", "1.35")),
    ("power.t", {"n": "20", "delta": "1.0", "power": "0.8", "sig.level": None}, "sig.level", ("0.02", "0.025")),
    ("power.prop", {"n": "50", "p1": "0.5", "p2": "0.75"}, "power", None),
    ("power.prop", {"p1": "0.5", "p2": "0.75", "power": "0.9"}, "n", ("76", "77")),
    ("power.prop", {"n": "50", "p1": "0.5", "power": "0.9"}, "p2", ("0.80", "0.81")),
    ("power.prop", {"n": "50", "p1": "0.5", "p2": "0.9", "power": "0.9", "sig.level": None}, "sig.level", ("0.0013", "0.00135")),
    ("power.prop", {"p1": "0.5", "p2": "0.501", "sig.level": "0.001", "power": "0.9"}, "n", ("10451936", "10451937")),
    ("power.prop", {"n": "100", "p2": "0.6", "power": "0.8"}, "p1", ("0.4", "0.41")),
    ("power.prop", {"n": "40", "p1": "0.3", "p2": "0.5", "strict": True}, "power", None),
    ("power.prop", {"n": "40", "p1": "0.3", "p2": "0.5", "sides": 1}, "power", None),
    ("power.anova", {"groups": "4", "n": "5", "between.var": "1.0", "within.var": "3.0"}, "power", None),
    ("power.anova", {"groups": "4", "between.var": "1.0", "within.var": "3.0", "power": "0.8"}, "n", ("11.9", "12")),
    # var(c(120, 130, 140, 150)) = 500/3, which Bund writes as a FLOAT.
    ("power.anova", {"groups": "4", "between.var": "166.66666666666666", "within.var": "500.0", "power": "0.9"}, "n", ("15.1", "15.2")),
    ("power.anova", {"n": "10", "between.var": "1.0", "within.var": "3.0", "power": "0.8"}, "groups", ("3", "3.5")),
    ("power.anova", {"groups": "4", "n": "10", "between.var": "1.0", "power": "0.8"}, "within.var", ("2", "2.2")),
    ("power.anova", {"groups": "4", "n": "10", "within.var": "3.0", "power": "0.8"}, "between.var", ("1.3", "1.4")),
    ("power.anova", {"groups": "4", "n": "10", "between.var": "1.0", "within.var": "3.0", "power": "0.8", "sig.level": None}, "sig.level", ("0.05", "0.06")),
    ("power.chisq", {"w": "0.3", "n": "100", "df": "2"}, "power", None),
    ("power.chisq", {"w": "0.3", "df": "2", "power": "0.8"}, "n", ("107", "108")),
    ("power.chisq", {"n": "100", "df": "3", "power": "0.8"}, "w", ("0.33", "0.34")),
    ("power.chisq", {"w": "0.3", "n": "100", "df": "2", "power": "0.8", "sig.level": None}, "sig.level", ("0.1", "0.11")),
]

INTEGERS = {"n", "groups", "df", "sides"}


def bund(word, given):
    """The Bund program building the options MAP and calling the word."""
    out = ["dict"]
    for k, x in given.items():
        if x is None:
            v = "na"
        elif isinstance(x, bool):
            v = "true" if x else "false"
        elif isinstance(x, int):
            v = str(x)
        elif k == "type":
            v = f'"{x}"'
        elif k in INTEGERS and "." not in x:
            v = x
        else:
            v = x if "." in x or "e" in x else x + ".0"
        out.append(f'"{k}" {v} set')
    out.append(word)
    return " ".join(out)


def params(word, given):
    p = dict(DEFAULTS[word])
    p.update({k: x for k, x in given.items() if x is not None})
    for k, x in p.items():
        if isinstance(x, str) and k != "type":
            p[k] = mpf(x)
    return p


def case(c):
    word, given, solve, start = c
    p = params(word, given)
    power = POWER[word]
    if solve == "power":
        value = power(p)
    else:
        target = p["power"]

        def g(x):
            q = dict(p)
            q[solve] = x
            return power(q) - target

        value = mpmath.findroot(g, (mpf(start[0]), mpf(start[1])), solver="secant", tol=mpf(10) ** -80, maxsteps=100)
        assert abs(g(value)) < mpf(10) ** -35, (word, given, g(value))
    return f"{bund(word, given)} | {solve} {show(value)}"


def main():
    with multiprocessing.Pool(min(len(CASES), multiprocessing.cpu_count())) as pool:
        for line in pool.map(case, CASES):
            print(line)


if __name__ == "__main__":
    main()
