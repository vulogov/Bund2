//! RFC-0014 M9's criteria: `smysl.prove` (§S7) and §S7.1's rendering,
//! driven through the word, with each library read back through SMYSL's
//! API as the second opinion.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use bund2_api::Vm as _;
use bund2_value::BundValue;
use smysl_core::{
    GranularityProfile, ProfileEstimator, Record, RelKind, Status, TokenEstimator, Uid, View,
    ViewId,
};

use super::forming_tests::{
    Dir, Run, add_text, add_unit, appended, citing, handle, items, list, m, raw, rel, rid, s, text,
    uid, unit, withdrawal,
};
use super::prove::{fit, render};
use super::{Code, Refusal};

// --- the harness -----------------------------------------------------------------

/// `smysl.prove` in `r`: the handle, the KB `kb` leaves, the goal and
/// `opts`. The handle is the variable `h` and the options `opts`, so `kb`
/// can name the library too.
fn prove_in(
    r: &mut Run,
    d: &Dir,
    kb: &str,
    goal: &str,
    opts: BundValue,
) -> Result<Vec<BundValue>, String> {
    r.i.register_var("h", handle(d));
    r.i.register_var("opts", opts);
    r.call(
        vec![],
        &format!(":h var? {kb} {goal} :opts var? smysl.prove"),
    )
    .map(|v| items(&v))
}

/// `smysl.prove` in a fresh VM: its roots and its warnings.
fn prove(d: &Dir, kb: &str, goal: &str) -> (Result<Vec<BundValue>, String>, Vec<String>) {
    let mut r = Run::new(false);
    let out = prove_in(&mut r, d, kb, goal, m(&[]));
    (out, r.warnings())
}

/// The one root `goal` proves to.
fn root(d: &Dir, kb: &str, goal: &str) -> String {
    let (out, _) = prove(d, kb, goal);
    let roots = out.unwrap_or_else(|e| panic!("{goal}: {e}"));
    assert_eq!(roots.len(), 1, "{goal}: {roots:?}");
    text(&roots[0])
}

/// The library's KB, `smysl.kb` over every unit, with `rules` after it.
fn with_library(rules: &str) -> String {
    format!(":h var? \"from library\" smysl.kb [ {rules} ] kb! kb.merge")
}

/// A unit as the library holds it.
fn core(d: &Dir, u: &str) -> smysl_core::UnitCore {
    d.open().store().get(&uid(u)).unwrap().core.clone()
}

/// Every file under `p` and its length: the logs, the catalog and the
/// marker, so an unchanged map means nothing was written.
fn lengths(p: &Path) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    let mut todo = vec![p.to_path_buf()];
    while let Some(dir) = todo.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let e = e.unwrap();
            let md = e.metadata().unwrap();
            if md.is_dir() {
                todo.push(e.path());
            } else {
                out.insert(e.path().display().to_string(), md.len());
            }
        }
    }
    out
}

/// The view `id`, of `profile`, written through SMYSL's API.
fn view(d: &Dir, id: &str, profile: GranularityProfile) {
    let mut v = View::new(ViewId::new(id).unwrap(), "proofs");
    v.granularity = profile;
    raw(d, &[Record::View(v)]);
}

fn bounded(l0_max: u32) -> GranularityProfile {
    let mut p = GranularityProfile::standard();
    p.l0_max = l0_max;
    p
}

/// `smysl.check`'s diagnostics, each `(code, severity)`.
fn checked(d: &Dir) -> Vec<(String, String)> {
    let (out, _) = super::forming_tests::call(vec![handle(d)], "smysl.check");
    items(&out.unwrap())
        .iter()
        .map(|x| {
            let x = x.as_map().unwrap();
            (text(&x["code"]), text(&x["severity"]))
        })
        .collect()
}

// --- criterion 13: rendering is stable --------------------------------------------

fn call_(n: &str) -> BundValue {
    BundValue::call(n)
}

/// Criterion 13: each row of §S7.1 and D246's LAMBDA row.
#[test]
fn each_row_of_the_rendering_is_fixed() {
    let r = |v: &BundValue| render(v, false).unwrap();
    assert_eq!(r(&call_("parent")), "parent");
    assert_eq!(r(&s("tom")), "tom");
    assert_eq!(r(&s("a.b_C9")), "a.b_C9");
    assert_eq!(r(&s("Tom")), "\"Tom\"");
    assert_eq!(r(&s("two words")), "\"two words\"");
    assert_eq!(r(&s("")), "\"\"");
    assert_eq!(r(&s("q\"b\\n\nt\tc\u{1}")), "\"q\\\"b\\\\n\\nt\\tc\\u{1}\"");
    assert_eq!(r(&BundValue::boolean(true)), "true");
    assert_eq!(r(&BundValue::boolean(false)), "false");
    assert_eq!(r(&BundValue::int(-42)), "-42");
    assert_eq!(r(&BundValue::float(1.5)), "1.5");
    assert_eq!(r(&BundValue::float(2.0)), "2.0");
    assert_eq!(r(&BundValue::float(0.1)), "0.1");
    assert_eq!(r(&BundValue::float(1e21)), "1000000000000000000000.0");
    assert_eq!(r(&BundValue::float(f64::NAN)), "nan");
    assert_eq!(r(&BundValue::float(f64::INFINITY)), "inf");
    assert_eq!(r(&BundValue::float(f64::NEG_INFINITY)), "-inf");
    assert_eq!(r(&BundValue::nodata()), "_");
    assert_eq!(
        r(&list(vec![call_("parent"), s("tom"), BundValue::nodata()])),
        "[ parent tom _ ]"
    );
    assert_eq!(r(&list(vec![])), "[ ]");
    // A partial list, and an improper one.
    assert_eq!(
        r(&list(vec![s("a"), s("b"), call_("|"), BundValue::nodata()])),
        "[ a b | _ ]"
    );
    assert_eq!(r(&list(vec![s("a"), call_("|"), s("b")])), "[ a | b ]");
    assert_eq!(
        r(&list(vec![s("x"), list(vec![s("y"), list(vec![])])])),
        "[ x [ y [ ] ] ]"
    );
    // D246 R26: a LAMBDA as written.
    assert_eq!(
        r(&BundValue::lambda(vec![
            call_("A"),
            BundValue::int(18),
            call_("<=")
        ])),
        "{ A 18 <= }"
    );
    assert_eq!(r(&BundValue::lambda(vec![])), "{ }");
}

/// Criterion 13: a uid is cut to 12 characters in a gist and kept whole
/// in a detail; a STRING `Uid::parse` refuses is rendered whole.
#[test]
fn a_uid_is_cut_only_in_a_gist() {
    let core = smysl_core::UnitCoreBuilder::new(
        smysl_core::KernelType::Claim,
        "Any unit",
        Status::Speculative,
    )
    .build()
    .unwrap();
    let u = super::map::uid(&smysl_core::canonical_uid(&core));
    let whole = text(&u);
    assert_eq!(
        render(&u, true).unwrap(),
        format!("\"{}\"", whole.chars().take(12).collect::<String>())
    );
    assert_eq!(render(&u, false).unwrap(), format!("\"{whole}\""));
    assert_eq!(render(&s("b3:x"), true).unwrap(), "\"b3:x\"");
}

/// Criterion 13: an opaque value is SM03.
#[test]
fn an_opaque_value_is_sm03() {
    let v = list(vec![call_("p"), m(&[("k", s("v"))])]);
    assert_eq!(
        render(&v, false).unwrap_err(),
        Refusal::new(Code::Sm03, "the goal holds a Map, which has no stable text")
    );
}

/// §S7: a rendering that fits every profile is the gist; one that does
/// not is cut at the shortest prefix, followed by `…`; a profile with
/// room for nothing is SM12.
#[test]
fn a_gist_is_fitted_to_every_profile() {
    let wide = bounded(30);
    let narrow = bounded(5);
    assert_eq!(
        fit("short", std::slice::from_ref(&wide)).ok().unwrap(),
        "short"
    );
    let long = "x".repeat(200);
    let one = fit(&long, std::slice::from_ref(&wide)).ok().unwrap();
    let both = fit(&long, &[wide.clone(), narrow.clone()]).ok().unwrap();
    assert!(one.ends_with('…') && both.ends_with('…'));
    assert!(both.len() < one.len());
    for p in [&wide, &narrow] {
        assert!(p.gist_within_bound(p.tokens(&both).unwrap()));
    }
    match fit(&long, &[bounded(0)]) {
        Err(super::prove::Failed::Refused(r)) => assert_eq!(
            r,
            Refusal::new(Code::Sm12, "the library's view allows no gist of 0 tokens")
        ),
        _ => panic!("l0_max 0 must be SM12"),
    }
}

// --- criterion 12: exported proofs keep rule M -------------------------------------

/// The fixture's units: a cited observation, an inferred finding resting
/// on it, a rebuttal of the observation, and a speculative claim. It keeps
/// rule M itself, so any `SMY-E030` is what a proof wrote.
struct Fixture {
    d: Dir,
    claim: String,
    finding: String,
    rebuttal: String,
    cited: String,
}

fn fixture(d: Dir) -> Fixture {
    let added = add_text(
        &d,
        "survey",
        "The survey found the well dry.\n\nIt was May.",
    );
    let cited = add_unit(&d, citing("The survey saw it dry", &added));
    let claim = add_unit(&d, unit("The well is dry"));
    let finding = add_unit(
        &d,
        m(&[
            ("schema", s("finding")),
            ("gist", s("The village must move")),
            ("status", s("inferred")),
            ("grounds", list(vec![s(&cited)])),
        ]),
    );
    let rebuttal = add_unit(&d, unit("The well has water"));
    appended(&d, vec![rel("rebuts", &rebuttal, &cited)]).unwrap();
    Fixture {
        d,
        claim,
        finding,
        rebuttal,
        cited,
    }
}

/// The rules every proof below is drawn from, between them every node
/// kind of §S7's table.
const RULES: &str = r#"
    [ [ base X ] ← [ unit X claim speculative ] [ gist X G ] ]
    [ [ firm X ] ← [ unit X finding inferred ] ]
    [ [ sure X ] ← [ unit X observation cited ] ]
    [ [ shaky F ] ← [ unit F finding _ ] [ grounds F C ] [ rel rebuts _ C ] ]
    [ [ linked A B ] ← [ rel rebuts A B ] ]
    [ [ ranked S ] ← [ rank S 3 ] ]
    [ tag alpha ]
    [ tag beta ]
    [ [ once1 X ] ← [ once [ tag X ] ] ]
    [ [ via X ] ← [ call [ tag X ] ] ]
    [ [ either X ] ← [ or [ tag X ] [ tag beta ] ] ]
    [ [ pick X ] ← [ if [ tag alpha ] [ = X alpha ] [ = X beta ] ] ]
    [ [ other X ] ← [ if [ tag gamma ] [ = X 1 ] [ unit X observation cited ] ] ]
    [ [ all L ] ← [ findall X [ tag X ] L ] ]
    [ [ none L ] ← [ findall X fail L ] ]
    [ [ absent X ] ← [ not [ tag gamma ] ] [ unit X observation cited ] ]
    [ [ five X N ] ← [ unit X observation cited ] [ is N { 2 3 + } ] ]
    [ [ path X Y ] ← [ edge X Y ] ]
    [ [ path X Y ] ← [ path X Z ] [ edge Z Y ] ]
    [ edge a b ]
    [ edge b c ]
    [ [ reach X Y ] ← [ unit X observation cited ] [ path a Y ] ]
"#;

/// The goals, each with the status §S7 computes for its root. Each has one
/// answer, whatever earlier proofs wrote into the library.
fn goals(f: &Fixture) -> Vec<(String, Status)> {
    let at = |g: &str, u: &str| g.replace('U', &format!("\"{u}\""));
    vec![
        // A fact standing for a speculative unit.
        (at("[ base U ]", &f.claim), Status::Speculative),
        (at("[ firm U ]", &f.finding), Status::Inferred),
        // Under `derived`: the lower of it and `cited`.
        (at("[ sure U ]", &f.cited), Status::Derived),
        (at("[ shaky U ]", &f.finding), Status::Speculative),
        // A goal with two uid arguments.
        ("[ linked A B ]".into(), Status::Speculative),
        // A `rank` fact stands for nothing: no grounds.
        ("[ ranked S ]".into(), Status::Speculative),
        ("[ once1 X ]".into(), Status::Speculative),
        ("[ via beta ]".into(), Status::Speculative),
        // `:branch 2`.
        ("[ either gamma ]".into(), Status::Speculative),
        ("[ pick X ]".into(), Status::Speculative),
        // `:else` caps a `derived` conclusion at `inferred`.
        ("[ other X ]".into(), Status::Inferred),
        ("[ all L ]".into(), Status::Speculative),
        // An empty `:findall` has no grounds.
        ("[ none L ]".into(), Status::Speculative),
        // `:no-proof` caps.
        ("[ absent X ]".into(), Status::Inferred),
        // `:bridge` and `:builtin` give no ground.
        ("[ five X N ]".into(), Status::Derived),
        // `:tabled` caps.
        ("[ reach X c ]".into(), Status::Inferred),
    ]
}

/// A long goal holding a multi-byte character near the cut, and a non-NFC
/// one holding U+0958, which NFC lengthens.
fn long_goals() -> [String; 2] {
    [
        format!("[ said \"{}\" ]", "é".repeat(90)),
        format!("[ said \"{}\" ]", "\u{958}".repeat(60)),
    ]
}

/// The library's facts, [`RULES`], the long goals as program facts, and
/// `path/2` tabled.
fn kb() -> String {
    let [a, b] = long_goals();
    format!(
        "{} :path 2 kb.table",
        with_library(&format!("{RULES} {a} {b}"))
    )
}

/// Prove every goal into `f`'s library, and check what criterion 12 asks
/// of each, against the profiles `profiles`.
fn proved_keeping_rule_m(f: &Fixture, profiles: &[GranularityProfile]) {
    let before = attestations(&f.d);
    let kb = kb();
    let goals = goals(f);
    let mut roots = Vec::new();
    for (goal, want) in &goals {
        let u = root(&f.d, &kb, goal);
        assert_eq!(core(&f.d, &u).status, *want, "{goal}");
        roots.push(u);
    }
    for goal in long_goals() {
        let u = root(&f.d, &kb, &goal);
        let c = core(&f.d, &u);
        assert!(c.gist.ends_with('…'), "{goal}: {}", c.gist);
        assert!(unicode_normalization::is_nfc(&c.gist));
        for p in profiles {
            assert!(
                p.gist_within_bound(p.tokens(&c.gist).unwrap()),
                "{}",
                c.gist
            );
        }
        // A program fact: its detail is the whole goal, normalised.
        assert_eq!(c.body.as_deref(), Some("program fact"));
    }

    // SMYSL accepted every unit; the library is clean of rule M's and the
    // gist bound's errors.
    let diags = checked(&f.d);
    assert!(
        !diags
            .iter()
            .any(|(c, _)| c == "SMY-E030" || c == "SMY-E022"),
        "{diags:?}"
    );
    assert!(!diags.iter().any(|(_, sev)| sev == "error"), "{diags:?}");

    // A corpus fact is grounded on by its uid, and not copied.
    let base = core(&f.d, &roots[0]);
    assert_eq!(base.grounds, BTreeSet::from([uid(&f.claim)]));
    assert_eq!(base.schema.to_string(), "finding");
    let firm = core(&f.d, &roots[1]);
    assert_eq!(firm.grounds, BTreeSet::from([uid(&f.finding)]));
    let sure = core(&f.d, &roots[2]);
    assert_eq!(sure.grounds, BTreeSet::from([uid(&f.cited)]));
    let linked = core(&f.d, &roots[4]);
    assert_eq!(
        linked.grounds,
        BTreeSet::from([uid(&f.rebuttal), uid(&f.cited)])
    );
    let short = |u: &str| u.chars().take(12).collect::<String>();
    assert!(linked.gist.contains(&short(&f.cited)), "{}", linked.gist);
    assert!(!linked.gist.contains(&f.cited), "{}", linked.gist);
    assert!(linked.detail.as_deref().unwrap_or("").contains(&f.cited));
    let lib = f.d.open();
    for (_, unit) in lib.store().units() {
        if unit.core.body.as_deref() == Some("program fact") {
            let d = unit.core.detail.clone().unwrap_or_default();
            for shape in ["[ unit ", "[ gist ", "[ grounds ", "[ rel ", "[ rank "] {
                assert!(!d.starts_with(shape), "copied: {d}");
            }
        }
    }

    // Each conclusion's attestation names its grounds as parents, and
    // every new clock is later than the agent's previous one.
    let prev = before.values().flatten().max().cloned();
    let after = attestations(&f.d);
    for (u, unit) in lib.store().units() {
        let Some(body) = unit.core.body.as_deref() else {
            continue;
        };
        if !body.starts_with("proved by") {
            continue;
        }
        let ours: Vec<_> = lib
            .store()
            .attestations_of(u)
            .iter()
            .filter(|a| a.agent.as_str() == "tool:bund2")
            .cloned()
            .collect();
        assert_eq!(ours.len(), 1, "{body}");
        let grounds: BTreeSet<Uid> = unit.core.grounds.iter().copied().collect();
        assert_eq!(ours[0].parents, grounds, "{body}");
    }
    for (u, clocks) in &after {
        if before.contains_key(u) {
            continue;
        }
        for c in clocks {
            assert!(prev.as_ref().is_none_or(|p| c > p));
        }
    }

    // Re-proving writes nothing.
    let lens = lengths(&f.d.0);
    for (goal, _) in &goals {
        root(&f.d, &kb, goal);
    }
    for goal in long_goals() {
        root(&f.d, &kb, &goal);
    }
    assert_eq!(lengths(&f.d.0), lens);
}

/// `tool:bund2`'s attestation clocks, per uid.
fn attestations(d: &Dir) -> BTreeMap<Uid, Vec<smysl_core::Hlc>> {
    let lib = d.open();
    let mut out: BTreeMap<Uid, Vec<smysl_core::Hlc>> = BTreeMap::new();
    for r in lib.store().iter() {
        if let Record::Attestation(a) = r
            && a.agent.as_str() == "tool:bund2"
        {
            out.entry(a.uid).or_default().push(a.ts.clone());
        }
    }
    out
}

/// Criterion 12, the library with no view: the `default` preset bounds
/// every gist.
#[test]
fn proofs_keep_rule_m_in_a_library_with_no_view() {
    let f = fixture(Dir::library("prove-none"));
    proved_keeping_rule_m(&f, &[GranularityProfile::standard()]);
}

/// Criterion 12, a view setting `l0_max: 20`.
#[test]
fn proofs_keep_rule_m_under_a_view_of_20_tokens() {
    let d = Dir::library("prove-20");
    view(&d, "v/narrow", bounded(20));
    let f = fixture(d);
    proved_keeping_rule_m(&f, &[bounded(20)]);
}

/// Criterion 12, two views of different bounds: every gist fits both.
#[test]
fn proofs_keep_rule_m_under_two_views() {
    let d = Dir::library("prove-two");
    view(&d, "v/narrow", bounded(12));
    view(&d, "v/wide", bounded(40));
    let f = fixture(d);
    proved_keeping_rule_m(&f, &[bounded(12), bounded(40)]);
}

/// Criterion 12, a view whose profile names `smysl/content/1`, with a goal
/// of CJK atoms.
#[test]
fn proofs_keep_rule_m_under_the_content_estimator() {
    let d = Dir::library("prove-cjk");
    let mut p = bounded(20);
    p.estimator = ProfileEstimator::Known(TokenEstimator::Content1);
    view(&d, "v/cjk", p.clone());
    let f = fixture(d);
    proved_keeping_rule_m(&f, std::slice::from_ref(&p));
    let goal = format!("[ said \"{}\" ]", "数据".repeat(40));
    let kb = with_library(&format!("[ said \"{}\" ]", "数据".repeat(40)));
    let c = core(&f.d, &root(&f.d, &kb, &goal));
    assert!(c.gist.ends_with('…'));
    assert!(p.gist_within_bound(p.tokens(&c.gist).unwrap()));
    // The cut is the content estimator's, not the bytes': CJK weighs 0.931
    // tokens a character under it and 0.75 by bytes.
    let r = format!("[ said \"{}\" ]", "数据".repeat(40));
    let at = |p: &GranularityProfile| format!("{}…", &r[..p.fit_gist_with(&r, "…").unwrap()]);
    assert_eq!(c.gist, at(&p));
    assert_ne!(c.gist, at(&bounded(20)));
}

/// Criterion 12's fifth library and SM12: a view with `l0_max: 0` leaves
/// room for no gist, and nothing is written.
#[test]
fn a_view_of_no_tokens_is_sm12_and_writes_nothing() {
    let d = Dir::library("prove-zero");
    view(&d, "v/none", bounded(0));
    let f = fixture(d);
    let lens = lengths(&f.d.0);
    let (out, _) = prove(&f.d, &kb(), "[ ranked S ]");
    assert_eq!(
        out.unwrap_err(),
        "SMYSL.PROVE: the library's view allows no gist of 0 tokens"
    );
    assert_eq!(lengths(&f.d.0), lens);
    assert!(f.d.no_lock());
}

/// Criterion 12: an askable goal is E50, with this word's prefix.
#[test]
fn an_askable_goal_is_e50() {
    let f = fixture(Dir::library("prove-ask"));
    let kb = format!(
        "{} :tag 1 kb.askable",
        with_library("[ [ t X ] ← [ tag X ] ]")
    );
    let (out, _) = prove(&f.d, &kb, "[ t alpha ]");
    assert_eq!(
        out.unwrap_err(),
        "SMYSL.PROVE: tag/1 is askable, and this query has no ask lambda"
    );
}

/// D246 R27: a root that becomes no single unit answers NODATA; a root
/// fact standing for one unit answers that unit.
#[test]
fn a_root_of_no_single_unit_answers_nodata() {
    let f = fixture(Dir::library("prove-roots"));
    let kb = ":h var? \"from library\" smysl.kb";
    let (out, _) = prove(&f.d, kb, "[ rank speculative N ]");
    assert_eq!(out.unwrap(), [BundValue::nodata()]);
    let (out, _) = prove(&f.d, kb, "[ rel rebuts A B ]");
    assert_eq!(out.unwrap(), [BundValue::nodata()]);
    let (out, _) = prove(&f.d, kb, "[ = 1 1 ]");
    assert_eq!(out.unwrap(), [BundValue::nodata()]);
    let (out, _) = prove(&f.d, kb, "[ unit X claim speculative ]");
    let mut got: Vec<String> = out.unwrap().iter().map(text).collect();
    got.sort();
    let mut want = vec![f.claim.clone(), f.rebuttal.clone()];
    want.sort();
    assert_eq!(got, want);
}

// --- criterion 18: what a fact stands for ------------------------------------------

/// The root's grounds, and which of them are program facts.
fn grounds_of(d: &Dir, root: &str) -> Vec<(Uid, bool)> {
    core(d, root)
        .grounds
        .iter()
        .map(|g| {
            let c = core(d, &g.canonical());
            (*g, c.body.as_deref() == Some("program fact"))
        })
        .collect()
}

/// Criterion 18: a `rel` fact from a hand-built KB, over an edge the
/// library has withdrawn, is a program fact.
#[test]
fn a_withdrawn_rel_is_a_program_fact() {
    let f = fixture(Dir::library("prove-withdrawn"));
    raw(
        &f.d,
        &[Record::Withdrawal(withdrawal(
            rid(RelKind::Rebuts, &f.rebuttal, &f.cited),
            "human:ann",
            "human:ann",
        ))],
    );
    let kb = format!(
        "[ [ rel rebuts \"{}\" \"{}\" ] [ [ hit X ] ← [ rel rebuts X \"{}\" ] ] ] kb!",
        f.rebuttal, f.cited, f.cited
    );
    let r = root(&f.d, &kb, "[ hit X ]");
    let g = grounds_of(&f.d, &r);
    assert_eq!(g.len(), 1);
    assert!(g[0].1, "the withdrawn edge's fact is a program fact");
}

/// Criterion 18: hand-built `grounds`, `label`, `effective` and `unit`
/// facts, each against what the library holds.
#[test]
fn hand_built_facts_stand_for_units_only_when_borne_out() {
    let f = fixture(Dir::library("prove-hand"));
    super::forming_tests::call(vec![handle(&f.d), s("c/well"), s(&f.claim)], "smysl.label")
        .0
        .unwrap();
    let one = |fact: &str| {
        let kb = format!("[ {fact} [ [ q ] ← {fact} ] ] kb!");
        grounds_of(&f.d, &root(&f.d, &kb, "[ q ]"))
    };
    // The finding has no ground `rebuttal`: a program fact.
    let g = one(&format!("[ grounds \"{}\" \"{}\" ]", f.finding, f.rebuttal));
    assert!(g.len() == 1 && g[0].1);
    // It does have the observation: it stands for the finding.
    let g = one(&format!("[ grounds \"{}\" \"{}\" ]", f.finding, f.cited));
    assert_eq!(g, [(uid(&f.finding), false)]);
    // The library binds the label to the claim: nothing written for it.
    let g = one(&format!("[ label \"c/well\" \"{}\" ]", f.claim));
    assert_eq!(g, [(uid(&f.claim), false)]);
    // No retraction touches the claim.
    let g = one(&format!("[ effective \"{}\" unfounded ]", f.claim));
    assert!(g.len() == 1 && g[0].1);
    // The claim is speculative, not cited.
    let g = one(&format!("[ unit \"{}\" claim cited ]", f.claim));
    assert!(g.len() == 1 && g[0].1);
    let g = one(&format!("[ unit \"{}\" claim speculative ]", f.claim));
    assert_eq!(g, [(uid(&f.claim), false)]);
}

/// Criterion 18: `[ grounds U G ]` from `smysl.kb` stands for `U`, whether
/// `G` is superseded and held, or not held at all.
#[test]
fn a_kb_grounds_fact_stands_for_its_unit_whatever_its_ground() {
    use smysl_core::{KernelType, UnitCoreBuilder, canonical_uid};
    let d = Dir::library("prove-grounds");
    let old = add_unit(&d, unit("The first account"));
    let new = add_unit(&d, unit("The second account"));
    appended(&d, vec![rel("supersedes", &new, &old)]).unwrap();
    let rests = add_unit(
        &d,
        m(&[
            ("schema", s("claim")),
            ("gist", s("Rests on the first")),
            ("status", s("inferred")),
            ("grounds", list(vec![s(&old)])),
        ]),
    );
    let absent = UnitCoreBuilder::new(KernelType::Claim, "Never written", Status::Speculative)
        .build()
        .unwrap();
    let orphan = UnitCoreBuilder::new(KernelType::Claim, "Rests on nothing held", Status::Inferred)
        .grounds([canonical_uid(&absent)])
        .build()
        .unwrap();
    let orphan_uid = canonical_uid(&orphan).canonical();
    raw(&d, &[Record::Unit(orphan)]);
    let kb = with_library("[ [ q U G ] ← [ grounds U G ] ]");
    let (out, _) = prove(&d, &kb, "[ q U G ]");
    let roots: Vec<String> = out.unwrap().iter().map(text).collect();
    let mut seen = BTreeSet::new();
    for r in &roots {
        for (g, program) in grounds_of(&d, r) {
            assert!(!program, "a held unit's grounds fact is no program fact");
            seen.insert(g.canonical());
        }
    }
    assert!(seen.contains(&rests), "{seen:?}");
    assert!(seen.contains(&orphan_uid), "{seen:?}");
    // No scope of current versions selects the superseded ground.
    let _ = new;
}

// --- the options, the warning, `bund2 check` and `--noio` ----------------------------

/// `opts`: `cited` with a source writes cited program facts; every other
/// status, and a malformed MAP, is refused before anything is solved.
#[test]
fn opts_name_the_status_of_program_facts() {
    let f = fixture(Dir::library("prove-opts"));
    let kb = "[ [ tag alpha ] [ [ q X ] ← [ tag X ] ] ] kb!";
    let source = m(&[
        ("kind", s("url")),
        ("reference", s("https://example.org/survey")),
    ]);
    let mut r = Run::new(false);
    let roots = prove_in(
        &mut r,
        &f.d,
        kb,
        "[ q X ]",
        m(&[("facts", s("cited")), ("source", source)]),
    )
    .unwrap();
    let g = grounds_of(&f.d, &text(&roots[0]));
    assert_eq!(core(&f.d, &g[0].0.canonical()).status, Status::Cited);
    // Under `derived`, the lower of it and `cited`.
    assert_eq!(core(&f.d, &text(&roots[0])).status, Status::Derived);

    let refused = |opts: BundValue| {
        let mut r = Run::new(false);
        prove_in(&mut r, &f.d, kb, "[ q X ]", opts).unwrap_err()
    };
    assert_eq!(
        refused(m(&[("facts", s("cited"))])),
        "SMYSL.PROVE: spec lacks source, which facts cited needs"
    );
    assert_eq!(
        refused(m(&[("facts", s("derived"))])),
        "SMYSL.PROVE: status derived needs grounds, which a program fact has none of"
    );
    assert_eq!(
        refused(m(&[("facts", s("inferred"))])),
        "SMYSL.PROVE: status inferred needs grounds, which a program fact has none of"
    );
    assert_eq!(
        refused(m(&[("facts", s("measured"))])),
        "SMYSL.PROVE: status measured cannot be authored"
    );
    // D246 R28.
    assert_eq!(
        refused(m(&[("facts", s("unfounded"))])),
        "SMYSL.PROVE: status unfounded cannot be authored"
    );
    assert_eq!(
        refused(m(&[("facts", s("doubtful"))])),
        "SMYSL.PROVE: unknown status: doubtful"
    );
    assert_eq!(
        refused(m(&[("budget", BundValue::int(1))])),
        "SMYSL.PROVE: spec has no key budget"
    );
    assert_eq!(
        refused(BundValue::int(1)),
        "SMYSL.PROVE: #1 parameter must be MAP, found Integer"
    );
}

/// §S7: a library whose every view names an estimator this build lacks
/// has its gists cut by the `default` preset, and one warning says so.
#[test]
fn an_unevaluable_bound_is_one_warning() {
    let d = Dir::library("prove-unknown");
    let mut p = bounded(20);
    p.estimator = ProfileEstimator::from_wire("x.example/est/9");
    view(&d, "v/odd", p);
    let f = fixture(d);
    let (out, warnings) = prove(&f.d, &kb(), "[ ranked S ]");
    out.unwrap();
    assert_eq!(
        warnings,
        [format!("SMYSL.PROVE: {}", super::prove::UNEVALUABLE)]
    );
}

/// Criterion 18: `bund2 check` reads `smysl.prove` as 4 → LIST.
#[test]
fn check_takes_prove_as_four_to_a_list() {
    let r = Run::new(false);
    let check =
        |src: &str| crate::check::check(&bund2_syntax::compile(src).unwrap(), &r.i.registry, 0);
    let report = check("1 2 3 4 smysl.prove drop 1 2 +");
    assert!(report.findings.is_empty(), "{report:?}");
    assert_eq!(report.abandoned, 0, "{report:?}");
    let report = check("1 2 3 smysl.prove");
    assert_eq!(report.findings.len(), 1, "{report:?}");
    assert_eq!(report.findings[0].word, "smysl.prove");
}

/// 18's `--noio` case for `smysl.prove`.
#[test]
fn under_noio_prove_is_sm06() {
    let h = m(&[
        ("type", s("smysl")),
        ("root", s("/nowhere")),
        ("agent", s("tool:bund2")),
    ]);
    let mut r = Run::new(true);
    assert_eq!(
        r.call(
            vec![h, BundValue::nodata(), BundValue::nodata(), m(&[])],
            "smysl.prove"
        )
        .unwrap_err(),
        super::words::NOIO
    );
}
