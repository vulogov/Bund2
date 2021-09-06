package vm

import (
	"fmt"
)

func DBlockParser(vm *VM, args ...interface{}) {
	vm.Info("DBLOCK %v", args)
}

func DBlockEval(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("DBLOCK %v", args)
	if len(args) > 0 {
		vm.GetNS(args[0].(string))
		vm.CurrentNS.SetOption("separateinline", true)
		return nil, nil
	}
	return nil, fmt.Errorf("Do not have enough data about BLOCK()")
}

func DBlockLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("DBLOCK(in lambda) %v", args)
	return &Elem{Type: "DBLOCK", Value: args[0].(string), Functor: args[2].(string)}, nil
}

func DBlockGen(vm *VM, args ...interface{}) error {
	vm.Fatal("DBLOCK Import not implemented")
	return nil
}

func DBlockExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("DBLOCK Export not implemented")
	return nil, nil
}

func ExitDBlockParser(vm *VM, args ...interface{}) {
	vm.Info("exit DBLOCK %v", args)
}

func ExitDBlockEval(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit DBLOCK %v", args)
	if vm.Current != nil {
		vm.Debug("EXITING Data Block. Stack size: %v", vm.Current.Len())
		res := new(Elem)
		res.Name = vm.CurrentNS.Name
		res.Type = "dblock"
		res.Value = vm.Current
		res.Functor = args[2].(string)
		vm.CurrentNS.SetOption("separateinline", false)
		vm.EndNS()
		if vm.IsStack() {
			vm.Put(res)
		}
	} else {
		vm.Debug("EXITING Data Block. No current stack")
		vm.EndNS()
	}
	return nil, nil
}

func ExitDBlockLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit DBLOCK(in lambda) %v", args)
	return &Elem{Type: "exitDBLOCK", Value: nil, Functor: args[2].(string)}, nil
}

func ExitDBlockGen(vm *VM, args ...interface{}) error {
	vm.Fatal("exit DBLOCK Import not implemented")
	return nil
}

func ExitDBlockExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("exit DBLOCK Export not implemented")
	return nil, nil
}

func InitOpcodeDBlock(vm *VM) {
	vm.RegisterOpcode("DBLOCK", DBlockParser, DBlockLambda, DBlockEval, DBlockGen, DBlockExec)
	vm.RegisterOpcode("exitDBLOCK", ExitDBlockParser, ExitDBlockLambda, ExitDBlockEval, ExitDBlockGen, ExitDBlockExec)
}
