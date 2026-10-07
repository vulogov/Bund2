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
//! **The reference's comparison across kinds is inconsistent, and Bund2's is
//! not — D106, D108.** Every arm there ends `_ => return true`, so for an
//! integer against a string both `a > b` and `a <= b` are true, and a list of
//! mixed kinds comes back in whatever order the swaps left. Bund2 gives the
//! sort a total order instead: kinds in a fixed rank, values within a kind.
//! The quicksort is still the transcription, so equal members still move.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, CALL, LIST, PAIR, STRING};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// What `sort` orders a value as. The variants are in rank order: a list of
/// mixed kinds comes back numbers first, then times, then text, then
/// everything else — D108.
enum Rank<'a> {
    Int(i64),
    /// A float, or a complex number by its real part.
    Float(f64),
    Time(u128),
    Text(&'a str),
    /// A list, a dict, a lambda, a bool: no order among them, so they compare
    /// equal and come out together at the end.
    Other,
}

impl Rank<'_> {
    fn kind(&self) -> u8 {
        match self {
            Rank::Int(_) | Rank::Float(_) => 0,
            Rank::Time(_) => 1,
            Rank::Text(_) => 2,
            Rank::Other => 3,
        }
    }
}

fn rank_of(v: &BundValue) -> Rank<'_> {
    use BundValue::{Float, Int};
    match v.unboxed() {
        Int(i, _) => Rank::Int(*i),
        Float(f, _) => Rank::Float(*f),
        u => {
            if let Some(t) = u.as_time() {
                Rank::Time(t)
            } else if let Some(s) = u.str_ref() {
                Rank::Text(s)
            } else if matches!(u.dt(), bund2_value::CFLOAT | bund2_value::CINTEGER)
                && let Some((re, _)) = crate::math::complex_parts(u)
            {
                Rank::Float(re)
            } else {
                Rank::Other
            }
        }
    }
}

/// The order `sort` sorts by. **Total**, which the reference's is not.
///
/// **Within a kind:**
///
/// - *Integers, floats and times* by value, as the reference's `>` and `<=`
///   order them (`reference/rust_dynamic/src/ord.rs:89,97,105` and
///   `:50,58,66`).
/// - *An integer against a float* by the values they denote — D33's order,
///   the one `<` answers by. The reference has no arm for the pair.
/// - *NaN* after every number, and equal to itself. It has no value to sort
///   by; the alternative is a comparison that is not an order.
/// - *A complex number* as its real part, among the numbers. The reference
///   orders two of them that way (`ord.rs:40-42,118-120`). Not measured: no
///   word was found that puts one in a list.
/// - *Text* by code point — D106. Anything holding text: strings, pointers,
///   text buffers.
///
/// **Across kinds** by rank, D108: numbers, times, text, the rest. The
/// reference answers `true` to both `>` and `<=` for every such pair
/// (`ord.rs:94,102,110,121`), which is no order at all.
fn order(a: &BundValue, b: &BundValue) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (x, y) = (rank_of(a), rank_of(b));
    match (&x, &y) {
        (Rank::Int(i), Rank::Int(j)) => i.cmp(j),
        (Rank::Float(f), Rank::Float(g)) => float_order(*f, *g),
        (Rank::Int(i), Rank::Float(f)) => {
            crate::logic::exact_int_float_ord(*i, *f).unwrap_or(Ordering::Less)
        }
        (Rank::Float(f), Rank::Int(i)) => crate::logic::exact_int_float_ord(*i, *f)
            .map_or(Ordering::Greater, Ordering::reverse),
        (Rank::Time(s), Rank::Time(t)) => s.cmp(t),
        (Rank::Text(s), Rank::Text(t)) => s.cmp(t),
        _ => x.kind().cmp(&y.kind()),
    }
}

/// Two floats, with NaN last and equal to itself.
fn float_order(f: f64, g: f64) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match f.partial_cmp(&g) {
        Some(o) => o,
        None => match (f.is_nan(), g.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            _ => Ordering::Less,
        },
    }
}

/// `a > b`, the first of the two questions the quicksort asks.
fn gt(a: &BundValue, b: &BundValue) -> bool {
    order(a, b) == std::cmp::Ordering::Greater
}

/// `a <= b`, the second.
fn le(a: &BundValue, b: &BundValue) -> bool {
    order(a, b) != std::cmp::Ordering::Greater
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

/// What makes two members of a list the same member, for `unique` — D109.
///
/// Equal keys are equal members and nothing else is, so a set of these
/// answers "is one already kept?" in one lookup.
///
/// - **Numbers by the value they denote**, which is what `==` answers by
///   (D30): `1` and `1.0` are one member, `1` and `1.9` are two. A float that
///   is a whole number inside `i64` takes the integer's key; any other float
///   keeps its own bits, with `-0.0` folded into `0.0`.
/// - **Times** by their count, **complex numbers** by both parts.
/// - **Text** by its characters. A STRING and a CALL, as the reference's
///   equality has it — both carry `Val::String` there — which is why
///   `[ true true ]` is one member: two CALLs named `true`, not two booleans.
///
/// `None` is a value equal to nothing, itself included: NaN, as `==` has it,
/// and everything the reference equates by id — a list, a dict, a lambda.
/// Two members of one list never share an id, so they are always both kept.
#[derive(PartialEq, Eq, Hash)]
enum Same {
    Int(i64),
    Float(u64),
    Time(u128),
    Complex(u64, u64),
    Text(String),
}

fn same_of(v: &BundValue) -> Option<Same> {
    use BundValue::{Float, Int};
    match v.unboxed() {
        Int(i, _) => Some(Same::Int(*i)),
        Float(f, _) => float_key(*f),
        u => {
            if let Some(t) = u.as_time() {
                Some(Same::Time(t))
            } else if u.dt() == STRING || u.dt() == CALL {
                u.as_str().map(Same::Text)
            } else if matches!(u.dt(), bund2_value::CFLOAT | bund2_value::CINTEGER) {
                let (re, im) = crate::math::complex_parts(u)?;
                if re.is_nan() || im.is_nan() {
                    return None;
                }
                // `+ 0.0` turns `-0.0` into `0.0` and changes nothing else.
                Some(Same::Complex((re + 0.0).to_bits(), (im + 0.0).to_bits()))
            } else {
                None
            }
        }
    }
}

/// A float's key: the integer it denotes when it denotes one, else its bits.
fn float_key(f: f64) -> Option<Same> {
    if f.is_nan() {
        return None;
    }
    // The same test `==` makes (`exact_int_float`): whole, and inside `i64`.
    // The range comes first because `as i64` saturates.
    if f.fract() == 0.0 && f >= -(2f64.powi(63)) && f < 2f64.powi(63) {
        #[expect(clippy::cast_possible_truncation, reason = "range-checked above")]
        return Some(Same::Int(f as i64));
    }
    Some(Same::Float(f.to_bits()))
}

/// `unique` and `unique.` — drop the members already seen
/// (`reference/Bund/src/stdlib/functions/values/listop.rs`,
/// `unique_list_base`).
///
/// **A member is dropped when an equal one is already kept.** First
/// occurrences stay, in the order they came, and no list is refused.
/// [`Same`] says what equal means.
///
/// **This is not what the reference does, and D109 says so.** It asks with a
/// Fibonacci search, which wants sorted data. Measured against the oracle:
///
/// | input | the reference | Bund2 |
/// |---|---|---|
/// | `[ 1 1 2 2 3 ]` | `[ 1 2 3 ]` | the same |
/// | `[ 2 1 ]` | `[ 2 1 ]` | the same |
/// | `[ 2 1 1 ]` | **refused**: "requires sorted input" | `[ 2 1 ]` |
/// | `[ "c" "b" "a" "c" "a" ]` | `[ c b a a ]` | `[ c b a ]` |
/// | `[ 1.9 1 ]` | `[ 1.9 ]` — the float truncated | both kept |
/// | `[ [ 1 ] [ 1 ] ]` | both kept | the same |
///
/// On floats the reference's answer changes between runs (F164).
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
    let mut seen: std::collections::HashSet<Same> = std::collections::HashSet::new();
    for item in items {
        let fresh = match same_of(item) {
            Some(key) => seen.insert(key),
            None => true,
        };
        if fresh {
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

    /// **D107, D109.** No list keeps a repeat, whatever its order or kind.
    /// Each line is commented with what the reference answers where that
    /// differs.
    #[test]
    fn unique_leaves_no_repeat() {
        for (src, want) in [
            // the reference: `[ c b a a ]`
            ("[ \"c\" \"b\" \"a\" \"c\" \"a\" ] unique", "[ c ::  b ::  a :: ]"),
            ("[ \"a\" \"a\" \"b\" ] unique", "[ a ::  b :: ]"),
            ("[ \"b\" \"a\" \"b\" ] unique", "[ b ::  a :: ]"),
            ("[ true true false ] unique", "[ F(true) ::  F(false) :: ]"),
            ("[ 1 1 2 2 3 3 ] unique", "[ 1 ::  2 ::  3 :: ]"),
            ("[ 2 1 ] unique", "[ 2 ::  1 :: ]"),
            // the reference refuses these three: "requires sorted input"
            ("[ 2 1 1 ] unique", "[ 2 ::  1 :: ]"),
            ("[ 3 1 3 1 2 ] unique", "[ 3 ::  1 ::  2 :: ]"),
            ("[ 5 4 3 2 1 ] unique", "[ 5 ::  4 ::  3 ::  2 ::  1 :: ]"),
            // the reference answers this one differently between runs (F164)
            ("[ 1.0 1.0 2.0 2.0 3.0 ] unique", "[ 1.0 ::  2.0 ::  3.0 :: ]"),
            ("[ 2.5 1.5 2.5 ] unique", "[ 2.5 ::  1.5 :: ]"),
            ("[ 1 \"a\" 1 \"a\" ] unique", "[ 1 ::  a :: ]"),
            ("[ [ 1 ] [ 1 ] ] unique", "[ [ 1 :: ] ::  [ 1 :: ] :: ]"),
        ] {
            assert_eq!(shown(src), want, "{src}");
        }
    }

    /// **An integer and a float are one member when they denote one value**,
    /// which is what `==` says of them (D30). The reference truncates the
    /// float when the integer comes second, so its `[ 1.9 1 ]` is `[ 1.9 ]`
    /// and its `[ 2.5 2 ]` is `[ 2.5 ]`.
    #[test]
    fn unique_equates_an_integer_and_a_float_as_the_equality_word_does() {
        assert_eq!(shown("[ 1 1.0 2 ] unique"), "[ 1 ::  2 :: ]");
        assert_eq!(shown("[ 1.0 1 2 ] unique"), "[ 1.0 ::  2 :: ]");
        assert_eq!(shown("[ 1 1.9 ] unique"), "[ 1 ::  1.9 :: ]");
        assert_eq!(shown("[ 1.9 1 ] unique"), "[ 1.9 ::  1 :: ]");
        assert_eq!(shown("[ 2.5 2 ] unique"), "[ 2.5 ::  2 :: ]");
        assert_eq!(shown("[ 0.0 -0.0 0 ] unique"), "[ 0.0 :: ]");
    }

    /// The key agrees with `==` at the edges a cast gets wrong: a float past
    /// `i64` is not the integer it would saturate to, and NaN is nothing.
    #[test]
    fn the_key_is_the_equality_words_at_the_edges() {
        assert!(float_key(f64::NAN).is_none());
        assert!(float_key(3.0) == Some(Same::Int(3)));
        assert!(float_key(-0.0) == Some(Same::Int(0)));
        assert!(float_key(1e30) != Some(Same::Int(i64::MAX)));
        assert!(float_key(2f64.powi(63)) != Some(Same::Int(i64::MAX)));
        assert!(float_key(-(2f64.powi(63))) == Some(Same::Int(i64::MIN)));
        assert!(float_key(2.5) == float_key(2.5));
        assert!(float_key(f64::INFINITY) != float_key(f64::NEG_INFINITY));
        for (i, f) in [(3i64, 3.0), (0, -0.0), (i64::MIN, -(2f64.powi(63)))] {
            assert!(crate::logic::exact_int_float(i, f), "{i} {f}");
        }
    }

    /// The guards, which no golden holds.
    #[test]
    fn unique_reports_a_shallow_side_and_a_non_list() {
        for (src, want) in [
            ("unique", "Stack is too shallow for inline UNIQUE"),
            ("unique.", "Workbench is too shallow for inline UNIQUE."),
            (
                "42 unique",
                "UNIQUE casting of list returned: This is not a LIST/PAIR value but 2",
            ),
        ] {
            match run_src(src) {
                Ok(_) => panic!("{src} did not fail"),
                Err(e) => assert!(e.ends_with(want), "{src}: {e}"),
            }
        }
    }

    /// **D108.** A list of mixed kinds sorts numbers, then text, then the
    /// rest, and an integer and a float sort together by value.
    #[test]
    fn a_mixed_list_sorts_by_kind_and_then_by_value() {
        assert_eq!(
            shown("[ \"b\" 2 \"a\" 1.5 1 \"c\" 0.5 ] sort"),
            "[ 0.5 ::  1 ::  1.5 ::  2 ::  a ::  b ::  c :: ]"
        );
        assert_eq!(shown("[ 3 1.5 2 0.5 1 ] sort"), "[ 0.5 ::  1 ::  1.5 ::  2 ::  3 :: ]");
        assert_eq!(
            shown("[ \"z\" [ 9 ] 3 \"a\" 1 ] sort"),
            "[ 1 ::  3 ::  a ::  z ::  [ 9 :: ] :: ]"
        );
        // Over the insertion-sort threshold, where the partition runs.
        assert_eq!(
            shown("[ \"d\" 4 \"b\" 2.5 9 \"a\" 1 7.5 \"c\" 3 0.5 \"e\" 8 ] sort"),
            "[ 0.5 ::  1 ::  2.5 ::  3 ::  4 ::  7.5 ::  8 ::  9 ::  a ::  b ::  c ::  d ::  e :: ]"
        );
    }

    /// The order is one: for every pair exactly one of less, equal, greater,
    /// and it reverses when the pair does. NaN is in the palette on purpose.
    #[test]
    fn the_order_is_total_over_a_palette() {
        use std::cmp::Ordering;
        let palette = [
            BundValue::int(-1),
            BundValue::int(2),
            BundValue::float(2.0),
            BundValue::float(2.5),
            BundValue::float(f64::NAN),
            BundValue::float(f64::INFINITY),
            BundValue::str("a"),
            BundValue::str("b"),
            BundValue::list(vec![BundValue::int(1)]),
        ];
        for a in &palette {
            assert_eq!(order(a, a), Ordering::Equal);
            for b in &palette {
                assert_eq!(order(a, b), order(b, a).reverse());
                for c in &palette {
                    if order(a, b) != Ordering::Greater && order(b, c) != Ordering::Greater {
                        assert_ne!(order(a, c), Ordering::Greater);
                    }
                }
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

    fn sorted_text(src: &str) -> Vec<String> {
        let i = run_src(src).expect("runs");
        let top = i.peek().expect("a list");
        top.as_list()
            .expect("a list")
            .iter()
            .map(|v| v.as_str().expect("text"))
            .collect()
    }

    /// **D106.** Strings sort by code point, under the insertion-sort
    /// threshold and over it. The reference returns a fixed shuffle for both
    /// — `[ c a b ]` for the first — so no golden can hold this.
    #[test]
    fn strings_sort_by_code_point() {
        assert_eq!(sorted_text("[ \"b\" \"a\" \"c\" ] sort"), ["a", "b", "c"]);
        assert_eq!(
            sorted_text(
                "[ \"pear\" \"apple\" \"fig\" \"kiwi\" \"lemon\" \"grape\" \"mango\" \
                 \"cherry\" \"plum\" \"lime\" \"date\" \"peach\" \"melon\" ] sort"
            ),
            [
                "apple", "cherry", "date", "fig", "grape", "kiwi", "lemon", "lime", "mango",
                "melon", "peach", "pear", "plum"
            ]
        );
    }

    /// Code point order is not an alphabet, and the test says which it is:
    /// the empty string first, digits as text, capitals before small letters,
    /// an accented letter after `z`.
    #[test]
    fn code_point_order_is_not_a_dictionary_s() {
        assert_eq!(
            sorted_text("[ \"b\" \"B\" \"a\" \"é\" \"z\" \"10\" \"9\" \"\" \"ab\" ] sort"),
            ["", "10", "9", "B", "a", "ab", "b", "z", "é"]
        );
    }

    /// The order is the payload's, as the reference's `cmp` arm is, so
    /// pointers sort among themselves and with strings.
    #[test]
    fn pointers_sort_as_the_text_they_hold() {
        assert_eq!(sorted_text("[ :b :a \"c\" :d ] sort"), ["a", "b", "c", "d"]);
    }

    /// **Times sort by value, as the oracle's do** — measured: 5 1 9 3 7
    /// comes back 1 3 5 7 9. The arm was missing from before D103.
    #[test]
    fn times_sort_by_value() {
        let i = run_src(
            "[ 0 ] 5 time.timestamp + 1 time.timestamp + 9 time.timestamp + \
             3 time.timestamp + 7 time.timestamp + cdr sort",
        )
        .expect("runs");
        let top = i.peek().expect("a list");
        let got: Vec<u128> = top
            .as_list()
            .expect("a list")
            .iter()
            .map(|v| v.as_time().expect("a time"))
            .collect();
        assert_eq!(got, [1, 3, 5, 7, 9]);
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
