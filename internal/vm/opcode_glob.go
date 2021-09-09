package vm

import (
	"fmt"
)

func GlobParser(vm *VM, args ...interface{}) {
	vm.Info("GLOB %v", args)
}

func GlobEval(vm *VM, args ...interface{}) (*Elem, error) {
	var val *Elem
	vm.Debug("GLOB: %v", args)
	eh, err := vm.GetType("glob")
	if err != nil {
		return nil, fmt.Errorf("BUND type 'glob' not defined: %v", err)
	}
	if len(args) > 0 {
		val = eh.FromString(vm, args[0].(string))
	} else {
		return nil, fmt.Errorf("Do not know how to convert 'glob'")
	}
	return val, nil
}

func GlobLambda(vm *VM, args ...interface{}) (*Elem, error) {
	val, err := GlobEval(vm, args...)
	if val != nil {
		return val, nil
	}
	return nil, err
}

func GlobGen(vm *VM, args ...interface{}) error {
	vm.Fatal("Glob Gen not implemented")
	return nil
}

func GlobExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("Glob Exec not implemented")
	return nil, nil
}

func InitOpcodeGlob(vm *VM) {
	vm.RegisterOpcode("glob", GlobParser, GlobLambda, GlobEval, GlobGen, GlobExec)
}
