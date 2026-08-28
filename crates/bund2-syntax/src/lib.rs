//! Surface syntax: source text to a term sequence.
//!
//! **A slice, not RFC-0003.** This lexes the forms the simplest corpus
//! programs use — comments, strings, integers, floats, names — so that
//! `bund2` can run one and `cargo xtask conform` can stop reading 0/69 for
//! want of any parser at all. The grammar it approximates is
//! `reference/bund_language_parser/bund.pest`, and RFC-0003 replaces this
//! with the real thing: an AST with spans, scoped blocks, and the twelve
//! `value` alternatives.
//!
//! What it deliberately does **not** do is guess. A form it does not know is
//! an error naming the form, not a token silently dropped — a lexer that
//! skips what it cannot read produces a program that runs and means something
//! else.

#![forbid(unsafe_code)]

/// One lexed term.
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Int(i64),
    Float(f64),
    Str(String),
    /// A word to call. The `$` sigil is **kept**, because it is stripped when
    /// the name is interned as a call and not by the parser — the grammar
    /// admits `$` inside `element` (`bund.pest:36`), so `$println` is one
    /// name, and D16 means a call target can be a string the parser never saw.
    Name(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub line: usize,
    pub what: String,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.what)
    }
}

/// Lex a program into terms.
pub fn lex(src: &str) -> Result<Vec<Term>, LexError> {
    let mut out = Vec::new();
    for (n, raw) in src.lines().enumerate() {
        let line = n + 1;
        // `//` to end of line, but not inside a string — the trap the corpus
        // lexer records, and the reason this scans rather than splits.
        let mut cs = raw.chars().peekable();
        while let Some(&c) = cs.peek() {
            if c.is_whitespace() {
                cs.next();
                continue;
            }
            if c == '/' {
                let rest: String = cs.clone().collect();
                if rest.starts_with("//") {
                    break;
                }
            }
            if c == '"' {
                cs.next();
                let mut s = String::new();
                let mut closed = false;
                for ch in cs.by_ref() {
                    if ch == '"' {
                        closed = true;
                        break;
                    }
                    s.push(ch);
                }
                if !closed {
                    return Err(LexError {
                        line,
                        what: "unterminated string".into(),
                    });
                }
                out.push(Term::Str(s));
                continue;
            }
            // A bare token: everything to the next whitespace.
            let mut tok = String::new();
            while let Some(&ch) = cs.peek() {
                if ch.is_whitespace() {
                    break;
                }
                tok.push(ch);
                cs.next();
            }
            out.push(classify(&tok));
        }
    }
    Ok(out)
}

/// An integer, a float, or a name.
///
/// The float rule follows the grammar's shape rather than Rust's: a digit
/// sequence with a `.` is a float, and an exponent without a `.` is not — the
/// corpus lexer records that `1e5` is a *name* in this language.
fn classify(tok: &str) -> Term {
    // A leading digit, or a sign followed by one. Anything else is a name,
    // including `-` alone, which is a word.
    let mut cs = tok.chars();
    let numeric_start = match cs.next() {
        Some(c) if c.is_ascii_digit() => true,
        Some('-') => cs.next().is_some_and(|c| c.is_ascii_digit()),
        _ => false,
    };
    if numeric_start {
        if tok.contains('.') {
            if let Ok(f) = tok.parse::<f64>() {
                return Term::Float(f);
            }
        } else if let Ok(i) = tok.parse::<i64>() {
            return Term::Int(i);
        }
    }
    Term::Name(tok.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_and_a_call() {
        assert_eq!(
            lex("\"Hello World!\" println").unwrap(),
            vec![
                Term::Str("Hello World!".into()),
                Term::Name("println".into())
            ]
        );
    }

    #[test]
    fn a_comment_runs_to_end_of_line() {
        assert_eq!(lex("// nothing here\n1").unwrap(), vec![Term::Int(1)]);
    }

    /// The trap the corpus lexer records: `//` inside a string is text.
    #[test]
    fn a_comment_marker_inside_a_string_is_not_a_comment() {
        assert_eq!(lex("\"a // b\"").unwrap(), vec![Term::Str("a // b".into())]);
    }

    /// `bund.pest` needs a `.` before an exponent, so `1e5` is a name — the
    /// corpus lexer records this and a Rust `parse::<f64>` would disagree.
    #[test]
    fn an_exponent_without_a_dot_is_a_name() {
        assert_eq!(lex("1e5").unwrap(), vec![Term::Name("1e5".into())]);
        assert_eq!(lex("1.5").unwrap(), vec![Term::Float(1.5)]);
    }

    /// The sigil survives lexing; it is separated when the name is interned
    /// as a call.
    #[test]
    fn a_dollar_name_lexes_whole() {
        assert_eq!(
            lex("$println").unwrap(),
            vec![Term::Name("$println".into())]
        );
    }

    #[test]
    fn an_unterminated_string_is_an_error_not_a_guess() {
        assert!(lex("\"oops").is_err());
    }
}
