package vm

import (
	"fmt"

	"math/big"
)

func FloatFactory(vm *VM) *Elem {
	return &Elem{Type: "flt", Value: big.NewFloat(0.0)}
}

func FloatToString(vm *VM, e *Elem) string {
	if e.Type == "flt" {
		i := e.Value.(*big.Float)
		return fmt.Sprintf("%v", i.String())
	}
	vm.Error("trying to convert an Float and it is not a Float: %v %T", e.Type, e.Value)
	return "<?>"
}

func FloatFromString(vm *VM, d string) *Elem {
	i := big.NewFloat(0.0)
	i.SetString(d)
	res := Elem{Type: "flt", Value: i}
	return &res
}

func FloatCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	if e1.Type == "flt" && e2.Type == "flt" {
		r1 := e1.Value.(*big.Float)
		r2 := e2.Value.(*big.Float)
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

func FloatDup(vm *VM, e *Elem) *Elem {
	if e.Type != "flt" {
		return nil
	}
	return FloatFromString(vm, FloatToString(vm, e))
}

func floatMathOp(vm *VM, e1 *Elem, e2 *Elem, op int) (*Elem, error) {
	if e1.Type != "flt" {
		return nil, fmt.Errorf("Invalid operand type for flt arithmetic operation")
	}
	res := big.NewFloat(0.0)
	s := big.NewFloat(0.0)
	switch e2.Type {
	case "int":
		s = s.SetInt(e2.Value.(*big.Int))
	case "flt":
		s = e2.Value.(*big.Float)
	default:
		return nil, fmt.Errorf("Unknown operand type for flt arithmetic operation ")
	}
	if e2.Type == "int" || e2.Type == "flt" {
		switch op {
		case Add:
			res.Add(e1.Value.(*big.Float), s)
		case Sub:
			res.Sub(e1.Value.(*big.Float), s)
		case Mul:
			res.Mul(e1.Value.(*big.Float), s)
		case Div:
			res.Quo(e1.Value.(*big.Float), s)
		default:
			return nil, fmt.Errorf("Unknown operation type for int arithmetic operation ")
		}
		return &Elem{Type: "flt", Value: res}, nil
	} else if e2.Type == "dblock" {
		return DblocksMathOp(vm, e1, e2, op)
	}
	return nil, fmt.Errorf("Unknown operation type for int arithmetic operation ")
}

func FltAdd(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return floatMathOp(v, e1, e2, Add)
}

func FltSub(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return floatMathOp(v, e1, e2, Sub)
}

func FltMul(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return floatMathOp(v, e1, e2, Mul)
}

func FltDiv(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return floatMathOp(v, e1, e2, Div)
}

func FloatApply(v *VM, e1 *Elem, f func(interface{}) interface{}) (*Elem, error) {
	res := f(e1.Value.(*big.Float))
	switch res.(type) {
	case *big.Float:
		return &Elem{Type: "flt", Value: res}, nil
	}
	return nil, fmt.Errorf("Return value for 'apply' function is not recognized")
}

func RegisterFloat(vm *VM) {
	flt_type := vm.GenType("flt", FloatFactory, FloatToString, FloatFromString, FloatCompare, FloatDup)
	flt_type.Add = FltAdd
	flt_type.Sub = FltSub
	flt_type.Mul = FltMul
	flt_type.Div = FltDiv
	flt_type.Apply = FloatApply
	vm.SetType("flt", flt_type)
}
