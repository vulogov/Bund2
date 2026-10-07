//! **`math.normalize` and `math.smoothing`, and the source reader they share.**
//!
//! Both are `bund/math`, which is in scope. The helper they call —
//! `statistics::get_data::get_data` — lives in `bund/statistics`, which D28
//! deferred for having "42 registered names, zero corpus uses". **So the
//! helper is in scope and the words that gave it a home are not**, which is
//! the shape D87 met with the bus: a deferred directory holding something an
//! in-scope word needs. Nothing here registers a statistics word.
//!
//! **`get_data` has three sources and the top of the stack chooses.** A LIST
//! is taken whole, a METRICS yields each sample's `data`, and anything else
//! **drains the stack** value by value until it empties or meets a `NODATA`
//! sentinel. The drain is why `1.0 2.0 3.0 math.normalize` answers
//! `[1.0, 0.5, 0.0]` and not `[0.0, 0.5, 1.0]`: it pulls from the top, so the
//! series arrives reversed. Measured on the oracle.
//!
//! **`Keep` restores only the first two sources.** A LIST or a METRICS is
//! pushed back; a drained stack is not, because there is nothing left to
//! restore it onto and the reference does not try. So `math.normalize,` keeps
//! a list and `1.0 2.0 math.normalize,` keeps nothing — faithful, and a shape
//! the `,` spelling does not suggest.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, FLOAT, LIST, METRICS, NODATA};

use crate::wb::Side;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// Whether the source value survives being read — the reference's
/// `statistics::SourceMode`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Consume,
    Keep,
}

/// The series a word is to work on.
fn get_data(vm: &mut dyn Vm, side: Side, mode: Mode, prefix: &str) -> Result<Vec<f64>, Error> {
    let shallow = || match side {
        Side::Stack => Error(format!("Stack is too shallow for inline {prefix}")),
        Side::Bench => Error(format!("Workbench is too shallow for inline {prefix}")),
    };
    if side.depth(vm) < 1 {
        return Err(shallow());
    }
    // **Peeked, not pulled**: the tag decides which reader runs, and two of
    // the three put the value back.
    let top = match side {
        Side::Stack => vm.peek(),
        Side::Bench => vm.snapshot_workbench().last().cloned(),
    }
    .ok_or_else(|| Error(format!("{prefix} returns NO DATA")))?;

    match top.dt() {
        LIST => from_list(vm, side, mode, prefix),
        METRICS => from_metrics(vm, side, mode, prefix),
        // A `NODATA` on top is the end of a series rather than an empty one.
        NODATA => Err(Error(format!("{prefix} END OF DATA"))),
        _ => drain(vm, side, prefix),
    }
}

fn from_list(vm: &mut dyn Vm, side: Side, mode: Mode, prefix: &str) -> Result<Vec<f64>, Error> {
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA")))?;
    if v.dt() != LIST {
        return Err(Error(format!(
            "{prefix} did not find a list type on the stack"
        )));
    }
    let items = v.as_list().unwrap_or(&[]).to_vec();
    let mut out = Vec::with_capacity(items.len());
    for item in &items {
        let f = crate::convert::conv_value(item, FLOAT)
            .map_err(|e| Error(format!("{prefix} error FLOAT conversion: {}", e.0)))?;
        let n = f.unboxed();
        match n {
            BundValue::Float(x, _) => out.push(*x),
            _ => {
                return Err(Error(format!(
                    "{prefix} error FLOAT casting: This Dynamic type is not float"
                )))
            }
        }
    }
    if mode == Mode::Keep {
        side.push(vm, v);
    }
    Ok(out)
}

fn from_metrics(vm: &mut dyn Vm, side: Side, mode: Mode, prefix: &str) -> Result<Vec<f64>, Error> {
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA")))?;
    if v.dt() != METRICS {
        return Err(Error(format!(
            "{prefix} did not find a metric type on the stack"
        )));
    }
    let out: Vec<f64> = v
        .as_metrics()
        .unwrap_or(&[])
        .iter()
        .map(|m| m.data)
        .collect();
    if mode == Mode::Keep {
        side.push(vm, v);
    }
    Ok(out)
}

/// **The drain, and `Mode` does not reach it.** The reference's third reader
/// takes no `smode` argument at all, so `Keep` restores nothing here — a
/// `math.normalize,` over loose values empties the stack exactly as
/// `math.normalize` does.
fn drain(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<Vec<f64>, Error> {
    let mut out = Vec::new();
    while let Some(v) = side.pull(vm) {
        // The sentinel ends the series and is consumed with it.
        if v.dt() == NODATA {
            break;
        }
        let f = crate::convert::conv_value(&v, FLOAT)
            .map_err(|e| Error(format!("{prefix} returns during conversion: {}", e.0)))?;
        match f.unboxed() {
            BundValue::Float(x, _) => out.push(*x),
            _ => {
                return Err(Error(format!(
                    "{prefix} returns during conversion: This Dynamic type is not float"
                )))
            }
        }
    }
    Ok(out)
}

/// `math.normalize` — each sample as its position between the extremes.
///
/// **An all-equal series is returned unchanged rather than as zeroes.** The
/// reference guards `max - min == 0.0` and pushes the originals, which is the
/// only sane answer and not the one `(v-min)/(max-min)` would give. Measured:
/// `[ 2 2 2 ] math.normalize` answers `[2.0, 2.0, 2.0]`.
fn normalize(vm: &mut dyn Vm, side: Side, mode: Mode, prefix: &str) -> Result<(), Error> {
    let source = get_data(vm, side, mode, prefix)
        .map_err(|e| Error(format!("{prefix} returned: {}", e.0)))?;
    // `fold` from NaN, as the reference does: `f64::min` and `f64::max` ignore
    // a NaN accumulator, so an empty series leaves NaN and `max - min` is not
    // zero — which is the arm that would divide. An empty series produces an
    // empty list either way, because the loop below does not run.
    let min = source.iter().copied().fold(f64::NAN, f64::min);
    let max = source.iter().copied().fold(f64::NAN, f64::max);
    let flat = max - min == 0.0;
    let out: Vec<BundValue> = source
        .iter()
        .map(|v| BundValue::float(if flat { *v } else { (v - min) / (max - min) }))
        .collect();
    side.push(vm, BundValue::list(out));
    Ok(())
}

/// `math.smoothing` — a three-period simple moving average.
///
/// **Through `ta`, the reference's own crate, for the reason comfy-table is a
/// dependency.** Its SMA is incremental — `sum = sum - old + input; sum /
/// count` — and a naive re-sum of the window gives different floating-point
/// answers for some series. A golden captures those floats, so a near-miss
/// would fail one while looking right.
///
/// **The window fills rather than waits**: the average is over however many
/// samples have arrived, so `[ 1 2 3 4 5 ]` answers `[1, 1.5, 2, 3, 4]` and
/// not three leading zeroes. Measured.
fn smoothing(vm: &mut dyn Vm, side: Side, mode: Mode, prefix: &str) -> Result<(), Error> {
    use ta::Next as _;
    let source = get_data(vm, side, mode, prefix)
        .map_err(|e| Error(format!("{prefix} returned: {}", e.0)))?;
    // `new(3)` is the reference's period and its `unwrap` — a period of 3 is
    // never the error case, and `Error::internal` names the invariant rather
    // than asserting it (D37).
    let mut sma = ta::indicators::SimpleMovingAverage::new(3)
        .map_err(|e| Error::internal(format!("a three-period SMA was refused: {e}")))?;
    let out: Vec<BundValue> = source
        .iter()
        .map(|v| BundValue::float(sma.next(*v)))
        .collect();
    side.push(vm, BundValue::list(out));
    Ok(())
}

/// `seq.asc` and `seq.desc` — a range of floats.
///
/// **Three operands, pulled x then step then n**, so the stack reads
/// `n step x` bottom to top: `10 1.0 0.0 seq.asc` is ten values from zero.
///
/// **`cast_float` is strict in the reference and so is this.** `10 1 0
/// seq.asc` fails with `Casting X returns: This Dynamic type is not float`,
/// because `cast_float` asserts the type rather than converting it — unlike
/// the `conv(FLOAT)` that `get_data` uses. Measured; an earlier reading of
/// this family assumed integers would do.
fn seq_ranged(vm: &mut dyn Vm, order: &'static str) -> Result<(), Error> {
    if vm.depth() < 3 {
        return Err(Error("Stack is too shallow for inline SEQ".into()));
    }
    let x = strict_float(vm.pull(), "X", 1)?;
    let step = strict_float(vm.pull(), "Step", 2)?;
    let n = vm
        .pull()
        .ok_or_else(|| Error("SEQ_OP returns: NO DATA #3".into()))?;
    let n = n
        .as_int()
        .ok_or_else(|| Error("Casting N returns: This Dynamic type is not integer".into()))?;
    // `range` refuses a non-positive step or a size over a million by
    // answering an empty vector, which `5 0.0 1.0 seq.asc` shows.
    let out: Vec<BundValue> = mathlab::functions::args::range(x, step, n as usize, order)
        .into_iter()
        .map(BundValue::float)
        .collect();
    vm.push(BundValue::list(out));
    Ok(())
}

/// One strictly-typed float operand, with the reference's own two messages.
fn strict_float(v: Option<BundValue>, which: &str, nth: u8) -> Result<f64, Error> {
    let v = v.ok_or_else(|| Error(format!("SEQ_OP returns: NO DATA #{nth}")))?;
    match v.unboxed() {
        BundValue::Float(f, _) => Ok(*f),
        other => Err(Error(format!(
            "Casting {which} returns: This Dynamic type is not float: {}",
            other.dt()
        ))),
    }
}

pub fn register(r: &mut Registry) {
    fn n_s(vm: &mut dyn Vm) -> Result<(), Error> {
        normalize(vm, Side::Stack, Mode::Consume, "MATH.NORMALIZE")
    }
    fn n_w(vm: &mut dyn Vm) -> Result<(), Error> {
        normalize(vm, Side::Bench, Mode::Consume, "MATH.NORMALIZE.")
    }
    fn n_sk(vm: &mut dyn Vm) -> Result<(), Error> {
        normalize(vm, Side::Stack, Mode::Keep, "MATH.NORMALIZE,")
    }
    fn n_wk(vm: &mut dyn Vm) -> Result<(), Error> {
        normalize(vm, Side::Bench, Mode::Keep, "MATH.NORMALIZE.,")
    }
    // **The prefix is `MATH.SMOOTH`, not `MATH.SMOOTHING`** — the word is
    // spelled one way and its errors another, in all four forms.
    fn s_s(vm: &mut dyn Vm) -> Result<(), Error> {
        smoothing(vm, Side::Stack, Mode::Consume, "MATH.SMOOTH")
    }
    fn s_w(vm: &mut dyn Vm) -> Result<(), Error> {
        smoothing(vm, Side::Bench, Mode::Consume, "MATH.SMOOTH.")
    }
    fn s_sk(vm: &mut dyn Vm) -> Result<(), Error> {
        smoothing(vm, Side::Stack, Mode::Keep, "MATH.SMOOTH,")
    }
    fn s_wk(vm: &mut dyn Vm) -> Result<(), Error> {
        smoothing(vm, Side::Bench, Mode::Keep, "MATH.SMOOTH.,")
    }
    fn asc(vm: &mut dyn Vm) -> Result<(), Error> {
        seq_ranged(vm, "asc")
    }
    fn desc(vm: &mut dyn Vm) -> Result<(), Error> {
        seq_ranged(vm, "desc")
    }

    // **`opaque(1)`, not a fixed pair.** A drain empties the stack, so what
    // these consume is not a number the audit can model — the same reason
    // `*loop` is opaque. The `.` forms take their series off the workbench and
    // leave their answer there, so they touch the stack not at all.
    r.register_native("math.normalize", n_s, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("math.normalize,", n_sk, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("math.normalize.", n_w, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("math.normalize.,", n_wk, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("math.smoothing", s_s, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("math.smoothing,", s_sk, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("math.smoothing.", s_w, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("math.smoothing.,", s_wk, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("seq.asc", asc, eff(3, 1), WordKind::Sync);
    crate::wb::bench!(r, "seq.asc", asc, 3, 0);
    r.register_native("seq.desc", desc, eff(3, 1), WordKind::Sync);
    crate::wb::bench!(r, "seq.desc", desc, 3, 0);
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    fn run(src: &str) -> Result<Vec<BundValue>, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let ir = bund2_syntax::compile(src).map_err(|e| format!("{e:?}"))?;
        i.eval(&ir).map_err(|e| e.0)?;
        Ok(i.snapshot())
    }

    fn floats(v: &BundValue) -> Vec<f64> {
        v.as_list()
            .unwrap_or(&[])
            .iter()
            .filter_map(|x| match x.unboxed() {
                BundValue::Float(f, _) => Some(*f),
                _ => None,
            })
            .collect()
    }

    /// The three sources `get_data` chooses between, and the reversal.
    #[test]
    fn the_source_is_chosen_by_the_tag_on_top() {
        // A LIST is taken in order.
        let out = run("[ 1 2 3 4 5 ] math.normalize").expect("runs");
        assert_eq!(floats(&out[0]), vec![0.0, 0.25, 0.5, 0.75, 1.0]);

        // **A drained stack arrives reversed**, because the drain pulls from
        // the top. Measured on the oracle, and the reason this is a test
        // rather than a comment.
        let out = run("1.0 2.0 3.0 math.normalize").expect("runs");
        assert_eq!(floats(&out[0]), vec![1.0, 0.5, 0.0]);

        // A `NODATA` on top is the end of a series, not an empty one.
        let e = run("nodata math.normalize").expect_err("END OF DATA");
        assert!(e.ends_with("MATH.NORMALIZE returned: MATH.NORMALIZE END OF DATA"), "{e}");
    }

    /// **An all-equal series comes back unchanged**, which is the guard the
    /// reference writes and not what `(v-min)/(max-min)` would give.
    #[test]
    fn a_flat_series_is_not_divided_by_zero() {
        let out = run("[ 2 2 2 ] math.normalize").expect("runs");
        assert_eq!(floats(&out[0]), vec![2.0, 2.0, 2.0]);
    }

    /// **`Keep` restores a LIST and not a drained stack.** The reference's
    /// third reader takes no mode argument at all, so the `,` spelling means
    /// nothing there — faithful, and not what the spelling suggests.
    #[test]
    fn keep_restores_a_list_and_cannot_restore_a_drain() {
        let out = run("[ 1 2 3 ] math.normalize,").expect("runs");
        assert_eq!(out.len(), 2, "the list and the answer: {out:?}");
        // **The kept list is the original, so its members are still
        // INTEGERs** — `get_data` converts for its own arithmetic and pushes
        // the value it was given back untouched.
        let kept: Vec<i64> = out[0]
            .as_list()
            .unwrap_or(&[])
            .iter()
            .filter_map(BundValue::as_int)
            .collect();
        assert_eq!(kept, vec![1, 2, 3], "the kept list");
        assert_eq!(floats(&out[1]), vec![0.0, 0.5, 1.0], "the answer");

        let out = run("1.0 2.0 math.normalize,").expect("runs");
        assert_eq!(out.len(), 1, "a drain leaves only the answer: {out:?}");
    }

    /// **The window fills rather than waits** — three leading values averaged
    /// over 1, 2 and 3 samples. Measured against the oracle.
    #[test]
    fn smoothing_averages_over_what_has_arrived() {
        let out = run("[ 1 2 3 4 5 ] math.smoothing").expect("runs");
        assert_eq!(floats(&out[0]), vec![1.0, 1.5, 2.0, 3.0, 4.0]);
    }

    /// **The prefix is `MATH.SMOOTH`, not `MATH.SMOOTHING`** — the word is
    /// spelled one way and complains in another, in all four forms.
    #[test]
    fn the_smoothing_prefix_is_not_the_words_name() {
        let e = run("math.smoothing").expect_err("too shallow");
        assert!(
            e.ends_with("MATH.SMOOTH returned: Stack is too shallow for inline MATH.SMOOTH"),
            "{e}"
        );
        let e = run("math.smoothing.").expect_err("too shallow");
        assert!(
            e.ends_with("MATH.SMOOTH. returned: Workbench is too shallow for inline MATH.SMOOTH."),
            "{e}"
        );
    }

    /// **`seq.asc` wants floats for x and step and an integer for n**, and
    /// `cast_float` asserts rather than converts — so integers are refused.
    #[test]
    fn seq_is_strict_about_its_operand_types() {
        let out = run("10 1.0 0.0 seq.asc").expect("runs");
        assert_eq!(floats(&out[0]).len(), 10);
        assert_eq!(floats(&out[0])[0], 0.0);
        assert_eq!(floats(&out[0])[9], 9.0);

        let out = run("5 2.0 10.0 seq.desc").expect("runs");
        assert_eq!(floats(&out[0]), vec![10.0, 8.0, 6.0, 4.0, 2.0]);

        // The dt is in the message, as the reference puts it there.
        let e = run("10 1 0 seq.asc").expect_err("integers are refused");
        assert!(
            e.ends_with("Casting X returns: This Dynamic type is not float: 2"),
            "{e}"
        );
        let e = run("seq.asc").expect_err("too shallow");
        assert!(e.ends_with("Stack is too shallow for inline SEQ"), "{e}");
    }

    /// **A non-positive step answers an empty list**, which is `range`'s own
    /// refusal rather than an error.
    #[test]
    fn a_step_of_zero_is_an_empty_sequence() {
        let out = run("5 0.0 1.0 seq.asc").expect("runs");
        assert!(floats(&out[0]).is_empty(), "{:?}", out[0]);
    }

    /// **F147: the reference's own parser, and untrimmed.** Both halves, since
    /// both were wrong and each is observable on its own.
    #[test]
    fn a_string_that_is_not_a_number_complains_as_the_reference_does() {
        for (src, want) in [
            (
                "\"x\" convert.to_float",
                "Can not convert string to float InvalidNumber(\"x\")",
            ),
            (
                "\"x\" convert.to_int",
                "Can not convert string to integer InvalidNumber(\"x\")",
            ),
            // **Untrimmed**: the reference refuses a padded number, and Bund2
            // used to accept it because std's habit of trimming survived.
            (
                "\" 42 \" convert.to_int",
                "Can not convert string to integer InvalidNumber(\" 42 \")",
            ),
            (
                "\"0x10\" convert.to_int",
                "Can not convert string to integer InvalidNumber(\"0x10\")",
            ),
        ] {
            let e = run(src).expect_err(src);
            assert!(e.ends_with(want), "{src}:\n got {e}\n want …{want}");
        }
        // And the ones that do parse still parse.
        assert_eq!(
            run("\"1.5\" convert.to_float").expect("runs")[0].unboxed(),
            &BundValue::Float(1.5, bund2_value::StackSym::NONE)
        );
    }
}
