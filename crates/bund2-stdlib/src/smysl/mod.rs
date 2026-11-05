//! SMYSL corpora formed, researched and queried from Bund: a family of
//! Bund2's own (RFC-0014, D244, D245).
//!
//! **None of this is the reference's.** No reference word is named `smysl.…`,
//! so `conform`, `IMPLEMENTED` and `COVERAGE` move by zero (D72), and what
//! checks the family is its unit tests. Compiled only under the `smysl`
//! feature (§S1); without it no `smysl.` word is registered.
//!
//! Built in the RFC's milestones, in order. So far, M2's to M5's:
//!
//! - [`handle`]: the handle MAP, its validation and canonicalisation, and
//!   opening the library it names (§S2, §S3);
//! - [`map`]: records read back as MAPs, and MAPs written as records
//!   (§S2), as functions the later words call;
//! - [`write`]: a guarded write (§S9): the process mutex, SMYSL's locks,
//!   the re-open, and R25's snapshot, which decides whether a failed write
//!   leaves the locks;
//! - [`form`]: units, relations and labels, with their attestations,
//!   decided under the locks as one batch, and retraction (§S4);
//! - [`surface`]: what `smysl.parse` answers and `smysl.write` writes,
//!   with their warnings (§S4);
//! - [`research`]: what the researching words answer (§S5);
//! - [`text`]: `smysl.text.add`'s spec, and its input read (§S4);
//! - [`unlock`]: what `bund2 smysl-unlock` does (§S9, R14);
//! - [`probe`]: the test-only probe, under `smysl-panic-probe` (§S9);
//! - [`words`]: the words, and the `--noio` stubs of those that touch
//!   disk (§S8, R11).

pub(crate) mod form;
pub mod handle;
pub mod map;
pub mod probe;
pub mod research;
pub mod surface;
pub(crate) mod text;
pub mod unlock;
pub mod words;
pub(crate) mod write;

use bund2_api::Error;
use smysl_core::error::LibError;

/// A refusal, before it is given its word: one of §S8's codes and the text
/// that follows `<W>: `.
///
/// The code travels with the text so that a test can assert both, and so
/// that the word that reports it is the one place a prefix is added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub code: Code,
    pub text: String,
}

/// §S8's codes. SM06 and SM11 are the house texts, unprefixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Sm00,
    Sm01,
    Sm02,
    Sm03,
    Sm04,
    Sm05,
    Sm06,
    Sm07,
    Sm08,
    Sm09,
    Sm10,
    Sm11,
    Sm12,
}

impl Refusal {
    pub fn new(code: Code, text: impl Into<String>) -> Refusal {
        Refusal {
            code,
            text: text.into(),
        }
    }

    /// The `Error` a word reports: `<W>: <text>`, the word's name in
    /// capitals, except for SM06 and SM11, whose texts stand alone.
    pub fn for_word(&self, word: &str) -> Error {
        match self.code {
            Code::Sm06 | Code::Sm11 => Error(self.text.clone()),
            _ => Error(format!("{}: {}", word.to_uppercase(), self.text)),
        }
    }
}

/// SM03 `<what> <reason>`.
pub(crate) fn sm03(text: impl Into<String>) -> Refusal {
    Refusal::new(Code::Sm03, text)
}

/// SM05 `unknown <kind>: <value>`.
pub(crate) fn sm05(kind: &str, value: &str) -> Refusal {
    Refusal::new(Code::Sm05, format!("unknown {kind}: {value}"))
}

/// SM02 `SMYSL refused it: <message>`, the message as SMYSL's `Display`
/// writes it (§S8).
pub(crate) fn sm02(message: impl std::fmt::Display) -> Refusal {
    Refusal::new(Code::Sm02, format!("SMYSL refused it: {message}"))
}

/// SMYSL's error as Bund2 displays it: a `LibError::Limit` without the
/// SMYSL CLI's flag for the cap, which Bund2 does not have (§S8), so it
/// reads `SMY-E440: <cap> limit <limit> exceeded by <saw>`.
pub(crate) fn shown(e: LibError) -> LibError {
    match e {
        LibError::Limit {
            cap, limit, saw, ..
        } => LibError::Limit {
            cap,
            limit,
            saw,
            flag: None,
        },
        other => other,
    }
}

/// SM02 for a `LibError`, displayed as [`shown`] says.
pub(crate) fn lib(e: LibError) -> Refusal {
    sm02(shown(e))
}

/// Register the family's words, or their `--noio` stubs.
pub fn register(r: &mut bund2_api::Registry, opts: &crate::host::HostOptions) {
    words::register(r, opts);
}

#[cfg(test)]
mod forming_tests;
#[cfg(test)]
mod research_tests;
