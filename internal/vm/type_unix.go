package vm

import (
	"fmt"

	"github.com/cstockton/go-conv"
)

func UnixFactory(vm *VM) *Elem {
	return &Elem{Type: "unix", Value: nil}
}

func UnixToString(vm *VM, e *Elem) string {
	if e.Type == "unix" {
		return fmt.Sprintf("%v", e.Value.(string))
	}
	vm.Error("trying to convert a unix and it is not a unix: %v %T", e.Type, e.Value)
	return "<?>"
}

func UnixFromString(vm *VM, d string) *Elem {
	res, err := conv.String(d)
	if err != nil {
		return nil
	}
	return &Elem{Type: "unix", Value: res}
}

func UnixCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	if e1.Type == "unix" && e2.Type == "unix" {
		r1 := e1.Value.(string)
		r2 := e2.Value.(string)
		if r1 == r2 {
			return Eq
		} else if r1 > r2 {
			return Gt
		} else {
			return Ls
		}
	}
	return IDK
}

func UnixDup(vm *VM, e *Elem) *Elem {
	if e.Type != "unix" {
		return nil
	}
	return &Elem{Type: "unix", Value: StringDeepCopy(e.Value.(string))}
}

func RegisterUnix(vm *VM) {
	vm.RegisterType("unix", UnixFactory, UnixToString, UnixFromString, UnixCompare, UnixDup)
}
