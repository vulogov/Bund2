package vm

import (
	"fmt"
)

func FloatParser(vm *VM, args ...interface{}) {
	vm.Info("FLOAT %v", args)
}

func FloatEval(vm *VM, args ...interface{}) (*Elem, error) {
	var val *Elem
	vm.Debug("FLOAT: %v", args)
	eh, err := vm.GetType("flt")
	if err != nil {
		return nil, fmt.Errorf("BUND type 'flt' not defined: %v", err)
	}
	if len(args) > 0 {
		val = eh.FromString(vm, args[0].(string))
		val.Prefun = args[1].(string)
		val.Functor = args[2].(string)
	} else {
		return nil, fmt.Errorf("Do not know how to convert 'flt'")
	}
	return val, nil
}

func FloatLambda(vm *VM, args ...interface{}) (*Elem, error) {
	val, err := FloatEval(vm, args...)
	if val != nil {
		return val, nil
	}
	return nil, err
}

func FloatGen(vm *VM, args ...interface{}) error {
	vm.Fatal("Float Gen not implemented")
	return nil
}

func FloatExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("Float Exec not implemented")
	return nil, nil
}

func InitOpcodeFloat(vm *VM) {
	vm.RegisterOpcode("flt", FloatParser, FloatLambda, FloatEval, FloatGen, FloatExec)
}
