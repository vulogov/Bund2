//! `convert.to_*` and `not` — the conversion table, and its one consumer
//! outside the family.
//!
//! Every `convert.to_X` is one call to `stdlib_convert_base` with a target tag
//! (`reference/rust_multistackvm/src/stdlib/convert/internal.rs:6-41`): guard
//! the depth, pull, `value.conv(target)`, push. The whole family is that
//! function plus sixteen registrations (`:107-125`), so the interesting part is
//! `conv` itself, in `rust_dynamic`.
//!
//! **`conv` dispatches on the payload arm first and the tag second**
//! (`reference/rust_dynamic/src/conv.rs:694-705`), and each per-source function
//! re-checks the tag and refuses if it does not match — a `Val::String` whose
//! `dt` is neither `STRING` nor `TEXTBUFFER` is rejected by
//! `value_string_conversion` (`:187-192`). That is the tag/payload split from
//! the conversion side, and it means a converted `PTR` takes a different path
//! from a converted `STRING` even though both hold a Rust `String`.
//!
//! Only the arms Bund2 can construct are implemented. The rest report the
//! reference's own `Can not convert …` text rather than being silently
//! approximated, so a program meeting an unimplemented conversion is told so.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{
    BOOL, BundValue, CALL, CLASS, CONDITIONAL, FLOAT, INTEGER, JSON, LAMBDA, LIST, MAP, MATRIX,
    NODATA, NONE, OBJECT, PTR, STRING, TEXTBUFFER, VALUEMAP,
};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `Value::conv`, for the tags Bund2 constructs
/// (`reference/rust_dynamic/src/conv.rs:106-265`).
///
/// The error text is the reference's, because a caught error's message is
/// observable: `stdlib_convert_base` interpolates it into
/// `CONVERT.TO_STRING returned error: {}` (`internal.rs:33`), and a program can
/// print that through `?try`.
pub(crate) fn conv_value(v: &BundValue, target: u16) -> Result<BundValue, Error> {
    // **Every source converts to STRING through `display`, which *is* this
    // table's STRING column.** Keeping a second copy here is what broke
    // `"dup" ptr convert.to_string`: `display` had been taught that a PTR
    // renders `` `(name) `` (`conv.rs:100`) and this had not, so the same
    // conversion answered two different things depending on which word asked.
    //
    // That is the third time a duplicated renderer diverged — `format` carried
    // one, `display` lacked the MAP arm, and this. One function, one answer.
    if target == STRING {
        return Ok(BundValue::str(v.display()));
    }
    // **TEXTBUFFER is the STRING column with a different tag — over a smaller
    // set of sources.** Every TEXTBUFFER arm in the reference computes exactly
    // the text its STRING neighbour computes and then calls `Value::text_buffer`
    // instead of `Value::from_string`: compare `:127-134` (float, both via
    // `dtoa`), `:167-174` (integer, both via `itoa`), `:212-217` (string) and
    // `:255-260` (bool). The container arms do not even duplicate the body —
    // `value_list_conversion` matches `STRING | TEXTBUFFER` together and picks
    // the constructor at the end (`:340,355-359`).
    //
    // So the text is `display()`, as it is for STRING. What is *not* shared is
    // the domain: `value_none_conversion` (`:55-72`), `value_nodata_conversion`
    // (`:36-53`), `value_call_conversion` (`:74-88`) and `value_ptr_conversion`
    // (`:90-104`) each offer STRING and no TEXTBUFFER, so those four sources
    // must fail here even though `display()` would happily render them. Falling
    // through to the per-source table below is what makes them fail, because no
    // arm there names TEXTBUFFER either.
    if target == TEXTBUFFER && admits_textbuffer(v.dt()) {
        return Ok(BundValue::textbuffer(v.display()));
    }
    // Look through boxing: `push` writes a `stack` tag onto every scalar, so a
    // pulled `1` is a `Heap { payload: Scalar(Int) }` and matching the outer
    // value alone takes the wrong arm.
    let u = v.unboxed();
    let dt = v.dt();
    match u {
        BundValue::Int(i, _) if dt == INTEGER => match target {
            // `itoa` and `{}` agree on i64: shortest decimal, no separators.
            STRING => Ok(BundValue::str(i.to_string())),
            INTEGER => Ok(BundValue::int(*i)),
            FLOAT => Ok(BundValue::float(*i as f64)),
            BOOL => Ok(BundValue::boolean(*i != 0)),
            LIST => Ok(BundValue::list(vec![BundValue::int(*i)])),
            _ => Err(Error(format!("Can not convert integer to {target}"))),
        },
        BundValue::Float(f, _) if dt == FLOAT => match target {
            // **`dtoa`, via `display`, not Rust's `{:?}`.** The two disagree
            // on which shortest form to print — `2.0 math.sqrt` renders
            // `1.4142135623730952` in the reference and `…951` under `{:?}`,
            // from identical bits (`conv.rs:128-130`). An earlier version of
            // this arm asserted they agreed; they agree on `1.0` and `3.14`,
            // which is why it survived until a `sqrt` golden appeared.
            STRING => Ok(BundValue::str(BundValue::float(*f).display())),
            INTEGER => Ok(BundValue::int(*f as i64)),
            FLOAT => Ok(BundValue::float(*f)),
            BOOL => Ok(BundValue::boolean(*f != 0.0)),
            LIST => Ok(BundValue::list(vec![BundValue::float(*f)])),
            _ => Err(Error(format!("Can not convert float to {target}"))),
        },
        BundValue::Bool(b, _) if dt == BOOL => match target {
            STRING => Ok(BundValue::str(b.to_string())),
            INTEGER => Ok(BundValue::int(i64::from(*b))),
            FLOAT => Ok(BundValue::float(if *b { 1.0 } else { 0.0 })),
            BOOL => Ok(BundValue::boolean(*b)),
            LIST => Ok(BundValue::list(vec![BundValue::boolean(*b)])),
            _ => Err(Error(format!("Can not convert bool to {target}"))),
        },
        // `value_string_conversion` admits STRING *and* TEXTBUFFER as sources
        // (`conv.rs:187`), which is why both reach this arm. They are one arm in
        // the reference too, not two that happen to agree.
        _ if dt == STRING || dt == TEXTBUFFER => {
            let Some(s) = u.as_str() else {
                return Err(Error(format!("Can not convert string to {target}")));
            };
            match target {
                STRING => Ok(BundValue::str(s)),
                BOOL => Ok(BundValue::boolean(string_to_bool(&s))),
                INTEGER => s
                    .trim()
                    .parse::<i64>()
                    .map(BundValue::int)
                    .map_err(|e| Error(format!("Can not convert string to integer {e:?}"))),
                FLOAT => s
                    .trim()
                    .parse::<f64>()
                    .map(BundValue::float)
                    .map_err(|e| Error(format!("Can not convert string to float {e:?}"))),
                LIST => Ok(BundValue::list(vec![BundValue::str(s)])),
                _ => Err(Error(format!("Can not convert string to {target}"))),
            }
        }
        // **A LIST converts to itself, to its length, and to a truth value**
        // (`value_list_conversion`, `conv.rs:293-341`, reached from `conv`'s
        // container arm at `:707-708`). There was no arm here at all, so every
        // conversion *from* a list fell through to the final refusal. `push`
        // converts both operands with `conv(LIST)`, and `[ 1 2 ] 3 push`
        // failed where the oracle answers `[3, [1, 2]]`. STRING and TEXTBUFFER
        // never reach this arm: they short-circuit through `display` above.
        // **A MATRIX is its own payload** (D57 as amended): rows are bare
        // vectors, because that is what the oracle renders and a golden
        // captures. It converts only to a LIST of its rows
        // (`value_matrix_conversion`, `reference/rust_dynamic/src/conv.rs:268-290`),
        // which is the whole of its return path.
        _ if dt == MATRIX => {
            let rows = v
                .as_matrix()
                .ok_or_else(|| Error::internal("a MATRIX value carried no rows"))?;
            match target {
                LIST => Ok(BundValue::list(
                    rows.iter().map(|r| BundValue::list(r.clone())).collect(),
                )),
                // **A MATRIX target is refused, including MATRIX to MATRIX.**
                // `value_matrix_conversion` accepts LIST and nothing else, so
                // a second conversion falls to its `_` arm — which says
                // "list", because the wording is shared with the list
                // converter (`conv.rs:288`). Confirmed on the oracle:
                // `[ [ 1 2 ] ] matrix matrix` answers
                // `CONVERT.TO_MATRIX returned error: Can not convert list to 26`.
                _ => Err(Error(format!("Can not convert list to {target}"))),
            }
        }
        _ if dt == LIST => {
            let items = v
                .as_list()
                .ok_or_else(|| Error::internal("a LIST value carried no items"))?;
            match target {
                // A MATRIX converts to a LIST of its rows, each a LIST
                // (`value_matrix_conversion`, `conv.rs:268-290`); a LIST
                // converts to itself.
                LIST => Ok(BundValue::list(items.to_vec())),
                INTEGER => Ok(BundValue::int(items.len() as i64)),
                FLOAT => Ok(BundValue::float(items.len() as f64)),
                BOOL => Ok(BundValue::boolean(!items.is_empty())),
                // Keyed by position as a string, `"0"`, `"1"`, … (`:334-342`).
                MAP => Ok(BundValue::map(
                    items
                        .iter()
                        .enumerate()
                        .map(|(i, x)| (i.to_string(), x.clone()))
                        .collect::<std::collections::BTreeMap<_, _>>(),
                )),
                // **Every row is converted through `conv(LIST)` and cast**,
                // and the two error texts are the reference's, because a
                // `?try` can print them (`conv.rs:361-378`). The rows land in
                // `Payload::Matrix` as bare vectors, so a row keeps no header
                // of its own — which is what the oracle renders.
                MATRIX => {
                    let mut rows: Vec<Vec<BundValue>> = Vec::new();
                    for r in items {
                        let row = conv_value(r, LIST).map_err(|e| {
                            Error(format!("Error converting row into matrix {}", e.0))
                        })?;
                        let cells = row.as_list().ok_or_else(|| {
                            Error(
                                "Error casting row into matrix This Dynamic type is not list"
                                    .to_string(),
                            )
                        })?;
                        rows.push(cells.to_vec());
                    }
                    Ok(BundValue::matrix(rows))
                }
                _ => Err(Error(format!("Can not convert Value from {dt}"))),
            }
        }
        // NODATA and NONE convert to their own names and to a one-element list,
        // and to nothing else (`conv.rs:36-72`).
        _ if dt == NODATA => match target {
            STRING => Ok(BundValue::str("NODATA")),
            LIST => Ok(BundValue::list(vec![BundValue::nodata()])),
            _ => Err(Error(format!("Can not convert NODATA to {target}"))),
        },
        _ if dt == NONE => match target {
            STRING => Ok(BundValue::str("NONE")),
            LIST => Ok(BundValue::list(vec![BundValue::none()])),
            _ => Err(Error(format!("Can not convert NONE to {target}"))),
        },
        // PTR and CALL have their own conversion functions offering STRING and
        // nothing else (`conv.rs:74-88` and `:90-104`), reached because `conv`
        // dispatches a `Val::String` on `type_of()` rather than on the payload
        // (`:698-703`). The STRING target never arrives here — it short-circuits
        // above — so every target that does is a refusal, and the refusal names
        // the tag rather than the number.
        _ if dt == PTR => Err(Error(format!("Can not convert PTR to {target}"))),
        _ if dt == CALL => Err(Error(format!("Can not convert CALL to {target}"))),
        // **Three tags routed to a function that then refuses them.** `conv`
        // sends CLASS and OBJECT to `value_map_conversion` (`:729-732`) and
        // VALUEMAP to `true_value_map_conversion` (`:733-736`), and neither
        // guard admits what it was sent (`:595`, `:518`). The result is a
        // *mismatched-source* complaint naming MAP — a sentence about the
        // dispatcher's routing, not about the conversion the program asked for.
        // It is reproduced because `?try` can read it.
        _ if dt == VALUEMAP || dt == CLASS || dt == OBJECT => Err(Error(format!(
            "Source value is not MAP but {dt} and not suitable for conversion"
        ))),
        // **No target in this one.** `conv`'s final arm interpolates only the
        // source (`:743`), unlike every per-source arm above it. Adding the
        // target here would read better and match nothing.
        _ => Err(Error(format!("Can not convert Value from {dt}"))),
    }
}

/// Which source tags have a TEXTBUFFER arm at all.
///
/// The set is *not* "everything that converts to STRING", and the difference is
/// the whole reason this predicate exists. Each per-source function re-checks
/// the source tag before looking at the target
/// (`reference/rust_dynamic/src/conv.rs:187`, and the same shape at `:10,37,56,
/// 75,91,107,147,230,273,298,390,463,518,595,667`), so a source reaches a
/// TEXTBUFFER arm only if its own function both has one and admits the tag.
///
/// Present: FLOAT (`:131`), INTEGER (`:171`), STRING and TEXTBUFFER (`:215`,
/// guard `:187`), BOOL (`:258`), LIST (`:340`), LAMBDA (`:490`), MAP-family
/// (`:637`), JSON (`:676`).
///
/// **Absent, though each converts to STRING:** NODATA (`:36-53`), NONE
/// (`:55-72`), CALL (`:74-88`), PTR (`:90-104`), BIN (`:9-34`) and MATRIX
/// (`:268-292`).
///
/// **Absent for a second reason — routed to a function that refuses them.**
/// `conv` sends CLASS and OBJECT to `value_map_conversion` (`:729-732`) whose
/// guard admits neither (`:595`), and VALUEMAP to `true_value_map_conversion`
/// (`:733-736`) whose guard does not admit VALUEMAP either (`:518`). All three
/// fail with a *mismatched-source* message rather than an unknown-target one.
/// Bund2 reproduces the refusal; it does not reproduce which sentence explains
/// it, and no golden reads that text.
///
/// RESULT, QUEUE, FIFO, ASSOCIATION, INFO, CONFIG and MESSAGE also have arms in
/// the reference. They are not named here because Bund2 defines no constant for
/// them and constructs no value carrying one, so no `dt()` can equal them.
fn admits_textbuffer(dt: u16) -> bool {
    matches!(
        dt,
        FLOAT | INTEGER | STRING | TEXTBUFFER | BOOL | LIST | LAMBDA | MAP | CONDITIONAL | JSON
    )
}

/// `rustils::parse::boolean::string_to_bool`, which `conv` uses for
/// STRING → BOOL (`reference/rust_dynamic/src/conv.rs:207-211`).
///
/// It is total — there is no error arm — so an unrecognised string is `false`
/// rather than a failure, which is why `"maybe" convert.to_bool` succeeds.
/// Confirmed against the oracle for `"true"`, `"TRUE"`, `"1"`, `"yes"`, `"no"`
/// and `"maybe"`.
fn string_to_bool(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "true" | "t" | "yes" | "y" | "1" | "on"
    )
}

fn convert_to(vm: &mut dyn Vm, target: u16, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let v = crate::pull::operand(vm, prefix, 1)?;
    match conv_value(&v, target) {
        Ok(nv) => {
            vm.push(nv);
            Ok(())
        }
        Err(e) => Err(Error(format!("{prefix} returned error: {}", e.0))),
    }
}

/// The `.` sibling: operand off the workbench, answer back to the workbench
/// (`reference/rust_multistackvm/src/stdlib/convert/internal.rs:14-15,21,29`).
///
/// The guard checks the **workbench** here, and says so — unlike the print
/// family, whose workbench form guards the stack.
fn convert_to_wb(vm: &mut dyn Vm, target: u16, prefix: &str) -> Result<(), Error> {
    if vm.workbench_depth() < 1 {
        return Err(Error(format!(
            "Workbench is too shallow for inline {prefix}"
        )));
    }
    let v = crate::wb::operand(vm, crate::wb::Side::Bench, prefix)?;
    match conv_value(&v, target) {
        Ok(nv) => {
            vm.push_workbench(nv);
            Ok(())
        }
        Err(e) => Err(Error(format!("{prefix} returned error: {}", e.0))),
    }
}

/// `not` — `conv(BOOL)` and negate
/// (`reference/rust_multistackvm/src/stdlib/logic/logic_ops_fun.rs:9-36`).
///
/// It converts rather than requiring a BOOL, so `1 not` is `false` and
/// `0 not` is `true`. That is why this word lives beside the conversion table
/// rather than beside the comparisons.
fn not(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline not".into()));
    }
    let v = crate::pull::operand(vm, "NOT", 1)?;
    let b = conv_value(&v, BOOL)
        .map_err(|e| Error(format!("NOT returns error during boolean conversion: {}", e.0)))?;
    let Some(BundValue::Bool(b, _)) = Some(b.unboxed().clone()) else {
        return Err(Error("NOT returns error: not a boolean".into()));
    };
    vm.push(BundValue::boolean(!b));
    Ok(())
}

/// `and` and `or` — pull two, `conv(BOOL)` each, and combine
/// (`reference/rust_multistackvm/src/stdlib/logic/logic_ops_fun.rs:39-130`).
///
/// They convert as `not` does, so `1 0 and` is `false`. **`or` speaks `and`'s
/// words**: its guard and every error name `and` or `AND` (`:85-130`), a copy
/// of `and` with only the operator changed. The text is kept, since a golden
/// would pin it.
///
/// A string that is no boolean spelling panics the reference inside
/// `string_to_bool` (F68). Here the conversion returns an error instead (D37).
fn and_word(vm: &mut dyn Vm) -> Result<(), Error> {
    bool_pair(vm, |a, b| a & b)
}

fn or_word(vm: &mut dyn Vm) -> Result<(), Error> {
    bool_pair(vm, |a, b| a | b)
}

fn bool_pair(vm: &mut dyn Vm, op: fn(bool, bool) -> bool) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline and".into()));
    }
    let a = crate::pull::operand(vm, "AND", 1)?;
    let b = crate::pull::operand(vm, "AND", 2)?;
    let as_bool = |v: &BundValue| -> Result<bool, Error> {
        let c = conv_value(v, BOOL)
            .map_err(|e| Error(format!("AND returns error during boolean conversion: {}", e.0)))?;
        match c.unboxed() {
            BundValue::Bool(b, _) => Ok(*b),
            _ => Err(Error("AND returns error: not a boolean".into())),
        }
    };
    let (a, b) = (as_bool(&a)?, as_bool(&b)?);
    vm.push(BundValue::boolean(op(a, b)));
    Ok(())
}

pub fn register_words(r: &mut Registry) {
    // Each `convert.to_X` has a `.` sibling that takes its operand off the
    // workbench and leaves the answer there
    // (`reference/rust_multistackvm/src/stdlib/convert/internal.rs:21,28-29`),
    // so it consumes nothing from the stack and produces nothing on it.
    macro_rules! conv_word {
        ($name:literal, $target:expr, $prefix:literal) => {
            r.register_native(
                $name,
                |vm| convert_to(vm, $target, $prefix),
                eff(1, 1),
                WordKind::Sync,
            );
            r.register_native(
                concat!($name, "."),
                |vm| convert_to_wb(vm, $target, concat!($prefix, ".")),
                eff(0, 0),
                WordKind::Sync,
            );
        };
    }
    conv_word!("convert.to_string", STRING, "CONVERT.TO_STRING");
    conv_word!("convert.to_textbuffer", TEXTBUFFER, "CONVERT.TO_TEXTBUFFER");
    conv_word!("convert.to_int", INTEGER, "CONVERT.TO_INTEGER");
    conv_word!("convert.to_float", FLOAT, "CONVERT.TO_FLOAT");
    conv_word!("convert.to_bool", BOOL, "CONVERT.TO_BOOL");
    conv_word!("convert.to_list", LIST, "CONVERT.TO_LIST");
    // **The MATRIX family.** `matrix` and `matrix.` are aliases onto these
    // two (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:43-44`),
    // which is the whole of the reference's matrix word surface: there is no
    // constructor word and no literal.
    conv_word!("convert.to_matrix", MATRIX, "CONVERT.TO_MATRIX");
    // **`convert.to_dict` targets MAP here, and MATRIX in the reference.**
    // Both reference bodies pass `MATRIX` while their error prefixes say
    // `CONVERT.TO_DICT` (`internal.rs:99-105`), so the oracle's `to_dict`
    // answers a matrix — confirmed, `dt: 26`. That is F120, and D57 is the
    // decision to correct it rather than reproduce it: `conv`'s MAP target
    // exists and keys rows by position (`conv.rs:426-434`), which is what
    // this reaches.
    conv_word!("convert.to_dict", MAP, "CONVERT.TO_DICT");
    r.register_alias("matrix", "convert.to_matrix");
    r.register_alias("matrix.", "convert.to_matrix.");
    r.register_native("not", not, eff(1, 1), WordKind::Sync);
    r.register_native("and", and_word, eff(2, 1), WordKind::Sync);
    r.register_native("or", or_word, eff(2, 1), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::Interp;

    /// A LIST of LISTs becomes a MATRIX and comes back, as
    /// `reference/rust_dynamic/tests/conv-test.rs:152-173` asserts for the
    /// oracle. The rows are the same values; only the tag moves.
    #[test]
    fn a_list_of_lists_round_trips_through_matrix() {
        let i = run("[ [ 42.0 41.0 ] ] matrix").expect("converts");
        let m = i.peek().expect("a value");
        assert_eq!(m.dt(), MATRIX, "the tag is MATRIX");
        // **Rows are bare vectors**, not LIST values: that is D57 as amended,
        // and it is what the oracle renders.
        let rows = m.as_matrix().expect("rows");
        assert_eq!(rows.len(), 1);
        // No `as_float` on the value; the crate destructures, as
        // `library.rs` does.
        let floats: Vec<f64> = rows[0]
            .iter()
            .filter_map(|x| match *x.unboxed() {
                BundValue::Float(f, _) => Some(f),
                _ => None,
            })
            .collect();
        assert_eq!(floats, vec![42.0, 41.0], "the cells survived");

        let j = run("[ [ 42.0 41.0 ] ] matrix convert.to_list").expect("converts back");
        let l = j.peek().expect("a value");
        assert_eq!(l.dt(), LIST);
        assert_eq!(l.as_list().map(<[BundValue]>::len), Some(1));
    }

    /// **A MATRIX converts to a LIST and to nothing else.**
    /// `value_matrix_conversion` accepts a LIST target and falls to its `_`
    /// arm otherwise (`reference/rust_dynamic/src/conv.rs:268-290`), so
    /// converting a matrix to a matrix is refused rather than being the
    /// identity — which is what an earlier draft of this assumed, from
    /// reading the LIST converter's MATRIX arm instead of the MATRIX
    /// converter's. Confirmed on the oracle, which answers
    /// `CONVERT.TO_MATRIX returned error: Can not convert list to 26`.
    #[test]
    fn a_matrix_converts_to_a_list_and_refuses_a_second_matrix() {
        let i = run("[ [ 1 2 ] ] matrix convert.to_list").expect("to a LIST");
        let l = i.peek().expect("a value");
        assert_eq!(l.dt(), LIST);
        let rows = l.as_list().expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].as_list().map(<[BundValue]>::len), Some(2));

        // `Interp` has no `Debug`, so the error is matched rather than
        // `expect_err`-ed.
        let Err(e) = run("[ [ 1 2 ] ] matrix matrix") else {
            panic!("a second matrix must be refused");
        };
        assert!(e.contains("Can not convert list to 26"), "{e}");
    }

    /// **D57, the deviation.** `convert.to_dict` answers a MAP keyed by
    /// position, where the oracle answers a MATRIX (F120).
    #[test]
    fn to_dict_answers_a_map_keyed_by_position() {
        let i = run("[ 7 8 ] convert.to_dict").expect("converts");
        let m = i.peek().expect("a value");
        assert_eq!(m.dt(), MAP, "a MAP, not the oracle's MATRIX");
        assert_eq!(m.get("0").and_then(|v| v.as_int()), Some(7));
        assert_eq!(m.get("1").and_then(|v| v.as_int()), Some(8));
    }

    fn run(src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    fn top_str(src: &str) -> String {
        let i = run(src).expect("runs");
        i.peek().and_then(|v| v.as_str()).expect("a string")
    }

    fn top_bool(src: &str) -> bool {
        let i = run(src).expect("runs");
        match i.peek().map(|v| v.unboxed().clone()) {
            Some(BundValue::Bool(b, _)) => b,
            other => panic!("expected a bool, got {other:?}"),
        }
    }

    /// The conversions checked against the oracle, byte for byte, when this
    /// module was written. Twenty-six agreed; this pins the shapes.
    #[test]
    fn to_string_matches_the_oracle() {
        assert_eq!(top_str("1 convert.to_string"), "1");
        assert_eq!(top_str("-7 convert.to_string"), "-7");
        // A whole float keeps its `.0`, as `dtoa` produces (`conv.rs:128-130`).
        assert_eq!(top_str("1.0 convert.to_string"), "1.0");
        assert_eq!(top_str("3.14 convert.to_string"), "3.14");
        assert_eq!(top_str("true convert.to_string"), "true");
        assert_eq!(top_str("\"hi\" convert.to_string"), "hi");
    }

    #[test]
    fn to_bool_follows_the_reference_for_every_string_it_survives() {
        for (src, want) in [
            ("1", true),
            ("0", false),
            ("0.0", false),
            ("2.5", true),
            ("\"true\"", true),
            ("\"TRUE\"", true),
            ("\"yes\"", true),
            ("\"no\"", false),
            ("\"1\"", true),
            ("\"0\"", false),
        ] {
            assert_eq!(
                top_bool(&format!("{src} convert.to_bool")),
                want,
                "{src}"
            );
        }
    }

    /// **F68.** `"maybe" convert.to_bool` aborts the oracle — `rustils`'
    /// `string_to_bool` panics rather than returning, and the process exits 101
    /// with a Rust backtrace and no `comfy_table` report, because it never
    /// reaches `print_error`.
    ///
    /// D37 forbids reproducing that, so Bund2 answers `false`. This is the
    /// deviation, and it has no golden to be recorded against — no corpus
    /// program converts a string to a bool — so it is pinned here.
    #[test]
    fn an_unrecognised_string_is_false_rather_than_an_abort() {
        assert!(!top_bool("\"maybe\" convert.to_bool"));
        assert!(!top_bool("\"\" convert.to_bool"));
    }

    /// `not` converts before negating, so it accepts non-booleans
    /// (`logic_ops_fun.rs:15`). All four confirmed against the oracle.
    #[test]
    fn not_converts_before_negating() {
        assert!(!top_bool("1 not"));
        assert!(top_bool("0 not"));
        assert!(!top_bool("true not"));
        assert!(top_bool("\"no\" not"));
    }

    #[test]
    fn a_shallow_stack_is_reported_not_asserted() {
        for src in ["convert.to_string", "not"] {
            match run(src) {
                Ok(_) => panic!("{src} was expected to fail"),
                Err(e) => assert!(e.contains("too shallow"), "{src}: {e}"),
            }
        }
    }

    /// A TEXTBUFFER carries the text its STRING sibling would and differs only
    /// in the tag (`conv.rs:212-217`). All five confirmed against the oracle.
    #[test]
    fn to_textbuffer_is_the_string_text_under_a_different_tag() {
        for src in ["\"x\"", "1", "1.5", "true", "list"] {
            let i = run(&format!("{src} convert.to_textbuffer")).expect("runs");
            let v = i.peek().expect("a value");
            assert_eq!(v.dt(), TEXTBUFFER, "{src}");
            let s = run(&format!("{src} convert.to_string")).expect("runs");
            assert_eq!(
                v.as_str(),
                s.peek().and_then(|w| w.as_str()),
                "{src}: TEXTBUFFER and STRING disagreed on the text"
            );
        }
    }

    /// A TEXTBUFFER is a *source* as well as a target (`conv.rs:187`), so it
    /// round-trips rather than being a dead end.
    #[test]
    fn a_textbuffer_converts_onward() {
        assert_eq!(top_str("\"a\" convert.to_textbuffer convert.to_string"), "a");
        let i = run("\"1\" convert.to_textbuffer convert.to_int").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(1));
    }

    /// **The half of the table that is not the STRING column.** Every source
    /// here renders perfectly well as a string and still has no TEXTBUFFER arm,
    /// so a conversion that reused `display` unconditionally would succeed where
    /// the reference fails. Each message is the reference's, verified against
    /// the oracle.
    #[test]
    fn to_textbuffer_refuses_the_sources_that_have_no_arm() {
        for (src, want) in [
            ("nodata", "Can not convert NODATA to 22"),
            ("\"a\" ptr", "Can not convert PTR to 22"),
            (
                "valuemap",
                "Source value is not MAP but 30 and not suitable for conversion",
            ),
            (
                "class",
                "Source value is not MAP but 31 and not suitable for conversion",
            ),
            ("1 2 pair", "Can not convert Value from 10"),
            ("metrics", "Can not convert Value from 16"),
        ] {
            match run(&format!("{src} convert.to_textbuffer")) {
                Ok(_) => panic!("{src} convert.to_textbuffer was expected to fail"),
                Err(e) => assert!(
                    e.contains(want),
                    "{src}: wanted {want:?}, got {e:?}"
                ),
            }
            // The same source *does* convert to STRING — that is what makes the
            // refusal above a fact about the table and not about `display`.
            assert!(
                run(&format!("{src} convert.to_string")).is_ok(),
                "{src} convert.to_string should still work"
            );
        }
    }
}
