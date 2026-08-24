# Open questions

Claims that could not be grounded in `reference/`, and questions raised during
drafting that are not yet decisions.

An entry here is a `[UNGROUNDED]` marker in some RFC. Either ground it, or
promote it to a decision, or delete the claim.

| # | Question | Raised in | Status |
|---|----------|-----------|--------|
| Q1 | The corpus never invokes `.id` or `.timestamp`, so it constrains neither D1 nor D2. Does the contract come from somewhere other than the example corpus? | `cargo xtask corpus` | **RESOLVED.** It does: `id` drives equality, ordering and hashing internally (`reference/rust_dynamic/src/eq.rs:53`, `ord.rs:199`, `hash.rs:6`), and `stamp` is read by metric iteration (`iter.rs:67`). D1 and D2 are both RESOLVED — lazy — with those constraints recorded. |
| Q2 | `ptr !` builds a word name by string concatenation at run time and calls it (`reference/Bund/examples/bund_dynamic_demos/dynamic_demo_2.bund:29-34`). Does any closed-world assumption survive this, independently of D3? See the Q2 section below for the grounded mechanism. | `cargo xtask corpus` | **RESOLVED — see D16.** No. The behaviour is preserved as-is and the world is permanently open. |
| Q9 | Forms with no corpus coverage and so no golden: the backtick PTR term (`reference/bund_language_parser/bund.pest:29`), and four of `execute`'s eight dispatch arms. `cargo xtask conform` cannot regress-test them. What hand-written tests cover them, and where do those live relative to `tests/golden/`? | D16 | **RESOLVED — see D21.** Authored probes in `tests/probes/`, goldens captured from the oracle into `tests/golden/probes/`. Expected output is never hand-written. Probes target behaviours, not words. |
| Q3 | `format` is registered under `vm/string` but 13 otherwise-basic programs depend on it. Is "implementing subsystem" the right axis for D14 at all, or does D14 need a per-word answer? | `cargo xtask corpus` | **PARTIALLY RESOLVED — see D17.** `format` is core, decided per-word, which settles the instance and shows subsystem is not the partition. The workbench-variant half is settled by D18. Still open: whether the remaining subsystems are resolved per-word or wholesale. |
| Q10 | D18 pairs a preserved word with its `.` workbench form. The `,` "keep" suffix is a second axis, giving four-way families like `forecast.markov` / `.` / `,` / `.,` (`reference/Bund/src/stdlib/functions/forecast/markov.rs:74-77`) — 16 base names carry `,` and 16 carry `.,`. Does D18's gap-filling extend to them? | D18 | **RESOLVED — see D22.** No. `,` carries two unrelated meanings (keep-operand vs in-place), so it fails the mechanical-determinism property D18 relies on. Existing forms preserved; none invented. |
| Q13 | D23 lets `<class> !` instantiate a class that was never registered. But object construction stamps `.class_name` from the name it was handed (`reference/rust_multistackvm/src/stdlib/bund_object.rs:36`), and a dynamically built class has none — `class` sets only `.super` (`reference/rust_multistackvm/src/stdlib/artefacts.rs:69-73`). An object from an anonymous class would lack `.class_name`, which `.str`/`.print`/`.println` all require (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:36-38`). What is such an object's class name? | D23 | **RESOLVED — see D25.** The class value's own `.class_name`, which is what all 12 built-in classes already carry. Absent it, construction fails rather than improvising a name. |
| Q12 | F16 is dispositioned FIX: `<class> !` creates an object of that class. But a CLASS value carries no name — `class` sets only `.super` (`reference/rust_multistackvm/src/stdlib/artefacts.rs:69-73`), `register` takes the name from beneath it on the stack (`reference/rust_multistackvm/src/stdlib/classes/registry.rs:9-20`), and `.super` holds parent *names* needing the registry (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:95`). Must `<class> !` require the class to be registered first, or build from the stack value and resolve only parents through the registry? | F16 | **RESOLVED — see D23.** Both provenances work; `!` builds from the CLASS value, and the registry is consulted only for parents. |
| Q11 | D18 fills missing `.` workbench forms, of which there are 262. Is that universal, or only where the form is semantically meaningful? Five are class constructors (`True`, `False`, `List`, `Floats`, `Intervals`) and six are binary operators (`==`, `!=`, `<`, `<=`, `>`, `>=`) — it is not obvious what `True.` or `==.` would do. | D18 | **RESOLVED — see D24.** Only operand-sourcing words get a `.` form; producers are covered by the existing `.` word. Application is blocked on `cargo xtask arity` — the `StackOps` proxy tried first turned out circular. |
| Q4 | `display` (`reference/Bund/src/stdlib/functions/system/display.rs:88`) is a terminal renderer filed under `system/`. How many other words are mis-filed relative to what they do, and does that matter to the D14 partition? | `cargo xtask corpus` | **RESOLVED — approach A+D.** Subsystem grouping stays as a reporting aid; an implementation-reachability pass was added. Both implemented. The accompanying effect audit found one false hermetic (`string.random.*`), now fixed. See the Q4 section below. |
| Q5 | 455 of 617 registered names are never exercised by the corpus. What is the conformance denominator for a word with no golden? | `cargo xtask corpus` | **RESOLVED — two numbers, neither substituting for the other.** `conform` stays goldens-only; `cargo xtask coverage` reports completeness. Implemented. See the Q5 section below. |
| Q6 | `bund/console` words are stdout-only but the spinner is an animation and `console.text.rainbow`/`.matrix`/`.randomcolor` pick colours at random (`reference/Bund/src/stdlib/functions/console/spinner.rs:278,310`). Can a golden capture animated or coloured stdout at all? | `cargo xtask corpus` | **RESOLVED — see D15.** Only basic console output is in scope; the whole `bund/console` subsystem is deferred. |
| Q7 | `display` renders markdown through `termimad::print_text` (`reference/Bund/src/stdlib/functions/system/display.rs:11`), which emits ANSI styling. D15 puts colour out of scope. Does `display` fall under D15, or is it in scope because it is not *chosen* colour? 12 programs depend on it. | D15 | **RESOLVED — in scope.** D15 defers the `bund/console` subsystem only; `display` is unaffected. |
| Q8 | `Value` derives `Serialize`/`Deserialize` with `id` and `stamp` as fields (`reference/rust_dynamic/src/value.rs:15-18`), and `save.lambdas` persists whole Values as bincode BLOBs (`reference/Bund/src/stdlib/helpers/world/lambdas.rs:81-84`). Does that make the id format and stamp precision a cross-run contract, independently of what programs observe? Bears on D1, D2 and D11. | Q1 follow-up | **RESOLVED — see D20.** Serialisation materialises; the wire format stays byte-identical; `dup` stops serialising (F13, FIX). The cross-run contract exists only for readers outside Bund2, which is D11 — still open. |

---

# Corpus evidence

Produced by `cargo xtask corpus`, which reads `.bund` files and Rust
registration sites and builds nothing. Re-run it to regenerate every number
below.

**This section reports evidence and resolves nothing.** The decisions it feeds
are recorded in `decisions.md`; of those it was gathered for, D1, D2, D5 and
D12 are now RESOLVED and D3 and D14 remain OPEN. In particular, no core word
list is proposed here: the frequency tables below are input to D14, not an
answer to it.

## Corpus and registry shape

132 programs — 120 under `reference/Bund/examples`, 12 under
`reference/Bund/tests` — totalling 3485 lines.

The reference binds names in six namespaces. Resolution order is
`reference/rust_multistackvm/src/multistackvm_apply.rs:16-60`: command, then
`$`-forced internal word, then alias, then lambda, then inline word.

| Namespace | Count | Registered by |
|---|---|---|
| inline words | 548 | `register_inline` |
| aliases | 70 | `register_alias` |
| methods | 37 | `register_method` |
| commands | 2 | `register_command` |
| classes | 14 | `register_class` |

Inline words come from three crates, not two. `i_direct` tries the VM's own
table (`reference/rust_multistackvm/src/multistackvm_inline.rs:42`) and on a
miss falls through to the stack layer's table
(`reference/rust_multistackvm/src/multistackvm_inline.rs:52`). That second
tier is `reference/rust_multistack`, and it holds the core stack words —
`take` at `reference/rust_multistack/src/stdlib/workbench.rs:81`, `drop` at
`reference/rust_multistack/src/stdlib/drop.rs:71`. A scan of only `Bund/src`
and `rust_multistackvm/src` reports both as unregistered, which is wrong.

`register_function`
(`reference/rust_multistack/src/ts_functions.rs:6`) is a seventh table that
`i_direct` never consults, so its names are not reachable as words.

## D1 — `.id` format contract

**The corpus does not exercise `.id` at all.** Zero invocations, and zero
occurrences in atom position (`:.id`). A direct `grep` over
`reference/Bund/examples` and `reference/Bund/tests` for `.id` confirms the
lexer's count.

`.id` is not an inline word. It is a *method*, registered at
`reference/Bund/src/stdlib/functions/oop/base_classes.rs:91` and installed on
the base `Object` class as a PTR attribute at
`reference/Bund/src/stdlib/functions/oop/base_classes.rs:98`. Its
implementation pushes `value.id` as a string
(`reference/Bund/src/stdlib/functions/oop/base_classes.rs:11-18`), where the
field is filled by `nanoid!()` at
`reference/rust_dynamic/src/id.rs:6`.

Consequence for D1: the corpus places **no constraint** on the format. It
neither prints an id, compares two ids, nor depends on their length or
alphabet. Whatever decides D1, it is not this evidence — see Q1.

Also unused: `id.ulid` (`reference/Bund/src/stdlib/functions/string`), which
would have been the other way a program could observe an identifier format.

**`id` is load-bearing, not a label.** The corpus cannot show this because it
never calls `.id`, but three internal readers depend on the field:

* **Equality** — for every non-scalar type, `==` *is* id comparison
  (`reference/rust_dynamic/src/eq.rs:53`; same fallback for mismatched scalar
  pairs at `:15,26,34,42`). Two structurally identical lists are unequal
  unless they share an id.
* **Ordering** — `reference/rust_dynamic/src/ord.rs:175,183,191,199` fall back
  to `self.id.cmp(&other.id)`. That arm is inconsistent with `PartialOrd` and
  currently unreachable; recorded as defect F12.
* **Hashing** — `reference/rust_dynamic/src/hash.rs:6` hashes the id and
  nothing else.

`Clone` preserves the id; `dup` mints a fresh one
(`reference/rust_dynamic/src/dup.rs:11`), as do `attr.rs:7`, `push.rs:165`,
`set.rs:16,31,43,58,76,91` and `bincode.rs:95`. Clone-equal versus dup-unequal
is the observable contract. This is what constrains D1's laziness: the cheap
token must answer equality, ordering and hashing without materialising a
string.

**`stamp` is likewise read outside `.timestamp`.** Iterating a METRICS value
materialises it into a `"ts"` field (`reference/rust_dynamic/src/iter.rs:67,82`,
`reference/rust_dynamic/src/carcdr.rs:127,203`). Neither path is
corpus-reachable — `metrics` and `sample` are unused — but both are live code,
and they mean a lazy stamp must still report *construction* time.

**And there is a third observation channel.** `Value`
derives `Serialize`/`Deserialize` and carries `id: String` and `stamp: f64` as
ordinary fields (`reference/rust_dynamic/src/value.rs:15-18`). `save.lambdas`
serialises whole Values with `to_binary`
(`reference/rust_dynamic/src/bincode.rs:8`) into a SQLite BLOB
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:81-84`), and
`load.lambdas` reads them back. Every id and stamp in a saved world is
therefore on disk, in a format a future version has to keep reading.

No corpus program exercises that path — `save.lambdas` and `load.lambdas` have
zero uses — so this is a latent contract, not an observed one. It is recorded
as Q8 because it bears on D1, D2 **and** D11 at once.

## D2 — `.timestamp` precision

**The corpus does not exercise `.timestamp` either.** Zero invocations of
`.timestamp` (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:92`),
zero of `time.timestamp`
(`reference/rust_multistackvm/src/stdlib/time/timestamp.rs:36`), and zero of
`time.now`. Zero occurrences in atom position.

`.timestamp` pushes `value.stamp` as a **float**, not an integer
(`reference/Bund/src/stdlib/functions/oop/base_classes.rs:25`), and the field
is set from `timestamp_ms()` alongside the id at
`reference/rust_dynamic/src/id.rs:7`.

Consequence for D2: no program in the corpus constructs two values in sequence
and compares their stamps, so the "must two values differ" half of the
question is untouched by this evidence. See Q1.

## D3 — tier policy for `bund.eval` output

Runtime code construction is **rare but present**, and it is not confined to
`bund.eval`.

| Word | Uses | Programs | Registered |
|---|---|---|---|
| `bund.eval` | 1 | 1 | `reference/Bund/src/stdlib/functions/bund/bund_eval.rs:124` |
| `compile` | 1 | 1 | `reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:76` |
| `apply` | 1 | 1 | `reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:76` |
| `!` (alias of `execute`) | **69** | **39** | `reference/rust_multistackvm/src/stdlib/create_aliases.rs:5` |
| `bund.eval-file`, `bund.eval.`, `bund.eval-file.`, `!!` | 0 | 0 | — |
| `load.script`, `load.lambdas`, `save.lambdas` | 0 | 0 | — |
| `execute`, `execute.`, `!.` | 0 | 0 | — |

Two observations that matter more than the counts:

1. **`bund.eval` is used inside a lambda body**, at lambda depth 2 —
   `reference/Bund/examples/code_snippets/bund_shell.bund:24`. It is not a
   top-level convenience.
2. **`execute` is never spelled `execute`.** It is spelled `!`, and `!` is one
   of the five most-used words in the corpus: 69 invocations across 39 of 132
   programs, 13 of them inside a lambda body. Probing the registered name
   alone returns 0 and reads as "no dynamic dispatch", which is the opposite
   of what the corpus does.

The persistence words are entirely unexercised: `load.lambdas`, `save.lambdas`
and `load.script` have zero uses. Whatever D3 decides about them, no golden
currently covers them.

## Closed-world reachability

| Word | Uses | Programs | Registered |
|---|---|---|---|
| `register` | 35 | 20 | `reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:88` |
| `unregister` | 1 | 1 | `reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:89` |
| `alias` | 0 | 0 | `reference/rust_multistackvm/src/stdlib/alias.rs:72` |
| `unalias` | 0 | 0 | `reference/rust_multistackvm/src/stdlib/alias.rs:73` |

`register` is registered twice — at
`reference/rust_multistackvm/src/stdlib/classes/registry.rs:61` and at
`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:88`.
`register_inline` overwrites (`reference/rust_multistackvm/src/multistackvm_inline.rs:6`),
so the lambda registry is what runs.

The sharper evidence is not `register` but **word names built at run time**.
`reference/Bund/examples/bund_dynamic_demos/dynamic_demo_2.bund:29-34`
concatenates an integer with the string `"Function"`, converts the result to a
PTR, and calls it:

    1 convert.to_string "Function"  +
      ptr !

The same shape appears at
`reference/Bund/examples/bund_dynamic_demos/dynamic_demo_3.bund:33` and
`reference/Bund/examples/bund_dynamic_demos/dynamic_demo_4.bund:39`. `ptr` is
used 5 times across 3 programs
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:136`). A callee named by
a computed string is not resolvable ahead of time.

### Q2 — the mechanism, and why it is independent of D3

`ptr` pulls a value, casts it to a string, and pushes a PTR carrying that
string as its name (`reference/rust_multistackvm/src/stdlib/artefacts.rs:80-93`;
the `apply` of a PTR falls to the push arm at
`reference/rust_multistackvm/src/multistackvm_apply.rs:88-99`). `!` is an alias
of `execute` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:5`),
which pulls that value and, for `PTR | STRING | CALL`, hands the string
straight to `vm.call(...)`
(`reference/rust_multistackvm/src/stdlib/execute.rs:26-33`) — full name
resolution, alias then lambda then inline.

Three properties follow, none of which involve `bund.eval`:

1. **The callee's name need not appear in the source.** In
   `dynamic_demo_2.bund:29-34` the string `"Function1"` is assembled from an
   integer and a literal. No scan of the program text finds it.
2. **`ptr` is not even required.** `execute` accepts a bare `STRING`
   (`reference/rust_multistackvm/src/stdlib/execute.rs:28`), so any string that
   reaches the top of the stack can become a call.
3. **`execute` is a general dispatcher, not just "call this lambda."** On a
   LIST it recurses over every element
   (`reference/rust_multistackvm/src/stdlib/execute.rs:36-48`); on
   `MAP | INFO | CONFIG | ASSOCIATION` it pulls a key off the stack and
   dispatches on it (`reference/rust_multistackvm/src/stdlib/execute.rs:53+`).

This is why the question is posed *independently of D3*. D3 asks what tier
`bund.eval` output runs at, and deciding it "permanently Tier 0" contains the
case where data becomes **new** code. `ptr !` creates no new code — it
dispatches to **existing** words by a name computed at run time. A program
containing zero `bund.eval` still cannot have its call graph closed. Whatever
D3 decides buys nothing here.

Scale: `!` is 69 uses across 39 of 132 programs, 13 inside lambda bodies —
pervasive. The computed-name form is narrow: `ptr` is 5 uses across 3
programs, all under `bund_dynamic_demos/`, whose stated purpose is to
demonstrate dynamism.

**Resolved by D16: the behaviour is preserved as it exists, and the world is
permanently open.** The narrowness of the computed-name form is not taken as a
licence to restrict it. Consequences — no AOT tree-shaking by word
reachability (RFC-0006), no static devirtualisation of `!` (RFC-0005), and a
word table that stays queryable by computed name at run time (RFC-0002) — are
recorded in full on D16.

Note that `register`/`unregister` (35 uses, 20 programs) also mutate the word
table at run time, but those names *are* visible in the source
(`:Function1 { ... } register`), so they are the tractable half of the problem.

On the backtick form specifically: it is a first-class grammar production, not
an alternative spelling. `ptr` is one of the twelve alternatives in `value`
(`reference/bund_language_parser/bund.pest:7-20`, rule at `:29`), it has a
dedicated parser handler
(`reference/bund_language_parser/src/vm/ptr.rs:7-10`), and `name` carries a
negative lookahead `!("`")` (`bund.pest:28`) reserving the character for it.
Bund2 implements it under the syntax-preservation mandate regardless of
corpus coverage. What the corpus gap costs is regression testing, not scope —
carried forward as Q9.

## D5 — lambda body mutability

**No program mutates a LAMBDA body after construction.** The mutators split
like this:

| Word | Uses | Programs |
|---|---|---|
| `set` | 265 | 65 |
| `+++.` (alias of `push.`) | 8 | 3 |
| `push`, `push.`, `+++` | 0 | 0 |

There are 45 sites where a closing `}` is immediately followed by a mutator.
Every one of them is the same shape, and **it is not lambda mutation**:

    :.init { :.class_name get "Constructor of {A}" format println } set register
    — reference/Bund/examples/object_oriented_programming/class_constructors_demo.bund:13

`set` pulls three values in the order stored-value, key, receiver
(`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:10-27`). So the
lambda here is the *value being stored* and the receiver is the class beneath
it. The lambda body is never touched; a lambda is being filed into a
dictionary.

`push` cannot mutate a lambda either: it converts its receiver with
`conv(LIST)` before appending
(`reference/Bund/src/stdlib/functions/values/push.rs:34`), so `push` applied
to a LAMBDA yields a LIST rather than a mutated LAMBDA. Both words pull their
operands off the stack and push a result back
(`reference/Bund/src/stdlib/functions/values/push.rs:26-52`), which is value
semantics throughout.

Consequence for D5: the corpus contains no counter-example to write-once, and
also no case that *requires* write-once. It does not exhibit the mutation the
default assumes must be invalidated.

## D12 — the `*` fold-family

**The arithmetic fold family is entirely unused by the corpus.**

| Word | Uses | Registered |
|---|---|---|
| `*+`, `*+.` | 0 | `reference/rust_multistackvm/src/stdlib/math/add.rs:25,26` |
| `*-`, `*-.` | 0 | `reference/rust_multistackvm/src/stdlib/math/sub.rs:25,26` |
| `**`, `**.` | 0 | `reference/rust_multistackvm/src/stdlib/math/mul.rs:25,26` |
| `*/`, `*/.` | 0 | `reference/rust_multistackvm/src/stdlib/math/div.rs:25,26` |
| `*loop`, `*loop.` | 0 | `reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:122,123` |
| `Σ`, `Σ.` (aliases of `*+`, `*+.`) | 0 | `reference/rust_multistackvm/src/stdlib/create_aliases.rs:37,38` |
| `global*` | 0 | `reference/Bund/src/stdlib/functions/bus/globals.rs:110` |
| `lambda*` | 2 | `reference/Bund/src/stdlib/functions/bund/bund_fun.rs:219` |
| `input*` | 1 | `reference/Bund/src/stdlib/functions/io/input.rs:144` |
| `generator.sample*` | 25 | `reference/Bund/src/stdlib/functions/generators/mod.rs:70` |

The three `*` words the corpus does use are not arithmetic folds:

* `lambda*` folds the whole stack into a LAMBDA —
  `reference/Bund/examples/bund_dynamic_demos/create_lambda_on_the_fly.bund:17`.
  Both uses are at lambda depth 0.
* `input*` is a REPL read loop —
  `reference/Bund/examples/code_snippets/bund_shell.bund:30`.
* `generator.sample*` takes an explicit integer count immediately before it in
  every use — `:DATASET1 16 generator.sample*`
  (`reference/Bund/examples/ai/predict_from_gen.bund:5`), `generator 64
  generator.sample*`
  (`reference/Bund/examples/data_analysis_fmt_with_template.bund:18`). The `*`
  in its name is not whole-stack arity.

Note that `*` alone is ordinary multiplication
(`reference/rust_multistackvm/src/stdlib/math/mul.rs:23`), used 3 times; `**`
is the variadic one. The naming does not follow the sigil.

## D14 — core / library partition

Words are grouped by the subsystem that *implements* them, taken from the
registration path. Aliases are attributed to their target.

168 distinct words are invoked, 1707 invocations in total. **455 of the 617
distinct registered names are never used by the corpus** — see Q5. (617, not
618: `swap` is registered both as an inline word,
`reference/rust_multistack/src/stdlib/swap.rs`, and as an alias of `swap_one`,
`reference/rust_multistackvm/src/stdlib/create_aliases.rs:19`. Alias
resolution runs first — `reference/rust_multistackvm/src/multistackvm_apply.rs:39`
— so the alias is what the 33 corpus uses reach.)

### Subsystems by programs that would break without them

| Subsystem | Programs | Distinct words used |
|---|---|---|
| `vm/print` | 93 | 3 |
| `vm/values` | 78 | 7 |
| `vm/artefacts` | 68 | 9 |
| `vm/logic` | 56 | 12 |
| `vm/string` | 52 | **1** |
| `vm/execute` | 39 | 1 |
| `stack/dup` | 38 | 1 |
| `stack/swap` | 33 | 1 |
| `stack/workbench` | 29 | 2 |
| `vm/bund_object` | 27 | 1 |
| `bund/generators` | 22 | 3 |
| `bund/values` | 22 | 8 |
| `bund/conditional` | 21 | 8 |
| `vm/math` | 21 | 6 |
| `vm/lambdas` | 20 | 2 |
| `stack/drop` | 16 | 1 |
| `bund/bund` | 15 | 15 |
| `bund/forecast` | 13 | 7 |
| `bund/io` | 13 | 5 |
| `bund/system` | 13 | 3 |

The tail — `bund/filesystem` (10), `bund/ai` (9), `bund/string` (8),
`bund/console` (5), `bund/graph` (5), `bund/bus` (3), `bund/internaldb` (2),
`bund/debug_fun` (1), `bund/sysinfo` (1) — is where the domain libraries sit.

### The words the most programs depend on

Reported as counts. This table is **not** a proposed core list.

| Word | Programs | Subsystem |
|---|---|---|
| `println` | 91 | `vm/print` |
| `set` | 65 | `vm/values` |
| `format` | 52 | `vm/string` |
| `!` | 39 | `vm/execute` |
| `config` | 39 | `vm/artefacts` |
| `dup` | 38 | `stack/dup` |
| `swap` | 33 | `stack/swap` |
| `take` | 29 | `stack/workbench` |
| `.` | 27 | `stack/workbench` |
| `object` | 27 | `vm/bund_object` |
| `loop` | 25 | `vm/logic` |
| `register` | 20 | `vm/lambdas` |
| `drop` | 16 | `stack/drop` |
| `get` | 13 | `vm/values` |
| `if` | 13 | `vm/logic` |
| `+` | 12 | `vm/math` |
| `==` | 12 | `vm/logic` |
| `display` | 12 | `bund/system` |
| `print` | 12 | `vm/print` |
| `times` | 12 | `vm/logic` |
| `,` | 11 | `vm/values` |
| `fmt` | 11 | `bund/conditional` |
| `seq` | 11 | `bund/math` |

`config` is an alias of `dict`
(`reference/Bund/src/stdlib/functions/create_aliases.rs:26`); `,` is an alias
of `set` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:21`); `.`
is an alias of `return`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:4`).

Conditionals and loops are thinner than expected. `vm/logic` registers 45
names and the corpus uses 12 of them:

| Word | Programs |
|---|---|
| `loop` | 25 |
| `if` | 13 |
| `==` | 12 |
| `times` | 12 |
| `?true*` (alias of `ifthenelse`, `reference/rust_multistackvm/src/stdlib/create_aliases.rs:10`) | 8 |
| `map` | 2 |
| `not` | 2 |
| `!=`, `<`, `>`, `for`, `while` | 1 each |

`for` (`reference/rust_multistackvm/src/stdlib/logic/for_fun.rs:101`) and
`while` (`reference/rust_multistackvm/src/stdlib/logic/while_fun.rs:96`) are
each exercised by exactly one program. The other 33 `vm/logic` names are never
used, including `and`, `or`, `while.`, `for.`, `ifthenelse` spelled directly,
`notifthenelse`, `?false` and its variants, and every workbench (`.`-suffixed)
form.

### Library-shaped subsystems that otherwise-basic programs reach into

A program counts here when it uses a subsystem outside the
stack/math/logic/lambda/oop set and everything *else* it touches is inside
that set. These are the D14 pressure points.

| Subsystem | Otherwise-basic programs | Words involved |
|---|---|---|
| `vm/string` | **13** | `format` |
| `bund/bund` | 6 | `?class` `?object` `args.parse` `save.model` |
| `bund/ai` | 5 | `classifier` `classify` `neuralnetwork` `predict` |
| `bund/bus` | 2 | `global` |
| `bund/console` | 2 | 16 `console.*` words |
| `bund/generators` | 2 | `generator` `generator.sample*` |
| `bund/string` | 2 | `string.random.lorem` `string.random.name` |
| `bund/system` | 2 | `display` |
| `vm/json` | 1 | `json.path` |

**`format` is the case the owner expected, and the evidence supports it.**
Registered at `reference/rust_multistackvm/src/stdlib/string/format.rs:135`,
it is used 119 times across 52 of 132 programs, and in 13 of those the program
touches nothing else outside the basic set. Examples:
`reference/Bund/examples/object_oriented_programming/class_constructors_demo.bund:13`,
`reference/Bund/examples/code_snippets/application_logic_demos.bund`,
`reference/Bund/examples/code_snippets/sorting_numbers_in_list.bund`.
`vm/string` contributes exactly **one** used word — `format` alone accounts
for the subsystem's entire 52-program footprint, while its other 8 names
(`format.`, `concat_with_space`/`sp`, `string.camel`, `string.lower`,
`string.snake`, `string.title`, `string.upper`) are never used.

**Correction — `format` is *not* reachable from the OOP layer.** An earlier
draft of this section implied it was, via `.template`. It is not. The OOP
`.format` method has its own implementation
(`reference/Bund/src/stdlib/functions/oop/display_class.rs:14-85`), which
resolves placeholder names from the object's own attributes
(`display_class.rs:22-44`). The `format` word
(`reference/rust_multistackvm/src/stdlib/string/format.rs:9`) pulls
placeholder values off the *stack* (`format.rs:28-36`). Both parse with
`leon::Template` (`display_class.rs:29`, `format.rs:17`), so they share a
template syntax and nothing else. Nothing internal calls
`stdlib_string_format` — the only reference to it outside its own file is its
registration (`reference/rust_multistackvm/src/stdlib/string/mod.rs:10`).

That matters for D14: deferring the `format` word would leave `.template`,
`.format` and `.display` working, and would break only the 52 programs that
call `format` directly. `format`'s claim on the core rests on corpus
dependency alone, not on structural reachability.

**`display` is the second case, and it is mis-filed rather than library.** It
is registered under `system/` at
`reference/Bund/src/stdlib/functions/system/display.rs:88`, but it renders
markdown to the terminal via `termimad::print_text`
(`reference/Bund/src/stdlib/functions/system/display.rs:11`) and delegates to
`conditional_fmt::conditional_run`
(`reference/Bund/src/stdlib/functions/system/display.rs:36`).
It is a printing word sitting in the same directory as `system.shell` and
`system.setproctitle`. 12 programs use it. See Q4.

**`fmt`** (11 programs, `reference/Bund/src/stdlib/functions/conditional/mod.rs:41`)
and **`?try`** (7 programs, `reference/Bund/src/stdlib/functions/conditional/mod.rs:37`)
are worth flagging alongside these: both live in `bund/conditional`, which the
grouping above treats as basic, but both are implemented in the `Bund` crate
rather than the VM.

## Hermetic partition

`tests/golden/HERMETIC.txt` applies two independent filters.

* **Hermetic** — every word invoked is pure, writes stdout, or writes a
  diagnostic to stderr. **80 of 132** programs pass.
* **In scope** — no word invoked has been deferred by a decision. D15 defers
  the `bund/console` subsystem, which removes 3 further programs:
  `ai/ollama_api_demo`, `code_snippets/string_wrap_demo` and
  `console/typewriter_demo`. (`console/spinner_demo` and
  `console/text_color_demo` were already non-hermetic — the spinner needs
  `sleep`, and three `console.text.*` words pick colours at random.)

**The conformance suite is the intersection: 77 of 132.**

Keeping the two axes separate matters. A colour-writing word is perfectly
hermetic — deterministic bytes on stdout — and still not something Bund2 will
emit. Folding scope into the effect classification would misreport both, so
`classify.rs` carries them as separate functions and `HERMETIC.txt` names
every program a scope decision removed rather than letting it vanish.

Effects are derived from each word's registration path, with per-word
overrides where the path misleads; the rules and their citations are in
`xtask/src/corpus/classify.rs`.

Disqualifying effects, by count of (program, word) pairs:

| Effect | Pairs |
|---|---|
| random | 47 |
| filesystem | 21 |
| network | 15 |
| bus | 5 |
| database | 5 |
| host | 5 |
| stdin | 3 |
| clock | 1 |
| process | 1 |

Randomness dominates, and almost all of it is one subsystem:
`bund/generators` uses `rand::thread_rng`
(`reference/Bund/src/stdlib/functions/generators/uniform.rs:49`) with no seed
input. `bund/math/rand.rs` is the other source, importing `rand_mt`,
`fastrand` and `rand_chacha`
(`reference/Bund/src/stdlib/functions/math/rand.rs:7-10`).

Two classification calls that a reader should check rather than take on trust:

* `log.*` (`reference/Bund/src/stdlib/functions/debug_fun/debug_trace.rs`) is
  treated as hermetic because `env_logger`
  (`reference/Bund/src/cmd/setloglevel.rs:10`) writes to stderr, and the
  golden format records stdout, not stderr. If goldens ever capture stderr,
  its default timestamped format makes these words non-hermetic.
* `bund/console` words are treated as stdout for the *hermetic* filter, and
  removed from the suite by the *scope* filter under D15. The spinner is an
  animation (`reference/Bund/src/stdlib/functions/console/spinner.rs:11`) and
  three `console.text.*` words pick colours at random
  (`reference/Bund/src/stdlib/functions/console/spinner.rs:278,310`).

## Q4 — the effect audit

The effect table keys off *registration paths*. A path says who wrote a word,
not what it does, so the classification needed checking rather than asserting.
`cargo xtask corpus` now cross-checks every used word's assigned effect against
the crates its registering file imports (`xtask/src/corpus/classify.rs`,
`EFFECT_MARKERS`). Import markers are indicators, not verdicts — a file may
import `rand` and use it in one function of ten — so the audit produces a
review list, not a ruling.

**One false hermetic, now fixed.** `string.random.name` and
`string.random.lorem` were classified pure by the `bund/string` default, but
`reference/Bund/src/stdlib/functions/string/random.rs:7-8` imports
`rand::thread_rng` and `passwords::PasswordGenerator`. Two programs —
`code_snippets/generate_100_random_first_names` and
`code_snippets/generate_25_loorem_ipsum_strings` — were in the golden suite
producing different output on every run. Reclassified as random; the suite
went from 79 to 77.

**One deliberate override, which the audit will keep flagging.**
`reference/Bund/src/stdlib/functions/bund/bund_exit.rs` imports
`std::process`, but the exit status is deterministic and goldens record it, so
`exit` stays hermetic. It is now named explicitly in the path rules rather
than inheriting `bund/bund -> pure` by fall-through.

**29 words are classified effectful with no import to support it.** These cost
coverage rather than correctness, and most are explained by the effect living
in a sibling file: `ai/mod.rs` registers `classifier` and `predict` while the
network calls sit in `ai/deepseek.rs` and friends, and `bus/crossbus.rs`
registers `send`/`recv` while zenoh is reached through
`reference/Bund/src/stdlib/helpers/zenoh`. Left as-is: over-strict costs a
golden, under-strict costs correctness.

Words whose directory misdescribes them, found by hand before the audit
existed and already carried as per-file rules: `display` (`system/`, but a
`termimad` renderer), `io/graph.rs` (`io/`, but a `rasciigraph` plot to
stdout), `io/banner.rs`, `bund/math/rand.rs`, `system/unixpath.rs` (pure
string surgery among process words), and the split inside `bund/bund` between
`bund_eval.rs` (pure) and `bund_load.rs`/`bund_save.rs` (filesystem).

### Resolution — approach A+D

Subsystem grouping stays as it is (A). It answers "which module could we not
ship", which is a real question and a different one from "what does this
ruling drag in"; bending one into the other loses both. Grouping by effect was
considered and rejected — effect is orthogonal to core-ness (`format` and
`string.distance.levenshtein` are both pure, one is core), and folding them
would repeat the error of conflating scope with effect.

Added instead (D): an **implementation-reachability pass**. For every
corpus-used word, `cargo xtask corpus` reports what its implementing file
references in other stdlib subsystems, so a per-word D14 ruling states its
closure instead of discovering it later. It scans `use` lines *and* inline
fully-qualified paths, because the reference uses both — `display` imports its
dependency (`reference/Bund/src/stdlib/functions/system/display.rs:6`) while
`.display` calls `stdlib_print_inline` fully qualified in the body
(`reference/Bund/src/stdlib/functions/oop/display_class.rs:93`). `crate::` is
resolved per crate: inside `Bund` it is `Bund/src/stdlib/`, and a capitalised
segment is a type or static, not a module — without both rules
`use crate::stdlib::BUND` reports the Bund global mutex as a VM subsystem.

**Result: of the 91 registration files providing the 168 corpus-used words, 81
are self-contained and 10 reach another subsystem.**

| File | Subsystem | Reaches | Corpus-used words |
|---|---|---|---|
| `system/display.rs` | `bund/system` | `bund/conditional` | `display` |
| `forecast/estimation.rs` | `bund/forecast` | `bund/math`, `bund/statistics` | `forecast.estimate!`, `sample.analysis` |
| `forecast/outliers.rs` | `bund/forecast` | `bund/math`, `bund/statistics` | `outlier.detect` |
| `forecast/outliers_dbscan.rs` | `bund/forecast` | `bund/math`, `bund/statistics` | `outlier.detect.dbscan` |
| `forecast/markov.rs` | `bund/forecast` | `bund/statistics` | `forecast.markov` |
| `forecast/mstl.rs` | `bund/forecast` | `bund/statistics` | `forecast.mstl` |
| `forecast/periodic_detector.rs` | `bund/forecast` | `bund/statistics` | `periodic.detect` |
| `math/interp.rs` | `bund/math` | `bund/statistics` | `math.interpolation` |
| `conditional/mod.rs` | `bund/conditional` | `vm/execute_types` | `?ifthenelse`, `?try`, `context`, `csv`, `curry`, `fmt`, `raise`, `sqlite` |
| `ai/deepseek.rs` | `bund/ai` | `bund/oop` | `deepseek.token` |

Three findings worth carrying into D14:

1. **`display` → `bund/conditional` is confirmed mechanically**, not just by
   the hand reading that produced D19.
2. **The whole `bund/forecast` family depends on `bund/statistics`**, a
   subsystem with 42 registered names and *zero* corpus uses. Any per-word
   ruling that preserves a forecast word preserves statistics machinery that
   no golden exercises.
3. **"Basic" leaks.** `bund/math` and `bund/conditional` are both in the
   TASK 3 basic set, yet `math.interpolation` reaches `bund/statistics` and
   the `bund/conditional` words reach `vm/execute_types`. The basic set is a
   reporting convenience, not a closed subsystem.

## Q5 — the coverage gap, and the second health number

**77 goldens exercise 162 of 617 registered names.** A Bund2 implementing only
those would print 77/77 from `cargo xtask conform` — 100% conformance — with
three quarters of the language unimplemented and nothing reporting it.

### Resolution

Two independent numbers. Neither is allowed to stand in for the other.

* **Conformance** (`cargo xtask conform`) — goldens passed over goldens. It
  stays a pure regression number over a fixed corpus. This is not a
  preference: CLAUDE.md requires the JIT and AOT milestones to move it by
  exactly zero, so that any movement is a bug. Fold word counts into that
  denominator and implementing a word moves the number, destroying the
  invariant that makes it useful.
* **Coverage** (`cargo xtask coverage`, implemented) — words with a test over
  words in scope. This is the completeness number, and the one that makes the
  gap visible.

### Current coverage

| | |
|---|---|
| registered names | 617 |
| out of scope by decision (D15) | 31 |
| **words in scope** | **586** |
| covered by a golden | 140 |
| covered by a hand-written test | 0 — not yet wired |
| **coverage** | **140 / 586 (23.9%)** |

The in-scope figures are lower than the raw ones because D15 removes all 31
`bund/console` words, 22 of which the corpus does exercise. Deferral removes
them from numerator and denominator alike.

### The 446 uncovered words

**51 are a suffix-variant of a covered word** — `format.`, `map.`, `car.`,
`loop.`, `if.`, `print.`, `println.`, `while.`, `times.`, `io.graph.`, the
`forecast.markov`/`,`/`.`/`.,` family, and so on. A `.` variant differs from
its base only by operand source — `StackOps::FromStack` versus
`FromWorkBench` (`reference/Bund/src/stdlib/functions/values/push.rs:11-25`) —
so one mechanical paired test per base word covers all 51. This is the
cheapest coverage available, and D18 preserves them as pairs regardless.

**395 have no covered base at all.** Genuinely untouched surface. Testing
these means probing the oracle for behaviour, not reading its source. By
subsystem, the largest are `bund/string` (47), `bund/statistics` (42),
`vm/math` (33), `vm/logic` (26), `bund/filesystem` (23), `bund/bund` (19),
`bund/math` (19), `bund/sysinfo` (19).

`bund/statistics` is the sharpest case: 42 uncovered words, and the
reachability pass shows the entire `bund/forecast` family depends on it, so
any per-word ruling preserving a forecast word commits to machinery with zero
coverage.

### Known limitation: coverage is per word, not per behaviour

The metric counts a word as covered when any golden invokes it. It cannot see
that a polymorphic word is only partly exercised.

`execute` — spelled `!`, 69 invocations across 39 of 132 programs — is the
worked example. It dispatches on eight arms
(`reference/rust_multistackvm/src/stdlib/execute.rs:26-97`):

| Arm | Reached by the corpus |
|---|---|
| `PTR` | yes — `reference/Bund/examples/bund_dynamic_demos/dynamic_demo_2.bund:31` |
| `LAMBDA` | yes — `reference/Bund/examples/bund_dynamic_demos/resolving_lambda.bund:33` |
| `CONDITIONAL` | yes — `reference/Bund/examples/code_snippets/application_conditional_demos.bund:13` |
| `OBJECT` | yes — `reference/Bund/examples/object_oriented_programming/value_demo.bund:19` |
| `STRING` | no |
| `CLASS` | no |
| `LIST` | no |
| `MAP\|INFO\|CONFIG\|ASSOCIATION` | no |
| non-executable (error) | no |

`!` therefore counts as covered while roughly half its dispatch is untested.
D21's probes close the testing gap; they do not make the metric finer. Read
the coverage number as an upper bound.

### `unblock` needs redesigning

`cargo xtask unblock` is specified as "for each unimplemented word, count
hermetic examples it alone gates; sort descending — this is the M6 work
queue". That ranking can only ever see the 140 words the goldens touch. As
specified it would report an **empty work queue with 446 words
unimplemented**: you would implement the covered set, watch `conform` reach
77/77, and find nothing left to do.

Whatever replaces it has to rank against the coverage denominator rather than
the golden corpus. Recorded here rather than fixed, because the replacement
ranking depends on how D14 settles the in-scope set. The command's help text
in `xtask/src/main.rs` carries the warning so it cannot be implemented as
written by accident.

## Lexer fidelity

The `.bund` lexer in `xtask/src/corpus/lex.rs` mirrors
`reference/bund_language_parser/bund.pest`. Across all 132 programs it reports
**zero anomalies**: every token matched a rule in the grammar as written. The
lexer is covered by 11 unit tests, including that `{}` inside a string does
not open a lambda (`reference/Bund/tests/testing_ifthenelse.bund:1`) and that
`1e5` lexes as integer `1` followed by name `e5`, since `float`
(`bund.pest:23`) requires a `.` before any exponent.
