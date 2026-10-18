# Statistics over one column of a CSV file, as a standalone binary.
#
#   make                 build ./stats
#   make run             ./stats over readings.csv, column temperature
#   make run COLUMN=humidity
#   make run FILE=other.csv COLUMN=total
#   make clean
#
# `bund2 build` copies the bund2 binary that runs it and writes the program,
# deflated, into a region reserved inside that copy. The CSV file is not in
# the binary: `csv` opens it at run time, relative to where the binary runs.

ROOT   := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO  ?= cargo
BUND2  := $(ROOT)/target/release/bund2
FILE   ?= readings.csv
COLUMN ?= temperature

.PHONY: all run clean FORCE

all: stats

# Cargo decides whether anything needs rebuilding.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli

stats: stats.bund $(BUND2)
	$(BUND2) build --file stats.bund --output $@

run: stats
	./stats --file $(FILE) --column $(COLUMN)

clean:
	rm -f stats
