package vm

import (
	"fmt"
)

func LambdaParser(vm *VM, args ...interface{}) {
	vm.Info("LAMBDA %v", args)
}

func LambdaEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("Do not have enough data about FUNCTION()")
	}
	funcname := args[0].(string)
	vm.Debug("LAMBDA(start): %v", funcname)
	if !vm.IsStack() {
		return nil, fmt.Errorf("Attempt of defining Lambda function with empty context")
	}
	vm.CurrentNS.GetLambda(funcname)
	vm.CurrentNS.InLambda(funcname)
	return nil, nil
}

func LambdaLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("FUNCTION(in lambda) %v", args)
	return &Elem{Type: "LAMBDA", Value: args[0].(string)}, nil
}

func LambdaGen(vm *VM, args ...interface{}) error {
	vm.Fatal("LAMBDA Gen not implemented")
	return nil
}

func LambdaExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("LAMBDA Exec not implemented")
	return nil, nil
}

func ExitLambdaParser(vm *VM, args ...interface{}) {
	vm.Info("exit LAMBDA %v", args)
}

func ExitLambdaEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("Do not have enough data about LAMBDA()")
	}
	if !vm.IsStack() {
		return nil, fmt.Errorf("Attempt to close Lambda function with empty context")
	}
	funcname := vm.CurrentNS.NameOfCurrentLambda()
	ls := vm.CurrentNS.CurrentGenerator()
	vm.CurrentNS.CloseGenerator()
	vm.Debug("LAMBDA(fin): %v, size: %v", funcname, ls.Len())
	vm.Put(&Elem{Type: "CALL", Value: funcname})
	return nil, nil
}

func ExitLambdaLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit LAMBDA(in lambda) %v", args)
	return &Elem{Type: "exitLAMBDA", Value: nil}, nil
}

func ExitLambdaGen(vm *VM, args ...interface{}) error {
	vm.Fatal("exit LAMBDA Gen not implemented")
	return nil
}

func ExitLambdaExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("exit LAMBDA Exec not implemented")
	return nil, nil
}

func InitOpcodeLambda(vm *VM) {
	vm.RegisterOpcode("LAMBDA", LambdaParser, LambdaLambda, LambdaEval, LambdaGen, LambdaExec)
	vm.RegisterOpcode("exitLAMBDA", ExitLambdaParser, ExitLambdaLambda, ExitLambdaEval, ExitLambdaGen, ExitLambdaExec)
}
