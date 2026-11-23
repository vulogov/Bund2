# RFC-0019 criterion 5's fixture for §S5's rank and table tests, computed
# with mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I further.py > further.txt
#
# Each line is a Bund program, " | ", the key it answers, and that key's
# value to 17 significant digits. Every value is a definition:
#
# - the signed-rank test: V = the sum of the ranks of |x - mu| over the
#   positive differences, zeros dropped. Exact, its distribution is that of
#   the sum of a random subset of 1..n, counted by enumerating the 2^n
#   subsets (n <= 16) or by expanding prod (1 + q^j) in integers (larger n),
#   and the two-sided p is 2 min(P(V <= v), P(V >= v)), at most 1. Normal,
#   z = (V - n(n+1)/4 - c) / sigma, sigma^2 = n(n+1)(2n+1)/24 - sum(t^3 - t)/48
#   over the groups of tied |x - mu|, c the continuity correction;
# - its estimate is the Hodges-Lehmann pseudomedian, the median of the Walsh
#   averages (x_i + x_j)/2, i <= j, of x with the zeros dropped and mu added
#   back. The exact interval is [A_(q), A_(N - q + 1)] over the sorted Walsh
#   averages A, N = n(n+1)/2, q the least with P(V <= q) >= alpha (alpha/2
#   two-sided), 1 at least. The normal interval's ends are where the
#   corrected z of x - d crosses the normal quantile: z is a step function
#   of d, so each end is the step, found by evaluating z on each side of
#   every Walsh average;
# - Kruskal-Wallis: H = (12/(n(n+1)) sum R_i^2/n_i - 3(n+1)) / (1 - sum(t^3 - t)/(n^3 - n));
# - Friedman: 12 sum (R_j - n(k+1)/2)^2 / (n k (k+1) - sum(t^3 - t)/(k - 1)),
#   ranks within each row, ties over each row's groups;
# - McNemar: sum over i < j of (|x_ij - x_ji| - 1)^2 / (x_ij + x_ji) on a
#   2x2 table that is not symmetric with the correction, the -1 left out
#   otherwise, on r(r-1)/2 df;
# - CMH: (|sum (a_k - E a_k)| - 1/2)^2 / sum Var a_k, the 1/2 only when the
#   sum's size is at least 1/2, with E a_k = (a+b)(a+c)/n and
#   Var a_k = (a+b)(c+d)(a+c)(b+d) / (n^2 (n-1)); the Mantel-Haenszel odds
#   ratio sum(ad/n) / sum(bc/n) and Robins, Breslow and Greenland's standard
#   error of its log, the interval exp(log OR -/+ z sd).
#
# Every chi-squared tail is the regularised upper incomplete gamma, every
# normal tail erfc. CMH's lines write each stratum as a LIST of two rows,
# which D271 reads as a MATRIX is read.
import itertools

import mpmath
from mpmath import mpf

mpmath.mp.dps = 50


def show(x):
    return mpmath.nstr(mpf(x), 17, min_fixed=-1, max_fixed=-1)


def bund_list(xs):
    return "[ " + " ".join(xs) + " ]"


def bund_matrix(rows):
    return "[ " + " ".join(bund_list(r) for r in rows) + " ] matrix"


def opts(d):
    s = "dict"
    for k, v in d.items():
        if isinstance(v, bool):
            v = "true" if v else "false"
        elif isinstance(v, str):
            v = '"' + v + '"'
        s += f' "{k}" {v} set'
    return s


def phi(z):
    return mpmath.erfc(-z / mpmath.sqrt(2)) / 2


def chisq_sf(x, df):
    return mpmath.gammainc(mpf(df) / 2, mpf(x) / 2, mpmath.inf, regularized=True)


def z_upper(a):
    """z with P(Z > z) = a."""
    return -mpmath.sqrt(2) * mpmath.erfinv(2 * mpf(a) - 1)


def mid_ranks(values):
    """Average ranks, 1-based, and the sizes of the groups of ties."""
    order = sorted(range(len(values)), key=lambda i: values[i])
    r = [mpf(0)] * len(values)
    ties = []
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and values[order[j + 1]] == values[order[i]]:
            j += 1
        avg = mpf(i + j + 2) / 2
        for t in range(i, j + 1):
            r[order[t]] = avg
        ties.append(j - i + 1)
        i = j + 1
    return r, ties


# The signed-rank test.


def signrank_counts(n):
    """counts[v] = the number of subsets of 1..n summing to v."""
    if n <= 16:
        u = n * (n + 1) // 2
        c = [0] * (u + 1)
        for signs in itertools.product((0, 1), repeat=n):
            c[sum(j + 1 for j in range(n) if signs[j])] += 1
        return c
    c = [1]
    for j in range(1, n + 1):
        nc = c + [0] * j
        for v, k in enumerate(c):
            nc[v + j] += k
        c = nc
    return c


def lower_tail(counts, v):
    total = sum(counts)
    v = int(v)
    if v < 0:
        return mpf(0)
    return mpf(sum(counts[: v + 1])) / total


def signrank_v(xs):
    r, ties = mid_ranks([abs(x) for x in xs])
    return sum(r[i] for i in range(len(xs)) if xs[i] > 0), r, ties


def signrank_z(xs, alternative, correct):
    """The normal path's z of xs, zeros already dropped."""
    n = len(xs)
    v, _, ties = signrank_v(xs)
    z = v - mpf(n) * (n + 1) / 4
    sigma = mpmath.sqrt(mpf(n) * (n + 1) * (2 * n + 1) / 24 - sum(mpf(t) ** 3 - t for t in ties) / 48)
    c = 0
    if correct:
        c = {"two.sided": mpmath.sign(z) / 2, "greater": mpf(1) / 2, "less": -mpf(1) / 2}[alternative]
    return (z - c) / sigma


def walsh(xs):
    return sorted((xs[i] + xs[j]) / 2 for i in range(len(xs)) for j in range(i, len(xs)))


def median(a):
    m = len(a)
    return a[m // 2] if m % 2 else (a[m // 2 - 1] + a[m // 2]) / 2


def step_root(xs, zq, alternative, correct):
    """The d at which the corrected z of xs - d, zeros dropped, steps across zq."""
    def f(d):
        xd = [x - d for x in xs if x - d != 0]
        return signrank_z(xd, alternative, correct) - zq

    points = sorted(set(walsh(xs)))
    eps = mpf(10) ** -30
    for b in points:
        left, right = f(b - eps), f(b + eps)
        if left > 0 and right <= 0:
            return b
    raise ValueError("no step crosses zq")


def signrank(x, mu, alternative="two.sided", exact=None, correct=True, conf=None):
    """V, p and method; with conf, the estimate and the ends too."""
    d = [x_ - mu for x_ in x]
    zeroes = any(v == 0 for v in d)
    d = [v for v in d if v != 0]
    n = len(d)
    if exact is None:
        exact = n < 50
    v, r, ties = signrank_v(d)
    has_ties = any(t > 1 for t in ties)
    out = {"statistic": v}
    xs = [v_ + mu for v_ in d]
    a = walsh(xs)
    if exact and not has_ties and not zeroes:
        counts = signrank_counts(n)
        u = n * (n + 1) // 2
        le = lower_tail(counts, v)
        ge = 1 - lower_tail(counts, v - 1)
        out["p"] = {"two.sided": min(2 * min(le, ge), 1), "less": le, "greater": ge}[alternative]
        out["method"] = "Wilcoxon signed rank exact test"
        if conf is not None:
            alpha = 1 - mpf(conf)
            side = alpha / 2 if alternative == "two.sided" else alpha
            q = 0
            while lower_tail(counts, q) < side:
                q += 1
            q = max(q, 1)
            if alternative in ("two.sided", "greater"):
                out["lower"] = a[q - 1]
            if alternative in ("two.sided", "less"):
                out["upper"] = a[u - q]
            out["estimate"] = median(a)
    else:
        z = signrank_z(d, alternative, correct)
        lower, upper = phi(z), 1 - phi(z)
        out["p"] = {"two.sided": 2 * min(lower, upper), "less": lower, "greater": upper}[alternative]
        out["method"] = "Wilcoxon signed rank test" + (" with continuity correction" if correct else "")
        if conf is not None:
            alpha = 1 - mpf(conf)
            side = alpha / 2 if alternative == "two.sided" else alpha
            if alternative in ("two.sided", "greater"):
                out["lower"] = step_root(xs, z_upper(side), alternative, correct)
            if alternative in ("two.sided", "less"):
                out["upper"] = step_root(xs, -z_upper(side), alternative, correct)
            out["estimate"] = median(a)
    return out


# Kruskal-Wallis and Friedman.


def kruskal(groups):
    allv = [v for g in groups for v in g]
    n = len(allv)
    r, ties = mid_ranks(allv)
    s, at = mpf(0), 0
    for g in groups:
        rs = sum(r[at : at + len(g)])
        s += rs**2 / len(g)
        at += len(g)
    h = (12 * s / (n * (n + 1)) - 3 * (n + 1)) / (1 - mpf(sum(t**3 - t for t in ties)) / (n**3 - n))
    df = len(groups) - 1
    return {"statistic": h, "p": chisq_sf(h, df), "df": df}


def friedman(rows):
    n, k = len(rows), len(rows[0])
    cols = [mpf(0)] * k
    tie = 0
    for row in rows:
        r, ties = mid_ranks(row)
        for j in range(k):
            cols[j] += r[j]
        tie += sum(t**3 - t for t in ties)
    s = 12 * sum((c - mpf(n) * (k + 1) / 2) ** 2 for c in cols) / (n * k * (k + 1) - mpf(tie) / (k - 1))
    df = k - 1
    return {"statistic": s, "p": chisq_sf(s, df), "df": df}


# McNemar and CMH.


def mcnemar(t, correct=True):
    r = len(t)
    symmetric = all(t[i][j] == t[j][i] for i in range(r) for j in range(r))
    corrected = correct and r == 2 and not symmetric
    s = mpf(0)
    for i in range(r):
        for j in range(i + 1, r):
            d = abs(t[i][j] - t[j][i])
            if corrected:
                d -= 1
            s += d**2 / (t[i][j] + t[j][i])
    df = r * (r - 1) // 2
    method = "McNemar's Chi-squared test" + (" with continuity correction" if corrected else "")
    return {"statistic": s, "p": chisq_sf(s, df), "df": df, "method": method}


def cmh(strata):
    delta, var, sd_, so = mpf(0), mpf(0), mpf(0), mpf(0)
    p1 = p2 = p3 = mpf(0)
    for (a, b), (c, d) in strata:
        n = a + b + c + d
        delta += a - (a + b) * (a + c) / n
        var += (a + b) * (c + d) * (a + c) * (b + d) / (n**2 * (n - 1))
        sd_ += a * d / n
        so += b * c / n
        p1 += (a + d) * a * d / n**2
        p2 += ((a + d) * b * c + (b + c) * a * d) / n**2
        p3 += (b + c) * b * c / n**2
    yates = mpf(1) / 2 if abs(delta) >= mpf(1) / 2 else 0
    x2 = (abs(delta) - yates) ** 2 / var
    est = sd_ / so
    sd = mpmath.sqrt(p1 / (2 * sd_**2) + p2 / (2 * sd_ * so) + p3 / (2 * so**2))
    z = z_upper(mpf("0.025"))
    method = "Mantel-Haenszel chi-squared test " + ("with" if yates else "without") + " continuity correction"
    return {
        "statistic": x2,
        "p": chisq_sf(x2, 1),
        "df": 1,
        "estimate": est,
        "lower": est * mpmath.exp(-z * sd),
        "upper": est * mpmath.exp(z * sd),
        "method": method,
    }


def emit(program, answer):
    for k in sorted(answer):
        if k == "method":
            continue
        print(f"{program} | {k} {show(answer[k])}")


def main():
    # R's help-page data for wilcox.test (Hollander and Wolfe, p. 29): the
    # paired depression scores, nine differences, no ties: exact.
    xs = "1.83 0.50 1.62 2.48 1.68 1.88 1.55 3.06 1.30".split()
    ys = "0.878 0.647 0.598 2.05 1.06 1.29 1.06 3.14 1.29".split()
    x, y = [mpf(v) for v in xs], [mpf(v) for v in ys]
    dif = [a - b for a, b in zip(x, y)]
    lx, ly = bund_list(xs), bund_list(ys)
    emit(f"{lx} {ly} stat.test.wilcoxon.paired", signrank(dif, 0))
    for alt in ("two.sided", "less", "greater"):
        emit(
            f"{lx} {ly} {opts({'alternative': alt})} stat.test.wilcoxon.paired.opts",
            signrank(dif, 0, alt, conf="0.95"),
        )
    emit(f"{lx} 1.0 stat.test.wilcoxon", signrank(x, 1))
    emit(f"{lx} 1.0 {opts({'conf': 0.9})} stat.test.wilcoxon.opts", signrank(x, 1, conf="0.9"))
    emit(
        f"{lx} 1.0 {opts({'exact': False})} stat.test.wilcoxon.opts",
        signrank(x, 1, exact=False, conf="0.95"),
    )
    # Ties and zeros: the normal path, corrected or not.
    ts = "1 2 2 3 4 5 5 5 6 7 8 0.5 -1 -2 3 9 11".split()
    t = [mpf(v) for v in ts]
    lt = bund_list(ts)
    emit(f"{lt} 3 stat.test.wilcoxon", signrank(t, 3))
    for alt in ("two.sided", "less", "greater"):
        for correct in (True, False):
            emit(
                f"{lt} 3 {opts({'alternative': alt, 'correct': correct})} stat.test.wilcoxon.opts",
                signrank(t, 3, alt, correct=correct, conf="0.95"),
            )
    # Sixty values with no ties: normal by default, exact when asked.
    big = [mpmath.nstr(mpmath.sin(i) * 10 + mpf(i) / 7, 8) for i in range(1, 61)]
    b = [mpf(v) for v in big]
    assert len(set(abs(v - 1) for v in b)) == 60 and all(v != 1 for v in b)
    lb = bund_list(big)
    emit(f"{lb} 1 stat.test.wilcoxon", signrank(b, 1))
    emit(f"{lb} 1 {opts({'exact': True})} stat.test.wilcoxon.opts", signrank(b, 1, exact=True, conf="0.95"))

    # Kruskal-Wallis: R's help-page data (Hollander and Wolfe, p. 116), and
    # one with ties.
    g1 = [["2.9", "3.0", "2.5", "2.6", "3.2"], ["3.8", "2.7", "4.0", "2.4"], ["2.8", "3.4", "3.7", "2.2", "2.0"]]
    g2 = [["1", "2", "2", "3"], ["2", "3", "4", "4", "5"], ["1", "1", "6"], ["7", "3"]]
    for g in (g1, g2):
        prog = "[ " + " ".join(bund_list(v) for v in g) + " ] stat.test.kruskal"
        emit(prog, kruskal([[mpf(v) for v in grp] for grp in g]))

    # Friedman: blocks as rows, ties within some.
    f1 = [
        ["5.40", "5.50", "5.55"],
        ["5.85", "5.70", "5.75"],
        ["5.20", "5.60", "5.50"],
        ["5.55", "5.50", "5.40"],
        ["5.90", "5.85", "5.70"],
        ["5.45", "5.55", "5.60"],
        ["5.40", "5.40", "5.35"],
        ["5.45", "5.50", "5.35"],
        ["5.25", "5.15", "5.00"],
        ["5.85", "5.80", "5.70"],
    ]
    f2 = [["1", "2", "3", "4"], ["2", "2", "1", "4"], ["4", "3", "3", "3"], ["1", "4", "2", "3"], ["2", "1", "4", "3"]]
    for f in (f1, f2):
        emit(f"{bund_matrix(f)} stat.test.friedman", friedman([[mpf(v) for v in row] for row in f]))

    # McNemar: R's help-page Performance table, corrected and not; a
    # symmetric pair; a 3x3 table.
    perf = [["794", "150"], ["86", "570"]]
    pm = [[mpf(v) for v in row] for row in perf]
    emit(f"{bund_matrix(perf)} stat.test.mcnemar", mcnemar(pm))
    emit(f"{bund_matrix(perf)} {opts({'correct': False})} stat.test.mcnemar.opts", mcnemar(pm, False))
    sym = [["10", "5"], ["5", "20"]]
    emit(f"{bund_matrix(sym)} stat.test.mcnemar", mcnemar([[mpf(v) for v in row] for row in sym]))
    three = [["20", "5", "3"], ["9", "30", "2"], ["4", "8", "25"]]
    emit(f"{bund_matrix(three)} stat.test.mcnemar", mcnemar([[mpf(v) for v in row] for row in three]))

    # CMH: R's help-page Rabbits data, five 2x2 strata (penicillin level),
    # rows the delay, columns cured or died; and two strata whose |delta|
    # is below 1/2, so no correction.
    rabbits = [
        [["0", "6"], ["0", "5"]],
        [["3", "3"], ["0", "6"]],
        [["6", "0"], ["2", "4"]],
        [["5", "1"], ["6", "0"]],
        [["2", "0"], ["5", "0"]],
    ]
    close_ = [[["3", "2"], ["2", "3"]], [["4", "3"], ["3", "3"]]]
    for strata in (rabbits, close_):
        prog = "[ " + " ".join("[ " + " ".join(bund_list(r) for r in s) + " ]" for s in strata) + " ] stat.test.cmh"
        emit(prog, cmh([[[mpf(v) for v in row] for row in s] for s in strata]))


if __name__ == "__main__":
    main()
