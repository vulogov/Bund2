# RFC-0015 criterion 16's fixture for S13's popgen.hwe: the exact test of
# Wigginton, Cutler & Abecasis (2005), as its appendix gives it, and the
# 1-df chi-square test, by mpmath 1.3.0 at 50 significant digits.
#
#     python3 -I popgen.py > popgen.txt
import mpmath
from mpmath import mpf

mpmath.mp.dps = 50


def exact(obs_hets, obs_hom1, obs_hom2):
    """SNPHWE: the appendix's recurrence, from the midpoint outwards."""
    obs_homc = max(obs_hom1, obs_hom2)
    obs_homr = min(obs_hom1, obs_hom2)
    rare = 2 * obs_homr + obs_hets
    genotypes = obs_hets + obs_homc + obs_homr
    probs = [mpf(0)] * (rare + 1)
    mid = rare * (2 * genotypes - rare) // (2 * genotypes)
    if (rare & 1) ^ (mid & 1):
        mid += 1
    curr_hets, curr_homr = mid, (rare - mid) // 2
    curr_homc = genotypes - curr_hets - curr_homr
    probs[mid] = mpf(1)
    total = mpf(1)
    while curr_hets >= 2:
        probs[curr_hets - 2] = (
            probs[curr_hets] * curr_hets * (curr_hets - 1)
            / (4 * (curr_homr + 1) * (curr_homc + 1))
        )
        total += probs[curr_hets - 2]
        curr_hets -= 2
        curr_homr += 1
        curr_homc += 1
    curr_hets, curr_homr = mid, (rare - mid) // 2
    curr_homc = genotypes - curr_hets - curr_homr
    while curr_hets <= rare - 2:
        probs[curr_hets + 2] = (
            probs[curr_hets] * 4 * curr_homr * curr_homc
            / ((curr_hets + 2) * (curr_hets + 1))
        )
        total += probs[curr_hets + 2]
        curr_hets += 2
        curr_homr -= 1
        curr_homc -= 1
    probs = [x / total for x in probs]
    return min(mpf(1), mpmath.fsum(x for x in probs if x <= probs[obs_hets]))


def chisq(aa, ab, bb):
    n = mpf(aa + ab + bb)
    p = (2 * aa + ab) / (2 * n)
    q = 1 - p
    e = [n * p * p, 2 * n * p * q, n * q * q]
    x = mpmath.fsum((o - x) ** 2 / x for o, x in zip([aa, ab, bb], e))
    return e, x, mpmath.erfc(mpmath.sqrt(x / 2))


def show(x):
    return mpmath.nstr(x, 17, min_fixed=-1, max_fixed=-1)


for aa, ab, bb in [(298, 489, 213), (14, 57, 50), (10, 0, 10), (0, 7, 0), (100, 200, 100)]:
    e, x, p = chisq(aa, ab, bb)
    key = f"hwe.{aa}.{ab}.{bb}"
    print(f"{key}.expected [{' '.join(show(v) for v in e)}]")
    print(f"{key}.chisq {show(x)}")
    print(f"{key}.p.chisq {show(p)}")
    print(f"{key}.p.exact {show(exact(ab, aa, bb))}")
