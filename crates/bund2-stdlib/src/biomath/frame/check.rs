//! RFC-0017 §S2's frame check, in the order of §S8's first eleven rows.
//!
//! Each rule runs over the columns in `columns` order and each column's rows
//! in order, and the check stops at the first failure, so a refusal names
//! the first offending part. Every rule runs before the next starts: a
//! LIST cell in the last column is refused before a NaN in the first.

use std::collections::BTreeMap;

use bund2_api::Error;
use bund2_value::{BOOL, BundValue, FLOAT, INTEGER, LIST, MAP, NODATA, NONE, STRING};

use super::render;
use crate::biomath::operand::{WIDTH, domain};
use crate::biomath::refuse::Refusal;

/// A frame that passed the check.
#[derive(Debug, Clone)]
pub(crate) struct Frame {
    /// `columns`, in order.
    pub names: Vec<String>,
    /// Each column's cells, in `names` order.
    pub cols: Vec<Vec<BundValue>>,
    /// `levels`: each factor column's levels, in order.
    pub levels: BTreeMap<String, Vec<String>>,
    /// The row count; 0 for a frame of no columns.
    pub nrow: usize,
}

impl Frame {
    /// The frame as the MAP §S2 describes.
    pub fn to_value(&self) -> BundValue {
        let mut data = BTreeMap::new();
        for (n, c) in self.names.iter().zip(&self.cols) {
            data.insert(n.clone(), BundValue::list(c.clone()));
        }
        let levels = self
            .levels
            .iter()
            .map(|(k, ls)| {
                let items = ls.iter().map(|l| BundValue::str(l.clone())).collect();
                (k.clone(), BundValue::list(items))
            })
            .collect();
        let mut m = BTreeMap::new();
        m.insert(
            "columns".into(),
            BundValue::list(
                self.names
                    .iter()
                    .map(|n| BundValue::str(n.clone()))
                    .collect(),
            ),
        );
        m.insert("data".into(), BundValue::map(data));
        m.insert("levels".into(), BundValue::map(levels));
        BundValue::map(m)
    }

    /// The column called `name`, if there is one.
    pub fn column(&self, name: &str) -> Option<&[BundValue]> {
        let i = self.index(name)?;
        self.cols.get(i).map(Vec::as_slice)
    }

    /// The position of the column called `name`.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// The kind of column `i`, read from its first non-NONE cell (§S2).
    pub fn kind(&self, i: usize) -> Kind {
        let Some(name) = self.names.get(i) else {
            return Kind::Unknown;
        };
        if self.levels.contains_key(name) {
            return Kind::Factor;
        }
        let first = self
            .cols
            .get(i)
            .and_then(|c| c.iter().find(|x| x.dt() != NONE));
        match first.map(BundValue::dt) {
            Some(INTEGER | FLOAT) => Kind::Numeric,
            Some(BOOL) => Kind::Logical,
            Some(STRING) => Kind::Text,
            _ => Kind::Unknown,
        }
    }

    /// The frame of the given rows, in that order, levels kept. Each index
    /// is below `nrow`; one that is not is a defect in the caller, and the
    /// row is skipped rather than invented.
    pub fn rows(&self, rows: &[usize]) -> Frame {
        let cols: Vec<Vec<BundValue>> = self
            .cols
            .iter()
            .map(|c| rows.iter().filter_map(|&r| c.get(r).cloned()).collect())
            .collect();
        let nrow = if self.names.is_empty() { 0 } else { rows.len() };
        Frame {
            names: self.names.clone(),
            cols,
            levels: self.levels.clone(),
            nrow,
        }
    }
}

/// Which operand a refusal names (§S8's preamble): the one the faulty part
/// came from. For a frame operand both are its name; `frame.new` names
/// `names` for a name fault and `columns` for a column or cell fault.
#[derive(Clone, Copy)]
pub(crate) struct Who<'a> {
    pub names: &'a str,
    pub cells: &'a str,
}

impl<'a> Who<'a> {
    /// A frame operand called `op`.
    pub fn frame(op: &'a str) -> Self {
        Who {
            names: op,
            cells: op,
        }
    }
}

/// How a value's type reads in a refusal: `a LIST`, `an INTEGER`.
pub(crate) fn kind_of(v: &BundValue) -> String {
    if v.dt() == NODATA {
        return "NODATA".into();
    }
    let t = v.type_name().to_uppercase();
    let article = if t.starts_with(['A', 'E', 'I', 'O', 'U']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {t}")
}

/// A column's kind (§S2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Numeric,
    Logical,
    Text,
    Factor,
    /// All NONE, or no rows, outside `levels`.
    Unknown,
}

impl Kind {
    /// How the kind reads in a refusal.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Numeric => "numeric",
            Kind::Logical => "logical",
            Kind::Text => "string",
            Kind::Factor => "a factor",
            Kind::Unknown => "of no kind",
        }
    }
}

fn is_scalar(v: &BundValue) -> bool {
    matches!(v.dt(), NONE | INTEGER | FLOAT | BOOL | STRING)
}

fn is_number(v: &BundValue) -> bool {
    matches!(v.dt(), INTEGER | FLOAT)
}

fn is_nonfinite(v: &BundValue) -> bool {
    matches!(v.unboxed(), BundValue::Float(x, _) if !x.is_finite())
}

/// Row 1 to 7 and 8 to 11 on a frame operand: E101, E102 or E103.
pub(crate) fn check(word: &str, op: &str, v: &BundValue) -> Result<Frame, Error> {
    let who = Who::frame(op);
    // Row 1: a MAP.
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(word, op, v.summary(WIDTH))),
    };
    // Row 2: exactly the three keys.
    for k in ["columns", "data", "levels"] {
        if !m.contains_key(k) {
            return Err(domain(
                word,
                op,
                format!("a missing key {}", render::text(k)),
            ));
        }
    }
    if let Some(k) = m
        .keys()
        .find(|k| !matches!(k.as_str(), "columns" | "data" | "levels"))
    {
        return Err(domain(
            word,
            op,
            format!("an extra key {}", render::text(k)),
        ));
    }
    let (columns, data, levels) = match (m.get("columns"), m.get("data"), m.get("levels")) {
        (Some(c), Some(d), Some(l)) => (c, d, l),
        _ => {
            return Err(Error::internal(format!(
                "{word}: the frame's three keys were checked present and one is missing"
            )));
        }
    };
    // Row 3: the names.
    let names = names(word, who, columns)?;
    // Row 4: `data`'s keys are exactly the names.
    let d = match data.as_map() {
        Some(d) if data.dt() == MAP => d,
        _ => return Err(domain(word, op, format!("data is {}", data.summary(WIDTH)))),
    };
    let mut cols = Vec::with_capacity(names.len());
    for n in &names {
        match d.get(n) {
            Some(c) => cols.push(c),
            None => {
                return Err(domain(
                    word,
                    op,
                    format!("data has no column {}", render::text(n)),
                ));
            }
        }
    }
    if let Some(k) = d.keys().find(|k| !names.contains(k)) {
        return Err(domain(
            word,
            op,
            format!("data has a key {} not in columns", render::text(k)),
        ));
    }
    parts(word, who, names, &cols, Some(levels))
}

/// Row 3: `columns` a LIST of distinct, non-empty STRINGs.
pub(crate) fn names(word: &str, who: Who, v: &BundValue) -> Result<Vec<String>, Error> {
    let items = match v.as_list() {
        Some(xs) if v.dt() == LIST => xs,
        _ => {
            return Err(domain(
                word,
                who.names,
                format!("columns is {}", v.summary(WIDTH)),
            ));
        }
    };
    let mut out: Vec<String> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let name = match item.as_str() {
            Some(s) if item.dt() == STRING && !s.is_empty() => s,
            _ => {
                return Err(domain(
                    word,
                    who.names,
                    format!("column name {} is {}", i + 1, item.summary(WIDTH)),
                ));
            }
        };
        if out.contains(&name) {
            return Err(domain(
                word,
                who.names,
                format!("a repeated name {}", render::text(&name)),
            ));
        }
        out.push(name);
    }
    Ok(out)
}

/// Rows 5 to 11, on names already checked and their columns.
pub(crate) fn parts(
    word: &str,
    who: Who,
    names: Vec<String>,
    cols: &[&BundValue],
    levels: Option<&BundValue>,
) -> Result<Frame, Error> {
    // Row 5: each column a LIST.
    let mut cells: Vec<Vec<BundValue>> = Vec::with_capacity(cols.len());
    for (n, c) in names.iter().zip(cols) {
        match c.as_list() {
            Some(xs) if c.dt() == LIST => cells.push(xs.to_vec()),
            _ => {
                return Err(domain(
                    word,
                    who.cells,
                    format!("column {} is {}", render::text(n), c.summary(WIDTH)),
                ));
            }
        }
    }
    // Row 6: one length.
    let nrow = cells.first().map_or(0, Vec::len);
    for (n, c) in names.iter().zip(&cells).skip(1) {
        if c.len() != nrow {
            let first = names.first().map_or(String::new(), |f| render::text(f));
            return Err(Refusal::Disagree {
                what: format!("the lengths of {first} and {}", render::text(n)),
                n: nrow,
                m: c.len(),
            }
            .error(word));
        }
    }
    // Row 7: `levels`.
    let levels = match levels {
        Some(l) => self::levels(word, who, &names, l)?,
        None => BTreeMap::new(),
    };
    cells_ok(word, who, &names, &cells, &levels)?;
    Ok(Frame {
        names,
        cols: cells,
        levels,
        nrow,
    })
}

/// Row 7: a MAP of column names to LISTs of distinct STRINGs.
fn levels(
    word: &str,
    who: Who,
    names: &[String],
    v: &BundValue,
) -> Result<BTreeMap<String, Vec<String>>, Error> {
    let op = who.cells;
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(word, op, format!("levels is {}", v.summary(WIDTH)))),
    };
    if let Some(k) = m.keys().find(|k| !names.contains(k)) {
        return Err(domain(
            word,
            op,
            format!("levels has a key {} not in columns", render::text(k)),
        ));
    }
    let mut out = BTreeMap::new();
    for n in names {
        let Some(l) = m.get(n) else { continue };
        let items = match l.as_list() {
            Some(xs) if l.dt() == LIST => xs,
            _ => {
                return Err(domain(
                    word,
                    op,
                    format!("levels of {} is {}", render::text(n), l.summary(WIDTH)),
                ));
            }
        };
        let mut ls: Vec<String> = Vec::with_capacity(items.len());
        for item in items {
            let s = match item.as_str() {
                Some(s) if item.dt() == STRING => s,
                _ => {
                    return Err(domain(
                        word,
                        op,
                        format!("levels of {} hold {}", render::text(n), item.summary(WIDTH)),
                    ));
                }
            };
            if ls.contains(&s) {
                return Err(domain(
                    word,
                    op,
                    format!("levels of {} repeat {}", render::text(n), render::text(&s)),
                ));
            }
            ls.push(s);
        }
        out.insert(n.clone(), ls);
    }
    Ok(out)
}

/// Rows 8 to 11, each over every column before the next.
fn cells_ok(
    word: &str,
    who: Who,
    names: &[String],
    cols: &[Vec<BundValue>],
    levels: &BTreeMap<String, Vec<String>>,
) -> Result<(), Error> {
    let op = who.cells;
    let at = |n: &String, r: usize| format!("column {} row {}", render::text(n), r + 1);
    // Row 8: scalars only.
    for (n, c) in names.iter().zip(cols) {
        if let Some(r) = c.iter().position(|x| !is_scalar(x)) {
            return Err(domain(
                word,
                op,
                format!("{} is {}", at(n, r), kind_of(&c[r])),
            ));
        }
    }
    // Row 9: no NaN or ±∞ (B12).
    for (n, c) in names.iter().zip(cols) {
        if let Some(r) = c.iter().position(is_nonfinite) {
            return Err(domain(
                word,
                op,
                format!("{} is {}", at(n, r), c[r].summary(WIDTH)),
            ));
        }
    }
    // Row 10: a number beside a STRING or BOOL, outside `levels`: E103 on
    // the first STRING or BOOL.
    for (n, c) in names.iter().zip(cols) {
        if levels.contains_key(n) || !c.iter().any(is_number) {
            continue;
        }
        if let Some(r) = c.iter().position(|x| matches!(x.dt(), STRING | BOOL)) {
            return Err(Refusal::NotNumbers {
                operand: format!("column {}", render::text(n)),
                container: "LIST",
                cell: format!("{} at row {}", render::value(&c[r]), r + 1),
            }
            .error(word));
        }
    }
    // Row 11: any other mixture outside `levels`; in a factor column, a
    // cell not among its levels.
    for (n, c) in names.iter().zip(cols) {
        if let Some(ls) = levels.get(n) {
            let bad = c.iter().position(|x| {
                x.dt() != NONE && !(x.dt() == STRING && x.as_str().is_some_and(|s| ls.contains(&s)))
            });
            if let Some(r) = bad {
                return Err(domain(
                    word,
                    op,
                    format!(
                        "{} is {}, not one of its levels",
                        at(n, r),
                        render::value(&c[r])
                    ),
                ));
            }
            continue;
        }
        let Some(first) = c.iter().find(|x| x.dt() != NONE) else {
            continue;
        };
        let class = |x: &BundValue| if is_number(x) { INTEGER } else { x.dt() };
        let kind = class(first);
        if let Some(r) = c.iter().position(|x| x.dt() != NONE && class(x) != kind) {
            let column = match kind {
                BOOL => "logical",
                STRING => "string",
                _ => "numeric",
            };
            return Err(domain(
                word,
                op,
                format!("{} is {}; the column is {column}", at(n, r), kind_of(&c[r])),
            ));
        }
    }
    Ok(())
}

/// Rows 8 to 11 on a frame built from a checked one and new cells
/// (`frame.with`), naming `who.cells` for a fault.
pub(crate) fn cells(word: &str, who: Who, f: &Frame) -> Result<(), Error> {
    cells_ok(word, who, &f.names, &f.cols, &f.levels)
}
