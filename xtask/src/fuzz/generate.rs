//! Program generation: the terms of `bund.pest`, and the words a program may
//! call.
//!
//! Every term the grammar has is generated —
//! `reference/bund_language_parser/bund.pest:7-20` lists them: integer,
//! float, string, literal, atom, stack, name, ptr, command, and the three
//! brackets, lambda, list and ctx. Each is written in a form its rule
//! accepts, so a generated program always parses and the run measures
//! evaluation, not the parser (`xtask parity` measures that).
//!
//! **A program of random terms tests almost nothing.** Its first word meets
//! the wrong operands, refuses, and the rest never runs. So a program is
//! built from *statements*: the operands a word consumes, then the word,
//! with the operands typed by what `docs/arity.md` measured the word to
//! accept. Most statements are fed right; some are fed anything, which is
//! how a refusal gets compared too.

use std::path::Path;

/// SplitMix64. Small, seedable, and the same stream on every host, so a seed
/// printed by one run reproduces it on another.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`; 0 when `n` is 0.
    pub(crate) fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }

    /// True `pct` times in a hundred.
    pub(crate) fn chance(&mut self, pct: u64) -> bool {
        self.next_u64() % 100 < pct
    }

    pub(crate) fn pick<'a, T>(&mut self, xs: &'a [T]) -> Option<&'a T> {
        xs.get(self.below(xs.len()))
    }
}

/// The operand type `docs/arity.md` found a word to accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Int,
    Float,
    Str,
    List,
    Any,
}

/// A word a program may call, and what it was measured to consume.
#[derive(Debug, Clone)]
pub(crate) struct Word {
    pub(crate) name: String,
    pub(crate) consumes: Option<usize>,
    pub(crate) kind: Kind,
}

/// Words left out of the vocabulary although their effect is pure or
/// stdout, each for a reason the effect classification cannot see.
///
/// The rest is decided by `corpus::classify`: a word whose effect is not
/// `Pure` or `Stdout`, a deferred word, and a word no golden can hold
/// (`corpus::NO_GOLDEN_CAN_HOLD`) never reach a program.
pub(crate) const LEFT_OUT: &[(&str, &str)] = &[
    // A run that never ends compares nothing; it costs the timeout and is
    // skipped. These loop until a condition the generator cannot steer.
    ("loop", "loops until its body says stop"),
    ("loop.", "loops until its body says stop"),
    ("*loop", "loops until its body says stop"),
    ("*loop.", "loops until its body says stop"),
    ("while", "loops until a condition fails"),
    ("while.", "loops until a condition fails"),
    // A word that names a body lets the program call it from inside itself,
    // and the oracle has no memory guard: D171 records a recursion that grew
    // to 18 GB and took the host down with eight fuzz runs beside it.
    (
        "register",
        "defines a word, so a program can recurse without bound",
    ),
    (
        "alias",
        "defines a word, so a program can recurse without bound",
    ),
    (
        "alias=",
        "defines a word, so a program can recurse without bound",
    ),
    (
        "alias=.",
        "defines a word, so a program can recurse without bound",
    ),
    // Classified pure with the rest of `bund_eval.rs`, but they read a file.
    ("bund.eval-file", "reads a file"),
    ("bund.eval-file.", "reads a file"),
    // 50 ms a character: a run is all sleep.
    ("console.typewriter", "sleeps 50 ms a character (D217)"),
    // Classified pure by their directory, and random in both engines; a run
    // that answers two ways is compared to nothing, so these only cost.
    ("id.ulid", "a fresh ULID each call"),
    ("id.uuid", "a fresh UUID each call"),
    // Given anything but a string, the whole value is encoded, its id and
    // stamp with it, so the text differs every run (F14's class).
    ("encode.base64", "encodes a value's id and stamp"),
    ("encode.base64.", "encodes a value's id and stamp"),
    // F74: a hash set's order, different every run in both engines.
    ("string.tokenize.unique", "answers in hash order (F74)"),
    ("string.tokenize.stemmed", "answers in hash order (F74)"),
    // F60 and F72, FIX: written bare, no context is open, the reference's
    // guard cannot fire and it drops the current stack; Bund2 refuses. That
    // approved difference is all a bare `endcontext` can show (`( … )`
    // closes its own context without it).
    ("endcontext", "closes a context never opened (F60, F72)"),
    // F120, CORRECTED by D57: a MAP where the reference makes a MATRIX, so
    // every answer and every refusal differs by ruling.
    (
        "convert.to_dict",
        "a MAP, where the reference makes a MATRIX (F120)",
    ),
    (
        "convert.to_dict.",
        "a MAP, where the reference makes a MATRIX (F120)",
    ),
];

/// The operand counts and types `docs/arity.md` records, by word.
///
/// `consumed` where the oracle was probed, else the word's own depth guard,
/// `declared`. A `+` (variadic) reads as its floor.
pub(crate) fn read_arity(repo: &Path) -> std::collections::BTreeMap<String, (Option<usize>, Kind)> {
    let mut out = std::collections::BTreeMap::new();
    let Ok(src) = std::fs::read_to_string(repo.join("docs/arity.md")) else {
        return out;
    };
    for line in src.lines() {
        // `| `name` | declared | wb | consumed | produced | via | …`. The name
        // is cut at its backticks, since a name may hold `|`.
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some(end) = rest.find("` |") else {
            continue;
        };
        let name = &rest[..end];
        let cells: Vec<&str> = rest[end + 3..].split('|').map(str::trim).collect();
        let count =
            |s: Option<&&str>| s.and_then(|c| c.trim_end_matches('+').parse::<usize>().ok());
        let consumes = count(cells.get(2)).or_else(|| count(cells.first()));
        let kind = match cells.get(4).copied() {
            Some("int") => Kind::Int,
            Some("float") => Kind::Float,
            Some("string") => Kind::Str,
            Some("list") => Kind::List,
            _ => Kind::Any,
        };
        out.insert(name.to_string(), (consumes, kind));
    }
    out
}

/// Names a program uses for stacks and atoms. Few, so that one statement's
/// `@b` and another's `:b` meet.
const NAMES: &[&str] = &["a", "b", "k", "main", "x"];

const STRINGS: &[&str] = &[
    "",
    "a",
    "hello world",
    "42",
    "3.14",
    "-7",
    "true",
    "a,b,c",
    "héllo",
    "line\\none",
    "q\\\"uote",
    "{}",
    "[1 2]",
    "x y z",
];

/// No digit separator: the grammar takes `1_000` and the token handler
/// cannot convert it, so a program holding one never runs (F49).
const INTEGERS: &[&str] = &[
    "0",
    "1",
    "2",
    "3",
    "-1",
    "7",
    "10",
    "+5",
    "100",
    "-42",
    "9223372036854775807",
    "-9223372036854775808",
];

const FLOATS: &[&str] = &[
    "0.0", "1.5", "-0.25", "3.", "2.5e3", "1.0e-2", "-0.0", "100.125", "0.1",
];

/// The words that change which stack is current without naming one. Inside
/// `( … )` the context then closes on another stack, and where it returns is
/// F72's ruled difference: Bund2 to the stack the context was opened from,
/// the reference to whatever its deque holds next.
const RING_WORDS: &[&str] = &[
    "stacks_right",
    "stacks_left",
    "->",
    "→",
    "<-",
    "←",
    "drop_stack",
];

/// The words a compiled body is made of: the integer fragments Tier 1
/// lowers (`bund2_ir::Op`), plus the stack words beside them.
const TIER_WORDS: &[&str] = &[
    "+", "-", "*", "<", ">", "<=", ">=", "==", "!=", "drop", "dup", "swap",
];

pub(crate) struct Gen<'a> {
    pub(crate) rng: Rng,
    pub(crate) vocab: &'a [Word],
    /// How many `{ }` and `[ ]` enclose the term being written.
    blocks: usize,
    /// How many `( )` do.
    contexts: usize,
}

impl<'a> Gen<'a> {
    pub(crate) fn new(seed: u64, vocab: &'a [Word]) -> Self {
        Gen {
            rng: Rng::new(seed),
            vocab,
            blocks: 0,
            contexts: 0,
        }
    }

    fn int(&mut self) -> String {
        // Mostly small, so a count or an index is usually in range.
        if self.rng.chance(70) {
            return (self.rng.below(12) as i64 - 2).to_string();
        }
        self.rng.pick(INTEGERS).copied().unwrap_or("0").to_string()
    }

    fn float(&mut self) -> String {
        self.rng.pick(FLOATS).copied().unwrap_or("0.0").to_string()
    }

    fn string(&mut self) -> String {
        format!("\"{}\"", self.rng.pick(STRINGS).copied().unwrap_or(""))
    }

    fn name(&mut self) -> &'static str {
        self.rng.pick(NAMES).copied().unwrap_or("a")
    }

    fn word(&mut self) -> Option<&Word> {
        let i = self.rng.below(self.vocab.len());
        self.vocab.get(i)
    }

    /// One or more terms inside a bracket; the grammar has `term+`, so never
    /// none.
    fn bracketed(&mut self, open: &str, close: &str, depth: usize) -> String {
        let block = open != "(";
        self.blocks += usize::from(block);
        self.contexts += usize::from(!block);
        let n = 1 + self.rng.below(3);
        let mut parts = vec![open.to_string()];
        for _ in 0..n {
            if depth < 2 && self.rng.chance(30) {
                parts.extend(self.statement(depth + 1));
            } else {
                parts.push(self.term(depth + 1));
            }
        }
        parts.push(close.to_string());
        self.blocks -= usize::from(block);
        self.contexts -= usize::from(!block);
        parts.join(" ")
    }

    /// A list of plain values, the shape list words are measured with.
    fn list(&mut self, depth: usize) -> String {
        let n = 1 + self.rng.below(4);
        let kind = *self
            .rng
            .pick(&[Kind::Int, Kind::Int, Kind::Float, Kind::Str, Kind::Any])
            .unwrap_or(&Kind::Int);
        self.blocks += 1;
        let items: Vec<String> = (0..n).map(|_| self.value(kind, depth + 1)).collect();
        self.blocks -= 1;
        format!("[ {} ]", items.join(" "))
    }

    /// A value of `kind`, or of any kind for `Kind::Any`.
    pub(crate) fn value(&mut self, kind: Kind, depth: usize) -> String {
        match kind {
            Kind::Int => self.int(),
            Kind::Float => self.float(),
            Kind::Str => self.string(),
            Kind::List if depth < 3 => self.list(depth),
            Kind::List => self.int(),
            Kind::Any => self.term(depth),
        }
    }

    /// Any term of the grammar.
    pub(crate) fn term(&mut self, depth: usize) -> String {
        let deep = depth >= 3;
        match self.rng.below(if deep { 9 } else { 12 }) {
            0 | 1 => self.int(),
            2 => self.float(),
            3 | 4 => self.string(),
            5 => format!(":{}", self.name()),
            6 => format!("'{}'", self.name()),
            7 => match self.word() {
                Some(w) => format!("`{}", w.name),
                None => self.int(),
            },
            8 => {
                if self.rng.chance(50) {
                    ":".to_string()
                } else {
                    ";".to_string()
                }
            }
            9 => self.list(depth),
            10 => self.bracketed("{", "}", depth),
            // F58, approved by D34: inside `{ }` or `[ ]` the reference hoists a
            // `( … )` and its terms to the top-level stream, and Bund2 keeps it
            // where it is written. Every program holding one would show that
            // ruled difference and nothing else, so a block gets a block.
            _ if self.blocks > 0 => self.bracketed("{", "}", depth),
            _ => self.bracketed("(", ")", depth),
        }
    }

    /// Statements that run inside a lambda the program then runs.
    ///
    /// **This is what gives Tier 1 something to do.** The tier compiles a
    /// body when it is run, not when it is pushed, and a lambda only a
    /// random statement made is almost never run: two thousand programs
    /// without these compiled none, so the tiers agreed by never differing.
    /// Each form here runs its body — `execute` and `!` once, `times` a few
    /// times, `map` once an item with the item on the stack.
    ///
    /// **And the body has to be one the tier will take.** It compiles a body
    /// of integer literals and integer operators and declines anything else
    /// — a float, a string, a word outside the fragments
    /// (`crates/bund2-stdlib/src/fragments.rs`) — so a body of random
    /// statements is declined and runs in Tier 0 either way. Most bodies are
    /// written from that subset, extreme integers included for the wrap
    /// (D4), and some now and then from anything, to test the decline.
    fn body_run(&mut self, depth: usize) -> Vec<String> {
        let mut body = vec!["{".to_string()];
        self.blocks += 1;
        if self.rng.chance(65) {
            let n = 1 + self.rng.below(6);
            for _ in 0..n {
                if self.rng.chance(45) {
                    body.push(self.int());
                } else {
                    let w = self.rng.pick(TIER_WORDS).copied().unwrap_or("+");
                    body.push(w.to_string());
                }
            }
            if self.rng.chance(10) {
                body.push(self.term(depth + 1));
            }
        } else {
            let n = 1 + self.rng.below(3);
            for _ in 0..n {
                body.extend(self.statement(depth + 1));
            }
        }
        body.push("}".to_string());
        self.blocks -= 1;
        let body = body.join(" ");
        match self.rng.below(4) {
            0 => vec![body, "execute".into()],
            1 => vec![body, "!".into()],
            2 => vec![(1 + self.rng.below(3)).to_string(), body, "times".into()],
            _ => vec![self.list(depth), body, "map".into()],
        }
    }

    /// The operands a word consumes, then the word; now and then a stack
    /// switch first, or operands of any type; and now and then a body run
    /// (`body_run`) instead.
    pub(crate) fn statement(&mut self, depth: usize) -> Vec<String> {
        if depth < 2 && self.rng.chance(if depth == 0 { 30 } else { 10 }) {
            return self.body_run(depth);
        }
        let mut out = Vec::new();
        if depth == 0 && self.rng.chance(8) {
            out.push(format!("@{}", self.name()));
        }
        let Some(w) = self.word().cloned() else {
            out.push(self.term(depth));
            return out;
        };
        if self.contexts > 0 && RING_WORDS.contains(&w.name.as_str()) {
            out.push(self.term(depth));
            return out;
        }
        let typed = self.rng.chance(75);
        let n = match w.consumes {
            Some(n) if typed => n,
            _ => self.rng.below(4),
        };
        for _ in 0..n {
            let kind = if typed { w.kind } else { Kind::Any };
            let v = if kind == Kind::Any && self.rng.chance(15) && depth < 2 {
                self.bracketed("{", "}", depth)
            } else {
                self.value(kind, depth)
            };
            out.push(v);
        }
        out.push(w.name);
        out
    }

    /// A program, one statement a line, so a finding can be cut down a line
    /// at a time.
    pub(crate) fn program(&mut self, statements: usize) -> Vec<String> {
        (0..statements)
            .map(|_| self.statement(0).join(" "))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_gives_the_same_program_every_time() {
        let vocab = vec![
            Word {
                name: "+".into(),
                consumes: Some(2),
                kind: Kind::Int,
            },
            Word {
                name: "len".into(),
                consumes: Some(1),
                kind: Kind::Str,
            },
        ];
        let a = Gen::new(7, &vocab).program(20);
        let b = Gen::new(7, &vocab).program(20);
        assert_eq!(a, b);
        let c = Gen::new(8, &vocab).program(20);
        assert_ne!(a, c);
    }

    /// Every generated program parses: the grammar's brackets take `term+`,
    /// so an empty one would be a parser test, not an evaluation one.
    #[test]
    fn no_bracket_is_empty() {
        let vocab = vec![Word {
            name: "dup".into(),
            consumes: Some(1),
            kind: Kind::Any,
        }];
        let mut g = Gen::new(1, &vocab);
        for line in g.program(500) {
            for empty in ["{ }", "[ ]", "( )"] {
                assert!(!line.contains(empty), "{line}");
            }
        }
    }

    /// F58: no `( … )` inside a `{ }` or a `[ ]`, at any depth, where the
    /// reference would hoist it.
    #[test]
    fn no_context_is_written_inside_a_block() {
        let vocab = vec![Word {
            name: "dup".into(),
            consumes: Some(1),
            kind: Kind::Any,
        }];
        let mut g = Gen::new(3, &vocab);
        let mut contexts = 0;
        for line in g.program(2000) {
            let mut open: Vec<&str> = Vec::new();
            for t in line.split(' ') {
                match t {
                    "{" | "[" | "(" => {
                        if t == "(" {
                            contexts += 1;
                            assert!(!open.iter().any(|o| *o != "("), "{line}");
                        }
                        open.push(t);
                    }
                    "}" | "]" | ")" => {
                        open.pop();
                    }
                    _ => {}
                }
            }
        }
        assert!(contexts > 0, "contexts are still written at the top");
    }

    #[test]
    fn the_arity_table_is_read() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let a = read_arity(&repo);
        assert_eq!(a.get("+"), Some(&(Some(2), Kind::Int)));
        assert_eq!(a.get("**").map(|x| x.0), Some(Some(2)));
    }
}
