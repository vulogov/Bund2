# Examples

Programs written in Bund, each in its own directory with a `Makefile` that
turns it into a standalone binary with `bund2 build` and a `README.md` that
explains it. They are not part of the conformance corpus; that lives under
`tests/`. These are here to be read, run and changed.

| Directory | What it is |
|---|---|
| [`fibonacci/`](fibonacci/) | The naive recursive Fibonacci, with the same algorithm in Python beside it and a `make bench` that times the two. The example used to measure the interpreter. |
| [`bund_repl/`](bund_repl/) | A read–eval–print loop written in Bund: `input*` reads the lines, `bund.eval` runs them in the same VM, `?try` keeps a failing line from ending the session, and a dictionary of lambdas is the command table. |

## Running an example

Any of them runs straight from its source, with `bund2` built:

    bund2 bund_examples/fibonacci/fibonacci.bund --n 20
    bund2 bund_examples/bund_repl/repl.bund

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

If `cargo` is managed by `mise`, pass the override through:

    make CARGO="env -u RUSTUP_TOOLCHAIN cargo"

## Adding one

Make a directory, put the program in it, copy a `Makefile` from a neighbour and
change the file names, and write a `README.md` that says what the program does
and how it does it. Add the built binary's name to the directory's
`.gitignore`, as the others do. Then add a row to the table above.
