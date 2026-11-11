# smysl_demo_full

Logic programming and SMYSL in one program, with a local model: why the
Fenwick reservoir turned green. It uses every `smysl.` word of RFC-0014,
Phase 1's and Phase 2's, and every `kb.` word of RFC-0013, with its
control, bridges, tabling, saturation, questions to the user, proof trees
and CLP(FD).

    make            # build a smysl-ollama bund2, then run demo.bund
    make clean      # remove the library a run left

or, with such a `bund2` on the PATH, from this directory:

    rm -rf fenwick && bund2 demo.bund

## What it needs

- A `bund2` built with `--features smysl-ollama`. The Makefile builds one
  in `target/smysl-ollama`, the directory `bund_examples/smysl_model` uses.
- An Ollama at `http://127.0.0.1:11434` that has pulled `llama3.2`, the
  model SMYSL asks for when no configuration names another
  (`ollama pull llama3.2`). `bund_examples/smysl_model/README.md` says how
  to name another model in `.smysl/config.hjson`.

Every model call passes `offline`, so a configuration that routed the work
to a hosted provider would be refused before anything was sent. A run took
under a minute when it was written, most of it the model's.

## What a run prints

A model's answers are not fixed, so a run prints differently each time and
there is no `expected.txt`. The analyst's units are the same on every run,
and so is everything reasoned from them alone: sections 10 to 14 print the
same lines every time. What the model adds changes the counts around them,
and how many units the rules in section 15 find.

The text ingest and the answer report SMYSL's warnings on standard error,
many of them with a small model: bodies under SMYSL's default length
(`SMY-W041`), a quotation reworded (`SMY-W308`), a `source` the reading
supplies itself (`SMY-W444`).

**`llama3.2` rarely answers cleanly.** Its answers often have a status SMYSL
does not know (`SMY-E001`) or a gist past the default profile's 30 tokens
(`SMY-E022`). Such a batch is not clean, so section 8 lists the errors and
does not commit it. That is the commit refusing to write what SMYSL's
checks would reject, not the program failing.

## The sections

| | What it shows | Words |
|---|---|---|
| 1 | A library, its handle, an agent to attest as, a Markdown text | `smysl.create`, `smysl.open`, `smysl.agent`, `smysl.text.add`, `smysl.texts` |
| 2 | A report read by the model, staged, and committed only if clean | `smysl.ingest`, `smysl.commit` |
| 3 | The text read window by window, then again from its journal with no model call | `smysl.ingest.text` |
| 4 | The analyst's account: SMYSL's syntax, one MAP at a time, labels, relations, and edges from the model's units to the reservoir | `smysl.parse`, `smysl.append`, `smysl.unit`, `smysl.label`, `smysl.rel` |
| 5 | SMYSL's checks, and the library as text | `smysl.check`, `smysl.write` |
| 6 | Lineage both ways, salience, a token budget, a dossier, a measure | `smysl.get`, `smysl.trace`, `smysl.status`, `smysl.salience`, `smysl.pack`, `smysl.dossier`, `smysl.measure` |
| 7 | A count, groups, a ranked search, and a lambda per claim | `smysl.query`, `smysl` |
| 8 | A question answered by the model over its dossier | `smysl.answer`, `smysl.commit` |
| 9 | The library as facts, asked every way, and a proof tree | `smysl.kb`, `kb.clauses`, `kb.query`, `kb.holds`, `kb.once`, `kb.call`, `query` with `skip_first`, `limit` and `prove` |
| 10 | A KB written in a named stack, edited, merged, edited on the workbench, and capped | `kb.consult*`, `kb.new`, `kb.assert`, `kb.asserta`, `kb.retract`, `kb.merge`, `kb.inferences`, `kb.assert.`, `kb.retract.`, `kb.merge.` |
| 11 | Control, and Bund inside a rule, `smysl.trace` among it | `=`, `not`, `or`, `if`, `once`, `call`, `findall`, `dif`; `is`, `test`, `in` |
| 12 | A left-recursive predicate tabled, saturation, a predicate made a word | `kb.table`, `kb.saturate`, `kb.word` |
| 13 | An expert system that asks, with seeded replies, and a proof | `kb.askable`, `query` with `ask` and `known`, `kb.prove` |
| 14 | A remediation plan by constraint propagation and labeling | `in`, `ins`, `#=`, `#≠`, `#<`, `#>`, `#<=`, `#>=`, `all_different`, `label`, `labeling`, and an `fd_dom` residual |
| 15 | Proofs written back into the library, over the analyst's units and the model's; a retraction and what it shakes | `smysl.prove`, `smysl.retract`, `smysl.status` |

## Three things a reader trips on

**A dossier needs an entity and mentions.** `smysl.dossier` and
`smysl.answer` build around an `x.text/entity` the question names, through
`x.text/mentions` edges. `smysl.ingest` writes neither, so section 4 writes
the reservoir as an entity, sourced to the text's alias, and links the
model's units to it.

**`[ june july ]` is a term, not a LIST.** A LIST whose first item is an
atom is a compound, `june(july)`. A LIST of atoms is written
`[ :june :july ]`, as section 11's `summer` rule does.

**A bridge's operands are in Bund's order.** `{ T C > }` asks whether C is
greater than T, and `{ J C - }` is C minus J.

## Locks

Every word that writes takes SMYSL's own locks on the library.
`smysl.ingest.text` holds them for the whole run, so another writer waits
until it is done. `bund_examples/smysl_demo/README.md` lists the SMYSL
commands that append without locking. None of them may write this library
while the demo runs.
