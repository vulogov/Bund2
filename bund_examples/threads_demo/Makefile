# Several Bund VMs, each on its own thread, exchanging work and answers
# over the local bus. The host is the `threads_demo` crate beside this
# file, a workspace member; the Bund it runs is compiled into it.
#
#   make                  run with 4 workers
#   make run WORKERS=8    run with 1 to 16 workers
#   make compare          run with 1 worker, then with 4, to see the time
#
# Which worker takes which job, and the order the pool's answers arrive
# in, change from run to run, so there is no `make check`.

ROOT    := $(abspath ../..)
include $(ROOT)/bund_examples/toolchain.mk
CARGO   ?= cargo
WORKERS ?= 4

.PHONY: all run compare

all: run

run:
	cd $(ROOT) && $(CARGO) run -q -p threads_demo -- $(WORKERS)

compare:
	cd $(ROOT) && $(CARGO) run -q -p threads_demo -- 1
	cd $(ROOT) && $(CARGO) run -q -p threads_demo -- 4
