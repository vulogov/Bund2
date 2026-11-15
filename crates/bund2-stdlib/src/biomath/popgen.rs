//! RFC-0015 §S13: population genetics.
//!
//! **Inputs.** A genotype is a STRING of two characters, one allele each,
//! read verbatim, so `A` and `a` are two alleles; anything else is E101.
//! Counts are non-negative INTEGERs (E101). An alignment is a LIST of
//! STRINGs of one length (E102).
//!
//! **Sites.** `popgen.pi`, `popgen.theta.w` and `popgen.tajima` read A, C,
//! G, T and U, upper or lower case, U as T. A site where any sequence holds
//! a gap (`-`) or an ambiguous IUPAC code is skipped; any other character
//! is E109.
//!
//! **Work (D39).** `popgen.hwe`'s exact test walks every heterozygote count
//! of the sample's parity, so it is capped at [`HWE_CAP`] individuals;
//! `popgen.wf*` cap N at 10¹² and `generations` at 10⁷ (E111), and draw one
//! binomial a generation from the VM's generator (§S8).

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::{BundValue, LIST, STRING};
use rand::distributions::Distribution;

use super::infer::chisq_p;
use super::operand::{WIDTH, domain, finite, list, operands, real};
use super::refuse::Refusal;
use super::rng::with_rng;

/// The most individuals `popgen.hwe` takes (§S13, D39).
const HWE_CAP: u64 = 100_000_000;

/// `popgen.wf*`'s caps (§S13).
const N_CAP: u64 = 1_000_000_000_000;
const GENERATION_CAP: u64 = 10_000_000;

/// How far below the observed table's probability another table may fall
/// and still count as no more probable: the recurrences answer tables that
/// are equally probable in exact arithmetic to within a few ulps of each
/// other, and the test must not split them.
const TIE: f64 = 1e-12;

fn cap(word: &str, what: &str, cap: u64) -> Error {
    Refusal::Cap {
        what: what.into(),
        cap,
    }
    .error(word)
}

/// A non-negative INTEGER, or E101.
fn count(word: &str, operand: &str, v: &BundValue) -> Result<u64, Error> {
    match v.unboxed() {
        BundValue::Int(n, _) if *n >= 0 => Ok(n.unsigned_abs()),
        _ => Err(domain(word, operand, v.summary(WIDTH))),
    }
}

/// A LIST of STRINGs, each its characters; empty is E101.
fn strings(word: &str, operand: &str, v: &BundValue) -> Result<Vec<Vec<char>>, Error> {
    let items = match v.as_list() {
        Some(items) if v.dt() == LIST && !items.is_empty() => items,
        _ => return Err(domain(word, operand, v.summary(WIDTH))),
    };
    items
        .iter()
        .map(|item| match item.as_str() {
            Some(s) if item.dt() == STRING => Ok(s.chars().collect()),
            _ => Err(domain(word, operand, item.summary(WIDTH))),
        })
        .collect()
}

/// Genotypes: two characters each, E101 otherwise.
fn genotypes(word: &str, v: &BundValue) -> Result<Vec<[char; 2]>, Error> {
    strings(word, "genotypes", v)?
        .into_iter()
        .map(|g| match g[..] {
            [x, y] => Ok([x, y]),
            _ => Err(domain(
                word,
                "genotypes",
                format!("{:?}", g.iter().collect::<String>()),
            )),
        })
        .collect()
}

/// Each allele's frequency among the genotypes' copies.
fn frequencies(gs: &[[char; 2]]) -> BTreeMap<char, f64> {
    let mut m: BTreeMap<char, usize> = BTreeMap::new();
    for g in gs {
        for c in g {
            *m.entry(*c).or_insert(0) += 1;
        }
    }
    let total = 2.0 * gs.len() as f64;
    m.into_iter().map(|(c, k)| (c, k as f64 / total)).collect()
}

fn float_map(pairs: impl IntoIterator<Item = (String, BundValue)>) -> BundValue {
    BundValue::map(pairs.into_iter().collect())
}

// The words.

fn popgen_freq(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.freq";
    let v = operands(vm, W, 1)?;
    let gs = genotypes(W, &v[0])?;
    vm.push(float_map(
        frequencies(&gs)
            .into_iter()
            .map(|(c, p)| (c.to_string(), BundValue::float(p))),
    ));
    Ok(())
}

fn popgen_het(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.het";
    let v = operands(vm, W, 1)?;
    let gs = genotypes(W, &v[0])?;
    let observed = gs.iter().filter(|g| g[0] != g[1]).count() as f64 / gs.len() as f64;
    let expected = 1.0 - frequencies(&gs).values().map(|p| p * p).sum::<f64>();
    vm.push(float_map([
        ("observed".to_string(), BundValue::float(observed)),
        ("expected".to_string(), BundValue::float(expected)),
    ]));
    Ok(())
}

/// The exact test's p: the probability of every heterozygote count no more
/// probable than the observed one, given the allele counts (Wigginton,
/// Cutler & Abecasis 2005, the appendix's recurrence from the midpoint).
fn hwe_exact(hets: u64, hom1: u64, hom2: u64) -> f64 {
    let homr = hom1.min(hom2);
    let homc = hom1.max(hom2);
    let rare = 2 * homr + hets;
    let n = hets + homr + homc;
    let mut probs = vec![0.0f64; usize::try_from(rare).unwrap_or(0) + 1];
    let mut mid = rare * (2 * n - rare) / (2 * n);
    if (rare & 1) != (mid & 1) {
        mid += 1;
    }
    let at = |h: u64| usize::try_from(h).unwrap_or(0);
    probs[at(mid)] = 1.0;
    let mut total = 1.0;
    // Downwards: one fewer heterozygote pair, one more of each homozygote.
    let (mut h, mut r, mut c) = (mid, (rare - mid) / 2, n - mid - (rare - mid) / 2);
    while h >= 2 {
        let p = probs[at(h)] * (h as f64) * ((h - 1) as f64)
            / (4.0 * ((r + 1) as f64) * ((c + 1) as f64));
        probs[at(h - 2)] = p;
        total += p;
        h -= 2;
        r += 1;
        c += 1;
    }
    // Upwards.
    let (mut h, mut r, mut c) = (mid, (rare - mid) / 2, n - mid - (rare - mid) / 2);
    while h + 2 <= rare {
        let p =
            probs[at(h)] * 4.0 * (r as f64) * (c as f64) / (((h + 2) as f64) * ((h + 1) as f64));
        probs[at(h + 2)] = p;
        total += p;
        h += 2;
        r = r.saturating_sub(1);
        c = c.saturating_sub(1);
    }
    let observed = probs[at(hets)];
    let p: f64 = probs
        .iter()
        .filter(|&&x| x <= observed * (1.0 + TIE))
        .sum::<f64>()
        / total;
    p.min(1.0)
}

fn popgen_hwe(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.hwe";
    let v = operands(vm, W, 3)?;
    let aa = count(W, "nAA", &v[0])?;
    let ab = count(W, "nAa", &v[1])?;
    let bb = count(W, "naa", &v[2])?;
    let n = aa.saturating_add(ab).saturating_add(bb);
    if n == 0 {
        return Err(domain(W, "counts", "no individuals".into()));
    }
    if n > HWE_CAP {
        return Err(cap(W, &format!("a sample of {n} individuals"), HWE_CAP));
    }
    let nf = n as f64;
    let p = (2.0 * aa as f64 + ab as f64) / (2.0 * nf);
    let q = 1.0 - p;
    if p == 0.0 || q == 0.0 {
        return Err(domain(
            W,
            "counts",
            "a monomorphic sample, whose χ² is undefined".into(),
        ));
    }
    let expected = [nf * p * p, 2.0 * nf * p * q, nf * q * q];
    let x2: f64 = [aa, ab, bb]
        .iter()
        .zip(expected)
        .map(|(&o, e)| (o as f64 - e).powi(2) / e)
        .sum();
    let x2 = finite(W, x2, || "χ²".into())?;
    let mut m = BTreeMap::new();
    m.insert("expected".to_string(), list(expected.to_vec()));
    m.insert("chisq".to_string(), BundValue::float(x2));
    m.insert(
        "p.chisq".to_string(),
        BundValue::float(chisq_p(W, x2, 1.0)?),
    );
    m.insert(
        "p.exact".to_string(),
        BundValue::float(hwe_exact(ab, aa, bb)),
    );
    vm.push(BundValue::map(m));
    Ok(())
}

fn popgen_fst(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.fst";
    let v = operands(vm, W, 4)?;
    let n1 = count(W, "n1", &v[0])?;
    let k1 = count(W, "k1", &v[1])?;
    let n2 = count(W, "n2", &v[2])?;
    let k2 = count(W, "k2", &v[3])?;
    for (name, n, val) in [("n1", n1, &v[0]), ("n2", n2, &v[2])] {
        if n < 2 {
            return Err(domain(W, name, val.summary(WIDTH)));
        }
    }
    for (name, k, n, val) in [("k1", k1, n1, &v[1]), ("k2", k2, n2, &v[3])] {
        if k > n {
            return Err(domain(W, name, val.summary(WIDTH)));
        }
    }
    let (p1, p2) = (k1 as f64 / n1 as f64, k2 as f64 / n2 as f64);
    let num = (p1 - p2).powi(2)
        - p1 * (1.0 - p1) / (n1 as f64 - 1.0)
        - p2 * (1.0 - p2) / (n2 as f64 - 1.0);
    let den = p1 * (1.0 - p2) + p2 * (1.0 - p1);
    if den == 0.0 {
        return Err(domain(
            W,
            "counts",
            "both populations fixed for one allele, so D̂ = 0".into(),
        ));
    }
    vm.push(BundValue::float(num / den));
    Ok(())
}

fn popgen_ld(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.ld";
    let v = operands(vm, W, 4)?;
    let mut c = [0.0f64; 4];
    for (i, name) in ["nAB", "nAb", "naB", "nab"].iter().enumerate() {
        c[i] = count(W, name, &v[i])? as f64;
    }
    let n: f64 = c.iter().sum();
    if n == 0.0 {
        return Err(domain(W, "counts", "no haplotypes".into()));
    }
    let p_ab = c[0] / n;
    let pa = (c[0] + c[1]) / n;
    let pb = (c[0] + c[2]) / n;
    if pa == 0.0 || pa == 1.0 || pb == 0.0 || pb == 1.0 {
        return Err(domain(
            W,
            "counts",
            "a monomorphic locus, whose D' and r² are undefined".into(),
        ));
    }
    let d = p_ab - pa * pb;
    let dmax = if d > 0.0 {
        (pa * (1.0 - pb)).min((1.0 - pa) * pb)
    } else {
        (pa * pb).min((1.0 - pa) * (1.0 - pb))
    };
    let r2 = d * d / (pa * (1.0 - pa) * pb * (1.0 - pb));
    vm.push(float_map([
        ("D".to_string(), BundValue::float(d)),
        ("D'".to_string(), BundValue::float(d / dmax)),
        ("r2".to_string(), BundValue::float(r2)),
    ]));
    Ok(())
}

/// What the three diversity words share: the number of sequences, the
/// segregating sites S, and the pairwise differences summed over sites.
struct Diversity {
    n: usize,
    segregating: usize,
    differences: f64,
}

fn diversity(word: &str, v: &BundValue, least: usize) -> Result<Diversity, Error> {
    let seqs = strings(word, "alignment", v)?;
    let width = seqs[0].len();
    if let Some(s) = seqs.iter().find(|s| s.len() != width) {
        return Err(Refusal::Disagree {
            what: "lengths".into(),
            n: width,
            m: s.len(),
        }
        .error(word));
    }
    if seqs.len() < least {
        return Err(domain(
            word,
            "alignment",
            format!("{} sequences, where {least} are needed", seqs.len()),
        ));
    }
    let mut segregating = 0;
    let mut differences = 0.0;
    'site: for j in 0..width {
        let mut counts = [0usize; 4];
        for s in &seqs {
            let c = s[j];
            match c.to_ascii_uppercase() {
                'A' => counts[0] += 1,
                'C' => counts[1] += 1,
                'G' => counts[2] += 1,
                'T' | 'U' => counts[3] += 1,
                '-' | 'R' | 'Y' | 'S' | 'W' | 'K' | 'M' | 'B' | 'D' | 'H' | 'V' | 'N' => {
                    continue 'site;
                }
                _ => {
                    return Err(Refusal::Alphabet {
                        ch: c,
                        pos: j + 1,
                        alphabet: "iupac".into(),
                    }
                    .error(word));
                }
            }
        }
        if counts.iter().filter(|&&k| k > 0).count() > 1 {
            segregating += 1;
        }
        // Pairs differing at the site: all pairs less those that agree.
        let n = seqs.len() as f64;
        let agree: f64 = counts
            .iter()
            .map(|&k| (k as f64) * (k as f64 - 1.0) / 2.0)
            .sum();
        differences += n * (n - 1.0) / 2.0 - agree;
    }
    Ok(Diversity {
        n: seqs.len(),
        segregating,
        differences,
    })
}

fn pi_of(d: &Diversity) -> f64 {
    let n = d.n as f64;
    d.differences / (n * (n - 1.0) / 2.0)
}

/// Watterson's a₁ and a₂: Σ 1/i and Σ 1/i² for i from 1 to n − 1.
fn harmonic(n: usize) -> (f64, f64) {
    (1..n).fold((0.0, 0.0), |(a1, a2), i| {
        let i = i as f64;
        (a1 + 1.0 / i, a2 + 1.0 / (i * i))
    })
}

fn popgen_pi(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.pi";
    let v = operands(vm, W, 1)?;
    let d = diversity(W, &v[0], 2)?;
    vm.push(BundValue::float(pi_of(&d)));
    Ok(())
}

fn popgen_theta_w(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.theta.w";
    let v = operands(vm, W, 1)?;
    let d = diversity(W, &v[0], 2)?;
    let (a1, _) = harmonic(d.n);
    vm.push(BundValue::float(d.segregating as f64 / a1));
    Ok(())
}

/// Tajima's D (1989), with his e₁ and e₂. Below four sequences e₁ and e₂
/// are 0, so four are needed; with no segregating site D is 0/0.
fn popgen_tajima(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.tajima";
    let v = operands(vm, W, 1)?;
    let d = diversity(W, &v[0], 4)?;
    if d.segregating == 0 {
        return Err(domain(W, "alignment", "no segregating site".into()));
    }
    let n = d.n as f64;
    let s = d.segregating as f64;
    let (a1, a2) = harmonic(d.n);
    let b1 = (n + 1.0) / (3.0 * (n - 1.0));
    let b2 = 2.0 * (n * n + n + 3.0) / (9.0 * n * (n - 1.0));
    let c1 = b1 - 1.0 / a1;
    let c2 = b2 - (n + 2.0) / (a1 * n) + a2 / (a1 * a1);
    let e1 = c1 / a1;
    let e2 = c2 / (a1 * a1 + a2);
    let tajima = (pi_of(&d) - s / a1) / (e1 * s + e2 * s * (s - 1.0)).sqrt();
    vm.push(BundValue::float(finite(W, tajima, || "Tajima's D".into())?));
    Ok(())
}

/// `N`, `p` and `generations`, checked.
fn wf_operands(
    word: &str,
    n: &BundValue,
    p: &BundValue,
    g: &BundValue,
) -> Result<(u64, f64, u64), Error> {
    let size = count(word, "N", n)?;
    if size == 0 {
        return Err(domain(word, "N", n.summary(WIDTH)));
    }
    if size > N_CAP {
        return Err(cap(word, &format!("N = {size}"), N_CAP));
    }
    let p0 = real(word, "p", p)?;
    if !(0.0..=1.0).contains(&p0) {
        return Err(domain(word, "p", p.summary(WIDTH)));
    }
    let generations = count(word, "generations", g)?;
    if generations > GENERATION_CAP {
        return Err(cap(
            word,
            &format!("{generations} generations"),
            GENERATION_CAP,
        ));
    }
    Ok((size, p0, generations))
}

/// A trajectory: from `p0`, each generation's expected frequency by
/// `select`, then 2N copies drawn binomially.
fn trajectory(
    vm: &mut dyn Vm,
    word: &str,
    size: u64,
    p0: f64,
    generations: u64,
    select: impl Fn(f64) -> f64,
) -> Result<BundValue, Error> {
    let copies = 2 * size;
    let out = with_rng(vm, |g| -> Result<Vec<f64>, Error> {
        let mut path = Vec::with_capacity(usize::try_from(generations).unwrap_or(0) + 1);
        let mut p = p0;
        path.push(p);
        for _ in 0..generations {
            let expected = select(p).clamp(0.0, 1.0);
            let draw = rand_distr::Binomial::new(copies, expected)
                .map_err(|e| Error::internal(format!("{word}: a checked binomial refused: {e}")))?;
            p = draw.sample(g) as f64 / copies as f64;
            path.push(p);
        }
        Ok(path)
    })?;
    Ok(list(out))
}

fn popgen_wf(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.wf";
    let v = operands(vm, W, 3)?;
    let (size, p0, generations) = wf_operands(W, &v[0], &v[1], &v[2])?;
    let path = trajectory(vm, W, size, p0, generations, |p| p)?;
    vm.push(path);
    Ok(())
}

/// Genic selection with dominance: fitnesses 1 + s, 1 + hs and 1 for AA, Aa
/// and aa, so p′ = p(1 + s(p + hq)) / (1 + s(p² + 2hpq)), which is p itself
/// when s = 0.
fn popgen_wf_selection(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "popgen.wf.selection";
    let v = operands(vm, W, 5)?;
    let (size, p0, generations) = wf_operands(W, &v[0], &v[1], &v[4])?;
    let s = real(W, "s", &v[2])?;
    let h = real(W, "h", &v[3])?;
    if s < -1.0 {
        return Err(domain(W, "s", v[2].summary(WIDTH)));
    }
    if 1.0 + h * s < 0.0 {
        return Err(domain(
            W,
            "h",
            format!(
                "{}, a heterozygote fitness 1 + hs below 0",
                v[3].summary(WIDTH)
            ),
        ));
    }
    let select = move |p: f64| {
        let q = 1.0 - p;
        let mean = 1.0 + s * (p * p + 2.0 * h * p * q);
        if mean > 0.0 {
            p * (1.0 + s * (p + h * q)) / mean
        } else {
            // Every genotype present has fitness 0; nothing changes.
            p
        }
    };
    let path = trajectory(vm, W, size, p0, generations, select)?;
    vm.push(path);
    Ok(())
}

/// Register §S13's words, each with its `.` form (§S3).
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
    word!("popgen.freq", popgen_freq, 1);
    word!("popgen.hwe", popgen_hwe, 3);
    word!("popgen.het", popgen_het, 1);
    word!("popgen.fst", popgen_fst, 4);
    word!("popgen.ld", popgen_ld, 4);
    word!("popgen.pi", popgen_pi, 1);
    word!("popgen.theta.w", popgen_theta_w, 1);
    word!("popgen.tajima", popgen_tajima, 1);
    word!("popgen.wf", popgen_wf, 3);
    word!("popgen.wf.selection", popgen_wf_selection, 5);
}

/// §S13's words; each also has its `.` form.
pub const WORDS: [&str; 10] = [
    "popgen.freq",
    "popgen.hwe",
    "popgen.het",
    "popgen.fst",
    "popgen.ld",
    "popgen.pi",
    "popgen.theta.w",
    "popgen.tajima",
    "popgen.wf",
    "popgen.wf.selection",
];

#[cfg(test)]
mod tests;
