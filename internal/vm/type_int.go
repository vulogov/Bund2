package vm

import (
	"fmt"

	"math/big"
)

func BigFactory(vm *VM) *Elem {
	return &Elem{Type: "int", Value: big.NewInt(0)}
}

func BigToString(vm *VM, e *Elem) string {
	if e.Type == "int" {
		i := e.Value.(*big.Int)
		return fmt.Sprintf("%v", i.String())
	}
	vm.Error("trying to convert an Int and it is not a Int: %v %T", e.Type, e.Value)
	return "<?>"
}

func BigFromString(vm *VM, d string) *Elem {
	i := big.NewInt(0)
	i.SetString(d, 0)
	res := Elem{Type: "int", Value: i}
	return &res
}

func BigCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	if e1.Type == "int" && e2.Type == "int" {
		r1 := e1.Value.(*big.Int)
		r2 := e2.Value.(*big.Int)
		res := r1.Cmp(r2)
		if res == 0 {
			return Eq
		} else if res == -1 {
			return Ls
		} else if res == 1 {
			return Gt
		}
	}
	return IDK
}

func BigDup(vm *VM, e *Elem) *Elem {
	if e.Type != "int" {
		return nil
	}
	return BigFromString(vm, BigToString(vm, e))
}

func bigMathOp(vm *VM, e1 *Elem, e2 *Elem, op int) (*Elem, error) {
	if e1.Type != "int" {
		return nil, fmt.Errorf("Invalid operand type for int arithmetic operation")
	}
	res := big.NewInt(0)
	s := big.NewInt(0)
	switch e2.Type {
	case "int":
		s = e2.Value.(*big.Int)
	case "flt":
		s, _ = e2.Value.(*big.Float).Int(s)
	}
	if e2.Type == "int" || e2.Type == "flt" {
		switch op {
		case Add:
			res.Add(e1.Value.(*big.Int), s)
		case Sub:
			res.Sub(e1.Value.(*big.Int), s)
		case Mul:
			res.Mul(e1.Value.(*big.Int), s)
		case Div:
			res.Div(e1.Value.(*big.Int), s)
		default:
			return nil, fmt.Errorf("Unknown operation type for int arithmetic operation ")
		}
		return &Elem{Type: "int", Value: res}, nil
	} else if e2.Type == "dblock" {
		return DblocksMathOp(vm, e1, e2, op)
	} else if e2.Type == "MAT" {
		return MatMathOperator(vm, e2, e1, op)
	}
	return nil, fmt.Errorf("Unknown operand type for int arithmetic operation ")
}

func BigAdd(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return bigMathOp(v, e1, e2, Add)
}

func BigSub(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return bigMathOp(v, e1, e2, Sub)
}

func BigMul(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return bigMathOp(v, e1, e2, Mul)
}

func BigDiv(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return bigMathOp(v, e1, e2, Div)
}

func BigApply(v *VM, e1 *Elem, f func(interface{}) interface{}) (*Elem, error) {
	res := f(e1.Value.(*big.Int))
	switch res.(type) {
	case *big.Int:
		return &Elem{Type: "int", Value: res}, nil
	}
	return nil, fmt.Errorf("Return value for 'apply' function is not recognized")
}

func RegisterInt(vm *VM) {
	int_type := vm.GenType("int", BigFactory, BigToString, BigFromString, BigCompare, BigDup)
	int_type.Add = BigAdd
	int_type.Sub = BigSub
	int_type.Mul = BigMul
	int_type.Div = BigDiv
	int_type.Apply = BigApply
	vm.SetType("int", int_type)
}
