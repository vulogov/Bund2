//! `smysl.text.add`'s spec (§S4): every field of SMYSL's `AddSpec` is a key
//! of the spec MAP or has the default named here, and the input is read by
//! Bund2, since a reader takes bytes.

use std::collections::BTreeMap;

use bund2_value::{BundValue, MAP, STRING};
use smysl_core::Carry;
use smysl_text::{AddSpec, Budget, Caps, LangTag};

use super::{Refusal, lib, sm03, sm05};

/// The readers this build enables (§S1).
const READERS: [&str; 2] = ["txt/1", "md/1"];

/// A spec, checked, and the input it names, read.
pub(crate) struct Spec {
    pub spec: AddSpec,
    pub input: Vec<u8>,
}

fn text<'a>(key: &str, v: &'a BundValue) -> Result<&'a str, Refusal> {
    v.str_ref().filter(|_| v.dt() == STRING).ok_or_else(|| {
        sm03(format!(
            "spec {key} must be STRING, found {}",
            v.type_name()
        ))
    })
}

fn required<'a>(m: &'a BTreeMap<String, BundValue>, key: &str) -> Result<&'a str, Refusal> {
    text(
        key,
        m.get(key)
            .ok_or_else(|| sm03(format!("spec lacks {key}")))?,
    )
}

/// SM02 `SMY-E440` for an input over `Caps::default().input_bytes`, by
/// SMYSL's own `Budget::new` (`smysl-text/src/limits.rs:168-172`), displayed
/// without the SMYSL CLI's flag (§S8).
fn within_cap(len: u64) -> Result<(), Refusal> {
    Budget::new(Caps::default(), len).map(|_| ()).map_err(lib)
}

/// The spec MAP `#k` holds, checked, and its input read.
///
/// - `path` or `text`, one of the two. A `path`'s length is checked against
///   the cap before a byte of it is read, and a `text`'s the same way.
/// - `reader`: `"txt/1"` or `"md/1"`; any other is SM05.
/// - `alias`, `licence` and `lang` are required: SMYSL requires a licence
///   (`smysl-text/src/library.rs:95`), and refuses an add when neither the
///   caller nor the reader names a language (`:1193-1203`), which neither
///   enabled reader does (`smysl-text/src/readers/mod.rs:253,268-269`).
/// - `title` is optional; `carry` is `"none"`, `"ref"` or `"text"`, and
///   `"none"` when absent, SMYSL's `Carry` default.
/// - `params`, the part policy, the pseudonym key and the versification
///   scheme are SMYSL's defaults: none is offered in Phase 1.
pub(crate) fn spec(v: &BundValue, k: usize) -> Result<Spec, Refusal> {
    const KEYS: [&str; 8] = [
        "path", "text", "reader", "alias", "licence", "lang", "title", "carry",
    ];
    let m = v.as_map().filter(|_| v.dt() == MAP).ok_or_else(|| {
        sm03(format!(
            "#{k} parameter must be MAP, found {}",
            v.type_name()
        ))
    })?;
    if let Some(key) = m.keys().find(|key| !KEYS.contains(&key.as_str())) {
        return Err(sm03(format!("spec has no key {key}")));
    }
    let reader = required(m, "reader")?;
    if !READERS.contains(&reader) {
        return Err(sm05("reader", reader));
    }
    let alias = required(m, "alias")?;
    let licence = required(m, "licence")?;
    let lang = required(m, "lang")?;
    let lang = LangTag::new(lang).map_err(|e| sm03(format!("spec lang {lang} is refused: {e}")))?;
    let mut spec = AddSpec::new(reader, alias, licence).with_lang(lang);
    if let Some(t) = m.get("title") {
        spec.title = Some(text("title", t)?.to_owned());
    }
    if let Some(c) = m.get("carry") {
        let c = text("carry", c)?;
        let carry = Carry::ALL
            .iter()
            .find(|x| x.as_str() == c)
            .ok_or_else(|| sm03(format!("spec carry must be none, ref or text, found {c}")))?;
        spec = spec.with_carry(*carry);
    }
    let input = match (m.get("path"), m.get("text")) {
        (Some(_), Some(_)) => return Err(sm03("spec has both path and text")),
        (None, None) => return Err(sm03("spec lacks path or text")),
        (Some(p), None) => {
            let p = text("path", p)?;
            let unreadable = |e: std::io::Error| sm03(format!("spec path {p} cannot be read: {e}"));
            within_cap(std::fs::metadata(p).map_err(unreadable)?.len())?;
            std::fs::read(p).map_err(unreadable)?
        }
        (None, Some(t)) => {
            let t = text("text", t)?;
            within_cap(t.len() as u64)?;
            t.as_bytes().to_vec()
        }
    };
    Ok(Spec { spec, input })
}
