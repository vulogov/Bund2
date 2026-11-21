//! RFC-0018 §S2's design check: what `fit.lm`, `fit.glm` and `model.anova`
//! ask of a design on entry, and `model.predict` of its fit's `spec`.
//!
//! A design may be made or changed by hand, so the check enforces what the
//! words rely on, each failure E101 naming the key. First each key present
//! with its type, the MATRIX n × k and every LIST of its length; then the
//! invariants, in §S2's order: `spec`'s shape, the intercept read from
//! `spec.intercept`, `assign`, `names` distinct, every value finite and
//! every weight positive, `offset` and `weights` agreeing with `spec`, and
//! n > k. Keys the design does not define are not read.

use std::collections::BTreeMap;

use bund2_api::Error;
use bund2_value::{BOOL, BundValue, FLOAT, INTEGER, LIST, MAP, MATRIX, STRING};

use crate::biomath::frame::render;
use crate::biomath::operand::{WIDTH, domain};

/// A design's `spec` (§S2).
#[derive(Debug, Clone)]
pub struct Spec {
    /// The response column's name.
    pub response: String,
    /// `terms`, as given.
    pub terms: BundValue,
    /// The number of terms.
    pub nterms: usize,
    /// Whether the design has an intercept.
    pub intercept: bool,
    /// Each factor column read, to the levels it was coded from.
    pub levels: BTreeMap<String, Vec<String>>,
    /// The logical columns read.
    pub logical: Vec<String>,
    /// The `weights` column, when one was given.
    pub weights: Option<String>,
    /// The `offset` column, when one was given.
    pub offset: Option<String>,
}

/// A design that passed the check.
#[derive(Debug, Clone)]
pub struct Design {
    /// The MATRIX, n rows of k cells.
    pub x: Vec<Vec<f64>>,
    /// The k column names.
    pub names: Vec<String>,
    /// Each column's term, 0 for the intercept.
    pub assign: Vec<usize>,
    /// The response.
    pub y: Vec<f64>,
    /// The prior weights.
    pub weights: Vec<f64>,
    /// The offset.
    pub offset: Vec<f64>,
    /// The rows `"omit"` dropped.
    pub dropped: i64,
    /// What `model.predict` codes a new frame by.
    pub spec: Spec,
}

/// A number's value, INTEGER or FLOAT.
fn num(v: &BundValue) -> Option<f64> {
    match v.unboxed() {
        BundValue::Float(x, _) if v.dt() == FLOAT => Some(*x),
        BundValue::Int(i, _) if v.dt() == INTEGER => Some(*i as f64),
        _ => None,
    }
}

/// How a FLOAT reads in a refusal.
fn shown(x: f64) -> String {
    BundValue::float(x).summary(WIDTH)
}

struct Check<'a> {
    word: &'a str,
    op: &'a str,
}

impl Check<'_> {
    fn no(&self, text: String) -> Error {
        domain(self.word, self.op, text)
    }

    fn key<'m>(&self, m: &'m BTreeMap<String, BundValue>, k: &str) -> Result<&'m BundValue, Error> {
        m.get(k)
            .ok_or_else(|| self.no(format!("a missing key {}", render::text(k))))
    }

    /// A LIST of `n` numbers.
    fn numbers(&self, key: &str, v: &BundValue, n: usize) -> Result<Vec<f64>, Error> {
        let items = match v.as_list() {
            Some(xs) if v.dt() == LIST => xs,
            _ => return Err(self.no(format!("{key} is {}", v.summary(WIDTH)))),
        };
        if items.len() != n {
            return Err(self.no(format!("{key} has {} items, not {n}", items.len())));
        }
        items
            .iter()
            .enumerate()
            .map(|(i, x)| {
                num(x)
                    .ok_or_else(|| self.no(format!("{key} row {} is {}", i + 1, render::value(x))))
            })
            .collect()
    }

    /// A LIST of STRINGs.
    fn strings(&self, key: &str, v: &BundValue) -> Result<Vec<String>, Error> {
        let items = match v.as_list() {
            Some(xs) if v.dt() == LIST => xs,
            _ => return Err(self.no(format!("{key} is {}", v.summary(WIDTH)))),
        };
        items
            .iter()
            .enumerate()
            .map(|(i, x)| match x.as_str() {
                Some(s) if x.dt() == STRING => Ok(s),
                _ => Err(self.no(format!("{key} item {} is {}", i + 1, render::value(x)))),
            })
            .collect()
    }

    fn string(&self, key: &str, v: &BundValue) -> Result<String, Error> {
        match v.as_str() {
            Some(s) if v.dt() == STRING => Ok(s),
            _ => Err(self.no(format!("{key} is {}", v.summary(WIDTH)))),
        }
    }

    /// `spec`'s keys and their types.
    fn spec(&self, v: &BundValue) -> Result<Spec, Error> {
        let m = match v.as_map() {
            Some(m) if v.dt() == MAP => m,
            _ => return Err(self.no(format!("spec is {}", v.summary(WIDTH)))),
        };
        let get = |k: &str| {
            m.get(k)
                .ok_or_else(|| self.no(format!("spec has no key {}", render::text(k))))
        };
        let response = self.string("spec.response", get("response")?)?;
        let terms = get("terms")?.clone();
        let nterms = match terms.as_list() {
            Some(xs) if terms.dt() == LIST => xs.len(),
            _ => return Err(self.no(format!("spec.terms is {}", terms.summary(WIDTH)))),
        };
        let iv = get("intercept")?;
        let intercept = match iv.unboxed() {
            BundValue::Bool(b, _) if iv.dt() == BOOL => *b,
            _ => return Err(self.no(format!("spec.intercept is {}", iv.summary(WIDTH)))),
        };
        let lv = get("levels")?;
        let lm = match lv.as_map() {
            Some(lm) if lv.dt() == MAP => lm,
            _ => return Err(self.no(format!("spec.levels is {}", lv.summary(WIDTH)))),
        };
        let mut levels = BTreeMap::new();
        for (c, ls) in lm {
            let key = format!("spec.levels of {}", render::text(c));
            levels.insert(c.clone(), self.strings(&key, ls)?);
        }
        let logical = self.strings("spec.logical", get("logical")?)?;
        let opt = |k: &str| -> Result<Option<String>, Error> {
            m.get(k)
                .map(|x| self.string(&format!("spec.{k}"), x))
                .transpose()
        };
        Ok(Spec {
            response,
            terms,
            nterms,
            intercept,
            levels,
            logical,
            weights: opt("weights")?,
            offset: opt("offset")?,
        })
    }
}

/// Check a `spec` MAP alone: its keys and their types (§S2), for a fit's
/// `spec` as for a design's.
pub(crate) fn spec(word: &str, op: &str, v: &BundValue) -> Result<Spec, Error> {
    Check { word, op }.spec(v)
}

/// Check a design operand `op` of `word` (§S2).
pub fn check(word: &str, op: &str, v: &BundValue) -> Result<Design, Error> {
    let c = Check { word, op };
    // Each key present with its type, and its shape.
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(c.no(v.summary(WIDTH))),
    };
    for k in [
        "matrix", "names", "assign", "y", "weights", "offset", "dropped", "spec",
    ] {
        c.key(m, k)?;
    }
    let mv = c.key(m, "matrix")?;
    let rows = match mv.as_matrix() {
        Some(rows) if mv.dt() == MATRIX => rows,
        _ => return Err(c.no(format!("matrix is {}", mv.summary(WIDTH)))),
    };
    let n = rows.len();
    let k = rows.first().map_or(0, Vec::len);
    let mut x = Vec::with_capacity(n);
    for (i, row) in rows.iter().enumerate() {
        if row.len() != k {
            return Err(c.no(format!(
                "matrix row {} has {} columns, not {k}",
                i + 1,
                row.len()
            )));
        }
        let cells = row
            .iter()
            .enumerate()
            .map(|(j, cell)| {
                num(cell).ok_or_else(|| {
                    c.no(format!(
                        "matrix row {} column {} is {}",
                        i + 1,
                        j + 1,
                        render::value(cell)
                    ))
                })
            })
            .collect::<Result<Vec<f64>, Error>>()?;
        x.push(cells);
    }
    let names = c.strings("names", c.key(m, "names")?)?;
    if names.len() != k {
        return Err(c.no(format!("names has {} items, not {k}", names.len())));
    }
    let av = c.key(m, "assign")?;
    let items = match av.as_list() {
        Some(xs) if av.dt() == LIST => xs,
        _ => return Err(c.no(format!("assign is {}", av.summary(WIDTH)))),
    };
    if items.len() != k {
        return Err(c.no(format!("assign has {} items, not {k}", items.len())));
    }
    let assign = items
        .iter()
        .enumerate()
        .map(|(i, a)| match a.unboxed() {
            BundValue::Int(t, _) if a.dt() == INTEGER && *t >= 0 => {
                usize::try_from(*t).map_err(|_| c.no(format!("assign item {} is {t}", i + 1)))
            }
            _ => Err(c.no(format!("assign item {} is {}", i + 1, render::value(a)))),
        })
        .collect::<Result<Vec<usize>, Error>>()?;
    let y = c.numbers("y", c.key(m, "y")?, n)?;
    let weights = c.numbers("weights", c.key(m, "weights")?, n)?;
    let offset = c.numbers("offset", c.key(m, "offset")?, n)?;
    let dv = c.key(m, "dropped")?;
    let dropped = match dv.unboxed() {
        BundValue::Int(d, _) if dv.dt() == INTEGER && *d >= 0 => *d,
        _ => return Err(c.no(format!("dropped is {}", dv.summary(WIDTH)))),
    };
    let spec = c.spec(c.key(m, "spec")?)?;

    // The intercept, read from `spec.intercept`.
    if spec.intercept {
        match assign.first() {
            Some(0) => {}
            Some(a) => {
                return Err(c.no(format!(
                    "assign item 1 is {a}, where the intercept's 0 belongs"
                )));
            }
            None => return Err(c.no("assign has no column for the intercept".into())),
        }
        if let Some(i) = assign.iter().skip(1).position(|&a| a == 0) {
            return Err(c.no(format!("assign item {} is 0, a second intercept", i + 2)));
        }
        if let Some(r) = x.iter().position(|row| row.first() != Some(&1.0)) {
            let cell = x
                .get(r)
                .and_then(|row| row.first())
                .copied()
                .unwrap_or(f64::NAN);
            return Err(c.no(format!(
                "matrix row {} column 1 is {}, not the intercept's 1",
                r + 1,
                shown(cell)
            )));
        }
    } else if let Some(i) = assign.iter().position(|&a| a == 0) {
        return Err(c.no(format!(
            "assign item {} is 0, and spec has no intercept",
            i + 1
        )));
    }
    // `assign`: non-decreasing, bounded, every term coded.
    for (i, w) in assign.windows(2).enumerate() {
        if let [a, b] = w
            && b < a
        {
            return Err(c.no(format!("assign item {} is {b}, after {a}", i + 2)));
        }
    }
    if let Some(i) = assign.iter().position(|&a| a > spec.nterms) {
        return Err(c.no(format!(
            "assign item {} is {}, above spec's {} terms",
            i + 1,
            assign.get(i).copied().unwrap_or(0),
            spec.nterms
        )));
    }
    if let Some(t) = (1..=spec.nterms).find(|t| !assign.contains(t)) {
        return Err(c.no(format!("assign codes no column for term {t}")));
    }
    // `names` distinct.
    for (j, nm) in names.iter().enumerate() {
        if names.get(..j).is_some_and(|b| b.contains(nm)) {
            return Err(c.no(format!("names repeat {}", render::text(nm))));
        }
    }
    // Every value finite, every weight positive.
    for (i, row) in x.iter().enumerate() {
        if let Some(j) = row.iter().position(|v| !v.is_finite()) {
            let cell = row.get(j).copied().unwrap_or(f64::NAN);
            return Err(c.no(format!(
                "matrix row {} column {} is {}",
                i + 1,
                j + 1,
                shown(cell)
            )));
        }
    }
    for (key, xs) in [("y", &y), ("weights", &weights), ("offset", &offset)] {
        if let Some(i) = xs.iter().position(|v| !v.is_finite()) {
            let cell = xs.get(i).copied().unwrap_or(f64::NAN);
            return Err(c.no(format!("{key} row {} is {}", i + 1, shown(cell))));
        }
    }
    if let Some(i) = weights.iter().position(|&w| w <= 0.0) {
        let cell = weights.get(i).copied().unwrap_or(0.0);
        return Err(c.no(format!("weights row {} is {}", i + 1, shown(cell))));
    }
    // `offset` and `weights` agreeing with `spec`.
    if spec.offset.is_none()
        && let Some(i) = offset.iter().position(|&o| o != 0.0)
    {
        let cell = offset.get(i).copied().unwrap_or(0.0);
        return Err(c.no(format!(
            "offset row {} is {}, and spec names no offset",
            i + 1,
            shown(cell)
        )));
    }
    if spec.weights.is_none()
        && let Some(i) = weights.iter().position(|&w| w != 1.0)
    {
        let cell = weights.get(i).copied().unwrap_or(1.0);
        return Err(c.no(format!(
            "weights row {} is {}, and spec names no weights",
            i + 1,
            shown(cell)
        )));
    }
    // n > k.
    if n <= k {
        return Err(c.no(format!("{n} observations for {k} coefficients")));
    }
    Ok(Design {
        x,
        names,
        assign,
        y,
        weights,
        offset,
        dropped,
        spec,
    })
}
