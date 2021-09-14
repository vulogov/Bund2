package vm

import (
	"fmt"
	"math/big"

	"github.com/gammazero/deque"
)

func DblocksMathOp(vm *VM, e1 *Elem, e2 *Elem, op int) (*Elem, error) {
	var err error
	if e1.Type != "dblock" && e1.Type == "dblock" {
		return nil, fmt.Errorf("Math oeration with DBLOCk have an incorrect values")
	}
	res := &Elem{Type: "flt", Value: big.NewFloat(0.0)}
	for i := 0; i < int(BlockLen(e2)); i++ {
		v, err := AtBlock(e1, int64(i))
		vm.OnError(err, "Error on math operation over DBLOCK")
		if v.Type == "int" || v.Type == "flt" {
			res, err = bigMathOp(vm, res, v, op)
			vm.OnError(err, "Error on math operation over DBLOCK")
		}
	}
	if e1.Type == "int" || e1.Type == "flt" {
		res, err = bigMathOp(vm, e1, res, op)
		vm.OnError(err, "Error on math operation over DBLOCK")
	} else {
		return nil, fmt.Errorf("Math oeration with DBLOCk have an incorrect operand")
	}
	return res, nil
}

func DblocksMathOperator(vm *VM, e1 *Elem, e2 *Elem, op int) (*Elem, error) {
	if e1.Type != "dblock" && e2.Type != "dblock" {
		return nil, fmt.Errorf("Math oeration with DBLOCK have an incorrect arity")
	}
	if !DblockSameArity(vm, e1, e2) {
		return nil, fmt.Errorf("Math oeration with DBLOCk have an incorrect values")
	}
	eh, err := vm.GetType("dblock")
	vm.OnError(err, "Error on math operation over DBLOCK")
	res := eh.Factory(vm)
	q := res.Value.(*deque.Deque)
	for i := 0; i < int(BlockLen(e1)); i++ {
		v1, err := AtBlock(e1, int64(i))
		vm.OnError(err, "Error on math operation over DBLOCK")
		v2, err := AtBlock(e2, int64(i))
		vm.OnError(err, "Error on math operation over DBLOCK")
		if v1.Type == "int" {
			r, err := bigMathOp(vm, v1, v2, op)
			vm.OnError(err, "Error on math operation over DBLOCK")
			q.PushBack(r)
		} else if v1.Type == "flt" {
			r, err := floatMathOp(vm, v1, v2, op)
			vm.OnError(err, "Error on math operation over DBLOCK")
			q.PushBack(r)
		} else {
			q.PushBack(v1)
		}
	}
	return res, nil
}

func DblocksMathOnOperator(vm *VM, e1 *Elem, e2 *Elem, op int) (*Elem, error) {
	if e1.Type != "dblock" {
		return nil, fmt.Errorf("Math oeration with DBLOCk have an incorrect values")
	}
	eh, err := vm.GetType("dblock")
	vm.OnError(err, "Error on math operation over DBLOCK")
	res := eh.Factory(vm)
	q := res.Value.(*deque.Deque)
	for i := 0; i < int(BlockLen(e1)); i++ {
		v1, err := AtBlock(e1, int64(i))
		vm.OnError(err, "Error on math operation over DBLOCK")
		if v1.Type == "int" {
			r, err := bigMathOp(vm, v1, e2, op)
			vm.OnError(err, "Error on math operation over DBLOCK")
			q.PushBack(r)
		} else if v1.Type == "flt" {
			r, err := floatMathOp(vm, v1, e2, op)
			vm.OnError(err, "Error on math operation over DBLOCK")
			q.PushBack(r)
		} else {
			q.PushBack(v1)
		}
	}
	return res, nil
}
