// SPDX-License-Identifier: GPL-2.0-or-later
// Contains code ported line for line from R 4.4.2's src/library/stats/src/hclust.f
// (HCLUST, HCASS2), src/library/stats/src/distance.c (the six distances) and
// src/library/stats/src/hclust-utils.c (cutree), which are GPL-2.0-or-later (D279).

//! RFC-0020 §S5: distances and hierarchical clustering, the `cluster.`
//! words of M2.
//!
//! - **`cluster.dist`** (`X opts -- MATRIX`) is R's `dist` over X's rows
//!   (`src/library/stats/R/dist.R:19-43`), each method R's arithmetic
//!   (`src/library/stats/src/distance.c:42-192`). `opts` takes `method`
//!   (`"euclidean"`, the default, `"maximum"`, `"manhattan"`,
//!   `"canberra"`, `"binary"` or `"minkowski"`) and `p` (a finite number
//!   above 0, default 2, read by `"minkowski"` only, as R's). It answers the
//!   full symmetric n × n MATRIX, zero diagonal. Canberra skips a
//!   coordinate where both values are within `DBL_MIN` of zero and rescales
//!   the sum by nc/count (`distance.c:123`, `:137`); a pair with no
//!   coordinate left, where R answers NA, is E101 naming the pair. Binary
//!   answers 0 for two all-zero rows (`:168`). A distance that overflows is
//!   E114. At most 5000 rows, and n²·p at most 10¹⁰ (§S11).
//! - **`cluster.hclust`** (`D opts -- MAP`) is R's `hclust`
//!   (`src/library/stats/R/hclust.R:47-113`): `HCLUST` and `HCASS2`
//!   (`hclust.f:34-215`, `:258-333`) ported statement for statement, so its
//!   merge order on ties is R's (V7). `opts` takes `method`, one of R's
//!   eight names (default `"complete"`), and `members`, a LIST of positive
//!   finite numbers, one per object (default all 1). D passes the family's
//!   distance checks ([`super::phylo::distances`]) with a cap of 5000. Every
//!   cell must lie below R's sentinel 10³⁰⁰ (`hclust.f:47`), and below
//!   10¹⁵⁰ under `ward.D2`, which squares them first (`:65-69`); else E101
//!   naming the cell, so the nearest-neighbour search always finds an index.
//!   An update that overflows or reaches the sentinel is E114. It answers
//!   `merge` ((n − 1) × 2 INTEGERs, R's signs), `height`, `order` and
//!   `method`.
//! - **`cluster.cut`** (`tree k -- LIST`) performs the first n − k merges,
//!   k an INTEGER in 1 … n; **`cluster.cut.h`** (`tree h -- LIST`) every
//!   merge at a height ≤ h, h a finite number, and refuses an unsorted
//!   `height` (an inversion), as R's `cutree` (`cutree.R:19-50`). Both
//!   number the clusters by their first member in observation order, R's
//!   `C_cutree` (`hclust-utils.c:25-113`). The tree is checked on entry: a
//!   MAP whose `merge` is an m × 2 MATRIX of INTEGERs and whose `height` is
//!   a LIST of m finite numbers, m ≥ 1, each merge naming a singleton or an
//!   earlier step, each once; else E101 naming `tree`.
//!
//! All four are fixed, plain and `.`.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::{BundValue, LIST, MAP, MATRIX, STRING};

use super::linalg::matrix;
use super::mva::{integer, opts_map, unknown};
use super::operand::{WIDTH, domain, operands, real, whole};
use super::phylo::distances;
use super::refuse::Refusal;

/// The most objects `cluster.dist` and `cluster.hclust` read (§S11).
const OBJECTS_CAP: u64 = 5_000;

/// `cluster.dist`'s cap on n²·p (§S11).
const DIST_WORK_CAP: u64 = 10_000_000_000;

/// R's "infinity" in `hclust.f` (`:47`).
const INF: f64 = 1.0e300;

fn string(word: &str, key: &str, x: &BundValue) -> Result<String, Error> {
    match x.as_str() {
        Some(s) if x.dt() == STRING => Ok(s),
        _ => Err(domain(word, key, x.summary(WIDTH))),
    }
}

fn overflow(word: &str, what: String) -> Error {
    Refusal::Overflow { expression: what }.error(word)
}

// cluster.dist.

#[derive(Clone, Copy, PartialEq)]
enum Metric {
    Euclidean,
    Maximum,
    Manhattan,
    Canberra,
    Binary,
    Minkowski,
}

/// `R_euclidean` … `R_minkowski` on two complete rows: `None` where R
/// answers NA (Canberra with no coordinate counted).
fn distance(metric: Metric, a: &[f64], b: &[f64], p: f64) -> Option<f64> {
    let nc = a.len();
    match metric {
        Metric::Euclidean => {
            let mut dist = 0.0;
            for (x, y) in a.iter().zip(b) {
                let dev = x - y;
                dist += dev * dev;
            }
            Some(dist.sqrt())
        }
        Metric::Maximum => {
            let mut dist = -f64::MAX;
            for (x, y) in a.iter().zip(b) {
                let dev = (x - y).abs();
                if dev > dist {
                    dist = dev;
                }
            }
            Some(dist)
        }
        Metric::Manhattan => {
            let mut dist = 0.0;
            for (x, y) in a.iter().zip(b) {
                dist += (x - y).abs();
            }
            Some(dist)
        }
        Metric::Canberra => {
            let mut dist = 0.0;
            let mut count = 0usize;
            for (x, y) in a.iter().zip(b) {
                let sum = x.abs() + y.abs();
                let diff = (x - y).abs();
                if sum > f64::MIN_POSITIVE || diff > f64::MIN_POSITIVE {
                    let mut dev = diff / sum;
                    let counted = if dev.is_nan() {
                        // `distance.c:126-127`: Inf/Inf is taken as 1.
                        if !diff.is_finite() && diff == sum {
                            dev = 1.0;
                            true
                        } else {
                            false
                        }
                    } else {
                        true
                    };
                    if counted {
                        dist += dev;
                        count += 1;
                    }
                }
            }
            if count == 0 {
                return None;
            }
            if count != nc {
                dist /= count as f64 / nc as f64;
            }
            Some(dist)
        }
        Metric::Binary => {
            let mut count = 0usize;
            let mut dist = 0usize;
            for (x, y) in a.iter().zip(b) {
                if *x != 0.0 || *y != 0.0 {
                    count += 1;
                    if !(*x != 0.0 && *y != 0.0) {
                        dist += 1;
                    }
                }
            }
            if count == 0 {
                return Some(0.0);
            }
            Some(dist as f64 / count as f64)
        }
        Metric::Minkowski => {
            let mut dist = 0.0;
            for (x, y) in a.iter().zip(b) {
                dist += r_pow((x - y).abs(), p);
            }
            Some(r_pow(dist, 1.0 / p))
        }
    }
}

/// R's `R_pow` for x ≥ 0 and a finite y > 0: 1 at x = 1, else `pow`.
fn r_pow(x: f64, y: f64) -> f64 {
    if x == 1.0 { 1.0 } else { x.powf(y) }
}

fn cluster_dist(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "cluster.dist";
    let v = operands(vm, W, 2)?;
    let x = matrix(W, "X", &v[0])?;
    let mut metric = Metric::Euclidean;
    let mut p = 2.0;
    for (k, o) in opts_map(W, &v[1])? {
        match k.as_str() {
            "method" => {
                metric = match string(W, k, o)?.as_str() {
                    "euclidean" => Metric::Euclidean,
                    "maximum" => Metric::Maximum,
                    "manhattan" => Metric::Manhattan,
                    "canberra" => Metric::Canberra,
                    "binary" => Metric::Binary,
                    "minkowski" => Metric::Minkowski,
                    _ => return Err(domain(W, k, o.summary(WIDTH))),
                }
            }
            "p" => {
                p = real(W, k, o)?;
                if p <= 0.0 {
                    return Err(domain(W, k, o.summary(WIDTH)));
                }
            }
            _ => return Err(unknown(W, k)),
        }
    }
    let (n, nc) = (x.nrows(), x.ncols());
    if n as u64 > OBJECTS_CAP {
        return Err(Refusal::Cap {
            what: format!("{n} rows"),
            cap: OBJECTS_CAP,
        }
        .error(W));
    }
    let work = (n as u128) * (n as u128) * (nc as u128);
    if work > u128::from(DIST_WORK_CAP) {
        return Err(Refusal::Cap {
            what: format!("n²·p = {work}, for a {n} × {nc} MATRIX,"),
            cap: DIST_WORK_CAP,
        }
        .error(W));
    }
    let rows: Vec<Vec<f64>> = (0..n).map(|i| x.row(i).iter().copied().collect()).collect();
    let mut d = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in i + 1..n {
            let Some(dij) = distance(metric, &rows[i], &rows[j], p) else {
                return Err(domain(
                    W,
                    "X",
                    format!(
                        "rows {} and {}, whose Canberra distance has no term",
                        i + 1,
                        j + 1
                    ),
                ));
            };
            if !dij.is_finite() {
                return Err(overflow(
                    W,
                    format!("the distance between rows {} and {}", i + 1, j + 1),
                ));
            }
            d[i][j] = dij;
            d[j][i] = dij;
        }
    }
    vm.push(BundValue::matrix(
        d.into_iter()
            .map(|r| r.into_iter().map(BundValue::float).collect())
            .collect(),
    ));
    Ok(())
}

// cluster.hclust.

/// R's eight methods, in `hclust.R`'s order: index + 1 is Fortran's `IOPT`
/// (`hclust.R:50-52`).
const METHODS: [&str; 8] = [
    "ward.D", "single", "complete", "average", "mcquitty", "median", "centroid", "ward.D2",
];

/// `IOFFST` (`hclust.f:219-233`): row I, column J > I of the upper half,
/// 1-based, onto `DISS`'s 1-based index.
fn ioffst(n: usize, i: usize, j: usize) -> usize {
    j + (i - 1) * n - (i * (i + 1)) / 2
}

/// The agglomerations: `IA`, `IB` and `CRIT`, 1-based (index 0 unused).
struct Agglomerations {
    ia: Vec<usize>,
    ib: Vec<usize>,
    crit: Vec<f64>,
}

/// `HCLUST` (`hclust.f:34-215`), statement for statement. `diss` is 1-based
/// (index 0 unused), every cell finite, non-negative and below the
/// sentinel, so every search finds an index. An update that is not finite
/// or reaches the sentinel is E114.
#[expect(
    clippy::needless_range_loop,
    reason = "HCLUST's 1-based index form, statement for statement"
)]
fn hclust(
    word: &str,
    n: usize,
    iopt: usize,
    membr: &mut [f64],
    diss: &mut [f64],
) -> Result<Agglomerations, Error> {
    let len = n * (n - 1) / 2;
    let mut ia = vec![0usize; n + 1];
    let mut ib = vec![0usize; n + 1];
    let mut crit = vec![0.0f64; n + 1];
    let mut nn = vec![0usize; n + 1];
    let mut disnn = vec![0.0f64; n + 1];
    let mut flag = vec![true; n + 1];
    let (mut im, mut jj, mut jm) = (0usize, 0usize, 0usize);
    let mut ncl = n;

    if iopt == 8 {
        for i in 1..=len {
            diss[i] *= diss[i];
        }
    }

    for i in 1..n {
        let mut dmin = INF;
        for j in i + 1..=n {
            let ind = ioffst(n, i, j);
            if dmin > diss[ind] {
                dmin = diss[ind];
                jm = j;
            }
        }
        nn[i] = jm;
        disnn[i] = dmin;
    }

    loop {
        // 400: the least dissimilarity, by the list of nearest neighbours.
        let mut dmin = INF;
        let mut found = false;
        for i in 1..n {
            if flag[i] && disnn[i] < dmin {
                dmin = disnn[i];
                im = i;
                jm = nn[i];
                found = true;
            }
        }
        if !found || jm == 0 || jm > n {
            return Err(Error::internal(format!(
                "{word}: hclust's nearest-neighbour search found no pair below the \
                 sentinel, though every cell and update was checked below it"
            )));
        }
        ncl -= 1;

        let i2 = im.min(jm);
        let j2 = im.max(jm);
        ia[n - ncl] = i2;
        ib[n - ncl] = j2;
        let is_ward = iopt == 1 || iopt == 8;
        if iopt == 8 {
            dmin = dmin.sqrt();
        }
        crit[n - ncl] = dmin;
        flag[j2] = false;

        dmin = INF;
        for k in 1..=n {
            if flag[k] && k != i2 {
                let ind1 = if i2 < k {
                    ioffst(n, i2, k)
                } else {
                    ioffst(n, k, i2)
                };
                let ind2 = if j2 < k {
                    ioffst(n, j2, k)
                } else {
                    ioffst(n, k, j2)
                };
                let d12 = diss[ioffst(n, i2, j2)];
                if is_ward {
                    diss[ind1] = (membr[i2] + membr[k]) * diss[ind1]
                        + (membr[j2] + membr[k]) * diss[ind2]
                        - membr[k] * d12;
                    diss[ind1] /= membr[i2] + membr[j2] + membr[k];
                } else if iopt == 2 {
                    diss[ind1] = diss[ind1].min(diss[ind2]);
                } else if iopt == 3 {
                    diss[ind1] = diss[ind1].max(diss[ind2]);
                } else if iopt == 4 {
                    diss[ind1] =
                        (membr[i2] * diss[ind1] + membr[j2] * diss[ind2]) / (membr[i2] + membr[j2]);
                } else if iopt == 5 {
                    diss[ind1] = (diss[ind1] + diss[ind2]) / 2.0;
                } else if iopt == 6 {
                    diss[ind1] = ((diss[ind1] + diss[ind2]) - d12 / 2.0) / 2.0;
                } else if iopt == 7 {
                    diss[ind1] = (membr[i2] * diss[ind1] + membr[j2] * diss[ind2]
                        - membr[i2] * membr[j2] * d12 / (membr[i2] + membr[j2]))
                        / (membr[i2] + membr[j2]);
                }
                if diss[ind1].is_nan() || diss[ind1].abs() >= INF {
                    return Err(overflow(
                        word,
                        format!(
                            "the {} update merging objects {i2} and {j2}",
                            METHODS[iopt - 1]
                        ),
                    ));
                }

                if i2 < k {
                    if diss[ind1] < dmin {
                        dmin = diss[ind1];
                        jj = k;
                    }
                } else if diss[ind1] < disnn[k] {
                    // JB's fix (`hclust.f:173-180`).
                    disnn[k] = diss[ind1];
                    nn[k] = i2;
                }
            }
        }
        membr[i2] += membr[j2];
        disnn[i2] = dmin;
        nn[i2] = jj;

        for i in 1..n {
            if flag[i] && (nn[i] == i2 || nn[i] == j2) {
                let mut dmin = INF;
                for j in i + 1..=n {
                    if flag[j] {
                        let ind = ioffst(n, i, j);
                        if diss[ind] < dmin {
                            dmin = diss[ind];
                            jj = j;
                        }
                    }
                }
                nn[i] = jj;
                disnn[i] = dmin;
            }
        }

        if ncl <= 1 {
            break;
        }
    }
    Ok(Agglomerations { ia, ib, crit })
}

/// `HCASS2` (`hclust.f:258-333`): R's `merge` rows and `order`, from the
/// 1-based `IA` and `IB`.
#[expect(
    clippy::mut_range_bound,
    reason = "HCASS2 grows LOC inside its DO J loop and leaves it at once (GOTO 171); the break keeps that"
)]
fn hcass2(
    word: &str,
    n: usize,
    ia: &[usize],
    ib: &[usize],
) -> Result<(Vec<[i64; 2]>, Vec<i64>), Error> {
    let to_i = |u: usize| i64::try_from(u).unwrap_or(i64::MAX);
    let mut iia: Vec<i64> = ia.iter().map(|&u| to_i(u)).collect();
    let mut iib: Vec<i64> = ib.iter().map(|&u| to_i(u)).collect();
    for i in 1..n.saturating_sub(1) {
        let k = to_i(ia[i].min(ib[i]));
        for j in i + 1..n {
            if to_i(ia[j]) == k {
                iia[j] = -to_i(i);
            }
            if to_i(ib[j]) == k {
                iib[j] = -to_i(i);
            }
        }
    }
    for i in 1..n {
        iia[i] = -iia[i];
        iib[i] = -iib[i];
    }
    for i in 1..n {
        if iia[i] > 0 && iib[i] < 0 {
            std::mem::swap(&mut iia[i], &mut iib[i]);
        }
        if iia[i] > 0 && iib[i] > 0 {
            let (k1, k2) = (iia[i].min(iib[i]), iia[i].max(iib[i]));
            iia[i] = k1;
            iib[i] = k2;
        }
    }

    // The order, 1-based (index 0 unused).
    let mut iorder = vec![0i64; n + 1];
    iorder[1] = iia[n - 1];
    iorder[2] = iib[n - 1];
    let mut loc = 2usize;
    for i in (1..n.saturating_sub(1)).rev() {
        let mut replaced = false;
        for j in 1..=loc {
            if iorder[j] == to_i(i) {
                iorder[j] = iia[i];
                if j == loc {
                    loc += 1;
                    iorder[loc] = iib[i];
                } else {
                    loc += 1;
                    for k in (j + 2..=loc).rev() {
                        iorder[k] = iorder[k - 1];
                    }
                    iorder[j + 1] = iib[i];
                }
                replaced = true;
                break;
            }
        }
        if !replaced {
            return Err(Error::internal(format!(
                "{word}: hcass2 found no place for step {i} in the order, though \
                 each step after the first is named by a later one"
            )));
        }
    }
    let merge = (1..n).map(|i| [iia[i], iib[i]]).collect();
    let order = (1..=n).map(|i| -iorder[i]).collect();
    Ok((merge, order))
}

fn cluster_hclust(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "cluster.hclust";
    let v = operands(vm, W, 2)?;
    let d = distances(W, "D", &v[0], OBJECTS_CAP, "objects")?;
    let n = d.len();
    let mut iopt = 3;
    let mut members: Option<Vec<f64>> = None;
    for (k, o) in opts_map(W, &v[1])? {
        match k.as_str() {
            "method" => {
                let name = string(W, k, o)?;
                iopt = match METHODS.iter().position(|m| *m == name) {
                    Some(i) => i + 1,
                    None => return Err(domain(W, k, o.summary(WIDTH))),
                };
            }
            "members" => {
                if o.dt() != LIST {
                    return Err(domain(W, k, o.summary(WIDTH)));
                }
                let m = whole(W, k, o)?;
                if let Some(i) = m.iter().position(|x| *x <= 0.0) {
                    return Err(domain(W, k, format!("{} at {}", m[i], i + 1)));
                }
                members = Some(m);
            }
            _ => return Err(unknown(W, k)),
        }
    }
    let mut membr = vec![1.0; n + 1];
    if let Some(m) = members {
        if m.len() != n {
            return Err(Refusal::Disagree {
                what: "members and objects".into(),
                n: m.len(),
                m: n,
            }
            .error(W));
        }
        membr[1..].copy_from_slice(&m);
    }
    let bound = if iopt == 8 { 1.0e150 } else { INF };
    let mut diss = vec![0.0; n * (n - 1) / 2 + 1];
    for i in 1..n {
        for j in i + 1..=n {
            let x = d[i - 1][j - 1];
            if x >= bound {
                return Err(domain(W, "D", format!("{x:e} at [{i},{j}]")));
            }
            diss[ioffst(n, i, j)] = x;
        }
    }
    let a = hclust(W, n, iopt, &mut membr, &mut diss)?;
    let (merge, order) = hcass2(W, n, &a.ia, &a.ib)?;
    let height = a.crit[1..n].to_vec();
    if let Some(i) = height.iter().position(|h| !h.is_finite()) {
        return Err(overflow(W, format!("the height of step {}", i + 1)));
    }
    let mut m = BTreeMap::new();
    m.insert(
        "merge".to_string(),
        BundValue::matrix(
            merge
                .iter()
                .map(|r| r.iter().map(|x| BundValue::int(*x)).collect())
                .collect(),
        ),
    );
    m.insert(
        "height".to_string(),
        BundValue::list(height.into_iter().map(BundValue::float).collect()),
    );
    m.insert(
        "order".to_string(),
        BundValue::list(order.into_iter().map(BundValue::int).collect()),
    );
    m.insert("method".to_string(), BundValue::str(METHODS[iopt - 1]));
    vm.push(BundValue::map(m));
    Ok(())
}

// cluster.cut and cluster.cut.h.

/// A dendrogram's `merge` rows and `height`, checked: E101 naming `tree`
/// for anything that is not one.
fn tree(word: &str, v: &BundValue) -> Result<(Vec<[i64; 2]>, Vec<f64>), Error> {
    let bad = |what: String| domain(word, "tree", what);
    let map = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(bad(v.summary(WIDTH))),
    };
    let merge_v = map
        .get("merge")
        .ok_or_else(|| bad("a MAP without merge".into()))?;
    let height_v = map
        .get("height")
        .ok_or_else(|| bad("a MAP without height".into()))?;
    let rows = match merge_v.as_matrix() {
        Some(r) if merge_v.dt() == MATRIX => r,
        _ => return Err(bad(format!("a merge of {}", merge_v.summary(WIDTH)))),
    };
    let m = rows.len();
    if m == 0 {
        return Err(bad("an empty merge".into()));
    }
    let n = i64::try_from(m + 1).unwrap_or(i64::MAX);
    let mut seen_leaf = vec![false; m + 2];
    let mut seen_step = vec![false; m + 1];
    let mut merge = Vec::with_capacity(m);
    for (s, row) in rows.iter().enumerate() {
        let step = s + 1;
        if row.len() != 2 {
            return Err(bad(format!("merge row {step} of {} cells", row.len())));
        }
        let mut pair = [0i64; 2];
        for (c, cell) in row.iter().enumerate() {
            let x = match cell.unboxed() {
                BundValue::Int(x, _) => *x,
                _ => {
                    return Err(bad(format!("{} in merge row {step}", cell.summary(WIDTH))));
                }
            };
            let ok = if x < 0 && x >= -n {
                let leaf = x.unsigned_abs() as usize;
                !std::mem::replace(&mut seen_leaf[leaf], true)
            } else if x > 0 && (x as usize) < step {
                !std::mem::replace(&mut seen_step[x as usize], true)
            } else {
                false
            };
            if !ok {
                return Err(bad(format!("{x} in merge row {step}")));
            }
            pair[c] = x;
        }
        merge.push(pair);
    }
    let height = match height_v.dt() {
        LIST => whole(word, "tree", height_v)?,
        _ => return Err(bad(format!("a height of {}", height_v.summary(WIDTH)))),
    };
    if height.len() != m {
        return Err(bad(format!("{} heights for {m} merges", height.len())));
    }
    Ok((merge, height))
}

/// `C_cutree` (`hclust-utils.c:25-113`) for one k: the clusters after the
/// first n − k merges, numbered by first member in observation order.
#[expect(
    clippy::needless_range_loop,
    reason = "C_cutree's 1-based index form, statement for statement"
)]
fn cutree(merge: &[[i64; 2]], k: usize) -> Vec<i64> {
    let n = merge.len() + 1;
    if k == n {
        return (1..=n)
            .map(|l| i64::try_from(l).unwrap_or(i64::MAX))
            .collect();
    }
    // 1-based, index 0 unused.
    let mut sing = vec![true; n + 1];
    let mut m_nr = vec![0usize; n + 1];
    let mut z = vec![0i64; n + 1];
    let mut ans = vec![0i64; n];
    for step in 1..n {
        let [m1, m2] = merge[step - 1];
        if m1 < 0 && m2 < 0 {
            let (a, b) = (m1.unsigned_abs() as usize, m2.unsigned_abs() as usize);
            m_nr[a] = step;
            m_nr[b] = step;
            sing[a] = false;
            sing[b] = false;
        } else if m1 < 0 || m2 < 0 {
            let (j, cluster) = if m1 < 0 { (m1, m2) } else { (m2, m1) };
            let j = j.unsigned_abs() as usize;
            let cluster = cluster.unsigned_abs() as usize;
            for l in 1..=n {
                if m_nr[l] == cluster {
                    m_nr[l] = step;
                }
            }
            m_nr[j] = step;
            sing[j] = false;
        } else {
            let (a, b) = (m1.unsigned_abs() as usize, m2.unsigned_abs() as usize);
            for l in 1..=n {
                if m_nr[l] == a || m_nr[l] == b {
                    m_nr[l] = step;
                }
            }
        }
        if k == n - step {
            let mut nclust = 0i64;
            for l in 1..=n {
                if sing[l] {
                    nclust += 1;
                    ans[l - 1] = nclust;
                } else {
                    if z[m_nr[l]] == 0 {
                        nclust += 1;
                        z[m_nr[l]] = nclust;
                    }
                    ans[l - 1] = z[m_nr[l]];
                }
            }
            break;
        }
    }
    ans
}

fn answer_groups(vm: &mut dyn Vm, groups: Vec<i64>) {
    vm.push(BundValue::list(
        groups.into_iter().map(BundValue::int).collect(),
    ));
}

fn cluster_cut(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "cluster.cut";
    let v = operands(vm, W, 2)?;
    let (merge, _) = tree(W, &v[0])?;
    let n = i64::try_from(merge.len() + 1).unwrap_or(i64::MAX);
    let k = integer(W, "k", &v[1], 1, n)?;
    answer_groups(vm, cutree(&merge, k));
    Ok(())
}

fn cluster_cut_h(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "cluster.cut.h";
    let v = operands(vm, W, 2)?;
    let (merge, height) = tree(W, &v[0])?;
    let h = real(W, "h", &v[1])?;
    if let Some(i) = height.windows(2).position(|w| w[1] < w[0]) {
        return Err(domain(
            W,
            "tree",
            format!("a height that falls at step {}, an inversion", i + 2),
        ));
    }
    // `cutree.R:32`: k is n + 1 less the first index whose height, Inf
    // appended, exceeds h.
    let n = merge.len() + 1;
    let performed = height.iter().take_while(|x| **x <= h).count();
    answer_groups(vm, cutree(&merge, n - performed));
    Ok(())
}

/// Register the `cluster.` words, each with its `.` form (RFC-0015 §S3).
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
    word!("cluster.dist", cluster_dist, 2);
    word!("cluster.hclust", cluster_hclust, 2);
    word!("cluster.cut", cluster_cut, 2);
    word!("cluster.cut.h", cluster_cut_h, 2);
}

/// The `cluster.` words; each also has its `.` form.
pub const WORDS: [&str; 4] = [
    "cluster.dist",
    "cluster.hclust",
    "cluster.cut",
    "cluster.cut.h",
];

#[cfg(test)]
mod tests;
