package vm

import (
	"github.com/google/uuid"

	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterMatchblock_term(c *parser.Matchblock_termContext) {
	l.VM.Opcode("MBLOCK").InParser(l.VM)
}

func (l *bundListener) ExitMatchblock_term(c *parser.Matchblock_termContext) {
	l.VM.Opcode("exitMBLOCK").InParser(l.VM)
}

func (l *bundExecListener) EnterMatchblock_term(c *parser.Matchblock_termContext) {
	blockname := uuid.New().String()
	l.VM.RunOp("MBLOCK", blockname, "", "")
}

func (l *bundExecListener) ExitMatchblock_term(c *parser.Matchblock_termContext) {
	l.VM.RunOp("exitMBLOCK", "", "", "")
}
