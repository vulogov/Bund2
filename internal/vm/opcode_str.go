package vm

import (
	"fmt"
)

func StrParser(vm *VM, args ...interface{}) {
	vm.Info("STRING %v", args)
}

func StrEval(vm *VM, args ...interface{}) (*Elem, error) {
	var val *Elem
	vm.Debug("STRING: %v", args)
	eh, err := vm.GetType("str")
	if err != nil {
		return nil, fmt.Errorf("BUND type 'str' not defined: %v", err)
	}
	if len(args) > 0 {
		val = eh.FromString(vm, args[0].(string))
		val.Prefun = args[1].(string)
		val.Functor = args[2].(string)
	} else {
		return nil, fmt.Errorf("Do not know how to convert 'str'")
	}
	return val, nil
}

func StrLambda(vm *VM, args ...interface{}) (*Elem, error) {
	val, err := StrEval(vm, args...)
	if val != nil {
		return val, nil
	}
	return nil, err
}

func StrGen(vm *VM, args ...interface{}) error {
	vm.Fatal("String Gen not implemented")
	return nil
}

func StrExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("String Export not implemented")
	return nil, nil
}

func InitOpcodeString(vm *VM) {
	vm.RegisterOpcode("str", StrParser, StrLambda, StrEval, StrGen, StrExec)
}
