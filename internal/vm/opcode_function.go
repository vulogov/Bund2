package vm

import (
	"fmt"
)

func FunctionParser(vm *VM, args ...interface{}) {
	vm.Info("FUNCTION %v", args)
}

func FunctionEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("Do not have enough data about FUNCTION()")
	}
	funcname := args[0].(string)
	vm.Debug("FUNCTION(start): %v", funcname)
	if !vm.IsStack() {
		return nil, fmt.Errorf("Attempt of defining Lambda function with empty context")
	}
	vm.CurrentNS.GetFunction(funcname)
	vm.CurrentNS.InFunction(funcname)
	return nil, nil
}

func FunctionLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("FUNCTION(in lambda) %v", args)
	return &Elem{Type: "FUNCTION", Value: args[0].(string)}, nil
}

func FunctionGen(vm *VM, args ...interface{}) error {
	vm.Fatal("FUNCTION Gen not implemented")
	return nil
}

func FunctionExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("FUNCTION Exec not implemented")
	return nil, nil
}

func ExitFunctionParser(vm *VM, args ...interface{}) {
	vm.Info("exit FUNCTION %v", args)
}

func ExitFunctionEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("Do not have enough data about FUNCTION()")
	}
	if !vm.IsStack() {
		return nil, fmt.Errorf("Attempt to close Lambda function with empty context")
	}
	funcname := args[0].(string)
	ls := vm.CurrentFunction()
	vm.CurrentNS.CloseFunction()
	vm.Debug("FUNCTION(fin): %v, size: %v", funcname, ls.Len())
	return nil, nil
}

func ExitFunctionLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit FUNCTION(in lambda) %v", args)
	return &Elem{Type: "exitFUNCTION", Value: nil}, nil
}

func ExitFunctionGen(vm *VM, args ...interface{}) error {
	vm.Fatal("exit FUNCTION Gen not implemented")
	return nil
}

func ExitFunctionExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("exit FUNCTION Exec not implemented")
	return nil, nil
}

func InitOpcodeFunction(vm *VM) {
	vm.RegisterOpcode("FUNCTION", FunctionParser, FunctionLambda, FunctionEval, FunctionGen, FunctionExec)
	vm.RegisterOpcode("exitFUNCTION", ExitFunctionParser, ExitFunctionLambda, ExitFunctionEval, ExitFunctionGen, ExitFunctionExec)
}
