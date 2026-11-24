# RFC-0019 criterion 7's fixture for §S12: the one-way F (classical and
# Welch's) with eta^2 and omega^2, Tukey-Kramer's rows, and Cohen's d
# interval, from their definitions with mpmath 1.3.0 at 40 digits.
#
#     python3 -I anova.py > anova.txt
#
# The one-way F: SSB/(k - 1) over SSW/(N - k); Welch's F as oneway.test.R
# writes it; p by F's upper tail. eta^2 = SSB/SST and omega^2 =
# (SSB - (k - 1) MSE)/(SST + MSE). Tukey-Kramer: for each pair i > j, in
# R's order, diff = m_i - m_j, se = sqrt(MSE/2 (1/n_i + 1/n_j)), the
# interval diff -+ q se with q the studentized range's quantile at conf on
# N - k df, and p its upper tail at |diff|/se; the studentized range is
# tukey.py's definition (the double integral by tanh-sinh quadrature), and
# q its root by the secant method. Cohen's d interval: the noncentral t's
# ncp delta_L with P(T > t; delta_L) = alpha/2 and delta_U with
# P(T <= t; delta_U) = alpha/2, t = d sqrt(n_a n_b/(n_a + n_b)), each
# times sqrt(1/n_a + 1/n_b); the noncentral t's tails are noncentral.py's
# definition, a quadrature over the chi-squared.
#
# Each line is a Bund program, " | ", a key and its value to 17 digits. A
# frame's cell is keyed column:row, the row counted from 0.
import mpmath
from mpmath import mpf

mpmath.mp.dps = 40


def show(x):
    return mpmath.nstr(x, 17, min_fixed=-1, max_fixed=-1)


def f(s):
    return mpf(float(s))


def line(program, key, value):
    print(f"{program} | {key} {show(value)}")


def f_sf(x, d1, d2):
    return mpmath.betainc(d2 / 2, d1 / 2, 0, d2 / (d2 + d1 * x), regularized=True)


# The studentized range (tukey.py's definition).

def range_cdf(k, w, upper=False):
    if k == 2:
        closed = mpmath.erf(w / 2)
        return 1 - closed if upper else closed
    if upper:
        def g(z):
            q = mpmath.ncdf(-z)
            d = mpmath.ncdf(z + w) - mpmath.ncdf(z)
            return k * mpmath.npdf(z) * (q ** (k - 1) - d ** (k - 1))
    else:
        def g(z):
            return k * mpmath.npdf(z) * (mpmath.ncdf(z + w) - mpmath.ncdf(z)) ** (k - 1)
    m = -mpmath.sqrt(2 * mpmath.log(k))
    pts = sorted(set([-mpmath.inf, m - 4, m - 1, m, m + 1, m + 4, mpmath.inf]))
    return mpmath.quad(g, pts)


def s_pdf(s, nu):
    return mpmath.exp(
        mpmath.log(2) + (nu / 2) * mpmath.log(nu / 2) - mpmath.loggamma(nu / 2)
        + (nu - 1) * mpmath.log(s) - nu * s * s / 2
    )


def tukey_tail(k, nu, q, upper):
    k, nu, q = mpf(k), mpf(nu), mpf(q)
    sm = mpmath.sqrt((nu - 1) / nu)
    sd = 1 / mpmath.sqrt(2 * nu)
    pts = [0] + [sm + j * sd for j in (-6, -3, -1, 0, 1, 3, 6) if sm + j * sd > 0] + [mpmath.inf]
    return mpmath.quad(lambda s: s_pdf(s, nu) * range_cdf(k, q * s, upper), pts)


# The noncentral t (noncentral.py's definition).

def chi2_pdf(u, k):
    if u <= 0:
        return mpf(0)
    return mpmath.exp((k / 2 - 1) * mpmath.log(u) - u / 2 - (k / 2) * mpmath.log(2) - mpmath.loggamma(k / 2))


def nct_tail(v, d, t, upper):
    cuts = [mpf(0), v / 8, v / 2, v, 2 * v, 4 * v, 16 * v, 64 * v]
    if t != 0 and d / t > 0:
        cuts.append(v * (d / t) ** 2)
    cuts = sorted(set(cuts)) + [mpmath.inf]
    if upper:
        g = lambda u: mpmath.ncdf(d - t * mpmath.sqrt(u / v))
    else:
        g = lambda u: mpmath.ncdf(t * mpmath.sqrt(u / v) - d)
    return mpmath.quad(lambda u: g(u) * chi2_pdf(u, v), cuts)


# The data.

GROUPS = [
    ["4.2", "4.8", "5.1", "3.9", "4.4", "4.6"],
    ["6.1", "5.2", "7.9", "4.8", "6.6", "5.5", "6.0"],
    ["3.0", "3.4", "2.9", "3.3", "3.8"],
]
LITERAL = "[ " + " ".join("[ " + " ".join(g) + " ]" for g in GROUPS) + " ]"


def mean(xs):
    return sum(xs) / len(xs)


def ss(xs):
    m = mean(xs)
    return sum((x - m) ** 2 for x in xs)


g = [[f(t) for t in grp] for grp in GROUPS]
k = len(g)
n = [mpf(len(x)) for x in g]
m = [mean(x) for x in g]
N = sum(n)
grand = sum(sum(x) for x in g) / N
ssb = sum(ni * (mi - grand) ** 2 for ni, mi in zip(n, m))
ssw = sum(ss(x) for x in g)
sst = ssb + ssw
mse = ssw / (N - k)
eta2 = ssb / sst
omega2 = (ssb - (k - 1) * mse) / (sst + mse)

F = (ssb / (k - 1)) / mse
for program in [f"{LITERAL} stat.test.anova1",
                f'{LITERAL} dict "var.equal" true set stat.test.anova1.opts']:
    line(program, "statistic", F)
    line(program, "p", f_sf(F, k - 1, N - k))
    line(program, "df1", mpf(k - 1))
    line(program, "df2", N - k)
    line(program, "eta2", eta2)
    line(program, "omega2", omega2)

v = [ss(x) / (len(x) - 1) for x in g]
w = [ni / vi for ni, vi in zip(n, v)]
sw = sum(w)
tmp = sum((1 - wi / sw) ** 2 / (ni - 1) for wi, ni in zip(w, n)) / (k * k - 1)
mw = sum(wi * mi for wi, mi in zip(w, m)) / sw
FW = sum(wi * (mi - mw) ** 2 for wi, mi in zip(w, m)) / ((k - 1) * (1 + 2 * (k - 2) * tmp))
program = f'{LITERAL} dict "var.equal" false set stat.test.anova1.opts'
line(program, "statistic", FW)
line(program, "p", f_sf(FW, k - 1, 1 / (3 * tmp)))
line(program, "df1", mpf(k - 1))
line(program, "df2", 1 / (3 * tmp))
line(program, "eta2", eta2)
line(program, "omega2", omega2)

# Tukey-Kramer at conf 0.95 and 0.9.
nu = N - k
for conf in ["0.95", "0.9"]:
    c = f(conf)
    q = mpmath.findroot(lambda x: tukey_tail(k, nu, x, False) - c, (mpf(3.4), mpf(3.6)),
                        solver="secant", tol=mpf(10) ** -30)
    program = f'{LITERAL} dict "conf" {conf} set stat.test.tukey'
    row = 0
    for j in range(k):
        for i in range(j + 1, k):
            d = m[i] - m[j]
            se = mpmath.sqrt(mse / 2 * (1 / n[i] + 1 / n[j]))
            line(program, f"diff:{row}", d)
            line(program, f"lower:{row}", d - q * se)
            line(program, f"upper:{row}", d + q * se)
            line(program, f"p:{row}", tukey_tail(k, nu, abs(d) / se, True))
            row += 1

# Cohen's d and its interval.

def cohend(a_s, b_s, conf):
    a = [f(t) for t in a_s]
    b = [f(t) for t in b_s]
    na, nb = mpf(len(a)), mpf(len(b))
    df = na + nb - 2
    s = mpmath.sqrt((ss(a) + ss(b)) / df)
    d = (mean(a) - mean(b)) / s
    j = mpmath.exp(mpmath.loggamma(df / 2) - mpmath.loggamma((df - 1) / 2)) / mpmath.sqrt(df / 2)
    t = d * mpmath.sqrt(na * nb / (na + nb))
    alpha = 1 - f(conf)
    dl = mpmath.findroot(lambda x: nct_tail(df, x, t, True) - alpha / 2, (t - 1, t),
                         solver="anderson", tol=mpf(10) ** -30)
    du = mpmath.findroot(lambda x: nct_tail(df, x, t, False) - alpha / 2, (t, t + 1),
                         solver="anderson", tol=mpf(10) ** -30)
    back = mpmath.sqrt(1 / na + 1 / nb)
    program = (f"[ {' '.join(a_s)} ] [ {' '.join(b_s)} ] "
               f'dict "conf" {conf} set stat.effect.cohend.opts')
    line(program, "d", d)
    line(program, "g", j * d)
    line(program, "lower", dl * back)
    line(program, "upper", du * back)


cohend(GROUPS[1], GROUPS[0], "0.95")
cohend(GROUPS[2], GROUPS[0], "0.9")
cohend(["1.1", "2.3", "1.9", "2.8", "2.2"], ["2.0", "1.6", "2.9", "2.4"], "0.95")
