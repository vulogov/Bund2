# What a small query pays to compile its predicate (Q46)

Measured 2026-11-01 on macOS (Darwin 27.0.0), arm64, an Apple M5 Max, with
rustc 1.95.0, in a release build (`cargo bench`). Bund2 is commit 71aa577
with the working tree of the same day (D238–D241, none of which touches
compilation). Asked by Q46, which the owner ruled "measure first": RFC-0013
§S2 proposes no compiled-form cache "until a measurement asks for one".

## The question

Each activation (every `kb.query`, `kb.holds`, `kb.once`, `kb.call`, a
`kb.word` call, a `query` run) compiles each predicate it calls on its first
call: every clause of the predicate is copied into the activation's template
store (`Solver::compile`). A KB is a value, so a cache of that work keyed on
its payload could never go stale (D5, D35). Does the compile cost enough,
against the solving, to ask for one?

## How it was run

The `kb` bench gains three cases over a KB of `n` facts `[ p 0 ] … [ p n-1 ]`
and one fact `[ r 1 ]`, for `n` of 100, 1,000 and 10,000. Each times one
activation from the start, as every asking word makes one:

- `kb/new{n}`: the activation made, nothing solved;
- `kb/hit{n}`: the first answer of `[ p 5 ]`, which compiles `p`'s `n`
  clauses and builds one first-argument index entry;
- `kb/any{n}`: the first answer of `[ p X ]`, which compiles them and needs
  no index;
- `kb/other{n}`: the first answer of `[ r 1 ]`, which compiles one clause.

Each first answer takes one inference.

    cargo bench -p bund2-bench --bench kb -- 'kb/(new|hit|any|other)'

## Results

Two runs; criterion's mean, the second run's.

| n | `new` | `hit` | `any` | `other` |
|---|---|---|---|---|
| 100 | 0.79 µs | 9.89 µs | 9.91 µs | 1.21 µs |
| 1,000 | 0.79 µs | 95.6 µs | 95.5 µs | 1.22 µs |
| 10,000 | 0.79 µs | 1.012 ms | 1.003 ms | 1.24 µs |

The first run, without `any`, agreed within 4%.

- **An activation costs the same whatever the KB holds**: 0.79 µs at every
  size, and a query of a one-clause predicate 1.2 µs. A predicate a query
  does not call costs it nothing.
- **A predicate it calls costs about 100 ns a clause, every activation**:
  9.9 µs for 100 clauses, 1.0 ms for 10,000, linear. `hit` and `any` agree,
  so the index entry is negligible and the cost is the copy.
- **Against the solving**, a query whose answer takes one inference spends
  over 99% of its time compiling at 1,000 clauses: 95.6 µs, against the
  1.2 µs the same query costs on a predicate of one clause.

## What it shows, and what it does not

**A cache would pay where a program asks many small queries of a large
predicate**: a `kb.word` or `kb.holds` called in a loop over a predicate of
10,000 facts recompiles them every call, 1 ms each, so 10,000 lookups take
about 10 s that a cache would cut to about 13 ms. Worked example 4's shape,
a word over a KB, is that loop when it is called many times.

**It would pay little for one large query**, where compiling is paid once
per predicate and not once per answer: `kb/join10000`
(`docs/measurements/kb-2026-10-30.md`) compiles its 20,001 clauses once for
10,000 answers. How that compile compares with its search was not measured.

It does not measure a cache: what one would cost to look up and to keep, or
how the template store would be shared between activations, since templates
now live in each activation's own arena. That is the design question a
decision would have to answer.

## After: D242, a clause compiled when it is tried

The owner ruled option (c) first: compile only the clauses a call tries,
reading the index's keys off the stored values (`Arena::shape`), and only
when a keyed call needs them. Measured the same way, one run:

| n | `new` | `hit` | `any` | `other` |
|---|---|---|---|---|
| 100 | 0.87 µs | 2.30 µs | 1.75 µs | 1.36 µs |
| 1,000 | 0.85 µs | 17.8 µs | 8.86 µs | 1.26 µs |
| 10,000 | 0.82 µs | 168 µs | 78.2 µs | 1.32 µs |

- **A keyed query of 10,000 clauses**, from 1.0 ms to 168 µs: about 17 ns
  a clause, the key scan and the copy of the stored values.
- **An unkeyed one**, from 1.0 ms to 78 µs: about 8 ns a clause, the copy
  of the stored values and the per-clause slots, with no key read.
- **An activation and a one-clause predicate** are unchanged within the
  runs' spread.

An intermediate build that read every clause's head and key at the first
call measured 246 µs and 244 µs at 10,000; reading only the first
argument, and only when needed, is the difference.

**Naive reverse is unchanged.** `kb/nrev30x10000` and `kb/nrev120x677`,
paired against `71aa577` in a separate worktree, two rounds alternated:

| Round | 71aa577, 30 / 120 | D238–D242, 30 / 120 |
|---|---|---|
| 1 | 2.147 s (noisy, ±0.11 s) / 1.791 s | 1.732 s / 1.750 s |
| 2 | 1.750 s / 1.758 s | 1.733 s / 1.756 s |

What remains is still linear in the predicate's size. A cache (Q46's
option (b)) would remove it, and stays a later step for a measurement to
ask for (D242).
