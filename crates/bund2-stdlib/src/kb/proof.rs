//! Proof trees (RFC-0013 §S15, Phase 2, M7).
//!
//! **A proof is a term**, built in the arena as the search goes. Each goal on
//! the goal list carries a proof slot, a fresh variable; justifying the goal
//! binds its slot to `[ goal ← parts… ]`, whose parts hold the slots of the
//! goals it was proved by. The binding is trailed, so a backtrack undoes a
//! proof exactly as it undoes the answer's bindings, and the query's slot,
//! read back at an answer, is that answer's whole proof. Nothing here
//! recurses, and nothing is built when proofs are not asked for: every slot
//! is then `None` (§S15's storage).
//!
//! **The goal in a proof is the goal itself**, sharing its variables, so it
//! reads back "as the answer instantiated it".
//!
//! **What reads back as what** (§S1): the arrow is the CALL `←`, kept as an
//! opaque constant so it is not read back as an atom's STRING; the markers
//! `fact`, `builtin`, `branch`, `then`, `else`, `findall`, `no-proof`,
//! `bridge`, and M8's `asked`, `known` and `yes`, are atoms, so they read back
//! as STRINGs, as every atom does.
//!
//! The slots are threaded through `solve.rs`, which owns the goal list.
//! Nothing here uses the `Vm`.

use bund2_value::BundValue;

use super::term::{Arena, Term};

/// The query's proof slot and the atoms a proof is built from, made once
/// when proofs are asked for, before any choicepoint, so no backtrack cuts
/// them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Proofs {
    /// The answer's proof: the query's slot.
    pub root: Term,
    /// The CALL `←`.
    pub arrow: Term,
    pub fact: Term,
    pub builtin: Term,
    pub branch: Term,
    pub then_: Term,
    pub else_: Term,
    pub findall: Term,
    pub no_proof: Term,
    pub bridge: Term,
    /// §S16's two: a reply the `ask` lambda gave, and one remembered.
    pub asked: Term,
    pub known: Term,
    pub yes: Term,
}

impl Proofs {
    pub fn new(a: &mut Arena) -> Proofs {
        Proofs {
            root: a.var(),
            arrow: a.opaque(BundValue::call("←")),
            fact: a.atom("fact"),
            builtin: a.atom("builtin"),
            branch: a.atom("branch"),
            then_: a.atom("then"),
            else_: a.atom("else"),
            findall: a.atom("findall"),
            no_proof: a.atom("no-proof"),
            bridge: a.atom("bridge"),
            asked: a.atom("asked"),
            known: a.atom("known"),
            yes: a.atom("yes"),
        }
    }

    /// `[ goal ← parts… ]`, a list in the arena.
    pub fn node(&self, a: &mut Arena, goal: Term, parts: &[Term]) -> Term {
        let mut l = a.nil();
        for t in parts.iter().rev() {
            l = a.cons(*t, l);
        }
        let l = a.cons(self.arrow, l);
        a.cons(goal, l)
    }
}

/// **Criterion 17**: `kb.prove` gives proofs as §S15 describes, for worked
/// examples 1, 2 and 4, for a rule using `or` and one using `findall`; and
/// re-running each `:bridge` λ′ reproduces its binding. M8 adds the asked
/// and remembered rows.
#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::{BOOL, BundValue, CALL, INTEGER, LAMBDA, LIST, MAP, NODATA, PTR, STRING};

    fn interp() -> Interp {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i
    }

    fn run(i: &mut Interp, src: &str) {
        let stream = bund2_syntax::compile(src).unwrap_or_else(|e| panic!("{}", e.render(src)));
        i.eval(&stream)
            .unwrap_or_else(|e| panic!("{src} failed: {}", e.0));
    }

    fn top(src: &str) -> BundValue {
        let mut i = interp();
        run(&mut i, src);
        i.peek().unwrap_or_else(|| panic!("{src}: empty"))
    }

    /// A value in §S15's own notation: a CALL bare, a STRING as `:name`, a
    /// LIST in brackets, a LAMBDA in braces, NODATA as `_`. So the CALL `←`
    /// and an atom's STRING are told apart, as a reader must tell them.
    fn shape(v: &BundValue) -> String {
        let items = |vs: &[BundValue]| vs.iter().map(shape).collect::<Vec<_>>().join(" ");
        match v.dt() {
            LIST => format!("[ {} ]", items(v.as_list().unwrap_or(&[]))),
            LAMBDA => format!("{{ {} }}", items(v.as_lambda().unwrap_or(&[]))),
            MAP => {
                let m = v.as_map().unwrap();
                let kv: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", shape(v)))
                    .collect();
                format!("{{{}}}", kv.join(", "))
            }
            CALL | PTR => v.as_str().unwrap(),
            STRING => format!(":{}", v.as_str().unwrap()),
            NODATA => "_".into(),
            INTEGER | BOOL => v.display(),
            _ => v.display(),
        }
    }

    /// `kb.prove`'s answers: each MAP's `answer` and `proof`, shaped.
    fn proofs(src: &str) -> Vec<(String, String)> {
        let list = top(src);
        list.as_list()
            .unwrap_or_else(|| panic!("{src}: not a LIST: {}", list.display()))
            .iter()
            .map(|a| {
                let m = a.as_map().unwrap();
                assert_eq!(m.len(), 2, "{{ answer, proof }}, and nothing else");
                (shape(&m["answer"]), shape(&m["proof"]))
            })
            .collect()
    }

    const FAMILY: &str = ":family [ [ parent tom bob ] [ parent bob ann ] [ parent bob pat ] \
                          [ [ grandparent X Z ] ← [ parent X Y ] [ parent Y Z ] ] ] kb! var";

    /// Worked example 1: a rule over facts.
    #[test]
    fn the_first_worked_example_is_proved() {
        assert_eq!(
            proofs(&format!(
                "{FAMILY} :family var? [ grandparent tom W ] kb.prove"
            )),
            [
                (
                    "{W: :ann}".to_string(),
                    "[ [ grandparent :tom :ann ] ← [ [ parent :tom :bob ] ← :fact ] \
                     [ [ parent :bob :ann ] ← :fact ] ]"
                        .to_string()
                ),
                (
                    "{W: :pat}".to_string(),
                    "[ [ grandparent :tom :pat ] ← [ [ parent :tom :bob ] ← :fact ] \
                     [ [ parent :bob :pat ] ← :fact ] ]"
                        .to_string()
                ),
            ]
        );
    }

    const PEOPLE: &str = ":people [ \
          [ age ann 34 ] [ age bob 15 ] [ age pat 71 ] \
          [ [ adult P ] ← [ age P A ] [ test { A 18 <= } ] ] \
          [ [ next_age P N ] ← [ age P A ] [ is N { A 1 + } ] ] \
        ] kb! var";

    /// Worked example 2: a bridge's proof holds λ′, the substituted λ; the
    /// goal keeps λ as written.
    #[test]
    fn the_second_worked_example_is_proved() {
        assert_eq!(
            proofs(&format!("{PEOPLE} :people var? [ adult P ] kb.prove")),
            [
                (
                    "{P: :ann}".to_string(),
                    "[ [ adult :ann ] ← [ [ age :ann 34 ] ← :fact ] \
                     [ [ test { A 18 <= } ] ← :bridge { 34 18 <= } ] ]"
                        .to_string()
                ),
                (
                    "{P: :pat}".to_string(),
                    "[ [ adult :pat ] ← [ [ age :pat 71 ] ← :fact ] \
                     [ [ test { A 18 <= } ] ← :bridge { 71 18 <= } ] ]"
                        .to_string()
                ),
            ]
        );
        assert_eq!(
            proofs(&format!(
                "{PEOPLE} :people var? [ next_age bob N ] kb.prove"
            )),
            [(
                "{N: 16}".to_string(),
                "[ [ next_age :bob 16 ] ← [ [ age :bob 15 ] ← :fact ] \
                 [ [ is 16 { A 1 + } ] ← :bridge { 15 1 + } ] ]"
                    .to_string()
            )]
        );
    }

    /// **Re-running each `:bridge` λ′ reproduces its binding**: `is`'s λ′
    /// leaves N's value, and `test`'s leaves `true`.
    #[test]
    fn re_running_a_bridges_lambda_reproduces_its_binding() {
        let rerun = |l: &BundValue| {
            let mut i = interp();
            i.eval(l.as_lambda().unwrap()).unwrap();
            i.peek().unwrap()
        };
        let answers = top(&format!(
            "{PEOPLE} :people var? [ next_age bob N ] kb.prove"
        ));
        let a = &answers.as_list().unwrap()[0];
        let m = a.as_map().unwrap();
        let proof = m["proof"].as_list().unwrap().to_vec();
        let is = proof[3].as_list().unwrap();
        assert_eq!(shape(&is[2]), ":bridge");
        let n = m["answer"].as_map().unwrap()["N"].clone();
        assert_eq!(rerun(&is[3]).display(), n.display());

        let answers = top(&format!("{PEOPLE} :people var? [ adult P ] kb.prove"));
        for a in answers.as_list().unwrap() {
            let proof = a.as_map().unwrap()["proof"].as_list().unwrap().to_vec();
            let test = proof[3].as_list().unwrap();
            assert_eq!(rerun(&test[3]).display(), "true");
        }
    }

    /// Worked example 4: a recursive rule, a fact and a second rule.
    #[test]
    fn the_fourth_worked_example_is_proved() {
        let ops = ":ops [ \
              [ depends web api ] [ depends api db ] [ depends api cache ] \
              [ down cache ] \
              [ [ needs S D ] ← [ depends S D ] ] \
              [ [ needs S D ] ← [ depends S M ] [ needs M D ] ] \
              [ [ impaired S D ] ← [ needs S D ] [ down D ] ] \
            ] kb! var";
        assert_eq!(
            proofs(&format!("{ops} :ops var? [ impaired :web D ] kb.prove")),
            [(
                "{D: :cache}".to_string(),
                "[ [ impaired :web :cache ] ← \
                 [ [ needs :web :cache ] ← [ [ depends :web :api ] ← :fact ] \
                 [ [ needs :api :cache ] ← [ [ depends :api :cache ] ← :fact ] ] ] \
                 [ [ down :cache ] ← :fact ] ]"
                    .to_string()
            )]
        );
    }

    /// A rule using `or`: `[ goal ← :branch i p ]`, the first branch 1.
    #[test]
    fn a_rule_using_or_is_proved_by_its_branch() {
        let kb = "[ [ colour snow white ] [ colour sun yellow ] \
                  [ [ light X ] ← [ or [ colour X :white ] [ colour X :yellow ] ] ] ] kb!";
        assert_eq!(
            proofs(&format!("{kb} [ light W ] kb.prove")),
            [
                (
                    "{W: :snow}".to_string(),
                    "[ [ light :snow ] ← [ [ or [ colour :snow :white ] [ colour :snow :yellow ] ] \
                     ← :branch 1 [ [ colour :snow :white ] ← :fact ] ] ]"
                        .to_string()
                ),
                (
                    "{W: :sun}".to_string(),
                    "[ [ light :sun ] ← [ [ or [ colour :sun :white ] [ colour :sun :yellow ] ] \
                     ← :branch 2 [ [ colour :sun :yellow ] ← :fact ] ] ]"
                        .to_string()
                ),
            ]
        );
    }

    /// A rule using `findall`: one proof per solution, each copied at its
    /// `$collect`, in the order found.
    #[test]
    fn a_rule_using_findall_is_proved_by_each_solution() {
        let kb = ":family [ [ parent tom bob ] [ parent bob ann ] [ parent bob pat ] \
                  [ [ kids P L ] ← [ findall C [ parent P C ] L ] ] ] kb! var";
        assert_eq!(
            proofs(&format!("{kb} :family var? [ kids :bob L ] kb.prove")),
            [(
                "{L: [ :ann :pat ]}".to_string(),
                "[ [ kids :bob [ :ann :pat ] ] ← [ [ findall _ [ parent :bob _ ] [ :ann :pat ] ] \
                 ← :findall [ [ parent :bob :ann ] ← :fact ] [ [ parent :bob :pat ] ← :fact ] ] ]"
                    .to_string()
            )]
        );
    }

    /// The rest of §S15's table: `=` and `true` are builtins, `not` has no
    /// proof, `if` names its branch, `once` keeps its solution's, `call` is
    /// transparent, and `in` over a LIST is a builtin each item shares.
    #[test]
    fn each_control_construct_is_proved_as_the_table_says() {
        let kb = "[ [ parent tom bob ] [ parent bob ann ] [ parent bob pat ] \
                  [ [ eq X ] ← [ = X 1 ] ] \
                  [ [ yes ] ← true ] \
                  [ [ nope ] ← [ not [ parent ann _ ] ] ] \
                  [ [ pick C X ] ← [ if [ = C 1 ] [ = X :then ] [ = X :else ] ] ] \
                  [ [ first X ] ← [ once [ parent :bob X ] ] ] \
                  [ [ via X ] ← [ call [ parent :tom X ] ] ] \
                  [ [ each X ] ← [ in X [ 1 2 ] ] ] ] kb!";
        let one = |goal: &str| {
            let all = proofs(&format!("{kb} {goal} kb.prove"));
            all.into_iter().map(|(_, p)| p).collect::<Vec<_>>()
        };
        assert_eq!(one("[ eq X ]"), ["[ [ eq 1 ] ← [ [ = 1 1 ] ← :builtin ] ]"]);
        assert_eq!(one("[ yes ]"), ["[ :yes ← [ true ← :builtin ] ]"]);
        assert_eq!(
            one("[ nope ]"),
            ["[ :nope ← [ [ not [ parent :ann _ ] ] ← :no-proof ] ]"]
        );
        assert_eq!(
            one("[ pick 1 X ]"),
            [
                "[ [ pick 1 :then ] ← [ [ if [ = 1 1 ] [ = :then :then ] [ = :then :else ] ] \
              ← :then [ [ = 1 1 ] ← :builtin ] [ [ = :then :then ] ← :builtin ] ] ]"
            ]
        );
        assert_eq!(
            one("[ pick 2 X ]"),
            [
                "[ [ pick 2 :else ] ← [ [ if [ = 2 1 ] [ = :else :then ] [ = :else :else ] ] \
              ← :else [ [ = :else :else ] ← :builtin ] ] ]"
            ]
        );
        assert_eq!(
            one("[ first X ]"),
            ["[ [ first :ann ] ← [ [ once [ parent :bob :ann ] ] \
              ← [ [ parent :bob :ann ] ← :fact ] ] ]"]
        );
        assert_eq!(
            one("[ via X ]"),
            ["[ [ via :bob ] ← [ [ parent :tom :bob ] ← :fact ] ]"]
        );
        assert_eq!(
            one("[ each X ]"),
            [
                "[ [ each 1 ] ← [ [ in 1 [ 1 2 ] ] ← :builtin ] ]",
                "[ [ each 2 ] ← [ [ in 2 [ 1 2 ] ] ← :builtin ] ]",
            ]
        );
    }

    /// `query`'s `prove` slot pushes what `kb.prove` answers, one MAP per
    /// answer (§S11, D227's refusal replaced); `prove false` and no slot
    /// push the answer alone.
    #[test]
    fn querys_prove_slot_pushes_answer_and_proof() {
        let mut i = interp();
        run(
            &mut i,
            &format!(
                "{FAMILY} query :kb :family var? set :goal [ grandparent tom W ] set \
                 :prove true set !"
            ),
        );
        let mut got = Vec::new();
        while let Some(v) = i.pull() {
            got.push(shape(&v));
        }
        got.reverse();
        let want: Vec<String> = top(&format!(
            "{FAMILY} :family var? [ grandparent tom W ] kb.prove"
        ))
        .as_list()
        .unwrap()
        .iter()
        .map(shape)
        .collect();
        assert_eq!(got, want);
        assert_eq!(
            shape(&top(&format!(
                "{FAMILY} query :kb :family var? set :goal [ grandparent tom W ] set \
                 :prove false set :limit 1 set !"
            ))),
            "{W: :ann}"
        );
    }

    /// Proofs are kept only when asked for: `kb.query`'s answers are as
    /// they were.
    #[test]
    fn kb_query_keeps_no_proof() {
        assert_eq!(
            shape(&top(&format!(
                "{FAMILY} :family var? [ grandparent tom W ] kb.query"
            ))),
            "[ {W: :ann} {W: :pat} ]"
        );
    }

    /// §S15's last row but one (M8): a reply the `ask` lambda gave is
    /// `[ goal ← :asked :yes ]`, and one remembered `[ goal ← :known :yes ]`,
    /// whether from `known` or from earlier in the run.
    #[test]
    fn an_asked_goal_is_proved_by_its_reply() {
        let q = |known: &str| {
            format!(
                ":k [ [ [ furry X ] ← [ has X fur ] ] [ [ both X ] ← [ has X fur ] [ has X fur ] ] ] \
                 kb! :has 2 kb.askable var {known} \
                 query :kb :k var? set :goal [ both rex ] set :ask {{ drop true }} set \
                 :prove true set !"
            )
        };
        assert_eq!(
            shape(&top(&q("")).as_map().unwrap()["proof"]),
            "[ [ both :rex ] ← [ [ has :rex :fur ] ← :asked :yes ] \
             [ [ has :rex :fur ] ← :known :yes ] ]"
        );
        let seeded = q(":seen [ [ [ has rex fur ] true ] ] var");
        let seeded = seeded.replace("query ", "query :known :seen set ");
        assert_eq!(
            shape(&top(&seeded).as_map().unwrap()["proof"]),
            "[ [ both :rex ] ← [ [ has :rex :fur ] ← :known :yes ] \
             [ [ has :rex :fur ] ← :known :yes ] ]"
        );
    }
}
