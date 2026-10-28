# Every part of Bund's object system: classes, objects, fields, methods,
# constructors, inheritance, the base and value classes, and what Bund2
# adds (D23, D25, D96, D222, D223, D224, D225).
#
#   make         run demo.bund
#   make twice   run it twice and compare: nothing it prints varies
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
	rm -f run1.txt run2.txt
