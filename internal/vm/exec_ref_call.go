package vm

import (
	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterRef_call_term(c *parser.Ref_call_termContext) {
	var functor string
	var prefunction string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	if c.GetPRE() != nil {
		prefunction = c.GetPRE().GetText()
	}
	l.VM.Opcode("RCALL").InParser(l.VM, c.GetVALUE().GetText(), prefunction, functor)
}

func (l *bundExecListener) EnterRef_call_term(c *parser.Ref_call_termContext) {
	var functor string
	var prefunction string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	if c.GetPRE() != nil {
		prefunction = c.GetPRE().GetText()
	}
	l.VM.RunOp("RCALL", c.GetVALUE().GetText(), prefunction, functor)
}
