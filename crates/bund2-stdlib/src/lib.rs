//! The standard library: native words, effects, and JIT lowerings.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

mod pull;
/// What a `.` suffix means — three shapes, none of them the default.
pub(crate) mod wb;

pub mod stack;

pub mod console;

pub mod logic;

pub mod values;

pub mod math;

pub mod control;

pub mod conditional;

pub mod seq;

pub mod oop;

/// The specialised arms Bund2 publishes — RFC-0005 §S6.
pub mod fragments;
pub mod library;
/// `bund/string`'s library group, and every `.`-suffixed sibling.
pub mod library_string;

pub mod check;

pub mod graph;

pub mod singles;

pub mod sysinfo;

pub mod json;

pub mod sort;

pub mod convert;

pub mod report;

/// Register everything this crate provides.
pub fn register_all(r: &mut bund2_api::Registry) {
    stack::register(r);
    console::register(r);
    logic::register(r);
    values::register_words(r);
    math::register(r);
    control::register(r);
    conditional::register(r);
    seq::register(r);
    oop::register(r);
    convert::register_words(r);
    sort::register(r);
    json::register(r);
    sysinfo::register(r);
    singles::register(r);
    graph::register(r);
    library::register(r);
    library_string::register(r);
}

/// **What RFC-0005 §S5's promotion trusts about natives, checked rather than
/// assumed** — criteria 24 and 25.
///
/// Promotion keeps values in registers across a call to a native whose effect
/// is a fixed pair, so two things about every native must hold: a fixed effect
/// never hides a body the native runs, and no native reports at `Error`
/// severity mid-body, where a snapshot would read a short stack (D45). Both
/// were conventions until the eighth review found the first one broken
/// (`execute.`, F87). These tests turn them into properties.
#[cfg(test)]
mod honesty_tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    /// **Criterion 24.** Run every native with a fixed effect against a stack
    /// *and* a workbench of lambdas, and assert no body starts. `entry_log`
    /// records every frame push (D42), so a native that runs its operand —
    /// through `eval_lambda`, `tail_lambda`, `scoped_call` or `apply` — shows
    /// up there whatever path it took. A word that is honest about running a
    /// body declares `StackEffect::opaque` and is skipped.
    ///
    /// Operands are lambdas only: a string would be a file name to half the
    /// library. So a native that runs a body only when handed a *name* is not
    /// caught here; the reviewer's hand audit of the sixteen re-entering
    /// functions found none that is not also reached by a lambda.
    #[test]
    fn no_fixed_effect_native_runs_a_body() {
        let mut probe = Interp::new();
        crate::register_all(&mut probe.registry);
        let body = bund2_syntax::compile("{ 1 }")
            .expect("compiles")
            .into_iter()
            .next()
            .expect("one value");
        let mut offenders = Vec::new();
        for (name, e) in probe.registry.declared_effects() {
            if e.opaque {
                continue;
            }
            let mut i = Interp::new();
            crate::register_all(&mut i.registry);
            for _ in 0..usize::from(e.consumes) + 2 {
                i.push(body.clone());
                i.push_workbench(body.clone());
            }
            i.entry_log = Some(Vec::new());
            // Failing is fine — most natives refuse a lambda. Running one is not.
            let _ = i.eval(&[BundValue::call(name.as_str())]);
            if i.entry_log.take().is_some_and(|log| !log.is_empty()) {
                offenders.push(name);
            }
        }
        assert!(
            offenders.is_empty(),
            "declared a fixed effect but ran a body: {offenders:?}"
        );
    }

    /// **Criterion 25.** No shipped code in this crate reports at `Error`
    /// severity. Natives return errors; the embedder reports them after
    /// evaluation has returned. A native that reported one mid-body would get
    /// a stack snapshot under the default reporter while values were promoted.
    /// A source scan, so it sees paths no run reaches; each file is cut at its
    /// first `#[cfg(test)]` module, which in this crate always runs to the end.
    #[test]
    fn no_native_reports_at_error_severity() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = Vec::new();
        for entry in std::fs::read_dir(&src).expect("src exists") {
            let path = entry.expect("entry").path();
            if path.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("reads");
            let shipped = text.split("#[cfg(test)]\nmod ").next().unwrap_or_default();
            for (n, line) in shipped.lines().enumerate() {
                if line.contains("Diagnostic::error") || line.contains("Severity::Error") {
                    found.push(format!("{}:{}", path.display(), n + 1));
                }
            }
        }
        assert!(found.is_empty(), "reports at Error severity: {found:?}");
    }
}
