//! A guarded write (§S9): the process mutex, SMYSL's locks, the re-open,
//! and the guard that decides whether the locks go when the write ends.
//!
//! 1. open the library, and read its shard names;
//! 2. take the library lock, then the shard locks in name order, through
//!    `acquire_all` (`smysl-text/src/lock.rs:24-43`), naming the command as
//!    `bund2 <word>`;
//! 3. open the library a second time, and use only that open from here on;
//! 4. if it names a shard the locks do not cover, drop them and start again;
//!    a second mismatch is SM07;
//! 5. take R25's snapshot before the first call that may write, and if a
//!    call fails, a second, and compare;
//! 6. drop the locks, or forget them after a panic (R20) or a failed call
//!    that changed a log or the marker (R25).
//!
//! Writes to one root are serialised within the process by a table of
//! mutexes keyed by the canonical root (R13), before the file locks are
//! taken. A poisoned mutex is recovered: it guards no data, only the order
//! of writers.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use smysl_core::error::LibError;
use smysl_core::{Record, from_cbor_seq};
use smysl_text::Library;
use smysl_text::library::MARKER;
use smysl_text::lock::{Lock, acquire_all};

use super::handle;
use super::{Code, Refusal, sm02};

/// What an SM02 says when R25 or a panic keeps the locks (§S8).
pub const LEFT_LOCKED: &str =
    "; the library is left locked until an operator runs bund2 smysl-unlock";

// --- the process mutex (R13) -----------------------------------------------

/// Each canonical root's mutex. Never removed, so each is leaked once, when
/// its root is first written, and lives as long as the process.
type Table = Mutex<HashMap<PathBuf, &'static Mutex<()>>>;

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The mutex for a canonical root, held until this drops.
pub(crate) struct Serial {
    _guard: MutexGuard<'static, ()>,
}

/// Serialise this thread's write to `canon` behind every other write to it
/// in this process.
pub(crate) fn serialise(canon: &Path) -> Serial {
    let mutex: &'static Mutex<()> = {
        let mut t = table()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        t.entry(canon.to_path_buf())
            .or_insert_with(|| Box::leak(Box::new(Mutex::new(()))))
    };
    Serial {
        _guard: mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    }
}

// --- the guard (R20, R25) ----------------------------------------------------

/// SMYSL's locks for one write. Dropped normally, it releases them; dropped
/// while the thread panics, or after `keep`, it forgets them, so `Lock`'s
/// `Drop` never removes the files (`smysl-graph/src/lock.rs:225-228`).
pub(crate) struct Guard {
    locks: Vec<Lock>,
    keep: bool,
}

impl Guard {
    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if self.keep || std::thread::panicking() {
            for l in self.locks.drain(..) {
                std::mem::forget(l);
            }
        }
    }
}

// --- R25's snapshot ----------------------------------------------------------

/// One log as the snapshot sees it: its length on disk, and, when it is
/// torn, where its decoded records end and the bytes past that.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Log {
    len: u64,
    torn: Option<(usize, Vec<u8>)>,
}

/// The logs `shards::list` names and the marker's contents, before a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    logs: BTreeMap<PathBuf, Log>,
    marker: Vec<u8>,
}

/// Why a snapshot could not be taken: `shards::list`, a `metadata` or a
/// read failed.
#[derive(Debug)]
pub(crate) struct Unreadable(pub String);

fn length(p: &Path) -> Result<u64, Unreadable> {
    match std::fs::metadata(p) {
        Ok(m) => Ok(m.len()),
        // A log absent counts as length 0 (§S9).
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(Unreadable(format!("{}: {e}", p.display()))),
    }
}

fn read(p: &Path) -> Result<Vec<u8>, Unreadable> {
    match std::fs::read(p) {
        Ok(b) => Ok(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(Unreadable(format!("{}: {e}", p.display()))),
    }
}

impl Snapshot {
    /// The snapshot of the library at `root`, whose open decoded `stamp_len`
    /// bytes of log in all.
    ///
    /// The total of the logs' lengths on disk exceeds the stamp's length
    /// exactly when some log is torn, since the stamp counts only what was
    /// decoded (`smysl-text/src/shards.rs:395-416`). Only then is each log
    /// read and decoded, as `ShardLog::read` does
    /// (`smysl-text/src/shards.rs:234-251`), to keep each torn tail.
    pub(crate) fn take(root: &Path, stamp_len: u64) -> Result<Snapshot, Unreadable> {
        let listed = smysl_text::shards::list(root).map_err(|e| Unreadable(e.to_string()))?;
        let mut logs = BTreeMap::new();
        let mut total = 0u64;
        for (_, p) in &listed {
            let len = length(p)?;
            total = total.saturating_add(len);
            logs.insert(p.clone(), Log { len, torn: None });
        }
        if total > stamp_len {
            for (p, log) in &mut logs {
                let bytes = read(p)?;
                let (_, decoded) = from_cbor_seq(&bytes)
                    .map_err(|e| Unreadable(format!("{}: {e}", p.display())))?;
                if decoded < bytes.len() {
                    log.torn = Some((decoded, bytes[decoded..].to_vec()));
                }
            }
        }
        let marker = read(&root.join(MARKER))?;
        Ok(Snapshot { logs, marker })
    }

    /// Whether the library at `root` changed since this snapshot (§S9):
    /// a log's length differs, a log appeared, a torn log's bytes past its
    /// decoded length differ, or the marker's contents differ.
    pub(crate) fn changed(&self, root: &Path) -> Result<bool, Unreadable> {
        let listed = smysl_text::shards::list(root).map_err(|e| Unreadable(e.to_string()))?;
        for (_, p) in &listed {
            if !self.logs.contains_key(p) && length(p)? > 0 {
                return Ok(true);
            }
        }
        for (p, before) in &self.logs {
            if length(p)? != before.len {
                return Ok(true);
            }
            if let Some((at, tail)) = &before.torn {
                let now = read(p)?;
                if now.get(*at..) != Some(tail.as_slice()) {
                    return Ok(true);
                }
            }
        }
        Ok(read(&root.join(MARKER))? != self.marker)
    }
}

// --- the write -----------------------------------------------------------------

/// The library under the locks, for one word's calls.
pub(crate) struct Writer {
    pub root: PathBuf,
    pub library: Library,
    guard: Guard,
    snapshot: Option<Snapshot>,
}

impl Writer {
    /// Append `records` under the locks. The first call of the acquisition
    /// takes R25's snapshot first; a call that fails compares a second, and
    /// keeps the locks if the library changed or cannot be read.
    pub(crate) fn append(&mut self, records: &[Record]) -> Result<(), Refusal> {
        if records.is_empty() {
            return Ok(());
        }
        self.call(|lib| lib.append_records(records))
    }

    pub(crate) fn call(
        &mut self,
        f: impl FnOnce(&mut Library) -> Result<(), LibError>,
    ) -> Result<(), Refusal> {
        if self.snapshot.is_none() {
            let (stamp_len, _) = self.library.stamp();
            let s = Snapshot::take(&self.root, stamp_len)
                .map_err(|u| sm02(format!("the library's logs cannot be read: {}", u.0)))?;
            self.snapshot = Some(s);
        }
        super::probe::at_panic(&self.root);
        let failed = match f(&mut self.library) {
            Ok(()) => return Ok(()),
            Err(e) => e,
        };
        let keep = match &self.snapshot {
            Some(s) => s.changed(&self.root).unwrap_or(true),
            None => true,
        };
        if keep {
            self.guard.keep();
            return Err(Refusal::new(
                Code::Sm02,
                format!("SMYSL refused it: {failed}{LEFT_LOCKED}"),
            ));
        }
        Err(sm02(failed))
    }
}

/// SM07 for a held lock, SM02 for any other failure to take one (§S8).
fn locked(e: LibError) -> Refusal {
    match e {
        LibError::Locked { .. } => Refusal::new(Code::Sm07, format!("the library is locked: {e}")),
        other => sm02(other),
    }
}

/// Take the locks on `canon` for `word`, and open the library under them.
fn acquire(canon: &Path, word: &str) -> Result<Writer, Refusal> {
    let command = format!("bund2 {word}");
    let mut restarted = false;
    loop {
        let (_, first) = handle::open(canon)?;
        let shards = first.shards();
        drop(first);
        super::probe::at_pause(canon);
        let locks = acquire_all(canon, &shards, &command).map_err(locked)?;
        let guard = Guard { locks, keep: false };
        let (_, library) = handle::open(canon)?;
        let now = library.shards();
        match now.iter().find(|s| !shards.contains(s)) {
            None => {
                return Ok(Writer {
                    root: canon.to_path_buf(),
                    library,
                    guard,
                    snapshot: None,
                });
            }
            Some(s) if restarted => {
                return Err(Refusal::new(
                    Code::Sm07,
                    format!("the library's shards changed while locking: {s}"),
                ));
            }
            Some(_) => {
                drop(guard);
                restarted = true;
            }
        }
    }
}

/// Run `f` as one guarded write to the library at `root`, for `word`.
pub(crate) fn write<T>(
    word: &str,
    root: &Path,
    f: impl FnOnce(&mut Writer) -> Result<T, Refusal>,
) -> Result<T, Refusal> {
    let canon = handle::canonical(root)?;
    let _serial = serialise(&canon);
    let mut w = acquire(&canon, word)?;
    f(&mut w)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use smysl_core::{KernelType, Status, UnitCoreBuilder, to_cbor};

    use super::*;

    struct Dir(PathBuf);

    impl Dir {
        fn library(tag: &str) -> Dir {
            static N: AtomicUsize = AtomicUsize::new(0);
            let p = std::env::temp_dir().join(format!(
                "bund2-smysl-write-{}-{tag}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = std::fs::remove_dir_all(&p);
            Library::create(&p).unwrap();
            Dir(std::fs::canonicalize(&p).unwrap())
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn record(gist: &str) -> Vec<u8> {
        let core = UnitCoreBuilder::new(KernelType::Claim, gist, Status::Speculative)
            .build()
            .unwrap();
        to_cbor(&Record::Unit(core))
    }

    fn catalog(root: &Path) -> PathBuf {
        root.join(smysl_text::library::CATALOG)
    }

    fn append_bytes(p: &Path, bytes: &[u8]) {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .unwrap();
        f.write_all(bytes).unwrap();
    }

    fn snapshot(root: &Path) -> Snapshot {
        let lib = Library::open(root).unwrap();
        Snapshot::take(root, lib.stamp().0).unwrap()
    }

    /// The whole record a torn tail is replaced by.
    fn short() -> Vec<u8> {
        record("short")
    }

    /// A library whose `zeta` shard log holds one whole record and, when
    /// `torn`, a torn tail after it: the first `short().len()` bytes of a
    /// longer record, which cannot decode.
    fn with_zeta(tag: &str, torn: bool) -> Dir {
        let d = Dir::library(tag);
        let zeta = d.0.join(smysl_text::shards::log_path("zeta"));
        std::fs::create_dir_all(zeta.parent().unwrap()).unwrap();
        append_bytes(&zeta, &record("whole"));
        if torn {
            let long = record("a gist a good deal longer than the short one");
            append_bytes(&zeta, &long[..short().len()]);
        }
        d
    }

    /// Criterion 15: a changed log keeps the locks, wherever the change is.
    #[test]
    fn a_snapshot_sees_every_change_and_only_changes() {
        // Nothing touched, with and without a torn tail: unchanged.
        for torn in [false, true] {
            let d = with_zeta("still", torn);
            let s = snapshot(&d.0);
            assert_eq!(s.logs.values().any(|l| l.torn.is_some()), torn);
            assert!(!s.changed(&d.0).unwrap(), "torn = {torn}");
        }

        // A whole record appended to the catalog, as a write whose
        // `sync_data` failed would leave it: the stamp does not move.
        let d = Dir::library("synced");
        let s = snapshot(&d.0);
        append_bytes(&catalog(&d.0), &record("appended"));
        assert!(s.changed(&d.0).unwrap());

        // A log shortened.
        let d = with_zeta("short", false);
        let s = snapshot(&d.0);
        let zeta = d.0.join(smysl_text::shards::log_path("zeta"));
        let len = std::fs::metadata(&zeta).unwrap().len();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&zeta)
            .unwrap()
            .set_len(len - 1)
            .unwrap();
        assert!(s.changed(&d.0).unwrap());

        // A log created under `log/shards`.
        let d = Dir::library("created");
        let s = snapshot(&d.0);
        let eta = d.0.join(smysl_text::shards::log_path("eta"));
        std::fs::create_dir_all(eta.parent().unwrap()).unwrap();
        append_bytes(&eta, &record("new shard"));
        assert!(s.changed(&d.0).unwrap());

        // The marker rewritten.
        let d = Dir::library("marker");
        let s = snapshot(&d.0);
        std::fs::write(d.0.join(MARKER), "smysl/library/1\n").unwrap();
        assert!(s.changed(&d.0).unwrap());

        // A torn tail replaced by a whole record of the same length: no
        // length changes, and the tail does.
        let d = with_zeta("replaced", true);
        let s = snapshot(&d.0);
        let zeta = d.0.join(smysl_text::shards::log_path("zeta"));
        let bytes = std::fs::read(&zeta).unwrap();
        let (_, decoded) = from_cbor_seq(&bytes).unwrap();
        assert_eq!(bytes.len() - decoded, short().len());
        let mut replaced = bytes[..decoded].to_vec();
        replaced.extend_from_slice(&short());
        std::fs::write(&zeta, &replaced).unwrap();
        assert_eq!(std::fs::metadata(&zeta).unwrap().len(), bytes.len() as u64);
        assert!(s.changed(&d.0).unwrap());
    }
}
