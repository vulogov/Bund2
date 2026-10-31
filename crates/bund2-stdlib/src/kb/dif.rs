//! `dif/2`: disequality as a suspended constraint (RFC-0013 §S17, M9).
//!
//! **Checking.** `[ dif A B ]` runs a trial unification of A and B and undoes
//! it. The trial is structural only, `unify` with no wake-ups:
//!
//! - A and B cannot unify: the `dif` holds and is discharged.
//! - They unify binding nothing: they are one term, and the `dif` fails.
//! - Otherwise the `dif` suspends, as an attribute, on the variables the
//!   trial bound and on the variables they were bound to.
//!
//! **Waking.** [`Difs::wake`] re-runs the check of every `dif` attached to a
//! variable a finished unification bound, so a half-built compound is never
//! judged. One still suspended is attached to the variables its new trial
//! binds, which is how an attribute reaches the surviving variable of a
//! variable–variable binding: the bound one wakes it, and the trial names
//! the survivor.
//!
//! **Trailed.** The constraints and every attachment are logged, and
//! [`Difs::undo_to`] takes both back to a choicepoint's [`DifMark`], as the
//! trail is unwound. A wake-up binds nothing, so the queue's fixpoint is one
//! pass over it.
//!
//! Nothing here uses the `Vm`.

use std::collections::HashMap;

use super::term::{Arena, Cell, Term};
use super::unify::{Trail, Wake, unify};

/// What a trial of one `dif` found.
#[derive(Debug, PartialEq, Eq)]
pub enum Check {
    /// The two sides cannot unify: the `dif` holds.
    Holds,
    /// They are one term: the `dif` fails.
    Fails,
    /// They could still become one term, by binding these variables.
    Suspends(Vec<Term>),
}

/// Where the store stood when a choicepoint was pushed.
#[derive(Debug, Clone, Copy)]
pub struct DifMark {
    posted: usize,
    log: usize,
}

/// The `dif`s posted in an activation, and which variables wake each.
#[derive(Debug, Default)]
pub struct Difs {
    /// Each posted `dif`'s two sides, in the order they were posted.
    posted: Vec<(Term, Term)>,
    /// A variable's attribute: the `dif`s its binding wakes.
    watch: HashMap<Term, Vec<usize>>,
    /// Each attachment's variable, newest last, so it can be undone.
    log: Vec<Term>,
}

impl Difs {
    pub fn mark(&self) -> DifMark {
        DifMark {
            posted: self.posted.len(),
            log: self.log.len(),
        }
    }

    /// Undo every `dif` posted and every attachment made since `m`.
    pub fn undo_to(&mut self, m: DifMark) {
        while self.log.len() > m.log {
            let Some(v) = self.log.pop() else { break };
            if let Some(ids) = self.watch.get_mut(&v) {
                ids.pop();
                if ids.is_empty() {
                    self.watch.remove(&v);
                }
            }
        }
        self.posted.truncate(m.posted);
    }

    /// Post `[ dif a b ]`: `false` when it fails at once.
    pub fn post(&mut self, arena: &mut Arena, trail: &mut Trail, a: Term, b: Term) -> bool {
        match check(arena, trail, a, b) {
            Check::Holds => true,
            Check::Fails => false,
            Check::Suspends(vars) => {
                self.posted.push((a, b));
                self.attach(self.posted.len() - 1, &vars);
                true
            }
        }
    }

    /// The `dif`s still suspended, each as its two sides: what an answer's
    /// `_residual` holds (§S8, L15).
    pub fn residual(&self, arena: &mut Arena, trail: &mut Trail) -> Vec<(Term, Term)> {
        self.posted
            .iter()
            .filter(|(a, b)| matches!(check(arena, trail, *a, *b), Check::Suspends(_)))
            .copied()
            .collect()
    }

    fn attach(&mut self, id: usize, vars: &[Term]) {
        for v in vars {
            let ids = self.watch.entry(*v).or_default();
            if !ids.contains(&id) {
                ids.push(id);
                self.log.push(*v);
            }
        }
    }
}

impl Wake for Difs {
    fn idle(&self) -> bool {
        self.watch.is_empty()
    }

    fn wake(&mut self, arena: &mut Arena, trail: &mut Trail, bound: &[Term]) -> bool {
        let mut queue: Vec<usize> = bound
            .iter()
            .filter_map(|v| self.watch.get(v))
            .flatten()
            .copied()
            .collect();
        queue.sort_unstable();
        queue.dedup();
        for id in queue {
            let Some(&(a, b)) = self.posted.get(id) else {
                continue;
            };
            match check(arena, trail, a, b) {
                Check::Holds => {}
                Check::Fails => return false,
                Check::Suspends(vars) => self.attach(id, &vars),
            }
        }
        true
    }
}

/// The trial (§S17): unify `a` and `b` with no wake-ups, note what it bound,
/// and undo it.
pub fn check(arena: &mut Arena, trail: &mut Trail, a: Term, b: Term) -> Check {
    let mark = trail.mark();
    if !unify(arena, trail, a, b) {
        return Check::Holds;
    }
    let bound = trail.since(mark).to_vec();
    if bound.is_empty() {
        return Check::Fails;
    }
    // Each bound variable, and the variable it was bound to, if any: either
    // may be bound first later, and each must wake the `dif`.
    let mut vars = Vec::with_capacity(bound.len() * 2);
    for v in bound {
        vars.push(v);
        if let Cell::Var(Some(to)) = arena.cell(v) {
            let to = arena.deref(to);
            if matches!(arena.cell(to), Cell::Var(None)) {
                vars.push(to);
            }
        }
    }
    trail.undo_to(arena, mark);
    Check::Suspends(vars)
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::{BundValue, CALL, LIST, MAP, NODATA, PTR, STRING};

    fn interp() -> Interp {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i
    }

    fn top(src: &str) -> BundValue {
        let mut i = interp();
        let stream = bund2_syntax::compile(src).unwrap_or_else(|e| panic!("{}", e.render(src)));
        i.eval(&stream)
            .unwrap_or_else(|e| panic!("{src} failed: {}", e.0));
        i.peek().unwrap_or_else(|| panic!("{src}: empty"))
    }

    /// A value as the tests write it: a CALL bare, a PTR as `*name`, a
    /// STRING as `:text`, NODATA as `_`, a LIST in brackets and a MAP in
    /// braces, so a residual's variables are told from its atoms.
    fn shape(v: &BundValue) -> String {
        match v.dt() {
            LIST => {
                let items: Vec<String> = v.as_list().unwrap_or(&[]).iter().map(shape).collect();
                format!("[ {} ]", items.join(" "))
            }
            MAP => {
                let kv: Vec<String> = v
                    .as_map()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| format!("{k}={}", shape(v)))
                    .collect();
                format!("{{{}}}", kv.join(", "))
            }
            CALL => v.as_str().unwrap(),
            PTR => format!("*{}", v.as_str().unwrap()),
            STRING => format!(":{}", v.as_str().unwrap()),
            NODATA => "_".into(),
            _ => v.display(),
        }
    }

    /// `kb.query`'s answers over `kb`, each shaped.
    fn answers(kb: &str, goal: &str) -> Vec<String> {
        let list = top(&format!("{kb} {goal} kb.query"));
        list.as_list()
            .unwrap_or_else(|| panic!("{goal}: not a LIST"))
            .iter()
            .map(shape)
            .collect()
    }

    fn query(goal: &str) -> Vec<String> {
        answers("kb.new", goal)
    }

    /// **Criterion 19.** `(G, S=T)` and `(S=T, G)` agree, for
    /// `G = [ dif X a ]`, over a table of `S = T`: the same answers and the
    /// same residuals, whichever comes first.
    #[test]
    fn dif_agrees_before_and_after_the_unification() {
        let g = "[ dif X a ]";
        let rows = [
            ("[ = X a ]", Vec::<&str>::new()),
            ("[ = X b ]", vec!["{X=:b}"]),
            ("[ = X 1 ]", vec!["{X=1}"]),
            ("[ = X [ f a ] ]", vec!["{X=[ f :a ]}"]),
            ("[ = X Y ]", vec!["{X=_, Y=_, _residual=[ [ dif *X :a ] ]}"]),
            ("[ = X [ f Y ] ]", vec!["{X=[ f _ ], Y=_}"]),
            (
                "[ = Y a ]",
                vec!["{X=_, Y=:a, _residual=[ [ dif *X :a ] ]}"],
            ),
            (
                "[ = [ X ] [ Y ] ]",
                vec!["{X=_, Y=_, _residual=[ [ dif *X :a ] ]}"],
            ),
            ("[ = [ X b ] [ a b ] ]", vec![]),
        ];
        for (st, want) in rows {
            let before = query(&format!("[ {g} {st} ]"));
            let after = query(&format!("[ {st} {g} ]"));
            // The variables are named in the order they first appear, so
            // the MAPs are compared without regard to their key order.
            let mut b: Vec<String> = before.iter().map(|a| sorted(a)).collect();
            let mut a: Vec<String> = after.iter().map(|a| sorted(a)).collect();
            b.sort();
            a.sort();
            assert_eq!(b, a, "{g} and {st} disagree by their order");
            let mut w: Vec<String> = want.iter().map(|a| sorted(a)).collect();
            w.sort();
            assert_eq!(b, w, "{g} {st}");
        }
    }

    /// A shaped MAP's entries in sorted order.
    fn sorted(map: &str) -> String {
        let inner = map.trim_start_matches('{').trim_end_matches('}');
        let mut parts: Vec<&str> = inner.split(", ").collect();
        parts.sort_unstable();
        parts.join(", ")
    }

    /// **Criterion 19.** The attribute moves to the surviving variable of a
    /// variable–variable binding, either way round.
    #[test]
    fn dif_follows_a_variable_to_its_survivor() {
        assert!(query("[ [ dif X a ] [ = X Y ] [ = Y a ] ]").is_empty());
        assert!(query("[ [ dif X a ] [ = Y X ] [ = Y a ] ]").is_empty());
        // Y is newer than X, so `[ = X Y ]` binds Y; the `dif` is X's.
        assert!(query("[ [ dif Y a ] [ = X Y ] [ = X a ] ]").is_empty());
        // Two variables, each bound to a third before they meet.
        assert!(query("[ [ dif X Y ] [ = X W ] [ = Y W ] ]").is_empty());
        assert!(query("[ [ = W W ] [ dif X Y ] [ = W X ] [ = Y W ] ]").is_empty());
        assert!(query("[ [ dif X Y ] [ = X Y ] ]").is_empty());
    }

    /// **Criterion 19.** A suspended `dif` reaches the answer as
    /// `_residual`, a LIST of goals that re-posted restore it (§S8, L15);
    /// one discharged, or never suspended, leaves no `_residual`.
    #[test]
    fn the_residual_appears_in_residual() {
        assert_eq!(query("[ dif X a ]"), ["{X=_, _residual=[ [ dif *X :a ] ]}"]);
        assert_eq!(
            query("[ [ dif [ f X b ] [ f a Y ] ] [ = X a ] ]"),
            ["{X=:a, Y=_, _residual=[ [ dif [ f :a :b ] [ f :a *Y ] ] ]}"]
        );
        assert_eq!(query("[ [ dif X a ] [ = X b ] ]"), ["{X=:b}"]);
        assert_eq!(query("[ dif a b ]"), ["{}"]);
        assert!(query("[ dif a a ]").is_empty());
        assert!(query("[ dif [ f X ] [ f X ] ]").is_empty());
        // The occurs check is on (L7): X and [ f X ] can never be one term.
        assert_eq!(query("[ dif X [ f X ] ]"), ["{X=_}"]);
        // A variable no name of the query's reads as `_G`*n*.
        let r = answers("[ [ [ p X ] ← [ dif X Z ] ] ] kb!", "[ p Q ]");
        assert_eq!(r.len(), 1);
        assert!(
            r[0].starts_with("{Q=_, _residual=[ [ dif *Q *_G"),
            "{}",
            r[0]
        );
        // Each posting is its own goal in the residual.
        assert_eq!(
            query("[ [ dif X a ] [ dif X a ] ]"),
            ["{X=_, _residual=[ [ dif *X :a ] [ dif *X :a ] ]}"]
        );
    }

    /// The store is taken back on backtrack: a `dif` posted under a
    /// choicepoint is gone from the alternatives.
    #[test]
    fn backtracking_takes_a_dif_back() {
        assert_eq!(
            query("[ [ dif X a ] [ or [ = X a ] [ = X b ] [ = X c ] ] ]"),
            ["{X=:b}", "{X=:c}"]
        );
        assert_eq!(
            query("[ [ or [ dif X a ] [ = X a ] ] [ = X a ] ]"),
            ["{X=:a}"]
        );
        assert_eq!(
            query("[ [ or [ dif X a ] true ] ]"),
            ["{X=_, _residual=[ [ dif *X :a ] ]}", "{X=_}"]
        );
    }

    /// A `dif` wakes on every binding the search makes: a head's, `in`'s,
    /// a bridge's and `findall`'s.
    #[test]
    fn every_binding_wakes_it() {
        let kb = "[ [ c red ] [ c green ] [ c blue ] ] kb!";
        assert_eq!(
            answers(kb, "[ [ dif X green ] [ c X ] ]"),
            ["{X=:red}", "{X=:blue}"]
        );
        assert_eq!(
            query("[ [ dif X 2 ] [ in X [ 1 2 3 ] ] ]"),
            ["{X=1}", "{X=3}"]
        );
        assert!(query("[ [ dif X 2 ] [ is X { 1 1 + } ] ]").is_empty());
        assert!(query("[ [ dif L [ 1 ] ] [ findall Y [ = Y 1 ] L ] ]").is_empty());
        // Inside `not`, the wake-up fails the inner goal.
        assert_eq!(
            query("[ [ dif X a ] [ not [ = X a ] ] ]"),
            ["{X=_, _residual=[ [ dif *X :a ] ]}"]
        );
    }

    /// `dif`'s proof is `[ goal ← :builtin ]`, as `=`'s is (§S15).
    #[test]
    fn dif_is_proved_as_a_builtin() {
        let list = top("kb.new [ dif X a ] kb.prove");
        let a = list.as_list().unwrap();
        assert_eq!(a.len(), 1);
        let m = a[0].as_map().unwrap();
        assert_eq!(shape(&m["answer"]), "{X=_, _residual=[ [ dif *X :a ] ]}");
        let p = shape(&m["proof"]);
        assert!(p.ends_with("← :builtin ]"), "{p}");
    }
}
