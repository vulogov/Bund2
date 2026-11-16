# RFC-0015's five worked examples and §S18's rule base, and the output they
# must print (criterion 21).
#
#   make         run demo.bund
#   make check   run it and compare with expected.txt
#   make clean

ROOT    := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO   ?= cargo
BUND2   := $(ROOT)/target/release/bund2

.PHONY: all run check clean FORCE

all: run

# Cargo decides whether anything needs rebuilding.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli

run: $(BUND2)
	$(BUND2) demo.bund < /dev/null

check: $(BUND2)
	$(BUND2) demo.bund < /dev/null > run.txt
	cmp run.txt expected.txt && echo "the run printed expected.txt"

clean:
	rm -f run.txt
