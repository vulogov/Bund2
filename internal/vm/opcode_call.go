package vm

import (
	"fmt"
)

func CallParser(vm *VM, args ...interface{}) {
	vm.Info("CALL %v", args)
}

func CallEval(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("I do not know what to CALL")
	}
	vm.Debug("CALL(%v): %v", vm.CurrentNS.Name, args)
	c := Elem{Type: "CALL", Value: args[0].(string), Prefun: args[1].(string), Functor: args[2].(string)}
	val, err := vm.Exec(&c)
	if err != nil {
		return nil, fmt.Errorf("F(%v) returns: %v", args[0].(string), err)
	}
	return val, nil
}

func CallLambda(vm *VM, args ...interface{}) (*Elem, error) {
	if len(args) == 0 {
		return nil, fmt.Errorf("CALL: I do not know what to call")
	}
	return &Elem{Type: "CALL", Value: args[0].(string), Prefun: args[1].(string), Functor: args[2].(string)}, nil
}

func CallGen(vm *VM, args ...interface{}) error {
	vm.Fatal("Call Gen not implemented")
	return nil
}

func CallExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("Call Export not implemented")
	return nil, nil
}

func InitOpcodeCall(vm *VM) {
	vm.RegisterOpcode("CALL", CallParser, CallLambda, CallEval, CallGen, CallExec)
}
