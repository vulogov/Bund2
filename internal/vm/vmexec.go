package vm

import (
	"fmt"
)

func (vm *VM) Exec(e *Elem) (*Elem, error) {
	// if vm.HasUserFunction(name) {
	// 	vm.CurrentNS.CurrentLambdaName = name
	// 	ls := vm.CurrentNS.GetLambda(name)
	// 	if ls == nil {
	// 		return nil, fmt.Errorf("Lambda %v not exist in %v", name, vm.Name)
	// 	}
	// 	err := vm.Apply(name)
	// 	vm.CurrentNS.CurrentLambdaName = ""
	// 	return nil, err
	// }
	if e.Type != "CALL" {
		return nil, fmt.Errorf("Incorrect parameter for vm.Exec()")
	}
	name := e.Value.(string)
	aval := vm.CurrentNS.GetAlias(name)
	if aval != nil {
		vm.Debug("EXEC(%v) returned from Aliases table", name)
		return aval.(*Elem), nil
	}
	g, _ := vm.GetGen(name)
	if g != nil {
		vm.Debug("Executing GENERATOR: %v", name)
		vm.CurrentNS.CurrentLambdaName = name
		res, err := g(vm)
		vm.CurrentNS.CurrentLambdaName = ""
		return res, err
	}
	f, _ := vm.GetFunction(name)
	if f != nil {
		if !vm.CanGet() {
			return nil, fmt.Errorf("Attempt to call a function: %v on empty stack", name)
		}
		val := vm.Take()
		vm.Debug("Executing FUNCTION: %v", name)
		vm.CurrentNS.CurrentLambdaName = name
		res, err := f(vm, val)
		vm.CurrentNS.CurrentLambdaName = ""
		return res, err
	}
	o, _ := vm.GetOperator(name)
	if o != nil {
		if !vm.CanGet() {
			return nil, fmt.Errorf("Attempt to call an operator: %v on empty stack", name)
		}
		e1 := vm.Take()
		if !vm.CanGet() {
			return nil, fmt.Errorf("Stack is too shallow for operator: %v", name)
		}
		e2 := vm.Take()
		vm.Debug("Executing OPERATOR: %v", name)
		vm.CurrentNS.CurrentLambdaName = name
		res, err := o(vm, e1, e2)
		vm.CurrentNS.CurrentLambdaName = ""
		return res, err
	}
	return nil, fmt.Errorf("User or Embedded function/operator/generator not found: %v", e.Value.(string))
}
