# RFC-0019 criterion 10's fixture for §S8: the bootstrap's bias, standard
# error and intervals on committed replicate and jackknife vectors, and the
# jackknife's leave-one-out values, from their definitions with mpmath 1.3.0
# at 50 digits.
#
#     python3 -I boot.py > boot.txt
#
# The intervals are boot 1.3-32's (R/bootfuns.q): norm.ci's
# t0 - bias -+ z se, z = qnorm((1 + conf)/2), se^2 the replicates' variance
# on R - 1; norm.inter's order statistic (R + 1) alpha, taken as it is when
# whole, the first below 1 and the last at R, else interpolated between its
# neighbours on the normal-quantile scale; basic.ci's 2 t0 - q, perc.ci's q,
# and bca.ci's w = qnorm(#{t < t0}/R) and adjusted levels
# pnorm(w + (w + z)/(1 - a (w + z))), with the jackknife's acceleration
# a = sum (tbar - t_i)^3 / (6 (sum (tbar - t_i)^2)^1.5) (RFC-0019 N4, A).
#
# Each input is a double, written as Rust and Python both read it, and taken
# into mpmath exactly, and printed as given. Each line is `case key value`;
# an interval's ends are `kind.lower` and `kind.upper`.

from mpmath import mp, mpf, sqrt, erfinv, ncdf, floor

mp.dps = 50

# The replicate vectors, t0 and the jackknife vectors: none of them drawn,
# so the test runs the interval function on exactly these.
CASES = {
    "a": {
        "t0": "5.03",
        "t": [
            "4.71", "5.22", "4.98", "5.41", "4.87", "5.09", "5.30", "4.62",
            "5.15", "4.95", "5.27", "5.01", "4.79", "5.36", "5.12", "4.91",
            "5.05", "5.48", "4.84",
        ],
        "jack": [
            "5.02", "5.09", "4.98", "5.06", "5.01", "5.11", "4.95", "5.04",
            "5.00", "5.07", "4.99", "5.05",
        ],
        "conf": ["0.95", "0.9", "0.8"],
    },
    "b": {
        "t0": "1.75",
        "t": [
            "1.62", "1.93", "1.81", "1.55", "1.77", "2.04", "1.69", "1.88",
            "1.71", "1.59", "1.96", "1.84", "1.66", "1.79", "2.11", "1.73",
            "1.91", "1.64", "1.86", "1.70", "1.98", "1.58", "1.82", "1.76",
            "2.07", "1.68", "1.89", "1.61", "1.80", "1.95", "1.74", "1.67",
            "2.01", "1.83", "1.72", "1.90", "1.63", "1.87", "1.78", "1.99",
        ],
        "jack": [
            "1.71", "1.79", "1.74", "1.83", "1.69", "1.76", "1.72", "1.81",
            "1.68", "1.77",
        ],
        "conf": ["0.95", "0.9"],
    },
}

# The jackknife's data: (case, data, statistic).
JACK = [
    ("m", ["2.1", "3.4", "1.9", "4.2", "2.8", "3.1", "2.5"], "mean"),
    ("d", ["2.1", "3.4", "1.9", "4.2", "2.8", "3.1", "2.5"], "median"),
    ("s", ["2.1", "3.4", "1.9", "4.2", "2.8", "3.1", "2.5", "3.9"], "sd"),
    ("v", ["2.1", "3.4", "1.9", "4.2", "2.8", "3.1", "2.5", "3.9"], "var"),
]


def x(s):
    return mpf(float(s))


def qnorm(p):
    return sqrt(2) * erfinv(2 * p - 1)


def mean(xs):
    return sum(xs) / len(xs)


def var(xs):
    m = mean(xs)
    return sum((v - m) ** 2 for v in xs) / (len(xs) - 1)


def median(xs):
    s = sorted(xs)
    n = len(s)
    return s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2


STAT = {"mean": mean, "median": median, "var": var, "sd": lambda xs: sqrt(var(xs))}


def norm_inter(t, alpha):
    r = len(t)
    rk = (r + 1) * alpha
    k = int(floor(rk))
    if k == 0:
        return t[0]
    if k >= r:
        return t[r - 1]
    if k == rk:
        return t[k - 1]
    z, z2, z3 = qnorm(alpha), qnorm(mpf(k) / (r + 1)), qnorm(mpf(k + 1) / (r + 1))
    tk, tk1 = t[k - 1], t[k]
    return tk + (z - z2) / (z3 - z2) * (tk1 - tk)


def out(case, key, v):
    print(f"{case} {key} {mp.nstr(v, 17, min_fixed=-30, max_fixed=30)}")


for name, c in CASES.items():
    t0 = x(c["t0"])
    t = [x(v) for v in c["t"]]
    jack = [x(v) for v in c["jack"]]
    s = sorted(t)
    print(f"{name} t0 {c['t0']}")
    print(f"{name} t [ {' '.join(c['t'])} ]")
    print(f"{name} jack [ {' '.join(c['jack'])} ]")
    bias = mean(t) - t0
    se = sqrt(var(t))
    out(name, "bias", bias)
    out(name, "se", se)
    tbar = mean(jack)
    a = sum((tbar - v) ** 3 for v in jack) / (6 * sum((tbar - v) ** 2 for v in jack) ** mpf(1.5))
    w = qnorm(mpf(sum(1 for v in t if v < t0)) / len(t))
    for conf_s in c["conf"]:
        conf = x(conf_s)
        lo, hi = (1 - conf) / 2, (1 + conf) / 2
        case = f"{name}@{conf_s}"
        merr = se * qnorm(hi)
        ends = {
            "normal": (t0 - bias - merr, t0 - bias + merr),
            "basic": (2 * t0 - norm_inter(s, hi), 2 * t0 - norm_inter(s, lo)),
            "percentile": (norm_inter(s, lo), norm_inter(s, hi)),
        }
        bca = []
        for level in (lo, hi):
            z = qnorm(level)
            bca.append(norm_inter(s, ncdf(w + (w + z) / (1 - a * (w + z)))))
        ends["bca"] = tuple(bca)
        for kind, (l, u) in ends.items():
            out(case, f"{kind}.lower", l)
            out(case, f"{kind}.upper", u)

for name, data, stat in JACK:
    xs = [x(v) for v in data]
    f = STAT[stat]
    t0 = f(xs)
    values = [f(xs[:i] + xs[i + 1:]) for i in range(len(xs))]
    n = len(xs)
    tbar = mean(values)
    print(f"{name} data [ {' '.join(data)} ]")
    print(f"{name} statistic {stat}")
    out(name, "t0", t0)
    for i, v in enumerate(values):
        out(name, f"values:{i}", v)
    out(name, "bias", (n - 1) * (tbar - t0))
    out(name, "se", sqrt(mpf(n - 1) / n * sum((v - tbar) ** 2 for v in values)))
