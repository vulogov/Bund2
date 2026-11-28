//! RFC-0020 §S10: the empirical distribution function, R's `ecdf`
//! (`src/library/stats/R/ecdf.R:23-36`) by its definition.
//!
//! - **`stat.ecdf`** (`x -- MAP`) answers `x`, the sorted distinct values
//!   (−0 and 0 one value, as R's `unique`), and `y`, #{xᵢ ≤ xⱼ}/n at each, a
//!   count over n as R's `cumsum(tabulate(...))/n`. Four forms.
//! - **`stat.ecdf.at`** (`x t -- FLOAT`) answers #{xᵢ ≤ t}/n. Two forms, as
//!   `stat.quantile`.
//!
//! An empty x is E101 for both, as R stops ("'x' must have 1 or more
//! non-missing values"), and a non-finite value of x or a NaN or ±∞ `t` is
//! E101, the family's rule (RFC-0015 §S11), where R drops NA, keeps ±∞ as
//! a knot and answers NaN, 0 and 1 at NaN, −∞ and +∞ (§S1). Not a port of
//! R's code, so this file stays Apache-2.0 (D279).

use std::collections::BTreeMap;

use bund2_api::{Error, Registry};
use bund2_value::BundValue;

use super::operand::{at_least, four_forms, list, scalar_forms};

fn ecdf(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    at_least(word, xs, 1)?;
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len() as f64;
    let mut knots: Vec<f64> = Vec::new();
    let mut ys = Vec::new();
    for (i, v) in s.iter().enumerate() {
        match knots.last() {
            Some(last) if last == v => {
                if let Some(y) = ys.last_mut() {
                    *y = (i + 1) as f64 / n;
                }
            }
            _ => {
                knots.push(*v);
                ys.push((i + 1) as f64 / n);
            }
        }
    }
    let mut m = BTreeMap::new();
    m.insert("x".to_string(), list(knots));
    m.insert("y".to_string(), list(ys));
    Ok(BundValue::map(m))
}

fn ecdf_at(word: &str, xs: &[f64], t: f64) -> Result<BundValue, Error> {
    at_least(word, xs, 1)?;
    let k = xs.iter().filter(|x| **x <= t).count();
    Ok(BundValue::float(k as f64 / xs.len() as f64))
}

pub fn register(r: &mut Registry) {
    four_forms!(r, "stat.ecdf", ecdf);
    scalar_forms!(r, "stat.ecdf.at", "t", ecdf_at);
}

/// `stat.ecdf` in its four forms and `stat.ecdf.at` in its two.
pub const WORDS: [&str; 6] = [
    "stat.ecdf",
    "stat.ecdf,",
    "stat.ecdf.",
    "stat.ecdf.,",
    "stat.ecdf.at",
    "stat.ecdf.at.",
];

#[cfg(test)]
mod tests;
