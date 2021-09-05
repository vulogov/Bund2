package bund

import (
	"fmt"

	"github.com/pieterclaerhout/go-log"

	"github.com/vulogov/Bund2/internal/banner"
	"github.com/vulogov/Bund2/internal/conf"
)

func Version() {
	Init()
	log.Debug("[ BUND ] bund.Version() is reached")
	banner.Banner(fmt.Sprintf("[ BUND %v ]", conf.EVersion))
	banner.Table()
}
