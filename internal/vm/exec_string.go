package vm

import (
	"strings"

	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterString_term(c *parser.String_termContext) {
	var functor string
	var prefunction string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	if c.GetPRE() != nil {
		prefunction = c.GetPRE().GetText()
	}
	l.VM.Opcode("str").InParser(l.VM, c.GetVALUE().GetText(), prefunction, functor)
}

func (l *bundExecListener) EnterString_term(c *parser.String_termContext) {
	var functor string
	var prefunction string
	if c.GetFUNCTOR() != nil {
		functor = c.GetFUNCTOR().GetText()
	}
	if c.GetPRE() != nil {
		prefunction = c.GetPRE().GetText()
	}
	tstring := c.GetVALUE().GetText()
	if strings.HasPrefix(tstring, "\"\"\"") {
		tstring = strings.TrimPrefix(tstring, "\"\"\"")
	} else if strings.HasPrefix(tstring, "\"") {
		tstring = strings.TrimPrefix(tstring, "\"")
	} else if strings.HasPrefix(tstring, "'''") {
		tstring = strings.TrimPrefix(tstring, "'''")
	} else if strings.HasPrefix(tstring, "'") {
		tstring = strings.TrimPrefix(tstring, "'")
	}
	if strings.HasSuffix(tstring, "\"\"\"") {
		tstring = strings.TrimSuffix(tstring, "\"\"\"")
	} else if strings.HasSuffix(tstring, "\"") {
		tstring = strings.TrimSuffix(tstring, "\"")
	} else if strings.HasSuffix(tstring, "'''") {
		tstring = strings.TrimSuffix(tstring, "'''")
	} else if strings.HasSuffix(tstring, "'") {
		tstring = strings.TrimSuffix(tstring, "'")
	}
	l.VM.RunOp("str", tstring, prefunction, functor)
}
