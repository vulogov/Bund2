# RFC-0017: Frames, factors and missing values

- Status: **Accepted** (2026-11-18) by the repository owner ("go-ahead
  for revision 11, then marking it Accepted"), at revision 11.
  Accepting means D253–D258 as ruled, with their dated notes; S1–S8 as
  specified, each **[author]** choice listed under §Open questions
  included; the milestones M1–M4 built in order, each gated by its
  criteria and by criteria 9 and 10; and S3–S6's names as Bund2's own
  (D72), as RFC-0015's and RFC-0016's families are.
- Revision 11 (2026-11-18). Revised after eleven reviews
  (`docs/rfc/reviews/RFC-0017-review-2026-11-18-4.md` of revision 10,
  `docs/rfc/reviews/RFC-0017-review-2026-11-18-3.md` of revision 9,
  `docs/rfc/reviews/RFC-0017-review-2026-11-18-2.md` of revision 8,
  `docs/rfc/reviews/RFC-0017-review-2026-11-18.md` of revision 7,
  `docs/rfc/reviews/RFC-0017-review-2026-11-17-7.md` of revision 6,
  `docs/rfc/reviews/RFC-0017-review-2026-11-17-6.md` of revision 5,
  `docs/rfc/reviews/RFC-0017-review-2026-11-17-5.md` of revision 4,
  `docs/rfc/reviews/RFC-0017-review-2026-11-17.md` and
  `docs/rfc/reviews/RFC-0017-review-2026-11-17-2.md` of the first draft,
  `docs/rfc/reviews/RFC-0017-review-2026-11-17-3.md` of revision 2,
  `docs/rfc/reviews/RFC-0017-review-2026-11-17-4.md` of revision 3).
  Q64 is answered from R's source (§Differences from R).
  Every owner question is ruled: D253's P1–P12 (all A), D254 (a quoted
  CSV field is text; a mixed numeric column keeps each cell's type), D255
  (a non-finite CSV field is refused), D256 (the first review's S1–S8),
  D257 (the second review's B2, B3, S6, S9, S10), D258 (the third
  review's B1, B2, S2). A passage that rests on a ruling is marked with
  it: **[P*n*]** for D253, **[D254]**–**[D258]** for the rest. A choice
  no ruling names is marked **[author]** and listed under §Open
  questions.
- Note (2026-11-18, after acceptance, during M2): `frame.with` on a
  frame of no columns takes a LIST of any length, and the new column
  sets the row count, by the owner's ruling ("letting frame.with take a
  list of any length when the frame has no columns, so the new column
  sets the row count"). S4's row, its paragraph on names, S8's length
  row and criterion 4 say so; nothing else moves.
- Note (2026-11-19, after acceptance, during M3): a frame does not
  round-trip through JSON, by the owner's ruling ("option 1"). S2 said
  `json.from_value` writes a frame as an object; it refuses one, since
  `columns` is a LIST of STRINGs and the reference's converter has no
  STRING arm, falling to `This Dynamic type is not supported for JSON`
  (`reference/rust_dynamic/src/cast_value_to_json.rs:9-84`), which
  Bund2 preserves (`crates/bund2-value/src/lib.rs`, `to_json`, the `_`
  arm, which `json.from_value` calls).
  S2's paragraph and criterion 7 say so; `json.from_value` is unchanged,
  and nothing else moves.
- Note (2026-11-19, after M4, on the twelfth review's SF2–SF4 and N1–N5,
  `docs/rfc/reviews/RFC-0017-review-2026-11-19.md`): wording only, and
  the code is unchanged. Changes:
  - S8 gains the texts M4 wrote that no row gave. These are a `stats`
    entry malformed or naming no column, a result name repeating
    another, a statistic or `frame.matrix` on a string or logical
    column, and the join cap's `<what>` (SF2).
  - S5 states `frame.join`'s and `frame.matrix`'s refusal order, and
    which NONE `"refuse"` names first (SF3).
  - Criterion 10 and the Preservation row say no *name* line of
    `PROMOTABLE.txt` moves, since its count line changes with every
    name added (SF4).
  - Criterion 7 says the refusal *ends with* the quoted text (N1).
  - Criterion 5 names the factor in its no-rows case as the `by` column
    (N2).
  - S5 drops the clause on a result name equal to `.dropped`, which
    cannot occur (N3).
  - §Current behaviour is marked as the tree before this RFC (N4).
  - It is recorded that M3's commit (672f453) left `PROMOTABLE.txt`
    stale, so criterion 10 held again only at M4's (9b71007) (N5).

  The ruling behind S1's "every refusal is listed in S8" is unchanged;
  the table now meets it. The review's SF1, a dated note on D252 that
  `json` does not carry a frame, is the owner's to record in
  `docs/registers/decisions.md`.
- Note (2026-11-19, after M4, on the twelfth review's assumptions A1–A6,
  `docs/rfc/reviews/RFC-0017-review-2026-11-19.md`): the code is
  unchanged. Each behaviour below was there in M4 but not written down;
  it is now stated where it applies, and each was probed again on
  `target/debug/bund2` before writing.
  - A1: a `by` column that a statistic also reads is read under `na` as
    that statistic's column (S5, Groups).
  - A2: a statistic's column of no kind is accepted with rows too, and so
    is a `frame.matrix` column of no kind (S5).
  - A3: BOOL keys match by value (S5, `frame.join`'s Keys).
  - A4: `.dropped` is added under `"omit"` even when every statistic is
    `"count"` (S5, `"omit"`).
  - A5: a LIST literal cannot hold NONE (S3).
  - A6: `vm.spawn` carrying a frame was never probed. It is not a claim
    of this RFC, which names only `json` and the wire format (S2); the
    claim is D252's, and D252's dated note of 2026-11-19 records it as
    unverified.
- Drafted 2026-11-17 on the owner's `/rfc-draft 0017`, after RFC-0016
  (research 07's Phase A) landed. The scope is research 07's Phase B
  (`docs/research/07-statistical-research.md:217`, §3.1 at `:85-107`):
  frames, factors and missing values, and reading a CSV into a frame and
  writing one out.
- Depends on: RFC-0015 (the `biomath` conventions: S3's forms, S11's
  refusals E101–E115, the operand readers, the lambda bridge), RFC-0016
  (§S1's options-MAP rules, D251), RFC-0004 (effects)
- Decisions consumed: **D252** (C1 ruled B, C2 ruled C, and its three
  dated notes), **D253**–**D258** (with the dated notes beside D254, D255,
  D256, and two beside D258), D21, D24, D32, D33, D36, D37, D39, D51, D72, D106, D108, D111,
  D119, D250 (B12, B15), F76.
- Reference SHA: `reference/rust_multistackvm`
  4605832678d4792c2b8e0564b7757d5aee1539a4; `reference/Bund`
  21b40b0213a76f3f39bd6de0d2bdfdf0a53b0dca; `reference/rust_dynamic`
  ceb27c96fa10c4edb09106f3f904f22f94d63119. Bund2's own code is cited by
  file and symbol, at `development/0.56.0` after f4a780c; the `csv-core`
  crate at 0.1.13, as the lock pins it.
- Supersedes: nothing. Grounds and refines research 07's Phase B (§4);
  departs from §3.1's "`"dose" get`" convention, recorded in ERRATA.

## Summary

A **frame** is a MAP with three reserved keys, as D252 ruled: `columns`,
an ordered LIST of names; `data`, a MAP from each name to a LIST, all of
one length; and `levels`, a MAP from each factor column's name to its
ordered LIST of levels. A **missing value** is NONE, the value a JSON
`null` already converts to. This RFC proposes the words that build, check,
read, reshape, summarise and print frames (`frame.`), the words that make,
test and remove missing values (`na`), and a CSV parser and writer that
work on STRINGs, so that the existing `file` and `file.write` remain the
only words that touch the file system. No existing word changes; both
health numbers are unchanged (D72).

## Motivation

Statistical work starts from a table of named columns of equal length,
some categorical, some with gaps (`docs/research/07-statistical-research.md:90-107`).
Bund2 has none of the three:

- **No table.** A MAP's keys sort: `BundValue::map` takes a `BTreeMap`
  (`crates/bund2-value/src/lib.rs`, `map`), so `dict "b" 1 set "a" 2 set
  println` prints `{ a=2 ::  b=1 :: }` (probed), and a MAP of columns
  loses their order. The reference's MAP order is not even stable between
  runs (F76). A program that wants a table today keeps the order itself.
- **No missing value that survives a CSV.** `csv`'s reader turns an empty
  field into a null and leaves the null **out of its row**, as the
  reference skips every polars value it has no arm for
  (`reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:90`, `reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:144`; Bund2's `crates/bund2-stdlib/src/data.rs`, the module doc and
  `run_csv`). A column with a gap arrives shorter, and its rows no longer
  line up with the other columns'.
- **No factor.** Nothing records that a column of STRINGs is categorical,
  or the order of its levels, which a model needs for its contrasts
  (research 07 §3.2, Phase C).

## Current behaviour

This section describes the tree this RFC was drafted against, before M1.
Two of its statements about Bund2 no longer hold at M4 (9b71007), both
by design. M4 corrected `describe.rs`'s module doc on the two means
(S5), and the `frame.` and `na` words now exist (Note, 2026-11-19).

### What the reference does

- **`csv`** pulls a file name, checks the file exists, and pushes a
  CONDITIONAL of type `csv` naming it
  (`reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:9-29`;
  registered at `reference/Bund/src/stdlib/functions/conditional/mod.rs:42`,
  its runner at `reference/Bund/src/stdlib/functions/conditional/mod.rs:26`).
  Run with `!`, it reads the file through polars with a header by default,
  the key `is_header`
  (`reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:44-47`, `reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:62`, `reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:122`), and either pushes one column as a
  LIST and runs the lambda once (`reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:61-120`, `column` set), or pushes
  each row as a LIST and runs the lambda per row (`reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:121-169`). A cell is
  pushed only when polars holds it as a STRING, an Int64, a Float64 or a
  Boolean (`reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:77-89`, `reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:131-143`); any other value, a null included,
  is skipped (`reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:90`, `reference/Bund/src/stdlib/functions/conditional/conditional_csv.rs:144`).
- **A JSON `null` converts to NONE.** `cast_json_to_value` answers
  `Value::none()` for a null (`reference/rust_dynamic/src/cast_json_to_value.rs:38-39`),
  inside an array or an object too (`:49-69`, `:70-94`), and `none()` is
  `Value::new()` (`reference/rust_dynamic/src/create_special.rs:19-21`),
  whose type is NONE and whose q is 0.0
  (`reference/rust_dynamic/src/value.rs:33-39`); NODATA is a separate
  constructor (`reference/rust_dynamic/src/create_special.rs:7-18`).
- **LIST indices count from 1.** `Value::at` answers `raw_list[(n-1)]`
  (`reference/rust_dynamic/src/carcdr.rs:57-60`); `1 [ 10 20 30 ] at
  println` prints `10` on Bund2 (probed). Q63 is answered by this.

### What Bund2 does

- **`csv` is the reference's word over the `csv` crate** (D51), which
  infers each column's type as polars 0.46 does: a case-insensitive
  `true`/`false`, then polars' float pattern (which takes `inf` and `NaN`
  with an optional sign, `is_float`), then an integer (an optional `-` and
  digits, `is_int`), else a string; a column that saw one kind keeps it, integers
  with floats are floats, anything else is a string, judged over the first
  100 rows (`crates/bund2-stdlib/src/data.rs`, the module doc, `infer`,
  `column_kind` and `read_frame`). The `csv` crate removes a field's
  quotes before the inference sees it, a difference from polars the
  module doc records. `read_frame` keeps an empty field as `None` per
  cell (`cell`), and `run_csv` then drops it (`flatten`, `filter_map`).
  Probed through `csv`: a column `1.5`, `NaN`, `inf` answers
  `[ 1.5 ::  NaN ::  inf :: ]`, and a column `1`, `2.5` answers
  `[ 1.0 ::  2.5 :: ]`. Under `--noio`, `csv` is a stub (D119).
- **`csv`'s tokeniser is `csv-core`**, which ignores empty lines
  (`csv-core-0.1.13/src/reader.rs:94`; `x\n1\n\n3\n` through `csv` reads
  two rows, probed), strips a leading UTF-8 byte-order mark
  (`Reader::strip_utf8_bom`, `reader.rs:607-619`), takes CR, LF and CRLF
  each as a terminator (`reader.rs:91-92`), never reports an error
  (`reader.rs:86-87`), and reports no flag saying a field was quoted
  (`read_field` answers a `ReadFieldResult`, `reader.rs:263-283`, which
  carries none).
- **`file` decodes lossily**: it answers `String::from_utf8_lossy` of the
  bytes (`crates/bund2-stdlib/src/host.rs`, the `file` reader), so a text
  `frame.csv.parse` receives is always valid UTF-8, and a mis-encoded file
  arrives with U+FFFD in place of its bad bytes.
- **NONE exists, and one word path makes it.** `json.to_value` of a
  `null` answers NONE with q 0.0 (`crates/bund2-stdlib/src/json.rs`,
  `to_value`, the `J::Null` arm); `'[1, null, 3]' json json.to_value
  println` prints `[ 1 ::  NONE ::  3 :: ]`, and `'null' json
  json.to_value type println` prints `None` (probed). D252's finding that
  "no word produces it" is therefore narrower than it reads; D252's dated
  note records the correction, and the ruling is unchanged.
- **NONE round-trips.** `json.from_value` writes it as `null`
  (`crates/bund2-value/src/lib.rs`, `to_json`, the `NONE` arm):
  `'[1.0, null]' json json.to_value json.from_value println` prints
  `[1.0,null]` (probed). The wire format carries it as its own type
  (`crates/bund2-value/src/wire.rs`, the decoder's `NONE` arm). `+`
  appends it to a LIST (`'[1, null]' json json.to_value 1 swap +` prints
  `[ 1 ::  NONE ::  1 :: ]`, probed), as it appends NODATA (D252's second
  dated note).
- **NONE cannot be compared, nor can a container.** `==` with NONE as an
  operand is `== returns error: COMPARE: unsupported operand #1` or `#2`,
  by its position, and so is `==` on two LISTs or two MAPs (probed).
  `BundValue`'s own equality compares a LIST or MAP by identity and an
  INTEGER equal to a FLOAT of the same value (`crates/bund2-value/src/lib.rs`,
  its `PartialEq`).
- **The family refuses NONE**, by either of its readers. A two-series
  word reads each LIST cell by converting it to FLOAT (`operand.rs`,
  `whole`), and NONE does not convert: `'[1.0, null, 3.0]' json
  json.to_value [ 1.0 2.0 3.0 ] stat.cov` is `STAT.COV: E103 a is not a
  LIST of numbers: none` (probed). A one-series word reads through
  `get_data`, whose conversion refuses it in the reference's words:
  `… stat.var` is `STAT.VAR returned: STAT.VAR error FLOAT conversion: Can
  not convert NONE to 3` (probed). B12 stands (D252).
- **NODATA ends a drained series** (`crates/bund2-stdlib/src/series.rs`,
  the module doc: "anything else drains the stack value by value until it
  empties or meets a `NODATA` sentinel"), which is why D252 did not choose
  it.
- **The statistics a group needs** live in three places.
  `stat.mean.arithmetic` (aliased `stat.mean`) is the reference's word,
  in `series.rs` (`amean`); it sums and divides with no overflow check, so
  `[ 1.0e308 1.0e308 ] stat.mean.arithmetic` prints `inf` (probed).
  `math.min` and `math.max` are the reference's too (`series.rs`), and
  fold from NaN. `stat.var`, `stat.sd` (n − 1) and `stat.median` are
  RFC-0015's, in `biomath/describe.rs`. Its module doc says its mean
  "may differ in the last bit" from `stat.mean.arithmetic`'s, but the
  code does not bear that out: `describe.rs`'s `mean` is
  `xs.iter().sum::<f64>() / xs.len() as f64`, the same operations in the
  same order as `amean`'s, so the same bits. `vec.sum` is
  RFC-0015's left-to-right sum, which refuses an overflow with E114
  (`biomath/vector.rs`, `sum`; `[ 1.0e308 1.0e308 ] vec.sum` is `VEC.SUM:
  E114 the answer overflows a FLOAT: Σ xᵢ`, probed).
- **`stat.lm`** takes `X y`, X a MATRIX read by `table` (INTEGER cells
  converted to FLOAT), and refuses unequal lengths with E102
  (`crates/bund2-stdlib/src/biomath/infer.rs`, `stat_lm` and `table`).
- **A lambda run by a word** goes through RFC-0013's bridge on a scratch
  stack, its result read from the top
  (`crates/bund2-stdlib/src/biomath/dynamics.rs`, the module doc and
  `call`); such a word is `StackEffect::opaque` and has its plain form
  only (`dynamics.rs`, `register`).
- **`file` and `file.write`** read and write a file, and `--noio` stubs
  both (`crates/bund2-stdlib/src/host.rs`, the `stub!` lines for
  `file.write`; `file` probed: `bund FILE functions disabled with
  --noio`).
- **No `frame.` or `na` word exists**: `bund2 words` lists none, and no
  existing name has `na` as a dot-separated part (probed).

## Design

### S1. Shape

- **Bund2's own (D72),** in the default build, under two prefixes
  **[P1]**: `frame.` for frames and `na` for missing values. `bund2 words`
  lists each; neither health number moves; no golden is added (D21).
- **Additive, strictly.** No existing word changes. `csv` keeps dropping
  a null cell, its 100-row window, its quote handling, its skipping of
  empty lines and its merging of integers with floats (D51, D252: "`csv`
  is unchanged"). `json.to_value` keeps answering NONE for `null`.
  RFC-0015's and RFC-0016's words keep refusing NONE (B12); RFC-0016
  §S1's note that "D252's future `na` key would be an addition to these
  MAPs" is not taken up here, so no RFC-0016 word gains a key.
- **Refusals** are RFC-0015 §S11's codes; no new code **[P9]**. Every
  refusal is listed with its text in S8 **[D256 S3]**. A row is named
  1-based **[D257 B3]**, counting data rows (row 1 is the first after a
  header), not lines of text **[author]**.
- **Options MAPs** follow RFC-0016 §S1's rules: keys are STRINGs, and an unknown
  key, a key the word does not take, or a value of the wrong type or
  range is E101 naming the key. The frame words read them with a reader
  of their own, since RFC-0016's (`biomath/infer/opts.rs`, `read`) is
  private to its module and knows only `alternative`, `conf` and `mu`;
  S8 gives its texts **[author]**. The keys it knows are those of S4–S6:
  `header`, `na`, `factors` and `sep` (`frame.csv.parse`), `na`
  (`frame.aggregate`, `frame.matrix`), `how` (`frame.join`) and
  `descending` (`frame.sort`); any of them given to another of these
  words is "a key the word does not take", any other key unknown. Keys
  are checked in two passes **[author]**. First, in the MAP's sorted
  key order, each key for being known and taken, and its value for
  type, range and repeated names; the first bad key is the refusal.
  Second, after the parse, the checks against the columns: a `factors`
  name that is no column, or names a numeric or logical column. So
  `{ factors: [ "zz" ] header: 1 }` is refused for `header`. An `opts` operand is always required; an
  empty `dict` takes every default **[author]**. (A trailing optional MAP would make
  the effect depend on the top operand, which RFC-0016 rejected for the
  same reason, `docs/rfc/RFC-0016-statistical-inference.md:670-672`.) The key `na` means a LIST of strings to
  `frame.csv.parse` and `"refuse"` or `"omit"` to `frame.aggregate` and
  `frame.matrix`.
- **Forms.** As RFC-0015 §S3: each word that consumes has a plain form
  and a `.` form taking its top operand from the workbench and answering
  there (`bench!`). `na`, which consumes nothing, has the plain form only.
  `frame.filter` runs a lambda, so it is opaque and plain only, as
  `ode.rk4` (`dynamics.rs`).
- **Every frame a word answers passes the check** (S2), and the cells it
  takes from outside a frame (`frame.new`, `frame.with`,
  `frame.csv.parse`) are checked as part of it.
- **Cells keep their types and q through moves.** `frame.take`, `.head`,
  `.filter`, `.sort`, `.select`, `.complete` and `.join` copy cells; an
  INTEGER stays an INTEGER, and a cell's q is the one it had (D32). A
  factor column keeps its levels through each of them. Cells a word
  makes (a statistic, a parsed field, `na`) carry q as their constructor
  gives it, except a NONE, whose q is S2's 0.0.

### S2. The frame [P2] [P3] [P4]

A frame is a MAP holding exactly these keys **[P3]**:

| Key | Holds |
|---|---|
| `columns` | a LIST of n ≥ 0 distinct, non-empty STRINGs: the column names, in order |
| `data` | a MAP whose keys are exactly the names in `columns`, each to a LIST; every LIST of one length, the frame's row count |
| `levels` | a MAP, possibly empty **[P4]**, whose keys are a subset of `columns`: each factor column's name to its LIST of distinct STRING levels, in order |

**Column kinds [P2].** A column's kind is read from its cells that are not
NONE:

| Kind | Cells | Notes |
|---|---|---|
| numeric | INTEGER or FLOAT, each finite | mixed INTEGER and FLOAT allowed; NaN and ±∞ refused (B12) |
| logical | BOOL | |
| string | STRING, the column not in `levels` | |
| factor | STRING, each one of the column's `levels` | a level with no cell is allowed |
| (any) | all NONE, or no rows, the column not in `levels` | takes no kind until a cell says one; accepted wherever a kind is asked for **[author]** |

A column named in `levels` is a factor whatever its cells, all NONE or
no rows included; "no kind" is for a column outside `levels` only
**[author]**. NONE may stand in any column. Every NONE a frame word
makes (a CSV `na` field, a `"left"` join's unmatched cell; a NONE a
word is given is kept as it is) has q 0.0, as `na`'s and JSON's
NONE have, not `BundValue::none()`'s 100.0 **[author]**. A cell that is not a scalar (a LIST, a MAP,
any value but INTEGER, FLOAT, BOOL, STRING and NONE) is E101 in any
column, a numeric one included, so E103 is kept for a STRING or BOOL among
numbers **[author]**, narrowing D256 S3's "a numeric column's non-number
cell". Among scalars, a column holding a number and a STRING or BOOL is
E103 naming that cell; any other mixture (BOOL with STRING) is E101
naming the column, the row and both kinds; a column's kind, for that
text, is its first non-NONE cell's. These two mixture rules apply to
columns outside `levels`; in a column in `levels`, any other non-NONE
scalar that is not one of its levels, a finite number or a BOOL
included, is S8's `not one of its levels`, naming that cell
**[author]**. A non-scalar or non-finite cell in a factor column keeps
S8's rows 8 and 9, which come first. S8 orders the rules so each case has
one code, and each rule is applied over the columns in `columns` order and
each column's rows in order, so the refusal names the first offending
cell **[author]**.

**Every `frame.` word checks its frame operand on entry**, in the order
of S8's table, and the check is O(cells): `frame.nrow` pays it too. One
cost, paid once per word, is the price of every word being able to trust
its operand without a separate validated type (D252's ruling made a frame
a plain MAP) **[author]**. A program that loops `frame.column` over n
columns pays O(n × cells).

**A frame is a plain MAP** (D252): `encode.base64` and
`decode.base64` carry it with its NONEs (probed: `{ a=[ 1 ::  NONE ::  2.5 :: ] :: }` comes back as it went).
Printing it with `println` prints the MAP, its keys sorted; `frame.text`
(S6) prints the table. A frame's column is reached in plain Bund as
`"data" get "dose" get`; `"dose" get` on the frame itself is `get`'s
missing-key error (`reference/rust_dynamic/src/get.rs:10-12`, with the
prefix from `reference/rust_multistackvm/src/stdlib/values/value_dict.rs:67`).
`json.from_value` refuses a frame: its `columns` holds STRINGs, which
the reference's converter does not take
(`reference/rust_dynamic/src/cast_value_to_json.rs:84`; Note,
2026-11-19).

**Equality, for tests [D257 B2].** Neither `==` nor `BundValue`'s
equality can say a frame came back unchanged (§Current behaviour). The
criteria use a test-only helper, `frame_eq`, and no word: the same
`columns` in order; the same `levels` keys, each LIST in order; and per
cell the same type (INTEGER 1 is not FLOAT 1.0), INTEGERs equal, FLOATs
equal by `to_bits`, STRINGs and BOOLs by content, NONE equal to NONE.
It does not compare q **[author]**; criterion 3 checks `na`'s q
directly.

### S3. Missing values [P1] [P8]

| Word | Effect | Answers |
|---|---|---|
| `na` | `-- x` | NONE, with q 0.0, the value JSON's `null` converts to |
| `na.is` | `x -- bool` | whether x is NONE; any value, never refused |
| `na.count` | `list -- n` | how many cells of a LIST are NONE |
| `na.omit` | `list -- list` | the LIST without its NONE cells, in order |
| `na.fill` | `list x -- list` | each NONE replaced by x |

`na.count`, `na.omit` and `na.fill` refuse an operand that is not a LIST
with E101. These words, and every word that only moves cells, take no
`na` option: D252's "a word that accepts NONE takes the `na` option"
means a word that computes from cells (D252's third dated note): the
`"refuse"`/`"omit"` option. `frame.csv.parse`'s key `na` is another
thing, P6's list of missing strings. `na.is` exists because `==` refuses
NONE (§Current behaviour).
A LIST literal cannot hold NONE: inside `[ ]`, `na` stays a word, as
`true` and `false` do, so `[ "x" ] [ [ 1 na ] ] frame.new` is `E101
columns is outside frame.new's domain: column "x" row 2 is a CALL`. A
program puts NONE into a LIST with `+` (`[ 1 ] na +` is `[ 1 :: NONE ::
]`), or gets it from JSON's `null` or `frame.csv.parse` **[author]**
(Note, 2026-11-19; probed).
`na.omit` hands *one* column to a word of the statistics family, which
refuses NONE: `"dose" frame.column na.omit stat.mean`. Two columns used
together are made complete together, with `frame.complete` (S4), never
by two `na.omit`s: dropping each column's own gaps would misalign their
rows without a refusal.

**The `na` option [P8].** `frame.aggregate` and `frame.matrix` take an
options MAP with `na`, `"refuse"` (the default) or `"omit"`, as D252
ruled. Under `"refuse"` a NONE in a column the word reads is E101 naming
the column and row. Under `"omit"` it drops each row with a NONE in any
column it reads, and its answer says how many it dropped (S5). What a
word "reads" is stated with the word; `frame.aggregate`'s `by` columns
are outside the option **[D256 S7]**. The words that only move cells
(S4) carry NONE and take no `na` option.

### S4. Building, reading and reshaping

| Word | Effect | Does |
|---|---|---|
| `frame.new` | `names columns -- frame` | a frame from a LIST of names and a LIST of column LISTs, no factors; unequal counts are E102 |
| `frame.check` | `x -- frame` | x, if it is a frame; else the refusal |
| `frame.nrow`, `frame.ncol` | `frame -- n` | the counts; a frame with no columns has 0 rows **[author]** |
| `frame.names` | `frame -- list` | `columns` |
| `frame.column` | `frame name -- list` | one column's cells |
| `frame.with` | `frame name list -- frame` | the column replaced, or appended last when new; a factor column replaced loses its levels **[author]**; on a frame of no columns, the new column sets the row count (Note, 2026-11-18) |
| `frame.select` | `frame names -- frame` | those columns, in that order, with their levels |
| `frame.take` | `frame rows -- frame` | the rows at the given **1-based** indices **[D257 B3]**, in that order, repeats allowed **[author]** |
| `frame.head` | `frame n -- frame` | the first min(n, nrow) rows **[author]** |
| `frame.filter` | `frame lambda -- frame` | the rows for which the lambda, given the row as a MAP of name to cell, leaves `true` **[P10]** |
| `frame.complete` | `frame names -- frame` | the rows with no NONE in those columns; every column when `names` is empty **[D256 S2]** |
| `frame.sort` | `frame names opts -- frame` | rows stably sorted by those columns; opts `descending`, a LIST of the names to sort descending **[P11]** |
| `frame.factor` | `frame name -- frame` | a string column made a factor, its levels its distinct values sorted **[P7]** |
| `frame.factor.levels` | `frame name levels -- frame` | the same with the given levels, each cell required to be one |
| `frame.rows` | `frame -- list` | each row as a MAP, for a program's own loop |

- **Indices and counts.** An index is an INTEGER from 1 to nrow; 0, a
  negative, one above nrow, or a FLOAT is E101 naming it. `frame.head`'s
  n is a non-negative INTEGER, else E101.
- **Names.** A name that is not a column (`frame.column`,
  `frame.select`, `frame.complete`, `frame.sort`, `frame.factor`,
  `frame.factor.levels`, `frame.matrix`'s `names`, `frame.aggregate`'s
  `by` and the columns in its `stats`, `frame.join`'s `by`, naming the
  frame that lacks it) is E101 naming it, and so is a name given twice in one `names`, `by`,
  `factors` or `descending` LIST (`frame.select`, `frame.sort`,
  `frame.complete`, `frame.matrix`, `frame.aggregate`, `frame.join`,
  `frame.csv.parse`) **[author]**. `frame.with`'s LIST must
  have nrow cells (E102), except on a frame of no columns, whose 0 rows
  are a convention no column carries: there any length is taken and
  sets the row count (Note, 2026-11-18). The LIST passes the column
  rules.
- **`frame.filter` runs its lambda** through the bridge, as `ode.rk4`
  runs its model: one row per call, on a scratch stack; a result other
  than a BOOL is E112. It calls the bridge itself (`kb/bridge.rs`,
  `ask`) and makes its own E112 check, since `dynamics.rs`'s `call`
  reads back a LIST of finite numbers only. The row's cells may be NONE, which `==`, `<` and
  `>` refuse, so a lambda on a column with gaps guards with `na.is`
  (criterion 4 shows the form). The row MAP's keys sort, as every MAP's
  do; a lambda reads cells by name.
- **Sorting** compares numbers as numbers (an INTEGER with an INTEGER
  exactly, and with a FLOAT by exact value, as D33 compares them; a
  FLOAT with a FLOAT by value, as D108's rank 1 orders numbers and `sort`
  already compares them (`sort.rs`, `float_order`), so
  `-0.0` and `0.0` compare equal and the stable sort keeps their row
  order, not `total_cmp`'s order, which puts `-0.0` first; NaN never
  reaches it, the frame check having refused it),
  STRINGs by UTF-8 bytes, which is code-point order (D106), BOOLs false
  first **[author]** (D108 gives BOOLs no order; this is `frame.sort`'s
  own), a factor by its level order. NONE sorts last whether the column
  is ascending or descending **[P11]**. A name in `descending` that is
  not among `names` is E101.
- **`frame.factor`** takes a string column; a factor column, whose
  unused levels it drops, keeping the rest in their order, as R's
  `factor` of a factor does (`src/library/base/R/factor.R:24-28`; "The
  sort order for factors is the order of their levels",
  `src/library/base/man/sort.Rd:64-65`) **[author]**; or a
  column of no kind, which becomes a factor with no levels. A numeric or logical
  column is E101 naming it **[author]**. `frame.factor.levels` takes the
  same columns and refuses alike; its `levels` must be a non-empty LIST of
  distinct STRINGs, and a cell not among them is E101 naming the `levels`
  operand and the row **[author]**.
- **`frame.rows` and `frame.filter`** build one MAP per row; their work
  is bounded by the frame's size, which the program already holds.

### S5. Summarising [P5] [P8]

| Word | Effect | Answers |
|---|---|---|
| `frame.aggregate` | `frame by stats opts -- frame` | one row per distinct combination of the `by` columns and one column per `[ column statistic ]` pair in `stats` |
| `frame.join` | `left right by opts -- frame` | the rows of `left` with the matching rows of `right` on the `by` columns; opts `how`, `"inner"` (the default **[author]**) or `"left"` **[P5]** |
| `frame.matrix` | `frame names opts -- MATRIX` | the named numeric columns as a MATRIX, rows by rows, for `stat.lm` and `linalg.` |

**`frame.aggregate` [D256 S1, S7] [D258 B2].**

- **Statistics are names, not lambdas [P5]**, each computed by the code
  of a named word, so that a group's finite answer equals that word's on
  the group's cells bit for bit:

  | Name | Computed as | Answer |
  |---|---|---|
  | `"count"` | the group's rows the statistics used | INTEGER; any kind of column |
  | `"sum"` | `vec.sum` (`biomath/vector.rs`, `sum`) **[D258 B2]** | FLOAT |
  | `"mean"` | `stat.mean.arithmetic` (`series.rs`, `amean`) | FLOAT |
  | `"var"`, `"sd"` | `stat.var`, `stat.sd` (`describe.rs`), n − 1 | FLOAT |
  | `"median"` | `stat.median` (`describe.rs`) | FLOAT |
  | `"min"`, `"max"` | the smallest and largest cell, compared as numbers, never folding from NaN; of cells comparing equal (`0.0`, `-0.0`, INTEGER `0`), the first in row order **[author]** | FLOAT |

  Each answer is checked: a non-finite one (the `"mean"` of two cells of
  1e308) is E114 naming the group and the statistic, and refuses the word
  **[D258 B2]**. Every statistic but `"count"` needs a numeric column,
  else E101 naming the column and the statistic. An unknown statistic
  name, or a `stats` entry that is not a two-item LIST of STRINGs, is
  E101; an empty `stats` is E101; an empty `by` makes one group of every
  row **[author]**. On a frame of no rows the answer is a frame of no
  rows, an empty `by` included, and a statistic's column of no kind is
  accepted **[author]**. It is accepted with rows too: being all NONE, it
  is E101 under `"refuse"` naming its first row, and under `"omit"`
  every row is dropped, so each group has no cells for the statistic
  (below) **[author]** (Note, 2026-11-19; probed).
- **The checks come first [author].** `frame.aggregate` checks a group's
  size before computing, and computes with the very functions the named
  words use, not with their checking wrappers. `describe.rs`'s `mean` and
  `variance` are already crate-internal functions over `&[f64]`; `amean`
  (`series.rs`) is changed to call `mean`, whose arithmetic is its own
  (§Current behaviour), and `describe.rs`'s module doc, which says the
  two may differ in the last bit, is corrected in the same change, and `"sd"` is `variance`'s square root. Two are
  inline today and are factored out the same way, each word then calling
  its own: `vec.sum`'s sum (`biomath/vector.rs`, `sum`) and
  `stat.median`'s median (`biomath/describe.rs`, `median`). Each such
  function states its precondition in its doc, as `variance` states "The
  caller has checked n ≥ 2"; the median's is n ≥ 1, since at n = 0 its
  even arm would index below 0. The words' behaviour
  and tests do not change (§Preservation analysis). `frame.aggregate`
  then checks each answer itself, so S8's E101 and E114 texts are the
  only ones a program sees, never `Σ xᵢ`, `the variance` or `a series of
  1 value`, and a finite answer is the named word's bit for bit by
  construction. INTEGER cells convert to f64 as the named words convert
  them, so an INTEGER above 2⁵³ is not exact in any statistic but
  `"count"`, nor in `frame.matrix` (`[ 9007199254740993 ] vec.sum`
  prints `9007199254740992.0`, probed by the fifth review).
- **A group with no cells for a statistic** (all dropped under
  `"omit"`), or too few (`"var"` of one), is E101 naming the group, and
  refuses the word **[author]**; `amean`'s own refusal text and
  `math.min`'s NaN on an empty series are never reached.
- **The order of `frame.aggregate`'s refusals** **[author]**: the
  frame check first, then the operand and option checks (S8's E101 rows for `by`, `stats` and
  `opts`, result names included), then the frame's cells under `na`;
  then the groups in the answer's order, and within a group the `stats`
  entries in order, each checked for size before it is computed and for
  finiteness after. The first failure is the refusal.
- **`"refuse"` names the first NONE** in the order the word reads its
  columns: for `frame.aggregate`, the `stats` columns in entry order,
  each column once and `"count"`'s left out; for `frame.matrix`, the
  `names` order. Each column is read from row 1 **[author]** (Note,
  2026-11-19).
- **`"count"`'s column is not read**: a NONE in it is neither refused nor
  dropped for, since `"count"` counts rows **[author]**.
- **Result names** are `<column>.<statistic>`, after the `by` columns; a
  result name that equals a `by` column is E101 at entry, naming it
  **[D256 S7]**, as is one that equals another result name, or a `by`
  column named `.dropped` under `"omit"` **[author]**. (No result name
  can equal `.dropped` itself: a column name is never empty, and no
  statistic is called `dropped`; Note, 2026-11-19.)
- **Groups** are ordered by the `by` columns as `frame.sort` orders them
  (a factor by its levels, NONE last), not by first appearance. Two rows
  are in one group when `frame.sort` compares each of their `by` cells
  equal: numbers by value, so INTEGER `1` and FLOAT `1.0` are one group,
  and so are `0.0` and `-0.0`, as `frame.join` matches them. The
  group's cell in the answer, and in a refusal's text, is the first of
  its rows' cells in row order, keeping its type **[author]**. NONE in a
  `by` column is a group of its own; `by` columns are outside the `na`
  option. A `by` column that a statistic also reads is read under `na`
  as that statistic's column: under `"refuse"` its NONE is E101, and
  under `"omit"` its row is dropped, so its NONE group never forms
  **[author]** (Note, 2026-11-19; probed).
- **`"omit"`** drops each row with a NONE in any column a statistic
  reads, and adds a column `.dropped`, the rows dropped per group, of
  INTEGERs **[P8]**, as the last column **[author]**. `.dropped` is
  added whenever `na` is `"omit"`, even when every statistic is
  `"count"` and its cells can only be 0 **[author]** (Note, 2026-11-19;
  probed). A `by` column that is a factor stays a factor with its
  levels.

**`frame.join` [D257 S6].**

- **Columns:** `left`'s, then `right`'s other than `by`. A name other
  than `by` that both frames hold is E101, so nothing is renamed
  **[author]**.
- **Rows:** in `left`'s order; a row matching several of `right`'s
  appears once per match, in `right`'s order. Under `"left"`, a row with
  no match gets NONE in `right`'s columns, and a factor column of
  `right`'s stays a factor **[author]**.
- **Keys:** an empty `by` is E101, so no join is a cross product
  **[author]**. A `by` column must be of one kind in both frames, a
  factor and a string column counting as one kind **[author]**, else
  E101 naming it; a `by` column of no kind (all NONE, or no rows) agrees
  with any **[author]**. Numbers match
  by value (1 matches 1.0, as `frame.sort` compares), and so do BOOLs
  (`true` matches `true`; Note, 2026-11-19; probed); factor keys match
  by level text, and the answer keeps `left`'s levels; NONE matches
  nothing, as in SQL. `right`'s factor columns keep their levels.
- **The cap:** the output's row count is computed before allocating;
  above 10⁸ is E111 (D39).
- **The order of `frame.join`'s refusals [author]** (Note, 2026-11-19).
  The first failure is the refusal, in this order:
  1. the check of `left`, then of `right`;
  2. `by`'s type and its items' types;
  3. an empty `by`;
  4. each `by` name in turn: absent from `left`, given twice, absent
     from `right`;
  5. `opts`;
  6. each key's kinds;
  7. each of `right`'s non-key names held by `left` too;
  8. the cap.

**`frame.matrix` [D256 S6].** Numeric columns only: a logical, string or
factor column is E101 naming it (coding a factor is Phase C's
`model.matrix`). INTEGER cells convert to FLOAT, as `stat.lm`'s `table`
does. A column of no kind (all NONE) is accepted, and then is E101 under
`"refuse"` or leaves no rows under `"omit"` **[author]** (Note,
2026-11-19; probed). An empty `names`, and zero rows before or after
`"omit"`, are E101 naming the cause. Under `"omit"` the word reads its `names` columns and
answers `{ matrix, dropped }` in place of the bare MATRIX **[P8]**; a
program whose `y` comes from another column uses `frame.complete` first
instead (S3). Its refusals come in this order, the first failure winning
**[author]** (Note, 2026-11-19):
1. the frame check;
2. `names`, each item's type, then each name's presence and repetition;
3. an empty `names`;
4. each named column's kind;
5. `opts`;
6. a frame of no rows;
7. the cells under `na`;
8. no rows left after `"omit"`.

### S6. CSV and text [P6] [P12] [D254] [D255] [D258]

| Word | Effect | Does |
|---|---|---|
| `frame.csv.parse` | `text opts -- frame` | a frame from CSV text |
| `frame.csv.text` | `frame -- text` | CSV text from a frame |
| `frame.text` | `frame -- text` | a plain-text table for reading |

- **No new I/O.** Reading is `"path" file dict frame.csv.parse`; writing
  is `frame.csv.text "path" file.write`. `file` and `file.write` are
  already stubbed by `--noio` (§Current behaviour), so `--noio` needs no
  change and D119 no amendment. A `text` that is not a STRING is E101.
- **Options** of `frame.csv.parse`: `header` (default `true`), `na`
  (default `[ "" "NA" ]` **[P6]**), `factors` (a LIST of column names to
  make factors, as `frame.factor` does; a name that is not a column is
  E101 **[D257 S10]**, and a numeric or logical column is E101 as
  `frame.factor` refuses it **[author]**), and `sep` (default `","`
  **[D257 S10]**; narrowed to one ASCII byte other than `"`, CR and LF,
  else E101 **[author]**, since a delimiter is one byte to every CSV
  reader). The option is `header`, not `csv`'s `is_header`: this RFC's
  option names, like RFC-0016's, carry no `is_` **[D256 S5]**.
- **The reader** is the frame reader's own, not `csv-core`, because
  `csv-core` reports no quote flag and skips empty lines, and both
  matter here (§Current behaviour). Its rules, each the RFC's own
  **[author]** except where marked:
  - **Quoting.** A field is quoted when its first byte is `"`; a doubled
    `""` inside it is a quote; it may hold the separator and line breaks,
    kept as written (a CRLF inside stays CRLF);
    it ends at a `"` followed by the separator, a line terminator or the
    end of the text. A byte after the closing quote other than those
    (`"a"b`), and a `"` anywhere in an unquoted field (`a"b`, or ` "a"`
    with its leading space), are E101 naming the row; so is an
    unterminated quote, where `csv-core` would swallow the rest of the
    text (`reader.rs:86-87`).
  - **Lines [D258 B1].** CR, LF and CRLF each end a line, as in
    `csv-core`. Every line outside a quoted field ends a record; only the
    text's final line terminator ends the text. In a one-column frame an
    empty line is one empty field, read as any empty field is: NONE under
    the default `na`, an empty STRING when `na` leaves out `""`
    **[D258 B1, and its second note]**. In a wider frame an empty line is a short
    row, E102 naming the row **[D258 B1]**, and so is any row longer or
    shorter than the header **[author]**, where `csv`'s reader is
    flexible (`data.rs`, `read_frame`, `.flexible(true)`) and skips
    empty lines. So `"\n"` is one empty record: with `header` true an
    empty header name (E101), with `header` false `column_1` holding one
    empty field.
  - **The byte-order mark.** A leading UTF-8 byte-order mark is removed,
    to match `csv-core`, which removes it (Q65), before any other rule,
    so text that is only a mark is empty text.
  - **Size.** Text above 10⁸ bytes is E111. Empty text is a frame of no
    columns and no rows.
  - **The header row.** A quoting fault in the header row names `the
    header` where a data row names `row <n>` **[author]**; S8 gives the
    three texts. The header sets the width, so it cannot have the wrong
    one. With `header` false the first row sets it, and a later row of
    another width names `row 1's` in place of `the header's`.
  - **Sharing.** `infer`, its `Kind`, `is_int` and `column_kind` are
    shared from `data.rs` by a visibility change only, and `data.rs`'s
    tests stay as they are.
- **Headers [D256 S5].** With `header` false the columns are `column_1`,
  `column_2`, … from the first row's width, as `read_frame` names them.
  A header field is a name: it is never matched against `na` and never
  inferred **[author]**. A duplicate or empty header name is E101 naming
  it; nothing is renamed.
- **Fields.**
  - **Quoted fields [D254].** A quoted field is a STRING whatever it
    holds: it takes no part in inference and is never an `na` string, so
    a quoted `""` is an empty STRING and a quoted `"NA"` the STRING `NA`.
    A quoted number in a column whose other fields infer as numbers is
    therefore a STRING in a numeric column, which the frame check refuses
    with E103; a file written with every field quoted reads as string
    columns.
  - **`na` strings [P6].** An unquoted field equal to an `na` string is
    NONE, in its place, before inference.
  - **An empty unquoted field** that the `na` strings do not name is an
    empty STRING and takes part in inference, which makes its column a
    string column **[D258 B1, and its second note]**. Under the default
    `na` it is NONE.
  - **Every other unquoted field** is inferred as `csv` infers it
    (`data.rs`, `infer`, and `column_kind`'s merge), with four
    differences from `csv`, each the frame reader's own rule:
    - a column's kind is judged over **every** row, not the first 100,
      so no late cell disagrees with it and no leading run of NONE hides
      it **[D256 S4]**;
    - in a column inferred FLOAT from INTEGER and FLOAT evidence, a field
      that matches the integer pattern (`data.rs`, `is_int`) is an
      INTEGER, any other a FLOAT **[D254 note]**;
    - in a column inferred FLOAT, a field that parses to NaN or ±∞
      (`NaN`, `inf`, `-inf`, `+inf`, or `1e400`, which overflows to +∞;
      probed: `csv` reads it as `inf`) is E101 naming the field, its row
      and its column, and pointing at the `na` option, which reads it as
      NONE when listed there **[D255, and its note]**. In a string column
      such a field is an ordinary STRING. Polars' pattern is
      case-sensitive (`data.rs`, `is_float`), so `nan` or `Inf` infers as
      a string, and its column becomes a string column without a
      refusal, as in `csv` **[D255]**;
    - in a column inferred INTEGER or FLOAT, a field that matches the
      integer pattern but does not fit an i64 (`99999999999999999999`)
      is E101 naming the field, its row and its column **[D258 S2, and
      its note]**. In a string column it is an ordinary STRING.
- **Writing.** `frame.csv.text` always writes a header and takes no
  options.
  - **Quoting.** It quotes every STRING cell, a factor's included
    **[D254]**, and every header name **[author]**: a quoted header reads
    back as the same name. That includes one beginning with U+FEFF, which
    unquoted would be taken for a byte-order mark.
  - **Cells.** NONE is an empty field **[P6]**; an INTEGER is written in
    decimal; a BOOL as `true` or `false`.
  - **FLOATs.** A FLOAT is written as Bund prints it, the shortest form
    that parses back to the same f64 (`crates/bund2-value/src/lib.rs`,
    `float_text`, by way of `dtoa`). So an integral FLOAT keeps its `.0`
    (`3.0 println` prints `3.0`), and an exponent form (`1.0e-7 println`
    prints `1e-7`) matches polars' float pattern (`data.rs`, `is_float`),
    both probed.
  - **Lines.** Each line ends with `\n`. A frame of no columns is the
    empty STRING **[author]**.
- **The round trip.** `frame.csv.parse` of `frame.csv.text` of a frame,
  with an empty options MAP, is `frame_eq` to it, factors aside, for a
  frame whose text is within 10⁸ bytes. A factor comes back a string
  column unless `factors` names it, and then with its levels sorted
  (criterion 6). A one-column frame's NONE comes back in place
  **[D258 B1]**.
- **`frame.text` [D257 S9]** prints a header row, then the rows, with one
  space between columns. Numbers are right-aligned and the rest
  left-aligned, as ruled. These details are the RFC's own **[author]**:
  - **Alignment and width.**
    - Alignment is per column: every cell of a numeric column is
      right-aligned, its header and its NONEs included; a column of no
      kind is left-aligned.
    - A column is as wide as its widest printed cell or its header,
      counted in characters (Unicode scalar values), not in `len`'s bytes.
    - No line ends in whitespace that is not a STRING's own: padding and
      separators after the last non-empty cell are dropped, however many
      empty cells end the row, but a STRING's own trailing spaces are
      kept.
    - A STRING holding a line break prints it, breaking the table's row;
      the width counts each character of the STRING, the break included.
      Accepted, as "neither truncated nor escaped" asks.
  - **Cells.**
    - A FLOAT prints as Bund prints it.
    - NONE prints as `NA` **[P12]**, so a STRING cell `NA` prints alike.
    - A STRING prints as it is, neither truncated nor escaped.
  - **Length.** At most 20 rows print, then a line `… <n> more rows`
    (`… 1 more row` for one). A
    frame of no columns prints the empty STRING.

  For the frame `columns [ "x" "group" ]`, `x` the cells INTEGER `1`,
  FLOAT `2.5` and NONE, and `group` a factor `[ "a" "bb" ]` with cells
  `a`, `bb` and NONE, it prints this (no line has a trailing space):

  ```
    x group
    1 a
  2.5 bb
   NA NA
  ```

### S7. Tiers, audits and generated files

As RFC-0015 §S17 and RFC-0016 §S8: effects as declared above;
`PROMOTABLE.txt` regenerated. `UNVARIED` gains exactly the natives the
audit's own rule selects (`crates/bund2-stdlib/src/lib.rs`, the
unvaried-operand audit: a plain word consuming three or more, a `.` form
whose stack operands are two or more, a `.` form declaring one fewer than
its word, `wb.rs`), each with every operand the palette leaves at 7:

| Names | Operands left at 7 |
|---|---|
| `frame.with`, `frame.with.` | `frame` |
| `frame.sort`, `frame.sort.` | `frame` |
| `frame.factor.levels`, `frame.factor.levels.` | `frame` |
| `frame.matrix`, `frame.matrix.` | `frame` |
| `frame.aggregate`, `frame.aggregate.` | `frame` and `by` |
| `frame.join`, `frame.join.` | `left` and `right` |

`na.fill` and every other two-operand word do not join. A counting test
per module, as RFC-0016's modules have.

### S8. Refusals

Each row is RFC-0015 §S11's code with its template filled, `<w>` the
word and the operand named. In the frame check's rows, `frame` stands
for the operand the faulty part came from **[author]**: `left` or
`right` for `frame.join`; for `frame.new`, `names` for a name fault and
`columns` for a column or cell fault; for `frame.with`, `list` for its cells; for `frame.csv.parse`, `text`, since
its answer is checked like any frame (a quoted number in a numeric
column is E103, S6).

**Bounded texts [author].** Every value a text interpolates from the
program (a cell, a CSV field, a column, header, level or result name,
a group's key, the first character after a closing quote) is rendered
by the frame words' own renderer, not by `summary`, which quotes and
truncates a STRING but does not escape it (`crates/bund2-value/src/lib.rs`,
`summarise`, the `Str` arm):

1. **Escape** the text: `\` as `\\`, `"` as `\"`, a line feed as
   `\n`, a carriage return as `\r`, a tab as `\t`, and any other
   control character (`char::is_control`, Unicode's Cc), and U+2028 and
   U+2029, which some terminals break lines on, as `\u{` and its code
   point in lower-case hex and `}` (ESC as `\u{1b}`).
2. **Truncate** the escaped text to at most `WIDTH` − 2 characters
   (`biomath/operand.rs`, `WIDTH`, 40), the last of them `…` when cut;
   the cut falls between escapes, never inside one, so a shorter text is
   kept rather than a split `\`; escaping comes first so the bound holds
   on what prints.
3. **Quote** it in double quotes.

A number, a BOOL and NONE print as `summary` prints them: NONE as
`none`, as RFC-0015's texts show it. So no interpolated value is
unbounded or breaks a line (`CLAUDE.md`'s rule on reports, D36). A
group text is bounded per key, so it grows with the number of `by`
columns, which the program chooses; every key is still shown.

**`<summary>`** in a row is `summary(WIDTH)` of the whole operand, as in
RFC-0015's texts: bounded, but not escaped, so a STRING operand holding
a line break prints it (probed by the ninth review through `vec.scale`).
Escaping inside `summary` would change every word that uses it and is
left to a change of its own; `summary` itself is unchanged here, so no
existing word's text moves.

The frame check runs in the order of its first eleven rows and stops at
the first that fails.

| Rule | Code | Text |
|---|---|---|
| not a MAP | E101 | `frame is outside <w>'s domain: <summary>` |
| a key missing, or another key | E101 | `frame is outside <w>'s domain: a missing key "levels"` / `an extra key "x"` |
| `columns` not a LIST, or a name not a STRING, empty or repeated | E101 | `frame is outside <w>'s domain: column name 3 is <summary>` / `a repeated name "x"` |
| `data` not a MAP, or its keys not `columns` | E101 | `frame is outside <w>'s domain: data has no column "x"` / `data has a key "y" not in columns` |
| a column not a LIST | E101 | `frame is outside <w>'s domain: column "x" is <summary>` |
| unequal lengths | E102 | `the lengths of "a" and "b" do not agree: 5 and 4` |
| `levels` not a MAP, a key not a column, levels not distinct STRINGs | E101 | `frame is outside <w>'s domain: levels is <summary>` / `levels has a key "g" not in columns` / `levels of "g" repeat "a"` (or `hold <summary>`, not a STRING) |
| a cell that is not a scalar | E101 | `frame is outside <w>'s domain: column "x" row 3 is a LIST` |
| a NaN or ±∞ cell (B12) | E101 | `frame is outside <w>'s domain: column "x" row 3 is NaN` |
| a number with a STRING or BOOL, outside `levels` | E103 | `column "x" is not a LIST of numbers: "abc" at row 3` |
| any other mixture outside `levels`; in a column in `levels`, any cell not among its levels, a number or BOOL included | E101 | `frame is outside <w>'s domain: column "ok" row 3 is a STRING; the column is logical` / `column "g" row 3 is "c", not one of its levels` |
| `frame.new`'s counts | E102 | `names and columns do not agree: 3 and 2` |
| `frame.with`'s length, on a frame with a column | E102 | `the list and the frame's rows do not agree: 4 and 5` |
| an operand of the wrong type (`names`, `by`, `rows`, `stats`, `levels`, `columns`, `frame.with`'s `list` not a LIST; `name` not a STRING; `frame.filter`'s `lambda` not a lambda); an item of a `names` or `by` LIST not a STRING; `frame.with`'s new name empty **[author]** | E101 | `<operand> is outside <w>'s domain: <summary>` / `<operand> is outside <w>'s domain: item 2 is <value>`, the value rendered as the preamble says / `name is outside <w>'s domain: an empty name` |
| an index or count | E101 | `rows is outside <w>'s domain: row 7 of a frame of 5 rows` / `n is outside <w>'s domain: -1` |
| a name absent or repeated | E101 | `<operand> is outside <w>'s domain: no column "dose"` (`no column "id" in right` for `frame.join`) / `"x" given twice`, `<operand>` being `name`, `names`, `by` or `opts` as the word takes it |
| `descending` naming a column not sorted by | E101 | `opts is outside <w>'s domain: descending names "z", not among names` |
| `frame.factor`, `frame.factor.levels` or `factors` on a numeric or logical column | E101 | `<operand> is outside <w>'s domain: column "x" is numeric`, `<operand>` being `name` or `opts` |
| `frame.factor.levels`'s `levels` not a non-empty LIST of distinct STRINGs, or a cell not among them | E101 | `levels is outside <w>'s domain: <summary>` / `"a" given twice` / `"c" at row 3 is not one of the levels` |
| `na.count`, `na.omit`, `na.fill` on a non-LIST | E101 | `list is outside <w>'s domain: <summary>` |
| a NONE under `"refuse"` | E101 | `frame is outside <w>'s domain: column "x" row 3 is none (na "refuse")` |
| a statistic unknown, malformed, or on a non-numeric column; `stats` empty | E101 | `stats is outside <w>'s domain: "mode" is not a statistic` / `"mean" of column "g", a factor` (`, a string column`, `, a logical column`) / `no statistics` / `item 2 is <value>` for an entry that is not a two-item LIST of STRINGs / `no column "zz"` (Note, 2026-11-19) |
| a group with too few cells | E101 | `stats is outside <w>'s domain: group [ "a" ] has 1 value for "var"` (`0 values`, the noun agreeing with the count) |
| a non-finite statistic | E114 | `the answer overflows a FLOAT: "x.mean" of group [ "a" ]` |
| how a group is named, in the two rows above | — | `group [ <k1> <k2> … ]`, each `by` value in `by` order, separated by one space, each rendered as the preamble says (a STRING or factor level escaped, bounded and quoted; NONE as `none`), every key shown, each bounded; an empty `by`'s one group is `group (all rows)` **[author]** |
| a result name colliding | E101 | `stats is outside <w>'s domain: result name "x.mean" repeats a by column` / `result name "x.mean" repeats another` (Note, 2026-11-19) |
| a join key of two kinds, a shared non-key name, an empty `by` | E101 | `by is outside <w>'s domain: key "id" is numeric in left and string in right` / `"dose" is in both frames` / `no keys` |
| `frame.matrix` on another kind, no names, no rows | E101 | `names is outside <w>'s domain: column "g" is a factor` (`is string`, `is logical`; Note, 2026-11-19) / `no columns` / `frame is outside <w>'s domain: no rows` |
| `frame.filter`'s lambda leaving a non-BOOL | E112 | `the model lambda left an INTEGER, where a BOOL was required` |
| CSV text not a STRING | E101 | `text is outside <w>'s domain: <summary>` |
| a CSV row of the wrong width, an empty line in a wider frame | E102 | `row 7's fields and the header's do not agree: 3 and 4` (`row 7's fields and row 1's` with `header` false) |
| an unterminated quote, a byte after a closing quote, a quote in an unquoted field | E101 | `text is outside <w>'s domain: a quote opened at row 7 is not closed` / `row 7 has "b" after a closing quote`, `"b"` the first character after it / `row 7 has a quote inside an unquoted field`; in the header row, `a quote opened in the header is not closed` / `the header has "b" after a closing quote` / `the header has a quote inside an unquoted field` |
| a non-finite CSV field | E101 | `text is outside <w>'s domain: field "NaN" at row 3, column "x" is not finite; list it in the na option to read it as missing` |
| an integer field beyond i64 | E101 | `text is outside <w>'s domain: field "99999999999999999999" at row 3, column "x" does not fit an INTEGER` |
| a CSV header name empty or repeated | E101 | `text is outside <w>'s domain: header name 2 is empty` / `a repeated header name "x"` |
| `sep` not one ASCII byte, or `"`, CR, LF | E101 | `opts is outside <w>'s domain: sep is <value>` |
| text above 10⁸ bytes, a join above 10⁸ rows | E111 | `<what> exceeds its cap of 100000000`, `<what>` being `text` or `the join's row count` (Note, 2026-11-19) |
| an options key unknown, or its value wrong | E101 | `opts is outside <w>'s domain: <summary>` (not a MAP) / `an unknown key "k"` / `the key "k", which this word does not take`, as RFC-0016's reader words it (`biomath/infer/opts.rs`, `read`) / `<key> is <value>`, a scalar value rendered as the preamble says; for a LIST value, the first bad item, `<key> item 2 is <value>`, rendered alike, or `<key> is a LIST of <n>` when the LIST itself is wrong (`sep`'s one byte aside, which names the value); the key leads the text, where RFC-0016's reader names it after the value **[author]** |

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| `csv` and its conditional, run with or without `column` (`conditional_csv.rs:9-173`; `data.rs`): its 100-row window, its quote removal, its merging of integers with floats, its NaN and `inf` cells, its dropped nulls, its skipped empty lines | **Preserved exactly** (D51, D252). `frame.csv.parse` shares `infer` and `is_int` and departs only by its own rules (S6) |
| `data.rs`'s `infer`, `Kind`, `is_int` and `column_kind` made visible to the frame reader | **Changed by design**, visibility only; `data.rs`'s tests unchanged |
| `vec.sum`'s sum (`vector.rs`) and `stat.median`'s median (`describe.rs`) factored into crate-internal functions over `&[f64]`, and `amean` (`series.rs`) calling `describe.rs`'s `mean` (S5) | **Changed by design**, a move only: each word calls the function its arithmetic now sits in, the same operations in the same order; its answers, refusals and tests are unchanged |
| `csv`'s `is_header` beside `frame.csv.parse`'s `header` | **Changed by design**: two spellings in one language, deliberate (S6) |
| `json.to_value`'s `null` → NONE, `json.from_value`'s NONE → `null` (`cast_json_to_value.rs:38-39`; `json.rs`, `to_value`; `to_json`) | **Preserved exactly**; this RFC gives that NONE its meaning |
| NONE's printing (`NONE` in a LIST, `None` as a type) and the wire format | **Preserved exactly** |
| `==` refusing NONE, LISTs and MAPs | **Preserved exactly**; `na.is` is the test, `frame_eq` a test helper only |
| RFC-0015's and RFC-0016's refusal of NONE and of non-finite cells (B12), and their texts: `stat.var`'s `Can not convert NONE`, `stat.cov`'s `E103 … none` | **Preserved exactly** |
| `stat.mean.arithmetic`'s `inf` on an overflowing sum | **Preserved exactly**; only `frame.aggregate` checks its answer |
| NODATA as the drain's sentinel (`series.rs`) | **Preserved exactly**: no frame word reads or makes NODATA |
| MAP key order (sorted in Bund2, F76 in the reference) | **Preserved exactly**: a frame's order lives in `columns` |
| `at`'s 1-based index | **Preserved**, and followed: `frame.take` counts from 1 |
| `--noio` (D119) | **Preserved exactly**: no new word touches the file system |
| A program that defines a `frame.` or `na` name itself | **Preserved**: a program's word is found before a native of the same name (`crates/bund2-api/src/lib.rs`, `Registry::resolve_uncached`, as RFC-0016 §S1 cites and probes) |
| A program that calls a new name without defining it | **Changed by design**: today it fails as an unregistered word (`na` refused as not registered, and `bund2 check` stops at `frame.nrow`, probed by the eighth review); it now runs the native |
| A program asking `?stdlib`/`?word` of one of the new names | **Changed by design**: the answer becomes true |
| `bund2 words`' and `bund2 effects`' listings | **Changed by design**: the new names, forms and effects appear |
| `PROMOTABLE.txt` | **Regenerated**; no existing name line moves; the count line at its head changes with every name added (Note, 2026-11-19) |
| `UNVARIED: [&str; 63]` in `lib.rs`'s tests | **Changed by design**: S7's twelve names join, 63 to 75 |
| conform 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602 | **Preserved exactly** (D72) |

## Alternatives considered

- **D252's other options** — a MAP of equal-length LISTs by convention, a
  CLASS instance, a new value type; NaN, NODATA or a mask for missing —
  were weighed and ruled there; this RFC does not reopen them.
- **A `frame.read.csv` that opens a file.** It would add an I/O word,
  which `--noio` would have to stub (D119) and a sandboxed host to audit.
  Parsing a STRING that `file` reads keeps the I/O surface where it is.
- **Changing `csv` to keep a null as NONE.** Rejected by D252 and D51: a
  program that runs `csv` today would see a new value in its rows.
- **Quoting only what must be quoted, and narrowing the round trip.**
  Rejected by D254: a STRING column of `"007"` would come back numeric.
- **Reading a non-finite field as NONE, or a `nonfinite` option.**
  Rejected by D255: an undefined result is not a value never recorded,
  and the `na` option already reads any listed string as NONE.
- **Writing NONE as `NA`, or skipping empty lines.** Rejected by D258 B1:
  the first reverses P6, the second loses a one-column frame's NONE.
- **Building the reader on `csv-core`.** It reports no quote flag and
  skips empty lines (§Current behaviour), the two things D254 and D258
  need.
- **`na.omit` on a frame.** Rejected by D256 S2 for `frame.complete`:
  one meaning per word.
- **Lambdas for `frame.aggregate`'s statistics** (research 07's C7). A
  named statistic runs natively; a lambda would run once per group at
  Tier 0. The names cover the common cases; a program wanting another
  statistic runs `frame.rows` or splits by `frame.filter` itself **[P5]**.
- **R's `NA` per type** (logical, integer, double, character). One NONE
  is enough when a column's kind is read from its other cells (S2).
- **Row-oriented frames** (a LIST of row MAPs). Column words, the CSV
  reader's columns, `stat.` words on a column and `frame.matrix` all want
  columns; `frame.rows` gives rows when a program wants them.
- **A `frame.equal` word.** Not needed by any criterion, and `==`'s
  refusal of containers is the reference's (D257 B2).

## Acceptance criteria

Each is a named test in `crates/bund2-stdlib` unless placed otherwise.
No golden is added (D21). Expected values are stated here or computed by
hand in the test; no fixture of R output is needed, since no answer here
is a statistic beyond existing words' own. "Equal" is `frame_eq` (S2) for
frames and `to_bits` for FLOATs, cell by cell in a LIST.

1. **Registered.** Every name of S3–S6 and its forms is in the registry
   with the effect stated, and a counting test per module counts its
   prefix. For one word of each effect shape, the `.` form takes its top
   operand from the workbench and answers there.
2. **The frame check.** For each of S8's first eleven rows, a frame
   breaking only that rule is refused with that code and text. A frame
   with zero columns, and one with zero rows, is accepted. A column in
   `levels` holding `1` beside a valid level is refused with row 11's
   text naming the `1`, not with E103.
3. **Missing values.** `na na.is` is true and `0 na.is`, `nodata na.is`,
   `"" na.is` are false; `na.count`, `na.omit` and `na.fill` on a LIST
   with NONE at its ends and middle, and on a non-LIST (E101);
   `'[1, null]' json json.to_value na.count` is 1, and `na` and JSON's
   NONE have the same type and q.
4. **Reading and reshaping.**
   - On a fixed 5-row frame with an INTEGER column, a FLOAT column, a
     factor and a NONE: `frame.new`, `.nrow`, `.ncol`, `.names`,
     `.column` and `.rows` (each row's MAP) answer what the test writes
     out; `frame.nrow` of a zero-column frame is 0.
   - `frame.select` (a repeated name E101), `.take` (1-based, with a
     repeat; index 0 refused), `.head`, `.with` (replace and append;
     a wrong length E102; on a frame of no columns, a LIST of 3 cells
     answering a frame of 3 rows), `.filter` (a lambda on two columns guarding a
     NONE with `na.is`), `.complete` (with named columns, and with an
     empty `names`), `.sort` (two keys, one descending, NONE last in both,
     stable; a factor key by level order; a BOOL key false first; a
     FLOAT key holding `-0.0` after `0.0` keeping that row order),
     `.factor` (a numeric column E101; an all-NONE column a factor with
     no levels; a factor with a custom level order and an unused level
     keeping that order, the unused level dropped) and `.factor.levels` (a cell not among the levels, and a
     repeated level, E101) answer frames equal to ones written out in the
     test, cells keeping their types and q; each answer passes
     `frame.check`. `frame.filter` with a lambda leaving `1` is E112. `frame.select` with
     `names` a STRING, `names` holding `1`, `frame.with` with `list` a
     STRING or an empty name, and `frame.filter` with a STRING for its
     lambda, refuse with S8's wrong-type texts.
5. **Summaries.**
   - `frame.aggregate` by a factor and a string column, with every
     statistic: each group's answer equal bit for bit to the named
     word's on its cells (S5's table; for `"min"` and `"max"`, the
     smallest and largest cell as a FLOAT, the first of a tie), groups in `frame.sort`'s order,
     a NONE group last; under `"omit"`, `.dropped` counts; under
     `"refuse"`, a NONE is E101 naming column and row; a group of two
     1e308 cells is E114 for `"mean"`, `"sum"` and `"var"`, with S8's
     text, not the shared code's; a group of one is E101 for `"var"` with
     S8's text; an emptied group, a `"mean"` of a string column, an
     unknown statistic, a repeated `by` name and a colliding result name
     are E101; an empty `by` answers one group; a NONE in a `"count"`
     column is neither refused nor dropped for; a frame of no rows
     answers a frame of no rows, with an empty `by` too; a tie of `0.0`
     and `-0.0` in `"min"` answers the first; a frame of no rows grouped
     by a factor column, under `"mean"` of a numeric column, answers no
     rows (Note, 2026-11-19); a `by` column holding
     INTEGER `1` then FLOAT `1.0` answers one group whose cell is INTEGER
     `1`, and one holding `0.0` then `-0.0` one group whose cell is
     `0.0`; a group refusal on a STRING key of 200 characters holding a
     `"` and a line break shows it escaped, then bounded, on one line,
     exactly as S8's renderer gives it; a NONE key shows as `none`; a
     group failing E101 for `"var"` before another failing E114 for
     `"mean"` refuses with the E101, and a group failing E114 before
     another failing E101 refuses with the E114, by S5's order.
   - `frame.join` inner and left on one and on two keys, with a left row
     matching two right rows, a factor key, an INTEGER key matching a
     FLOAT, a NONE key matching nothing, and a left row with no match
     given NONE with q 0.0; a shared non-key name, an empty `by` and a repeated `by`
     name E101; the E111 cap.
   - The model path: a frame with a NONE in `x` at one row and in `y` at
     another, through `[ "x" "y" ] frame.complete`, then `frame.matrix`
     and `"y" frame.column`, then `stat.lm`, equals `stat.lm` on the
     complete rows written by hand: every FLOAT, in every LIST of its
     answer, by `to_bits`.
   - `frame.matrix` under `"omit"` answers `{ matrix, dropped }` with the
     right count; it refuses a factor column, an empty `names` and zero
     rows with E101.
6. **CSV.**
   - `frame.csv.parse` on a text with a header, an empty field, an `NA`,
     a quoted `"NA"`, a quoted `"007"`, a quoted comma, a quoted line
     break, a BOOL column and a column of `1` and `2.5`: each column's
     kind and each cell's type as S6 gives, NONE in place; `"NA"` kept as
     a STRING when `na` is `[ "" ]`; a FLOAT first at row 150 of an
     integer column; a numeric column whose first 120 cells are empty; a
     ragged row and an empty line in a two-column text E102; a `NaN`
     field in a numeric column E101, and NONE when `na` lists it; a
     `1e400` field in a FLOAT column E101; a
     column of `inf` and `ctl` read as STRINGs; `nan` making a string
     column; `99999999999999999999` E101 in an integer column and a
     STRING in a column with `A1`; an empty field under `na` `[ "NA" ]`
     an empty STRING, and an empty line in a one-column text an empty
     STRING under it; an unterminated quote, `"a"b` and `a"b` E101, in a
     data row and in the header row (`the header`); a quoted CRLF kept as
     written; CR
     and CRLF terminators; a leading byte-order mark removed; a header
     named `NA` kept as a name; text above 10⁸ bytes E111; empty text,
     and text that is only a byte-order mark, a frame of no columns and
     no rows; `"\n"` an empty header name E101 with `header` true and
     `column_1` holding one NONE with `header` false; `sep` `";"`, and
     `sep` `"\""` and `sep` `";;"` E101; a text of its own whose numeric
     column holds a quoted `"007"` E103, naming `text`'s column and row;
     each `na` field's NONE with q 0.0; options: an unknown key, `how`
     given to `frame.csv.parse`, and `header` `1`, each E101 with S8's
     text, and two bad keys refused for the first in sorted order
     (`{ header: 1, sep: ";;" }` for `header`), a column check after
     every key check (`{ factors: [ "zz" ] header: 1 }` for `header`);
     `na` `[ "" 1 ]` refused naming item 2; `header` false naming `column_1`, …; a repeated and an empty
     header name E101; `factors` making a factor with sorted levels, and
     a name that is not a column E101.
   - `frame.csv.parse` of `frame.csv.text` of a frame of every kind, with
     NONE, STRING cells `""`, `"NA"`, `"007"` and `"true"`, a header
     name holding a comma, a first header name beginning with U+FEFF, and
     a column mixing INTEGER and FLOAT, is
     equal to it, factors aside as S6 states; so is a one-column frame
     with a NONE at its start, middle and end.
   - The same file through `csv` with `column` set answers that column
     with the empty cell dropped, unchanged.
7. **Round trips.** A frame with NONE and a factor through
   `encode.base64 decode.base64` is equal to itself and passes
   `frame.check`; `json.from_value` of it is refused with a text ending
   `This Dynamic type is not supported for JSON: 4` (Notes, 2026-11-19).
8. **Text.** `frame.text` of S6's example frame prints exactly S6's text;
   25 rows print 20 and `… 5 more rows`; a zero-column frame prints the
   empty STRING; a column holding `µg` is padded by characters, a
   last-column STRING ending in a space keeps it, an empty last cell, and two empty last cells,
   leave no separator at the line's end, and 21 rows end `… 1 more
   row`.
9. **No abort.** Every name and form, on a palette of NaN, ±∞, wrong
   types, empty, ragged, NONE-filled and malformed frames, answers or
   refuses with an E1nn, a shallow-stack error or a bridge error.
10. **Health.** `cargo xtask conform` and `coverage` print §Preservation
    analysis's numbers; clippy and `cargo deny` are clean;
    `PROMOTABLE.txt` is regenerated with no existing name line moved; the
    `UNVARIED` audit passes with S7's twelve names; the `csv` tests in
    `data.rs`, and the tests of `vector.rs`, `describe.rs` and
    `series.rs`, pass unchanged, which is the check that S5's move moved
    nothing.

## Implementation plan

| M | Delivers |
|---|---|
| M1 | S2's frame, its check and S8's frame rules, `frame_eq`; S3's `na` words; `frame.new`, `.check`, `.nrow`, `.ncol`, `.names`, `.column`, `.rows`; criteria 1–3, and criterion 4's first part |
| M2 | the rest of S4, `frame.complete` included; the rest of criterion 4 |
| M3 | S6, its reader included; criteria 6–8 |
| M4 | S5; criterion 5 |

Each ends with criteria 9 and 10.

## Open questions

Every owner question is ruled:

- **D253, P1–P12**, all A (2026-11-17): the names `frame.` and `na`;
  typed columns; three keys and no others; `levels` always present; S5 in
  scope; `[ "" "NA" ]` and NONE written empty; sorted levels; `.dropped`
  and `{ matrix, dropped }`; no new code; the row as a MAP; NONE last;
  `NA` in text.
- **D254**: a quoted field is text and every STRING is written quoted;
  its note: a mixed numeric column keeps each cell's type.
- **D255**: a non-finite CSV field is E101; the `na` option reads it as
  NONE; its note: in a column inferred FLOAT only.
- **D256, S1–S8**: the statistics' words, `frame.complete`, the refusal
  texts, inference over every row, headers, `frame.matrix`'s kinds,
  `frame.aggregate`'s cases, the Preservation rows; its note: `vec.sum`
  exists.
- **D257**: 1-based indices; `frame_eq`; `frame.join`'s rules;
  `frame.text`'s layout; `sep` and unknown `factors` names.
- **D258**: every line a record; a non-finite statistic E114 and
  `"sum"` as `vec.sum`; an integer beyond i64 E101; its notes: in a
  numeric column only, and B1's NONE under the default `na` only.
- **D252's third note**: the `na` option belongs to words that compute
  from cells; words that move, test, count or make NONE take none.

Choices this revision makes that no ruling names, each marked
**[author]** where it stands, for the owner to see:

- rows named as data rows, not lines of text (S1);
- an `opts` operand always required, an empty `dict` taking the
  defaults (S1);
- `frame_eq` does not compare q (S2);
- the O(cells) check on every word (S2), each rule applied in column then
  row order, a column's kind for a refusal's text its first non-NONE
  cell's (S2);
- a LIST or MAP cell is E101 in a numeric column too, narrowing D256
  S3's E103 (S2);
- `frame.with` drops a replaced factor's levels and appends a new column
  last (S4);
- a name given twice in one `names`, `by`, `factors` or `descending`
  LIST is E101, in each word S4 lists (S4);
- BOOLs sort false first (S4);
- `frame.factor` refuses a numeric or logical column, and `factors`
  refuses one alike (S4, S6);
- `frame.aggregate`: an empty `by` is one group; a group with too few
  cells refuses the whole word; `"count"`'s column is not read; a
  result name may not repeat another, nor a `by` column `.dropped`
  under `"omit"` (S5);
- `frame.aggregate` checks first and computes with the named words' own
  functions, three of them factored out of their wrappers, so S8's texts
  are the only ones (S5);
- `frame.aggregate` on a frame of no rows answers no rows; `"min"` and
  `"max"` take the first of a tie (S5);
- a column of no kind is accepted wherever a kind is asked for, and
  `frame.factor` makes it a factor with no levels; `frame.factor.levels`
  refuses as `frame.factor` does, with its own `levels` refusals (S2,
  S4);
- `frame.join`: `"inner"` by default; an empty `by` is E101; a factor and
  a string key are one kind, and a key of no kind agrees with any; a
  shared non-key name is E101; a `right` factor column stays a factor
  under `"left"` (S5);
- the CSV reader is the RFC's own: its quoting rules, with E101 for an
  unterminated quote, a byte after a closing quote or a quote in an
  unquoted field; CR, LF and CRLF; the byte-order mark removed to match
  `csv-core`, before any other rule; a ragged row E102 (the empty line
  in a wider frame is D258's); text above 10⁸ bytes E111; empty text a
  frame of nothing (S6);
- `sep` narrowed from "one character" (D257 S10) to one ASCII byte other
  than `"`, CR and LF (S6);
- a quoting fault in the header row names `the header`; with `header`
  false a wrong width names `row 1's` (S6);
- a group in a refusal's text named `group [ … ]`, every key shown, and
  `group (all rows)` for an empty `by` (S8);
- a column in `levels` is a factor whatever its cells; "no kind" is for
  columns outside `levels` (S2);
- `frame.factor` of a factor keeps its levels' order, dropping unused
  ones, as R does (S4);
- `.dropped` is the last column (S5);
- `by` values are one group when `frame.sort` compares them equal, `1`
  and `1.0` included, the group's cell its first in row order (S5);
- every interpolated value in a refusal escaped, then truncated to
  `WIDTH` − 2 between escapes, then quoted, by the frame words' own
  renderer; NONE shown as `none`; `<summary>` bounded but not escaped,
  as RFC-0015's; a group text bounded per key (S8);
- the frame words' own options reader, with S8's texts, its known keys,
  checked in two passes (keys in sorted order, then the column checks
  after the parse), RFC-0016's "does not take" wording, a LIST value's
  first bad item named, and the key leading the wrong-value text (S1,
  S8);
- the wrong-type row's operands, an item of `names` or `by` named by
  its position, and `frame.with`'s empty name (S8);
- a control character is `char::is_control`'s, U+2028 and U+2029
  escaped too (S8);
- the frame check names the operand the fault came from, not always
  `frame` (S8);
- every NONE a frame word makes has q 0.0 (S2);
- `frame.aggregate`'s refusals in a stated order, the first failure
  winning (S5);
- a non-scalar or non-finite cell in a factor column keeps rows 8 and 9
  (S2);
- in a column in `levels`, any cell not among the levels is the
  `not one of its levels` refusal, whatever its type (S2, S8);
- `frame.take` allows repeats, `frame.head` takes min(n, nrow), and a
  frame of no columns has 0 rows (S4);
- a quoted line break kept as written (S6);
- a header field never matched against `na` nor inferred; every header
  name written
  quoted; a zero-column frame written and printed as the empty STRING
  (S6);
- `frame.text`'s alignment per column, a column of no kind left-aligned,
  width in characters over the printed rows, no padding or separator at
  a line's end but a STRING's own trailing spaces kept, a STRING's line
  break printed as it is, `… 1 more row` (S6, extending D257 S9).

Marked **[author]** after acceptance (Notes, 2026-11-19). M4 built each
of these, and the twelfth review found them unstated. They are not
among the choices accepted on 2026-11-18, and are listed for the owner
to see:

- `frame.join`'s and `frame.matrix`'s refusals in a stated order, and
  `"refuse"` naming the first NONE in the order the word reads its
  columns, each from row 1 (S5; SF3). The note applying SF3 marked
  these without adding them here; they are added now;
- a `by` column that a statistic also reads is read under `na` (S5; A1);
- a column of no kind accepted with rows, by `frame.aggregate`'s
  statistics and by `frame.matrix` (S5; A2);
- `.dropped` added under `"omit"` even when every statistic is
  `"count"` (S5; A4);
- a LIST literal cannot hold NONE, so NONE enters a LIST by `+`, JSON's
  `null` or `frame.csv.parse` (S3; A5).

None is open in `docs/registers/open-questions.md` for this RFC: Q63 is
answered (D257 B3), Q65 (the byte-order mark) by the third review, and
Q64 (R's defaults) from R's source, below.

### Differences from R

P6, P7 and P11 cite R as reasoning, not as a reference this RFC must
match. Read from R 4.4.2's source (Q64, which cites each line through
the `wch/r-source` mirror at the tag `R-4-4-2`). Q64 asked about P6, P7
and P11 and found the first two differences below; the blank-line and
ragged-row differences are this section's own, read from the same
files:

- **P11 matches R.** `order`'s default is `na.last = TRUE`, missing
  values last (`src/library/base/R/sort.R:192`).
- **P6 differs on blanks in text columns.** R's `read.table`, and so
  `read.csv`, defaults to `na.strings = "NA"`
  (`src/library/utils/R/readtable.R:68`, `:297-302`), and treats a blank
  field as missing only in logical, integer, numeric and complex columns
  (`src/library/utils/man/read.table.Rd:124-127`): a blank in a text
  column stays `""`. Here `""` is among the default `na` strings, so a
  blank is NONE in every column, as polars reads it (`data.rs`'s module
  doc: "An empty field is null"); a program wanting
  R's reading of a text column passes `{ na: [ "NA" ] }` and accepts
  that a gap then makes its numeric column a string column.
- **P7 differs on collation.** R's `factor` sorts its levels
  (`src/library/base/R/factor.R:24-28`), but a character vector is
  ordered by the locale's collation (`src/library/base/man/factor.Rd:181-183`,
  `src/library/base/man/sort.Rd:61-63`). Here levels sort by UTF-8 bytes,
  which is code-point order (D106), the same on every machine and the
  same as `frame.sort`.
- **D258 B1 differs on blank lines.** R's `read.table` skips them by
  default (`blank.lines.skip = TRUE`, `src/library/utils/R/readtable.R:71`;
  "blank lines in the input are ignored",
  `src/library/utils/man/read.table.Rd:169-170`). Here every line is a
  record: a blank line is a NONE in a one-column frame and E102 in a
  wider one (D258 B1).
- **A ragged row differs.** `read.csv` defaults to `fill = TRUE`
  (`src/library/utils/R/readtable.R:299`): "If TRUE then in case the rows
  have unequal length, blank fields are implicitly added"
  (`src/library/utils/man/read.table.Rd:159-160`). Here a row of another
  width than the header is E102 (S6).
- **`frame.factor` of a factor matches R**: unused levels dropped, the
  rest kept in their order (S4).
