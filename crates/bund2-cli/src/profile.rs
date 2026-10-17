//! **RFC-0011 §S5 and §S6: the two renderings of a profile, and the files.**
//!
//! The record is `bund2_interp::profile::Profile`; nothing here changes it.
//! The markdown is written for a reader who did not see the run, human or
//! model: two header tables, the hottest words by exclusive time, the call
//! tree to a capped depth, callers and callees of the hottest words, and an
//! anomalies section that is derived rather than judged. The `dot` file is
//! the aggregated call graph, one node per word and one edge per pair,
//! which `dot -Tsvg` must accept with no preprocessing.
//!
//! **The report says what its clock can and cannot show — D188.** The
//! header prints the clock's cost and its resolution, how many times it was
//! read and what that came to against the wall; every word has a per-call
//! figure, and one under the clock's floor is marked, because its time is
//! the clock's and only its count and its steps are the word's. Steps, the
//! values dispatched while a word was innermost, are exact and the same on
//! every host, and sit beside the time in every table.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use bund2_api::{MethodEffect, Registry, StackEffect, WordKind};
use bund2_interp::profile::{Key, Kind, Node, Profile};

/// How the run ended, as the command line saw it. The record knows which
/// word was innermost when the last failure was raised; this says whether
/// that failure ended the run.
pub enum Ended {
    /// The program ran to its end.
    Normally,
    /// `bund.exit` with this code; the program ran no further.
    Exit(i32),
    /// The program raised and was not caught; the text is the diagnostic's
    /// reason and, where the source gave one, its location.
    Error(String),
    /// The source did not parse, so nothing ran.
    ParseError(String),
}

/// What the header says about the run beyond what the record knows.
pub struct Meta<'a> {
    pub program: &'a str,
    /// `--jit-threshold` was given and is ignored (§S3).
    pub jit_threshold_given: bool,
    pub ended: Ended,
}

/// What the clock is worth on this host, measured once before the run.
#[derive(Debug, Clone, Copy)]
pub struct Clock {
    /// What one reading costs, as a throughput: readings back to back,
    /// averaged.
    pub cost: Duration,
    /// The smallest difference two readings showed: the tick the clock
    /// advances by, or the cost of a reading where the clock is finer than
    /// that.
    pub resolution: Duration,
}

impl Clock {
    /// Below this, a per-call figure is the clock's rather than the word's:
    /// the two readings a call carries, plus one tick of quantisation.
    pub fn floor(self) -> Duration {
        self.cost * 2 + self.resolution
    }
}

/// How many rows the hottest-words table prints before it says "and more".
const HOTTEST_ROWS: usize = 60;
/// How deep the call tree goes.
const TREE_DEPTH: usize = 6;
/// How many of the hottest words get a callers-and-callees section.
const DETAILED: usize = 10;

/// What the clock is worth here, measured now: twenty thousand readings
/// back to back give the cost of one, averaged, and the smallest non-zero
/// step between two of them is the resolution. Printed in the header so a
/// reader can discount the cheapest words by both (§S2, D188).
pub fn calibrate() -> Clock {
    const N: u32 = 20_000;
    let start = Instant::now();
    let mut last = start;
    let mut resolution = Duration::MAX;
    for _ in 0..N {
        let now = Instant::now();
        let step = now.duration_since(last);
        if !step.is_zero() && step < resolution {
            resolution = step;
        }
        last = now;
    }
    let cost = last.duration_since(start) / N;
    // A clock that never advanced in twenty thousand readings has a
    // resolution coarser than the whole loop; say so rather than claim one.
    if resolution == Duration::MAX {
        resolution = last.duration_since(start);
    }
    Clock { cost, resolution }
}

/// Today's date in UTC as `YYYY-MM-DD`, from the system clock, with no
/// calendar crate: Howard Hinnant's civil-from-days, pure integer
/// arithmetic. A system clock before 1970 reads as the epoch.
fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// A markdown table cell: one line, with the pipe escaped.
fn cell(s: &str) -> String {
    s.replace('\n', " ").replace('|', "\\|")
}

fn name_of(n: &Node, reg: &Registry) -> String {
    match &n.key {
        Key::Sym(s, _) => reg.interner.name(*s).to_string(),
        Key::Name(_, Kind::Program) => "⟨program⟩".to_string(),
        Key::Name(name, _) => name.clone(),
    }
}

fn spelled_of(n: &Node, reg: &Registry) -> String {
    n.spelled
        .iter()
        .map(|s| format!("`{}`", reg.interner.name(*s)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The kind column: what the node is, and for a native what `WordKind` it
/// declared, because a `Blocking` word that waits on input or a bus reads
/// as hot on a wall clock and is not (§S2).
fn kind_of(n: &Node) -> String {
    let kind = match n.kind {
        Kind::Program => "program",
        Kind::Native => "native",
        Kind::Method => "method",
        Kind::Conditional => "conditional",
        Kind::Word => "word",
    };
    match n.word_kind {
        Some(WordKind::Blocking) => format!("{kind}, blocking"),
        Some(WordKind::Async) => format!("{kind}, async"),
        Some(WordKind::Sync) | None => kind.to_string(),
    }
}

/// `part` as a percentage of `whole`, both counts.
fn pct_n(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "—".to_string();
    }
    format!("{:.1}", part as f64 / whole as f64 * 100.0)
}

/// A per-call figure: the duration over the count, or nothing for no calls.
fn per_call(d: Duration, calls: u64) -> Duration {
    match u32::try_from(calls) {
        Ok(n) if n > 0 => d / n,
        Ok(_) => Duration::ZERO,
        // More calls than a `u32` holds: the mean is under a nanosecond.
        Err(_) => Duration::ZERO,
    }
}

fn pair(e: StackEffect) -> String {
    if e.opaque {
        "opaque".to_string()
    } else {
        format!("({}, {})", e.consumes, e.produces)
    }
}

/// The effect column: declared for a native, inferred for a word (RFC-0004
/// §S3), the receiver rule for a method, nothing for a runner.
fn effect_of(n: &Node, reg: &Registry) -> String {
    let name = name_of(n, reg);
    match n.kind {
        Kind::Native => reg.effect_of(&name).map(pair).unwrap_or_default(),
        Kind::Word => reg
            .lambda_body(&name)
            .map(|body| pair(bund2_stdlib::check::infer(&body, reg, 0)))
            .unwrap_or_default(),
        Kind::Method => match reg.method_effect(&name) {
            Some(MethodEffect::Peek { produces }) => format!("peek, +{produces}"),
            Some(MethodEffect::Rebuild { below }) => format!("rebuild, {below} beneath"),
            Some(MethodEffect::Consume { below, produces }) => {
                format!("consume {below}, +{produces}")
            }
            Some(MethodEffect::Opaque) => "opaque".to_string(),
            None => String::new(),
        },
        Kind::Conditional | Kind::Program => String::new(),
    }
}

fn dur(d: Duration) -> String {
    let ns = d.as_nanos();
    if ns >= 1_000_000_000 {
        format!("{:.3} s", d.as_secs_f64())
    } else if ns >= 1_000_000 {
        format!("{:.3} ms", d.as_secs_f64() * 1e3)
    } else if ns >= 1_000 {
        format!("{:.1} µs", d.as_secs_f64() * 1e6)
    } else {
        format!("{ns} ns")
    }
}

fn pct(part: Duration, whole: Duration) -> String {
    if whole.is_zero() {
        return "—".to_string();
    }
    format!("{:.1}", part.as_secs_f64() / whole.as_secs_f64() * 100.0)
}

fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Non-root nodes, hottest first, by exclusive time then name.
fn ranked<'a>(p: &'a Profile, reg: &Registry) -> Vec<(usize, &'a Node)> {
    let mut v: Vec<(usize, &Node)> = p.nodes().iter().enumerate().skip(1).collect();
    v.sort_by(|a, b| {
        b.1.exclusive
            .cmp(&a.1.exclusive)
            .then_with(|| name_of(a.1, reg).cmp(&name_of(b.1, reg)))
    });
    v
}

/// An edge as the renderers read it: the other end, the native it went
/// `via` if any, the count and the time.
struct Hop {
    other: usize,
    via: Option<usize>,
    calls: u64,
    time: Duration,
}

fn via_text(p: &Profile, reg: &Registry, via: Option<usize>) -> String {
    via.map(|v| format!(" via `{}`", name_of(&p.nodes()[v], reg)))
        .unwrap_or_default()
}

/// Edges out of a node, heaviest first.
fn children_of(p: &Profile, reg: &Registry, from: usize) -> Vec<Hop> {
    let mut v: Vec<Hop> = p
        .edges()
        .filter(|(a, _, _, _)| *a == from)
        .map(|(_, b, via, e)| Hop {
            other: b,
            via,
            calls: e.calls,
            time: e.time,
        })
        .collect();
    v.sort_by(|a, b| {
        b.time
            .cmp(&a.time)
            .then_with(|| b.calls.cmp(&a.calls))
            .then_with(|| name_of(&p.nodes()[a.other], reg).cmp(&name_of(&p.nodes()[b.other], reg)))
    });
    v
}

/// Edges into a node, most calls first.
fn callers_of(p: &Profile, reg: &Registry, to: usize) -> Vec<Hop> {
    let mut v: Vec<Hop> = p
        .edges()
        .filter(|(_, b, _, _)| *b == to)
        .map(|(a, _, via, e)| Hop {
            other: a,
            via,
            calls: e.calls,
            time: e.time,
        })
        .collect();
    v.sort_by(|a, b| {
        b.calls
            .cmp(&a.calls)
            .then_with(|| name_of(&p.nodes()[a.other], reg).cmp(&name_of(&p.nodes()[b.other], reg)))
    });
    v
}

/// The `ended` cell: how the run ended and, when it ended in a failure,
/// the word that was innermost when the failure was raised.
fn ended_of(p: &Profile, reg: &Registry, meta: &Meta<'_>) -> String {
    let failing = p
        .failing()
        .map(|id| format!(" in `{}`", name_of(&p.nodes()[id], reg)))
        .unwrap_or_default();
    match &meta.ended {
        Ended::Normally => "normally".to_string(),
        Ended::Exit(code) => format!("`bund.exit` {code}{failing}"),
        Ended::Error(what) => format!("error{failing}: {}", cell(what)),
        Ended::ParseError(what) => format!("parse error, nothing ran: {}", cell(what)),
    }
}

/// §S5's markdown.
pub fn render_markdown(p: &Profile, reg: &Registry, meta: &Meta<'_>) -> String {
    let wall = p.wall.unwrap_or_default();
    let nodes = p.nodes();
    let calls: u64 = nodes.iter().skip(1).map(|n| n.calls).sum();
    let clock = Clock {
        cost: p.reading_cost,
        resolution: p.resolution,
    };
    let floor = clock.floor();
    let overhead = p.reading_cost * u32::try_from(p.readings).unwrap_or(u32::MAX);
    let mut out = String::new();
    let _ = writeln!(out, "# bund2 profile\n");
    let _ = writeln!(
        out,
        "| program | ended | tier | console | wall | steps | calls |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|");
    let _ = writeln!(
        out,
        "| {} | {} | declined (profiling){} | {} | {} | {} | {} |\n",
        cell(meta.program),
        ended_of(p, reg, meta),
        if meta.jit_threshold_given {
            ", jit-threshold ignored"
        } else {
            ""
        },
        if p.console_attached {
            "attached during the run"
        } else {
            "none"
        },
        dur(wall),
        thousands(p.steps()),
        thousands(calls),
    );
    let _ = writeln!(
        out,
        "| bund2 | date | clock cost / reading | clock resolution | clock readings | clock overhead | % of wall | per-call floor |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|");
    let _ = writeln!(
        out,
        "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
        env!("CARGO_PKG_VERSION"),
        today_utc(),
        dur(p.reading_cost),
        dur(p.resolution),
        thousands(p.readings),
        dur(overhead),
        pct(overhead, wall),
        dur(floor),
    );
    let _ = writeln!(
        out,
        "Percentages are of wall time, measured from the record's creation to the \
         program's end. The time inside no word at all is the program's own: {} \
         ({} %). **The clock is in the figures.** A word's time carries two \
         readings per call, about {} each, and every span is quantised to the \
         clock's resolution of {}; the {} readings this record took come to about \
         {}, {} % of the wall, and that time is inside the words' figures, not \
         beside them. A per-call figure under the floor of {} is marked \
         *(floor)*: it is the clock's, not the word's, and only the word's calls \
         and steps are to be trusted. **Steps are exact.** A step is one value the \
         interpreter dispatched; a word's steps are those dispatched while it was \
         the innermost named word open, so a native's are zero and the step that \
         named it is its caller's. Steps do not depend on the host or the clock, \
         and two profiles of one program agree on them to the step. `min` and \
         `max` are of one outermost span; a nested activation of a recursive word \
         lies inside an outer one and is counted in `calls` alone.\n",
        dur(nodes[0].exclusive),
        pct(nodes[0].exclusive, wall),
        dur(p.reading_cost),
        dur(p.resolution),
        thousands(p.readings),
        dur(overhead),
        pct(overhead, wall),
        dur(floor),
    );

    // --- the hottest words ---------------------------------------------------
    let _ = writeln!(out, "## Hottest words, by exclusive time\n");
    let _ = writeln!(
        out,
        "| word | spelled | kind | effect | calls | steps | % of steps | exclusive | % of wall | per call | inclusive | % of wall | min | max |"
    );
    let _ = writeln!(
        out,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    let ranked = ranked(p, reg);
    let total_steps = p.steps();
    for (_, n) in ranked.iter().take(HOTTEST_ROWS) {
        let each = per_call(n.exclusive, n.calls);
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {}{} | {} | {} | {} | {} |",
            name_of(n, reg),
            spelled_of(n, reg),
            kind_of(n),
            effect_of(n, reg),
            thousands(n.calls),
            thousands(n.steps),
            pct_n(n.steps, total_steps),
            dur(n.exclusive),
            pct(n.exclusive, wall),
            dur(each),
            if each < floor { " *(floor)*" } else { "" },
            dur(n.inclusive),
            pct(n.inclusive, wall),
            dur(if n.min == Duration::MAX {
                Duration::ZERO
            } else {
                n.min
            }),
            dur(n.max),
        );
    }
    if ranked.len() > HOTTEST_ROWS {
        let _ = writeln!(
            out,
            "\nand {} more, each under {} of exclusive time.",
            ranked.len() - HOTTEST_ROWS,
            dur(ranked[HOTTEST_ROWS].1.exclusive)
        );
    }
    out.push('\n');

    // --- the call tree ------------------------------------------------------
    let _ = writeln!(
        out,
        "## Call tree (depth ≤ {TREE_DEPTH}, repeated siblings collapsed)\n"
    );
    let _ = writeln!(out, "- ⟨program⟩ {}", dur(wall));
    let mut path = vec![0usize];
    tree(p, reg, 0, 1, &mut path, &mut out);
    out.push('\n');

    // --- callers and callees -------------------------------------------------
    let _ = writeln!(out, "## Callers and callees of the hottest words\n");
    for (id, n) in ranked.iter().take(DETAILED) {
        let _ = writeln!(out, "### `{}`\n", name_of(n, reg));
        let callers = callers_of(p, reg, *id);
        if !callers.is_empty() {
            let _ = writeln!(out, "called by:");
            for h in callers {
                let _ = writeln!(
                    out,
                    "- `{}` ×{}{}{}",
                    name_of(&nodes[h.other], reg),
                    thousands(h.calls),
                    if h.other == *id {
                        String::new()
                    } else {
                        format!(", {}", dur(h.time))
                    },
                    via_text(p, reg, h.via)
                );
            }
        }
        let callees = children_of(p, reg, *id);
        if !callees.is_empty() {
            let _ = writeln!(out, "\ncalls:");
            for h in callees {
                let _ = writeln!(
                    out,
                    "- `{}` ×{}{}{}",
                    name_of(&nodes[h.other], reg),
                    thousands(h.calls),
                    if h.other == *id {
                        String::new()
                    } else {
                        format!(", {}", dur(h.time))
                    },
                    via_text(p, reg, h.via)
                );
            }
        }
        out.push('\n');
    }

    // --- anomalies ------------------------------------------------------------
    let _ = writeln!(out, "## Anomalies\n");
    let mut any = false;
    for (_, n) in &ranked {
        let name = name_of(n, reg);
        if n.max_depth > 1 {
            let _ = writeln!(out, "- `{name}` is recursive to depth {}.", n.max_depth);
            any = true;
        }
        // Over outermost activations only: a nested span of a recursive
        // word lies inside its outer one, so comparing the outermost with a
        // mean over every activation said something false of every
        // recursive word (F214).
        if n.outer >= 2 && !wall.is_zero() {
            let mean = per_call(n.inclusive, n.outer);
            if n.max > mean * 10 && n.max.as_secs_f64() / wall.as_secs_f64() >= 0.01 {
                let _ = writeln!(
                    out,
                    "- `{name}`: one outermost call holds {} % of wall time (max {}, mean {} over {} outermost calls).",
                    pct(n.max, wall),
                    dur(n.max),
                    dur(mean),
                    thousands(n.outer)
                );
                any = true;
            }
        }
        // A word under the clock's floor that still holds a visible share of
        // the wall: the share is the clock's.
        if n.calls > 0
            && !wall.is_zero()
            && per_call(n.exclusive, n.calls) < floor
            && n.exclusive.as_secs_f64() / wall.as_secs_f64() >= 0.01
        {
            let _ = writeln!(
                out,
                "- `{name}` holds {} % of wall at {} a call, under the clock's floor of {}: that share is the clock's cost around {} calls, not the word's.",
                pct(n.exclusive, wall),
                dur(per_call(n.exclusive, n.calls)),
                dur(floor),
                thousands(n.calls)
            );
            any = true;
        }
        if n.calls == 1 && !wall.is_zero() && n.exclusive.as_secs_f64() / wall.as_secs_f64() >= 0.5
        {
            let _ = writeln!(
                out,
                "- `{name}` was called once and holds {} % of wall time by itself.",
                pct(n.exclusive, wall)
            );
            any = true;
        }
    }
    let mut redefined: Vec<&String> = p.redefined.iter().collect();
    redefined.sort();
    for name in redefined {
        let _ = writeln!(
            out,
            "- `{name}` was registered more than once; its figures are one node."
        );
        any = true;
    }
    if p.console_attached {
        let _ = writeln!(
            out,
            "- A debugger console was attached during the run: the tier was off for that \
             reason too, and a stop's wall time lies in whichever word was open."
        );
        any = true;
    }
    if !any {
        let _ = writeln!(out, "- none derived.");
    }
    out
}

fn tree(
    p: &Profile,
    reg: &Registry,
    from: usize,
    depth: usize,
    path: &mut Vec<usize>,
    out: &mut String,
) {
    if depth > TREE_DEPTH {
        return;
    }
    for h in children_of(p, reg, from) {
        let indent = "  ".repeat(depth);
        let name = name_of(&p.nodes()[h.other], reg);
        let recursive = path.contains(&h.other);
        let _ = writeln!(
            out,
            "{indent}- `{name}` ×{}{}{}{}",
            thousands(h.calls),
            if recursive {
                String::new()
            } else {
                format!(" {}", dur(h.time))
            },
            via_text(p, reg, h.via),
            if recursive { " (recursive)" } else { "" }
        );
        if recursive {
            continue;
        }
        path.push(h.other);
        tree(p, reg, h.other, depth + 1, path, out);
        path.pop();
    }
}

fn dot_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// §S5's dot file: one node per word, two clusters, the root as a box, an
/// edge label of count and time, or count alone on a self edge.
pub fn render_dot(p: &Profile, reg: &Registry) -> String {
    let wall = p.wall.unwrap_or_default();
    let nodes = p.nodes();
    let mut out = String::new();
    let _ = writeln!(out, "digraph profile {{");
    let _ = writeln!(out, "  rankdir=LR;");
    let _ = writeln!(out, "  node [shape=ellipse, fontname=\"Helvetica\"];");
    let _ = writeln!(
        out,
        "  n0 [shape=box, label=\"{}\\n{}\"];",
        dot_escape(&name_of(&nodes[0], reg)),
        dur(wall)
    );
    for (cluster, label, kinds) in [
        (
            "natives",
            "natives",
            &[Kind::Native, Kind::Method, Kind::Conditional][..],
        ),
        ("words", "words", &[Kind::Word][..]),
    ] {
        let _ = writeln!(out, "  subgraph cluster_{cluster} {{");
        let _ = writeln!(out, "    label=\"{label}\";");
        for (id, n) in nodes.iter().enumerate().skip(1) {
            if !kinds.contains(&n.kind) {
                continue;
            }
            let share = if wall.is_zero() {
                0.0
            } else {
                n.exclusive.as_secs_f64() / wall.as_secs_f64()
            };
            // White at nothing, a full red at half the run or more.
            let level = (255.0 - (share * 2.0).min(1.0) * 160.0) as u8;
            let _ = writeln!(
                out,
                "    n{id} [label=\"{}\\n{} calls\\n{} self\", style=filled, fillcolor=\"#ff{level:02x}{level:02x}\"];",
                dot_escape(&name_of(n, reg)),
                thousands(n.calls),
                dur(n.exclusive)
            );
        }
        let _ = writeln!(out, "  }}");
    }
    let mut edges: Vec<(usize, usize, Option<usize>, u64, Duration)> = p
        .edges()
        .map(|(a, b, via, e)| (a, b, via, e.calls, e.time))
        .collect();
    edges.sort_by_key(|x| (x.0, x.1, x.2));
    for (a, b, via, calls, time) in edges {
        let label = if a == b {
            thousands(calls)
        } else {
            format!("{} · {}", thousands(calls), dur(time))
        };
        let via_label = via
            .map(|v| format!(" via {}", dot_escape(&name_of(&nodes[v], reg))))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "  n{a} -> n{b} [label=\"{label}{via_label}\"{}];",
            if via.is_some() { ", style=dashed" } else { "" }
        );
    }
    let _ = writeln!(out, "}}");
    out
}

/// Write `text` at `path` as `bund2 build` writes: staged beside the target,
/// then renamed over it, so nothing is ever half-written at the name (F181).
fn place_text(path: &str, text: &str) -> Result<(), String> {
    use std::path::Path;
    let dest = Path::new(path);
    let Some(name) = dest.file_name() else {
        return Err(format!("{path} names no file"));
    };
    let parent = match dest.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let staged = parent.join(format!(
        ".bund2-profile-{}-{}",
        std::process::id(),
        name.to_string_lossy()
    ));
    let done = std::fs::write(&staged, text)
        .map_err(|e| format!("writing {}: {e}", staged.display()))
        .and_then(|()| {
            std::fs::rename(&staged, dest).map_err(|e| format!("moving the file to {path}: {e}"))
        });
    if done.is_err() {
        let _ = std::fs::remove_file(&staged);
    }
    done
}

/// §S6 and D187: the `dot` file first and the markdown last, so the
/// markdown's presence means both are there. Answers the two paths written.
pub fn write_both(base: &str, dot: &str, md: &str) -> Result<(String, String), String> {
    let dot_path = format!("{base}.profile.dot");
    let md_path = format!("{base}.profile.md");
    place_text(&dot_path, dot).map_err(|e| format!("--profile: {e}. No profile was written."))?;
    place_text(&md_path, md).map_err(|e| {
        format!("--profile: {e}. The graph is at {dot_path}; the report was not written.")
    })?;
    Ok((dot_path, md_path))
}
