//! Surface syntax: source text to an AST with spans, and an AST to a value
//! stream. **RFC-0003 §S1 and §S2.**
//!
//! This replaces the lexer slice that preceded it. The grammar it implements
//! is `reference/bund_language_parser/bund.pest`, with the deviations RFC-0003
//! states and no others.
//!
//! # What the grammar gets wrong, and is preserved anyway
//!
//! Four forms read one way and lex another. Each is a recorded defect, each is
//! reproduced here, and each has a test — because a parser that "fixes" them
//! silently accepts or rejects programs the reference does not.
//!
//! - **F49** — `1_000` is a *parse* error. `digits` admits `_` (`bund.pest:49`)
//!   and the reference's handler then fails to convert it
//!   (`reference/bund_language_parser/src/vm/integer.rs:8-14`).
//! - **F50** — `007` is **three** integers. `int` is `"0" | (NONZERO ~ digits?)`
//!   (`bund.pest:48`), so a leading-zero run decomposes rather than failing.
//! - **F51** — `{}` and `[]` do not parse. Both require `term+` (`:31-32`).
//! - **F61** — `1 2 +// add` lexes `+//` as one **name**: `element` admits `/`
//!   (`:36`) and pest applies `COMMENT` between tokens (`:54`), so a comment
//!   marker abutting a word is swallowed by it.
//!
//! # The two deviations
//!
//! - **S1** — the five atomic rules terminate on whitespace **or end of
//!   input**, where the grammar demands `WHITESPACE+` (`:26-30`). Four of the
//!   reference's five parse sites append `\n` to compensate; one does not
//!   (`reference/bund_language_parser/src/compile.rs:15`). The accepted
//!   language is unchanged: `{ 1 println}` still fails, because `}` is not
//!   whitespace.
//! - **F63** — an atom is interchangeable with a string, and its content is any
//!   run of characters without whitespace. `aelement` (`:38`) admits far less,
//!   so `:my-word` fails while `foo-bar` is a legal word name. Widened here.

#![forbid(unsafe_code)]

use bund2_value::BundValue;

/// A byte range in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// The 1-based line this span starts on, for error text.
    pub fn line_in(&self, src: &str) -> usize {
        src[..self.start.min(src.len())].bytes().filter(|b| *b == b'\n').count() + 1
    }
}

/// One parsed term.
///
/// The twelve `value` alternatives of `bund.pest:7-20`, less `literal` and
/// `string`, which collapse into [`Term::Str`] because both produce a STRING —
/// and less `atom`, which collapses into it for the same reason (F63).
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Int(i64, Span),
    Float(f64, Span),
    /// A double-quoted string, a `'…'` literal, or a `:atom`. All three are a
    /// STRING downstream and nothing distinguishes them
    /// (`reference/bund_language_parser/src/vm/atom.rs:7-10`).
    Str(String, Span),
    /// A word to call. The `$` sigil is **kept**: it is honoured at dispatch,
    /// not stripped at parse (D16).
    Name(String, Span),
    /// `:` or `;` — the autoadd commands (`bund.pest:44`).
    Command(String, Span),
    /// `` `name `` — a PTR.
    Ptr(String, Span),
    /// `@name` — a stack switch.
    Stack(String, Span),
    /// `{ … }`, always non-empty (F51).
    Lambda(Vec<Term>, Span),
    /// `[ … ]`, always non-empty (F51).
    List(Vec<Term>, Span),
    /// `( … )`. **A scope**, lowered in place (D34) — the reference hoists its
    /// contents into the enclosing stream instead (F58).
    Ctx(Vec<Term>, Span),
}

impl Term {
    pub fn span(&self) -> Span {
        match self {
            Term::Int(_, s)
            | Term::Float(_, s)
            | Term::Str(_, s)
            | Term::Name(_, s)
            | Term::Command(_, s)
            | Term::Ptr(_, s)
            | Term::Stack(_, s)
            | Term::Lambda(_, s)
            | Term::List(_, s)
            | Term::Ctx(_, s) => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub span: Span,
    pub what: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.what)
    }
}

impl ParseError {
    pub fn render(&self, src: &str) -> String {
        format!("line {}: {}", self.span.line_in(src), self.what)
    }
}

/// `element` — `bund.pest:36`. `LETTER | SYMBOL` plus seventeen punctuation
/// characters. `$` is `Sc` and therefore a `SYMBOL`, which is why `$println`
/// is one name.
fn is_element(c: char) -> bool {
    c.is_alphabetic()
        || is_symbol(c)
        || matches!(
            c,
            '.' | ',' | '=' | '>' | '<' | '-' | '+' | '^' | '?' | '!' | '/' | '*' | '|' | '&'
                | '#' | '%' | '_'
        )
}

/// Unicode `S*` — `Sm`, `Sc`, `Sk`, `So`. Approximated by exclusion, since the
/// alternative is a full character-category table for four operators.
fn is_symbol(c: char) -> bool {
    if c.is_alphanumeric() || c.is_whitespace() || c.is_control() {
        return false;
    }
    matches!(c,
        '$' | '+' | '<' | '=' | '>' | '^' | '`' | '|' | '~'
        | '\u{00A2}'..='\u{00A6}' | '\u{00A8}' | '\u{00A9}' | '\u{00AC}' | '\u{00AE}'..='\u{00B1}'
        | '\u{00B4}' | '\u{00B8}' | '\u{00D7}' | '\u{00F7}'
        | '\u{02C2}'..='\u{02C5}' | '\u{03D0}'..='\u{03F6}'
        | '\u{2000}'..='\u{2BFF}' | '\u{1D400}'..='\u{1D7FF}')
}

/// `nelement` — `element` or an ASCII digit (`bund.pest:37`).
fn is_nelement(c: char) -> bool {
    is_element(c) || c.is_ascii_digit()
}

struct Parser<'a> {
    src: &'a str,
    /// Byte offsets of every char, plus the source length, so a span is a byte
    /// range even though scanning is by `char`.
    chars: Vec<(usize, char)>,
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            chars: src.char_indices().collect(),
            i: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.i).map(|(_, c)| *c)
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.i + n).map(|(_, c)| *c)
    }

    fn offset(&self) -> usize {
        self.chars
            .get(self.i)
            .map(|(o, _)| *o)
            .unwrap_or(self.src.len())
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.i += 1;
        }
        c
    }

    fn at_end(&self) -> bool {
        self.i >= self.chars.len()
    }

    /// Whitespace and `//` comments — pest's implicit rules (`bund.pest:52,54`),
    /// applied *between* tokens and therefore never inside one. That is exactly
    /// why F61's `+//` is a single name.
    fn skip_trivia(&mut self) {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.i += 1;
            }
            if self.peek() == Some('/') && self.peek_at(1) == Some('/') {
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.i += 1;
                }
                continue;
            }
            return;
        }
    }

    /// **S1's terminator rule.** The five atomic rules require `WHITESPACE+`
    /// after them (`bund.pest:26-30`); Bund2 accepts whitespace **or end of
    /// input**, which is what the reference's `\n` append achieves at four of
    /// its five parse sites.
    ///
    /// A closing bracket is *not* a terminator, so `{ 1 println}` fails here as
    /// it does in the reference.
    fn at_terminator(&self) -> bool {
        match self.peek() {
            None => true,
            Some(c) => c.is_whitespace(),
        }
    }

    fn err<T>(&self, start: usize, what: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            span: Span {
                start,
                end: self.offset(),
            },
            what: what.into(),
        })
    }

    fn terms_until(&mut self, close: char) -> Result<Vec<Term>, ParseError> {
        let open = self.offset();
        let mut out = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                None => return self.err(open, format!("unclosed block, expected `{close}`")),
                Some(c) if c == close => {
                    self.bump();
                    // `term+`: all three bracket forms require at least one
                    // term, so the empty form is a parse error (F51).
                    if out.is_empty() {
                        return self.err(open, format!("empty block: `{close}` needs a term before it"));
                    }
                    return Ok(out);
                }
                _ => out.push(self.term()?),
            }
        }
    }

    fn term(&mut self) -> Result<Term, ParseError> {
        let start = self.offset();
        let c = self.peek().expect("term called at end of input");

        // Bracket forms first: they are not atomic, so trivia nests inside.
        if c == '{' {
            self.bump();
            let inner = self.terms_until('}')?;
            return Ok(Term::Lambda(inner, self.span_from(start)));
        }
        if c == '[' {
            self.bump();
            let inner = self.terms_until(']')?;
            return Ok(Term::List(inner, self.span_from(start)));
        }
        if c == '(' {
            self.bump();
            let inner = self.terms_until(')')?;
            return Ok(Term::Ctx(inner, self.span_from(start)));
        }
        if c == '"' {
            return self.string(start);
        }
        if c == '\'' {
            return self.literal(start);
        }
        if c == '`' {
            return self.sigil_word(start, '`');
        }
        if c == '@' {
            return self.sigil_word(start, '@');
        }
        if c == ':' || c == ';' {
            return self.colon_or_atom(start);
        }
        if self.number_starts_here() {
            return self.number(start);
        }
        self.name(start)
    }

    fn span_from(&self, start: usize) -> Span {
        Span {
            start,
            end: self.offset(),
        }
    }

    fn string(&mut self, start: usize) -> Result<Term, ParseError> {
        self.bump();
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return self.err(start, "unterminated string"),
                Some('"') => return Ok(Term::Str(out, self.span_from(start))),
                Some('\\') => match self.bump() {
                    None => return self.err(start, "unterminated escape"),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('b') => out.push('\u{8}'),
                    Some('f') => out.push('\u{c}'),
                    Some(other) => out.push(other),
                },
                Some(ch) => out.push(ch),
            }
        }
    }

    /// `'…'` — `literal`, `bund.pest:25`. `literal_element` lists escape
    /// alternatives after `ANY`, which matches everything, so they are
    /// unreachable and the body is taken verbatim.
    fn literal(&mut self, start: usize) -> Result<Term, ParseError> {
        self.bump();
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return self.err(start, "unterminated literal"),
                Some('\'') => return Ok(Term::Str(out, self.span_from(start))),
                Some(ch) => out.push(ch),
            }
        }
    }

    /// `` `name `` and `@name`. Both are atomic and both need a terminator.
    fn sigil_word(&mut self, start: usize, sigil: char) -> Result<Term, ParseError> {
        self.bump();
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                break;
            }
            out.push(ch);
            self.i += 1;
        }
        if out.is_empty() {
            return self.err(start, format!("`{sigil}` needs a name after it"));
        }
        if !self.at_terminator() {
            return self.err(start, "expected whitespace or end of input");
        }
        let span = self.span_from(start);
        Ok(if sigil == '`' {
            Term::Ptr(out, span)
        } else {
            Term::Stack(out, span)
        })
    }

    /// `:` and `;`.
    ///
    /// `cmd+ ~ WHITESPACE+` (`bund.pest:30,44`) against
    /// `":" ~ aelement+ ~ WHITESPACE+` (`:26`) — so a `:` followed by a
    /// terminator is a **command** and a `:` followed by content is an
    /// **atom**. The disambiguation is the terminator, not a lookahead.
    ///
    /// `cmd+` means `::`, `;;` and `:;` are single command tokens that resolve
    /// to nothing.
    fn colon_or_atom(&mut self, start: usize) -> Result<Term, ParseError> {
        let mut cmd = String::new();
        while matches!(self.peek(), Some(':') | Some(';')) {
            cmd.push(self.bump().expect("peeked"));
        }
        if self.at_terminator() {
            return Ok(Term::Command(cmd, self.span_from(start)));
        }
        // Not a command, so it is an atom — and only a single leading `:` can
        // begin one.
        if cmd.len() != 1 || !cmd.starts_with(':') {
            return self.err(start, "expected whitespace or end of input after a command");
        }
        // F63: an atom is interchangeable with a string, so its content is any
        // run of non-whitespace characters. `aelement` (`bund.pest:38`) admits
        // far less, which is the defect.
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                break;
            }
            out.push(ch);
            self.i += 1;
        }
        Ok(Term::Str(out, self.span_from(start)))
    }

    /// Does a number begin here? `integer` and `float` allow a leading sign
    /// (`bund.pest:22-23`), so `-` alone is a name and `-1` is a number.
    fn number_starts_here(&self) -> bool {
        match self.peek() {
            Some(c) if c.is_ascii_digit() => true,
            Some('+') | Some('-') => self.peek_at(1).is_some_and(|c| c.is_ascii_digit()),
            _ => false,
        }
    }

    /// An integer or a float.
    ///
    /// Two traps live here and both are deliberate.
    ///
    /// **F50** — `int` is `"0" | (NONZERO ~ digits?)` (`bund.pest:48`), so a
    /// leading zero matches *just* the zero and the rest becomes the next term.
    /// `007` is three integers, silently.
    ///
    /// **F49** — `digits` admits `_` (`:49`) but the reference converts with
    /// `lexical_core`, which rejects it, so `1_000` is a hard error rather than
    /// a name.
    fn number(&mut self, start: usize) -> Result<Term, ParseError> {
        let mut tok = String::new();
        if matches!(self.peek(), Some('+') | Some('-')) {
            tok.push(self.bump().expect("peeked"));
        }
        // int
        if self.peek() == Some('0') {
            tok.push(self.bump().expect("peeked"));
        } else {
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                tok.push(self.bump().expect("peeked"));
            }
        }
        let mut is_float = false;
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            tok.push(self.bump().expect("peeked"));
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                tok.push(self.bump().expect("peeked"));
            }
            // `exp` is only reachable after a `.` (`bund.pest:23`), which is
            // why `1e5` is a name and `1.5e5` is a float.
            if matches!(self.peek(), Some('e') | Some('E'))
                && (self.peek_at(1).is_some_and(|c| c.is_ascii_digit())
                    || (matches!(self.peek_at(1), Some('+') | Some('-'))
                        && self.peek_at(2).is_some_and(|c| c.is_ascii_digit())))
            {
                tok.push(self.bump().expect("peeked"));
                if matches!(self.peek(), Some('+') | Some('-')) {
                    tok.push(self.bump().expect("peeked"));
                }
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    tok.push(self.bump().expect("peeked"));
                }
            }
        }
        // F49: the grammar matched `_`; conversion refuses it.
        if tok.contains('_') {
            let kind = if is_float { "FLOAT" } else { "INT" };
            return self.err(
                start,
                format!(
                    "Error converting {kind} to VALUE: lexical parse error: \
                     'invalid digit found' at index {}",
                    tok.find('_').expect("contains checked")
                ),
            );
        }
        let span = self.span_from(start);
        if is_float {
            match tok.parse::<f64>() {
                Ok(f) => Ok(Term::Float(f, span)),
                Err(e) => self.err(start, format!("Error converting FLOAT to VALUE: {e}")),
            }
        } else {
            match tok.parse::<i64>() {
                Ok(n) => Ok(Term::Int(n, span)),
                Err(e) => self.err(start, format!("Error converting INT to VALUE: {e}")),
            }
        }
    }

    /// A word. `element ~ nelement*` with a `` !("`") `` guard (`bund.pest:28`),
    /// terminated per S1.
    ///
    /// **F61 lives here.** `/` is an `element`, and comments are only skipped
    /// *between* tokens, so `+//` is one name and not `+` plus a comment.
    fn name(&mut self, start: usize) -> Result<Term, ParseError> {
        let first = self.peek().expect("name called at end of input");
        if !is_element(first) {
            self.bump();
            return self.err(start, format!("unexpected character `{first}`"));
        }
        let mut out = String::new();
        out.push(self.bump().expect("peeked"));
        while let Some(ch) = self.peek() {
            if !is_nelement(ch) {
                break;
            }
            out.push(ch);
            self.i += 1;
        }
        if !self.at_terminator() {
            let bad = self.peek().expect("not at terminator");
            return self.err(
                start,
                format!("expected whitespace or end of input, found `{bad}`"),
            );
        }
        Ok(Term::Name(out, self.span_from(start)))
    }
}

/// Parse a program into terms.
pub fn parse(src: &str) -> Result<Vec<Term>, ParseError> {
    let mut p = Parser::new(src);
    let mut out = Vec::new();
    loop {
        p.skip_trivia();
        if p.at_end() {
            return Ok(out);
        }
        out.push(p.term()?);
    }
}

/// Lower an AST to the value stream the evaluator consumes.
///
/// The stream ends with EXIT, as `EOI` does
/// (`reference/bund_language_parser/src/vm/eoi.rs:7-9`).
pub fn lower(terms: &[Term]) -> Vec<BundValue> {
    let mut out = Vec::new();
    for t in terms {
        lower_into(t, &mut out);
    }
    out.push(BundValue::exit());
    out
}

/// Lower one term.
///
/// **`( … )` lowers in place** — D34. Its CONTEXT marker, its inner terms and
/// its `endcontext` call are emitted together, into whichever stream contains
/// it, so the bracket is balanced wherever it appears. The reference instead
/// writes the marker and inner terms to the *top-level* stream while leaving
/// `endcontext` in the block (F58), which splits an open from its close in
/// time.
fn lower_into(t: &Term, out: &mut Vec<BundValue>) {
    match t {
        Term::Int(n, _) => out.push(BundValue::Int(*n)),
        Term::Float(f, _) => out.push(BundValue::Float(*f)),
        Term::Str(s, _) => out.push(BundValue::str(s.clone())),
        Term::Name(n, _) => out.push(BundValue::call(n.clone())),
        Term::Command(c, _) => out.push(BundValue::call(c.clone())),
        Term::Ptr(p, _) => out.push(BundValue::ptr(p.clone())),
        Term::Stack(s, _) => out.push(BundValue::named_context(s.clone())),
        Term::Lambda(inner, _) => {
            let mut body = Vec::new();
            for i in inner {
                lower_into(i, &mut body);
            }
            out.push(BundValue::lambda(body));
        }
        Term::List(inner, _) => {
            let mut items = Vec::new();
            for i in inner {
                lower_into(i, &mut items);
            }
            out.push(BundValue::list(items));
        }
        Term::Ctx(inner, _) => {
            out.push(BundValue::context());
            for i in inner {
                lower_into(i, out);
            }
            out.push(BundValue::call("endcontext"));
        }
    }
}

/// Parse and lower in one step.
pub fn compile(src: &str) -> Result<Vec<BundValue>, ParseError> {
    Ok(lower(&parse(src)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(src: &str) -> Vec<Term> {
        parse(src).expect("parses")
    }

    #[test]
    fn a_string_and_a_call() {
        let t = names("\"Hello World!\" println");
        assert!(matches!(&t[0], Term::Str(s, _) if s == "Hello World!"));
        assert!(matches!(&t[1], Term::Name(n, _) if n == "println"));
    }

    /// The trap the corpus lexer recorded: `//` inside a string is text.
    #[test]
    fn a_comment_marker_inside_a_string_is_not_a_comment() {
        assert!(matches!(&names("\"a // b\"")[0], Term::Str(s, _) if s == "a // b"));
    }

    /// **F61.** `/` is an `element` and comments are skipped only between
    /// tokens, so a comment abutting a word is swallowed by it — *and the rest
    /// of the line stops being a comment*. Confirmed against the oracle:
    /// `"1 2 +// add" compile` yields `CALL("+//")` then `CALL("add")`.
    #[test]
    fn a_comment_marker_abutting_a_word_is_part_of_the_word() {
        let t = names("1 2 +// add");
        assert_eq!(t.len(), 4, "the comment was eaten, so `add` is code: {t:?}");
        assert!(matches!(&t[2], Term::Name(n, _) if n == "+//"), "{:?}", t[2]);
        assert!(matches!(&t[3], Term::Name(n, _) if n == "add"), "{:?}", t[3]);
        // With whitespace it is an ordinary comment and `add` disappears.
        let u = names("1 2 + // add");
        assert_eq!(u.len(), 3);
        assert!(matches!(&u[2], Term::Name(n, _) if n == "+"));
    }

    /// **F50.** A leading-zero run decomposes; it does not error.
    #[test]
    fn a_leading_zero_integer_becomes_several() {
        let t = names("007");
        assert_eq!(t.len(), 3, "{t:?}");
        assert!(matches!(t[0], Term::Int(0, _)));
        assert!(matches!(t[1], Term::Int(0, _)));
        assert!(matches!(t[2], Term::Int(7, _)));
    }

    /// **F49.** The grammar admits `_`; conversion refuses it.
    #[test]
    fn a_digit_separator_is_a_hard_error() {
        let e = parse("1_000").expect_err("must fail");
        assert!(e.what.contains("Error converting INT to VALUE"), "{}", e.what);
        assert!(parse("1_000.5").is_err());
    }

    /// **F51.** All three bracket forms need `term+`.
    #[test]
    fn empty_bracket_forms_do_not_parse() {
        assert!(parse("{} 1").is_err());
        assert!(parse("[] 1").is_err());
        assert!(parse("() 1").is_err());
        assert!(parse("{ 1 }").is_ok());
    }

    /// **S1's terminator rule.** End of input terminates a name, so no `\n`
    /// append is needed — but a closing brace does not, so `{ 1 println}` fails
    /// exactly as it does in the reference.
    #[test]
    fn a_name_terminates_on_whitespace_or_end_of_input_only() {
        assert!(parse("1 2 +").is_ok(), "end of input terminates");
        let e = parse("{ 1 println}").expect_err("a brace is not a terminator");
        assert!(e.what.contains("expected whitespace"), "{}", e.what);
    }

    /// `exp` is reachable only after a `.` (`bund.pest:23`), so `1e5` is **not**
    /// a float — and not one name either. `integer` is atomic with no
    /// terminator requirement, so it takes the `1` and `e5` becomes a separate
    /// name. Confirmed against the oracle: `"1e5" compile` yields `I64(1)` then
    /// `CALL("e5")`.
    ///
    /// The stopgap lexer this parser replaced asserted `1e5` was a single name.
    /// It was wrong; `docs/registers/open-questions.md` had it right.
    #[test]
    fn an_exponent_without_a_dot_splits() {
        let t = names("1e5");
        assert_eq!(t.len(), 2, "{t:?}");
        assert!(matches!(t[0], Term::Int(1, _)));
        assert!(matches!(&t[1], Term::Name(n, _) if n == "e5"));
        assert!(matches!(names("1.5")[0], Term::Float(f, _) if f == 1.5));
        assert!(matches!(names("1.5e2")[0], Term::Float(f, _) if f == 150.0));
    }

    /// `:` before a terminator is a command; `:` before content is an atom.
    #[test]
    fn a_colon_is_a_command_or_an_atom_by_what_follows() {
        assert!(matches!(&names("lambda : 1 ;")[1], Term::Command(c, _) if c == ":"));
        assert!(matches!(&names(":Greet register")[0], Term::Str(s, _) if s == "Greet"));
        // `cmd+`: `::` is one token, and it resolves to nothing.
        assert!(matches!(&names(":: 1")[0], Term::Command(c, _) if c == "::"));
    }

    /// **F63.** An atom is interchangeable with a string, so its content is any
    /// run of non-whitespace characters — where `aelement` admitted neither `-`
    /// nor most punctuation.
    #[test]
    fn an_atom_admits_what_a_name_admits() {
        assert!(matches!(&names(":my-word x")[0], Term::Str(s, _) if s == "my-word"));
        assert!(matches!(&names(":a.b_c x")[0], Term::Str(s, _) if s == "a.b_c"));
        // And it is the same value a quoted string produces.
        assert_eq!(compile(":k").unwrap()[0], compile("\"k\"").unwrap()[0]);
    }

    /// The sigil survives parsing; dispatch honours it (D16).
    #[test]
    fn a_dollar_name_parses_whole() {
        assert!(matches!(&names("$println")[0], Term::Name(n, _) if n == "$println"));
    }

    #[test]
    fn ptr_and_stack_sigils() {
        assert!(matches!(&names("`dup !")[0], Term::Ptr(p, _) if p == "dup"));
        assert!(matches!(&names("@main 1")[0], Term::Stack(s, _) if s == "main"));
    }

    #[test]
    fn an_unterminated_string_is_an_error_not_a_guess() {
        assert!(parse("\"oops").is_err());
        assert!(parse("'oops").is_err());
        assert!(parse("{ 1 ").is_err());
    }

    /// Spans point at the term, so an error can name a line.
    #[test]
    fn terms_carry_spans() {
        let t = names("1\n  println");
        assert_eq!(t[1].span().line_in("1\n  println"), 2);
    }

    /// **D34.** A nested `( … )` lowers *in place*: its marker and terms stay
    /// inside the block. The reference hoists them to the top level (F58),
    /// which is what made `:F { ( 7 ) 9 } register` fail.
    #[test]
    fn a_nested_context_lowers_in_place() {
        let stream = compile(":F { ( 7 ) 9 } register").expect("parses");
        // Top level: "F", the lambda, `register`, EXIT. Nothing hoisted.
        assert_eq!(stream.len(), 4, "{stream:?}");
        assert_eq!(stream[0].as_str().as_deref(), Some("F"));
        assert_eq!(stream[1].dt(), bund2_value::LAMBDA);
        assert_eq!(stream[2].as_str().as_deref(), Some("register"));
        assert_eq!(stream[3].dt(), bund2_value::EXIT);
    }

    /// A top-level `( … )` is unchanged from the reference: marker, terms,
    /// `endcontext`, flat in the stream.
    #[test]
    fn a_top_level_context_is_flat() {
        let s = compile("( 7 )").expect("parses");
        assert_eq!(s.len(), 4, "{s:?}");
        assert_eq!(s[0].dt(), bund2_value::CONTEXT);
        assert_eq!(s[1].as_int(), Some(7));
        assert_eq!(s[2].as_str().as_deref(), Some("endcontext"));
        assert_eq!(s[3].dt(), bund2_value::EXIT);
    }

    /// Every context gets its own name, so two cannot collide.
    #[test]
    fn contexts_are_uniquely_named() {
        let s = compile("( 1 ) ( 2 )").expect("parses");
        assert_ne!(s[0].as_str(), s[3].as_str());
    }

    /// Lowering is total over the AST and always ends with EXIT.
    #[test]
    fn the_stream_ends_with_exit() {
        for src in ["1", "\"s\"", "`p", "@s", "{ 1 }", "[ 1 ]", "( 1 )", ":a", ":"] {
            let s = compile(src).unwrap_or_else(|e| panic!("{src}: {e}"));
            assert_eq!(s.last().map(BundValue::dt), Some(bund2_value::EXIT), "{src}");
        }
    }
}
