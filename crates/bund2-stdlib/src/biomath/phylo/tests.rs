//! RFC-0015 criteria 2, 3, 6 and 17, for §S14.

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::{Tree, WORDS, parse_newick};
use crate::biomath::testkit::{close, err, floats, run};

fn pulled(src: &str) -> BundValue {
    run(src)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
        .pull()
        .unwrap_or_else(|| panic!("{src}: empty"))
}

fn string(src: &str) -> String {
    let v = pulled(src);
    v.as_str().unwrap_or_else(|| panic!("{src}: {v:?}"))
}

/// A tree answer, read back through the checking reader.
fn tree_of(src: &str) -> Tree {
    super::tree("test", &pulled(src)).unwrap_or_else(|e| panic!("{src}: {e:?}"))
}

fn matrix(src: &str) -> Vec<Vec<f64>> {
    let v = pulled(src);
    v.as_matrix()
        .unwrap_or_else(|| panic!("{src}: {v:?}"))
        .iter()
        .map(|r| r.iter().flat_map(floats).collect())
        .collect()
}

/// The five-taxon matrix of criterion 17.
const FIVE: &str = "[ [ 0 5 9 9 8 ] [ 5 0 10 10 9 ] [ 9 10 0 8 7 ] \
                    [ 9 10 8 0 3 ] [ 8 9 7 3 0 ] ] matrix [ \"a\" \"b\" \"c\" \"d\" \"e\" ]";

// Criterion 2.

#[test]
fn every_word_is_registered_with_its_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
        assert!(i.registry.effect_of(&format!("{w}.")).is_some(), "{w}.");
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("phylo."))
        .count();
    assert_eq!(ours, 2 * WORDS.len());
}

// Criterion 17.

#[test]
fn jc69_and_k2p_against_their_closed_forms() {
    // Twenty sites; b differs from a at three: two transitions (A→G) and
    // one transversion (A→C). c has two gaps, so a and c compare 18 sites
    // and differ at none.
    let a = "AAAAAAAAAAAAAAAAAAAA";
    let b = "GGCAAAAAAAAAAAAAAAAA";
    let c = "AAAAAAAAAAAAAAAAAA--";
    let al = format!("[ \"{a}\" \"{b}\" \"{c}\" ]");
    let p = matrix(&format!("{al} \"p\" phylo.dist"));
    assert_eq!(p[0], [0.0, 0.15, 0.0]);
    // p = 0.15: −¾ ln(1 − 4p/3) = −¾ ln 0.8.
    let jc = matrix(&format!("{al} \"jc69\" phylo.dist"));
    close(
        "jc69",
        &[jc[0][1], jc[1][0]],
        &[-0.75 * 0.8f64.ln(); 2],
        1e-15,
    );
    // P = 0.1, Q = 0.05: −½ ln(1 − 2P − Q) − ¼ ln(1 − 2Q).
    let k2p = matrix(&format!("{al} \"k2p\" phylo.dist"));
    close(
        "k2p",
        &[k2p[0][1]],
        &[-0.5 * 0.75f64.ln() - 0.25 * 0.9f64.ln()],
        1e-15,
    );
    assert_eq!(k2p[0][2], 0.0);
    assert_eq!(k2p[1][1], 0.0);
}

#[test]
fn nj_on_the_five_taxa() {
    let t = tree_of(&format!("{FIVE} phylo.nj"));
    assert_eq!(t.names.len(), 8, "{t:?}");
    for (leaf, want) in [("a", 2.0), ("b", 3.0), ("c", 4.0), ("d", 2.0), ("e", 1.0)] {
        let i = t.names.iter().position(|n| n == leaf).unwrap();
        close(leaf, &[t.length[i]], &[want], 1e-12);
    }
    let root = t.root();
    assert_eq!(t.length[root], 0.0);
    let mut inner: Vec<f64> = (5..8).filter(|&i| i != root).map(|i| t.length[i]).collect();
    inner.sort_by(f64::total_cmp);
    close("inner branches", &inner, &[2.0, 3.0], 1e-12);
    // Rooted at the last join, which has three children.
    assert_eq!(t.children()[root].len(), 3);
}

/// Each number after a `:`, rounded to five decimals, trailing zeros off.
fn rounded(newick: &str) -> String {
    let mut out = String::new();
    let mut cs = newick.chars().peekable();
    while let Some(c) = cs.next() {
        out.push(c);
        if c == ':' {
            let mut num = String::new();
            while let Some(&d) = cs.peek() {
                if d.is_ascii_digit() || d == '.' || d == '-' || d == 'e' {
                    num.push(d);
                    cs.next();
                } else {
                    break;
                }
            }
            let x: f64 = num.parse().unwrap();
            let s = format!("{x:.5}");
            out.push_str(s.trim_end_matches('0').trim_end_matches('.'));
        }
    }
    out
}

#[test]
fn upgma_on_the_five_taxa() {
    let s = string(&format!("{FIVE} phylo.upgma phylo.newick"));
    assert_eq!(
        rounded(&s),
        "((a:2.5,b:2.5):2.08333,(c:3.75,(d:1.5,e:1.5):2.25):0.83333);"
    );
}

#[test]
fn newick_round_trips_names_verbatim() {
    let s = "((A:1,'b c':2)x:0.5,'it''s':3,_u_:0,(Seq1:1,seq1:1):1);";
    let t = tree_of(&format!("\"{s}\" phylo.newick!"));
    // Nodes in the order the parser meets them: a parent before its
    // children.
    assert_eq!(
        t.names,
        ["", "x", "A", "b c", "it's", "_u_", "", "Seq1", "seq1"]
    );
    assert_eq!(string(&format!("\"{s}\" phylo.newick! phylo.newick")), s);
    // A tree's own Newick reads back as the same tree, lengths exactly.
    let built = string(&format!("{FIVE} phylo.nj phylo.newick"));
    let again = string(&format!("\"{built}\" phylo.newick! phylo.newick"));
    assert_eq!(again, built);
    // Whitespace and comments between tokens are skipped.
    let t = tree_of("\" ( a : 1 , [a comment] b:2 ) ; \" phylo.newick!");
    assert_eq!(t.names, ["", "a", "b"]);
    assert_eq!(t.length, [0.0, 1.0, 2.0]);
}

#[test]
fn a_newick_100000_deep_parses() {
    let depth = 100_000;
    let s = format!("{}a{};", "(".repeat(depth), ")".repeat(depth));
    let t = parse_newick("test", &s).unwrap();
    assert_eq!(t.names.len(), depth + 1);
    assert_eq!(t.parent[depth], depth as i64 - 1);
    // Through the words, and back out: no recursion on the depth anywhere.
    let mut i = run(&format!("\"{s}\" phylo.newick! dup phylo.newick")).unwrap();
    let written = i.pull().unwrap().as_str().unwrap();
    assert!(written.starts_with("((((") && written.ends_with(":0);"));
    let tree = i.pull().unwrap();
    assert!(super::tree("test", &tree).is_ok());
}

#[test]
fn graph_refuses_names_that_title_case_alike() {
    let e = err("\"(seq1:1,Seq1:2);\" phylo.newick! phylo.graph");
    assert!(
        e.starts_with(
            "PHYLO.GRAPH: E101 tree is outside phylo.graph's domain: seq1 and Seq1 both read as"
        ),
        "{e}"
    );
}

#[test]
fn a_graph_word_runs_over_a_tree() {
    // Alpha to Gamma: Alpha–x 1, x–root 1, root–Gamma 3.
    let src = "\"Alpha\" \"((Alpha:1,beta:2)x:1,Gamma:3);\" phylo.newick! \
               phylo.graph graph.paths";
    let rows = pulled(src);
    let rows = rows.as_list().unwrap_or_else(|| panic!("{rows:?}"));
    let path_to = |node: &str| -> Vec<f64> {
        let row = rows
            .iter()
            .find(|r| r.as_map().and_then(|m| m.get("node")?.as_str()).as_deref() == Some(node))
            .unwrap_or_else(|| panic!("{node}: {rows:?}"));
        floats(&row.as_map().unwrap()["path"])
    };
    assert_eq!(path_to("Gamma"), [5.0]);
    assert_eq!(path_to("Beta"), [3.0]);
}

// Criterion 3.

#[test]
fn each_refusal_is_its_template() {
    let ok = "\"(a:1,b:2);\" phylo.newick!";
    for (src, want) in [
        (
            "[ \"AC\" \"AG\" ] \"f81\" phylo.dist".to_string(),
            "PHYLO.DIST: E101 model is outside phylo.dist's domain: \"f81\"".to_string(),
        ),
        (
            "[ \"AC\" \"CA\" ] \"jc69\" phylo.dist".into(),
            "PHYLO.DIST: E101 alignment is outside phylo.dist's domain: sequences 1 and 2 are saturated under jc69"
                .into(),
        ),
        (
            "[ \"AC\" \"--\" ] \"p\" phylo.dist".into(),
            "PHYLO.DIST: E101 alignment is outside phylo.dist's domain: sequences 1 and 2 share no site"
                .into(),
        ),
        (
            "[ \"AC\" \"ACG\" ] \"p\" phylo.dist".into(),
            "PHYLO.DIST: E102 lengths do not agree: 2 and 3".into(),
        ),
        (
            "[ \"AC\" \"AX\" ] \"p\" phylo.dist".into(),
            "PHYLO.DIST: E109 X at 2 is not in the iupac alphabet".into(),
        ),
        (
            "[ [ 0 1 ] [ 2 0 ] ] matrix [ \"a\" \"b\" ] phylo.upgma".into(),
            "PHYLO.UPGMA: E107 the matrix is not symmetric: [1,2] and [2,1] differ by 1".into(),
        ),
        (
            "[ [ 1 1 ] [ 1 0 ] ] matrix [ \"a\" \"b\" ] phylo.nj".into(),
            "PHYLO.NJ: E101 matrix is outside phylo.nj's domain: 1 at [1,1]".into(),
        ),
        (
            "[ [ 0 1 ] [ 1 0 ] ] matrix [ \"a\" ] phylo.nj".into(),
            "PHYLO.NJ: E102 rows and names do not agree: 2 and 1".into(),
        ),
        (
            "2001 linalg.identity list phylo.nj".into(),
            "PHYLO.NJ: E111 2001 taxa exceeds its cap of 2000".into(),
        ),
        (
            "\"(a,b;\" phylo.newick!".into(),
            "PHYLO.NEWICK!: E113 malformed Newick at 5: a ( is not closed".into(),
        ),
        (
            "\"(a,b));\" phylo.newick!".into(),
            "PHYLO.NEWICK!: E113 malformed Newick at 6: a ) with no ( before it".into(),
        ),
        (
            "\"(a,b)\" phylo.newick!".into(),
            "PHYLO.NEWICK!: E113 malformed Newick at 6: the string ends before ;".into(),
        ),
        (
            "\"(a:x,b);\" phylo.newick!".into(),
            "PHYLO.NEWICK!: E113 malformed Newick at 4: a branch length is not a number".into(),
        ),
        (
            "\"(a,b); c\" phylo.newick!".into(),
            "PHYLO.NEWICK!: E113 malformed Newick at 8: text after ;".into(),
        ),
        (
            format!("{ok} \"parent\" [ -1 -1 0 ] set phylo.newick"),
            "PHYLO.NEWICK: E101 tree is outside phylo.newick's domain: 2 roots, where 1 is needed"
                .into(),
        ),
        (
            format!("{ok} \"parent\" [ -1 2 1 ] set phylo.newick"),
            "PHYLO.NEWICK: E101 tree is outside phylo.newick's domain: a cycle through node 1"
                .into(),
        ),
        (
            format!("{ok} \"length\" [ 0.0 ] set phylo.newick"),
            "PHYLO.NEWICK: E102 lengths do not agree: 3 and 1".into(),
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

// No abort, whatever the operand.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "-1",
        "0",
        "\"\"",
        "\";\"",
        "\"(\"",
        "\")\"",
        "\"'\"",
        "\"jc69\"",
        "list",
        "dict",
        "[ \"AC\" \"GT\" ]",
        "[ [ 0 ] ] matrix",
        "[ [ 0 1 ] [ 1 0 ] ] matrix",
        "dict \"names\" list set \"parent\" list set \"length\" list set",
    ];
    for w in WORDS {
        for x in operands {
            for src in [format!("{x} {w}"), format!("{x} {x} {w}")] {
                if let Err(e) = run(&src) {
                    assert!(
                        e.contains(": E1") || e.contains("too shallow"),
                        "{src}: {e}"
                    );
                }
            }
        }
    }
}

// The `.` forms.

#[test]
fn each_dot_form_answers_on_the_workbench() {
    let (m, names) = FIVE.split_at(FIVE.find("matrix").unwrap() + "matrix".len());
    for (below, top_, w) in [
        (m, names, "phylo.upgma"),
        (m, names, "phylo.nj"),
        ("", "\"(a:1,b:2);\"", "phylo.newick!"),
        ("[ \"ACGTACGTAC\" \"ACGTACGTAT\" ]", "\"k2p\"", "phylo.dist"),
    ] {
        let plain = pulled(&format!("{below} {top_} {w}"));
        let mut i = run(&format!("7 {below} {top_} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        let got = i.pull_workbench().unwrap();
        assert_eq!(format!("{got:?}"), format!("{plain:?}"), "{w}.");
    }
}

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":phylo.newick { 42 } register 1 phylo.newick").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
