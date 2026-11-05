# A SMYSL library formed, researched, queried and reasoned over (RFC-0014,
# criterion 17), and the output it must print. Each run starts from no
# library: `mill` is removed first.
#
#   make         run demo.bund
#   make check   run it and compare with expected.txt
#   make clean

ROOT    := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO   ?= cargo
BUND2   := $(ROOT)/target/smysl/release/bund2

.PHONY: all run check clean FORCE

all: run

# The `smysl.` words exist only under their feature, so this `bund2` is
# built in its own target directory, beside the default build's.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli --features smysl \
		--target-dir target/smysl

run: $(BUND2)
	rm -rf mill
	$(BUND2) demo.bund < /dev/null

check: $(BUND2)
	rm -rf mill
	$(BUND2) demo.bund < /dev/null > run.txt
	cmp run.txt expected.txt && echo "the run printed expected.txt"

clean:
	rm -rf mill run.txt
