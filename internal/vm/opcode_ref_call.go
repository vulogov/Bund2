package vm

import (
	"fmt"
)

func RCallParser(vm *VM, args ...interface{}) {
	vm.Info("RCALL %v", args)
}

func RCallEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("I do not know what to RCALL")
	}
	vm.Debug("RCALL: %v", args)
	return &Elem{Type: "CALL", Value: args[0].(string), Prefun: args[1].(string), Functor: args[2].(string)}, nil
}

func RCallLambda(vm *VM, args ...interface{}) (*Elem, error) {
	return CallEval(vm, args...)
}

func RCallGen(vm *VM, args ...interface{}) error {
	vm.Fatal("RCall Gen not implemented")
	return nil
}

func RCallExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("RCall Exec not implemented")
	return nil, nil
}

func InitOpcodeRCall(vm *VM) {
	vm.RegisterOpcode("RCALL", RCallParser, RCallLambda, RCallEval, RCallGen, RCallExec)
}
