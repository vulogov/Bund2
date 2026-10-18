# Examples

Programs written in Bund, each in its own directory with a `Makefile` that
turns it into a standalone binary with `bund2 build` and a `README.md` that
explains it. They are not part of the conformance corpus; that lives under
`tests/`. These are here to be read, run and changed.

| Directory | What it is |
|---|---|
| [`fibonacci/`](fibonacci/) | The naive recursive Fibonacci, with the same algorithm in Python beside it and a `make bench` that times the two. The example used to measure the interpreter. |
| [`bund_repl/`](bund_repl/) | A read–eval–print loop written in Bund: `input*` reads the lines, `bund.eval` runs them in the same VM, `?try` keeps a failing line from ending the session, and a dictionary of lambdas is the command table. |
| [`jit_vs_tier0/`](jit_vs_tier0/) | One numeric loop built twice, into a Tier 0 bundle and a Cranelift JIT bundle, and a `make bench` that times the two: ten million iterations in 2.5 s on the interpreter and 0.42 s on the JIT. The example that shows what the JIT is for, and the shapes it does not speed up. |
| [`csv_stats/`](csv_stats/) | One column of a CSV file, read through the `csv` conditional, and its count, minimum, maximum and mean from the four statistics words, with their `,` forms keeping the list between questions. The example for reading data in. |
| [`profiler_demo/`](profiler_demo/) | Every feature of `debug.profile`, which writes the profile of the run so far: snapshots at the top level, from deep inside a recursion, and one file rewritten as a run goes on, beside the end-of-run report; a failed write caught by `?try`; the warning without `--profile`, and the stub under `--noio`. The example for profiling. |
| [`graph_demo/`](graph_demo/) | Every graph word — the reference's seven, the twelve Bund2 adds, their workbench forms and `io.graph` — over build tasks, roads, ferries and water mains: building and checking a graph, a build order and the cycle that breaks it, the groups a graph falls into, a minimum spanning tree, shortest paths and distances with negative weights, an exact maximum flow, a graph kept in a file with `encode.base64` and read back, and what the words refuse: weights that are not finite, wrong types named in the message, and `found=false` where the reference's `graph.path` crashes. Every answer in node order, so two runs print the same. The example for graphs. |
| [`object_oriented/`](object_oriented/) | Every part of Bund's object system, over shapes, a duck, small class trees and the built-in classes: defining, registering and resolving a class, objects, fields and methods, constructors and the order they run in, inheritance and multiple inheritance with the first match winning, a method written once that each object answers in its own way, the base classes' `.id`, `.timestamp`, `str` and `.template`, the classes that wrap one value with `wrap`, `unwrap`, `is` and `#`, `List`, `Floats` and `Intervals`, and what Bund2 adds: `<class> !`, `?is` and the workbench forms. The example for objects. |

## Running an example

Any of them runs straight from its source, with `bund2` built:

    bund2 bund_examples/fibonacci/fibonacci.bund --n 20
    bund2 bund_examples/bund_repl/repl.bund
    bund2 bund_examples/jit_vs_tier0/kernel.bund --n 1000000
    cd bund_examples/csv_stats && bund2 stats.bund --column humidity
    cd bund_examples/profiler_demo && mkdir -p profiles && \
        bund2 script --profile profiles/end --file demo.bund -- --out profiles
    bund2 bund_examples/graph_demo/demo.bund
    cd bund_examples/object_oriented && bund2 demo.bund

With the bare path, everything after it belongs to the program. The
`script --file` form takes the program's arguments after `--`:

    bund2 script --file bund_examples/fibonacci/fibonacci.bund -- --n 20

## Building one

Each `Makefile` builds `bund2` itself in release mode, then runs
`bund2 build --file <program> --output <binary>`:

    cd bund_examples/fibonacci && make
    ./fibonacci --n 25

`bund2 build` compiles nothing. It copies the `bund2` executable and writes the
program's text, deflated, into a region reserved inside every `bund2` binary;
the copy runs the program it finds in its own memory, and hands it the whole
command line. No Rust toolchain, C compiler or linker is needed on the machine
doing the building. On macOS the copy is re-signed with the system's
`codesign`; on Linux nothing more is needed. `bund2 build --inspect <binary>`
prints what a built binary carries: its program's size, the restrictions it
was built with, the `bund2` that built it and the oracle it conformed to.

A bundle carries whatever runtime built it, so the same program can be built
twice. `fibonacci/` and `jit_vs_tier0/` both do that: their Makefiles build a
second `bund2` with `--features jit` in its own target directory,
`target/jit`, and make a second bundle from it, so `make bench-jit` in the
first and `make bench` in the second time the Tier 0 interpreter and the
Cranelift JIT on one and the same program.

Every `Makefile` includes `toolchain.mk`, which builds with the Rust
release `rust-toolchain.toml` pins even when the shell sets its own
`RUSTUP_TOOLCHAIN`, as `mise` does. No override is needed.

## Adding one

Make a directory, put the program in it, copy a `Makefile` from a neighbour and
change the file names (keep its `include` of `toolchain.mk`), and write a `README.md` that says what the program does
and how it does it. Add the built binary's name to the directory's
`.gitignore`, as the others do. Then add a row to the table above.
