# RFC-0003: Surface syntax, BundIR, and the Tier 0 interpreter

- Status: **Draft**
- Depends on: RFC-0001 (the value), RFC-0002 (symbols and the word table)
- Decisions consumed: D3 (no eval-specific tier rule, as amended 2026-08-29),
  D5 (lambda bodies are write-once; the compiled cache is keyed on identity and
  needs no invalidation), D11 (no external dependents of `compile_to_binary`;
  version the IR format freshly), D16 (the world is permanently open, so a call
  target may be a name computed at runtime), D34 (`( … )` lowers in place).
  D27 informs but is not consumed.
- Blocked on: nothing. D34 (lower in place) and F59 (`execute.` recurses
  `FromStack`) are both resolved; F59's scope caveat — `execute_class` and
  `execute_object` are unread — is a read before implementing, not a blocker.
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: nothing. An earlier draft claimed the roadmap's §1.1, which it
  does not contradict — the alternatives section restates §1.1's finding rather
  than refining it.
- **Oracle caveat.** Every "confirmed against the oracle" claim below inherits
  **F21**: the built oracle links crates.io releases, not the pinned submodules.
  `cargo xtask cite` compares the two `src/` trees byte for byte on every run and
  currently passes, so the claims hold — conditionally on that check, not because
  the submodules were executed.

## Summary

Bund's front end is 54 lines of grammar and fourteen token handlers that
produce a flat `Vec<Value>`. Its interpreter is a loop over that vector calling
`apply` on each element, and `apply` reaches back into the same loop through
`lambda_eval`, so Rust stack depth tracks Bund call depth. This RFC specifies
Bund2's replacement: the same grammar with its traps stated rather than
inherited by accident, one evaluator instead of two, a **flat frame loop** with
no Rust recursion, and BundIR as a compiled *cache* over lambda bodies that
remain `Vec<BundValue>` — because `compile`, `lambda!`, `lambda*` and `curry`
all build those bodies at runtime, and a representation Bund code cannot
construct is not a representation of this language.

## Motivation

**The interpreter cannot be made deeper, and it exists twice.** `Bund::eval`
(`reference/bundcore/src/bundcore_eval.rs:7-45`) and `bund_compile_and_eval`
(`reference/Bund/src/stdlib/helpers/eval.rs`) are the same loop,
down to identical error strings, in two crates. Both call `vm.apply`, which for
a lambda calls `lambda_eval` (`reference/rust_multistackvm/src/multistackvm_apply.rs:49`),
which calls `apply` on each body element
(`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:13-14`). A
recursive Bund word is a recursive Rust call. `execute` recurses too, on its
LIST and MAP arms (`reference/rust_multistackvm/src/stdlib/execute.rs:42,67`).

**The grammar and the token handlers disagree.** `digits` admits `_` between
digits (`reference/bund_language_parser/bund.pest:49`) and
`lexical_core::parse` rejects it (`reference/bund_language_parser/src/vm/integer.rs:8-14`),
so `1_000` parses and then fails conversion — F49. `int` cannot match a
leading-zero run (`bund.pest:48`), so `007` decomposes into three integers
rather than failing — F50. Neither is written down anywhere.

**Every caller papers over the same grammar defect.** `name`, `atom`, `ptr`,
`stack` and `command` all require *trailing* whitespace (`bund.pest:26-30`), so
a program ending in a word does not parse. The three sites that parse all append
`\n` first — `bundcore_eval.rs:8`, `helpers/eval.rs`, and
`bund_interpreter.rs:29`. `helpers/run_snippet.rs` is not itself an evaluation
path but appends at five places (`:15,62,100,138,162`) before delegating to
`Bund::eval`, which appends again — so the CLI paths append **twice**. The
workaround is uniform, undocumented, load-bearing, and partly duplicated.

**Control flow is partly data, and the dispatch table for it is global
mutable state.** `?ifthenelse` and `?try` push CONDITIONAL values whose
branches are lambdas in named slots
(`reference/Bund/src/stdlib/functions/conditional/conditional_ifthenelse.rs:8-10`),
executed by looking the conditional's `type` string up in `CF`, a
`Mutex<BTreeMap<String, ConditionalFn>>`
(`reference/rust_multistackvm/src/stdlib/execute_types/mod.rs:11-16`). That is
a fourth live dispatch table, and RFC-0000's "three live tables" counted word
registration only.

## Current behaviour

### 1. The grammar is 54 lines and admits twelve value forms

`value` is an ordered choice of `float`, `integer`, `lambda`, `list`, `ctx`,
`ptr`, `name`, `command`, `atom`, `stack`, `string`, `literal`
(`reference/bund_language_parser/bund.pest:7-20`). Order is significant: `ptr`
precedes `name`, and `name` carries an explicit `!("`")` guard (`:28-29`).

Five rules are atomic and require trailing whitespace — `atom`, `stack`,
`name`, `ptr`, `command` (`:26-30`). Three are not atomic and nest normally —
`lambda`, `list`, `ctx` (`:31-33`), each requiring `term+`.

`element` admits `LETTER`, `SYMBOL` and seventeen punctuation characters
(`:36`), which is why `←`, `Σ` and `$` are ordinary name characters. `:` and
`@` are **not** in `element`, which is what separates `atom` and `stack` from
`name`. `cmd` is `":" | ";"` (`:44`), so `:` followed by whitespace is a
command and `:` followed by a name is an atom — the disambiguation is the
trailing-whitespace requirement, not a lookahead.

`COMMENT` is `//` to end of line (`:54`), applied by pest between tokens and
therefore *not* inside atomic rules or string bodies.

### 2. Fourteen handlers, and one of them writes to the output

`parse_pair` dispatches on the rule
(`reference/bund_language_parser/src/parse.rs:7-52`). Most handlers are a line:

| form | produces | citation |
|---|---|---|
| `name` | `Value::call(name)` | `vm/name.rs` |
| `command` | `Value::call(":")` / `(";")` | `vm/command.rs` |
| `atom` `:foo` | `Value::from_string("foo")` | `vm/atom.rs` |
| `ptr` `` `foo `` | `Value::ptr("foo")` | `vm/ptr.rs` |
| `stack` `@main` | `Value::named_context("main")` | `vm/stack.rs` |
| `lambda` `{…}` | `Value::to_lambda(terms)` | `vm/lambda.rs` |
| `list` `[…]` | `Value::from_list(terms)` | `vm/list.rs` |
| `EOI` | `Value::exit()` | `vm/eoi.rs` |

`atom` slices `[1..len-1]` then trims, dropping the leading `:` and the
trailing whitespace the grammar required.

**`ctx` is the exception, and it is F9.** `( … )` does not return a tree. It
pushes `Value::context()` into the shared output vector, parses its inner terms
pushing each into that same vector, and returns `Value::call("endcontext")`
(`reference/bund_language_parser/src/vm/ctx.rs:8-20`). The vector it mutates is
the one `bund_parse` is building (`reference/bund_language_parser/src/lib.rs:16,21-23`),
so `( a b )` flattens to four stream elements. Two of the three bracket forms
nest; the third is a side channel.

Every parse ends with an EXIT value, from the `EOI` handler.

### 3. The evaluator, twice

Both copies do: append `\n`, `bund_parse`, then per element — `NONE` continue,
`EXIT` break, `ERROR` bail, otherwise `apply`
(`reference/bundcore/src/bundcore_eval.rs:8-43`). The error text on the last
arm is `Attempt to evaluate value {:?} returned error: {}`, which is what the
oracle's error table shows.

`apply` (`reference/rust_multistackvm/src/multistackvm_apply.rs:8-104`)
branches on `dt`:

- **CALL** — `is_command` first (`:16`), then `autoadd` (`:19`), then the `$`
  sigil routing to `call_internal_word` (`:33`), then alias resolution (`:39`),
  then lambda (`:46`), then inline (`:59`).
- **CONTEXT** — switch stacks, unless `autoadd` (`:69-87`).
- **everything else** — push, unless `autoadd`, in which case append into the
  value on top of the stack (`:88-101`).

`autoadd` is one `bool` on the VM (`reference/rust_multistackvm/src/multistackvm.rs:22,41`),
toggled by the `:` and `;` **commands** (`reference/rust_multistackvm/src/stdlib/autoadd.rs:28-29`).
It is not nestable — enabling while enabled bails "You can not nest
autocollection" (`:6-8`) — and enabling requires a non-empty stack (`:9-11`),
because the collected values append into the top value.

### 4. `execute` is the polymorphic call, and it is spelled `!`

`!` is an alias for `execute` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:5`),
and ERRATA records it as one of the five most-used words in the corpus — 69
invocations across 39 of 132 programs — while the registered name `execute` is
never spelled by any program.

`stdlib_execute_base_inline` dispatches on `type_of()` over eight arms
(`reference/rust_multistackvm/src/stdlib/execute.rs:26-98`):

| `dt` | behaviour |
|---|---|
| `PTR`/`STRING`/`CALL` | `vm.call(name)`, if the payload is `Val::String` |
| `LIST` | push each element, **recurse** (`:40-48`) |
| `MAP`/`INFO`/`CONFIG`/`ASSOCIATION` | pull a key, `get`, push, **recurse** (`:56-72`) |
| `CONDITIONAL` | `execute_conditionals`, via the `CF` table |
| `CLASS` | `execute_class` |
| `OBJECT` | `execute_object` |
| `LAMBDA` | `lambda_eval` |
| otherwise | "Received value is not of executable type" |

`execute.`'s wrapper tests the **main** stack and then pulls the **workbench**
(`:117` against `:22`) — F53.

### 5. Conditionals are values, dispatched through a global table

`?ifthenelse`, `?try`, `?error`, `context`, `curry`, `fmt`, `csv` and `sqlite`
each push a `Value::conditional()` carrying `type` — `?ifthenelse` at
`reference/Bund/src/stdlib/functions/conditional/conditional_ifthenelse.rs:8-10`,
`?try` at `conditional_tryexcept.rs:7-11`, `context` at
`conditional_ctx.rs:19-22`, `curry` at `conditional_curry.rs:19-22` — and each
registers a Rust handler under that type string into `CF`
(`conditional/mod.rs:20-27`), with the words themselves registered at `:36-44`.

**There is a ninth type and a ninth word, both outside that enumeration.**
`through` is inserted into `CF` from a different crate
(`reference/rust_multistackvm/src/stdlib/execute_types/mod.rs:36`), and `raise`
is registered alongside the eight (`conditional/mod.rs:44`) but pushes nothing —
it pulls a message and fails
(`reference/Bund/src/stdlib/functions/conditional/raise.rs:6-17`). The
cross-crate half is what makes D7's "populated at construction" non-trivial.

`!` on a CONDITIONAL reads `type`, looks it up, and calls the handler
(`reference/rust_multistackvm/src/stdlib/execute_types/execute_conditionals.rs:9-25`).

Branches are lambdas in named slots, populated by `set` between construction
and execution, and **a missing slot defaults to an empty lambda** rather than
erroring (`conditional_ifthenelse.rs:15-26`). `?try` additionally reifies a
caught failure: it builds an `error` conditional carrying `context` (the error
string) and `associated`, pushes it, then runs `except` and `recovery`
(`conditional_tryexcept.rs:32-49`). Errors are `easy_error::Error` — a string
context, with no position.

`curry` is code generation: it builds a lambda of the captured data followed by
the target lambda followed by a `CALL "!"`, and registers it under a name
(`conditional_curry.rs:44-53`).

### 6. Programs are data, and that is the idiom

`compile` parses a string into a **LIST of values**, dropping the trailing EXIT
(`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:28-42`).
`lambda!` converts a LIST to a LAMBDA
(`reference/Bund/src/stdlib/functions/bund/bund_fun.rs:163-186`). `lambda*`
drains the **entire current stack** into a LAMBDA, preserving order
(`:189-202`). `register` installs a LAMBDA or CLASS under a name
(`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:5-22`).

So `"…" compile lambda! :name swap register` is a complete, supported route
from text to a callable word, and §1.3 of the roadmap records this as the
documented idiom rather than an edge case.

### 7. The corpus idiom for blocks

`{ }` is grammar, not a word, and the corpus uses it as a lambda-valued
argument:

```
:PrintHelloWorld { "Hello World!" println } register
42 42 == { true } if
?try :try { … } set :except { … } set !
```

`{` and `}` appear in 42 of the 69 captured goldens, which is the single
largest blocker to conformance.

## Design

### D1. One grammar, one parser, stated traps

`bund2-syntax` implements the grammar above, producing a typed AST with spans
rather than a `Vec<Value>`. The twelve value forms and the ordered choice are
preserved exactly, including the `ptr`-before-`name` order and the `!("`")`
guard.

Three grammar traps are **preserved and documented** rather than silently
inherited:

- `1_000` is rejected (F49). The grammar admits it, the reference cannot
  convert it, and the observable behaviour is an error.
- `007` is three integers (F50). Not an error, and no program that contains
  one means anything else today.
- `{}` and `[]` do not parse (F51). An empty lambda is constructible at
  runtime and unspellable as a literal.

The trailing-whitespace requirement (`bund.pest:26-30`) is preserved with its
terminator set widened by exactly one member: `name`, `atom`, `ptr`, `stack`
and `command` must be followed by whitespace **or end of input**.

This is subtler than it first looks, and an earlier draft of this section got
it wrong by proposing to drop the requirement entirely. The requirement is
load-bearing, not vestigial: a name abutting a closing brace does **not** parse,
because `}` is not whitespace. Confirmed against the oracle —

    { 1 println} !     parse error
    [1 2] println      accepted

The second is accepted because `integer` carries no such requirement
(`bund.pest:22`); only the five atomic rules do. So dropping the rule would
newly accept `{ 1 println}`, which is a program the reference rejects — a
deviation, not a simplification.

Admitting end-of-input as a terminator is what the reference already achieves
by appending `\n` at all four evaluation sites. Under it, `1 2 +` with no
trailing newline parses, exactly as the reference parses it after the append;
`{ 1 println}` still fails, exactly as the reference fails it. The set of
accepted programs is unchanged. What changes is that the `\n` append stops
being load-bearing and disappears from the call sites.

### D2. Scoped blocks — and the nesting case is a decision, not a cleanup

`( … )` becomes a node in the AST like `{ … }` and `[ … ]`, not a mutation of
the output vector.

**At top level that is behaviour-preserving; nested, it is not.** An earlier
draft claimed lowering "emits the same three-part stream the reference
produces". That is true only when the `( … )` is outermost. `lambda.rs:11` and
`list.rs:11` pass the shared output vector down while collecting their own terms
into a local `res`, so a `( … )` inside a block writes its CONTEXT marker and
inner terms to the **top-level** stream and leaves only `endcontext` in the
block. Confirmed against the oracle:

    :F { 7 9 } register        ok
    :F { ( 7 ) 9 } register    REGISTER expecting lambda name to be string

Recorded as **F58**. Lowering the node in place is a **deviation**, not a
representation change, and **D34 approves it**: `( … )` is a scope in the block
that lexically contains it, and lowering emits the CONTEXT marker, the inner
terms and the `endcontext` call together inside that block.

Top-level `( … )` is unchanged, which is where every corpus use is — exactly one
of the 132 programs uses the form at all
(`reference/Bund/examples/bund_dynamic_demos/create_lambda_on_the_fly_in_the_context.bund:10,20`),
wrapping the `call,` → `lambda*` → `register` idiom. So no golden moves.

**Why the hoist is not an alternative scoping rule.** `Value::context()` names a
fresh anonymous scratch stack
(`reference/rust_dynamic/src/create_special.rs:22-33`); `apply` switches to it
through `to_stack`, which also pushes the name onto the runtime `stacks_stack`
(`reference/rust_multistackvm/src/multistackvm_to_stack.rs:5-19`); `endcontext`
carries the top value out to the workbench, drops the stack and pops that
nesting stack (`reference/rust_multistackvm/src/stdlib/ctx.rs:5-27`). The two
are a balanced pair over runtime state, and the hoist separates them in *time* —
the open runs with the enclosing stream, the close only when the block is
called. Measured on the oracle, a lambda holding a hoisted-open context destroys
its caller's stack when invoked, taking values that were never inside the
parentheses.

Under D4 this falls out rather than being added: a context is a frame with an
exit action, the same mechanism that fixes F57.

F9 closes at the representation; D34 closes the behavioural half.

### D3. BundIR, and what stays a value

A lambda body **stays `Vec<BundValue>`**. This is not a concession; it is
forced. `compile` yields a LIST of values, `lambda!` retypes a LIST as a
LAMBDA, `lambda*` folds the live stack into one, and `curry` assembles one
element by element. Any representation Bund code cannot build is not this
language's representation.

BundIR is therefore a **cache over** a body, never the body itself:

- keyed on the body's **identity**, as D5 resolved. D5 finds lambda bodies
  write-once — `set` on a LAMBDA returns a new value
  (`reference/rust_dynamic/src/set.rs:11-13`) and `push` converts to LIST first
  (`reference/Bund/src/stdlib/functions/values/push.rs:34`) — and concludes
  that an identity-keyed cache "simply does not contain the replacement";
- **no invalidation machinery**, which is D5's stated consequence and not an
  independent choice here. An earlier draft of this section said "invalidated
  when the body changes", which contradicted a resolved decision;
- entirely optional — Tier 0 interprets the `Vec<BundValue>` directly, and must,
  because that is the fallback whenever a body is built and run once.

**Content hashing is explicitly not adopted, and could not be adopted naively.**
Every `BundValue` carries a lazily minted `id` and a sampled `stamp` (D1, D2), so
a literal hash over a freshly parsed body never equals the hash of an earlier
parse of the same text. Any future move to content keying — whose one benefit is
letting structurally identical lambdas share compiled code — must first define
the hash to exclude `id` and `stamp`. That is not decided here.

D3's **amended** resolution follows from identity keying rather than from
content hashing, and the amendment reverses the original reasoning: eval'd code
re-parses and mints fresh values on every call, so it cannot hit an
identity-keyed cache at all and stays at Tier 0 without any rule. The conclusion
— no eval-specific tier rule — is unchanged.

**D11 is discharged here.** D11 resolves "no external dependents of
`compile_to_binary`; version the IR format freshly." Under this design BundIR is
an in-process cache with no serialised form, so there is no format to version.
If a later RFC serialises it — for AOT (RFC-0006) or an image (RFC-0010) — D11's
"version freshly" applies at that point and this RFC's silence must not be read
as a format decision.

### D4. The flat frame loop

Tier 0 is one loop over an explicit frame stack. No Rust recursion **for Bund
call depth** — a bound that needs stating precisely, because an earlier draft
claimed "no Rust recursion, anywhere on the evaluation path" and that is not
what this design delivers.

Two exclusions are deliberate:

- **The parser still recurses.** `parse_pair` descends through `lambda.rs:11`,
  `list.rs:11` and `ctx.rs:11`, so deeply nested brackets overflow at parse
  time, not at call time. Bund2's parser inherits that shape. Nesting depth and
  call depth are different limits and only the second is addressed here.
- **Native words that re-enter evaluation** need a mechanism, described below;
  without one the `?`-family reintroduces exactly the recursion this section
  removes.

A frame carries: the body being executed, an instruction pointer, and an
optional **exit action**. Calling a lambda pushes a frame; returning pops one.
The three sites that recurse today become frame pushes:

- `lambda_eval` (`multistackvm_lambda_eval.rs:13-14`),
- `execute`'s LIST arm (`execute.rs:42`),
- `execute`'s MAP arm (`execute.rs:67`).

This buys four things at once, which is why it is the centre of this RFC: deep
Bund recursion stops overflowing the Rust stack; the debugger (RFC-0008) gets a
steppable state; async (RFC-0007) gets a suspendable one; and unwinding gets a
place to run exit actions.

**Re-entrant natives are resumable, not recursive.** Exit actions run on the way
*out* of a frame; the `?`-family needs to suspend in the *middle* of a native and
resume with a result. `conditional_run` for `ifthenelse` evaluates the `if`
lambda, then pulls and casts the result, then evaluates a branch
(`reference/Bund/src/stdlib/functions/conditional/conditional_ifthenelse.rs:27-49`)
— the second evaluation is not in tail position, so no exit action expresses it.
`?try` is worse: it catches the Rust `Err` from `lambda_eval` and then runs two
further lambdas (`conditional_tryexcept.rs:32-49`).

So a native word does not call back into evaluation. It returns a **request** to
the loop — *push this body, resume me at state k* — and the loop re-enters the
native with the state tag when that frame completes. Each `?`-family handler
becomes a small state machine: for `ifthenelse`, state 0 requests the `if` body,
state 1 pulls and casts and requests a branch, state 2 returns.

`?try` becomes a **handler frame** rather than a Rust `catch`. It marks the
frame stack; a failure unwinds to the nearest marked frame, discarding the
frames above it, and resumes the handler at its except-state. What that unwind
does to the error *text* is not settled — `err.ctx` is today a concatenation
assembled by `bail!` at every level a failure passed through
(`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:17`,
`reference/rust_multistackvm/src/stdlib/execute.rs:45`,
`reference/bundcore/src/bundcore_eval.rs:33`), so the string encodes the Rust
nesting this design replaces. Carried as **Q21**.

**Exit actions fix F57.** `context` switches stacks and restores afterwards,
but the restore sits after three early returns and is skipped on error
(`conditional_ctx.rs:60-72`). As a frame with an exit action, the restore runs
on both paths. The reference's shape cannot express this; the flat loop does so
structurally rather than as a patch.

### D4a. Context depth is represented, so `endcontext` can refuse

D34 balances every `( … )` the parser produces. It does not make `endcontext`
safe, because `endcontext` is a registered inline
(`reference/rust_multistackvm/src/stdlib/ctx.rs:31`) and a program may call it
by hand, balanced or not.

Today that is silent corruption — **F60**. The guard
`if vm.stacks_stack.len() < 1 { bail!("Context is empty") }` (`ctx.rs:6-8`)
cannot fire: `stacks_stack` is initialised holding `"main"`
(`reference/rust_multistackvm/src/multistackvm.rs:38-39`) and `pop_stacks`
refuses to go below one
(`reference/rust_multistackvm/src/multistackvm_stacks_stack.rs:10-16`). So a
bare `endcontext` passes the guard, moves the top value to the workbench, and
drops the **current** stack. Confirmed against the oracle: `111 222 333` on
`main`, then `endcontext`, leaves `main` empty with `333` on the workbench and
no diagnostic.

The invariant is unrepresentable as written, because `stacks_stack` conflates
the base stack with stacks opened by `(`. Bund2 separates them: a context is a
**frame with an exit action** (D4), so context depth is the count of context
frames, and `endcontext` fails with the reference's own message — `Context is
empty` — when that count is zero.

This is a **narrowing**, and the only one in this RFC: a program that today
destroys a stack silently now gets an error. It is safe because the current
behaviour produces no output for a golden to have captured.

### D4b. `execute.` takes only its receiver from the workbench

F53 corrects `execute.`'s wrapper, which guards the main stack
(`reference/rust_multistackvm/src/stdlib/execute.rs:117`) and then pulls the
workbench (`:22`). That fix alone is not safe, because it exposes two arms with
no coherent workbench behaviour — **F59**.

`op` is threaded through every *pull* site and no *push* site: the LIST arm
pushes each element onto the **main** stack (`:41`) and the MAP arm pushes the
resolved value there (`:66`), and both then recurse re-passing `op` (`:42`,
`:67`). So each resolves a value onto one stack and looks for it on another.
Measured: with `dup` and a `LIST(111, 222)` on the workbench and `7` on main,
`execute.` leaves `7, 111, 111, 222` and fails — the elements pushed as data and
never executed, the workbench's `dup` executed in their place.

**The `,`-family already settles what `op` means.** `get,`/`set,` pull the
receiver per `op`
(`reference/Bund/src/stdlib/functions/values/getsetinplace.rs:42-45`) while
taking the key from the main stack unconditionally (`:54`) and pushing the
result there unconditionally (`:70`). `op` selects where the *receiver* lives;
operands and results live on the main stack.

So Bund2's `execute.` **takes only its receiver from the workbench and then
proceeds exactly as `execute`**: the recursion passes `FromStack`, because by
then the value to execute is on the main stack. The MAP arm's key continues to
come from the main stack, which under this rule was always right.

That is the sibling convention rather than a new one, and it is the only reading
under which F53's fix is coherent — which is why this RFC treats them as one
change. No corpus program uses `execute.` or its alias `!.`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:6`), so no golden is
at risk.

**Scope: two arms, not four.** `execute_class` and `execute_object` were read to
settle this. Neither splits pushes from pulls, because neither consults `op` —
both take it as `_op`. `execute_class` pushes the value onto the main stack and
delegates
(`reference/rust_multistackvm/src/stdlib/bund_execute/execute_class.rs:8-10`);
`execute_object` pulls a method name from the main stack, pushes the value
there, and dispatches with `vm.m`
(`reference/rust_multistackvm/src/stdlib/bund_execute/execute_object.rs:8-20`).

Both therefore already behave as this rule prescribes — receiver in hand,
operands and results on main — which is independent confirmation of the
convention rather than an exception to it. LIST and MAP are the only arms that
ever split the two.

### D5. Errors carry position

Errors become a structured value carrying a span, not an `easy_error::Error`
string. The reference's error text is preserved verbatim where a golden
captures it — the wrapping shape `Attempt to evaluate value {:?} returned
error: {}` included — and the span is additional, not a replacement.

`?try` continues to reify a caught error as an `error` conditional with a
`context` slot (`conditional_tryexcept.rs:35-40`); the slot's string is
unchanged, and the span rides alongside.

### D6. One evaluator

F52's duplication does not survive. `bund2-interp` has a single evaluation
entry point; `bund.eval` and the top-level script path both use it. This is not
a deviation — the two copies are currently identical — but it is a precondition
for D5 and D4 being true of both.

### D7. Conditional dispatch is a registry, not a global mutex

The `CF` table becomes a field on the registry RFC-0002 already owns, keyed by
type string, populated at construction. Same lookup, same failure message
(`EXECUTE:CONDITIONAL conditionals handler does not exist: {type}`), no global
mutable state. The `STDLIB` mutex went the same way in RFC-0002 and for the
same reason.

### D8. Inline caches on call sites

Each call site carries a monomorphic cache of `(Symbol, generation) →
resolution`, invalidated by RFC-0002's generation counter. This is where the
double alias resolution F6 records — `apply` at `:39` and `i()` at
`multistackvm_inline.rs:71-72` — stops being paid twice per call. The alias
table has no multi-hop chains (`create_aliases.rs:4-45`), so a resolved site is
stable until a generation bumps.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| Twelve value forms, ordered choice, `ptr` before `name` | preserved exactly |
| `atom` → STRING, `name` → CALL, `` ` `` → PTR, `@x` → named context | preserved exactly |
| EOI emits EXIT; EXIT breaks the eval loop | preserved exactly |
| `{}`/`[]` unparseable; `007` decomposes; `1_000` rejected | preserved, and stated (F51, F50, F49) |
| Trailing-whitespace requirement on five rules | preserved; terminator set gains end-of-input, which is what the `\n` append already achieved. `{ 1 println}` still fails |
| `ctx` at top level | preserved exactly; representation changed (F9) |
| `ctx` nested in a block, hoisting out of it | **deliberately changed** — F58, approved by D34; lowered in place |
| `endcontext` callable unbalanced, dropping the current stack | **fixed** (F60) — a narrowing; today it is silent |
| `apply`'s resolution order, `autoadd` semantics, non-nestability | preserved exactly |
| `execute`'s eight arms and their errors | preserved exactly |
| `execute.` guarding the wrong stack | **fixed** (F53), but *not* widening only — see F59 below |
| `execute.` on a LIST or a MAP | **deliberately changed** — F59; recursion passes `FromStack`, so only the receiver comes from the workbench |
| `through`, the ninth conditional type, registered cross-crate | preserved; D7 must carry it |
| `raise`, registered with the family but pushing nothing | preserved exactly |
| `autoadd` disable: same "cannot nest" message, same non-empty-stack guard (`autoadd.rs:17-18,20-22`) | preserved exactly, including the duplicated message |
| `cmd+` — `::`, `;;`, `:;` are single command tokens resolving to nothing | preserved exactly |
| `atom`'s byte slice `[1..len-1]`, safe only because the terminator was ASCII whitespace | **must be re-derived** under D1's widened terminator set |
| `Bund::run` pulls workbench-first-then-stack and prints a result (`bundcore_run.rs:6-22`) | preserved; D6's "one evaluator" must keep the CLI `eval` subcommand's return-value behaviour |
| Conditionals as values; missing slot defaults to empty lambda | preserved exactly |
| `CF` as a global mutex | **changed** to registry state; lookup and errors identical (D7) |
| `context` leaving the wrong stack on error | **fixed** (F57) — structural, via exit actions |
| Two evaluator copies | **collapsed to one** (F52); behaviour identical |
| Rust recursion for lambda/list/map | **removed** (D4); observable only as programs that no longer overflow |
| Errors as strings without position | **extended** with spans; text preserved |
| `$` alias for `take` | **not ported** (F56); it is unreachable in the reference |

**One claim in the row above needs retracting.** An earlier draft said the four
fixes (F52, F53, F56, F57) are "widenings or internal — none changes an answer a
passing golden already captures". That holds for F52, F56 and F57. It does not
hold for F53: correcting the guard makes `execute.`'s LIST and MAP arms
reachable, and those arms are wrong for the workbench — the LIST arm pushes onto
the **main** stack (`reference/rust_multistackvm/src/stdlib/execute.rs:41`) and
then recurses still pulling from the workbench (`:42`), and the MAP arm takes its
key from the main stack (`:56`) whatever the operand says. The wrong guard is
currently shielding them. Recorded as **F59** and resolved in D4b: the recursion
passes `FromStack`, so only the receiver comes from the workbench. F53 and F59
land together or not at all.

**The error-text preservation in D5 collides with F14.** The eval loop
interpolates the offending word with `{:?}` (`bundcore_eval.rs:33`), and F14
records that this text embeds `id` and `stamp` and therefore cannot reproduce.
What is preserved is the **wrapping shape**, not the interpolated value.

F49/F50/F51 are preserved *because* changing them would change what existing
programs mean.

## Alternatives considered

**BundIR as the lambda body, replacing `Vec<BundValue>`.** Rejected. `compile`
returns a LIST of values and `lambda!` retypes it; `lambda*` folds the stack.
Bund programs construct bodies, so the body must be a value. An IR that is not
a value could only be a cache, which is what D3 makes it.

**Keeping Rust recursion and raising the stack size.** Rejected. It defers the
overflow rather than removing it, and it forecloses the debugger and async work
that RFC-0008 and RFC-0007 both need a materialised frame stack for.

**Treating `?ifthenelse` and `?try` as structured control flow the compiler can
see.** Rejected on the evidence. They are MAPs assembled at runtime by `set`
and dispatched by a string lookup; the branches are not known until execution.
Literal `if`, `times`, `while` and `loop` remain statically analysable and are
lowered as control flow; the `?`-family is a helper call. This refines §1.1
rather than contradicting it.

**Dropping the trailing-whitespace requirement.** Rejected, after a draft of
this RFC proposed it. It reads like a vestigial quirk and is not one: `}` is
not whitespace, so `{ 1 println}` fails to parse in the reference, and dropping
the rule would accept it. Admitting end-of-input as a terminator gets the
intended result — no `\n` append, no change to the accepted language — where
dropping the rule silently widens it.

**Porting the `$` alias.** Rejected — F56. It cannot fire in the reference, so
porting it would add a word the language does not have.

## Acceptance criteria

Each names the tool that decides it. Where no tool exists, that is stated as
work rather than assumed.

1. **No conformance regression, and a stated parse-reach number.** `cargo xtask
   conform` must not drop below the recorded baseline in
   `tests/golden/CONFORMANCE.txt`. Separately — because `conform` compares
   captured bytes (`xtask/src/conform/mod.rs:150`) and has no parse-only mode —
   this RFC adds `cargo xtask conform --parse-only`, reporting how many of the
   69 goldens' sources parse without error. That number must rise from its
   pre-RFC value to **at least 42**, the count of goldens using `{` or `}`
   outside string literals. It is not a conformance claim: those goldens also
   need `set`, `!`, `register` and `format`, which this RFC does not implement.

   *(An earlier draft said "42 of 69 goldens" without stating the measure, and a
   reviewer reading it as brace characters anywhere got 49. The seven-golden gap
   is `format` template placeholders inside string literals — `{answer}`, `{A}` —
   which are consumed by `leon` at runtime and are not block syntax.)*

2. **Bund call depth is bounded by heap, not by the Rust stack.** A
   self-recursive Bund word at depth 100,000 either completes or reports a
   Bund-level error, within a 60-second wall clock, without a stack overflow.
   Scoped to *call* depth: parser nesting depth is excluded by D4 and is not
   tested here.

3. **The parser accepts exactly the reference's language.** A differential
   harness — `cargo xtask parity`, which does not exist and is part of this
   RFC's work — parses each corpus program through both front ends and compares
   the emitted value streams element by element under the golden normaliser,
   which erases `id` and `stamp` (F14). Without that normalisation the
   comparison is unsatisfiable, since every `Value` carries a fresh identity.
   The 69 golden-backed programs compare against captured streams; the remaining
   63 require a live oracle run and therefore inherit F21.

4. **The three grammar traps are pinned against the oracle**, as probes under
   `tests/probes/` per D21: `1_000` errors, `007` yields three integers, `{}`
   and `[]` fail to parse, and `{ 1 println}` fails while `1 2 +` with no
   trailing newline succeeds.

5. **F53, F57 and F60 are fixed and observable.**
   - **F53** — `execute.` succeeds with a value on the workbench and an empty
     stack, and **F59** is answered: `execute.` on a LIST and on a MAP is
     specified and tested, not left to the exposed arms.
   - **F57** — a `context` whose lambda raises inside a `?try` leaves the
     interpreter on the stack it started from, asserted via `current_name` on
     the VM rather than via a word, since no word this RFC cites prints the
     current stack name.
   - **F60** — a bare `endcontext` with no context open fails with `Context is
     empty` and leaves the current stack intact. Asserted against the recorded
     deviation rather than against the oracle, whose behaviour here is silent
     destruction.
   - **D34** — `:F { ( 7 ) 9 } register` registers `F`, and a lambda containing
     a `( … )` opens and closes its own context on every call, leaving the
     caller's stack unchanged across two invocations.

6. **One evaluator.** A test that a behavioural change made at the single entry
   point is visible from both `bund.eval` and the script path. The earlier
   "checkable by grep" is dropped — a duplicated loop has no literal to match.

7. **`cite` and `lint` clean, with the load-bearing citations quoted.** `cargo
   xtask cite`'s only exact check is on fenced blocks whose info string carries
   a citation (`xtask/src/cite/mod.rs`); a document with none passes it while
   citing wrong lines, which is how four bad line numbers survived this RFC's
   first draft. Before acceptance, the citations this RFC's design rests on —
   `execute.rs`'s dispatch arms, `ctx.rs`'s push-into-state, `bundcore_eval.rs`'s
   loop, `autoadd.rs`'s guards — must be quoted as cited fenced blocks so `cite`
   verifies their content and not merely their existence.

## Open questions

- **F54 is a citation hazard, not just dead code.**
  `reference/rust_multistackvm/src/stdlib/execute_types/execute_object.rs`
  declares `execute_object` over a body byte-identical to
  `execute_conditionals`. A citation into it resolves and returns the wrong
  subsystem, and `cite` cannot catch that — the path and line exist.
- **Q21** — how the frame loop reproduces the nested `bail!` concatenation that
  `?try` files into its `context` slot. Registered.
- **Q22** — the compiled-cache promotion threshold and cap, which D3's amended
  resolution presumes and RFC-0005 owns. Registered.
- **F48 applies to criterion 1.** `conform` cannot express an approved
  deviation, so the baseline it is measured against is understated by D30's two
  existing deviations.
- **D8 depends on an unfinished part of RFC-0002.** The inline cache keys on
  `(Symbol, generation)`, and RFC-0002 records its generation overflow policy as
  unresolved. D8 also has two consequences not yet analysed: it removes the only
  alias resolution `$name` gets (F26 — `apply` skips the alias step, but
  `call_internal_word` calls `i()`, which resolves), and every curried word ends
  in a `Value::call("!")` (`conditional_curry.rs:51`), an alias, so its last call
  site must invalidate on an alias-table generation bump.
- **RFC-0000 assigns `bund2-ir` a role this RFC redraws.** RFC-0000:180 gives it
  "AST to BundIR; the only crate that defines the instruction set". Here the
  pipeline is AST → `Vec<BundValue>` → optional BundIR cache, and which crate
  owns the cache is unstated. That is an architectural change and belongs stated
  against RFC-0000 rather than inferred.
- **Lowering is assumed total and information-preserving.** Criterion 3 depends
  on it; D2 introduces an AST node whose nested lowering is undecided. Nothing
  yet states that lowering is a function.
- **The read behind this RFC is complete for the front end and dispatch core,
  and partial for the word vocabulary.** `conditional/`'s `csv` and `sqlite`
  handlers (372 lines) were surveyed for shape, not read; `values/`'s `merge`,
  `unfold`, `listop` and `sort_lists` were enumerated by registration; and
  `execute_class` and `execute_object` have since been read, closing F59's scope
  and removing that gap. Neither re-enters evaluation, so D4's frame model is
  unaffected by them. What remains unread constrains RFC-0004's effect table,
  not the IR's shape.
