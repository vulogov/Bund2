//! RFC-0018 §S1's options reader for the model words.
//!
//! `frame/opts.rs` is not changed: its `KEYS` is one list for every frame
//! word, and adding this RFC's keys to it would change the frame words'
//! texts for those keys from "an unknown key" to "which this word does not
//! take". So the model words read theirs here, by the same rules and in
//! the same text forms: keys in the MAP's sorted order, the first bad key
//! the refusal, an unknown key, a key the word does not take, or a value
//! of the wrong type or range E101 naming the key.
//!
//! **Every key of every model word is known from the first milestone**, so
//! a key given to the wrong word reads "which this word does not take"
//! now, and the text does not move when a later milestone reads the key.
//! M1 reads `model.matrix`'s four: `intercept`, `weights`, `offset` and
//! `na`, the last with the frame words' text. M3 reads `fit.glm`'s `family`
//! and `link`, each a STRING whose value the word checks.

use bund2_api::Error;
use bund2_value::{BOOL, BundValue, MAP, STRING};

use crate::biomath::frame::opts::Missing;
use crate::biomath::frame::render;
use crate::biomath::operand::{WIDTH, domain};

/// Every key a model word takes (§S2 to §S7).
const KEYS: [&str; 10] = [
    "conf",
    "family",
    "intercept",
    "interval",
    "link",
    "na",
    "offset",
    "test",
    "type",
    "weights",
];

/// The options a word asked for, each `None` when not given.
#[derive(Debug, Default)]
pub(crate) struct Opts {
    /// `intercept` (`model.matrix`).
    pub intercept: Option<bool>,
    /// `weights`: a column's name.
    pub weights: Option<String>,
    /// `offset`: a column's name.
    pub offset: Option<String>,
    /// `na`: `"refuse"` or `"omit"`.
    pub missing: Option<Missing>,
    /// `family` (`fit.glm`, `model.anova`): its name, checked by the word.
    pub family: Option<String>,
    /// `link` (`fit.glm`, `model.anova`): its name, checked by the word.
    pub link: Option<String>,
}

/// E101 on the `opts` operand.
pub(crate) fn refuse(word: &str, text: String) -> Error {
    domain(word, "opts", text)
}

/// A column's name: a non-empty STRING.
fn column(word: &str, key: &str, v: &BundValue) -> Result<String, Error> {
    match v.as_str() {
        Some(s) if v.dt() == STRING && !s.is_empty() => Ok(s),
        _ => Err(refuse(word, format!("{key} is {}", render::value(v)))),
    }
}

/// A STRING naming a choice, checked against the choices by the word.
fn name(word: &str, key: &str, v: &BundValue) -> Result<String, Error> {
    match v.as_str() {
        Some(s) if v.dt() == STRING => Ok(s),
        _ => Err(refuse(word, format!("{key} is {}", render::value(v)))),
    }
}

/// Read an options MAP for a word taking the keys `takes`.
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
            "intercept" => match x.unboxed() {
                BundValue::Bool(b, _) if x.dt() == BOOL => o.intercept = Some(*b),
                _ => return Err(refuse(word, format!("intercept is {}", render::value(x)))),
            },
            "weights" => o.weights = Some(column(word, k, x)?),
            "family" => o.family = Some(name(word, k, x)?),
            "link" => o.link = Some(name(word, k, x)?),
            "offset" => o.offset = Some(column(word, k, x)?),
            "na" => {
                let s = x.as_str().filter(|_| x.dt() == STRING);
                o.missing = Some(match s.as_deref() {
                    Some("refuse") => Missing::Refuse,
                    Some("omit") => Missing::Omit,
                    _ => return Err(refuse(word, format!("na is {}", render::value(x)))),
                });
            }
            _ => {
                return Err(Error::internal(format!(
                    "{word} takes the key {k:?}, which the model options reader \
                     does not read yet"
                )));
            }
        }
    }
    Ok(o)
}
