package vm

import (
	"fmt"
)

func NSParser(vm *VM, args ...interface{}) {
	vm.Info("NAMESPACE %v", args)
}

func NSEval(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("NAMESPACE: %v", args)
	if len(args) > 0 {
		vm.GetNS(args[0].(string))
		return nil, nil
	}
	return nil, fmt.Errorf("Do not have enough data about NS()")
}

func NSLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("NAMESPACE(in lambda) %v", args)
	return &Elem{Type: "NS", Value: args[0].(string)}, nil
}

func NSExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("NS Import not implemented")
	return nil, nil
}

func NSGen(vm *VM, args ...interface{}) error {
	vm.Fatal("NS Export not implemented")
	return nil
}

func InitOpcodeNs(vm *VM) {
	vm.RegisterOpcode("NS", NSParser, NSLambda, NSEval, NSGen, NSExec)
}
