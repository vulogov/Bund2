package vm

import (
	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterOperator_term(c *parser.Operator_termContext) {
	l.VM.Opcode("OP").InParser(l.VM, c.GetName().GetText())
}

func (l *bundListener) ExitOperator_term(c *parser.Operator_termContext) {
	l.VM.Opcode("exitOP").InParser(l.VM, c.GetName().GetText())
}

func (l *bundExecListener) EnterOperator_term(c *parser.Operator_termContext) {
	l.VM.LambdaRunOp("OP", c.GetName().GetText())
}

func (l *bundExecListener) ExitOperator_term(c *parser.Operator_termContext) {
	l.VM.LambdaRunOp("exitOP", c.GetName().GetText())
}
