# RFC-0014: SMYSL corpora — forming, researching and querying them from Bund

- Status: **Draft — revision 8** (2026-11-02). Written on the owner's
  request: "Create RFC for SMYSL project that will make it integrateable
  into Bund. The idea: use SMYSL ability to form, research and query SMYSL
  corpus/database of data. Before writing RFC, assess if this idea is
  reachable/worthy." The assessment is §A, before the design. The rulings
  are **D244**, points R1–R21, with R14 re-decided by revision 6, and
  **D245**, points R22–R25, raised by revision 8. **D245 is RESOLVED
  (2026-11-02):** the owner ruled R22–R25 A, as the design assumes.
  **D244 is RESOLVED (2026-11-02):** the owner ruled R1 A and R2 A, then
  R3–R21 "as recommended", so every point is A, as the design assumes.
  D244's heading still reads "R1–R12", because it predates the
  amendments that added the rest.
  - *Revision 2 (2026-11-02)* answers the first review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-02.md`): B1–B4, S1–S12 and
    N1–N12. The answer to each finding is listed at the end, in
    "Revision 2: the review answered".
  - *Revision 3 (2026-11-02)* answers the second review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-02-2.md`): B1–B4, S1–S12,
    N1–N13 and its assumptions, in "Revision 3: the second review
    answered". The largest changes:
    - `smysl.merge` leaves Phase 1;
    - `smysl.retract` goes through SMYSL's retraction plan and authority;
    - exported proofs keep gists within SMYSL's bound;
    - §S7.1 renders RFC-0013's read-back values.
  - *Revision 4 (2026-11-02)* answers the third review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-02-3.md`): B1, S1–S9, N1–N10
    and its assumptions, in "Revision 4: the third review answered". That
    review also corrected an error the second review made and revision 3
    built on: an attestation is a record of its own, outside the unit's
    identity.
  - *Revision 5 (2026-11-02)* answers the fourth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-02-4.md`): B1, S1–S8,
    N1–N12 and its assumptions, in "Revision 5: the fourth review
    answered". The largest changes:
    - gists are fitted to the library's own profiles by SMYSL's
      `fit_gist_with`;
    - a panic in a write leaves SMYSL's lock in place as its crash notice
      (R20);
    - retraction authority is stated as advisory, since agent ids are
      self-asserted (R21).
  - *Revision 6 (2026-11-02)* answers the fifth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-02-5.md`): B1, S1–S4, N1–N9
    and its assumptions, in "Revision 6: the fifth review answered". The
    largest changes:
    - **SMYSL has no `--break-lock` command.** Earlier revisions said it
      had. R14 is re-decided: a `bund2 smysl-unlock` subcommand, run by
      an operator, breaks a library's locks through `Lock::break_lock`.
      SMYSL is asked for its own command as U8.
    - Every write re-opens the library after taking its locks, and
      decides everything from that second open.
    - Gists are NFC-normalised before they are fitted.
    - R20 also keeps the locks after an append that fails part way.
  - *Revision 7 (2026-11-02)* adds the "Implementation plan", M1–M10, on
    the owner's "proceed as recommended". Q52 and Q55 were answered
    before it. No design changes.
  - *Revision 8 (2026-11-02)* answers the sixth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-02-6.md`), the first by a
    reader independent of the earlier five: B1–B3, S1–S8, N1–N12 and its
    assumptions, in "Revision 8: the sixth review answered". The largest
    changes:
    - **Five SMYSL commands append to a library without its locks.**
      Every earlier revision said each SMYSL writer locks. The claim is
      withdrawn, what Bund2 can promise is stated, U9 asks SMYSL to lock
      them, and whether Phase 1 waits for that is R22 (D245).
    - The implementation plan is reordered, so that no milestone claims a
      criterion before the words it uses exist.
    - Criterion 15's failed-append and re-open cases are given mechanisms
      that exist: an unwritable shard log, and a probe that pauses a
      write.
    - Whether a failed write keeps the locks is decided by comparing the
      library's stamp before and after the call. That restates the test
      under R20, which is ruled, so it is put as R25.
    - A `retracts` relation written through `smysl.rel` or `smysl.append`
      is R23; extension relation kinds are R24.
- Depends on: RFC-0001 (values), RFC-0002 (words), RFC-0004 (effects),
  RFC-0005 (promotion), RFC-0007 (threads and the bus), RFC-0013
  (knowledge bases, proof trees)
- Decisions consumed: D14, D18, D26, D27, D28, D32, D36, D37, D38, D49,
  D51, D72, D168, D220, D226 (L11), D230
- Reference SHA: `reference/Bund` 21b40b0, `reference/rust_multistackvm`
  4605832, `reference/rust_dynamic` ceb27c9
- External source: **SMYSL at commit 4f5ecb9**,
  https://github.com/vulogov/smysl.
  - 4f5ecb9 is "Merge 1.14.0". Its tree is identical to the tag `v1.14.0`'s
    (`git diff v1.14.0 4f5ecb9` is empty).
  - It was read from a clone at that commit, file by file. Nothing was
    built or run.
  - **No release of SMYSL after 1.11.0 is on crates.io.** For each of the
    eight crates §S1 names, and for the facade, the registry's newest
    version is 1.11.0, published 2026-10-30 (asked of the crates.io API on
    2026-11-02). R2 decides what follows from that.
- Supersedes: nothing

**How SMYSL is cited.** A citation into SMYSL is written relative to its
crate directory, as in `smysl-graph/src/merge/retraction.rs:70`, or for a
file at its root as `smysl:README.md:34`. Both refer to commit 4f5ecb9.
These are not paths in this repository, and `cargo xtask cite` does not
resolve them; a reviewer opens them in a clone at that commit.

---

## Summary

- **Bund programs can build, study and ask a SMYSL library.**
  - SMYSL is a format and a Rust library for claims that carry their
    certainty, their grounds and their rebuttals, together with the texts
    those claims are drawn from.
  - Bund2 gains a `smysl.` word family over it, inside a Cargo feature that
    is **off by default** (D28).
- **Forming.** A program can:
  - create or open a library;
  - add texts through SMYSL's readers;
  - append units and relations built from Bund values;
  - retract under SMYSL's authority rules, and parse and write SMYSL's
    surface syntax;
  - run SMYSL's checks.
- **Researching.** A program can:
  - trace a unit's lineage;
  - read each unit's effective status after retraction;
  - rank units by salience, and pack them to a budget;
  - assemble a dossier for a question;
  - run SMYSL's measures.
- **Querying.** A program can run an `sq/1` query and get its answer as a
  Bund value, or run a lambda once per answer through a `smysl`
  conditional.
- **A bridge to RFC-0013, in both directions:**
  - A SMYSL scope becomes a knowledge base of facts, so Bund's rules can
    ask what `sq/1` cannot.
  - A `kb.prove` proof becomes SMYSL units. Their statuses are computed so
    that rule M holds, so a conclusion leaves Bund no more certain than
    what it rests on.
- **Phase 1 links no model, no async runtime and no HTTP client.** The
  steps of SMYSL that call a model are Phase 2 and are not designed here
  (R1).
- **Nothing in Bund's syntax, value types or existing words changes.**
  Every new word is Bund2's own, so `conform`, `COVERAGE` and `IMPLEMENTED`
  move by zero (D72).
- **§S11 lists what SMYSL should change** to make it a better dependency.
  None of it blocks Phase 1, but U6, publishing 1.14, decides R2.

---

## §A. Is it reachable, and is it worth it?

The owner asked for this before the RFC. The answer is **yes to both, for
the pure half**. The model half is also reachable, but it would add a
dependency set Bund2 has never linked.

### Reachable

| Requirement | Finding | Where |
|---|---|---|
| Form a corpus without a model | A library is created or opened by `Library::create` (idempotent) and `Library::open`. `Library::add` reads a text in, and its caller must hold the library's locks. `Library::append_records` appends records. SMYSL's `lib` and `text` commands hold the locks around it, and five of its saving commands do not (§Current, "The locks"; R22) | `smysl-text/src/library.rs:285-289,302,1001-1013,1341-1379`; `smysl:src/main.rs:6297,6324,8618-8628` |
| Research without a model | Lineage, retraction, salience, packing, dossiers and measures are pure functions of a store. A dossier and a measure return what they found and **write nothing** | `smysl-graph/src/lineage.rs:149`; `smysl-graph/src/merge/retraction.rs:70-127`; `smysl-query/src/dossier/mod.rs:325-326,819`; `smysl-measure/src/lib.rs:239-248` |
| Query | `parse`, `plan` and `run`, each answer one of five shapes | `smysl-query/src/parse.rs:42`; `smysl-query/src/plan.rs:680`; `smysl-query/src/exec.rs:70-83,115` |
| Query a library, not only a store | `smysl-text` implements the query crate's scope ports over an open library | `smysl-text/src/scope.rs:1-8` |
| No async runtime, HTTP client or argument parser in the pure set | SMYSL states it and checks it in CI | `smysl:README.md:34-35` |
| Values Bund can hold | Units, relations and diagnostics are structs of strings, sets of ids, enums and numbers. A payload decodes to a closed JSON-like tree, and the decode can fail | `smysl-core/src/types/unit.rs:25-58`; `smysl-core/src/types/relation.rs:175-186`; `smysl-core/src/diag.rs:528-536`; `smysl-core/src/surface/payload.rs:177`; `smysl-core/src/surface/hjson.rs:45-58` |
| Threads | `Store` is `Send + Sync`, which a test asserts | `smysl-graph/tests/store.rs:619-625` |
| A panic in SMYSL does not take Bund down | Every native call already runs under `catch_panic`, and a panic returns `Error::internal` (D49) | `catch_panic` in `crates/bund2-api/src/lib.rs` |
| Toolchain and licence | MSRV is 1.85 (1.88 for the facade); Bund2 pins 1.95.0. SMYSL is MPL-2.0, which is already on `deny.toml`'s allow list | `smysl:Cargo.toml:16-17,136`; `rust-toolchain.toml`; `deny.toml` |
| A source Bund2 may depend on | A git dependency pinned by revision is a mechanism Bund2 already uses, for `algos`. D38 justified that one by the oracle, which SMYSL is not, so allowing SMYSL's is R2's own ruling | `deny.toml`, `[sources]`; `crates/bund2-stdlib/Cargo.toml` |

Nothing here blocks Phase 1, unless R22 rules that it waits for SMYSL to
lock its saving commands (U9). These things are still unknown, and are
recorded as questions rather than blockers:

- what a library holds after an append interrupted part way (Q51);
- whether the pure `blake3` build invokes a C compiler (Q52, answered
  since: it does not);
- whether every type the words need can be built from outside SMYSL's
  crates (Q55, answered since: they can);
- what a dossier loses without stemming, and which template sets this
  build carries (Q56).

### Worth it

**What Bund gains.**

- **A corpus that keeps provenance.** Bund2 can persist values in the
  world file (D27), but it has no store of claims carrying sources,
  certainty or disagreement, and its database layer is deferred (D26).
  SMYSL is such a store. Identity is by content, merging needs no
  coordinator, and the store has checks.
- **Rules over that corpus.** `sq/1` filters and walks. It has no
  user-defined rules and no joins. Its pattern form, `match`, is parsed
  and refused with `SMY-E461`
  (`smysl:Documentation/SMYSL-2_RFC/SMYSL-2.9_QUERY_GRAMMAR.md:479-486`).
  RFC-0013's engine has rules, joins, recursion under conditions,
  negation, tabling and constraints. Loading a scope as facts (§S7) lets a
  program ask what the query language cannot.
- **Certainty for derived conclusions, without touching `q`.**
  - RFC-0013 deferred certainty factors (D226, L11), and `q` is reserved
    for fuzzy math (D32).
  - SMYSL's rule M says that "a conclusion may never be more certain than
    the weakest thing it rests on" (`smysl:README.md:152`).
  - §S7 applies rule M to exported proofs, which gives each conclusion a
    status without a number.
- **Scripting for SMYSL.** SMYSL's operations are offered as a CLI and a
  library. Bund gives them a language to be composed in, with stacks,
  lambdas, graphs (RFC-0012) and statistics alongside.

**What SMYSL gains.** The inference it does not do. Its `check` "verifies
consistency, never correctness" (`smysl-check/src/lib.rs:8-9`). No pure
crate read for this RFC derives new units from old ones; the measures are
the exception, and what they derive is data.

**What it costs.**

- **Size.** The crates Phase 1 needs (core, graph, check, query, retrieve,
  pack, text, measure) are about 147,000 lines of Rust, tests included,
  at 4f5ecb9, counted with `wc -l` for this RFC. They are compiled only
  under the feature (D28).
- **Youth, and an unpublished version.**
  - SMYSL's first commit is 2026-07-24, and it has cut 29 tags since.
  - Its API contract names the facade's twelve operations
    (`smysl:src/lib.rs:47-53`). It does not name the query module, the
    library or the measures, which are what this RFC uses most.
  - The version read here is not on crates.io (R2).
- **Panics remain in SMYSL's shipped code.**
  - SMYSL forbids `unsafe` (`smysl-core/src/lib.rs:11`) and guarantees no
    panic on untrusted input (`smysl:src/lib.rs:16`).
  - It does not deny `unwrap`, `expect` or `unreachable!`, and some remain
    on internal invariants: for example
    `smysl-graph/src/store/index.rs:194-195`,
    `smysl-query/src/parse.rs:188` and `smysl-check/src/passes/trust.rs:38`.
  - D49 turns any of them into a reported error. §S8 says what is left on
    disk afterwards, and §S11 asks SMYSL to deny them.
- **No oracle.** The reference has no SMYSL words, so no golden can test
  them. Unit tests and an example pin them, as they do the `kb.` family.

**Verdict.** It is worth it if Bund is meant to be a research and
reasoning layer over curated corpora, which is what SMYSL is for. The two
do not overlap; each supplies the half the other lacks. If that is not a
use Bund is meant for, the size and youth of the dependency do not justify
it, and the right ruling on R1 is to stop here.

---

## Motivation

Bund cannot keep a body of knowledge with its sources and ask questions
about it.

- **The world file stores bare values.** It stores named values, opened by
  name for each call and closed after it
  (`reference/Bund/src/stdlib/functions/bund/bund_models.rs:44,54,105,115`).
  Bund2 keeps them in redb (D27). A value carries no source, no certainty
  and no relation to any other value.
- **There is no database layer.** The embedded one, `bund/internaldb`, is
  deferred (D26).
- **Knowledge bases do not persist.** RFC-0013's knowledge bases are values
  held in a program. They reason, but they do not persist, do not merge
  across agents, and do not keep what a fact was drawn from.

SMYSL is a store of exactly that:

- **Units whose identity is their content,** each with a status on a
  six-level scale (`smysl-core/src/types/epistemics.rs:21-29`), and with
  grounds and dependencies (`smysl-core/src/types/unit.rs:33-34`).
- **Relations between units,** among them `rebuts` and `retracts`.
- **A library** holding the texts the units are drawn from.
- **Checks** that keep the statuses honest.
- **Retraction that cascades:** a withdrawn ground takes down whatever
  rests on it (`smysl-graph/src/merge/retraction.rs:70-127`).

The owner wrote both projects. This RFC joins them.

---

## Current behaviour

### Bund and Bund2

- **No SMYSL word exists.** Nothing under `reference/` names SMYSL, and
  `bund2 words` lists no `smysl` name. Both were checked by search for this
  RFC.
- **Persistent values live in the world file.**
  - `save.model` and `load.model` open the world file by name for each
    call (`reference/Bund/src/stdlib/functions/bund/bund_models.rs:44,105`)
    and close it before returning (`:54,115`).
  - Under `--noio` both are registered as stubs that refuse
    (`reference/Bund/src/stdlib/functions/bund/bund_models.rs:125,129,140-146`).
    Every one of the reference's 16 `cli.noio` sites registers words
    that touch the filesystem, the terminal, the process or the network
    (a search of `reference/Bund/src`; for example
    `reference/Bund/src/stdlib/functions/io/banner.rs:82`).
- **A structured value is a MAP with a `type`.**
  - A graph is a MAP with `type = "graph"` (`graph_word` in
    `crates/bund2-stdlib/src/graph.rs`).
  - A knowledge base is a MAP with `type = "kb"` (`Kb::read` in
    `crates/bund2-stdlib/src/kb/store.rs`; RFC-0013 §S2).
  - No Bund2 value type holds a native resource; every payload is data
    (`Payload` in `crates/bund2-value/src/lib.rs`).
  - `bund2-api` has no per-VM extension slot (RFC-0013 §S2).
- **Conditionals.** RFC-0013's `query` runs a lambda for each answer
  (§S11).
  - `bund2 check` has one arm for the conditionals that run a lambda per
    row or answer: `"csv" | "sqlite" | "query"`.
  - It has others for `ifthenelse`, `tryexcept`, `curry` and `context`.
  - It abandons any type it does not list (the conditional match in
    `crates/bund2-stdlib/src/check.rs`).
- **Effects and promotion.**
  - `check` reads a fixed-effect native's declared effect (`effect_of_name`
    in `crates/bund2-stdlib/src/check.rs`).
  - An opaque word is never promoted, and a native that may re-enter
    evaluation must be opaque.
  - The `REENTERING` audit in `crates/bund2-stdlib/src/lib.rs` is
    different. It is a source scan, independent of features, for
    *functions* that directly call `eval_lambda`, `apply` or
    `scoped_call`, compared by set equality with a list of
    `"<file>: <fn>"`. The KB engine's entry there is `kb/bridge.rs: run`;
    `kb.prove`'s own function is not listed.
  - `no_fixed_effect_native_runs_a_body` feeds only lambdas to fixed-effect
    natives, so it cannot catch one that re-enters by another route.
- **Audits of feature-gated and host-acting words.**
  - The palette audit runs every fixed-effect native on palette values,
    except those in two lists, `ACTS_ON_HOST` and `FEATURE_GATED`, in
    `crates/bund2-stdlib/src/lib.rs`.
  - "A new feature-gated native runs for real until it is named here."
  - `PROMOTABLE.txt` is one file for two builds (F123): a feature-gated
    native is excluded from both builds. The header says so
    (`tests/golden/PROMOTABLE.txt:8-15`), and the names are listed as
    `# feature-gated:` lines at the file's end (`:513-516`).
  - `FEATURE_GATED` is a `const` local to the palette audit's test
    function. `UNVARIED` is asserted in another test function, which does
    not see it.
- **Features.**
  - A feature routed through the `bund2` facade reaches no code (F124,
    D168).
    `bund2-cli` forwards a feature through `bund2-runtime` and the crate
    that holds it.
  - A bundle records the features of the runtime that built it
    (`features_built_in` in `crates/bund2-cli/src/main.rs`).
  - CI builds and tests features in its `feature matrix` step
    (`.github/workflows/ci.yml`).
- **Proofs.** `kb.prove` is `opaque(2)` and answers each solution with a
  proof tree (RFC-0013 §S9, §S15).
  - A rule node is `[ goal ← p₁ … pₖ ]`. It names neither its clause nor
    the KB a fact came from.
  - The other node kinds are `:fact`, `:branch i`, `:then`, `:else`,
    `:findall`, `:no-proof`, `:builtin`, `:bridge λ′`, `:asked`, `:known`
    and `:tabled`.
  - `call` is transparent. `once` is `[ goal ← p ]`, or `[ goal ← p₁ … pₖ ]`
    over k goals (D230), so by its shape it is a rule.
  - `:asked` and `:known` arise only under the `query` conditional, which
    alone has `ask` and `known` slots. Through any other asking word, an
    askable goal is E50 (RFC-0013 §S16; `ask` in
    `crates/bund2-stdlib/src/kb/solve.rs`).
  - The goal is a **value read back** (RFC-0013 §S1's table):
    - atoms come back as STRING, and `true` and `false` as BOOL;
    - a compound comes back as a LIST headed by a CALL;
    - an unbound variable comes back as NODATA;
    - a partial list's tail comes back as a `|` CALL then NODATA, and an
      improper one's as `|` then the value.
- **Panics in dependencies** are caught where the native is called, and
  become `Error::internal` naming what was running (D49; `catch_panic` in
  `crates/bund2-api/src/lib.rs`, called from `Interp::invoke`).
- **Words report; they do not print** (D36): a `Diagnostic` goes through
  `Vm::report`. Refusal texts begin `<W>: `, the word's name in capitals
  (RFC-0013 §S12).
- **Workbench forms.** D18 gives a `.` form to every word D14 *preserves*.
  RFC-0013 gave `.` forms to the eight `kb.` words that transform a KB
  value (§S9).

### SMYSL, as this RFC uses it

- **Units.**
  - `UnitCore` holds a schema, a gist, an optional body and detail, `deps`,
    `grounds`, a status, an optional source and payload, a language, and
    `extra`, the unknown keys of a later minor version
    (`smysl-core/src/types/unit.rs:25-58`).
  - Its hash is its identity: `uid = BLAKE3-256(det_cbor(UnitCore))`
    (`:20-22`), so `extra` is part of it (`:56-57`), and a change of status
    is a new uid.
  - A unit SMYSL refuses to build is one with:
    - an empty gist;
    - detail but no body;
    - status `unfounded`;
    - status `cited` or `measured` and no source;
    - status `derived` or `inferred` and no grounds.

    (`smysl-core/src/types/unit.rs:146-178`; `smysl-core/src/types/epistemics.rs:77-80`.)
- **Statuses.**
  - The order is `unfounded` < `speculative` < `inferred` < `derived` <
    `cited` < `measured`. `unfounded` is reachable only by retraction
    (`smysl-core/src/types/epistemics.rs:21-29`).
  - A source has `kind`, `reference`, `captured`, `observed` and
    `published` (`smysl-core/src/types/epistemics.rs:330-372`), and a byte
    span with its manifest (`:373-402`), which are read by the accessors
    `span` and `manifest` (`:524,529`) and set only by `SourceRef::in_part`
    (`:514`). All of it is part of identity.
  - `measured` may be assigned only by an import recording
    `op: Imported` (`smysl-core/src/types/epistemics.rs:325-327`).
- **Rule M** applies only to `derived` and `inferred`
  (`smysl-core/src/types/epistemics.rs:82-85`). It reports a unit whose
  status exceeds that of its weakest present ground, in one pass in
  dependency order, and does not propagate
  (`smysl-check/src/passes/epistemics.rs:23-66`).
- **Attestations and trust.**
  - An attestation names an agent, a hop count, parents, a clock, an
    operation and a rung (`smysl-core/src/types/provenance.rs:249-256`).
    `Attestation::new` is public and leaves the hop at 0 and the parents
    empty (`:269-282`). `trace`'s `parents` kind follows attestation
    parents (`smysl-graph/src/lineage.rs:30`).
  - Salience reads the parents too.
    - It groups a unit's attestations by agent and recipe, collecting
      each group's parents as its ancestry.
    - It counts as independent corroboration only the groups "whose
      ancestry is disjoint from everything kept so far"
      (`smysl-graph/src/salience.rs:353-367`).
  - "A unit's hop is the earliest any agent attested it", and `hop_diff`
    partitions units by it (`smysl-graph/src/lineage.rs:718-736`).
  - The clock is a hybrid logical clock. `Hlc::now(prev, agent)` advances
    from a previous clock (`provenance.rs:55-57`): to the wall time if that
    is later, and otherwise to `prev`'s time with its counter plus one
    (`:61-76`). `Hlc::new` is public (`:33`).
  - An agent id matches `^(model|human|tool):…` (`smysl-core/src/ids.rs:521`).
    It is "self-asserted until COSE signing lands", and "corroboration is
    gameable by fabricated attestations until then" (`:523-524`).
  - The operations are `Authored`, `Transformed`, `Imported` and
    `Attested` (`smysl-core/src/types/provenance.rs:118-122`).
  - Each rung caps the status it can carry: `Computed` caps at `derived`,
    `Document` at `cited` (`:172-180,218-225`).
- **Retraction.**
  - Every target of a `retracts` edge becomes `unfounded`. Under the
    default strict policy, a unit whose present grounds are all
    `unfounded` becomes `unfounded` too, repeated to a fixpoint
    (`smysl-graph/src/merge/retraction.rs:70-127`;
    `smysl-graph/src/merge/policy.rs:55-59`).
  - Who may retract defaults to `Origin`: "only an agent that attested a
    unit can retract it" (`smysl-graph/src/merge/policy.rs:106-116`).
  - That authority is checked by `plan_retraction`, which takes the
    agents and the authority (`smysl-graph/src/merge/retraction.rs:181-191`),
    and not by `effective_status`, which honours any `retracts` edge.
  - SMYSL writes a retraction as a self-edge, `retracts` from the target to
    itself (`:218`).
  - SMYSL's `retract` command:
    1. does nothing, and says so, when `effective_status` already reads
       the target as retracted. It checks this before authority.
    2. Otherwise it refuses an unauthorised plan.
    3. Otherwise it writes the self-edge, **with no attestation**.

    (`smysl:src/main.rs:4484-4505`.)
- **Lineage.** `trace` takes a `TraceKind`:
  - `parents` (attestation parents and `supersedes`);
  - `grounds` (`grounds` and `deps`);
  - `both`;
  - `rests-on` (`smysl-graph/src/lineage.rs:29-55`). That kind's walk is
    `trace_via`, over the edges the caller names (`:222-233`).

  Dependents are a separate call, `dependents_trace`, over the edges it is
  given (`:389-394`). `EdgeSet::premises` is `deps`, `grounds` and
  `conditions` (`smysl-graph/src/adjacency.rs:165`).
- **Salience and packing.** `salience` (`smysl-graph/src/salience.rs:229`)
  and `pack` (`smysl-pack/src/solve.rs:242`).
- **Merge** works on a bare `Store` (`smysl-graph/src/merge/mod.rs:100`),
  and SMYSL's CLI merges store files.
- **Relations** carry a kind, two uids, an optional weight and an optional
  note, plus attestations and `extra` (`smysl-core/src/types/relation.rs:175-186`).
  A weight is clamped to [0, 1] and quantised to 1/1024 (`:179-181`,
  `:205-208`), and a float in a payload is quantised too
  (`smysl-core/src/surface/payload.rs:253-257`).
  - A payload decodes to an object, `HObject`
    (`smysl-core/src/surface/payload.rs:177-179`).
  - `UnitCore::new` NFC-normalises the gist, body and detail, and drops a
    body or detail that is empty (`smysl-core/src/types/unit.rs:147-152`).
    Its `normalise` is `pub(crate)`, built on `unicode-normalization`
    (`smysl-core/src/types/mod.rs:274-281`; `smysl-core/Cargo.toml:20`).
    NFC can lengthen a string: a composition exclusion such as U+0958
    becomes two code points.
  - A source's `observed` is a `u64` of milliseconds
    (`smysl-core/src/types/epistemics.rs:349`). `SourceRef::in_part`
    answers `None` for an empty or reversed span (`:514-517`).
- **Labels.** After a merge, one label may be bound to several units, and
  SMYSL reports the collision without resolving it
  (`smysl-graph/src/labels.rs:3-8`).
- **Diagnostics** carry a code, a severity, a subject, a message and an
  optional suggestion (`smysl-core/src/diag.rs:528-536`). The subject is a
  span, a unit, a record or the store (`:510-522`).
- **Granularity.**
  - `check`'s profile is the one its options name
    (`smysl-check/src/lib.rs:172-175`). Without one, it comes from the
    store's views (`:255-268` and the lines after):
    - with no view, it is the `default` preset;
    - with one view, it is that view's profile;
    - with several, it is the widest envelope over them: the largest
      `l0_max`, counted by the first view's estimator.

    `with_granularity` names one (`:209`).
  - The `default` preset is built by `GranularityProfile::standard()`. It
    bounds a gist at 30 tokens and a body at 40 to 120
    (`smysl-core/src/types/view.rs:123-131`), and `preset` maps the name
    `"default"` to it (`:150-153`). Every preset bounds a gist at 30
    tokens (`:109-148`).
  - **A view is not limited to the presets.**
    - The surface parser takes a preset by name and then sets any
      `l0_max` the view gives (`smysl-core/src/surface/parse.rs:907-908`).
    - A profile also names its token estimator, carried in CBOR key 5
      (`smysl-core/src/types/estimate.rs:228`).
    - The default estimator is `ceil(bytes / 4)`
      (`smysl-core/src/types/mod.rs:42-44`). `smysl/content/1` weighs a
      CJK character at 0.931 tokens and a space at 0.456
      (`estimate.rs:50-58`).
  - `GranularityProfile::fit_gist_with(text, suffix)` gives the longest
    prefix of `text` that fits `l0_max` under the profile's own estimator,
    with room for `suffix`. That prefix is 0 bytes when no character fits,
    and also when the suffix alone exceeds the bound
    (`smysl-core/src/types/estimate.rs:189-203`). It answers `None` for an
    estimator the build
    does not have, which leaves the bound unevaluable (`SMY-W025`)
    (`smysl-core/src/types/view.rs:189-212`).
  - A gist over the bound is the error `SMY-E022`
    (`smysl-check/src/passes/shape.rs:42-50`).
- **Records and de-duplication.**
  - A unit is the record `Record::Unit(UnitCore)`, and each attestation is
    a record of its own, `Record::Attestation`
    (`smysl-core/src/types/record.rs:85-88`).
  - Attestations are "not hashed" and "accrete": a unit's support grows as
    agents corroborate it (`smysl-core/src/types/provenance.rs:243-246`).
  - A library's write skips a record whose encoding is already present,
    or that appears twice in one batch, before it writes the shard logs
    (`encode_by_shard`, `smysl-text/src/shards.rs:516-522`, called from
    `library.rs:1355-1356`). A bare store's `append` does the same for
    itself (`smysl-graph/src/store/mod.rs:434-446`). So re-appending a unit
    adds nothing to a log, and a new attestation of it is one new record.
- **The library.**
  - `Library::create` creates a library, or opens one that is already
    there (`smysl-text/src/library.rs:285-289`). `Library::open` opens one
    (`:302`).
  - `catalog` gives, for each alias, its **heads**: the newest manifest of
    each line of versions (`:1586-1590`).
  - A text's `licence` is required (`:95`).
  - A library's in-memory store has no path, and its `append` "writes
    nothing, having no path"; records reach disk through the library's
    logs (`:1372-1378`).
  - `Library::add` reads a text into the library (`:1008-1013`). Its
    caller must hold "the library lock, then the shards in name order"
    (`:1001-1004`).
    - It takes an `AddSpec` and `Caps`. `AddSpec`'s fields are `reader`,
      `params`, `alias`, `lang`, `licence`, `carry`, `policy`, `title`, a
      pseudonym key and a versification scheme (`:86-116`).
    - `carry` defaults to `Carry::None`
      (`smysl-core/src/types/library.rs:32-36`).
    - An absent `policy` means the reader's top level as the part
      boundary, with `Policy::default()` otherwise
      (`smysl-text/src/library.rs:1176-1179`).
    - `Caps::default()` is SMYSL-2.4 §3.9.1's caps, among them 1 GiB per
      input (`smysl-text/src/limits.rs:27-33,79,98-101`). A cap exceeded
      is `SMY-E440` (`smysl-core/src/diag.rs:317`).
    - A reader takes bytes, never a path: "a reader that could open a
      path would be a reader the purity gate cannot describe"
      (`smysl-text/src/readers/mod.rs:45-53`).
  - `Library::open` reads every shard's log into the in-memory store
    (`:327-350`), and nothing reloads it afterwards.
  - `Library::open` also writes. When the library records redactions, it
    unlinks the objects of the redacted parts (`enforce_z`,
    `:370,676-698`), and takes no lock to do it. Every open does this,
    SMYSL's own included.
  - `append_records` appends records (`:1341`). It refuses a batch
    `check_loggable` refuses before any shard is written (`:1347-1354`).
    It appends to each shard's log in a loop, then to the in-memory store
    (`:1364-1379`). Its doc comment names no lock; who locks is under
    "The locks".
    - A unit citing a part is routed to the shard of that part's alias
      (`smysl-text/src/shards.rs:456-466`), and the loop creates that
      shard's log if it is new (`smysl-text/src/library.rs:1364-1369`).
    - In a layout-1 library, a batch that creates a shard first rewrites
      the `LIBRARY` marker to layout 2 (`:1355-1361`). Layout 2 dates from
      SMYSL 1.10.0 (`smysl:CHANGELOG.md:958,1323-1325`), so releases
      before it no longer read the library whole.
    - A shard log's length grows only once a write and its `sync_data`
      have both succeeded (`smysl-text/src/shards.rs:380-382`). The
      library's stamp is a function of those lengths and hashes
      (`smysl-text/src/library.rs:993-995`).
    - Reading a log counts an undecodable tail as torn
      (`smysl-text/src/shards.rs:236-251`). The next append to that log
      cuts it back to the decoded length before writing (`:376-379`).
    - A shard that `lib migrate --primary` moved into the unit store
      cannot be read without `store-redb`, and the open fails with an I/O
      error naming the shard (`smysl-text/src/shards.rs:300-322`).
- **The locks.**
  - `acquire_all` takes the library's locks, in order
    (`smysl-text/src/lock.rs:24-43`).
  - A lock is taken by writing its holder to a staged file beside it, then
    hard-linking that file to the lock's path: "A hard link fails if the
    lock exists, which is the `create_new` this replaces"
    (`smysl-graph/src/lock.rs:150-158`). The module comment still says
    `create_new` (`:3-5`).
    - A lock already held fails with `SMY-E445`, naming the holder
      (`:165`).
    - Any other error from the link is `LibError::Io` (`:177`). So on a
      filesystem without hard links every write fails, and not with
      `SMY-E445`.
  - A lock is released when it is dropped: `Drop` removes the file
    (`smysl-graph/src/lock.rs:225-228`). A process killed mid-write leaves
    its lock behind by design: "the stale lock *is* the crash notice". The
    truncated tail such a crash leaves "is already tolerated by
    `from_cbor_seq`" (`:20-26`).
  - So a panic that unwinds past a held lock removes it, which a kill does
    not. That erases the notice SMYSL keeps for the case it calls
    interesting (`:21-23`).
  - `Lock::break_lock` exists, but "breaking a lock is an operator's
    decision", and "Nothing in this crate calls it"
    (`smysl-graph/src/lock.rs:188-193`). Its doc comment names a
    `--break-lock` option, but **SMYSL's CLI does not implement one** at
    4f5ecb9.
    - `src/main.rs` uses the lock module only for `acquire_all`
      (`smysl:src/main.rs:8628,12656,12790,12914,13033,13104`).
    - A search of the tree finds `break_lock` only in `lock.rs`, its
      tests and `smysl-graph/tests/redb_crash.rs:276`. `--break-lock`
      appears only in prose, for example `smysl:CHANGELOG.md:4101`.
    - An operator facing a stale lock at 4f5ecb9 can only delete the lock
      files by hand.
  - The files are the library lock `<root>/lock` and one lock per shard,
    `<root>/log/<stem>.lock` (`smysl-text/src/lock.rs:30-41`). Each holds
    its holder's pid, host and command, which `Lock::holder` reads
    (`smysl-graph/src/lock.rs:38-41,182-185`). A `Lock` holds a path, a
    name and a flag, not an open file (`:126-130`).
  - **Not every SMYSL writer takes them.** `acquire_all` is called by
    `with_library`, which `lib migrate`, `verify`, `import`, `accept` and
    `discard` use (`smysl:src/main.rs:8618-8628`), and by five `text`
    subcommands (`:12656,12790,12914,13033,13104`). These commands call
    `append_records` on a library they opened, and never lock:

    | Command | Opens | Appends |
    |---|---|---|
    | `q --library --save` | `smysl:src/main.rs:6297` | `:6324` |
    | `ask --save` | `Corpus::open`, `:13827` | `:6702` |
    | `measure --save` | `:7348` | `:7579` |
    | `assess` | `:7780` | `:8094` |
    | `same-as` | `Corpus::open` | `:14019`, `:14162` |

    SMYSL's own comment assumes otherwise: "every library-wide writer
    holds the library lock first" (`smysl-text/src/library.rs:765-769`).
    Its CLI breaks that assumption. U9 asks SMYSL to lock them.
- **Queries.**
  - `Env` borrows a store and a scope immutably
    (`smysl-query/src/plan.rs:51-53`).
  - `parse`, `plan` and `run` (`smysl-query/src/parse.rs:42`;
    `smysl-query/src/plan.rs:680`; `smysl-query/src/exec.rs:115`).
  - `plan` and `run` fail with a list of diagnostics, and a successful run
    also carries `warnings` (`exec.rs:105-112`).
  - An answer is a scope, a ranking, a count, groups or a pack, and is
    `#[non_exhaustive]` (`exec.rs:70-83`). The units it names are those of
    every shape but `count`, which names none (`exec.rs:86-95`).
  - A ranked hit carries a uid, a score and placements
    (`smysl-query/src/fuse.rs:65-71`).
- **Dossiers.** `ask` classifies a question by a rule table, resolves its
  subject and runs a fixed set of slot queries
  (`smysl-query/src/lib.rs:10-12`; `smysl-query/src/dossier/mod.rs:819`).
  It returns the dossier with the `records` that would save it, and
  writes nothing (`:325-326`). Its options take the clock and the owner as
  inputs, "so a dossier is a function of its inputs", and name a template
  set, which is `SMY-E477` when the build does not carry it (`:118-127`).
  Rule 26 finds other forms of a topic's words by stem (`:318-319`). Answering a dossier calls a model and lives
  in `smysl-ingest` (`smysl-ingest/src/answer.rs:1-3`).
- **Measures** are pure functions of a store, a scope and a recipe. A
  result is a `data` unit of status `derived` (`smysl-measure/src/lib.rs:1-2,26`).
  - `run` returns one result per id requested, each carrying diagnostics,
    and writes nothing (`:239-248`).
  - An unknown id is a result of its own, `SMY-E489` (`:250-253`).
  - A result is grounded on a `Basis`, a saved scope unit and its edges,
    which is "to be appended with the result"
    (`smysl-measure/src/scope.rs:52-60`).
- **The model layer.**
  - `smysl-provider` links `tokio` and, behind features, `ureq`
    (`smysl-provider/Cargo.toml:39-40`).
  - `smysl-ingest` turns it on **by default** (`smysl-ingest/Cargo.toml:20,25`).
- **Heavier optional pieces.**
  - `lingua` for language detection (`smysl-text/Cargo.toml:118`, feature
    `detect`).
  - redb `=2.6.4` (`smysl:Cargo.toml:123`), where Bund2 links redb 4.2.0.

---

## Design — Phase 1

### S1. The build

- **A Cargo feature `smysl` on `bund2-stdlib`, off by default (D28).**
  - `bund2-cli` declares `smysl = ["bund2-stdlib/smysl"]`. It does not
    route the feature through the `bund2` facade, where a feature reaches
    no code (F124, D168), and `bund2-runtime` has nothing to forward.
  - `bund2-stdlib` exports `SMYSL_BUILT_IN`. The CLI's
    `features_built_in` reads it and records the feature, as it reads
    `GROK_BUILT_IN` for `grok`.
  - Without the feature, nothing below is compiled, no `smysl.` word is
    registered, and `cargo tree` names no SMYSL crate.
- **The crates.**
  - `smysl-core`, `smysl-graph`, `smysl-check`, `smysl-query`,
    `smysl-retrieve`, `smysl-pack`, `smysl-text` and `smysl-measure`, each
    with `default-features = false`.
  - `smysl-text` enables `reader-txt` and `reader-md`.
  - `smysl-check`'s own `text` feature is not enabled, so §S4's
    `smysl.check` runs the store's checks only.
- **Where they come from** [R2].
  - Recommended: a git dependency on `https://github.com/vulogov/smysl` at
    `rev = "4f5ecb9"`, moving to crates.io `=1.14.x` once SMYSL publishes
    it (U6).
  - The mechanism is the one `algos` uses: an `allow-git` entry in
    `deny.toml`.
  - D38's reason for `algos` was the oracle, and SMYSL has none, so the
    entry is R2's own ruling. It carries its own comment in `deny.toml`,
    naming D244.
- **Not enabled:**
  - `smysl-ingest` and `smysl-provider` (Phase 2, R1);
  - `detect`, `stem`, `store-redb` and `substrate-redb`;
  - every reader except the two above (R12).
- **A change of revision or version** is a commit of its own, and it
  re-runs §Acceptance's SMYSL criteria.
- **No C compiler** [R18]. This RFC adds a rule of its own: the feature
  builds with no C compiler (criterion 3). D51 does not ask it, because
  D51 governs the default build only.
- **CI** gains five steps in `.github/workflows/ci.yml`:
  - in the `check` job, where suites run under nextest with a time bound
    on each test (F141):
    - `cargo nextest run -p bund2-stdlib --features smysl --profile ci`;
    - `cargo nextest run -p bund2-cli --features smysl --test smysl_demo
      --profile ci`;
  - `cargo clippy -p bund2-stdlib -p bund2-cli --all-targets --features
    smysl`, so that the denied `unwrap`, `expect` and `panic` lints see
    the feature's code;
  - `--features smysl` in the `feature matrix` step's `cargo check`, and
    the same build with `CC` and `CXX` set to `false` (criterion 3);
  - a second `cargo deny` run with `--features smysl` in the `deny` job.
- **Pins that must agree.** In the crates Phase 1 links, SMYSL pins five
  crates exactly:
  - `unicode-segmentation = "=1.13.3"` (`smysl:Cargo.toml:68`);
  - `caseless = "=0.2.2"` (`smysl:Cargo.toml:73`, a `smysl-text`
    dependency at `smysl-text/Cargo.toml:116`);
  - `pulldown-cmark = "=0.13.4"` (`smysl:Cargo.toml:80`), which
    `reader-md` turns on;
  - `libm = "=0.2.16"` (`smysl-measure/Cargo.toml:42`);
  - `brotli = "=8.0.4"` (`smysl-measure/Cargo.toml:54`), which is pure
    Rust.

  `rand_chacha` is `"0.9"`, which is not an exact pin
  (`smysl-measure/Cargo.toml:45`).

  Bund2's `Cargo.lock` has `unicode-segmentation` 1.13.3 and `libm`
  0.2.16 today, and none of `caseless`, `pulldown-cmark` or `brotli`. A Bund2 bump of the first
  two, or a later Bund2 dependency on the last two, must keep the feature
  building, which the `feature matrix` step checks.

### S2. Values

**A handle is a MAP** [R3]:

    { type: "smysl", root: <STRING, a canonical path>, agent: <STRING> }

- **It names a library on disk and holds nothing open.**
  - Phase 1 handles libraries only. A bare store takes no lock (`append`
    at `smysl-graph/src/store/mod.rs:414` takes none).
  - A document in the surface syntax is brought into a library with
    `smysl.parse` and `smysl.append`.
- **Its `root` is canonicalised** (`std::fs::canonicalize`) when
  `smysl.create` or `smysl.open` builds the handle.
  - `smysl.create` creates the library first and canonicalises after,
    because a path that does not exist yet has no canonical form.
  - **Every word canonicalises the root it is given again**, before it
    takes §S9's mutex, so a hand-built handle with another spelling of the
    same directory still finds the same mutex.
  - A handle saved by `save.model` names the same library from any working
    directory.
- **A handle is plain data.** It may be saved, sent over the bus, or
  copied between VMs (RFC-0007), and it names the same library wherever it
  goes.
- **Its `agent`** is the SMYSL agent id Bund2 attests as [R15]. It defaults
  to `"tool:bund2"`.
- **The agent is self-asserted** [R21], as every SMYSL agent id is until
  SMYSL signs attestations (`smysl-core/src/ids.rs:523-524`).
  - A program may name any agent the grammar allows, through
    `smysl.agent` or a hand-built handle.
  - So `Origin` authority (§S4) is advisory inside Bund2, as it is for
    SMYSL's own CLI. It keeps an honest program from retracting what it
    never vouched for, and it stops nothing that chooses to claim another
    agent's id.
  - Recommended: say so and restrict nothing. The alternative is to
    restrict `smysl.agent` to the `tool:` kind.
- **Every word validates the handle it is given.** That covers `type`, a
  STRING `root`, and an `agent` matching SMYSL's grammar
  (`smysl-core/src/ids.rs:521`). A hand-built handle with a malformed
  `agent` is SM00.

**Records read back as MAPs:**

| SMYSL | Bund value |
|---|---|
| a uid | STRING, its full text form `b3:…` |
| a status | STRING, `"unfounded"` … `"measured"` |
| a unit | MAP: `uid`, `schema` and `gist`, and, where present, `body`, `detail`, `status`, `grounds` (LIST of uid), `deps` (LIST of uid), `source`, `lang`, `labels` (LIST of STRING), `payload` (the decoded tree) and `opaque` (BOOL, see below) |
| a source | MAP: `kind`, `reference`, and, where present, `captured` (STRING, the date as SMYSL writes it), `observed` (INT of milliseconds; a value above `i64::MAX` reads back as a STRING of its decimal digits, and is written from one, so the two forms round-trip; any other STRING is SM03), `published`, `span` (LIST of two INTs) and `manifest` |
| a relation | MAP: `rid`, `kind`, `from` and `to`, and, where present, `weight` (FLOAT), `note` (STRING: the uid of a unit carrying commentary on the edge, `smysl-core/src/types/relation.rs:182-183`) and `opaque` (BOOL, see below) |
| a diagnostic | MAP: `code` (SMYSL's own, such as `"SMY-E030"`), `severity`, `subject` and `message`, and, where present, `suggestion` |
| a subject | MAP `{ kind, value }`: `"unit"` with a uid; `"record"` with the hash in hex; `"span"` with LIST `[ start end ]` of byte offsets; or `"store"` with NODATA |
| a payload tree | `Null` → NODATA; `Bool`, `Int`, `Float` and `Str` as themselves; `Array` → LIST; `Object` → MAP |

- **A unit read back is a view, not the record.**
  - It leaves out the unit's `extra`, its source's `extra`
    (`smysl-core/src/types/epistemics.rs:403`), any payload that does not
    decode, and its attestations.
  - When any of the first three applies, the MAP has `opaque = true`, and
    `payload` is absent if it did not decode.
- **A relation read back is a view too.** Its `extra`
  (`smysl-core/src/types/relation.rs:185`) and its attestations are left
  out, and a non-empty `extra` sets `opaque = true`.
- **A source of a kind this build cannot name** reads back with `kind` as
  an INT: the raw code SMYSL keeps on `SourceRef`, and reports as
  `SMY-W409` (`smysl-core/src/types/epistemics.rs:180-191,432-439`). The
  unit's `opaque` is set. A source Bund2 writes names a kind SMYSL knows;
  any other is SM05.
- **A relation kind** reads back as its text. That covers the fourteen
  kernel kinds and SMYSL's extension kinds, `x.<domain>/<kind>`
  (`smysl-core/src/types/relation.rs:34,103-125`). Whether `smysl.rel` and
  `smysl.append` accept an extension kind is R24. Recommended: they accept
  any kind `RelKind::parse` accepts, and any other is SM05.
- **Copying records between libraries is not in Phase 1** (§S10).
- **Units are written from MAPs of the same shape, without `uid`**, which
  SMYSL computes.
  - A MAP carrying `uid`, `rid` or `opaque` is refused (SM03, "computed,
    not given"), so a view is never re-appended as if it were the record.
  - A `source` with `span` must name its `manifest`, and is built by
    `SourceRef::in_part`. An empty or reversed span is SM03.
  - `payload`, when given, must be a MAP, since a payload is an object.
    Any other value there is SM03.
  - SMYSL NFC-normalises the gist, body and detail, and drops an empty
    body or detail. So a unit read back may differ from the MAP written in
    those three fields. A MAP with `body: ""` and a `detail` is SMYSL's
    "detail without a body", which is SM02.
- **Labels in `grounds` or `deps`** are resolved to uids before the unit is
  built. A label bound to several units is SM08
  (`smysl-graph/src/labels.rs:3-8`).
- **A Bund value with no SMYSL form** (BIN, MATRIX, LAMBDA, OBJECT, any
  other non-tree value) is SM03, naming the key.
- **Floats in a weight or a payload** are quantised by SMYSL, and a
  weight is first clamped to [0, 1]. A program reads back the clamped,
  quantised value.
- **The variants data can reach have arms.** A later SMYSL can write three
  things this build cannot name, and each is handled above or in §S7: an
  unknown source kind, an extension relation kind, and an unknown profile
  estimator (`smysl-core/src/types/estimate.rs:228-230`).
- **A SMYSL value Bund2 has no arm for** is a variant added to a
  `#[non_exhaustive]` enum after 4f5ecb9. Only a change of the pin can
  bring one, and the code does not compile it in; Bund2 answers
  `Error::internal`, naming the type (D37). A pin moves in a commit of its
  own that re-runs the criteria (§S1), and that commit adds the arm.

### S3. Opening

- **Each word opens the library its handle names, and closes it before
  returning**, as `save.model` opens the world file
  (`reference/Bund/src/stdlib/functions/bund/bund_models.rs:105,115`).
- **There is no cache** [R4]. A cache that outlives a call would need one
  of two things, and neither exists:
  - a per-VM slot, which `bund2-api` lacks, or a process-wide table;
  - a way to tell that the cache is stale without reading the library.
    The library's stamp is computed over the logs as loaded
    (`smysl-text/src/library.rs:993-995`).
- **A cache is an amendment** if Q54's measurement asks for one.

### S4. Words — forming

Every word is a native with a fixed effect unless it is marked opaque. "W"
marks the words that write; every write is made under §S9's locks.

| Word | Operands → result | W | Does |
|---|---|---|---|
| `smysl.create` | root → handle | ✓ | creates a library at `root`, or opens the one already there, as `Library::create` does |
| `smysl.open` | root → handle | | opens the library at `root`; SM01 if there is none |
| `smysl.agent` | handle agent → handle | | the handle with another `agent`; SM03 if the agent does not match SMYSL's grammar |
| `smysl.text.add` | handle spec → MAP | ✓ | adds a text through a reader. Answers `{ mid, parts }`, the manifest's id and its parts' ids. `spec` is a MAP, given below the table |
| `smysl.texts` | handle → MAP | | each alias with its heads, a LIST of manifest ids (`catalog`) |
| `smysl.unit` | handle unit → uid | ✓ | appends one unit written as §S2's MAP |
| `smysl.rel` | handle relation → rid | ✓ | appends one relation. A `retracts` relation is SM03, naming `smysl.retract` [R23] |
| `smysl.append` | handle list → LIST | ✓ | appends a LIST of unit, relation and label MAPs, in order and under one acquisition of the locks. Answers each one's id: a unit's uid, a relation's rid, and for a label, the uid it binds. A `retracts` relation anywhere in the LIST is SM03, and nothing is written [R23] |
| `smysl.label` | handle label uid → | ✓ | binds a label |
| `smysl.retract` | handle uid → rid | ✓ | retracts a unit as SMYSL does (below); NODATA if it was already retracted |
| `smysl.parse` | text → LIST | | parses the surface syntax into §S2's MAPs of units, relations and labels, without their ids. Any other record (a view, a schema declaration, a thread, a manifest, a commitment, …) has no §S2 form. It is skipped, and one `Warning` per record kind names how many were skipped. Nothing is written |
| `smysl.write` | handle → text | | the surface syntax of every unit, relation and label the library holds |
| `smysl.check` | handle → LIST | | every diagnostic of `check`'s store passes, as MAPs, under `CheckOptions::default()` [R19]. The profile is therefore the one SMYSL derives from the library's views: the `default` preset with none, the view's own with one, the widest envelope with several |

**`smysl.text.add`'s spec.** Every field of SMYSL's `AddSpec` is a key of
the spec or has a default named here:

| Key | Required | Absent means |
|---|---|---|
| `path` or `text` | one of the two | — |
| `reader` | yes: `"txt/1"` or `"md/1"`; any other is SM05 | — |
| `alias` | yes | — |
| `licence` | yes: SMYSL requires one (`smysl-text/src/library.rs:95`); missing is SM03 | — |
| `lang` | yes: a language tag. SMYSL refuses an add when neither the caller nor the reader names a language (`smysl-text/src/library.rs:1193-1203`), and neither enabled reader names one: `ReadOutput.lang` starts `None`, and only `with_lang` sets it (`smysl-text/src/readers/mod.rs:253,268-269`). Missing is SM03 | — |
| `title` | no | none |
| `carry` | no: `"none"`, `"ref"` or `"text"` | `"none"`, SMYSL's `Carry` default |

- `params` is SMYSL's empty `Params`; the part policy, the pseudonym
  key and the versification scheme are absent. None of the four is
  offered in Phase 1. An absent policy is SMYSL's default: the reader's
  top level as the part boundary (`library.rs:1176-1179`).
- `Caps` is `Caps::default()`.
- **A `path` is read by Bund2,** since a reader takes bytes. Bund2 checks
  the file's length against `Caps::default().input_bytes` first, and
  refuses a larger file with SM02, carrying `SMY-E440`, before reading
  it. A `text` is checked the same way.

**Attestation** [R15]. Every unit and relation Bund2 writes is followed by
one attestation, written as its own `Record::Attestation`. The retraction
edge is the exception (below). The attestation has these fields:

- its agent is the handle's `agent`;
- its operation is `Authored`. SMYSL attests its own computed units
  `Transformed` (`smysl:src/main.rs:7566-7575`), from the unit they were
  computed from. A unit Bund2 writes was authored by the program that
  wrote it, and rule T reads only the rung
  (`smysl-check/src/passes/trust.rs:33-38`), so the choice changes no
  status;
- its rung is `Document` for a `cited` unit, and `Computed` for every
  other;
- its hop is 0;
- its parents are empty, except for a unit `smysl.prove` writes, whose
  parents are its grounds. So `smysl.trace … "parents"` walks a proof's
  conclusions back to their premises.
  - Salience reads the same parents. Bund2's attestation of a conclusion
    therefore counts as corroboration independent of another agent's
    only when their ancestries are disjoint, which is the intended
    reading, since a proof rests on its premises.
- with hop 0, every unit Bund2 writes sits at hop 0 in `hop_diff`'s
  partition.
- **Its clock** is `Hlc::now(prev, agent)`.
  - `prev` is the newest clock among this agent's attestations in the
    library as re-opened under the locks (§S9), or `Hlc::zero(agent)` if
    there is none.
  - Within one word, each attestation advances from the one before it.
  - So an agent's clocks in a library never go backwards, even when the
    wall clock does.

So rule T caps what a program can author at `derived`, unless the unit is
cited (`smysl-core/src/types/provenance.rs:218-225`). A `measured` unit is
refused with SM03, since only an import may assign that status
(`smysl-core/src/types/epistemics.rs:325-327`).

**Re-writing a unit SMYSL already holds.**

- The unit record itself is a duplicate, and SMYSL's `append` skips it
  (`smysl-graph/src/store/mod.rs:434-446`).
- Bund2 writes the attestation only if **this agent** has not attested
  that uid already. So a program that re-proves or re-appends adds
  nothing to the log.
- A unit another agent wrote first gains this agent's attestation, as
  SMYSL's "a rerun adds an attestation" intends
  (`smysl-measure/src/lib.rs:28-29`).
- `Origin` authority then lets this agent retract a unit it has also
  vouched for. That authority is advisory (§S2, R21).

The word answers the unit's uid either way.

**Retraction** follows SMYSL's own `retract` command
(`smysl:src/main.rs:4484-4505`), step for step:

1. If the effective status under the strict policy already reads the
   target as retracted, nothing is written and the word answers NODATA.
   As in SMYSL, this is checked before authority.
2. Plan it with `plan_retraction`. The agents are the handle's `agent`;
   the policy is `Strict` and the authority is `Origin`
   (`smysl-graph/src/merge/retraction.rs:181-191`).
3. If the plan is refused, the word refuses with SM09, which carries
   SMYSL's reason. With `Origin`, a unit no attestation of this agent
   vouches for cannot be retracted.
4. Otherwise, append the self-edge `retracts` from the target to itself,
   **unattested**, as SMYSL writes it (`:218`; `smysl:src/main.rs:4499`).
   The word answers its rid.

The edge is unattested because that is the record `plan_retraction`
simulates when it decides whether to accept
(`smysl-graph/src/merge/retraction.rs:218`), and the record SMYSL's
`retract` writes to a bare store (`smysl:src/main.rs:4499`). That
command cannot write to a library, since it reads its `--store` argument
as a file (`:4537`). So no CLI retraction in a library exists to match.
Attesting the edge would be an amendment to R15.

**`retracts` is written only by `smysl.retract`** [R23].
- `effective_status` retracts the `to` of **any** `retracts` relation
  whose target is present, checking neither authority nor `from`
  (`smysl-graph/src/merge/retraction.rs:83-88`). Only `plan_retraction`
  checks authority (`:181-209`).
- So a `retracts` relation appended by `smysl.rel` or `smysl.append`
  would retract any unit, skip the "already retracted" test, and not even
  need to claim another agent's id. R21's advisory authority is a
  narrower hole than that.
- Recommended (R23 A): both words refuse it with SM03, naming
  `smysl.retract`. The alternative (R23 B) accepts it, and states the
  bypass beside R21 with a Preservation row.
- Under A, a library holding retractions cannot be copied through
  `smysl.write`, `smysl.parse` and `smysl.append`. Copying between
  libraries is not in Phase 1 (§S10).

**A clean library.**

- `smysl.check` answers every diagnostic, warnings included.
- A library is **clean** when no diagnostic has severity `error`.
- Warnings, such as `SMY-W041` for a body under the profile's lower bound,
  are information. Which warnings appear depends on the library's profile.
  So does the bound on gists, since a view may set its own `l0_max` and
  estimator. §S7 fits exported gists to it.

**Refusals and warnings.**

- A unit SMYSL refuses to build is SM02, carrying SMYSL's reason.
- A warning SMYSL raises while writing is reported through `Vm::report` as
  a `Warning`, one line on stderr (D36).

### S5. Words — researching

None of these appends a record. Each opens the library, and an open
unlinks the objects of redacted parts, as every SMYSL open does
(§Current, "The library").

| Word | Operands → result | Does |
|---|---|---|
| `smysl.get` | handle uid → unit | the unit, as §S2's MAP; NODATA if absent |
| `smysl.trace` | handle uid kind depth → LIST | the lineage. `kind` is one of SMYSL's four or Bund2's `"dependents"`. `"parents"`, `"grounds"` (which follows `grounds` and `deps`) and `"both"` go through `trace` (`smysl-graph/src/lineage.rs:29-55,149`). `"rests-on"` goes through `trace_via` over `EdgeSet::premises` (`:222-233`). `"dependents"` goes through `dependents_trace` over the same edges (`:389-394`), which are those `"grounds"` follows plus `conditions`. `depth` is an INT, or NODATA for unbounded. Any other `kind` is SM05 |
| `smysl.status` | handle → MAP | effective status after retraction under the strict policy: uid → status, for every unit whose status changed |
| `smysl.salience` | handle spec → LIST | MAPs `{ uid, score }`, highest first (`salience`); `spec` may name a `focus` |
| `smysl.pack` | handle spec → LIST | the uids `pack` selects to `spec.budget` tokens (`smysl-pack/src/solve.rs:242`) |
| `smysl.dossier` | handle question → MAP | the dossier `ask` assembles: `subject`, `entities` (LIST of uid) and `slots` (LIST of MAP `{ name, units }`). The options give the handle's `agent` as owner, and a clock from the wall clock |
| `smysl.measure` | handle ids scope → LIST | runs the named measures over the scope an `sq/1` text names. Each result is a MAP `{ id, ok, value, uid, diagnostics }`. When the measure ran, `ok` is true, `value` is the headline as SMYSL renders it in the result unit's gist (a STRING; `smysl-measure/src/lib.rs:26-28`), and `uid` is the id that unit would have. When it did not run, `ok` is false and `value` is SMYSL's reason, with its code among `diagnostics`; an unknown id is such a result, `SMY-E489` |

**Nothing is saved** [R17].

- A dossier's `records` and a measure's unit are not appended.
- A measure's unit grounds on a `Basis` that must be appended with it
  (`smysl-measure/src/scope.rs:52-60`), so keeping a measure is not a
  matter of appending one unit. Saving either is an amendment.

**The environment** of a dossier, a measure and a query (§S6) is the one
SMYSL's `q --library` and `measure` build:
- the scope is `LibraryScope::new(name, &library)`, where `name` is the
  root's file stem, as `measure` names it (`smysl:src/main.rs:7355-7362`);
- the `Env` is `Env::new(library.store(), &scope)` with `with_times` and
  `with_entities` over that scope (`:6313-6315`). `ask` fails without the
  entities port (`smysl-query/src/dossier/mod.rs:829-830`), and the time
  stages are refused without the times port
  (`smysl-query/src/plan.rs:54-57`);
- no semantic port is bound, since Phase 1 has no model. `~sem`,
  `near(…)` and `lang:any` are then refused by name
  (`smysl-query/src/plan.rs:63-66`), which reaches the program as SM04;
- the `PlanOptions` are `PlanOptions::default()`: 10 hits per partition,
  current versions only, no parameters (`smysl-query/src/plan.rs:161-175`);
- a measure's base `Recipe` is `Recipe::new("", 0, scope)`, every other
  field at its default (`smysl-measure/src/recipe.rs:191-213`). That is
  where `measure` starts before applying its flags
  (`smysl:src/main.rs:7448`), and its seed, 0, is `measure`'s default
  (`:1149-1153`).

### S6. Words — querying

| Word | Operands → result | Does |
|---|---|---|
| `smysl.query` | handle text → MAP | parses, plans and runs one `sq/1` query |
| `smysl` | → c | a conditional, `Conditional("smysl")`, with slots `handle`, `query` and `lambda`. It runs the lambda once per unit of the answer, with the unit's MAP on top |

**An answer is a MAP** with `kind` and the fields that kind has:

| `kind` | Fields |
|---|---|
| `"scope"` | `units`: LIST of uid |
| `"ranked"` | `hits`: LIST of MAP `{ uid, score }` |
| `"count"` | `count`: INT |
| `"groups"` | `groups`: LIST of MAP `{ key, units }` |
| `"packed"` | `units`: LIST of uid |

**The conditional visits the units `Answer::uids` names**
(`smysl-query/src/exec.rs:86-95`).

- A `count` answer names none, so the lambda does not run.
- A `ranked` answer's scores, and a `groups` answer's keys and boundaries,
  are in `smysl.query`'s MAP but not in what the conditional sees.

**Queries are SMYSL's text** [R6].

- A query that `parse` or `plan` refuses is SM04, which carries every
  diagnostic SMYSL gave, so `SMY-E461` reaches the program as SMYSL wrote
  it.
- A run's `warnings` are each reported as a `Warning` (D36).

**The runner's own refusals:**

- a missing slot, or a slot of the wrong type, is SM03, naming the slot.
  A missing `lambda` is SM03 too, where RFC-0013 §S11 reads a missing
  `lambda` slot as an empty lambda: a `smysl` conditional with nothing to
  run is taken as a mistake;
- an error raised inside `lambda` is SM10, as RFC-0013's E25 is for
  `query`.

The runner runs the lambda through `eval_lambda`, as RFC-0013's `query`
runner does (§S11), and its function is added to `REENTERING`.

### S7. The bridge to logic programming

**`smysl.kb`** (handle scope → kb) loads the units an `sq/1` scope selects
as the facts of a new knowledge base [R7].

- The KB is built through the same validation `kb!` uses.
- Uids, schemas and statuses are STRING atoms (RFC-0013 §S1).
- Statuses are the declared ones; effective statuses appear only as
  `effective` facts.

| Fact | Holds | In a proof, stands for |
|---|---|---|
| `[ unit U Schema Status ]` | one per unit | `U` |
| `[ gist U Text ]` | its gist | `U` |
| `[ grounds U G ]` | one per ground | `U` |
| `[ deps U D ]` | one per dependency | `U` |
| `[ label L U ]` | one per label | `U` |
| `[ effective U Status ]` | one per unit whose status retraction changed (§S5) | `U` |
| `[ rel Kind From To ]` | one per relation with both ends in scope | `From` and `To` |
| `[ rank Status N ]` | six facts, `unfounded` 0 … `measured` 5, so a rule can compare statuses | nothing: a constant of the scale |

A rule can ask, for example, which findings rest on a rebutted claim:

    [ [ shaky F ] ← [ unit F "finding" _ ] [ grounds F C ]
                    [ rel "rebuts" _ C ] ]

**`smysl.prove`** (handle kb goal opts → LIST) is **`opaque(4)`** [R8].

- **It re-enters evaluation.** It runs RFC-0013's engine with proofs on,
  and the engine may run bridges, as `kb.prove` does (RFC-0013 §S9).
- **It has no `ask` slot,** as no asking word but `query` has. An askable
  goal it reaches is E50 (RFC-0013 §S16), so its proofs hold no `:asked`
  or `:known` node.
- **It is never promoted,** and `bund2 check` gets an arm for it: 4 →
  LIST.
- **Its refusals are `kb.prove`'s codes,** with its own prefix
  `SMYSL.PROVE: `. E00 and E40 keep their house texts, unprefixed, as
  RFC-0013 §S12 leaves them.
- **It solves first and writes after.**
  - It finds every answer and its proof, holding no lock.
  - Only then does it take §S9's locks and write.
  - So a bridge that writes to the same library while the proof runs
    neither deadlocks nor meets SM07.
- **It answers the uid of each answer's root.**
- **The `REENTERING` audit.** Its re-entry goes through the engine's
  bridge (`kb/bridge.rs: run`), which the audit already names. The `smysl`
  conditional's runner calls `eval_lambda` directly (§S6), so its function
  is added to `REENTERING` as `"<file>: <fn>"`.

**What a fact stands for is decided by its shape and the library, not by
where the fact came from.**

- A fact node whose goal has one of the first seven shapes in the table
  above, and whose uid arguments all name units in the handle's library,
  stands for those units. Nothing is written for it.
  - This is judged **at write time**, against the library as re-opened
    under the locks (§S9). A unit a bridge wrote while the proof ran
    therefore counts.
- A `[ rank Status N ]` fact with one of the six statuses and its number
  stands for nothing.
- Every other fact node is a **program fact**. That includes a fact of one
  of these shapes whose uid is not in the library, wherever it came from.

**Each node of RFC-0013 §S15 becomes:**

| Node | Becomes |
|---|---|
| a fact standing for units | those units, as grounds |
| a fact standing for nothing (`rank`) | no ground |
| a program fact | a `claim` of the status `opts.facts` names: `"speculative"` (the default), or `"cited"` with `opts.source`. `derived`, `inferred` and `measured` are SM03, since the first two need grounds and the last is never authored. Any other value is SM05 |
| a rule `[ g ← p₁ … pₖ ]`, and `once`, which has a rule's shape | a `claim` (a `finding` at the root) whose grounds are what its premises became |
| `call`, `:branch i p` | transparent: what the inner proof became |
| `:then pC pT` | as a rule with premises `pC` and `pT` |
| `:else pE` | as a rule with premise `pE`, **capped at `inferred`**: the condition's failure is negation as failure |
| `:findall p₁ … pₙ` | as a rule with premises `p₁ … pₙ`; with none, it has no grounds |
| `:no-proof` | no ground; **caps the conclusion at `inferred`** |
| `:builtin` | no ground |
| `:bridge λ′` | no ground |
| `:tabled` | no ground; **caps the conclusion at `inferred`**, since a table answer keeps no proof |
| `:asked`, `:known` | cannot occur, since `smysl.prove` has no `ask` slot. Should a later amendment give it one, the row would be a `speculative` claim stating the reply |

**A written conclusion's status.** Let `G` be its grounds.

- If `G` is empty, the conclusion is `speculative`. It cannot be `derived`
  or `inferred`, since SMYSL refuses those without grounds.
- Otherwise its status is the lower of `derived` and the weakest
  **declared** status in `G`. Rule M uses declared statuses too
  (`smysl-check/src/passes/epistemics.rs:23-66`).
- That status is lowered to `inferred` if a capping node lies under the
  conclusion.

So rule M holds by construction, and `smysl.check` reports no `SMY-E030`
for what was written.

**The text of a written unit** keeps the gist within the bound
`smysl.check` will apply to the library, and carries the full goal in the
unbounded detail.

**The profiles a gist must fit** are read from the library as re-opened
under the locks (§S9), at write time:
- every distinct granularity profile among the library's views;
- the `default` preset if the library has no view.

That set includes every profile SMYSL's `check` can derive (§Current,
"Granularity"). A gist that fits each of them also fits their widest
envelope, which counts with the first view's estimator against the
largest `l0_max`.

- **gist:** §S7.1's rendering of the goal, with every uid shortened to its
  first 12 characters, **normalised to NFC first**.
  - A uid is a STRING atom that `Uid::parse` accepts
    (`smysl-core/src/ids.rs:71`). Any other atom, `"b3:x"` for one, is
    rendered whole.
  - Bund2 normalises it with `unicode-normalization`, the crate SMYSL
    normalises with, and depends on it directly under the feature. So
    SMYSL's own normalisation leaves the fitted gist unchanged, and
    cannot lengthen it past the bound. This assumes that a prefix of an
    NFC string, followed by `…`, is still NFC; criterion 12's non-NFC
    atom near the cut checks it.
  - The detail is normalised the same way.
  - If the rendering fits every profile in the set, it is the gist.
  - Otherwise, the cut is the shortest of the prefixes that
    `fit_gist_with(rendering, "…")` gives for each profile
    (`smysl-core/src/types/view.rs:204-212`). The gist is that prefix
    followed by `…`. SMYSL counts in its own estimators, so the cut needs
    no byte arithmetic in Bund2.
  - A profile whose estimator this build does not have answers `None`.
    `check` cannot evaluate that bound either (`SMY-W025`), so the profile
    is left out of the set. If every profile is left out, the gist is cut
    by the `default` preset, and one `Warning` says the library's bound
    could not be evaluated.
  - If a profile leaves room for no character of the rendering, or not
    even for the `…` alone, the write is SM12: `the library's view allows no gist of <n> tokens`. Nothing is
    written.
- **body:** `proved by <functor>/<n> from <k> premises`, or `program fact`,
  or `reply`;
- **detail:** §S7.1's full rendering of the goal, then one line per
  premise with that premise's full rendering.

The detail is part of the unit's identity, so two goals whose gists
coincide are still two units. A body under 40 tokens draws the warning
`SMY-W041`, which is not an error.

**Identity does the rest.** A sub-proof that two answers share renders the
same and is one unit. Proving the same goal twice writes no new unit the
second time (§S4, units already present).

#### S7.1. Rendering a value

The goal arrives as a value read back by RFC-0013 §S1. The rendering is
defined here, not by any printer, so a uid built from it does not change
when a printer does.

| Value | Text |
|---|---|
| CALL (a functor) | its name |
| STRING (an atom) | its text, if it matches `[a-z][A-Za-z0-9_.]*`. Otherwise it is in double quotes: `"` and `\` are escaped by `\`, a newline is `\n`, a tab is `\t`, and any other control character is `\u{…}`, so a rendering is always one line |
| BOOL | `true` or `false` |
| INT | decimal |
| FLOAT | the shortest decimal that reads back as the same `f64`, with `.0` if integral; `nan`, `inf` and `-inf` for the non-finite values |
| NODATA (an unbound variable) | `_` |
| LIST (a compound or a list) | `[ ` then each item rendered, separated by single spaces, then ` ]`; the empty LIST is `[ ]` |
| a `\|` CALL inside a LIST (a partial or improper tail) | `\|`, then the next item rendered, so a partial list is `[ a b \| _ ]` |
| any other value (opaque) | SM03: it has no stable text |

### S8. Refusals

Each text but SM06's and SM11's follows `<W>: ` (RFC-0013 §S12). Those
two are the house texts, unprefixed, as the reference writes them.

| Code | When | Text after `<W>: ` |
|---|---|---|
| SM00 | an operand that should be a handle is not a MAP of `type = "smysl"` with a STRING `root` and a well-formed `agent` | `#<k> parameter must be a SMYSL handle` |
| SM01 | `smysl.open`, or a word given a handle, finds no library at `root` | `no SMYSL library at <root>` |
| SM02 | SMYSL refused the operation, or a SMYSL call failed with an I/O error: among them a lock that could not be linked (§S9), and a primary shard this build cannot read (§Current, "The library") | `SMYSL refused it: <code> <message>` |
| SM03 | an operand of the wrong type (`#<k> parameter must be <kind>, found <type>`); a spec, record or slot lacking a required key or holding one of the wrong type; a value with no SMYSL form; a computed key given; a status Bund2 cannot author; a `smysl.text.add` spec with no `lang`; a `retracts` relation outside `smysl.retract` [R23] | `<what> <reason>`, e.g. `uid is computed, not given` |
| SM04 | `sq/1` refused the query | `the query was refused: <code> <message>`, one per diagnostic, joined by `; ` |
| SM05 | an unknown status, relation kind, source kind, reader or trace kind | `unknown <kind>: <value>` |
| SM06 | `--noio`, for a word that acts on disk [R11] | unprefixed, in the pattern of the house text `bund SAVE.MODEL functions disabled with --noio` (`bund_models.rs:125`): `bund SMYSL functions disabled with --noio` |
| SM07 | a lock is held (`SMY-E445`); or the shard set changed twice while locking (§S9) | `the library is locked: <holder as SMYSL names it>`, or `the library's shards changed while locking: <shard>` |
| SM08 | a label bound to several units | `label <label> binds <n> units` |
| SM09 | `plan_retraction` refused the retraction | `retraction refused: <SMYSL's reason>` |
| SM10 | the `smysl` conditional's `lambda` failed | `lambda returns: <error>` |
| SM11 | the stack holds fewer operands than the word takes | the house text, unprefixed, as `reference/Bund/src/stdlib/functions/statistics/get_data.rs:12` writes it: `Stack is too shallow for inline <w>` |
| SM12 | a profile of the library's views leaves no room for an exported gist (§S7) | `the library's view allows no gist of <n> tokens` |

- **An unknown measure id is not a refusal.** It is a result, as SMYSL
  reports it (§S5).
- **A panic inside SMYSL is not a refusal.** D49 returns it as
  `Error::internal`, and §S9 says what it leaves behind.
- **Warnings** from appends, from `run`, and from a measure's diagnostics
  are each reported as a `Warning` (D36). `smysl.parse` routes SMYSL's
  `SMY-W054`, a label declared twice in one document
  (`smysl-graph/src/labels.rs:10-12`), the same way. A measure's diagnostics are also
  kept in its result MAP.

### S9. Effects, locks and threads

**Effects.**

- Every word but `smysl` and `smysl.prove` has a fixed effect and does not
  re-enter evaluation. `bund2 check` reads each one's declared effect.
- `smysl` is a conditional that re-enters, as `query` does.
  - `bund2 check`'s per-row conditional arm becomes `"csv" | "sqlite" |
    "query" | "smysl"`.
  - The runner's function is added to `REENTERING` (§S7).
- `smysl.prove` is `opaque(4)` (§S7).

**`.` forms** [R16]. There are none in Phase 1.

- D18 gives `.` forms to the words D14 preserves, and these words are
  Bund2's own.
- RFC-0013 gave forms to the `kb.` words that transform a KB value held on
  a stack. Every `smysl.` word acts on a library on disk named by a
  handle, so a workbench form would move only the name.

**Locks.** Every write takes SMYSL's locks in the order SMYSL's locking
commands use (`smysl:src/main.rs:8618-8628`), then **re-opens** the
library:

1. open the library;
2. read its shard names (`Library::shards`);
3. take the library lock, then the shard locks in name order, through
   `acquire_all` (`smysl-text/src/lock.rs:24-43`);
4. **open the library a second time, and use only this second open from
   here on;**
5. if the second open names a shard the locks do not cover, drop the
   locks and start again from step 1. A second mismatch is SM07, naming
   the shard;
6. validate, decide and write;
7. drop the locks before the word returns.

**Why step 4.** `Library::open` loads every log into memory, and nothing
reloads it (`smysl-text/src/library.rs:327-350`). Without step 4, every
decision a write makes would rest on the library as it was before the
locks, not as it is under them, because another process may append in
between. That covers:
- the gist profiles (§S7);
- whether a fact stands for library units (§S7);
- the attestation clock's `prev`, and "already attested" (§S4);
- retraction's "already retracted" and its plan (§S4);
- `encode_by_shard`'s de-duplication, which checks the in-memory store
  (`smysl-text/src/shards.rs:520`).

SMYSL's own commands do not re-open. Bund2 does, at the cost of a second
open per write (Q54).

**The shard set is stable against writers that lock.** `acquire_all`
takes the library lock first (`smysl-text/src/lock.rs:30`), so while
Bund2 holds it no writer that locks can add a shard. A mismatch at step 5
is a shard such a writer added between steps 1 and 3, which a restart
covers. A shard Bund2's own batch creates is covered by the library lock,
not by a shard lock, as SMYSL intends
(`smysl-text/src/library.rs:765-769`).

**What the locks do not cover** [R22]. Five SMYSL commands append without
locking (§Current, "The locks"): `q --save`, `ask --save`,
`measure --save`, `assess` and `same-as`.
- **Bund2 promises serialisation only against writers that lock:**
  itself, in this process and in others, and SMYSL's `lib` and `text`
  commands.
- **Against the five it promises nothing, and cannot.** They write at any
  moment, including between Bund2's second open and its append. Then:
  - Bund2's de-duplication, its "already attested" and "already
    retracted" tests, and its clock's `prev`, may rest on a library that
    has since moved on;
  - worse, if Bund2 opens while one of them is part way through a record,
    it reads that record as a torn tail, and its append cuts it off
    (`smysl-text/src/shards.rs:236-251,376-379`). The other process then
    writes the rest of its record at the new end. Neither side reports
    anything.
- A locking SMYSL command opening beside one of the five can do the same,
  so the hazard is SMYSL's. Bund2 makes it neither better nor worse, and
  the remedy is SMYSL's: U9 asks it to lock those commands.
- **Recommended (R22 A):** Phase 1 proceeds, and its documentation says a
  library a Bund program writes must not be written by those five
  commands at the same time. The alternative (R22 B) is that Phase 1 waits
  for U9, and pins a SMYSL revision that has it.
- Q57 is re-opened with the hazard named.

**Filesystems.** A lock is a hard link (§Current, "The locks"), so a
library must sit on a filesystem that has them. Elsewhere every write
fails with an I/O error from taking the lock, which is SM02, not SM07.

- **A lock held by another process is SM07.** It is not waited for. SMYSL
  fails fast, and a program that wants to retry can catch SM07 with
  `?try`.
- **A stale lock stays SM07** until an operator breaks it [R14].
  - SMYSL says "breaking a lock is an operator's decision"
    (`smysl-graph/src/lock.rs:188-193`). It has no command for it at
    4f5ecb9 (§Current, "The locks").
  - **Bund2 gives the operator one:** a CLI subcommand,
    `bund2 smysl-unlock <root>`, in the `smysl` build only. It is
    dispatched as `bund2 words` is, in `crates/bund2-cli/src/main.rs`.
    - It lists `<root>/lock` and every `<root>/log/*.lock`, and prints
      each one's holder through `Lock::holder`.
    - With `--yes`, it breaks each lock with `Lock::break_lock`: the shard
      locks first, then the library lock.
    - Without `--yes`, it only lists them.
  - It is a subcommand and not a word, so a Bund program cannot break a
    lock. Breaking stays an operator's act, as SMYSL asks.
  - SMYSL is asked for its own command as U8. If it ships one, the
    subcommand may be withdrawn by an amendment.
- **A panic inside a write leaves SMYSL's locks in place** [R20], as a
  killed SMYSL process would. That keeps SMYSL's crash notice.
  - **So does a failed call after which a log has grown** [R25, revised
    by revision 8].
    - Bund2 reads the library's stamp (`Library::stamp`,
      `smysl-text/src/library.rs:993-995`) before it calls
      `append_records` or `Library::add`, and again if the call fails.
    - A shard log's length grows only once a write and its `sync_data`
      have succeeded (`smysl-text/src/shards.rs:380-382`), and the stamp
      is a function of those lengths. So a stamp that grew means part of
      the batch is on disk and part may not be. The guard then forgets the
      locks, exactly as on a panic, and the word answers SM02, saying the
      library is left locked.
    - An unchanged stamp means no record reached any log whole. At most
      one log holds a torn tail, which readers tolerate and the next
      append cuts (`smysl-text/src/shards.rs:236-251,376-379`).
      `Library::add` may also have written text objects, which are
      addressed by their identity (`smysl-text/src/library.rs:680-682`)
      and which no manifest names. The locks are dropped as usual, and
      the word answers SM02 with SMYSL's reason.
    - So every refusal that writes nothing leaves no lock:
      `check_loggable`'s (`:1347-1354`), a language-less `add`, an I/O
      error before the first shard. Revision 6 ran `check_loggable` first
      instead. The stamp makes that unnecessary, and also covers `add`,
      whose manifest is built inside the call, where Bund2 cannot check
      it first.
  - Bund2 holds the locks `acquire_all` answers in a guard of its own.
    When that guard is dropped while the thread is panicking
    (`std::thread::panicking()`), or after a failed call whose stamp grew,
    it `std::mem::forget`s each lock instead of dropping it. `Lock`'s `Drop`,
    which would remove the file (`smysl-graph/src/lock.rs:225-228`), then
    never runs. A `Lock` holds no open file, so forgetting it leaks only
    its path and name (`:126-130`).
  - The guard starts once `acquire_all` returns. A panic inside
    `acquire_all` itself unwinds inside SMYSL, which drops the locks it
    had taken. Nothing has been written by then, so nothing is lost.
  - The panic still unwinds through `catch_panic` and is reported as
    `Error::internal` (D49).
  - Every later write to that library, from this process or any other,
    is SM07 naming this process as the holder, until an operator has read
    the library and runs `bund2 smysl-unlock <root> --yes` (R14), or
    deletes the lock files by hand. Reads are not locked and still
    succeed.
  - What the log holds afterwards is Q51. A truncated tail is tolerated
    when the log is read (`smysl-graph/src/lock.rs:20-22`).
  - Rejected: letting the drop release the lock. A panic would then erase
    the notice SMYSL keeps for the interesting case (`:21-23`), and the
    program would write on to a library that may be half-written.

**Threads** (RFC-0007) [R13].

- **Each VM opens its own library.**
- **Writes to one root are serialised within the process.**
  - SMYSL's lock is a file, so a second VM in the same process would get
    SM07 just as another process would.
  - Instead, a process-wide table of mutexes, keyed by the canonical root,
    serialises writes before the file locks are taken.
  - The table holds only paths and mutexes, which are `Send`. RFC-0013
    §S2's objection to global state was to holding values containing
    `Rc`.
- **A poisoned mutex is recovered** with `into_inner`. The mutex guards no
  data, only the order of writers, so a panic in an earlier writer leaves
  nothing for it to protect. D49 has already reported that panic, and the
  lock it left (above) refuses the next write with SM07.
- **A test-only probe** makes this checkable. A feature
  `smysl-panic-probe`, on no default and in no shipped build, as
  `panic-probe` gives `test.panic` (D220), adds a process-wide probe that
  the crate's own tests arm. It acts at two points of a write, and only
  when armed:
  - **`panic`**, after the mutex and SMYSL's locks are taken and before
    the append: the write panics;
  - **`pause`**, after the first open (step 1) and before the locks: the
    write creates a file the test names, to say it has arrived, and waits
    until the test removes it. After 30 s it panics instead, so that a
    broken test fails rather than hangs.

  A failed append needs no probe. The test makes one shard's log
  unwritable, so that `ShardLog::append`'s open fails
  (`smysl-text/src/shards.rs:371-375`) after an earlier shard has been
  written (criterion 15).
- **`smysl.prove` takes no mutex or lock while solving** (§S7).

### S10. What is not in Phase 1

- **A model.** Neither `ingest_text` nor dossier answering is included;
  §S12 sets out what Phase 2 would need.
- **Merging libraries.**
  - `smysl_graph::merge` works on a bare store (`smysl-graph/src/merge/mod.rs:100`).
  - A library's in-memory store writes nothing
    (`smysl-text/src/library.rs:1372-1378`).
  - A merge of libraries would also have to carry text objects and
    manifests.

  §S11 asks SMYSL for a library-level merge (U7).
- **Bare stores** (§S2).
- **Saving dossiers and measures** (§S5, R17).
- **A cache** (§S3).
- **Every reader but two** (R12).

### S11. What SMYSL should change

These would make SMYSL a better dependency for Bund2 and for any other
embedder. U6 bears on Phase 1 through R2, and U9 through R22. U8 would
let the subcommand of R14 be withdrawn.

- **U1. Deny panics in the pure crates.** Add `unwrap_used`,
  `expect_used`, `panic` and `unreachable` to the workspace's clippy
  denies, and replace the invariant sites (§A lists three) with errors.
  This extends SMYSL's own A1 from untrusted input to every input.
- **U2. Put the embedder's surface under the contract.**
  - The query crate's `Env`, `parse`, `plan`, `run`, `Answer` and
    `dossier::ask`.
  - The library's `create`, `open`, `add`, `append_records`, `catalog`,
    `shards` and scope.
  - `lock::acquire_all`, `plan_retraction`, and `trace` and
    `dependents_trace`.
  - The measures' `run`, `Recipe` and `Input`.

  A pin could then move with semver's protection, and Q55 would be
  answered by the contract.
- **U3. A writer lock on the bare log store,** as libraries and the redb
  store have, or a documented statement that one writer at a time is the
  caller's job.
- **U4. A plain-value projection of a record.** `HValue` is already a
  closed tree (`smysl-core/src/surface/hjson.rs:45-58`). Exporting such a
  projection for units and relations, flagging what it cannot carry, would
  spare every embedder from writing §S2's mapping.
- **U5. `smysl-ingest` without the model by default.** Its default `model`
  feature (`smysl-ingest/Cargo.toml:20`) links the provider layer into any
  dependent that forgets to turn it off.
- **U6. Publish 1.12–1.14 to crates.io,** so Bund2 can depend on a
  release rather than a revision.
- **U7. A library-level merge,** carrying records, text objects and
  manifests under both libraries' locks.
- **U8. The `--break-lock` command `lock.rs` names.** It is documented in
  `Lock::break_lock`'s comment and in SMYSL-2.4, and not implemented in
  the CLI (§Current, "The locks").
- **U9. Lock every command that appends to a library.** `q --save`,
  `ask --save`, `measure --save`, `assess` and `same-as` call
  `append_records` without `acquire_all` (§Current, "The locks"), which
  the comment on `Library::shards` says no library-wide writer does
  (`smysl-text/src/library.rs:765-769`). Since an append cuts a torn tail,
  one of them can silently cut off a record another writer is part way
  through.

### S12. Phase 2, not designed here

The steps that call a model are extracting units from a text
(`ingest_text`) and answering a dossier (`smysl-ingest/src/answer.rs:1-3`).

- Both need `smysl-provider`, which links `tokio`
  (`smysl-provider/Cargo.toml:39`).
- Bund2 links no async runtime of that kind today.

Whether it ever should, and behind which feature, is R1. If R1 rules
Phase 2 in, it comes as an amendment with its own criteria.

---

## Implementation plan

Everything lives in a `smysl` module of `bund2-stdlib`, compiled only
under the feature. Every milestone ends with:
- its criteria passing, under the feature and in the default build;
- `cargo xtask conform` and `cargo xtask coverage` unchanged (D72);
- clippy passing under the feature.

Each word is named in `FEATURE_GATED`, and each writing word in
`ACTS_ON_HOST`, in the milestone that registers it. Each refusal code is
tested in the milestone that first raises it, so criterion 14 is complete
when M10 is.

| Milestone | Builds | Criteria |
|---|---|---|
| M1 | the `smysl` feature on `bund2-stdlib` and its forwarding from `bund2-cli` (§S1); the eight crates at `rev = "4f5ecb9"` and the `allow-git` entry naming D244; `SMYSL_BUILT_IN` in `features_built_in`; four of §S1's five CI steps, all but the `smysl_demo` run; `FEATURE_GATED` moved to module scope and `UNVARIED` filtered by it. **First, Q55's spike:** in a test, build each value §S4–§S7 need against a fixture library, and run one query, one dossier, one measure and one attested append, through SMYSL's API alone | 1, 2, 3; 16's audits; 18's `features_built_in` case |
| M2 | handles and values (§S2), and no word that writes: the handle MAP, canonicalisation, validation; `smysl.open`, `smysl.agent`; the read-back and write mappings of units, relations, sources, diagnostics and payloads, tested as functions against records built through SMYSL's API | 14's SM00, SM01, SM03 and SM05 cases that need no write; 18's read-back cases |
| M3 | the first writes, with their locks (§S9, §S4): the process-wide mutex table; open, lock, re-open and the shard-set restart; the guard and its stamp rule; the probe feature; the `bund2 smysl-unlock` subcommand; `smysl.create` and `smysl.unit`, with attestations, their clock, rung, hop and parents, and the already-attested rule | 4; 15's stale-lock, thread, panic and re-open cases; SM07 |
| M4 | the rest of forming (§S4): `smysl.rel`, `smysl.label`, `smysl.append`; `smysl.text.add` and its spec, `smysl.texts`; `smysl.parse`, `smysl.write`; `smysl.check` | 5, 7; 15's failed-append and nothing-written cases; 18's `smysl.rel` and `smysl.label` cases; SM02, SM08 |
| M5 | retraction and research (§S4, §S5): `smysl.retract` in SMYSL's sequence; `smysl.get`, `smysl.trace` with all five kinds, `smysl.status`, `smysl.salience`, `smysl.pack` | 8; 15's attestation and advisory-authority cases; 18's salience and pack case; SM09 |
| M6 | dossiers and measures (§S5), in the environment §S5 names | 10 |
| M7 | querying (§S6): `smysl.query` and its answer MAPs; the `smysl` conditional and its runner; `bund2 check`'s arm; the runner's `REENTERING` entry | 6, 9; 16's `REENTERING` scan; 18's `bund2 check` and warning cases; SM04, SM10 |
| M8 | `smysl.kb` (§S7) | 11 |
| M9 | `smysl.prove` (§S7): proof nodes to units, statuses under rule M, gists fitted to the library's profiles after NFC, solve then write; §S7.1's rendering; its `bund2 check` arm | 12, 13; 18's `smysl.prove` case; SM12 |
| M10 | `bund_examples/smysl_demo/` and `smysl_demo.rs`, with the fifth CI step, which runs it; the `--noio` stubs (§S8); the editor syntax file left as at 0.53.0 | 14 complete (SM06, SM11), 16 complete, 17; 18's `--noio` case |

Each criterion is claimed by the first milestone at which every word it
uses exists. A criterion split across milestones is complete at the last
of them.

**What may stop a milestone.**
- **M1.** If Q55's spike finds two values that cannot be composed from
  outside SMYSL, M1 stops and the gap goes to SMYSL as part of U2, or to
  an amendment.
- **M3.** Its words are the first that write, and they arrive with §S9's
  locks, so that no word ever writes without them. `smysl.create` writes,
  which is why it waits for M3 rather than joining `smysl.open` in M2.
- **R22** is ruled A, so M3 does not wait for U9. The documentation
  M10 writes for the example says that a library a Bund program writes
  must not be written at the same time by SMYSL's five unlocked commands.
- **Q51** stays open through M4. If M4's failed-append test shows a
  library that SMYSL cannot read back, that is an amendment for a repair
  step.

---

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| Every word the reference registers | **Preserved exactly.** No name is reused: no `smysl` name exists in the reference or in Bund2 |
| The default build | **Preserved exactly.** The feature is off, so nothing is compiled or registered |
| `save.model`, `load.model` and the world file | **Preserved exactly.** A handle is a MAP with a canonical root, and saves like any other value |
| `--noio` | **Extended in kind** [R11]. Every `smysl.` word that touches disk is stubbed, as the words at the reference's 16 `cli.noio` sites are. `smysl.parse` and `smysl.agent` do no I/O and are not stubbed |
| `conform`, `COVERAGE` and `IMPLEMENTED` | **Unchanged.** Words only Bund2 has are not in the reference's registry (D72), and the default build does not register them |
| `bund2 words` | **Differs by build**, as it already does for `grok`. The `smysl` build lists the new words |
| SMYSL's lock as a crash notice | **Kept** [R20]. A panic, or a failed call after which a log has grown, leaves the locks, as a killed process does, so the library refuses writes until an operator breaks them with `bund2 smysl-unlock` (R14), since SMYSL has no command for it |
| The `bund2` CLI | **Extended in the `smysl` build** by one subcommand, `smysl-unlock` [R14]. The default build's CLI is unchanged |
| A write's view of the library | **Stricter than SMYSL's commands.** Bund2 re-opens under the locks (§S9), so a record another locking writer appended just before is not appended again |
| Concurrent use with SMYSL's saving commands | **Not safe, as in SMYSL itself** [R22]. `q --save`, `ask --save`, `measure --save`, `assess` and `same-as` append without locks, so a Bund2 write beside one of them may decide from a stale library, or cut off a record it is part way through (§S9). Bund2 makes this neither safer nor less safe; U9 asks SMYSL to lock them |
| Reading a library | **Not strictly read-only, as in SMYSL.** Every open, a read's included, unlinks the objects of redacted parts, without a lock (§Current, "The library") |
| A library's layout | **May be raised.** A Bund2 write that creates a shard in a layout-1 library rewrites its marker to layout 2, as any SMYSL write does. SMYSL releases before 1.10.0 then no longer read it whole (§Current, "The library") |
| Filesystems | **As SMYSL's.** A lock is a hard link, so a library on a filesystem without hard links cannot be written; each write is SM02 (§S9) |
| `retracts` relations | **Written only by `smysl.retract`** [R23], so every retraction passes SMYSL's plan and its authority check. A document holding a `retracts` relation cannot be appended with `smysl.append` |
| Salience and hop | **Affected as intended.** A proof conclusion's attestation names its grounds as parents, which salience reads as its ancestry; every Bund2 unit sits at hop 0 (§S4) |
| SMYSL's retraction authority | **Kept, and stated as advisory** [R21]. `Origin` is checked as SMYSL checks it; agent ids are self-asserted, in Bund2 as in SMYSL |
| What a manifest records | **Named.** Bund2's defaults for `carry` (`none`) and the part policy (the reader's top level) are SMYSL's, and are recorded in each manifest Bund2 writes (§S4) |
| The palette audit | **Kept safe.** Every `smysl.` word, and the constructor `smysl`, is named in `FEATURE_GATED`; every one that writes is also named in `ACTS_ON_HOST`. So under `--all-features` the audit never runs `smysl.create` on a palette string, which would create directories |
| The `UNVARIED` audit | **Changed to filter `FEATURE_GATED`**, as the palette audit does. `FEATURE_GATED` moves from the palette test's body to the test module's scope so that both tests see it, and its comment that `grok` is the crate's only feature is rewritten. It asserts an exact list of the fixed-effect natives whose operands the palette does not all vary (`crates/bund2-stdlib/src/lib.rs`, `UNVARIED`). `smysl.trace`, `smysl.label` and `smysl.measure` would otherwise enter it in one build only, which is F123's shape |
| `PROMOTABLE.txt` | **Gains one comment line per gated word** (`# feature-gated: <name>`), since its regeneration names each `FEATURE_GATED` entry. The list of promotable natives is unchanged. Under D72 its regeneration is the owner's to run |
| Tier 1 promotion | **Unchanged.** `smysl.prove` is opaque and never crossed. The fixed-effect `smysl.` words stay unlisted, so promotion syncs before them, which costs speed and never meaning. The `smysl` constructor is feature-gated, so it is never certified |
| The `REENTERING` audit | **Extended by one entry**, the `smysl` runner's function, which calls `eval_lambda`. The scan reads sources whatever the build, so the entry is checked in the default build too |
| `bund2 check` | **Extended.** The per-row conditional arm gains `"smysl"`, and `smysl.prove` gets an opaque arm. The fixed-effect words need nothing new |
| Bundles | **Unchanged in mechanism.** `features_built_in` records `smysl` from `SMYSL_BUILT_IN`. A bundle that uses `smysl.` words, run on a runtime without the feature, meets an unbound word, as it would meet `string.grok` |
| The editor syntax file | **Unchanged.** `editors/nvim/syntax/bund_words.vim` is generated from whichever `bund2` is on the PATH. It is regenerated from a default build, as at 0.53.0 |
| CI | **Extended** (§S1): two nextest runs, a clippy run, the feature-matrix check with and without a C compiler, and a `cargo deny` run under the feature |
| RFC-0013's words and engine | **Preserved exactly.** `smysl.kb` builds a KB through `kb!`'s validation, and `smysl.prove` reads proofs as §S15 gives them. No clause and no source is added to a proof |
| `q` | **Preserved exactly** (D32). No status becomes a `q` |

No deviation from the reference is proposed.

---

## Alternatives considered

- **No integration.** §A rejects it if Bund is to reason over corpora. It
  is R1's option C, and the right ruling if Bund is not meant for that.
- **Drive the `smysl` CLI as a subprocess.** This needs no dependency and
  keeps a panic out of the process. Rejected because every call pays for a
  process and a parse, and answers come back as text that must be parsed
  again. It stays the fallback if R2's pin proves unworkable.
- **The facade crate `smysl` with defaults off.** One dependency instead
  of eight, and it is what SMYSL's contract covers. Rejected for Phase 1,
  because it also pulls in `smysl-thread` and `smysl-render`, which
  Phase 1 does not use, and it re-exports the query module whole without
  the contract covering it. R2 may prefer it.
- **crates.io 1.11.0.** A published release, but 2,853 changed lines
  (2,602 inserted, 251 deleted) behind 4f5ecb9 in the crates this RFC
  uses, and not the version read here. It is R2's option B.
- **A native handle value type.** It would avoid reopening the library.
  Rejected because Bund2's values are data (RFC-0001), and such a handle
  would survive neither `save.model` nor the bus.
- **Bare stores beside libraries.** They take no lock (U3). Rejected for
  Phase 1.
- **`smysl.merge` over `append_records`.** It would have to read the
  second library under its own locks, copy its text objects as well as
  its records, and detect contention itself. That is a library-level merge
  SMYSL does not have yet (U7). Rejected for Phase 1.
- **Fixed clocks for attestations.** A fixed `Hlc` per proof would make
  re-proving byte-identical. Rejected: skipping units already present
  (§S4) gives the same result, and the clock records when a program
  vouched for a unit.
- **The narrow bridge alone** (§S7 without §S4–§S6). Smaller, and the part
  of this RFC with the most value per line. Rejected as the whole, because
  the owner asked to form, research and query. It is R1's option B.
- **Re-implementing SMYSL's model in Bund.** Rule M and retraction are
  Datalog-shaped, and RFC-0013 could express them. Rejected, because most
  of SMYSL is not logic: identity by BLAKE3 over a deterministic CBOR,
  the library and its readers, and `sq/1`.

---

## Acceptance criteria

1. **The default build is untouched.** Without the feature:
   - `cargo tree -p bund2-cli` names no `smysl-` crate;
   - `bund2 words` lists no `smysl` name;
   - `conform` and its CEILING, `COVERAGE` and `IMPLEMENTED` are unchanged
     from the parent commit.
2. **The feature's dependency tree is pure and licensed.**
   - `cargo tree -p bund2-cli --features smysl` names none of `tokio`,
     `ureq`, `lingua`, `smysl-ingest` and `smysl-provider`, and no redb
     2.x.
   - `cargo deny --features smysl check` passes, and CI runs it (§S1).
3. **No C compiler is needed** [R18]. `cargo build -p bund2-cli --features
   smysl` succeeds with `CC` and `CXX` set to a program that fails (Q52).
   CI runs it (§S1).
4. **Uids agree with SMYSL's own parser.** For each unit of a `.smy`
   fixture, the uid `smysl.unit` answers for the same unit written as
   §S2's MAP equals the uid SMYSL computes. The test computes SMYSL's uid
   by calling `parse_surface` on the fixture's text and `canonical_uid` on
   the record, directly.
5. **The surface round-trips through a library.**
   - **The documents.** At least five, written for this test in Bund2's
     own tree, so no SMYSL file is copied. Between them they hold:
     - units of every status a program can author;
     - relations of each of SMYSL's fourteen kernel kinds but `retracts`,
       which `smysl.append` refuses [R23], and of one extension kind
       [R24];
     - labels, none of them bound twice in one document, since a
       document's last binding wins (`smysl-graph/src/labels.rs:10`);
     - a payload that is lossless;
     - a `@doc` header and a thread, which §S4 says are skipped.
   - **The test**, for each document:
     1. `smysl.parse` it, then `smysl.append` the MAPs into a fresh
        library;
     2. `smysl.write` the library;
     3. `smysl.parse` and `smysl.append` that text into a second fresh
        library.

     The two LISTs `smysl.append` answered are equal **as sets**, and the
     skipped kinds draw the warnings §S4 names.
   - A sixth document, holding a `retracts` relation, is refused by
     `smysl.append` with SM03, and the library is unchanged.
6. **Forming persists.** A test:
   - creates a library in a temporary directory;
   - adds one text per alias with each enabled reader, each naming its
     `lang`;
   - appends units and relations.

   Then, in a fresh VM:
   - `smysl.texts` names each alias with the `mid` `smysl.text.add`
     answered as its head;
   - every unit is found by `smysl.get` and by `smysl.query`;
   - each manifest records carry `none` and SMYSL's default part policy,
     read through SMYSL's API.
7. **Checks report.**
   - A library whose `derived` unit exceeds its weakest ground gives
     `smysl.check` a diagnostic with `code = "SMY-E030"` and severity
     `error`.
   - A library holding only well-formed units is clean (§S4).
8. **Retraction is SMYSL's.**
   - For a fixture with a retraction and a two-step cascade, `smysl.status`
     equals `effective_status` under the strict policy, compared in the
     test.
   - A unit attested only by `tool:other` is refused with SM09 when a
     handle with agent `tool:bund2` retracts it.
   - The edge written by an accepted retraction runs from the target to
     itself, and carries no attestation.
   - Retracting a unit already read as retracted writes nothing and
     answers NODATA, whatever the handle's agent.
   - `smysl.trace` with `"rests-on"` equals `trace_via` over
     `EdgeSet::premises`, compared in the test.
9. **Every answer shape is mapped.**
   - One query per `Answer` variant gives §S6's MAP, with the uids `run`
     gives, in the same order.
   - A query `plan` refuses is SM04, carrying every diagnostic.
   - A `count` answer runs the `smysl` conditional's lambda zero times.
10. **Dossiers and measures match, and write nothing.**
    - One dossier, and the measures T1 and A4, over a fixture library give
      what the direct calls give. The direct calls use the environment §S5
      names: the same `LibraryScope`, an `Env` with times and entities,
      `PlanOptions::default()` and the base `Recipe`.
    - A measure id `"XX"` gives a result with `SMY-E489`.
    - The library's catalog and units are unchanged afterwards.
11. **The bridge loads.** `smysl.kb` over a scope gives the facts of §S7's
    table, each kind counted against the library.
12. **Exported proofs keep rule M.**
    - **The proofs.** A test proves goals whose proofs contain, between
      them, every node kind in §S7's table:
      - a fact standing for a unit, and a `rank` fact;
      - a program fact at each allowed status;
      - a rule, `once` and `call`;
      - `:branch`, `:then` and `:else`;
      - a `:findall` with members, and an empty one;
      - `:no-proof`, `:builtin`, `:bridge` and `:tabled`;
      - a goal with two uid arguments;
      - a goal whose rendering is too long for the profile and holds a
        multi-byte character near the cut;
      - a goal with an atom that is not NFC, with U+0958 near the cut.
    - **The libraries.** Each proof is written into four libraries:
      - one with no view;
      - one whose view sets `l0_max: 20`;
      - one with two views of different bounds;
      - one whose view's profile names `smysl/content/1`, built by a
        test through SMYSL's API, with a goal of CJK atoms.

      A fifth library, whose view sets `l0_max: 0`, gives SM12 and
      writes nothing.
    - **For each proof:**
      - SMYSL accepts every unit written;
      - `smysl.check` finds the library clean, with no `SMY-E030` and no
        `SMY-E022`;
      - every status equals §S7's computation;
      - a corpus fact is grounded on by its uid, and not copied;
      - each written conclusion's attestation names its grounds as
        parents, and its clock is later than the agent's previous one in
        that library.
    - **Re-proving** the same goals leaves every shard log's length
      unchanged.
    - **An askable goal** reached by `smysl.prove` is E50, prefixed
      `SMYSL.PROVE: `.
13. **Rendering is stable.** Each row of §S7.1 has a test that fixes its
    text, including a BOOL, a partial list and an improper list. An opaque
    value is SM03.
14. **Refusals.** SM00–SM12 each have a test asserting §S8's text. The
    tests include:
    - an empty span, and a non-MAP payload;
    - for `smysl.text.add`, a `path` to a sparse file one byte longer than
      `Caps::default().input_bytes`, made with `set_len`, which is SM02
      with `SMY-E440` and reads nothing. The `text` form shares the length
      check, and is not tested with a STRING over 1 GiB;
    - a `smysl.text.add` spec with no `lang`, which is SM03;
    - a gist of spaces, which SMYSL refuses as empty after `trim`
      (`smysl-core/src/types/unit.rs:146-149`), so SM02;
    - a unit MAP with `body: ""` and a `detail`, which is SM02.
15. **Locks are SMYSL's.**
    - With a lock file left as a crashed SMYSL process would leave it, a
      write is SM07 naming the holder, and `smysl.open` of the library
      succeeds.
    - Four VMs on four threads each append 250 units to one library,
      through two spellings of its root. All 1,000 are present afterwards,
      read through SMYSL's API.
    - With the probe armed to `panic`, a write that panics with the mutex
      and the locks held:
      - is reported as `Error::internal`;
      - leaves the library lock and every shard lock it took in place;
      - makes the next write to that root, from a fresh VM on another
        thread, SM07 naming this process;
      - leaves `smysl.open` of that root succeeding.

      After `bund2 smysl-unlock <root>` lists every one of those locks with
      its holder, and `--yes` breaks them all, a write succeeds.
    - **A failed append.**
      - The fixture adds a text under the alias `zeta` and appends one unit
        citing a part of it, so the shard `zeta`, which sorts after
        `catalog`, has a log.
      - The test makes `zeta`'s log read-only, then appends in one
        `smysl.append` a unit with no source, routed to `catalog`, and a
        unit citing the part, routed to `zeta`
        (`smysl-text/src/shards.rs:456-466`).
      - So `catalog`'s log grows and `zeta`'s open fails. The word answers
        SM02 saying the library is left locked, and every lock stays, as
        after a panic.
      - The test first checks that the read-only log does refuse an
        append. If the environment ignores permissions, as a root user
        does, the test fails with a message saying so.
    - **A failure that writes nothing leaves no lock.** A batch
      `check_loggable` refuses, and a `Library::add` that SMYSL refuses for
      want of a language, called through Bund2's guarded write directly
      (the word's own SM03 stops it first), each leave no lock, and the
      next write succeeds.
    - **The re-open.** With the probe armed to `pause`, a `smysl.append`
      of two new units stops after its first open. The test then appends
      the first of them, with an attestation by the same agent, through
      SMYSL's API under `acquire_all`, and releases the pause.
      - The write appends neither that unit nor a second attestation of
        it.
      - The attestation it writes for the second unit has a clock later
        than the other writer's. That holds because both attest as the
        same agent, whose newest clock is `prev` (§S4).
    - A unit written first by `tool:other` and then appended by a
      `tool:bund2` handle gains one attestation, and that handle may then
      retract it.
    - A handle whose agent is set to `tool:other` by `smysl.agent` may
      retract a unit only `tool:other` attested. This pins R21's statement
      that authority is advisory.
16. **The audits see the words.**
    - `cargo nextest run -p bund2-stdlib --features smysl` passes, and so
      does the default build's run. Under the feature:
      - every `smysl.` word and the constructor `smysl` are in
        `FEATURE_GATED`;
      - the writing words are in `ACTS_ON_HOST`;
      - the palette and `UNVARIED` audits both filter `FEATURE_GATED`.
    - The default build's `REENTERING` scan names the `smysl` runner's
      function.
    - Clippy passes under the feature.
17. **An example runs.**
    - `bund_examples/smysl_demo/` forms a small library, researches and
      queries it, and reasons over it with `smysl.kb` and `smysl.prove`.
    - Its `expected.txt` and a new test, `smysl_demo.rs` in the CLI
      crate's tests,
      compare the output, as for `logic_demo`, and CI runs that test
      (§S1).
18. **What the other criteria do not reach.**
    - `smysl.salience` and `smysl.pack` over a fixture library equal
      `salience` and `pack` called directly with the same inputs.
    - `smysl.rel` and `smysl.label`, each alone, write what the same record
      written through `smysl.append` writes. `smysl.rel` given a
      `retracts` relation is SM03 [R23].
    - `bund2 check` reads a program using the `smysl` conditional, and one
      using `smysl.prove`, with no unknown-effect diagnostic, and takes
      `smysl.prove` as 4 → LIST.
    - `features_built_in` names `smysl` in the `smysl` build and not in the
      default one.
    - Under `--noio`, `smysl.parse` and `smysl.agent` work, and every other
      `smysl.` word is SM06.
    - The function that reports a run's `warnings`, a measure's
      diagnostics and a write's warnings gives one `Warning` per warning
      (D36), driven with a diagnostic of each severity, since which
      queries warn is SMYSL's to change.
    - A source whose `observed` is above `i64::MAX`, written from its
      STRING form, reads back as that STRING.
    - Data a later SMYSL may write reads back: a source of an unknown kind
      code, built with `SourceRef::with_unknown_kind`, with `kind` as that
      INT and `opaque = true`; an extension relation kind as its text; and
      a relation with a non-empty `extra` with `opaque = true`.

---

## Open questions

Each is in `docs/registers/open-questions.md`.

- **Q50. Answered by revision 2.** Libraries are locked by SMYSL, and
  Phase 1 writes only libraries. *Corrected by revision 8:* a lock is a
  hard link, not a `create_new` file, and not every SMYSL writer takes
  it (§Current, "The locks"; Q57).
- **Q51 [UNGROUNDED], narrowed.**
  - A truncated tail is tolerated (`smysl-graph/src/lock.rs:20-22`).
  - Since revision 5, a panic leaves the lock in place (§S9, R20), so a
    half-written library refuses writes until an operator breaks the
    lock. This question asks what that operator finds.
  - `append_records` appends to each shard's log in turn, with an `expect`
    inside the loop, before it updates the in-memory store
    (`smysl-text/src/library.rs:1364-1379`). A failure part-way leaves
    some shards written and others not.
  - What remains is a write interrupted between its records, such as a
    text's objects without its manifest.
  - *Narrowed by revision 8.* Shards written before a failure keep their
    records. The failing shard may hold a torn tail, which reading
    tolerates and the next append cuts
    (`smysl-text/src/shards.rs:236-251,376-379`). So after
    `bund2 smysl-unlock`, the next write silently discards that partial
    record. What stays open is whether a batch written to some shards and
    not others leaves a library SMYSL's own checks object to.
  - If that can leave a library half-written, a repair step is an
    amendment.
- **Q52.** Does `blake3` with `pure` and no default features invoke a C
  compiler at build time? **Answered 2026-11-02, after D244: no.**
  - **Read.** `blake3` 1.8.5, the version SMYSL's lock file holds, is read
    from the crates.io source.
    - With `pure`, its build script compiles no C on any target. On x86
      it takes Rust intrinsics and skips AVX-512 (`blake3` 1.8.5
      `build.rs:346-356`). On aarch64, `pure` turns off the NEON C path
      (`:367-371`).
    - On x86 it still probes for a compiler (`:161-194`), and a failing
      one is caught as "no compiler", with a cargo warning (`:186-192`).
    - `cc` is only its build-dependency, and the only other user of `cc`
      in SMYSL's lock is `ring`, which comes through `rustls` to the
      model layer (Phase 2).
  - **Built.** A scratch crate depending on the eight Phase 1 crates at
    4f5ecb9, with `default-features = false` and `smysl-text`'s
    `reader-txt` and `reader-md`, was built on aarch64 macOS with Rust
    1.95.0 and `CC=false CXX=false`.
    - It succeeded.
    - `blake3`'s build script set no C-path configuration.
    - The tree held no `tokio`, `ureq`, `ring`, `lingua` or `redb`.
  - x86_64 is left to criterion 3's CI step, which builds the same way.
- **Q53. Answered by revision 2.** A dossier and a measure write nothing.
- **Q54.** How long does `Library::open` take at the sizes a Bund program
  is likely to hold?
  - SMYSL reports 1.5 ms for opening a view at 155k units
    (`smysl:README.md:57`), which is not the same operation.
  - Phase 1 opens per call. A cache is an amendment if the measurement
    asks for one.
- **Q55.** Can every value the words need be built from outside SMYSL's
  crates at 4f5ecb9?
  - The values: an `Env` over a library's scope, `AskOptions` and its
    templates, a measure's base `Recipe`, a `measure::Input`, and an
    `Attestation` with an `Hlc`.
  - Reading a source's span and manifest is settled: `span` and `manifest`
    are public (`smysl-core/src/types/epistemics.rs:524,529`).
  - The attestation is settled too: `Attestation::new` and `Hlc::new` are
    public (`smysl-core/src/types/provenance.rs:33,269`), and so is
    `AddSpec`, whose fields are all public (`smysl-text/src/library.rs:86-116`).
  - **Answered 2026-11-02, after D244: yes, at 4f5ecb9, by reading.**
    Every remaining value has a public constructor:
    - `Env::new` (`smysl-query/src/plan.rs:70`), with `with_times` and
      `with_entities_of`, over `Library::store()`
      (`smysl-text/src/library.rs:757`);
    - `LibraryScope::new` (`smysl-text/src/scope.rs:122`);
    - `AskOptions::new` (`smysl-query/src/dossier/mod.rs:133`);
    - `Recipe::new` (`smysl-measure/src/recipe.rs:193`);
    - `measure::Input::new`, `with_library` and `with_text`
      (`smysl-measure/src/lib.rs:146,170,176`);
    - `scope::resolve` (`smysl-measure/src/scope.rs:100`);
    - `Corpus::from_docs`, and a builder over a library
      (`smysl-measure/src/text/mod.rs:351,436-440`).
  - `Env` holds an `Rc`, so it is not `Send`. Phase 1 builds one per call
    and never shares it.
  - Whether the pieces compose is left to M1's first step, a spike that
    builds each value against a fixture library and runs one query, one
    dossier, one measure and one attested append. U2 would keep the
    answer true across revisions.
- **Q56.** What does a dossier lose without stemming, and which template
  sets does this build carry?
  - Rule 26 finds a topic's other forms by stem
    (`smysl-query/src/dossier/mod.rs:318-319`), and Phase 1 does not
    enable `stem`.
  - A template set the build does not carry is `SMY-E477` (`:118-119`).
- **Q57.** Can a writer append between a library's open and its locks?
  - SMYSL's own commands open, read the shard names, then lock
    (`smysl:src/main.rs:8618-8628`), and §S9 follows them.
  - If a shard can appear in that window, the write may miss its lock.
    That is SMYSL's to answer (U2, U3).
  - *Narrowed by revision 6*, on the premise that every SMYSL writer
    takes the library lock first.
  - **Re-opened by revision 8:** the premise is false. Five SMYSL
    commands append without any lock (§Current, "The locks"), and an
    append cuts another writer's partial record as a torn tail (§S9).
    Against writers that lock, the re-open and the restart answer the
    question. Against the five, nothing Bund2 does answers it. U9 asks
    SMYSL; R22 decides whether Phase 1 waits.
- **Q58. Answered by revision 6:** no, and the reverse is the point.
  `SMY-E020` resolves a body token as a label only through the `labels`
  map the caller gives (`smysl-check/src/passes/closure.rs:46-80`). Under
  `CheckOptions::default()` that map is empty, so a library's label
  references go **unchecked** by `smysl.check`. Closure reads only the
  body (`:60`), and Bund2's bodies hold no uid. Checking label
  references would mean passing the library's bindings through
  `with_labels`, an amendment to R19. *As raised:* does a library that
  binds labels draw closure diagnostics from `smysl.check`?
  - `CheckOptions::default()` passes no labels. SMYSL's comment says
    "Labels have no wire record, so only a caller that parsed the surface
    text has them" (`smysl-check/src/lib.rs:176-178`).
  - Yet a store does hold `Record::LabelBinding`
    (`smysl-graph/src/store/mod.rs:932`).
  - If a body that names a unit by label is reported, "clean" (§S4,
    criterion 7) depends on whether Bund2 passes the library's bindings
    through `with_labels`. That would be an amendment to R19.
  - Bund2's own writes put no label in a body, so §S7's exports are not
    affected.

---

## Rulings asked for (D244, D245)

| | Point | Options | Recommended |
|---|---|---|---|
| R1 | Scope | **A** Phase 1 as designed; Phase 2 a later amendment. **B** the bridge alone (§S7). **C** no integration. **D** Phases 1 and 2 together | **A** — **ruled A** 2026-11-02 |
| R2 | Dependency source | **A** git at `rev = "4f5ecb9"`, with an `allow-git` entry ruled here (D38's mechanism, not its reason), moving to crates.io when U6 lands. **B** crates.io `=1.11.0`, with this RFC re-grounded against it. **C** wait for U6 | **A** — **ruled A** 2026-11-02 |
| R3 | Handle | **A** a MAP of `type = "smysl"` with a canonical root and an agent. **B** a STRING path. **C** a new native value type | **A** — **ruled A** 2026-11-02 |
| R4 | Opening | **A** per call, no cache, until Q54 asks for one. **B** per call, with a process-wide cache | **A** — **ruled A** 2026-11-02 |
| R5 | Writes | **A** each word appends, and `smysl.append` appends a LIST under one acquisition of the locks. **B** single-record words only | **A** — **ruled A** 2026-11-02 |
| R6 | Queries | **A** `sq/1` text as written. **B** queries built from Bund values | **A** — **ruled A** 2026-11-02 |
| R7 | The fact schema | **A** §S7's table. **B** one fact per unit, with every field as an argument | **A** — **ruled A** 2026-11-02 |
| R8 | Exported proofs | **A** §S7: provenance by shape and library; the node table; `derived` capped by declared statuses; `else`, `not` and `tabled` capping at `inferred`; no grounds meaning `speculative`; gists fitted by `fit_gist_with` to every profile of the library's views, SM12 when none fits, and the full goal in the detail; §S7.1's rendering; solve, then write. **B** refuse any proof that holds a program fact or a capping node | **A** — **ruled A** 2026-11-02 |
| R9 | Names | **A** the `smysl.` prefix and the conditional `smysl`. **B** a `corpus.` prefix | **A** — **ruled A** 2026-11-02 |
| R10 | Upstream | **A** U1–U5, U7 and U8 do not block Phase 1. **B** Phase 1 waits for U1 and U2 | **A** — **ruled A** 2026-11-02 |
| R11 | `--noio` | **A** every word that touches disk is stubbed; `smysl.parse` and `smysl.agent` are not. **B** reads allowed, writes stubbed | **A** — **ruled A** 2026-11-02 |
| R12 | Readers | **A** `txt/1` and `md/1`. **B** every reader SMYSL has. **C** `txt/1` alone | **A** — **ruled A** 2026-11-02 |
| R13 | Writers in one process | **A** serialised by a process-wide table of mutexes keyed by canonical root, a poisoned mutex recovered. **B** SM07, as between processes | **A** — **ruled A** 2026-11-02 |
| R14 | A stale lock | *Re-decided by revision 6: SMYSL has no `--break-lock`.* **A** SM07 until an operator breaks it with a `bund2 smysl-unlock <root>` subcommand (lists the holders; `--yes` breaks every lock through `Lock::break_lock`), with U8 asked of SMYSL. **B** SM07 until an operator deletes the lock files by hand, which the RFC names. **C** a `smysl.unlock` word | **A** — **ruled A** 2026-11-02 |
| R15 | Attestation | **A** the handle's `agent` (default `"tool:bund2"`), `Authored`, rung `Document` for `cited` and `Computed` otherwise, hop 0, parents empty except a proof's conclusions, whose parents are their grounds, and a clock that is `Hlc::now` from this agent's newest clock in the library. Each is written as its own record; `measured` is refused; no attestation is written where this agent has already attested the uid; the retraction edge is unattested, as SMYSL writes it. **B** no attestation | **A** — **ruled A** 2026-11-02 |
| R16 | `.` forms | **A** none in Phase 1. **B** a form for `smysl.parse` | **A** — **ruled A** 2026-11-02 |
| R17 | Dossiers and measures | **A** answered, not saved; saving is an amendment. **B** saved by default. **C** a `save` key | **A** — **ruled A** 2026-11-02 |
| R18 | No C compiler | **A** the feature must build without one (criterion 3), a rule this RFC sets. **B** no such rule; the feature may compile C, as `grok` does | **A** — **ruled A** 2026-11-02 |
| R19 | `smysl.check`'s profile | **A** `CheckOptions::default()`: the profile SMYSL derives from the library's views, or the `default` preset without one. **B** always the `default` preset | **A** — **ruled A** 2026-11-02 |
| R20 | A panic or a failed append in a write | **A** SMYSL's locks are left in place (forgotten, not dropped), so the library refuses writes with SM07 until an operator breaks them; a batch `check_loggable` refuses keeps no lock. **B** the locks are released and the next write proceeds | **A** — **ruled A** 2026-11-02 |
| R21 | Agent ids | **A** any agent SMYSL's grammar allows; `Origin` authority stated as advisory, since ids are self-asserted. **B** `smysl.agent` and handles restricted to the `tool:` kind | **A** — **ruled A** 2026-11-02 |
| R22 | SMYSL's unlocked writers | *Raised by revision 8 (D245).* **A** Phase 1 proceeds; its documentation says a library a Bund program writes must not be written at the same time by `q --save`, `ask --save`, `measure --save`, `assess` or `same-as`; U9 is asked of SMYSL. **B** Phase 1 waits for U9, and pins a SMYSL revision that has it | **A** — **ruled A** 2026-11-02 |
| R23 | `retracts` outside `smysl.retract` | *Raised by revision 8 (D245).* **A** `smysl.rel` and `smysl.append` refuse a `retracts` relation with SM03, naming `smysl.retract`. **B** accept it, and state beside R21 that it bypasses the plan and authority | **A** — **ruled A** 2026-11-02 |
| R24 | Extension relation kinds | *Raised by revision 8 (D245).* **A** `smysl.rel` and `smysl.append` accept any kind `RelKind::parse` accepts, `x.<domain>/<kind>` included, and read them back as text. **B** read them back, and refuse them on write with SM05 | **A** — **ruled A** 2026-11-02 |
| R25 | R20's test for a failed write | *Raised by revision 8 (D245).* R20 A is ruled; this restates when it applies. **A** the locks are kept when the library's stamp grew during the failed call, and dropped when it did not, for `append_records` and `Library::add` alike. **B** revision 6's test: `check_loggable` first, then every later error from `append_records` keeps the locks. Under B, an `add` that SMYSL refuses before writing anything still needs a rule, since its manifest is built inside the call | **A** — **ruled A** 2026-11-02 |

---

## Revision 2: the review answered

| Finding | Answer |
|---|---|
| B1 `smysl.prove` re-enters | Declared opaque, listed in `REENTERING`, never promoted, a `check` arm; §S7, §S9, criterion 16 |
| B2 SMYSL's lock omitted | Every write takes `acquire_all`; `SMY-E445` is SM07; stale locks are R14; Q50 answered; §S9, criterion 15 |
| B3 proofs carry no clause or source | No clause is named, and the engine is unchanged; provenance by shape and library, with a stand-for column for all eight fact kinds; §S7 |
| B4 refused shapes, unmapped nodes, gist | A node table for every §S15 kind; no grounds means `speculative`; program facts only `speculative` or `cited`; §S7.1 renders; criteria 12–13 |
| S1 dossier and measure write nothing | A "W" column; §S5 is all reads; Q53 answered; R17 |
| S2 Q51 partly grounded | Grounded parts cited; the remainder narrowed |
| S3 criterion 5 | `smysl.append` composes parse and write; criterion 5 rewritten |
| S4 criterion 6 | `smysl.texts` over `catalog`; `smysl.text.add` answers `{ mid, parts }` |
| S5 preservation omissions | Rows for `FEATURE_GATED`, `ACTS_ON_HOST`, `PROMOTABLE.txt`, Tier 1, the audits, `check`, bundles, feature forwarding (§S1), `bund2 words`, the syntax file and CI. The review's D55 sentence was set aside on filing |
| S6 `check` arms | Fixed-effect words need none; the conditional arm and `smysl.prove`'s arm are named |
| S7 refusals | Texts for SM00–SM08; SM04 carries every diagnostic; unknown measure ids are results; warnings routed through D36 |
| S8 lossy mapping | `extra`, undecodable payloads and attestations are named; views are marked `opaque` and never re-appended; non-tree values are SM03; `source` and `subject` have shapes; quantisation stated |
| S9 agents | The handle carries `agent`; R15; retraction authority stays SMYSL's default |
| S10 the cache | No cache in Phase 1 (R4 A); §S3 is consistent |
| S11 `.` forms | R16, with D18's scope as the reason |
| S12 roots and the bus | The root is made absolute; a handle is data on the bus |
| N1 | The commit and the tag are stated; crates.io has 1.11.0 only, so R2 |
| N2, N3 | Cited |
| N4 | A non-exhaustive variant is `Error::internal` (§S2) |
| N5 | §S6 says what the conditional skips |
| N6 | The unit MAP says `schema` |
| N7 | `smysl.create` is idempotent, as SMYSL's is; SM01 is `open`'s alone |
| N8 | SM08 |
| N9 | `smysl-check`'s `text` feature is not enabled; store checks only |
| N10 | Criterion 1 says "unchanged from the parent commit" |
| N11 | R5 has one recommendation; criterion 3 is stated as this RFC's own rule |
| N12 | `smysl.parse` is not stubbed; fixtures are copied with their notices; `cargo deny` runs under the feature |
| Assumptions 1–7 | 1 is Q55 and R2; 2 is R13, with its reason; 3 is SMYSL's (`create_new`), and Bund2 inherits it; 4 is §S7.1; 5 holds, since RFC-0013 Phase 2 is built; 6 is moot without a cache; 7 is SM03, applied before SMYSL sees a value |

## Revision 3: the second review answered

| Finding | Answer |
|---|---|
| B1 the check profile, E022, W041 and re-runs | `smysl.check` uses `CheckOptions::default()`, the `standard` profile. "Clean" means no error-severity diagnostic. Gists are bounded and the full goal goes in the detail. Units already present are not re-written. §S4, §S7, criteria 7 and 12 |
| B2 `smysl.merge` has no SMYSL operation under it | Removed from Phase 1 (§S10). Asked upstream as U7 |
| B3 retraction | `smysl.retract handle uid` plans with `plan_retraction` under `Origin` authority, refuses with SM09, and writes SMYSL's self-edge. §S4, criterion 8 |
| B4 the proof nodes | `once` takes a rule's shape; `rank` facts stand for nothing and add no ground; §S7.1 renders RFC-0013 §S1's read-back values, with rows for BOOL and for partial and improper tails. Criteria 12 and 13 |
| S1 `REENTERING` | The audit is described as the source scan it is. The runner's function is added; `smysl.prove` re-enters through `kb/bridge.rs: run`, which is already listed. Criterion 16 |
| S2 keeping a measure | The result carries `uid`, not a unit MAP. Saving needs the `Basis`, so it is an amendment (R17) |
| S3 trace kinds | SMYSL's `parents`, `grounds` and `both`, and `dependents` through `dependents_trace` |
| S4 criterion 5 | `measured` units excluded; the uid LISTs from `smysl.append` compared |
| S5 criterion 4 | Compared against `parse_surface` and `canonical_uid` on the fixture's text |
| S6 the mutex | Solve first, then lock and write; a poisoned mutex recovered with `into_inner`. Criterion 15 |
| S7 the view | `source` gains `observed`, `published`, `span` and `manifest`; `subject` gains all four cases |
| S8 D38 | Cited for the mechanism only. The `allow-git` entry is R2's own ruling |
| S9 register drift | Dated notes on Q51, Q52 and Q54, and a further amendment of D244 |
| S10 refusals | SM00 validates `agent`; SM03 covers operand types; SM05 covers `opts.facts` and trace kinds; SM09–SM11 added; `smysl.prove` refuses as `SMYSL.PROVE: ` |
| S11 the order of open and lock | SMYSL's order, stated and cited; the window recorded as Q57; `Library::shards` added to U2 |
| S12 CI and features | `SMYSL_BUILT_IN`; the CLI forwards to `bund2-stdlib` only; four CI changes named in §S1 |
| N1 | `smysl.texts` answers heads; criterion 6 has one text per alias |
| N2 | The `check` arms described as they are |
| N3 | §A says the caller must hold the locks |
| N4, N5 | Cited |
| N6 | The survey of the 16 `cli.noio` sites cited |
| N7 | `unfounded` is in the status row |
| N8 | `licence` is required, SM03 when missing |
| N9 | The syntax file's generator described as it is |
| N10 | Criterion 3 is R18 |
| N11 | The conditional's loss of group keys stated; `smysl.prove` is `opaque(4)` |
| N12 | The root is canonicalised |
| N13 | Declared statuses, cited |
| Assumptions 1–7 | 1 is stated in §S4. 2 is answered by skipping units already present, with fixed clocks rejected under Alternatives. 3 is Q57. 4 is §S7.1. 5 is Q56. 6 is a limit `smysl.kb` meets as `kb!` does, through RFC-0013's guard. 7 is §S9 |

## Revision 4: the third review answered

| Finding | Answer |
|---|---|
| B1 `:asked` cannot occur | `smysl.prove` has no `ask` slot, so askable goals are E50. The `:asked`/`:known` row is marked as one that cannot occur. Criterion 12 drops it and tests E50 |
| S1 attestations are their own records | §Current and §S4 corrected. A duplicate unit record is skipped by SMYSL; the attestation is written unless this agent has already attested the uid. Criteria 12 and 15 |
| S2 the check profile | `CheckOptions::default()` stated as SMYSL decides it, the library's view first; R19. The gist bound is 30 tokens in every preset |
| S3 `UNVARIED` | Filters `FEATURE_GATED`; a preservation row; criterion 16 |
| S4 the constructor and `PROMOTABLE.txt` | `smysl` named in `FEATURE_GATED`; the row says the file gains comment lines and is regenerated by the owner |
| S5 CI | nextest forms in the `check` job, a clippy run, criterion 3 gated |
| S6 criterion 5 | Documents written for the test; compared as sets; `smysl.parse` skips records with no §S2 form, with a warning; a label's answer defined |
| S7 hand-built handles | Every word canonicalises before taking the mutex; `create` creates first |
| S8 the panic test | `smysl-panic-probe`, after D220 |
| S9 SM06 | Unprefixed, the house text |
| N1 | `EdgeSet::premises` named |
| N2 | `source.extra` sets `opaque` |
| N3 | The cut is at a character boundary; criterion 12 tests it |
| N4 | The runner uses `eval_lambda` |
| N5 | Provenance is judged at write time |
| N6 | D14 and D230 consumed, along with D220 |
| N7 | Criterion 1 names COVERAGE and IMPLEMENTED |
| N8 | Q51 cites the append loop |
| N9 | Non-finite floats and control characters rendered |
| N10 | `captured` is a STRING; a measure's `value` defined |
| "registrations" | "Sites" |
| Assumptions 1–5 | 1 is R19. 2 is §S4. 3 is §S2's re-canonicalising. 4 is §S7. 5 is §S1's note on shared pins |

## Revision 5: the fourth review answered

| Finding | Answer |
|---|---|
| B1 the gist bound depends on the profile | The gist is fitted by `fit_gist_with` to every profile of the library's views, or to the `default` preset with none. Unknown estimators are left out with a warning; SM12 when no character fits. §Current "Granularity", §S4, §S7, R8, criterion 12 |
| S1 de-duplication through a library | Cited to `encode_by_shard` (`smysl-text/src/shards.rs:516-522`) |
| S2 a panic erases the crash notice | The lock is forgotten while panicking, so it stays; later writes are SM07 until an operator breaks it. R20, §S9, criterion 15, a Preservation row |
| S3 self-asserted agents | `Origin` stated as advisory; R21; criterion 15 pins it; a Preservation row |
| S4 `AddSpec` | Every field named: keys for `title`, `carry` and `policy` with SMYSL's defaults; `params`, the pseudonym key and versification absent; `Caps::default()`; the file length checked before the read, SM02 with `SMY-E440`. Criterion 6 |
| S5 pins | Cited; `rand_chacha` corrected; `caseless` and `pulldown-cmark` added |
| S6 clock, hop and parents | `Hlc::now` from the agent's newest clock in the library; hop 0; a proof conclusion's parents are its grounds. R15, criterion 12; Q55 noted |
| S7 retraction | SMYSL's sequence step for step: already retracted is NODATA before authority; the edge unattested. Criterion 8 |
| S8 `"rests-on"` | Mapped through `trace_via` over `EdgeSet::premises`. Criterion 8 |
| N1, N2 | A non-MAP payload and an empty span are SM03. Criterion 14 |
| N3 | `observed` above `i64::MAX` reads back as a STRING |
| N4 | Normalisation stated; an empty body with a detail is SM02 |
| N5 | The preset is called `default`, as SMYSL calls it |
| N6 | Clamping stated |
| N7 | `PROMOTABLE.txt`'s names at its end |
| N8 | `FEATURE_GATED` moves to module scope; its comment is rewritten |
| N9 | The header notes D244's heading |
| N10 | SM11 cited |
| N11 | Q58 |
| N12 | §A's ranges cover the claims |
| Assumptions 1–6 | 1 is intended: the profiles are read under the locks at write time, so a gist fits the library as it then is. 2 is S6's clock. 3 is R20. 4 is S4's length check. 5 is criterion 5's documents. 6 is R21 |

## Revision 6: the fifth review answered

| Finding | Answer |
|---|---|
| B1 no `--break-lock` | The claim is withdrawn, and what exists is cited. R14 is re-decided: a `bund2 smysl-unlock` subcommand lists every lock's holder and, with `--yes`, breaks each through `Lock::break_lock`. Hand deletion is option B. U8 asks SMYSL for the command. Criterion 15 breaks every lock |
| S1 the open before the locks | Every write re-opens under the locks and decides from the second open; a shard that appears restarts the write once, then SM07. Q57 narrowed; criterion 15 |
| S2 NFC after the fit | The rendering and detail are NFC-normalised by `unicode-normalization` before fitting. Criterion 12 adds a non-NFC atom |
| S3 errors part way | Bund2 runs `check_loggable` first, so a whole-batch refusal keeps no lock; any later `append_records` error keeps the locks as a panic does. R20, criterion 15 |
| S4 salience and hop | Stated in §Current and §S4, with a Preservation row |
| N1 | `main.rs:4484-4505` |
| N2 | Q58 answered |
| N3 | SM12 covers a bound too small for `…` alone |
| N4 | `policy` is not a key in Phase 1; SMYSL's default always applies |
| N5 | `observed` is accepted as the same STRING it reads back as |
| N6 | The guard begins when `acquire_all` returns, as stated |
| N7 | A forgotten `Lock` leaks only its path and name |
| N8 | `SMY-W054` routed as a `Warning` |
| N9 | `Cargo.lock` named |
| Assumptions 1–5 | 1 is B1. 2 is S1. 3 is S2. 4 is S3. 5 is S4 |

## Revision 8: the sixth review answered

| Finding | Answer |
|---|---|
| B1 five SMYSL commands append unlocked | The claim that every SMYSL writer locks is withdrawn in §A, §Current and §S9, and the commands are listed with their lines. Bund2 promises serialisation against writers that lock, and nothing against the five, whose torn-tail hazard §S9 names. Q57 re-opened; U9 asked; R22 (D245) decides whether Phase 1 waits; a Preservation row |
| B2 criteria before their words | The plan is reordered: `smysl.create` and `smysl.unit` arrive with the locks in M3; criterion 4 is M3's; criterion 6 is M7's; 15's failed-append case is M4's and its attestation case M5's; the `smysl_demo` CI step lands in M10. Each criterion is claimed where every word it uses exists |
| B3 the probe cannot inject or pause | The failed append uses a read-only shard log, in a two-shard batch whose fixture is named. The probe gains a defined `pause`, released by the test, and the other writer is the test itself through SMYSL's API. Criterion 15, §S9 |
| S1 the lock is a hard link | §Current corrected; an `Io` from taking a lock is SM02; the filesystem assumption is stated, with a Preservation row. Q50 noted |
| S2 `lang` is required | Required, SM03 when missing; criteria 6 and 14 |
| S3 the guard and `Library::add` | R25 (D245), recommending that the guard compare the library's stamp before and after a failed call: grown keeps the locks, unchanged drops them. `check_loggable`-first is then unnecessary. Criterion 15 tests a refused `add` |
| S4 `retracts` outside `smysl.retract` | R23 (D245), recommending SM03; criteria 5 and 18; a Preservation row |
| S5 variants data can reach | An unknown source kind reads back as its code with `opaque`; extension relation kinds read back as text, and writing them is R24 (D245). `Error::internal` is kept for a variant a later pin brings |
| S6 the environment | §S5 names the scope, the `Env` and its ports, `PlanOptions::default()` and the base `Recipe`, each as SMYSL's commands build them. Criterion 10 uses them |
| S7 untested pieces | Criterion 18; criterion 14's over-cap case uses a sparse file |
| S8 the retraction edge's reason | Restated as the record `plan_retraction` simulates; the CLI's `retract` cannot write a library |
| N1 opens write | §Current, §S5, and a Preservation row |
| N2 layout raised | §Current and a Preservation row |
| N3 a shard no lock covers | §S9: the library lock covers it, as SMYSL intends |
| N4 Q51 | Narrowed |
| N5 primary shards | §Current; SM02 |
| N6 relation read-back | `note` is a uid STRING; `extra` sets `opaque` |
| N7 a gist of spaces | Criterion 14 |
| N8 `Authored` | Its reason is stated in §S4 |
| N9 the re-open clock | Criterion 15 says the other writer attests as the same agent |
| N10 uid shortening | A STRING `Uid::parse` accepts |
| N11 RFC-0013 conventions | The missing `lambda` and the unprefixed E00 and E40 are stated |
| N12 SM06's text | Described as following the house text's pattern |
| Assumptions 1–7 | 1 is B1 and R22. 2 is S1. 3 is S2. 4 is S3's stamp rule. 5 is S5. 6 is N5. 7 is stated in §S7, and criterion 12 checks it |
