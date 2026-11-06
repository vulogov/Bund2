# SMYSL and logic programming together (RFC-0013, RFC-0014): a library
# formed, researched, queried, reasoned over, and the proofs written back.
# Each run starts from no library: `ashby` is removed first.
#
#   make         run demo.bund
#   make check   run it and compare with expected.txt and expected.stderr.txt
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
	rm -rf ashby
	$(BUND2) demo.bund < /dev/null

check: $(BUND2)
	rm -rf ashby
	$(BUND2) demo.bund < /dev/null > run.txt 2> run.stderr.txt
	cmp run.txt expected.txt && cmp run.stderr.txt expected.stderr.txt \
		&& echo "the run printed expected.txt and expected.stderr.txt"

clean:
	rm -rf ashby run.txt run.stderr.txt
