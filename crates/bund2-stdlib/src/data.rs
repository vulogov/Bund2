//! `csv` — a data file read row by row into a conditional's lambda.
//!
//! It is a conditional. The word checks that the file exists and pushes a
//! CONDITIONAL of type `csv` naming it. The program `set`s a `lambda`, and
//! optionally `column` or `is_header`, and runs it with `!`
//! (`reference/Bund/src/stdlib/functions/conditional/mod.rs:26,42`).
//!
//! **D51: the `csv` crate, not polars.** The reference reads through a polars
//! DataFrame (`reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:62-168`),
//! and what a row looks like is polars' answer: which columns become integers,
//! floats, booleans or strings, and which cells are null. So this reads with the
//! pure-Rust `csv` crate and then **infers types as polars 0.46 does**. Each
//! field is tested in polars' order: a case-insensitive `true`/`false`, then
//! polars' float pattern, then an integer, else a string
//! (`polars-io-0.46.0/src/csv/read/schema_inference.rs:139-146`,
//! patterns at `src/utils/other.rs:156-170`). A column that saw one kind keeps
//! it, one that saw only integers and floats is a float, and anything else is a
//! string (`schema_inference.rs:91-105`). This is judged over the first 100
//! rows, polars' default (`src/csv/read/options.rs:84`). An empty field is null,
//! and a null cell is **left out of its row**: the reference skips every
//! polars value it has no arm for (`conditional_csv.rs:90,144`).
//!
//! Differences left under D51:
//! - polars treats a field written in quotes as a string whatever it holds, and
//!   the `csv` crate removes the quotes before this sees the field;
//! - a later cell that does not parse as its column's type is an error in both
//!   engines, but the text is this module's own;
//! - dates stay strings, as they do in the reference, which never asks polars
//!   to parse them.

use std::collections::BTreeSet;
use std::path::Path;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LAMBDA};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `csv` — make the conditional (`conditional_csv.rs:9-29`).
fn csv_word(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for CSV".into()));
    }
    let v = vm
        .pull()
        .ok_or_else(|| Error("CONTEXT: No CSV filename discovered on the stack".into()))?;
    let name = v.as_str().ok_or_else(|| {
        Error("CONTEXT: Error name casting: This Dynamic type is not string".into())
    })?;
    if !Path::new(&name).is_file() {
        return Err(Error(format!("CSV: csv file not found: {name}")));
    }
    vm.push(crate::conditional::new_conditional("csv").set("name", BundValue::str(name)));
    Ok(())
}

/// A column's type, in polars' test order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    Bool,
    Float,
    Int,
    Str,
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// polars' `BOOLEAN_RE`: `^(true|false)$`, case-insensitive.
fn is_bool(s: &str) -> bool {
    s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("false")
}

/// polars' `FLOAT_RE`:
/// `^[-+]?((\d*\.\d+)([eE][-+]?\d+)?|inf|NaN|(\d+)[eE][-+]?\d+|\d+\.)$`.
fn is_float(s: &str) -> bool {
    let t = s.strip_prefix(['-', '+']).unwrap_or(s);
    if t == "inf" || t == "NaN" {
        return true;
    }
    if t.strip_suffix('.').is_some_and(digits) {
        return true;
    }
    let (mantissa, exponent) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], Some(&t[i + 1..])),
        None => (t, None),
    };
    if exponent.is_some_and(|e| !digits(e.strip_prefix(['-', '+']).unwrap_or(e))) {
        return false;
    }
    match mantissa.split_once('.') {
        Some((int_part, frac)) => {
            (int_part.is_empty() || digits(int_part)) && digits(frac)
        }
        None => exponent.is_some() && digits(mantissa),
    }
}

/// polars' `INTEGER_RE`: `^-?(\d+)$`.
fn is_int(s: &str) -> bool {
    digits(s.strip_prefix('-').unwrap_or(s))
}

fn infer(s: &str) -> Kind {
    if is_bool(s) {
        Kind::Bool
    } else if is_float(s) {
        Kind::Float
    } else if is_int(s) {
        Kind::Int
    } else {
        Kind::Str
    }
}

/// polars' `finish_infer_field_schema`: one kind stays, integers with floats
/// are floats, anything else, including no evidence at all, is a string.
fn column_kind(seen: &BTreeSet<Kind>) -> Kind {
    match (seen.len(), seen.iter().next()) {
        (1, Some(k)) => *k,
        (2, _) if seen.contains(&Kind::Int) && seen.contains(&Kind::Float) => Kind::Float,
        _ => Kind::Str,
    }
}

fn finishing(e: impl std::fmt::Display) -> Error {
    Error(format!("CONTEXT.RUN: Error finishing dataframe: {e}"))
}

/// One cell as its column's type; an empty field is null.
fn cell(s: &str, kind: Kind, column: &str) -> Result<Option<BundValue>, Error> {
    if s.is_empty() {
        return Ok(None);
    }
    let bad = |dtype: &str| finishing(format!("could not parse `{s}` as dtype `{dtype}` at column '{column}'"));
    Ok(Some(match kind {
        Kind::Str => BundValue::str(s),
        Kind::Int => BundValue::int(s.parse::<i64>().map_err(|_| bad("i64"))?),
        Kind::Float => BundValue::float(s.parse::<f64>().map_err(|_| bad("f64"))?),
        Kind::Bool if is_bool(s) => BundValue::boolean(s.eq_ignore_ascii_case("true")),
        Kind::Bool => return Err(bad("bool")),
    }))
}

/// The file as named columns of typed cells, as the DataFrame would hold it.
struct Frame {
    names: Vec<String>,
    rows: Vec<Vec<Option<BundValue>>>,
}

fn read_frame(path: &str, has_header: bool) -> Result<Frame, Error> {
    let creating = |e: csv::Error| Error(format!("CONTEXT.RUN: Error creating dataframe: {e}"));
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(has_header)
        .flexible(true)
        .from_path(path)
        .map_err(creating)?;
    let header: Vec<String> = if has_header {
        rdr.headers().map_err(creating)?.iter().map(str::to_string).collect()
    } else {
        Vec::new()
    };
    let mut raw: Vec<Vec<String>> = Vec::new();
    for rec in rdr.records() {
        raw.push(rec.map_err(finishing)?.iter().map(str::to_string).collect());
    }
    // Without a header, polars names the columns `column_1`, `column_2`, …
    // from the first row's width.
    let names: Vec<String> = if has_header {
        header
    } else {
        let width = raw.first().map_or(0, Vec::len);
        (1..=width).map(|i| format!("column_{i}")).collect()
    };
    let kinds: Vec<Kind> = (0..names.len())
        .map(|c| {
            let seen: BTreeSet<Kind> = raw
                .iter()
                .take(100)
                .filter_map(|r| r.get(c))
                .filter(|f| !f.is_empty())
                .map(|f| infer(f))
                .collect();
            column_kind(&seen)
        })
        .collect();
    let mut rows = Vec::with_capacity(raw.len());
    for r in &raw {
        let mut row = Vec::with_capacity(names.len());
        for (c, (kind, name)) in kinds.iter().zip(&names).enumerate() {
            row.push(cell(r.get(c).map_or("", String::as_str), *kind, name)?);
        }
        rows.push(row);
    }
    Ok(Frame { names, rows })
}

/// Run a `csv` conditional (`conditional_csv.rs:31-173`).
///
/// Without `column`, each row goes to the stack as a list and the lambda runs
/// once per row. With `column`, that column's cells go to the stack as one list
/// and the lambda runs once. `is_header` defaults to true. A missing `lambda`
/// is an empty one, and a `lambda` that is not a LAMBDA fails when it would
/// run, as `lambda_eval` refuses it.
fn run_csv(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    let name_v = c.get("name").ok_or_else(|| {
        Error("CONTEXT.RUN: getting CSV file name returns error: Key not found: name".into())
    })?;
    let name = name_v.as_str().ok_or_else(|| {
        Error(
            "CONTEXT.RUN: casting CSV file name returns error: This Dynamic type is not string"
                .into(),
        )
    })?;
    let column = c.get("column");
    let has_header = match c.get("is_header") {
        None => true,
        Some(v) => match *v.unboxed() {
            BundValue::Bool(b, _) => b,
            _ => {
                return Err(Error(
                    "CONTEXT.RUN: casting CSV is_header returns error: This Dynamic type is not bool"
                        .into(),
                ))
            }
        },
    };
    if !Path::new(&name).is_file() {
        return Err(Error(format!("CSV: csv file not found: {name}")));
    }
    let lambda = c
        .get("lambda")
        .unwrap_or_else(|| BundValue::lambda(Vec::new()));
    let run = |vm: &mut dyn Vm| -> Result<(), Error> {
        if lambda.dt() != LAMBDA {
            return Err(Error("CSV processing lambda returns: This is not a lambda".into()));
        }
        vm.eval_lambda(&lambda)
            .map_err(|e| Error(format!("CSV processing lambda returns: {}", e.0)))
    };
    let frame = read_frame(&name, has_header)?;
    match column {
        Some(col_v) => {
            let col = col_v.as_str().ok_or_else(|| {
                Error("CONTEXT.RUN error casting column name: This Dynamic type is not string".into())
            })?;
            let Some(ix) = frame.names.iter().position(|n| *n == col) else {
                return Err(Error(format!(
                    "CONTEXT.RUN: Error selecting column {col}: not found: {col}"
                )));
            };
            let cells: Vec<BundValue> = frame
                .rows
                .iter()
                .filter_map(|r| r.get(ix).cloned().flatten())
                .collect();
            vm.push(BundValue::list(cells));
            run(vm)?;
        }
        None => {
            for row in frame.rows {
                vm.push(BundValue::list(row.into_iter().flatten().collect()));
                run(vm)?;
            }
        }
    }
    Ok(())
}

pub fn register(r: &mut Registry) {
    // `reference/Bund/src/stdlib/functions/conditional/mod.rs:26,42`.
    r.register_native("csv", csv_word, eff(1, 1), WordKind::Sync);
    r.register_conditional("csv", run_csv);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_classified_as_polars_classifies_them() {
        assert_eq!(infer("TRUE"), Kind::Bool);
        for f in ["1.5", ".5", "5.", "-2.5e3", "1e5", "inf", "NaN", "+3.0"] {
            assert_eq!(infer(f), Kind::Float, "{f}");
        }
        for f in ["12", "-7"] {
            assert_eq!(infer(f), Kind::Int, "{f}");
        }
        for f in ["+7", "5.e3", "abc", "1,5", "e5"] {
            assert_eq!(infer(f), Kind::Str, "{f}");
        }
    }

    #[test]
    fn columns_settle_as_polars_settles_them() {
        let k = |v: &[Kind]| column_kind(&v.iter().copied().collect());
        assert_eq!(k(&[Kind::Int]), Kind::Int);
        assert_eq!(k(&[Kind::Int, Kind::Float]), Kind::Float);
        assert_eq!(k(&[Kind::Int, Kind::Bool]), Kind::Str);
        assert_eq!(k(&[]), Kind::Str);
    }
}
