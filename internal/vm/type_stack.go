package vm

import (
	"github.com/gammazero/deque"
)

func StackFactory(vm *VM) *Elem {
	return &Elem{Type: "stack", Value: new(deque.Deque)}
}

func StackToString(vm *VM, e *Elem) string {

	vm.Error("trying to convert an Int and it is not a Int: %v %T", e.Type, e.Value)
	return "<?>"
}

func StackFromString(vm *VM, d string) *Elem {
	res := Elem{Type: "stack", Value: new(deque.Deque)}
	return &res
}

func StackCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	return IDK
}

func StackDup(vm *VM, e *Elem) *Elem {
	if e.Type != "stack" {
		return nil
	}
	return StackFactory(vm)
}

func RegisterStack(vm *VM) {
	vm.RegisterType("stack", StackFactory, StackToString, StackFromString, StackCompare, StackDup)
}
