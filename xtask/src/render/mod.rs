//! `cargo xtask render` — check Bund2's `Debug` rendering against every
//! rendering the goldens hold.
//!
//! RFC-0001's criterion D3 asks the rendering to reproduce the text the
//! goldens carry. Until now it was checked against **five hand-copied
//! strings** in a unit test, out of **303 renderings across 32 goldens**. Five
//! samples cannot find a discrepancy in the 298 they do not cover, and four of
//! the five findings this implementation has produced came from making a claim
//! executable rather than from reading it again.
//!
//! So this parses each captured rendering back into a `BundValue`, renders it,
//! and compares. A round trip that returns the input is a rendering that
//! matches the reference for that value; anything else is a difference worth
//! looking at.
//!
//! **What a failure here means, and does not.** Bund2 cannot yet construct
//! every value the corpus produces — a `CURRY`, an `ENVELOPE`, a `Metrics`
//! with real contents. A rendering it cannot parse is reported as
//! *unsupported* rather than *wrong*, and the two are counted separately,
//! because conflating "not built yet" with "built wrong" is how a coverage
//! number turns into a pass.

use std::path::Path;

use bund2_value::{BundValue, Payload};

/// One captured rendering.
struct Rendering {
    golden: String,
    text: String,
}

/// Split a golden into the top-level `Value { … }` renderings it contains.
///
/// Brace-counting rather than a regex: renderings nest, and a `Map` holds
/// `Value { … }` inside itself. Strings are tracked so a brace inside `"…"`
/// does not shift the depth — the same trap the corpus lexer records.
fn renderings_in(src: &str, golden: &str) -> Vec<Rendering> {
    let cs: Vec<char> = src.chars().collect();
    let open: Vec<char> = "Value { ".chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i..].starts_with(&open[..]) {
            let start = i;
            let mut depth = 0usize;
            let mut in_str = false;
            let mut j = i;
            while j < cs.len() {
                match cs[j] {
                    '\\' if in_str => j += 1,
                    '"' => in_str = !in_str,
                    '{' if !in_str => depth += 1,
                    '}' if !in_str => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if depth == 0 && j < cs.len() {
                out.push(Rendering {
                    golden: golden.to_string(),
                    text: cs[start..=j].iter().collect(),
                });
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// A cursor over a rendering.
struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn new(s: &'a str) -> Self {
        P {
            s: s.as_bytes(),
            i: 0,
        }
    }
    fn eat(&mut self, lit: &str) -> Result<(), String> {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            Ok(())
        } else {
            let got: String =
                String::from_utf8_lossy(&self.s[self.i..(self.i + 24).min(self.s.len())])
                    .to_string();
            Err(format!("expected {lit:?}, found {got:?}"))
        }
    }
    fn until(&mut self, stop: char) -> String {
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i] != stop as u8 {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.i]).to_string()
    }
    /// A quoted string, **un-escaping** what Rust's `{:?}` escaped.
    ///
    /// The escapes are the point. A newline inside a captured string appears
    /// in the golden as the two characters `\` and `n`, and a parser that
    /// drops the backslash and keeps the `n` produces a string that renders
    /// back one character shorter. `cargo xtask render` reported that as a
    /// renderer difference on `compile_and_apply.golden` until this was fixed
    /// — the harness was wrong, not the renderer, which is why a difference
    /// is worth reading before it is worth believing.
    fn string(&mut self) -> Result<String, String> {
        self.eat("\"")?;
        let mut out = String::new();
        while self.i < self.s.len() {
            match self.s[self.i] {
                b'\\' => {
                    self.i += 1;
                    let c = *self.s.get(self.i).ok_or("escape at end of input")?;
                    out.push(match c {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'0' => '\0',
                        b'\'' => '\'',
                        other => other as char,
                    });
                }
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                c => out.push(c as char),
            }
            self.i += 1;
        }
        Err("unterminated string".into())
    }
}

/// Parse one rendering into a value, or say why not.
fn parse(text: &str) -> Result<BundValue, String> {
    let mut p = P::new(text);
    p.eat("Value { id: ")?;
    let _id = p.string()?;
    p.eat(", stamp: ")?;
    let _stamp = p.until(',');
    p.eat(", dt: ")?;
    let dt: u16 = p.until(',').parse().map_err(|_| "bad dt".to_string())?;
    p.eat(", q: ")?;
    let q: f64 = p.until(',').parse().map_err(|_| "bad q".to_string())?;
    p.eat(", data: ")?;
    let payload = parse_payload(&mut p)?;
    // A populated `attr` is reported unsupported rather than silently
    // rendered without it — the harness must not manufacture a match.
    p.eat(", attr: [")?;
    if !p.s[p.i..].starts_with(b"]") {
        return Err("populated attr not reconstructible yet".into());
    }
    p.eat("], curr: ")?;
    let curr: i32 = p.until(',').parse().map_err(|_| "bad curr".to_string())?;
    if curr != -1 {
        return Err("advanced curr not reconstructible yet".into());
    }
    // `tags` is real state — `TS::push` writes one on every push — so a
    // reconstruction that dropped it would differ from every captured stack
    // value, which is exactly what the first run of this harness reported.
    p.eat(", tags: {")?;
    let mut tags = std::collections::BTreeMap::new();
    if p.eat("}").is_err() {
        loop {
            let k = p.string()?;
            p.eat(": ")?;
            let v = p.string()?;
            tags.insert(k, v);
            if p.eat(", ").is_err() {
                p.eat("}")?;
                break;
            }
        }
    }
    Ok(BundValue::with_dt(dt, payload).with_q(q).with_tags(tags))
}

fn parse_payload(p: &mut P) -> Result<Payload, String> {
    if p.eat("I64(").is_ok() {
        let n: i64 = p.until(')').parse().map_err(|_| "bad i64".to_string())?;
        p.eat(")")?;
        return Ok(Payload::Scalar(BundValue::Int(n)));
    }
    if p.eat("F64(").is_ok() {
        let f: f64 = p.until(')').parse().map_err(|_| "bad f64".to_string())?;
        p.eat(")")?;
        return Ok(Payload::Scalar(BundValue::Float(f)));
    }
    if p.eat("Bool(").is_ok() {
        let b = p.until(')') == "true";
        p.eat(")")?;
        return Ok(Payload::Scalar(BundValue::Bool(b)));
    }
    if p.eat("Null").is_ok() {
        return Ok(Payload::Scalar(BundValue::Nodata));
    }
    if p.eat("String(").is_ok() {
        let s = p.string()?;
        p.eat(")")?;
        return Ok(Payload::Str(s));
    }
    if p.eat("List([").is_ok() {
        let mut v = Vec::new();
        if p.eat("]").is_err() {
            loop {
                let start = p.i;
                let rest = String::from_utf8_lossy(&p.s[start..]).to_string();
                let inner = first_rendering(&rest).ok_or("bad list element")?;
                v.push(parse(&inner)?);
                p.i += inner.len();
                if p.eat(", ").is_err() {
                    p.eat("]")?;
                    break;
                }
            }
        }
        p.eat(")")?;
        return Ok(Payload::List(v));
    }
    let head: String = String::from_utf8_lossy(&p.s[p.i..(p.i + 12).min(p.s.len())]).to_string();
    Err(format!("unsupported payload: {head}"))
}

/// The first complete `Value { … }` at the head of `s`.
fn first_rendering(s: &str) -> Option<String> {
    renderings_in(s, "").into_iter().next().map(|r| r.text)
}

pub fn run(_args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let mut all = Vec::new();
    collect(&repo.join("tests/golden"), &repo, &mut all);

    println!("# cargo xtask render\n");
    println!("RFC-0001 criterion D3, checked against every rendering the");
    println!("goldens hold rather than against five hand-copied samples.\n");
    println!("Each captured rendering is parsed into a `BundValue`, rendered,");
    println!("and compared. A round trip that returns its input is a rendering");
    println!("that matches the reference for that value.\n");

    let (mut matched, mut differed, mut unsupported) = (0usize, 0usize, 0usize);
    let mut diffs: Vec<(String, String, String)> = Vec::new();
    let mut reasons: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    for r in &all {
        match parse(&r.text) {
            Ok(v) => {
                let got = v.render(true);
                if got == r.text {
                    matched += 1;
                } else {
                    differed += 1;
                    if diffs.len() < 10 {
                        diffs.push((r.golden.clone(), r.text.clone(), got));
                    }
                }
            }
            Err(why) => {
                unsupported += 1;
                *reasons.entry(short(&why)).or_default() += 1;
            }
        }
    }

    println!("  renderings in tests/golden        {:>5}", all.len());
    println!("  round-tripped identically         {:>5}", matched);
    println!("  differed                          {:>5}", differed);
    println!("  not constructible yet             {:>5}", unsupported);
    println!();

    if !reasons.is_empty() {
        println!("## not constructible yet, by reason\n");
        println!("  These are values Bund2 cannot build, not renderings it gets");
        println!("  wrong. The two are counted apart on purpose.\n");
        let mut rows: Vec<_> = reasons.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1));
        for (why, n) in rows {
            println!("  {n:>5}  {why}");
        }
        println!();
    }

    if !diffs.is_empty() {
        println!("## differences\n");
        for (g, want, got) in &diffs {
            println!("  {g}");
            println!("      want: {}", trunc(want));
            println!("      got:  {}", trunc(got));
        }
        println!();
        return Err(format!("{differed} rendering(s) differ"));
    }
    println!("  No rendering that Bund2 can build renders differently.\n");
    Ok(())
}

fn short(s: &str) -> String {
    s.split(':').next().unwrap_or(s).trim().to_string()
}

fn trunc(s: &str) -> String {
    if s.chars().count() <= 110 {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(110).collect::<String>())
    }
}

fn collect(dir: &Path, repo: &Path, out: &mut Vec<Rendering>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            collect(&p, repo, out);
        } else if p.extension().is_some_and(|e| e == "golden")
            && let Ok(src) = std::fs::read_to_string(&p)
        {
            let name = p
                .strip_prefix(repo)
                .unwrap_or(&p)
                .to_string_lossy()
                .to_string();
            out.extend(renderings_in(&src, &name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nested_rendering_is_split_at_the_right_brace() {
        let src = r#"Value { id: "a", data: Map({"k": Value { id: "b" }}) }"#;
        let found = renderings_in(src, "t");
        assert_eq!(found.len(), 1, "the outer value is one rendering");
        assert!(found[0].text.ends_with("}) }"));
    }

    /// A brace inside a string must not shift the depth — the trap the corpus
    /// lexer records for `bund.pest`.
    #[test]
    fn a_brace_inside_a_string_is_not_a_brace() {
        let src = r#"Value { id: "a", data: String("{"), x: 1 }"#;
        let found = renderings_in(src, "t");
        assert_eq!(found.len(), 1);
        assert!(found[0].text.ends_with("x: 1 }"));
    }

    #[test]
    fn an_integer_rendering_round_trips() {
        let t = r#"Value { id: "<id>", stamp: <stamp>, dt: 2, q: 100.0, data: I64(42), attr: [], curr: -1, tags: {} }"#;
        let v = parse(t).expect("parses");
        assert_eq!(v.render(true), t);
    }
}
