# smysl

SMYSL and logic programming together in Bund: why the village well at
Ashby ran dry. A SMYSL library is formed three ways, then checked,
researched, measured and queried. It then becomes a knowledge base that
rules reason over, and `smysl.prove` writes each proof back into the
library, where the next query, rule or proof can use it.

[`smysl_demo/`](../smysl_demo/) is the short tour RFC-0014 asks for.
This is the long one: every `smysl.` word, and every place the `kb.` words
meet a library.

| | What it shows | Words |
|---|---|---|
| 1 | A library, and a Markdown text read into it; the same text added twice is one manifest | `smysl.create`, `smysl.text.add`, `smysl.texts` |
| 2 | A corpus in SMYSL's own syntax: an entity, a question, observations, claims and their relations | `smysl.parse`, `smysl.append` |
| 3 | Later evidence as Bund MAPs, naming its grounds by label, and a LIST of MAPs built with `map` | `smysl.unit`, `smysl.label`, `smysl.rel`, `smysl.append`, `smysl.kb` |
| 4 | SMYSL's checks, clean because the `x.text` schema is declared with the edges that use it, and the library written out as text and parsed back | `smysl.check`, `smysl.write`, `smysl.parse` |
| 5 | Lineage both ways, salience, and what fits a token budget | `smysl.trace`, `smysl.get`, `smysl.salience`, `smysl.pack` |
| 6 | A dossier around an entity, a measure that runs and one that cannot | `smysl.dossier`, `smysl.measure` |
| 7 | A count, groups, a ranking, and a lambda run per unit | `smysl.query`, the `smysl` conditional |
| 8 | The library's facts, asked together; CLP(FD) over `rank`; the `query` conditional over a KB of one kind | `smysl.kb`, `kb.query`, `kb.holds`, `#>=` |
| 9 | Rules: a tabled closure, a predicate as a word, `test`, `is`, and `in` calling `smysl.trace` while solving; saturation | `kb!`, `kb.merge`, `kb.table`, `kb.word`, `kb.saturate` |
| 10 | A goal no fact proves, answered from seeded replies; `smysl.prove` refusing to guess | `kb.askable`, `query`'s `known` and `ask` |
| 11 | Proofs written as findings at the status their grounds allow, a proof by negation, a program fact authored as cited with a source | `smysl.prove` |
| 12 | A rule resting on the findings section 11 wrote, and the proof grounded on them | `smysl.kb`, `smysl.prove` |
| 13 | A retraction, seen as an `effective` fact, and the claims it shakes proved | `smysl.retract`, `smysl.status` |
| 14 | What the words refuse: SM08, SM03, SM05, SM00 | `?try` |

## Running it

The `smysl.` words exist only in a `bund2` built with `--features smysl`:

    make            # build that bund2, then run demo.bund
    make check      # run it and compare with expected.txt and expected.stderr.txt

or, with such a `bund2` on the PATH, from this directory:

    rm -rf ashby && bund2 demo.bund

**Each run starts from no library.** The demo writes its library to
`ashby`, a root relative to where it runs. A second run against the same
library would find the first's units and proofs there and print
otherwise, so the Makefile removes `ashby` first. Nothing is read from the
terminal: section 10's replies are seeded in `known`.

## The output

`expected.txt` is what a run prints. It shows no clock and no path: uids
are hashes of content, the same on every machine, and a proof's gist cuts
each uid to twelve characters. `expected.stderr.txt` holds the three
warnings raised on the way. Re-parsing the written library skips its
text's manifest and its `x.text` schema declaration, neither of which has a
MAP form. The T1 measure needs ten thousand tokens of text and the survey
has 59.

A test in `crates/bund2-cli/tests`, `smysl_logic_demo.rs`, copies
`demo.bund` into a fresh directory, runs it there and compares both
files; CI runs it in the `smysl` build. It is not a golden: the reference
has none of the `smysl.` words.

## How the two meet

- **`smysl.kb` is the bridge in.** It turns the units a query names into
  facts: `unit` (uid, schema, status), `gist`, `grounds`, `deps`,
  `label`, `rel` (kind, from, to), `effective` for a unit retraction
  changed, and six `rank` facts, `unfounded` 0 to `measured` 5. Those
  facts merge with rules like any KB's.
- **Bund reaches into a rule** through bridges: `test` and `is` run a
  lambda over the bindings, and `in` ranges over what one answers. A
  lambda can call any `smysl.` word, so a rule can read the library while
  it is being solved, as section 9's `upstream` does.
- **`smysl.prove` is the bridge out.** Each proof becomes units:
  - a fact the library bears out stands for its unit;
  - any other fact is a program fact, a claim at `opts.facts`'s status,
    speculative unless named, with `opts.source` when it is cited;
  - each conclusion is a claim grounded on its premises, and the root a
    finding, never stronger than its weakest ground, and capped at
    `inferred` when it went through `not`, the `else` of an `if`, or a
    tabled call.

  The attestation of each names its grounds as parents.
- **The `x.text` schema is declared for you.** A dossier needs an
  `x.text/entity` and `x.text/mentions` edges onto it. A batch that writes
  one carries SMYSL's `x.text/v1` declaration when the library lacks it,
  as SMYSL's own text ingest does, so `smysl.check` does not warn
  `SMY-W013` (D247).
- **A proof that asks cannot be written.** `smysl.prove` has no `ask`
  slot, so a goal that reaches an askable predicate is refused (E50), and
  nothing is written.

## One writer at a time

Every `smysl.` word that writes takes SMYSL's own locks on the library.
Six SMYSL commands append without them: `q --save`, `ask --save`,
`measure` and `assess` unless given `--dry-run`, `same-as`, and
`ingest --text`. **A library a Bund program writes must not be written by
any of those six at the same time** (RFC-0014, R22).
