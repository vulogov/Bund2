# Every feature of `debug.profile`, the word that writes the profile of the
# run so far.
#
#   make             run demo.bund profiled: four reports in profiles/
#   make graphs      draw each report's .dot as an .svg (needs Graphviz)
#   make unprofiled  the same program without --profile: one warning, once
#   make noio        under --noio: the first snapshot is an error
#   make bundle      build ./profiler_demo; a bundle has no --profile, so it warns
#   make clean
#
# `debug.profile` writes only when the run keeps a profile, and only
# `bund2 script --profile <base>` keeps one, so the demo runs from source.

ROOT    := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO   ?= cargo
BUND2   := $(ROOT)/target/release/bund2
OUT     ?= profiles

.PHONY: all run graphs unprofiled noio bundle clean FORCE

all: run

# Cargo decides whether anything needs rebuilding.
$(BUND2): FORCE
	cd $(ROOT) && $(CARGO) build --release -p bund2-cli

run: $(BUND2)
	mkdir -p $(OUT)
	$(BUND2) script --profile $(OUT)/end --file demo.bund -- --out $(OUT)
	@ls -1 $(OUT)

graphs: run
	for f in $(OUT)/*.profile.dot; do dot -Tsvg "$$f" -o "$${f%.dot}.svg"; done
	@ls -1 $(OUT)/*.svg

unprofiled: $(BUND2)
	$(BUND2) script --file demo.bund

# The run stops at the first snapshot, so make is told the failure is the
# point of the target.
noio: $(BUND2)
	-$(BUND2) script --noio --profile $(OUT)/end --file demo.bund -- --out $(OUT)

profiler_demo: demo.bund $(BUND2)
	$(BUND2) build --file demo.bund --output $@

bundle: profiler_demo
	./profiler_demo

clean:
	rm -rf $(OUT) profiler_demo
