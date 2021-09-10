package vm

import (
	"fmt"
)

func evalElement(vm *VM, e *Elem) (*Elem, error) {
	if e.Type == "bund" {
		res, err := vm.Opcode("bund").InEval(vm, e.Value.(string), e.Prefun, e.Functor)
		vm.OnError(err, "Error in executing on BUND statement")
		return res, nil
	}
	if e.Type == "str" {
		vm.ParserExec(e.Value.(string))
		return nil, nil
	}
	return nil, fmt.Errorf("Incorrect datatype for eval: %v", e.Type)
}

func EvalElement(vm *VM, e *Elem) (*Elem, error) {
	return evalElement(vm, e)
}

func EvalCurrentElement(vm *VM, e *Elem) (*Elem, error) {
	c := fmt.Sprintf("[ \"%v\" : %v ;;", vm.CurrentNS.Name, e.Value.(string))
	return evalElement(vm, &Elem{Type: "str", Value: c})
}

func EvalInElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e1.Type != "str" {
		return nil, fmt.Errorf("Incorrect datatype for eval: %v", e.Type)
	}
	c := fmt.Sprintf("[ \"%v\" : %v ;;", e1.Value.(string), e.Value.(string))
	return evalElement(vm, &Elem{Type: "str", Value: c})
}

func InitEvalFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitEvalFunctions() reached")
	vm.AddFunction("eval/Out", EvalElement)
	vm.AddOperator("eval/In", EvalInElement)
	vm.AddFunction("eval/Current", EvalCurrentElement)
	vm.AddFunction("eval", EvalCurrentElement)
}
