//! RFC-0015 §S10: sequences, as STRINGs (§S2).
//!
//! **Operands.** A sequence is a STRING; anything else is E101, and so is an
//! alphabet, a table or a `k` the word does not know. Positions in a refusal
//! count from 1, as `at` does and as the family's other refusals do;
//! coordinates in an answer (`bio.orfs`, `bio.pwm.scan`) count from 0, end
//! exclusive, as §S10 states.
//!
//! **Which characters a word reads**, upper or lower case alike:
//!
//! - `bio.complement` and `bio.revcomp`: the IUPAC codes, ACGT and
//!   RYSWKMBDHVN; anything else, U and `-` among them, is E109;
//! - `bio.translate`, `bio.orfs`, `bio.gc` and `bio.pwm.scan`: the IUPAC
//!   codes and U, which reads as T; anything else is E109. An ambiguous code
//!   is never guessed at: its codon is `X`, it is no base to `bio.gc`, and a
//!   window holding one is not scored;
//! - `bio.pwm`: A, C, G and T, E109 otherwise;
//! - `bio.transcribe`, `bio.backtranscribe`, `bio.kmers`, `bio.count`,
//!   `bio.hamming` and `bio.consensus`: any character.
//!
//! **Case.** The words that rewrite a sequence keep each character's case;
//! the words that count (`bio.kmers`, `bio.count`, `bio.consensus`) fold to
//! upper case first, so a soft-masked stretch counts with the rest, and
//! `bio.translate` answers upper case. `bio.hamming` compares characters as
//! `string.distance.hamming` does, case and all.
//!
//! **The genetic codes** are [`CODES`], transcribed from NCBI's `gc.prt`,
//! Version 4.6, which is committed beside the fixtures; criterion 14 compares
//! every codon of every table with that file.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::{BundValue, LIST, STRING};

use super::infer::table;
use super::operand::{WIDTH, domain, operands, real};
use super::refuse::Refusal;

/// NCBI's genetic codes: each table's id and its `ncbieaa` row, the amino
/// acid of each codon with the bases in the order T, C, A, G and the first
/// base slowest (`gc.prt`, Version 4.6).
pub(crate) const CODES: [(i64, &str); 27] = [
    // Standard
    (
        1,
        "FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Vertebrate Mitochondrial
    (
        2,
        "FFLLSSSSYY**CCWWLLLLPPPPHHQQRRRRIIMMTTTTNNKKSS**VVVVAAAADDEEGGGG",
    ),
    // Yeast Mitochondrial
    (
        3,
        "FFLLSSSSYY**CCWWTTTTPPPPHHQQRRRRIIMMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Mold, Protozoan and Coelenterate Mitochondrial; Mycoplasma; Spiroplasma
    (
        4,
        "FFLLSSSSYY**CCWWLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Invertebrate Mitochondrial
    (
        5,
        "FFLLSSSSYY**CCWWLLLLPPPPHHQQRRRRIIMMTTTTNNKKSSSSVVVVAAAADDEEGGGG",
    ),
    // Ciliate, Dasycladacean and Hexamita Nuclear
    (
        6,
        "FFLLSSSSYYQQCC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Echinoderm and Flatworm Mitochondrial
    (
        9,
        "FFLLSSSSYY**CCWWLLLLPPPPHHQQRRRRIIIMTTTTNNNKSSSSVVVVAAAADDEEGGGG",
    ),
    // Euplotid Nuclear
    (
        10,
        "FFLLSSSSYY**CCCWLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Bacterial, Archaeal and Plant Plastid
    (
        11,
        "FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Alternative Yeast Nuclear
    (
        12,
        "FFLLSSSSYY**CC*WLLLSPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Ascidian Mitochondrial
    (
        13,
        "FFLLSSSSYY**CCWWLLLLPPPPHHQQRRRRIIMMTTTTNNKKSSGGVVVVAAAADDEEGGGG",
    ),
    // Alternative Flatworm Mitochondrial
    (
        14,
        "FFLLSSSSYYY*CCWWLLLLPPPPHHQQRRRRIIIMTTTTNNNKSSSSVVVVAAAADDEEGGGG",
    ),
    // Blepharisma Macronuclear
    (
        15,
        "FFLLSSSSYY*QCC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Chlorophycean Mitochondrial
    (
        16,
        "FFLLSSSSYY*LCC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Trematode Mitochondrial
    (
        21,
        "FFLLSSSSYY**CCWWLLLLPPPPHHQQRRRRIIMMTTTTNNNKSSSSVVVVAAAADDEEGGGG",
    ),
    // Scenedesmus obliquus Mitochondrial
    (
        22,
        "FFLLSS*SYY*LCC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Thraustochytrium Mitochondrial
    (
        23,
        "FF*LSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Rhabdopleuridae Mitochondrial
    (
        24,
        "FFLLSSSSYY**CCWWLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSSKVVVVAAAADDEEGGGG",
    ),
    // Candidate Division SR1 and Gracilibacteria
    (
        25,
        "FFLLSSSSYY**CCGWLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Pachysolen tannophilus Nuclear
    (
        26,
        "FFLLSSSSYY**CC*WLLLAPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Karyorelict Nuclear
    (
        27,
        "FFLLSSSSYYQQCCWWLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Condylostoma Nuclear
    (
        28,
        "FFLLSSSSYYQQCCWWLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Mesodinium Nuclear
    (
        29,
        "FFLLSSSSYYYYCC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Peritrich Nuclear
    (
        30,
        "FFLLSSSSYYEECC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Blastocrithidia Nuclear
    (
        31,
        "FFLLSSSSYYEECCWWLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Balanophoraceae Plastid
    (
        32,
        "FFLLSSSSYY*WCC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG",
    ),
    // Cephalodiscidae Mitochondrial
    (
        33,
        "FFLLSSSSYYY*CCWWLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSSKVVVVAAAADDEEGGGG",
    ),
];

/// The largest `k` `bio.kmers` counts (§S10).
const KMER_CAP: i64 = 32;

/// ATG's index in a table: A is 2, T 0 and G 3 in the order T, C, A, G.
const ATG: usize = 16 * 2 + 3;

// Operands.

/// A STRING operand's characters, or E101.
fn chars(word: &str, operand: &str, v: &BundValue) -> Result<Vec<char>, Error> {
    match v.as_str() {
        Some(s) if v.dt() == STRING => Ok(s.chars().collect()),
        _ => Err(domain(word, operand, v.summary(WIDTH))),
    }
}

/// A LIST of STRINGs, each as its characters, of one length (E102); an
/// empty LIST, or an element that is not a STRING, is E101.
fn aligned(word: &str, v: &BundValue) -> Result<Vec<Vec<char>>, Error> {
    let items = match v.as_list() {
        Some(items) if v.dt() == LIST && !items.is_empty() => items,
        _ => return Err(domain(word, "sequences", v.summary(WIDTH))),
    };
    let rows = items
        .iter()
        .map(|item| chars(word, "sequences", item))
        .collect::<Result<Vec<_>, _>>()?;
    let n = rows[0].len();
    if let Some(r) = rows.iter().find(|r| r.len() != n) {
        return Err(disagree(word, n, r.len()));
    }
    Ok(rows)
}

fn disagree(word: &str, n: usize, m: usize) -> Error {
    Refusal::Disagree {
        what: "lengths".into(),
        n,
        m,
    }
    .error(word)
}

/// E109, `i` counted from 0.
fn alphabet(word: &str, ch: char, i: usize, name: &str) -> Error {
    Refusal::Alphabet {
        ch,
        pos: i + 1,
        alphabet: name.into(),
    }
    .error(word)
}

/// A nucleotide's index in the order T, C, A, G, `None` for an ambiguous
/// IUPAC code, or E109. U reads as T.
fn bases(word: &str, s: &[char]) -> Result<Vec<Option<usize>>, Error> {
    s.iter()
        .enumerate()
        .map(|(i, c)| match c.to_ascii_uppercase() {
            'T' | 'U' => Ok(Some(0)),
            'C' => Ok(Some(1)),
            'A' => Ok(Some(2)),
            'G' => Ok(Some(3)),
            'R' | 'Y' | 'S' | 'W' | 'K' | 'M' | 'B' | 'D' | 'H' | 'V' | 'N' => Ok(None),
            _ => Err(alphabet(word, *c, i, "iupac")),
        })
        .collect()
}

/// A codon's index in a table, or `None` when a base is ambiguous.
fn codon(b: &[Option<usize>]) -> Option<usize> {
    match b {
        [Some(x), Some(y), Some(z)] => Some(16 * x + 4 * y + z),
        _ => None,
    }
}

/// The amino acid of each whole codon, `X` for an ambiguous one.
fn protein(b: &[Option<usize>], code: &[u8]) -> String {
    b.chunks_exact(3)
        .map(|c| codon(c).map_or('X', |k| char::from(code[k])))
        .collect()
}

/// A table by its NCBI id, or E101.
fn code_of(word: &str, v: &BundValue) -> Result<&'static [u8], Error> {
    match v.unboxed() {
        BundValue::Int(n, _) => CODES
            .iter()
            .find(|(id, _)| id == n)
            .map(|(_, aa)| aa.as_bytes())
            .ok_or_else(|| domain(word, "table", v.summary(WIDTH))),
        _ => Err(domain(word, "table", v.summary(WIDTH))),
    }
}

/// A non-negative INTEGER operand, or E101.
fn count(word: &str, operand: &str, v: &BundValue) -> Result<usize, Error> {
    match v.unboxed() {
        BundValue::Int(n, _) if *n >= 0 => {
            usize::try_from(*n).map_err(|_| domain(word, operand, v.summary(WIDTH)))
        }
        _ => Err(domain(word, operand, v.summary(WIDTH))),
    }
}

/// Counts as a MAP of INTEGERs.
fn counts(m: BTreeMap<String, usize>) -> BundValue {
    BundValue::map(
        m.into_iter()
            .map(|(k, n)| (k, BundValue::int(i64::try_from(n).unwrap_or(i64::MAX))))
            .collect(),
    )
}

fn int(n: usize) -> BundValue {
    BundValue::int(i64::try_from(n).unwrap_or(i64::MAX))
}

// The words.

fn bio_valid(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.valid?";
    let v = operands(vm, W, 2)?;
    let s = chars(W, "s", &v[0])?;
    let letters = match v[1].as_str().filter(|_| v[1].dt() == STRING).as_deref() {
        Some("dna") => "ACGT",
        Some("rna") => "ACGU",
        Some("protein") => "ACDEFGHIKLMNPQRSTVWY*",
        Some("iupac") => "ACGTRYSWKMBDHVN",
        _ => return Err(domain(W, "alphabet", v[1].summary(WIDTH))),
    };
    let ok = s.iter().all(|c| letters.contains(c.to_ascii_uppercase()));
    vm.push(BundValue::boolean(ok));
    Ok(())
}

/// The IUPAC complement, keeping case, or E109.
fn complemented(word: &str, s: &[char]) -> Result<Vec<char>, Error> {
    s.iter()
        .enumerate()
        .map(|(i, &c)| {
            let k = match c.to_ascii_uppercase() {
                'A' => 'T',
                'T' => 'A',
                'C' => 'G',
                'G' => 'C',
                'R' => 'Y',
                'Y' => 'R',
                'S' => 'S',
                'W' => 'W',
                'K' => 'M',
                'M' => 'K',
                'B' => 'V',
                'V' => 'B',
                'D' => 'H',
                'H' => 'D',
                'N' => 'N',
                _ => return Err(alphabet(word, c, i, "iupac")),
            };
            Ok(if c.is_ascii_lowercase() {
                k.to_ascii_lowercase()
            } else {
                k
            })
        })
        .collect()
}

fn bio_complement(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.complement";
    let v = operands(vm, W, 1)?;
    let s = chars(W, "s", &v[0])?;
    let out: String = complemented(W, &s)?.into_iter().collect();
    vm.push(BundValue::str(out));
    Ok(())
}

fn bio_revcomp(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.revcomp";
    let v = operands(vm, W, 1)?;
    let s = chars(W, "s", &v[0])?;
    let out: String = complemented(W, &s)?.into_iter().rev().collect();
    vm.push(BundValue::str(out));
    Ok(())
}

/// Swap one letter for another in both cases, every other character kept.
fn swapped(s: &[char], from: char, to: char) -> String {
    let (fl, tl) = (from.to_ascii_lowercase(), to.to_ascii_lowercase());
    s.iter()
        .map(|&c| match c {
            c if c == from => to,
            c if c == fl => tl,
            c => c,
        })
        .collect()
}

fn bio_transcribe(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.transcribe";
    let v = operands(vm, W, 1)?;
    let s = chars(W, "s", &v[0])?;
    vm.push(BundValue::str(swapped(&s, 'T', 'U')));
    Ok(())
}

fn bio_backtranscribe(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.backtranscribe";
    let v = operands(vm, W, 1)?;
    let s = chars(W, "s", &v[0])?;
    vm.push(BundValue::str(swapped(&s, 'U', 'T')));
    Ok(())
}

fn bio_translate(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.translate";
    let v = operands(vm, W, 2)?;
    let s = chars(W, "s", &v[0])?;
    let code = code_of(W, &v[1])?;
    let b = bases(W, &s)?;
    vm.push(BundValue::str(protein(&b, code)));
    Ok(())
}

/// One strand's ORFs under `code`: the first ATG after the last stop in
/// its frame, through the next stop, as (start, end, protein), the span
/// holding the stop and the protein not.
fn strand_orfs(b: &[Option<usize>], min: usize, code: &[u8]) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    for frame in 0..3 {
        let mut open: Option<usize> = None;
        let mut i = frame;
        while i + 3 <= b.len() {
            let c = codon(&b[i..i + 3]);
            match open {
                None if c == Some(ATG) => open = Some(i),
                Some(start) if c.is_some_and(|k| code[k] == b'*') => {
                    if (i - start) / 3 >= min {
                        out.push((start, i + 3, protein(&b[start..i], code)));
                    }
                    open = None;
                }
                _ => {}
            }
            i += 3;
        }
    }
    out
}

fn bio_orfs(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.orfs";
    let v = operands(vm, W, 2)?;
    let s = chars(W, "s", &v[0])?;
    let min = count(W, "min", &v[1])?;
    let (_, code) = CODES[0];
    let code = code.as_bytes();
    let forward = bases(W, &s)?;
    // T↔A is 0↔2 and C↔G 1↔3: the complement flips the index's second bit.
    let reverse: Vec<Option<usize>> = forward.iter().rev().map(|b| b.map(|k| k ^ 2)).collect();
    let n = forward.len();
    let mut found: Vec<(usize, usize, &str, String)> = strand_orfs(&forward, min, code)
        .into_iter()
        .map(|(a, e, p)| (a, e, "+", p))
        .collect();
    found.extend(
        strand_orfs(&reverse, min, code)
            .into_iter()
            .map(|(a, e, p)| (n - e, n - a, "-", p)),
    );
    found.sort();
    let answer = found
        .into_iter()
        .map(|(start, end, strand, p)| {
            let mut m = BTreeMap::new();
            m.insert("start".to_string(), int(start));
            m.insert("end".to_string(), int(end));
            m.insert("strand".to_string(), BundValue::str(strand));
            m.insert("protein".to_string(), BundValue::str(p));
            BundValue::map(m)
        })
        .collect();
    vm.push(BundValue::list(answer));
    Ok(())
}

fn bio_gc(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.gc";
    let v = operands(vm, W, 1)?;
    let s = chars(W, "s", &v[0])?;
    let b = bases(W, &s)?;
    let known = b.iter().flatten().count();
    if known == 0 {
        return Err(domain(W, "s", "no unambiguous base".into()));
    }
    let gc = b.iter().flatten().filter(|&&k| k == 1 || k == 3).count();
    vm.push(BundValue::float(gc as f64 / known as f64));
    Ok(())
}

fn upper(s: &[char]) -> Vec<char> {
    s.iter().map(char::to_ascii_uppercase).collect()
}

fn bio_kmers(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.kmers";
    let v = operands(vm, W, 2)?;
    let s = upper(&chars(W, "s", &v[0])?);
    let k = match v[1].unboxed() {
        BundValue::Int(k, _) if (1..=KMER_CAP).contains(k) => k.unsigned_abs() as usize,
        _ => return Err(domain(W, "k", v[1].summary(WIDTH))),
    };
    let mut m = BTreeMap::new();
    for w in s.windows(k) {
        *m.entry(w.iter().collect::<String>()).or_insert(0) += 1;
    }
    vm.push(counts(m));
    Ok(())
}

fn bio_count(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.count";
    let v = operands(vm, W, 1)?;
    let mut m = BTreeMap::new();
    for c in upper(&chars(W, "s", &v[0])?) {
        *m.entry(c.to_string()).or_insert(0) += 1;
    }
    vm.push(counts(m));
    Ok(())
}

fn bio_hamming(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.hamming";
    let v = operands(vm, W, 2)?;
    let a = chars(W, "a", &v[0])?;
    let b = chars(W, "b", &v[1])?;
    if a.len() != b.len() {
        return Err(disagree(W, a.len(), b.len()));
    }
    let d = a.iter().zip(&b).filter(|(x, y)| x != y).count();
    vm.push(int(d));
    Ok(())
}

fn bio_consensus(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.consensus";
    let v = operands(vm, W, 1)?;
    let rows: Vec<Vec<char>> = aligned(W, &v[0])?.iter().map(|r| upper(r)).collect();
    let width = rows[0].len();
    let out: String = (0..width)
        .map(|j| {
            let mut m: BTreeMap<char, usize> = BTreeMap::new();
            for r in &rows {
                if r[j] != '-' {
                    *m.entry(r[j]).or_insert(0) += 1;
                }
            }
            // The first of the most frequent, in the BTreeMap's order, so a
            // tie goes to the symbol first in the alphabet.
            let best = m.values().copied().max().unwrap_or(0);
            m.into_iter()
                .find(|&(_, n)| n == best)
                .map_or('-', |(c, _)| c)
        })
        .collect();
    vm.push(BundValue::str(out));
    Ok(())
}

/// A DNA base's row in a PWM, A C G T, or E109.
fn pwm_row(word: &str, c: char, i: usize) -> Result<usize, Error> {
    match c.to_ascii_uppercase() {
        'A' => Ok(0),
        'C' => Ok(1),
        'G' => Ok(2),
        'T' => Ok(3),
        _ => Err(alphabet(word, c, i, "dna")),
    }
}

fn bio_pwm(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.pwm";
    let v = operands(vm, W, 1)?;
    let sites = aligned(W, &v[0])?;
    let width = sites[0].len();
    if width == 0 {
        return Err(domain(W, "sequences", "sites of length 0".into()));
    }
    let mut c = vec![[0usize; 4]; width];
    for site in &sites {
        for (j, &ch) in site.iter().enumerate() {
            c[j][pwm_row(W, ch, j)?] += 1;
        }
    }
    // log₂ of the frequency, with 0.5 added to each count, over the uniform
    // background's 0.25.
    let n = sites.len() as f64;
    let rows = (0..4)
        .map(|r| {
            (0..width)
                .map(|j| BundValue::float(((c[j][r] as f64 + 0.5) / (n + 2.0) / 0.25).log2()))
                .collect()
        })
        .collect();
    vm.push(BundValue::matrix(rows));
    Ok(())
}

fn bio_pwm_scan(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.pwm.scan";
    let v = operands(vm, W, 3)?;
    let s = chars(W, "s", &v[0])?;
    let pwm = table(W, "pwm", &v[1])?;
    if pwm.len() != 4 {
        return Err(domain(
            W,
            "pwm",
            format!("a MATRIX of {} rows, where A C G T are 4", pwm.len()),
        ));
    }
    let width = pwm[0].len();
    if width == 0 {
        return Err(domain(W, "pwm", "an empty MATRIX".into()));
    }
    let threshold = real(W, "threshold", &v[2])?;
    let b = bases(W, &s)?;
    // T C A G, as `bases` indexes them, to the PWM's rows A C G T.
    const ROW: [usize; 4] = [3, 1, 0, 2];
    let mut out = Vec::new();
    for (pos, window) in b.windows(width).enumerate() {
        let Some(score) = window
            .iter()
            .enumerate()
            .map(|(j, k)| k.map(|k| pwm[ROW[k]][j]))
            .sum::<Option<f64>>()
        else {
            continue;
        };
        let score = super::operand::finite(W, score, || format!("the score at {pos}"))?;
        if score >= threshold {
            let mut m = BTreeMap::new();
            m.insert("pos".to_string(), int(pos));
            m.insert("score".to_string(), BundValue::float(score));
            out.push(BundValue::map(m));
        }
    }
    vm.push(BundValue::list(out));
    Ok(())
}

/// Register §S10's words, each with its `.` form (§S3).
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native(
                $name,
                $f,
                StackEffect::fixed($n, 1),
                bund2_api::WordKind::Sync,
            );
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("bio.valid?", bio_valid, 2);
    word!("bio.complement", bio_complement, 1);
    word!("bio.revcomp", bio_revcomp, 1);
    word!("bio.transcribe", bio_transcribe, 1);
    word!("bio.backtranscribe", bio_backtranscribe, 1);
    word!("bio.translate", bio_translate, 2);
    word!("bio.orfs", bio_orfs, 2);
    word!("bio.gc", bio_gc, 1);
    word!("bio.kmers", bio_kmers, 2);
    word!("bio.count", bio_count, 1);
    word!("bio.hamming", bio_hamming, 2);
    word!("bio.consensus", bio_consensus, 1);
    word!("bio.pwm", bio_pwm, 1);
    word!("bio.pwm.scan", bio_pwm_scan, 3);
}

/// §S10's words; each also has its `.` form.
pub const WORDS: [&str; 14] = [
    "bio.valid?",
    "bio.complement",
    "bio.revcomp",
    "bio.transcribe",
    "bio.backtranscribe",
    "bio.translate",
    "bio.orfs",
    "bio.gc",
    "bio.kmers",
    "bio.count",
    "bio.hamming",
    "bio.consensus",
    "bio.pwm",
    "bio.pwm.scan",
];

#[cfg(test)]
mod tests;
