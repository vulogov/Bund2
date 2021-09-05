package vm

import (
	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterComplex_term(c *parser.Complex_termContext) {
	var functor string
	var prefunction string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	if c.GetPRE() != nil {
		prefunction = c.GetPRE().GetText()
	}
	l.VM.Opcode("cpx").InParser(l.VM, c.GetVALUE().GetText(), prefunction, functor)
}

func (l *bundExecListener) EnterComplex_term(c *parser.Complex_termContext) {
	var functor string
	var prefunction string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	if c.GetPRE() != nil {
		prefunction = c.GetPRE().GetText()
	}
	l.VM.RunOp("cpx", c.GetVALUE().GetText(), prefunction, functor)
}
