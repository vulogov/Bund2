//! What `bund2 smysl-unlock <root>` does (§S9, R14): list a library's locks
//! with their holders, and, when told, break them.
//!
//! A subcommand and not a word, so a Bund program cannot break a lock:
//! breaking stays an operator's act, as SMYSL asks
//! (`smysl-graph/src/lock.rs:188-193`). The work is here so that a test can
//! call it after a probe-armed panic; the subcommand prints what this
//! answers and nothing more.

use std::path::{Path, PathBuf};

use smysl_text::lock::{Holder, Lock};

/// One lock found: where it is, and who holds it, if its file says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub path: PathBuf,
    pub holder: Option<Holder>,
}

/// Every lock of the library at `root`: `<root>/lock`, then every
/// `<root>/log/*.lock` in name order. With `breaking`, each is broken with
/// `Lock::break_lock`, the shard locks first, then the library lock.
pub fn unlock(root: &Path, breaking: bool) -> Result<Vec<Found>, String> {
    let library = root.join("lock");
    let mut shards: Vec<PathBuf> = match std::fs::read_dir(root.join("log")) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "lock") && p.is_file())
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("{}: {e}", root.join("log").display())),
    };
    shards.sort();
    let mut found = Vec::new();
    if library.is_file() {
        found.push(Found {
            holder: Lock::holder(&library),
            path: library.clone(),
        });
    }
    for p in &shards {
        found.push(Found {
            holder: Lock::holder(p),
            path: p.clone(),
        });
    }
    if breaking {
        for p in shards.iter().chain(library.is_file().then_some(&library)) {
            Lock::break_lock(p).map_err(|e| format!("{}: {e}", p.display()))?;
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_lists_then_breaks_the_library_lock_and_the_shard_locks() {
        let root = std::env::temp_dir().join(format!("bund2-smysl-unlock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        smysl_text::Library::create(&root).unwrap();
        let held =
            smysl_text::lock::acquire_all(&root, &["catalog".to_owned()], "bund2 test").unwrap();
        // As a crashed process leaves them.
        for l in held {
            std::mem::forget(l);
        }
        let found = unlock(&root, false).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].path, root.join("lock"));
        let holder = found[0].holder.as_ref().unwrap();
        assert_eq!(holder.pid, std::process::id());
        assert_eq!(holder.command, "bund2 test");
        assert!(root.join("lock").exists());

        assert_eq!(unlock(&root, true).unwrap().len(), 2);
        assert!(unlock(&root, false).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
