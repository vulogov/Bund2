# Every graph word: the reference's seven, Bund2's twelve (D192, RFC-0012),
# their workbench forms, io.graph, a graph kept in a file, and what the
# words refuse (D193, F219).
#
#   make         run demo.bund
#   make twice   run it twice and compare: every answer is in node order
#   make clean

ROOT    := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO   ?= cargo
BUND2   := $(ROOT)/target/release/bund2

.PHONY: all run twice clean FORCE

all: run

# Cargo decides whether anything needs rebuilding.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli

run: $(BUND2)
	$(BUND2) demo.bund

twice: $(BUND2)
	$(BUND2) demo.bund > run1.txt
	$(BUND2) demo.bund > run2.txt
	cmp run1.txt run2.txt && echo "the two runs printed the same"

clean:
	rm -f run1.txt run2.txt roads.graph
