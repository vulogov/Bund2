# `bund2-bench` — in-process benchmarks

    cargo bench -p bund2-bench
    cargo bench -p bund2-bench -- arith          # one group
    cargo bench -p bund2-bench -- --save-baseline before

## Why this exists

**Q14.** `cargo xtask bench` times `bund2 script --file …` as a subprocess, and
that measurement cannot see the interpreter:

| | end to end | in process |
|---|---|---|
| register the whole stdlib | — | **32 µs** |
| evaluate a real corpus program | — | **~9 µs** |
| fastest program, whole run | **2.3 ms** | — |

Registering 261 words is 32 µs against a 2.3 ms floor, so **98.6% of the fixed
cost is process spawn** and an ordinary program's evaluation is about 0.4% of
what the subprocess harness reports. Three consecutive harness runs spread the
corpus total by ~8 ms, which is several times the corpus's entire interpreted
content. No performance criterion for RFC-0001 or RFC-0005 can rest on it.

`cargo xtask bench` keeps the job it is good for: catching a regression that
makes startup dramatically worse, and comparing the two targets end to end —
Bund2 is 5.5× faster than the oracle over the corpus that way, which is a fair
use of it because that difference *is* startup.

## The rule

**Parse and register outside the timed region.** Every `iter_batched` builds its
`Interp` in the setup closure, so registration is excluded from everything
except the group that measures it deliberately. Getting this wrong produces a
"20% faster" that is 80% stdlib registration.

## The groups

| group | what it is for |
|---|---|
| `startup` | registration and parsing. **A JIT must not move these** |
| `dispatch` | repeated word calls — what an inline cache acts on |
| `arith` | tight arithmetic, the classic JIT target |
| `lambda` | call overhead — RFC-0003's frame loop, tail calls |
| `corpus` | real programs, evaluated. The honest headline |
| `rendering` | **not interpretation** — `display` through termimad |

`rendering` is separate because it has to be. `pull_demo` reaches `display` and
costs **3.9 ms** where the corpus programs beside it cost 8.5 µs — four hundred
times an entire ordinary program, for one rendered table. In the first version
of this harness it sat in `corpus` and was 99.7% of the group; any JIT claim
measured there would have been a claim about terminal layout.

Corpus programs write to stdout, so all of them carry some I/O.
`programs/mixed.bund` is deliberately silent and is the clean interpretation
number.

## Comparing a change

    cargo bench -p bund2-bench -- --save-baseline before
    # ... change something ...
    cargo bench -p bund2-bench -- --baseline before

Criterion reports the delta per benchmark with a confidence interval, which is
what an RFC-0005 acceptance criterion should be phrased against — a named group,
a direction, and an interval — rather than a percentage over the corpus.
