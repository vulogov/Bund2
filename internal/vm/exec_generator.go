package vm

import (
	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterGenerator_term(c *parser.Generator_termContext) {
	l.VM.Opcode("GENERATOR").InParser(l.VM, c.GetName().GetText(), "", "")
}

func (l *bundListener) ExitGenerator_term(c *parser.Generator_termContext) {
	l.VM.Opcode("exitGENERATOR").InParser(l.VM, c.GetName().GetText(), "", "")
}

func (l *bundExecListener) EnterGenerator_term(c *parser.Generator_termContext) {
	l.VM.LambdaRunOp("GENERATOR", c.GetName().GetText(), "", "")
}

func (l *bundExecListener) ExitGenerator_term(c *parser.Generator_termContext) {
	l.VM.LambdaRunOp("exitGENERATOR", c.GetName().GetText(), "", "")
}
