//! RFC-0017 §S8's bounded texts: how a refusal shows a value it takes from
//! the program.
//!
//! `summary` quotes and truncates a STRING but does not escape it, so a
//! STRING holding a line break would break the refusal across lines. The
//! frame words render a STRING themselves, in three steps:
//!
//! 1. **escape**: `\` as `\\`, `"` as `\"`, line feed, carriage return and
//!    tab as `\n`, `\r` and `\t`, any other control character
//!    (`char::is_control`) and U+2028 and U+2029 as `\u{` and its code point
//!    in lower-case hex and `}`;
//! 2. **truncate** the escaped text to at most `WIDTH` − 2 characters, the
//!    last of them `…` when cut, the cut falling between escapes;
//! 3. **quote** it.
//!
//! A number, a BOOL and NONE show as `summary` shows them, NONE as `none`.

use bund2_value::{BundValue, STRING};

use crate::biomath::operand::WIDTH;

/// One character, escaped.
fn escape(c: char) -> String {
    match c {
        '\\' => "\\\\".into(),
        '"' => "\\\"".into(),
        '\n' => "\\n".into(),
        '\r' => "\\r".into(),
        '\t' => "\\t".into(),
        c if c.is_control() || c == '\u{2028}' || c == '\u{2029}' => {
            format!("\\u{{{:x}}}", u32::from(c))
        }
        c => c.to_string(),
    }
}

/// A STRING's text, escaped, bounded and quoted.
pub(crate) fn text(s: &str) -> String {
    let room = WIDTH - 2;
    let pieces: Vec<String> = s.chars().map(escape).collect();
    let total: usize = pieces.iter().map(|p| p.chars().count()).sum();
    let mut out = String::from("\"");
    if total <= room {
        for p in &pieces {
            out.push_str(p);
        }
    } else {
        let mut used = 0;
        for p in &pieces {
            let n = p.chars().count();
            if used + n > room - 1 {
                break;
            }
            out.push_str(p);
            used += n;
        }
        out.push('…');
    }
    out.push('"');
    out
}

/// A value a refusal interpolates: a STRING by [`text`], anything else as
/// `summary` shows it.
pub(crate) fn value(v: &BundValue) -> String {
    if v.dt() == STRING {
        text(&v.as_str().unwrap_or_default())
    } else {
        v.summary(WIDTH)
    }
}

#[cfg(test)]
mod tests {
    use super::text;

    #[test]
    fn escapes_then_bounds_then_quotes() {
        assert_eq!(text("dose"), "\"dose\"");
        assert_eq!(text("a\"b\\c\nd\re\tf"), r#""a\"b\\c\nd\re\tf""#);
        assert_eq!(text("\u{1b}\u{2028}"), r#""\u{1b}\u{2028}""#);
        // 38 characters fit; 39 do not.
        let fits = "x".repeat(38);
        assert_eq!(text(&fits), format!("\"{fits}\""));
        let cut = text(&"x".repeat(200));
        assert_eq!(cut, format!("\"{}…\"", "x".repeat(37)));
        assert_eq!(cut.chars().count(), 40);
        // The cut never splits an escape: 36 x's and a `\n` is 38 escaped
        // characters, and one more byte makes it 39, so the `\n` goes whole.
        let s = format!("{}\nx", "x".repeat(36));
        assert_eq!(text(&s), format!("\"{}…\"", "x".repeat(36)));
        // One line, whatever the STRING holds.
        assert!(!text(&"a\nb".repeat(50)).contains('\n'));
    }
}
