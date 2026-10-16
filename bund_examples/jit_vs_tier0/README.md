# JIT versus Tier 0

One numeric loop, built twice from the same source: once into a bundle that
runs on the Tier 0 interpreter, once into a bundle whose runtime carries the
Cranelift JIT. `make bench` times the two. It is the example that shows what
the JIT is for, and, just as deliberately, the shapes of program it does not
speed up.

The comparison is Bund2 against Bund2. It is not a comparison with Python or
any other language; `fibonacci/` has one of those, and the last section says
why this kernel is not the place for another.

## The program

```bund
dup { 1 + 2 + 3 - 4 + 5 + 6 - 7 + 8 + 9 - drop } times
print " iterations" println
```

`times` runs the lambda N times and pushes the iteration index before each
run, so every run starts with an integer on top. The body is nine literal
operations on it and a `drop`. Bund's binary words take their **right**
operand first, so `x 3 -` is `3 - x`; followed through, the chain turns the
index `x` into `-(x + 3)`:

    x  →  x+1  →  x+3  →  3-(x+3) = -x  →  4-x  →  9-x  →  6-(9-x) = x-3
       →  x+4  →  x+12  →  9-(x+12) = -(x+3)

Then `drop` discards it. The loop keeps nothing: it exists to make the
interpreter do ten million rounds of pushing literals and adding and
subtracting integers, which is the work the JIT is built to take over. The
`dup` before the loop keeps a copy of N for the line printed at the end.

`N` comes from `--n`, ten million when absent, read the way `fibonacci.bund`
reads it: `args.parse` turns the arguments into a dictionary, `?key` asks
whether `n` is there, and `?true*` picks the default or the conversion.

## Running and building

    bund2 bund_examples/jit_vs_tier0/kernel.bund --n 1000000

    cd bund_examples/jit_vs_tier0
    make                 # ./kernel from the default bund2, ./kernel-jit from
                         # a bund2 built with --features jit
    make bench           # time both at N=10000000
    make bench N=1000000
    make stats           # what the tier did, from the JIT bundle
    make clean

The `Makefile` builds both `bund2` binaries first, the JIT one in its own
target directory so the default build is left alone, and makes a bundle from
each. A bundle is the `bund2` that built it with the program written,
deflated, into a region reserved inside the copy, so the two bundles hold the
same program and differ only in the runtime around it. Both are ignored by
git. If `cargo` is managed by `mise` on your machine:

    make bench CARGO="env -u RUSTUP_TOOLCHAIN cargo"

## What it measures

On the machine this was written on, an Apple Silicon laptop, `make bench`
gives, in `user` seconds for ten million iterations:

| Runtime | user | per iteration |
|---|---|---|
| Tier 0 interpreter | 2.6 s | 260 ns |
| Cranelift JIT | 0.43 s | 43 ns |

The JIT runs this loop about six times faster. `make stats` says what it did:

    bund2: tier compiled 1 bodies (9998976 entered), inlined 10 sites,
           promoted 9 values, 0 generic of 19, 0 crossed at threshold 1024

Read it left to right. The lambda's body was compiled once, after it had run
1024 times on the interpreter (the threshold; `BUND2_JIT_THRESHOLD` changes
it), and the compiled code was entered for every run after that. Its ten
sites, the nine operators and the `drop`, were all inlined: each is a word
the standard library publishes a *fragment* for, a few IR operations that
say what the word does to integers, and the tier lowers the fragment in
place instead of calling the word. The nine literals were promoted, held as
machine integers in registers rather than built as Bund values on the stack.
No site was generic, which is the figure to watch: a generic site is a word
with no fragment, and the compiled body has to box its values back onto the
stack, call the word, and read the result off again.

The time that remains, about 40 ns an iteration, is the loop itself: `times`
pushing the index as a Bund value and entering the compiled body through the
tier's calling convention. The body's arithmetic is a handful of machine
instructions inside that.

## The shapes it does not speed up

Change the body and run `make bench` again. Three edits, each measured on the
same machine, say where the gain comes from and where it stops.

**A user word in the body.** Move the chain into a word and call it:

```bund
:poly { 1 + 2 + 3 - 4 + 5 + 6 - 7 + 8 + 9 - } register
dup { poly drop } times
```

| Runtime | user |
|---|---|
| Tier 0 | 2.7 s |
| JIT | 0.68 s |

Still four times faster, but the stats line now reads `2 bodies` and
`1 generic of 20`. Words you register have no fragment, so the call to `poly`
is a generic site, made through the runtime, and the values cross the stack
in both directions on every iteration.
A lambda that is nothing but one word, `{ poly }`, measures the same as the
chain written out, one body and no generic site; it is the word *alongside*
other sites that pays.

**Division.** Replace the first `+` with `/`:

```bund
dup { 1 + 2 / 3 - 4 + 5 + 6 - 7 + 8 + 9 - drop } times
```

| Runtime | user |
|---|---|
| Tier 0 | 2.7 s |
| JIT | 0.83 s |

The tier's integer fragments are `+`, `-`, `*` and the six comparisons
`<`, `>`, `<=`, `>=`, `==` and `!=`, each added on its own with a measurement
before it and a differential test against the word after it. `/` has none, so it is a generic site in the middle of the chain,
and the values around it are boxed and unboxed on every iteration. One `/`
doubles the compiled time: it costs about as much as the loop and the other
eight operators together. `*` was in that position when this example was
first written, one `*` in the chain costing 1.1 s, and it got its arm the
same day; the stats line for the `*` variant now reads `0 generic of 19` and
the time is the 0.42 s of the original.

**A body of one word.** The classic accumulating counter:

```bund
0 swap { + } times println
```

| Runtime | user |
|---|---|
| Tier 0 | 0.29 s |
| JIT | 0.29 s |

No gain at all, and none is possible here. The body is a single `+`, and
nearly all of the 0.29 s is `times` pushing the index and entering the body,
which is the same for both runtimes. The JIT compiles the body, inlines its
one site, and has nothing left to remove.

## Why not against Python

The floor the last shape exposes, about 30 to 40 ns an iteration, is paid
for every run of every body, and a CPython bytecode loop spends about half
that on its whole iteration. So a demonstration against Python on a loop like
this would show Python ahead whatever the body does, and would be measuring
the loop's overhead rather than the compiled code. The gain the JIT delivers
is inside the body, which is why the honest comparison is the one `make
bench` makes: the same program, the same loop, with and without the tier.
`fibonacci/` is the example to run against Python, because there the work is
in word calls, which both interpreters pay for.
