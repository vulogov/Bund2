//! RFC-0015 §S9: linear algebra on MATRIXes of numbers, by `nalgebra`.
//!
//! **Operands are MATRIXes whose cells convert to FLOAT** (§S2): a cell that
//! does not is E103, a ragged row E103, a NaN or ±∞ cell E101, an empty
//! MATRIX E101. A vector operand or answer is a LIST.
//!
//! **Every shape is checked before `nalgebra` is called**, since its
//! products, inverses and decompositions assert their shapes and panic on a
//! mismatch: inner dimensions that do not agree, or a matrix that must be
//! square and is not, are E102.
//!
//! **Iterative decompositions are bounded** (D39): the symmetric
//! eigendecomposition and the Schur form are `nalgebra`'s `try_new` with
//! `max_niter` = 1000·n and the default epsilon, and `None` is E110;
//! `linalg.dominant`'s inverse iteration stops at 100 steps, E110.
//!
//! **No NaN or ±∞ out [B12]:** an answer with a cell that is not finite is
//! E114.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::BundValue;
use nalgebra::{Complex, DMatrix, DVector, Schur, SymmetricEigen};

use super::infer::table;
use super::operand::{domain, list, operands};
use super::refuse::Refusal;
use super::special::eti;

/// The largest identity `linalg.identity` builds (§S9).
const IDENTITY_CAP: i64 = 10_000;

/// The most steps `linalg.dominant`'s inverse iteration takes (§S9).
const INVERSE_CAP: u64 = 100;

/// A MATRIX operand as an `nalgebra` matrix.
fn matrix(word: &str, operand: &str, v: &BundValue) -> Result<DMatrix<f64>, Error> {
    let t = table(word, operand, v)?;
    let (r, c) = (t.len(), t.first().map_or(0, Vec::len));
    if r == 0 || c == 0 {
        return Err(domain(word, operand, "an empty MATRIX".into()));
    }
    Ok(DMatrix::from_fn(r, c, |i, j| t[i][j]))
}

/// A LIST (or METRICS) of finite numbers.
fn vector(word: &str, operand: &str, v: &BundValue) -> Result<Vec<f64>, Error> {
    use bund2_value::{LIST, METRICS};
    match v.dt() {
        LIST | METRICS => super::operand::whole(word, operand, v),
        _ => Err(domain(word, operand, v.summary(super::operand::WIDTH))),
    }
}

fn disagree(word: &str, what: &str, n: usize, m: usize) -> Error {
    Refusal::Disagree {
        what: what.into(),
        n,
        m,
    }
    .error(word)
}

fn square(word: &str, a: &DMatrix<f64>) -> Result<usize, Error> {
    if a.nrows() == a.ncols() {
        Ok(a.nrows())
    } else {
        Err(disagree(word, "rows and columns", a.nrows(), a.ncols()))
    }
}

fn overflow(word: &str, what: &str) -> Error {
    Refusal::Overflow {
        expression: what.into(),
    }
    .error(word)
}

/// A MATRIX answer, every cell finite.
fn answer(word: &str, what: &str, m: &DMatrix<f64>) -> Result<BundValue, Error> {
    if m.iter().any(|x| !x.is_finite()) {
        return Err(overflow(word, what));
    }
    Ok(BundValue::matrix(
        (0..m.nrows())
            .map(|i| {
                (0..m.ncols())
                    .map(|j| BundValue::float(m[(i, j)]))
                    .collect()
            })
            .collect(),
    ))
}

/// A LIST answer, every element finite.
fn answer_list(word: &str, what: &str, xs: Vec<f64>) -> Result<BundValue, Error> {
    if xs.iter().any(|x| !x.is_finite()) {
        return Err(overflow(word, what));
    }
    Ok(list(xs))
}

fn max_abs(a: &DMatrix<f64>) -> f64 {
    a.iter().fold(0.0, |m, x| m.max(x.abs()))
}

// The words.

fn linalg_mul(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.mul";
    let v = operands(vm, W, 2)?;
    let a = matrix(W, "A", &v[0])?;
    let b = matrix(W, "B", &v[1])?;
    if a.ncols() != b.nrows() {
        return Err(disagree(W, "inner dimensions", a.ncols(), b.nrows()));
    }
    vm.push(answer(W, "the product", &(a * b))?);
    Ok(())
}

fn linalg_t(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.t";
    let v = operands(vm, W, 1)?;
    let a = matrix(W, "A", &v[0])?;
    vm.push(answer(W, "the transpose", &a.transpose())?);
    Ok(())
}

fn linalg_identity(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.identity";
    let v = operands(vm, W, 1)?;
    let n = match v[0].unboxed() {
        BundValue::Int(n, _) if (1..=IDENTITY_CAP).contains(n) => *n as usize,
        _ => return Err(domain(W, "n", v[0].summary(super::operand::WIDTH))),
    };
    vm.push(answer(W, "the identity", &DMatrix::identity(n, n))?);
    Ok(())
}

fn linalg_det(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.det";
    let v = operands(vm, W, 1)?;
    let a = matrix(W, "A", &v[0])?;
    square(W, &a)?;
    let d = a.lu().determinant();
    if !d.is_finite() {
        return Err(overflow(W, "the determinant"));
    }
    vm.push(BundValue::float(d));
    Ok(())
}

fn linalg_inv(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.inv";
    let v = operands(vm, W, 1)?;
    let a = matrix(W, "A", &v[0])?;
    square(W, &a)?;
    let Some(inv) = a.lu().try_inverse() else {
        return Err(Refusal::Singular.error(W));
    };
    vm.push(answer(W, "the inverse", &inv)?);
    Ok(())
}

fn linalg_solve(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.solve";
    let v = operands(vm, W, 2)?;
    let a = matrix(W, "A", &v[0])?;
    let b = vector(W, "b", &v[1])?;
    let n = square(W, &a)?;
    if b.len() != n {
        return Err(disagree(W, "lengths", n, b.len()));
    }
    let Some(x) = a.lu().solve(&DVector::from_vec(b)) else {
        return Err(Refusal::Singular.error(W));
    };
    vm.push(answer_list(W, "the solution", x.iter().copied().collect())?);
    Ok(())
}

/// The cap on an iterative decomposition of an n×n matrix (§S9).
fn niter(n: usize) -> usize {
    1000 * n
}

fn linalg_eigen_sym(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.eigen.sym";
    let v = operands(vm, W, 1)?;
    let a = matrix(W, "A", &v[0])?;
    let n = square(W, &a)?;
    let tol = 1e-12 * max_abs(&a);
    for i in 0..n {
        for j in i + 1..n {
            let by = (a[(i, j)] - a[(j, i)]).abs();
            if by > tol {
                return Err(Refusal::NotSymmetric {
                    i: i + 1,
                    j: j + 1,
                    by,
                }
                .error(W));
            }
        }
    }
    let Some(e) = SymmetricEigen::try_new(a, f64::EPSILON, niter(n)) else {
        return Err(eti(W, "the symmetric eigendecomposition", niter(n) as u64));
    };
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| e.eigenvalues[i].total_cmp(&e.eigenvalues[j]));
    let values: Vec<f64> = order.iter().map(|&i| e.eigenvalues[i]).collect();
    let vectors = DMatrix::from_fn(n, n, |r, c| e.eigenvectors[(r, order[c])]);
    let mut m = BTreeMap::new();
    m.insert(
        "values".to_string(),
        answer_list(W, "an eigenvalue", values)?,
    );
    m.insert(
        "vectors".to_string(),
        answer(W, "an eigenvector", &vectors)?,
    );
    vm.push(BundValue::map(m));
    Ok(())
}

/// Every eigenvalue, by descending modulus, then descending real and
/// imaginary part, so a conjugate pair is `+` before `−`.
fn spectrum(word: &str, a: DMatrix<f64>) -> Result<Vec<Complex<f64>>, Error> {
    let n = square(word, &a)?;
    let Some(s) = Schur::try_new(a, f64::EPSILON, niter(n)) else {
        return Err(eti(word, "the Schur decomposition", niter(n) as u64));
    };
    let mut ev: Vec<Complex<f64>> = s.complex_eigenvalues().iter().copied().collect();
    if ev.iter().any(|z| !z.re.is_finite() || !z.im.is_finite()) {
        return Err(overflow(word, "an eigenvalue"));
    }
    ev.sort_by(|x, y| {
        y.norm()
            .total_cmp(&x.norm())
            .then(y.re.total_cmp(&x.re))
            .then(y.im.total_cmp(&x.im))
    });
    Ok(ev)
}

/// How large an imaginary part may be and the eigenvalue still be real:
/// 10⁻¹² of A's Frobenius norm (§S9).
fn real_tol(a: &DMatrix<f64>) -> f64 {
    1e-12 * a.norm()
}

fn complex(z: Complex<f64>) -> Refusal {
    Refusal::Complex { re: z.re, im: z.im }
}

fn linalg_eigenvalues(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.eigenvalues";
    let v = operands(vm, W, 1)?;
    let a = matrix(W, "A", &v[0])?;
    let tol = real_tol(&a);
    let ev = spectrum(W, a)?;
    if let Some(z) = ev.iter().find(|z| z.im.abs() > tol) {
        return Err(complex(*z).error(W));
    }
    vm.push(list(ev.iter().map(|z| z.re).collect()));
    Ok(())
}

fn linalg_eigenvalues_complex(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.eigenvalues.complex";
    let v = operands(vm, W, 1)?;
    let a = matrix(W, "A", &v[0])?;
    let ev = spectrum(W, a)?;
    vm.push(BundValue::list(
        ev.iter().map(|z| list(vec![z.re, z.im])).collect(),
    ));
    Ok(())
}

fn linalg_dominant(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.dominant";
    let v = operands(vm, W, 1)?;
    let a = matrix(W, "A", &v[0])?;
    let n = square(W, &a)?;
    let tol = real_tol(&a);
    let ev = spectrum(W, a.clone())?;
    let lead = ev[0];
    if lead.im.abs() > tol {
        return Err(complex(lead).error(W));
    }
    if let Some(second) = ev.get(1)
        && (lead.norm() - second.norm()).abs() <= tol.max(1e-12 * lead.norm())
    {
        return Err(Refusal::NotUnique {
            first: lead.re,
            second: if second.im.abs() > tol {
                second.norm()
            } else {
                second.re
            },
        }
        .error(W));
    }
    let lambda = lead.re;
    // Inverse iteration on A − σI, σ a hair from λ so the factor is not
    // exactly singular; each step multiplies the wanted direction by
    // 1 / (λ − σ) and every other by at most 1 / (|μ − λ| − |λ − σ|).
    let sigma = lambda + 1e-10 * a.norm().max(1.0);
    let shifted = &a - DMatrix::identity(n, n) * sigma;
    let lu = shifted.lu();
    let mut x = DVector::from_element(n, 1.0 / (n as f64).sqrt());
    let mut converged = false;
    for _ in 0..INVERSE_CAP {
        let Some(y) = lu.solve(&x) else {
            return Err(Refusal::Singular.error(W));
        };
        let norm = y.norm();
        if !norm.is_finite() || norm == 0.0 {
            return Err(overflow(W, "the inverse iteration's vector"));
        }
        let mut y = y / norm;
        // Fix the sign, so successive iterates are comparable.
        if y.iter().sum::<f64>() < 0.0 {
            y = -y;
        }
        let moved = (&y - &x).norm();
        x = y;
        if moved <= 1e-14 {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(eti(W, "the inverse iteration", INVERSE_CAP));
    }
    let total: f64 = x.iter().sum();
    if total.abs() <= f64::EPSILON * x.norm() {
        return Err(domain(
            W,
            "A",
            "a dominant eigenvector summing to 0, which cannot be scaled to sum 1".into(),
        ));
    }
    let vector: Vec<f64> = x.iter().map(|xi| xi / total).collect();
    let mut m = BTreeMap::new();
    m.insert("value".to_string(), BundValue::float(lambda));
    m.insert(
        "vector".to_string(),
        answer_list(W, "the dominant eigenvector", vector)?,
    );
    vm.push(BundValue::map(m));
    Ok(())
}

fn linalg_power(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.power";
    let v = operands(vm, W, 2)?;
    let a = matrix(W, "A", &v[0])?;
    let n = square(W, &a)?;
    let mut k = match v[1].unboxed() {
        BundValue::Int(k, _) if *k >= 0 => k.unsigned_abs(),
        _ => return Err(domain(W, "k", v[1].summary(super::operand::WIDTH))),
    };
    let mut out = DMatrix::identity(n, n);
    let mut base = a;
    let finite = |m: &DMatrix<f64>| m.iter().all(|x| x.is_finite());
    while k > 0 {
        if k & 1 == 1 {
            out = &out * &base;
            if !finite(&out) {
                return Err(overflow(W, "a cell of the power"));
            }
        }
        k >>= 1;
        if k > 0 {
            base = &base * &base;
            if !finite(&base) {
                return Err(overflow(W, "a cell of the power"));
            }
        }
    }
    vm.push(answer(W, "a cell of the power", &out)?);
    Ok(())
}

fn linalg_mulv(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "linalg.mulv";
    let v = operands(vm, W, 2)?;
    let a = matrix(W, "A", &v[0])?;
    let x = vector(W, "v", &v[1])?;
    if a.ncols() != x.len() {
        return Err(disagree(W, "inner dimensions", a.ncols(), x.len()));
    }
    let w = a * DVector::from_vec(x);
    vm.push(answer_list(W, "the product", w.iter().copied().collect())?);
    Ok(())
}

/// Register §S9's words, each with its `.` form (§S3).
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
    word!("linalg.mul", linalg_mul, 2);
    word!("linalg.t", linalg_t, 1);
    word!("linalg.identity", linalg_identity, 1);
    word!("linalg.det", linalg_det, 1);
    word!("linalg.inv", linalg_inv, 1);
    word!("linalg.solve", linalg_solve, 2);
    word!("linalg.eigen.sym", linalg_eigen_sym, 1);
    word!("linalg.eigenvalues", linalg_eigenvalues, 1);
    word!("linalg.eigenvalues.complex", linalg_eigenvalues_complex, 1);
    word!("linalg.dominant", linalg_dominant, 1);
    word!("linalg.power", linalg_power, 2);
    word!("linalg.mulv", linalg_mulv, 2);
}

/// §S9's words; each also has its `.` form.
pub const WORDS: [&str; 12] = [
    "linalg.mul",
    "linalg.t",
    "linalg.identity",
    "linalg.det",
    "linalg.inv",
    "linalg.solve",
    "linalg.eigen.sym",
    "linalg.eigenvalues",
    "linalg.eigenvalues.complex",
    "linalg.dominant",
    "linalg.power",
    "linalg.mulv",
];

#[cfg(test)]
mod tests;
