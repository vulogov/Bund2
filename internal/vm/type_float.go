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

func FloatAdd(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	res := big.NewFloat(0.0)
	res = res.Add(e1.Value.(*big.Float), e2.Value.(*big.Float))
	return &Elem{Type: "flt", Value: res}, nil
}

func FloatSub(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	res := big.NewFloat(0.0)
	res = res.Sub(e1.Value.(*big.Float), e2.Value.(*big.Float))
	return &Elem{Type: "flt", Value: res}, nil
}

func FloatMul(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	res := big.NewFloat(0.0)
	res = res.Mul(e1.Value.(*big.Float), e2.Value.(*big.Float))
	return &Elem{Type: "flt", Value: res}, nil
}

func FloatDiv(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	res := big.NewFloat(0.0)
	res = res.Quo(e1.Value.(*big.Float), e2.Value.(*big.Float))
	return &Elem{Type: "flt", Value: res}, nil
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
	flt_type.Add = FloatAdd
	flt_type.Add = FloatSub
	flt_type.Add = FloatMul
	flt_type.Add = FloatDiv
	flt_type.Apply = FloatApply
	vm.SetType("flt", flt_type)
}
