package vm

import (
	"fmt"

	"github.com/gammazero/deque"
)

func DblockFactory(vm *VM) *Elem {
	return &Elem{Type: "dblock", Value: new(deque.Deque)}
}

func DblockToString(vm *VM, e *Elem) string {
	if e.Type == "dblock" {
		res := "(* "
		for i := 0; i < e.Value.(*deque.Deque).Len(); i++ {
			_e := e.Value.(*deque.Deque).At(i)
			eh, err := vm.GetType(_e.(*Elem).Type)
			if err != nil {
				vm.Error("Can not get type: %v", _e.(*Elem).Type)
				continue
			}
			res = res + eh.ToString(vm, _e.(*Elem))
			res = res + " ,"
		}
		res = res + " )"
		return res
	}
	vm.Error("trying to convert an Dblock and it is not an Dblock: %v %T", e.Type, e.Value)
	return "<?>"
}

func DblockFromString(vm *VM, d string) *Elem {
	res := new(deque.Deque)
	return &Elem{Type: "dblock", Value: &res}
}

func DblockCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	if DblockSame(vm, e1, e2) {
		return Eq
	}
	return Ne
}

func DblockDup(vm *VM, e *Elem) *Elem {
	if e.Type != "dblock" {
		return nil
	}
	res := DblockFactory(vm)
	q1 := e.Value.(*deque.Deque)
	q2 := res.Value.(*deque.Deque)
	for i := 0; i < q1.Len(); i++ {
		e := q1.At(i).(*Elem)
		q2.PushBack(e)
	}
	return res
}

func dblockMathOp(vm *VM, e1 *Elem, e2 *Elem, op int) (*Elem, error) {
	if e1.Type == "dblock" && e2.Type == "dblock" {
		return DblocksMathOperator(vm, e1, e2, op)
	} else if e1.Type == "dblock" && (e2.Type == "int" || e2.Type == "flt") {
		return DblocksMathOnOperator(vm, e1, e2, op)
	}
	return nil, fmt.Errorf("Unknown operand type for flt arithmetic operation ")
}

func DblockAdd(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return dblockMathOp(v, e1, e2, Add)
}

func DblockSub(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return dblockMathOp(v, e1, e2, Sub)
}

func DblockMul(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return dblockMathOp(v, e1, e2, Mul)
}

func DblockDiv(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return dblockMathOp(v, e1, e2, Div)
}

func DblockApply(v *VM, e1 *Elem, f func(interface{}) interface{}) (*Elem, error) {
	res := f(e1.Value.(*deque.Deque))
	switch res.(type) {
	case *deque.Deque:
		return &Elem{Type: "dblock", Value: res}, nil
	}
	return nil, fmt.Errorf("Return value for 'apply' function is not recognized")
}

func RegisterDblock(vm *VM) {
	c_type := vm.GenType("dblock", DblockFactory, DblockToString, DblockFromString, DblockCompare, DblockDup)
	c_type.Add = DblockAdd
	c_type.Sub = DblockSub
	c_type.Mul = DblockMul
	c_type.Div = DblockDiv
	c_type.Apply = DblockApply
	vm.SetType("dblock", c_type)
}
