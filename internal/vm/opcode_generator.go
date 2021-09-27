package vm

import (
	"fmt"
)

func GeneratorParser(vm *VM, args ...interface{}) {
	vm.Info("GENERFATOR %v", args)
}

func GeneratorEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("Do not have enough data about GENERATOR()")
	}
	funcname := args[0].(string)
	vm.Debug("GENERATOR(start): %v", funcname)
	if !vm.IsStack() {
		return nil, fmt.Errorf("Attempt of defining Lambda function with empty context")
	}
	vm.CurrentNS.GetGenerator(funcname)
	vm.CurrentNS.InGenerator(funcname)
	return nil, nil
}

func GeneratorLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("GENERATOR(in lambda) %v", args)
	return &Elem{Type: "GENERATOR", Value: args[0].(string)}, nil
}

func GeneratorGen(vm *VM, args ...interface{}) error {
	vm.Fatal("FUNCTION Gen not implemented")
	return nil
}

func GeneratorExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("GENERATOR Exec not implemented")
	return nil, nil
}

func ExitGeneratorParser(vm *VM, args ...interface{}) {
	vm.Info("exit GENERATOR %v", args)
}

func ExitGeneratorEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("Do not have enough data about GENERATOR()")
	}
	if !vm.IsStack() {
		return nil, fmt.Errorf("Attempt to close Generator function with empty context")
	}
	funcname := args[0].(string)
	ls := vm.CurrentNS.CurrentGenerator()
	vm.CurrentNS.CloseGenerator()
	vm.Debug("GENERATOR(fin): %v, size: %v", funcname, ls.Len())
	return nil, nil
}

func ExitGeneratorLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit GENERATOR(in lambda) %v", args)
	return &Elem{Type: "exitGENERATOR", Value: nil}, nil
}

func ExitGeneratorGen(vm *VM, args ...interface{}) error {
	vm.Fatal("exit GENERATOR Gen not implemented")
	return nil
}

func ExitGeneratorExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("exit GENERATOR Exec not implemented")
	return nil, nil
}

func InitOpcodeGenerator(vm *VM) {
	vm.RegisterOpcode("GENERATOR", GeneratorParser, GeneratorLambda, GeneratorEval, GeneratorGen, GeneratorExec)
	vm.RegisterOpcode("exitGENERATOR", ExitGeneratorParser, ExitGeneratorLambda, ExitGeneratorEval, ExitGeneratorGen, ExitFunctionExec)
}
