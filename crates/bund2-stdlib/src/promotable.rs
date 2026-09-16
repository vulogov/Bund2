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
