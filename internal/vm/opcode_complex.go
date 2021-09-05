package vm

import (
	"fmt"
)

func ComplexParser(vm *VM, args ...interface{}) {
	vm.Info("COMPLEX %v", args)
}

func ComplexEval(vm *VM, args ...interface{}) (*Elem, error) {
	var val *Elem
	vm.Debug("COMPLEX: %v", args)
	eh, err := vm.GetType("cpx")
	if err != nil {
		return nil, fmt.Errorf("BUND type 'cpx' not defined: %v", err)
	}
	if len(args) > 0 {
		val = eh.FromString(vm, args[0].(string))
	} else {
		return nil, fmt.Errorf("Do not know how to convert 'cpx'")
	}
	return val, nil
}

func ComplexLambda(vm *VM, args ...interface{}) (*Elem, error) {
	val, err := ComplexEval(vm, args...)
	if val != nil {
		return val, nil
	}
	return nil, err
}

func ComplexGen(vm *VM, args ...interface{}) error {
	vm.Fatal("Integer Gen not implemented")
	return nil
}

func ComplexExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("Complex128 Exec not implemented")
	return nil, nil
}

func InitOpcodeComplex(vm *VM) {
	vm.RegisterOpcode("cpx", ComplexParser, ComplexLambda, ComplexEval, ComplexGen, ComplexExec)
}
