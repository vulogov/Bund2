// SPDX-License-Identifier: GPL-2.0-or-later
// Contains code ported line for line from R 4.6.2's src/library/stats/src/kmns.f
// (KMNS, OPTRA and QTRAN, Algorithm AS 136) and src/library/stats/src/kmeans.c
// (kmeans_Lloyd, kmeans_MacQueen), which are GPL-2.0-or-later (D279).

//! RFC-0020 §S6: `cluster.kmeans` (`X centers opts -- MAP`), R's `kmeans`
//! (`src/library/stats/R/kmeans.R:19-145`), the word of M3.
//!
//! - **`centers`** is an INTEGER k, from 1 to the number of distinct rows of
//!   X (E101), or a k × p MATRIX of starting centres. A MATRIX with two
//!   equal rows (R's "initial centers are not distinct", `kmeans.R:109-110`)
//!   or more rows than X (`:113-114`) is E101; another column count is E102
//!   (`:121-122`). A 1 × 1 MATRIX is one centre, not k.
//! - **`opts`:** `algorithm`, `"hartigan-wong"` (the default), `"lloyd"`
//!   (R's `"Lloyd"` and `"Forgy"`, one code) or `"macqueen"`; `iter.max`, an
//!   INTEGER ≥ 1, default 10; `nstart`, an INTEGER ≥ 1, default 1, checked
//!   and then ignored with a MATRIX of centres, as R ignores it
//!   (`kmeans.R:126`; the owner's ruling, D278).
//! - **The algorithms** are `KMNS` with `OPTRA` and `QTRAN`
//!   (`kmns.f:11-447`), and `kmeans_Lloyd` and `kmeans_MacQueen`
//!   (`kmeans.c:22-145`), statement for statement, so their ties and
//!   iteration counts are R's. `BIG` is Fortran's single-precision `1.E30`
//!   widened (`kmns.f:30`), and the quick-transfer cap R's 50·m steps
//!   (`kmeans.R:28`). k = 1 runs MacQueen, as R forces it (`kmeans.R:118`),
//!   which answers the one cluster with `iter` 1. Hartigan–Wong with k equal
//!   to the number of rows is E101, as R stops (`kmns.f:34-35`,
//!   `kmeans.R:43-45`).
//! - **Starts.** An INTEGER k draws, for each of `nstart` starts, k of X's
//!   distinct rows (in first-occurrence order) by `rng.choice`'s draw
//!   ([`super::super::sample::choose`]) over the VM's generator; the least
//!   `tot.withinss` wins, the earliest on a tie (`kmeans.R:126-134`).
//! - **Refusals where R warns (V6):** an empty cluster is E101 under every
//!   algorithm; not converging within `iter.max`, or Hartigan–Wong's
//!   quick-transfer cap, is E110. Lloyd counts its first pass, which only
//!   assigns, so Lloyd at `iter.max` 1 is always E110, as R always warns.
//! - **Overflow.** X's column sums and squared ranges, with any given
//!   centres, are checked before the work: n·max|x| or n·Σ range² that
//!   overflows a FLOAT is E114, so no sum the algorithms form can.
//! - **Work:** n·k·p·`iter.max`·`nstart` above 10¹⁰ is E111 before any
//!   draw, `nstart` counting 1 with a MATRIX of centres.
//! - **Answers,** under R's names: `cluster` (a LIST of INTEGERs 1 … k,
//!   cluster j the one started at centre j), `centers` (k × p), `totss`,
//!   `withinss`, `tot.withinss`, `betweenss` (`totss` − `tot.withinss`),
//!   `size` and `iter`.

use std::collections::{BTreeMap, HashMap};

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, INTEGER, MATRIX};
use nalgebra::DMatrix;

use super::super::linalg::matrix;
use super::super::mva::{int, integer, opts_map, unknown};
use super::super::operand::{WIDTH, domain, operands};
use super::super::refuse::Refusal;
use super::super::rng::with_rng;
use super::super::sample::choose;
use super::{overflow, string};

/// Fortran's `1.E30`, a REAL widened to DOUBLE PRECISION (`kmns.f:30`).
pub(super) const BIG: f64 = 1.000_000_015_047_466_2e30;

/// `cluster.kmeans`'s cap on n·k·p·`iter.max`·`nstart` (§S6, D39).
const WORK_CAP: u64 = 10_000_000_000;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Algorithm {
    HartiganWong,
    Lloyd,
    MacQueen,
}

/// One run's answer: clusters 1-based, centres k × p by rows.
#[derive(Debug)]
pub(super) struct Run {
    pub cluster: Vec<usize>,
    pub centers: Vec<Vec<f64>>,
    pub wss: Vec<f64>,
    pub size: Vec<i64>,
    pub iter: usize,
    /// R's `ifault`: 0, 1 (an empty cluster), 2 (no convergence) or 4
    /// (the quick-transfer cap); 3 is refused before the run.
    pub ifault: u8,
}

// Hartigan–Wong, `kmns.f`. Arrays are 1-based, as Fortran's: index 0 is
// unused, so every subscript reads as the source's.

struct Hw<'a> {
    a: &'a [Vec<f64>],
    m: usize,
    n: usize,
    c: Vec<Vec<f64>>,
    k: usize,
    ic1: Vec<usize>,
    ic2: Vec<usize>,
    nc: Vec<i64>,
    an1: Vec<f64>,
    an2: Vec<f64>,
    ncp: Vec<i64>,
    d: Vec<f64>,
    itran: Vec<i64>,
    live: Vec<i64>,
}

fn idx(i: usize) -> i64 {
    i64::try_from(i).unwrap_or(i64::MAX)
}

/// `KMNS` (`kmns.f:11-201`). `a` is X by rows, 1-based with a zero row
/// and column; `c` the centres likewise.
#[expect(
    clippy::needless_range_loop,
    reason = "KMNS's 1-based index form, statement for statement"
)]
fn kmns(a: &[Vec<f64>], m: usize, n: usize, c: Vec<Vec<f64>>, k: usize, iter: usize) -> Run {
    let mut imaxqtr = i64::from(i32::MAX).min(50 * idx(m));
    let failed = |ifault| Run {
        cluster: Vec::new(),
        centers: Vec::new(),
        wss: Vec::new(),
        size: Vec::new(),
        iter: 0,
        ifault,
    };
    if k <= 1 || k >= m {
        return failed(3);
    }
    let mut s = Hw {
        a,
        m,
        n,
        c,
        k,
        ic1: vec![0; m + 1],
        ic2: vec![0; m + 1],
        nc: vec![0; k + 1],
        an1: vec![0.0; k + 1],
        an2: vec![0.0; k + 1],
        ncp: vec![0; k + 1],
        d: vec![0.0; m + 1],
        itran: vec![0; k + 1],
        live: vec![0; k + 1],
    };
    let mut dt = [0.0f64; 3];
    for i in 1..=m {
        s.ic1[i] = 1;
        s.ic2[i] = 2;
        for il in 1..=2 {
            dt[il] = 0.0;
            for j in 1..=n {
                let da = a[i][j] - s.c[il][j];
                dt[il] += da * da;
            }
        }
        if dt[1] > dt[2] {
            s.ic1[i] = 2;
            s.ic2[i] = 1;
            dt.swap(1, 2);
        }
        'l: for l in 3..=k {
            let mut db = 0.0;
            for j in 1..=n {
                let dc = a[i][j] - s.c[l][j];
                db += dc * dc;
                if db >= dt[2] {
                    continue 'l;
                }
            }
            if db >= dt[1] {
                dt[2] = db;
                s.ic2[i] = l;
            } else {
                dt[2] = dt[1];
                s.ic2[i] = s.ic1[i];
                dt[1] = db;
                s.ic1[i] = l;
            }
        }
    }
    for l in 1..=k {
        s.nc[l] = 0;
        for j in 1..=n {
            s.c[l][j] = 0.0;
        }
    }
    for i in 1..=m {
        let l = s.ic1[i];
        s.nc[l] += 1;
        for j in 1..=n {
            s.c[l][j] += a[i][j];
        }
    }
    for l in 1..=k {
        if s.nc[l] == 0 {
            return failed(1);
        }
        let aa = s.nc[l] as f64;
        for j in 1..=n {
            s.c[l][j] /= aa;
        }
        s.an2[l] = aa / (aa + 1.0);
        s.an1[l] = BIG;
        if aa > 1.0 {
            s.an1[l] = aa / (aa - 1.0);
        }
        s.itran[l] = 1;
        s.ncp[l] = -1;
    }

    let mut indx: i64 = 0;
    let mut ifault = 2;
    // A DO loop that runs out leaves its index at ITER + 1 (`:124`, `:171`).
    let mut ij_out = iter + 1;
    for ij in 1..=iter {
        optra(&mut s, &mut indx);
        if indx == idx(m) {
            ifault = 0;
            ij_out = ij;
            break;
        }
        qtran(&mut s, &mut indx, &mut imaxqtr);
        if imaxqtr < 0 {
            ifault = 4;
            ij_out = ij;
            break;
        }
        if k == 2 {
            ifault = 0;
            ij_out = ij;
            break;
        }
        for l in 1..=k {
            s.ncp[l] = 0;
        }
    }

    // `:175-198`: the centres afresh, and the sums of squares, J outermost.
    let mut wss = vec![0.0; k + 1];
    for l in 1..=k {
        wss[l] = 0.0;
        for j in 1..=n {
            s.c[l][j] = 0.0;
        }
    }
    for i in 1..=m {
        let ii = s.ic1[i];
        for j in 1..=n {
            s.c[ii][j] += a[i][j];
        }
    }
    for j in 1..=n {
        for l in 1..=k {
            s.c[l][j] /= s.nc[l] as f64;
        }
        for i in 1..=m {
            let ii = s.ic1[i];
            let da = a[i][j] - s.c[ii][j];
            wss[ii] += da * da;
        }
    }
    Run {
        cluster: s.ic1[1..].to_vec(),
        centers: s.c[1..].iter().map(|r| r[1..].to_vec()).collect(),
        wss: wss[1..].to_vec(),
        size: s.nc[1..].to_vec(),
        iter: ij_out,
        ifault,
    }
}

/// Move point `i` from cluster `l1` to `l2`, updating the centres, counts
/// and AN1/AN2 (`kmns.f:297-313`, `:421-437`).
fn transfer(s: &mut Hw, i: usize, l1: usize, l2: usize) {
    let al1 = s.nc[l1] as f64;
    let alw = al1 - 1.0;
    let al2 = s.nc[l2] as f64;
    let alt = al2 + 1.0;
    for j in 1..=s.n {
        s.c[l1][j] = (s.c[l1][j] * al1 - s.a[i][j]) / alw;
        s.c[l2][j] = (s.c[l2][j] * al2 + s.a[i][j]) / alt;
    }
    s.nc[l1] -= 1;
    s.nc[l2] += 1;
    s.an2[l1] = alw / al1;
    s.an1[l1] = BIG;
    if alw > 1.0 {
        s.an1[l1] = alw / (alw - 1.0);
    }
    s.an1[l2] = alt / al2;
    s.an2[l2] = alt / (alt + 1.0);
    s.ic1[i] = l2;
    s.ic2[i] = l1;
}

/// The squared distance from point `i` to centre `l`.
fn dist2(s: &Hw, i: usize, l: usize) -> f64 {
    let mut de = 0.0;
    for j in 1..=s.n {
        let df = s.a[i][j] - s.c[l][j];
        de += df * df;
    }
    de
}

/// `OPTRA`, the optimal-transfer stage (`kmns.f:204-329`).
fn optra(s: &mut Hw, indx: &mut i64) {
    let (m, k) = (idx(s.m), s.k);
    for l in 1..=k {
        if s.itran[l] == 1 {
            s.live[l] = m + 1;
        }
    }
    for i in 1..=s.m {
        let ii = idx(i);
        *indx += 1;
        let l1 = s.ic1[i];
        let mut l2 = s.ic2[i];
        let ll = l2;
        if s.nc[l1] != 1 {
            if s.ncp[l1] != 0 {
                s.d[i] = dist2(s, i, l1) * s.an1[l1];
            }
            let da = dist2(s, i, l2);
            let mut r2 = da * s.an2[l2];
            'l: for l in 1..=k {
                if ii >= s.live[l1] && ii >= s.live[l] || l == l1 || l == ll {
                    continue;
                }
                let rr = r2 / s.an2[l];
                let mut dc = 0.0;
                for j in 1..=s.n {
                    let dd = s.a[i][j] - s.c[l][j];
                    dc += dd * dd;
                    if dc >= rr {
                        continue 'l;
                    }
                }
                r2 = dc * s.an2[l];
                l2 = l;
            }
            if r2 >= s.d[i] {
                s.ic2[i] = l2;
            } else {
                *indx = 0;
                s.live[l1] = m + ii;
                s.live[l2] = m + ii;
                s.ncp[l1] = ii;
                s.ncp[l2] = ii;
                transfer(s, i, l1, l2);
            }
        }
        if *indx == m {
            return;
        }
    }
    for l in 1..=k {
        s.itran[l] = 0;
        s.live[l] -= m;
    }
}

/// `QTRAN`, the quick-transfer stage (`kmns.f:332-447`). Sets `imaxqtr`
/// to −1 when its step cap is reached, as R's does.
fn qtran(s: &mut Hw, indx: &mut i64, imaxqtr: &mut i64) {
    let m = idx(s.m);
    let mut icoun: i64 = 0;
    let mut istep: i64 = 0;
    loop {
        for i in 1..=s.m {
            icoun += 1;
            istep += 1;
            if istep >= *imaxqtr {
                *imaxqtr = -1;
                return;
            }
            let l1 = s.ic1[i];
            let l2 = s.ic2[i];
            if s.nc[l1] != 1 {
                if istep <= s.ncp[l1] {
                    s.d[i] = dist2(s, i, l1) * s.an1[l1];
                }
                if istep < s.ncp[l1] || istep < s.ncp[l2] {
                    let r2 = s.d[i] / s.an2[l2];
                    let mut dd = 0.0;
                    let mut moves = true;
                    for j in 1..=s.n {
                        let de = s.a[i][j] - s.c[l2][j];
                        dd += de * de;
                        if dd >= r2 {
                            moves = false;
                            break;
                        }
                    }
                    if moves {
                        icoun = 0;
                        *indx = 0;
                        s.itran[l1] = 1;
                        s.itran[l2] = 1;
                        s.ncp[l1] = istep + m;
                        s.ncp[l2] = istep + m;
                        transfer(s, i, l1, l2);
                    }
                }
            }
            if icoun == m {
                return;
            }
        }
    }
}

// Lloyd and MacQueen, `kmeans.c`, 0-based as the C is.

/// The nearest centre to `x[i]`, the first on a tie, leaving `inew` as it
/// was if no distance is below +∞, as the C's does (`kmeans.c:35-46`).
fn nearest(x: &[Vec<f64>], i: usize, cen: &[Vec<f64>], inew: &mut usize) {
    let mut best = f64::INFINITY;
    for (j, cj) in cen.iter().enumerate() {
        let mut dd = 0.0;
        for (xc, cc) in x[i].iter().zip(cj) {
            let tmp = xc - cc;
            dd += tmp * tmp;
        }
        if dd < best {
            best = dd;
            *inew = j;
        }
    }
}

/// The sums of squares, points outermost (`kmeans.c:64-71`).
fn within(x: &[Vec<f64>], cen: &[Vec<f64>], cl: &[i64]) -> Vec<f64> {
    let mut wss = vec![0.0; cen.len()];
    for (xi, ci) in x.iter().zip(cl) {
        let it = usize::try_from(ci - 1).unwrap_or(0);
        for (xc, cc) in xi.iter().zip(&cen[it]) {
            let tmp = xc - cc;
            wss[it] += tmp * tmp;
        }
    }
    wss
}

/// Every point's cluster, recounted, and the centres as their means
/// (`kmeans.c:54-60`, `:99-105`); an empty cluster's centre is 0/0, NaN.
fn centroids(x: &[Vec<f64>], cen: &mut [Vec<f64>], nc: &mut [i64], cl: &[i64]) {
    for cj in cen.iter_mut() {
        cj.iter_mut().for_each(|c| *c = 0.0);
    }
    nc.iter_mut().for_each(|c| *c = 0);
    for (xi, ci) in x.iter().zip(cl) {
        let it = usize::try_from(ci - 1).unwrap_or(0);
        nc[it] += 1;
        for (c, xc) in cen[it].iter_mut().zip(xi) {
            *c += xc;
        }
    }
    for (cj, nj) in cen.iter_mut().zip(nc.iter()) {
        cj.iter_mut().for_each(|c| *c /= *nj as f64);
    }
}

fn label(j: usize) -> i64 {
    idx(j) + 1
}

/// `kmeans_Lloyd` (`kmeans.c:22-72`).
fn lloyd(x: &[Vec<f64>], mut cen: Vec<Vec<f64>>, maxiter: usize) -> Run {
    let k = cen.len();
    let mut cl = vec![-1i64; x.len()];
    let mut nc = vec![0i64; k];
    let mut inew = 0;
    let mut iter = 0;
    while iter < maxiter {
        let mut updated = false;
        for (i, ci) in cl.iter_mut().enumerate() {
            nearest(x, i, &cen, &mut inew);
            if *ci != label(inew) {
                updated = true;
                *ci = label(inew);
            }
        }
        if !updated {
            break;
        }
        centroids(x, &mut cen, &mut nc, &cl);
        iter += 1;
    }
    finish(x, cen, cl, nc, iter + 1)
}

/// `kmeans_MacQueen` (`kmeans.c:74-145`).
fn macqueen(x: &[Vec<f64>], mut cen: Vec<Vec<f64>>, maxiter: usize) -> Run {
    let k = cen.len();
    let mut cl = vec![0i64; x.len()];
    let mut nc = vec![0i64; k];
    let mut inew = 0;
    for (i, ci) in cl.iter_mut().enumerate() {
        nearest(x, i, &cen, &mut inew);
        if *ci != label(inew) {
            *ci = label(inew);
        }
    }
    centroids(x, &mut cen, &mut nc, &cl);
    let mut iter = 0;
    while iter < maxiter {
        let mut updated = false;
        for i in 0..x.len() {
            nearest(x, i, &cen, &mut inew);
            let iold = usize::try_from(cl[i] - 1).unwrap_or(0);
            if iold != inew {
                updated = true;
                cl[i] = label(inew);
                nc[iold] -= 1;
                nc[inew] += 1;
                for c in 0..x[i].len() {
                    cen[iold][c] += (cen[iold][c] - x[i][c]) / nc[iold] as f64;
                    cen[inew][c] += (x[i][c] - cen[inew][c]) / nc[inew] as f64;
                }
            }
        }
        if !updated {
            break;
        }
        iter += 1;
    }
    finish(x, cen, cl, nc, iter + 1)
}

fn finish(x: &[Vec<f64>], cen: Vec<Vec<f64>>, cl: Vec<i64>, nc: Vec<i64>, iter: usize) -> Run {
    let wss = within(x, &cen, &cl);
    Run {
        cluster: cl
            .iter()
            .map(|c| usize::try_from(*c).unwrap_or(0))
            .collect(),
        centers: cen,
        wss,
        size: nc,
        iter,
        ifault: 0,
    }
}

/// One start from `centres`, k ≥ 1, by `algorithm`; k = 1 runs MacQueen,
/// as R forces it (`kmeans.R:118`).
pub(super) fn run_one(
    x: &[Vec<f64>],
    centres: Vec<Vec<f64>>,
    algorithm: Algorithm,
    iter_max: usize,
) -> Run {
    let k = centres.len();
    let algorithm = if k == 1 {
        Algorithm::MacQueen
    } else {
        algorithm
    };
    match algorithm {
        Algorithm::HartiganWong => {
            let n = x.first().map_or(0, Vec::len);
            let pad = |rows: &[Vec<f64>]| -> Vec<Vec<f64>> {
                std::iter::once(vec![0.0; n + 1])
                    .chain(rows.iter().map(|r| {
                        let mut v = Vec::with_capacity(n + 1);
                        v.push(0.0);
                        v.extend_from_slice(r);
                        v
                    }))
                    .collect()
            };
            kmns(&pad(x), x.len(), n, pad(&centres), k, iter_max)
        }
        Algorithm::Lloyd => lloyd(x, centres, iter_max),
        Algorithm::MacQueen => macqueen(x, centres, iter_max),
    }
}

/// A row's bits, −0 as +0, for R's `duplicated` and `unique` (whose hash
/// maps −0 to 0).
fn key(row: &[f64]) -> Vec<u64> {
    row.iter()
        .map(|x| if *x == 0.0 { 0 } else { x.to_bits() })
        .collect()
}

fn rows_of(m: &DMatrix<f64>) -> Vec<Vec<f64>> {
    (0..m.nrows())
        .map(|i| m.row(i).iter().copied().collect())
        .collect()
}

/// E114 unless n·max|v| and n·Σ range² over X's rows and `centres` are
/// finite: every sum, centre and sum of squares the algorithms form is
/// bounded by one of them.
fn bounded(word: &str, x: &[Vec<f64>], centres: &[Vec<f64>]) -> Result<(), Error> {
    let n = x.len() as f64;
    let p = x.first().map_or(0, Vec::len);
    let mut big = 0.0f64;
    let mut span2 = 0.0f64;
    for c in 0..p {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for r in x.iter().chain(centres) {
            lo = lo.min(r[c]);
            hi = hi.max(r[c]);
        }
        big = big.max(lo.abs()).max(hi.abs());
        let span = hi - lo;
        span2 += span * span;
    }
    if !(n * big).is_finite() {
        return Err(overflow(word, "the column sums of X".into()));
    }
    if !(n * span2).is_finite() {
        return Err(overflow(word, "the sums of squares of X".into()));
    }
    Ok(())
}

/// `totss`: the squares about the column means, summed by columns, as
/// `sum(scale(x, scale = FALSE)^2)` (`kmeans.R:140`).
fn totss(x: &[Vec<f64>]) -> f64 {
    let n = x.len() as f64;
    let p = x.first().map_or(0, Vec::len);
    let mut total = 0.0;
    for c in 0..p {
        let mean = x.iter().map(|r| r[c]).sum::<f64>() / n;
        for r in x {
            let d = r[c] - mean;
            total += d * d;
        }
    }
    total
}

enum Centres {
    Count(usize),
    Given(Vec<Vec<f64>>),
}

pub(super) fn cluster_kmeans(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "cluster.kmeans";
    let v = operands(vm, W, 3)?;
    let xm = matrix(W, "X", &v[0])?;
    let mut algorithm = Algorithm::HartiganWong;
    let mut iter_max = 10;
    let mut nstart = 1;
    for (k, o) in opts_map(W, &v[2])? {
        match k.as_str() {
            "algorithm" => {
                algorithm = match string(W, k, o)?.as_str() {
                    "hartigan-wong" => Algorithm::HartiganWong,
                    "lloyd" => Algorithm::Lloyd,
                    "macqueen" => Algorithm::MacQueen,
                    _ => return Err(domain(W, k, o.summary(WIDTH))),
                }
            }
            "iter.max" => iter_max = integer(W, k, o, 1, i64::MAX)?,
            "nstart" => nstart = integer(W, k, o, 1, i64::MAX)?,
            _ => return Err(unknown(W, k)),
        }
    }
    let x = rows_of(&xm);
    let (m, p) = (x.len(), xm.ncols());
    let mut distinct: Vec<usize> = Vec::new();
    let centres = match v[1].dt() {
        INTEGER => {
            let k = integer(W, "centers", &v[1], 1, i64::MAX)?;
            let mut seen = HashMap::new();
            for (i, r) in x.iter().enumerate() {
                seen.entry(key(r)).or_insert_with(|| {
                    distinct.push(i);
                });
            }
            if k > distinct.len() {
                return Err(domain(
                    W,
                    "centers",
                    format!("{k}, above X's {} distinct rows", distinct.len()),
                ));
            }
            Centres::Count(k)
        }
        MATRIX => {
            let c = rows_of(&matrix(W, "centers", &v[1])?);
            let mut seen: HashMap<Vec<u64>, usize> = HashMap::new();
            for (i, r) in c.iter().enumerate() {
                if let Some(first) = seen.insert(key(r), i) {
                    return Err(domain(
                        W,
                        "centers",
                        format!("rows {} and {} are equal", first + 1, i + 1),
                    ));
                }
            }
            if c.len() > m {
                return Err(domain(
                    W,
                    "centers",
                    format!("{} rows, above X's {m}", c.len()),
                ));
            }
            let q = c.first().map_or(0, Vec::len);
            if q != p {
                return Err(Refusal::Disagree {
                    what: "the columns of X and centers".into(),
                    n: p,
                    m: q,
                }
                .error(W));
            }
            Centres::Given(c)
        }
        _ => return Err(domain(W, "centers", v[1].summary(WIDTH))),
    };
    let (k, starts) = match &centres {
        Centres::Count(k) => (*k, nstart),
        Centres::Given(c) => (c.len(), 1),
    };
    let work = [m, k, p, iter_max, starts]
        .iter()
        .try_fold(1u128, |w, f| w.checked_mul(*f as u128))
        .unwrap_or(u128::MAX);
    if work > u128::from(WORK_CAP) {
        return Err(Refusal::Cap {
            what: format!("n·k·p·iter.max·nstart = {work}"),
            cap: WORK_CAP,
        }
        .error(W));
    }
    if algorithm == Algorithm::HartiganWong && k >= 2 && k >= m {
        return Err(domain(
            W,
            "centers",
            format!("{k} centres for {m} rows, which Hartigan-Wong refuses"),
        ));
    }
    bounded(
        W,
        &x,
        match &centres {
            Centres::Given(c) => c,
            Centres::Count(_) => &[],
        },
    )?;

    let mut best: Option<(f64, Run)> = None;
    for _ in 0..starts {
        let start = match &centres {
            Centres::Given(c) => c.clone(),
            Centres::Count(k) => {
                let mut pool = distinct.clone();
                let picked = with_rng(vm, |g| choose(g, &mut pool, *k));
                picked.into_iter().map(|i| x[i].clone()).collect()
            }
        };
        let run = run_one(&x, start, algorithm, iter_max);
        check(W, &run, algorithm, iter_max, m)?;
        let total: f64 = run.wss.iter().sum();
        if best.as_ref().is_none_or(|(b, _)| total < *b) {
            best = Some((total, run));
        }
    }
    let Some((tot, run)) = best else {
        return Err(Error::internal(
            "cluster.kmeans ran no start, though nstart is at least 1",
        ));
    };
    let totss = totss(&x);
    let mut out = BTreeMap::new();
    let ints = |xs: Vec<i64>| BundValue::list(xs.into_iter().map(BundValue::int).collect());
    let floats = |xs: Vec<f64>| BundValue::list(xs.into_iter().map(BundValue::float).collect());
    out.insert(
        "cluster".to_string(),
        BundValue::list(run.cluster.into_iter().map(int).collect()),
    );
    out.insert(
        "centers".to_string(),
        BundValue::matrix(
            run.centers
                .into_iter()
                .map(|r| r.into_iter().map(BundValue::float).collect())
                .collect(),
        ),
    );
    out.insert("totss".to_string(), BundValue::float(totss));
    out.insert("withinss".to_string(), floats(run.wss));
    out.insert("tot.withinss".to_string(), BundValue::float(tot));
    out.insert("betweenss".to_string(), BundValue::float(totss - tot));
    out.insert("size".to_string(), ints(run.size));
    out.insert("iter".to_string(), int(run.iter));
    vm.push(BundValue::map(out));
    Ok(())
}

/// R's failures, refused (V6): an empty cluster first, as R warns of it
/// first (`kmeans.R:65-69`), then the caps.
fn check(
    word: &str,
    run: &Run,
    algorithm: Algorithm,
    iter_max: usize,
    m: usize,
) -> Result<(), Error> {
    if run.ifault == 1 {
        return Err(domain(
            word,
            "centers",
            "a start that leaves a cluster empty".into(),
        ));
    }
    if let Some(j) = run.size.iter().position(|s| *s == 0) {
        return Err(domain(
            word,
            "centers",
            format!("a start that leaves cluster {} empty", j + 1),
        ));
    }
    if run.ifault == 4 {
        return Err(Refusal::NoConvergence {
            method: "Hartigan-Wong's quick-transfer stage".into(),
            cap: 50 * m as u64,
        }
        .error(word));
    }
    if run.ifault == 2 || run.iter > iter_max {
        let method = match algorithm {
            Algorithm::HartiganWong => "Hartigan-Wong",
            Algorithm::Lloyd => "Lloyd",
            Algorithm::MacQueen => "MacQueen",
        };
        return Err(Refusal::NoConvergence {
            method: format!("{method} k-means"),
            cap: iter_max as u64,
        }
        .error(word));
    }
    Ok(())
}
