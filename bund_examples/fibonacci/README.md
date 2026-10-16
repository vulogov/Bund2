# Fibonacci

A naive recursive Fibonacci in Bund, with the same algorithm in Python beside
it, and a `Makefile` that builds a standalone binary from the Bund program and
times the two. It is the example used to measure the interpreter, so it is
deliberately the slow way to compute Fibonacci numbers.

## The algorithm

    fib(n) = n                      when n < 2
    fib(n) = fib(n-1) + fib(n-2)    otherwise

Every call for `n ≥ 2` makes two more, so `fib(30)` makes about 2.7 million
calls. That exponential cost is the point: the program spends its time in word
calls, integer arithmetic and stack shuffling, which is what the interpreter's
speed is made of.

## The Bund program

```bund
:fib {
  dup 1 < {
    dup 1 swap - fib    // n fib(n-1)
    swap 2 swap - fib   // fib(n-1) fib(n-2)
    +
  } if
} register
```

Read it with the stack in mind. `:fib { … } register` binds the word `fib` to
the body. On entry the stack holds `n`.

- `dup 1 <` copies `n` and asks whether `1 < n`. Bund's binary words take their
  **right** operand first: `a b <` asks `b < a`, and `a b -` computes `b - a`.
  So `1 <` with `n` beneath it is the test `n > 1`, which is the `n < 2` base
  case turned around.
- `{ … } if` runs the block only when the test is true. When it is false the
  stack still holds `n`, and `n` is the answer for 0 and 1.
- Inside the block, `dup 1 swap -` leaves `n` and `n - 1`; `fib` replaces the
  top with `fib(n-1)`. The stack is now `n fib(n-1)`.
- `swap 2 swap -` brings `n` up and computes `n - 2`; `fib` replaces it with
  `fib(n-2)`. The stack is now `fib(n-1) fib(n-2)`.
- `+` adds them, leaving one value, as the base case did.

The recursion is direct, so it runs on the interpreter's heap-allocated frames
rather than the machine stack, and depth is not a concern at these sizes.

The rest of the file reads `N` from the command line:

```bund
args.parse :n ?key
{ drop 30 }
{ :n get car convert.to_int }
?true*

fib println
```

`args.parse` turns the program's arguments into a dictionary of flags, so
`--n 25` becomes the key `n` holding the list `["25"]`. `:n ?key` leaves the
dictionary and pushes whether that key is present. `?true*` then runs one of
the two blocks: the first when the flag is absent, which drops the dictionary
and pushes the default `30`; the second when it is present, which fetches the
list, takes its first element with `car`, and converts the string to an
integer. Either way one integer remains, and `fib println` prints its
Fibonacci number.

## Running it

From the repository root, with `bund2` built:

    bund2 bund_examples/fibonacci/fibonacci.bund                 # fib(30) = 832040
    bund2 bund_examples/fibonacci/fibonacci.bund --n 10          # 55
    bund2 script --file bund_examples/fibonacci/fibonacci.bund -- --n 10

With the bare path everything after it belongs to the program; with `--file`
the program's arguments follow `--`. The Python twin takes the same flag:

    python3 bund_examples/fibonacci/fibonacci.py --n 10

## Building a standalone binary

`bund2 build` compiles nothing. It copies the `bund2` executable that runs it
and writes the program's text, deflated, into a region reserved inside every
`bund2` binary. The copy finds the program in its own memory when it starts and
runs it, handing it the whole command line. No Rust toolchain, C compiler or
linker is needed on the machine doing the building. On macOS the copy is
re-signed with the system's `codesign`, because an edited binary will not run
there until it is; on Linux nothing more is needed.

    cd bund_examples/fibonacci
    make                 # ./fibonacci, from the Tier 0 interpreter
    ./fibonacci --n 25
    bund2 build --inspect fibonacci     # what the binary says about itself

The bundle carries whatever runtime built it. The `Makefile` builds `bund2`
itself first, in release mode, so the artefact is as fast as the interpreter
can be:

    make bench           # time ./fibonacci and fibonacci.py at N=30
    make bench N=25
    make bench-jit       # also build bund2 with the Cranelift JIT and time
                         # a bundle made from it
    make clean

`make bench-jit` builds a second `bund2` with `--features jit` in its own
target directory, so the default build is left alone, and makes `fibonacci-jit`
from it. Both binaries are ignored by git.

If `cargo` is managed by `mise` on your machine, the Makefile's `cargo` call
may need the toolchain override dropped:

    make bench CARGO="env -u RUSTUP_TOOLCHAIN cargo"

## What the numbers mean

`make bench` prints `real`, `user` and `sys` for each run. `user` is the time
the interpreter spent computing; compare that one across runs. `real` includes
whatever else the machine was doing. The Bund program and the Python program
make the same calls in the same order, so the ratio between them is a fair
measure of the two interpreters on this kind of work and nothing else.
