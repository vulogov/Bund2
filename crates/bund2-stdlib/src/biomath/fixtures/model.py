# RFC-0018's fixture for the model words, by mpmath 1.3.0 at 50 significant
# digits.
#
#     python3 -I model.py > model.txt
#
# R is not installed where this was written, so each answer is computed here
# from the definition RFC-0018 states for it, as fit.py's and infer.py's
# are. M2 writes the fit.lm part (criterion 2); later milestones add to it.
#
# fit.lm (RFC-0018 S3): weighted least squares, minimising
# sum w (y - offset - x b)^2, solved exactly by the normal equations:
#
# - coef = (X'WX)^-1 X'W z, z = y - offset; RSS = sum w r^2, r = z - X coef;
# - sigma^2 = RSS/(n - k); vcov = sigma^2 (X'WX)^-1; se its diagonal's root;
# - t = coef/se; p two-sided on n - k df;
# - TSS = sum w (z - m)^2, m the weighted mean of z with an intercept and 0
#   without; r2 = 1 - RSS/TSS; adj.r2 = 1 - (1 - r2)(n - i)/(n - k);
# - f = ((TSS - RSS)/(k - i))/(RSS/(n - k)), f.p its upper F tail;
# - aic = n(ln(2 pi RSS/n) + 1) - sum ln w + 2(k + 1);
#   bic = aic - 2(k + 1) + (k + 1) ln n;
# - hat_i = w_i x_i' (X'WX)^-1 x_i; fitted = X coef + offset;
# - null.deviance = TSS.
#
# The data: the seven complete rows of bund_examples/frames/'s dose trial,
# with the patient number as a second covariate, and weights and an offset
# made up for the test.
import mpmath
from mpmath import mpf, matrix

mpmath.mp.dps = 50

DOSE = [10, 20, 10, 10, 20, 30, 30]
PATIENT = [1, 2, 3, 5, 6, 7, 8]
Y = ["4.2", "5.9", "3.8", "4.6", "6.3", "7.1", "6.8"]
W = ["1", "2", "1", "3", "1", "2", "1"]
OFF = ["0.1", "0", "0.2", "0", "0.1", "0", "0.3"]


def show(label, xs):
    print(label, "[" + " ".join(mpmath.nstr(x, 17, min_fixed=-30, max_fixed=30)
                                 for x in xs) + "]")


def t_two_sided(t, df):
    return mpmath.betainc(df / 2, mpf(1) / 2, 0, df / (df + t * t),
                          regularized=True)


def f_upper(f, d1, d2):
    return mpmath.betainc(mpf(d2) / 2, mpf(d1) / 2, 0, d2 / (d2 + d1 * f),
                          regularized=True)


def lm(label, intercept, cols, weights, offset):
    n = len(Y)
    y = [mpf(v) for v in Y]
    w = [mpf(v) for v in weights] if weights else [mpf(1)] * n
    off = [mpf(v) for v in offset] if offset else [mpf(0)] * n
    rows = [([mpf(1)] if intercept else []) + [mpf(c[r]) for c in cols]
            for r in range(n)]
    k = len(rows[0])
    i = 1 if intercept else 0
    z = [y[r] - off[r] for r in range(n)]
    xtwx = matrix(k, k)
    xtwz = matrix(k, 1)
    for r in range(n):
        for a in range(k):
            xtwz[a] += w[r] * rows[r][a] * z[r]
            for b in range(k):
                xtwx[a, b] += w[r] * rows[r][a] * rows[r][b]
    inv = xtwx ** -1
    coef = inv * xtwz
    b = [coef[a] for a in range(k)]
    res = [z[r] - sum(rows[r][a] * b[a] for a in range(k)) for r in range(n)]
    rss = sum(w[r] * res[r] ** 2 for r in range(n))
    df = n - k
    s2 = rss / df
    se = [mpmath.sqrt(s2 * inv[a, a]) for a in range(k)]
    t = [b[a] / se[a] for a in range(k)]
    p = [t_two_sided(x, df) for x in t]
    m = sum(w[r] * z[r] for r in range(n)) / sum(w) if intercept else mpf(0)
    tss = sum(w[r] * (z[r] - m) ** 2 for r in range(n))
    show(f"{label} coef", b)
    show(f"{label} se", se)
    show(f"{label} t", t)
    show(f"{label} p", p)
    show(f"{label} sigma", [mpmath.sqrt(s2)])
    show(f"{label} deviance", [rss])
    show(f"{label} null.deviance", [tss])
    if k > i:
        r2 = 1 - rss / tss
        adj = 1 - (1 - r2) * (n - i) / mpf(n - k)
        f = ((tss - rss) / (k - i)) / (rss / (n - k))
        show(f"{label} r2", [r2])
        show(f"{label} adj.r2", [adj])
        show(f"{label} f", [f])
        show(f"{label} f.p", [f_upper(f, k - i, n - k)])
    aic = n * (mpmath.log(2 * mpmath.pi * rss / n) + 1) \
        - sum(mpmath.log(x) for x in w) + 2 * (k + 1)
    show(f"{label} aic", [aic])
    show(f"{label} bic", [aic - 2 * (k + 1) + (k + 1) * mpmath.log(n)])
    show(f"{label} vcov", [s2 * inv[a, c] for a in range(k) for c in range(k)])
    show(f"{label} hat", [w[r] * sum(rows[r][a] * inv[a, c] * rows[r][c]
                                     for a in range(k) for c in range(k))
                          for r in range(n)])
    show(f"{label} fitted", [sum(rows[r][a] * b[a] for a in range(k)) + off[r]
                             for r in range(n)])
    mu = [sum(rows[r][a] * b[a] for a in range(k)) + off[r] for r in range(n)]
    return {"family": "gaussian", "link": "identity", "y": y, "w": w, "off": off,
            "mu": mu, "eta": mu, "beta": b, "cov": [[s2 * inv[a, c] for c in range(k)]
                                                    for a in range(k)],
            "phi": s2, "df": df, "k": k, "dev": rss, "nd": tss,
            "hat": [w[r] * sum(rows[r][a] * inv[a, c] * rows[r][c]
                               for a in range(k) for c in range(k)) for r in range(n)]}


LM_PLAIN = lm("lm.plain", True, [DOSE, PATIENT], None, None)
LM_WEIGHTED = lm("lm.weighted", True, [DOSE, PATIENT], W, None)
lm("lm.offset", True, [DOSE, PATIENT], W, OFF)
lm("lm.origin", False, [DOSE, PATIENT], W, OFF)
lm("lm.mean", True, [], W, OFF)


# fit.glm (RFC-0018 S4): IRLS from the family's mustart, eta = g(mustart);
# each step z = (eta - offset) + (y - mu)/mu'(eta), W = w mu'^2/V(mu), the
# weighted least squares solved exactly; a step whose deviance is not
# finite, then one whose eta or mu leaves the valid range, halved towards
# the previous coefficients (R's two loops, glm.R:271-303); stopped when
# |dev - devold|/(0.1 + |dev|) < 1e-8. The links' clamps are R's, logit's
# fit.logistic's continuous one at -ln eps. Run at 50 digits, so the
# iterate printed is where the same rule stops the same iteration, not the
# exact maximum-likelihood estimate (S4, "Where IRLS stops").

EPS = mpf(2) ** -52
ETA_LIMIT = -mpmath.log(EPS)
PROBIT = -mpmath.sqrt(2) * mpmath.erfinv(2 * EPS - 1)
CAUCHIT = -mpmath.tan(mpmath.pi * (EPS - mpf(1) / 2))


def clamp(x, lo, hi):
    return min(max(x, lo), hi)


def phi(x):
    return mpmath.erfc(-x / mpmath.sqrt(2)) / 2


LINKS = {
    "logit": (lambda m: mpmath.log(m / (1 - m)),
              lambda e: 1 / (1 + mpmath.exp(-clamp(e, -ETA_LIMIT, ETA_LIMIT))),
              None, lambda e: True),
    "probit": (lambda m: mpmath.sqrt(2) * mpmath.erfinv(2 * m - 1),
               lambda e: phi(clamp(e, -PROBIT, PROBIT)),
               lambda e: max(mpmath.exp(-e * e / 2) / mpmath.sqrt(2 * mpmath.pi), EPS),
               lambda e: True),
    "cloglog": (lambda m: mpmath.log(-mpmath.log(1 - m)),
                lambda e: max(min(-mpmath.expm1(-mpmath.exp(e)), 1 - EPS), EPS),
                lambda e: max(mpmath.exp(min(e, 700)) * mpmath.exp(-mpmath.exp(min(e, 700))), EPS),
                lambda e: True),
    "identity": (lambda m: m, lambda e: e, lambda e: mpf(1), lambda e: True),
    "log": (lambda m: mpmath.log(m), lambda e: max(mpmath.exp(e), EPS),
            lambda e: max(mpmath.exp(e), EPS), lambda e: True),
    "sqrt": (lambda m: mpmath.sqrt(m), lambda e: e * e, lambda e: 2 * e,
             lambda e: e > 0),
    "1/mu^2": (lambda m: 1 / m ** 2, lambda e: 1 / mpmath.sqrt(e),
               lambda e: -1 / (2 * e ** mpf(1.5)), lambda e: e > 0),
    "inverse": (lambda m: 1 / m, lambda e: 1 / e, lambda e: -1 / e ** 2,
                lambda e: e != 0),
}


def mu_eta(link, e):
    g, inv, d, _ = LINKS[link]
    if link == "logit":
        m = inv(e)
        return m * (1 - m)
    return d(e)


def ylogy(y, m):
    return y * mpmath.log(y / m) if y != 0 else mpf(0)


FAMILIES = {
    "binomial": (lambda m: m * (1 - m), lambda m: 0 < m < 1,
                 lambda y, m, w: 2 * w * (ylogy(y, m) + ylogy(1 - y, 1 - m)),
                 lambda y, w: (w * y + mpf(1) / 2) / (w + 1)),
    "poisson": (lambda m: m, lambda m: m > 0,
                lambda y, m, w: 2 * w * (y * mpmath.log(y / m) - (y - m)) if y > 0 else 2 * m * w,
                lambda y, w: y + mpf(1) / 10),
    "gaussian": (lambda m: mpf(1), lambda m: True,
                 lambda y, m, w: w * (y - m) ** 2, lambda y, w: y),
    "gamma": (lambda m: m ** 2, lambda m: m > 0,
              lambda y, m, w: -2 * w * (mpmath.log(y / m) - (y - m) / m),
              lambda y, w: y),
    "inverse.gaussian": (lambda m: m ** 3, lambda m: m > 0,
                         lambda y, m, w: w * (y - m) ** 2 / (y * m ** 2),
                         lambda y, w: y),
}


class FirstStep(Exception):
    pass


def solve(rows, w, z):
    k = len(rows[0])
    a = matrix(k, k)
    b = matrix(k, 1)
    for r in range(len(rows)):
        for i in range(k):
            b[i] += w[r] * rows[r][i] * z[r]
            for j in range(k):
                a[i, j] += w[r] * rows[r][i] * rows[r][j]
    inv = a ** -1
    return [x for x in inv * b], inv


def irls(family, link, rows, y, w, off, start):
    var, validmu, devres, _ = FAMILIES[family]
    g, inv, _, valideta = LINKS[link]
    n = len(y)

    def pred(beta):
        return [sum(rows[r][i] * beta[i] for i in range(len(beta))) + off[r]
                for r in range(n)]

    def dev(mu):
        return sum(devres(y[r], mu[r], w[r]) for r in range(n))

    def valid(eta, mu):
        return all(valideta(e) for e in eta) and all(validmu(m) for m in mu)

    def finite(x):
        return mpmath.isfinite(x) and not mpmath.isnan(x)

    eta = [g(m) for m in start]
    mu = [inv(e) for e in eta]
    devold = dev(mu)
    coefold = None
    halvings = 0
    for it in range(1, 101):
        d = [mu_eta(link, e) for e in eta]
        z = [(eta[r] - off[r]) + (y[r] - mu[r]) / d[r] for r in range(n)]
        ww = [w[r] * d[r] ** 2 / var(mu[r]) for r in range(n)]
        beta, _ = solve(rows, ww, z)
        eta = pred(beta)
        mu = [inv(e) for e in eta]
        try:
            dv = dev(mu)
        except (ValueError, ZeroDivisionError):
            dv = mpmath.nan
        if not finite(dv):
            if coefold is None:
                raise FirstStep("deviance")
            while not finite(dv):
                halvings += 1
                beta = [(beta[i] + coefold[i]) / 2 for i in range(len(beta))]
                eta = pred(beta)
                mu = [inv(e) for e in eta]
                try:
                    dv = dev(mu)
                except (ValueError, ZeroDivisionError):
                    dv = mpmath.nan
        if not valid(eta, mu):
            if coefold is None:
                raise FirstStep("range")
            while not valid(eta, mu):
                halvings += 1
                beta = [(beta[i] + coefold[i]) / 2 for i in range(len(beta))]
                eta = pred(beta)
                mu = [inv(e) for e in eta]
            dv = dev(mu)
        if abs(dv - devold) / (mpf(1) / 10 + abs(dv)) < mpf("1e-8"):
            return beta, eta, mu, dv, it, halvings
        devold = dv
        coefold = beta
    raise RuntimeError("no convergence")


def aic(family, y, mu, w, dv, k):
    n = len(y)
    sw = sum(w)
    if family == "gaussian":
        core = n * (mpmath.log(dv / n * 2 * mpmath.pi) + 1) + 2 - sum(mpmath.log(x) for x in w)
    elif family == "poisson":
        if any(abs(v - mpmath.nint(v)) > mpf("1e-9") * max(1, abs(v)) for v in y):
            return None
        core = -2 * sum(w[r] * (mpmath.nint(y[r]) * mpmath.log(mu[r]) - mu[r]
                                - mpmath.loggamma(mpmath.nint(y[r]) + 1)) for r in range(n))
    elif family == "binomial":
        core = mpf(0)
        for r in range(n):
            m, s = mpmath.nint(w[r]), mpmath.nint(w[r] * y[r])
            ld = mpmath.loggamma(m + 1) - mpmath.loggamma(s + 1) - mpmath.loggamma(m - s + 1)
            if s > 0:
                ld += s * mpmath.log(mu[r])
            if m - s > 0:
                ld += (m - s) * mpmath.log(1 - mu[r])
            core += ld
        core = -2 * core
    elif family == "gamma":
        disp = dv / sw
        shape = 1 / disp
        core = -2 * sum(w[r] * (-mpmath.loggamma(shape) - shape * mpmath.log(mu[r] * disp)
                                + (shape - 1) * mpmath.log(y[r]) - y[r] / (mu[r] * disp))
                        for r in range(n)) + 2
    else:
        core = sw * (1 + mpmath.log(dv / sw * 2 * mpmath.pi)) + 3 * sum(mpmath.log(y[r]) * w[r] for r in range(n)) + 2
    return core + 2 * k


def glm(label, family, link, cols, y, w=None, off=None, intercept=True):
    n = len(y)
    y = [mpf(v) for v in y]
    w = [mpf(v) for v in w] if w else [mpf(1)] * n
    off = [mpf(v) for v in off] if off else [mpf(0)] * n
    rows = [([mpf(1)] if intercept else []) + [mpf(c[r]) for c in cols] for r in range(n)]
    k = len(rows[0])
    var, _, devres, mustart = FAMILIES[family]
    start = [mustart(y[r], w[r]) for r in range(n)]
    beta, eta, mu, dv, it, halvings = irls(family, link, rows, y, w, off, start)
    ww = [w[r] * mu_eta(link, eta[r]) ** 2 / var(mu[r]) for r in range(n)]
    _, inv = solve(rows, ww, [mpf(0)] * n)
    estimated = family not in ("binomial", "poisson")
    df = n - k
    phi_ = (sum(w[r] * (y[r] - mu[r]) ** 2 / var(mu[r]) for r in range(n)) / df
            if estimated else mpf(1))
    se = [mpmath.sqrt(phi_ * inv[i, i]) for i in range(k)]
    stat = [beta[i] / se[i] for i in range(k)]
    if estimated:
        p = [t_two_sided(s, df) for s in stat]
    else:
        p = [mpmath.erfc(abs(s) / mpmath.sqrt(2)) for s in stat]
    if not intercept:
        inv_ = LINKS[link][1]
        nd = sum(devres(y[r], inv_(off[r]), w[r]) for r in range(n))
    elif all(o == 0 for o in off) and not GIVEN_OFFSET.get(label):
        m = sum(w[r] * y[r] for r in range(n)) / sum(w)
        nd = sum(devres(y[r], m, w[r]) for r in range(n))
    else:
        nd = irls(family, link, [[mpf(1)] for _ in range(n)], y, w, off, mu)[3]
    show(f"{label} coef", beta)
    show(f"{label} se", se)
    show(f"{label} {'t' if estimated else 'z'}", stat)
    show(f"{label} p", p)
    show(f"{label} deviance", [dv])
    show(f"{label} null.deviance", [nd])
    show(f"{label} dispersion", [phi_])
    a = aic(family, y, mu, w, dv, k)
    if a is not None:
        pk = k + (1 if estimated else 0)
        show(f"{label} aic", [a])
        show(f"{label} bic", [a - 2 * pk + pk * mpmath.log(n)])
    show(f"{label} fitted", mu)
    show(f"{label} iterations", [it])
    show(f"{label} halvings", [halvings])
    return {"family": family, "link": link, "y": y, "w": w, "off": off, "mu": mu,
            "eta": eta, "beta": beta, "phi": phi_, "df": df, "k": k, "dev": dv, "nd": nd,
            "cov": [[phi_ * inv[a, c] for c in range(k)] for a in range(k)],
            "hat": [ww[r] * sum(rows[r][a] * inv[a, c] * rows[r][c]
                                for a in range(k) for c in range(k)) for r in range(n)]}


GIVEN_OFFSET = {"glm.rate": True, "glm.rate0": True}

# R's own example, glm.Rd:335-340: Dobson's randomised controlled trial.
COUNTS = [18, 17, 15, 20, 10, 20, 25, 13, 12]
OUT2 = [0, 1, 0, 0, 1, 0, 0, 1, 0]
OUT3 = [0, 0, 1, 0, 0, 1, 0, 0, 1]
TR2 = [0, 0, 0, 1, 1, 1, 0, 0, 0]
TR3 = [0, 0, 0, 0, 0, 0, 1, 1, 1]
DOBSON = glm("glm.dobson", "poisson", "log", [OUT2, OUT3, TR2, TR3], COUNTS)

# A rate model: counts over exposures, log exposure the offset.
RATE_Y = [2, 3, 6, 7, 8, 9, 10, 12, 15]
EXPO = [10, 17, 26, 34, 40, 43, 50, 60, 75]
X9 = list(range(1, 10))
glm("glm.rate", "poisson", "log", [X9], RATE_Y,
    off=[mpmath.log(mpf(e)) for e in EXPO])

# Proportions with their trials as weights.
TRIALS = [10, 12, 15, 9, 20, 11]
SUCC = [1, 3, 6, 5, 15, 10]
PROP = [mpf(s) / t for s, t in zip(SUCC, TRIALS)]
X6 = list(range(1, 7))
for lk in ("logit", "probit", "cloglog"):
    glm(f"glm.binomial.{lk}", "binomial", lk, [X6], PROP, w=TRIALS)

POS = ["2.1", "3.4", "1.9", "5.6", "4.2", "6.8", "7.5", "3.3"]
X8 = list(range(1, 9))
GAMMA_INV = glm("glm.gamma.inverse", "gamma", "inverse", [X8], POS)
glm("glm.gamma.log", "gamma", "log", [X8], POS)
glm("glm.ig", "inverse.gaussian", "1/mu^2", [X8], POS)
glm("glm.gaussian.log", "gaussian", "log", [X8], POS)

# Step halving: a Poisson identity model whose second step leaves mu > 0.
glm("glm.halving", "poisson", "identity", [X8], [1, 1, 2, 0, 6, 2, 0, 6])

# The zero group: an intercept and an indicator, counts 3, 5, 4, 0, 0, 0.
ZERO = glm("glm.zero", "poisson", "log", [[0, 0, 0, 1, 1, 1]], [3, 5, 4, 0, 0, 0])


# A Poisson sqrt model whose step leaves eta <= 0 and is halved, converging
# with every mu inside the range, and a Poisson identity model whose first
# step leaves it, at x = 1..6. The sqrt input is the result of a search,
# run by `python3 -I model.py --search` (some minutes): of the counts in
# {0, 1, 2, 4, 7, 12, 20}^6 whose fit is halved and converges with every mu
# above 1e-6, the fewest halvings, then the fewest iterations, then the
# first in the search's order. A halved fit that converges towards eta = 0
# instead, as most do, is refused as "fitted rate numerically 0" (S4). The
# first-step input is the first, in the same order over {0, 1, 2, 3, 5, 8},
# with at least three nonzero counts.
import itertools
import sys


def fits(family, link, values):
    for ys in itertools.product(values, repeat=6):
        y = [mpf(v) for v in ys]
        rows = [[mpf(1), mpf(x)] for x in X6]
        start = [FAMILIES[family][3](v, mpf(1)) for v in y]
        try:
            yield ys, irls(family, link, rows, y, [mpf(1)] * 6, [mpf(0)] * 6, start)
        except FirstStep:
            yield ys, None
        except (RuntimeError, ZeroDivisionError, ValueError):
            continue


if "--search" in sys.argv:
    found = [(r[5], r[4], ys) for ys, r in fits("poisson", "sqrt", [0, 1, 2, 4, 7, 12, 20])
             if r is not None and r[5] > 0 and min(r[2]) > mpf("1e-6")]
    print("glm.sqrt.y", list(min(found)[2]))
    sys.exit(0)

SQRT_Y = [2, 0, 0, 7, 20, 20]
print("glm.sqrt.y", "[" + " ".join(str(v) for v in SQRT_Y) + "]")
glm("glm.sqrt", "poisson", "sqrt", [X6], SQRT_Y)
FIRST_Y = next(list(ys) for ys, r in fits("poisson", "identity", [0, 1, 2, 3, 5, 8])
               if r is None and sum(1 for v in ys if v) >= 3)
print("glm.first.y", "[" + " ".join(str(v) for v in FIRST_Y) + "]")


# model.residuals and model.influence (RFC-0018 S5), from each fit's
# estimate: "response" y - mu, "pearson" (y - mu) sqrt(w)/sqrt(V(mu)),
# "deviance" sign(y - mu) sqrt(d_i), "working" (y - mu)/mu'(eta),
# "standardized" the deviance residual over sqrt(phi (1 - h_i)); Cook's
# distance (r_P/(1 - h))^2 h/(phi k).

def residuals(label, st):
    var, _, devres, _ = FAMILIES[st["family"]]
    y, mu, w, eta, h = st["y"], st["mu"], st["w"], st["eta"], st["hat"]
    n = len(y)
    resp = [y[r] - mu[r] for r in range(n)]
    pear = [(y[r] - mu[r]) * mpmath.sqrt(w[r]) / mpmath.sqrt(var(mu[r])) for r in range(n)]
    dev = [mpmath.sign(y[r] - mu[r]) * mpmath.sqrt(max(devres(y[r], mu[r], w[r]), 0))
           for r in range(n)]
    work = [(y[r] - mu[r]) / mu_eta(st["link"], eta[r]) for r in range(n)]
    std = [dev[r] / mpmath.sqrt(st["phi"] * (1 - h[r])) for r in range(n)]
    cooks = [(pear[r] / (1 - h[r])) ** 2 * h[r] / (st["phi"] * st["k"]) for r in range(n)]
    for key, xs in [("response", resp), ("pearson", pear), ("deviance", dev),
                    ("working", work), ("standardized", std), ("hat", h),
                    ("cooks", cooks)]:
        show(f"{label} resid.{key}", xs)


residuals("glm.dobson", DOBSON)
residuals("glm.gamma.inverse", GAMMA_INV)
residuals("lm.weighted", LM_WEIGHTED)


# model.predict (RFC-0018 S6) at new rows: on the link scale eta = x b +
# offset and se = sqrt(x' V x); on the response scale mu = g^-1(eta) and
# se |mu'(eta)|. A confidence interval is eta -/+ q se, q normal when phi
# is fixed and t on the residual df when estimated, at (1 + conf)/2, its
# ends mapped through g^-1, lower the smaller; a prediction interval adds
# phi/w to the variance, w the new row's weight.

def quantile(p, df):
    if df is None:
        return mpmath.sqrt(2) * mpmath.erfinv(2 * p - 1)
    return mpmath.findroot(lambda t: 1 - t_two_sided(t, df) / 2 - p, 2)


def predict(label, st, xs, ws=None, conf=mpf("0.95"), prediction=False):
    inv = LINKS[st["link"]][1]
    df = st["df"] if st["family"] not in ("binomial", "poisson") else None
    q = quantile((1 + conf) / 2, df)
    k = st["k"]
    out = {key: [] for key in ("link.fit", "link.se", "link.lower", "link.upper",
                                "response.fit", "response.se", "response.lower",
                                "response.upper", "pred.lower", "pred.upper")}
    for j, x in enumerate(xs):
        x = [mpf(v) for v in x]
        eta = sum(x[a] * st["beta"][a] for a in range(k))
        se = mpmath.sqrt(sum(x[a] * st["cov"][a][c] * x[c] for a in range(k) for c in range(k)))
        lo, hi = eta - q * se, eta + q * se
        out["link.fit"].append(eta)
        out["link.se"].append(se)
        out["link.lower"].append(lo)
        out["link.upper"].append(hi)
        out["response.fit"].append(inv(eta))
        out["response.se"].append(se * abs(mu_eta(st["link"], eta)))
        a, b = inv(lo), inv(hi)
        out["response.lower"].append(min(a, b))
        out["response.upper"].append(max(a, b))
        if prediction:
            w = mpf(ws[j]) if ws else mpf(1)
            half = q * mpmath.sqrt(se ** 2 + st["phi"] / w)
            out["pred.lower"].append(eta - half)
            out["pred.upper"].append(eta + half)
    for key, v in out.items():
        if v:
            show(f"{label} predict.{key}", v)


# Dobson at (outcome, treatment) = (1, 1), (2, 3), (3, 2): the intercept,
# outcome2, outcome3, treatment2, treatment3.
predict("glm.dobson", DOBSON, [[1, 0, 0, 0, 0], [1, 1, 0, 0, 1], [1, 0, 1, 1, 0]])
predict("glm.gamma.inverse", GAMMA_INV, [[1, "1.5"], [1, "4.5"], [1, 7]])
NEW_LM = [[1, 15, 4], [1, 25, 9], [1, 5, 1]]
predict("lm.plain", LM_PLAIN, NEW_LM, prediction=True)
predict("lm.weighted", LM_WEIGHTED, NEW_LM, prediction=True)
predict("lm.weighted.w", LM_WEIGHTED, NEW_LM, ws=[2, 1, 3], prediction=True)
# The zero group's upper end: exp of about 4.8e4, so the key is left out;
# its lower end, through log's clamp, is eps.
predict("glm.zero", ZERO, [[1, 1]])


# model.compare and model.anova (RFC-0018 S7): R's stat.anova (anova.R:34-50).
# A difference in deviance, taken as 0 when negative, over the larger
# model's dispersion phi: LR is delta/phi on chi^2(df); F is
# (delta/df)/phi on df and the larger model's residual df. A row of
# model.anova is a model through one more term, the first the null model;
# its tests take the full model's phi and residual df (glm.R:548-551).

def chisq_upper(x, df):
    return mpmath.gammainc(mpf(df) / 2, x / 2, mpmath.inf, regularized=True)


def tested(test, delta, df, phi, df2):
    delta = max(delta, mpf(0))
    if delta == 0:
        return delta, mpf(0), mpf(1)
    if test == "F":
        stat = (delta / df) / phi
        return delta, stat, f_upper(stat, df, df2)
    stat = delta / phi
    return delta, stat, chisq_upper(stat, df)


def compare(label, test, small, big):
    df = small["df"] - big["df"]
    phi = big["phi"] if big["family"] not in ("binomial", "poisson") else mpf(1)
    d, st, p = tested(test, small["dev"] - big["dev"], df, phi, big["df"])
    show(f"compare.{label} deviance", [d])
    show(f"compare.{label} statistic", [st])
    show(f"compare.{label} p", [p])


def anova(label, test, null, n_null, fits):
    full = fits[-1]
    phi = full["phi"] if full["family"] not in ("binomial", "poisson") else mpf(1)
    devs = [null] + [f["dev"] for f in fits]
    dfs = [n_null] + [f["df"] for f in fits]
    rows = [tested(test, devs[r - 1] - devs[r], dfs[r - 1] - dfs[r], phi, full["df"])
            for r in range(1, len(devs))]
    show(f"anova.{label} resid.deviance", devs)
    show(f"anova.{label} deviance", [r[0] for r in rows])
    show(f"anova.{label} statistic", [r[1] for r in rows])
    show(f"anova.{label} p", [r[2] for r in rows])


# Dobson without and with treatment: LR by default, and the frame.
DOBSON_OUT = glm("glm.dobson.outcome", "poisson", "log", [OUT2, OUT3], COUNTS)
compare("dobson", "LR", DOBSON_OUT, DOBSON)
anova("dobson", "LR", DOBSON["nd"], 8, [DOBSON_OUT, DOBSON])
# Without an intercept: the null model at linkinv(0) = 1, outcome coded
# fully.
OUT1 = [1, 0, 0, 1, 0, 0, 1, 0, 0]
D0_OUT = glm("glm.dobson0.outcome", "poisson", "log", [OUT1, OUT2, OUT3], COUNTS,
             intercept=False)
D0 = glm("glm.dobson0", "poisson", "log", [OUT1, OUT2, OUT3, TR2, TR3], COUNTS,
         intercept=False)
NULL0 = sum(FAMILIES["poisson"][2](mpf(y), mpf(1), mpf(1)) for y in COUNTS)
anova("dobson0", "LR", NULL0, 9, [D0_OUT, D0])

# Gamma: the intercept-only model against x, F by default; and a design of
# x then z, each row's F on the full model's phi and residual df.
GAMMA_NULL = glm("glm.gamma.null", "gamma", "inverse", [], POS)
compare("gamma", "F", GAMMA_NULL, GAMMA_INV)
Z8 = [0, 1, 1, 0, 1, 0, 0, 1]
GAMMA_XZ = glm("glm.gamma.xz", "gamma", "inverse", [X8, Z8], POS)
anova("gamma", "F", GAMMA_XZ["nd"], 7, [GAMMA_INV, GAMMA_XZ])

# Gaussian identity: fit.lm's least squares, dose then patient.
LM_DOSE = lm("lm.dose", True, [DOSE], None, None)
anova("lm", "F", LM_PLAIN["nd"], 6, [LM_DOSE, LM_PLAIN])
