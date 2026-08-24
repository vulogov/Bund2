# Bund2 — Addendum: Dynamic Lambdas, the OOP Layer, and the Debugger

**Status:** Research / pre-RFC addendum
**Date:** 2026-08-23
**Governing constraint:** Bund syntax as defined and Bund logic must be 100% preserved.

Read at HEAD for this addendum: `vulogov/Bund` (200 files, 18,996 LOC), specifically
`src/stdlib/functions/oop/` and `src/stdlib/functions/debug_fun/`.

---

## 0. Two corrections to the earlier documents

Reading the main `Bund` repository invalidates two things I wrote earlier. Both matter
enough to state before answering the questions.

### 0.1 The scope was understated

The main study counted **156 words** in `rust_multistackvm`. The CLI registers **357
words**, plus **37 methods** and **13 classes**, across 18,996 LOC — roughly 1.4× the
combined size of the five library crates. Effort estimates and stack-effect-annotation
budgets in the main study §5 should be scaled accordingly.

### 0.2 `bund.eval` exists — the open question is answered, and the answer is the harder one

Main study §10 Q1 and native-binary addendum §10 Q1 both asked whether any word turns a
string into code at runtime. The CLI registers:

`bund.eval`, `bund.eval.`, `bund.eval-file`, `bund.eval-file.`, `compile`,
`load.script`, `load.lambdas`, `save.lambdas`, `load.aliases`, `save.aliases`,
`make.call`, `curry`, `lambda!`, `lambda*`, `lambda=`, `?word`, `?lambda`, `?alias`,
`?class`, `?object`, `?stdlib`

`bund_eval.rs` pulls a string off the stack and calls `bund_compile_and_eval`. So:

- **The full front end — pest grammar, AST, IR lowering — is a mandatory runtime component
  of every Bund2 binary.** Not optional, not feature-gated. The native-binary addendum
  treated this as conditional; it is not.
- **Closed-world analysis (native-binary addendum §4) has a much larger trigger set** and
  will rarely fire on real programs. `load.lambdas` alone means the word table can be
  rewritten from a file on disk. Direct calls and cross-word inlining should be treated as
  a rare bonus, not a planned optimization.
- The compiler is not a build-time tool that happens to also JIT. It is **a runtime
  service** that `bund.eval` calls. That is a cleaner framing than the one in the main
  study, and the architecture already has the right shape for it.

This is not bad news for the design. It is bad news for any hope of treating Bund2 as a
mostly-static language.

---

## 1. Dynamically created lambdas

**Feasible, and they get materially better than they are today.** But it forces one
correction to the main study §4.3.

### 1.1 The correction: a lambda body is program-visible data, not IR

The main study assumed a lambda would *be* an `IrBody`. That is wrong under 100%
preservation. `Value::to_lambda(Vec<Value>)` builds a lambda from an arbitrary vector of
values, `make.call` fabricates `CALL` values at runtime, and `save.lambdas` /
`load.lambdas` round-trip the registry through bincode. A Bund program can construct,
inspect, and serialise a lambda body as ordinary data. That property is the substrate the
whole metaprogramming story stands on, and replacing it with an opaque IR breaks it.

**Design: keep `Vec<BundValue>` as the canonical, program-visible lambda body, and hang a
lazily-populated compiled form off it.**

```rust
pub struct Lambda {
    body:     Vec<BundValue>,        // canonical. What the program sees, edits, serialises.
    compiled: RefCell<Option<Rc<Compiled>>>,  // cache. Invalidated on mutation.
}

pub struct Compiled {
    ir:    IrBody,
    code:  Option<*const u8>,        // Tier 1, if hot
    calls: Cell<u64>,
}
```

Lowering `Vec<BundValue>` → `IrBody` happens on first execution and is cached. Any
mutation of `body` (`push`, `set`) drops the cache. Serialisation writes `body` only —
compiled code is never serialised, which is exactly what AOT is for.

This preserves every metaprogramming behaviour unchanged while giving dynamic lambdas
something they have never had: **a compilation path.** Today `lambda_eval` walks a
`Vec<Value>` calling `apply` per element, and `get_lambda` deep-clones the whole vector on
every invocation. In Bund2 a runtime-built lambda called 10,000 times gets lowered once and
compiled once.

### 1.2 Identity for tier-up

Anonymous lambdas need a call counter, and the obvious key — pointer identity of the `Rc` —
fails for lambdas rebuilt from parts on each pass through a loop. **Key the compilation
cache on a content hash of the body.** Two consequences, both good:

- structurally identical lambdas built independently share one compiled artifact;
- a metaprogramming loop that regenerates the same lambda shape repeatedly compiles it
  once rather than never.

Combined with the code-memory cap from the main study §3.2a, this also bounds the damage
from a program that generates genuinely unbounded distinct lambdas: distinct shapes hit the
cap and stay at Tier 0.

### 1.3 What stays interpreted

Per the native-binary addendum §3, case 3 — computed lambdas, computed word names,
`bund.eval` output — cannot be AOT-compiled at build time. Under the JIT they can still
tier up at runtime. Under a pure AOT binary they run at Tier 0. Both are correct; the
difference is only speed.

**Verdict: yes, and this is one of the places Bund2 is a clear improvement rather than a
preservation exercise.**

---

## 2. The OOP layer

**It ports unchanged, because it is a library, not a language feature — and it can be made
substantially faster without touching its semantics. But it breaks one of the main study's
central recommendations, and that has to be resolved first.**

### 2.1 What the layer actually is

Read from `oop/base_classes.rs`, `oop/value_class.rs`, `oop/object_execute.rs`, and
`rust_multistackvm`'s `bund_object.rs` / `multistackvm_object.rs`:

- A **class** is `Value::make_class()` — a `CLASS`-tagged MAP holding `.class_name`,
  `.super` (a LIST of parent class *names* as strings), and method slots whose values are
  either `PTR` (naming a native method in `VM.methods_fun`) or `LAMBDA`.
- **Instantiation** (`make_bund_object`): `dup()` the class, flip `dt` to `OBJECT`, resolve
  each `.super` name to its registered class, recursively instantiate it as an *embedded
  parent object* in the `.super` list, and run each parent's `.init` — dispatching on `PTR`
  → native fn or `LAMBDA` → `lambda_eval`.
- **Dispatch** (`VM::m`): peek the OBJECT, `locate_value_in_object` walks depth-first
  through `.super`, then `PTR` → native, `LAMBDA` → `lambda_eval`, anything else → push.
- `#` / `#.` (`object_execute.rs`) apply a lambda to an object's unwrapped `.data`.
- Registered hierarchy: `Object → Printable → Display`, plus `Value`, `Integer`, `Float`,
  `Bool`, `List`, `Floats`, `Intervals`, `IMAGE`, `OLLAMA`, `DEEPSEEK`.

The important structural fact: **methods are already either native functions or Bund
lambdas, and the dispatcher explicitly handles both.** That is the same duality as words
(previous addendum §1), so it maps onto the same machinery. If §1 works, this works.

### 2.2 The conflict: `.id` and `.timestamp` are load-bearing

`base_classes.rs`:

```rust
fn register_method_id(vm: &mut VM) -> … { … vm.stack.push(Value::from_string(value.id)); … }
fn register_method_timestamp(vm: &mut VM) -> … { … vm.stack.push(Value::from_float(value.stamp)); … }
```

These are registered on the base `Object` class, so **every object in Bund exposes the
per-value nanoid and the per-value construction timestamp as public API.**

The main study §4.1 recommended deleting both fields from `Value` — and identified them as
the single largest cost in the hot path (a heap allocation plus RNG per value, plus a clock
read per value). Main study §10 Q2 asked whether they were load-bearing anywhere. **They
are.** Under 100% preservation they cannot simply be removed.

Recalibrating the cost, honestly: `SystemTime::now()` is a vDSO call on Linux, on the order
of tens of nanoseconds — not free, but not the disaster. **`nanoid!()` is the real problem**:
RNG plus a 21-character heap allocation per value constructed.

Options:

| Option | `.id` semantics | `.timestamp` semantics | Value size | Verdict |
|---|---|---|---|---|
| (a) Keep as-is | exact | exact | ~180 B | forfeits the main win |
| (b) `id: u64` monotonic counter, rendered as string by `.id` | unique per value, format changes | — | +8 B | recommended |
| (c) Lazy nanoid derived from counter + VM seed on first `.id` call | exact format, exact uniqueness | — | +8 B | recommended if format matters |
| (d) id only on heap values | two separately-pushed `42`s share an id | — | +0 B | **semantic change — rejected** |
| (e) `stamp` from a periodically sampled clock + monotonic offset | — | ms-accurate, not per-value-exact | +8 B | acceptable at ms granularity |

`stamp` is already an `f64` of milliseconds since epoch, so (e) preserves observable
behaviour at the precision the field actually carries.

**Recommendation: (c) + (e).** `BundValue` becomes 24 bytes — `{ tag: u64, payload: u64,
birth: u64 }` — where `birth` is a monotonic counter serving as both identity and timestamp
basis. Still `Copy`, still allocation-free for scalars, still passed by Cranelift as three
`i64`s, and roughly 7.5× smaller than today. That is a worse number than the 16 bytes the
main study proposed, and it is the honest price of preserving the OOP layer's base class.

**This changes the Phase 1 RFC and must be settled before it is written.**

### 2.3 The opportunity: real vtables, no semantic change

`locate_value_in_object` walks the `.super` chain doing `HashMap<String, Value>` lookups
with `String` keys on **every method call**, at O(inheritance depth). `make_bund_object`
deep-`dup()`s the class and recursively instantiates every ancestor on **every**
instantiation.

Both are avoidable without touching semantics:

- **Method resolution is a pure function of the class graph.** At `register_class` time,
  flatten the `.super` chain into a per-class method table indexed by interned method
  symbol. Dispatch becomes one array index instead of a depth-N chain of string-keyed hash
  lookups. Invalidate on `register_class` / `unregister.class` via the same generation
  counter used for words (previous addendum §1.2).
- **Do not flatten fields.** `set_value_in_object` mutates values *inside* embedded parent
  objects, and `.super` holds real instantiated parents with their own `.data`. The object
  is genuinely a tree, and introspection can see it. Flatten the vtable; leave the data
  tree exactly as it is.

That split — vtable flattened, field tree preserved — is the whole answer, and it is likely
the largest single win available to OOP-heavy Bund code.

**Verdict: yes, portable as-is, faster, with one prerequisite decision (§2.2) that must
land in Phase 1.**

---

## 3. The debugger

### 3.1 What exists

| Word | Behaviour |
|---|---|
| `debug` | Pull a **string** off the stack, `bund_parse` it, and for each top-level term: print a `comfy_table` dump (type / value / `{:?}`), `vm.apply()` it, then drop into a `rustyline` loop where any Bund snippet can be evaluated; empty line advances to the next term |
| `debug.shell` | Unconditional REPL drop-in with history |
| `debug.display_stack` / `debug.display_workbench` | `comfy_table` dump of the current stack / workbench |
| `debug.dump` | Hexdump of the top value's raw bytes — `bytemuck::bytes_of` for scalars, `to_binary()` otherwise |
| `debug.display_hostinfo` / `.display_memstat` / `.display_distributed_info` | Environment info |
| `log.info` / `.warning` / `.error` / `.debug` / `.trace` | User-level logging via the `log` crate |

The interactive-inspection idea is good, and the `comfy_table` presentation is genuinely
nice. Three parts are worth keeping essentially unchanged: `display_stack`,
`display_workbench`, and `dump`.

### 3.2 The structural limits

1. **It debugs a string, not the running program.** `"…" debug` re-parses the snippet and
   steps *that*. There is no way to break into code already executing, no way to set a
   breakpoint inside a lambda or a word.
2. **Granularity is one top-level term of the debugged string.** `vm.apply(word)` executes
   an entire word or lambda call atomically. There is no step-into.
3. **There is no call stack to show.** The interpreter recurses on the Rust stack
   (`apply → lambda_eval → apply`), so execution state lives partly in native frames and
   cannot be inspected. This is the same fact that blocks async (async addendum §2.2).
4. No breakpoints, watchpoints, or conditional stops.
5. Under Bund2 there would be a fourth limit: JIT'd frames are opaque.
6. History files (`bund_debug_debugger_history.txt`) are written to CWD.

### 3.3 Proposal

The enabling refactor is already required for other reasons. Async addendum §2.2 mandates
that **Tier 0 be a flat loop over an explicit frame stack, no Rust recursion.** Once
`frames: Vec<Frame>` and `pc` are plain data, a real debugger nearly falls out.

**(a) Debugging is an execution mode of Tier 0, not a second parser.**
`Interp::step()` executes one IR op and returns. The debugger drives it. `step` /
`next` / `finish` are the three standard behaviours and all three are just comparisons of
frame-stack depth before and after — exactly the gdb model. Step-into works for words,
lambdas, and `bund.eval`'d code alike, because they are all IR bodies in frames.

**(b) A real backtrace.** `bt` prints `frames` with word symbol, IR offset, and source
span. Requires spans in BundIR (main study §4.3) — another consumer of that requirement.

**(c) Breakpoints, in three forms:**
- by word symbol — break when a frame for that symbol is pushed;
- by source location — file + line, resolved through the span table to an IR offset;
- conditional — the condition is **a Bund lambda**, evaluated in a child VM context at the
  breakpoint. This is natural here in a way it is not in most languages, because lambdas
  are already first-class values and `bund.eval` already exists.

**(d) Watchpoints on stacks — the Bund-specific feature.** `watch @errors` breaks when
anything is pushed to that named stack; `watch depth > 100` on the current stack;
`watch workbench`. No general-purpose debugger offers this because no general-purpose
language has named stacks as a first-class construct. This is where a Bund2 debugger can be
better than a port of gdb's model rather than a weaker version of it.

**(e) Inspection words, largely as they are.** Keep `debug.display_stack`,
`debug.display_workbench`, `debug.dump`. Add `stacks` (every stack at once, which the
multi-stack model makes the natural default view), `words` (slot table with tier,
generation, and call count), `classes` (registered classes with flattened vtables).

**(f) JIT interaction: reuse the async coloring machinery.** A word containing a breakpoint
is pinned to Tier 0 — the same fixpoint-over-the-call-graph mechanism as await points
(async addendum §2.4). That this mechanism now has two independent consumers is good
evidence it is the right abstraction. `bund2 run --debug` simply disables tiering globally.

**(g) Keep `"…" debug` working.** It becomes a thin wrapper: eval the string with the
debugger attached and a breakpoint on entry. Backward compatible, and now it steps *into*
things.

**(h) Separate tracing from logging.** The current `log.*` words are user-level logging and
should stay exactly as they are. Add a distinct execution trace at IR-op granularity behind
a flag, emitting `(depth, symbol, op, stack effect)`. It feeds the debugger, and it also
replaces the `time_graph::instrument` attributes currently scattered through the hot path —
the tier-up call counters give you a profiler for free.

**(i) Move history to a config directory** rather than CWD.

**(j) Reachable but not promised: DAP and time travel.** With execution state fully
reified, a Debug Adapter Protocol server (VS Code, and every other DAP client) becomes
plausible, and so does snapshot/rewind. Both are consequences of the flat-frame-stack
design rather than features to plan for now. Worth noting so the state representation is
kept serialisable, and worth not committing to.

### 3.4 Verdict

Better than the current design, at low marginal cost, because the expensive prerequisite —
removing Rust recursion from the interpreter — is already required by async and by the
stack-depth bug it also fixes. The debugger is the third feature to fall out of that single
refactor, which is a strong argument for doing it properly in Phase 3 rather than
incrementally.

---

## 4. Consolidated impact on the plan

| Finding | Affects | Action |
|---|---|---|
| `bund.eval` exists | native-binary addendum §2, §4 | Front end is mandatory in every binary; closed-world analysis demoted to opportunistic |
| Lambda body must stay `Vec<BundValue>` | main study §4.3 | Canonical body + lazy compiled cache keyed by content hash |
| `.id` / `.timestamp` are public API | **main study §4.1, Phase 1** | `BundValue` is 24 B, not 16 B; lazy nanoid + sampled clock |
| OOP dispatch is O(depth) string lookups | new | Flatten vtables at `register_class`; leave the field tree alone |
| 357 words, 37 methods, 13 classes | main study §5, §6 | Scale effort and stack-effect-annotation estimates ~2.3× |
| Debugger needs reified frames | async addendum §2.2 | Same refactor; third consumer of it |
| Breakpoints need tier pinning | async addendum §2.4 | Same coloring machinery; second consumer of it |

The recurring pattern is worth naming: **three independent features — async suspension,
the debugger, and stack-depth safety — all reduce to the same Phase 3 refactor**, and
**two independent features — await points and breakpoints — reduce to the same tier-pinning
mechanism.** That convergence is the strongest signal so far that the Tier-0 design in the
main study is the right one.

---

## 5. Open questions

1. **Does `.id`'s exact nanoid format matter to any existing Bund program**, or is
   "unique opaque string" the contract? Determines option (b) vs (c) in §2.2.
2. **What precision does `.timestamp` actually need?** If ms is the contract, clock
   sampling (§2.2e) is free. If any program relies on sub-ms distinctness between two
   values constructed in sequence, it is not.
3. **How deep do real class hierarchies go?** Determines how much the vtable flattening in
   §2.3 is worth.
4. **Do any programs mutate a lambda body after construction** (`push` onto a LAMBDA), or
   is the body write-once in practice? Write-once would let the compiled cache skip
   invalidation entirely.
5. **Should `bund.eval`'d code be JIT-eligible at all**, or permanently Tier 0? Permanently
   Tier 0 is simpler and removes a whole class of code-memory growth; eligible is faster
   for REPL-driven and generated-code workloads. This is a policy decision, not a technical
   one.
