//! What the family's tests share: run a program, read its answer, compare
//! with a fixture.

use bund2_api::Vm as _;
use bund2_interp::Interp;
use bund2_value::BundValue;

pub(crate) fn run(src: &str) -> Result<Interp, String> {
    let mut i = Interp::new();
    crate::register_all(&mut i.registry);
    let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
    i.eval(&stream).map_err(|e| {
        e.0.rsplit_once(" returned error: ")
            .map_or(e.0.clone(), |(_, m)| m.to_string())
    })?;
    Ok(i)
}

pub(crate) fn err(src: &str) -> String {
    match run(src) {
        Ok(i) => panic!("{src} was expected to fail: {:?}", i.snapshot()),
        Err(e) => e,
    }
}

/// A FLOAT, or a LIST of them, as f64s.
pub(crate) fn floats(v: &BundValue) -> Vec<f64> {
    match v.unboxed() {
        BundValue::Float(x, _) => vec![*x],
        BundValue::Int(i, _) => vec![*i as f64],
        _ => match v.as_list() {
            Some(items) => items.iter().flat_map(floats).collect(),
            None => panic!("not a number: {v:?}"),
        },
    }
}

/// The top of the stack after `src`, as f64s.
pub(crate) fn top(src: &str) -> Vec<f64> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src} failed: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    floats(&v)
}

/// Each element within `tol`, relative, or absolute beside 0.
pub(crate) fn close(what: &str, got: &[f64], want: &[f64], tol: f64) {
    assert_eq!(got.len(), want.len(), "{what}: {got:?} against {want:?}");
    for (g, w) in got.iter().zip(want) {
        let e = if *w == 0.0 {
            g.abs()
        } else {
            ((g - w) / w).abs()
        };
        assert!(e <= tol, "{what}: {got:?} against {want:?}, {e:e}");
    }
}

/// A fixture line's answer: the text after the last space, or a `[…]`.
pub(crate) fn fixture(text: &str, key: &str) -> Vec<f64> {
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("{key} ")))
        .unwrap_or_else(|| panic!("{key} is not in the fixture"));
    let answer = match line.find('[') {
        Some(i) => line[i + 1..].trim_end_matches(']'),
        None => line.rsplit(' ').next().unwrap_or(""),
    };
    answer
        .split_whitespace()
        .map(|v| v.parse().unwrap_or_else(|_| panic!("{line}")))
        .collect()
}
