# The toolchain every example builds bund2 with: the one rust-toolchain.toml
# pins, read from it so there is one place that names it.
#
# rustup takes a RUSTUP_TOOLCHAIN in the environment over the file, and a
# shell that sets one for its own projects (mise does) would build this
# repository with the wrong rustc, which the workspace's rust-version then
# refuses. Exporting the pinned channel here puts the file back in charge.
# A cargo that rustup does not manage ignores the variable.

TOOLCHAIN := $(shell sed -n 's/^channel *= *"\(.*\)"/\1/p' $(ROOT)/rust-toolchain.toml)
ifneq ($(TOOLCHAIN),)
export RUSTUP_TOOLCHAIN := $(TOOLCHAIN)
endif
