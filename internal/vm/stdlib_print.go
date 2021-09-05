package vm

import (
	"fmt"
)

func PrintElement(vm *VM, e *Elem) (*Elem, error) {
	eh, err := vm.GetType(e.Type)
	if err != nil {
		vm.OnError(err, "Error in println functor")
		return nil, err
	}
	fmt.Printf("%s", eh.ToString(vm, e))
	return e, nil
}

func PrintlnElement(vm *VM, e *Elem) (*Elem, error) {
	eh, err := vm.GetType(e.Type)
	if err != nil {
		vm.OnError(err, "Error in println functor")
		return nil, err
	}
	fmt.Printf("%s\n", eh.ToString(vm, e))
	return e, nil
}

func InitPrintFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitPrintFunctions() reached")
	vm.AddFunction("print", PrintElement)
	vm.AddFunction("println", PrintlnElement)
}
