package vm

import (
	"strings"

	"github.com/vulogov/Bund2/internal/parser"
)

func (l *bundListener) EnterNs(c *parser.NsContext) {
	tstring := c.GetName().GetText()
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
	l.VM.Opcode("NS").InParser(l.VM, tstring)
}

func (l *bundListener) ExitNs(c *parser.NsContext) {
	tstring := c.GetName().GetText()
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
	l.VM.Opcode("exitNS").InParser(l.VM, tstring)
}

func (l *bundExecListener) EnterNs(c *parser.NsContext) {
	tstring := c.GetName().GetText()
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
	l.VM.RunOp("NS", tstring, "", "")
}

func (l *bundExecListener) ExitNs(c *parser.NsContext) {
	tstring := c.GetName().GetText()
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
	l.VM.RunOp("exitNS", tstring, "", "")
}
