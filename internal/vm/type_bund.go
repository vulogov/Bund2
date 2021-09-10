package vm

import (
	"fmt"

	"github.com/cstockton/go-conv"
)

func BundFactory(vm *VM) *Elem {
	return &Elem{Type: "bund", Value: "()"}
}

func BundToString(vm *VM, e *Elem) string {
	if e.Type == "bund" {
		return fmt.Sprintf("%v", e.Value.(string))
	}
	vm.Error("trying to convert a bund and it is not a bund: %v %T", e.Type, e.Value)
	return "<?>"
}

func BundFromString(vm *VM, d string) *Elem {
	res, err := conv.String(d)
	if err != nil {
		return nil
	}
	return &Elem{Type: "bund", Value: res}
}

func BundCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	if e1.Type == "bund" && e2.Type == "bund" {
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

func BundDup(vm *VM, e *Elem) *Elem {
	if e.Type != "bund" {
		return nil
	}
	return &Elem{Type: "bund", Value: StringDeepCopy(e.Value.(string))}
}

func RegisterBund(vm *VM) {
	vm.RegisterType("bund", BundFactory, BundToString, BundFromString, BundCompare, BundDup)
}
