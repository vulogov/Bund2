//! Domains, and the arithmetic their bounds are computed in (RFC-0013 §S21).
//!
//! **A bound is an `i128`**, with `i128::MIN` and `i128::MAX` as −∞ and +∞
//! (`inf` and `sup`). A finite bound of a domain is an `i64`; one computed
//! outside that range is mapped by §S21's table when it narrows a domain.
//!
//! **A sum is exact.** [`Acc`] adds terms of up to 126 bits each, as a
//! coefficient times an `i64` makes them, without overflowing, so terms of
//! opposite signs cancel as they should. Only the sum's value, read back,
//! saturates to the sentinel of its sign.
//!
//! Nothing here uses the `Vm`.

/// A domain's bound, or a bound being computed.
pub type Bound = i128;

/// −∞, `inf`.
pub const INF: Bound = i128::MIN;
/// +∞, `sup`.
pub const SUP: Bound = i128::MAX;

const MIN: Bound = i64::MIN as i128;
const MAX: Bound = i64::MAX as i128;

fn infinite(b: Bound) -> bool {
    b == INF || b == SUP
}

/// The sentinel of `positive`'s sign.
fn sentinel(positive: bool) -> Bound {
    if positive { SUP } else { INF }
}

/// `a × b` by §S21's table: ±∞ × 0 is 0, an infinity takes the product's
/// sign, and an overflow saturates to the sentinel of its sign.
pub fn mul(a: Bound, b: Bound) -> Bound {
    if a == 0 || b == 0 {
        return 0;
    }
    if infinite(a) || infinite(b) {
        return sentinel((a > 0) == (b > 0));
    }
    a.checked_mul(b)
        .unwrap_or_else(|| sentinel((a > 0) == (b > 0)))
}

/// `-a`, an infinity to the other.
pub fn neg(a: Bound) -> Bound {
    match a {
        INF => SUP,
        SUP => INF,
        _ => -a,
    }
}

/// ⌊a / b⌋ for a finite, non-zero `b`; an infinite `a` gives the infinity of
/// the quotient's sign.
pub fn div_floor(a: Bound, b: Bound) -> Bound {
    if infinite(a) {
        return sentinel((a > 0) == (b > 0));
    }
    let q = a / b;
    if a % b != 0 && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

/// ⌈a / b⌉, as [`div_floor`].
pub fn div_ceil(a: Bound, b: Bound) -> Bound {
    if infinite(a) {
        return sentinel((a > 0) == (b > 0));
    }
    let q = a / b;
    if a % b != 0 && ((a < 0) == (b < 0)) {
        q + 1
    } else {
        q
    }
}

/// An exact sum of finite terms: `hi × 2⁶⁴ + lo`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Acc {
    hi: i128,
    lo: u128,
}

impl Acc {
    pub fn add(&mut self, t: Bound) {
        self.hi += t >> 64;
        self.lo += u128::from(t as u64);
    }

    pub fn sub(&mut self, t: Bound) {
        // `-t` overflows only for `i128::MIN`, which is `INF` and never a
        // finite term; it saturates to `SUP` there, as `neg` would.
        self.add(t.checked_neg().unwrap_or(SUP));
    }

    /// The sum, saturated to the sentinel of its sign outside `i128`.
    pub fn value(self) -> Bound {
        let hi = self.hi + (self.lo >> 64) as i128;
        let lo = (self.lo & u128::from(u64::MAX)) as i128;
        // `hi` within 63 bits keeps `hi × 2⁶⁴ + lo` inside `i128`.
        if hi >= 1 << 62 {
            return SUP;
        }
        if hi < -(1 << 62) {
            return INF;
        }
        (hi << 64) + lo
    }
}

/// A domain: sorted, disjoint closed intervals. An empty one has failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dom(Vec<(Bound, Bound)>);

impl Dom {
    /// `inf..sup`: what an unconstrained variable may be.
    pub fn full() -> Dom {
        Dom(vec![(INF, SUP)])
    }

    /// `[lo, hi]`, its ends mapped by §S21's table: a lower bound above
    /// `i64::MAX`, or an upper below `i64::MIN`, empties it; one beyond the
    /// other end widens to `inf` or `sup`.
    pub fn interval(lo: Bound, hi: Bound) -> Dom {
        if lo > MAX || hi < MIN || lo > hi {
            return Dom(Vec::new());
        }
        let lo = if lo < MIN { INF } else { lo };
        let hi = if hi > MAX { SUP } else { hi };
        Dom(vec![(lo, hi)])
    }

    /// The union of `parts`, each already within the table.
    pub fn union(mut parts: Vec<(Bound, Bound)>) -> Dom {
        parts.sort_unstable();
        let mut out: Vec<(Bound, Bound)> = Vec::with_capacity(parts.len());
        for (lo, hi) in parts {
            match out.last_mut() {
                Some(last) if lo <= last.1.saturating_add(1) => last.1 = last.1.max(hi),
                _ => out.push((lo, hi)),
            }
        }
        Dom(out)
    }

    pub fn intervals(&self) -> &[(Bound, Bound)] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.0 == [(INF, SUP)]
    }

    pub fn min(&self) -> Bound {
        self.0.first().map_or(SUP, |i| i.0)
    }

    pub fn max(&self) -> Bound {
        self.0.last().map_or(INF, |i| i.1)
    }

    pub fn contains(&self, v: Bound) -> bool {
        self.0.iter().any(|&(lo, hi)| lo <= v && v <= hi)
    }

    /// The one value, if there is one.
    pub fn single(&self) -> Option<i64> {
        match self.0.as_slice() {
            [(lo, hi)] if lo == hi => i64::try_from(*lo).ok(),
            _ => None,
        }
    }

    /// How many values, `None` when an end is infinite.
    pub fn size(&self) -> Option<u128> {
        if infinite(self.min()) || infinite(self.max()) {
            return None;
        }
        Some(self.0.iter().map(|&(lo, hi)| (hi - lo) as u128 + 1).sum())
    }

    pub fn intersect(&self, other: &Dom) -> Dom {
        let mut out = Vec::new();
        let (mut i, mut j) = (0, 0);
        while let (Some(&(a0, a1)), Some(&(b0, b1))) = (self.0.get(i), other.0.get(j)) {
            let (lo, hi) = (a0.max(b0), a1.min(b1));
            if lo <= hi {
                out.push((lo, hi));
            }
            if a1 < b1 {
                i += 1;
            } else {
                j += 1;
            }
        }
        Dom(out)
    }

    /// Without `v`.
    pub fn remove(&self, v: Bound) -> Dom {
        let mut out = Vec::with_capacity(self.0.len() + 1);
        for &(lo, hi) in &self.0 {
            if v < lo || v > hi {
                out.push((lo, hi));
                continue;
            }
            if lo < v {
                out.push((lo, v - 1));
            }
            if v < hi {
                out.push((v + 1, hi));
            }
        }
        Dom(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds() {
        assert_eq!(mul(SUP, 3), SUP);
        assert_eq!(mul(INF, -3), SUP);
        assert_eq!(mul(SUP, -3), INF);
        assert_eq!(mul(INF, 3), INF);
        assert_eq!(mul(SUP, 0), 0);
        assert_eq!(mul(INF, 0), 0);
        assert_eq!(mul(i128::MAX - 1, 2), SUP);
        assert_eq!(mul(i128::MIN + 1, 2), INF);
        assert_eq!(Dom::interval(MAX + 1, SUP), Dom(Vec::new()));
        assert_eq!(Dom::interval(INF, MIN - 1), Dom(Vec::new()));
        assert_eq!(Dom::interval(MIN - 1, 0), Dom(vec![(INF, 0)]));
        assert_eq!(Dom::interval(0, MAX + 1), Dom(vec![(0, SUP)]));
    }

    #[test]
    fn division_rounds_as_named() {
        assert_eq!((div_floor(7, 2), div_ceil(7, 2)), (3, 4));
        assert_eq!((div_floor(-7, 2), div_ceil(-7, 2)), (-4, -3));
        assert_eq!((div_floor(7, -2), div_ceil(7, -2)), (-4, -3));
        assert_eq!((div_floor(-6, 2), div_ceil(-6, 2)), (-3, -3));
        assert_eq!(div_floor(SUP, -2), INF);
    }

    /// Four products at the edge of `i64` sum exactly, and cancel.
    #[test]
    fn a_sum_is_exact() {
        let p = MAX * MAX;
        let mut acc = Acc::default();
        for _ in 0..4 {
            acc.add(p);
        }
        assert_eq!(acc.value(), SUP);
        for _ in 0..4 {
            acc.sub(p);
        }
        acc.add(-5);
        assert_eq!(acc.value(), -5);
        let mut acc = Acc::default();
        acc.add(MIN * MAX);
        acc.add(MIN * MAX);
        acc.sub(MIN * MAX);
        assert_eq!(acc.value(), MIN * MAX);
    }

    #[test]
    fn domains_cut_and_join() {
        let d = Dom::interval(0, 9).remove(5);
        assert_eq!(d.intervals(), [(0, 4), (6, 9)]);
        assert_eq!(d.size(), Some(9));
        assert_eq!(
            d.intersect(&Dom::interval(3, 7)).intervals(),
            [(3, 4), (6, 7)]
        );
        assert_eq!(
            Dom::union(vec![(6, 9), (0, 4), (5, 5)]).intervals(),
            [(0, 9)]
        );
        assert_eq!(Dom::interval(4, 4).single(), Some(4));
        assert_eq!(Dom::full().size(), None);
    }
}
