//! **The workbench half of each native's effect — RFC-0004 §S1.**
//!
//! A `.` word takes its primary operand from the workbench (D24), and where
//! its answer goes is not uniform across the family (F73), so the half is
//! measured rather than derived from the name. [`MEASURED`] is what
//! `cargo xtask arity`'s two-axis probe observed against the oracle, the word
//! run with the workbench seeded (`docs/arity.md`, columns `wb in` and
//! `wb out`), kept only where the probe's main-stack pair is the one Bund2
//! declares — a row whose two axes were not both confirmed is not one to
//! trust on either. `the_table_is_what_the_probe_measured` holds the two
//! together.
//!
//! Every other `.` native is [`Bench::Unknown`]: nobody has measured it, and
//! `bund2 check` stops counting the workbench at one rather than guessing.
//! `format.` is among them on purpose: its stack half depends on its
//! template, as `format`'s does (D178).

use bund2_api::{Bench, Registry};

/// `(word, taken from the workbench, left on it)`, as probed.
pub const MEASURED: &[(&str, u8, u8)] = &[
    ("**.", 1, 1),
    ("*+.", 1, 1),
    ("*-.", 1, 1),
    ("*.", 1, 1),
    ("*/.", 1, 1),
    ("+.", 1, 1),
    ("-.", 1, 1),
    ("/.", 1, 1),
    ("at.", 2, 1),
    ("car.", 1, 1),
    ("cdr.", 1, 1),
    ("convert.from_html.", 1, 1),
    ("convert.to_bool.", 1, 1),
    ("convert.to_dict.", 1, 1),
    ("convert.to_float.", 1, 1),
    ("convert.to_int.", 1, 1),
    ("convert.to_list.", 1, 1),
    ("convert.to_matrix.", 1, 1),
    ("convert.to_string.", 1, 1),
    ("convert.to_textbuffer.", 1, 1),
    // The sample pass's: a value `encode.base64` made.
    ("decode.base64.", 1, 1),
    ("encode.base64.", 1, 1),
    ("graph.", 1, 1),
    ("head.", 2, 1),
    ("io.banner.", 1, 0),
    // The sample pass's: a list of floats; the drawing goes to the stack.
    ("io.graph.", 1, 0),
    ("make.call.", 1, 1),
    ("math.max.", 1, 1),
    ("math.min.", 1, 1),
    ("math.normalize.", 1, 1),
    ("math.smoothing.", 1, 1),
    ("merge.", 1, 1),
    ("print.", 1, 0),
    ("println.", 1, 0),
    ("push.", 1, 1),
    // The sample pass's: a pipe made by a `send`, its name on the workbench.
    ("recv.", 1, 0),
    ("send.", 1, 0),
    ("send.quick.", 1, 0),
    ("sort.", 1, 1),
    ("stat.count.", 1, 1),
    ("stat.mean.arithmetic.", 1, 1),
    ("string.deunicode.", 1, 1),
    ("string.distance.", 1, 1),
    ("string.distance.dameraulevenshtein.", 1, 1),
    ("string.distance.hamming.", 1, 1),
    ("string.distance.jarowinkler.", 1, 1),
    ("string.distance.levenshtein.", 1, 1),
    ("string.distance.sift3.", 1, 1),
    // The sample pass's: the expression on the workbench, the text on the
    // stack, the answer back on the workbench.
    ("string.expressionmatch.", 1, 1),
    ("string.fuzzymatch.", 1, 1),
    ("string.prefix.", 1, 0),
    ("string.regex.", 1, 0),
    ("string.regex.matches.", 1, 0),
    ("string.regex.split.", 1, 0),
    ("string.suffix.", 1, 0),
    ("string.tokenize.", 1, 0),
    ("string.tokenize.lines.", 1, 0),
    ("string.tokenize.stemmed.", 1, 0),
    ("string.tokenize.unique.", 1, 0),
    ("string.wildcard.", 1, 0),
    // The mixed pass's: the string on the workbench, the width on the stack.
    ("string.wrap.english.", 1, 1),
    ("system.path.filename.", 1, 1),
    ("system.path.split.", 1, 1),
    ("tail.", 2, 1),
    ("unfold.", 1, 1),
    ("unique.", 1, 1),
];

/// The four that reach the workbench without a `.`, read from their source.
/// `return` (`.` is its alias) moves the stack's top there and `take` moves
/// the workbench's top back, each refusing when there is nothing to move
/// (`reference/rust_multistack/src/ts_workbench.rs:29-35,45-51`);
/// `return_from` and `return_to` do the same between the workbench and a
/// stack named by their operand (`:37-43,53-58`).
const SOURCED: &[(&str, u8, u8)] = &[
    ("return", 0, 1),
    ("take", 1, 0),
    ("return_from", 0, 1),
    ("return_to", 1, 0),
];

/// RFC-0012's twelve graph words' forms, Bund2's own, so there is nothing to
/// measure and nothing for the oracle to say. Their half is known by how they
/// are built: `bench_form` takes the graph from the workbench and moves what
/// the plain word left there, and each plain word leaves exactly one answer
/// (criterion 1, `each_form_holds_the_graph_on_the_workbench`). The fifth
/// RFC-0012 review's finding 4: declared `Unknown` before, `check` could not
/// see a form run with no graph on the workbench.
const BUILT: &[&str] = &[
    "graph.bfs.",
    "graph.dfs.",
    "graph.reachable.",
    "graph.neighbors.",
    "graph.degree.",
    "graph.toposort.",
    "graph.cycle.",
    "graph.components.",
    "graph.scc.",
    "graph.mst.",
    "graph.bellmanford.",
    "graph.maxflow.",
];

/// Two `.` words that read the main stack and never the workbench, from
/// their source: `for.` pulls its body and its condition with
/// `vm.stack.pull()` (`reference/rust_multistackvm/src/stdlib/logic/for_fun.rs:56,62`),
/// and `*loop.` passes `StackOps::FromStack`
/// (`reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:116`, F78).
const STACK_ONLY: &[&str] = &["for.", "*loop."];

/// Declare every workbench half. **Last of all registration**, so a stub
/// `--noio` or `--noeval` put in a native's place is declared too.
pub fn declare(r: &mut Registry) {
    for &(word, consumes, produces) in MEASURED.iter().chain(SOURCED) {
        r.set_bench(word, Bench::Fixed { consumes, produces });
    }
    for word in BUILT {
        r.set_bench(
            word,
            Bench::Fixed {
                consumes: 1,
                produces: 1,
            },
        );
    }
    let unmeasured: Vec<String> = r
        .interner
        .names()
        .iter()
        .filter(|n| n.len() > 1 && n.ends_with('.') && !STACK_ONLY.contains(&n.as_str()))
        .filter(|n| r.effect_of(n).is_some_and(|e| e.bench == Bench::Untouched))
        .cloned()
        .collect();
    for n in unmeasured {
        r.set_bench(&n, Bench::Unknown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The table is the probe's, row for row** — criterion 3's
    /// cross-check, for the second axis. Every row of `MEASURED` is a row of
    /// `docs/arity.md` with the same `wb in` and `wb out`, so regenerating
    /// the table and forgetting this file, or the reverse, fails here.
    #[test]
    fn the_table_is_what_the_probe_measured() {
        let doc = include_str!("../../../docs/arity.md");
        for &(word, consumes, produces) in MEASURED {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| `{word}` |")))
                .unwrap_or_else(|| panic!("{word} is not in docs/arity.md"));
            let cells: Vec<&str> = row.split('|').map(str::trim).collect();
            assert_eq!(
                (cells[9], cells[10]),
                (consumes.to_string().as_str(), produces.to_string().as_str()),
                "{word}: {row}"
            );
        }
    }

    /// Declared on the registry the CLI builds: a measured word has its pair,
    /// an unmeasured `.` word is unknown, and a word with no `.` is untouched.
    #[test]
    fn the_halves_are_declared() {
        let mut i = bund2_interp::Interp::new();
        crate::register_all(&mut i.registry);
        let bench = |w: &str| i.registry.effect_of(w).map(|e| e.bench);
        assert_eq!(
            bench("+."),
            Some(Bench::Fixed {
                consumes: 1,
                produces: 1
            })
        );
        assert_eq!(
            bench("println."),
            Some(Bench::Fixed {
                consumes: 1,
                produces: 0
            })
        );
        assert_eq!(
            bench("take"),
            Some(Bench::Fixed {
                consumes: 1,
                produces: 0
            })
        );
        assert_eq!(
            bench("."),
            Some(Bench::Fixed {
                consumes: 0,
                produces: 1
            })
        );
        assert_eq!(bench("format."), Some(Bench::Unknown));
        assert_eq!(bench("+"), Some(Bench::Untouched));
        for w in BUILT {
            assert_eq!(
                bench(w),
                Some(Bench::Fixed {
                    consumes: 1,
                    produces: 1
                }),
                "{w}"
            );
        }
    }
}
