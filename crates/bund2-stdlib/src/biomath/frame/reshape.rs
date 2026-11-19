//! RFC-0017 M2: the rest of §S4. `frame.with`, `frame.select`,
//! `frame.take`, `frame.head`, `frame.filter`, `frame.complete`,
//! `frame.sort`, `frame.factor` and `frame.factor.levels`.
//!
//! Each word but `frame.with` only moves cells or changes `levels`, so the
//! frame it answers passes the check because its operand did: a cell keeps
//! its type and q, and a factor column keeps its levels (§S1). `frame.with`
//! takes cells from outside the frame and checks them as §S8's rows 8 to
//! 11, naming `list`.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BOOL, BundValue, INTEGER, LIST, NONE, STRING};

use super::check::{self, Frame, Kind, Who};
use super::{name_operand, opts, render};
use crate::biomath::dynamics::lambda;
use crate::biomath::operand::{WIDTH, domain, operands};
use crate::biomath::refuse::Refusal;

/// The column a `name` operand names, or E101.
fn column_of(word: &str, f: &Frame, v: &BundValue) -> Result<usize, Error> {
    let name = name_operand(word, v)?;
    f.index(&name)
        .ok_or_else(|| domain(word, "name", format!("no column {}", render::text(&name))))
}

/// A `names` operand: a LIST of STRINGs, each a column, none twice. Every
/// item's type is checked before any is looked up.
fn names_of(word: &str, op: &str, f: &Frame, v: &BundValue) -> Result<Vec<usize>, Error> {
    let items = match v.as_list() {
        Some(xs) if v.dt() == LIST => xs,
        _ => return Err(domain(word, op, v.summary(WIDTH))),
    };
    let mut names = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        match item.as_str() {
            Some(s) if item.dt() == STRING => names.push(s),
            _ => {
                return Err(domain(
                    word,
                    op,
                    format!("item {} is {}", i + 1, render::value(item)),
                ));
            }
        }
    }
    let mut out: Vec<usize> = Vec::with_capacity(names.len());
    for n in &names {
        let Some(i) = f.index(n) else {
            return Err(domain(word, op, format!("no column {}", render::text(n))));
        };
        if out.contains(&i) {
            return Err(domain(word, op, format!("{} given twice", render::text(n))));
        }
        out.push(i);
    }
    Ok(out)
}

/// The frame of the columns at `idx`, in that order, with their levels.
fn columns(f: &Frame, idx: &[usize]) -> Frame {
    let mut out = Frame {
        names: Vec::with_capacity(idx.len()),
        cols: Vec::with_capacity(idx.len()),
        levels: BTreeMap::new(),
        nrow: if idx.is_empty() { 0 } else { f.nrow },
    };
    for &i in idx {
        let (Some(n), Some(c)) = (f.names.get(i), f.cols.get(i)) else {
            continue;
        };
        if let Some(ls) = f.levels.get(n) {
            out.levels.insert(n.clone(), ls.clone());
        }
        out.names.push(n.clone());
        out.cols.push(c.clone());
    }
    out
}

/// `frame.with` (`frame name list -- frame`): the column replaced, or
/// appended last when new. A factor column replaced loses its levels. On a
/// frame of no columns any length is taken and sets the row count.
fn frame_with(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.with";
    let v = operands(vm, W, 3)?;
    let mut f = check::check(W, "frame", &v[0])?;
    let name = name_operand(W, &v[1])?;
    let cells = match v[2].as_list() {
        Some(xs) if v[2].dt() == LIST => xs.to_vec(),
        _ => return Err(domain(W, "list", v[2].summary(WIDTH))),
    };
    // A frame of no columns has 0 rows by convention only, so the new
    // column sets the count (RFC-0017, Note of 2026-11-18).
    if f.names.is_empty() {
        f.nrow = cells.len();
    } else if cells.len() != f.nrow {
        return Err(Refusal::Disagree {
            what: "the list and the frame's rows".into(),
            n: cells.len(),
            m: f.nrow,
        }
        .error(W));
    }
    match f.index(&name) {
        Some(i) => {
            if let Some(c) = f.cols.get_mut(i) {
                *c = cells;
            }
            f.levels.remove(&name);
        }
        None => {
            f.names.push(name);
            f.cols.push(cells);
        }
    }
    check::cells(
        W,
        Who {
            names: "frame",
            cells: "list",
        },
        &f,
    )?;
    vm.push(f.to_value());
    Ok(())
}

/// `frame.select` (`frame names -- frame`).
fn frame_select(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.select";
    let v = operands(vm, W, 2)?;
    let f = check::check(W, "frame", &v[0])?;
    let idx = names_of(W, "names", &f, &v[1])?;
    vm.push(columns(&f, &idx).to_value());
    Ok(())
}

/// `frame.take` (`frame rows -- frame`): 1-based indices, in order,
/// repeats allowed.
fn frame_take(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.take";
    let v = operands(vm, W, 2)?;
    let f = check::check(W, "frame", &v[0])?;
    let items = match v[1].as_list() {
        Some(xs) if v[1].dt() == LIST => xs,
        _ => return Err(domain(W, "rows", v[1].summary(WIDTH))),
    };
    let mut rows = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let r = match item.as_int() {
            Some(r) if item.dt() == INTEGER => r,
            _ => {
                return Err(domain(
                    W,
                    "rows",
                    format!("item {} is {}", i + 1, render::value(item)),
                ));
            }
        };
        match usize::try_from(r) {
            Ok(r) if r >= 1 && r <= f.nrow => rows.push(r - 1),
            _ => {
                let noun = if f.nrow == 1 { "row" } else { "rows" };
                return Err(domain(
                    W,
                    "rows",
                    format!("row {r} of a frame of {} {noun}", f.nrow),
                ));
            }
        }
    }
    vm.push(f.rows(&rows).to_value());
    Ok(())
}

/// `frame.head` (`frame n -- frame`): the first min(n, nrow) rows.
fn frame_head(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.head";
    let v = operands(vm, W, 2)?;
    let f = check::check(W, "frame", &v[0])?;
    let n = match v[1].as_int().map(usize::try_from) {
        Some(Ok(n)) if v[1].dt() == INTEGER => n,
        _ => return Err(domain(W, "n", v[1].summary(WIDTH))),
    };
    let rows: Vec<usize> = (0..n.min(f.nrow)).collect();
    vm.push(f.rows(&rows).to_value());
    Ok(())
}

/// Row `r` as a MAP of name to cell.
fn row_map(f: &Frame, r: usize) -> BundValue {
    BundValue::map(
        f.names
            .iter()
            .zip(&f.cols)
            .filter_map(|(n, c)| c.get(r).map(|x| (n.clone(), x.clone())))
            .collect(),
    )
}

/// `frame.filter` (`frame lambda -- frame`): the rows for which the lambda,
/// given the row as a MAP, leaves `true`. The lambda runs as `ode.rk4`'s
/// model runs, through the bridge, one row per call on a scratch stack;
/// anything but a BOOL on top is E112.
fn frame_filter(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.filter";
    let v = operands(vm, W, 2)?;
    let f = check::check(W, "frame", &v[0])?;
    let body = lambda(W, "lambda", &v[1])?;
    let mut keep = Vec::new();
    for r in 0..f.nrow {
        let left = crate::kb::bridge::ask(vm, body, row_map(&f, r))?;
        let refuse = |left: String| {
            Refusal::Model {
                left,
                expected: "a BOOL".into(),
            }
            .error(W)
        };
        match left {
            None => return Err(refuse("nothing".into())),
            Some(b) if b.dt() == BOOL => {
                if matches!(b.unboxed(), BundValue::Bool(true, _)) {
                    keep.push(r);
                }
            }
            Some(other) => return Err(refuse(check::kind_of(&other))),
        }
    }
    vm.push(f.rows(&keep).to_value());
    Ok(())
}

/// `frame.complete` (`frame names -- frame`): the rows with no NONE in
/// those columns, or in any column when `names` is empty.
fn frame_complete(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.complete";
    let v = operands(vm, W, 2)?;
    let f = check::check(W, "frame", &v[0])?;
    let mut idx = names_of(W, "names", &f, &v[1])?;
    if idx.is_empty() {
        idx = (0..f.names.len()).collect();
    }
    let rows: Vec<usize> = (0..f.nrow)
        .filter(|&r| {
            idx.iter().all(|&i| {
                f.cols
                    .get(i)
                    .and_then(|c| c.get(r))
                    .is_some_and(|x| x.dt() != NONE)
            })
        })
        .collect();
    vm.push(f.rows(&rows).to_value());
    Ok(())
}

/// Two non-NONE cells of one column, in §S4's order: numbers by exact
/// value (`-0.0` equal to `0.0`), STRINGs by bytes, BOOLs false first, a
/// factor's cells by their levels' order.
fn cell_order(a: &BundValue, b: &BundValue, levels: Option<&[String]>) -> Ordering {
    use BundValue::{Bool, Float, Int};
    match (a.unboxed(), b.unboxed()) {
        (Int(i, _), Int(j, _)) => i.cmp(j),
        (Float(x, _), Float(y, _)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (Int(i, _), Float(x, _)) => {
            crate::logic::exact_int_float_ord(*i, *x).unwrap_or(Ordering::Equal)
        }
        (Float(x, _), Int(i, _)) => {
            crate::logic::exact_int_float_ord(*i, *x).map_or(Ordering::Equal, Ordering::reverse)
        }
        (Bool(p, _), Bool(q, _)) => p.cmp(q),
        _ => {
            let (s, t) = (
                a.as_str().unwrap_or_default(),
                b.as_str().unwrap_or_default(),
            );
            match levels {
                Some(ls) => {
                    let at = |x: &String| ls.iter().position(|l| l == x);
                    at(&s).cmp(&at(&t))
                }
                None => s.as_bytes().cmp(t.as_bytes()),
            }
        }
    }
}

/// One sort key: the column's cells, its levels if a factor, and whether
/// it sorts descending.
type SortKey<'a> = (&'a [BundValue], Option<&'a [String]>, bool);

/// `frame.sort` (`frame names opts -- frame`): rows stably sorted by those
/// columns, in order; opts `descending` names those sorted descending.
/// NONE sorts last either way.
fn frame_sort(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.sort";
    let v = operands(vm, W, 3)?;
    let f = check::check(W, "frame", &v[0])?;
    let idx = names_of(W, "names", &f, &v[1])?;
    let o = opts::read(W, &v[2], &["descending"])?;
    let mut desc = vec![false; idx.len()];
    for d in o.descending.unwrap_or_default() {
        match idx
            .iter()
            .position(|&i| f.names.get(i).is_some_and(|n| *n == d))
        {
            Some(k) => {
                if let Some(slot) = desc.get_mut(k) {
                    *slot = true;
                }
            }
            None => {
                return Err(domain(
                    W,
                    "opts",
                    format!("descending names {}, not among names", render::text(&d)),
                ));
            }
        }
    }
    let keys: Vec<SortKey> = idx
        .iter()
        .zip(&desc)
        .filter_map(|(&i, &d)| {
            let c = f.cols.get(i)?;
            let ls = f
                .names
                .get(i)
                .and_then(|n| f.levels.get(n))
                .map(Vec::as_slice);
            Some((c.as_slice(), ls, d))
        })
        .collect();
    let mut rows: Vec<usize> = (0..f.nrow).collect();
    rows.sort_by(|&p, &q| {
        for (c, ls, d) in &keys {
            let (Some(a), Some(b)) = (c.get(p), c.get(q)) else {
                continue;
            };
            let o = match (a.dt() == NONE, b.dt() == NONE) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => {
                    let o = cell_order(a, b, *ls);
                    if *d { o.reverse() } else { o }
                }
            };
            if o != Ordering::Equal {
                return o;
            }
        }
        Ordering::Equal
    });
    vm.push(f.rows(&rows).to_value());
    Ok(())
}

/// The column `i` for `frame.factor`, `frame.factor.levels` and
/// `frame.csv.parse`'s `factors`: a string column, a factor or one of no
/// kind; a numeric or logical one is E101 naming `op`.
pub(super) fn factorable(word: &str, op: &str, f: &Frame, i: usize) -> Result<Kind, Error> {
    match f.kind(i) {
        k @ (Kind::Numeric | Kind::Logical) => {
            let n = f.names.get(i).map_or(String::new(), |n| render::text(n));
            Err(domain(word, op, format!("column {n} is {}", k.word())))
        }
        k => Ok(k),
    }
}

/// The STRING cells of column `i`, NONE skipped.
fn strings(f: &Frame, i: usize) -> impl Iterator<Item = (usize, String)> + '_ {
    f.cols
        .get(i)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(r, x)| x.as_str().filter(|_| x.dt() == STRING).map(|s| (r, s)))
}

/// `frame.factor` (`frame name -- frame`): a string column made a factor,
/// its levels its distinct values sorted by bytes; a factor's unused levels
/// dropped, the rest kept in order; a column of no kind a factor with no
/// levels.
fn frame_factor(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.factor";
    let v = operands(vm, W, 2)?;
    let mut f = check::check(W, "frame", &v[0])?;
    let i = column_of(W, &f, &v[1])?;
    let kind = factorable(W, "name", &f, i)?;
    make_factor(&mut f, i, kind);
    vm.push(f.to_value());
    Ok(())
}

/// Make column `i`, of `kind` (not numeric or logical), a factor: a factor
/// keeps its used levels in order, any other column takes its distinct
/// STRINGs sorted by bytes.
pub(super) fn make_factor(f: &mut Frame, i: usize, kind: Kind) {
    let used: Vec<String> = strings(f, i).map(|(_, s)| s).collect();
    let name = f.names.get(i).cloned().unwrap_or_default();
    let levels = match kind {
        Kind::Factor => f
            .levels
            .get(&name)
            .map(|ls| ls.iter().filter(|l| used.contains(l)).cloned().collect())
            .unwrap_or_default(),
        _ => {
            let mut ls = used;
            ls.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            ls.dedup();
            ls
        }
    };
    f.levels.insert(name, levels);
}

/// `frame.factor.levels` (`frame name levels -- frame`): the column made a
/// factor with the given levels, each cell required to be one.
fn frame_factor_levels(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.factor.levels";
    let v = operands(vm, W, 3)?;
    let mut f = check::check(W, "frame", &v[0])?;
    let i = column_of(W, &f, &v[1])?;
    factorable(W, "name", &f, i)?;
    let lv = &v[2];
    let items = match lv.as_list() {
        Some(xs) if lv.dt() == LIST && !xs.is_empty() => xs,
        _ => return Err(domain(W, "levels", lv.summary(WIDTH))),
    };
    let mut levels: Vec<String> = Vec::with_capacity(items.len());
    for item in items {
        let s = match item.as_str() {
            Some(s) if item.dt() == STRING => s,
            _ => return Err(domain(W, "levels", lv.summary(WIDTH))),
        };
        if levels.contains(&s) {
            return Err(domain(
                W,
                "levels",
                format!("{} given twice", render::text(&s)),
            ));
        }
        levels.push(s);
    }
    if let Some((r, s)) = strings(&f, i).find(|(_, s)| !levels.contains(s)) {
        return Err(domain(
            W,
            "levels",
            format!(
                "{} at row {} is not one of the levels",
                render::text(&s),
                r + 1
            ),
        ));
    }
    let name = f.names.get(i).cloned().unwrap_or_default();
    f.levels.insert(name, levels);
    vm.push(f.to_value());
    Ok(())
}

/// Register M2's words: each plain and `.`, but `frame.filter`, which runs
/// a lambda and so is opaque and plain only (§S1).
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("frame.with", frame_with, 3);
    word!("frame.select", frame_select, 2);
    word!("frame.take", frame_take, 2);
    word!("frame.head", frame_head, 2);
    word!("frame.complete", frame_complete, 2);
    word!("frame.sort", frame_sort, 3);
    word!("frame.factor", frame_factor, 2);
    word!("frame.factor.levels", frame_factor_levels, 3);
    r.register_native(
        "frame.filter",
        frame_filter,
        StackEffect::opaque(2),
        WordKind::Sync,
    );
}

/// M2's `frame.` words with their forms.
pub const WORDS: [&str; 17] = [
    "frame.complete",
    "frame.complete.",
    "frame.factor",
    "frame.factor.",
    "frame.factor.levels",
    "frame.factor.levels.",
    "frame.filter",
    "frame.head",
    "frame.head.",
    "frame.select",
    "frame.select.",
    "frame.sort",
    "frame.sort.",
    "frame.take",
    "frame.take.",
    "frame.with",
    "frame.with.",
];

#[cfg(test)]
mod tests;
