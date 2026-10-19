# Bund support for Neovim.
#
#   make install     link this directory into Neovim's package path
#   make uninstall   remove the link
#   make words       regenerate syntax/bund_words.vim from bund2
#   make test        run the headless checks

ROOT  := $(abspath ../..)
HERE  := $(abspath .)
NVIM_BIN ?= nvim
BUND2 ?= $(ROOT)/target/release/bund2
DATA  ?= $(or $(XDG_DATA_HOME),$(HOME)/.local/share)
PACK  := $(DATA)/nvim/site/pack/bund/start

.PHONY: install uninstall words test

install:
	mkdir -p $(PACK)
	ln -sfn $(HERE) $(PACK)/bund
	@echo "linked $(PACK)/bund -> $(HERE)"

uninstall:
	rm -f $(PACK)/bund

words:
	BUND2=$(BUND2) tools/generate-words.sh

test:
	$(NVIM_BIN) --headless --clean --cmd 'set rtp^=$(HERE)' -c 'luafile test/run.lua'
