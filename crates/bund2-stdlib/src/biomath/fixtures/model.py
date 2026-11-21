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


lm("lm.plain", True, [DOSE, PATIENT], None, None)
lm("lm.weighted", True, [DOSE, PATIENT], W, None)
lm("lm.offset", True, [DOSE, PATIENT], W, OFF)
lm("lm.origin", False, [DOSE, PATIENT], W, OFF)
lm("lm.mean", True, [], W, OFF)
