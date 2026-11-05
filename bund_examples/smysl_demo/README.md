# smysl_demo

A SMYSL library formed, researched, queried and reasoned over from Bund,
with the `smysl.` words of RFC-0014: why the old mill grinds slowly.

1. **Forming, one MAP at a time.** Two cited observations and a claim
   inferred from them, written with `smysl.unit` and labelled with
   `smysl.label`; the claim names its grounds by their labels.
2. **Forming from text.** A letter and a second claim in SMYSL's own
   syntax, read by `smysl.parse` and written at once by `smysl.append`,
   then a `rebuts` relation from the new claim to the first with
   `smysl.rel`.
3. **Researching.** The claim read back with `smysl.get`, what it rests on
   with `smysl.trace`, and the most salient unit with `smysl.salience`.
4. **Retracting.** One observation retracted with `smysl.retract`, and
   `smysl.status` showing that under SMYSL's strict policy only that unit
   changes status: the claim resting on it keeps `inferred`.
5. **Querying.** A count with `smysl.query`, and the observations visited
   one by one through the `smysl` conditional.
6. **Writing it out.** The library as SMYSL's syntax, from `smysl.write`.
7. **Reasoning.** The library made a knowledge base with `smysl.kb`, three
   rules merged in, and `smysl.prove` finding the claim both rebutted and
   resting on a retracted ground. The proof is written back as a `finding`,
   grounded on the units its facts stand for, at a status no stronger than
   theirs.

## Running it

The `smysl.` words exist only in a `bund2` built with `--features smysl`:

    make            # build that bund2, then run demo.bund
    make check      # run it and compare the output with expected.txt

or, with such a `bund2` on the PATH, from this directory:

    rm -rf mill && bund2 demo.bund

**Each run starts from no library.** The demo writes its library to
`mill`, a root relative to where it runs. A second run against the same
library would find the first's retraction and proofs there and print
otherwise, so the Makefile removes `mill` first.

## The output

`expected.txt` is what a run prints. It shows no clock and no path: uids
are hashes of a unit's content, the same on every machine, and a proof's
gist cuts each uid to twelve characters. A test in `crates/bund2-cli/tests`,
`smysl_demo.rs`, copies `demo.bund` into a fresh directory, runs it there
and compares the two; CI runs it in the `smysl` build. It is not a golden:
the reference has none of the `smysl.` words, so there is no oracle to
capture one from.

## One writer at a time

Every `smysl.` word that writes takes SMYSL's own locks on the library, so
it is serialised against other Bund programs, in this process or another,
and against SMYSL's `lib` and `text` commands, which take them too.

Six SMYSL commands append **without** locking: `q --save`, `ask --save`,
`measure` and `assess` unless given `--dry-run`, `same-as`, and
`ingest --text`. **A library a Bund program writes must not be written by
any of those six at the same time.** Against them nothing can be promised:
one of them part way through a record when Bund opens the library leaves a
tail that Bund's append cuts off, and neither side reports it (RFC-0014,
"What the locks do not cover", R22).
