//! **Temporary** — D152. Bund2's `io.banner` against `cfonts`, the crate it
//! replaces, over every ASCII character and a few thousand mixed strings.
//! `cfonts` is a dev-dependency for this file alone and is linked into
//! nothing shipped; both go once this has passed on Linux as well as macOS.

#![allow(clippy::expect_used, clippy::panic)]

use bund2_api::Vm;
use bund2_interp::Interp;
use bund2_value::BundValue;

fn theirs(text: &str) -> String {
    cfonts::render(cfonts::Options {
        text: text.to_string(),
        font: cfonts::Fonts::FontTiny,
        ..cfonts::Options::default()
    })
    .text
}

fn ours(i: &mut Interp, text: &str) -> String {
    i.push(BundValue::str(text));
    let stream = bund2_syntax::compile("io.banner").expect("parses");
    i.eval(&stream).expect("io.banner");
    i.pull().and_then(|v| v.as_str()).expect("a string")
}

#[test]
fn the_banner_is_the_one_cfonts_draws() {
    let mut i = Interp::new();
    bund2_stdlib::register_all(&mut i.registry);
    let mut cases: Vec<String> = vec![String::new()];
    // Every ASCII character alone, between two letters, and doubled.
    for b in 0u8..=0x7f {
        let c = char::from(b);
        cases.push(c.to_string());
        cases.push(format!("A{c}B"));
        cases.push(format!("{c}{c}"));
    }
    // Beyond ASCII, and lines long enough to wrap more than once.
    for s in ["é", "AéB", "日本", "a\u{a0}b", "🙂", "ß"] {
        cases.push(s.to_string());
    }
    for c in ['I', 'M', 'W', ' ', '.', 'a', '|'] {
        for n in [39, 40, 41, 79, 80, 81, 200, 500] {
            cases.push(c.to_string().repeat(n));
        }
    }
    // Mixed strings from a fixed sequence, so a failure can be run again.
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as usize
    };
    for _ in 0..3000 {
        let len = next() % 140;
        let s: String = (0..len)
            .map(|_| char::from((next() % 0x60) as u8 + 0x20))
            .collect();
        cases.push(s);
    }
    for text in &cases {
        let (a, b) = (ours(&mut i, text), theirs(text));
        assert!(a == b, "for {text:?}\n ours: {a:?}\ntheirs: {b:?}");
    }
    assert!(cases.len() > 3000);
}
