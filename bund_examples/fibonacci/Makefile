# Build a standalone `fibonacci` binary from fibonacci.bund and time it
# against the same algorithm in Python.
#
#   make            build ./fibonacci (Tier 0 interpreter)
#   make bench      time Bund2 and Python at N (default 30)
#   make bench N=25
#   make bench-jit  also build and time a bundle on the Cranelift JIT runtime
#   make clean
#
# `bund2 build` copies the bund2 binary that runs it and writes the program,
# deflated, into a region reserved inside that copy, so the bundle carries
# whatever runtime built it. The JIT bundle is built by a
# bund2 compiled with `--features jit`, in its own target directory so the
# default build is left alone.

ROOT       := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO      ?= cargo
PYTHON     ?= python3
N          ?= 30

BUND2      := $(ROOT)/target/release/bund2
JIT_TARGET := $(ROOT)/target/jit
BUND2_JIT  := $(JIT_TARGET)/release/bund2

TIME       := /usr/bin/time -p

.PHONY: all bench bench-jit clean FORCE

all: fibonacci

# Cargo decides whether anything needs rebuilding.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli

$(BUND2_JIT): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli --features jit --target-dir $(JIT_TARGET)

fibonacci: fibonacci.bund $(BUND2)
	$(BUND2) build --file fibonacci.bund --output $@

fibonacci-jit: fibonacci.bund $(BUND2_JIT)
	$(BUND2_JIT) build --file fibonacci.bund --output $@

bench: fibonacci
	@echo "== Bund2 bundle, Tier 0, fib($(N))"
	@$(TIME) ./fibonacci --n $(N)
	@echo "== Python $$($(PYTHON) -c 'import platform; print(platform.python_version())'), fib($(N))"
	@$(TIME) $(PYTHON) fibonacci.py --n $(N)

bench-jit: bench fibonacci-jit
	@echo "== Bund2 bundle, Cranelift JIT, fib($(N))"
	@$(TIME) ./fibonacci-jit --n $(N)

clean:
	rm -f fibonacci fibonacci-jit
