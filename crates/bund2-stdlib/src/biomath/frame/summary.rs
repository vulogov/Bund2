//! RFC-0017 M4: §S5's summaries. `frame.aggregate`, `frame.join` and
//! `frame.matrix`.
//!
//! **`frame.aggregate`** names its statistics rather than taking lambdas
//! (D253 P5), and computes each with the very function the named word
//! uses (`vec.sum`'s [`total`], `stat.mean.arithmetic`'s and `stat.var`'s
//! [`mean`] and [`variance`], `stat.median`'s [`median_of`]), so a group's
//! finite answer is that word's on the group's cells, bit for bit. It
//! checks a group's size before computing and the answer's finiteness
//! after, so §S8's texts are the only ones a program sees. Its refusals
//! come in §S5's order: the frame, then `by`, `stats`, `opts` and the
//! result names, then the cells under `na`, then each group in the
//! answer's order and, within it, each statistic in order.
//!
//! **Groups** are `frame.sort`'s order over the `by` columns, NONE last;
//! two rows are one group when every `by` cell compares equal there, so
//! `1` and `1.0` are one group, and the group's cell is its first in row
//! order.
//!
//! **`frame.join`** matches rows by sorting `right`'s keyed rows once and
//! searching that order for each row of `left`, so a row matching several
//! of `right`'s gets them in `right`'s order; the answer's row count is
//! summed and capped (D39) before anything is allocated.
//!
//! **`frame.matrix`** answers the named numeric columns as a MATRIX of
//! FLOATs, as `stat.lm`'s `table` reads one.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, FLOAT, LIST, NONE, STRING};

use super::check::{self, Frame, Kind};
use super::opts::{self, How, Missing};
use super::render;
use super::reshape::{cell_order, names_of};
use crate::biomath::describe::{mean, median_of, variance};
use crate::biomath::na::none;
use crate::biomath::operand::{WIDTH, domain, operands};
use crate::biomath::refuse::Refusal;
use crate::biomath::vector::total;

/// The most rows `frame.join` answers (D39).
const CAP: u64 = 100_000_000;

/// The name of `.dropped`, the count of rows `"omit"` dropped.
const DROPPED: &str = ".dropped";

/// A statistic `frame.aggregate` knows (§S5's table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stat {
    Count,
    Sum,
    Mean,
    Var,
    Sd,
    Median,
    Min,
    Max,
}

impl Stat {
    const ALL: [Stat; 8] = [
        Stat::Count,
        Stat::Sum,
        Stat::Mean,
        Stat::Var,
        Stat::Sd,
        Stat::Median,
        Stat::Min,
        Stat::Max,
    ];

    fn name(self) -> &'static str {
        match self {
            Stat::Count => "count",
            Stat::Sum => "sum",
            Stat::Mean => "mean",
            Stat::Var => "var",
            Stat::Sd => "sd",
            Stat::Median => "median",
            Stat::Min => "min",
            Stat::Max => "max",
        }
    }

    fn parse(s: &str) -> Option<Stat> {
        Self::ALL.into_iter().find(|t| t.name() == s)
    }

    /// The fewest cells the statistic is computed from: two for the
    /// sample variance and its root, as `stat.var` asks, one for the
    /// others; `"count"` reads no cell.
    fn least(self) -> usize {
        match self {
            Stat::Count => 0,
            Stat::Var | Stat::Sd => 2,
            _ => 1,
        }
    }

    /// The statistic of `xs`, unchecked; the caller has checked
    /// `xs.len() >= self.least()`.
    fn compute(self, xs: &[f64]) -> f64 {
        match self {
            Stat::Count => xs.len() as f64,
            Stat::Sum => total(xs),
            Stat::Mean => mean(xs),
            Stat::Var => variance(xs),
            Stat::Sd => variance(xs).sqrt(),
            Stat::Median => median_of(xs),
            Stat::Min => first_best(xs, |x, m| x < m),
            Stat::Max => first_best(xs, |x, m| x > m),
        }
    }
}

/// The cell `better` picks over every other, the first of a tie: `-0.0`
/// and `0.0` compare equal, so whichever comes first is kept. The cells
/// are finite (the frame check's row 9), so nothing folds from NaN.
fn first_best(xs: &[f64], better: fn(f64, f64) -> bool) -> f64 {
    let mut it = xs.iter().copied();
    let Some(first) = it.next() else {
        return f64::NAN;
    };
    it.fold(first, |m, x| if better(x, m) { x } else { m })
}

/// A numeric cell as the named words convert it (`conv_value` to FLOAT).
fn number(word: &str, v: &BundValue) -> Result<f64, Error> {
    let f = crate::convert::conv_value(v, FLOAT).ok();
    match f.as_ref().map(BundValue::unboxed) {
        Some(BundValue::Float(x, _)) => Ok(*x),
        _ => Err(Error::internal(format!(
            "{word}: a cell of a numeric column, {}, did not convert to FLOAT; \
             the frame check admits only INTEGER and FLOAT there",
            v.summary(WIDTH)
        ))),
    }
}

/// How a column that is not numeric reads after `"mean" of column "g", `.
fn not_numeric(k: Kind) -> &'static str {
    match k {
        Kind::Factor => "a factor",
        Kind::Text => "a string column",
        Kind::Logical => "a logical column",
        Kind::Numeric | Kind::Unknown => "a numeric column",
    }
}

/// The name of column `i`; every index here came from `f.index`.
fn name(f: &Frame, i: usize) -> &str {
    f.names.get(i).map_or("", String::as_str)
}

/// The cells of column `i`.
fn col(f: &Frame, i: usize) -> &[BundValue] {
    f.cols.get(i).map_or(&[], Vec::as_slice)
}

/// `stats`: a non-empty LIST of `[ column statistic ]` pairs, each a known
/// statistic of a column, a numeric one or one of no kind unless the
/// statistic is `"count"`.
fn stats_of(word: &str, f: &Frame, v: &BundValue) -> Result<Vec<(usize, Stat)>, Error> {
    let refuse = |t: String| domain(word, "stats", t);
    let items = match v.as_list() {
        Some(xs) if v.dt() == LIST => xs,
        _ => return Err(refuse(v.summary(WIDTH))),
    };
    if items.is_empty() {
        return Err(refuse("no statistics".into()));
    }
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let pair: Option<(String, String)> = match item.as_list() {
            Some([c, s]) if item.dt() == LIST && c.dt() == STRING && s.dt() == STRING => {
                c.as_str().zip(s.as_str())
            }
            _ => None,
        };
        let Some((c, s)) = pair else {
            return Err(refuse(format!("item {} is {}", i + 1, render::value(item))));
        };
        let Some(stat) = Stat::parse(&s) else {
            return Err(refuse(format!("{} is not a statistic", render::text(&s))));
        };
        let Some(ci) = f.index(&c) else {
            return Err(refuse(format!("no column {}", render::text(&c))));
        };
        let k = f.kind(ci);
        if stat != Stat::Count && !matches!(k, Kind::Numeric | Kind::Unknown) {
            return Err(refuse(format!(
                "{} of column {}, {}",
                render::text(&s),
                render::text(&c),
                not_numeric(k)
            )));
        }
        out.push((ci, stat));
    }
    Ok(out)
}

/// The rows of `f` with no NONE in the columns `reads`, under `na`: every
/// row under `"omit"`'s keep mask, or E101 at the first NONE under
/// `"refuse"`, columns in `reads` order, rows from the first.
fn kept(word: &str, f: &Frame, reads: &[usize], missing: Missing) -> Result<Vec<bool>, Error> {
    let mut keep = vec![true; f.nrow];
    for &c in reads {
        for (r, x) in col(f, c).iter().enumerate() {
            if x.dt() != NONE {
                continue;
            }
            match missing {
                Missing::Refuse => {
                    return Err(domain(
                        word,
                        "frame",
                        format!(
                            "column {} row {} is none (na \"refuse\")",
                            render::text(name(f, c)),
                            r + 1
                        ),
                    ));
                }
                Missing::Omit => {
                    if let Some(k) = keep.get_mut(r) {
                        *k = false;
                    }
                }
            }
        }
    }
    Ok(keep)
}

/// Two rows' cells in the `by` columns, in `frame.sort`'s order: NONE last,
/// a factor by its levels.
fn by_order(keys: &[(&[BundValue], Option<&[String]>)], p: usize, q: usize) -> Ordering {
    for (c, ls) in keys {
        let (Some(a), Some(b)) = (c.get(p), c.get(q)) else {
            continue;
        };
        let o = match (a.dt() == NONE, b.dt() == NONE) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => cell_order(a, b, *ls),
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    Ordering::Equal
}

/// How a group reads in a refusal (§S8).
fn group_text(f: &Frame, by: &[usize], first: usize) -> String {
    if by.is_empty() {
        return "group (all rows)".into();
    }
    let keys: Vec<String> = by
        .iter()
        .map(|&i| col(f, i).get(first).map_or_else(String::new, render::value))
        .collect();
    format!("group [ {} ]", keys.join(" "))
}

/// `frame.aggregate` (`frame by stats opts -- frame`): one row per group of
/// the `by` columns, one column per statistic, and `.dropped` under
/// `"omit"`.
fn frame_aggregate(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.aggregate";
    let v = operands(vm, W, 4)?;
    let f = check::check(W, "frame", &v[0])?;
    let by = names_of(W, "by", &f, &v[1])?;
    let stats = stats_of(W, &f, &v[2])?;
    let missing = opts::read(W, &v[3], &["na"])?
        .missing
        .unwrap_or(Missing::Refuse);
    let by_names: Vec<&str> = by.iter().map(|&i| name(&f, i)).collect();
    let mut results: Vec<String> = Vec::with_capacity(stats.len());
    for &(c, s) in &stats {
        let r = format!("{}.{}", name(&f, c), s.name());
        if by_names.contains(&r.as_str()) {
            return Err(domain(
                W,
                "stats",
                format!("result name {} repeats a by column", render::text(&r)),
            ));
        }
        if results.contains(&r) {
            return Err(domain(
                W,
                "stats",
                format!("result name {} repeats another", render::text(&r)),
            ));
        }
        results.push(r);
    }
    if missing == Missing::Omit && by_names.contains(&DROPPED) {
        return Err(domain(
            W,
            "stats",
            format!("result name {} repeats a by column", render::text(DROPPED)),
        ));
    }
    // What the statistics read: `"count"` counts rows and reads no column.
    let mut reads: Vec<usize> = Vec::new();
    for &(c, s) in &stats {
        if s != Stat::Count && !reads.contains(&c) {
            reads.push(c);
        }
    }
    let keep = kept(W, &f, &reads, missing)?;

    let keys: Vec<(&[BundValue], Option<&[String]>)> = by
        .iter()
        .map(|&i| {
            let ls = f.levels.get(name(&f, i)).map(Vec::as_slice);
            (col(&f, i), ls)
        })
        .collect();
    let mut order: Vec<usize> = (0..f.nrow).collect();
    order.sort_by(|&p, &q| by_order(&keys, p, q));
    let groups: Vec<&[usize]> = order
        .chunk_by(|&p, &q| by_order(&keys, p, q) == Ordering::Equal)
        .collect();

    let mut key_cols: Vec<Vec<BundValue>> = vec![Vec::with_capacity(groups.len()); by.len()];
    let mut stat_cols: Vec<Vec<BundValue>> = vec![Vec::with_capacity(groups.len()); stats.len()];
    let mut dropped: Vec<BundValue> = Vec::with_capacity(groups.len());
    for rows in &groups {
        let Some(&first) = rows.first() else {
            continue;
        };
        for (out, &i) in key_cols.iter_mut().zip(&by) {
            out.push(col(&f, i).get(first).cloned().unwrap_or_else(none));
        }
        let used: Vec<usize> = rows
            .iter()
            .copied()
            .filter(|&r| keep.get(r) == Some(&true))
            .collect();
        dropped.push(super::count(rows.len() - used.len()));
        for ((out, &(c, s)), result) in stat_cols.iter_mut().zip(&stats).zip(&results) {
            if s == Stat::Count {
                out.push(super::count(used.len()));
                continue;
            }
            let cells = col(&f, c);
            let xs = used
                .iter()
                .filter_map(|&r| cells.get(r))
                .map(|x| number(W, x))
                .collect::<Result<Vec<f64>, Error>>()?;
            if xs.len() < s.least() {
                let noun = if xs.len() == 1 { "value" } else { "values" };
                return Err(domain(
                    W,
                    "stats",
                    format!(
                        "{} has {} {noun} for {}",
                        group_text(&f, &by, first),
                        xs.len(),
                        render::text(s.name())
                    ),
                ));
            }
            let y = s.compute(&xs);
            if !y.is_finite() {
                return Err(Refusal::Overflow {
                    expression: format!(
                        "{} of {}",
                        render::text(result),
                        group_text(&f, &by, first)
                    ),
                }
                .error(W));
            }
            out.push(BundValue::float(y));
        }
    }

    let mut out = Frame {
        names: by_names.iter().map(|n| (*n).to_string()).collect(),
        cols: key_cols,
        levels: BTreeMap::new(),
        nrow: groups.len(),
    };
    for n in &by_names {
        if let Some(ls) = f.levels.get(*n) {
            out.levels.insert((*n).to_string(), ls.clone());
        }
    }
    out.names.extend(results);
    out.cols.extend(stat_cols);
    if missing == Missing::Omit {
        out.names.push(DROPPED.into());
        out.cols.push(dropped);
    }
    vm.push(out.to_value());
    Ok(())
}

/// `frame.join`'s `by`: a non-empty LIST of STRINGs, each a column of both
/// frames, none twice; each as its index in `left` and in `right`.
fn keys_of(
    word: &str,
    left: &Frame,
    right: &Frame,
    v: &BundValue,
) -> Result<Vec<(usize, usize)>, Error> {
    let refuse = |t: String| domain(word, "by", t);
    let items = match v.as_list() {
        Some(xs) if v.dt() == LIST => xs,
        _ => return Err(refuse(v.summary(WIDTH))),
    };
    let mut names = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        match item.as_str() {
            Some(s) if item.dt() == STRING => names.push(s),
            _ => {
                return Err(refuse(format!("item {} is {}", i + 1, render::value(item))));
            }
        }
    }
    if names.is_empty() {
        return Err(refuse("no keys".into()));
    }
    let mut out: Vec<(usize, usize)> = Vec::with_capacity(names.len());
    for n in &names {
        let Some(l) = left.index(n) else {
            return Err(refuse(format!("no column {}", render::text(n))));
        };
        if out.iter().any(|&(i, _)| i == l) {
            return Err(refuse(format!("{} given twice", render::text(n))));
        }
        let Some(r) = right.index(n) else {
            return Err(refuse(format!("no column {} in right", render::text(n))));
        };
        out.push((l, r));
    }
    Ok(out)
}

/// Whether two key columns' kinds agree: a factor and a string column are
/// one kind, and a column of no kind agrees with any.
fn agree(a: Kind, b: Kind) -> bool {
    let text = |k| matches!(k, Kind::Factor | Kind::Text);
    a == b || a == Kind::Unknown || b == Kind::Unknown || (text(a) && text(b))
}

/// `frame.join` (`left right by opts -- frame`): `left`'s rows with their
/// matches in `right`, `left`'s columns then `right`'s other than `by`.
fn frame_join(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.join";
    let v = operands(vm, W, 4)?;
    let left = check::check(W, "left", &v[0])?;
    let right = check::check(W, "right", &v[1])?;
    let keys = keys_of(W, &left, &right, &v[2])?;
    let how = opts::read(W, &v[3], &["how"])?.how.unwrap_or(How::Inner);
    for &(l, r) in &keys {
        let (a, b) = (left.kind(l), right.kind(r));
        if !agree(a, b) {
            return Err(domain(
                W,
                "by",
                format!(
                    "key {} is {} in left and {} in right",
                    render::text(name(&left, l)),
                    a.word(),
                    b.word()
                ),
            ));
        }
    }
    let rest: Vec<usize> = (0..right.names.len())
        .filter(|j| !keys.iter().any(|&(_, r)| r == *j))
        .collect();
    for &j in &rest {
        let n = name(&right, j);
        if left.index(n).is_some() {
            return Err(domain(
                W,
                "by",
                format!("{} is in both frames", render::text(n)),
            ));
        }
    }

    let lk: Vec<&[BundValue]> = keys.iter().map(|&(l, _)| col(&left, l)).collect();
    let rk: Vec<&[BundValue]> = keys.iter().map(|&(_, r)| col(&right, r)).collect();
    let keyed =
        |cs: &[&[BundValue]], p: usize| cs.iter().all(|c| c.get(p).is_some_and(|x| x.dt() != NONE));
    // Right row q against left row p, key by key. Factor keys compare by
    // their text, which is what matching by level text asks.
    let vs = |q: usize, p: usize| -> Ordering {
        for (rc, lc) in rk.iter().zip(&lk) {
            if let (Some(a), Some(b)) = (rc.get(q), lc.get(p)) {
                let o = cell_order(a, b, None);
                if o != Ordering::Equal {
                    return o;
                }
            }
        }
        Ordering::Equal
    };
    let mut order: Vec<usize> = (0..right.nrow).filter(|&q| keyed(&rk, q)).collect();
    order.sort_by(|&p, &q| {
        for c in &rk {
            if let (Some(a), Some(b)) = (c.get(p), c.get(q)) {
                let o = cell_order(a, b, None);
                if o != Ordering::Equal {
                    return o;
                }
            }
        }
        Ordering::Equal
    });
    // Each left row's run of matches in `order`, which keeps `right`'s
    // order among equal keys because the sort is stable.
    let spans: Vec<(usize, usize)> = (0..left.nrow)
        .map(|p| {
            if !keyed(&lk, p) {
                return (0, 0);
            }
            let lo = order.partition_point(|&q| vs(q, p) == Ordering::Less);
            let hi = order.partition_point(|&q| vs(q, p) != Ordering::Greater);
            (lo, hi.max(lo))
        })
        .collect();
    let mut n: u64 = 0;
    for &(lo, hi) in &spans {
        let m = (hi - lo) as u64;
        n = n.saturating_add(if m == 0 && how == How::Left { 1 } else { m });
    }
    if n > CAP {
        return Err(Refusal::Cap {
            what: "the join's row count".into(),
            cap: CAP,
        }
        .error(W));
    }
    let mut pairs: Vec<(usize, Option<usize>)> = Vec::with_capacity(n as usize);
    for (p, &(lo, hi)) in spans.iter().enumerate() {
        let matched = order.get(lo..hi).unwrap_or(&[]);
        if matched.is_empty() && how == How::Left {
            pairs.push((p, None));
        }
        pairs.extend(matched.iter().map(|&q| (p, Some(q))));
    }

    let mut out = Frame {
        names: left.names.clone(),
        cols: left
            .cols
            .iter()
            .map(|c| {
                pairs
                    .iter()
                    .filter_map(|&(p, _)| c.get(p).cloned())
                    .collect()
            })
            .collect(),
        levels: left.levels.clone(),
        nrow: pairs.len(),
    };
    for &j in &rest {
        let n = name(&right, j).to_string();
        let c = col(&right, j);
        out.cols.push(
            pairs
                .iter()
                .map(|&(_, q)| q.and_then(|q| c.get(q).cloned()).unwrap_or_else(none))
                .collect(),
        );
        if let Some(ls) = right.levels.get(&n) {
            out.levels.insert(n.clone(), ls.clone());
        }
        out.names.push(n);
    }
    vm.push(out.to_value());
    Ok(())
}

/// `frame.matrix` (`frame names opts -- MATRIX`): the named numeric columns
/// as a MATRIX of FLOATs, rows by rows; under `"omit"`, `{ matrix,
/// dropped }`.
fn frame_matrix(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.matrix";
    let v = operands(vm, W, 3)?;
    let f = check::check(W, "frame", &v[0])?;
    let idx = names_of(W, "names", &f, &v[1])?;
    if idx.is_empty() {
        return Err(domain(W, "names", "no columns".into()));
    }
    for &i in &idx {
        let k = f.kind(i);
        if !matches!(k, Kind::Numeric | Kind::Unknown) {
            return Err(domain(
                W,
                "names",
                format!("column {} is {}", render::text(name(&f, i)), k.word()),
            ));
        }
    }
    let missing = opts::read(W, &v[2], &["na"])?
        .missing
        .unwrap_or(Missing::Refuse);
    let no_rows = || domain(W, "frame", "no rows".into());
    if f.nrow == 0 {
        return Err(no_rows());
    }
    let keep = kept(W, &f, &idx, missing)?;
    let rows: Vec<usize> = (0..f.nrow)
        .filter(|&r| keep.get(r) == Some(&true))
        .collect();
    if rows.is_empty() {
        return Err(no_rows());
    }
    let mut m = Vec::with_capacity(rows.len());
    for &r in &rows {
        let row = idx
            .iter()
            .filter_map(|&i| col(&f, i).get(r))
            .map(|x| number(W, x).map(BundValue::float))
            .collect::<Result<Vec<BundValue>, Error>>()?;
        m.push(row);
    }
    let matrix = BundValue::matrix(m);
    match missing {
        Missing::Refuse => vm.push(matrix),
        Missing::Omit => {
            let mut out = BTreeMap::new();
            out.insert("matrix".to_string(), matrix);
            out.insert("dropped".to_string(), super::count(f.nrow - rows.len()));
            vm.push(BundValue::map(out));
        }
    }
    Ok(())
}

/// Register M4's words, each plain and `.` (§S1).
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("frame.aggregate", frame_aggregate, 4);
    word!("frame.join", frame_join, 4);
    word!("frame.matrix", frame_matrix, 3);
}

/// M4's `frame.` words with their forms.
pub const WORDS: [&str; 6] = [
    "frame.aggregate",
    "frame.aggregate.",
    "frame.join",
    "frame.join.",
    "frame.matrix",
    "frame.matrix.",
];

#[cfg(test)]
mod tests;
