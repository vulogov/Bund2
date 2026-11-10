//! RFC-0014 M14: `smysl.ingest.text`, criterion 27, and criteria 22, 23 and
//! 28's `smysl.ingest.text` cases.
//!
//! Every provider is `bund2-script` (§P8), with a context window small
//! enough that the fixture's three paragraphs are three windows. Each window
//! makes two first calls, one per pass (`Passes::TEXT`).

#![cfg(feature = "smysl-model-probe")]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bund2_value::BundValue;
use smysl_core::{AgentId, Op, Record, Rung};
use smysl_ingest::text::Journal;
use smysl_ingest::text::window::leaf_level;
use smysl_provider::ProviderError;
use smysl_text::{AddSpec, Caps, Input};

use super::forming_tests::{Dir, Run, get, m, s, text};
use super::script::{self, Answer, Probing};
use super::staged_tests::{Project, endpoint, opts};
use super::write::LEFT_LOCKED;

const MARKED: &str = "the library is locked: bund2 smysl.ingest.text is writing it in this process";

/// One provider, `content-ingest` routed to it, its context small enough
/// that a window holds one of the fixture's paragraphs.
fn config(endpoint: &str) -> String {
    format!(
        r#"{{
  providers: {{
    s: {{
      kind: bund2-script
      endpoint: "{endpoint}"
      model: "script-1"
      context_window: 1024
      max_output: 256
      structured: json-schema
    }}
  }}
  routing: {{ content-ingest: s }}
}}"#
    )
}

/// A numbered paragraph that fills one window: SMYSL makes one window of
/// each of three such paragraphs at this context, two of shorter ones, and
/// splits longer ones (`smysl-ingest/src/text/window.rs:221-281`).
fn paragraph(n: usize) -> String {
    let mut p = format!("Paragraph {n} says that the pool wait rose.");
    for i in 0..12 {
        p.push_str(&format!(" On day {i} of week {n} the queue grew and the rollback held."));
    }
    p
}

/// Three paragraphs of Markdown, under a heading.
fn markdown() -> String {
    format!(
        "# Incident\n\n{}\n\n{}\n\n{}\n",
        paragraph(1),
        paragraph(2),
        paragraph(3)
    )
}

/// A project whose library holds `body`, read by `reader`, as `incident`,
/// and whose sidecar routes to a fresh script.
struct Text {
    p: Project,
    ep: String,
}

impl Text {
    fn new(tag: &str, reader: &str, body: &str) -> Text {
        let p = Project::new(tag);
        let ep = endpoint("localhost", tag);
        p.sidecar(&config(&ep));
        let mut lib = p.open();
        lib.add(
            &Input::new(body.as_bytes()),
            &AddSpec::new(reader, "incident", "CC0-1.0")
                .with_lang(smysl_core::ids::LangTag::new("en").unwrap()),
            &Caps::DEFAULT,
        )
        .unwrap();
        Text { p, ep }
    }

    fn md(tag: &str) -> Text {
        Text::new(tag, "md/1", &markdown())
    }

    fn run(&self, r: &mut Run, o: BundValue) -> Result<BundValue, String> {
        r.call(vec![self.p.handle(), s("incident"), o], "smysl.ingest.text")
    }

    fn journal(&self) -> usize {
        Journal::open(&self.p.root).len()
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        script::disarm(&self.ep);
    }
}

/// A model's answer for one call: one speculative claim, spanless.
fn claim(label: &str) -> Answer {
    Answer::Text(format!(
        r#"{{"units":[{{"type":"claim","label":"{label}","gist":"the queue grew in {label}","status":"speculative"}}]}}"#
    ))
}

/// A model's answer with no units.
fn nothing() -> Answer {
    Answer::Text(r#"{"units":[]}"#.to_string())
}

/// Two answers per window, for `windows` windows.
fn answers(windows: usize) -> Vec<Answer> {
    (1..=windows)
        .flat_map(|w| [claim(&format!("c/w{w}")), claim(&format!("c/w{w}"))])
        .collect()
}

fn n(v: &BundValue, key: &str) -> i64 {
    get(v, key).as_int().unwrap()
}

/// Wait until `flag` exists, or fail the test after 30 s.
fn arrived(flag: &Path) {
    let start = Instant::now();
    while !flag.exists() {
        assert!(start.elapsed() < Duration::from_secs(30), "never paused");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// `smysl.unit` from a fresh VM, as another thread's writer; the uid it
/// answers.
fn unit(root: &str) -> Result<String, String> {
    let mut r = Run::new(false);
    let h = m(&[
        ("type", s("smysl")),
        ("root", s(root)),
        ("agent", s("tool:other")),
    ]);
    let u = m(&[
        ("schema", s("claim")),
        ("gist", s("another thread's unit")),
        ("status", s("speculative")),
    ]);
    r.call(vec![h, u], "smysl.unit").map(|v| text(&v))
}

/// `smysl.ingest.text` of `incident` on another thread, from a fresh VM, as
/// `tool:bund2`; the report's `windows`.
fn ingest_elsewhere(root: &str) -> std::thread::JoinHandle<Result<i64, String>> {
    let root = root.to_string();
    std::thread::spawn(move || {
        let mut r = Run::new(false);
        let h = m(&[
            ("type", s("smysl")),
            ("root", s(&root)),
            ("agent", s("tool:bund2")),
        ]);
        r.call(vec![h, s("incident"), opts(&[])], "smysl.ingest.text")
            .map(|v| n(&v, "windows"))
    })
}

fn flag_in(tag: &str) -> (Dir, PathBuf) {
    let d = Dir::fresh(tag);
    std::fs::create_dir_all(&d.0).unwrap();
    let f = d.0.join("arrived");
    (d, f)
}

// --- criterion 27 ----------------------------------------------------------------

/// Three windows, written and journalled; a rerun skips them all with one
/// probe and no call; `reextract` extracts them again. The run's units are
/// SMYSL's own, `Imported` by the handle's agent at rung `Document`, at one
/// clock later than the agent's last.
#[test]
fn a_text_ingest_writes_journals_and_resumes() {
    let t = Text::md("m14-resume");
    script::arm(&t.ep, answers(3));
    let mut r = Run::new(false);
    let v = t.run(&mut r, opts(&[])).unwrap();
    assert_eq!(n(&v, "windows"), 3, "{v:?}");
    assert_eq!(n(&v, "skipped"), 0);
    assert_eq!(n(&v, "failed"), 0);
    assert_eq!(n(&v, "passes"), 2);
    assert_eq!(text(&get(&v, "alias")), "incident");
    assert_eq!(t.journal(), 3);
    assert!(n(&v, "units") >= 3, "{v:?}");

    let lib = t.p.open();
    let agent = AgentId::new("tool:bund2").unwrap();
    let mine: Vec<_> = lib
        .store()
        .iter()
        .filter_map(|r| match r {
            Record::Attestation(a) if a.agent == agent => Some(a.clone()),
            _ => None,
        })
        .collect();
    assert!(!mine.is_empty());
    for a in &mine {
        assert_eq!(a.op, Op::Imported);
        assert_eq!(a.rung, Rung::Document);
        assert_eq!(a.ts, mine[0].ts, "one clock for the run");
    }
    drop(lib);
    assert!(super::unlock::unlock(&t.p.root, false).unwrap().is_empty());

    let calls = script::calls(&t.ep).len();
    let probes = script::probes(&t.ep);
    let again = t.run(&mut r, opts(&[])).unwrap();
    assert_eq!(n(&again, "skipped"), 3);
    assert_eq!(script::calls(&t.ep).len(), calls, "no completion");
    assert_eq!(script::probes(&t.ep), probes + 1, "one probe");

    script::queue(&t.ep, answers(3));
    let re = t
        .run(&mut r, opts(&[("reextract", BundValue::boolean(true))]))
        .unwrap();
    assert_eq!(n(&re, "windows"), 3);
    assert_eq!(n(&re, "skipped"), 0);
    assert_eq!(script::calls(&t.ep).len(), calls + 6);
}

/// The default level is `leaf_level`'s over the first part's structure,
/// for a `txt/1` and an `md/1` text: a run naming that level has the same
/// recipe, and one naming another level does not.
#[test]
fn the_default_level_is_the_readings_leaves() {
    for (reader, body) in [
        ("txt/1", "the pool wait rose\nthe queue grew\nthe rollback held\n".to_string()),
        ("md/1", markdown()),
    ] {
        let tag = format!("m14-level-{}", reader.replace('/', ""));
        let t = Text::new(&tag, reader, &body);
        let lib = t.p.open();
        let mid = lib.head("incident").unwrap();
        let tid = lib.store().manifest(&mid).unwrap().parts[0].tid;
        let leaves = leaf_level(&lib.structure(&mid, &tid, &Caps::DEFAULT).unwrap()).unwrap();
        drop(lib);
        let mut r = Run::new(false);
        let mut recipe = |o: BundValue| {
            script::queue(&t.ep, (0..16).map(|_| nothing()));
            text(&get(&t.run(&mut r, o).unwrap(), "recipe"))
        };
        let default = recipe(opts(&[]));
        let named = recipe(opts(&[
            ("level", s(leaves.as_str())),
            ("reextract", BundValue::boolean(true)),
        ]));
        assert_eq!(default, named, "{reader}: {leaves}");
        let other = if leaves.as_str() == "line" { "paragraph" } else { "line" };
        let elsewhere = recipe(opts(&[
            ("level", s(other)),
            ("reextract", BundValue::boolean(true)),
        ]));
        assert_ne!(default, elsewhere, "{reader}");
    }
}

/// While a run is paused inside a call, another thread's `smysl.unit` and
/// a second text ingest are SM07 naming the run, without waiting, and
/// another process is refused by SMYSL's locks.
#[test]
fn a_running_text_ingest_refuses_other_writers() {
    let t = Text::md("m14-hold");
    let (_flags, flag) = flag_in("m14-hold-flag");
    let mut first = vec![Answer::Pause(flag.clone())];
    first.extend(answers(3));
    script::arm(&t.ep, first);
    let root = t.p.root();
    let run = ingest_elsewhere(&root);
    arrived(&flag);

    let r2 = root.clone();
    let e = std::thread::spawn(move || unit(&r2)).join().unwrap().unwrap_err();
    assert_eq!(e, format!("SMYSL.UNIT: {MARKED}"));
    let e = ingest_elsewhere(&root).join().unwrap().unwrap_err();
    assert_eq!(e, format!("SMYSL.INGEST.TEXT: {MARKED}"));
    // Another process meets SMYSL's own locks, which the run holds.
    let lib = t.p.open();
    let other = smysl_text::lock::acquire_all(&t.p.root, &lib.shards(), "another process");
    assert!(
        matches!(other, Err(smysl_core::error::LibError::Locked { .. })),
        "{other:?}"
    );
    drop(lib);

    std::fs::remove_file(&flag).unwrap();
    assert_eq!(run.join().unwrap().unwrap(), 3);
    unit(&root).unwrap();
}

/// A window whose answer has no units is not a failed commit: the run
/// answers, and leaves no lock.
#[test]
fn a_window_with_no_units_leaves_no_lock() {
    let t = Text::md("m14-empty");
    let mut a = answers(1);
    a.extend([nothing(), nothing()]);
    a.extend(answers(1));
    script::arm(&t.ep, a);
    let mut r = Run::new(false);
    let v = t.run(&mut r, opts(&[])).unwrap();
    assert_eq!(n(&v, "windows"), 3);
    assert!(super::unlock::unlock(&t.p.root, false).unwrap().is_empty());
    unit(&t.p.root()).unwrap();
}

/// Every call of the second window `Unreachable`: the report counts them,
/// a Warning gives the count and names `reextract`, and a rerun with
/// `reextract` extracts all three windows again.
#[test]
fn failed_calls_are_counted_and_name_reextract() {
    let t = Text::md("m14-failed");
    let mut a = answers(1);
    a.extend([
        Answer::Fail(ProviderError::Unreachable),
        Answer::Fail(ProviderError::Unreachable),
    ]);
    a.extend(answers(1));
    script::arm(&t.ep, a);
    let mut r = Run::new(false);
    let v = t.run(&mut r, opts(&[])).unwrap();
    assert_eq!(n(&v, "failed"), 2, "{v:?}");
    let said = r.warnings();
    assert!(
        said.iter()
            .any(|w| w.contains("2 model call(s) failed") && w.contains("reextract")),
        "{said:?}"
    );
    assert_eq!(t.journal(), 3, "SMYSL journals the degraded window");

    script::queue(&t.ep, answers(3));
    let re = t
        .run(&mut r, opts(&[("reextract", BundValue::boolean(true))]))
        .unwrap();
    assert_eq!(n(&re, "windows"), 3);
    assert_eq!(n(&re, "skipped"), 0);
    assert_eq!(n(&re, "failed"), 0);
}

/// A commit SMYSL reports as failed, after the first window wrote: the
/// script makes the expression's log unwritable on its third call, the
/// second window's first, so the second window's batch fails and the
/// library has changed. The word is SM02 with the "left locked" suffix, and
/// the next write is SM07 (R25).
#[test]
fn a_failed_commit_inside_the_run_leaves_the_library_locked() {
    let t = Text::md("m14-commit");
    let mut a = vec![nothing(), nothing()];
    a.extend(answers(2));
    script::arm(&t.ep, a);
    let log = t.p.root.join(smysl_text::shards::log_path("incident"));
    let l = log.clone();
    script::on_call(
        &t.ep,
        3,
        Box::new(move || {
            std::fs::create_dir_all(&l).unwrap();
        }),
    );
    let mut r = Run::new(false);
    let e = t.run(&mut r, opts(&[])).unwrap_err();
    assert!(e.starts_with("SMYSL.INGEST.TEXT: SMYSL refused it: the batch could not be committed: "), "{e}");
    assert!(e.ends_with(LEFT_LOCKED), "{e}");
    // The test's own sabotage goes first, or the next write's open would
    // meet it before the locks do.
    std::fs::remove_dir_all(&log).unwrap();
    let next = unit(&t.p.root()).unwrap_err();
    assert!(next.starts_with("SMYSL.UNIT: the library is locked: "), "{next}");
    assert!(next.contains("bund2 smysl.ingest.text"), "{next}");
    super::unlock::unlock(&t.p.root, true).unwrap();
}

// --- criteria 22, 23, 28 ----------------------------------------------------------

/// `opts` and target refusals: any `temperature` and a `level` SMYSL's
/// `Level::new` refuses are SM03; a target the library does not hold, with
/// or without a level, and a granularity SMYSL refuses, are SM13.
#[test]
fn text_opts_and_targets_are_checked() {
    let t = Text::md("m14-opts");
    let mut r = Run::new(false);
    let e = t
        .run(&mut r, opts(&[("temperature", BundValue::float(0.0))]))
        .unwrap_err();
    assert_eq!(e, "SMYSL.INGEST.TEXT: opts has no key temperature");
    let e = t.run(&mut r, opts(&[("level", s("Not A Level"))])).unwrap_err();
    assert!(e.starts_with("SMYSL.INGEST.TEXT: opts level Not A Level is not a level"), "{e}");
    for o in [opts(&[]), opts(&[("level", s("paragraph"))])] {
        let e = r
            .call(vec![t.p.handle(), s("absent"), o], "smysl.ingest.text")
            .unwrap_err();
        assert!(e.starts_with("SMYSL.INGEST.TEXT: "), "{e}");
        assert!(e.contains("absent"), "{e}");
        assert!(!e.contains("SM02"), "{e}");
    }
    let e = t
        .run(&mut r, opts(&[("granularity", s("enormous"))]))
        .unwrap_err();
    assert!(e.contains("the model refused it") || e.contains("enormous"), "{e}");
    let e = r
        .call(
            vec![t.p.handle(), m(&[("manifest", s("not a mid"))]), opts(&[])],
            "smysl.ingest.text",
        )
        .unwrap_err();
    assert_eq!(e, "SMYSL.INGEST.TEXT: manifest not a mid is not a manifest id");
    assert!(script::calls(&t.ep).is_empty());
}

/// A target by manifest id ingests that manifest.
#[test]
fn a_target_may_name_a_manifest() {
    let t = Text::md("m14-mid");
    let mid = t.p.open().head("incident").unwrap();
    script::arm(&t.ep, answers(3));
    let mut r = Run::new(false);
    let v = r
        .call(
            vec![t.p.handle(), m(&[("manifest", s(&mid.canonical()))]), opts(&[])],
            "smysl.ingest.text",
        )
        .unwrap();
    assert_eq!(n(&v, "windows"), 3);
    assert_eq!(text(&get(&v, "manifest")), mid.canonical());
}

/// With `offline`, a provider at a documentation address is SM13 naming
/// `SMY-E301`, before any probe or call.
#[test]
fn offline_refuses_a_remote_provider_for_a_text() {
    let t = Text::md("m14-offline");
    let far = endpoint("192.0.2.1", "m14-offline");
    t.p.sidecar(&config(&far));
    let mut r = Run::new(false);
    let e = t
        .run(&mut r, opts(&[("offline", BundValue::boolean(true))]))
        .unwrap_err();
    assert!(e.starts_with("SMYSL.INGEST.TEXT: the model refused it: SMY-E301"), "{e}");
    assert_eq!(script::probes(&far), 0);
    assert!(script::calls(&far).is_empty());
    assert_eq!(t.journal(), 0);
}

/// A probe that fails, answers unreachable, or does not offer the model is
/// SM13 before any mark or lock, with nothing written.
#[test]
fn a_failed_probe_refuses_a_text_before_it_writes() {
    let t = Text::md("m14-probe");
    let logs = t.p.logs();
    let mut r = Run::new(false);
    for p in [
        Probing::Fail(ProviderError::Unreachable),
        Probing::Unreachable,
        Probing::Models(vec!["other".into()]),
        Probing::Models(vec![]),
    ] {
        script::probing(&t.ep, p);
        let e = t.run(&mut r, opts(&[])).unwrap_err();
        assert!(e.starts_with("SMYSL.INGEST.TEXT: "), "{e}");
        assert!(script::calls(&t.ep).is_empty());
    }
    assert_eq!(t.p.logs(), logs);
    assert_eq!(t.journal(), 0);
}

/// A script that panics makes the word `Error::internal`, and the run keeps
/// SMYSL's locks (R20) but clears its mark: another thread's `smysl.unit`
/// is SM07 with the lock's text, naming the run and this process's pid.
#[test]
fn a_panicking_model_keeps_the_locks_and_clears_the_mark() {
    let t = Text::md("m14-panic");
    script::arm(&t.ep, [Answer::Panic]);
    let mut r = Run::new(false);
    let e = t.run(&mut r, opts(&[])).unwrap_err();
    assert!(
        e.starts_with("internal error: native `smysl.ingest.text` panicked: "),
        "{e}"
    );
    let next = unit(&t.p.root()).unwrap_err();
    assert!(next.starts_with("SMYSL.UNIT: the library is locked: "), "{next}");
    assert!(next.contains("bund2 smysl.ingest.text"), "{next}");
    assert!(next.contains(&format!("pid {}", std::process::id())), "{next}");
    assert!(!next.contains("in this process"), "{next}");
    super::unlock::unlock(&t.p.root, true).unwrap();
}

// --- R42 with Phase 1's probe -------------------------------------------------------

/// A writer that began waiting behind an earlier write, before the run
/// marked the root, is SM07 once the mark is set and does not wait for the
/// run; the run waits behind the same write and then proceeds; a second
/// text ingest meanwhile is SM07.
#[cfg(feature = "smysl-panic-probe")]
#[test]
fn a_waiting_writer_finds_the_mark() {
    use super::probe::{Action, arm};
    let t = Text::md("m14-wait");
    script::arm(&t.ep, answers(3));
    let (_flags, flag) = flag_in("m14-wait-flag");
    arm(&t.p.root, Action::Pause(flag.clone()));
    let root = t.p.root();

    let r1 = root.clone();
    let earlier = std::thread::spawn(move || unit(&r1));
    arrived(&flag);
    let r2 = root.clone();
    let waiter = std::thread::spawn(move || unit(&r2));
    // The waiter has begun waiting on the held root by now.
    std::thread::sleep(Duration::from_millis(300));
    assert!(!waiter.is_finished(), "the waiter waits behind the earlier write");

    let run = ingest_elsewhere(&root);
    let e = waiter.join().unwrap().unwrap_err();
    assert_eq!(e, format!("SMYSL.UNIT: {MARKED}"));
    assert!(!run.is_finished(), "the run waits behind the earlier write");
    let second = ingest_elsewhere(&root).join().unwrap().unwrap_err();
    assert_eq!(second, format!("SMYSL.INGEST.TEXT: {MARKED}"));

    std::fs::remove_file(&flag).unwrap();
    earlier.join().unwrap().unwrap();
    assert_eq!(run.join().unwrap().unwrap(), 3);
}

/// After Phase 1's `panic` probe unwinds a write, the root is not left
/// held: another thread's `smysl.unit` answers SM07 with the lock's text at
/// once, rather than waiting.
#[cfg(feature = "smysl-panic-probe")]
#[test]
fn a_panicking_write_does_not_leave_the_root_held() {
    use super::probe::{Action, arm};
    let t = Text::md("m14-unwind");
    arm(&t.p.root, Action::Panic);
    let e = unit(&t.p.root()).unwrap_err();
    assert!(e.contains("smysl-panic-probe"), "{e}");
    let root = t.p.root();
    let next = std::thread::spawn(move || unit(&root));
    let start = Instant::now();
    while !next.is_finished() {
        assert!(start.elapsed() < Duration::from_secs(10), "the next write waited");
        std::thread::sleep(Duration::from_millis(10));
    }
    let e = next.join().unwrap().unwrap_err();
    assert!(e.starts_with("SMYSL.UNIT: the library is locked: "), "{e}");
    assert!(!e.contains("in this process"), "{e}");
    super::unlock::unlock(&t.p.root, true).unwrap();
}

