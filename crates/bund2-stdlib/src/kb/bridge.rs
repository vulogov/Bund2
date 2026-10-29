//! Bridges: where a query re-enters Bund (RFC-0013 §S5, §S6).
//!
//! **[`run`] holds every `scoped_call`, `to_stack` and `drop_stack` the family
//! makes**, so it is named in `REENTERING` and in `SWITCHES`. It runs λ on a
//! scratch stack and brings the caller's stack back: the current stack is the
//! same before and after a bridge, and the scratch stack exists only during
//! it.
//!
//! **The scratch-name pool.** One name per activation depth, thread-local,
//! minted with `fresh_name` the first time that depth is reached. Activations
//! nest strictly, so no two live ones share a depth, and [`Depth`] gives the
//! depth back on every exit path (D37). The interned-name table therefore
//! grows by the greatest nesting depth a thread reaches, plus one name per
//! collision with a stack the program made, and not with the number of
//! bridges run.
//!
//! [`substitute`] uses no `Vm`; the solver hands it a lookup over the clause
//! instance's variables.

use std::cell::{Cell, RefCell};

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, CALL, LAMBDA};

thread_local! {
    /// The scratch name for each depth.
    static POOL: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// How many bridges are running on this thread.
    static DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// One level of bridge nesting, given back when dropped.
struct Depth(usize);

impl Depth {
    fn enter() -> Depth {
        let d = DEPTH.with(Cell::get);
        DEPTH.with(|c| c.set(d + 1));
        Depth(d)
    }
}

impl Drop for Depth {
    fn drop(&mut self) {
        DEPTH.with(|c| c.set(self.0));
    }
}

/// The scratch name for depth `d`. If a stack of that name already exists,
/// the program made it (`load.stacks` can restore one under any name), so a
/// new name is minted for the depth and kept (§S5 step 3.1).
fn scratch_name(vm: &dyn Vm, d: usize) -> String {
    POOL.with(|p| {
        let mut pool = p.borrow_mut();
        while pool.len() <= d {
            pool.push(bund2_value::fresh_name());
        }
        if vm.stack_exists(&pool[d]) {
            pool[d] = bund2_value::fresh_name();
        }
        pool[d].clone()
    })
}

/// Run `body` on a scratch stack and answer the value it left on top, or
/// `None` (§S5 step 3).
///
/// 1. Take the scratch name for this depth.
/// 2. `scoped_call` creates the stack and runs the body on it.
/// 3. Read the top of the scratch stack with `pull_from`.
/// 4. If the scratch stack is still current, switch back to the caller's.
///    Since F231's fix `scoped_call` never returns that way; this guards
///    against a `Vm` that breaks the contract (criterion 7).
/// 5. Drop the scratch stack. It is not current, so the ring is untouched;
///    if λ dropped it already, this changes nothing.
pub fn run(vm: &mut dyn Vm, body: Vec<BundValue>) -> Result<Option<BundValue>, Error> {
    let depth = Depth::enter();
    let name = scratch_name(vm, depth.0);
    let prev = vm.current_name();
    let ran = vm.scoped_call(&name, body);
    let left = vm.pull_from(&name);
    if vm.current_name() == name {
        vm.to_stack(&prev);
    }
    vm.drop_stack(&name);
    ran.map(|()| left)
}

/// λ's items with each CALL that names a variable of the clause instance
/// replaced by the variable's value, read back as §S1 says (§S5 step 1).
///
/// `lookup` answers `None` for a name that is not a variable, `Some(None)`
/// for an unbound one, which is refused with its name (E20), and
/// `Some(Some(v))` for a bound one.
///
/// **It enters nested lambdas**, and so the items of a `( … )` block, which
/// lower in place (D34). **It does not enter list literals**: a list inside λ
/// is quoted data, often an inner goal whose variables are the inner query's.
/// A worklist, so a deeply nested λ costs heap and not Rust stack (D39).
pub fn substitute(
    items: &[BundValue],
    lookup: &dyn Fn(&str) -> Option<Option<BundValue>>,
) -> Result<Vec<BundValue>, String> {
    // Each frame: the items being walked, where it is, and what it has made.
    let mut frames: Vec<(&[BundValue], usize, Vec<BundValue>)> =
        vec![(items, 0, Vec::with_capacity(items.len()))];
    loop {
        let Some((items, at, _)) = frames.last_mut() else {
            return Ok(Vec::new());
        };
        let Some(item) = items.get(*at) else {
            let Some((_, _, done)) = frames.pop() else {
                return Ok(Vec::new());
            };
            match frames.last_mut() {
                Some((_, _, out)) => out.push(BundValue::lambda(done)),
                None => return Ok(done),
            }
            continue;
        };
        *at += 1;
        if item.dt() == LAMBDA {
            let inner = item.as_lambda().unwrap_or(&[]);
            frames.push((inner, 0, Vec::with_capacity(inner.len())));
            continue;
        }
        let replaced = if item.dt() == CALL {
            match item.str_ref().map(lookup) {
                Some(Some(Some(v))) => v,
                Some(Some(None)) => return Err(item.str_ref().unwrap_or_default().to_string()),
                _ => item.clone(),
            }
        } else {
            item.clone()
        };
        if let Some((_, _, out)) = frames.last_mut() {
            out.push(replaced);
        }
    }
}

#[cfg(test)]
mod tests {
    use bund2_api::{
        Cells, ConditionalFn, Debugging, Error, InlineSite, NativeFn, StackEffect, Vm, diag, input,
    };
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    fn interp() -> Interp {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i
    }

    fn run_in(i: &mut Interp, src: &str) -> Result<(), String> {
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| {
            e.0.rsplit_once(" returned error: ")
                .map_or(e.0.clone(), |(_, m)| m.to_string())
        })
    }

    fn top(src: &str) -> BundValue {
        let mut i = interp();
        run_in(&mut i, src).unwrap_or_else(|e| panic!("{src} failed: {e}"));
        i.peek().unwrap_or_else(|| panic!("{src}: empty"))
    }

    fn err(src: &str) -> String {
        match run_in(&mut interp(), src) {
            Ok(()) => panic!("{src} was expected to fail"),
            Err(e) => e,
        }
    }

    fn yes() -> String {
        BundValue::boolean(true).display()
    }

    fn no() -> String {
        BundValue::boolean(false).display()
    }

    const PEOPLE: &str = "[ [ age ann 34 ] [ age bob 15 ] [ age pat 71 ] \
        [ [ adult P ] ← [ age P A ] [ test { A 18 <= } ] ] \
        [ [ older P Q ] ← [ age P A ] [ age Q B ] [ test { A B < } ] ] \
        [ [ next_age P N ] ← [ age P A ] [ is N { A 1 + } ] ] ] kb!";

    /// The second worked example, as the RFC traces it.
    #[test]
    fn the_second_worked_example_answers() {
        let adults = top(&format!("{PEOPLE} [ adult P ] kb.query"));
        let names: Vec<String> = adults
            .as_list()
            .unwrap()
            .iter()
            .map(|a| a.as_map().unwrap().get("P").unwrap().display())
            .collect();
        assert_eq!(names, ["ann", "pat"]);
        let next = top(&format!("{PEOPLE} [ next_age bob N ] kb.once"));
        assert_eq!(
            next.as_map().unwrap().get("N").and_then(BundValue::as_int),
            Some(16)
        );
        assert_eq!(
            top(&format!("{PEOPLE} [ older pat ann ] kb.holds")).display(),
            yes()
        );
        assert_eq!(
            top(&format!("{PEOPLE} [ older bob ann ] kb.holds")).display(),
            no()
        );
    }

    /// `in` over a λ: X takes each item of the LIST it leaves.
    #[test]
    fn in_over_a_lambda_takes_each_item() {
        let got = top("kb.new [ in X { [ :a :b :c ] } ] kb.query");
        assert_eq!(got.as_list().unwrap().len(), 3);
        assert_eq!(
            top("kb.new [ [ in X { [ :a :b ] } ] [ = X b ] ] kb.holds").display(),
            yes()
        );
    }

    // **Criterion 1c**: E20-E24, and `test` on true, false, a non-BOOL and
    // nothing.

    #[test]
    fn test_answers_on_a_bool_and_refuses_anything_else() {
        assert_eq!(top("kb.new [ test { true } ] kb.holds").display(), yes());
        assert_eq!(top("kb.new [ test { false } ] kb.holds").display(), no());
        let e = err("kb.new [ test { 1 } ] kb.holds");
        assert!(
            e.starts_with("KB.HOLDS: test: expected BOOL, found Integer in "),
            "{e}"
        );
        let e = err("kb.new [ test { 1 drop } ] kb.holds");
        assert!(
            e.starts_with("KB.HOLDS: test: the lambda left nothing in "),
            "{e}"
        );
    }

    /// **Criterion 6b**: E20 names the variable and the goal.
    #[test]
    fn e20_names_the_variable_and_the_goal() {
        let e = err("[ [ [ p X ] ← [ test { X 1 = } ] ] ] kb! [ p Q ] kb.query");
        assert!(
            e.starts_with("KB.QUERY: instantiation error: X is unbound in the lambda of "),
            "{e}"
        );
        assert!(e.contains("test"), "names the goal: {e}");
        // A bound compound with unbound parts is substituted, NODATA in them.
        let got = top("[ [ [ p X ] ← [ = X [ f _ ] ] [ is Y { X } ] ] ] kb! [ p Q ] kb.holds");
        assert_eq!(got.display(), yes());
    }

    #[test]
    fn e21_is_nothing_left() {
        let e = err("kb.new [ is X { 1 drop } ] kb.once");
        assert!(
            e.starts_with("KB.ONCE: is: the lambda left nothing in "),
            "{e}"
        );
        let e = err("kb.new [ in X { 1 drop } ] kb.once");
        assert!(
            e.starts_with("KB.ONCE: in: the lambda left nothing in "),
            "{e}"
        );
    }

    #[test]
    fn e23_is_not_a_list() {
        let e = err("kb.new [ in X { 1 } ] kb.query");
        assert!(
            e.starts_with("KB.QUERY: in: expected a list, found 1 in "),
            "{e}"
        );
        // A literal: a compound is not a list (S1's traps).
        let e = err("kb.new [ in X [ a b c ] ] kb.query");
        assert!(
            e.starts_with("KB.QUERY: in: expected a list, found "),
            "{e}"
        );
    }

    #[test]
    fn e24_wraps_the_lambdas_error_and_try_catches_it() {
        let e = err("kb.new [ test { no_such_word_here } ] kb.query");
        assert!(e.starts_with("KB.QUERY: bridge lambda in "), "{e}");
        assert!(e.contains(" returns: "), "{e}");
        let caught = top(
            "?try :try { kb.new [ test { no_such_word_here } ] kb.holds } set \
             :except { \"caught\" } set !",
        );
        assert_eq!(caught.as_str().as_deref(), Some("caught"));
    }

    /// The scratch stack's name for depth `d`, if minted.
    fn pooled(d: usize) -> Option<String> {
        super::POOL.with(|p| p.borrow().get(d).cloned())
    }

    /// **Criterion 7.** Three bridges: one that succeeds, one that fails, and
    /// one whose λ runs an inner query whose bridge fails. After each the
    /// current stack is the same and no scratch stack is left.
    #[test]
    fn bridges_leave_no_trace() {
        let mut i = interp();
        run_in(&mut i, ":inner [ [ [ q ] ← [ test { 1 } ] ] ] kb! var").unwrap();
        let home = i.current_name();
        let stacks = i.stack_names();
        for (src, ok) in [
            ("kb.new [ test { true } ] kb.holds", true),
            ("kb.new [ test { no_such_word_here } ] kb.holds", false),
            (
                "kb.new [ test { :inner var? [ q ] kb.holds } ] kb.holds",
                false,
            ),
        ] {
            assert_eq!(run_in(&mut i, src).is_ok(), ok, "{src}");
            assert_eq!(i.current_name(), home, "{src}");
            assert_eq!(i.stack_names(), stacks, "{src}");
        }
    }

    /// A `Vm` whose `scoped_call` fails with the scratch stack current, as
    /// F231 did before its fix; every other method is the `Interp`'s.
    struct Broken {
        inner: Interp,
    }

    impl Vm for Broken {
        fn scoped_call(&mut self, stack: &str, _body: Vec<BundValue>) -> Result<(), Error> {
            Vm::to_stack(&mut self.inner, stack);
            Err(Error("refused with the scratch stack current".into()))
        }
        fn push(&mut self, v: BundValue) {
            Vm::push(&mut self.inner, v)
        }
        fn pull(&mut self) -> Option<BundValue> {
            Vm::pull(&mut self.inner)
        }
        fn depth(&self) -> usize {
            Vm::depth(&self.inner)
        }
        fn peek(&self) -> Option<BundValue> {
            Vm::peek(&self.inner)
        }
        fn autoadd(&self) -> bool {
            Vm::autoadd(&self.inner)
        }
        fn set_autoadd(&mut self, on: bool) {
            Vm::set_autoadd(&mut self.inner, on)
        }
        fn peek_at(&self, n: usize) -> Option<BundValue> {
            Vm::peek_at(&self.inner, n)
        }
        fn clear(&mut self) {
            Vm::clear(&mut self.inner)
        }
        fn snapshot(&self) -> Vec<BundValue> {
            Vm::snapshot(&self.inner)
        }
        fn snapshot_workbench(&self) -> Vec<BundValue> {
            Vm::snapshot_workbench(&self.inner)
        }
        fn rotate_left(&mut self) {
            Vm::rotate_left(&mut self.inner)
        }
        fn rotate_right(&mut self) {
            Vm::rotate_right(&mut self.inner)
        }
        fn current_name(&self) -> String {
            Vm::current_name(&self.inner)
        }
        fn to_stack(&mut self, name: &str) {
            Vm::to_stack(&mut self.inner, name)
        }
        fn stack_exists(&self, name: &str) -> bool {
            Vm::stack_exists(&self.inner, name)
        }
        fn ensure_stack_with_capacity(&mut self, name: &str, cap: usize) {
            Vm::ensure_stack_with_capacity(&mut self.inner, name, cap)
        }
        fn ensure_stack(&mut self, name: &str) {
            Vm::ensure_stack(&mut self.inner, name)
        }
        fn depth_of(&self, name: &str) -> usize {
            Vm::depth_of(&self.inner, name)
        }
        fn stack_names(&self) -> Vec<String> {
            Vm::stack_names(&self.inner)
        }
        fn snapshot_of(&self, name: &str) -> Vec<BundValue> {
            Vm::snapshot_of(&self.inner, name)
        }
        fn push_to(&mut self, name: &str, v: BundValue) {
            Vm::push_to(&mut self.inner, name, v)
        }
        fn pull_from(&mut self, name: &str) -> Option<BundValue> {
            Vm::pull_from(&mut self.inner, name)
        }
        fn clear_stack(&mut self, name: &str) {
            Vm::clear_stack(&mut self.inner, name)
        }
        fn drop_stack(&mut self, name: &str) {
            Vm::drop_stack(&mut self.inner, name)
        }
        fn rotate_stacks_left(&mut self) {
            Vm::rotate_stacks_left(&mut self.inner)
        }
        fn rotate_stacks_right(&mut self) {
            Vm::rotate_stacks_right(&mut self.inner)
        }
        fn push_workbench(&mut self, v: BundValue) {
            Vm::push_workbench(&mut self.inner, v)
        }
        fn pull_workbench(&mut self) -> Option<BundValue> {
            Vm::pull_workbench(&mut self.inner)
        }
        fn workbench_depth(&self) -> usize {
            Vm::workbench_depth(&self.inner)
        }
        fn apply(&mut self, v: BundValue) -> Result<(), Error> {
            Vm::apply(&mut self.inner, v)
        }
        fn eval_lambda(&mut self, lambda: &BundValue) -> Result<(), Error> {
            Vm::eval_lambda(&mut self.inner, lambda)
        }
        fn tail_lambda(&mut self, lambda: BundValue) {
            Vm::tail_lambda(&mut self.inner, lambda)
        }
        fn clear_tail_request(&mut self) {
            Vm::clear_tail_request(&mut self.inner)
        }
        fn drain_tail_request(&mut self) -> Result<(), Error> {
            Vm::drain_tail_request(&mut self.inner)
        }
        fn apply_filing(&mut self, v: BundValue) -> Result<(), Error> {
            Vm::apply_filing(&mut self.inner, v)
        }
        fn register_lambda(&mut self, name: &str, body: BundValue) {
            Vm::register_lambda(&mut self.inner, name, body)
        }
        fn unregister_lambda(&mut self, name: &str) {
            Vm::unregister_lambda(&mut self.inner, name)
        }
        fn register_alias(&mut self, alias: &str, target: &str) -> bool {
            Vm::register_alias(&mut self.inner, alias, target)
        }
        fn unregister_alias(&mut self, alias: &str) {
            Vm::unregister_alias(&mut self.inner, alias)
        }
        fn is_lambda(&self, name: &str) -> bool {
            Vm::is_lambda(&self.inner, name)
        }
        fn is_native(&self, name: &str) -> bool {
            Vm::is_native(&self.inner, name)
        }
        fn is_alias(&self, name: &str) -> bool {
            Vm::is_alias(&self.inner, name)
        }
        fn get_lambda(&self, name: &str) -> Option<BundValue> {
            Vm::get_lambda(&self.inner, name)
        }
        fn lambda_names(&self) -> Vec<String> {
            Vm::lambda_names(&self.inner)
        }
        fn alias_pairs(&self) -> Vec<(String, String)> {
            Vm::alias_pairs(&self.inner)
        }
        fn conditional(&self, ty: &str) -> Option<ConditionalFn> {
            Vm::conditional(&self.inner, ty)
        }
        fn register_class(&mut self, name: &str, class: BundValue) {
            Vm::register_class(&mut self.inner, name, class)
        }
        fn class(&self, name: &str) -> Option<BundValue> {
            Vm::class(&self.inner, name)
        }
        fn is_class(&self, name: &str) -> bool {
            Vm::is_class(&self.inner, name)
        }
        fn unregister_class(&mut self, name: &str) {
            Vm::unregister_class(&mut self.inner, name)
        }
        fn method(&self, name: &str) -> Option<NativeFn> {
            Vm::method(&self.inner, name)
        }
        fn is_method(&self, name: &str) -> bool {
            Vm::is_method(&self.inner, name)
        }
        fn call_method(&mut self, name: &str) -> Option<Result<(), Error>> {
            Vm::call_method(&mut self.inner, name)
        }
        fn run_conditional(&mut self, ty: &str, c: BundValue) -> Option<Result<(), Error>> {
            Vm::run_conditional(&mut self.inner, ty, c)
        }
        fn effect_of(&self, name: &str) -> Option<StackEffect> {
            Vm::effect_of(&self.inner, name)
        }
        fn register_var(&mut self, name: &str, value: BundValue) {
            Vm::register_var(&mut self.inner, name, value)
        }
        fn var(&self, name: &str) -> Option<BundValue> {
            Vm::var(&self.inner, name)
        }
        fn unregister_var(&mut self, name: &str) {
            Vm::unregister_var(&mut self.inner, name)
        }
        fn report(&mut self, d: diag::Diagnostic) {
            Vm::report(&mut self.inner, d)
        }
        fn note_source_failure(
            &mut self,
            _label: &str,
            _src: &str,
            _span: (usize, usize),
            _e: &Error,
        ) {
            Vm::note_source_failure(&mut self.inner, _label, _src, _span, _e)
        }
        fn note_positions(
            &mut self,
            _label: &str,
            _src: &str,
            _bodies: &[(BundValue, Vec<(usize, usize)>)],
        ) {
            Vm::note_positions(&mut self.inner, _label, _src, _bodies)
        }
        fn place_top(
            &mut self,
            _label: &str,
            _src: &str,
            _spans: &[(usize, usize)],
        ) -> Option<usize> {
            Vm::place_top(&mut self.inner, _label, _src, _spans)
        }
        fn apply_placed(
            &mut self,
            _stream: Option<usize>,
            _i: usize,
            v: BundValue,
        ) -> Result<(), Error> {
            Vm::apply_placed(&mut self.inner, _stream, _i, v)
        }
        fn failure_caught(&mut self, _e: &Error) {
            Vm::failure_caught(&mut self.inner, _e)
        }
        fn wants_stack(&self, _severity: diag::Severity) -> bool {
            Vm::wants_stack(&self.inner, _severity)
        }
        fn tier_frame(&mut self, _body: &BundValue) {
            Vm::tier_frame(&mut self.inner, _body)
        }
        fn tier_at(&mut self, _ip: usize) {
            Vm::tier_at(&mut self.inner, _ip)
        }
        fn tier_never(&mut self, _body: &BundValue) {
            Vm::tier_never(&mut self.inner, _body)
        }
        fn tier_offered(&self) -> bool {
            Vm::tier_offered(&self.inner)
        }
        fn read_line(&mut self, _ask: &input::Ask<'_>) -> Result<input::Read, String> {
            Vm::read_line(&mut self.inner, _ask)
        }
        fn open_input(&mut self, _history: Option<&str>) -> Result<(), String> {
            Vm::open_input(&mut self.inner, _history)
        }
        fn remember_line(&mut self, _history: &str, _line: &str) {
            Vm::remember_line(&mut self.inner, _history, _line)
        }
        fn read_secret(&mut self, _prompt: &str) -> Result<String, String> {
            Vm::read_secret(&mut self.inner, _prompt)
        }
        fn feed_line(&mut self, _line: String) -> bool {
            Vm::feed_line(&mut self.inner, _line)
        }
        fn debugging(&mut self, _what: Debugging) -> Result<Option<String>, Error> {
            Vm::debugging(&mut self.inner, _what)
        }
        fn context_depth(&self) -> usize {
            Vm::context_depth(&self.inner)
        }
        fn push_context(&mut self, name: &str) {
            Vm::push_context(&mut self.inner, name)
        }
        fn pop_context(&mut self) -> Option<String> {
            Vm::pop_context(&mut self.inner)
        }
        fn request_exit(&mut self, code: i32) {
            Vm::request_exit(&mut self.inner, code)
        }
        fn exit_requested(&self) -> Option<i32> {
            Vm::exit_requested(&self.inner)
        }
        fn cells(&self) -> Option<&Cells> {
            Vm::cells(&self.inner)
        }
        fn inline_site(&mut self, name: &str) -> Option<InlineSite> {
            Vm::inline_site(&mut self.inner, name)
        }
    }

    /// **Criterion 7**, the restore: `run` switches back when the scratch
    /// stack is still current, and drops it.
    #[test]
    fn run_restores_the_stack_a_broken_scoped_call_left() {
        let mut vm = Broken { inner: interp() };
        let home = vm.current_name();
        let stacks = vm.stack_names();
        let r = super::run(&mut vm, vec![BundValue::int(1)]);
        assert!(r.is_err());
        assert_eq!(vm.current_name(), home);
        assert_eq!(vm.stack_names(), stacks);
    }

    /// **Criterion 7**, the collision: a stack `load.stacks` made under a
    /// pooled name is left intact, and the bridge reads its own result.
    #[test]
    fn a_program_stack_under_a_pooled_name_is_left_intact() {
        // Mint this thread's depth-0 name.
        run_in(&mut interp(), "kb.new [ test { true } ] kb.holds").unwrap();
        let name = pooled(0).unwrap();
        let world = format!("bund2-kb-bridge-{}", std::process::id());
        let file = format!("{world}.world");
        let _ = std::fs::remove_file(&file);
        let mut maker = interp();
        maker.push_to(&name, BundValue::int(99));
        let saved = run_in(&mut maker, &format!("\"{world}\" save.stacks"));
        let mut i = interp();
        let loaded = run_in(&mut i, &format!("\"{world}\" load.stacks"));
        let _ = std::fs::remove_file(&file);
        saved.unwrap();
        loaded.unwrap();
        assert!(i.stack_exists(&name), "load.stacks made it");
        let home = i.current_name();
        run_in(&mut i, "kb.new [ is X { 7 } ] kb.once").unwrap();
        let answer = i.pull().unwrap();
        assert_eq!(
            answer
                .as_map()
                .unwrap()
                .get("X")
                .and_then(BundValue::as_int),
            Some(7)
        );
        assert_eq!(i.current_name(), home);
        assert_eq!(
            i.snapshot_of(&name)
                .iter()
                .map(BundValue::display)
                .collect::<Vec<_>>(),
            ["99"]
        );
        assert_ne!(pooled(0).unwrap(), name, "a new name for the depth");
    }

    /// The interned-name table's size, measured by interning a fresh probe.
    fn names_now() -> usize {
        let sym = bund2_value::intern_stack_name(&bund2_value::fresh_name());
        format!("{sym:?}")
            .trim_start_matches("StackSym(")
            .trim_end_matches(')')
            .parse()
            .unwrap()
    }

    /// **Criterion 8a.** 100,000 sequential `kb.holds` activations, each with
    /// a bridge, grow the name table by at most the nesting depth, 1.
    #[test]
    fn sequential_bridges_do_not_grow_the_name_table() {
        let mut i = interp();
        let before = names_now();
        run_in(
            &mut i,
            "100000 { drop kb.new [ test { true } ] kb.holds drop } times",
        )
        .unwrap();
        let grew = names_now() - before - 1;
        assert!(grew <= 1, "the name table grew by {grew}");
    }

    /// **Criterion 8c.** One query whose 100,000 bridges each run an inner
    /// `kb.query` grows the table by at most the nesting depth, 2.
    #[test]
    fn nested_bridges_grow_the_name_table_by_their_depth() {
        let mut i = interp();
        run_in(
            &mut i,
            ":inner [ [ p 1 ] ] kb! var \
             :outer [ [ [ step X ] ← [ test { :inner var? [ p Y ] kb.query len swap drop 1 == } ] ] ] kb! var",
        )
        .unwrap();
        let items: Vec<BundValue> = (0..100_000).map(BundValue::int).collect();
        let goal = BundValue::list(vec![
            BundValue::list(vec![
                BundValue::call("in"),
                BundValue::call("X"),
                BundValue::list(items),
            ]),
            BundValue::list(vec![BundValue::call("step"), BundValue::call("X")]),
            BundValue::call("fail"),
        ]);
        let before = names_now();
        run_in(&mut i, ":outer var?").unwrap();
        i.push(goal);
        run_in(&mut i, "kb.holds").unwrap();
        assert_eq!(i.pull().unwrap().display(), no());
        let grew = names_now() - before - 1;
        assert!(grew <= 2, "the name table grew by {grew}");
    }

    /// **Criterion 9**, the logical update view: a bridge that asserts into
    /// the KB being solved does not change the answers.
    #[test]
    fn a_bridge_asserting_into_the_kb_does_not_change_the_answers() {
        let mut i = interp();
        run_in(
            &mut i,
            ":k [ [ n 1 ] [ n 2 ] \
             [ [ grow X ] ← [ n X ] [ test { :k :k var? [ n 99 ] kb.assert var true } ] ] ] kb! var \
             :k var? [ grow X ] kb.query",
        )
        .unwrap();
        let got = i.pull().unwrap();
        let xs: Vec<i64> = got
            .as_list()
            .unwrap()
            .iter()
            .filter_map(|a| a.as_map().unwrap().get("X").and_then(BundValue::as_int))
            .collect();
        assert_eq!(xs, [1, 2]);
        run_in(&mut i, ":k var? [ n 99 ] kb.query").unwrap();
        assert_eq!(
            i.pull().unwrap().as_list().unwrap().len(),
            2,
            "both asserts kept"
        );
    }

    /// **Criterion 14.** An inner list literal's `Y` is not substituted; a
    /// nested λ's, and a `( … )` block's, are.
    #[test]
    fn substitution_enters_lambdas_and_blocks_but_not_lists() {
        let lookup = |n: &str| match n {
            "X" => Some(Some(BundValue::int(1))),
            "Y" => Some(Some(BundValue::int(2))),
            "U" => Some(None),
            _ => None,
        };
        let body = bund2_syntax::compile("{ X [ parent X Y ] { Y } ( Y ) other }")
            .unwrap()
            .remove(0);
        let got = super::substitute(body.as_lambda().unwrap(), &lookup).unwrap();
        // A `( … )` block's CONTEXT is minted afresh by each compile, so it
        // is compared by kind.
        let shown = |items: &[BundValue]| -> Vec<String> {
            items
                .iter()
                .map(|v| {
                    if v.dt() == bund2_value::CONTEXT {
                        "CONTEXT".to_string()
                    } else {
                        v.display()
                    }
                })
                .collect()
        };
        let want = bund2_syntax::compile("{ 1 [ parent X Y ] { 2 } ( 2 ) other }")
            .unwrap()
            .remove(0);
        assert_eq!(shown(&got), shown(want.as_lambda().unwrap()));
        let body = bund2_syntax::compile("{ { U } }").unwrap().remove(0);
        assert_eq!(
            super::substitute(body.as_lambda().unwrap(), &lookup),
            Err("U".to_string())
        );
        // End to end: the inner query's `Y` stays the inner query's own, so
        // it counts every parent fact rather than those of `nobody`.
        let got = top(":fam [ [ parent tom bob ] [ parent bob ann ] ] kb! var \
             [ [ [ kids N ] ← [ = Y nobody ] \
                 [ is N { :fam var? [ parent X Y ] kb.query len swap drop } ] ] ] kb! \
             [ kids N ] kb.once");
        assert_eq!(
            got.as_map().unwrap().get("N").and_then(BundValue::as_int),
            Some(2)
        );
    }
}
