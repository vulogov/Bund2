package vm

import (
	"fmt"
)

func runcmp(vm *VM, e1 *Elem, e2 *Elem, cmpf func(*VM, int) bool) (*Elem, error) {
	var cmpv *Elem
	cmpv = e2
	if e1.Type != e2.Type {
		if e1.Type == "glob" {
			eh, err := vm.GetType(e2.Type)
			if err != nil {
				vm.OnError(err, "Error in logical operator for datatype: %v", e2.Type)
			}
			cmpv = &Elem{Type: "str", Value: eh.ToString(vm, e2)}
		} else {
			return nil, fmt.Errorf("Datatypes for the glob operations are not valid: %v %v", e1.Type, e2.Type)
		}
	}
	eh, err := vm.GetType(e1.Type)
	if err != nil {
		return nil, err
	}
	res := eh.Compare(vm, e1, cmpv)
	return &Elem{Type: "bool", Value: cmpf(vm, res)}, nil
}

func runcmporeq(vm *VM, e1 *Elem, e2 *Elem, cmpf func(*VM, int) bool) (*Elem, error) {
	var cmpv *Elem
	cmpv = e2
	if e1.Type != e2.Type {
		if e1.Type == "glob" {
			eh, err := vm.GetType(e2.Type)
			if err != nil {
				vm.OnError(err, "Error in logical operator for datatype: %v", e2.Type)
			}
			cmpv = &Elem{Type: "str", Value: eh.ToString(vm, e2)}
		} else {
			return nil, fmt.Errorf("Datatypes for the glob operations are not valid: %v %v", e1.Type, e2.Type)
		}
	}
	eh, err := vm.GetType(e1.Type)
	if err != nil {
		return nil, err
	}
	res := eh.Compare(vm, e1, cmpv)
	return &Elem{Type: "bool", Value: (cmpf(vm, res) || iseq(vm, res))}, nil
}

func iseq(vm *VM, res int) bool {
	if res == Eq {
		return true
	} else {
		return false
	}
}

func isneq(vm *VM, res int) bool {
	vm.Debug("CMP result is %v", res)
	if res != Eq {
		return true
	} else {
		return false
	}
}

func isls(vm *VM, res int) bool {
	vm.Debug("CMP result is %v", res)
	if res == Ls {
		return true
	} else {
		return false
	}
}

func isgt(vm *VM, res int) bool {
	vm.Debug("CMP result is %v", res)
	if res == Gt {
		return true
	} else {
		return false
	}
}

func EqOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return runcmp(vm, e1, e2, iseq)
}

func NEqOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return runcmp(vm, e1, e2, isneq)
}

func LSqOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return runcmp(vm, e1, e2, isls)
}

func LSEQqOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return runcmporeq(vm, e1, e2, isls)
}

func GTqOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return runcmp(vm, e1, e2, isgt)
}

func GTEQqOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return runcmporeq(vm, e1, e2, isgt)
}

func NOTElement(vm *VM, e *Elem) (*Elem, error) {
	if e.Type != "bool" {
		vm.Debug("Element on the stack is not 'bool' for NOT: %v", e.Type)
		return e, nil
	}
	return &Elem{Type: "bool", Value: (!e.Value.(bool))}, nil
}

func AndFunction(vm *VM, e *Elem) (*Elem, error) {
	if e.Type != "bool" {
		vm.Debug("Element on the stack is not 'bool' for AND: %v", e.Type)
		return e, nil
	}
	if !vm.CanGet() {
		return nil, fmt.Errorf("Stack is too shallow for AND")
	}
	e1 := vm.Take()
	if e1.Type != "bool" {
		vm.Put(e1)
		vm.Debug("Element on the stack is not 'bool' for AND: %v", e1.Type)
		return e, nil
	}
	return &Elem{Type: "bool", Value: (e.Value.(bool) && e1.Value.(bool))}, nil
}

func OrFunction(vm *VM, e *Elem) (*Elem, error) {
	if e.Type != "bool" {
		vm.Debug("Element on the stack is not 'bool' for OR: %v", e.Type)
		return e, nil
	}
	if !vm.CanGet() {
		return nil, fmt.Errorf("Stack is too shallow for OR")
	}
	e1 := vm.Take()
	if e1.Type != "bool" {
		vm.Put(e1)
		vm.Debug("Element on the stack is not 'bool' for OR: %v", e1.Type)
		return e, nil
	}
	return &Elem{Type: "bool", Value: (e.Value.(bool) || e1.Value.(bool))}, nil
}

func InitOpCmp(vm *VM) {
	vm.Debug("[ BUND ] bund.InitOpCmp() reached")
	vm.AddOperator("=", EqOperator)
	vm.AddOperator("<>", NEqOperator)
	vm.AddOperator("<", GTqOperator)
	vm.AddOperator("<=", GTEQqOperator)
	vm.AddOperator(">", LSqOperator)
	vm.AddOperator(">=", LSEQqOperator)
	vm.AddFunction("NOT", NOTElement)
	vm.AddFunction("¬", NOTElement)
	vm.AddFunction("AND", AndFunction)
	vm.AddFunction("∧", AndFunction)
	vm.AddFunction("OR", OrFunction)
	vm.AddFunction("∨", OrFunction)
}
