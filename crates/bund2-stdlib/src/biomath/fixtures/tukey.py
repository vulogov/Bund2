# RFC-0019 criterion 3's fixture for the studentized range (§S3), computed
# with mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I tukey.py > tukey.txt
#
# Each line is a Bund program and its answer to 17 significant digits, as
# dist.py's are. The cdf is the definition, the double integral
#
#     P(Q <= q) = int_0^inf f_s(s) W(q s) ds,
#     W(w) = k int phi(z) (Phi(z + w) - Phi(z))^(k - 1) dz,
#
# with f_s the density of s = sqrt(chi2_nu / nu), normalised by its closed
# form, by mpmath's tanh-sinh quadrature split at the integrands' modes; not
# Bund2's pieces, cut-offs or Gauss-Kronrod rule. For k = 2 the inner
# integral is also its closed form, erf(w / 2), and the two are checked to
# agree within 1e-40. The upper tail is integrated directly, over 1 - W,
# and each pair of tails is checked to sum to 1 within 1e-30.
#
# The grid spans k from 2 to 1000 and df from 2 to 1000, and each tail's
# far end: a cdf below 1e-3 and an sf below 1e-4.
import multiprocessing

import mpmath
from mpmath import mpf

mpmath.mp.dps = 50

GRID = [
    (2, 2, 1.5),
    (2, 1000, 0.5),
    (3, 10, 3.5),
    (3, 10, 8.0),
    (5, 20, 4.0),
    (10, 5, 2.0),
    (20, 60, 5.0),
    (100, 30, 6.0),
    (1000, 120, 6.5),
]


def show(x):
    return mpmath.nstr(x, 17, min_fixed=-1, max_fixed=-1)


def num(x):
    s = repr(float(x))
    return s if "." in s or "e" in s else s + ".0"


def range_cdf(k, w, upper=False):
    """W(w), or 1 - W(w) integrated directly when `upper`."""
    if k == 2:
        closed = mpmath.erf(w / 2)
        return 1 - closed if upper else closed
    if upper:
        # P(range > w) = k int phi(z) [(1 - Phi(z))^(k-1) - (Phi(z+w) - Phi(z))^(k-1)] dz
        def g(z):
            q = mpmath.ncdf(-z)
            d = mpmath.ncdf(z + w) - mpmath.ncdf(z)
            return k * mpmath.npdf(z) * (q ** (k - 1) - d ** (k - 1))
    else:
        def g(z):
            return k * mpmath.npdf(z) * (mpmath.ncdf(z + w) - mpmath.ncdf(z)) ** (k - 1)
    # The smallest of k normals sits near Phi^-1(1/k); split around it.
    m = -mpmath.sqrt(2 * mpmath.log(k))
    pts = [-mpmath.inf, m - 4, m - 1, m, m + 1, m + 4, mpmath.inf]
    pts = sorted(set(pts), key=lambda x: x)
    return mpmath.quad(g, pts)


def s_pdf(s, nu):
    return mpmath.exp(
        mpmath.log(2) + (nu / 2) * mpmath.log(nu / 2) - mpmath.loggamma(nu / 2)
        + (nu - 1) * mpmath.log(s) - nu * s * s / 2
    )


def tails(k, nu, q):
    k, nu, q = mpf(k), mpf(nu), mpf(q)
    sm = mpmath.sqrt((nu - 1) / nu)
    sd = 1 / mpmath.sqrt(2 * nu)
    pts = [0] + [sm + j * sd for j in (-6, -3, -1, 0, 1, 3, 6) if sm + j * sd > 0] + [mpmath.inf]
    lo = mpmath.quad(lambda s: s_pdf(s, nu) * range_cdf(k, q * s), pts)
    hi = mpmath.quad(lambda s: s_pdf(s, nu) * range_cdf(k, q * s, upper=True), pts)
    assert abs(lo + hi - 1) < mpf(10) ** -30, (k, nu, q, lo + hi - 1)
    return lo, hi


def point(args):
    k, nu, q = args
    lo, hi = tails(k, nu, q)
    d = f"{num(q)} {k} {num(nu)} dist.tukey"
    return f"{d} dist.cdf {show(lo)}\n{d} dist.sf {show(hi)}"


def main():
    for w in (mpf("0.3"), mpf(2), mpf(5)):
        k = mpf(2)
        g = lambda z: k * mpmath.npdf(z) * (mpmath.ncdf(z + w) - mpmath.ncdf(z))
        assert abs(mpmath.quad(g, [-mpmath.inf, 0, mpmath.inf]) - mpmath.erf(w / 2)) < mpf(10) ** -40
    # One process per grid point; the lines in GRID's order.
    with multiprocessing.Pool(len(GRID)) as pool:
        for lines in pool.map(point, GRID):
            print(lines)


if __name__ == "__main__":
    main()
