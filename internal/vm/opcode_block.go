package vm

import (
	"fmt"
)

func BlockParser(vm *VM, args ...interface{}) {
	vm.Info("BLOCK %v", args)
}

func BlockEval(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("BLOCK %v", args)
	if len(args) > 0 {
		vm.GetNS(args[0].(string))
		return nil, nil
	}
	return nil, fmt.Errorf("Do not have enough data about BLOCK()")
}

func BlockLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("BLOCK(in lambda) %v", args)
	return &Elem{Type: "BLOCK", Value: args[0].(string)}, nil
}

func BlockExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("BLOCK Import not implemented")
	return nil, nil
}

func BlockGen(vm *VM, args ...interface{}) error {
	vm.Fatal("BLOCK Gen not implemented")
	return nil
}

func ExitBlockParser(vm *VM, args ...interface{}) {
	vm.Info("exit BLOCK %v", args)
}

func ExitBlockEval(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit BLOCK %v", args)
	name := vm.CurrentNS.Name
	vm.EndNS()
	if len(args[2].(string)) > 0 {
		if vm.IsStack() {
			vm.Fatal("Functors for anonymous blocks not yet supported")
		}
	}
	vm.DelNS(name)
	return nil, nil
}

func ExitBlockLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit BLOCK(in lambda) %v", args)
	return &Elem{Type: "exitBLOCK", Value: nil}, nil
}

func ExitBlockGen(vm *VM, args ...interface{}) error {
	vm.Fatal("exit BLOCK Gen not implemented")
	return nil
}

func ExitBlockExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("exit BLOCK Exec not implemented")
	return nil, nil
}

func InitOpcodeBlock(vm *VM) {
	vm.RegisterOpcode("BLOCK", BlockParser, BlockLambda, BlockEval, BlockGen, BlockExec)
	vm.RegisterOpcode("exitBLOCK", ExitBlockParser, ExitBlockLambda, ExitBlockEval, ExitBlockGen, ExitBlockExec)
}
