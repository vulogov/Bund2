package vm

import (
	"fmt"
)

func OpParser(vm *VM, args ...interface{}) {
	vm.Info("OP %v", args)
}

func OpEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("Do not have enough data about OPERATION()")
	}
	funcname := args[0].(string)
	vm.Debug("OPERATOR(start): %v", funcname)
	if !vm.IsStack() {
		return nil, fmt.Errorf("Attempt of defining Lambda function with empty context")
	}
	vm.CurrentNS.GetOperator(funcname)
	vm.CurrentNS.InOperator(funcname)
	return nil, nil
}

func OpLambda(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("OP: I do not know what to call")
	}
	vm.Debug("OP(in lambda): %v", args)
	return &Elem{Type: "OP", Value: args[0].(string)}, nil
}

func OpGen(vm *VM, args ...interface{}) error {
	vm.Fatal("Operator Gen not implemented")
	return nil
}

func OpExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("Operator Exec not implemented")
	return nil, nil
}

func ExitOpParser(vm *VM, args ...interface{}) {
	vm.Info("exit FUNCTION %v", args)
}

func ExitOpEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("Do not have enough data about OPERATOR()")
	}
	if !vm.IsStack() {
		return nil, fmt.Errorf("Attempt to close Operator function with empty context")
	}
	funcname := args[0].(string)
	ls := vm.CurrentNS.CurrentOperator()
	vm.CurrentNS.CloseOperator()
	vm.Debug("OPERATOR(fin): %v, size: %v", funcname, ls.Len())
	return nil, nil
}

func ExitOpLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit OPERATOR(in lambda) %v", args)
	return &Elem{Type: "exitOP", Value: nil}, nil
}

func ExitOpGen(vm *VM, args ...interface{}) error {
	vm.Fatal("exit OPERATOR Gen not implemented")
	return nil
}

func ExitOpExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("exit OPERATOR Exec not implemented")
	return nil, nil
}

func InitOpcodeOperator(vm *VM) {
	vm.RegisterOpcode("OP", OpParser, OpLambda, OpEval, OpGen, OpExec)
	vm.RegisterOpcode("exitOP", ExitOpParser, ExitOpLambda, ExitOpEval, ExitOpGen, ExitOpExec)
}
