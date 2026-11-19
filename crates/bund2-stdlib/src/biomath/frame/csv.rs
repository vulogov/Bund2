//! RFC-0017 §S6: `frame.csv.parse` and `frame.csv.text`.
//!
//! **The reader is the frame words' own**, not `csv-core`, which reports no
//! quote flag and skips empty lines; both matter here (D254, D258 B1).
//!
//! - A field is **quoted** when its first byte is `"`; `""` inside is a
//!   quote; it may hold the separator and line breaks, kept as written. It
//!   ends at a `"` followed by the separator, a line terminator or the end.
//!   Any other byte after the closing quote, a `"` in an unquoted field, and
//!   an unterminated quote are E101 naming the row.
//! - **Lines.** CR, LF and CRLF each end a line, and every line outside a
//!   quoted field ends a record; only the text's final terminator ends the
//!   text, so `"\n"` is one record of one empty field.
//! - A leading **byte-order mark** is removed first; text above 10⁸ bytes
//!   is E111; empty text is a frame of no columns and no rows.
//! - **Widths.** The header (or, without one, the first row) sets the
//!   width; a row of another width is E102.
//!
//! **Fields.** A quoted field is a STRING. An unquoted field equal to an
//! `na` string is NONE (q 0.0). Every other unquoted field is inferred as
//! `csv` infers it (`crate::data`, `infer` and `column_kind`), over every
//! row; in a FLOAT column an integer-pattern field is an INTEGER; a
//! non-finite FLOAT or an integer beyond i64 is E101. The answer passes the
//! frame check like any frame, naming `text`.

use std::collections::BTreeSet;

use bund2_api::{Error, Vm};
use bund2_value::{BOOL, BundValue, FLOAT, INTEGER, NONE, STRING};

use super::check::{self, Frame, Who};
use super::opts;
use super::render;
use super::reshape::{factorable, make_factor};
use crate::biomath::na::none;
use crate::biomath::operand::{WIDTH, domain, operands};
use crate::biomath::refuse::Refusal;
use crate::data::{Kind, column_kind, infer, is_int};

/// The most bytes `frame.csv.parse` reads (§S6).
const TEXT_CAP: u64 = 100_000_000;

const W: &str = "frame.csv.parse";

/// One field: its text, and whether it was quoted.
type Field = (String, bool);

/// Where a fault is: `row <n>` or the header.
#[derive(Clone, Copy)]
enum At {
    Header,
    Row(usize),
}

/// E101 on `text`.
fn bad(text: String) -> Error {
    domain(W, "text", text)
}

/// The text's records, each a LIST of fields.
fn records(text: &str, sep: u8, header: bool) -> Result<Vec<Vec<Field>>, Error> {
    let b = text.as_bytes();
    let n = b.len();
    let mut out: Vec<Vec<Field>> = Vec::new();
    let mut pos = 0;
    // Record k's place: the header, or its 1-based data row.
    let at = |k: usize| match (header, k) {
        (true, 0) => At::Header,
        (true, k) => At::Row(k),
        (false, k) => At::Row(k + 1),
    };
    let text_of = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    while pos < n {
        let place = at(out.len());
        let mut fields: Vec<Field> = Vec::new();
        loop {
            if b.get(pos) == Some(&b'"') {
                // A quoted field.
                pos += 1;
                let mut buf = Vec::new();
                loop {
                    match b.get(pos) {
                        None => {
                            return Err(bad(match place {
                                At::Header => "a quote opened in the header is not closed".into(),
                                At::Row(r) => format!("a quote opened at row {r} is not closed"),
                            }));
                        }
                        Some(b'"') if b.get(pos + 1) == Some(&b'"') => {
                            buf.push(b'"');
                            pos += 2;
                        }
                        Some(b'"') => {
                            pos += 1;
                            break;
                        }
                        Some(&c) => {
                            buf.push(c);
                            pos += 1;
                        }
                    }
                }
                fields.push((text_of(&buf), true));
                match b.get(pos) {
                    None => break,
                    Some(&c) if c == sep => {
                        pos += 1;
                        continue;
                    }
                    Some(b'\r' | b'\n') => {}
                    Some(_) => {
                        let ch: String = text
                            .get(pos..)
                            .and_then(|t| t.chars().next())
                            .map(String::from)
                            .unwrap_or_default();
                        let shown = render::text(&ch);
                        return Err(bad(match place {
                            At::Header => format!("the header has {shown} after a closing quote"),
                            At::Row(r) => format!("row {r} has {shown} after a closing quote"),
                        }));
                    }
                }
            } else {
                // An unquoted field: to the separator, a terminator or the end.
                let start = pos;
                while let Some(&c) = b.get(pos) {
                    if c == sep || c == b'\r' || c == b'\n' {
                        break;
                    }
                    if c == b'"' {
                        return Err(bad(match place {
                            At::Header => "the header has a quote inside an unquoted field".into(),
                            At::Row(r) => format!("row {r} has a quote inside an unquoted field"),
                        }));
                    }
                    pos += 1;
                }
                fields.push((text_of(b.get(start..pos).unwrap_or_default()), false));
                match b.get(pos) {
                    Some(&c) if c == sep => {
                        pos += 1;
                        continue;
                    }
                    _ => {}
                }
            }
            // A terminator, or the end: the record ends.
            match b.get(pos) {
                Some(b'\r') if b.get(pos + 1) == Some(&b'\n') => pos += 2,
                Some(b'\r' | b'\n') => pos += 1,
                _ => {}
            }
            break;
        }
        out.push(fields);
    }
    Ok(out)
}

/// One column's cells from its fields: §S6's "Fields".
fn column(name: &str, fields: &[&Field], na: &[String]) -> Result<Vec<BundValue>, Error> {
    let is_na = |(s, quoted): &Field| !quoted && na.contains(s);
    let seen: BTreeSet<Kind> = fields
        .iter()
        .filter(|f| !f.1 && !is_na(f))
        .map(|(s, _)| infer(s))
        .collect();
    let kind = column_kind(&seen);
    let field_at = |s: &str, r: usize| {
        format!(
            "field {} at row {}, column {}",
            render::text(s),
            r + 1,
            render::text(name)
        )
    };
    let whole = |s: &str, r: usize| {
        s.parse::<i64>()
            .map(BundValue::int)
            .map_err(|_| bad(format!("{} does not fit an INTEGER", field_at(s, r))))
    };
    fields
        .iter()
        .enumerate()
        .map(|(r, f)| {
            let (s, quoted) = (&f.0, f.1);
            if quoted {
                return Ok(BundValue::str(s.clone()));
            }
            if is_na(f) {
                return Ok(none());
            }
            match kind {
                Kind::Str => Ok(BundValue::str(s.clone())),
                Kind::Bool => Ok(BundValue::boolean(s.eq_ignore_ascii_case("true"))),
                Kind::Int => whole(s, r),
                Kind::Float if is_int(s) => whole(s, r),
                Kind::Float => match s.parse::<f64>() {
                    Ok(x) if x.is_finite() => Ok(BundValue::float(x)),
                    Ok(_) => Err(bad(format!(
                        "{} is not finite; list it in the na option to read it as missing",
                        field_at(s, r)
                    ))),
                    Err(_) => Err(Error::internal(format!(
                        "{W}: the field {s:?} matched the float pattern and does not parse"
                    ))),
                },
            }
        })
        .collect()
}

/// `frame.csv.parse` (`text opts -- frame`).
fn frame_csv_parse(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = operands(vm, W, 2)?;
    let text = match v[0].as_str() {
        Some(s) if v[0].dt() == STRING => s,
        _ => return Err(bad(v[0].summary(WIDTH))),
    };
    let o = opts::read(W, &v[1], &["factors", "header", "na", "sep"])?;
    let header = o.header.unwrap_or(true);
    let na = o.na.unwrap_or_else(|| vec![String::new(), "NA".into()]);
    let sep = o.sep.unwrap_or(b',');
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let size = u64::try_from(text.len()).unwrap_or(u64::MAX);
    if size > TEXT_CAP {
        return Err(Refusal::Cap {
            what: "text".into(),
            cap: TEXT_CAP,
        }
        .error(W));
    }
    let mut recs = records(text, sep, header)?.into_iter();
    let first = recs.next();
    let width = first.as_ref().map_or(0, Vec::len);
    let (names, data): (Vec<String>, Vec<Vec<Field>>) = match (header, first) {
        (_, None) => (Vec::new(), Vec::new()),
        (true, Some(h)) => {
            let mut names: Vec<String> = Vec::with_capacity(h.len());
            for (i, (s, _)) in h.into_iter().enumerate() {
                if s.is_empty() {
                    return Err(bad(format!("header name {} is empty", i + 1)));
                }
                if names.contains(&s) {
                    return Err(bad(format!("a repeated header name {}", render::text(&s))));
                }
                names.push(s);
            }
            (names, recs.collect())
        }
        (false, Some(r)) => {
            let names = (1..=width).map(|i| format!("column_{i}")).collect();
            (names, std::iter::once(r).chain(recs).collect())
        }
    };
    for (k, r) in data.iter().enumerate() {
        if r.len() != width {
            let row = k + 1;
            let against = if header { "the header's" } else { "row 1's" };
            return Err(Refusal::Disagree {
                what: format!("row {row}'s fields and {against}"),
                n: r.len(),
                m: width,
            }
            .error(W));
        }
    }
    // Pass two of the options: `factors` against the columns.
    let factors = o.factors.unwrap_or_default();
    if let Some(z) = factors.iter().find(|z| !names.contains(z)) {
        return Err(domain(W, "opts", format!("no column {}", render::text(z))));
    }
    let mut cols = Vec::with_capacity(width);
    for (c, name) in names.iter().enumerate() {
        let fields: Vec<&Field> = data.iter().filter_map(|r| r.get(c)).collect();
        cols.push(BundValue::list(column(name, &fields, &na)?));
    }
    let refs: Vec<&BundValue> = cols.iter().collect();
    let mut f = check::parts(W, Who::frame("text"), names, &refs, None)?;
    for z in &factors {
        let Some(i) = f.index(z) else { continue };
        let kind = factorable(W, "opts", &f, i)?;
        make_factor(&mut f, i, kind);
    }
    vm.push(f.to_value());
    Ok(())
}

/// A STRING quoted for CSV: `"` doubled, the whole in quotes.
fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// One cell as `frame.csv.text` writes it.
fn written(v: &BundValue) -> String {
    match v.dt() {
        NONE => String::new(),
        STRING => quoted(&v.as_str().unwrap_or_default()),
        INTEGER | FLOAT | BOOL => v.display(),
        _ => v.summary(WIDTH),
    }
}

/// The CSV text of a checked frame: a header, every STRING quoted, NONE an
/// empty field, each line ending `\n`; a frame of no columns the empty
/// STRING.
pub(super) fn csv_text(f: &Frame) -> String {
    if f.names.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let header: Vec<String> = f.names.iter().map(|n| quoted(n)).collect();
    out.push_str(&header.join(","));
    out.push('\n');
    for r in 0..f.nrow {
        let row: Vec<String> = f
            .cols
            .iter()
            .map(|c| c.get(r).map(written).unwrap_or_default())
            .collect();
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

/// `frame.csv.text` (`frame -- text`).
fn frame_csv_text(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.csv.text";
    let v = operands(vm, W, 1)?;
    let f = check::check(W, "frame", &v[0])?;
    vm.push(BundValue::str(csv_text(&f)));
    Ok(())
}

/// `frame.text` (`frame -- text`): the table of [`super::table`].
fn frame_text(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "frame.text";
    let v = operands(vm, W, 1)?;
    let f = check::check(W, "frame", &v[0])?;
    vm.push(BundValue::str(super::table::table(&f)));
    Ok(())
}

/// Register M3's words, each plain and `.` (RFC-0015 §S3).
pub fn register(r: &mut bund2_api::Registry) {
    use bund2_api::{StackEffect, WordKind};
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("frame.csv.parse", frame_csv_parse, 2);
    word!("frame.csv.text", frame_csv_text, 1);
    word!("frame.text", frame_text, 1);
}

/// M3's words with their forms.
pub const WORDS: [&str; 6] = [
    "frame.csv.parse",
    "frame.csv.parse.",
    "frame.csv.text",
    "frame.csv.text.",
    "frame.text",
    "frame.text.",
];

#[cfg(test)]
mod tests;
