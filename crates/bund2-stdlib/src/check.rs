//! `bund2 check` — static stack-underflow detection. **RFC-0004 §S4.**
//!
//! Walks a lowered program with an abstract stack and reports each site that
//! would take more than it can have. It reports; it never changes what
//! `script` does, and a program it warns about still runs and still fails the
//! way it fails today.
//!
//! # Why the answer is a warning
//!
//! D36's ladder: nothing has stopped, so a finding is a `Warning` — one line,
//! no table, no stack dump. And RFC-0004's alternatives section rejects making
//! `check` fail a build, because `!` is `Opaque` and one of the five most-used
//! words in the corpus: a false-positive rate above zero is certain, and a
//! checker that blocks would be turned off. A warning that is right is worth
//! more than an error that is disabled.
//!
//! # What it tracks, and where it stops
//!
//! One number per program point — the depth of the current stack — plus the
//! deepest underflow seen. A literal is `+1`; a call is its declared
//! [`StackEffect`]; a block is `+1`, because lowering a `{ … }` pushes a
//! LAMBDA rather than running it.
//!
//! Four things end tracking, and they are §S6's barriers rather than
//! shortcuts:
//!
//! - an **`Opaque`** word — `!`, `apply`, `lambda*`, `graph!`;
//! - a **stack switch**, `@name`, because the depth being tracked is no longer
//!   the depth in play;
//! - an **unknown word**, which may be a lambda registered at run time;
//! - a **`CONTEXT`**, which switches for the same reason `@name` does.
//!
//! At each of those the analysis abandons what it knows and starts again from
//! an unknown depth, which is why the count of abandonments is reported
//! alongside the findings (criterion 5). A run that says "no problems" without
//! saying it skipped every `!` is misleading.

use bund2_api::{Registry, StackEffect};
use bund2_value::{BundValue, CALL, CONTEXT, LAMBDA, PTR};

/// One site that would take more than the stack can give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Index into the lowered stream, so a caller can map it to a span.
    pub at: usize,
    pub word: String,
    /// What the word needs.
    pub needs: u8,
    /// What the analysis could prove was there.
    pub have: i64,
}

/// What a walk learned.
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// Sites the analysis could reason about.
    pub analysed: usize,
    /// Sites where it gave up: an `Opaque` word, a stack switch, or a name it
    /// does not know. **Reported, never hidden** — criterion 5.
    pub abandoned: usize,
    /// Why it gave up, most common first. For the report's prose.
    pub reasons: Vec<(String, usize)>,
}

/// Walk a lowered stream and report what it would underflow on.
///
/// `depth_in` is what the stack holds when the stream starts — 0 for a whole
/// program.
/// Bind the words a program registers with a **literal** body, before walking.
///
/// **Q23, answered.** `check` runs on a fresh registry, so without this it
/// abandons at every word the program defines — which is most of what a real
/// program calls. The corpus idiom is `:Name { … } register`, three adjacent
/// values in the lowered stream, and binding those is reading the program
/// rather than guessing about it.
///
/// It deliberately handles **only** the literal form. `<computed> <lambda>
/// register` is D16's open world and stays unknown; so does a body built at
/// run time. Recognising a shape that is actually there is not the same as
/// assuming one that might be.
fn prebind(stream: &[BundValue], registry: &Registry) -> Registry {
    let mut r = registry.clone();
    for w in stream.windows(3) {
        let (name, body, call) = (&w[0], &w[1], &w[2]);
        if call.dt() != CALL || call.as_str().as_deref() != Some("register") {
            continue;
        }
        if body.dt() != LAMBDA {
            continue;
        }
        let (Some(n), Some(_)) = (name.as_str(), body.as_lambda()) else {
            continue;
        };
        if name.dt() == bund2_value::STRING {
            r.register_lambda(&n, body.clone());
        }
    }
    r
}

pub fn check(stream: &[BundValue], registry: &Registry, depth_in: i64) -> Report {
    let registry = &prebind(stream, registry);
    let mut r = Report::default();
    let mut depth = Some(depth_in);
    let mut why: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    for (i, v) in stream.iter().enumerate() {
        match v.dt() {
            // `@name` and a CONTEXT both change which stack the count is
            // about, so the count stops meaning anything.
            CONTEXT => {
                if depth.is_some() {
                    r.abandoned += 1;
                    *why.entry("a stack switch".into()).or_default() += 1;
                }
                depth = None;
            }
            CALL => {
                let Some(name) = v.as_str() else {
                    depth = None;
                    continue;
                };
                let Some(eff) = effect_of(registry, &name) else {
                    // An unknown name may be a lambda registered at run time —
                    // D16's open world — so this is "cannot tell", not "wrong".
                    if depth.is_some() {
                        r.abandoned += 1;
                        // **Name it.** "a word with no declared effect" is
                        // true and useless: the reader cannot tell an
                        // unimplemented word from a lambda the program
                        // registered, and those want opposite responses —
                        // implement the first, infer the second.
                        *why
                            .entry(format!("`{name}`, which declares no effect"))
                            .or_default() += 1;
                    }
                    depth = None;
                    continue;
                };
                if eff.opaque {
                    if depth.is_some() {
                        r.abandoned += 1;
                        *why.entry(format!("`{name}`, whose effect is not a fixed pair"))
                            .or_default() += 1;
                    }
                    depth = None;
                    continue;
                }
                if let Some(d) = depth {
                    r.analysed += 1;
                    if d < i64::from(eff.consumes) {
                        r.findings.push(Finding {
                            at: i,
                            word: name.to_string(),
                            needs: eff.consumes,
                            have: d,
                        });
                        // Continue from what the word would leave, so one
                        // underflow does not cascade into a report per term.
                        depth = Some(i64::from(eff.produces));
                    } else {
                        depth = Some(d - i64::from(eff.consumes) + i64::from(eff.produces));
                    }
                }
            }
            // Everything else is a value the stream pushes: a literal, a
            // string, a PTR, a lowered `{ … }` block.
            LAMBDA | PTR => depth = depth.map(|d| d + 1),
            _ => depth = depth.map(|d| d + 1),
        }
    }
    let mut reasons: Vec<(String, usize)> = why.into_iter().collect();
    reasons.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    r.reasons = reasons;
    r
}

/// The effect of a name: **declared** for a native, **inferred** for a Bund
/// word. RFC-0004 §S3.
///
/// A native's is written at its registration site. A lambda's is composed from
/// the effects of the words in its body, which is what §S3 means by inference
/// and what makes `check` able to reason past a program's own vocabulary
/// instead of stopping at it.
fn effect_of(registry: &Registry, name: &str) -> Option<StackEffect> {
    if let Some(e) = registry.effect_of(name) {
        return Some(e);
    }
    let body = registry.lambda_body(name)?;
    Some(infer(&body, registry, 0))
}

/// How deep recursion may go before inference gives up.
///
/// A self-recursive word would otherwise infer itself forever. Bund2 does not
/// hang (D39), so this is a bound rather than a guard: past it the answer is
/// `Opaque`, which is the honest reading of "I could not work it out".
const MAX_INFER_DEPTH: u8 = 8;

/// Compose a body's terms into one effect. **RFC-0004 §S3.**
///
/// The abstract machine is `check`'s, run relative to an unknown entry depth:
/// the simulated depth is allowed to go negative, and how far it goes is what
/// the body **needs**. So
///
/// - `consumes` is the deepest the walk reached below its start, and
/// - `produces` is that plus the net change.
///
/// `{ + }` needs 2 and leaves 1 — the walk dips to −2 and ends at −1.
/// `{ 1 2 + }` needs nothing and leaves 1.
///
/// **Composition saturates.** Anything composed with `Opaque` is `Opaque`,
/// because a term whose effect is unknown makes every depth after it unknown
/// too. That is not a shortcut: it is why a body containing `!` cannot be
/// given a number, and `!` is one of the five most-used words in the corpus.
pub fn infer(body: &[BundValue], registry: &Registry, depth: u8) -> StackEffect {
    if depth >= MAX_INFER_DEPTH {
        return StackEffect::opaque(0);
    }
    let mut net: i64 = 0;
    let mut low: i64 = 0;
    for v in body {
        match v.dt() {
            CONTEXT => return StackEffect::opaque(0),
            CALL => {
                let Some(name) = v.as_str() else {
                    return StackEffect::opaque(0);
                };
                let eff = match registry.effect_of(&name) {
                    Some(e) => e,
                    None => match registry.lambda_body(&name) {
                        Some(inner) => infer(&inner, registry, depth + 1),
                        None => return StackEffect::opaque(0),
                    },
                };
                if eff.opaque {
                    return StackEffect::opaque(0);
                }
                net -= i64::from(eff.consumes);
                low = low.min(net);
                net += i64::from(eff.produces);
            }
            _ => net += 1,
        }
    }
    let consumes = (-low).clamp(0, i64::from(u8::MAX)) as u8;
    let produces = (i64::from(consumes) + net).clamp(0, i64::from(u8::MAX)) as u8;
    StackEffect::fixed(consumes, produces)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::Interp;

    fn reg() -> bund2_api::Registry {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i.registry
    }

    fn eff_of(src: &str) -> StackEffect {
        let r = reg();
        let body = bund2_syntax::compile(src).expect("compiles");
        // `compile` appends EXIT; inference ignores it as a pushed value, so
        // trim it rather than let it inflate `produces`.
        let body: Vec<_> = body
            .into_iter()
            .take_while(|v| v.dt() != bund2_value::EXIT)
            .collect();
        infer(&body, &r, 0)
    }

    /// **RFC-0004 §S3.** `consumes` is how far below its entry depth the body
    /// reaches; `produces` is that plus the net.
    #[test]
    fn composition_counts_the_dip_not_the_terms() {
        let e = eff_of("+");
        assert_eq!((e.consumes, e.produces, e.opaque), (2, 1, false));

        let e = eff_of("1");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));

        let e = eff_of("drop");
        assert_eq!((e.consumes, e.produces, e.opaque), (1, 0, false));

        // Needs nothing: it supplies its own operands.
        let e = eff_of("1 2 +");
        assert_eq!((e.consumes, e.produces, e.opaque), (0, 1, false));

        // Two additions in a row need **three**, not four: the first leaves a
        // value the second consumes. Counting terms instead of tracking the
        // dip would say four.
        let e = eff_of("+ +");
        assert_eq!((e.consumes, e.produces, e.opaque), (3, 1, false));
    }

    /// Composition saturates: one unknown term makes the whole unknown,
    /// because every depth after it is unknown too.
    #[test]
    fn opaque_saturates() {
        assert!(eff_of("!").opaque, "`!` dispatches on eight tags");
        assert!(eff_of("1 2 + !").opaque, "an opaque term poisons the rest");
        assert!(eff_of("{ drop } loop").opaque, "`loop` runs a body");
        assert!(
            eff_of("nosuchwordanywhere").opaque,
            "an unknown name may be bound at run time — D16"
        );
    }
}
