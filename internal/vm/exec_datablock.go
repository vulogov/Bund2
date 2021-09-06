package vm

import (
	"github.com/google/uuid"

	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterDatablock_term(c *parser.Datablock_termContext) {
	var functor string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	l.VM.Opcode("DBLOCK").InParser(l.VM, "", "", functor)
}

func (l *bundListener) ExitDatablock_term(c *parser.Datablock_termContext) {
	var functor string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	l.VM.Opcode("exitDBLOCK").InParser(l.VM, "", "", functor)
}

func (l *bundExecListener) EnterDatablock_term(c *parser.Datablock_termContext) {
	var functor string
	blockname := uuid.New().String()
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	l.VM.RunOp("DBLOCK", blockname, "", functor)
}

func (l *bundExecListener) ExitDatablock_term(c *parser.Datablock_termContext) {
	var functor string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	l.VM.RunOp("exitDBLOCK", "", "", functor)
}
