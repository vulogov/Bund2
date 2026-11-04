# RFC-0014: SMYSL corpora — forming, researching and querying them from Bund

- Status: **Draft — revision 21** (2026-11-04). Written on the owner's
  request: "Create RFC for SMYSL project that will make it integrateable
  into Bund. The idea: use SMYSL ability to form, research and query SMYSL
  corpus/database of data. Before writing RFC, assess if this idea is
  reachable/worthy." The assessment is §A, before the design. The rulings
  are **D244**, points R1–R21, with R14 re-decided by revision 6, and
  **D245**, points R22–R25, raised by revision 8. **D245 is RESOLVED
  (2026-11-02):** the owner ruled R22–R25 A, as the design assumes.
  **Amended 2026-11-03** on the owner's word, after the seventh review:
  R22's list of SMYSL's unlocked commands is corrected, and R25's test
  compares the shard logs' lengths on disk instead of the stamp, with its
  intent unchanged. **Amended again 2026-11-03** on the owner's word, after
  the eighth review: R25's test is made exact for a log that held a torn
  tail, by comparing that tail's bytes.
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
  - *Revision 9 (2026-11-03)* answers the seventh review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-02-7.md`): S1–S6, N1–N10
    and its assumptions, in "Revision 9: the seventh review answered". The
    largest changes:
    - **R25's stamp does not show that nothing reached a log.** The stamp
      is computed over in-memory lengths, which move only after a write
      and its `sync_data` both succeed. So a short write or a failed sync
      can leave whole records on disk with the stamp unchanged. R25 is
      amended, on the owner's word, to compare each shard log's length on
      disk.
    - **R22's list was wrong.** `measure` has no `--save` and writes
      unless given `--dry-run`, and `ingest --text` also appends unlocked.
      Six commands, not five; R22 is otherwise unchanged.
    - Under `--noio`, the `smysl` conditional's handler is stubbed too, as
      D119 does for `csv` and `sqlite`.
    - A measure's `Input` is named; the probe-gated tests get a CI step;
      the re-open case moves to M4, where `smysl.append` exists.
  - *Revision 10 (2026-11-03)* answers the eighth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-03.md`): S1–S3, N1–N12 and
    its assumptions, in "Revision 10: the eighth review answered", on the
    owner's "A" to each of S1–S3. The largest changes:
    - **R25 is exact.** Revision 9's torn-tail clause compared totals, so
      over a library with a torn tail anywhere, a refusal that wrote
      nothing kept every lock. Now each torn log's tail is snapshotted and
      compared byte for byte, the marker is compared too, and the
      snapshot is taken once per acquisition of the locks.
    - **`smysl.create` runs `Library::create` outside SMYSL's locks**, as
      SMYSL's own commands do, since it appends nothing. U10 asks SMYSL to
      write the marker atomically.
    - **SM02's text is SMYSL's `Display`**, which carries the code when
      there is one, with a fixed suffix when the locks are kept.
  - *Revision 11 (2026-11-03)* answers the ninth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-03-2.md`): S1–S4, N1–N9 and
    its assumptions, in "Revision 11: the ninth review answered", on the
    owner's "go ahead". No ruling changes. A held lock is SM07 only;
    `smysl.create` canonicalises before it takes the mutex; the label MAP
    is defined; every new criterion case is placed in a milestone.
  - *Revision 12 (2026-11-03)* answers the tenth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-03-3.md`): S1–S4, N1–N9 and
    its assumptions, in "Revision 12: the tenth review answered", on the
    owner's "go ahead". No ruling changes. `smysl.parse` takes labels from
    SMYSL's `LabelBinding` records only, and refuses a document SMYSL could
    not parse whole; `smysl.append` decides a MAP's kind by its keys; the
    `smysl` constructor declares `Leaves::Conditional("smysl")` (D181);
    each `--noio` stub arrives with its word.
  - *Revision 13 (2026-11-03)* answers the eleventh review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-03-4.md`): S1, S2, N1–N14
    and its assumptions, in "Revision 13: the eleventh review answered",
    on the owner's "go ahead". No ruling changes. `smysl.write` reports
    what SMYSL's writer omits, and writes a label bound to several units
    on none of them; `smysl.agent` does not canonicalise; `smysl.parse`
    reports dropped comments and salience; each milestone tests its own
    `--noio` stubs.
  - *Revision 14 (2026-11-03)* answers the twelfth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-03-5.md`): S1–S3, N1–N8 and
    its assumptions, in "Revision 14: the twelfth review answered", on the
    owner's "go ahead". No ruling changes. An edge re-written with another
    weight or note is SM03; `smysl.write` builds its labels from the units
    it writes, so a reference to an omitted unit is its uid; "already
    retracted" is SMYSL's `is_retracted`; a measure's `value` is SMYSL's
    public `gist`.
  - *Revision 15 (2026-11-03)* answers the thirteenth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-03-6.md`): S1, S2, N1–N11
    and its assumptions, in "Revision 15: the thirteenth review
    answered", on the owner's "go". No ruling changes; D244 carries a
    dated note on R3. `smysl.write` passes every record but attestations,
    so a withdrawn edge is written with its withdrawal; `smysl.parse`
    leaves out an edge its document withdraws; `smysl.kb` loads no
    withdrawn edge; a handle's root must be absolute.
  - *Revision 16 (2026-11-04)* answers the fourteenth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-03-7.md`): S1, N1–N10 and
    its assumptions, in "Revision 16: the fourteenth review answered", on
    the owner's "go". No ruling changes. Every withdrawn-edge rule is
    SMYSL's `is_withdrawn`, so a `supersedes` or `retracts` edge is never
    withdrawn; a withdrawal `smysl.write` cannot spell is named; a `rel`
    fact stands for its units only over a live edge.
  - *Revision 17 (2026-11-04)* answers the fifteenth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-04.md`): S1, N1–N5 and its
    assumptions, in "Revision 17: the fifteenth review answered", on the
    owner's "go". No ruling changes. `smysl lib import` is named as a
    SMYSL command that withdraws an edge in a library; `smysl.write` names
    an edge only when it reads back live; every fact shape stands for its
    units only when the library bears it out.
  - *Revision 18 (2026-11-04)* answers the sixteenth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-04-2.md`): S1, N1–N4 and
    its assumptions, in "Revision 18: the sixteenth review answered", on
    the owner's "go ahead". No ruling changes; D244 carries a dated note
    on R8. A fact stands for its units by a test per shape, with a
    superseded unit the library holds counting as held; `smysl.kb`
    resolves its scope in §S5's environment.
  - *Revision 19 (2026-11-04)* answers the seventeenth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-04-3.md`): S1, N1–N5 and
    its assumptions, in "Revision 19: the seventeenth review answered",
    on the owner's "go ahead". No ruling changes; D244 carries a further
    dated note on R8. Only the units a fact stands for must be held, so a
    ground or dependency the library does not hold leaves a `grounds` or
    `deps` fact standing; `smysl.kb` resolves its scope with
    `scope::resolve`; a ranked query in a library with a supersession
    answers a scope.
  - *Revision 20 (2026-11-04)* answers the eighteenth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-04-4.md`): S1–S3, N1–N4
    and its assumptions, in "Revision 20: the eighteenth review
    answered", on the owner's "follow that plan". No ruling changes; D244
    carries a further dated note on R8. A packed query keeps its kind; an
    `effective` fact can lapse when an absent ground is appended;
    `smysl.kb` runs `parse`, `plan` and `run` itself and reports `run`'s
    warnings; a uid that is not a held unit is left out of the scope.
  - *Revision 21 (2026-11-04)* answers the nineteenth review
    (`docs/rfc/reviews/RFC-0014-review-2026-11-04-5.md`): S1, S2, N1–N4,
    in "Revision 21: the nineteenth review answered", on the owner's
    "go". No ruling changes. An answer's kind is the one `run` gives,
    with no list of the stages that change it; the `smysl` conditional
    skips a uid with no unit.
- Depends on: RFC-0001 (values), RFC-0002 (words), RFC-0004 (effects),
  RFC-0005 (promotion), RFC-0007 (threads and the bus), RFC-0013
  (knowledge bases, proof trees)
- Decisions consumed: D14, D18, D26, D27, D28, D32, D36, D37, D38, D49,
  D51, D72, D119, D168, D181, D220, D226 (L11), D230
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
| Form a corpus without a model | A library is created or opened by `Library::create` (idempotent) and `Library::open`. `Library::add` reads a text in, and its caller must hold the library's locks. `Library::append_records` appends records. SMYSL's `lib` and `text` commands hold the locks around it, and six of its commands that save do not (§Current, "The locks"; R22) | `smysl-text/src/library.rs:285-289,302,1001-1013,1341-1379`; `smysl:src/main.rs:6297,6324,8618-8628` |
| Research without a model | Lineage, retraction, salience, packing, dossiers and measures are pure functions of a store. A dossier and a measure return what they found and **write nothing** | `smysl-graph/src/lineage.rs:149`; `smysl-graph/src/merge/retraction.rs:70-127`; `smysl-query/src/dossier/mod.rs:325-326,819`; `smysl-measure/src/lib.rs:239-248` |
| Query | `parse`, `plan` and `run`, each answer one of five shapes | `smysl-query/src/parse.rs:42`; `smysl-query/src/plan.rs:680`; `smysl-query/src/exec.rs:70-83,115` |
| Query a library, not only a store | `smysl-text` implements the query crate's scope ports over an open library | `smysl-text/src/scope.rs:1-8` |
| No async runtime, HTTP client or argument parser in the pure set | SMYSL states it and checks it in CI | `smysl:README.md:34-35` |
| Values Bund can hold | Units, relations and diagnostics are structs of strings, sets of ids, enums and numbers. A payload decodes to a closed JSON-like tree, and the decode can fail | `smysl-core/src/types/unit.rs:25-58`; `smysl-core/src/types/relation.rs:175-186`; `smysl-core/src/diag.rs:528-536`; `smysl-core/src/surface/payload.rs:177`; `smysl-core/src/surface/hjson.rs:45-58` |
| Threads | `Store` is `Send + Sync`, which a test asserts | `smysl-graph/tests/store.rs:619-625` |
| A panic in SMYSL does not take Bund down | Every native call already runs under `catch_panic`, and a panic returns `Error::internal` (D49) | `catch_panic` in `crates/bund2-api/src/lib.rs` |
| Toolchain and licence | MSRV is 1.85 (1.88 for the facade); Bund2 pins 1.95.0. SMYSL is MPL-2.0, which is already on `deny.toml`'s allow list | `smysl:Cargo.toml:16-17,136`; `rust-toolchain.toml`; `deny.toml` |
| A source Bund2 may depend on | A git dependency pinned by revision is a mechanism Bund2 already uses, for `algos`. D38 justified that one by the oracle, which SMYSL is not, so allowing SMYSL's is R2's own ruling | `deny.toml`, `[sources]`; `crates/bund2-stdlib/Cargo.toml` |

Nothing here blocks Phase 1. R22 is ruled A, so it does not wait for SMYSL
to lock its commands that save (U9). These things are still unknown, and are
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
    1. does nothing, and says so, when the target was retracted
       directly, `effective_status(…).is_retracted` (`smysl:src/main.rs:4484`).
       It checks this before authority.
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
  - **A relation's rid is its kind and its two ends alone.** `Relation::uid`
    hashes nothing else (`:231-238`), so the weight, the note and `extra`
    are outside its identity. The store files a relation under
    `(kind, from, to)` (`smysl-graph/src/store/interp.rs:38-40`), keeps
    the first record it holds under that key and only unions a later one's
    attestations into it (`:47-58`), and every read goes through that
    record (`relation_by_id`, `smysl-graph/src/store/mod.rs:1520-1526`).
    The log skips a record only when its bytes are already there
    (`smysl-text/src/shards.rs:516-522`), so the same edge with another
    weight is appended, and no interpreting read sees it: not
    `relation_by_id`, `Store::relations` or the adjacency. `Store::iter`
    still carries it (`smysl-graph/src/store/mod.rs:536-538`), and so
    does what SMYSL's `merge` writes from it (`smysl:src/main.rs:4327`).
  - **Which variant the store keeps** of an edge a library holds twice
    is whichever its open reads first: `absorb_hot` keeps the first
    (`smysl-graph/src/store/interp.rs:47-58`), `absorb_hashed` feeds it
    records in log order (`smysl-graph/src/store/mod.rs:1149-1166`), and
    the open reads the shards in name order (`smysl-text/src/library.rs:327-350`).
    Bund2 never writes a second (§S4), but a SMYSL-written library can
    hold one, and then §S4's "held with another weight" test and
    `smysl.write` read the variant that order gives.
  - **A withdrawn relation** is kept, and no longer followed: "nothing
    interpreting the graph follows it" (`smysl-core/src/types/lifecycle.rs:18-22`).
    A `supersedes` or `retracts` edge cannot be withdrawn: a withdrawal of
    one is kept and ignored, and the edge goes on applying
    (`smysl-core/src/types/lifecycle.rs:22`; `SMY-W056`,
    `smysl-check/src/passes/integrity.rs:40-44`). A withdrawal may name a
    rid before its relation arrives (`smysl-graph/src/store/interp.rs:31-32`).
    `is_withdrawn` tests both: a relation is withdrawn when it is not a
    lifecycle edge (`is_lifecycle`, `smysl-core/src/types/relation.rs:157-160`)
    and a withdrawal names its rid (`smysl-graph/src/store/mod.rs:1500-1504`),
    and SMYSL's interpreting reads leave it out: the adjacency every
    traversal reads (`:1276-1286`), `relations_of_kind` (`:1573-1580`),
    live rebuttals (`:1506-1513`) and propositions
    (`smysl-graph/src/proposition.rs:140-153`). `Store::relations` does
    not (`smysl-graph/src/store/mod.rs:554-556`). SMYSL's `withdraw`
    command writes a withdrawal to a store file, as `retract` does
    (`smysl:src/main.rs:170,4896,5015-5016`), not to a library, and
    refuses a lifecycle edge (`:4916-4923`). A library gains withdrawals
    through SMYSL's API, and through `smysl lib import` of a document
    holding `@withdraw`. That command opens the library under its locks
    (`with_library`, `smysl:src/main.rs:8618-8628,8785-8787`) and decodes
    a surface document with SMYSL's parser
    (`smysl-lib/src/import/decode.rs:143-150`), which makes the withdrawal
    a `Record::Withdrawal` (`smysl-core/src/surface/parse.rs:2674-2679`).
    The import classes it by the rid it names
    (`smysl-lib/src/import/classify.rs:95-99`) and appends what it accepts
    to the library (`smysl-lib/src/import/mod.rs:754`;
    `smysl-lib/src/access.rs:176-193`): at once with `--accept`
    (`smysl-lib/src/import/mod.rs:467`), otherwise when `smysl lib accept`
    accepts the staged import (`smysl:src/main.rs:8962-8970`;
    `smysl-lib/src/import/mod.rs:824,855`). So a text `smysl.write` wrote,
    imported so, withdraws its edges in that library. The criteria build
    their withdrawals through the API.
- **A weight** is clamped to [0, 1] and quantised to 1/1024
  (`smysl-core/src/types/relation.rs:179-181,205-208`), and a float in a
  payload is quantised too (`smysl-core/src/surface/payload.rs:253-257`).
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
    (`smysl-text/src/library.rs:327-350`), and nothing reloads it afterwards.
  - `Library::open` also writes. When the library records redactions, it
    unlinks the objects of the redacted parts (`enforce_z`,
    `smysl-text/src/library.rs:370,676-698`), and takes no lock to do it. Every open does this,
    SMYSL's own included.
  - `append_records` appends records (`smysl-text/src/library.rs:1341`). It refuses a batch
    `check_loggable` refuses before any shard is written (`smysl-text/src/library.rs:1347-1354`).
    It appends to each shard's log in a loop, then to the in-memory store
    (`smysl-text/src/library.rs:1364-1379`). Its doc comment names no lock; who locks is under
    "The locks".
    - A unit citing a part is routed to the shard of that part's alias
      (`smysl-text/src/shards.rs:456-466`), and the loop creates that
      shard's log if it is new (`smysl-text/src/library.rs:1364-1369`).
    - In a layout-1 library, a batch that creates a shard first rewrites
      the `LIBRARY` marker to layout 2 (`:1355-1361`). Layout 2 dates from
      SMYSL 1.10.0 (`smysl:CHANGELOG.md:958,1323-1325`), so releases
      before it no longer read the library whole.
    - A shard log appends a shard's whole batch with one `write_all`,
      then `sync_data`, and only then adds to its in-memory length and
      hash (`smysl-text/src/shards.rs:380-384`). The library's stamp is
      computed over those in-memory fields (`smysl-text/src/library.rs:993-995`;
      `smysl-text/src/shards.rs:395-416`), not over the files. So a
      `write_all` that fails part way, or a `sync_data` that fails, can
      leave whole records of the batch in the file with the stamp
      unchanged. A shard's batch is every record routed to it,
      concatenated (`smysl-text/src/shards.rs:516-524`).
    - A log's path is public: `shards::log_path` gives it relative to the
      root (`smysl-text/src/shards.rs:65-71`). The catalog's log is
      `log/catalog.cbor` (`smysl-text/src/library.rs:77`), and every
      other shard's is under `log/shards` (`smysl-text/src/shards.rs:42`).
    - Reading a log counts a truncated tail as torn
      (`smysl-text/src/shards.rs:236-251`): `from_cbor_seq` stops at a
      truncated record and returns any other decode error
      (`smysl-core/src/cbor/envelope.rs:2015-2028`), which fails the open. The next append to that log
      cuts it back to the decoded length before writing (`smysl-text/src/shards.rs:376-379`).
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
    subcommands (`:12656,12790,12914,13033,13104`). These six commands
    call `append_records` on a library they opened, and never lock:

    | Command | Opens | Appends |
    |---|---|---|
    | `q --library --save` | `smysl:src/main.rs:6297` | `:6324` |
    | `ask --save` | `Corpus::open`, `:13827` | `:6702` |
    | `measure`, unless `--dry-run` | `:7348` | `:7579` |
    | `assess`, unless `--dry-run` | `:7780` | `:8094` |
    | `same-as` | `Corpus::open` | `:14019`, `:14162` |
    | `ingest --text` | `Library::open`, `:10803` | each window, `smysl-ingest/src/text/run.rs:546,635` |

    `measure` has no `--save`: its one write flag is `--dry-run`, "Print
    the values and refusals and write nothing" (`smysl:src/main.rs:1163-1167`),
    and without it the command appends (`:7536-7539,7579`). `assess` has
    the same flag (`:1245-1249`). `ingest --text` builds a `TextIngest`
    over the library it opened (`:10947-10948`) and commits window after
    window across a model run.

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
- **CI** gains seven steps in `.github/workflows/ci.yml`:
  - in the `check` job, where suites run under nextest with a time bound
    on each test (F141):
    - `cargo nextest run -p bund2-stdlib --features smysl --profile ci`;
    - `cargo nextest run -p bund2-stdlib --features smysl,smysl-panic-probe
      --profile ci`, which adds the probe-gated tests of §S9, as D220's
      step "a panic under the tier, reported" runs `test.panic`'s;
    - `cargo nextest run -p bund2-cli --features smysl --test smysl_demo
      --profile ci`;
  - `cargo clippy -p bund2-stdlib -p bund2-cli --all-targets --features
    smysl`, so that the denied `unwrap`, `expect` and `panic` lints see
    the feature's code;
  - `cargo clippy -p bund2-stdlib --all-targets --features
    smysl,smysl-panic-probe`, so that they see the probe's code too. Its
    one `panic!` carries an `#[expect(clippy::panic, reason = …)]` naming
    this RFC, as `test.panic`'s `panic_probe` does (`crates/bund2-runtime/src/lib.rs`);
  - `--features smysl` in the `feature matrix` step's `cargo check`, and
    `cargo build -p bund2-cli --features smysl` with `CC` and `CXX` set
    to `false` (criterion 3);
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
- **Its `root` is canonicalised** (`std::fs::canonicalize`; a failure is
  SM01) when
  `smysl.create` or `smysl.open` builds the handle. A hand-built handle's
  root must be absolute, or it is SM00; every word checks this without
  I/O (`Path::is_absolute`). It may still be another spelling of the
  directory, which `smysl.agent` passes on unchanged, and the words that
  open it canonicalise it again (below). So a root is canonical when
  Bund2 built it and absolute always, which is what R3 needs of it
  (D244, note of 2026-11-03).
  - `smysl.create` first makes the root's directory
    (`std::fs::create_dir_all`), because a path that does not exist yet
    has no canonical form. It then canonicalises, takes §S9's mutex under
    that canonical root, and only then calls `Library::create`, which
    begins with the same `create_dir_all` and tolerates the directory
    (`smysl-text/src/library.rs:289`). A failed `create_dir_all` is SM01,
    with the I/O error.
  - **Every word that opens the library or takes §S9's mutex
    canonicalises the root it is given again**, before it does either, so
    a hand-built handle with another spelling of the same directory still
    finds the same mutex.
  - **`smysl.agent` does neither.** It validates the handle (SM00) and
    answers it with another `agent`, without canonicalising the root or
    asking whether it exists. So it does no I/O [R11], and it works on a
    handle for a library not yet created.
  - A handle saved by `save.model` names the same library from any working
    directory, since its root is absolute.
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
  STRING `root` that is an absolute path, and an `agent` matching SMYSL's grammar
  (`smysl-core/src/ids.rs:521`). A hand-built handle with a malformed
  `agent` is SM00.

**Records read back as MAPs:**

| SMYSL | Bund value |
|---|---|
| a uid | STRING, its full text form `b3:…` |
| a status | STRING, `"unfounded"` … `"measured"` |
| a unit | MAP: `uid`, `schema` and `gist`, and, where present, `body`, `detail`, `status`, `grounds` (LIST of uid), `deps` (LIST of uid), `source`, `lang`, `labels` (LIST of STRING), `payload` (the decoded tree) and `opaque` (BOOL, see below) |
| a source | MAP: `kind`, `reference`, and, where present, `captured` (STRING, the date as SMYSL writes it), `observed` (INT of milliseconds; a value above `i64::MAX` reads back as a STRING of its decimal digits, and is written from one, so the two forms round-trip; any other STRING is SM03), `published`, `span` (LIST of two INTs) and `manifest` |
| a label | MAP: `label` (STRING) and `uid` (STRING, the unit it binds). SMYSL's record is `LabelBinding { label, uid, extra }` (`smysl-core/src/types/annex.rs:488-493`). This is the one MAP whose `uid` is given rather than computed, because a binding names a unit and is not identified by one. It has `opaque` only when read back with a non-empty `extra`, as `{ label, uid, opaque = true }`; a binding SMYSL or Bund2 writes has an empty `extra` |
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
  out, and a non-empty `extra` sets `opaque = true`. No Phase 1 word
  answers a relation MAP read from a library: `smysl.get` answers units,
  and `smysl.parse` reads text, which has no `extra`. So this mapping is
  reached through the function criterion 18 tests, and is there for the
  words a later phase adds.
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
  - A payload key named `status`, `deps`, `grounds`, `source` or
    `salience` is accepted, as SMYSL's API accepts it. Such a unit has no
    surface form, since the key cannot be told from the header field of
    that name when the text is read back
    (`smysl-core/src/surface/write.rs:248-252,267-295`), so `smysl.write`
    leaves it out and says so (§S4).
  - SMYSL NFC-normalises the gist, body and detail, and drops an empty
    body or detail. So a unit read back may differ from the MAP written in
    those three fields. A MAP with `body: ""` and a `detail` is SMYSL's
    "detail without a body", which is SM02.
  - An empty `payload` MAP is no payload: SMYSL encodes an empty object as
    none, so that one core has one encoding
    (`smysl-core/src/surface/payload.rs:30-36`). So `payload: {}` gives
    the uid of the same unit written without `payload`, and reads back
    without the key.
- **Labels in `grounds` or `deps`** are resolved to uids before the unit is
  built, against the library as re-opened under the locks and the label
  MAPs earlier in the same `smysl.append` LIST. A label bound to several
  units, or to none (`LabelError::Unbound`, `smysl-graph/src/labels.rs:20-29`),
  is SM08 (`:3-8`).
  A STRING in `grounds` or `deps` is a uid when `Uid::parse` accepts it,
  and a label when SMYSL's label syntax accepts it (`<ns>/<name>`,
  `Label::new` and `is_ident`, `smysl-core/src/ids.rs:442-466`); a uid begins `b3:`, so no STRING is
  both. One that is neither is SM03.
- **A unit MAP's `labels` key is read-back only.** On write it is SM03,
  naming `smysl.label` and the label MAP, which are how a label is bound.
  `smysl.parse` gives its unit MAPs no `labels` key, since their labels
  travel as label MAPs.
- **A MAP's kind is decided by its keys**, in this order, wherever a
  record MAP is written (`smysl.append`'s LIST, and the operand of
  `smysl.unit` and `smysl.rel`):
  1. a MAP with `label` is a label MAP;
  2. otherwise, one with `kind`, `from` and `to` is a relation MAP;
  3. otherwise, one with `gist` is a unit MAP;
  4. anything else is SM03, `record is not a unit, relation or label`.

  A MAP decided as one kind and given to a word that takes another
  (`smysl.unit` given a relation) is SM03 naming both. Within its kind a
  MAP is then checked as §S2 says, so a label MAP that also carries
  `gist` is SM03 for the key it should not have.
  - **A key its kind does not have is SM03** on every kind, naming the
    key: a label MAP has `label` and `uid`; a relation and a unit MAP have
    the keys the table above names for them.
  - **Step 4 names what is missing** when it can. A MAP without `label`
    that holds one or two of `kind`, `from` and `to` is SM03 `relation
    lacks <key>`; one that holds `schema` but no `gist` is SM03 `unit
    lacks gist`. Only a MAP with none of these keys has step 4's text.
- **A label MAP's `uid` must name a unit** in the library as re-opened
  under the locks, or a unit earlier in the same LIST; otherwise SM03,
  `uid <uid> names no unit in the library`. `smysl.label` checks the same.
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
marks the words that write; every write that appends is made under §S9's
locks. `smysl.create` appends nothing and is the exception (below).

| Word | Operands → result | W | Does |
|---|---|---|---|
| `smysl.create` | root → handle | ✓ | creates a library at `root`, or opens the one already there, as `Library::create` does. It takes the process mutex for the root, canonicalised as §S2 says, and takes no SMYSL lock: SMYSL's own `text add` and `with_library` create the library before they lock (`smysl:src/main.rs:12645-12656,8618-8631`), and there is nothing to lock before the marker exists, since `Library::open` fails without it (`smysl-text/src/library.rs:288-304`). `create` writes the marker with `std::fs::write` after checking it is absent (`:290-293`), which is not atomic, so an open that lands between the two reads an empty marker and is SM02, naming the layout, and so may another process's create, whose own open is the one that lands there; a retry succeeds. A create killed or failing while it writes the marker leaves it empty for good, since `create` never rewrites a marker that exists (`:291-293`): every open is then SM02 naming the layout, until an operator removes the empty marker. U10 asks SMYSL to write the marker atomically, which would close both |
| `smysl.open` | root → handle | | opens the library at `root`; SM01 if there is none |
| `smysl.agent` | handle agent → handle | | the handle with another `agent`; SM03 if the agent does not match SMYSL's grammar |
| `smysl.text.add` | handle spec → MAP | ✓ | adds a text through a reader. Answers `{ mid, parts, lossy, redacted }`: the manifest's id, its parts' ids, whether the reader dropped something the source carried, and how many objects rule Z withheld, from SMYSL's `Added` (`smysl-text/src/library.rs:160-187`), as SMYSL's `text add` prints them (`smysl:src/main.rs:12681-12692`). Under the locks it calls `complete_index` before `add`, and `write_index` after it succeeds, as `text add` does (`:12667,12680`; `smysl-text/src/library.rs:416-424,494-509`). A failed `write_index` is a `Warning`, as SMYSL prints it, since the sidecar is rebuilt on the next open (`:390-398`). `spec` is a MAP, given below the table |
| `smysl.texts` | handle → MAP | | each alias with its heads, a LIST of manifest ids (`catalog`) |
| `smysl.unit` | handle unit → uid | ✓ | appends one unit written as §S2's MAP |
| `smysl.rel` | handle relation → rid | ✓ | appends one relation. A `retracts` relation is SM03, naming `smysl.retract` [R23] |
| `smysl.append` | handle list → LIST | ✓ | appends a LIST of unit, relation and label MAPs, in order and under one acquisition of the locks. Answers each one's id: a unit's uid, a relation's rid, and for a label, the uid it binds. A `retracts` relation anywhere in the LIST is SM03, and nothing is written [R23] |
| `smysl.label` | handle label uid → | ✓ | binds a label. A label the library already binds to another unit is SM08, and nothing is written, since nothing removes a binding and every later use of the label would be SM08 (`smysl-graph/src/labels.rs:3-8`). A label already bound to the same unit writes nothing. A label MAP in `smysl.append` follows the same rule, and so do two label MAPs in one LIST that bind one label to two units: SM08, and nothing of the LIST is written |
| `smysl.retract` | handle uid → rid | ✓ | retracts a unit as SMYSL does (below); NODATA if it was already retracted |
| `smysl.parse` | text → LIST | | parses the surface syntax into §S2's MAPs of units, relations and labels, without the units' and relations' ids. SMYSL's parser resolves a document's labels to uids itself (`smysl-core/src/surface/parse.rs:12-14`) and emits one `Record::LabelBinding` per label after the units it names (`:2596-2606`); the same set is kept as a map (`:63,2793`). `smysl.parse` reads labels from the records only, each `LabelBinding` becoming a label MAP whose uid is the uid `smysl.append` computes for the same unit (criterion 4); the map is not read, and nothing is counted as skipped twice. **A document SMYSL could not parse whole is refused** with SM02, and nothing is answered: the parser recovers from a malformed record by skipping it and reporting it (`:1-6,66-67,348-352`), and an unresolved label is `SMY-E060`, which in a unit's `grounds` or `deps` drops the reference and keeps the unit, under another uid (`resolve`, `:2800-2804,2836-2846`), and in a relation drops the whole relation (`lookup`, `:2553-2562,2608-2614`). Answering what survived would let `smysl.append` write a library that differs from the document. The refusal is any diagnostic of error severity, which is SMYSL's own `has_errors` (`:121-123`) and the test SMYSL's `fmt` refuses a document on (`smysl:src/main.rs:2635-2638`), or any recovered record. A `ParseError` is SM02 too: SMYSL has three, `SMY-E001`–`E003` (`smysl-core/src/error.rs:202-210`), and only the `@doc` header raises them (`parse.rs:823-870`). Warnings are reported as a `Warning` each, a refused document's included, before the refusal. A warning is not a refusal, so a relation of a kind SMYSL does not know, which parses with `SMY-W013` as an extension kind it is not (`:668-678`), is answered, and `smysl.append` then refuses it with SM05, since `RelKind::parse` does (`smysl-core/src/types/relation.rs:103-124`) [R24]. **Comments and authored salience are dropped, and said so.** Neither is a record: SMYSL counts comments so that a writer can say they are lost (`smysl-core/src/surface/parse.rs:68-75,370-373`), as `fmt` does (`smysl:src/main.rs:2640-2649`), and keeps authored salience beside the records (`smysl-core/src/surface/parse.rs:44-46,2587-2590`). `smysl.parse` reports one `Warning` with the number of comment lines, and one with the number of units whose authored salience has no §S2 form. Any other record (a view, a schema declaration, a thread, a manifest, a commitment, …) has no §S2 form. It is skipped, and one `Warning` per record kind names how many were skipped. **A relation the document withdraws is left out too**, by `is_withdrawn`'s test applied to the document's records: a relation that is not `supersedes` or `retracts`, whose rid a withdrawal in the document names (§Current, "Relations"). A `Warning` counts such relations: the withdrawal is one of the records skipped, so answering the relation would let `smysl.append` write live an edge the document says nothing should follow (`smysl-core/src/types/lifecycle.rs:18-22`). A withdrawal of a `supersedes` or `retracts` edge withdraws nothing, so that relation is answered, and `smysl.parse` reports a `Warning` carrying `SMY-W056`'s meaning for each distinct withdrawal of a `supersedes` or `retracts` relation the document holds, naming its agent and the edge's kind. That is what SMYSL's `check` does for a relation the store holds (`smysl-check/src/passes/integrity.rs:46-56`), and identical withdrawals are one, as the store keeps them in a set per rid (`smysl-graph/src/store/interp.rs:31-32`). Only that check raises it (`smysl-check/src/passes/integrity.rs:40-59`). A withdrawal naming a rid under which the document declares no relation has no kind to name: it draws no such `Warning`, and is counted among the records skipped. A `retracts` relation then reaches `smysl.append`'s SM03 as any other does [R23]. Nothing is written |
| `smysl.write` | handle → text | | the surface syntax of the library's units, relations and labels, as SMYSL's `write_surface` gives it. **What it passes:** every record the store holds (`Store::iter`, `smysl-graph/src/store/mod.rs:536-538`), as SMYSL's `merge` does (`smysl:src/main.rs:4327`), but for two kinds. Relations are passed one per rid, as the store resolves them (`Store::relations`, `smysl-graph/src/store/mod.rs:554-556`), each in the place of its rid's first record in `Store::iter`, so the text keeps the log's order, which is the order the writer writes them in (`smysl-core/src/surface/write.rs:158-165`). A second variant of an edge is never written beside the first, and is counted (below). Attestations are not passed: the surface syntax has no form for one (`smysl-core/src/surface/write.rs:218-220`), and every Bund2 write makes one, so counting them would warn on every call. A text therefore carries no attestation, and appending it attests its records afresh as the appending handle's agent. So SMYSL's writer writes the manifests, threads, schema declarations, withdrawals, resolutions, commitments, redactions and datings it has a form for (`write.rs:129-144,158-188`), and a withdrawn edge is written with its withdrawal, which SMYSL's parser reads back as withdrawn; `smysl.parse` leaves such an edge out (above). A withdrawal with no surface form, one with an `extra` or whose clock's agent is not the withdrawing agent (`write.rs:236-240`), is left out and counted. The writer writes every relation it is given (`:161`), so when the library holds the relation the withdrawal names, `is_withdrawn` reports it, and no withdrawal naming its rid has a surface form, the edge reads back live, and the `Warning` for left-out records names it. A withdrawal whose edge another, written withdrawal still withdraws, of a `supersedes` or `retracts` edge, which it never withdrew, or of a rid the library holds no relation under, is only counted. Labels are not records the writer reads: it spells a label only through the context, in its unit's header (`:676-683`). **The labels.** The context gives a uid the first label in canonical order among the labels that bind that uid alone, and only when its unit has a surface form (`unit_has_surface_form`, `smysl-core/src/surface/write.rs:267-295`); any other uid gets none. Every reference the writer emits goes through the context, a relation's ends and note and a unit's grounds and deps among them, and a uid with no label is spelled as itself (`write.rs:104-110,781-794,823-828`). So a reference to an omitted unit is its uid, and the text re-parses. This departs from SMYSL's `merge`, which folds every label in first-wins (`smysl:src/main.rs:4320-4326`): built that way, an omitted unit's label is spelled in references the text never declares, and the re-parse is `SMY-E060`. **What is omitted, and said.** SMYSL's writer leaves out, without a word, a unit with no surface form (`write.rs:158-164`): one with an `extra`, a source with an `extra` or an unknown kind, a payload that does not decode, or a payload key in `UNIT_HEADER_WORDS` (§S2; `:248-252,267-295`). It writes no relation's `extra` (`:781-799`), and a label binding's `extra` has no spelling either. `smysl.write` reports one `Warning` per record kind with records left out, with its count, each record judged by SMYSL's own `has_surface_form` (`:205-221`): units with no surface form, contentions, part texts, readings, and any other. A label binding the context does not admit is one of those: a unit's second label, an omitted unit's label, or a label bound to several units, which also draws a `Warning` naming it. Relations and label bindings whose `extra` was dropped, and the second variants of edges, are counted in one more. A library with none of these draws no `Warning`. SMYSL's CLI prints one total (`warn_surface_omissions`, `smysl:src/main.rs:5702-5720`); counting per kind is `SMY-W428`'s (`smysl-core/src/diag.rs:431`). **A unit with two labels** is written with one, the first in canonical order, since the syntax has room for one (`write.rs:51-61`). **A label the library binds to several units** (`smysl-graph/src/labels.rs:3-8`) is written on none of them, which departs from `merge`'s first-wins: writing it on one unit would re-point the others' references when the text is read back, since a document's last declaration of a label wins (`parse.rs:2500-2507`). No view is passed, so the text has no `@doc` header, and reads back as the default format version (`parse.rs:102-108`) |
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
  `Transformed` (`smysl:src/main.rs:7566-7575`). A unit Bund2 writes was authored by the program that
  wrote it. Rule T reads the attestation's ceiling
  (`smysl-check/src/passes/trust.rs:33-38`), which is the rung's, except
  that an operation that may assign `measured` at the `Computed` rung
  raises it to `measured` (`smysl-core/src/types/provenance.rs:326-331`).
  That operation is `Imported`, which neither `Authored` nor `Transformed`
  is, so the choice changes no status;
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

**Re-writing a relation SMYSL already holds.** A relation whose rid the
library holds, as re-opened under the locks, or which an earlier MAP in the
same `smysl.append` LIST gives:

- **With the same weight and note**, after SMYSL's clamping and
  quantising, nothing is appended for the relation, and its attestation
  follows the unit's rule above. The word answers the rid.
- **With another weight or note**, it is SM03, `rid <rid> is held with
  another weight or note`, and nothing is written; in `smysl.append`,
  nothing from the LIST. Appending it would add a record no read sees,
  under a rid the word would answer as if written (§Current,
  "Relations"). A program that means to change an edge's weight has no
  way to in Phase 1, as SMYSL has none: the rid leaves the weight out
  (`smysl-core/src/types/relation.rs:231-238`), and SMYSL's `withdraw`
  names an edge by its rid and acts on a store file
  (`smysl:src/main.rs:170,4896,5015-5016`). So withdrawing an edge
  withdraws every weight it was given, and Bund2 has no word that
  withdraws one.

**A withdrawn edge.** Each relation `smysl.rel` answers, or a relation MAP
of `smysl.append` answers, that `is_withdrawn` reports draws one `Warning`
that the edge is withdrawn, whether the library held it or it is new. A
held edge stays withdrawn, and a new edge whose rid a withdrawal already
names arrives withdrawn, because withdrawals are kept by rid whether or not
the relation has arrived (`smysl-graph/src/store/interp.rs:31-32`), and
`is_withdrawn` tests only the rid and the kind
(`smysl-graph/src/store/mod.rs:1500-1504`). A `supersedes` or `retracts`
edge never draws it.

**Retraction** follows SMYSL's own `retract` command
(`smysl:src/main.rs:4484-4505`), step for step:

1. If the target was **retracted directly**, nothing is written and the
   word answers NODATA. That is SMYSL's test,
   `effective_status(&store, Strict).is_retracted(&target)`
   (`smysl:src/main.rs:4484`), which reads only the targets of `retracts`
   edges (`smysl-graph/src/merge/retraction.rs:42-45`). A unit `unfounded`
   only because its grounds were retracted is orphaned, not retracted
   (`:47-50,109-117`), and is planned and retracted like any other,
   although `smysl.status` reads both as `"unfounded"`. As in SMYSL, this
   is checked before authority.
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
| `smysl.measure` | handle ids scope → LIST | runs the named measures over the scope an `sq/1` text names. Each result is a MAP `{ id, ok, value, uid, diagnostics }`. When the measure ran, `ok` is true, `value` is the STRING SMYSL's public `gist` gives for the recipe and the value: the measure's id, its headline when it has one, and the scope (`smysl-measure/src/outcome.rs:268-292`, re-exported at `lib.rs:81`). It is never cut. The result unit's gist is the same text cut to the default profile with `…` by a crate-private `fit` (`lib.rs:446-449,461-475`), so the two can differ for a long scope, and `value` exists for a result that has no unit, and `uid` is the id that unit would have. A result that ran but is not a unit has `uid` NODATA, and the diagnostic `to_unit_on` returns for it is added to its `diagnostics` with its own severity and reported as a `Warning`, since the measure ran and only its unit cannot be written. That is `NotPersisted::Unwritable`, `SMY-E489`, a value past a payload integer or a unit that does not build (`smysl-measure/src/lib.rs:408-417,446-458`). `NoBasis`, `SMY-W483`, is mapped the same way, though `scope::resolve` always gives a basis (`smysl-measure/src/scope.rs:109-117`). When it did not run, `ok` is false and `value` is SMYSL's reason, with its code among `diagnostics`; an unknown id is such a result, `SMY-E489` |

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
  (`:1149-1153`);
- a measure's `Input` is built as `measure` builds it
  (`smysl:src/main.rs:7366-7438`):
  1. the scope text is resolved with
     `scope::resolve(text, &env, &PlanOptions::default())`
     (`smysl-measure/src/scope.rs:100`);
  2. when any id named needs `TEXT`, the text is read with
     `texts(text, &library)` and `Corpus::load(&library, &found, level,
     analyzer)` (`smysl-measure/src/text/mod.rs:160,435`), where the level
     and analyzer are the base `Recipe`'s: `Level::default()` and none,
     so each text is read under its own language's analyzer
     (`smysl-measure/src/recipe.rs:202,206`);
  3. the `Input` is `Input::new(library.store(), &resolved)
     .with_library(&library)`, then `.with_text(&corpus)` when step 2 read
     one.

  A text measure such as T1 needs a text scope (`from texts …`); a unit
  measure's scope may filter, which a text scope refuses with `SMY-E461`
  (`smysl:src/main.rs:7385-7387`). Either refusal reaches the program as
  SMYSL reports it. Channels, the second scope `measure --channel` takes,
  are not offered in Phase 1.

### S6. Words — querying

| Word | Operands → result | Does |
|---|---|---|
| `smysl.query` | handle text → MAP | parses, plans and runs one `sq/1` query |
| `smysl` | → c | a conditional, `Conditional("smysl")`, with slots `handle`, `query` and `lambda`. Its effect is `StackEffect::fixed(0, 1).leaving(Leaves::Conditional("smysl"))`, as `query` declares (`crates/bund2-stdlib/src/kb/query.rs`, `register`), since `bund2 check` learns a CONDITIONAL's type only from what its constructor leaves (D181). It runs the lambda once per uid of the answer that names a unit the store holds, with the unit's MAP on top |

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
- A uid with no unit in the store is not visited, as `smysl.kb` leaves
  it out of its scope (§S7). A `grounds` closure adds every ground of a
  seed, held or not (`smysl-query/src/exec.rs:414-418`). `smysl.query`'s
  MAP keeps such a uid.
- A `ranked` answer's scores, and a `groups` answer's keys and boundaries,
  are in `smysl.query`'s MAP but not in what the conditional sees.
- **The kind is the one `run` gives**, which need not be the one the
  query's output asks for. SMYSL turns a `Ranked` or `Packed` answer into
  a `Scope` at several stages. Among them are the supersession pass once
  the library holds any supersession, since "a head that replaced two
  different hits has no one score" (`smysl-query/src/exec.rs:476-482,497-508`),
  a traversal or closure (`:311-315,426`), a set operation (`:169-173`),
  an ordering stage (`:1033-1035`), and a `limit` after `pack`
  (`:1172-1176`). `smysl.query` maps the answer it is given and lists no
  such stage itself, so a program reads `kind` rather than assuming it.

**Queries are SMYSL's text** [R6].

- A query that `parse`, `plan` or `run` refuses is SM04, which carries every
  diagnostic SMYSL gave, so `SMY-E461` reaches the program as SMYSL wrote
  it.
- A run's `warnings` are each reported as a `Warning` (D36).

**The runner's own refusals:**

- a missing slot, or a slot of the wrong type, is SM03, naming the slot.
  A missing `lambda` is SM03 too, where RFC-0013 §S11 reads a missing
  `lambda` slot as an empty lambda: a `smysl` conditional with nothing to
  run is taken as a mistake. `bund2 check`'s arm reads a missing
  `lambda` as empty, as it does for `query`: the checker models effects,
  and the refusal is the runner's;
- an error raised inside `lambda` is SM10, as RFC-0013's E25 is for
  `query`.

The runner runs the lambda through `eval_lambda`, as RFC-0013's `query`
runner does (§S11), and its function is added to `REENTERING`.

- **The answer is computed in full before the lambda first runs**, and the
  library is closed by then. So a lambda that writes to the same library
  does not change the units the conditional still visits, and the runner
  holds no lock or mutex while a lambda runs.
- **The lambda runs on the current stack**, and nothing restores it, as
  RFC-0013 §S11 says of `query`.

### S7. The bridge to logic programming

**`smysl.kb`** (handle scope → kb) loads the units an `sq/1` scope selects
as the facts of a new knowledge base [R7].

- **The scope** is resolved in §S5's environment, as a measure's is: the
  query is parsed, planned and run with `PlanOptions::default()`, the
  three steps `scope::resolve` takes (`smysl-measure/src/scope.rs:101-103`).
  `smysl.kb` takes them itself, since `resolve` keeps only the answer
  (`:108`) and builds a saved-query unit `smysl.kb` has no use for. The
  scope is the units the answer names (`Answer::uids`,
  `smysl-query/src/exec.rs:86-98`). No order of the KB's facts is
  promised: a KB groups its clauses by predicate
  (`crates/bund2-stdlib/src/kb/store.rs`, `Kb::build`).
  - A refusal from any step is SM04, carrying every diagnostic.
  - `run`'s warnings are reported as `smysl.query`'s are (§S8), among
    them `SMY-W464` for a fork in a supersession
    (`smysl-query/src/exec.rs:512-519`).
  - A uid the answer names that is not a unit the store holds is left out
    of the scope, so no fact names it as `U` and no `rel` fact ends on it.
    A `grounds` closure adds every ground of a seed, held or not
    (`smysl-query/src/exec.rs:414-418`). M8's first step establishes what
    a `from texts` answer names.
  - Only current versions are answered, so a superseded unit is in the
    scope only when a history traversal reaches it
    (`smysl-query/src/exec.rs:429-454`).
  - A `ranked` answer gives its hits, as many as `PlanOptions::default()`
    allows (§S5), unless `run` has already made it a `scope` answer
    (§S6); a `groups` answer gives the units of every group.
  - A `count` answer names no units, so its KB holds only the `rank`
    facts.
- The KB is built through the same validation `kb!` uses.
- Uids, schemas and statuses are STRING atoms (RFC-0013 §S1).
- Statuses are the declared ones; effective statuses appear only as
  `effective` facts.

| Fact | Holds | In a proof, stands for |
|---|---|---|
| `[ unit U Schema Status ]` | one per unit in scope | `U` |
| `[ gist U Text ]` | one per unit in scope, its gist | `U` |
| `[ grounds U G ]` | one per ground of a unit in scope, held or not | `U` |
| `[ deps U D ]` | one per dependency of a unit in scope, held or not | `U` |
| `[ label L U ]` | one per label of a unit in scope | `U` |
| `[ effective U Status ]` | one per unit in scope whose status retraction changed, computed over the whole library under the strict policy as `smysl.status` computes it (§S5) | `U` |
| `[ rel Kind From To ]` | one per relation with both ends in scope that is not withdrawn (`is_withdrawn`, `smysl-graph/src/store/mod.rs:1500-1504`), as SMYSL's own interpreting reads leave a withdrawn edge out (§Current, "Relations") | `From` and `To` |
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
  above stands for its units when the library bears it out. Nothing is
  written for it.
  - The units it stands for, `U` in each shape and `From` and `To` in
    `rel`, are units the library holds. A superseded unit the library
    holds counts as held.
  - `G` and `D` need only be among `U`'s grounds or deps, held or not: a
    unit's grounds are fixed in its identity, and SMYSL keeps a unit whose
    ground is absent, reporting it as `SMY-E060`
    (`smysl-graph/src/store/mod.rs:1019-1029`) and counting only the
    grounds present (`smysl-graph/src/merge/retraction.rs:103-108`).
    `Schema`, `Status`, `Text`, `L` and `Kind` are not uids, whatever they
    spell.
  - `[ unit U Schema Status ]`: `U`'s schema and declared status are
    those given.
  - `[ gist U Text ]`: `U`'s gist is `Text`.
  - `[ grounds U G ]`, `[ deps U D ]`: `G` is among `U`'s grounds, or
    `D` among its deps.
  - `[ label L U ]`: the library binds `L` to `U`.
  - `[ effective U Status ]`: `effective_status` over the whole library
    under the strict policy gives `U` that status, and it is not `U`'s
    declared status (§S5's `smysl.status`).
  - `[ rel Kind From To ]`: the library holds a relation of that kind
    from `From` to `To`, and `is_withdrawn` does not report it.

  These are the facts `smysl.kb` loads for a scope holding those units,
  so a fact `smysl.kb` loaded stands for its units whatever scope it came
  from, at write time: an edge withdrawn since, through SMYSL's API or
  `smysl lib import` in another process (§Current, "Relations"), leaves
  its `rel` fact a program fact. A unit's schema, status, gist, grounds
  and deps are fixed in its uid, and a retraction appended since only
  ever makes a unit `unfounded`
  (`smysl-graph/src/merge/retraction.rs:83-86,112-117`), so an
  `effective` fact `smysl.kb` loaded stays borne out under later
  retractions. It can lapse when a ground absent at load time is appended
  since, by any writer, a bridge while the proof runs among them: the
  orphaning test
  counts only the grounds present (`:97-124`), so `U` may return to its
  declared status, and the fact is then a program fact. Any other fact of
  these shapes is a program fact, so no conclusion is written as resting
  on a status, ground or dependency `U` does not have, a label or edge
  the library does not hold, or on an edge SMYSL does not follow. A fact node
  is read back as the answer instantiated it (RFC-0013 §S15), so one
  still holding a variable is a program fact too.
  - This is judged **at write time**, against the library as re-opened
    under the locks (§S9). A unit a bridge wrote while the proof ran
    therefore counts.
- A `[ rank Status N ]` fact with one of the six statuses and its number
  stands for nothing.
- Every other fact node is a **program fact**. That includes a fact of one
  of these shapes the library does not bear out, wherever it came from.

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
| SM00 | an operand that should be a handle is not a MAP of `type = "smysl"` with a STRING `root` that is an absolute path and a well-formed `agent` | `#<k> parameter must be a SMYSL handle` |
| SM01 | `smysl.open`, or a word that opens the library, finds no library at `root`; or `root` cannot be canonicalised, or made by `smysl.create`'s `create_dir_all` (§S2) | `no SMYSL library at <root>`; when the failure is an I/O error other than a missing path, `cannot reach <root>: <the I/O error>` |
| SM02 | SMYSL refused the operation, or a SMYSL call failed with an I/O error: among them a lock that could not be linked (§S9), and a primary shard this build cannot read (§Current, "The library"). Also a `smysl.text.add` input over `Caps::default().input_bytes`, which Bund2 refuses before reading it (§S4) by calling SMYSL's own `Budget::new` with the input's length (`smysl-text/src/limits.rs:168-172`). That error names the SMYSL CLI's flag for the cap, `(raise it with --max-input)` (`smysl-text/src/limits.rs:109-111`; `smysl-core/src/error.rs:905-916`), which Bund2 does not have. So Bund2 displays every `LibError::Limit`, whichever cap a reader exceeded, with `flag` set to none: `SMY-E440: <cap> limit <limit> exceeded by <saw>`. Also `smysl.parse` of a document SMYSL could not parse whole (§S4), whose text joins each error diagnostic as SMYSL's `Diagnostic` `Display` writes it, with its severity, code, message, `(at <span>)` and any `[try: …]` (`smysl-core/src/diag.rs:605-619`), with `; `, then `; <n> records skipped` when any were | `SMYSL refused it: <message>`, where `<message>` is SMYSL's error as its `Display` writes it. That begins with the code when the error has one (`SMY-E402: …`, `smysl-core/src/error.rs:895-926`), and has none for the variants `LibError::code` gives none, `Io` among them (`:858-878`), so no code is written twice or invented. A `ShapeError` displays the same way, with its code (`:179-192`). A held lock is never SM02: it is SM07. When R25 keeps the locks, the text continues `; the library is left locked until an operator runs bund2 smysl-unlock` |
| SM03 | an operand of the wrong type (`#<k> parameter must be <kind>, found <type>`); a spec, record or slot lacking a required key or holding one of the wrong type; a value with no SMYSL form; a computed key given; a unit MAP's `labels` on write; a status Bund2 cannot author; a `smysl.text.add` spec with no `lang`; a `retracts` relation outside `smysl.retract` [R23]; a record MAP of no kind, of the wrong kind for its word, or with a key its kind does not have (§S2); a label MAP whose `uid` names no unit; a `grounds` or `deps` STRING that is neither a uid nor a label; an edge whose rid the library holds with another weight or note (§S4) | `<what> <reason>`, e.g. `uid is computed, not given` |
| SM04 | `sq/1` refused the query | `the query was refused: <code> <message>`, one per diagnostic, joined by `; ` |
| SM05 | an unknown status, relation kind, source kind, reader or trace kind | `unknown <kind>: <value>` |
| SM06 | `--noio`, for a word that acts on disk, and for the constructor `smysl` and the conditional's handler, as D119 stubs the word `csv` and its handler (D119) [R11]. Each stub is registered `opaque(0)`, as D119's are | unprefixed, in the pattern of the house text `bund SAVE.MODEL functions disabled with --noio` (`bund_models.rs:125`): `bund SMYSL functions disabled with --noio` |
| SM07 | a lock is held (`SMY-E445`, which only `Lock::acquire` raises, `smysl-graph/src/lock.rs:165`, and Bund2 reaches only through `acquire_all`); or the shard set changed twice while locking (§S9) | `the library is locked: <SMYSL's Display of the error>`, which names the holder (`smysl-core/src/error.rs:918-926`), or `the library's shards changed while locking: <shard>` |
| SM08 | a label bound to several units, or to none; or `smysl.label`, or a label MAP in `smysl.append`, binding a label the library or the same LIST binds to another unit (§S4) | `label <label> binds <n> units`, `<n>` being 0 for an unbound label; at bind time, `label <label> is bound to <uid>, not <uid>` |
| SM09 | `plan_retraction` refused the retraction | `retraction refused: <SMYSL's reason>` |
| SM10 | the `smysl` conditional's `lambda` failed | `lambda returns: <error>` |
| SM11 | the stack holds fewer operands than the word takes | the house text, unprefixed, as `reference/Bund/src/stdlib/functions/statistics/get_data.rs:12` writes it: `Stack is too shallow for inline <w>` |
| SM12 | a profile of the library's views leaves no room for an exported gist (§S7) | `the library's view allows no gist of <n> tokens` |

- **An unknown measure id is not a refusal.** It is a result, as SMYSL
  reports it (§S5).
- **A panic inside SMYSL is not a refusal.** D49 returns it as
  `Error::internal`, and §S9 says what it leaves behind.
- **Warnings** from `run`, from a measure's diagnostics, and from a
  failed `write_index` are each reported as a `Warning` (D36). An append
  reports none: `append_records` returns `()` on success
  (`smysl-text/src/library.rs:1341`). `smysl.parse` routes SMYSL's
  `SMY-W054`, a label declared twice in one document
  (`smysl-graph/src/labels.rs:10-12`), the same way, and its dropped
  comments and salience; `smysl.write` reports what it omits (§S4). A measure's diagnostics are also
  kept in its result MAP.

### S9. Effects, locks and threads

**Effects.**

- Every word but `smysl.prove` has a fixed effect, the constructor
  `smysl`'s included (`fixed(0, 1)`, leaving a CONDITIONAL; §S6).
  `bund2 check` reads each one's declared effect. No word but
  `smysl.prove` and the `smysl` conditional's handler re-enters
  evaluation.
- The `smysl` conditional's handler re-enters, as `query`'s does.
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
   `acquire_all` (`smysl-text/src/lock.rs:24-43`), naming the command
   as `bund2 <word>`, for example `bund2 smysl.append`, which a lock's
   holder records and SM07 and `bund2 smysl-unlock` print;
4. **open the library a second time, and use only this second open from
   here on;**
5. if the second open names a shard the locks do not cover, drop the
   locks and start again from step 1. A second mismatch is SM07, naming
   the shard;
6. validate and decide, then take R25's snapshot (below);
7. write. If a call fails, take a second snapshot and compare (R25);
8. drop the locks before the word returns, or forget them where R25 or a
   panic says so.

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

**What the locks do not cover** [R22]. Six SMYSL commands append without
locking (§Current, "The locks"): `q --save`, `ask --save`, `measure` and
`assess` unless given `--dry-run`, `same-as`, and `ingest --text`.
- **Bund2 promises serialisation only against writers that lock:**
  itself, in this process and in others, and SMYSL's `lib` and `text`
  commands.
- **Against the six it promises nothing, and cannot.** They write at any
  moment, including between Bund2's second open and its append. Then:
  - Bund2's de-duplication, its "already attested" and "already
    retracted" tests, and its clock's `prev`, may rest on a library that
    has since moved on;
  - worse, if Bund2 opens while one of them is part way through a record,
    it reads that record as a torn tail, and its append cuts it off
    (`smysl-text/src/shards.rs:236-251,376-379`). The other process then
    writes the rest of its record at the new end. Neither side reports
    anything.
- A locking SMYSL command opening beside one of the six can do the same,
  so the hazard is SMYSL's. Bund2 makes it neither better nor worse, and
  the remedy is SMYSL's: U9 asks it to lock those commands.
- **R22 A, as ruled and amended on 2026-11-03:** Phase 1 proceeds, and
  its documentation says a library a Bund program writes must not be
  written by those six commands at the same time. (R22 B was that Phase 1
  waits for U9, and pins a SMYSL revision that has it.)
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
    - The work is a function of `bund2-stdlib`'s `smysl` module, which
      lists and, when told, breaks the locks, and returns what it found.
      The subcommand prints that and nothing more. So a `bund2-stdlib`
      test can call the function directly after a probe-armed panic
      (criterion 15), and the probe needs no forwarding to `bund2-cli`.
  - It is a subcommand and not a word, so a Bund program cannot break a
    lock. Breaking stays an operator's act, as SMYSL asks.
  - SMYSL is asked for its own command as U8. If it ships one, the
    subcommand may be withdrawn by an amendment.
- **A panic inside a write leaves SMYSL's locks in place** [R20], as a
  killed SMYSL process would. That keeps SMYSL's crash notice.
  - **So does a failed call after which a log changed** [R25, as amended
    twice on 2026-11-03].
    - **When.** Bund2 takes a snapshot once, after the re-open (step 4)
      and before its first call to `append_records` or `Library::add`.
      If any call of that acquisition fails, it takes a second and
      compares. So a word that wrote one batch and then failed on a second
      still keeps the locks.
    - **Which logs.** Those `shards::list` names
      (`smysl-text/src/shards.rs:84-113`), which are exactly the logs the
      open reads, since the open iterates `list`
      (`smysl-text/src/library.rs:332`), each by
      its `std::fs::metadata` length. A log absent counts as length 0.
      The `LIBRARY` marker's contents are in the snapshot too.
    - **Torn tails, exactly.** An append cuts a torn tail before it writes
      (`smysl-text/src/shards.rs:376-379`), so a write that then fails can
      leave a file of the same length, or shorter, holding a new record.
      - Before the call, the total of the logs' lengths on disk exceeds
        the stamp's length exactly when some log is torn, since the stamp
        counts only what was decoded (`smysl-text/src/shards.rs:395-416`).
        That is the cheap test, and it is false in the usual case.
      - When it is true, Bund2 reads each log and decodes it with the
        public `from_cbor_seq` (`smysl-core/src/cbor/envelope.rs:2015-2028`),
        as `ShardLog::read` does (`smysl-text/src/shards.rs:234-251`). For
        each torn log the snapshot keeps the tail: the bytes past what
        decoded.
    - **Changed** means: a log's length differs; or a torn log's bytes
      past its decoded length differ from its snapshot; or the marker's
      contents differ. Otherwise a log is unchanged, because a log opened
      for append without a cut can only grow.
    - **If anything changed,** part of the batch may be on disk and part
      not. The guard forgets the locks, exactly as on a panic, and the
      word answers SM02 with the "left locked" suffix (§S8).
      - A changed marker alone keeps them too. In a layout-1 library a
        batch that creates a shard rewrites the marker first
        (`smysl-text/src/library.rs:1355-1362`), and a write that fails
        part way leaves a library no release can open (`:304-319`).
    - **If nothing changed,** nothing of the batch reached a log.
      `Library::add` may still have written text objects, which are
      written to a temporary file and renamed into place
      (`smysl-text/src/objects.rs:241-264`), are addressed by their
      identity (`smysl-text/src/library.rs:680-682`), and are named by no
      manifest. The locks are dropped as usual, and the word answers SM02
      with SMYSL's reason.
    - So every refusal that writes no log, made by the first call of an
      acquisition, leaves no lock, over a torn
      library as over any other: `check_loggable`'s (`:1347-1354`), every
      `Library::add` refusal before its objects (a licence that forbids
      the carry, a reader cap, an unbuildable structure, no parts,
      `:1148-1190`), a language-less `add`, an I/O error before the first
      shard. A refusal by a later call of the same acquisition keeps the
      locks if an earlier call wrote, which is what the snapshot is for.
    - **If a snapshot cannot be taken,** because `shards::list`,
      `metadata` or a read fails: before the first call, the word is SM02
      and calls nothing; after a failure, the locks are kept, since
      nothing then shows they may go.
    - **Not the stamp,** which revision 8 used. It is computed over
      in-memory lengths that move only after `write_all` and `sync_data`
      both succeed (`smysl-text/src/shards.rs:380-384`). A short write,
      or a failed sync, leaves whole records in the file with the stamp
      unchanged.
    - **The test is exact outside R22's hazard.** One of R22's unlocked
      writers can change a log during the call, and the locks are then
      kept although Bund2 may have written nothing. That is conservative.
      Such a writer can also shorten a log by cutting what it took for a
      torn tail, which the snapshot cannot tell from Bund2's own cut; that
      too keeps the locks. The same holds for a shard log such a writer
      created that the batch then read from disk
      (`smysl-text/src/library.rs:1366-1368`).
  - Bund2 holds the locks `acquire_all` answers in a guard of its own.
    When that guard is dropped while the thread is panicking
    (`std::thread::panicking()`), or after a failed call that changed a
    log or the marker, as defined above,
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

  CI runs the probe-gated tests in a step of their own, and clippy reads
  them (§S1).

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
  `ask --save`, `measure` and `assess` unless given `--dry-run`, `same-as`
  and `ingest --text` call `append_records` without `acquire_all` (§Current, "The locks"), which
  the comment on `Library::shards` says no library-wide writer does
  (`smysl-text/src/library.rs:765-769`). Since an append cuts a torn tail,
  one of them can silently cut off a record another writer is part way
  through.

- **U10. Write the `LIBRARY` marker atomically.** `Library::create`
  checks that the marker is absent, then writes it with `std::fs::write`
  (`smysl-text/src/library.rs:290-293`), and `append_records` rewrites it
  the same way when it raises the layout (`:1355-1362`). An open, which
  takes no lock, can read it empty between the two and fail (`:304-319`).
  A temporary file renamed into place, as objects are written
  (`smysl-text/src/objects.rs:241-264`), would close that.

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
`ACTS_ON_HOST`, in the milestone that registers it. Each word that touches
disk gets its `--noio` stub (§S8) in the same milestone, so no build reads
a library under `--noio` between milestones. Each refusal code is
tested in the milestone that first raises it, so criterion 14 is complete
when M10 is.

| Milestone | Builds | Criteria |
|---|---|---|
| M1 | the `smysl` feature on `bund2-stdlib` and its forwarding from `bund2-cli` (§S1); the eight crates at `rev = "4f5ecb9"` and the `allow-git` entry naming D244; `SMYSL_BUILT_IN` in `features_built_in`; four of §S1's seven CI steps, all but the probe's nextest run and clippy run, which arrive with the probe in M3, and the `smysl_demo` run; `FEATURE_GATED` moved to module scope and `UNVARIED` filtered by it. **First, Q55's spike:** in a test, build each value §S4–§S7 need against a fixture library, and run one query, one dossier, one measure and one attested append, through SMYSL's API alone | 1, 2, 3; 16's audits; 18's `features_built_in` case |
| M2 | handles and values (§S2), and no word that writes: the handle MAP, canonicalisation, validation; `smysl.open`, `smysl.agent`; the read-back and write mappings of units, relations, sources, diagnostics and payloads, tested as functions against records built through SMYSL's API | 14's SM00, SM01, SM02, SM03, SM05 and SM11 cases that need no write (SM02 by a primary shard `smysl.open` cannot read: Phase 1 builds without `store-redb`, so the fixture is a marker made by hand beside where the shard's log would be, `log/shards/<stem>.primary` for any shard but the catalog (`primary_marker` and `log_path`, `smysl-text/src/shards.rs:42,65-71,115-119`), which `shards::list` and `is_primary` read and `read_primary` refuses, `:96-99,188-190,306-322`); 18's read-back cases; 18's `--noio` case for its words, `smysl.agent` on a root that does not exist among them; 14's relative-root SM00 case for `smysl.agent` |
| M3 | the first writes, with their locks (§S9, §S4): the process-wide mutex table; open, lock, re-open and the shard-set restart; the guard and its snapshot of the logs, their torn tails and the marker; the probe feature and §S1's two CI steps for it; the `bund2 smysl-unlock` subcommand and the function it calls; `smysl.create` and `smysl.unit`, with attestations, their clock, rung, hop and parents, and the already-attested rule | 4; 15's stale-lock, thread, panic and changed-log cases; 18's create-and-open race; SM02's write cases, SM07, SM08 (an unbound label in `grounds`); SM01's `cannot reach` case; 18's `--noio` case for its words |
| M4 | the rest of forming (§S4): `smysl.rel`, `smysl.label`, `smysl.append`; `smysl.text.add` and its spec, `smysl.texts`; `smysl.parse`, `smysl.write`; `smysl.check` | 5, 7; 15's failed-append, nothing-written, torn-refusal, SM02-text and re-open cases; 18's `smysl.rel`, `smysl.label`, label-MAP, record-kind, parse-refusal, parse-warning, write-omission, withdrawn-edge, lifecycle-withdrawal, empty-payload, relation re-write and `smysl.text.add` reporting cases; 14's `smysl.text.add` cases, the sparse file over the input cap and the spec with no `lang`, and its relative-root SM00 case for `smysl.texts`; SM08's bind-time cases; 18's `--noio` case for its words |
| M5 | retraction and research (§S4, §S5): `smysl.retract` in SMYSL's sequence; `smysl.get`, `smysl.trace` with all five kinds, `smysl.status`, `smysl.salience`, `smysl.pack` | 8; 15's attestation and advisory-authority cases; 18's salience and pack case; SM09; 18's `--noio` case for its words |
| M6 | dossiers and measures (§S5), in the environment §S5 names | 10; 18's `Unwritable` case; 18's `--noio` case for its words |
| M7 | querying (§S6): `smysl.query` and its answer MAPs; the `smysl` conditional and its runner; `bund2 check`'s arm; the runner's `REENTERING` entry | 6, 9; 16's `REENTERING` scan; 18's `bund2 check` case for the conditional, and its warning case; SM04, SM10; 18's `--noio` case for the constructor, the handler and `smysl.query` |
| M8 | `smysl.kb` (§S7); its first step establishes, against a fixture, what a `from texts` scope's answer names, and whether §S7's rule for a uid with no unit covers it | 11, its withdrawn-edge and scope-environment cases among them; 18's `--noio` case for `smysl.kb` |
| M9 | `smysl.prove` (§S7): proof nodes to units, statuses under rule M, gists fitted to the library's profiles after NFC, solve then write; §S7.1's rendering; its `bund2 check` arm | 12, 13; 18's `bund2 check` case for `smysl.prove`, and its withdrawn-`rel`, hand-built `grounds`, `label`, `effective` and `unit`, superseded-ground and unheld-ground cases; SM12; 18's `--noio` case for `smysl.prove` |
| M10 | `bund_examples/smysl_demo/` and `smysl_demo.rs`, with §S1's CI step that runs it; no `--noio` stub, each having arrived and been tested with its word; the editor syntax file left as at 0.53.0 | 14 complete (SM06), 16 complete, 17; 18's `--noio` case complete |

Each criterion is claimed by the first milestone at which every word it
uses exists. A criterion split across milestones is complete at the last
of them. Criterion 18's `bund2 check` bullet is split so: its conditional
half in M7, its `smysl.prove` half in M9.

**What may stop a milestone.**
- **M1.** If Q55's spike finds two values that cannot be composed from
  outside SMYSL, M1 stops and the gap goes to SMYSL as part of U2, or to
  an amendment.
- **M3.** Its words are the first that write, and they arrive with §S9's
  locks, so that no word ever appends without them. `smysl.create`
  takes no SMYSL lock, but it takes the process mutex, which M3 builds,
  and so waits for M3 rather than joining `smysl.open` in M2.
- **R22** is ruled A, so M3 does not wait for U9. The documentation
  M10 writes for the example says that a library a Bund program writes
  must not be written at the same time by SMYSL's six unlocked commands.
- **Q51** stays open through M4. If M4's failed-append test shows a
  library that SMYSL cannot read back, that is an amendment for a repair
  step.

---

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| Every word the reference registers | **Preserved exactly.** No name is reused: no `smysl` name exists in the reference or in Bund2 |
| The default build | **Preserved exactly.** The feature is off, so nothing is compiled or registered |
| `save.model`, `load.model` and the world file | **Preserved exactly.** A handle is a MAP with an absolute root, canonical when `smysl.create` or `smysl.open` built it, and saves like any other value |
| `--noio` | **Extended in kind** [R11]. Every `smysl.` word that touches disk is stubbed, as the words at the reference's 16 `cli.noio` sites are. So is the `smysl` conditional's handler, as D119 stubs `csv`'s and `sqlite`'s (`crates/bund2-stdlib/src/data.rs`, `csv_disabled` and the `noio` arm of `register`): `conditional` builds a CONDITIONAL of any type, so stubbing the constructor alone would leave a library readable. `smysl.parse` and `smysl.agent` do no I/O and are not stubbed; `smysl.agent` does not canonicalise its handle's root (§S2) |
| `conform`, `COVERAGE` and `IMPLEMENTED` | **Unchanged.** Words only Bund2 has are not in the reference's registry (D72), and the default build does not register them |
| `bund2 words` | **Differs by build**, as it already does for `grok`. The `smysl` build lists the new words |
| SMYSL's lock as a crash notice | **Kept** [R20, R25]. A panic, or a failed call after which a shard log or the marker changed (a length, a torn tail's bytes, or the marker's contents; §S9), leaves the locks, as a killed process does, so the library refuses writes until an operator breaks them with `bund2 smysl-unlock` (R14), since SMYSL has no command for it |
| The `bund2` CLI | **Extended in the `smysl` build** by one subcommand, `smysl-unlock` [R14]. The default build's CLI is unchanged |
| A write's view of the library | **Stricter than SMYSL's commands.** Bund2 re-opens under the locks (§S9), so a record another locking writer appended just before is not appended again |
| Concurrent use with SMYSL's commands that save | **Not safe, as in SMYSL itself** [R22]. `q --save`, `ask --save`, `measure` and `assess` unless given `--dry-run`, `same-as` and `ingest --text` append without locks, so a Bund2 write beside one of them may decide from a stale library, or cut off a record it is part way through (§S9). Bund2 makes this neither safer nor less safe; U9 asks SMYSL to lock them |
| Reading a library | **Not strictly read-only, as in SMYSL.** Every open, a read's included, unlinks the objects of redacted parts, without a lock (§Current, "The library") |
| A library's layout | **May be raised.** A Bund2 write that creates a shard in a layout-1 library rewrites its marker to layout 2, as any SMYSL write does. SMYSL releases before 1.10.0 then no longer read it whole (§Current, "The library"). The marker is rewritten before the first log, so a write that then fails raises it with no new shard, and one that fails while writing it leaves a library nothing can open (`smysl-text/src/library.rs:304-319,1355-1362`). Either keeps the locks (§S9). U10 asks SMYSL to write it atomically |
| Creating a library | **As SMYSL's, outside the locks.** `smysl.create` does what SMYSL's `text add` does before it locks, under Bund2's process mutex. An open that races a create can read an empty marker and is SM02; a create killed while writing the marker leaves it empty until an operator removes it (§S4) |
| SMYSL's index sidecar | **Kept current, as SMYSL's `text add` keeps it.** `smysl.text.add` completes the index under the locks and writes the sidecar after (§S4). The other writing words do not, as SMYSL's other appenders do not, so the next open rebuilds the catalog tables when the sidecar's stamp no longer matches (`smysl-text/src/library.rs:390-398`), which costs speed, not meaning (Q54) |
| A parsed document | **Refused whole, or answered whole.** `smysl.parse` refuses a document SMYSL could not parse without skipping or dropping something, where SMYSL's own parser recovers and reports. So `smysl.parse` then `smysl.append` never writes a library that differs from its document by a record the parser lost (§S4). Comments and authored salience are not records, and are dropped with a `Warning` each, where SMYSL's `fmt` warns of the first and keeps the second. A document answered with a warning can still be refused by `smysl.append`: a relation of an unknown kind is SM05 there [R24] |
| Units with no surface form | **Left out of `smysl.write`'s text, and counted.** SMYSL's writer omits them silently, and SMYSL's CLI prints one total; `smysl.write` reports a `Warning` per record kind, as `SMY-W428` describes. Bund2 can write such a unit itself, through a payload key named as a header field, so `smysl.write`, `smysl.parse` and `smysl.append` can copy a library short a unit and its labels, said by that `Warning` (§S4) |
| A label bound to several units | **Written on none of them**, where SMYSL's `merge` keeps it on the first. `smysl.write` writes each such unit unlabelled and its references as uids, and names the label in a `Warning`, so a re-parse re-points no reference (§S4) |
| Records other than units, relations and labels | **Written as SMYSL's writer writes them**, through `smysl.write`: manifests, threads, withdrawals and the rest it has a form for, and counted in a `Warning` when it has none. Attestations are never written, so a text appended elsewhere is attested afresh by the appending agent (§S4) |
| A withdrawn edge | **Never followed, and absent from a Bund2 copy.** "Withdrawn" is SMYSL's `is_withdrawn`, so a `supersedes` or `retracts` edge never is. `smysl.write` writes a withdrawal with its edge, which SMYSL's parser reads back withdrawn, or names the edge in a `Warning` when none of the edge's withdrawals has a surface form; a library's withdrawals may come from SMYSL's API or `smysl lib import` (§Current), and the rules are the same; `smysl.parse` leaves out an edge its document withdraws, with a `Warning`, so `smysl.write`, `smysl.parse` and `smysl.append` copy a library without it; `smysl.kb` loads no `rel` fact for it, and `smysl.prove` lets no `rel` fact stand for its ends (§S4, §S7) |
| A fact a proof uses | **Stands for its units only when the library bears it out**, judged at write time; otherwise it is a program fact, written as a claim. A hand-built fact whose units are held but whose schema, status, gist, ground or dependency `U` does not have, or whose label the library does not bind, or which still holds a variable, is a program fact; revision 16 let it stand for its units, though it already asked a `rel` fact's edge to be held and live. A superseded unit the library holds counts as held, and a ground or dependency need not be held, so a fact `smysl.kb` loaded stands for its units at write time, unless its edge was withdrawn since, or its `effective` status lapsed when an absent ground was appended (§S7) |
| An omitted unit's references | **Written as its uid.** `smysl.write` gives no label to a unit it omits, so a relation or a ground naming it still parses back (§S4) |
| A relation's and a label binding's `extra` | **Dropped by `smysl.write`, and counted** in a `Warning`, since the surface syntax has no place for either (§S4) |
| An edge re-written with another weight or note | **Refused**, SM03. SMYSL's API would append it and every read would keep the first (§S4) |
| Retracting an orphaned unit | **As SMYSL's.** Only a unit retracted directly answers NODATA; one `unfounded` by cascade is retracted (§S4) |
| A unit's second label | **Bound, and not written.** `smysl.label` can give a unit a second label; `smysl.write` writes one, as SMYSL's writer does (§S4) |
| What `smysl.text.add` reports | **As SMYSL's `text add`.** `lossy` and the count of objects rule Z withheld are in its answer MAP (§S4) |
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
| CI | **Extended** (§S1): three nextest runs, two clippy runs, the feature-matrix check with and without a C compiler, and a `cargo deny` run under the feature |
| The wall clock | **Enters the library.** Attestation clocks and a dossier's clock come from the wall clock, so two runs of one program write different attestation records. Uids do not change, since attestations are outside identity (`smysl-core/src/types/unit.rs:20-22`). What a program prints from a library is the same across runs only if it prints no clock (criterion 17) |
| Process-wide state | **One table that only grows.** The mutex table (§S9) gains an entry per canonical root written, and none is removed. It holds a path and a mutex per entry, and outlives every VM |
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
     skipped kinds draw the warnings §S4 names. No unit MAP `smysl.parse`
     answers has a `labels` key, and each label appears once.
   - A sixth document, holding a `retracts` relation, is refused by
     `smysl.append` with SM03, and the library is unchanged.
   - A seventh, with one malformed record; an eighth, whose `grounds`
     name a label it never binds (`SMY-E060`, which would keep the unit
     with fewer grounds); and a ninth, with a relation naming a label it
     never binds (`SMY-E060`, which would drop the relation), are each
     refused by `smysl.parse` with SM02 carrying SMYSL's diagnostics,
     spans included.
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
   - Retracting a unit already retracted directly (`is_retracted`)
     writes nothing and answers NODATA, whatever the handle's agent.
   - Retracting a unit that is `unfounded` only by the cascade, through a
     handle whose agent attested it, writes its self-edge and answers the
     rid.
   - `smysl.trace` with `"rests-on"` equals `trace_via` over
     `EdgeSet::premises`, compared in the test.
9. **Every answer shape is mapped.**
   - One query per `Answer` variant gives §S6's MAP, with the uids `run`
     gives, in the same order. The fixture holds no `supersedes` edge;
     the ranked query has no traversal, closure, set operation or
     ordering stage, and the packed query no `limit` after `pack`, so
     each keeps its kind. In a second fixture that holds a `supersedes`
     edge, the ranked query answers `"scope"` and the packed query
     `"packed"`.
   - A `grounds` closure from a unit with a ground the library does not
     hold: `smysl.query`'s `units` include the ground, and the `smysl`
     conditional's lambda does not run for it.
   - A query `plan` refuses is SM04, carrying every diagnostic.
   - A `count` answer runs the `smysl` conditional's lambda zero times.
10. **Dossiers and measures match, and write nothing.**
    - One dossier, and the measures T1 and A4, over a fixture library give
      what the direct calls give. The direct calls use the environment §S5
      names: the same `LibraryScope`, an `Env` with times and entities,
      `PlanOptions::default()` and the base `Recipe`, and for each measure
      an `Input` built by §S5's three steps, T1's over a text scope with
      the corpus `Corpus::load` reads.
    - Each measure's `value` equals `smysl_measure::gist` of the
      result's recipe and value. One measure runs over a scope long
      enough that the unit's gist is cut, so that `value` and the unit's
      gist differ.
    - A measure id `"XX"` gives a result with `SMY-E489`.
    - The library's catalog and units are unchanged afterwards.
11. **The bridge loads.** `smysl.kb` over a scope gives the facts of §S7's
    table, each kind counted against the library. The `rel` facts are
    counted against `Store::relations` with both ends in the scope, less
    what `is_withdrawn` reports:
    a fixture with a `rebuts` edge withdrawn through SMYSL's API gives no
    `rel` fact for it, and the example rule above finds no finding
    resting on its target through it. The scope is resolved in §S5's
    environment: a superseded unit that a scope matches without a history
    traversal gives no `unit` fact, and its newest version gives one; a
    scope whose answer is a `count` gives only the six `rank` facts; a
    scope over a forked supersession reports `SMY-W464` as a `Warning`;
    and a `grounds` closure from a unit with a ground the library does
    not hold gives no `unit` fact for that ground.
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
    - **The libraries.** Each proof is written into four libraries.
      No `smysl.` word writes a view, so a test builds each library's
      view through SMYSL's API:
      - one with no view;
      - one whose view sets `l0_max: 20`;
      - one with two views of different bounds;
      - one whose view's profile names `smysl/content/1`, with a goal of
        CJK atoms.

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
    - a unit MAP with `body: ""` and a `detail`, which is SM02;
    - `smysl.create` of a root beneath a regular file, which
      `create_dir_all` cannot make, giving SM01's `cannot reach` text;
    - a hand-built handle with a relative root, given to `smysl.agent` and
      to `smysl.texts`, which is SM00 from each and touches no file.
15. **Locks are SMYSL's.**
    - With a lock file left as a crashed SMYSL process would leave it, a
      write is SM07 naming the holder, and `smysl.open` of the library
      succeeds.
    - Four VMs on four threads each append 250 units to one library,
      through two spellings of its root, both absolute. All 1,000 are present afterwards,
      read through SMYSL's API.
    - With the probe armed to `panic`, a write that panics with the mutex
      and the locks held:
      - is reported as `Error::internal`;
      - leaves the library lock and every shard lock it took in place;
      - makes the next write to that root, from a fresh VM on another
        thread, SM07 naming this process;
      - leaves `smysl.open` of that root succeeding.

      The test then calls the function `bund2 smysl-unlock` calls
      (§S9): it lists every one of those locks with its holder, and told
      to break them, breaks them all, after which a write succeeds. A CLI
      test runs `bund2 smysl-unlock <root>`, then with `--yes`, over a lock
      file left by hand, and checks what each prints and that the lock is
      gone after the second.
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
    - **A changed log keeps the locks, wherever the change is.** The
      guard's snapshot and comparison are tested by themselves over a
      fixture library. The test takes a snapshot, changes the library by
      hand, and takes a second:
      - a whole record's bytes appended to the catalog's log, as a write
        whose `sync_data` failed would leave it: *changed*, although the
        library's stamp is unchanged;
      - a log shortened; a log created under `log/shards`; the marker
        rewritten: each *changed*;
      - over a library whose `zeta` log was given a torn tail before the
        open, that tail replaced by a whole record of the same length:
        *changed*, although no length changed;
      - nothing touched, over a library with and without a torn tail:
        *unchanged*.
    - **A refusal over a torn library leaves no lock.** Over a library
      whose `zeta` log holds a torn tail, a `smysl.text.add` that SMYSL
      refuses for `NoParts` (an empty `text`) is SM02 without the "left
      locked" suffix, leaves no lock, and the next write succeeds.
    - **A failure that writes nothing leaves no lock.** A batch
      `check_loggable` refuses (a part text, which no `smysl.` word can
      build, `smysl-text/src/library.rs:1332-1334`), and a `Library::add`
      that SMYSL refuses for want of a language (the word's own SM03 stops
      it first), each called through Bund2's guarded write directly, leave
      no lock, and the next write succeeds.
    - **SM02's text.** The failed-append case's answer is `SMYSL refused
      it: ` followed by SMYSL's `Display` of the `Io` error, with no code,
      and then the "left locked" suffix. A `smysl.text.add` whose licence
      forbids `carry: "text"` is the same prefix followed by
      `SMY-E402: …`, once, with no suffix. The stale-lock case's SM07
      text is `the library is locked: ` followed by SMYSL's `Display` of
      the `Locked` error.
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
    - **The demo starts from no library.** Run twice against one library,
      `smysl.retract` would answer NODATA the second time and
      `smysl.text.add` would return the existing manifest. So the demo
      names its library by a relative root, and the test copies
      `demo.bund` into a fresh temporary directory and runs it there as
      the working directory. The demo prints no clock (Preservation, "The
      wall clock").
18. **What the other criteria do not reach.**
    - `smysl.salience` and `smysl.pack` over a fixture library equal
      `salience` and `pack` called directly with the same inputs.
    - `smysl.append` decides each MAP's kind by §S2's rule: a MAP with
      none of `label`, `kind`/`from`/`to` or `gist` is SM03 `record is
      not a unit, relation or label`; `smysl.unit` given a relation MAP
      is SM03; a label MAP naming a uid no unit has is SM03; a `grounds`
      STRING that is neither a uid nor a label is SM03; a MAP with `kind`
      and `from` but no `to` is SM03 `relation lacks to`; a unit MAP with
      a key §S2 does not name is SM03 naming it.
    - `smysl.write` of a library holding a labelled unit whose payload has
      a `status` key, written by `smysl.unit`, a relation naming it, a
      unit grounded on it, and a unit whose source has an unknown kind,
      written through SMYSL's API, leaves out the two units and the label,
      and reports a `Warning` counting 2 units and one counting 1 label,
      and no other. Its text, parsed and appended to a fresh library,
      succeeds: every uid but the two omitted units' is present, the
      relation and the grounded unit among them, referring to the omitted
      unit by uid. A library binding one label to two units, built through
      SMYSL's API, is written with neither unit labelled and a `Warning`
      naming the label, and its text, parsed and appended to a fresh
      library, answers the same uids. A library Bund2 wrote with
      `smysl.unit`, `smysl.rel` and `smysl.label` alone, its units each
      with a surface form and one label, and no text, draws no `Warning`.
    - `smysl.parse` of a document that holds a `supersedes` relation and a
      withdrawal of it answers the relation, does not count it among the
      left-out relations, and reports one `SMY-W056` `Warning`, naming the
      withdrawing agent and `supersedes`; with withdrawals of it by two
      agents, two; with two identical withdrawals, one.
    - `smysl.prove` of a proof using `[ rel "rebuts" X C ]` from a
      hand-built KB, where the library's `rebuts` edge from `X` to `C` is
      withdrawn, writes a `claim` for that fact as a program fact. So does
      a proof using `[ grounds U G ]` from a hand-built KB, where `U` and
      `G` are held and `U` has no ground `G`; one using `[ label L U ]`,
      where the library binds `L` to `U`, writes nothing for that fact.
      Nor does one using `[ grounds U G ]` from `smysl.kb`, where `G` is
      superseded and held, though no scope of current versions selects
      `G`, nor one using `[ grounds U G ]` from `smysl.kb`, where `G` is a
      uid the library does not hold. A hand-built
      `[ effective U "unfounded" ]` for a unit no retraction touches, and a
      hand-built `[ unit U S "cited" ]` for a `speculative` `U` whose
      schema is `S`, are each written as a program fact.
    - `smysl.rel` of a new edge whose rid a withdrawal already names,
      built through SMYSL's API, answers its rid and reports the
      withdrawn-edge `Warning`.
    - `smysl.write` of a library holding a relation withdrawn through
      SMYSL's API, by a `Withdrawal` whose clock's agent is the
      withdrawing agent, and a text added by `smysl.text.add`, writes the
      relation with its withdrawal and the text's manifest when
      `has_surface_form` admits it, counting it in a `Warning` otherwise.
      `smysl.parse` of that text answers no relation MAP for the
      withdrawn edge and reports the `Warning` counting it. A withdrawal
      built with another clock agent, the edge's only one, is counted as
      left out, and the `Warning` names its edge. Beside a second
      withdrawal of that edge built with the withdrawing agent's clock, it
      is counted, and the edge is not named.
    - A unit MAP with `payload: {}` gives the uid of the same MAP without
      `payload`, and reads back without the key.
    - `smysl.rel` of an edge the library holds, with the same weight,
      through a handle whose agent has attested its rid, writes nothing
      and answers its rid; with another weight, or another note, it is
      SM03 and the library is unchanged. Of a withdrawn edge, with the
      same weight, it answers the rid, reports the `Warning`, and the
      edge stays withdrawn. `smysl.append` of a LIST holding that
      relation's MAP reports the same `Warning`.
    - `smysl.parse` of a document with a comment and an authored salience
      reports one `Warning` for each, and answers the same LIST as
      without them.
    - A label MAP `{ label, uid }` in `smysl.append` writes the
      `LabelBinding` `smysl.label` writes; a unit MAP with `labels` is
      SM03; a `grounds` entry naming an unbound label is SM08 with `0`;
      `smysl.parse` of a document that labels a unit gives a label MAP
      whose `uid` equals the uid `smysl.append` answers for that unit.
    - `smysl.rel` and `smysl.label`, each alone, write what the same record
      written through `smysl.append` writes. `smysl.rel` given a
      `retracts` relation is SM03 [R23]. `smysl.label` of a label bound to
      another unit is SM08 and writes nothing; of a label bound to the same
      unit, it writes nothing. A `smysl.append` LIST with two label MAPs
      binding one label to two units is SM08 and writes nothing.
    - `smysl.text.add`'s `lossy` and `redacted` equal what `Library::add`
      answers for the same text added through SMYSL's API to a twin
      library, including a text with a part the library has redacted, so
      that `redacted` is not 0. No word redacts, so the test redacts that
      part through SMYSL's `Library::redact` (`smysl-text/src/library.rs:713`). The sidecar is current afterwards: its
      stamp matches the library's (`smysl-text/src/library.rs:390-398`).
    - `smysl.create` and `smysl.open` of the same new root on two threads,
      repeated, never answer anything but a handle, SM01, or SM02 naming
      the layout.
    - `bund2 check` reads a program using the `smysl` conditional, and one
      using `smysl.prove`, with no unknown-effect diagnostic, and takes
      `smysl.prove` as 4 → LIST.
    - `features_built_in` names `smysl` in the `smysl` build and not in the
      default one.
    - Under `--noio`, `smysl.parse` works, and so does `smysl.agent`, on a
      handle whose root does not exist; every other
      `smysl.` word is SM06. Each milestone tests the stubs of the words it
      registers (§Implementation plan). So is the constructor `smysl`, and so is a
      CONDITIONAL of type `smysl` built with `conditional` and run, which
      reads nothing.
    - The function that maps a measure result to §S5's MAP, given a result
      with `NotPersisted::Unwritable` (`smysl-measure/src/lib.rs:408-417`),
      gives `uid` NODATA, keeps the diagnostic's own severity among
      `diagnostics`, and reports one `Warning`.
    - The function that reports a run's `warnings`, a measure's
      diagnostics and a failed `write_index` gives one `Warning` per warning
      (D36), driven with a diagnostic of each severity, since which
      queries warn is SMYSL's to change.
    - A source whose `observed` is above `i64::MAX`, written from its
      STRING form, reads back as that STRING.
    - Data a later SMYSL may write reads back: a source of an unknown kind
      code, built with `SourceRef::with_unknown_kind`, with `kind` as that
      INT and `opaque = true`; an extension relation kind as its text; and
      a relation with a non-empty `extra` with `opaque = true`, through
      the mapping function, since no Phase 1 word reads a relation back.

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
  - *Revision 10:* "torn" means a truncated tail; any other undecodable
    tail fails the open (§Current). R25's snapshot now compares a torn
    log's tail byte for byte.
  - *Corrected by revision 9.* The failing shard may also hold whole
    records of its batch: a short write, or a write whose `sync_data`
    failed, leaves them in the file (`smysl-text/src/shards.rs:380-384`).
    Since R25's amendment such a write keeps the locks, so the operator
    still finds the library locked.
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
  - A write opens twice (§S9). Over a library holding a torn tail, R25's
    snapshot reads every log a third time, to find which is torn; without
    one, it reads only the logs' lengths. The measurement should include
    a torn library.
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
      `with_entities` (`:97-106`), over `Library::store()`
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
  - *Corrected by revision 9:* six, not five. `measure` writes unless
    given `--dry-run`, and has no `--save`; `ingest --text` also appends
    unlocked (§Current, "The locks").
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
| R22 | SMYSL's unlocked writers | *Raised by revision 8 (D245).* **A** Phase 1 proceeds; its documentation says a library a Bund program writes must not be written at the same time by `q --save`, `ask --save`, `measure --save`, `assess` or `same-as`; U9 is asked of SMYSL. **B** Phase 1 waits for U9, and pins a SMYSL revision that has it. *Amended 2026-11-03 (revision 9):* the commands are `q --save`, `ask --save`, `measure` and `assess` unless given `--dry-run`, `same-as` and `ingest --text`; the ruling is otherwise unchanged | **A** — **ruled A** 2026-11-02, list amended 2026-11-03 |
| R23 | `retracts` outside `smysl.retract` | *Raised by revision 8 (D245).* **A** `smysl.rel` and `smysl.append` refuse a `retracts` relation with SM03, naming `smysl.retract`. **B** accept it, and state beside R21 that it bypasses the plan and authority | **A** — **ruled A** 2026-11-02 |
| R24 | Extension relation kinds | *Raised by revision 8 (D245).* **A** `smysl.rel` and `smysl.append` accept any kind `RelKind::parse` accepts, `x.<domain>/<kind>` included, and read them back as text. **B** read them back, and refuse them on write with SM05 | **A** — **ruled A** 2026-11-02 |
| R25 | R20's test for a failed write | *Raised by revision 8 (D245).* R20 A is ruled; this restates when it applies. **A** the locks are kept when the library's stamp grew during the failed call, and dropped when it did not, for `append_records` and `Library::add` alike. **B** revision 6's test: `check_loggable` first, then every later error from `append_records` keeps the locks. Under B, an `add` that SMYSL refuses before writing anything still needs a rule, since its manifest is built inside the call. *Amended 2026-11-03 (revision 9), on the owner's word:* A's test is not the stamp, which misses a short write and a failed sync, but each shard log's length on disk: the locks are kept when any log's length changed during the failed call, or when a log held a torn tail before it (an append cuts one first, so length cannot see what replaced it), and dropped otherwise. The intent is unchanged. *Amended again 2026-11-03 (revision 10), on the owner's "A":* the torn-tail clause compared totals, so a refusal that wrote nothing kept the locks over any torn library. Each torn log's tail is now snapshotted and compared byte for byte, the marker is compared too, the snapshot is taken once per acquisition, and a snapshot that cannot be taken keeps the locks. The locks are kept exactly when a log or the marker changed | **A** — **ruled A** 2026-11-02, test amended twice 2026-11-03 |

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

## Revision 9: the seventh review answered

| Finding | Answer |
|---|---|
| S1 the stamp misses whole records | §Current states what the stamp is computed over. R25 is amended on the owner's word (D245): the guard compares each shard log's length on disk, and any change keeps the locks, as does a torn tail present before the call. §S9; criterion 15's changed-log case; Q51 corrected |
| S2 R22's list | `measure` and `assess` unless given `--dry-run`, and `ingest --text` added: six commands. §Current, §S9, U9, the Preservation row and R22 (amended, D245 noted); Q57 corrected |
| S3 the re-open case in M3 | Moved to M4, where `smysl.append` exists |
| S4 the probe in CI | Two CI steps, a nextest run and a clippy run with `smysl-panic-probe`, landing in M3. The unlock work is a `bund2-stdlib` function the subcommand calls, so the probe needs no forwarding; a CLI test covers the subcommand |
| S5 `--noio` and the conditional | The handler is stubbed too, as D119 does; D119 consumed; SM06, the Preservation row and criterion 18 |
| S6 a measure's `Input` | §S5 names its three steps, with level and analyzer from the base `Recipe`; criterion 10's direct calls use them |
| N1 rule T | §S4's reason names the ceiling and the `Imported` exception |
| N2 `with_entities` | Q55 corrected |
| N3 `check_loggable` | Criterion 15 calls both refusals through the guarded write directly |
| N4 refusal codes | SM11 in M2, SM02 in M3; criterion 18's `bund2 check` split stated |
| N5 views | Criterion 12 builds every view through SMYSL's API |
| N6 re-binding a label | SM08 at bind time for another unit; nothing written for the same unit. §S4, §S8, criterion 18 |
| N7 the demo's library | A fresh temporary directory per run; criterion 17 |
| N8 SM02 for `SMY-E440` | §S8 says Bund2 writes it |
| N9 no basis | `uid` is NODATA and `SMY-W483` is reported |
| N10 the stamp grows without Bund2 | Stated beside R25, for the on-disk comparison |
| Preservation | Rows for the wall clock and the mutex table; `--noio`, concurrent use and the crash notice corrected |
| Assumptions 1–7 | 1 is S1. 2 is S2. 3 is S4. 4 is S5. 5 is stated in §S6. 6 is N7. 7 is N6 |

## Revision 10: the eighth review answered

The owner chose A for each of S1–S3.

| Finding | Answer |
|---|---|
| S1 `smysl.create` and the locks | A: `Library::create` outside SMYSL's locks and under the process mutex, as SMYSL's commands create before they lock. The empty-marker race is stated as SM02. U10 asks SMYSL for an atomic marker. §S4, the plan's M3 note, a Preservation row, criterion 18 |
| S2 the torn-tail clause over-locks | A: R25 amended again (D245). Torn logs' tails are snapshotted and compared byte for byte, so a refusal that writes nothing leaves no lock over a torn library. §S9; criterion 15's snapshot and torn-refusal cases |
| S3 SM02's text | A: `SMYSL refused it: <SMYSL's Display>`, with the "left locked" suffix when R25 keeps the locks. The over-cap refusal calls SMYSL's own `Budget::new`. §S8; criterion 15 |
| N1 R25's files and errors | `shards::list`; a snapshot that fails is SM02 before the call and keeps the locks after one |
| N2 the torn-tail clause omitted | The guard's description and the crash-notice row name the full test |
| N3 the layout | The marker is in the snapshot; a Preservation row; U10 |
| N4 torn means truncated | §Current corrected, with `from_cbor_seq`; Q51 noted |
| N5 CI counts | M1 lands four steps; M10 names the demo step; §S1's no-C-compiler step is criterion 3's `cargo build` |
| N6 the torn-tail case's call | Criterion 15 tests the snapshot directly, and a refusal over a torn library through `smysl.text.add` |
| N7 what a write reports | `smysl.text.add` answers `lossy` and `redacted`; "warnings from appends" withdrawn; criterion 18 |
| N8 the index sidecar | `smysl.text.add` completes and writes it as `text add` does; a Preservation row |
| N9 measure results | `Unwritable` mapped: `uid` NODATA and its diagnostic reported; `NoBasis` noted as unreachable from `resolve` |
| N10 `Transformed` | The unsupported clause dropped |
| N11 labels in one LIST | SM08's bind-time text; two MAPs binding one label is SM08; earlier label MAPs in a LIST resolve later `grounds` and `deps` |
| N12 the stubs' effects | `opaque(0)`, as D119's |
| Assumptions 1–7 | 1 is S1. 2 is S2. 3 is S3. 4 is N1. 5 is stated: the test is exact outside R22's hazard. 6: the snapshot is taken once per acquisition. 7 is stated in §S9's "if nothing changed" |

## Revision 11: the ninth review answered

| Finding | Answer |
|---|---|
| S1 a held lock mapped two ways | SM07 only. SM02's example is `SMY-E402`; SM07's text is SMYSL's `Display` of `Locked`; criterion 15 asserts both |
| S2 `smysl.create`'s mutex | `create_dir_all`, canonicalise, take the mutex, then `Library::create`. §S2, §S4 |
| S3 the label MAP | `{ label, uid }`, the one MAP with a given `uid`; `smysl.parse` emits one per entry of SMYSL's resolved label map; a unit MAP's `labels` is read-back only, SM03 on write; an unbound label is SM08 with `0`. §S2, §S4, §S8, criterion 18 |
| S4 unclaimed cases | M3 claims the create race; M4 the torn-refusal, SM02-text, label-MAP and `smysl.text.add` reporting cases; M6 the `Unwritable` case |
| N1 `ShapeError`'s `Display` | Cited at `error.rs:179-192` |
| N2 `--max-input` | The over-cap error is displayed with `flag` set to none |
| N3 an empty marker for good | Stated, with the cross-process case; U10 would close both |
| N4 `Unwritable` untested | Criterion 18; its severity is kept in the MAP and reported as a `Warning` |
| N5 the first call | Stated: a refusal by a later call keeps the locks if an earlier call wrote |
| N6 the open iterates `list` | Cited at `library.rs:332` |
| N7 the constructor under `--noio` | Stubbed, as D119 stubs `csv` |
| N8 redaction in the test | Through SMYSL's `Library::redact` |
| N9 the snapshot in the steps | §S9's steps 6–8 |

## Revision 12: the tenth review answered

| Finding | Answer |
|---|---|
| S1 labels given twice | `smysl.parse` reads the `LabelBinding` records only; parsed unit MAPs have no `labels`. §S2, §S4, criterion 5 |
| S2 a MAP's kind | Decided by its keys: `label`, then `kind`/`from`/`to`, then `gist`, else SM03. §S2, criterion 18 |
| S3 parse errors | Any error diagnostic or recovered record refuses the document with SM02 carrying the diagnostics; the `@doc` header's hard error too. §S4, §S8, criterion 5, a Preservation row |
| S4 `bund2 check` | The constructor leaves `Conditional("smysl")`, as `query`'s does; D181 consumed. The checker's empty-`lambda` reading is stated |
| N1 SM08's "When" | Includes a label bound to none |
| N2 other caps' flags | Every `Limit` is displayed with `flag` none |
| N3 first raised | SM02 in M2, SM08 in M3 |
| N4 `--noio` stubs | Each arrives with its word |
| N5 the lock's command | `bund2 <word>` |
| N6 canonicalisation | SM01 |
| N7 a second label | Stated in `smysl.write`'s row, and a Preservation row |
| N8 a label's `extra` | Stated; a non-empty one reads back `opaque` |
| N9 the ninth review's leftovers | Its Preservation items are rows of revision 11; the third read over a torn library is under Q54 |
| Assumptions 1–8 | 1 and 2 are S1. 3 is S2. 4 is stated in §S2. 5 is S3. 6 is S4. 7: a label's `uid` must name a unit, SM03 otherwise. 8 is N6 |

## Revision 13: the eleventh review answered

| Finding | Answer |
|---|---|
| S1 units with no surface form | `smysl.write` omits what SMYSL's writer omits and reports a `Warning` per kind, as SMYSL's CLI does; payload keys named as header fields are accepted and stated. §S2, §S4, criterion 18, a Preservation row |
| S2 `smysl.agent` and canonicalisation | `smysl.agent` neither canonicalises nor checks the root; "every word" is narrowed to the words that open the library or take the mutex. §S2, criterion 18, the `--noio` row |
| N1 the two E060 paths | Both cited; `has_errors` and `fmt` cited as the precedent. Criterion 5 gains a ninth document |
| N2 three hard errors | `SMY-E001`–`E003`, all from the `@doc` header |
| N3 SM02's parse text | SMYSL's `Diagnostic` `Display`, span and suggestion included; a refused document's warnings are reported first |
| N4 comments and salience | A `Warning` each from `smysl.parse`; the Preservation row names them |
| N5 §S9 and §S6 | `smysl` has a fixed effect; its handler re-enters |
| N6 the kind rule's gaps | Unknown keys are SM03 on every kind; step 4 names a missing `to` or `gist`; SM03's "When" lists the new cases |
| N7 a label bound to several units | Written on none of them, with a `Warning`; references as uids |
| N8 `create_dir_all` and SM01's text | SM01, with `cannot reach <root>: <the I/O error>` for any failure but a missing path |
| N9 the label grammar | `ids.rs:442-466` |
| N10 the label row | `opaque` only when read back with an `extra` |
| N11 Q54's register row | A dated note |
| N12 `--noio` stubs per milestone | Each milestone tests its own words' stubs; M10 completes the case |
| N13 the primary-shard fixture | A hand-made `.primary` marker, stated in M2 |
| N14 parsed, then refused | Stated in §S4 and the Preservation row |
| Assumptions 1–8 | 1 and 2 are S1. 3 is S2. 4 is stated in `smysl.write`'s row. 5 is N6. 6 is N3. 7 is N7. 8 is N8 |

## Revision 14: the twelfth review answered

| Finding | Answer |
|---|---|
| S1 re-writing a relation | A rid is `(kind, from, to)`, stated in §Current. The same weight and note writes nothing; another is SM03 and writes nothing. `smysl.write` passes one relation per rid. §S4, criterion 18, a Preservation row |
| S2 `smysl.write`'s labels | Built per uid from labels binding that uid alone, for units with a surface form only, so an omitted unit is referred to by uid; the departure from `merge` is stated. Criterion 18 re-parses the text |
| S3 already retracted | SMYSL's `is_retracted`: retracted directly, not orphaned. Criterion 8 gains the orphan case; a Preservation row |
| N1 the count | One `Warning` per kind of loss, citing `SMY-W428`; the CLI's single total stated. `smysl.write` passes units, relations and the admitted labels only, so attestations are never counted |
| N2 a relation's `extra` | Dropped by `smysl.write` and counted; the relation read-back is reached through the mapping function only, stated |
| N3 an empty payload | No payload, stated in §S2 |
| N4 a measure's `value` | SMYSL's public `gist`, uncut; it can differ from the unit's cut gist, and exists without a unit |
| N5 the primary marker | `log/shards/<stem>.primary`, through `primary_marker` |
| N6 criterion 14's `smysl.text.add` cases | Placed in M4 |
| N7 `merge`'s first-wins | The departure is stated in §S4 and the Preservation row |
| N8 the handle's root | Canonical when `smysl.create` or `smysl.open` builds it; a hand-built root is re-canonicalised by the words that open it |
| Assumptions | Each is answered by the finding it names |

## Revision 15: the thirteenth review answered

| Finding | Answer |
|---|---|
| S1 records `smysl.write` did not pass | It passes every record but attestations, with relations one per rid, so SMYSL's writer writes manifests, withdrawals and the rest; what has no form is counted per kind by `has_surface_form`. `smysl.parse` leaves out an edge its document withdraws, since the withdrawal is skipped. Criterion 18, two Preservation rows |
| S2 `smysl.kb` and withdrawn edges | No `rel` fact for a withdrawn edge, as SMYSL's interpreting reads. §S7, criterion 11, M8 |
| N1 the re-scoped citation | `smysl-core/src/types/relation.rs:179-181,205-208`, a bullet of its own |
| N2 "no read" | No interpreting read; `Store::iter` and `merge` still carry it |
| N3 SM03's row | Lists the held-edge case; it stays SM03 |
| N4 a withdrawn edge re-asserted | Stays withdrawn, with a `Warning`; criterion 18 names the agent |
| N5 no way to change a weight | Cited: the rid, and `withdraw` by rid |
| N6 the handle's root | Must be absolute (SM00); canonical when Bund2 built it; D244 carries a dated note on R3 |
| N7 the Preservation row | One total from the CLI; per kind is `SMY-W428`'s |
| N8 "already read as retracted" | "Retracted directly" in §Current and criterion 8 |
| N9 `value` | "Its headline when it has one"; criterion 10 compares with `gist` and tells the cut apart |
| N10 the no-`Warning` fixture, `payload: {}` | The fixture is named; an empty-payload case is added |
| N11 label bindings passed | Labels reach the text through the context alone, stated |
| Assumptions 1–6 | 1 is S1. 2 is S2. 3 is stated in §Current. 4 is N4. 5 is N6. 6 is N9 |

## Revision 16: the fourteenth review answered

| Finding | Answer |
|---|---|
| S1 lifecycle edges | Every withdrawn-edge rule is `is_withdrawn`'s: a `supersedes` or `retracts` edge is never withdrawn. `smysl.parse` answers such a relation and reports `SMY-W056`'s meaning, so a `retracts` relation still reaches SM03. §Current, §S4, criterion 18 in M4, the Preservation row |
| N1 seven bare citations | Each written with its full path |
| N2 two re-grounded claims | `absorb_hot`, `absorb_hashed` and the open's shard order; `shards.rs:516-522` |
| N3 where withdrawals come from | `withdraw` acts on a store file; a library gains them through the API or a command that opens it, not established which |
| N4 a withdrawal with no surface form | Its edge is written live and named in the `Warning`; criterion 18 builds its withdrawal with the withdrawing agent's clock, and adds the other case |
| N5 "reads back withdrawn" | SMYSL's parser; a Bund2 copy leaves the edge out. The Preservation row says so |
| N6 a second variant | Counted in the `Warning` |
| N7 where relations go | In the place of each rid's first record, keeping the log's order |
| N8 a relation arriving withdrawn | The `Warning` covers every relation the word answers that `is_withdrawn` reports; criterion 18 |
| N9 the absolute root | In §S2's validation bullet; criterion 14 in M2; criterion 15's spellings both absolute |
| N10 `smysl.prove`'s `rel` facts | A `rel` fact stands for its units only over a held, live edge, else it is a program fact; criterion 18 in M9. Criterion 11 counts within the scope |
| Assumptions 1–7 | 1 is S1. 2 is N4. 3 is N3. 4 is N6. 5 is N7. 6 is N8. 7 is N10 |

## Revision 17: the fifteenth review answered

| Finding | Answer |
|---|---|
| S1 `smysl lib import` | §Current names it, with `smysl lib accept`, as a SMYSL command that withdraws an edge in a library; §S4 says only that Bund2 has no word that withdraws one. This settles the fourteenth review's N3. The Preservation row says so |
| N1 "reads back live" | An edge is named only when the library holds it, `is_withdrawn` reports it, and none of its withdrawals has a surface form; any other left-out withdrawal is only counted. Criterion 18 adds a second withdrawal with a surface form |
| N2 the withdrawn-edge bullet | Its own paragraph after the held-edge list; grounded in withdrawals kept by rid and `is_withdrawn`'s test; it names `smysl.rel` and `smysl.append`, one `Warning` per relation. Criterion 18 adds the `smysl.append` case |
| N3 criterion 14 in M2 | The relative-root case is `smysl.agent`'s in M2 and `smysl.texts`'s in M4 |
| N4 other fact shapes | Every shape stands for its units only when it is a fact `smysl.kb` would load for a scope of exactly its uids; else it is a program fact. Criterion 18 adds a `grounds` and a `label` case in M9. D244 R8 A, provenance by shape and library, is unchanged |
| N5 `SMY-W056`'s granularity | One `Warning` per withdrawal, naming its agent and the edge's kind, as SMYSL's check does; criterion 18 |
| Assumptions 1–5 | 1 is S1. 2 is N1. 3 is N4. 4 is N2. 5 is N5 |

## Revision 18: the sixteenth review answered

| Finding | Answer |
|---|---|
| S1 the stand-for test's scope | §S7 states the test per shape, with its uid positions named and no scope in it; a superseded unit the library holds counts as held. `smysl.kb` resolves its scope in §S5's environment, by `PlanOptions::default()` and `Answer::uids`, and says what `ranked`, `groups` and `count` answers give. Criterion 11 fixes the environment; criterion 18 adds a superseded-ground case in M9 |
| N1 three shapes of seven | Criterion 18 adds an `effective` and a `unit` case in M9 |
| N2 `SMY-W056`'s count | One per distinct withdrawal of a `supersedes` or `retracts` relation the document holds; a withdrawal of an undeclared rid draws none. Criterion 18 adds the identical-withdrawal case |
| N3 a Preservation row | "A fact a proof uses" is its own row; the withdrawn-edge row keeps to edges |
| N4 D244 R8 | A dated note on R8 records revision 17's and 18's reading of "provenance by shape and library"; the ruling is unchanged |
| Assumptions 1–4 | 1 is S1. 2 is stated in §S7's table. 3 is S1. 4 is N2 |

## Revision 19: the seventeenth review answered

| Finding | Answer |
|---|---|
| S1 an unheld ground | Option (a): only the units a fact stands for, `U`, `From` and `To`, must be held; `G` and `D` need only be among `U`'s grounds or deps, as SMYSL keeps a unit whose ground is absent. The table's rows say "held or not". Criterion 18 adds an unheld-ground case in M9. D244 carries a further dated note |
| N1 at write time | §S7 and the Preservation row say a loaded `rel` fact is a program fact if its edge was withdrawn since; the other shapes stay borne out |
| N2 revision 16 misstated | The Preservation row says revision 16 already asked a `rel` fact's edge to be held and live; D244's further note corrects the earlier one |
| N3 facts in scope | Every row of §S7's table but `rel` and `rank` says "per unit in scope" |
| N4 ranked in a superseded library | §S6 says a ranked or packed query answers `"scope"` once the library holds a supersession; criterion 9's fixture holds none, and a second fixture holds one |
| N5 the call, and `S` | `scope::resolve` is named, and its refusal, `SMY-E460` included, is SM04; criterion 18's `S` is `U`'s schema |
| Assumptions 1–5 | 1 is S1. 2 is N1. 3 is N3. 4 is N4. 5: a uid that is not a unit gives no fact, and M8's first step establishes what a `from texts` answer names |

## Revision 20: the eighteenth review answered

| Finding | Answer |
|---|---|
| S1 packed queries | A packed query keeps its kind, since `Packed` is built after the supersession pass; §S6 and criterion 9 say so. The revision 19 answer table's N4 row stands corrected here |
| S2 `effective` lapsing | §S7 says an `effective` fact stays borne out under later retractions, and can lapse when an absent ground is appended since; the Preservation row and a further D244 note say so |
| S3 `resolve`'s warnings | Option (a): `smysl.kb` takes `parse`, `plan` and `run` itself, reporting `run`'s warnings as `smysl.query` does; criterion 11 adds a forked-supersession case. No saved-query unit is built, so `SMY-E460` from it no longer arises |
| N1 traversals and closures | §S6 cites them; criterion 9's ranked query has neither |
| N2 unheld uids in a scope | Left out of the scope; the `grounds` closure is cited; criterion 11 adds the case |
| N3 fact order | The answer's order, `rank` facts last |
| N4 "does not hold" | "a ground or dependency `U` does not have" in §S7 and the Preservation row |

## Revision 21: the nineteenth review answered

| Finding | Answer |
|---|---|
| S1 when an answer changes kind | §S6 says the kind is the one `run` gives, names the supersession pass, traversals and closures, set operations, ordering stages and a `limit` after `pack` as examples, and lists no stage as complete; a program reads `kind`. Criterion 9's queries avoid every such stage |
| S2 the conditional and a uid with no unit | Not visited, as `smysl.kb` leaves it out; `smysl.query`'s MAP keeps it. Criterion 9 adds the case |
| N1 `run` refuses | "`parse`, `plan` or `run`" in §S6 |
| N2 fact order | No order is promised; the revision 20 answer table's N3 row stands corrected here |
| N3 any writer | §S7 says any writer may append the ground |
| N4 M8's first step | The M8 row names it |
