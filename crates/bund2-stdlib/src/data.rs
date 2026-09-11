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

/// `sqlite` — make the conditional
/// (`reference/Bund/src/stdlib/functions/conditional/conditional_sqlite.rs:48-68`).
fn sqlite_word(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for sqlite".into()));
    }
    let v = vm
        .pull()
        .ok_or_else(|| Error("CONTEXT: No data file name discovered on the stack".into()))?;
    let name = v.as_str().ok_or_else(|| {
        Error("CONTEXT: Error name casting: This Dynamic type is not string".into())
    })?;
    if !Path::new(&name).is_file() {
        return Err(Error(format!("SQLITE: sqlite file not found: {name}")));
    }
    vm.push(crate::conditional::new_conditional("sqlite").set("name", BundValue::str(name)));
    Ok(())
}

/// Compile PRQL to SQLite SQL, as the reference does, through the same
/// `prqlc` with the same options (`conditional_sqlite.rs:11-34`). `color` is
/// deprecated in `prqlc`, but the reference sets it, and so does this.
#[allow(deprecated)]
fn prql_to_sql(query: &str) -> Result<String, Error> {
    let opts = prqlc::Options {
        format: true,
        signature_comment: false,
        color: false,
        display: prqlc::DisplayOptions::Plain,
        target: prqlc::Target::Sql(Some(prqlc::sql::Dialect::SQLite)),
    };
    let pl = prqlc::prql_to_pl(query)
        .map_err(|e| Error(format!("PRQL.COMPILE(PL) returns: {e}")))?;
    let rq = prqlc::pl_to_rq(pl).map_err(|e| Error(format!("PRQL.COMPILE(RQ) returns: {e}")))?;
    prqlc::rq_to_sql(rq, &opts).map_err(|e| Error(format!("PRQL.COMPILE(SQL) returns: {e}")))
}

/// One result cell as a Bund value (`conditional_sqlite.rs:152-172`).
///
/// A BLOB is where the engines part (F109). The reference decodes the bytes
/// with bincode as a serialised Bund value
/// (`reference/rust_dynamic/src/bincode.rs:51-77`). Bund2 has the reference's
/// wire types (`bund2_value::wire`), but nothing yet converts a wire value into
/// a `BundValue`. So a non-NULL BLOB is refused with a reason, rather than
/// decoded into something the reference would not produce.
fn sql_cell(v: &graphitesql::Value) -> Result<BundValue, Error> {
    use graphitesql::Value as V;
    Ok(match v {
        V::Null => BundValue::nodata(),
        V::Integer(i) => BundValue::int(*i),
        V::Real(f) => BundValue::float(*f),
        V::Text(t) => BundValue::str(std::str::from_utf8(t.as_bytes()).map_err(|e| {
            Error(format!("CONTEXT.RUN error converting string data: {e}"))
        })?),
        V::Blob(_) => {
            return Err(Error(
                "CONTEXT.RUN: a BLOB column holds a serialised Bund value in the reference, which Bund2 cannot decode"
                    .into(),
            ))
        }
    })
}

/// An engine error as rusqlite would show it: SQLite's own message, bare.
///
/// graphitesql's `Display` adds a prefix to the message kinds (`error: …`,
/// `SQL error: …`), and rusqlite shows `sqlite3_errmsg` alone, which
/// graphitesql's messages already copy word for word. So the message kinds
/// print their text, and the others keep graphitesql's display.
fn sql_err(e: &graphitesql::Error) -> String {
    use graphitesql::Error as E;
    match e {
        E::Error(m) | E::ErrorAt(m, _) | E::Parse(m) | E::ParseAt(m, _) | E::Constraint(m) => {
            m.clone()
        }
        other => other.to_string(),
    }
}

/// Run a `sqlite` conditional (`conditional_sqlite.rs:70-199`).
///
/// The query is `sql`, or else `prql` compiled to SQL. Each result row goes to
/// the stack as a MAP from column name to value, and the lambda runs once per
/// row. The first `skip_first` rows are skipped; the count is cast `as usize`
/// (`:142`), so a negative one skips every row. A NULL is NODATA.
///
/// **D51: graphitesql, not rusqlite.** The reference prepares the statement
/// and then steps it, with a separate message for each stage (`:124-196`).
/// graphitesql compiles and runs in one call, so any failure is reported with
/// the compile stage's message. The engine's own text follows, and graphitesql
/// writes it as `sqlite3` does, which is what rusqlite shows too.
fn run_sqlite(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    let skip_first = match c.get("skip_first") {
        None => 0,
        Some(v) => v.as_int().ok_or_else(|| {
            Error(
                "CONTEXT.RUN: casting SQLITE skip_first returns error: This Dynamic type is not integer"
                    .into(),
            )
        })?,
    };
    let name_v = c.get("name").ok_or_else(|| {
        Error("CONTEXT.RUN: getting SQLITE file name returns error: Key not found: name".into())
    })?;
    let name = name_v.as_str().ok_or_else(|| {
        Error(
            "CONTEXT.RUN: casting SQLITE file name returns error: This Dynamic type is not string"
                .into(),
        )
    })?;
    let sql_v = match c.get("sql") {
        Some(v) => v,
        None => match c.get("prql") {
            Some(p) => {
                let prql = p.as_str().ok_or_else(|| {
                    Error("CONTEXT.RUN: PRQL casting returns: This Dynamic type is not string".into())
                })?;
                BundValue::str(prql_to_sql(&prql)?)
            }
            None => {
                return Err(Error(
                    "CONTEXT.RUN: can not get ether SQL or PRQL query: Key not found: prql".into(),
                ))
            }
        },
    };
    let sql = sql_v.as_str().ok_or_else(|| {
        Error("CONTEXT.RUN error casting SQL query: This Dynamic type is not string".into())
    })?;
    if !Path::new(&name).is_file() {
        return Err(Error(format!("SQLITE: sqlite file not found: {name}")));
    }
    let lambda = c
        .get("lambda")
        .unwrap_or_else(|| BundValue::lambda(Vec::new()));
    // **Read-only, where the reference opens read-write** (`:36-46`). The
    // read-write open leaves empty `-journal` and `-wal` files beside the
    // database, which rusqlite does not do for a query. `query` runs only a
    // SELECT, so nothing a program can ask for here needs write access.
    let conn = graphitesql::Connection::open_readonly(&name).map_err(|e| {
        Error(format!(
            "CONTEXT.RUN error creating SQLITE connection: Open world operation returns: {}",
            sql_err(&e)
        ))
    })?;
    let result = conn
        .query(&sql)
        .map_err(|e| Error(format!("CONTEXT.RUN error compile SQL statement: {}", sql_err(&e))))?;
    for row in result.rows.iter().skip(skip_first as usize) {
        let mut out = BundValue::map(Default::default());
        for (col, v) in result.columns.iter().zip(row) {
            out = out.set(col, sql_cell(v)?);
        }
        vm.push(out);
        if lambda.dt() != LAMBDA {
            return Err(Error(
                "SQLITE3 processing lambda returns: This is not a lambda".into(),
            ));
        }
        vm.eval_lambda(&lambda)
            .map_err(|e| Error(format!("SQLITE3 processing lambda returns: {}", e.0)))?;
    }
    Ok(())
}

pub fn register(r: &mut Registry) {
    // `reference/Bund/src/stdlib/functions/conditional/mod.rs:26-27,42-43`.
    r.register_native("csv", csv_word, eff(1, 1), WordKind::Sync);
    r.register_conditional("csv", run_csv);
    r.register_native("sqlite", sqlite_word, eff(1, 1), WordKind::Sync);
    r.register_conditional("sqlite", run_sqlite);
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
