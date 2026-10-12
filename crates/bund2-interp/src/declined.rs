//! **The bodies the tier has declined for good** — D146, F201.
//!
//! A body with nothing to inline is planned once, refused and demoted (F136),
//! and until D146 it was offered again on every entry: about 120 instructions
//! to clone the tier, call it through its trait object and have it look the
//! body up, for an answer that could not change. The tier now says so once,
//! through `Vm::tier_never`, and `Interp::push_frame` asks here first.
//!
//! **Keyed and kept as the tier's own demoted set is** (D35 as amended by
//! Q32): on `BundValue::payload_key`, holding the `Weak` that keeps that
//! address out of reuse, so an entry can never answer for a different body.
//!
//! **A table of fixed size, one body to a slot, and a second body that lands
//! on a slot takes it.** A slot is chosen by the address, so asking is a
//! shift, a mask, a load and a compare. That is D39's bound without a sweep: nothing here
//! grows or loops. The body that lost its slot is offered once more, declined
//! once more, and takes the slot back, so two hot refused bodies sharing a
//! slot cost what they cost before D146 and never more.
//!
//! **Forgetting is always safe.** A body not found here is offered, which is
//! what happened to every body before. Remembering wrongly is what the `Weak`
//! rules out.

use std::rc::Weak;

use bund2_value::{BundValue, Payload};

/// See [`slot`].
const SPACING: u32 = 7;

/// How many slots. A power of two, so that [`slot`] is a shift and a mask.
const SLOTS: usize = 1 << 10;

/// Which slot a key belongs in.
///
/// **One field of the address**, above the bits two bodies made one after
/// another share: a body is a payload behind several allocations, so its
/// neighbours are further off than `1 << SPACING` bytes and take the slots
/// either side. Ten thousand bodies made in a row fill 419 slots of 1,024
/// at a spacing of 32 bytes, 821 at 64 and all of them at 128, measured on
/// macOS arm64; another allocator may spread them worse, which costs speed
/// and nothing else. A multiplicative hash would not depend on the allocator
/// and takes five instructions more, on a path where D146's condition left
/// about twelve to spend.
fn slot(key: usize) -> usize {
    (key >> SPACING) & (SLOTS - 1)
}

/// The table, once there is something to put in it.
struct Table {
    /// The key each slot answers for, and 0 for an empty slot: no payload is
    /// at address 0. Apart from `kept` so that asking reads one word.
    keys: [usize; SLOTS],
    /// What keeps each key's address out of reuse. Never read.
    kept: [Option<Weak<Payload>>; SLOTS],
}

/// See the module docs.
#[derive(Default)]
pub(crate) struct Declined {
    /// `None` until the first body is declined, so an `Interp` whose tier
    /// never declines one — or that has no tier — holds no table and
    /// [`Declined::holds`] is one test.
    table: Option<Box<Table>>,
}

impl Declined {
    /// Whether the body at `key` was declined for good.
    #[inline]
    pub(crate) fn holds(&self, key: usize) -> bool {
        // `get` and not an index, though the mask in `slot` keeps it inside:
        // D37 would rather that be checked than argued.
        self.table
            .as_ref()
            .and_then(|t| t.keys.get(slot(key)))
            .is_some_and(|held| *held == key)
    }

    /// Remember that `body` is not to be offered again.
    ///
    /// Cold and out of line: it runs once a body, from `Interp::push_frame`,
    /// which runs for every body entered.
    #[cold]
    #[inline(never)]
    pub(crate) fn remember(&mut self, body: &BundValue) {
        let (Some(key), Some(weak)) = (body.payload_key(), body.payload_weak()) else {
            return;
        };
        let table = self.table.get_or_insert_with(|| {
            Box::new(Table {
                keys: [0; SLOTS],
                kept: [const { None }; SLOTS],
            })
        });
        let at = slot(key);
        if let (Some(k), Some(w)) = (table.keys.get_mut(at), table.kept.get_mut(at)) {
            *k = key;
            *w = Some(weak);
        }
    }

    /// Forget everything: the answers were one tier's, and it has gone.
    pub(crate) fn clear(&mut self) {
        self.table = None;
    }

    /// How many bodies are remembered.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.table
            .as_ref()
            .map_or(0, |t| t.keys.iter().filter(|k| **k != 0).count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(n: i64) -> BundValue {
        BundValue::lambda(vec![BundValue::int(n)])
    }

    /// A remembered body is held, another is not, and no table exists until
    /// one is remembered.
    #[test]
    fn a_remembered_body_is_held_and_no_other() {
        let mut d = Declined::default();
        let (a, b) = (body(1), body(2));
        let (ka, kb) = (
            a.payload_key().expect("a key"),
            b.payload_key().expect("a key"),
        );
        assert!(!d.holds(ka), "nothing is held by an empty table");
        assert!(d.table.is_none(), "and there is no table yet");
        d.remember(&a);
        assert!(d.holds(ka));
        assert!(!d.holds(kb), "a body that was not declined is not held");
        d.clear();
        assert!(!d.holds(ka), "cleared");
    }

    /// **The address cannot be reused while it is remembered.** The body is
    /// dropped and two thousand more are made; none of them is at its address,
    /// so none inherits its entry.
    #[test]
    fn a_dropped_bodys_address_is_not_handed_to_another() {
        let mut d = Declined::default();
        let gone = body(0);
        let key = gone.payload_key().expect("a key");
        d.remember(&gone);
        drop(gone);
        let later: Vec<BundValue> = (1..2000).map(body).collect();
        assert!(
            later.iter().all(|b| b.payload_key() != Some(key)),
            "the allocation was reused under a remembered key"
        );
        assert!(
            later
                .iter()
                .all(|b| !d.holds(b.payload_key().expect("a key")))
        );
    }

    /// **Bounded by construction.** Ten thousand bodies are remembered and
    /// the table holds at most its size; the last one remembered is held.
    #[test]
    fn the_table_never_holds_more_than_its_size() {
        let mut d = Declined::default();
        let bodies: Vec<BundValue> = (0..10_000).map(body).collect();
        for b in &bodies {
            d.remember(b);
        }
        assert!(d.len() <= SLOTS, "{} entries", d.len());
        assert!(
            d.len() > SLOTS / 2,
            "{} entries: the slots are not being used",
            d.len()
        );
        let last = bodies.last().expect("ten thousand");
        assert!(d.holds(last.payload_key().expect("a key")));
        let held = bodies
            .iter()
            .filter(|b| d.holds(b.payload_key().expect("a key")))
            .count();
        assert_eq!(
            held,
            d.len(),
            "every entry answers for exactly its own body"
        );
    }

    /// A scalar has no payload, so there is nothing to remember.
    #[test]
    fn a_scalar_is_not_remembered() {
        let mut d = Declined::default();
        d.remember(&BundValue::int(7));
        assert!(d.table.is_none());
    }
}
