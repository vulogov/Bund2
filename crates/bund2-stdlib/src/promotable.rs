//! The natives promotion may cross, as registration ids — D48, D68.
//!
//! # Why this exists beside [`crate::fragments`]
//!
//! §S6 gives the lowering a table of *fragments* so it knows which calls it may
//! inline. D68 needs the other half: which calls it may **cross**, keeping
//! values in registers rather than syncing them first. Both are questions about
//! a callee that the lowering cannot answer for itself, and both are answered
//! the same way — a table built per registry, at registration time.
//!
//! # The list is not the gate on its own
//!
//! D48: "promotion crosses a native only if it is listed in
//! `tests/golden/PROMOTABLE.txt`". That list holds the fixed-effect natives
//! criterion 28's palette brought to `Ok` with no breach of their declared
//! effect, and that D55's audit did not see reading beyond their operands.
//!
//! It is one of **four** gates, and a lowering that used it alone would be
//! wrong:
//!
//! - **D46** — the name must resolve to a native, not a lambda. A lambda has no
//!   declared effect and its callee can be rebound underneath it.
//! - **D47** — `bund2-stdlib` must have registered it. An embedder's native
//!   with a declared effect gets no such audit.
//! - **D48** — it must be on the list, which is this module.
//! - **D68** — its declared effect must **produce nothing**. A callee that
//!   leaves a result puts it above the promoted values permanently, and the
//!   final sync would then write them in the wrong order, silently.
//!
//! This module answers D47 and D48 together, because membership is keyed on a
//! registration id and only `bund2-stdlib` mints those for these names. The
//! caller still owes D46 and D68.
//!
//! # A fifth gate: a callee that reports mid-body (D71, F137)
//!
//! §S5 states a rule about the *reporter*: "while the reporter wants a snapshot
//! for a severity natives report mid-body, `Warning` or `Notice`, no value
//! stays promoted across a call". The reason is Q34's shape — `Interp::report`
//! takes a snapshot when the reporter wants one, so a native reporting while
//! values below its arity sit in registers would show a **short stack**.
//!
//! D71 implements that rule **statically and more strongly**: a callee that can
//! report mid-body is never crossed, whatever the reporter wants. See
//! [`REPORTS_MID_BODY`] for why that is one name today, and D71 for why the
//! static form was preferred to reading the reporter at each body's entry.
//! The `Vm::wants_stack` gate exists as well, in the tier, and this table is
//! what makes it defence in depth rather than the only thing standing between a
//! warning and a short stack.
//!
//! # Keyed by registration, not by name
//!
//! The same rule §S6 states for fragments, for the same reason: "recognise the
//! registration, not the name, and not the address". A different native
//! registered under one of these names has a different id and is not on this
//! table — which is precisely D47's hazard, since
//! `Registry::register_native` is public.
//!
//! # One source of truth
//!
//! The list is `include_str!`'d from the file the audit writes, rather than
//! copied into a const array here. `BUND2_UPDATE_PROMOTABLE=1 cargo test -p
//! bund2-stdlib promotable` rewrites it, and that test asserts the file matches
//! what the palette found — so the honesty mechanism stays where it already is
//! instead of being duplicated into a second place that could drift.

use std::collections::BTreeSet;

/// The audit's own output, compiled in.
///
/// **Every `#` line is a comment, not only the header.** The file ends with
/// `# not reached: …` lines naming the natives no palette run brought to `Ok`;
/// a parser that skipped a leading block alone would read those as entries and
/// cross exactly the words D48 excludes.
const LIST: &str = include_str!("../../../tests/golden/PROMOTABLE.txt");

/// **The natives that report a `Warning` or `Notice` mid-body** — D71's gate.
///
/// Promotion never crosses one of these, so no crossed call can reach a
/// `Vm::report` while values it cannot see are held in registers.
///
/// # Why this is one name
///
/// Shipped `bund2-stdlib` code reports mid-body in five places, and four of
/// them are unreachable as a crossed call *already*:
///
/// | reports | reached through | why it is not crossed |
/// |---|---|---|
/// | `alias` (`singles.rs`) | `alias`, `eff(2, 0)` | **nothing — this table is why** |
/// | `while_base` (`control.rs`) | `while`, `while.` | `StackEffect::opaque` |
/// | `for_base` (`control.rs`) | `for`, `for.` | `StackEffect::opaque` |
/// | `loop_over_base` (`seq.rs`) | `*loop`, `*loop.` | `StackEffect::opaque` |
/// | `run_error` (`conditional.rs`) | `register_conditional`, run by `!` | not a native; `!` is opaque |
///
/// D68 already refuses an opaque callee, so only `alias` needed excluding. The
/// set is pinned by `every_native_reporting_mid_body_is_named`
/// (`crates/bund2-stdlib/src/lib.rs`), which fails if shipped code gains a
/// sixth site — so this cannot go stale silently, which is the hazard a list
/// kept in prose carries.
///
/// # What it rests on
///
/// That a **non-opaque** native never re-enters evaluation, and so cannot reach
/// a reporting native indirectly. `StackEffect::opaque` is exactly the marker
/// for a word that runs a body, and it is the same assumption `PROMOTABLE.txt`
/// already rests on (D55). Stated here because it is load-bearing rather than
/// obvious.
const REPORTS_MID_BODY: [&str; 1] = ["alias"];

/// **The natives that change which stack is current** — D73's gate, F140.
///
/// Promotion holds values in registers and syncs them by *pushing*, and a push
/// goes to whatever stack is current at that moment. So a callee that changes
/// the current stack while values are held moves them: the body's final sync
/// lands them on the stack in force **after** the call, where Tier 0 pushed
/// them before it.
///
/// **F140 is that, measured.** `:w { 1 2 + stacks_left }` at the shipped
/// threshold leaves one value on the wrong stack — `main` holds 33 without the
/// tier and 32 with it. `stacks_left` is `eff(0, 0)`, so a crossing syncs
/// *nothing* before it and every promoted value rides across the rotation.
///
/// **Why the operand-free ones are the exposure, and why this gate names the
/// others anyway.** `to_stack` and `to_current` are `eff(1, 0)`: their name
/// operand is pushed *after* the promoted values, and a symbol is not a
/// promotable literal, so pushing it is a generic apply that syncs everything
/// first. Nothing is ever held across them today. That protection is a
/// consequence of what happens to be promotable, not a rule, and it would go
/// quietly if that ever changed — so the gate does not lean on it.
///
/// Some of these restore the stack before returning (`swap_in`,
/// `rotate_stack_left`). The gate does not try to tell restoring from not: a
/// source scan cannot, and refusing to cross them costs nothing measurable.
///
/// Pinned by `every_native_that_changes_the_current_stack_is_named`
/// (`crates/bund2-stdlib/src/lib.rs`), so a new one fails the build until it is
/// named here.
const SWITCHES_STACK: [&str; 8] = [
    "endcontext",
    "rotate_stack_left",
    "rotate_stack_right",
    "stacks_left",
    "stacks_right",
    "swap_in",
    "to_current",
    "to_stack",
];

/// The names the audit certified, in file order.
fn names() -> impl Iterator<Item = &'static str> {
    LIST.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
}

/// **The registrations promotion may cross** — D47 and D48, together.
///
/// Built per registry, at registration time, exactly as
/// [`crate::fragments::published`] is: the ids are whatever the registry's
/// slots hold *now*, so this must be called once the vocabulary is registered
/// and the result kept.
///
/// A name the registry does not hold is **skipped rather than refused**. One
/// file serves two builds — `string.grok` is feature-gated, which is F123 — and
/// a missing name means the native was not registered, which is the same answer
/// as "not crossable" for every purpose here.
///
/// A name that resolves to something with no `native` is skipped too: an alias
/// holds no `Native` and therefore no id, so keying on it would put an entry in
/// the table that can never match a callee. That is the `dup`/`dup_one`
/// distinction §S6 records, and it applies unchanged.
pub fn crossable(r: &bund2_api::Registry) -> BTreeSet<bund2_api::RegistrationId> {
    let mut out = BTreeSet::new();
    for name in names() {
        // **D71's gate, applied by name before the id is taken.** A native that
        // reports a `Warning` or `Notice` mid-body would snapshot a stack
        // missing every value a crossing holds in a register (F137, §S5's
        // reporter rule). Excluded here rather than in the lowering, because
        // this is the crate that knows which of its natives report — the
        // lowering sees only a registration id.
        if REPORTS_MID_BODY.contains(&name) {
            continue;
        }
        // **D73's gate, F140.** A callee that changes the current stack moves
        // every value promotion is holding: the sync that follows pushes them
        // onto the stack in force afterwards, not the one they were computed
        // on. Excluded by name here for the same reason the reporters are —
        // this crate knows which of its natives switch stacks, and the lowering
        // sees only a registration id.
        if SWITCHES_STACK.contains(&name) {
            continue;
        }
        // `Interner::lookup_call` rather than `intern`, for the reason
        // `fragments::published` gives: this takes `&Registry`, and looking up
        // a miss must not retain a slot.
        let Some((s, _)) = r.interner.lookup_call(name) else {
            continue;
        };
        let Some(id) = r.slot(s).and_then(|sl| sl.native.as_ref()).and_then(|n| n.id) else {
            continue;
        };
        out.insert(id);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parse must see exactly the entries the file holds — not the
    /// comments, and not the `# not reached:` tail.
    #[test]
    fn the_parse_reads_entries_and_no_comments() {
        let parsed: Vec<&str> = names().collect();
        let expected = LIST
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .count();
        assert_eq!(parsed.len(), expected, "every non-comment line is an entry");
        assert!(
            parsed.iter().all(|n| !n.starts_with('#')),
            "a comment reached the entry list"
        );
        assert!(
            parsed.iter().any(|n| *n == "+"),
            "`+` is the arm §S6 publishes and the audit certifies; its absence \
             means the file or the parse is wrong"
        );
        assert!(
            !parsed.iter().any(|n| n.starts_with("not reached")),
            "the `# not reached:` tail was read as entries, so D48's exclusions \
             would be crossed"
        );
    }

    /// **The table keys on registrations a real registry minted**, and answers
    /// for the natives the audit certified.
    #[test]
    fn crossable_answers_for_registered_natives() {
        let mut r = bund2_api::Registry::new();
        crate::register_all(&mut r);
        let ids = crossable(&r);

        assert!(
            !ids.is_empty(),
            "an empty table would silently disable promotion across every call"
        );

        // `+` is on the list and is registered under its own name, so its id
        // must be present — the same name §S6's fragment table keys on.
        let (s, _) = r.interner.lookup_call("+").expect("`+` is registered");
        let id = r
            .slot(s)
            .and_then(|sl| sl.native.as_ref())
            .and_then(|n| n.id)
            .expect("`+` carries a registration id");
        assert!(ids.contains(&id), "`+` is certified and must be crossable");
    }

    /// **D71: a native that reports mid-body is certified and still not
    /// crossable.**
    ///
    /// `alias` is on `PROMOTABLE.txt` — criterion 28's palette brought it to
    /// `Ok` and D55's audit saw it read no further than its operands — and it
    /// declares `eff(2, 0)`, so D46, D47, D48 and D68 all admit it. The fifth
    /// gate is the only thing that refuses it, which is what makes this test
    /// worth having: remove [`REPORTS_MID_BODY`] and every other gate still
    /// says yes.
    #[test]
    fn a_native_that_reports_mid_body_is_not_crossable() {
        let mut r = bund2_api::Registry::new();
        crate::register_all(&mut r);
        let ids = crossable(&r);

        let (s, _) = r.interner.lookup_call("alias").expect("`alias` is registered");
        let id = r
            .slot(s)
            .and_then(|sl| sl.native.as_ref())
            .and_then(|n| n.id)
            .expect("`alias` carries a registration id");

        assert!(
            names().any(|n| n == "alias"),
            "the premise of this test is that the audit certified `alias`; if it \
             no longer does, the fifth gate is not what excludes it"
        );
        assert!(
            !ids.contains(&id),
            "`alias` reports a Warning mid-body (F137): crossing it would let a \
             native snapshot a stack missing every promoted value"
        );
    }

    /// **D73: a native that changes the current stack is certified and still
    /// not crossable** — F140.
    ///
    /// `stacks_left` is on `PROMOTABLE.txt` — the palette brought it to `Ok`
    /// and D55's audit saw it read no further than its operands, of which it
    /// has none — and it declares `eff(0, 0)`, so D46, D47, D48 and D68 all
    /// admit it and D71 does not exclude it. This gate is the only thing that
    /// refuses it, which is what makes the test worth having: remove
    /// [`SWITCHES_STACK`] and every other gate still says yes, and
    /// `:w { 1 2 + stacks_left }` starts leaving values on the wrong stack
    /// again.
    #[test]
    fn a_native_that_changes_the_current_stack_is_not_crossable() {
        let mut r = bund2_api::Registry::new();
        crate::register_all(&mut r);
        let ids = crossable(&r);

        for word in ["stacks_left", "stacks_right", "to_stack", "to_current"] {
            let Some((s, _)) = r.interner.lookup_call(word) else {
                panic!("`{word}` is registered");
            };
            let id = r
                .slot(s)
                .and_then(|sl| sl.native.as_ref())
                .and_then(|n| n.id)
                .unwrap_or_else(|| panic!("`{word}` carries a registration id"));
            assert!(
                !ids.contains(&id),
                "`{word}` changes which stack is current (F140): crossing it \
                 would sync promoted values onto the stack in force after the \
                 call, not the one they were computed on"
            );
        }

        assert!(
            names().any(|n| n == "stacks_left"),
            "the premise is that the audit certified `stacks_left`; if it no \
             longer does, this gate is not what excludes it"
        );
    }

    /// **A native the audit did not certify is absent**, which is the half that
    /// makes this a gate rather than a list of everything.
    #[test]
    fn a_native_the_audit_excluded_is_not_crossable() {
        let mut r = bund2_api::Registry::new();
        crate::register_all(&mut r);
        let ids = crossable(&r);

        // `debug.display_stack` reads the whole stack, so D55's audit flags it
        // and D48 keeps it off the list. Criterion 14 asserts the barrier it
        // needs; this asserts the table that places it.
        let Some((s, _)) = r.interner.lookup_call("debug.display_stack") else {
            panic!("`debug.display_stack` is registered");
        };
        if let Some(id) = r
            .slot(s)
            .and_then(|sl| sl.native.as_ref())
            .and_then(|n| n.id)
        {
            assert!(
                !ids.contains(&id),
                "`debug.display_stack` observes beyond its operands (D55) and \
                 must never be crossed"
            );
        }
    }
}
