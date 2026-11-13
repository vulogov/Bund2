# threads_demo_inner

Several Bund VMs, each on its own thread, sending work to one another and
answers back over the local bus, started and joined from Bund. The same
pool and pipeline as [`threads_demo`](../threads_demo/), without the Rust
host: one `demo.bund`, run by `bund2`.

    make

or, from this directory:

    bund2 demo.bund

## The three words

D249 added them, in every build, alongside everything that was there; a
program that defines a word of the same name still calls its own.

| Word | Stack | What it does |
|---|---|---|
| `vm.spawn` | `program values -- handle` | Starts a VM on its own thread. `program` is a LAMBDA or a STRING of Bund source; `values` is a LIST, whose items the new VM finds on its stack, first item lowest. Answers an INTEGER handle, never reused |
| `vm.join` | `handle -- list` | Waits for that VM and answers its final stack as a LIST. If the VM stopped with an error, `vm.join` fails with it. A handle is joined once |
| `vm.done?` | `handle -- bool` | Whether that VM has finished, without waiting |

A new VM starts with the standard vocabulary and nothing else: no words or
variables of the VM that started it. So a LAMBDA program defines every
word it needs inside itself, as `:worker` defines `fib`. It also starts
with the same `--noio`, `--noeval` and `--nocolor` as its parent, and under
`--noeval` a STRING program is refused, a LAMBDA is not.

At most 64 VMs started with `vm.spawn` run at once; past that `vm.spawn`
fails until one ends. A program that ends with VMs it never joined gets one
warning, and those VMs end with it.

## Part 1: a worker pool

    this VM --jobs--> w1 … w4 --results--> this VM

`:worker` is one LAMBDA, started four times with its name as the start-up
value. The program sends twelve Fibonacci jobs, 16 to 25, to `jobs`, then
one `"stop"` per worker, and prints the answers as they arrive on
`results`. A worker that reads `"stop"` ends, leaving its name and how many
jobs it handled on its stack, and `vm.join` hands that back. One run:

    w2  fib(18) = 2584
    w4  fib(20) = 6765
    …
    12 answers, total 316214
    w1 handled 1
    w2 handled 2
    w3 handled 4
    w4 handled 5

Which worker takes which job, the order of the answers and each worker's
count change from run to run; the answers and their total do not.

## Part 2: a pipeline

    this VM --p.numbers--> square --p.squares--> sum --p.sums--> this VM

`:stage` is one LAMBDA, started twice with an inbox, an outbox and a step,
a LAMBDA it registers as the word `step`. The program sends 1 to 10 and a
`"stop"`, which each stage passes on before it ends. This part prints the
same lines every run:

    running sums of the squares of 1 to 10:
    1 5 14 30 55 91 140 204 285 385
    square handled 10
    sum handled 10

## Compared with threads_demo

There, a Rust host starts every VM, including the one that drives, and a
worker reports its count on a tally channel. Here the program that drives
is the one `bund2` runs, it starts the others, and a worker's count is its
final stack. The bus is used the same way, and
[`threads_demo`'s README](../threads_demo/README.md) says the three things
to know about it: create a channel before `recv` on it, `recv` does not
wait, and a shared channel gives each value to one reader.

## Pitfalls

**`[ ]` and `{ }` are parse errors.** An empty LIST is `list`; a LAMBDA
needs a term.

**`times` pushes its counter each round.** A body that does not use it
starts with `drop`, or the counters pile up on the stack.

**A VM's values do not cross by reference.** The program, the start-up
values and the final stack are encoded and copied, as values sent on the
bus are.
