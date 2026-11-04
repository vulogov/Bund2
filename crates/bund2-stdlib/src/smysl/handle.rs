//! The handle MAP (§S2, R3) and opening the library it names (§S3).
//!
//! A handle is plain data, `{ type: "smysl", root, agent }`. It holds
//! nothing open: each word opens the library its handle names and closes it
//! before returning, with no cache (R4).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bund2_value::BundValue;
use smysl_core::AgentId;
use smysl_text::Library;
use smysl_text::library::MARKER;

use super::{Code, Refusal, sm02};

/// The agent a handle attests as when nothing names another (§S2, R15).
pub const DEFAULT_AGENT: &str = "tool:bund2";

/// A handle, validated: an absolute root, as given, and an agent SMYSL's
/// grammar accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handle {
    pub root: PathBuf,
    pub agent: AgentId,
}

impl Handle {
    /// The handle `v` holds, as operand `#k`, or SM00.
    ///
    /// Every word validates the handle it is given, without I/O: `type`, a
    /// STRING `root` that is an absolute path (`Path::is_absolute`), and an
    /// `agent` SMYSL's grammar accepts (`AgentId::new`). A hand-built handle
    /// may name another spelling of the directory; the words that open it
    /// canonicalise it again (§S2).
    pub fn from_value(v: &BundValue, k: usize) -> Result<Handle, Refusal> {
        let refuse = || Refusal::new(Code::Sm00, format!("#{k} parameter must be a SMYSL handle"));
        let m = v.as_map().ok_or_else(refuse)?;
        let field = |key: &str| m.get(key).and_then(|f| f.str_ref().map(str::to_owned));
        if field("type").as_deref() != Some("smysl") {
            return Err(refuse());
        }
        let root = PathBuf::from(field("root").ok_or_else(refuse)?);
        if !root.is_absolute() {
            return Err(refuse());
        }
        let agent = AgentId::new(field("agent").ok_or_else(refuse)?).map_err(|_| refuse())?;
        Ok(Handle { root, agent })
    }

    /// The handle as the MAP a word answers.
    pub fn to_value(&self) -> BundValue {
        handle_value(&self.root, &self.agent)
    }

    /// The same handle with another agent. No I/O: the root is passed on as
    /// it is, so this works on a handle for a library not yet created
    /// (§S2, R11).
    pub fn with_agent(&self, agent: AgentId) -> Handle {
        Handle {
            root: self.root.clone(),
            agent,
        }
    }
}

fn handle_value(root: &Path, agent: &AgentId) -> BundValue {
    let mut m = BTreeMap::new();
    m.insert("type".to_owned(), BundValue::str("smysl"));
    m.insert(
        "root".to_owned(),
        BundValue::str(root.to_string_lossy().into_owned()),
    );
    m.insert("agent".to_owned(), BundValue::str(agent.as_str()));
    BundValue::map(m)
}

/// `root`'s canonical form (`std::fs::canonicalize`), or SM01: `no SMYSL
/// library at <root>` for a path that does not exist, and `cannot reach
/// <root>: <the I/O error>` for any other failure (§S8).
pub fn canonical(root: &Path) -> Result<PathBuf, Refusal> {
    let canon = std::fs::canonicalize(root).map_err(|e| unreachable_root(root, &e))?;
    // A handle's root is a STRING, so a canonical path that is not Unicode
    // could not be handed back and found again.
    if canon.to_str().is_none() {
        return Err(Refusal::new(
            Code::Sm01,
            format!(
                "cannot reach {}: its canonical path is not valid Unicode",
                root.display()
            ),
        ));
    }
    Ok(canon)
}

fn unreachable_root(root: &Path, e: &std::io::Error) -> Refusal {
    if e.kind() == std::io::ErrorKind::NotFound {
        no_library(root)
    } else {
        Refusal::new(Code::Sm01, format!("cannot reach {}: {e}", root.display()))
    }
}

fn no_library(root: &Path) -> Refusal {
    Refusal::new(
        Code::Sm01,
        format!("no SMYSL library at {}", root.display()),
    )
}

/// Open the library at `root`: canonicalise it, find its marker, then
/// `Library::open`. Answers the canonical root with the library.
///
/// - No directory, or a directory with no marker, is SM01 (§S8): there is
///   no library there. SMYSL's own open reports a missing marker as an I/O
///   error that names the file (`smysl-text/src/library.rs:301-304`), which
///   would read as SM02; asking first keeps "nothing here" apart from
///   "something here SMYSL refused".
/// - Anything `Library::open` then refuses is SM02, with SMYSL's text: a
///   marker of another layout, or a primary shard this build cannot read
///   (§Current, "The library").
pub fn open(root: &Path) -> Result<(PathBuf, Library), Refusal> {
    let canon = canonical(root)?;
    match std::fs::metadata(canon.join(MARKER)) {
        Ok(m) if m.is_file() => {}
        Ok(_) => return Err(no_library(root)),
        Err(e) => return Err(unreachable_root(root, &e)),
    }
    let library = Library::open(&canon).map_err(sm02)?;
    Ok((canon, library))
}

/// The handle `smysl.open` answers for a canonical root.
pub fn opened(canon: &Path) -> Result<BundValue, Refusal> {
    let agent = AgentId::new(DEFAULT_AGENT).map_err(|e| {
        Refusal::new(
            Code::Sm02,
            format!("the default agent {DEFAULT_AGENT} is refused: {e}"),
        )
    })?;
    Ok(handle_value(canon, &agent))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle(root: &str, agent: &str) -> BundValue {
        let mut m = BTreeMap::new();
        m.insert("type".to_owned(), BundValue::str("smysl"));
        m.insert("root".to_owned(), BundValue::str(root));
        m.insert("agent".to_owned(), BundValue::str(agent));
        BundValue::map(m)
    }

    #[test]
    fn a_handle_needs_its_type_an_absolute_root_and_an_agent() {
        let ok = Handle::from_value(&handle("/tmp/lib", "tool:bund2"), 1).unwrap();
        assert_eq!(ok.root, PathBuf::from("/tmp/lib"));
        assert_eq!(ok.agent.as_str(), "tool:bund2");

        let sm00 = |v: &BundValue, k| {
            let r = Handle::from_value(v, k).unwrap_err();
            assert_eq!(r.code, Code::Sm00);
            r.text
        };
        assert_eq!(
            sm00(&BundValue::str("/tmp/lib"), 1),
            "#1 parameter must be a SMYSL handle"
        );
        assert_eq!(
            sm00(&handle("lib", "tool:bund2"), 2),
            "#2 parameter must be a SMYSL handle"
        );
        sm00(&handle("/tmp/lib", "not an agent"), 1);
        let mut wrong_type = handle("/tmp/lib", "tool:bund2").as_map().unwrap().clone();
        wrong_type.insert("type".to_owned(), BundValue::str("kb"));
        sm00(&BundValue::map(wrong_type), 1);
        let mut int_root = handle("/tmp/lib", "tool:bund2").as_map().unwrap().clone();
        int_root.insert("root".to_owned(), BundValue::int(7));
        sm00(&BundValue::map(int_root), 1);
    }

    #[test]
    fn a_handle_round_trips_through_its_map() {
        let h = Handle::from_value(&handle("/tmp/lib", "human:ann"), 1).unwrap();
        assert_eq!(Handle::from_value(&h.to_value(), 1).unwrap(), h);
    }
}
