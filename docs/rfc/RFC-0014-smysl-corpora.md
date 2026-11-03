# RFC-0014: SMYSL corpora — forming, researching and querying them from Bund

- Status: **Draft — revision 5** (2026-11-02). Written on the owner's
  request: "Create RFC for SMYSL project that will make it integrateable
  into Bund. The idea: use SMYSL ability to form, research and query SMYSL
  corpus/database of data. Before writing RFC, assess if this idea is
  reachable/worthy." The assessment is §A, before the design. The rulings
  are **D244, OPEN**, points R1–R21. D244's heading still reads "R1–R12",
  because it predates the amendments that added the rest. Nothing in this
  RFC takes the default of any of them.
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
| Form a corpus without a model | A library is created or opened by `Library::create` (idempotent) and `Library::open`. `Library::add` reads a text in, and its caller must hold the library's locks. `Library::append_records` appends records, and SMYSL's own commands hold the same locks around it | `smysl-text/src/library.rs:285-289,302,1001-1013,1341-1379`; `smysl:src/main.rs:8618-8628` |
| Research without a model | Lineage, retraction, salience, packing, dossiers and measures are pure functions of a store. A dossier and a measure return what they found and **write nothing** | `smysl-graph/src/lineage.rs:149`; `smysl-graph/src/merge/retraction.rs:70-127`; `smysl-query/src/dossier/mod.rs:325-326,819`; `smysl-measure/src/lib.rs:239-248` |
| Query | `parse`, `plan` and `run`, each answer one of five shapes | `smysl-query/src/parse.rs:42`; `smysl-query/src/plan.rs:680`; `smysl-query/src/exec.rs:70-83,115` |
| Query a library, not only a store | `smysl-text` implements the query crate's scope ports over an open library | `smysl-text/src/scope.rs:1-8` |
| No async runtime, HTTP client or argument parser in the pure set | SMYSL states it and checks it in CI | `smysl:README.md:34-35` |
| Values Bund can hold | Units, relations and diagnostics are structs of strings, sets of ids, enums and numbers. A payload decodes to a closed JSON-like tree, and the decode can fail | `smysl-core/src/types/unit.rs:25-58`; `smysl-core/src/types/relation.rs:175-186`; `smysl-core/src/diag.rs:528-536`; `smysl-core/src/surface/payload.rs:177`; `smysl-core/src/surface/hjson.rs:45-58` |
| Threads | `Store` is `Send + Sync`, which a test asserts | `smysl-graph/tests/store.rs:619-625` |
| A panic in SMYSL does not take Bund down | Every native call already runs under `catch_panic`, and a panic returns `Error::internal` (D49) | `catch_panic` in `crates/bund2-api/src/lib.rs` |
| Toolchain and licence | MSRV is 1.85 (1.88 for the facade); Bund2 pins 1.95.0. SMYSL is MPL-2.0, which is already on `deny.toml`'s allow list | `smysl:Cargo.toml:16-17,136`; `rust-toolchain.toml`; `deny.toml` |
| A source Bund2 may depend on | A git dependency pinned by revision is a mechanism Bund2 already uses, for `algos`. D38 justified that one by the oracle, which SMYSL is not, so allowing SMYSL's is R2's own ruling | `deny.toml`, `[sources]`; `crates/bund2-stdlib/Cargo.toml` |

Nothing here blocks Phase 1. Three things are still unknown, and they are
recorded as questions rather than blockers:

- what a library holds after an append interrupted part way (Q51);
- whether the pure `blake3` build invokes a C compiler (Q52);
- whether every type the words need can be built from outside SMYSL's
  crates (Q55);
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

    (`smysl:src/main.rs:4486-4505`.)
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
    with room for `suffix`. It answers `None` for an estimator the build
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
  - `append_records` appends records (`:1341`). It appends to each shard's
    log in a loop, then to the in-memory store (`:1364-1379`). Its doc
    comment names no lock, but SMYSL's commands take the locks around
    every write (`smysl:src/main.rs:8618-8628`).
- **The locks.**
  - `acquire_all` takes the library's locks, in order
    (`smysl-text/src/lock.rs:24-43`).
  - A lock is a file created with `create_new`. A second writer fails with
    `SMY-E445`, naming the holder (`smysl-graph/src/lock.rs:3-5`).
  - A lock is released when it is dropped: `Drop` removes the file
    (`smysl-graph/src/lock.rs:225-228`). A process killed mid-write leaves
    its lock behind by design: "the stale lock *is* the crash notice". The
    truncated tail such a crash leaves "is already tolerated by
    `from_cbor_seq`" (`:20-26`).
  - So a panic that unwinds past a held lock removes it, which a kill does
    not. That erases the notice SMYSL keeps for the case it calls
    interesting (`:21-23`).
  - `break_lock` exists, but "breaking a lock is an operator's decision"
    (`smysl-graph/src/lock.rs:188-193`).
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
- **Pins that must agree.** In the crates Phase 1 links, SMYSL pins four
  crates exactly:
  - `unicode-segmentation = "=1.13.3"` (`smysl:Cargo.toml:68`);
  - `caseless = "=0.2.2"` (`smysl:Cargo.toml:73`, a `smysl-text`
    dependency at `smysl-text/Cargo.toml:116`);
  - `pulldown-cmark = "=0.13.4"` (`smysl:Cargo.toml:80`), which
    `reader-md` turns on;
  - `libm = "=0.2.16"` (`smysl-measure/Cargo.toml:42`).

  `rand_chacha` is `"0.9"`, which is not an exact pin
  (`smysl-measure/Cargo.toml:45`).

  Bund2's lock has `unicode-segmentation` 1.13.3 and `libm` 0.2.16 today,
  and neither `caseless` nor `pulldown-cmark`. A Bund2 bump of the first
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
| a source | MAP: `kind`, `reference`, and, where present, `captured` (STRING, the date as SMYSL writes it), `observed` (INT of milliseconds; a value above `i64::MAX` reads back as a STRING of its decimal digits), `published`, `span` (LIST of two INTs) and `manifest` |
| a relation | MAP: `rid`, `kind`, `from` and `to`, and, where present, `weight` (FLOAT) and `note` |
| a diagnostic | MAP: `code` (SMYSL's own, such as `"SMY-E030"`), `severity`, `subject` and `message`, and, where present, `suggestion` |
| a subject | MAP `{ kind, value }`: `"unit"` with a uid; `"record"` with the hash in hex; `"span"` with LIST `[ start end ]` of byte offsets; or `"store"` with NODATA |
| a payload tree | `Null` → NODATA; `Bool`, `Int`, `Float` and `Str` as themselves; `Array` → LIST; `Object` → MAP |

- **A unit read back is a view, not the record.**
  - It leaves out the unit's `extra`, its source's `extra`
    (`smysl-core/src/types/epistemics.rs:403`), any payload that does not
    decode, and its attestations.
  - When any of the first three applies, the MAP has `opaque = true`, and
    `payload` is absent if it did not decode.
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
- **A SMYSL value Bund2 has no arm for** — a variant added to a
  `#[non_exhaustive]` enum after 4f5ecb9 — is `Error::internal`, naming
  the type (D37). It cannot occur at the pinned revision.

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
| `smysl.rel` | handle relation → rid | ✓ | appends one relation |
| `smysl.append` | handle list → LIST | ✓ | appends a LIST of unit, relation and label MAPs, in order and under one acquisition of the locks. Answers each one's id: a unit's uid, a relation's rid, and for a label, the uid it binds |
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
| `lang` | no | the reader's own answer |
| `title` | no | none |
| `carry` | no: `"none"`, `"ref"` or `"text"` | `"none"`, SMYSL's `Carry` default |
| `policy` | no | SMYSL's default: the reader's top level as the boundary (`library.rs:1176-1179`) |

- `params` is SMYSL's empty `Params`; the pseudonym key and the
  versification scheme are absent. None of the three is offered in
  Phase 1.
- `Caps` is `Caps::default()`.
- **A `path` is read by Bund2,** since a reader takes bytes. Bund2 checks
  the file's length against `Caps::default().input_bytes` first, and
  refuses a larger file with SM02, carrying `SMY-E440`, before reading
  it. A `text` is checked the same way.

**Attestation** [R15]. Every unit and relation Bund2 writes is followed by
one attestation, written as its own `Record::Attestation`. The retraction
edge is the exception (below). The attestation has these fields:

- its agent is the handle's `agent`;
- its operation is `Authored`;
- its rung is `Document` for a `cited` unit, and `Computed` for every
  other;
- its hop is 0;
- its parents are empty, except for a unit `smysl.prove` writes, whose
  parents are its grounds. So `smysl.trace … "parents"` walks a proof's
  conclusions back to their premises.
- **Its clock** is `Hlc::now(prev, agent)`.
  - `prev` is the newest clock among this agent's attestations in the
    library, read under the locks, or `Hlc::zero(agent)` if there is
    none.
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
(`smysl:src/main.rs:4486-4505`), step for step:

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

The edge's lack of an attestation is SMYSL's choice, kept so that a
Bund2 retraction and a CLI retraction are the same records. Attesting it
would be an amendment to R15.

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

None of these writes.

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

- a missing slot, or a slot of the wrong type, is SM03, naming the slot;
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
  `SMYSL.PROVE: `.
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
  - This is judged **at write time**, against the library as it is when
    the locks are held. A unit a bridge wrote while the proof ran
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

**The profiles a gist must fit** are read under the locks, at write time:
- every distinct granularity profile among the library's views;
- the `default` preset if the library has no view.

That set includes every profile SMYSL's `check` can derive (§Current,
"Granularity"). A gist that fits each of them also fits their widest
envelope, which counts with the first view's estimator against the
largest `l0_max`.

- **gist:** §S7.1's rendering of the goal, with every uid shortened to its
  first 12 characters.
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
  - If a profile leaves room for no character of the rendering, the write
    is SM12: `the library's view allows no gist of <n> tokens`. Nothing is
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
| SM02 | SMYSL refused the operation | `SMYSL refused it: <code> <message>` |
| SM03 | an operand of the wrong type (`#<k> parameter must be <kind>, found <type>`); a spec, record or slot lacking a required key or holding one of the wrong type; a value with no SMYSL form; a computed key given; a status Bund2 cannot author | `<what> <reason>`, e.g. `uid is computed, not given` |
| SM04 | `sq/1` refused the query | `the query was refused: <code> <message>`, one per diagnostic, joined by `; ` |
| SM05 | an unknown status, relation kind, reader or trace kind | `unknown <kind>: <value>` |
| SM06 | `--noio`, for a word that acts on disk [R11] | the house text, unprefixed, as `bund_models.rs:125` writes it: `bund SMYSL functions disabled with --noio` |
| SM07 | a lock is held (`SMY-E445`) | `the library is locked: <holder as SMYSL names it>` |
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
  are each reported as a `Warning` (D36). A measure's diagnostics are also
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

**Locks.** Every write follows the order SMYSL's own commands use
(`smysl:src/main.rs:8618-8628`):

1. open the library;
2. read its shard names (`Library::shards`);
3. take the library lock, then the shard locks in name order, through
   `acquire_all` (`smysl-text/src/lock.rs:24-43`);
4. write;
5. drop the locks before the word returns.

A process that appends between steps 1 and 3 is a race SMYSL's own commands
share. It is recorded as Q57, not designed around.

- **A lock held by another process is SM07.** It is not waited for. SMYSL
  fails fast, and a program that wants to retry can catch SM07 with
  `?try`.
- **A stale lock stays SM07.** A lock left behind by a crashed process
  stays SM07 until an operator breaks it with SMYSL's `--break-lock`.
  Bund2 offers no word that breaks one [R14]. SMYSL says why: "breaking a
  lock is an operator's decision" (`smysl-graph/src/lock.rs:188-193`).
- **A panic inside a write leaves SMYSL's lock in place** [R20], as a
  killed SMYSL process would. That keeps SMYSL's crash notice.
  - Bund2 holds the locks `acquire_all` answers in a guard of its own.
    When that guard is dropped while the thread is panicking
    (`std::thread::panicking()`), it `std::mem::forget`s each lock instead
    of dropping it. `Lock`'s `Drop`, which would remove the file
    (`smysl-graph/src/lock.rs:225-228`), then never runs.
  - The panic still unwinds through `catch_panic` and is reported as
    `Error::internal` (D49).
  - Every later write to that library, from this process or any other,
    is SM07 naming this process as the holder, until an operator has read
    the library and runs SMYSL's `--break-lock` (R14). Reads are not
    locked and still succeed.
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
  `smysl-panic-probe`, on no default and in no shipped build, makes a
  write panic after its mutex and SMYSL's locks are taken, and before it
  appends, as `panic-probe` gives `test.panic` (D220).
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
embedder. Only U6 bears on Phase 1, through R2.

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

### S12. Phase 2, not designed here

The steps that call a model are extracting units from a text
(`ingest_text`) and answering a dossier (`smysl-ingest/src/answer.rs:1-3`).

- Both need `smysl-provider`, which links `tokio`
  (`smysl-provider/Cargo.toml:39`).
- Bund2 links no async runtime of that kind today.

Whether it ever should, and behind which feature, is R1. If R1 rules
Phase 2 in, it comes as an amendment with its own criteria.

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
| SMYSL's lock as a crash notice | **Kept** [R20]. A panic in a write leaves the lock, as a killed process does, so the library refuses writes until an operator breaks it |
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
     - relations of each of SMYSL's fourteen kernel kinds;
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
6. **Forming persists.** A test:
   - creates a library in a temporary directory;
   - adds one text per alias with each enabled reader;
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
      what the direct calls give.
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
        multi-byte character near the cut.
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
    - an empty span, a non-MAP payload, and an over-cap `text` for
      `smysl.text.add`, which is SM02 with `SMY-E440`;
    - a unit MAP with `body: ""` and a `detail`, which is SM02.
15. **Locks are SMYSL's.**
    - With a lock file left as a crashed SMYSL process would leave it, a
      write is SM07 naming the holder, and a read succeeds.
    - Four VMs on four threads each append 250 units to one library,
      through two spellings of its root. All 1,000 are present afterwards.
    - Under `smysl-panic-probe`, a write that panics with the mutex and
      the locks held:
      - is reported as `Error::internal`;
      - leaves the library's lock file in place;
      - makes the next write to that root, from a fresh VM on another
        thread, SM07 naming this process;
      - leaves a read of that root succeeding.

      After the test breaks the lock with SMYSL's `Lock::break_lock`, a
      write succeeds.
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

---

## Open questions

Each is in `docs/registers/open-questions.md`.

- **Q50. Answered by revision 2.** Libraries are locked by SMYSL, and
  Phase 1 writes only libraries.
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
  - If that can leave a library half-written, a repair step is an
    amendment.
- **Q52 [UNGROUNDED].** Does `blake3` with `pure` and no default features
  invoke a C compiler at build time? `cc` appears beneath it in SMYSL's
  lock file. Criterion 3 tests it, under R18.
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
  - The question is answered by a spike before M1, or by U2.
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
- **Q58.** Does a library that binds labels draw closure diagnostics from
  `smysl.check`?
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

## Rulings asked for (D244)

| | Point | Options | Recommended |
|---|---|---|---|
| R1 | Scope | **A** Phase 1 as designed; Phase 2 a later amendment. **B** the bridge alone (§S7). **C** no integration. **D** Phases 1 and 2 together | **A** |
| R2 | Dependency source | **A** git at `rev = "4f5ecb9"`, with an `allow-git` entry ruled here (D38's mechanism, not its reason), moving to crates.io when U6 lands. **B** crates.io `=1.11.0`, with this RFC re-grounded against it. **C** wait for U6 | **A** |
| R3 | Handle | **A** a MAP of `type = "smysl"` with a canonical root and an agent. **B** a STRING path. **C** a new native value type | **A** |
| R4 | Opening | **A** per call, no cache, until Q54 asks for one. **B** per call, with a process-wide cache | **A** |
| R5 | Writes | **A** each word appends, and `smysl.append` appends a LIST under one acquisition of the locks. **B** single-record words only | **A** |
| R6 | Queries | **A** `sq/1` text as written. **B** queries built from Bund values | **A** |
| R7 | The fact schema | **A** §S7's table. **B** one fact per unit, with every field as an argument | **A** |
| R8 | Exported proofs | **A** §S7: provenance by shape and library; the node table; `derived` capped by declared statuses; `else`, `not` and `tabled` capping at `inferred`; no grounds meaning `speculative`; gists fitted by `fit_gist_with` to every profile of the library's views, SM12 when none fits, and the full goal in the detail; §S7.1's rendering; solve, then write. **B** refuse any proof that holds a program fact or a capping node | **A** |
| R9 | Names | **A** the `smysl.` prefix and the conditional `smysl`. **B** a `corpus.` prefix | **A** |
| R10 | Upstream | **A** U1–U5 and U7 do not block Phase 1. **B** Phase 1 waits for U1 and U2 | **A** |
| R11 | `--noio` | **A** every word that touches disk is stubbed; `smysl.parse` and `smysl.agent` are not. **B** reads allowed, writes stubbed | **A** |
| R12 | Readers | **A** `txt/1` and `md/1`. **B** every reader SMYSL has. **C** `txt/1` alone | **A** |
| R13 | Writers in one process | **A** serialised by a process-wide table of mutexes keyed by canonical root, a poisoned mutex recovered. **B** SM07, as between processes | **A** |
| R14 | A stale lock | **A** SM07 until an operator breaks it with SMYSL's `--break-lock`. **B** a `smysl.unlock` word | **A** |
| R15 | Attestation | **A** the handle's `agent` (default `"tool:bund2"`), `Authored`, rung `Document` for `cited` and `Computed` otherwise, hop 0, parents empty except a proof's conclusions, whose parents are their grounds, and a clock that is `Hlc::now` from this agent's newest clock in the library. Each is written as its own record; `measured` is refused; no attestation is written where this agent has already attested the uid; the retraction edge is unattested, as SMYSL writes it. **B** no attestation | **A** |
| R16 | `.` forms | **A** none in Phase 1. **B** a form for `smysl.parse` | **A** |
| R17 | Dossiers and measures | **A** answered, not saved; saving is an amendment. **B** saved by default. **C** a `save` key | **A** |
| R18 | No C compiler | **A** the feature must build without one (criterion 3), a rule this RFC sets. **B** no such rule; the feature may compile C, as `grok` does | **A** |
| R19 | `smysl.check`'s profile | **A** `CheckOptions::default()`: the profile SMYSL derives from the library's views, or the `default` preset without one. **B** always the `default` preset | **A** |
| R20 | A panic in a write | **A** SMYSL's lock is left in place (forgotten, not dropped), so the library refuses writes with SM07 until an operator breaks it. **B** the lock is released and the next write proceeds | **A** |
| R21 | Agent ids | **A** any agent SMYSL's grammar allows; `Origin` authority stated as advisory, since ids are self-asserted. **B** `smysl.agent` and handles restricted to the `tool:` kind | **A** |

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
