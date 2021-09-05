package vm

import (
	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterNs(c *parser.NsContext) {
	l.VM.Opcode("NS").InParser(l.VM, c.GetName().GetText())
}

func (l *bundListener) ExitNs(c *parser.NsContext) {
	l.VM.Opcode("exitNS").InParser(l.VM, c.GetName().GetText())
}

func (l *bundExecListener) EnterNs(c *parser.NsContext) {
	l.VM.RunOp("NS", c.GetName().GetText(), "", "")
}

func (l *bundExecListener) ExitNs(c *parser.NsContext) {
	l.VM.RunOp("exitNS", c.GetName().GetText(), "", "")
}
