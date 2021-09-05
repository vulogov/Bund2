package vm

import (
	"fmt"
)

func BooleanParser(vm *VM, args ...interface{}) {
	vm.Info("BOOLEAN %v", args)
}

func BooleanEval(vm *VM, args ...interface{}) (*Elem, error) {
	var val *Elem
	vm.Debug("BOOLEAN: %v", args)
	eh, err := vm.GetType("bool")
	if err != nil {
		return nil, fmt.Errorf("BUND type 'bool' not defined: %v", err)
	}
	if len(args) > 0 {
		val = eh.FromString(vm, args[0].(string))
		val.Prefun = args[1].(string)
		val.Functor = args[2].(string)
	} else {
		return nil, fmt.Errorf("Do not know how to convert 'bool'")
	}
	return val, nil
}

func BooleanLambda(vm *VM, args ...interface{}) (*Elem, error) {
	val, err := BooleanEval(vm, args...)
	if val != nil {
		return val, nil
	}
	return nil, err
}

func BooleanGen(vm *VM, args ...interface{}) error {
	vm.Fatal("Boolean Gen not implemented")
	return nil
}

func BooleanExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("Boolean Exec not implemented")
	return nil, nil
}

func InitOpcodeBoolean(vm *VM) {
	vm.RegisterOpcode("bool", BooleanParser, BooleanLambda, BooleanEval, BooleanGen, BooleanExec)
}
