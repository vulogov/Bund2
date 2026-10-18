# Build a standalone `bund_repl` binary from repl.bund, a read–eval–print loop
# written in Bund itself.
#
#   make            build ./bund_repl
#   make run        build it and start a session
#   make try        build it and drive one scripted session through a pipe
#   make clean
#
# `bund2 build` copies the bund2 binary that runs it and writes the program,
# deflated, into a region reserved inside that copy, so the bundle carries
# whatever runtime built it. Nothing is compiled: no Rust toolchain, C
# compiler or linker is needed here, only `bund2`.

ROOT   := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO  ?= cargo

BUND2  := $(ROOT)/target/release/bund2

.PHONY: all run try clean FORCE

all: bund_repl

# Cargo decides whether anything needs rebuilding.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli

bund_repl: repl.bund $(BUND2)
	$(BUND2) build --file repl.bund --output $@

run: bund_repl
	./bund_repl

try: bund_repl
	printf '1 2 +\n:sq { dup * } register\n7 sq\nfoo\nstack\nquit\n' | ./bund_repl

clean:
	rm -f bund_repl
