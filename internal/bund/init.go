package bund

import (
	"github.com/cosiner/argv"
	"github.com/pieterclaerhout/go-log"

	"github.com/vulogov/Bund2/internal/conf"
	tlog "github.com/vulogov/Bund2/internal/log"
	"github.com/vulogov/Bund2/internal/signal"
	vmmod "github.com/vulogov/Bund2/internal/vm"
)

var vm *vmmod.VM

func Init() {
	tlog.Init()
	log.Debug("[ BUND ] bund.Init() is reached")
	signal.InitSignal()
	if len(*conf.Args) > 0 {
		Argv, err := argv.Argv(*conf.Args, func(backquoted string) (string, error) {
			return backquoted, nil
		}, nil)
		if err != nil {
			log.Fatalf("Error parsing ARGS: %v", err)
		}
		log.Debugf("ARGV: %v", Argv)
		conf.Argv = Argv
	}
	log.Debug("[ BUND ] VM is initialized")
	vm = vmmod.NewVM("<eval>")
}
