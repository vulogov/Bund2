# Time one numeric loop on the Tier 0 interpreter and on the Cranelift JIT.
#
#   make            build ./kernel (Tier 0) and ./kernel-jit (JIT runtime)
#   make bench      time both at N iterations (default 10000000)
#   make bench N=1000000
#   make stats      what the tier did: the JIT bundle with BUND2_STATS set
#   make clean
#
# `bund2 build` copies the bund2 binary that runs it and writes the program,
# deflated, into a region reserved inside that copy, so a bundle carries
# whatever runtime built it. ./kernel is made by the default bund2 and runs
# on the Tier 0 interpreter; ./kernel-jit is made by a bund2 compiled with
# `--features jit`, in its own target directory so the default build is left
# alone. The two bundles hold the same program byte for byte.

ROOT       := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO      ?= cargo
N          ?= 10000000

BUND2      := $(ROOT)/target/release/bund2
JIT_TARGET := $(ROOT)/target/jit
BUND2_JIT  := $(JIT_TARGET)/release/bund2

TIME       := /usr/bin/time -p

.PHONY: all bench stats clean FORCE

all: kernel kernel-jit

# Cargo decides whether anything needs rebuilding.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli

$(BUND2_JIT): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli --features jit --target-dir $(JIT_TARGET)

kernel: kernel.bund $(BUND2)
	$(BUND2) build --file kernel.bund --output $@

kernel-jit: kernel.bund $(BUND2_JIT)
	$(BUND2_JIT) build --file kernel.bund --output $@

bench: all
	@echo "== Tier 0 interpreter, N=$(N)"
	@$(TIME) ./kernel --n $(N)
	@echo "== Cranelift JIT, N=$(N)"
	@$(TIME) ./kernel-jit --n $(N)

stats: kernel-jit
	BUND2_STATS=1 ./kernel-jit --n $(N)

clean:
	rm -f kernel kernel-jit
