//! RFC-0018 §S2: `model.matrix`, a frame coded as a design.
//!
//! The word refuses in §S2's order, steps 0 to 7, and stops at the first
//! failure. Every column is planned before any cell is computed: a column
//! is a product of atoms (a numeric cell, a logical cell read as 1.0 or
//! 0.0, or a factor cell's indicator of one level), so its name, its
//! source and the design's size are known, and the cap checked, before
//! n·k cells are allocated.
//!
//! **An interaction's members** are taken in R's variable order: the order
//! in which their columns first appear in `terms`, read left to right. The
//! first varies fastest, as `addfactor` places the next variable's columns
//! after the existing ones (R 4.4.2's `model.c:263-278`), and the names are
//! joined in the same order (D265).

use std::collections::{BTreeMap, BTreeSet};

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, LIST, NONE, STRING};

use super::opts;
use crate::biomath::frame::check::{self, Frame, Kind};
use crate::biomath::frame::opts::Missing;
use crate::biomath::frame::render;
use crate::biomath::operand::{WIDTH, domain, operands};
use crate::biomath::refuse::Refusal;

/// The most cells a design holds (D39), as `frame.join`'s rows are capped.
pub(crate) const CAP: u64 = 100_000_000;

const W: &str = "model.matrix";

/// One factor of a column's product, by column name.
#[derive(Debug, Clone)]
pub(crate) enum Atom {
    /// A numeric column's cell.
    Number(String),
    /// A logical column's cell, 1.0 for `true`.
    Logical(String),
    /// 1.0 where the factor column's cell is the level.
    Level(String, String),
}

/// A planned column: its name, where it came from, its term, its atoms.
pub(crate) struct Column {
    pub name: String,
    pub source: String,
    pub assign: usize,
    pub atoms: Vec<Atom>,
}

/// A cell of a numeric column as a FLOAT (B15).
fn number(v: &BundValue) -> Result<f64, Error> {
    match v.unboxed() {
        BundValue::Float(x, _) => Ok(*x),
        BundValue::Int(i, _) => Ok(*i as f64),
        _ => Err(Error::internal(format!(
            "{W}: a cell of a numeric column, {}, is not a number; the frame \
             check admits only INTEGER and FLOAT there",
            v.summary(WIDTH)
        ))),
    }
}

/// A logical or numeric cell as a FLOAT: `true` is 1.0.
pub(crate) fn cell_value(v: &BundValue) -> Result<f64, Error> {
    match v.unboxed() {
        BundValue::Bool(b, _) => Ok(if *b { 1.0 } else { 0.0 }),
        _ => number(v),
    }
}

/// The cells of column `i`.
fn col(f: &Frame, i: usize) -> &[BundValue] {
    f.cols.get(i).map_or(&[], Vec::as_slice)
}

/// The name of column `i`.
fn name(f: &Frame, i: usize) -> &str {
    f.names.get(i).map_or("", String::as_str)
}

impl Atom {
    fn label(&self) -> String {
        match self {
            Atom::Number(c) => c.clone(),
            Atom::Logical(c) => format!("{c}.true"),
            Atom::Level(c, l) => format!("{c}.{l}"),
        }
    }

    fn column(&self) -> &str {
        match self {
            Atom::Number(c) | Atom::Logical(c) | Atom::Level(c, _) => c,
        }
    }

    fn value(&self, cell: &BundValue) -> Result<f64, Error> {
        match self {
            Atom::Number(_) => number(cell),
            Atom::Logical(_) => cell_value(cell),
            Atom::Level(_, l) => Ok(if cell.as_str().as_deref() == Some(l.as_str()) {
                1.0
            } else {
                0.0
            }),
        }
    }
}

/// R's variable order: each column in the order it first appears in
/// `terms`, read left to right (`model.c:1091-1112`).
pub(crate) fn variable_order(terms: &[Vec<String>]) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    for t in terms {
        for m in t {
            if !order.contains(m) {
                order.push(m.clone());
            }
        }
    }
    order
}

/// Each column of a design (§S2): the intercept's, then each term's, an
/// interaction's members in R's variable order, the first fastest. A
/// factor is coded from `levels`, its first the baseline; without an
/// intercept, the first factor term codes every level.
pub(crate) fn plan(
    word: &str,
    terms: &[Vec<String>],
    kinds: &BTreeMap<String, Kind>,
    levels: &BTreeMap<String, Vec<String>>,
    intercept: bool,
) -> Result<Vec<Column>, Error> {
    let order = variable_order(terms);
    let full = if intercept {
        None
    } else {
        terms
            .iter()
            .position(|t| matches!(t.as_slice(), [m] if kinds.get(m) == Some(&Kind::Factor)))
    };
    let atoms_of = |m: &String, fully: bool| -> Result<Vec<Atom>, Error> {
        Ok(match kinds.get(m) {
            Some(Kind::Numeric) => vec![Atom::Number(m.clone())],
            Some(Kind::Logical) => vec![Atom::Logical(m.clone())],
            Some(Kind::Factor) => {
                let ls = levels.get(m).map(Vec::as_slice).unwrap_or(&[]);
                let skip = usize::from(!fully);
                ls.iter()
                    .skip(skip)
                    .map(|l| Atom::Level(m.clone(), l.clone()))
                    .collect()
            }
            _ => {
                return Err(Error::internal(format!(
                    "{word}: column {m:?} reached the plan with a kind that codes nothing"
                )));
            }
        })
    };
    let mut columns: Vec<Column> = Vec::new();
    if intercept {
        columns.push(Column {
            name: "(intercept)".into(),
            source: "the intercept".into(),
            assign: 0,
            atoms: Vec::new(),
        });
    }
    for (t, members) in terms.iter().enumerate() {
        let mut sorted: Vec<&String> = members.iter().collect();
        sorted.sort_by_key(|m| order.iter().position(|o| o == *m));
        let mut products: Vec<Vec<Atom>> = vec![Vec::new()];
        for m in sorted {
            let fully = members.len() == 1 && full == Some(t);
            let atoms = atoms_of(m, fully)?;
            let mut next = Vec::with_capacity(products.len() * atoms.len());
            for a in &atoms {
                for p in &products {
                    let mut q = p.clone();
                    q.push(a.clone());
                    next.push(q);
                }
            }
            products = next;
        }
        for atoms in products {
            let label = atoms.iter().map(Atom::label).collect::<Vec<_>>().join(":");
            let source = match atoms.as_slice() {
                [Atom::Level(c, l)] => format!("{} level {}", render::text(c), render::text(l)),
                [a] => format!("column {}", render::text(a.column())),
                _ => format!("term {}", t + 1),
            };
            columns.push(Column {
                name: label,
                source,
                assign: t + 1,
                atoms,
            });
        }
    }
    Ok(columns)
}

/// The design's cells for `rows` of `f`, each column the product of its
/// atoms, in order.
pub(crate) fn cells(
    word: &str,
    f: &Frame,
    columns: &[Column],
    rows: &[usize],
) -> Result<Vec<Vec<f64>>, Error> {
    // Each atom's column, resolved once.
    let mut resolved: Vec<Vec<(usize, &Atom)>> = Vec::with_capacity(columns.len());
    for c in columns {
        let mut atoms = Vec::with_capacity(c.atoms.len());
        for a in &c.atoms {
            let i = f.index(a.column()).ok_or_else(|| {
                Error::internal(format!(
                    "{word}: column {:?} was checked present and is gone",
                    a.column()
                ))
            })?;
            atoms.push((i, a));
        }
        resolved.push(atoms);
    }
    let mut out = Vec::with_capacity(rows.len());
    for &r in rows {
        let mut row = Vec::with_capacity(columns.len());
        for atoms in &resolved {
            let mut x = 1.0;
            for (i, a) in atoms {
                let cell = col(f, *i).get(r).ok_or_else(|| {
                    Error::internal(format!("{word}: row {} is past the frame's end", r + 1))
                })?;
                x *= a.value(cell)?;
            }
            row.push(x);
        }
        out.push(row);
    }
    Ok(out)
}

/// One `terms` item's members: a STRING, or a LIST of two or more STRINGs.
pub(crate) fn members_of(item: &BundValue) -> Option<Vec<String>> {
    if item.dt() == STRING {
        item.as_str().map(|s| vec![s])
    } else if item.dt() == LIST {
        item.as_list().filter(|xs| xs.len() >= 2).and_then(|xs| {
            xs.iter()
                .map(|x| x.as_str().filter(|_| x.dt() == STRING))
                .collect()
        })
    } else {
        None
    }
}

/// E101 on `terms`.
fn terms_refusal(text: String) -> Error {
    domain(W, "terms", text)
}

/// Step 3: each item a STRING or a LIST of two or more STRINGs, naming a
/// column, its members distinct, not repeated, its members earlier.
fn read_terms(f: &Frame, v: &BundValue) -> Result<Vec<Vec<String>>, Error> {
    let items = match v.as_list() {
        Some(xs) if v.dt() == LIST => xs,
        _ => return Err(terms_refusal(v.summary(WIDTH))),
    };
    let mut terms: Vec<Vec<String>> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let t = i + 1;
        let Some(members) = members_of(item) else {
            return Err(terms_refusal(format!(
                "item {t} is {}",
                render::value(item)
            )));
        };
        if let Some(m) = members.iter().find(|m| f.index(m).is_none()) {
            return Err(terms_refusal(format!("no column {}", render::text(m))));
        }
        for (j, m) in members.iter().enumerate() {
            if members.get(..j).is_some_and(|before| before.contains(m)) {
                return Err(terms_refusal(format!(
                    "term {t} repeats {}",
                    render::text(m)
                )));
            }
        }
        let set: BTreeSet<&String> = members.iter().collect();
        if let Some(p) = terms
            .iter()
            .position(|e| e.len() == members.len() && e.iter().collect::<BTreeSet<_>>() == set)
        {
            return Err(terms_refusal(format!("term {t} repeats term {}", p + 1)));
        }
        if members.len() > 1 {
            for m in &members {
                if !terms.iter().any(|e| e.len() == 1 && e.first() == Some(m)) {
                    return Err(terms_refusal(format!(
                        "term {t} needs {} as a term before it",
                        render::text(m)
                    )));
                }
            }
        }
        terms.push(members);
    }
    Ok(terms)
}

/// The rows with no NONE in the columns `reads`, under `na`: E101 at the
/// first NONE under `"refuse"`, columns in `reads` order, rows from the
/// first, as RFC-0017's words refuse it.
pub(crate) fn kept(
    word: &str,
    f: &Frame,
    reads: &[usize],
    missing: Missing,
) -> Result<Vec<usize>, Error> {
    let mut keep = vec![true; f.nrow];
    for &c in reads {
        for (r, x) in col(f, c).iter().enumerate() {
            if x.dt() != NONE {
                continue;
            }
            match missing {
                Missing::Refuse => {
                    return Err(domain(
                        word,
                        "frame",
                        format!(
                            "column {} row {} is none (na \"refuse\")",
                            render::text(name(f, c)),
                            r + 1
                        ),
                    ));
                }
                Missing::Omit => {
                    if let Some(k) = keep.get_mut(r) {
                        *k = false;
                    }
                }
            }
        }
    }
    Ok((0..f.nrow)
        .filter(|&r| keep.get(r) == Some(&true))
        .collect())
}

/// A column's index, which step 2 or 3 found.
fn index(f: &Frame, n: &str) -> Result<usize, Error> {
    f.index(n).ok_or_else(|| {
        Error::internal(format!(
            "{W}: column {n:?} was found by an earlier step and is gone"
        ))
    })
}

/// `model.matrix` (`frame response terms opts -- design`).
pub(crate) fn model_matrix(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = operands(vm, W, 4)?;
    let (frame_v, response_v, terms_v, opts_v) = match v.as_slice() {
        [a, b, c, d] => (a, b, c, d),
        _ => {
            return Err(Error::internal(format!(
                "{W}: four operands were pulled and {} are held",
                v.len()
            )));
        }
    };
    // Step 0: the frame.
    let f = check::check(W, "frame", frame_v)?;
    // Step 1: the options.
    let o = opts::read(W, opts_v, &["intercept", "na", "offset", "weights"])?;
    let intercept = o.intercept.unwrap_or(true);
    let missing = o.missing.unwrap_or(Missing::Refuse);
    // Step 2: the response, then `weights` and `offset`.
    let response = match response_v.as_str() {
        Some(s) if response_v.dt() == STRING => s,
        _ => return Err(domain(W, "response", response_v.summary(WIDTH))),
    };
    let ri = f.index(&response).ok_or_else(|| {
        domain(
            W,
            "response",
            format!("no column {}", render::text(&response)),
        )
    })?;
    let named = |key: &str, n: &Option<String>| -> Result<Option<usize>, Error> {
        match n {
            Some(n) => f.index(n).map(Some).ok_or_else(|| {
                opts::refuse(W, format!("{key} names no column {}", render::text(n)))
            }),
            None => Ok(None),
        }
    };
    let wi = named("weights", &o.weights)?;
    let oi = named("offset", &o.offset)?;
    // Step 3: the terms; then no columns at all.
    let terms = read_terms(&f, terms_v)?;
    if !intercept && terms.is_empty() {
        return Err(terms_refusal(
            "no columns, with no intercept and no terms".into(),
        ));
    }
    // Step 4: the overlaps.
    if let Some(t) = terms.iter().position(|t| t.contains(&response)) {
        return Err(terms_refusal(format!(
            "term {} reads the response {}",
            t + 1,
            render::text(&response)
        )));
    }
    for (key, n) in [("weights", &o.weights), ("offset", &o.offset)] {
        if n.as_deref() == Some(response.as_str()) {
            return Err(opts::refuse(
                W,
                format!("{key} names the response {}", render::text(&response)),
            ));
        }
    }
    if let (Some(w), Some(off)) = (&o.weights, &o.offset)
        && w == off
    {
        return Err(opts::refuse(
            W,
            format!("offset names {}, as weights does", render::text(w)),
        ));
    }
    // The columns the terms read, in order of first appearance: R's
    // variable order (`model.c:1091-1112`).
    let order = variable_order(&terms);
    // Step 5: each column's kind.
    let rk = f.kind(ri);
    if !matches!(rk, Kind::Numeric | Kind::Logical) {
        return Err(domain(
            W,
            "response",
            format!("column {} is {}", render::text(&response), rk.word()),
        ));
    }
    let mut kinds: BTreeMap<String, Kind> = BTreeMap::new();
    for c in &order {
        let k = f.kind(index(&f, c)?);
        if matches!(k, Kind::Text | Kind::Unknown) {
            return Err(terms_refusal(format!(
                "column {} is {}",
                render::text(c),
                k.word()
            )));
        }
        kinds.insert(c.clone(), k);
    }
    for (key, i) in [("weights", wi), ("offset", oi)] {
        let Some(i) = i else { continue };
        let k = f.kind(i);
        if k != Kind::Numeric {
            return Err(opts::refuse(
                W,
                format!("{key} column {} is {}", render::text(name(&f, i)), k.word()),
            ));
        }
    }
    // Step 6: NONE, then the cells of `weights` (`offset`'s are finite by
    // the frame check).
    let mut reads = vec![ri];
    for c in &order {
        let i = index(&f, c)?;
        if !reads.contains(&i) {
            reads.push(i);
        }
    }
    for i in [wi, oi].into_iter().flatten() {
        if !reads.contains(&i) {
            reads.push(i);
        }
    }
    let rows = kept(W, &f, &reads, missing)?;
    if let Some(w) = wi {
        for &r in &rows {
            let Some(cell) = col(&f, w).get(r) else {
                continue;
            };
            let x = number(cell)?;
            if x <= 0.0 {
                let aside = if x == 0.0 {
                    "; frame.filter sets a row aside"
                } else {
                    ""
                };
                return Err(opts::refuse(
                    W,
                    format!("weights row {} is {}{aside}", r + 1, render::value(cell)),
                ));
            }
        }
    }
    // Step 7: the rows kept.
    if rows.is_empty() {
        return Err(domain(W, "frame", "no rows".into()));
    }
    // Each factor's used levels, in the frame's level order.
    let mut used: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for c in &order {
        if kinds.get(c) != Some(&Kind::Factor) {
            continue;
        }
        let i = index(&f, c)?;
        let cells = col(&f, i);
        let seen: BTreeSet<String> = rows
            .iter()
            .filter_map(|&r| cells.get(r).and_then(BundValue::as_str))
            .collect();
        let all = f.levels.get(c).cloned().unwrap_or_default();
        let ls: Vec<String> = all.into_iter().filter(|l| seen.contains(l)).collect();
        used.insert(c.clone(), ls);
    }
    for (t, members) in terms.iter().enumerate() {
        let [m] = members.as_slice() else { continue };
        if let Some(ls) = used.get(m)
            && ls.len() < 2
        {
            let level = ls.first().map_or("none".to_string(), |l| render::text(l));
            return Err(terms_refusal(format!(
                "term {}, {}, has one used level, {level}",
                t + 1,
                render::text(m)
            )));
        }
    }
    // The plan: each column's atoms.
    let columns = plan(W, &terms, &kinds, &used, intercept)?;
    // Two columns of one name.
    for (j, c) in columns.iter().enumerate() {
        if let Some(e) = columns
            .get(..j)
            .and_then(|b| b.iter().find(|e| e.name == c.name))
        {
            return Err(terms_refusal(format!(
                "column name {} is both {} and {}",
                render::text(&c.name),
                e.source,
                c.source
            )));
        }
    }
    let (n, k) = (rows.len(), columns.len());
    if n <= k {
        return Err(domain(
            W,
            "frame",
            format!("{n} observations for {k} coefficients"),
        ));
    }
    if (n as u64).saturating_mul(k as u64) > CAP {
        return Err(Refusal::Cap {
            what: "the design's cells".into(),
            cap: CAP,
        }
        .error(W));
    }
    // The cells.
    let matrix: Vec<Vec<BundValue>> = cells(W, &f, &columns, &rows)?
        .into_iter()
        .map(|row| row.into_iter().map(BundValue::float).collect())
        .collect();
    let floats_of = |i: Option<usize>, default: f64| -> Result<BundValue, Error> {
        let xs = rows
            .iter()
            .map(|&r| match i {
                Some(i) => col(&f, i)
                    .get(r)
                    .map_or(Ok(default), cell_value)
                    .map(BundValue::float),
                None => Ok(BundValue::float(default)),
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(BundValue::list(xs))
    };
    let mut spec = BTreeMap::new();
    spec.insert("response".to_string(), BundValue::str(response.clone()));
    spec.insert("terms".to_string(), terms_v.clone());
    spec.insert("intercept".to_string(), BundValue::boolean(intercept));
    spec.insert(
        "levels".to_string(),
        BundValue::map(
            used.iter()
                .map(|(c, ls)| {
                    let items = ls.iter().map(|l| BundValue::str(l.clone())).collect();
                    (c.clone(), BundValue::list(items))
                })
                .collect(),
        ),
    );
    spec.insert(
        "logical".to_string(),
        BundValue::list(
            order
                .iter()
                .filter(|c| kinds.get(*c) == Some(&Kind::Logical))
                .map(|c| BundValue::str(c.clone()))
                .collect(),
        ),
    );
    if let Some(w) = &o.weights {
        spec.insert("weights".to_string(), BundValue::str(w.clone()));
    }
    if let Some(off) = &o.offset {
        spec.insert("offset".to_string(), BundValue::str(off.clone()));
    }
    let count = |x: usize| BundValue::int(i64::try_from(x).unwrap_or(i64::MAX));
    let mut d = BTreeMap::new();
    d.insert("matrix".to_string(), BundValue::matrix(matrix));
    d.insert(
        "names".to_string(),
        BundValue::list(
            columns
                .iter()
                .map(|c| BundValue::str(c.name.clone()))
                .collect(),
        ),
    );
    d.insert(
        "assign".to_string(),
        BundValue::list(columns.iter().map(|c| count(c.assign)).collect()),
    );
    d.insert("y".to_string(), floats_of(Some(ri), 0.0)?);
    d.insert("weights".to_string(), floats_of(wi, 1.0)?);
    d.insert("offset".to_string(), floats_of(oi, 0.0)?);
    d.insert("dropped".to_string(), count(f.nrow - n));
    d.insert("spec".to_string(), BundValue::map(spec));
    vm.push(BundValue::map(d));
    Ok(())
}
