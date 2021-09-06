package vm

import (
	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterSeparate_term(c *parser.Separate_termContext) {
	l.VM.Opcode("SEPARATE").InParser(l.VM, "", "", "")
}

func (l *bundExecListener) EnterSeparate_term(c *parser.Separate_termContext) {
	l.VM.RunOp("SEPARATE", "", "", "")
}
