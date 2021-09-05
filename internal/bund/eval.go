package bund

import (
	"github.com/pieterclaerhout/go-log"

	"github.com/vulogov/Bund2/internal/conf"
	"github.com/vulogov/Bund2/internal/signal"
)

func Eval() {
	Init()
	log.Debug("[ BUND ] bund.Eval() is reached")
	if *conf.LexerPrint {
		vm.LexerPrint(*conf.Expr)
		return
	}
	if *conf.ParserPrint {
		vm.ParserPrint(*conf.Expr)
		return
	}
	vm.ParserExec(*conf.Expr)
	signal.ExitRequest()
}
