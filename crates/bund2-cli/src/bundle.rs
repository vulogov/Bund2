//! `--emit=bundle` — the program carried inside the runtime's own image.
//!
//! **RFC-0006 §B1 and §B2.** A bundle is this binary with a program written
//! into a reserved region of it. `bund2 build` copies the running executable,
//! writes the program into that region **in place**, and changes nothing about
//! the file's size or layout.
//!
//! **Why in place rather than appended — Q40, measured 2026-09-30.** Appending
//! to a Mach-O leaves it runnable and permanently unvalidatable: `codesign -v`
//! reports "main executable failed strict validation", and `codesign -f -s -`
//! does not repair it, while the same re-sign of an unmodified copy validates.
//! Trailing data outside the image is what `codesign` refuses. A payload inside
//! the image can be signed afterwards, and signing stays outside `bund2 build`
//! so D10's toolchain-free half is untouched.
//!
//! **The runtime never reads its own executable.** The region is a `static`, so
//! a bundle asks its own memory what program it carries. That removes the whole
//! class of failures around locating the current binary — a moved, renamed, or
//! deleted executable, a hard link, a path with no read permission — none of
//! which a running program should have to solve to know what it is.
//!
//! **Only the builder touches a file**, and only a copy of one.

/// The program a bundle may carry — RFC-0006, 1 MiB on the owner's ruling.
///
/// Two hundred times the corpus's largest program (`workbench-variants.bund`,
/// 4,893 bytes) and negligible beside a runtime built with the JIT. It is a
/// fixed cost in every bundle runtime, present whether or not a program fills
/// it, which is why the number was decided rather than assumed.
pub const CAPACITY: usize = 1024 * 1024;

/// This container's version. Read before anything else in the region.
pub const FORMAT: u32 = 1;

/// Written where a payload would be, so the region lands in the image.
///
/// **Not zero, deliberately.** A zero-initialised static goes to `.bss` and
/// occupies no bytes in the file, so there would be nothing on disk for
/// `bund2 build` to write into.
const FILLER: u8 = 0xA5;

/// The sentinel, masked, so that **the plain bytes occur exactly once in the
/// image** — inside the region.
///
/// The builder scans a copy of the executable for the plain sentinel. If the
/// same 32 bytes also appeared as a comparison constant in this module's code,
/// the scan would find two matches and could patch the wrong one. Masking the
/// stored copy and unmasking it in a `const fn` keeps the literal out of the
/// text section while still giving the `static` below a compile-time value.
const SENTINEL_MASKED: [u8; 32] = [
    0xE7, 0xC8, 0xC7, 0xC3, 0xC9, 0xF1, 0xD7, 0xC4, 0xC9, 0xD2, 0xF1, 0xD6, 0xC4, 0xC2, 0xC8,
    0xC9, 0xC7, 0xD7, 0xF1, 0xB4, 0xB1, 0xB0, 0xB6, 0xF4, 0x19, 0x2A, 0x3B, 0x4C, 0x5D, 0x6E,
    0x7F, 0x80,
];

const fn unmask(mut m: [u8; 32]) -> [u8; 32] {
    let mut i = 0;
    while i < m.len() {
        m[i] ^= FILLER;
        i += 1;
    }
    m
}

/// The plain sentinel. Materialised into the region's initialiser, and rebuilt
/// at run time for the scan, so it is never a literal in the code.
const SENTINEL: [u8; 32] = unmask(SENTINEL_MASKED);

/// `state`: this runtime carries no program and is the plain interpreter.
const EMPTY: u8 = 0;
/// `state`: this runtime carries a program and runs it.
const FILLED: u8 = 1;

/// `flags`: D78's build-time floor. Run time may add, never remove.
pub const FLAG_NOIO: u8 = 1 << 0;
pub const FLAG_NOEVAL: u8 = 1 << 1;

/// The reserved region, as it sits in the file and in memory.
///
/// `repr(C)` because `bund2 build` addresses these fields by offset in a file
/// it did not compile.
#[repr(C)]
struct Region {
    sentinel: [u8; 32],
    format: u32,
    state: u8,
    flags: u8,
    _pad: [u8; 2],
    len: u32,
    /// Bund2's own version, NUL-padded. **Not the pinned `reference/` SHAs**:
    /// those say which oracle the program's meaning was fixed against and
    /// nothing about the code interpreting it, so a builder/runtime skew would
    /// otherwise be undetectable (RFC-0006 §B1).
    bund2_version: [u8; 32],
    /// The source path as `bund2 build` was given it, NUL-padded, so a
    /// diagnostic from a bundle names the same file a `script` run would.
    source_name: [u8; 256],
    payload: [u8; CAPACITY],
}

/// Where each field begins, for a builder writing into a file.
mod at {
    pub const FORMAT: usize = 32;
    pub const STATE: usize = 36;
    pub const FLAGS: usize = 37;
    pub const LEN: usize = 40;
    pub const VERSION: usize = 44;
    pub const SOURCE: usize = 76;
    pub const PAYLOAD: usize = 332;
}

/// The whole region's size on disk.
pub const REGION_BYTES: usize = at::PAYLOAD + CAPACITY;

#[used]
static REGION: Region = Region {
    sentinel: SENTINEL,
    format: FORMAT,
    state: EMPTY,
    flags: 0,
    _pad: [0; 2],
    len: 0,
    bund2_version: [0; 32],
    source_name: [0; 256],
    payload: [FILLER; CAPACITY],
};

/// What this binary carries, read from its own image.
pub struct Carried {
    pub source: String,
    pub name: String,
    pub flags: u8,
}

/// Why a region could not be read as a program.
///
/// Every one of these is an artefact that was damaged or built by a different
/// Bund2 — never a reason to abort. D37, and RFC-0006 criterion 12.
pub enum Damaged {
    /// A `state` byte that is neither empty nor filled.
    State(u8),
    /// A container version this runtime does not know.
    Format(u32),
    /// A length past the region's capacity.
    Length(u32),
    /// A payload that is not UTF-8 — the payload is source text (§B2).
    NotText,
}

impl std::fmt::Display for Damaged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State(b) => write!(
                f,
                "this artefact's payload marker is {b}, which is neither empty ({EMPTY}) \
                 nor a program ({FILLED}); the file is damaged"
            ),
            Self::Format(v) => write!(
                f,
                "this artefact carries container version {v} and this runtime reads \
                 version {FORMAT}; it was built by a different bund2"
            ),
            Self::Length(n) => write!(
                f,
                "this artefact claims a {n}-byte program and the region holds at most \
                 {CAPACITY}; the file is damaged"
            ),
            Self::NotText => write!(
                f,
                "this artefact's program is not valid UTF-8; a bundle carries source \
                 text, so the file is damaged"
            ),
        }
    }
}

/// Read a NUL-padded field as a `String`, stopping at the first NUL.
fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// What program this binary carries, if any.
///
/// `Ok(None)` is the plain interpreter: a runtime nobody has built a bundle
/// from. `Err` is an artefact that cannot be run and says why.
pub fn carried() -> Result<Option<Carried>, Damaged> {
    if REGION.format != FORMAT {
        return Err(Damaged::Format(REGION.format));
    }
    match REGION.state {
        EMPTY => Ok(None),
        FILLED => {
            let len = REGION.len as usize;
            if len > CAPACITY {
                return Err(Damaged::Length(REGION.len));
            }
            let bytes = REGION.payload.get(..len).ok_or(Damaged::Length(REGION.len))?;
            let source = std::str::from_utf8(bytes)
                .map_err(|_| Damaged::NotText)?
                .to_string();
            Ok(Some(Carried {
                source,
                name: text(&REGION.source_name),
                flags: REGION.flags,
            }))
        }
        other => Err(Damaged::State(other)),
    }
}

/// Find the region in a copy of this executable.
///
/// The scan is for the plain sentinel, which occurs exactly once in the image
/// by construction — see `SENTINEL_MASKED`. Two matches means the invariant
/// broke, and that is reported rather than guessed at.
fn locate(image: &[u8]) -> Result<usize, String> {
    let sentinel = unmask(SENTINEL_MASKED);
    let mut found: Option<usize> = None;
    for (i, w) in image.windows(sentinel.len()).enumerate() {
        if w == sentinel {
            if found.is_some() {
                return Err(
                    "the payload region's sentinel occurs more than once in this binary, \
                     so `bund2 build` cannot tell which one is the region"
                        .into(),
                );
            }
            found = Some(i);
        }
    }
    let start = found.ok_or_else(|| {
        "this binary has no payload region, so it cannot carry a program. A runtime \
         built without one is not a bundling runtime."
            .to_string()
    })?;
    if start + REGION_BYTES > image.len() {
        return Err(
            "the payload region runs past the end of this binary, which means the \
             file is truncated"
                .into(),
        );
    }
    Ok(start)
}

/// Write `source` into a copy of this executable, in place.
///
/// Returns the finished image. Nothing about the file's length or layout
/// changes, which is what Q40 showed a signature requires.
pub fn write_into(
    image: &mut [u8],
    source: &str,
    name: &str,
    flags: u8,
    version: &str,
) -> Result<(), String> {
    let start = locate(image)?;
    if source.len() > CAPACITY {
        return Err(format!(
            "the program is {} bytes and a bundle carries at most {CAPACITY}. \
             Nothing was written.",
            source.len()
        ));
    }
    let put = |image: &mut [u8], off: usize, bytes: &[u8]| -> Result<(), String> {
        let at = start + off;
        image
            .get_mut(at..at + bytes.len())
            .ok_or_else(|| format!("the payload region is short at offset {off}"))?
            .copy_from_slice(bytes);
        Ok(())
    };
    let len = u32::try_from(source.len())
        .map_err(|_| "the program's length does not fit the container".to_string())?;
    put(image, at::FORMAT, &FORMAT.to_le_bytes())?;
    put(image, at::STATE, &[FILLED])?;
    put(image, at::FLAGS, &[flags])?;
    put(image, at::LEN, &len.to_le_bytes())?;
    let mut ver = [0u8; 32];
    let vb = version.as_bytes();
    let n = vb.len().min(ver.len());
    ver[..n].copy_from_slice(&vb[..n]);
    put(image, at::VERSION, &ver)?;
    let mut nm = [0u8; 256];
    let nb = name.as_bytes();
    // **The tail, not the head.** A path longer than the field is truncated
    // from the left, because the file name carries more for a diagnostic than
    // the leading directories do.
    let keep = nb.len().min(nm.len());
    nm[..keep].copy_from_slice(&nb[nb.len() - keep..]);
    put(image, at::SOURCE, &nm)?;
    // The payload, then the filler over whatever a previous build left.
    put(image, at::PAYLOAD, source.as_bytes())?;
    let tail = at::PAYLOAD + source.len();
    let rest = CAPACITY - source.len();
    let filler = vec![FILLER; rest];
    put(image, tail, &filler)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The region is in the file, not in `.bss`.** If it were not, there
    /// would be nothing for `bund2 build` to write into and the failure would
    /// be a bundle that silently carried no program.
    #[test]
    fn the_region_is_present_in_this_binary() {
        let exe = std::env::current_exe().expect("a test binary has a path");
        let image = std::fs::read(&exe).expect("and it is readable");
        let start = locate(&image).expect("the region is located");
        assert_eq!(
            image.get(start..start + 32).map(<[u8]>::to_vec),
            Some(unmask(SENTINEL_MASKED).to_vec()),
            "the sentinel is where the scan says it is"
        );
    }

    /// The sentinel occurs **once**, which `locate` depends on.
    #[test]
    fn the_sentinel_occurs_exactly_once() {
        let exe = std::env::current_exe().expect("a test binary has a path");
        let image = std::fs::read(&exe).expect("and it is readable");
        let s = unmask(SENTINEL_MASKED);
        let n = image.windows(s.len()).filter(|w| *w == s).count();
        assert_eq!(n, 1, "the masked constant keeps the plain bytes unique");
    }

    /// A round trip through the region a builder writes and a runtime reads.
    #[test]
    fn a_written_program_reads_back() {
        let exe = std::env::current_exe().expect("a test binary has a path");
        let mut image = std::fs::read(&exe).expect("and it is readable");
        let before = image.len();
        write_into(&mut image, "1 2 + println\n", "/tmp/x.bund", FLAG_NOEVAL, "9.9.9")
            .expect("writes");
        assert_eq!(image.len(), before, "the file's length does not change");

        let start = locate(&image).expect("still locatable");
        assert_eq!(image[start + at::STATE], FILLED);
        assert_eq!(image[start + at::FLAGS], FLAG_NOEVAL);
        let len = u32::from_le_bytes([
            image[start + at::LEN],
            image[start + at::LEN + 1],
            image[start + at::LEN + 2],
            image[start + at::LEN + 3],
        ]) as usize;
        assert_eq!(len, "1 2 + println\n".len());
        assert_eq!(
            std::str::from_utf8(&image[start + at::PAYLOAD..start + at::PAYLOAD + len]),
            Ok("1 2 + println\n")
        );
        assert_eq!(text(&image[start + at::VERSION..start + at::VERSION + 32]), "9.9.9");
    }

    /// A program over capacity is refused, and **nothing is written** — the
    /// error names both numbers, so a build failure says what to do about it.
    #[test]
    fn a_program_over_capacity_is_refused_and_writes_nothing() {
        let exe = std::env::current_exe().expect("a test binary has a path");
        let mut image = std::fs::read(&exe).expect("and it is readable");
        let untouched = image.clone();
        let big = "x".repeat(CAPACITY + 1);
        let err = write_into(&mut image, &big, "big.bund", 0, "1.0").expect_err("refused");
        assert!(err.contains(&format!("{}", CAPACITY + 1)), "names the size: {err}");
        assert!(err.contains(&CAPACITY.to_string()), "names the capacity: {err}");
        assert_eq!(image, untouched, "and the image is unchanged");
    }

    /// This runtime carries no program, so `carried` is `Ok(None)` — the plain
    /// interpreter, not a failure.
    #[test]
    fn an_unbuilt_runtime_carries_nothing() {
        match carried() {
            Ok(None) => {}
            Ok(Some(_)) => panic!("a test binary is not a bundle"),
            Err(e) => panic!("and it is not damaged: {e}"),
        }
    }

    /// Every `Damaged` says which artefact fact was wrong, in numbers.
    #[test]
    fn every_damaged_case_explains_itself() {
        for (d, needle) in [
            (Damaged::State(7), "7"),
            (Damaged::Format(99), "99"),
            (Damaged::Length(CAPACITY as u32 + 1), "1048577"),
            (Damaged::NotText, "UTF-8"),
        ] {
            let text = d.to_string();
            assert!(text.contains(needle), "{text} should name {needle}");
            assert!(!text.is_empty());
        }
    }
}
