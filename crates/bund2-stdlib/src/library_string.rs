//! `bund/string` — the library half's string group, and every `.` sibling.
//!
//! # The `.` suffix is not one contract
//!
//! A `.`-suffixed word is usually described as "the same word, on the
//! workbench". In `bund/string` that is false in two separate ways, and both
//! are the reference's behaviour rather than a simplification here.
//!
//! **The second operand always comes from the stack.** A binary `.` word pulls
//! operand 1 from the workbench and operand 2 from the *current stack*
//! (`reference/Bund/src/stdlib/functions/string/prefix_suffix.rs:32-39`), which
//! is why its guard checks both depths (`:23-30`). Nothing reads a second
//! workbench cell.
//!
//! **Where the answer goes differs per family**, with no rule behind it:
//!
//! | family | `.` pushes to | source |
//! |---|---|---|
//! | `string.prefix` / `.suffix` | the **stack** | `prefix_suffix.rs:52` |
//! | `string.regex` | the **stack** | `regex.rs:47` |
//! | `string.regex.matches` | the **stack** | `regex_matches.rs:57` |
//! | `string.regex.split` | the **stack** | `regex_split.rs:50` |
//! | `string.wildcard` | the **stack** | `wildmatch.rs:44,46` |
//! | `string.grok` | the **stack** | `grok.rs:64` |
//! | `string.tokenize*` | the **stack** | `tokenize.rs:73` |
//! | `string.distance*` | the **workbench** | `distance.rs:90-91` |
//! | `string.expressionmatch` | the **workbench** | `textexpr_match.rs:66-67` |
//! | `string.fuzzymatch` | the **workbench** | `fuzzy_match.rs:67-70` |
//! | `string.deunicode` | the **workbench** | `unicode.rs:39-42` |
//! | `string.wrap.english` | the **workbench** | `textwrap.rs:76-79` |
//!
//! Seven of those push a result the caller asked for on the workbench onto the
//! stack instead. Recorded as **F73** — it is preserved, not corrected, but a
//! program written on the analogy of its neighbour will lose its answer.
//!
//! # What is not here, and why
//!
//! - **`id.ulid` and the `string.random.*` family** answer differently on
//!   every call. There is no golden to capture and no differential test to run,
//!   so implementing them buys a coverage point and no confidence.
//! - **`string.tokenize.stemmed`** would pull `rnltk` in for one stemmer, and
//!   `rnltk` pulls `nalgebra` — a linear-algebra crate — behind it
//!   (`reference/Bund/Cargo.lock`). For a word whose output order is not
//!   reproducible anyway (see below) that is a bad trade, and it is the one
//!   word of this group left undone.
//!
//! `string.tokenize.unique` **is** here, with a caveat that belongs in the
//! open: it builds its answer by iterating a `HashSet` (`tokenize.rs:49-57`),
//! whose order is seeded per process, so the oracle does not agree with
//! *itself* between two runs — five runs gave five orders, **F74**. What can be
//! reproduced is the property, not an order, so the implementation uses a
//! `HashSet` too and **no golden may capture the word**. Sorting the result
//! does not rescue it either: `sort` is not alphabetical, so equal-length
//! tokens stay in hash order.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// Which of the two forms a word was called as — the reference's `StackOps`
/// (`prefix_suffix.rs:17-31`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Stack,
    Bench,
}

/// Where a family's `.` form leaves its answer. See the table above; there is
/// no rule, so each word names its own.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Out {
    /// Always the current stack, whichever form was called.
    Stack,
    /// The stack for the plain form, the workbench for the `.` form.
    Mirror,
}

/// Guard both depths before pulling anything.
///
/// `stack_min` is the plain form's requirement, which is **not always the
/// number of operands**: `string.distance` pulls two and guards one
/// (`distance.rs:24`), and that F18 shape is reproduced rather than corrected
/// — the difference shows up as `NO DATA #2` on an already-shortened stack.
///
/// The `.` form's guard is fixed at one workbench cell plus one stack cell for
/// a binary word, one workbench cell for a unary one (`:29-34`).
fn guard(vm: &dyn Vm, side: Side, stack_min: usize, binary: bool, prefix: &str) -> Result<(), Error> {
    match side {
        Side::Stack => {
            if vm.depth() < stack_min {
                return Err(Error(format!("Stack is too shallow for inline {prefix}")));
            }
        }
        Side::Bench => {
            if vm.workbench_depth() < 1 {
                return Err(Error(format!(
                    "Workbench is too shallow for inline {prefix}"
                )));
            }
            if binary && vm.depth() < 1 {
                return Err(Error(format!("Stack is too shallow for inline {prefix}")));
            }
        }
    }
    Ok(())
}

/// Operand 1: the workbench for a `.` word, the stack otherwise.
fn first(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<BundValue, Error> {
    let v = match side {
        Side::Stack => vm.pull(),
        Side::Bench => vm.pull_workbench(),
    };
    v.ok_or_else(|| Error(format!("{prefix} returns: NO DATA #1")))
}

/// Operand 2: **always** the current stack (`prefix_suffix.rs:36-39`).
fn second(vm: &mut dyn Vm, prefix: &str) -> Result<BundValue, Error> {
    vm.pull()
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #2")))
}

fn deliver(vm: &mut dyn Vm, side: Side, out: Out, v: BundValue) {
    match (out, side) {
        (Out::Mirror, Side::Bench) => vm.push_workbench(v),
        _ => vm.push(v),
    }
}

/// `cast_string`, which **refuses a number**.
///
/// Verified against the oracle: `"1" 123 string.prefix` reports
/// `This Dynamic type is not string` rather than comparing `"123"`. That is the
/// opposite of the `string.upper` family, which converts first — the two
/// groups take different casts and neither can be written from the other.
fn text(v: &BundValue, prefix: &str, n: usize) -> Result<String, Error> {
    v.as_str().ok_or_else(|| {
        Error(format!(
            "{prefix} returned for #{n}: This Dynamic type is not string"
        ))
    })
}

// --- the shapes ------------------------------------------------------------

/// One string in, one value out.
fn unary(
    vm: &mut dyn Vm,
    side: Side,
    out: Out,
    prefix: &str,
    f: impl Fn(&str) -> BundValue,
) -> Result<(), Error> {
    guard(vm, side, 1, false, prefix)?;
    let a = first(vm, side, prefix)?;
    let a = text(&a, prefix, 1)?;
    let r = f(&a);
    deliver(vm, side, out, r);
    Ok(())
}

/// Two strings in, one value out. **Operand 1 is the subject**, operand 2 the
/// pattern — `string_data.starts_with(&pattern)` with `string_data` the first
/// pull (`prefix_suffix.rs:32-49`).
fn binary(
    vm: &mut dyn Vm,
    side: Side,
    out: Out,
    stack_min: usize,
    prefix: &str,
    f: impl Fn(&str, &str) -> Result<BundValue, Error>,
) -> Result<(), Error> {
    guard(vm, side, stack_min, true, prefix)?;
    let a = first(vm, side, prefix)?;
    let b = second(vm, prefix)?;
    let a = text(&a, prefix, 1)?;
    let b = text(&b, prefix, 2)?;
    let r = f(&a, &b)?;
    deliver(vm, side, out, r);
    Ok(())
}

fn list_of(items: impl Iterator<Item = String>) -> BundValue {
    BundValue::list(items.map(BundValue::str).collect())
}

// --- the words -------------------------------------------------------------

/// `string.prefix` / `string.suffix` (`prefix_suffix.rs:48-52`).
fn prefix_suffix(vm: &mut dyn Vm, side: Side, suffix: bool, prefix: &str) -> Result<(), Error> {
    binary(vm, side, Out::Stack, 2, prefix, |s, pat| {
        Ok(BundValue::boolean(if suffix {
            s.ends_with(pat)
        } else {
            s.starts_with(pat)
        }))
    })
}

/// `string.regex.matches` — every match, in order (`regex_matches.rs:43-57`).
///
/// A pattern that fails to compile is an error; a pattern that matches nothing
/// is an **empty list**, not a failure.
fn regex_matches(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    binary(vm, side, Out::Stack, 2, prefix, |s, pat| {
        let re = fancy_regex::Regex::new(pat)
            .map_err(|e| Error(format!("{prefix} compile returns: {e}")))?;
        Ok(list_of(
            re.find_iter(s)
                .filter_map(|m| m.ok())
                .map(|m| m.as_str().to_string()),
        ))
    })
}

/// `string.regex.split` (`regex_split.rs:43-50`).
fn regex_split(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    binary(vm, side, Out::Stack, 2, prefix, |s, pat| {
        let re = fancy_regex::Regex::new(pat)
            .map_err(|e| Error(format!("{prefix} compile returns: {e}")))?;
        Ok(list_of(
            re.split(s).filter_map(|p| p.ok()).map(|p| p.to_string()),
        ))
    })
}

/// `string.regex` and `string.wildcard`, `.` forms only — the plain forms are
/// F18 words and live in `singles.rs`.
///
/// **The subject is operand 2 here, not operand 1.** `regex.rs:28,31` pulls the
/// subject first *in the stack form*; the `.` form pulls the subject from the
/// workbench and the pattern from the stack, which is the same assignment. So
/// the workbench holds what is being matched.
fn pattern_match_wb(vm: &mut dyn Vm, wild: bool, prefix: &str) -> Result<(), Error> {
    binary(vm, Side::Bench, Out::Stack, 1, prefix, |subj, pat| {
        Ok(BundValue::boolean(if wild {
            wildmatch::WildMatch::new(pat).matches(subj)
        } else {
            fancy_regex::Regex::new(pat)
                .map_err(|e| Error(format!("{prefix} returned error: {e}")))?
                .is_match(subj)
                .unwrap_or(false)
        }))
    })
}

/// `string.tokenize` and `string.tokenize.lines` (`tokenize.rs:44-48,67-71`).
///
/// Both **trim every token**. `lines` splits on `str::lines`, so it needs no
/// library; `tokenize` uses `natural`, whose definition of a token is what a
/// golden would capture.
fn tokenize(vm: &mut dyn Vm, side: Side, lines: bool, prefix: &str) -> Result<(), Error> {
    unary(vm, side, Out::Stack, prefix, |s| {
        if lines {
            list_of(s.lines().map(|t| t.trim().to_string()))
        } else {
            list_of(
                natural::tokenize::tokenize(s)
                    .into_iter()
                    .map(|t| t.trim().to_string()),
            )
        }
    })
}

/// `string.tokenize.unique` — the tokens, **lower-cased and deduplicated**
/// (`tokenize.rs:49-57`).
///
/// The result is a LIST built by iterating a `HashSet`, so its **order is not
/// reproducible**: five runs of the oracle on one input gave five orders
/// (F74). A `HashSet` here reproduces the property — unordered, deduplicated —
/// which is all there is to reproduce, since no particular order is the
/// reference's answer either. A program that wants a stable answer must sort
/// it, and **no golden may capture this word**.
fn tokenize_unique(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    unary(vm, side, Out::Stack, prefix, |s| {
        let set: std::collections::HashSet<String> = natural::tokenize::tokenize(&s.to_lowercase())
            .into_iter()
            .map(|t| t.trim().to_string())
            .collect();
        list_of(set.into_iter())
    })
}

/// `string.wrap.english` — wrap to a width, hyphenating
/// (`textwrap.rs:63-79`).
///
/// **Operand 2 is a width, and it is `cast_int`** (`:57`) rather than
/// `cast_string`, so this is the one binary word here whose second operand is
/// not text. The wrap uses `Language::EnglishUS`'s embedded hyphenation
/// dictionary (`:64-70,85`), which is why a long word may break mid-word and
/// where it breaks is the dictionary's business.
fn wrap_english(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, 2, true, prefix)?;
    let s = first(vm, side, prefix)?;
    let n = second(vm, prefix)?;
    let s = text(&s, prefix, 1)?;
    let Some(n) = n.as_int() else {
        return Err(Error(format!(
            "{prefix} returned for #2: This Dynamic type is not integer"
        )));
    };
    use hyphenation::Load as _;
    let dict = hyphenation::Standard::from_embedded(hyphenation::Language::EnglishUS)
        .map_err(|e| Error(format!("{prefix} error creating dictionary: {e}")))?;
    let options = textwrap::Options::new(n.max(0) as usize)
        .word_splitter(textwrap::WordSplitter::Hyphenation(dict));
    let wrapped = textwrap::wrap(&s, &options);
    let r = list_of(wrapped.iter().map(|l| l.to_string()));
    deliver(vm, side, Out::Mirror, r);
    Ok(())
}

/// `string.grok` — parse a line with a grok pattern into a MAP
///
/// **Behind the `grok` feature, which is off by default — D10 and D40.** The
/// crate binds Oniguruma, a C library, and is the workspace's only `cc`
/// consumer. D10 lets `bund2 build` require a C toolchain; it does not let the
/// *interpreter* require one, and `--emit=bundle` exists so a target without a
/// toolchain can still run Bund.
#[cfg(feature = "grok")]
/// (`grok.rs:43-64`).
///
/// Three details decide the answer:
///
/// - The compiler is built `with_default_patterns` and `compile(pattern,
///   false)` (`:44-45`) — `false` is `named_only`, so **every** capture lands
///   in the map, not just the named ones.
/// - An **empty capture is dropped** (`:50-51`), so an optional field that did
///   not match is absent rather than `""`.
/// - A pattern that matches nothing is an **empty MAP**, not a failure (`:55-57`
///   logs and falls through). Only a pattern that fails to *compile* is an
///   error.
fn grok_word(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    binary(vm, side, Out::Stack, 2, prefix, |s, pattern| {
        let g = grok::Grok::with_default_patterns();
        let compiled = g
            .compile(pattern, false)
            .map_err(|e| Error(format!("{prefix} error while compiling the pattern: {e}")))?;
        let mut res = BundValue::map(Default::default());
        if let Some(m) = compiled.match_against(s) {
            for (k, v) in &m {
                if !v.is_empty() {
                    res = res.set(k, BundValue::str(v));
                }
            }
        }
        Ok(res)
    })
}

/// `string.deunicode` (`unicode.rs:38-42`). Transliterates to ASCII.
fn deunicode_word(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    unary(vm, side, Out::Mirror, prefix, |s| {
        BundValue::str(deunicode::deunicode(s))
    })
}

/// `string.fuzzymatch` — a Skim score, **0 when there is no match**
/// (`fuzzy_match.rs:62-66`).
///
/// `fuzzy_match(choice, pattern)`: operand 1 is the text searched, operand 2
/// the pattern searched for. The score is the matcher's, not a distance, so
/// larger is better and the scale is `fuzzy-matcher`'s to define.
fn fuzzymatch(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    use fuzzy_matcher::FuzzyMatcher;
    binary(vm, side, Out::Mirror, 2, prefix, |choice, pattern| {
        let m = fuzzy_matcher::skim::SkimMatcherV2::default();
        Ok(BundValue::int(m.fuzzy_match(choice, pattern).unwrap_or(0)))
    })
}

/// `string.distance.*` — the `.` forms (`distance.rs:73-91`).
///
/// The plain forms are in `singles.rs`; only the workbench siblings are new.
/// The stack guard is `< 1` for a word that pulls two, which is why
/// `stack_min` is 1 rather than 2 — F18's shape, preserved.
fn distance_wb(
    vm: &mut dyn Vm,
    prefix: &str,
    f: impl Fn(&str, &str) -> Result<BundValue, Error>,
) -> Result<(), Error> {
    binary(vm, Side::Bench, Out::Mirror, 1, prefix, f)
}

/// `string.expressionmatch.` (`textexpr_match.rs:28-32,66-67`).
///
/// **The expression is operand 1**, the opposite assignment to
/// `string.regex` — the two words have the same shape and swap their operands.
fn expressionmatch_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    binary(
        vm,
        Side::Bench,
        Out::Mirror,
        1,
        "STRING.EXPRESSIONMATCH.",
        |expr, subject| {
            let m = srch::Expression::new(&expr.to_string()).map_err(|e| {
                Error(format!(
                    "STRING.EXPRESSIONMATCH. returned error when creates matcher: {e:?}"
                ))
            })?;
            Ok(BundValue::boolean(m.matches(subject)))
        },
    )
}

/// The declared effect of a `.` form.
///
/// **Operand 1 does not appear in it.** It comes off the workbench, which no
/// [`StackEffect`] describes — RFC-0004 counts one stack, so a word that reads
/// the workbench is under-declared by construction and `check` will believe the
/// smaller number. That is a stated limit of the effect type rather than a
/// number chosen here, and it is why the `.` forms are not in the conformance
/// denominator's arity table.
///
/// So: `consumes` is 1 for a binary word and 0 for a unary one, and `produces`
/// is 1 only when the family pushes to the *stack* (see the table above).
fn wb_eff(binary: bool, out: Out) -> StackEffect {
    let consumes = u8::from(binary);
    let produces = u8::from(out == Out::Stack);
    eff(consumes, produces)
}

pub fn register(r: &mut Registry) {
    // The plain forms.
    r.register_native(
        "string.prefix",
        |vm| prefix_suffix(vm, Side::Stack, false, "STRING.PREFIX"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.suffix",
        |vm| prefix_suffix(vm, Side::Stack, true, "STRING.SUFFIX"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.regex.matches",
        |vm| regex_matches(vm, Side::Stack, "STRING.REGEX.MATCHES"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.regex.split",
        |vm| regex_split(vm, Side::Stack, "STRING.REGEX.SPLIT"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.tokenize",
        |vm| tokenize(vm, Side::Stack, false, "STRING.TOKENIZE"),
        eff(1, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.tokenize.lines",
        |vm| tokenize(vm, Side::Stack, true, "STRING.TOKENIZE.LINES"),
        eff(1, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.deunicode",
        |vm| deunicode_word(vm, Side::Stack, "STRING.DEUNICODE"),
        eff(1, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.fuzzymatch",
        |vm| fuzzymatch(vm, Side::Stack, "STRING.FUZZYMATCH"),
        eff(2, 1),
        WordKind::Sync,
    );

    // The workbench forms of the same eight.
    r.register_native(
        "string.prefix.",
        |vm| prefix_suffix(vm, Side::Bench, false, "STRING.PREFIX."),
        wb_eff(true, Out::Stack),
        WordKind::Sync,
    );
    r.register_native(
        "string.suffix.",
        |vm| prefix_suffix(vm, Side::Bench, true, "STRING.SUFFIX."),
        wb_eff(true, Out::Stack),
        WordKind::Sync,
    );
    r.register_native(
        "string.regex.matches.",
        |vm| regex_matches(vm, Side::Bench, "STRING.REGEX.MATCHES."),
        wb_eff(true, Out::Stack),
        WordKind::Sync,
    );
    r.register_native(
        "string.regex.split.",
        |vm| regex_split(vm, Side::Bench, "STRING.REGEX.SPLIT."),
        wb_eff(true, Out::Stack),
        WordKind::Sync,
    );
    r.register_native(
        "string.tokenize.",
        |vm| tokenize(vm, Side::Bench, false, "STRING.TOKENIZE."),
        wb_eff(false, Out::Stack),
        WordKind::Sync,
    );
    r.register_native(
        "string.tokenize.lines.",
        |vm| tokenize(vm, Side::Bench, true, "STRING.TOKENIZE.LINES."),
        wb_eff(false, Out::Stack),
        WordKind::Sync,
    );
    r.register_native(
        "string.deunicode.",
        |vm| deunicode_word(vm, Side::Bench, "STRING.DEUNICODE."),
        wb_eff(false, Out::Mirror),
        WordKind::Sync,
    );
    r.register_native(
        "string.fuzzymatch.",
        |vm| fuzzymatch(vm, Side::Bench, "STRING.FUZZYMATCH."),
        wb_eff(true, Out::Mirror),
        WordKind::Sync,
    );

    r.register_native(
        "string.tokenize.unique",
        |vm| tokenize_unique(vm, Side::Stack, "STRING.TOKENIZE.UNIQUE"),
        eff(1, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.tokenize.unique.",
        |vm| tokenize_unique(vm, Side::Bench, "STRING.TOKENIZE.UNIQUE."),
        wb_eff(false, Out::Stack),
        WordKind::Sync,
    );
    #[cfg(feature = "grok")]
    {
        r.register_native(
            "string.grok",
            |vm| grok_word(vm, Side::Stack, "STRING.GROK"),
            eff(2, 1),
            WordKind::Sync,
        );
        r.register_native(
            "string.grok.",
            |vm| grok_word(vm, Side::Bench, "STRING.GROK."),
            wb_eff(true, Out::Stack),
            WordKind::Sync,
        );
    }
    r.register_native(
        "string.wrap.english",
        |vm| wrap_english(vm, Side::Stack, "STRING.WRAP.ENGLISH"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.wrap.english.",
        |vm| wrap_english(vm, Side::Bench, "STRING.WRAP.ENGLISH."),
        wb_eff(true, Out::Mirror),
        WordKind::Sync,
    );

    // The `.` siblings of words whose plain form is an F18 word in
    // `singles.rs`. Their stack guard is one, not two — see `distance_wb`.
    r.register_native(
        "string.regex.",
        |vm| pattern_match_wb(vm, false, "STRING.REGEX."),
        wb_eff(true, Out::Stack),
        WordKind::Sync,
    );
    r.register_native(
        "string.wildcard.",
        |vm| pattern_match_wb(vm, true, "STRING.WILDCARD."),
        wb_eff(true, Out::Stack),
        WordKind::Sync,
    );
    r.register_native(
        "string.expressionmatch.",
        expressionmatch_wb,
        wb_eff(true, Out::Mirror),
        WordKind::Sync,
    );

    macro_rules! dist {
        ($name:literal, $prefix:literal, $f:expr) => {
            r.register_native(
                $name,
                |vm| distance_wb(vm, $prefix, $f),
                wb_eff(true, Out::Mirror),
                WordKind::Sync,
            );
        };
    }
    dist!("string.distance.", "STRING.DISTANCE.", |a, b| Ok(
        BundValue::int(distance::levenshtein(a, b) as i64)
    ));
    dist!(
        "string.distance.levenshtein.",
        "STRING.DISTANCE.LEVENSHTEIN.",
        |a, b| Ok(BundValue::int(distance::levenshtein(a, b) as i64))
    );
    dist!(
        "string.distance.dameraulevenshtein.",
        "STRING.DISTANCE.DAMERAULEVENSHTEIN.",
        |a, b| Ok(BundValue::int(distance::damerau_levenshtein(a, b) as i64))
    );
    dist!(
        "string.distance.sift3.",
        "STRING.DISTANCE.SIFT3.",
        |a, b| Ok(BundValue::float(f64::from(distance::sift3(a, b))))
    );
    dist!(
        "string.distance.jarowinkler.",
        "STRING.DISTANCE.JAROWINKLER.",
        |a, b| Ok(BundValue::float(f64::from(
            natural::distance::jaro_winkler_distance(a, b)
        )))
    );
    // The one that can refuse: hamming is undefined for unequal lengths
    // (`distance.rs:77-82`).
    dist!(
        "string.distance.hamming.",
        "STRING.DISTANCE.HAMMING.",
        |a, b| match distance::hamming(a, b) {
            Ok(d) => Ok(BundValue::int(d as i64)),
            Err(e) => Err(Error(format!(
                "STRING.DISTANCE.HAMMING. returned hamming error: {e:?}"
            ))),
        }
    );

    // `lines` is an alias, not a word (`reference/Bund/src/stdlib/functions/create_aliases.rs:29-30`).
    r.register_alias("lines", "string.tokenize.lines");
    r.register_alias("lines.", "string.tokenize.lines.");
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm;
    use bund2_interp::Interp;

    fn run(src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    /// **F73, as an assertion.** The half of the group that answers on the
    /// stack and the half that answers on the workbench, checked by looking at
    /// where the value actually is.
    ///
    /// The differential probe cannot make this distinction on its own: it reads
    /// a result back with `println` or with `take println`, and if a word put
    /// the answer in the wrong place *and* the probe took it from the wrong
    /// place, the two mistakes cancel and the run still matches the oracle.
    #[test]
    fn the_dot_forms_answer_where_their_family_does() {
        // Answers on the stack, so the workbench is left empty.
        for src in [
            r#""hello" return "he" string.prefix."#,
            r#""hello" return "lo" string.suffix."#,
            r#""a1" return "[0-9]+" string.regex.matches."#,
            r#""a,b" return "," string.regex.split."#,
            r#""hello" return "h.*o" string.regex."#,
            r#""hello" return "h*o" string.wildcard."#,
            r#""a b" return string.tokenize."#,
            #[cfg(feature = "grok")]
            r#""a" return "%{WORD:w}" string.grok."#,
        ] {
            let i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
            assert_eq!(i.depth(), 1, "{src}: the answer belongs on the stack");
            assert_eq!(i.workbench_depth(), 0, "{src}: the workbench is spent");
        }

        // Answers on the workbench, so the stack is left empty.
        for src in [
            r#""cafe" return string.deunicode."#,
            r#""abc" return "abc" string.fuzzymatch."#,
            r#""kitten" return "sitting" string.distance."#,
            r#""kitten" return "sitting" string.distance.levenshtein."#,
            r#""ab" return "cd" string.distance.hamming."#,
            r#""a" return 5 string.wrap.english."#,
        ] {
            let i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
            assert_eq!(i.depth(), 0, "{src}: the stack operand is spent");
            assert_eq!(
                i.workbench_depth(),
                1,
                "{src}: the answer belongs on the workbench"
            );
        }
    }

    /// A `.` word's second operand comes off the **stack**, not a second
    /// workbench cell (`prefix_suffix.rs:36-39`), so a binary one guards both
    /// depths and a unary one guards only the workbench.
    #[test]
    fn a_dot_word_guards_both_depths_before_pulling() {
        // Workbench empty: refused before the stack is touched.
        for src in [
            r#""he" string.prefix."#,
            r#"string.tokenize."#,
            r#""x" string.distance."#,
        ] {
            let mut i = Interp::new();
            crate::register_all(&mut i.registry);
            let before = i.depth();
            let stream = bund2_syntax::compile(src).expect("compiles");
            let _ = i.eval(&stream);
            assert!(
                i.depth() <= before + 1,
                "{src}: nothing beyond the literals should have been pulled"
            );
            assert_eq!(i.workbench_depth(), 0, "{src}");
        }

        // Workbench full but the stack empty: a binary word still refuses, and
        // **the workbench cell survives** — the guard runs before the pull.
        //
        // One element without the `grok` feature, two with it; clippy sees the
        // default build and calls it a loop over a single element, which is
        // true and not worth restructuring around a feature gate.
        #[allow(clippy::single_element_loop)]
        for src in [
            r#""hello" return string.prefix."#,
            #[cfg(feature = "grok")]
            r#""a" return string.grok."#,
        ] {
            let mut i = Interp::new();
            crate::register_all(&mut i.registry);
            let stream = bund2_syntax::compile(src).expect("compiles");
            assert!(i.eval(&stream).is_err(), "{src} should refuse");
            assert_eq!(
                i.workbench_depth(),
                1,
                "{src}: the operand was consumed by a word that then failed"
            );
        }
    }

    /// `lines` is an alias, so it must dispatch to the same word rather than be
    /// a second registration that can drift from it
    /// (`reference/Bund/src/stdlib/functions/create_aliases.rs:29-30`).
    #[test]
    fn lines_is_an_alias_for_tokenize_lines() {
        let a = run("\"x\ny\" lines").expect("lines runs");
        let b = run("\"x\ny\" string.tokenize.lines").expect("tokenize.lines runs");
        assert_eq!(a.peek().map(|v| v.display()), b.peek().map(|v| v.display()));
        assert!(a.registry.effect_of("lines").is_some(), "alias resolves");
    }
}
