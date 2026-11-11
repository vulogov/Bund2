# Logic programming and SMYSL together, with a local model (RFC-0013 and
# RFC-0014, Phases 1 and 2), run by hand against a local Ollama. Each run
# starts from no library: `fenwick` is removed first.
#
#   make         run demo.bund
#   make clean
#
# A model's answers are not fixed, so there is no `make check`.

ROOT    := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO   ?= cargo
BUND2   := $(ROOT)/target/smysl-ollama/release/bund2

.PHONY: all run clean FORCE

all: run

# The model words exist only under `smysl-model`, and reach Ollama only
# with its mapper, so this `bund2` is built in its own target directory,
# the one `bund_examples/smysl_model` builds.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli --features smysl-ollama \
		--target-dir target/smysl-ollama

run: $(BUND2)
	rm -rf fenwick
	$(BUND2) demo.bund < /dev/null

clean:
	rm -rf fenwick
