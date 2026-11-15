//! RFC-0015 §S12: pairwise alignment, by Gotoh's affine-gap recurrences.
//!
//! **Three modes.** `global` is Needleman–Wunsch with end gaps charged as
//! interior ones; `semiglobal` frees the gaps at both ends of both
//! sequences; `local` is Smith–Waterman. A gap of length L costs `gap.open`
//! + (L − 1)·`gap.extend`, EMBOSS's convention.
//!
//! **Memory (D39).** An alignment of n·m cells above [`CELL_CAP`] is E111
//! before anything is allocated. `bio.align` keeps one byte of traceback a
//! cell, reserved fallibly, and three rows of scores; `bio.align.score`
//! keeps the rows alone, over the shorter sequence, so O(min(n, m)).
//!
//! **Ties** go the same way in both words, so their scores agree: within a
//! cell a match is preferred to a gap in `b`, and that to a gap in `a`.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::{BundValue, MAP, STRING};

use super::operand::{WIDTH, domain, operands};
use super::refuse::Refusal;

/// The most cells an alignment may have (§S12).
pub(crate) const CELL_CAP: u64 = 100_000_000;

/// BLOSUM62's symbols, in the order of NCBI's file.
const BLOSUM_SYMBOLS: &str = "ARNDCQEGHILKMFPSTWYVBZX*";

/// BLOSUM62, from NCBI's `blast/matrices/BLOSUM62`, which is committed
/// beside the fixtures; a test compares every cell with that file.
#[rustfmt::skip]
pub(crate) const BLOSUM62: [[i8; 24]; 24] = [
    [4, -1, -2, -2, 0, -1, -1, 0, -2, -1, -1, -1, -1, -2, -1, 1, 0, -3, -2, 0, -2, -1, 0, -4],
    [-1, 5, 0, -2, -3, 1, 0, -2, 0, -3, -2, 2, -1, -3, -2, -1, -1, -3, -2, -3, -1, 0, -1, -4],
    [-2, 0, 6, 1, -3, 0, 0, 0, 1, -3, -3, 0, -2, -3, -2, 1, 0, -4, -2, -3, 3, 0, -1, -4],
    [-2, -2, 1, 6, -3, 0, 2, -1, -1, -3, -4, -1, -3, -3, -1, 0, -1, -4, -3, -3, 4, 1, -1, -4],
    [0, -3, -3, -3, 9, -3, -4, -3, -3, -1, -1, -3, -1, -2, -3, -1, -1, -2, -2, -1, -3, -3, -2, -4],
    [-1, 1, 0, 0, -3, 5, 2, -2, 0, -3, -2, 1, 0, -3, -1, 0, -1, -2, -1, -2, 0, 3, -1, -4],
    [-1, 0, 0, 2, -4, 2, 5, -2, 0, -3, -3, 1, -2, -3, -1, 0, -1, -3, -2, -2, 1, 4, -1, -4],
    [0, -2, 0, -1, -3, -2, -2, 6, -2, -4, -4, -2, -3, -3, -2, 0, -2, -2, -3, -3, -1, -2, -1, -4],
    [-2, 0, 1, -1, -3, 0, 0, -2, 8, -3, -3, -1, -2, -1, -2, -1, -2, -2, 2, -3, 0, 0, -1, -4],
    [-1, -3, -3, -3, -1, -3, -3, -4, -3, 4, 2, -3, 1, 0, -3, -2, -1, -3, -1, 3, -3, -3, -1, -4],
    [-1, -2, -3, -4, -1, -2, -3, -4, -3, 2, 4, -2, 2, 0, -3, -2, -1, -2, -1, 1, -4, -3, -1, -4],
    [-1, 2, 0, -1, -3, 1, 1, -2, -1, -3, -2, 5, -1, -3, -1, 0, -1, -3, -2, -2, 0, 1, -1, -4],
    [-1, -1, -2, -3, -1, 0, -2, -3, -2, 1, 2, -1, 5, 0, -2, -1, -1, -1, -1, 1, -3, -1, -1, -4],
    [-2, -3, -3, -3, -2, -3, -3, -3, -1, 0, 0, -3, 0, 6, -4, -2, -2, 1, 3, -1, -3, -3, -1, -4],
    [-1, -2, -2, -1, -3, -1, -1, -2, -2, -3, -3, -1, -2, -4, 7, -1, -1, -4, -3, -2, -2, -1, -2, -4],
    [1, -1, 1, 0, -1, 0, 0, 0, -1, -2, -2, 0, -1, -2, -1, 4, 1, -3, -2, -2, 0, 0, 0, -4],
    [0, -1, 0, -1, -1, -1, -1, -2, -2, -1, -1, -1, -1, -2, -1, 1, 5, -2, -2, 0, -1, -1, 0, -4],
    [-3, -3, -4, -4, -2, -2, -3, -2, -2, -3, -2, -3, -1, 1, -4, -3, -2, 11, 2, -3, -4, -3, -2, -4],
    [-2, -2, -2, -3, -2, -1, -2, -3, 2, -1, -1, -2, -1, 3, -3, -2, -2, 2, 7, -1, -3, -2, -1, -4],
    [0, -3, -3, -3, -1, -2, -2, -3, -3, 3, 1, -2, 1, -1, -2, -2, 0, -3, -1, 4, -3, -2, -1, -4],
    [-2, -1, 3, 4, -3, 0, 1, -1, 0, -3, -4, 0, -3, -3, -2, 0, -1, -4, -3, -3, 4, 1, -1, -4],
    [-1, 0, 0, 1, -3, 3, 4, -2, 0, -3, -3, 1, -1, -3, -1, 0, -1, -3, -2, -2, 1, 4, -1, -4],
    [0, -1, -1, -1, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -2, 0, 0, -2, -1, -1, -1, -1, -1, -4],
    [-4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, 1],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Global,
    Semiglobal,
    Local,
}

/// What the words need of `opts`, and the two sequences as indices into a
/// score table.
pub(crate) struct Problem {
    mode: Mode,
    open: f64,
    extend: f64,
    /// Each sequence's characters, as given, for the answer.
    a: Vec<char>,
    b: Vec<char>,
    /// Each character's index: `a`'s into the table's rows, `b`'s into its
    /// columns.
    ai: Vec<usize>,
    bi: Vec<usize>,
    /// Scores, `rows × cols`.
    table: Vec<f64>,
    cols: usize,
    /// Whether characters fold to upper case, to compare for identity.
    fold: bool,
}

impl Problem {
    fn score(&self, i: usize, j: usize) -> f64 {
        self.table[self.ai[i] * self.cols + self.bi[j]]
    }
}

fn opt<'a>(
    word: &str,
    opts: &'a BTreeMap<String, BundValue>,
    key: &str,
) -> Result<&'a BundValue, Error> {
    opts.get(key)
        .ok_or_else(|| domain(word, "opts", format!("no {key}")))
}

fn positive(word: &str, key: &str, v: &BundValue) -> Result<f64, Error> {
    let x = match v.unboxed() {
        BundValue::Float(x, _) => *x,
        BundValue::Int(i, _) => *i as f64,
        _ => f64::NAN,
    };
    if x.is_finite() && x > 0.0 {
        Ok(x)
    } else {
        Err(domain(word, key, v.summary(WIDTH)))
    }
}

fn string(word: &str, operand: &str, v: &BundValue) -> Result<Vec<char>, Error> {
    match v.as_str() {
        Some(s) if v.dt() == STRING => Ok(s.chars().collect()),
        _ => Err(domain(word, operand, v.summary(WIDTH))),
    }
}

/// Each character's index in `symbols`, upper-cased, or E109.
fn indices(word: &str, s: &[char], symbols: &str, alphabet: &str) -> Result<Vec<usize>, Error> {
    s.iter()
        .enumerate()
        .map(|(i, &c)| {
            symbols
                .chars()
                .position(|x| x == c.to_ascii_uppercase())
                .ok_or_else(|| {
                    Refusal::Alphabet {
                        ch: c,
                        pos: i + 1,
                        alphabet: alphabet.into(),
                    }
                    .error(word)
                })
        })
        .collect()
}

/// Each sequence's characters as indices, and the score table, `rows ×
/// cols`, with its `cols`.
type Scored = (Vec<usize>, Vec<usize>, Vec<f64>, usize);

/// A program's own matrix: a MAP from two-character STRINGs to numbers,
/// read symmetrically. The table holds only the pairs `a` and `b` meet, so
/// a pair the alignment needs and the MAP lacks is refused here.
fn custom(
    word: &str,
    m: &BTreeMap<String, BundValue>,
    a: &[char],
    b: &[char],
) -> Result<Scored, Error> {
    let mut pairs: BTreeMap<(char, char), f64> = BTreeMap::new();
    for (k, v) in m {
        let cs: Vec<char> = k.chars().collect();
        let x = match v.unboxed() {
            BundValue::Float(x, _) => *x,
            BundValue::Int(i, _) => *i as f64,
            _ => f64::NAN,
        };
        if cs.len() != 2 || !x.is_finite() {
            return Err(domain(
                word,
                "matrix",
                format!("{k:?} => {}", v.summary(WIDTH)),
            ));
        }
        let (p, q) = (cs[0], cs[1]);
        if let Some(y) = pairs.get(&(q, p)).filter(|y| p != q && **y != x) {
            return Err(domain(
                word,
                "matrix",
                format!("{q}{p} is {y} and {p}{q} is {x}"),
            ));
        }
        pairs.insert((p, q), x);
    }
    let distinct = |s: &[char]| {
        let mut d: Vec<char> = s.to_vec();
        d.sort_unstable();
        d.dedup();
        d
    };
    let (ra, rb) = (distinct(a), distinct(b));
    let mut table = Vec::with_capacity(ra.len() * rb.len());
    for &p in &ra {
        for &q in &rb {
            let x = pairs
                .get(&(p, q))
                .or_else(|| pairs.get(&(q, p)))
                .ok_or_else(|| domain(word, "matrix", format!("no score for {p}{q}")))?;
            table.push(*x);
        }
    }
    let index = |s: &[char], d: &[char]| -> Vec<usize> {
        s.iter()
            .map(|c| d.binary_search(c).unwrap_or_default())
            .collect()
    };
    Ok((index(a, &ra), index(b, &rb), table, rb.len()))
}

/// Read `a b opts`, and refuse an alignment over the cap before anything
/// sized by it is allocated.
pub(crate) fn problem(word: &str, v: &[BundValue]) -> Result<Problem, Error> {
    let a = string(word, "a", &v[0])?;
    let b = string(word, "b", &v[1])?;
    let opts = match v[2].as_map() {
        Some(m) if v[2].dt() == MAP => m,
        _ => return Err(domain(word, "opts", v[2].summary(WIDTH))),
    };
    let mode_v = opt(word, opts, "mode")?;
    let mode = match mode_v.as_str().filter(|_| mode_v.dt() == STRING).as_deref() {
        Some("global") => Mode::Global,
        Some("semiglobal") => Mode::Semiglobal,
        Some("local") => Mode::Local,
        _ => return Err(domain(word, "mode", mode_v.summary(WIDTH))),
    };
    let open = positive(word, "gap.open", opt(word, opts, "gap.open")?)?;
    let extend = positive(word, "gap.extend", opt(word, opts, "gap.extend")?)?;
    let cells = (a.len() as u64).saturating_mul(b.len() as u64);
    if cells > CELL_CAP {
        return Err(Refusal::Cap {
            what: format!("an alignment of {}·{} cells", a.len(), b.len()),
            cap: CELL_CAP,
        }
        .error(word));
    }
    let matrix = opt(word, opts, "matrix")?;
    let named = matrix.as_str().filter(|_| matrix.dt() == STRING);
    let (ai, bi, table, cols, fold) = match named.as_deref() {
        Some("blosum62") => {
            let ai = indices(word, &a, BLOSUM_SYMBOLS, "blosum62")?;
            let bi = indices(word, &b, BLOSUM_SYMBOLS, "blosum62")?;
            let table = BLOSUM62.iter().flatten().map(|&x| f64::from(x)).collect();
            (ai, bi, table, 24, true)
        }
        Some("dna") => {
            let ai = indices(word, &a, "ACGT", "dna")?;
            let bi = indices(word, &b, "ACGT", "dna")?;
            let table = (0..16)
                .map(|k| if k / 4 == k % 4 { 5.0 } else { -4.0 })
                .collect();
            (ai, bi, table, 4, true)
        }
        Some(_) => return Err(domain(word, "matrix", matrix.summary(WIDTH))),
        None => match matrix.as_map() {
            Some(m) if matrix.dt() == MAP => {
                let (ai, bi, table, cols) = custom(word, m, &a, &b)?;
                (ai, bi, table, cols, false)
            }
            _ => return Err(domain(word, "matrix", matrix.summary(WIDTH))),
        },
    };
    Ok(Problem {
        mode,
        open,
        extend,
        a,
        b,
        ai,
        bi,
        table,
        cols,
        fold,
    })
}

// The recurrences. M ends in a pair, X in `a`'s character against a gap
// (a gap in `b`), Y in `b`'s against a gap in `a`.

const NEG: f64 = f64::NEG_INFINITY;

/// Where a state's value came from: a state, or (M only, local) a fresh
/// start.
const FROM_M: u8 = 0;
const FROM_X: u8 = 1;
const FROM_Y: u8 = 2;
const START: u8 = 3;

/// The best of three, the first on a tie, with where it came from.
fn best3(m: f64, x: f64, y: f64) -> (f64, u8) {
    let mut out = (m, FROM_M);
    if x > out.0 {
        out = (x, FROM_X);
    }
    if y > out.0 {
        out = (y, FROM_Y);
    }
    out
}

/// One row of the three states.
#[derive(Clone)]
struct Row {
    m: Vec<f64>,
    x: Vec<f64>,
    y: Vec<f64>,
}

impl Row {
    fn new(n: usize) -> Self {
        Row {
            m: vec![NEG; n],
            x: vec![NEG; n],
            y: vec![NEG; n],
        }
    }
}

/// The cost of a leading gap of length `k`, charged or free.
fn lead(p: &Problem, k: usize) -> f64 {
    match p.mode {
        Mode::Global => -(p.open + (k as f64 - 1.0) * p.extend),
        Mode::Semiglobal => 0.0,
        Mode::Local => NEG,
    }
}

/// Fill row `i` (≥ 1) from row `i − 1`, calling `ptr(j, byte)` for each
/// cell's traceback.
fn fill(p: &Problem, i: usize, prev: &Row, cur: &mut Row, mut ptr: impl FnMut(usize, u8)) {
    let local = p.mode == Mode::Local;
    cur.m[0] = NEG;
    cur.y[0] = NEG;
    cur.x[0] = if i == 0 { NEG } else { lead(p, i) };
    ptr(0, (if i > 1 { FROM_X } else { FROM_M }) << 2);
    for j in 1..cur.m.len() {
        let (d, mut from_m) = best3(prev.m[j - 1], prev.x[j - 1], prev.y[j - 1]);
        let mut d = d;
        if local && 0.0 > d {
            d = 0.0;
            from_m = START;
        }
        cur.m[j] = d + p.score(i - 1, j - 1);
        let (x, from_x) = best3(prev.m[j] - p.open, prev.x[j] - p.extend, prev.y[j] - p.open);
        cur.x[j] = x;
        let (y, from_y) = best3(
            cur.m[j - 1] - p.open,
            cur.x[j - 1] - p.open,
            cur.y[j - 1] - p.extend,
        );
        cur.y[j] = y;
        ptr(j, from_m | (from_x << 2) | (from_y << 4));
    }
}

/// Row 0: only gaps in `a` (state Y), and M at the origin.
fn first_row(p: &Problem, n: usize) -> Row {
    let mut r = Row::new(n);
    // Local alignments start only by a pair's START, so the origin is no
    // state to come from there.
    if p.mode != Mode::Local {
        r.m[0] = 0.0;
    }
    for j in 1..n {
        r.y[j] = lead(p, j);
    }
    r
}

/// The best end over a filled row, as the mode reads it. Global and
/// semiglobal look at the last cell of the last row (`last`), and
/// semiglobal also at each row's last cell (`edge`), so the caller offers
/// both; local at every M.
#[derive(Clone, Copy)]
struct End {
    score: f64,
    i: usize,
    j: usize,
    state: u8,
}

fn better(e: &mut Option<End>, cand: End) {
    if e.is_none_or(|e| cand.score > e.score) {
        *e = Some(cand);
    }
}

/// The three states at a cell, as an end, the first best on a tie.
fn cell_end(r: &Row, i: usize, j: usize) -> End {
    let (score, state) = best3(r.m[j], r.x[j], r.y[j]);
    End { score, i, j, state }
}

/// The score alone, rows kept over the shorter sequence.
pub(crate) fn score_only(p: &Problem) -> f64 {
    // The score is the same with the sequences swapped: the matrices are
    // symmetric and a gap costs the same in either.
    let swapped;
    let p = if p.b.len() > p.a.len() {
        swapped = Problem {
            mode: p.mode,
            open: p.open,
            extend: p.extend,
            a: p.b.clone(),
            b: p.a.clone(),
            ai: p.bi.clone(),
            bi: p.ai.clone(),
            table: transpose(&p.table, p.cols),
            cols: p.table.len().checked_div(p.cols).unwrap_or(0),
            fold: p.fold,
        };
        &swapped
    } else {
        p
    };
    let (n, m) = (p.a.len(), p.b.len());
    let mut prev = first_row(p, m + 1);
    let mut end: Option<End> = None;
    let mut consider = |r: &Row, i: usize| match p.mode {
        Mode::Global => {
            if i == n {
                better(&mut end, cell_end(r, i, m));
            }
        }
        Mode::Semiglobal => {
            if i == n {
                for j in (0..=m).rev() {
                    better(&mut end, cell_end(r, i, j));
                }
            } else {
                better(&mut end, cell_end(r, i, m));
            }
        }
        Mode::Local => {
            for j in 1..=m {
                better(
                    &mut end,
                    End {
                        score: r.m[j],
                        i,
                        j,
                        state: FROM_M,
                    },
                );
            }
        }
    };
    consider(&prev, 0);
    let mut cur = Row::new(m + 1);
    for i in 1..=n {
        fill(p, i, &prev, &mut cur, |_, _| {});
        consider(&cur, i);
        std::mem::swap(&mut prev, &mut cur);
    }
    let s = end.map_or(0.0, |e| e.score);
    if p.mode == Mode::Local { s.max(0.0) } else { s }
}

fn transpose(t: &[f64], cols: usize) -> Vec<f64> {
    let rows = t.len().checked_div(cols).unwrap_or(0);
    (0..cols)
        .flat_map(|j| (0..rows).map(move |i| t[i * cols + j]))
        .collect()
}

/// An alignment: its score, the gapped strings, and where it starts and
/// ends in each sequence.
pub(crate) struct Alignment {
    pub score: f64,
    pub a: String,
    pub b: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub identity: f64,
}

/// The alignment, with its traceback, one byte a cell.
pub(crate) fn full(word: &str, p: &Problem) -> Result<Alignment, Error> {
    let (n, m) = (p.a.len(), p.b.len());
    let w = m + 1;
    let cells = (n + 1) * w;
    let mut tb: Vec<u8> = Vec::new();
    tb.try_reserve_exact(cells).map_err(|e| {
        Error(format!(
            "{}: the host could not give {cells} bytes for the alignment's traceback: {e}",
            word.to_uppercase()
        ))
    })?;
    tb.resize(cells, 0);
    let mut prev = first_row(p, w);
    for (j, cell) in tb.iter_mut().enumerate().take(w).skip(1) {
        *cell = (if j > 1 { FROM_Y } else { FROM_M }) << 4;
    }
    let mut end: Option<End> = None;
    let consider = |end: &mut Option<End>, r: &Row, i: usize| match p.mode {
        Mode::Global => {
            if i == n {
                better(end, cell_end(r, i, m));
            }
        }
        Mode::Semiglobal => {
            if i == n {
                for j in (0..=m).rev() {
                    better(end, cell_end(r, i, j));
                }
            } else {
                better(end, cell_end(r, i, m));
            }
        }
        Mode::Local => {
            for j in 1..=m {
                better(
                    end,
                    End {
                        score: r.m[j],
                        i,
                        j,
                        state: FROM_M,
                    },
                );
            }
        }
    };
    consider(&mut end, &prev, 0);
    let mut cur = Row::new(w);
    for i in 1..=n {
        let row = &mut tb[i * w..(i + 1) * w];
        fill(p, i, &prev, &mut cur, |j, byte| row[j] = byte);
        consider(&mut end, &cur, i);
        std::mem::swap(&mut prev, &mut cur);
    }
    let end = match end {
        Some(e) if !(p.mode == Mode::Local && e.score <= 0.0) => e,
        // Local, with no pair scoring above 0: the empty alignment.
        _ => End {
            score: 0.0,
            i: 0,
            j: 0,
            state: START,
        },
    };
    let (mut ra, mut rb) = (Vec::new(), Vec::new());
    // Semiglobal's free trailing gaps: what is left of either sequence.
    if p.mode == Mode::Semiglobal {
        for c in p.a[end.i..].iter().rev() {
            ra.push(*c);
            rb.push('-');
        }
        for c in p.b[end.j..].iter().rev() {
            ra.push('-');
            rb.push(*c);
        }
    }
    let (mut i, mut j, mut state) = (end.i, end.j, end.state);
    let done = |i: usize, j: usize, state: u8| match p.mode {
        Mode::Local => state == START,
        _ => i == 0 && j == 0,
    };
    while !done(i, j, state) {
        let byte = tb[i * w + j];
        match state {
            FROM_M if i > 0 && j > 0 => {
                ra.push(p.a[i - 1]);
                rb.push(p.b[j - 1]);
                state = byte & 3;
                i -= 1;
                j -= 1;
            }
            FROM_X if i > 0 => {
                ra.push(p.a[i - 1]);
                rb.push('-');
                state = (byte >> 2) & 3;
                i -= 1;
            }
            FROM_Y if j > 0 => {
                ra.push('-');
                rb.push(p.b[j - 1]);
                state = (byte >> 4) & 3;
                j -= 1;
            }
            _ => {
                return Err(Error::internal(format!(
                    "{word}: the traceback reached state {state} at ({i}, {j}), \
                     which no recurrence writes"
                )));
            }
        }
    }
    let (start, stop) = match p.mode {
        Mode::Local => ((i, j), (end.i, end.j)),
        _ => ((0, 0), (n, m)),
    };
    ra.reverse();
    rb.reverse();
    let same = |x: char, y: char| {
        if p.fold {
            x.eq_ignore_ascii_case(&y)
        } else {
            x == y
        }
    };
    let columns = ra.len();
    let identical = ra
        .iter()
        .zip(&rb)
        .filter(|&(&x, &y)| x != '-' && y != '-' && same(x, y))
        .count();
    let identity = if columns == 0 {
        0.0
    } else {
        identical as f64 / columns as f64
    };
    Ok(Alignment {
        score: end.score,
        a: ra.into_iter().collect(),
        b: rb.into_iter().collect(),
        start,
        end: stop,
        identity,
    })
}

fn pair(x: usize, y: usize) -> BundValue {
    let i = |n: usize| BundValue::int(i64::try_from(n).unwrap_or(i64::MAX));
    BundValue::list(vec![i(x), i(y)])
}

fn bio_align(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.align";
    let v = operands(vm, W, 3)?;
    let p = problem(W, &v)?;
    let al = full(W, &p)?;
    let mut m = BTreeMap::new();
    m.insert("score".to_string(), BundValue::float(al.score));
    m.insert("a".to_string(), BundValue::str(al.a));
    m.insert("b".to_string(), BundValue::str(al.b));
    m.insert("start".to_string(), pair(al.start.0, al.start.1));
    m.insert("end".to_string(), pair(al.end.0, al.end.1));
    m.insert("identity".to_string(), BundValue::float(al.identity));
    vm.push(BundValue::map(m));
    Ok(())
}

fn bio_align_score(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "bio.align.score";
    let v = operands(vm, W, 3)?;
    let p = problem(W, &v)?;
    vm.push(BundValue::float(score_only(&p)));
    Ok(())
}

/// Register §S12's words, each with its `.` form (§S3).
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
    word!("bio.align", bio_align, 3);
    word!("bio.align.score", bio_align_score, 3);
}

/// §S12's words; each also has its `.` form.
pub const WORDS: [&str; 2] = ["bio.align", "bio.align.score"];

#[cfg(test)]
mod tests;
