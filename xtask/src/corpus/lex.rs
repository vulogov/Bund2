//! A lexer for `.bund` source.
//!
//! This mirrors `reference/bund_language_parser/bund.pest` closely enough to
//! decide, for every token in the corpus, whether it is a word invocation. It
//! is deliberately not a parser: `corpus` answers "which words does this
//! program name", and that is a lexical question.
//!
//! Fidelity notes, each against the grammar at
//! `reference/bund_language_parser/bund.pest`:
//!
//! * `COMMENT` (line 54) is `"//"` to end of line. pest applies implicit
//!   COMMENT/WHITESPACE only *between* tokens, never inside an atomic (`@`)
//!   rule, so `//` inside a `string` or a `name` is not a comment. We only
//!   test for `//` at a token boundary, which reproduces that.
//! * `name` (line 28), `atom` (line 26), `stack` (line 27), `ptr` (line 29)
//!   and `command` (line 30) all require *trailing whitespace*. A name butted
//!   against `}` is therefore a parse error in the reference, not a name. We
//!   still lex it, and record an [`Anomaly`] so the report can say so.
//! * `float` (line 23) and `integer` (line 22) do **not** require trailing
//!   whitespace, and `float` requires a `.` before any exponent. So `1e5`
//!   lexes as integer `1` followed by name `e5`. We reproduce that rather
//!   than silently "fixing" it.
//! * The ordered choice in `value` (lines 7-20) puts `name` before `command`
//!   and `atom`. `element` (line 36) does not contain `:`, so no name can
//!   start with `:` and the ordering does not affect us.

use std::fmt;

/// What the grammar's `value` alternatives reduce to for our purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `name` — a word invocation. This is the token TASK 2 counts.
    Name,
    /// `` `word `` — names a word as a PTR value without invoking it.
    Ptr,
    /// `:name` — pushes the name as a value. Used for word definition,
    /// class and attribute names.
    Atom,
    /// `@name` — a stack selector.
    StackSel,
    /// A run of `:` / `;` — `command` in the grammar.
    Command,
    Str,
    Literal,
    Int,
    Float,
    OpenLambda,
    CloseLambda,
    OpenList,
    CloseList,
    OpenCtx,
    CloseCtx,
}

impl Kind {
    pub fn closes(self) -> bool {
        matches!(self, Kind::CloseLambda | Kind::CloseList | Kind::CloseCtx)
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Kind,
    /// Payload with any sigil (`` ` ``, `:`, `@`) stripped. For brackets and
    /// commands, the literal text.
    pub text: String,
    /// 1-indexed.
    pub line: usize,
    /// Lambda nesting depth *at* the token: 0 outside any `{ }`.
    pub lambda_depth: usize,
    /// Total bracket nesting depth at the token, any bracket kind.
    pub depth: usize,
}

/// A place where our lexer had to accept something the reference grammar
/// would reject. Reported, never silently swallowed.
#[derive(Debug, Clone)]
pub struct Anomaly {
    pub line: usize,
    pub what: String,
}

impl fmt::Display for Anomaly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.what)
    }
}

#[derive(Debug, Default)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub anomalies: Vec<Anomaly>,
}

/// `element`, bund.pest:36. LETTER and SYMBOL are Unicode categories; for
/// SYMBOL we take the ASCII members not already listed (`~`, `$`). The
/// backtick is also Sk, but `name` (line 28) forbids it in first position and
/// names are whitespace-terminated, so admitting it mid-name would only
/// create false joins. We exclude it and record an anomaly if one appears.
fn is_element(c: char) -> bool {
    c.is_alphabetic()
        || matches!(
            c,
            '.' | ','
                | '='
                | '>'
                | '<'
                | '-'
                | '+'
                | '^'
                | '?'
                | '!'
                | '/'
                | '*'
                | '|'
                | '&'
                | '#'
                | '%'
                | '_'
                | '~'
                | '$'
        )
}

/// `nelement`, bund.pest:37.
fn is_nelement(c: char) -> bool {
    is_element(c) || c.is_ascii_digit()
}

/// `aelement`, bund.pest:38. Note there is no `-` here: `:a-b` does not lex
/// as an atom in the reference.
fn is_aelement(c: char) -> bool {
    c.is_alphanumeric() || c == '.' || c == '_'
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// Characters that end a token without being whitespace. The reference
/// requires whitespace, so hitting one of these is an anomaly for the
/// whitespace-terminated kinds.
fn is_hard_break(c: char) -> bool {
    matches!(c, '{' | '}' | '[' | ']' | '(' | ')' | '"' | '\'')
}

pub fn lex(src: &str) -> Lexed {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Lexed::default();
    let mut i = 0usize;
    let mut line = 1usize;
    // Bracket stack, so `lambda_depth` counts only `{ }`.
    let mut brackets: Vec<Kind> = Vec::new();

    macro_rules! push {
        ($kind:expr, $text:expr, $at:expr) => {{
            let lambda_depth = brackets.iter().filter(|k| **k == Kind::OpenLambda).count();
            out.tokens.push(Token {
                kind: $kind,
                text: $text,
                line: $at,
                lambda_depth,
                depth: brackets.len(),
            });
        }};
    }

    while i < chars.len() {
        let c = chars[i];

        if is_ws(c) {
            if c == '\n' {
                line += 1;
            }
            i += 1;
            continue;
        }

        // COMMENT, bund.pest:54. Only at a token boundary — see module docs.
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        let at = line;

        // string, bund.pest:24.
        if c == '"' {
            let start = i;
            i += 1;
            let mut closed = false;
            while i < chars.len() {
                match chars[i] {
                    '\\' => {
                        if chars.get(i + 1) == Some(&'\n') {
                            line += 1;
                        }
                        i += 2;
                    }
                    '"' => {
                        i += 1;
                        closed = true;
                        break;
                    }
                    '\n' => {
                        line += 1;
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            if !closed {
                out.anomalies.push(Anomaly {
                    line: at,
                    what: "unterminated string".into(),
                });
            }
            let text: String = chars[start..i.min(chars.len())].iter().collect();
            push!(Kind::Str, text, at);
            continue;
        }

        // literal, bund.pest:25.
        if c == '\'' {
            let start = i;
            i += 1;
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == '\n' {
                    line += 1;
                }
                if chars[i] == '\'' {
                    i += 1;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                out.anomalies.push(Anomaly {
                    line: at,
                    what: "unterminated literal".into(),
                });
            }
            let text: String = chars[start..i.min(chars.len())].iter().collect();
            push!(Kind::Literal, text, at);
            continue;
        }

        // lambda / list / ctx, bund.pest:31-33.
        if let Some(kind) = match c {
            '{' => Some(Kind::OpenLambda),
            '}' => Some(Kind::CloseLambda),
            '[' => Some(Kind::OpenList),
            ']' => Some(Kind::CloseList),
            '(' => Some(Kind::OpenCtx),
            ')' => Some(Kind::CloseCtx),
            _ => None,
        } {
            if kind.closes() {
                // Pop before pushing the token, so the closer reads at the
                // same depth as its opener.
                if brackets.pop().is_none() {
                    out.anomalies.push(Anomaly {
                        line: at,
                        what: format!("unmatched `{c}`"),
                    });
                }
                push!(kind, c.to_string(), at);
            } else {
                push!(kind, c.to_string(), at);
                brackets.push(kind);
            }
            i += 1;
            continue;
        }

        // stack, bund.pest:27: "@" ~ LETTER+.
        if c == '@' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i].is_alphabetic() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            if text.is_empty() {
                out.anomalies.push(Anomaly {
                    line: at,
                    what: "`@` with no stack name".into(),
                });
            }
            push!(Kind::StackSel, text, at);
            continue;
        }

        // ptr, bund.pest:29: "`" ~ element ~ nelement*.
        if c == '`' {
            i += 1;
            let start = i;
            while i < chars.len() && is_nelement(chars[i]) {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            if text.is_empty() {
                out.anomalies.push(Anomaly {
                    line: at,
                    what: "`` ` `` with no name".into(),
                });
            }
            push!(Kind::Ptr, text, at);
            continue;
        }

        // atom (bund.pest:26) vs command (bund.pest:30/44).
        if c == ':' || c == ';' {
            if c == ':' && chars.get(i + 1).copied().is_some_and(is_aelement) {
                i += 1;
                let start = i;
                while i < chars.len() && is_aelement(chars[i]) {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                if i < chars.len() && !is_ws(chars[i]) {
                    let c2 = chars[i];
                    if is_hard_break(c2) {
                        out.anomalies.push(Anomaly {
                            line: at,
                            what: format!(
                                "atom `:{text}` is terminated by `{c2}`, not whitespace; \
                                           bund.pest:26 requires WHITESPACE+"
                            ),
                        });
                    } else {
                        out.anomalies.push(Anomaly {
                            line: at,
                            what: format!(
                                "atom `:{text}` contains `{c2}`, which is not an \
                                           aelement (bund.pest:38)"
                            ),
                        });
                    }
                }
                push!(Kind::Atom, text, at);
            } else {
                let start = i;
                while i < chars.len() && (chars[i] == ':' || chars[i] == ';') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                push!(Kind::Command, text, at);
            }
            continue;
        }

        // float / integer, bund.pest:22-23.
        let numeric_start = c.is_ascii_digit()
            || ((c == '+' || c == '-')
                && chars
                    .get(i + 1)
                    .copied()
                    .is_some_and(|n| n.is_ascii_digit()));
        if numeric_start {
            let start = i;
            if c == '+' || c == '-' {
                i += 1;
            }
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') {
                i += 1;
            }
            let mut is_float = false;
            if chars.get(i) == Some(&'.') {
                is_float = true;
                i += 1;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') {
                    i += 1;
                }
                // exp, bund.pest:50 — only reachable after the `.`.
                if matches!(chars.get(i), Some('e') | Some('E')) {
                    let save = i;
                    i += 1;
                    if matches!(chars.get(i), Some('+') | Some('-')) {
                        i += 1;
                    }
                    if chars.get(i).copied().is_some_and(|n| n.is_ascii_digit()) {
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            i += 1;
                        }
                    } else {
                        i = save;
                    }
                }
            }
            let text: String = chars[start..i].iter().collect();
            push!(if is_float { Kind::Float } else { Kind::Int }, text, at);
            continue;
        }

        // name, bund.pest:28.
        if is_element(c) {
            let start = i;
            while i < chars.len() && is_nelement(chars[i]) {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            if i < chars.len() && !is_ws(chars[i]) {
                let c2 = chars[i];
                if is_hard_break(c2) {
                    out.anomalies.push(Anomaly {
                        line: at,
                        what: format!(
                            "name `{text}` is terminated by `{c2}`, not whitespace; \
                                       bund.pest:28 requires WHITESPACE+"
                        ),
                    });
                }
            }
            push!(Kind::Name, text, at);
            continue;
        }

        out.anomalies.push(Anomaly {
            line: at,
            what: format!(
                "character `{c}` (U+{:04X}) matches no grammar rule",
                c as u32
            ),
        });
        i += 1;
    }

    if !brackets.is_empty() {
        out.anomalies.push(Anomaly {
            line,
            what: format!("{} unclosed bracket(s) at end of file", brackets.len()),
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(src: &str) -> Vec<String> {
        lex(src)
            .tokens
            .into_iter()
            .filter(|t| t.kind == Kind::Name)
            .map(|t| t.text)
            .collect()
    }

    fn kinds(src: &str) -> Vec<Kind> {
        lex(src).tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn comments_are_stripped() {
        // reference/Bund/examples/helloworld.bund
        let src = "//\n// This is faimous \"Hello World!\" program\n//\n\"Hello World!\" println\n";
        assert_eq!(words(src), ["println"]);
    }

    #[test]
    fn braces_inside_a_string_are_not_brackets() {
        // reference/Bund/tests/testing_ifthenelse.bund:1 — the format
        // placeholders in the string must not open a lambda.
        let src = "\"Testing {} {} ?true* word\"\n42 42 != {\n true\n} {\n false\n} ?true*\n";
        assert_eq!(words(src), ["!=", "true", "false", "?true*"]);
        assert_eq!(
            kinds(src)
                .iter()
                .filter(|k| **k == Kind::OpenLambda)
                .count(),
            2
        );
    }

    #[test]
    fn atom_versus_command() {
        // bund.pest:26 vs :30. `:name` is an atom; a bare `:` or `;` is a
        // command.
        let t = lex(":A class ; : x\n").tokens;
        assert_eq!(t[0].kind, Kind::Atom);
        assert_eq!(t[0].text, "A");
        assert_eq!(t[1].kind, Kind::Name);
        assert_eq!(t[2].kind, Kind::Command);
        assert_eq!(t[3].kind, Kind::Command);
    }

    #[test]
    fn dotted_atoms_lex_whole() {
        // reference/Bund/examples/object_oriented_programming/create_class_demo.bund
        let t = lex(":.super [ :Object ] set\n").tokens;
        assert_eq!(t[0].kind, Kind::Atom);
        assert_eq!(t[0].text, ".super");
    }

    #[test]
    fn numbers_versus_operator_names() {
        // `+.` and `-` are names; `-1` and `3.14` are numbers. bund.pest:22-23.
        let t = lex("3.14 -1 +. - +\n").tokens;
        assert_eq!(t[0].kind, Kind::Float);
        assert_eq!(t[1].kind, Kind::Int);
        assert_eq!(t[1].text, "-1");
        assert_eq!(t[2].kind, Kind::Name);
        assert_eq!(t[2].text, "+.");
        assert_eq!(t[3].kind, Kind::Name);
        assert_eq!(t[4].kind, Kind::Name);
    }

    #[test]
    fn exponent_needs_a_dot_first() {
        // float (bund.pest:23) requires `.` before exp, so `1e5` is an
        // integer followed by a name. Faithfully odd, not corrected.
        let t = lex("1e5 1.5e3\n").tokens;
        assert_eq!((t[0].kind, t[0].text.as_str()), (Kind::Int, "1"));
        assert_eq!((t[1].kind, t[1].text.as_str()), (Kind::Name, "e5"));
        assert_eq!((t[2].kind, t[2].text.as_str()), (Kind::Float, "1.5e3"));
    }

    #[test]
    fn ptr_and_stack_selector() {
        let t = lex("`myword @main foo\n").tokens;
        assert_eq!((t[0].kind, t[0].text.as_str()), (Kind::Ptr, "myword"));
        assert_eq!((t[1].kind, t[1].text.as_str()), (Kind::StackSel, "main"));
        assert_eq!(t[2].kind, Kind::Name);
    }

    #[test]
    fn double_slash_inside_a_string_is_not_a_comment() {
        let src = "\"http://x\" println\n";
        assert_eq!(words(src), ["println"]);
    }

    #[test]
    fn lambda_depth_is_tracked() {
        let t = lex("a { b { c } d } e\n").tokens;
        let depth = |name: &str| {
            t.iter()
                .find(|x| x.kind == Kind::Name && x.text == name)
                .unwrap()
                .lambda_depth
        };
        assert_eq!(depth("a"), 0);
        assert_eq!(depth("b"), 1);
        assert_eq!(depth("c"), 2);
        assert_eq!(depth("d"), 1);
        assert_eq!(depth("e"), 0);
    }

    #[test]
    fn a_name_butted_against_a_brace_is_an_anomaly() {
        // bund.pest:28 requires WHITESPACE+ after a name.
        let l = lex("{ println}\n");
        assert_eq!(l.anomalies.len(), 1);
        assert!(l.anomalies[0].what.contains("terminated by `}`"));
    }

    #[test]
    fn unterminated_string_is_reported() {
        let l = lex("\"oops\n");
        assert!(
            l.anomalies
                .iter()
                .any(|a| a.what.contains("unterminated string"))
        );
    }
}
