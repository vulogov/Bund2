package vm

import (
	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterFloat_term(c *parser.Float_termContext) {
	var functor string
	var prefunction string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	if c.GetPRE() != nil {
		prefunction = c.GetPRE().GetText()
	}
	l.VM.Opcode("flt").InParser(l.VM, c.GetVALUE().GetText(), prefunction, functor)
}

func (l *bundExecListener) EnterFloat_term(c *parser.Float_termContext) {
	var functor string
	var prefunction string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	if c.GetPRE() != nil {
		prefunction = c.GetPRE().GetText()
	}
	l.VM.RunOp("flt", c.GetVALUE().GetText(), prefunction, functor)
}
