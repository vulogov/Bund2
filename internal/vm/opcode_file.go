package vm

import (
	"fmt"
)

func FileParser(vm *VM, args ...interface{}) {
	vm.Info("FILE %v", args)
}

func FileEval(vm *VM, args ...interface{}) (*Elem, error) {
	var val *Elem
	vm.Debug("File: %v", args)
	eh, err := vm.GetType("file")
	if err != nil {
		return nil, fmt.Errorf("BUND type 'file' not defined: %v", err)
	}
	if len(args) > 0 {
		val = eh.FromString(vm, args[0].(string))
	} else {
		return nil, fmt.Errorf("Do not know how to convert 'file'")
	}
	return val, nil
}

func FileLambda(vm *VM, args ...interface{}) (*Elem, error) {
	val, err := FileEval(vm, args...)
	if val != nil {
		return val, nil
	}
	return nil, err
}

func FileGen(vm *VM, args ...interface{}) error {
	vm.Fatal("File Gen not implemented")
	return nil
}

func FileExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("File Exec not implemented")
	return nil, nil
}

func InitOpcodeFile(vm *VM) {
	vm.RegisterOpcode("file", FileParser, FileLambda, FileEval, FileGen, FileExec)
}
