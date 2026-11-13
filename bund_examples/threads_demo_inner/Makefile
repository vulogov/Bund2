# Several Bund VMs, each on its own thread, exchanging work and answers
# over the local bus, started and joined from Bund with `vm.spawn` and
# `vm.join` (D249). No host code: compare `bund_examples/threads_demo`.
#
#   make         run demo.bund
#
# Which worker takes which job, and the order the pool's answers arrive
# in, change from run to run, so there is no `make check`.

ROOT    := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO   ?= cargo
BUND2   := $(ROOT)/target/release/bund2

.PHONY: all run FORCE

all: run

# Cargo decides whether anything needs rebuilding.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli

run: $(BUND2)
	$(BUND2) demo.bund < /dev/null
