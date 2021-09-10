package vm

import (
	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterIndex_term(c *parser.Index_termContext) {
	var v string
	var ev string
	if c.GetVALUE() != nil {
		v = c.GetVALUE().GetText()
	}
	if c.GetENDVALUE() != nil {
		ev = c.GetENDVALUE().GetText()
	}
	l.VM.Opcode("index").InParser(l.VM, v, ev)
}

func (l *bundExecListener) EnterIndex_term(c *parser.Index_termContext) {
	var v string
	var ev string
	if l.VM.CheckIgnore() {
		return
	}
	if c.GetVALUE() != nil {
		v = c.GetVALUE().GetText()
	}
	if c.GetENDVALUE() != nil {
		ev = c.GetENDVALUE().GetText()
	}
	res, err := l.VM.Opcode("index").InEval(l.VM, v, ev)
	l.VM.OnError(err, "Error in #<index>")
	if res != nil {
		l.VM.Put(res)
	}
}
