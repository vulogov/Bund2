package vm

import (
	"github.com/gammazero/deque"
	"github.com/lrita/cmap"
)

const (
	Lidk = 0
	Llam = 1
	Lgen = 2
	Lfun = 3
	Lops = 4
)

type NS struct {
	Name              string
	VM                *VM
	Stack             deque.Deque
	Fun               *cmap.Cmap
	Ops               *cmap.Cmap
	Gen               *cmap.Cmap
	Options           cmap.Cmap
	LambdasStack      deque.Deque
	LSMode            deque.Deque
	Aliases           cmap.Cmap
	CurrentLambdaName string
	LCache            cmap.Cmap
}

func NewNS(v *VM, name string) *NS {
	v.Debug("Creating NAMESPACE: %v", name)
	return &NS{Name: name, VM: v, Fun: new(cmap.Cmap), Ops: new(cmap.Cmap), Gen: new(cmap.Cmap)}
}

func (ns *NS) SetOption(key string, val interface{}) {
	ns.VM.Debug("NS(%s) OPTION %v=%v ", ns.Name, key, val)
	ns.Options.Store(key, val)
}

func (ns *NS) GetOption(key string, deflt interface{}) interface{} {
	ns.VM.Debug("NS(%s) OPTION-GET %v ", ns.Name, key)
	if res, ok := ns.Options.Load(key); ok {
		return res
	}
	return deflt
}
