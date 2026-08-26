# Errata

Supersessions of `docs/research/`. One line each. Never edit the originals.

Format: `<document> §<section> -> superseded by <RFC> §<section> (<reason>)`

---

- `00-jit-feasibility.md` §4.1 -> superseded by the OOP finding in
  `03-metaprogramming-oop-debugger.md` §2.2: `.id` and `.timestamp` are public
  API on the base `Object` class, so the value cannot shrink to 16 bytes.

- `02-native-binaries.md` §4 -> superseded by `03-metaprogramming-oop-debugger.md`
  §0.2: `bund.eval` exists, so closed-world analysis rarely fires and the front
  end is mandatory in every binary.

- `00-jit-feasibility.md` §"Variadic words" (line 485) and the risk row at
  line 587 -> corrected by `cargo xtask corpus`: the survey the risk row asks
  for ("survey real Bund programs before committing") has been done, and the
  arithmetic fold-family is **entirely unused**. `*+`, `*+.`, `*-`, `*-.`,
  `**`, `**.`, `*/`, `*/.`, `*loop`, `*loop.` and the Unicode aliases `Σ`,
  `Σ.` have zero uses across all 132 programs. The mitigation "be prepared to
  restrict the `*`-family in JIT-able positions" is therefore unnecessary, and
  D12 accepts the barrier because it costs nothing measurable. Note the open
  question at line 650 — "how variadic is real Bund code?" — is answered: not
  at all, in the corpus. `lambda*` is the exception, at 2 uses, and it is not
  an arithmetic fold.

- `00-jit-feasibility.md` §5 (line 128) and `02-native-binaries.md` line 151
  -> corrected by `cargo xtask corpus`: both discuss `execute` /
  `stdlib_execute_base_inline` by its registered name, but **no program spells
  it `execute`**. It is spelled `!`, an alias registered at
  `reference/rust_multistackvm/src/stdlib/create_aliases.rs:5`, and that
  spelling is one of the five most-used words in the corpus — 69 invocations
  across 39 of 132 programs, 13 of them inside a lambda body. A survey keyed
  on the registered name returns zero and reads as "no dynamic dispatch",
  which is the opposite of the truth. `execute` is also polymorphic over eight
  dispatch arms, of which the corpus reaches four
  (`reference/rust_multistackvm/src/stdlib/execute.rs:26-97`).

- `00-jit-feasibility.md` §2 (lines 48-55) and
  `03-metaprogramming-oop-debugger.md` §1 (line 19) -> corrected by
  `cargo xtask corpus`: both describe word registration as coming from
  `rust_multistackvm` (156 words) plus the CLI (357 total), which is **two
  tiers**. There are **three**. `i_direct` tries the VM's own inline table
  (`reference/rust_multistackvm/src/multistackvm_inline.rs:42`) and on a miss
  falls through to the stack layer's own inline table (`:52`), which is
  `reference/rust_multistack` — and that third tier holds the core stack
  words: `take` (`rust_multistack/src/stdlib/workbench.rs:81`, used by 29
  programs), `drop` (`stdlib/drop.rs:71`, 16 programs), `dup_one`, `swap_one`,
  `return`. A scan of only the two documented crates reports those as
  unregistered. RFC-0002's slot table has to absorb all three.

- `04-consolidated-architecture.md` §2-§3 -> superseded by
  `docs/rfc/RFC-0000-architecture.md` as the statement of record for crate
  layout, boundaries and the tier model. §1.1 is deliberately not superseded:
  it mixes layout with value, symbol and lambda claims belonging to RFC-0001,
  RFC-0002 and RFC-0003, and RFC-0000 disclaims that scope.

- `00-jit-feasibility.md` §"leading `$`" (line 119) and §"`$name`" (line 556)
  -> corrected by F26: `$` does **not** bypass alias resolution. It skips the
  lambda check only. `call_internal_word` strips the sigil and calls `i()`
  (`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`),
  and `i()` resolves aliases first
  (`reference/rust_multistackvm/src/multistackvm_inline.rs:71-72`). Verified
  against the oracle: `1 "$dup" ptr !` resolves through the alias table.
  Line 556 proposes a distinct IR opcode premised on full bypass; that premise
  is false.

- `05-rfc-roadmap.md` §6, the row "24-byte `{tag, payload, birth}`; lazy
  `.id`; sampled `.timestamp`" -> superseded by RFC-0001: the measured value
  is **16 bytes**, and the tag is not in it. `cargo xtask layout` puts
  candidate A (identity in the heap header) at 16 and candidate D (the same
  plus the reference's `dt`) also at 16, because the tag is only ambiguous for
  heap types and rides in the header. Lazy `.id` and sampled `.timestamp` are
  unchanged and are D1 and D2. The 24-byte figure survives only as candidate
  C, the cheapest *inline*-identity shape, which the scan rules out.

- `00-jit-feasibility.md` §4.1's supersession above — "`.id` and `.timestamp`
  are public API on the base `Object` class, so the value cannot shrink to 16
  bytes" -> **retired by RFC-0001**. The premise is right and the conclusion
  does not follow. `.id` and `.timestamp` are public
  (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:91-92`) and `.id`
  returns a string (`:16`), but identity moves to the *heap header*, not off
  the value entirely, so both stay answerable while the value measures 16
  bytes — `cargo xtask layout`, candidate D. What the entry correctly rules
  out is candidate B, identity carried inline, which measures 32.
