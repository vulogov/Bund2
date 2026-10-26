//! What two runs of one program are compared on.
//!
//! **Between the tiers, everything.** Tier 0 and Tier 1 are one binary's
//! two paths to one meaning, so their normalised output and exit status are
//! compared byte for byte, the error report included.
//!
//! **Against the oracle, three parts.** Its error report is not Bund2's:
//! D36 replaced the reference's two-row table, whose `Location` row is a
//! Rust path on the machine that built the oracle (F66), with a report that
//! names the Bund source. So a run is cut into what the program wrote before
//! it failed, the error's message, and what was dumped after — the stack
//! and the workbench, which the capture epilogue prints on a clean run and
//! the engine prints under a failure. Bund2 runs with `--raw-values` so the
//! dump renders values as the oracle's does, and with `--report trace` so
//! its report is one shape to cut.

/// Messages Bund2 is decided to word differently: the oracle's, Bund2's,
/// and the ruling. Whole messages, so a neighbour that shares the text —
/// `pair`'s own guard, which `complex` copied — is still compared.
const APPROVED_MESSAGES: &[(&str, &str, &str)] = &[
    // F40, FIX: `wrap`'s guard and first two refusals name `UNWRAP`
    // (`reference/Bund/src/stdlib/functions/oop/value_class.rs:85,91,93`).
    (
        "Stack is too shallow for inline UNWRAP",
        "Stack is too shallow for inline WRAP",
        "F40",
    ),
    ("UNWRAP NO OBJECT IN #1", "WRAP NO OBJECT IN #1", "F40"),
    ("UNWRAP NO DATA IN #1", "WRAP NO DATA IN #1", "F40"),
    // F53, FIX: `execute.` guards the workbench it pulls from, not the main
    // stack (`reference/rust_multistackvm/src/stdlib/execute.rs:117`).
    (
        "Stack is too shallow for inline execute()",
        "Stack is too shallow for inline EXECUTE.()",
        "F53",
    ),
    // F18 as D105 applies it: the two-operand `math.*` words guard at two and
    // name themselves, where the reference guards at one as `float_op`
    // (`reference/Bund/src/stdlib/functions/math/math.rs:55`).
    (
        "Stack is too shallow for inline float_op",
        "Stack is too shallow for inline MATH.NROOT",
        "F18, D105",
    ),
    (
        "Stack is too shallow for inline float_op",
        "Stack is too shallow for inline MATH.PERIMETER",
        "F18, D105",
    ),
    (
        "Stack is too shallow for inline float_op",
        "Stack is too shallow for inline MATH.POWER",
        "F18, D105",
    ),
    // F40, FIX: `complex`'s guard names `pair`
    // (`reference/rust_multistackvm/src/stdlib/artefacts.rs:27`).
    (
        "Stack is too shallow for inline pair()",
        "Stack is too shallow for inline complex()",
        "F40",
    ),
];

/// F18, FIX, and D105 after it: these words guard at the arity they consume,
/// so where the reference pulls one operand and then misses the second
/// (`NO DATA #2`, or a cast of what it found), Bund2 fails before pulling and
/// leaves the stack as it was. The message and the dump both differ, by
/// ruling.
const F18_GUARDS: &[&str] = &[
    "Stack is too shallow for inline AT()",
    "Stack is too shallow for inline complex()",
    "Stack is too shallow for inline HEAD()",
    "Stack is too shallow for inline pair()",
    "Stack is too shallow for inline TAIL()",
    "Stack is too shallow for inline STRING.DISTANCE.LEVENSHTEIN",
    "Stack is too shallow for inline STRING.DISTANCE.DAMERAULEVENSHTEIN",
    "Stack is too shallow for inline STRING.DISTANCE.HAMMING",
    "Stack is too shallow for inline STRING.DISTANCE.JAROWINKLER",
    "Stack is too shallow for inline STRING.DISTANCE.SIFT3",
    "Stack is too shallow for inline STRING.REGEX",
    "Stack is too shallow for inline STRING.REGEX.MATCHES",
    "Stack is too shallow for inline STRING.WILDCARD",
    "Stack is too shallow for inline MATH.NROOT",
    "Stack is too shallow for inline MATH.PERIMETER",
    "Stack is too shallow for inline MATH.POWER",
];

/// Whether Bund2 stopped where a ruling says the reference goes on: F60 and
/// F72's `Context is empty`, which Bund2 says only where the reference's dead
/// guard lets `endcontext` drop the current stack — a bare `endcontext`, or a
/// `( … )` whose CONTEXT value collecting pushed rather than switched to. What
/// either engine does after that point is not compared; what came before is.
pub(crate) fn ruled_stop(bund2: &str) -> bool {
    without_wrappers(bund2) == "Context is empty"
}

/// Whether Bund2 stopped on an F18 guard the reference does not have.
pub(crate) fn f18_guard(bund2: &str, oracle: &str) -> bool {
    let b = without_wrappers(bund2);
    b != without_wrappers(oracle) && F18_GUARDS.contains(&b)
}

/// A context's stack name made a placeholder, as F14 makes `id` one.
///
/// `( … )` runs on a stack the engine names with a fresh nanoid, and a value
/// made there keeps the name in its `stack` tag, so the oracle prints a
/// different one every run; Bund2 names it from a counter in the same
/// alphabet (D1). Only a 21-character name in nanoid's alphabet is replaced,
/// so `"stack": "main"` and every name a program chose are compared.
pub(crate) fn stack_ids(out: &str) -> String {
    const KEY: &str = "\"stack\": \"";
    let is_nanoid = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    let mut result = String::with_capacity(out.len());
    let mut rest = out;
    while let Some(at) = rest.find(KEY) {
        let from = at + KEY.len();
        result.push_str(&rest[..from]);
        let tail = &rest[from..];
        let name_len = tail.find('"').unwrap_or(0);
        let name = &tail[..name_len];
        if name.chars().count() == 21 && name.chars().all(is_nanoid) {
            result.push_str("<id>");
            rest = &tail[name_len..];
        } else {
            rest = tail;
        }
    }
    result.push_str(rest);
    result
}

/// The oracle's output without its `env_logger` lines.
///
/// A library word that logs — `seq` given a config that is not a dict logs
/// `Error casting conf object` (`reference/Bund/src/stdlib/helpers/conf.rs:9`)
/// — writes `[<wall clock> ERROR <Rust module path>] …` to stderr. Neither
/// field is a contract Bund2 can keep (the `log.*` ruling in the decision
/// register says why), and the clock makes two runs of one program differ,
/// so the line would also read as the oracle disagreeing with itself.
pub(crate) fn without_log_lines(out: &str) -> String {
    let is_log = |l: &str| {
        let b = l.as_bytes();
        b.len() > 22
            && b[0] == b'['
            && b[1..5].iter().all(u8::is_ascii_digit)
            && b[5] == b'-'
            && l.get(11..12) == Some("T")
            && l[1..].find("Z ").is_some_and(|z| z < 26)
    };
    let kept: String = out.split_inclusive('\n').filter(|l| !is_log(l)).collect();
    // `normalise` joins lines with no newline after the last, so a dropped
    // last line leaves one behind that the other engine's text lacks.
    if kept.len() < out.len() && !out.ends_with('\n') {
        return kept.strip_suffix('\n').unwrap_or(&kept).to_string();
    }
    kept
}

/// The ruling under which two messages may differ, if one does.
pub(crate) fn approved_message(bund2: &str, oracle: &str) -> Option<&'static str> {
    let (b, o) = (without_wrappers(bund2), without_wrappers(oracle));
    if let Some(why) = APPROVED_MESSAGES
        .iter()
        .find(|(ot, bt, _)| *ot == o && *bt == b)
        .map(|(_, _, why)| *why)
    {
        return Some(why);
    }
    // F120, CORRECTED by D57: `convert.to_dict` targets MAP (11) where the
    // reference targets MATRIX (26,
    // `reference/rust_multistackvm/src/stdlib/convert/internal.rs:101,104`), so
    // a refusal names the other target.
    if let (Some(bt), Some(ot)) = (b.strip_suffix(" to 11"), o.strip_suffix(" to 26"))
        && bt == ot
        && bt.starts_with("CONVERT.TO_DICT")
    {
        return Some("F120");
    }
    if parse_errors(b, o) {
        return Some("D219");
    }
    (b == o && (b != bund2 || o != oracle)).then_some("D196")
}

/// The sentences Bund2's parser writes (`bund2_syntax`, `Parser::err`), by
/// their opening words; `parse_sentences_are_all_recognised` holds this to
/// the parser.
const PARSE_SENTENCES: &[&str] = &[
    "unclosed block, expected",
    "empty block:",
    "expected a term, found end of input",
    "unterminated string",
    "unterminated escape",
    "unterminated literal",
    "expected whitespace or end of input",
    "expected a name, found end of input",
    "unexpected character",
    "nesting deeper than",
    "Error converting FLOAT to VALUE:",
    "Error converting INT to VALUE:",
];

/// D219: a parse error from `bund.compile` or `bund.eval` is the reference's
/// prefix and then each parser's own reason. The reference's is pest's,
/// ` --> line:col` on the first line (`reference/bund_language_parser/src/lib.rs:31-32`,
/// re-raised bare by `reference/Bund/src/stdlib/helpers/eval.rs:40-41` and
/// under `BUND.COMPILE error in parsing of BUND code:` by
/// `reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:46`); Bund2's
/// is its parser's sentence, after `line N: ` where `compile` renders one.
/// Both must be parse errors, under the same prefix.
fn parse_errors(bund2: &str, oracle: &str) -> bool {
    let Some(at) = oracle.rfind(" --> ") else {
        return false;
    };
    let place = &oracle[at + " --> ".len()..];
    let is_place = place.split_once(':').is_some_and(|(l, c)| {
        !l.is_empty()
            && !c.is_empty()
            && l.chars().all(|ch| ch.is_ascii_digit())
            && c.chars().all(|ch| ch.is_ascii_digit())
    });
    if !is_place {
        return false;
    }
    let prefix = oracle[..at].trim_end();
    let Some(rest) = bund2.strip_prefix(prefix) else {
        return false;
    };
    let rest = rest.trim_start();
    let rest = match rest.strip_prefix("line ").and_then(|r| r.split_once(": ")) {
        Some((n, r)) if n.chars().all(|ch| ch.is_ascii_digit()) => r,
        _ => rest,
    };
    PARSE_SENTENCES.iter().any(|p| rest.starts_with(p)) || rest.contains("` needs a name after it")
}

/// A message without the wrappers each lambda a failure leaves adds.
///
/// The reference wraps a failure once per body it unwinds through —
/// `Lambda content evaluation returned error: `
/// (`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:13-18`), and
/// `<WORD>: lambda execution returns error: ` from each looping word — and
/// Bund2 reports the failure itself, with D173's traceback saying where it
/// was; D196 ruled the wrapper is written once, at most. Only a leading
/// wrapper is removed, so a message that merely mentions one is compared.
///
/// Also removed, from either engine's message: the evaluator's own
/// `Attempt to evaluate value … returned error: `, which Bund2 keeps for the
/// callers that want the reference's text (`Interp::eval`; `debug` is one),
/// and trailing blanks — the reference's `… to stack {name}` with an empty
/// name ends in a space that a report's row does not keep.
pub(crate) fn without_wrappers(message: &str) -> &str {
    const LAMBDA: &str = "Lambda content evaluation returned error: ";
    const LOOP: &str = ": lambda execution returns error: ";
    const EVAL: &str = "Attempt to evaluate value ";
    const EVAL_END: &str = " returned error: ";
    let mut m = message.trim_end();
    loop {
        if let Some(rest) = m.strip_prefix(LAMBDA) {
            m = rest;
            continue;
        }
        if m.starts_with(EVAL)
            && let Some(i) = m.find(EVAL_END)
        {
            m = &m[i + EVAL_END.len()..];
            continue;
        }
        // `MAP: …`, `TIMES: …`, `FOR.: …`: one word, upper case, then the
        // wrapper.
        if let Some(i) = m.find(LOOP)
            && m[..i]
                .chars()
                .all(|c| c.is_ascii_uppercase() || c == '.' || c == '_' || c == '*')
            && i > 0
        {
            m = &m[i + LOOP.len()..];
            continue;
        }
        return m;
    }
}

/// A run cut into its three parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Parts {
    pub(crate) before: String,
    pub(crate) error: Option<String>,
    pub(crate) after: String,
}

/// The oracle's error report: a `comfy_table` box whose first row is
/// `│ Error    ┆ <message> │`, then a `Location` row.
pub(crate) fn oracle_parts(out: &str) -> Parts {
    const ROW: &str = "│ Error    ┆ ";
    let Some(row) = out.find(ROW) else {
        return pest_parts(out).unwrap_or_else(|| clean(out));
    };
    // The box opens on the `╭` before the row, which may follow output the
    // program wrote without a newline.
    let start = out[..row].rfind('╭').unwrap_or(row);
    let msg_from = row + ROW.len();
    let msg_end = out[msg_from..]
        .find('\n')
        .map_or(out.len(), |i| msg_from + i);
    let message = out[msg_from..msg_end]
        .trim_end()
        .trim_end_matches('│')
        .trim_end()
        .to_string();
    // The box closes on the first line after the row that begins with `╰`.
    let after = match out[msg_end..].find("\n╰") {
        Some(i) => {
            let close = msg_end + i + 1;
            out[close..]
                .find('\n')
                .map_or("", |j| &out[close + j + 1..])
        }
        None => "",
    };
    Parts {
        before: out[..start].to_string(),
        error: Some(oracle_message(&message)),
        after: after.to_string(),
    }
}

/// Bund2's trace report: `Error: <message>`, then a source excerpt that
/// opens on `╭─[` and closes on a line of `─` ending `╯`.
pub(crate) fn bund2_parts(out: &str) -> Parts {
    let Some(excerpt) = out.find("╭─[") else {
        return clean(out);
    };
    let Some(start) = out[..excerpt].rfind("Error: ") else {
        return clean(out);
    };
    let msg_from = start + "Error: ".len();
    let msg_end = out[msg_from..]
        .find('\n')
        .map_or(out.len(), |i| msg_from + i);
    let mut after = "";
    let mut at = excerpt;
    while let Some(i) = out[at..].find('\n') {
        let line_start = at + i + 1;
        let line_end = out[line_start..]
            .find('\n')
            .map_or(out.len(), |j| line_start + j);
        let line = &out[line_start..line_end];
        if line.starts_with('─') && line.trim_end().ends_with('╯') {
            after = out.get(line_end + 1..).unwrap_or("");
            break;
        }
        at = line_start;
    }
    Parts {
        before: out[..start].to_string(),
        error: Some(out[msg_from..msg_end].trim_end().to_string()),
        after: after.to_string(),
    }
}

fn clean(out: &str) -> Parts {
    Parts {
        before: out.to_string(),
        error: None,
        after: String::new(),
    }
}

/// The reference's message as Bund2 words it.
///
/// Three wrappers come off. A failure inside an evaluation is reported as
/// `Attempt to evaluate value Value { … } returned error: <message>`, the
/// value being the word that failed; a token the parser cannot convert, as
/// `Error parsing token: <message>` (F49, D194); and `easy_error`'s `bail!`
/// appends the Rust location it was raised at, ` (src/…rs:16:5)` (F66).
pub(crate) fn oracle_message(raw: &str) -> String {
    let mut m = raw.trim().to_string();
    if let Some(rest) = m.strip_prefix("Error parsing token: ") {
        m = rest.to_string();
    }
    while m.starts_with("Attempt to evaluate value ") {
        match m.find(" returned error: ") {
            Some(i) => m = m[i + " returned error: ".len()..].to_string(),
            None => break,
        }
    }
    strip_rust_locations(&m)
}

/// The oracle's other report: an error whose text holds a pest parse error
/// — `bund.compile`'s and `bund.eval`'s, on a string that does not parse — is
/// printed as `Error occured:`, the message, pest's caret block, and then the
/// dump (`reference/Bund/src/stdlib/helpers/print_error.rs`), with no box. The
/// message is its first line, which ends at pest's ` --> line:col`.
fn pest_parts(out: &str) -> Option<Parts> {
    const HEAD: &str = "Error occured";
    const DUMP: &str = "Content of the stack";
    let head = out.find(HEAD)?;
    let start = out[..head].rfind('\n').map_or(0, |i| i + 1);
    let msg_from = out[head..].find('\n').map(|i| head + i + 1)?;
    let msg_end = out[msg_from..]
        .find('\n')
        .map_or(out.len(), |i| msg_from + i);
    let message = without_escapes(&out[msg_from..msg_end]);
    let after = out[msg_end..]
        .find(DUMP)
        .map(|i| msg_end + i)
        .map_or("", |d| &out[out[..d].rfind('\n').map_or(0, |i| i + 1)..]);
    Some(Parts {
        before: out[..start].to_string(),
        error: Some(oracle_message(&message)),
        after: after.to_string(),
    })
}

/// A line with its ANSI colour escapes removed.
fn without_escapes(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Remove every ` (<path>.rs:<line>:<col>)`.
fn strip_rust_locations(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find(" (") {
        let tail = &rest[open + 2..];
        let close = tail.find(')');
        let is_location = close.is_some_and(|c| {
            let inner = &tail[..c];
            let mut it = inner.rsplitn(3, ':');
            let col = it.next().unwrap_or("");
            let line = it.next().unwrap_or("");
            let path = it.next().unwrap_or("");
            path.ends_with(".rs")
                && !col.is_empty()
                && col.chars().all(|c| c.is_ascii_digit())
                && !line.is_empty()
                && line.chars().all(|c| c.is_ascii_digit())
        });
        match (is_location, close) {
            (true, Some(c)) => {
                out.push_str(&rest[..open]);
                rest = &tail[c + 1..];
            }
            _ => {
                out.push_str(&rest[..open + 2]);
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whether Bund2 stopped on its own defect rather than the program's: an
/// internal error (D37's third way out), a panic, which shipped code must
/// never reach, or an exit no clean or failed program produces.
///
/// **Exit 1 with a report is a program that did not parse** (D194: the
/// reference exits 0 there, Bund2 1; no word the vocabulary holds sets a
/// status).
pub(crate) fn bund2_broke(status: i32, out: &str) -> Option<String> {
    if out.contains("internal error: ") {
        return Some("an internal error".into());
    }
    if out.contains("panicked at") {
        return Some("a panic".into());
    }
    if status == 1 && bund2_parts(out).error.is_some() {
        return None;
    }
    if status != 0 {
        return Some(format!("exit status {status}"));
    }
    None
}

/// Whether the oracle aborted: a panic, which D37 says Bund2 declines to
/// reproduce, so there is nothing to compare.
pub(crate) fn oracle_aborted(status: i32, out: &str) -> bool {
    status != 0 || out.contains("panicked at") || out.contains("Backtrace (most recent call last)")
}

/// The first line where two texts part, for a finding's report.
pub(crate) fn first_difference(got: &str, want: &str) -> String {
    let (g, w): (Vec<&str>, Vec<&str>) = (got.lines().collect(), want.lines().collect());
    for i in 0..g.len().max(w.len()) {
        let (a, b) = (g.get(i).copied(), w.get(i).copied());
        if a != b {
            return format!(
                "line {}:\n    bund2:  {}\n    oracle: {}",
                i + 1,
                a.unwrap_or("<none>"),
                b.unwrap_or("<none>")
            );
        }
    }
    "texts agree line by line; they differ in a trailing newline".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORACLE: &str = "x╭──────────┬────╮\n│ Error    ┆ Attempt to evaluate value Value { id: \"<id>\", data: String(\"execute\") } returned error: i(nosuchword) for stack returned: Inline nosuchword not registered (/Users/u/.cargo/registry/src/x/rust_multistack-0.33.0/src/ts_inline.rs:39:9)  │\n├╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌┤\n│ Location ┆ /a/b.rs:64:21 │\n╰──────────┴────╯\n[BUND]  Content of the stack\n╭───╮\n│ 1 │\n╰───╯\n";

    const BUND2: &str = "xError: i(nosuchword) for stack returned: Inline nosuchword not registered\n   ╭─[ case.bund:1:17 ]\n   │\n 1 │ \"x\" print nosuchword\n   │ \n   │ Note: on stack `main`\n───╯\n[BUND]  Content of the stack\n╭───╮\n│ 1 │\n╰───╯\n";

    #[test]
    fn the_two_reports_cut_to_the_same_parts() {
        let o = oracle_parts(ORACLE);
        let b = bund2_parts(BUND2);
        assert_eq!(o.before, "x");
        assert_eq!(
            o.error.as_deref(),
            Some("i(nosuchword) for stack returned: Inline nosuchword not registered")
        );
        assert_eq!(o, b);
    }

    #[test]
    fn a_clean_run_is_all_before() {
        let p = oracle_parts("1\n2\n");
        assert_eq!(p.error, None);
        assert_eq!(p.before, "1\n2\n");
        assert_eq!(bund2_parts("1\n2\n"), p);
    }

    #[test]
    fn the_evaluators_wrapper_and_trailing_blanks_are_not_compared() {
        assert_eq!(
            approved_message(
                "Attempt to evaluate value Value { data: String(\"x\") } returned error: i(x) for stack returned: Inline x not registered",
                "i(x) for stack returned: Inline x not registered"
            ),
            Some("D196")
        );
        assert_eq!(
            approved_message(
                "VM inline function returned error: Nothing has been returned from workbench to stack",
                "VM inline function returned error: Nothing has been returned from workbench to stack "
            ),
            Some("D196")
        );
        assert!(ruled_stop(
            "Lambda content evaluation returned error: Context is empty"
        ));
        assert!(!ruled_stop("Context is empty for a while"));
    }

    #[test]
    fn a_parse_error_is_approved_by_d219_under_the_same_prefix() {
        let compile = "BUND.COMPILE error in parsing of BUND code:";
        assert_eq!(
            approved_message(
                &format!("{compile} line 1: expected whitespace or end of input, found `\\`"),
                &format!("{compile}  --> 1:1")
            ),
            Some("D219")
        );
        assert_eq!(
            approved_message("unclosed block, expected `}`", " --> 2:1"),
            Some("D219")
        );
        // Not a parse error on Bund2's side, or not under the same prefix.
        assert_eq!(approved_message("Context is empty", " --> 1:1"), None);
        assert_eq!(
            approved_message("unterminated string", &format!("{compile}  --> 1:1")),
            None
        );
        assert_eq!(approved_message("unterminated string", "x --> a:b"), None);
    }

    /// Every sentence Bund2's parser writes is one `parse_errors` knows.
    #[test]
    fn parse_sentences_are_all_recognised() {
        let deep = format!("{}1", "{ ".repeat(1000));
        for src in [
            "{ 1",
            "[ ]",
            "\"oops",
            "\"oops\\",
            "'oops",
            "`",
            "q\\\"x",
            "}",
            "1_000",
            "1_000.5",
            deep.as_str(),
        ] {
            let Err(e) = bund2_syntax::compile(src) else {
                panic!("{src:?} was expected not to parse");
            };
            assert!(
                parse_errors(&e.to_string(), " --> 1:1"),
                "{src:?}: {e} is not in PARSE_SENTENCES"
            );
            assert!(
                parse_errors(&format!("P: {}", e.render(src)), "P:  --> 1:1"),
                "{src:?}: {}",
                e.render(src)
            );
        }
    }

    #[test]
    fn the_wrappers_a_lambda_adds_are_approved_by_d196() {
        assert_eq!(
            approved_message(
                "Stack is too shallow for inline ==",
                "MAP: lambda execution returns error: Lambda content evaluation returned error: Stack is too shallow for inline =="
            ),
            Some("D196")
        );
        assert_eq!(
            approved_message(
                "Workbench is too shallow for inline STRING.REGEX.",
                "FOR.: lambda execution returns error: Lambda content evaluation returned error: Workbench is too shallow for inline STRING.REGEX."
            ),
            Some("D196")
        );
        assert_eq!(
            approved_message("x", "Lambda content evaluation returned error: y"),
            None
        );
        assert_eq!(
            approved_message(
                "CONVERT.TO_DICT returned error: Can not convert lambda to 11",
                "CONVERT.TO_DICT returned error: Can not convert lambda to 26"
            ),
            Some("F120")
        );
        assert_eq!(
            approved_message("a b: lambda execution returns error: x", "x"),
            None
        );
    }

    #[test]
    fn a_pest_error_is_read_as_an_error() {
        let out = "x\n\u{1b}[33mError occured\u{1b}[0m:\n   \u{1b}[31mAttempt to evaluate value Value { data: String(\"compile\") } returned error: BUND.COMPILE error in parsing of BUND code:  --> 1:1\n  |\n1 | q\n  = expected EOI\n[BUND]  Content of the stack\n╭╮\n╰╯";
        let p = oracle_parts(out);
        assert_eq!(p.before, "x\n");
        assert_eq!(
            p.error.as_deref(),
            Some("BUND.COMPILE error in parsing of BUND code:  --> 1:1")
        );
        assert_eq!(p.after, "[BUND]  Content of the stack\n╭╮\n╰╯");
    }

    #[test]
    fn an_approved_message_is_matched_whole() {
        assert_eq!(
            approved_message("WRAP NO OBJECT IN #1", "UNWRAP NO OBJECT IN #1"),
            Some("F40")
        );
        assert_eq!(
            approved_message(
                "Stack is too shallow for inline complex()",
                "Stack is too shallow for inline pair()"
            ),
            Some("F40")
        );
        assert_eq!(
            approved_message(
                "Stack is too shallow for inline pair()",
                "Stack is too shallow for inline pair()"
            ),
            None
        );
    }

    #[test]
    fn a_context_stack_name_is_a_placeholder_and_a_chosen_one_is_not() {
        let o = r#"tags: {"stack": "j-Wh009imz1nv1rZ_AmPC"} tags: {"stack": "main"}"#;
        let b = r#"tags: {"stack": "-____________________"} tags: {"stack": "main"}"#;
        assert_eq!(stack_ids(o), stack_ids(b));
        assert_eq!(
            stack_ids(o),
            r#"tags: {"stack": "<id>"} tags: {"stack": "main"}"#
        );
    }

    #[test]
    fn log_lines_are_dropped_and_nothing_else() {
        let out = "1\n[2026-10-26T14:53:41Z ERROR bund::stdlib::helpers::conf] Error casting conf object: x\n[1 2]\n";
        assert_eq!(without_log_lines(out), "1\n[1 2]\n");
        // As `normalise` leaves it: no newline after the last line.
        let last = "1\n[2026-10-26T14:53:41Z ERROR bund::x] y";
        assert_eq!(without_log_lines(last), "1");
        assert_eq!(without_log_lines("[2026] x\n"), "[2026] x\n");
    }

    #[test]
    fn only_rust_locations_are_stripped() {
        assert_eq!(
            strip_rust_locations("Because I can (src/stdlib/raise.rs:16:5) (x) (a.rs:1)"),
            "Because I can (x) (a.rs:1)"
        );
    }

    #[test]
    fn breakage_is_told_from_a_failed_program() {
        assert_eq!(bund2_broke(0, "Error: no"), None);
        assert!(bund2_broke(0, "internal error: x").is_some());
        assert!(bund2_broke(101, "").is_some());
        // D194: a program that does not parse exits 1, with a report.
        assert_eq!(bund2_broke(1, BUND2), None);
        assert!(bund2_broke(1, "no report").is_some());
    }
}
