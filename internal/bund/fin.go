package bund

import (
	"github.com/pieterclaerhout/go-log"

	"github.com/vulogov/Bund2/internal/banner"
)

func Fin() {
	banner.Banner("[ Zay Gezunt ]")
	log.Debug("[ BUND ] bund.Fin() is reached")
}
