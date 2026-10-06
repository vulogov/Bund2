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
use bund2_value::{BundValue, CALL, LIST, PAIR, STRING};

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
        .ok_or_else(|| Error(format!("{prefix} casting of list returned: This Dynamic type is not list")))?
        .to_vec();
    quicksort(&mut items);
    side.push(vm, BundValue::list(items));
    Ok(())
}

/// What the reference's `Val` holds, as far as `unique` can tell values
/// apart. Its ordering and its equality each have arms for integers, floats
/// and strings and fall through to the value's **id** for everything else
/// (`reference/rust_dynamic/src/ord.rs`, `eq.rs`), so three kinds and a
/// remainder is the whole of the distinction.
///
/// `Text` covers a STRING and a CALL alike: both carry `Val::String` in the
/// reference, which is why `[ true true ] unique` deduplicates — those are
/// two CALLs named `true`, not two booleans.
enum Key {
    Int(i64),
    Float(f64),
    Text(String),
    Other,
}

fn key_of(v: &BundValue) -> Key {
    match v.unboxed() {
        BundValue::Int(i, _) => Key::Int(*i),
        BundValue::Float(f, _) => Key::Float(*f),
        _ if v.dt() == STRING || v.dt() == CALL => {
            v.as_str().map_or(Key::Other, Key::Text)
        }
        _ => Key::Other,
    }
}

/// `Value`'s `<=` — the reference's `le` override, which reads no id.
///
/// **True for anything that is not two integers or two floats.** So a list of
/// strings is always "sorted", and so is any mixed list; only same-kind
/// numbers can fail the test `unique` makes before each search.
fn key_le(a: &Key, b: &Key) -> bool {
    match (a, b) {
        (Key::Int(x), Key::Int(y)) => x <= y,
        (Key::Float(x), Key::Float(y)) => x <= y,
        _ => true,
    }
}

/// `Value`'s `Ord::cmp`, where the reference compares **by value**.
///
/// Integers and strings are the reference's own arms. Floats are F12's
/// disposition — FIX — since the reference has no float arm and orders two
/// floats by random id. `None` is every pair the reference orders by id.
fn key_cmp(a: &Key, b: &Key) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Key::Int(x), Key::Int(y)) => Some(x.cmp(y)),
        (Key::Text(x), Key::Text(y)) => Some(x.cmp(y)),
        (Key::Float(x), Key::Float(y)) => x.partial_cmp(y),
        _ => None,
    }
}

/// `Value`'s `==`. An integer against a float **truncates the float**, as the
/// reference's `*i == *f as i64` does; the other direction widens the
/// integer. Anything else is equal only by id, which two members of one list
/// never share.
fn key_eq(a: &Key, b: &Key) -> bool {
    match (a, b) {
        (Key::Int(x), Key::Int(y)) => x == y,
        #[expect(clippy::cast_possible_truncation, reason = "the reference's own cast")]
        (Key::Int(x), Key::Float(y)) => *x == *y as i64,
        (Key::Float(x), Key::Float(y)) => x == y,
        #[expect(clippy::cast_precision_loss, reason = "the reference's own cast")]
        (Key::Float(x), Key::Int(y)) => *x == *y as f64,
        (Key::Text(x), Key::Text(y)) => x == y,
        _ => false,
    }
}

/// Is `target` already in `data`? — `algos::cs::search::fibonacci::search`,
/// ported step for step, because its *quirks* are the behaviour.
///
/// `Err` is the reference's "requires sorted input". The check is on the
/// accumulator as it stands, with [`key_le`].
///
/// **The walk is kept exactly, including where it is wrong.** `unique` feeds
/// it an accumulator in *input* order, which for strings is never rejected as
/// unsorted, so the search runs over unsorted data and skips positions:
/// `[ "c" "b" "a" "c" "a" ] unique` drops the second `c` and keeps the second
/// `a`. A `contains` here would be tidier and would answer differently.
///
/// **D100: where the reference walks by random id, this answers by its
/// equality.** [`key_cmp`] giving `None` is a pair the reference orders by
/// id, so its walk takes an arbitrary branch and the answer changes between
/// runs — measured, six runs of five floats gave two answers. There the
/// question is put directly: found if and only if some element is equal by
/// the reference's own `==`.
fn fib_contains(data: &[Key], target: &Key) -> Result<bool, Error> {
    if data.is_empty() {
        return Ok(false);
    }
    if !data.windows(2).all(|w| key_le(&w[0], &w[1])) {
        return Err(Error(
            "Invalid input: Fibonacci search requires sorted input".into(),
        ));
    }
    let len = i64::try_from(data.len()).unwrap_or(i64::MAX);
    let (mut fib2, mut fib1, mut fib) = (0i64, 1i64, 1i64);
    while fib < len {
        fib2 = fib1;
        fib1 = fib;
        fib = fib1.saturating_add(fib2);
    }
    let mut offset: i64 = -1;
    // The triple stays a run of consecutive Fibonacci numbers, so `fib` falls
    // at every step and the loop ends in at most ninety-odd; the count is
    // D39's belt, for a loop whose termination is otherwise an argument.
    let mut steps = 0u32;
    while fib > 1 {
        steps += 1;
        if steps > 128 {
            return Err(Error::internal(
                "the Fibonacci walk in `unique` did not shrink its interval",
            ));
        }
        // The reference computes `(offset + fib2) as usize` and clamps: a
        // negative sum wraps to the largest `usize`, so it too lands on the
        // last index.
        let raw = offset + fib2;
        let i = if raw < 0 { len - 1 } else { raw.min(len - 1) };
        let at = usize::try_from(i).unwrap_or(0);
        match key_cmp(target, &data[at]) {
            Some(std::cmp::Ordering::Less) => {
                fib = fib2;
                fib1 -= fib2;
                fib2 = fib - fib1;
            }
            Some(std::cmp::Ordering::Greater) => {
                fib = fib1;
                fib1 = fib2;
                fib2 = fib - fib1;
                offset = i;
            }
            Some(std::cmp::Ordering::Equal) => return Ok(true),
            None => return Ok(data.iter().any(|d| key_eq(target, d))),
        }
    }
    if fib1 == 1 && offset + 1 < len {
        let at = usize::try_from(offset + 1).unwrap_or(0);
        if key_eq(target, &data[at]) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `unique` and `unique.` — drop the members already seen
/// (`reference/Bund/src/stdlib/functions/values/listop.rs`,
/// `unique_list_base`).
///
/// **It is not a set operation.** The accumulator is built in input order and
/// searched with a Fibonacci search, which wants sorted data and is told so
/// only for same-kind numbers. Measured against the oracle:
///
/// | input | answer |
/// |---|---|
/// | `[ 1 1 2 2 3 ]` | `[ 1 2 3 ]` |
/// | `[ 2 1 ]` | `[ 2 1 ]` — the pair is never searched again |
/// | `[ 2 1 1 ]` | **error**, at the third member |
/// | `[ "b" "a" "b" ]` | `[ b a ]` — strings are never "unsorted" |
/// | `[ "c" "b" "a" "c" "a" ]` | `[ c b a a ]` — the search misses one |
/// | `[ [ 1 ] [ 1 ] ]` | both kept — lists are equal only by id |
///
/// So F149's summary, "refuses any list that is not already ascending", is
/// true of numbers and not of strings.
///
/// Both forms answer on their own side.
fn unique_base(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    let prefix = &format!("UNIQUE{}", side.dot());
    crate::host::guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let dt = v.dt();
    if dt != LIST && dt != PAIR {
        return Err(Error(format!(
            "{prefix} casting of list returned: This is not a LIST/PAIR value but {dt}"
        )));
    }
    let items = v
        .as_list()
        .ok_or_else(|| Error(format!("{prefix} casting of list returned: This Dynamic type is not list")))?;
    let mut kept: Vec<BundValue> = Vec::new();
    let mut keys: Vec<Key> = Vec::new();
    for item in items {
        let k = key_of(item);
        let seen = fib_contains(&keys, &k).map_err(|e| {
            if e.is_internal() {
                e
            } else {
                Error(format!("{prefix} returns error during the scan: {}", e.0))
            }
        })?;
        if !seen {
            keys.push(k);
            kept.push(item.clone());
        }
    }
    side.push(vm, BundValue::list(kept));
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.register_native(
        "unique",
        |vm| unique_base(vm, crate::wb::Side::Stack),
        eff(1, 1),
        WordKind::Sync,
    );
    r.register_native(
        "unique.",
        |vm| unique_base(vm, crate::wb::Side::Bench),
        eff(0, 0),
        WordKind::Sync,
    );
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

    fn shown(src: &str) -> String {
        let i = run_src(src).unwrap_or_else(|e| panic!("{src}: {e}"));
        i.peek().map(|v| v.display()).expect("a value")
    }

    /// `unique` where the reference is **random**, which no golden can hold
    /// and the probe therefore leaves out — F164, D100.
    ///
    /// The reference's ordering has no float arm and falls through to
    /// comparing ids, so its Fibonacci walk over floats takes arbitrary
    /// branches: six runs of this very list gave `[ 1.0 2.0 2.0 3.0 ]` five
    /// times and `[ 1.0 2.0 3.0 ]` once. Bund2 orders floats by value (F12's
    /// disposition was already FIX), so the answer is one answer.
    ///
    /// Run several times on purpose: the claim is that it does not vary.
    #[test]
    fn unique_on_floats_is_deterministic_where_the_reference_is_not() {
        for _ in 0..8 {
            assert_eq!(
                shown("[ 1.0 1.0 2.0 2.0 3.0 ] unique"),
                "[ 1.0 ::  2.0 ::  3.0 :: ]"
            );
        }
        // An integer against a float is the other pair the reference orders
        // by id and equates by value -- and its `==` is not symmetric. A
        // *new integer* is compared as `i == f as i64`, truncating what is
        // already there, so `1` is a duplicate of `1.9`; a new float is
        // compared as `f == i as f64`, so `1.9` is not a duplicate of `1`.
        // All four measured on the oracle, which answers these the same
        // every run: with one member kept, no ordering is consulted.
        assert_eq!(shown("[ 1 1.0 2 ] unique"), "[ 1 ::  2 :: ]");
        assert_eq!(shown("[ 1 1.9 ] unique"), "[ 1 ::  1.9 :: ]");
        assert_eq!(shown("[ 1.9 1 ] unique"), "[ 1.9 :: ]");
        assert_eq!(shown("[ 2.5 2 ] unique"), "[ 2.5 :: ]");
    }

    /// The failure a golden cannot hold, and the two guards.
    ///
    /// **The error comes at the third member, not the second.** The
    /// accumulator is tested for order only when it is next *searched*, so a
    /// descending pair is returned untouched and a descending triple fails.
    #[test]
    fn unique_refuses_disordered_numbers_at_the_third_member() {
        assert_eq!(shown("[ 2 1 ] unique"), "[ 2 ::  1 :: ]");
        for src in ["[ 2 1 1 ] unique", "[ 3 1 3 1 2 ] unique", "[ 5 4 3 2 1 ] unique"] {
            let e = match run_src(src) {
                Ok(_) => panic!("{src} did not fail"),
                Err(e) => e,
            };
            assert!(
                e.ends_with(
                    "UNIQUE returns error during the scan: \
                     Invalid input: Fibonacci search requires sorted input"
                ),
                "{src}: {e}"
            );
        }
        for (src, want) in [
            ("unique", "Stack is too shallow for inline UNIQUE"),
            ("unique.", "Workbench is too shallow for inline UNIQUE."),
            (
                "42 unique",
                "UNIQUE casting of list returned: This is not a LIST/PAIR value but 2",
            ),
        ] {
            let e = match run_src(src) {
                Ok(_) => panic!("{src} did not fail"),
                Err(e) => e,
            };
            assert!(e.ends_with(want), "{src}:\n  got {e}\n want ...{want}");
        }
    }

    /// The walk's own arithmetic, over every position of every length up to
    /// 40: a value present in a sorted run is found, and one absent is not.
    ///
    /// The differential against the oracle is what says the *quirks* match;
    /// this says the port has no off-by-one of its own, and that the loop
    /// D39 bounds never reaches its bound.
    #[test]
    fn the_fibonacci_walk_finds_exactly_what_a_sorted_run_holds() {
        for len in 1..=40i64 {
            let data: Vec<Key> = (0..len).map(|x| Key::Int(x * 2)).collect();
            for x in 0..len {
                assert!(
                    fib_contains(&data, &Key::Int(x * 2)).expect("sorted"),
                    "len {len}: {} should be found",
                    x * 2
                );
                assert!(
                    !fib_contains(&data, &Key::Int(x * 2 + 1)).expect("sorted"),
                    "len {len}: {} should not be found",
                    x * 2 + 1
                );
            }
        }
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
