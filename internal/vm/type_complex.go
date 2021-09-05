package vm

import (
	"fmt"

	"strconv"
)

func ComplexFactory(vm *VM) *Elem {
	return &Elem{Type: "cpx", Value: complex(0, 0)}
}

func ComplexToString(vm *VM, e *Elem) string {
	if e.Type == "cpx" {
		switch v := e.Value.(type) {
		case complex128:
			return fmt.Sprint(v)
		}
	}
	vm.Error("trying to convert an Complex and it is not an Complex: %v %T", e.Type, e.Value)
	return "<?>"
}

func ComplexFromString(vm *VM, d string) *Elem {
	res, err := strconv.ParseComplex(d, 128)
	if err != nil {
		vm.OnError(err, "Error parsing complex number %v", d)
		return nil
	}
	return &Elem{Type: "cpx", Value: res}
}

func ComplexCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	var fe1, fe2 complex128
	if e1.Type == "cpx" {
		fe1 = e1.Value.(complex128)
	} else {
		return IDK
	}
	if e2.Type == "cpx" {
		fe2 = e2.Value.(complex128)
	} else {
		return IDK
	}
	if fe1 == fe2 {
		return Eq
	} else {
		return IDK
	}
}

func ComplexDup(vm *VM, e *Elem) *Elem {
	if e.Type != "cpx" {
		return nil
	}
	return &Elem{Type: "cpx", Value: e.Value}
}

func CpxAdd(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type != "cpx" && e1.Type != e2.Type {
		return nil, fmt.Errorf("Invalid operand type for complex operator")
	}
	return &Elem{Type: "cpx", Value: (e1.Value.(complex128) + e2.Value.(complex128))}, nil
}

func CpxMul(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type != "cpx" && e1.Type != e2.Type {
		return nil, fmt.Errorf("Invalid operand type for complex operator")
	}
	return &Elem{Type: "cpx", Value: (e1.Value.(complex128) * e2.Value.(complex128))}, nil
}

func CpxDiv(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type != "cpx" && e1.Type != e2.Type {
		return nil, fmt.Errorf("Invalid operand type for complex operator")
	}
	return &Elem{Type: "cpx", Value: (e1.Value.(complex128) / e2.Value.(complex128))}, nil
}

func CpxSub(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type != "cpx" && e1.Type != e2.Type {
		return nil, fmt.Errorf("Invalid operand type for complex operator")
	}
	return &Elem{Type: "cpx", Value: (e1.Value.(complex128) - e2.Value.(complex128))}, nil
}

func CpxApply(v *VM, e1 *Elem, f func(interface{}) interface{}) (*Elem, error) {
	if e1.Type != "cpx" {
		return nil, fmt.Errorf("Invalid operand type for complex Apply")
	}
	res := f(e1.Value.(complex128))
	switch res.(type) {
	case complex128:
		return &Elem{Type: "cpx", Value: res}, nil
	}
	return nil, fmt.Errorf("Return value for 'apply' function is not recognized")
}

func RegisterCpx(vm *VM) {
	cpx_type := vm.GenType("cpx", ComplexFactory, ComplexToString, ComplexFromString, ComplexCompare, ComplexDup)
	cpx_type.Add = CpxAdd
	cpx_type.Add = CpxSub
	cpx_type.Add = CpxMul
	cpx_type.Add = CpxDiv
	cpx_type.Apply = CpxApply
	vm.SetType("cpx", cpx_type)
}
