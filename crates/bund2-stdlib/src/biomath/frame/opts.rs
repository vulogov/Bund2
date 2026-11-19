//! RFC-0017 §S1's options reader for the frame words.
//!
//! RFC-0016's reader (`biomath/infer/opts.rs`, `read`) is private to its
//! module and knows only its own keys, so the frame words read theirs here.
//! The rules are RFC-0016 §S1's: keys are STRINGs; an unknown key, a key
//! the word does not take, or a value of the wrong type or range is E101
//! naming the key, with §S8's texts.
//!
//! **Two passes.** This module is the first: in the MAP's sorted key order,
//! each key for being known and taken, and its value for type, range and
//! repeated names; the first bad key is the refusal. The second, the checks
//! against the columns, is the word's own, after this answers.
//!
//! M2 reads `descending` (`frame.sort`). The other keys are known, so that
//! one given to the wrong word is "a key the word does not take"; their
//! values are read in the milestones that add the words taking them.

use bund2_api::Error;
use bund2_value::{BundValue, LIST, MAP, STRING};

use super::render;
use crate::biomath::operand::{WIDTH, domain};

/// Every key a frame word takes (§S1).
const KEYS: [&str; 6] = ["descending", "factors", "header", "how", "na", "sep"];

/// The options a word asked for, each `None` when not given.
#[derive(Debug, Default)]
pub(crate) struct Opts {
    /// `descending`: the names to sort descending, in the order given.
    pub descending: Option<Vec<String>>,
}

/// E101 on the `opts` operand.
fn refuse(word: &str, text: String) -> Error {
    domain(word, "opts", text)
}

/// A LIST of distinct STRINGs, for a key holding names.
fn names(word: &str, key: &str, v: &BundValue) -> Result<Vec<String>, Error> {
    let items = match v.as_list() {
        Some(xs) if v.dt() == LIST => xs,
        _ => return Err(refuse(word, format!("{key} is {}", render::value(v)))),
    };
    let mut out: Vec<String> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let s = match item.as_str() {
            Some(s) if item.dt() == STRING => s,
            _ => {
                return Err(refuse(
                    word,
                    format!("{key} item {} is {}", i + 1, render::value(item)),
                ));
            }
        };
        if out.contains(&s) {
            return Err(refuse(word, format!("{} given twice", render::text(&s))));
        }
        out.push(s);
    }
    Ok(out)
}

/// Read an options MAP for a word taking the keys `takes` (pass one).
pub(crate) fn read(word: &str, v: &BundValue, takes: &[&str]) -> Result<Opts, Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(refuse(word, v.summary(WIDTH))),
    };
    let mut o = Opts::default();
    for (k, x) in m {
        if !KEYS.contains(&k.as_str()) {
            return Err(refuse(word, format!("an unknown key {}", render::text(k))));
        }
        if !takes.contains(&k.as_str()) {
            return Err(refuse(
                word,
                format!("the key {}, which this word does not take", render::text(k)),
            ));
        }
        match k.as_str() {
            "descending" => o.descending = Some(names(word, k, x)?),
            _ => {
                return Err(Error::internal(format!(
                    "{word} takes the key {k:?}, which the frame options reader \
                     does not read yet"
                )));
            }
        }
    }
    Ok(o)
}
