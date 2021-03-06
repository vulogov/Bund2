package bund

import (
	"os"

	"gopkg.in/alecthomas/kingpin.v2"

	"github.com/vulogov/Bund2/internal/conf"
)

func Main() {
	switch kingpin.MustParse(conf.App.Parse(os.Args[1:])) {
	case conf.Version.FullCommand():
		Version()
	case conf.Shell.FullCommand():
		Shell()
	case conf.Run.FullCommand():
		Run()
	case conf.Eval.FullCommand():
		Eval()
	case conf.Compile.FullCommand():
		Compile()
	case conf.Exec.FullCommand():
		Exec()
	}
	Fin()
}
