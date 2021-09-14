package vm

import (
	"fmt"

	"gonum.org/v1/gonum/mat"
)

func MatFactory(vm *VM) *Elem {
	return &Elem{Type: "MAT", Value: new(mat.Dense)}
}

func MatToString(vm *VM, e *Elem) string {
	if e.Type == "MAT" {
		m := e.Value.(*mat.Dense)
		fm := mat.Formatted(m, mat.Prefix(""), mat.Squeeze())
		return fmt.Sprintf("%v", fm)
	}
	vm.Error("trying to convert a MAT and it is not a MAT: %v %T", e.Type, e.Value)
	return "<?>"
}

func MatFromString(vm *VM, d string) *Elem {
	return &Elem{Type: "MAT", Value: new(mat.Dense)}
}

func MatCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	if e1.Type == "MAT" && e2.Type == "MAT" {
		switch v1 := e1.Value.(type) {
		case *mat.Dense:
			switch v2 := e2.Value.(type) {
			case *mat.Dense:
				if mat.Equal(v1, v2) {
					return Eq
				} else {
					return Ne
				}
			}
		}
	}
	return IDK
}

func MatDup(vm *VM, e *Elem) *Elem {
	if e.Type != "MAT" {
		return nil
	}
	m := new(mat.Dense)
	switch v := e.Value.(type) {
	case *mat.Dense:
		m.CloneFrom(v)
		return &Elem{Type: "MAT", Value: m}
	}
	return &Elem{Type: "MAT", Value: m}
}

func matMathOp(vm *VM, e1 *Elem, e2 *Elem, op int) (*Elem, error) {
	switch e1.Type {
	case "MAT":
		switch e2.Type {
		case "MAT":
			return MatMathOp(vm, e1, e2, op)
		case "int":
			return MatMathOperator(vm, e1, e2, op)
		case "flt":
			return MatMathOperator(vm, e1, e2, op)
		}
	case "int":
		switch e2.Type {
		case "MAT":
			return MatMathOperator(vm, e2, e1, op)
		}
	case "flt":
		switch e2.Type {
		case "MAT":
			return MatMathOperator(vm, e2, e1, op)
		}
	}
	return nil, fmt.Errorf("Invalid operands for MATRIX arithmetic")
}

func MatAdd(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return matMathOp(v, e1, e2, Add)
}

func MatSub(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return matMathOp(v, e1, e2, Sub)
}

func MatMul(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return matMathOp(v, e1, e2, Mul)
}

func MatDiv(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return matMathOp(v, e1, e2, Div)
}

func MatApply(v *VM, e1 *Elem, f func(interface{}) interface{}) (*Elem, error) {
	res := f(e1.Value.(*mat.Dense))
	switch res.(type) {
	case *mat.Dense:
		return &Elem{Type: "MAT", Value: res}, nil
	}
	return nil, fmt.Errorf("Return value for 'apply' function is not recognized")
}

func RegisterMatrix(vm *VM) {
	c_type := vm.GenType("MAT", MatFactory, MatToString, MatFromString, MatCompare, MatDup)
	c_type.Add = MatAdd
	c_type.Sub = MatSub
	c_type.Mul = MatMul
	c_type.Div = MatDiv
	c_type.Apply = MatApply
	vm.SetType("MAT", c_type)
}
