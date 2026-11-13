# threads_demo

Several Bund VMs, each on its own thread, sending work to one another and
answers back over the local bus.

    make                  run with 4 workers
    make run WORKERS=8    run with 1 to 16 workers
    make compare          run with 1 worker, then with 4

or, from the repository root:

    cargo run -p threads_demo -- 4

## Why there is Rust here

The reference has no word that starts a VM or a thread, and RFC-0007 §C6
made concurrency in Bund2 an embedding feature: a host starts the VMs, and
each runs a Bund program. This example shows that embedding. Since D249,
Bund can also start VMs itself with `vm.spawn`;
[`threads_demo_inner`](../threads_demo_inner/) is this demo written that
way, with no Rust. `src/main.rs` is the host, and all
it does is start VMs with `bund2_async::host::Host`, wait for them, and
time each part. Everything the VMs do is Bund, in the four `.bund` files,
which the host compiles in.

A VM cannot move between threads (`Interp` is not `Send`), so `Host`
builds each one on its own thread. What crosses between them is values,
encoded, through the bus: `send` and `recv` use one map of named channels
for the whole process. Two separate `bund2` processes do not share it.

## The files

| File | What it is |
|---|---|
| `stage.bund` | A stage: read a value from an inbox, run `step` on it, send the result to an outbox, until it reads `"stop"` |
| `words.bund` | The work: `fib`, `fib-job`, `square`, `running-sum`. Every VM loads it, because each VM has its own word table |
| `pool.bund` | Part 1's driver |
| `pipeline.bund` | Part 2's driver |
| `src/main.rs` | The host |

## Part 1: a worker pool

    driver --jobs--> w1, w2, … wN --results--> driver

The driver sends twelve jobs, Fibonacci numbers from 16 to 25, to `jobs`,
then one `"stop"` per worker. The workers all read `jobs`, so each job goes
to whichever worker asks first, and a slow job does not hold up the rest.
Each answer comes back on `results` as a MAP of `n`, `value` and `by`. The
driver prints the answers as they arrive, then the total, then how many
jobs each worker handled, which each worker reports on `pool.tally` when it
stops.

Which worker takes which job, the order the answers arrive in and each
worker's count change from run to run. The twelve answers and their total,
316214, do not. One run, in a debug build:

    w4  fib(18) = 2584
    w3  fib(20) = 6765
    …
    12 answers, total 316214
    w4 handled 2
    w2 handled 1
    w3 handled 5
    w1 handled 4
    (633 ms)

With one worker the same part took 1750 ms.

## Part 2: a pipeline

    driver --p.numbers--> square --p.squares--> sum --p.sums--> driver

The driver sends 1 to 10, then `"stop"`. `square` squares each number,
`sum` keeps a running total, and the driver prints the totals as they come
out. Each channel has one reader and delivers in order, and the `"stop"`
travels behind the numbers, so each stage stops only after it has passed
everything on. This part prints the same lines on every run:

    running sums of the squares of 1 to 10:
    1 5 14 30 55 91 140 204 285 385
    square handled 10
    sum handled 10

## Three things about the bus

**A channel must exist before anyone calls `recv` on it.** `recv` on a
name nothing has created is an error, not an empty answer, and with
several VMs no program order decides who creates a channel first.
`bus.data` creates the channel it asks about, so every VM creates its own
inbox with `"name" bus.data drop` before reading it. `send` creates a
channel too.

**`recv` does not wait.** An empty channel answers NODATA (type 97), so
each loop asks, and if it gets NODATA it yields the thread with `0 sleep`
and asks again. An idle VM therefore spins rather than sleeps, which is
why a run uses more CPU time than wall time.

**A shared channel gives each value to one reader.** Several readers can
share a channel, as the pool's workers share `jobs`. Each value goes to
exactly one of them, and the others are answered NODATA. `bus.data` only
says whether there was data when it asked, and with several readers
another may take it first, so the loops test what `recv` answered rather
than asking `bus.data` beforehand.

## When a VM fails

The host joins each VM as it finishes. If one answers an error, the host
prints it and exits with status 1 straight away. The other VMs may be
waiting on a channel the failed one would have written, and `recv` does
not wait, so they would spin for ever. Ending the process ends them too.
