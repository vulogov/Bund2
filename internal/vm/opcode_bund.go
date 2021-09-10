package vm

import (
	"fmt"
)

func BundParser(vm *VM, args ...interface{}) {
	vm.Info("BUND %v", args)
}

func BundEval(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("BUND: %v", args)
	if len(args) == 0 {
		return nil, fmt.Errorf("We do not know what to execute BUND statement")
	}
	vm.ParserExec(args[0].(string))
	return nil, nil
}

func BundLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("BUND(in lambda) %v", args)
	return &Elem{Type: "bund", Value: args[0].(string)}, nil
}

func BundGen(vm *VM, args ...interface{}) error {
	vm.Fatal("BUND Import not implemented")
	return nil
}

func BundExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("BUND Exec not implemented")
	return nil, nil
}

func InitOpcodeBund(vm *VM) {
	vm.RegisterOpcode("bund", BundParser, BundLambda, BundEval, BundGen, BundExec)
}
