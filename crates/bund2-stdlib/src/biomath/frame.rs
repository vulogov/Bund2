//! RFC-0017: frames, factors and missing values. M1 adds §S2's frame and
//! its check, and §S4's first words: `frame.new`, `frame.check`,
//! `frame.nrow`, `frame.ncol`, `frame.names`, `frame.column` and
//! `frame.rows`. M2 adds the rest of §S4 in [`reshape`], with the options
//! reader its `frame.sort` needs ([`opts`]). M3 adds §S6: the CSV reader
//! and writer ([`csv`]) and `frame.text`'s table ([`table`]). M4 adds §S5:
//! `frame.aggregate`, `frame.join` and `frame.matrix` ([`summary`]).
//!
//! **A frame is a plain MAP** (D252) of exactly `columns`, `data` and
//! `levels` (D253 P3): the names in order, each name to its LIST of cells,
//! and each factor column's name to its levels. Every word checks its frame
//! operand on entry ([`check`]), in the order of §S8's first eleven rows, and
//! every frame it answers passes the same check. Cells are moved, never
//! converted, so each keeps its type and q (D32).
//!
//! **Refusals** are RFC-0015 §S11's codes, with §S8's texts. A value a text
//! takes from the program is rendered by [`render`]: escaped, bounded and
//! quoted, so no refusal breaks across lines.

pub(crate) mod check;
pub mod csv;
pub(crate) mod opts;
pub(crate) mod render;
pub mod reshape;
pub mod summary;
pub(crate) mod table;

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST, STRING};

use super::operand::{WIDTH, domain, operands};
use super::refuse::Refusal;
use check::{Frame, Who};

/// The count as an INTEGER.
fn count(n: usize) -> BundValue {
    BundValue::int(i64::try_from(n).unwrap_or(i64::MAX))
}

/// A `name` operand: a non-empty STRING, else E101.
pub(crate) fn name_operand(word: &str, v: &BundValue) -> Result<String, Error> {
    match v.as_str() {
        Some(s) if v.dt() == STRING && !s.is_empty() => Ok(s),
        Some(_) if v.dt() == STRING => Err(domain(word, "name", "an empty name".into())),
        _ => Err(domain(word, "name", v.summary(WIDTH))),
    }
}

/// `frame.new` (`names columns -- frame`): no factors.
fn frame_new(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.new";
    let v = operands(vm, W, 2)?;
    let (names_v, cols_v) = (&v[0], &v[1]);
    if names_v.dt() != LIST {
        return Err(domain(W, "names", names_v.summary(WIDTH)));
    }
    let cols = match cols_v.as_list() {
        Some(xs) if cols_v.dt() == LIST => xs,
        _ => return Err(domain(W, "columns", cols_v.summary(WIDTH))),
    };
    let n = names_v.as_list().map_or(0, <[BundValue]>::len);
    if n != cols.len() {
        return Err(Refusal::Disagree {
            what: "names and columns".into(),
            n,
            m: cols.len(),
        }
        .error(W));
    }
    let who = Who {
        names: "names",
        cells: "columns",
    };
    let names = check::names(W, who, names_v)?;
    let refs: Vec<&BundValue> = cols.iter().collect();
    let frame = check::parts(W, who, names, &refs, None)?;
    vm.push(frame.to_value());
    Ok(())
}

/// `frame.check` (`x -- frame`): x, if it is a frame.
fn frame_check(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.check";
    let v = operands(vm, W, 1)?;
    check::check(W, "frame", &v[0])?;
    let [x] = <[BundValue; 1]>::try_from(v)
        .map_err(|_| Error::internal(format!("{W}: one operand was pulled and none is held")))?;
    vm.push(x);
    Ok(())
}

/// The one frame operand, checked.
fn one(vm: &mut dyn Vm, word: &str) -> Result<Frame, Error> {
    let v = operands(vm, word, 1)?;
    check::check(word, "frame", &v[0])
}

/// `frame.nrow` (`frame -- n`): 0 for a frame of no columns.
fn frame_nrow(vm: &mut dyn Vm) -> Result<(), Error> {
    let f = one(vm, "frame.nrow")?;
    vm.push(count(f.nrow));
    Ok(())
}

/// `frame.ncol` (`frame -- n`).
fn frame_ncol(vm: &mut dyn Vm) -> Result<(), Error> {
    let f = one(vm, "frame.ncol")?;
    vm.push(count(f.names.len()));
    Ok(())
}

/// `frame.names` (`frame -- list`): `columns`.
fn frame_names(vm: &mut dyn Vm) -> Result<(), Error> {
    let f = one(vm, "frame.names")?;
    vm.push(BundValue::list(
        f.names.into_iter().map(BundValue::str).collect(),
    ));
    Ok(())
}

/// `frame.column` (`frame name -- list`): one column's cells.
fn frame_column(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.column";
    let v = operands(vm, W, 2)?;
    let f = check::check(W, "frame", &v[0])?;
    let name = name_operand(W, &v[1])?;
    match f.column(&name) {
        Some(c) => {
            vm.push(BundValue::list(c.to_vec()));
            Ok(())
        }
        None => Err(domain(
            W,
            "name",
            format!("no column {}", render::text(&name)),
        )),
    }
}

/// `frame.rows` (`frame -- list`): each row as a MAP of name to cell.
fn frame_rows(vm: &mut dyn Vm) -> Result<(), Error> {
    let f = one(vm, "frame.rows")?;
    let rows = (0..f.nrow)
        .map(|r| {
            let m: BTreeMap<String, BundValue> = f
                .names
                .iter()
                .zip(&f.cols)
                .filter_map(|(n, c)| c.get(r).map(|x| (n.clone(), x.clone())))
                .collect();
            BundValue::map(m)
        })
        .collect();
    vm.push(BundValue::list(rows));
    Ok(())
}

/// Register M1's `frame.` words, each plain and `.`: the `.` form takes the
/// top operand from the workbench and answers there (RFC-0015 §S3).
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("frame.new", frame_new, 2);
    word!("frame.check", frame_check, 1);
    word!("frame.nrow", frame_nrow, 1);
    word!("frame.ncol", frame_ncol, 1);
    word!("frame.names", frame_names, 1);
    word!("frame.column", frame_column, 2);
    word!("frame.rows", frame_rows, 1);
    reshape::register(r);
    csv::register(r);
    summary::register(r);
}

/// M1's `frame.` words with their forms; M2's are [`reshape::WORDS`], M3's
/// [`csv::WORDS`], M4's [`summary::WORDS`].
pub const WORDS: [&str; 14] = [
    "frame.check",
    "frame.check.",
    "frame.column",
    "frame.column.",
    "frame.names",
    "frame.names.",
    "frame.ncol",
    "frame.ncol.",
    "frame.new",
    "frame.new.",
    "frame.nrow",
    "frame.nrow.",
    "frame.rows",
    "frame.rows.",
];

#[cfg(test)]
mod tests;
