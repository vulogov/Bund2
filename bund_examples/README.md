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
| [`logic_demo/`](logic_demo/) | RFC-0013's seven worked examples of logic programming, run unattended: facts and a rule asked three ways, bridges that run Bund inside a rule, a graph word as a generator, a predicate made into a word, an expert system whose replies are seeded, a left-recursive predicate tabled and a KB saturated, and SEND + MORE = MONEY by constraint propagation. `expected.txt` is what it prints, and a test compares the two. The example for `kb.` words. |
| [`logic_programming/`](logic_programming/) | The Zebra Puzzle solved twice, by search over a list of houses and by CLP(FD) constraints over house numbers, with each part written in its own namespace, a named stack: `@lists` a small list library, `@search` and `@constraints` the two models, built from their stacks by `kb.consult*` and joined by `kb.merge`, and `@answers` where `move` brings both answers to be compared. `expected.txt` is what it prints, and a test compares the two. The example for named stacks with `kb.` words. |
| [`smysl_demo/`](smysl_demo/) | A SMYSL library about why an old mill grinds slowly, formed from Bund one MAP at a time and from SMYSL's own syntax, researched by its grounds and salience, a ground retracted, queried with `smysl.query` and the `smysl` conditional, written back out as text, then made a knowledge base with `smysl.kb` and reasoned over with `smysl.prove`, which writes its proof into the library as a grounded `finding`. Needs a `bund2` built with `--features smysl`; each run starts from no library. `expected.txt` is what it prints, and a test compares the two. The example for SMYSL corpora. |
| [`smysl/`](smysl/) | SMYSL and logic programming together, over why a village well ran dry: a library formed from a Markdown text, from SMYSL's own syntax and from Bund MAPs; checked, written out and parsed back; researched by lineage, salience and budget; a dossier and measures; every query shape; then the library as a knowledge base, with CLP(FD) over its ranks, tabled rules, a predicate as a word, bridges that call `smysl.trace` while solving, saturation and askable goals; `smysl.prove` writing findings, a proof by negation and a cited program fact back into the library, a rule resting on those findings, and a retraction the facts then show. Needs a `bund2` built with `--features smysl`. `expected.txt` and `expected.stderr.txt` are what it prints, and a test compares them. The long tour of SMYSL with `kb.` words. |
| [`smysl_demo_full/`](smysl_demo_full/) | Logic programming and SMYSL together with a local model, over why a reservoir turned green: a report and a Markdown text read by the model and committed when clean, a question answered over its dossier; the analyst's own units, checked, researched, measured and queried; every `kb.` word, from a KB written in a named stack and edited on the workbench, through control, bridges, tabling, saturation, an expert system that asks and proof trees, to a remediation plan by CLP(FD); and `smysl.prove` writing findings over the analyst's units and the model's back into the library, then a retraction and what it shakes. Needs a `bund2` built with `--features smysl-ollama` and a local Ollama with `llama3.2`. A model's answers are not fixed, so there is no `expected.txt`. Every `smysl.` and `kb.` word in one program. |

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
    cd bund_examples/logic_demo && bund2 demo.bund
    cd bund_examples/logic_programming && bund2 zebra.bund
    cd bund_examples/smysl_demo && rm -rf mill && bund2 demo.bund   # a --features smysl build
    cd bund_examples/smysl && rm -rf ashby && bund2 demo.bund       # a --features smysl build
    cd bund_examples/smysl_demo_full && rm -rf fenwick && bund2 demo.bund   # smysl-ollama, and Ollama

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
