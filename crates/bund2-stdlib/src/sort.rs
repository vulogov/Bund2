//! `sort` — and the quicksort it has to reproduce exactly.
//!
//! The reference is one line: `algos::sort::quicksort::sort::<Value>(&mut
//! raw_list)` (`reference/Bund/src/stdlib/functions/values/sort_lists.rs:36`).
//! Reproducing it means reproducing **which** sort, because that sort is
//! **unstable** and the goldens capture element order — its own doc says "Not
//! stable: equal elements may be reordered".
//!
//! So this is `algos` v0.6.8's `cs::sort::quicksort` transcribed: median-of-three
//! pivot, insertion sort below a threshold of 10, and an explicit stack rather
//! than recursion. Transcribed rather than depended on because `algos` is a
//! **git** dependency declared as `version="0.6.*"` against a bare URL with no
//! rev or tag (`reference/Bund/Cargo.toml:78`) — the oracle builds against
//! whatever the branch head is, so depending on it the same way would make
//! Bund2's sort order a function of when it was built.
//!
//! # The comparator is the subtle part
//!
//! `algos`' sort is `<T: Ord>` and uses the operators `>` and `<=`. In Rust
//! those call `PartialOrd::gt` and `PartialOrd::le`, **not** `Ord::cmp` — and
//! `Value` overrides all four (`reference/rust_dynamic/src/ord.rs:9,48,87,126`)
//! while its `cmp` is a different function entirely (`:168`).
//!
//! That distinction decides real behaviour, and it is **F12**, already in the
//! register: `cmp` has no `F64` arm, so two floats fall through to
//! `self.id.cmp(&other.id)` (`:199`) and order by random nanoid, while `lt`,
//! `le`, `gt` and `ge` each have one. So sorting floats works *only* because
//! the sort reaches for the operators; had it used `cmp`,
//! `[ 3.5 1.5 2.5 ] sort` would answer differently on every run. Confirmed
//! against the oracle: it is stable across runs and correctly ordered.
//!
//! F12 calls those four "the reachable path". This word is what makes them
//! reachable.
//!
//! **Cross-type comparison is inconsistent, by construction.** Every arm ends
//! `_ => return true`, so for an integer against a string both `a > b` and
//! `a <= b` are true. A list of mixed types therefore has no well-defined
//! sorted order — not an approximation here, the reference's own property.
//! It is reproduced rather than repaired, and `std`'s `sort_by` is avoided for
//! exactly this reason: it detects an inconsistent comparator and can panic,
//! which D37 forbids.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST, PAIR};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `PartialOrd::gt` for `Value` (`reference/rust_dynamic/src/ord.rs:87-124`).
///
/// **Three payload arms, and that is the whole comparator: `I64`, `F64` and
/// `Time`** (`:89,97,105`). Everything else — including **strings** — falls to
/// `_ => return true` (`:121`), as does every cross-arm pair (`:94,102,110`).
///
/// A first version of this function added a string comparison, on the
/// reasonable-looking assumption that `sort` orders strings. It does not:
/// sorting eleven fruit names against the oracle returns
/// `lemon kiwi grape mango cherry …`, not alphabetical order, because both
/// `a > b` and `a <= b` are true for every pair and the result is whatever the
/// partition's swaps leave behind. Nine other cases — integers, floats, heavy
/// ties — matched with the string arm present; only strings exposed it.
///
/// `Ord::cmp` *does* compare strings (`:186-191`), which is why the mistake is
/// easy: the type has two orderings and only one of them is the sort's.
///
/// The `Time` arm is omitted because Bund2 has the tag but no constructor for
/// it, so no `Time` value can reach here; a `TIME` value would take the `_`
/// arm, which is where every unconstructible tag already goes.
fn gt(a: &BundValue, b: &BundValue) -> bool {
    use BundValue::{Float, Int};
    match (a.unboxed(), b.unboxed()) {
        (Int(x, _), Int(y, _)) => x > y,
        (Float(x, _), Float(y, _)) => x > y,
        _ => true,
    }
}

/// `PartialOrd::le` (`ord.rs:48-85`), same three arms and the same fallback.
fn le(a: &BundValue, b: &BundValue) -> bool {
    use BundValue::{Float, Int};
    match (a.unboxed(), b.unboxed()) {
        (Int(x, _), Int(y, _)) => x <= y,
        (Float(x, _), Float(y, _)) => x <= y,
        _ => true,
    }
}

/// Below this length the sort switches to insertion sort. `algos`' constant.
const INSERTION_SORT_THRESHOLD: usize = 10;

fn insertion_sort(arr: &mut [BundValue]) {
    for i in 1..arr.len() {
        let mut j = i;
        // `j - 1` and `j` are both < len because `j <= i < len`, so neither
        // index can be out of bounds and neither subtraction can underflow.
        while j > 0 && gt(&arr[j - 1], &arr[j]) {
            arr.swap(j - 1, j);
            j -= 1;
        }
    }
}

/// Median-of-three partition, transcribed from `algos`' `partition`.
///
/// Every index below is in range for `len >= 2`, which the early return
/// establishes: `mid = len/2 <= last`, and `last - 1` needs `last >= 1`.
fn partition(arr: &mut [BundValue]) -> usize {
    let len = arr.len();
    if len <= 1 {
        return 0;
    }
    let mid = len / 2;
    let last = len - 1;

    if gt(&arr[0], &arr[mid]) {
        arr.swap(0, mid);
    }
    if gt(&arr[mid], &arr[last]) {
        arr.swap(mid, last);
    }
    if gt(&arr[0], &arr[mid]) {
        arr.swap(0, mid);
    }

    arr.swap(mid, last - 1);
    let pivot_idx = last - 1;

    let mut i = 0;
    let mut j = pivot_idx;
    while i < j {
        while i < j && le(&arr[i], &arr[pivot_idx]) {
            i += 1;
        }
        while i < j && gt(&arr[j - 1], &arr[pivot_idx]) {
            j -= 1;
        }
        if i < j {
            arr.swap(i, j - 1);
        }
    }

    if gt(&arr[i], &arr[pivot_idx]) {
        arr.swap(i, pivot_idx);
        i
    } else {
        pivot_idx
    }
}

/// The sort itself: an explicit stack, larger partition pushed first.
pub(crate) fn quicksort(arr: &mut [BundValue]) {
    let mut stack: Vec<(usize, usize)> = Vec::with_capacity(32);
    stack.push((0, arr.len()));
    while let Some((start, end)) = stack.pop() {
        // `pop` only yields ranges this loop pushed, all within `0..=len`, so
        // the slice below is always in range and `end - start` cannot underflow.
        if end <= start {
            continue;
        }
        let len = end - start;
        if len <= 1 {
            continue;
        }
        if len < INSERTION_SORT_THRESHOLD {
            insertion_sort(&mut arr[start..end]);
            continue;
        }
        let pivot_idx = partition(&mut arr[start..end]) + start;
        if pivot_idx - start > end - (pivot_idx + 1) {
            stack.push((start, pivot_idx));
            stack.push((pivot_idx + 1, end));
        } else {
            stack.push((pivot_idx + 1, end));
            stack.push((start, pivot_idx));
        }
    }
}

/// `sort` — sort a list in place and push it back
/// (`reference/Bund/src/stdlib/functions/values/sort_lists.rs:11-41`).
///
/// The operand goes through `cast_list`, which admits `LIST` **and** `PAIR`
/// (`reference/rust_dynamic/src/cast.rs:58`), and the result is pushed with
/// `from_list` (`:38`) — so sorting a PAIR answers a LIST.
fn sort(vm: &mut dyn Vm) -> Result<(), Error> {
    sort_base(vm, crate::wb::Side::Stack)
}

/// `sort.` — the workbench mirror
/// (`reference/Bund/src/stdlib/functions/values/sort_lists.rs:19-20,26,39`).
///
/// Unlike the print family (F77), this one guards the side it reads, and says
/// "Workbench is too shallow".
fn sort_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    sort_base(vm, crate::wb::Side::Bench)
}

fn sort_base(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    let prefix = &format!("SORT{}", side.dot());
    if side.depth(vm) < 1 {
        return Err(Error(format!(
            "{} is too shallow for inline {prefix}",
            if side == crate::wb::Side::Stack { "Stack" } else { "Workbench" }
        )));
    }
    let v = crate::wb::operand(vm, side, prefix)?;
    let dt = v.dt();
    if dt != LIST && dt != PAIR {
        return Err(Error(format!(
            "{prefix} casting of list returned: This is not a LIST/PAIR value but {dt}"
        )));
    }
    let mut items = v
        .as_list()
        .ok_or_else(|| Error(format!("{prefix} casting of list returned: not a list")))?
        .to_vec();
    quicksort(&mut items);
    side.push(vm, BundValue::list(items));
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.register_native("sort", sort, eff(1, 1), WordKind::Sync);
    // Workbench in, workbench out — the plain mirror
    // (`reference/Bund/src/stdlib/functions/values/sort_lists.rs:19-20,26,39`).
    r.register_native("sort.", sort_wb, eff(0, 0), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::Interp;

    fn run_src(src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    fn sorted_ints(src: &str) -> Vec<i64> {
        let i = run_src(src).expect("runs");
        i.peek()
            .and_then(|v| v.as_list().map(|l| l.to_vec()))
            .expect("a list")
            .iter()
            .filter_map(|v| v.as_int())
            .collect()
    }

    /// Past the insertion-sort threshold, so the quicksort path runs.
    #[test]
    fn sorts_more_than_the_threshold() {
        let src = "[ 9 3 7 1 8 2 6 0 5 4 11 10 ] sort";
        assert_eq!(sorted_ints(src), (0..=11).collect::<Vec<i64>>());
    }

    /// Under the threshold, so insertion sort runs instead.
    #[test]
    fn sorts_under_the_threshold() {
        assert_eq!(sorted_ints("[ 3 1 2 ] sort"), vec![1, 2, 3]);
        assert_eq!(sorted_ints("[ 1 ] sort"), vec![1]);
    }

    /// Floats sort by value. This is the case that would break if the
    /// comparator went through `Ord::cmp`, which has no `F64` arm and falls
    /// back to comparing random identities.
    #[test]
    fn floats_sort_by_value_not_identity() {
        let i = run_src("[ 3.5 1.5 2.5 ] sort").expect("runs");
        let got: Vec<f64> = i
            .peek()
            .and_then(|v| v.as_list().map(|l| l.to_vec()))
            .expect("a list")
            .iter()
            .filter_map(|v| match v.unboxed() {
                BundValue::Float(f, _) => Some(*f),
                _ => None,
            })
            .collect();
        assert_eq!(got, vec![1.5, 2.5, 3.5]);
    }

    #[test]
    fn a_non_list_is_reported() {
        match run_src("42 sort") {
            Ok(_) => panic!("expected a failure"),
            Err(e) => assert!(e.contains("This is not a LIST/PAIR value"), "{e}"),
        }
    }
}
