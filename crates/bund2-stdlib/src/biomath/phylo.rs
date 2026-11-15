//! RFC-0015 §S14: phylogenetics, over flat trees.
//!
//! **A tree [B7]** is a MAP of three LISTs, one entry a node: `names`
//! (STRINGs, verbatim, `""` unnamed), `parent` (an INTEGER index, −1 at the
//! root) and `length` (to the parent, 0.0 at the root). Every word walks
//! it with loops over indices, never recursion on its depth (D39), so a
//! tree 10⁵ deep is as safe as a shallow one.
//!
//! **A tree operand is checked whole**: three LISTs of one length (E102), a
//! STRING name, an INTEGER parent in range, a finite length, exactly one
//! root and no cycle (each E101).
//!
//! **Work (D39).** `phylo.upgma` and `phylo.nj` take O(n³) for n taxa, so
//! n is capped at [`TAXA_CAP`]; `phylo.dist` compares every pair at every
//! site, so n²·sites is capped at [`DIST_CAP`] (E111).

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::{BundValue, LIST, MAP, STRING};

use super::infer::table;
use super::operand::{WIDTH, domain, operands};
use super::refuse::Refusal;

/// The most taxa `phylo.upgma` and `phylo.nj` join (§S14, D39).
const TAXA_CAP: u64 = 2_000;

/// The most site comparisons `phylo.dist` makes, sequences² × sites.
const DIST_CAP: u64 = 10_000_000_000;

/// A tree, as its three columns.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tree {
    pub names: Vec<String>,
    pub parent: Vec<i64>,
    pub length: Vec<f64>,
}

impl Tree {
    fn value(self) -> BundValue {
        let mut m = BTreeMap::new();
        m.insert(
            "names".to_string(),
            BundValue::list(self.names.into_iter().map(BundValue::str).collect()),
        );
        m.insert(
            "parent".to_string(),
            BundValue::list(self.parent.into_iter().map(BundValue::int).collect()),
        );
        m.insert(
            "length".to_string(),
            BundValue::list(self.length.into_iter().map(BundValue::float).collect()),
        );
        BundValue::map(m)
    }

    /// Each node's children, in index order.
    fn children(&self) -> Vec<Vec<usize>> {
        let mut c = vec![Vec::new(); self.names.len()];
        for (i, &p) in self.parent.iter().enumerate() {
            if let Ok(p) = usize::try_from(p) {
                c[p].push(i);
            }
        }
        c
    }

    fn root(&self) -> usize {
        self.parent.iter().position(|&p| p < 0).unwrap_or(0)
    }
}

fn bad_tree(word: &str, why: String) -> Error {
    domain(word, "tree", why)
}

/// Read and check a tree operand.
fn tree(word: &str, v: &BundValue) -> Result<Tree, Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(bad_tree(word, v.summary(WIDTH))),
    };
    let column = |key: &str| -> Result<&[BundValue], Error> {
        match m.get(key) {
            Some(c) if c.dt() == LIST => Ok(c.as_list().unwrap_or(&[])),
            _ => Err(bad_tree(word, format!("no LIST {key}"))),
        }
    };
    let (names, parent, length) = (column("names")?, column("parent")?, column("length")?);
    let n = names.len();
    for c in [parent, length] {
        if c.len() != n {
            return Err(Refusal::Disagree {
                what: "lengths".into(),
                n,
                m: c.len(),
            }
            .error(word));
        }
    }
    if n == 0 {
        return Err(bad_tree(word, "a tree of no nodes".into()));
    }
    let mut t = Tree {
        names: Vec::with_capacity(n),
        parent: Vec::with_capacity(n),
        length: Vec::with_capacity(n),
    };
    for i in 0..n {
        match names[i].as_str() {
            Some(s) if names[i].dt() == STRING => t.names.push(s),
            _ => {
                return Err(bad_tree(
                    word,
                    format!("name {} is {}", i, names[i].summary(WIDTH)),
                ));
            }
        }
        match parent[i].unboxed() {
            BundValue::Int(p, _) if *p >= -1 && *p < n as i64 && *p != i as i64 => {
                t.parent.push(*p)
            }
            _ => {
                return Err(bad_tree(
                    word,
                    format!("parent {} is {}", i, parent[i].summary(WIDTH)),
                ));
            }
        }
        let x = match length[i].unboxed() {
            BundValue::Float(x, _) => *x,
            BundValue::Int(k, _) => *k as f64,
            _ => f64::NAN,
        };
        if !x.is_finite() {
            return Err(bad_tree(
                word,
                format!("length {} is {}", i, length[i].summary(WIDTH)),
            ));
        }
        t.length.push(x);
    }
    let roots = t.parent.iter().filter(|&&p| p < 0).count();
    if roots != 1 {
        return Err(bad_tree(word, format!("{roots} roots, where 1 is needed")));
    }
    // No cycle: walk up from each node, marking what reaches the root.
    let mut state = vec![0u8; n]; // 0 unseen, 1 on this walk, 2 reaches the root
    for start in 0..n {
        let mut path = Vec::new();
        let mut i = start;
        loop {
            match state[i] {
                2 => break,
                1 => return Err(bad_tree(word, format!("a cycle through node {i}"))),
                _ => {}
            }
            state[i] = 1;
            path.push(i);
            match usize::try_from(t.parent[i]) {
                Ok(p) => i = p,
                Err(_) => break,
            }
        }
        for k in path {
            state[k] = 2;
        }
    }
    Ok(t)
}

// Distances.

fn phylo_dist(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "phylo.dist";
    let v = operands(vm, W, 2)?;
    let seqs: Vec<Vec<char>> = match v[0].as_list() {
        Some(items) if v[0].dt() == LIST && !items.is_empty() => items
            .iter()
            .map(|s| match s.as_str() {
                Some(x) if s.dt() == STRING => Ok(x.chars().collect()),
                _ => Err(domain(W, "alignment", s.summary(WIDTH))),
            })
            .collect::<Result<_, _>>()?,
        _ => return Err(domain(W, "alignment", v[0].summary(WIDTH))),
    };
    let model = match v[1].as_str().filter(|_| v[1].dt() == STRING).as_deref() {
        Some(m @ ("p" | "jc69" | "k2p")) => m.to_string(),
        _ => return Err(domain(W, "model", v[1].summary(WIDTH))),
    };
    let width = seqs[0].len();
    if let Some(s) = seqs.iter().find(|s| s.len() != width) {
        return Err(Refusal::Disagree {
            what: "lengths".into(),
            n: width,
            m: s.len(),
        }
        .error(W));
    }
    let n = seqs.len();
    let work = (n as u64)
        .saturating_mul(n as u64)
        .saturating_mul(width as u64);
    if work > DIST_CAP {
        return Err(Refusal::Cap {
            what: format!("{n}² sequences × {width} sites"),
            cap: DIST_CAP,
        }
        .error(W));
    }
    // A, G purines 0, 2; C, T pyrimidines 1, 3; None skips the site.
    let code: Vec<Vec<Option<u8>>> = seqs
        .iter()
        .map(|s| {
            s.iter()
                .enumerate()
                .map(|(j, &c)| match c.to_ascii_uppercase() {
                    'A' => Ok(Some(0)),
                    'C' => Ok(Some(1)),
                    'G' => Ok(Some(2)),
                    'T' | 'U' => Ok(Some(3)),
                    '-' | 'R' | 'Y' | 'S' | 'W' | 'K' | 'M' | 'B' | 'D' | 'H' | 'V' | 'N' => {
                        Ok(None)
                    }
                    _ => Err(Refusal::Alphabet {
                        ch: c,
                        pos: j + 1,
                        alphabet: "iupac".into(),
                    }
                    .error(W)),
                })
                .collect()
        })
        .collect::<Result<_, _>>()?;
    let mut d = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in i + 1..n {
            let (mut sites, mut ts, mut tv) = (0usize, 0usize, 0usize);
            for (x, y) in code[i].iter().zip(&code[j]) {
                if let (Some(x), Some(y)) = (x, y) {
                    sites += 1;
                    if x != y {
                        // Same parity, same class: a transition.
                        if x % 2 == y % 2 {
                            ts += 1;
                        } else {
                            tv += 1;
                        }
                    }
                }
            }
            if sites == 0 {
                return Err(domain(
                    W,
                    "alignment",
                    format!("sequences {} and {} share no site", i + 1, j + 1),
                ));
            }
            let l = sites as f64;
            // Transitions P, transversions Q, and every difference, each
            // over the sites compared, so p is not P + Q rounded twice.
            let (p, q, all) = (ts as f64 / l, tv as f64 / l, (ts + tv) as f64 / l);
            let saturated = || {
                domain(
                    W,
                    "alignment",
                    format!(
                        "sequences {} and {} are saturated under {model}",
                        i + 1,
                        j + 1
                    ),
                )
            };
            let x = match model.as_str() {
                "jc69" => {
                    let a = 1.0 - 4.0 / 3.0 * all;
                    if a <= 0.0 {
                        return Err(saturated());
                    }
                    -0.75 * a.ln()
                }
                "k2p" => {
                    let (a, b) = (1.0 - 2.0 * p - q, 1.0 - 2.0 * q);
                    if a <= 0.0 || b <= 0.0 {
                        return Err(saturated());
                    }
                    -0.5 * a.ln() - 0.25 * b.ln()
                }
                _ => all,
            };
            d[i][j] = x;
            d[j][i] = x;
        }
    }
    vm.push(BundValue::matrix(
        d.into_iter()
            .map(|r| r.into_iter().map(BundValue::float).collect())
            .collect(),
    ));
    Ok(())
}

// Building trees.

/// A distance MATRIX and its names: square (E102), from two taxa to the cap
/// (E101, E111), symmetric (E107), a zero diagonal and no negative cell
/// (E101), one name a row (E102).
fn taxa(word: &str, v: &[BundValue]) -> Result<(Vec<Vec<f64>>, Vec<String>), Error> {
    let d = table(word, "matrix", &v[0])?;
    let n = d.len();
    if let Some(r) = d.iter().find(|r| r.len() != n) {
        return Err(Refusal::Disagree {
            what: "rows and columns".into(),
            n,
            m: r.len(),
        }
        .error(word));
    }
    if n < 2 {
        return Err(domain(
            word,
            "matrix",
            format!("{n} taxa, where 2 are needed"),
        ));
    }
    if n as u64 > TAXA_CAP {
        return Err(Refusal::Cap {
            what: format!("{n} taxa"),
            cap: TAXA_CAP,
        }
        .error(word));
    }
    for (i, row) in d.iter().enumerate() {
        if row[i] != 0.0 {
            return Err(domain(
                word,
                "matrix",
                format!("{} at [{},{}]", row[i], i + 1, i + 1),
            ));
        }
        for (j, &x) in row.iter().enumerate() {
            if x < 0.0 {
                return Err(domain(
                    word,
                    "matrix",
                    format!("{x} at [{},{}]", i + 1, j + 1),
                ));
            }
            if x != d[j][i] {
                return Err(Refusal::NotSymmetric {
                    i: i + 1,
                    j: j + 1,
                    by: (x - d[j][i]).abs(),
                }
                .error(word));
            }
        }
    }
    let names: Vec<String> = match v[1].as_list() {
        Some(items) if v[1].dt() == LIST => items
            .iter()
            .map(|s| match s.as_str() {
                Some(x) if s.dt() == STRING => Ok(x),
                _ => Err(domain(word, "names", s.summary(WIDTH))),
            })
            .collect::<Result<_, _>>()?,
        _ => return Err(domain(word, "names", v[1].summary(WIDTH))),
    };
    if names.len() != n {
        return Err(Refusal::Disagree {
            what: "rows and names".into(),
            n,
            m: names.len(),
        }
        .error(word));
    }
    Ok((d, names))
}

/// A tree under construction: leaves first, each join appended.
struct Builder {
    names: Vec<String>,
    parent: Vec<i64>,
    length: Vec<f64>,
}

impl Builder {
    fn new(leaves: Vec<String>) -> Self {
        let n = leaves.len();
        Builder {
            names: leaves,
            parent: vec![-1; n],
            length: vec![0.0; n],
        }
    }

    /// A new inner node over `kids`, each with its branch length.
    fn join(&mut self, kids: &[(usize, f64)]) -> usize {
        let u = self.names.len();
        self.names.push(String::new());
        self.parent.push(-1);
        self.length.push(0.0);
        for &(k, len) in kids {
            self.parent[k] = u as i64;
            self.length[k] = len;
        }
        u
    }

    fn done(self) -> Tree {
        Tree {
            names: self.names,
            parent: self.parent,
            length: self.length,
        }
    }
}

/// The active pair with the least `score`, the first in active order on a
/// tie.
fn least(active: &[usize], score: impl Fn(usize, usize) -> f64) -> (usize, usize) {
    let mut best = (0, 1, f64::INFINITY);
    for a in 0..active.len() {
        for b in a + 1..active.len() {
            let s = score(a, b);
            if s < best.2 {
                best = (a, b, s);
            }
        }
    }
    (best.0, best.1)
}

fn phylo_upgma(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "phylo.upgma";
    let v = operands(vm, W, 2)?;
    let (d0, names) = taxa(W, &v)?;
    let n = names.len();
    let mut tb = Builder::new(names);
    // Distances between clusters, indexed by node; sizes and heights.
    let mut d = vec![vec![0.0; 2 * n - 1]; 2 * n - 1];
    for i in 0..n {
        d[i][..n].copy_from_slice(&d0[i]);
    }
    let mut size = vec![1.0f64; 2 * n - 1];
    let mut height = vec![0.0f64; 2 * n - 1];
    let mut active: Vec<usize> = (0..n).collect();
    while active.len() > 1 {
        let (a, b) = least(&active, |a, b| d[active[a]][active[b]]);
        let (x, y) = (active[a], active[b]);
        let h = d[x][y] / 2.0;
        let u = tb.join(&[(x, h - height[x]), (y, h - height[y])]);
        height[u] = h;
        size[u] = size[x] + size[y];
        for &k in &active {
            if k != x && k != y {
                let duk = (size[x] * d[x][k] + size[y] * d[y][k]) / size[u];
                d[u][k] = duk;
                d[k][u] = duk;
            }
        }
        active.remove(b);
        active.remove(a);
        active.push(u);
    }
    vm.push(tb.done().value());
    Ok(())
}

/// Saitou and Nei's neighbour joining, rooted at its last join: when two
/// nodes are left, the earlier becomes a child of the later.
fn phylo_nj(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "phylo.nj";
    let v = operands(vm, W, 2)?;
    let (d0, names) = taxa(W, &v)?;
    let n = names.len();
    let mut tb = Builder::new(names);
    let mut d = vec![vec![0.0; 2 * n - 1]; 2 * n - 1];
    for i in 0..n {
        d[i][..n].copy_from_slice(&d0[i]);
    }
    let mut active: Vec<usize> = (0..n).collect();
    while active.len() > 2 {
        let r = active.len() as f64;
        let sums: Vec<f64> = active
            .iter()
            .map(|&i| active.iter().map(|&k| d[i][k]).sum())
            .collect();
        let (a, b) = least(&active, |a, b| {
            (r - 2.0) * d[active[a]][active[b]] - sums[a] - sums[b]
        });
        let (f, g) = (active[a], active[b]);
        let dfg = d[f][g];
        let lf = dfg / 2.0 + (sums[a] - sums[b]) / (2.0 * (r - 2.0));
        let u = tb.join(&[(f, lf), (g, dfg - lf)]);
        for &k in &active {
            if k != f && k != g {
                let duk = (d[f][k] + d[g][k] - dfg) / 2.0;
                d[u][k] = duk;
                d[k][u] = duk;
            }
        }
        active.remove(b);
        active.remove(a);
        active.push(u);
    }
    // Two left: the last join is the root, the other its child.
    let (x, root) = (active[0], active[1]);
    tb.parent[x] = root as i64;
    tb.length[x] = d[x][root];
    vm.push(tb.done().value());
    Ok(())
}

// Newick.

/// A name as Newick writes it: quoted, with `'` doubled, when it holds a
/// character Newick reads as structure or a blank.
fn label(name: &str) -> String {
    if name
        .chars()
        .any(|c| "()[]':;,".contains(c) || c.is_whitespace())
    {
        format!("'{}'", name.replace('\'', "''"))
    } else {
        name.to_string()
    }
}

fn newick_of(t: &Tree) -> String {
    let kids = t.children();
    let mut out = String::new();
    // (node, next child to write); a node is opened when first pushed.
    let mut stack: Vec<(usize, usize)> = vec![(t.root(), 0)];
    if !kids[t.root()].is_empty() {
        out.push('(');
    }
    while let Some(top) = stack.last_mut() {
        let (node, next) = *top;
        if next < kids[node].len() {
            top.1 += 1;
            let child = kids[node][next];
            if next > 0 {
                out.push(',');
            }
            if !kids[child].is_empty() {
                out.push('(');
            }
            stack.push((child, 0));
            continue;
        }
        stack.pop();
        if !kids[node].is_empty() {
            out.push(')');
        }
        out.push_str(&label(&t.names[node]));
        if t.parent[node] >= 0 || t.length[node] != 0.0 {
            out.push(':');
            out.push_str(&t.length[node].to_string());
        }
    }
    out.push(';');
    out
}

fn phylo_newick(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "phylo.newick";
    let v = operands(vm, W, 1)?;
    let t = tree(W, &v[0])?;
    vm.push(BundValue::str(newick_of(&t)));
    Ok(())
}

/// E113 at character `pos`, counted from 1.
fn malformed(word: &str, pos: usize, why: &str) -> Error {
    Refusal::Newick {
        pos: pos + 1,
        why: why.into(),
    }
    .error(word)
}

/// Parse Newick without recursion: a stack of the open inner nodes.
pub(crate) fn parse_newick(word: &str, s: &str) -> Result<Tree, Error> {
    let cs: Vec<char> = s.chars().collect();
    let mut t = Tree {
        names: Vec::new(),
        parent: Vec::new(),
        length: Vec::new(),
    };
    let mut open: Vec<usize> = Vec::new();
    let mut i = 0;
    let skip = |i: &mut usize| -> Result<(), Error> {
        loop {
            while *i < cs.len() && cs[*i].is_whitespace() {
                *i += 1;
            }
            if *i < cs.len() && cs[*i] == '[' {
                let from = *i;
                while *i < cs.len() && cs[*i] != ']' {
                    *i += 1;
                }
                if *i == cs.len() {
                    return Err(malformed(word, from, "a comment is not closed"));
                }
                *i += 1;
            } else {
                return Ok(());
            }
        }
    };
    let new_node = |t: &mut Tree, open: &[usize]| -> usize {
        t.names.push(String::new());
        t.parent.push(open.last().map_or(-1, |&p| p as i64));
        t.length.push(0.0);
        t.names.len() - 1
    };
    // A label, quoted or bare, and an optional `:length`, for `node`.
    let tail = |t: &mut Tree, node: usize, i: &mut usize| -> Result<(), Error> {
        skip(i)?;
        if *i < cs.len() && cs[*i] == '\'' {
            let from = *i;
            *i += 1;
            let mut name = String::new();
            loop {
                match cs.get(*i) {
                    None => return Err(malformed(word, from, "a quoted name is not closed")),
                    Some('\'') if cs.get(*i + 1) == Some(&'\'') => {
                        name.push('\'');
                        *i += 2;
                    }
                    Some('\'') => {
                        *i += 1;
                        break;
                    }
                    Some(&c) => {
                        name.push(c);
                        *i += 1;
                    }
                }
            }
            t.names[node] = name;
        } else {
            let from = *i;
            while *i < cs.len() && !"()[]':;,".contains(cs[*i]) && !cs[*i].is_whitespace() {
                *i += 1;
            }
            t.names[node] = cs[from..*i].iter().collect();
        }
        skip(i)?;
        if *i < cs.len() && cs[*i] == ':' {
            *i += 1;
            skip(i)?;
            let from = *i;
            while *i < cs.len() && (cs[*i].is_ascii_alphanumeric() || "+-.".contains(cs[*i])) {
                *i += 1;
            }
            let text: String = cs[from..*i].iter().collect();
            match text.parse::<f64>() {
                Ok(x) if x.is_finite() => t.length[node] = x,
                _ => return Err(malformed(word, from, "a branch length is not a number")),
            }
        }
        Ok(())
    };
    // `expect_node`: at the start of a subtree; otherwise after one.
    let mut expect_node = true;
    loop {
        skip(&mut i)?;
        let Some(&c) = cs.get(i) else {
            return Err(malformed(word, i, "the string ends before ;"));
        };
        if expect_node {
            if c == '(' {
                let u = new_node(&mut t, &open);
                open.push(u);
                i += 1;
            } else {
                let u = new_node(&mut t, &open);
                tail(&mut t, u, &mut i)?;
                expect_node = false;
            }
            continue;
        }
        match c {
            ',' if !open.is_empty() => {
                i += 1;
                expect_node = true;
            }
            ')' => {
                let Some(u) = open.pop() else {
                    return Err(malformed(word, i, "a ) with no ( before it"));
                };
                i += 1;
                tail(&mut t, u, &mut i)?;
            }
            ';' if open.is_empty() => {
                i += 1;
                skip(&mut i)?;
                if i < cs.len() {
                    return Err(malformed(word, i, "text after ;"));
                }
                return Ok(t);
            }
            ';' => return Err(malformed(word, i, "a ( is not closed")),
            ',' => return Err(malformed(word, i, "a , outside any ( )")),
            other => {
                return Err(malformed(
                    word,
                    i,
                    &format!("{other} where , ) or ; was expected"),
                ));
            }
        }
    }
}

fn phylo_newick_bang(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "phylo.newick!";
    let v = operands(vm, W, 1)?;
    let s = match v[0].as_str() {
        Some(s) if v[0].dt() == STRING => s,
        _ => return Err(domain(W, "s", v[0].summary(WIDTH))),
    };
    vm.push(parse_newick(W, &s)?.value());
    Ok(())
}

// To a GRAPH.

/// A GRAPH of the tree's nodes, each branch an edge both ways weighted by
/// its length. An unnamed node is named `node<i>`, its index. Names are
/// refused when two Title-case alike, since `graph.*` would merge them.
fn phylo_graph(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "phylo.graph";
    let v = operands(vm, W, 1)?;
    let t = tree(W, &v[0])?;
    let names: Vec<String> = t
        .names
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if s.is_empty() {
                format!("node{i}")
            } else {
                s.clone()
            }
        })
        .collect();
    let mut seen: BTreeMap<String, &str> = BTreeMap::new();
    for name in &names {
        let title = crate::graph::node_name(name);
        if let Some(first) = seen.insert(title.clone(), name) {
            return Err(bad_tree(
                W,
                format!("{first} and {name} both read as {title} in a GRAPH"),
            ));
        }
    }
    let mut edges = Vec::new();
    for (i, &p) in t.parent.iter().enumerate() {
        if let Ok(p) = usize::try_from(p) {
            for (a, b) in [(p, i), (i, p)] {
                edges.push(BundValue::list(vec![
                    BundValue::str(names[a].clone()),
                    BundValue::str(names[b].clone()),
                    BundValue::float(t.length[i]),
                ]));
            }
        }
    }
    let g = BundValue::map(Default::default())
        .set("type", BundValue::str("graph"))
        .set(
            "nodes",
            BundValue::list(names.into_iter().map(BundValue::str).collect()),
        )
        .set("edges", BundValue::list(edges));
    vm.push(g);
    Ok(())
}

/// Register §S14's words, each with its `.` form (§S3).
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
    word!("phylo.dist", phylo_dist, 2);
    word!("phylo.upgma", phylo_upgma, 2);
    word!("phylo.nj", phylo_nj, 2);
    word!("phylo.newick", phylo_newick, 1);
    word!("phylo.newick!", phylo_newick_bang, 1);
    word!("phylo.graph", phylo_graph, 1);
}

/// §S14's words; each also has its `.` form.
pub const WORDS: [&str; 6] = [
    "phylo.dist",
    "phylo.upgma",
    "phylo.nj",
    "phylo.newick",
    "phylo.newick!",
    "phylo.graph",
];

#[cfg(test)]
mod tests;
