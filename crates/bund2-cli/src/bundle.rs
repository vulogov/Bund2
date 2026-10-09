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
//! the image can be signed afterwards — and has to be, before it will run at
//! all, so `bund2 build` re-signs ad hoc on macOS. `/usr/bin/codesign` is a
//! base-system binary and not the toolchain D10 forbids (D81).
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

/// The sentinel, masked so that the plain bytes are **less** likely to appear
/// in the code — never "exactly once", which is what the first version of this
/// comment claimed and release builds disproved.
///
/// **What went wrong, recorded because the fix depends on it.** The argument
/// was that masking kept the literal out of the text section, so a scan for
/// the plain bytes would find one match. That holds in a debug build, where
/// `unmask` is a call. In release, thin LTO const-folds it and materialises
/// the plain 32 bytes for the comparison — so the scan finds two, and
/// `bund2 build` refused every release bundle with "the sentinel occurs more
/// than once". The unit test asserting uniqueness passed throughout, because
/// `cargo test` builds in debug.
///
/// **So uniqueness is not relied on.** [`locate`] disambiguates
/// *structurally*: a real region is a sentinel **followed by a header that
/// validates**, and a bare constant in the text section is not. The masking
/// stays because fewer candidates is still better than more.
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

/// How much of the source path a bundle records. A longer one keeps its tail.
pub const SOURCE_NAME_BYTES: usize = 256;

/// `flags`: D78's build-time floor. Run time may add, never remove.
pub const FLAG_NOIO: u8 = 1 << 0;
pub const FLAG_NOEVAL: u8 = 1 << 1;
/// Every restriction this runtime can enforce.
///
/// **A bit outside this is refused, not ignored.** Both readers tested the
/// two bits they knew and passed over the rest, so an artefact carrying a
/// restriction a later bund2 defines inspected as `restrictions none` and ran
/// unrestricted on an earlier one — the silent loosening D78 exists to
/// prevent, reported by the tool D78 gives for seeing it. A restriction that
/// cannot be enforced is a reason not to run.
pub const FLAGS_KNOWN: u8 = FLAG_NOIO | FLAG_NOEVAL;

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
    source_name: [u8; SOURCE_NAME_BYTES],
    /// **Which cargo features the runtime was built with**, NUL-padded.
    ///
    /// §B4: D16 means no word can be proven unused, so the *feature set*
    /// decides which words exist — a program calling `string.grok` works
    /// under one bundle and fails at run time under another. Unrecorded,
    /// nothing can tell a holder which artefact they have.
    features: [u8; 64],
    /// **Which oracle the program's meaning was fixed against**, NUL-padded:
    /// the `reference/PINNED.txt` SHAs, compacted.
    ///
    /// §B1: `bund2_version` says what code is interpreting, and this says what
    /// it was conformed to. A bundle built before a conformance change can
    /// then be identified rather than silently reinterpreted.
    pinned: [u8; 256],
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
    pub const FEATURES: usize = 332;
    pub const PINNED: usize = 396;
    pub const PAYLOAD: usize = 652;
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
    source_name: [0; SOURCE_NAME_BYTES],
    features: [0; 64],
    pinned: [0; 256],
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
    /// Restriction bits outside [`FLAGS_KNOWN`]; the unknown bits are carried.
    Restriction(u8),
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
            Self::Restriction(bits) => write!(
                f,
                "this artefact records a restriction this runtime does not know \
                 (flag bits {bits:#04x}); it cannot be enforced here, so the program is \
                 not run. A bund2 writes only bits it knows into its own artefact, so \
                 the byte was changed after the build, or the file is damaged"
            ),
        }
    }
}

/// A NUL-padded fixed field, truncated rather than refused.
///
/// **Truncation is right for these.** They are for a reader to identify an
/// artefact; a feature list or a SHA summary that does not fit is still worth
/// most of what it says, and refusing a build over it would be worse.
fn padded<const N: usize>(text: &str) -> [u8; N] {
    let mut out = [0u8; N];
    let b = text.as_bytes();
    let n = b.len().min(N);
    out[..n].copy_from_slice(&b[..n]);
    out
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
    // **Read through `black_box`, or release builds see the initialiser — the
    // second blocker a release build found.**
    //
    // `REGION` is an immutable `static`, so a compiler is entitled to fold
    // every read of it to the value written here at compile time. That is
    // exactly what happened: the builder patched `state` to `FILLED` in the
    // file, the bytes on disk said so, and the running program still took the
    // `EMPTY` arm and behaved as the plain interpreter. In debug nothing folds
    // and it worked, which is why eleven tests passed over a design that could
    // not work in the profile that ships.
    //
    // `black_box` exists for this: it hints that the value may be anything, so
    // the load happens. **It is a hint and not a guarantee**, which is why
    // `cargo xtask bundle` builds and runs a release artefact — if a future
    // compiler folds through it, that check fails rather than a user's
    // artefact silently becoming an interpreter. **No fallback is decided.**
    // A runtime that reads its payload from its own file cannot be folded, and
    // it is the construction D118 did not take; reaching for it would be a new
    // decision (RFC-0006 §B1).
    let region = std::hint::black_box(&REGION);
    if region.format != FORMAT {
        return Err(Damaged::Format(region.format));
    }
    match region.state {
        EMPTY => Ok(None),
        FILLED => {
            let len = region.len as usize;
            if len > CAPACITY {
                return Err(Damaged::Length(region.len));
            }
            let bytes = region.payload.get(..len).ok_or(Damaged::Length(region.len))?;
            let source = std::str::from_utf8(bytes)
                .map_err(|_| Damaged::NotText)?
                .to_string();
            if region.flags & !FLAGS_KNOWN != 0 {
                return Err(Damaged::Restriction(region.flags & !FLAGS_KNOWN));
            }
            Ok(Some(Carried {
                source,
                name: text(&region.source_name),
                flags: region.flags,
            }))
        }
        other => Err(Damaged::State(other)),
    }
}

/// What an artefact says about itself — RFC-0006 §B3a and D78.
pub struct Inspected {
    pub format: u32,
    pub carries_program: bool,
    pub used: usize,
    pub capacity: usize,
    pub noio: bool,
    pub noeval: bool,
    /// Restriction bits this bund2 does not define; zero for anything it built.
    pub unknown_flags: u8,
    pub bund2_version: String,
    pub features: String,
    pub pinned: String,
    pub source_name: String,
}

/// Read an artefact's region **from a file**, for `bund2 build --inspect`.
///
/// **D78 requires this.** "A restriction the runner cannot observe is one they
/// cannot rely on" — so the floor a build wrote has to be readable by whoever
/// was handed the result, not only by whoever produced it.
pub fn inspect(image: &[u8]) -> Result<Inspected, String> {
    let start = locate(image)?;
    let field = |off: usize, n: usize| -> String {
        image
            .get(start + off..start + off + n)
            .map(text)
            .unwrap_or_default()
    };
    let byte = |off: usize| image.get(start + off).copied().unwrap_or(0);
    let word = |off: usize| -> u32 {
        image
            .get(start + off..start + off + 4)
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0)
    };
    let flags = byte(at::FLAGS);
    Ok(Inspected {
        format: word(at::FORMAT),
        carries_program: byte(at::STATE) == FILLED,
        used: word(at::LEN) as usize,
        capacity: CAPACITY,
        noio: flags & FLAG_NOIO != 0,
        noeval: flags & FLAG_NOEVAL != 0,
        unknown_flags: flags & !FLAGS_KNOWN,
        bund2_version: field(at::VERSION, 32),
        features: field(at::FEATURES, 64),
        pinned: field(at::PINNED, 256),
        source_name: field(at::SOURCE, SOURCE_NAME_BYTES),
    })
}

/// Find the region in a copy of this executable.
///
/// **Identified, not assumed unique.** A candidate is a sentinel match whose
/// header then validates: a known `format`, a `state` that is empty or filled,
/// a length within capacity, and the whole region inside the file. A bare
/// 32-byte constant that the optimiser left in the text section satisfies none
/// of that, because what follows it is instructions.
///
/// Refusing on two *validating* candidates is deliberate. It cannot happen
/// from one `static`, so it would mean something about the image this code does
/// not understand, and patching the wrong one produces an artefact that runs
/// the wrong program.
fn locate(image: &[u8]) -> Result<usize, String> {
    let sentinel = unmask(SENTINEL_MASKED);
    let mut found: Option<usize> = None;
    let mut candidates = 0usize;
    for (i, w) in image.windows(sentinel.len()).enumerate() {
        if w != sentinel {
            continue;
        }
        candidates += 1;
        if !header_validates(image, i) {
            continue;
        }
        if found.is_some() {
            return Err(format!(
                "{candidates} sentinel matches in this binary and more than one carries a \
                 valid header, so `bund2 build` cannot tell which is the payload region"
            ));
        }
        found = Some(i);
    }
    found.ok_or_else(|| {
        if candidates == 0 {
            "this binary has no payload region, so it cannot carry a program. A runtime \
             built without one is not a bundling runtime."
                .to_string()
        } else {
            // **Not "the region is absent".** That is one of three things this
            // can be, and it was the only one the message named — said of an
            // artefact whose container version was 99, which has a region and
            // says so when it is run.
            format!(
                "{candidates} sentinel match(es) in this binary and none is followed by \
                 a header this bund2 reads (container version {FORMAT}). The artefact was \
                 built by a different bund2, or it is damaged, or it has no region and \
                 the matches are constants in the code. Running it reports which."
            )
        }
    })
}

/// Does a header at `start` describe a region this runtime could read?
///
/// The whole point is to reject a sentinel that is a constant rather than a
/// region, so every field that has a small legal range is checked.
fn header_validates(image: &[u8], start: usize) -> bool {
    let Some(region) = image.get(start..) else {
        return false;
    };
    if region.len() < REGION_BYTES {
        return false;
    }
    let Some(fmt) = region.get(at::FORMAT..at::FORMAT + 4) else {
        return false;
    };
    let Ok(fmt) = <[u8; 4]>::try_from(fmt) else {
        return false;
    };
    if u32::from_le_bytes(fmt) != FORMAT {
        return false;
    }
    match region.get(at::STATE) {
        Some(&EMPTY) | Some(&FILLED) => {}
        _ => return false,
    }
    let Some(len) = region.get(at::LEN..at::LEN + 4) else {
        return false;
    };
    let Ok(len) = <[u8; 4]>::try_from(len) else {
        return false;
    };
    u32::from_le_bytes(len) as usize <= CAPACITY
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
    features: &str,
    pinned: &str,
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
    let mut nm = [0u8; SOURCE_NAME_BYTES];
    let nb = name.as_bytes();
    // **The tail, not the head.** A path longer than the field is truncated
    // from the left, because the file name carries more for a diagnostic than
    // the leading directories do.
    //
    // **Cut on a character, not inside one.** The cut was by byte, so a path
    // whose 256th byte from the end fell inside a character began with a
    // fragment, which reads back as U+FFFD in `--inspect` and in every
    // diagnostic's location. At most three bytes fewer are kept. The walk is
    // bounded by the field: a boundary is never more than three bytes on.
    let mut from = nb.len() - nb.len().min(nm.len());
    while from < nb.len() && !name.is_char_boundary(from) {
        from += 1;
    }
    let kept = nb.get(from..).unwrap_or_default();
    nm[..kept.len()].copy_from_slice(kept);
    put(image, at::SOURCE, &nm)?;
    put(image, at::FEATURES, &padded::<64>(features))?;
    put(image, at::PINNED, &padded::<256>(pinned))?;
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

    /// **The region is identified whatever the optimiser did with the
    /// sentinel** — the blocker a release build found.
    ///
    /// This test used to assert the plain bytes occurred exactly once, and it
    /// passed in debug while release builds carried two matches and refused
    /// every bundle. What is asserted now is the property `locate` actually
    /// needs: exactly one candidate whose header validates, however many
    /// sentinel matches the image contains.
    #[test]
    fn the_region_is_identified_however_many_sentinels_the_image_has() {
        let exe = std::env::current_exe().expect("a test binary has a path");
        let image = std::fs::read(&exe).expect("and it is readable");
        let s = unmask(SENTINEL_MASKED);
        let matches = image.windows(s.len()).filter(|w| *w == s).count();
        let validating = (0..image.len().saturating_sub(s.len()))
            .filter(|i| image[*i..*i + s.len()] == s && header_validates(&image, *i))
            .count();
        assert!(matches >= 1, "the region's own sentinel is in the image");
        assert_eq!(
            validating, 1,
            "exactly one candidate may carry a valid header, out of {matches} \
             sentinel match(es); a constant left in the text section carries none"
        );
        assert!(locate(&image).is_ok(), "so locate succeeds");
    }

    /// **Two candidates that both validate is a refusal, not a guess.**
    ///
    /// `locate` identifies the region structurally because release builds leave
    /// a second copy of the sentinel in the text section. That fix rests on a
    /// bare constant never carrying a valid header — so this plants a whole
    /// header that does, and checks the answer is a refusal rather than a coin
    /// toss. Patching the wrong candidate produces an artefact that runs the
    /// wrong program, which no later check would catch.
    #[test]
    fn two_validating_candidates_are_refused() {
        let exe = std::env::current_exe().expect("a test binary has a path");
        let mut image = std::fs::read(&exe).expect("and it is readable");
        let start = locate(&image).expect("the real region");
        assert!(header_validates(&image, start));

        // A second, complete region: the real header plus a full-size body, so
        // it satisfies every check `header_validates` makes.
        let clone: Vec<u8> = image
            .get(start..start + REGION_BYTES)
            .expect("the region is whole")
            .to_vec();
        image.extend_from_slice(&clone);

        let err = locate(&image).expect_err("two valid candidates must refuse");
        assert!(
            err.contains("more than one"),
            "and say why, rather than picking: {err}"
        );
    }

    /// A round trip through the region a builder writes and a runtime reads.
    #[test]
    fn a_written_program_reads_back() {
        let exe = std::env::current_exe().expect("a test binary has a path");
        let mut image = std::fs::read(&exe).expect("and it is readable");
        let before = image.len();
        write_into(
            &mut image,
            "1 2 + println\n",
            "/tmp/x.bund",
            FLAG_NOEVAL,
            "9.9.9",
            "jit",
            "abc123 reference/Bund",
        )
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

        // The two fields the RFC promised and the container lacked until now.
        let seen = inspect(&image).expect("the artefact describes itself");
        assert_eq!(seen.features, "jit");
        assert_eq!(seen.pinned, "abc123 reference/Bund");
        assert_eq!(seen.bund2_version, "9.9.9");
        assert!(seen.carries_program && seen.noeval && !seen.noio);
        assert_eq!(seen.used, "1 2 + println\n".len());
        assert_eq!(seen.capacity, CAPACITY);
    }

    /// A program over capacity is refused, and **nothing is written** — the
    /// error names both numbers, so a build failure says what to do about it.
    #[test]
    fn a_program_over_capacity_is_refused_and_writes_nothing() {
        let exe = std::env::current_exe().expect("a test binary has a path");
        let mut image = std::fs::read(&exe).expect("and it is readable");
        let untouched = image.clone();
        let big = "x".repeat(CAPACITY + 1);
        let err = write_into(&mut image, &big, "big.bund", 0, "1.0", "", "")
            .expect_err("refused");
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
