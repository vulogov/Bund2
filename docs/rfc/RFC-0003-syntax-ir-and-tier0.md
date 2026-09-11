# RFC-0003: Surface syntax, BundIR, and the Tier 0 interpreter

- Status: **Accepted** (2026-09-04), **amended 2026-09-10** — two passages of
  §S3 written while D35 was open are superseded by its resolution; see the
  amendment at the end and the marker above each. A second amendment the same
  day puts S4's frame on the body's value (D42) and the cache's reference on a
  `Weak` (D35 as amended), and a third puts a floor under native-mediated
  nesting (F85). All seven criteria re-run at acceptance and
  each has *grown* rather than merely held: parse-reach is **82/82** where the
  criterion pinned 69, `cargo xtask parity` is **51/51** where it was 47, and
  `cargo xtask depth` still completes all three axes — call at 100,000, nesting
  at 10,000, class reporting a Bund-level error at 10,000.
- Previously: **Proposed** (2026-09-01), after three reviews. **All seven
  acceptance criteria are met and every one of them runs** — parse-reach 69/69,
  call depth 100,000, parity 47/47, the grammar traps pinned, F53/F57/F59/F60
  and D34 implemented, one evaluator behind an observer, and six exact-match
  fenced blocks verified by `cite`.

  That is a different footing from RFC-0001 and RFC-0002, which are Proposed
  because their criteria **cannot** run — the code they describe does not exist.
  These do run, and pass, against `conform`, `parity`, `depth` and `cite`.

  Proposed rather than Accepted for one reason: RFC-0000's bar is a review pass
  that finds nothing, and this RFC's third pass found four blockers — a
  contradiction between S1 and S7, a preservation row that inverted how a PTR
  slot behaves, an identity claim that contradicted its own `set` row, and a
  criterion whose grep matched text the tool never prints. Three of those four
  were introduced by the response to the previous review, which is why the
  owner directed that implementation, not a fourth review, decide the design.
  Implementation has now agreed with the design at every point it could
  disagree.

  What remains open is listed below and none of it gates the design: two
  questions belong to RFC-0002 and RFC-0005, one is an amendment RFC-0000 should
  carry, and F62 is a hazard the reference shares.
- Depends on: RFC-0001 (the value), RFC-0002 (symbols and the word table)
- Decisions consumed: D3 (no eval-specific tier rule, as amended 2026-08-29),
  D5 (lambda bodies are write-once, so the compiled cache needs no
  invalidation), D11 (no external dependents of `compile_to_binary`; version the
  IR format freshly), D16 (the world is permanently open, so a call target may
  be a name computed at runtime), D34 (`( … )` lowers in place), D35 (the cache
  keys on the body's `Rc` pointer). D13 and D20 are relied on and D27 informs;
  none of those three is consumed.
- Blocked on: nothing. D34 resolves `( … )` lowering, D35 resolves the cache
  key, and F59 is dispositioned. F62 and S6's observer interface remain
  unresolved design questions inside this RFC, not blockers on it.
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
(`reference/Bund/src/stdlib/helpers/eval.rs:7-41`) are the same loop,
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
a program ending in a word does not parse. There are **five** non-test callers of
`bund_parse` and **four** append `\n` first — `bundcore_eval.rs:8`,
`helpers/eval.rs:8`, `bund_interpreter.rs:29`, and the debugger's
`debug_debug.rs:52`. The fifth, the library entry point
`bund_language_parser/src/compile.rs:15`, does **not**, so it fails on exactly
the input the other four are compensating for.

`helpers/run_snippet.rs` is not itself a parse site but appends at five places
(`:15,62,100,138,162`) before delegating to `Bund::eval`, which appends again —
so the CLI paths append **twice**. The workaround is undocumented,
load-bearing, duplicated on one path and absent on another.

**Control flow is partly data, and the dispatch table for it is global
mutable state.** `?ifthenelse` and `?try` push CONDITIONAL values whose
branches are lambdas in named slots
(`reference/Bund/src/stdlib/functions/conditional/conditional_ifthenelse.rs:8-10`),
executed by looking the conditional's `type` string up in `CF`, a
`Mutex<BTreeMap<String, ConditionalFn>>`
(`reference/rust_multistackvm/src/stdlib/execute_types/mod.rs:11-16`). That is
a fourth live dispatch table. RFC-0000's "three dispatch tiers, three live
tables" (`docs/rfc/RFC-0000-architecture.md:135`) counts word registration only
and remains correct on its own terms.

## Current behaviour

### 1. The grammar is 54 lines and admits twelve value forms

`value` is an ordered choice of `float`, `integer`, `lambda`, `list`, `ctx`,
`ptr`, `name`, `command`, `atom`, `stack`, `string`, `literal`
(`reference/bund_language_parser/bund.pest:7-20`). Order is significant: `ptr`
precedes `name` (`:13-14`), and `name` carries an explicit `` !("`") `` guard
(`:28`).

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
| `name` | `Value::call(name)` | `vm/name.rs:7-10` |
| `command` | `Value::call(":")` / `(";")` | `vm/command.rs:7-9` |
| `atom` `:foo` | `Value::from_string("foo")` | `vm/atom.rs:7-10` |
| `ptr` `` `foo `` | `Value::ptr("foo")` | `vm/ptr.rs:7-10` |
| `stack` `@main` | `Value::named_context("main")` | `vm/stack.rs:7-10` |
| `lambda` `{…}` | `Value::to_lambda(terms)` | `vm/lambda.rs:8-20` |
| `list` `[…]` | `Value::from_list(terms)` | `vm/list.rs:8-20` |
| `EOI` | `Value::exit()` | `vm/eoi.rs:7-9` |

`atom` slices `[1..len-1]` then trims, dropping the leading `:` and the
trailing whitespace the grammar required.

**An atom is interchangeable with a string.** `:foo` and `"foo"` both produce a
STRING and nothing downstream distinguishes them
(`reference/bund_language_parser/src/vm/atom.rs:7-10`); the atom is a surface
form whose content is any run of characters without whitespace. The grammar does
not implement that — `aelement` admits only
`ASCII_ALPHANUMERIC | LETTER | "." | "_"` (`bund.pest:38`) where `element`
additionally admits `-` and sixteen others (`:36`) — so `foo-bar` is a legal word
name while `:foo-bar` is a parse error, and the `:name { … } register` idiom
cannot name it. Recorded as **F63**.

**`ctx` is the exception, and it is F9.** `( … )` does not return a tree. It
pushes `Value::context()` into the shared output vector, parses its inner terms
pushing each into that same vector, and returns `Value::call("endcontext")`
(`reference/bund_language_parser/src/vm/ctx.rs:8-20`). The vector it mutates is
the one `bund_parse` is building (`reference/bund_language_parser/src/lib.rs:16,21-23`),
so `( a b )` flattens to four stream elements. Two of the three bracket forms
nest; the third is a side channel.

Every parse ends with an EXIT value, from the `EOI` handler.

### 3. The evaluator, three times

`Bund::eval` (`reference/bundcore/src/bundcore_eval.rs:7-45`),
`bund_compile_and_eval` (`reference/Bund/src/stdlib/helpers/eval.rs:7-41`) and the
debugger's loop
(`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:52-95`) are the
same loop. The first two are identical; the third prints each word before
applying it (`:74`) and runs a readline loop after every word (`:81-95`).

**The third is the strongest argument in the reference for a materialised frame
stack.** A per-word step loop is what a frame stack gives for free; the debugger
exists as a duplicated interpreter precisely because the interpreter has no
steppable state.

All three do: append `\n`, `bund_parse`, then per element — `NONE` continue,
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
cross-crate half is what makes S7's "populated at construction" non-trivial.

`!` on a CONDITIONAL reads `type`, looks it up, and calls the handler
(`reference/rust_multistackvm/src/stdlib/execute_types/execute_conditionals.rs:9-25`).

Branches are lambdas in named slots, populated by `set` between construction
and execution, and **a missing slot defaults to an empty lambda** rather than
erroring (`conditional_ifthenelse.rs:15-26`). `?try` additionally reifies a
caught failure: it builds an `error` conditional carrying `context` (the error
string) and `associated`, pushes it, then runs `except` and `recovery`
(`conditional_tryexcept.rs:32-49`). Errors are `easy_error::Error` — a string
context, with no position.

`curry` is code generation: it builds a lambda from the captured data — pushed
in **reverse**, `data.into_iter().rev()` — followed by the target lambda and a
`CALL "!"`, then registers it under a name (`conditional_curry.rs:44-53`). The
`!` is an alias, so a curried word's last instruction is a call site S8's cache
must invalidate on an alias-table generation bump.

### 6. Programs are data, and that is the idiom

`compile` parses a string into a **LIST of values**, dropping the trailing EXIT
(`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:29-43`).
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

`{` or `}` appears outside string literals in 42 of the 69 captured goldens —
37 of the 57 suite goldens and 5 of the 12 probes. That is the largest single
count among unimplemented constructs measured this way; it is not a claim about
which construct is hardest, since every one of those 42 needs other words too.

## Design

**Namespace note.** Sections here are numbered **S1–S8**. Register decisions are
`D1`–`D35` in `docs/registers/decisions.md` and are always written `D<n>` with a
description. An earlier draft numbered these sections `D1`–`D8`, so a sentence
could contain both namespaces — "as D5 resolved" inside a section itself called
D3 — and that collision is what produced the first review's missed decision.


### S1. One grammar, one parser, stated traps

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
- `1 2 +// add` lexes `+//` as one **name** (F61). `element` admits `/`
  (`bund.pest:36`) and pest applies `COMMENT` between tokens (`:54`), so a
  comment marker abutting a word is swallowed into it. The oracle answers
  `Inline +// not registered` with `1` and `2` still unadded — an error naming
  a word the programmer never wrote.

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
by appending `\n` at four of its five parse sites. Under it, `1 2 +` with no
trailing newline parses, exactly as the reference parses it after the append;
`{ 1 println}` still fails, exactly as the reference fails it. The set of
accepted programs is unchanged. What changes is that the `\n` append stops
being load-bearing and disappears from the call sites.

### S2. Scoped blocks — `( … )` lowers in place

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

Under S4 this falls out rather than being added: a context is a frame with an
exit action, the same mechanism that fixes F57.

F9 closes at the representation; D34 closes the behavioural half.

**`endcontext` is a shadowable name, and D34 makes every `( … )` depend on it.**
`apply` consults the lambda table before the inline table
(`reference/rust_multistackvm/src/multistackvm_apply.rs:46,59`), so
`:endcontext { … } register` captures the closing half of every parenthesis in
the program. Confirmed against the oracle: the shadow runs, the context is
opened, and it is never closed. The reference emits the same call and has the
same hazard, so D34 does not introduce it — but D34 makes it uniform, so this
RFC cannot leave it unsaid. Recorded as **F62** and open: lowering to an opcode
the word table cannot intercept is cleanest, and it removes a hook that D16's
permanently-open world otherwise grants.

### S3. BundIR, and what stays a value

A lambda body **stays `Vec<BundValue>`**. This is not a concession; it is
forced. `compile` yields a LIST of values, `lambda!` retypes a LIST as a
LAMBDA, `lambda*` folds the live stack into one, and `curry` assembles one
element by element. Any representation Bund code cannot build is not this
language's representation.

BundIR is therefore a **cache over** a body, never the body itself:

- keyed on the body's **`Rc` pointer**, as **D35** resolves.
  `BundValue::Heap` holds `payload: Rc<Payload>` and a lambda's payload *is*
  its body, so the pointer names the compilation unit directly. The cache holds
  a strong clone, or a freed body's address could be reused and a stale entry
  become a false hit.

  D5 supplies the reason no invalidation is needed: lambda bodies are
  write-once — `set` on a LAMBDA returns a new value
  (`reference/rust_dynamic/src/set.rs:11-13`) and `push` converts to LIST first
  (`reference/Bund/src/stdlib/functions/values/push.rs:34`) — so a changed body
  is a new `Rc` and a stale entry is unreachable. D5 phrased that for
  *identity* keying; D35 found identity untenable, because it would mint on the
  hottest path (a materialisation point D20 does not list) and could never hit
  for a `dup`'d body. `dup` shares the payload, so pointer keying hits where
  identity keying misses;
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

> **Superseded 2026-09-10** — see *Amendment, 2026-09-10* at the end. Written
> while D35 was open. No cache here is identity-keyed, and D3 rests on a
> different ground.

D3's **amended** resolution follows from identity keying rather than from
content hashing, and the amendment reverses the original reasoning: eval'd code
re-parses and mints fresh values on every call, so it cannot hit an
identity-keyed cache at all and stays at Tier 0 without any rule. The conclusion
— no eval-specific tier rule — is unchanged.

> **Superseded 2026-09-10** — see *Amendment, 2026-09-10* at the end. This
> paragraph and the next predate D35's resolution. D35 is RESOLVED — the body's
> `Rc` pointer, stated in this section's first bullet — and nothing here is
> blocked.

**D35 blocks this section.** Two facts D5 did not weigh make identity keying
untenable as stated. **It is an unlisted materialisation point**: D20 enumerates
where lazy identity ends — `save.*`, `compile`, `wrap` — and executing a lambda
is not on that list, so an identity-keyed cache materialises an identity on the
hottest path in the language. **And `dup` makes it miss**: F13's disposition is
a structural clone *plus fresh identity*, and `dup` is 55 invocations across 38
of 132 programs, so a dup'd lambda can never hit the cache for its original.

D5 is not wrong; it answered whether the cache can go **stale**, and identity
keying cannot. It did not answer whether the cache can **hit**. D35 carries the
question, recommends content keying with `id` and `stamp` excluded from the
hash, and is OPEN — so this section states the shape and not the key.

**D11 is discharged here.** D11 resolves "no external dependents of
`compile_to_binary`; version the IR format freshly." Under this design BundIR is
an in-process cache with no serialised form, so there is no format to version.
If a later RFC serialises it — for AOT (RFC-0006) or an image (RFC-0010) — D11's
"version freshly" applies at that point and this RFC's silence must not be read
as a format decision.

### S4. The flat frame loop

Tier 0 is one loop over an explicit frame stack. No Rust recursion **for Bund
call depth** — a bound that needs stating precisely, because an earlier draft
claimed "no Rust recursion, anywhere on the evaluation path" and that is not
what this design delivers.

Three depth axes exist and only the first is addressed here:

- **The parser still recurses.** `parse_pair` descends through `lambda.rs:11`,
  `list.rs:11` and `ctx.rs:11`, so deeply nested brackets overflow at parse
  time, not at call time. Bund2's parser inherits that shape. Nesting depth and
  call depth are different limits and only the second is addressed here.
- **Class-hierarchy depth is a separate Rust recursion.** `make_bund_object`
  calls itself for each superclass
  (`reference/rust_multistackvm/src/stdlib/bund_object.rs:50`), so a deep class
  chain overflows independently of call depth. Flattening it is RFC-0009's
  concern (flattened per-class vtables); this RFC only names it.
- **Native words that re-enter evaluation** need a mechanism, described below;
  without one the `?`-family — and object and class construction — reintroduce
  exactly the recursion this section removes.

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
(`conditional_ctx.rs:60-74`). As a frame with an exit action, the restore runs
on both paths. The reference's shape cannot express this; the flat loop does so
structurally rather than as a patch.

### S4a. Context depth is represented, so `endcontext` can refuse

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

### S4b. `execute.` takes only its receiver from the workbench

F53 corrects `execute.`'s wrapper, which guards the main stack
(`reference/rust_multistackvm/src/stdlib/execute.rs:117`) and then pulls the
workbench (`:22`). That fix alone is not safe, because it exposes two arms with
no coherent workbench behaviour — **F59**.

`op` is threaded through the *receiver* pull and no *push* site — and not
through every pull either: the MAP arm takes its key from the main stack
regardless (`:56`), which under the convention below is correct. The LIST arm
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
proceeds exactly as `execute`** — a statement about which stack each operation
touches, not about message text: the `EXECUTE.` error prefix stays distinct from
`EXECUTE` (`reference/rust_multistackvm/src/stdlib/execute.rs:113,120`). Read
literally the phrase would merge them, which no golden would forgive. the recursion passes `FromStack`, because by
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

**They do, however, re-enter evaluation, one call deeper than those two files.**
`execute_object` ends in `vm.m`, whose LAMBDA arm is `lambda_eval`
(`reference/rust_multistackvm/src/multistackvm_object.rs:77-78`).
`execute_class` delegates to `stdlib_object_inline` → `make_bund_object`, which
**recurses over the superclass chain**
(`reference/rust_multistackvm/src/stdlib/bund_object.rs:50`), evaluates each
`.init` lambda (`:76`, `:156`), then inspects the stack and `apply`s
(`:166,169,173`). That is S4's evaluate-inspect-evaluate shape, so both are
re-entrant natives under S4a's request/resume rule — and class construction adds
a **third depth axis**, hierarchy depth, alongside call depth and parser nesting
depth. An earlier draft of this RFC asserted the opposite in its closing note.

This does not change F59's scope, which is about the push/pull split and remains
two arms.

### S5. Errors carry position

Errors become a structured value carrying a span, not an `easy_error::Error`
string. The reference's error text is preserved verbatim where a golden
captures it — the wrapping shape `Attempt to evaluate value {:?} returned
error: {}` included — and the span is additional, not a replacement.

`?try` continues to reify a caught error as an `error` conditional with a
`context` slot (`conditional_tryexcept.rs:35-40`); the slot's string is
unchanged, and the span rides alongside.

### S6. One evaluator, parameterised by an observer

F52's duplication does not survive. `bund2-interp` has a single evaluation entry
point used by all three of today's callers: the top-level script path,
`bund.eval`, and `--debugger`.

The first two are identical, so collapsing them preserves what both do. The
third is not — it prints and it steps — so the single evaluator takes a per-word
**observer**: nothing for the two silent callers, print-and-step for the
debugger. That reproduces all three without three loops.

An earlier draft called this a pure non-deviation on the grounds that "the two
copies are currently identical". There are three, and the third differs; the
collapse is still behaviour-preserving, but because of the observer, not because
the copies agree.

### S7. Conditional dispatch is a registry, not a global mutex

The `CF` table becomes a field on the registry RFC-0002 already owns, keyed by
type string, populated at construction. Same lookup, same failure message
(`EXECUTE:CONDITIONAL conditionals handler does not exist: {type}`), no global
mutable state. The `BUND` global mutex went the same way in RFC-0002 and for
the same reason (`docs/rfc/RFC-0002-symbols-and-words.md:537`).

### S8. Inline caches on call sites

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
| `endcontext` shadowable by a registered lambda | preserved for now — **F62**, open |
| `endcontext`'s conditional carry-out: top value to the workbench if the stack is non-empty | preserved exactly; S4's exit action must reproduce it — `( 7 ) take` yields `7` |
| `execute.`'s error prefix `EXECUTE.` distinct from `EXECUTE` | preserved exactly; S4b's "proceeds exactly as `execute`" governs stack selection, not message text |
| `:atom`'s character class narrower than `name`'s | **deliberately changed** — F63, widened to any non-whitespace run |
| `+//` lexing as one name | preserved, and stated (F61) |
| The debugger's third eval loop, which prints and steps | preserved via S6's observer |
| `execute_class` / `execute_object` re-entering evaluation | preserved; both are re-entrant natives under S4a |
| `make_bund_object` recursing over the superclass chain | preserved; a third depth axis, flattening deferred to RFC-0009 |
| `apply`'s resolution order, `autoadd` semantics, non-nestability | preserved exactly |
| `execute`'s eight arms and their errors | preserved exactly |
| `execute.` guarding the wrong stack | **fixed** (F53), but *not* widening only — see F59 below |
| `execute.` on a LIST or a MAP | **deliberately changed** — F59; recursion passes `FromStack`, so only the receiver comes from the workbench |
| `through`, the ninth conditional type, registered cross-crate | preserved; D7 must carry it |
| `raise`, registered with the family but pushing nothing | preserved exactly |
| `autoadd` disable: same "cannot nest" message, same non-empty-stack guard (`autoadd.rs:17-18,20-22`) | preserved exactly, including the duplicated message |
| `cmd+` — `::`, `;;`, `:;` are single command tokens resolving to nothing | preserved exactly |
| `atom`'s byte slice `[1..len-1]`, safe only because the terminator was ASCII whitespace | **must be re-derived** under S1's widened terminator set |
| `Bund::run` pulls workbench-first-then-stack and prints a result (`bundcore_run.rs:6-22`) | preserved; S6's "one evaluator" must keep the CLI `eval` subcommand's return-value behaviour |
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
currently shielding them. Recorded as **F59** and resolved in S4b: the recursion
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
lowered as control flow; the `?`-family is a helper call. This **restates**
§1.1 rather than refining it, which is why the header supersedes nothing.

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
   captured bytes (`xtask/src/conform/mod.rs`) and has no parse-only mode —
   this RFC adds `cargo xtask conform --parse-only`, reporting how many golden
   sources parse without error.

   **Measured against a fixed denominator: the 69 goldens as of this RFC — 57
   suite plus 12 probes.** Of those, 42 use `{` or `}` outside string literals
   (37 suite, 5 probes), and that is the number `--parse-only` must reach.

   Criterion 4 adds five probes, which would otherwise move both figures — a
   denominator of 74 and a brace count of 44 — and **two of its probes are
   required not to parse** (`{}` and `{ 1 println}`). Counting them as
   parse-reach failures would make criteria 1 and 4 contradict each other, so
   `--parse-only` reports against the fixed set and lists deliberately-rejecting
   probes separately.

   It is not a conformance claim: those goldens also need `set`, `!`, `register`
   and `format`, which this RFC does not implement.

   *(An earlier draft said "42 of 69 goldens" without stating the measure, and a
   reviewer reading it as brace characters anywhere got 49. The seven-golden gap
   is `format` template placeholders inside string literals — `{answer}`, `{A}` —
   which are consumed by `leon` at runtime and are not block syntax.)*

2. **Bund call depth is bounded by heap, not by the Rust stack.** Decided by
   `cargo xtask depth`. **Met**: the call axis completes at 100,000. Before the
   frame loop it aborted between 5,000 and 20,000, so the criterion failed
   before the work and passes after — which is what made it worth having. It it runs a self-recursive Bund word
   at depth 100,000 in a subprocess and asserts the process neither overflows
   nor exceeds a 60-second wall clock, reporting completion or a Bund-level
   error as a pass. Scoped to *call* depth — parser nesting depth and
   class-hierarchy depth are the other two axes S4 names, and neither is tested
   here.

3. **The parser accepts exactly the reference's language.** A differential
   harness — `cargo xtask parity`, which does not exist and is part of this
   RFC's work.

   An earlier draft made this unsatisfiable by comparing "value streams element
   by element" against captured goldens. No golden holds a value stream: all 69
   carry exactly `## exit` and `## output`. And the golden normaliser is a text
   substitution, not a `Vec<Value>` comparator.

   The mechanism that makes it satisfiable is already in the language.
   **`compile` is the oracle's dump mode** — it parses a string and pushes the
   token stream as a LIST
   (`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:29-43`), so
   `"<source>" compile debug.display_stack` renders the reference's own stream
   in the same `Value { … }` text the normaliser already handles. No
   instrumentation of `reference/` is required, and `reference/` stays
   read-only.

   So: for each corpus program, render the reference's stream that way, render
   Bund2's the same way, and compare the **normalised text**. Five constraints
   follow and must be honoured rather than discovered.

   - `compile` drops the trailing EXIT (`:33-35`), so the comparison excludes it.
   - Source is passed as a Bund string literal, so embedded quotes need escaping.
   - The run goes through the oracle binary, so it inherits F21.
   - **The golden normaliser is not sufficient.** It rewrites `id:` and `stamp:`
     (`xtask/src/golden/mod.rs`).
     But `Value::context()` puts a fresh `nanoid!()` in **`data`**, not in `id`
     (`reference/rust_dynamic/src/create_special.rs:28`). Two oracle runs of
     `"( 4 )" compile debug.display_stack` therefore differ after normalisation
     — measured. The parity harness needs its own rule erasing a CONTEXT value's
     payload, or every program containing `( … )` is permanently unstable.
   - **Nested `( … )` is expected to differ.** D34 approves lowering in place, so
     Bund2's stream for a nested context is deliberately not the reference's.
     Those programs compare only their *top-level* streams; the nested case is
     covered by the D34 tests in criterion 5, not here. Without this carve-out
     the criterion contradicts an approved deviation.

4. **The grammar traps are pinned against the oracle.** Two are probes under
   `tests/probes/` per D21 and both pass: `grammar-leading-zero` (`007` is three
   integers, F50) and `grammar-terminator` (a word at end of input parses, §S1).

   **The rejecting traps cannot be probes, and the reason is F66.** `1_000`,
   `{}`, `[]`, `{ 1 println}` and `+//` all *fail*, and a golden of a failure
   captures the oracle's error report — whose `Location` row holds an absolute
   path into the capture machine's `~/.cargo/registry`, which no other machine
   reproduces. `xtask parity` cannot cover them either, since it compares the
   parse output of programs that parse. They are pinned by test in
   `crates/bund2-syntax/src/lib.rs`, each against an oracle observation recorded
   in its doc comment.

   An earlier version of this criterion asked for all of them as probes. That
   was not achievable, and saying so is better than a probe that pins a path.

5. **F53, F57 and F60 are fixed and observable.**
   - **F53 and F59 — met.** `execute.` succeeds with a value on the workbench
     and an empty stack, and only the *receiver* comes from the workbench, so a
     LIST's elements execute from the main stack rather than the arm pushing to
     one stack and reading another. Both tested.
   - **F57** — a `context` whose lambda raises inside a `?try` leaves the
     interpreter on the stack it started from. Asserted through Bund2's `Vm`
     trait, whose accessor is `current_name`; the reference spells the same
     thing `TS::current_stack_name`
     (`reference/rust_multistack/src/ts_current.rs:6`) — the assertion is on
     Bund2's API, so no reference spelling is implied.
   - **The carry-out survives.** `( 7 ) take` yields `7`: S4's exit action
     reproduces `endcontext`'s conditional move of the top value to the
     workbench, not merely the stack drop.
   - **F60** — a bare `endcontext` with no context open fails with `Context is
     empty` and leaves the current stack intact. Asserted against the recorded
     deviation rather than against the oracle, whose behaviour here is silent
     destruction.
   - **D34** — `:F { ( 7 ) 9 } register` registers `F`, and a lambda containing
     a `( … )` opens and closes its own context on every call, leaving the
     caller's stack unchanged across two invocations.

6. **One evaluator, three callers.** A test that a behavioural change made at
   the single entry point is visible from **all three** of the script path,
   `bund.eval`, and `--debugger` — the third being the one S6's observer exists
   for. The earlier "checkable by grep" is dropped: a duplicated loop has no
   literal to match, and the earlier "both callers" predated finding the third
   copy.

7. **`cite` and `lint` clean, with the load-bearing citations quoted.** `cargo
   xtask cite`'s only exact check is on fenced blocks whose info string carries
   a citation (`xtask/src/cite/mod.rs`); a document with none passes it while
   citing wrong lines, which is how four bad line numbers survived this RFC's
   first draft. **Met** — the five this design rests on:

   ```pest reference/bund_language_parser/bund.pest:48
int     = @{ "0" | (ASCII_NONZERO_DIGIT ~ digits?) }
   ```

   ```pest reference/bund_language_parser/bund.pest:31
lambda  = { "{" ~ term+ ~ "}" }
   ```

   ```rust reference/bund_language_parser/src/vm/string.rs:8
    let the_str: &str = &t.as_str()[1..t.len() - 1];
   ```

   ```rust reference/bund_language_parser/src/vm/ctx.rs:9
    state.push(Value::context());
   ```

   ```rust reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:13-14
                        for v in lambda_content {
                            match self.apply(v) {
   ```

   In order: F50's decomposing integer, F51's `term+`, the raw-text string
   slice that showed escapes are never translated, F9's push-into-the-output
   side channel, and the recursion §S4 replaces.

## Open questions

- **D35 is resolved** (the compiled cache keys on the body's `Rc` pointer), so
  S3 states the key rather than deferring it. Content keying stays available as
  a strict upgrade if a workload ever shows structurally identical lambdas
  worth sharing.

- **F62 blocks nothing but is unresolved.** `endcontext` is shadowable and D34
  routes every `( … )` through it. Three shapes are available — an
  un-interceptable opcode, refusing registration over the name, or documenting
  the hazard — and the first conflicts with D16's permanently-open world.
- **The `--debugger` path is now in D6's scope.** Collapsing three loops into
  one plus an observer is specified but untested; the observer's interface is
  not designed here.
- **F54 is a citation hazard, not just dead code.**
  `reference/rust_multistackvm/src/stdlib/execute_types/execute_object.rs`
  declares `execute_object` over a body byte-identical to
  `execute_conditionals`. A citation into it resolves and returns the wrong
  subsystem, and `cite` cannot catch that — the path and line exist.
- **Q21** — how the frame loop reproduces the nested `bail!` concatenation that
  `?try` files into its `context` slot. Registered.
- **Q22** — the compiled-cache promotion threshold and cap, which D3's amended
  resolution presumes and RFC-0005 owns. Registered.
- **F48 is fixed**, so criterion 1's denominator is no longer understated by
  approved deviations: `tests/golden/DEVIATIONS.txt` records them with a hash
  of the output Bund2 must keep producing, and `conform` counts them apart from
  the ratio.

- **S8 depends on an unfinished part of RFC-0002.** The inline cache keys on
  `(Symbol, generation)`, and RFC-0002 records its generation overflow policy as
  unresolved. S8 also has two consequences not yet analysed: it removes the only
  alias resolution `$name` gets (F26 — `apply` skips the alias step, but
  `call_internal_word` calls `i()`, which resolves), and every curried word ends
  in a `Value::call("!")` (`conditional_curry.rs:51`), an alias, so its last call
  site must invalidate on an alias-table generation bump.
- **RFC-0000's `bund2-ir` row is amended**, not left to be inferred. Its
  boundary rule now reads `Vec<BundValue>` to BundIR, optional, with Tier 0
  never requiring it, and names `bund2-syntax` as lowering's home. The
  amendment is appended beside the original row because RFC-0000 is Accepted.

- **Lowering is assumed total and information-preserving.** Criterion 3 depends
  on it; S2 introduces an AST node whose nested lowering is undecided. Nothing
  yet states that lowering is a function.
- **The read behind this RFC is complete for the front end and dispatch core,
  and partial for the word vocabulary.** `conditional/`'s `csv` and `sqlite`
  handlers (372 lines) were surveyed for shape, not read; `values/`'s `merge`,
  `unfold`, `listop` and `sort_lists` were enumerated by registration; and
  `execute_class` and `execute_object` have since been read, closing F59's scope
  and removing that gap. Neither re-enters evaluation, so S4's frame model is
  unaffected by them. What remains unread constrains RFC-0004's effect table,
  not the IR's shape.

## Amendment, 2026-09-10 — §S3's text from before D35 was resolved

§S3 was drafted while D35 was OPEN and kept two passages from that draft after
D35 resolved. They contradict the section's own first bullet ("keyed on the
body's **`Rc` pointer**, as **D35** resolves"), this RFC's Status ("Blocked on:
nothing … D35 resolves the cache key") and its open-questions list ("D35 is
resolved"). They are left as written — this RFC is Accepted — each marked where
it stands, and superseded here:

| passage, by its opening words | said | now |
|---|---|---|
| "D3's **amended** resolution follows from identity keying rather than from content hashing" | eval'd code cannot hit an *identity*-keyed cache, so it stays at Tier 0 with no rule | **no cache here is identity-keyed**: D35 chose option 4, the body's `Rc` pointer. D3's conclusion — no eval-specific tier rule — stands, on a different ground: an eval'd token stream is applied token by token and retains nothing, so it is never a body and there is nothing to key. A lambda *inside* eval'd code is an ordinary body (RFC-0005 §S3) |
| "**D35 blocks this section.**" and the paragraph after it, "D5 is not wrong" | D35 is OPEN and recommends content keying, so §S3 "states the shape and not the key" | **D35 is RESOLVED**, option 4. Content keying is deferred as "a strict upgrade", not recommended, and D35 records that it superseded its own earlier recommendation. §S3 states the key, and nothing in it is blocked |

**Two questions this amendment does not settle**, both the owner's and both
able to amend §S3 again:

- **Q32** — the first bullet's reason for a strong reference ("or a freed
  body's address could be reused") does not hold as written: a `Weak` also
  keeps the allocation, and therefore the address, out of reuse.
- **Q36** — the key §S3 states never reaches execution. S4's `Frame` holds a
  copied `Vec<BundValue>`, and `Vm::eval_body` takes a slice, so the body's
  `Rc` is discarded before any body runs.

**Nothing any program does changes.** This corrects the record of a decision
already taken.

- Amended by: repository owner, 2026-09-10. The contradiction was carried as a
  finding by RFC-0005's fifth and sixth reviews.

## Amendment, 2026-09-10 (second) — S4's frame holds the body's value

Both questions the first amendment left open were decided the same day.

**D42 (Q36).** S4's `Frame` held `body: Vec<BundValue>`, a copy, so the `Rc`
§S3 keys on never reached the frame loop. The frame now holds the body's
**value** — a LAMBDA, or a LIST for a body assembled at run time — and an
instruction pointer, and reads each item through it. `Vm::eval_lambda` and
`Vm::tail_lambda` hand it the value (RFC-0002's amendment of the same date). A
body is therefore alive whenever a frame is running it, and its key is in hand
at every entry.

**D35 as amended (Q32).** §S3's first bullet says the cache holds "a strong
clone, or a freed body's address could be reused". The key stands; the reason
does not. A `Weak` keeps the allocation, and so the address, out of reuse just
as surely, and with D42 the frame keeps a running body alive. The cache holds a
`Weak`.

S4's criterion 2 — call depth bounded by heap, not the Rust stack — is
unaffected in kind: the loop is still flat, and only what a frame holds
changed. Nothing a program does changes.

- Amended by: repository owner, 2026-09-10, on Q36 and Q32

## Amendment, 2026-09-10 (third) — a floor under native-mediated nesting (F85)

S4 bounds Bund call depth by the heap for **direct** calls, and its
criterion 2 measures exactly that. A native that runs a body synchronously —
`times`, `loop`, `map`, `while`, the conditionals, `?try`, the method paths —
still spends a Rust frame per level, and recursion through one aborted the
process on the machine stack (F85), as the reference's does. That is a fourth
depth axis, and S4 did not name it.

It is now bounded. `bund2` runs evaluation on a thread whose stack it sizes,
and every `Interp` on that thread takes a floor from it. `Vm::eval_lambda`,
`Vm::apply` and `Vm::scoped_call` refuse below the floor with a Bund-level
error instead of nesting further. The mechanism is RFC-0005 §S8's Tier 0
floor, which needs no JIT. `cargo xtask depth` runs the new axis at 100,000
levels through `times`, and it reports the error.

Direct recursion is unaffected, and so is criterion 2. The only change a
program can see is that this recursion now fails the way a Bund program is
allowed to fail — with a diagnostic — instead of killing the process.

- Amended by: repository owner, 2026-09-10, on F85

## Dated note, 2026-09-11 — a synchronous run refuses after an exit (F112)

S4's frame loop pops a finished frame without consulting anything, so
`Vm::eval_lambda`, `Vm::apply` and `Vm::scoped_call` returned `Ok` when the
body they ran ended in `bund.exit`. The native that called them then went on.
D52, which came after this RFC, says nothing more runs after an exit. Since
F112 each of the three consults the exit gate after the loop returns, and
answers the refusal. The frame loop itself is unchanged, and so is every
criterion here. Only a program that exits from inside a native's body can see
the difference.
