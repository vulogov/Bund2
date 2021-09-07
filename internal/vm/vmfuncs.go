package vm

import (
	"fmt"

	"github.com/gammazero/deque"
)

func (vm *VM) AddFunction(name string, fn FunctionFun) bool {
	if _, ok := vm.Functions.Load(name); ok {
		vm.Warning("Function %v already registered", name)
		return true
	}
	vm.Functions.Store(name, fn)
	vm.Debug("Register BUND function: %v", name)
	return true
}

func (vm *VM) AddOperator(name string, fn OperatorFun) bool {
	if _, ok := vm.Operators.Load(name); ok {
		vm.Warning("Operator %v already registered", name)
		return true
	}
	vm.Operators.Store(name, fn)
	vm.Debug("[ BUND ] Register BUND operator: %v", name)
	return true
}

func (vm *VM) AddGen(name string, fn GenFun) bool {
	if _, ok := vm.Generators.Load(name); ok {
		vm.Warning("Generator %v already registered", name)
		return true
	}
	vm.Generators.Store(name, fn)
	vm.Debug("Register BUND generator: %v", name)
	return true
}

func (vm *VM) HasUserFunction(name string) bool {
	if !vm.IsStack() {
		vm.Error("Attempt to HasUserFunction(%v) on empty context", name)
		return false
	}
	return vm.CurrentNS.HasLambda(name)
}

func (vm *VM) HasUserFunctionInNS(name string, nsname string) bool {
	if !vm.IsStack() {
		vm.Error("Attempt to HasUserFunctionInNs(%v) on empty context", name)
		return false
	}
	ns := vm.AsNS(nsname)
	if ns == nil {
		vm.Error("Ns(%v) not exists", nsname)
		return false
	}
	return ns.HasLambda(name)
}

func (vm *VM) SetUserFunctionInNS(nsname string, name string, c *deque.Deque) bool {
	if !vm.IsStack() {
		vm.Error("Attempt to SetUserFunctionInNS(%v) on empty context", name)
		return false
	}
	ns := vm.AsNS(nsname)
	if ns == nil {
		vm.Error("Ns(%v) not exists", nsname)
		return false
	}
	if _, ok := ns.Fun.Load(name); ok {
		ns.VM.Debug("LAMBDA already exists %v: %v", ns.Name, name)
		return false
	} else {
		ns.VM.Debug("Registering LAMBDA in %v: %v", ns.Name, name)
		ns.Fun.Store(name, c)
	}
	return true
}

func (vm *VM) GetFunction(name string) (FunctionFun, error) {
	if res, ok := vm.Functions.Load(name); ok {
		return res.(FunctionFun), nil
	}
	return nil, fmt.Errorf("BUND do not have function: %v", name)
}

func (vm *VM) GetOperator(name string) (OperatorFun, error) {
	if res, ok := vm.Operators.Load(name); ok {
		return res.(OperatorFun), nil
	}
	return nil, fmt.Errorf("BUND do not have operator: %v", name)
}

func (vm *VM) GetGen(name string) (GenFun, error) {
	if res, ok := vm.Generators.Load(name); ok {
		return res.(GenFun), nil
	}
	return nil, fmt.Errorf("BUND do not have system function: %v", name)
}
