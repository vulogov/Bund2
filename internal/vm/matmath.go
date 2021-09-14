package vm

import (
	"fmt"
	"math/big"

	"gonum.org/v1/gonum/mat"
)

func MatMathOp(vm *VM, e1 *Elem, e2 *Elem, op int) (*Elem, error) {
	if e1.Type == "MAT" && e2.Type == "MAT" {
		vm.Debug("MAT operation %v requested", op)
		switch v1 := e1.Value.(type) {
		case *mat.Dense:
			switch v2 := e2.Value.(type) {
			case *mat.Dense:
				x1, y1 := v1.Dims()
				x2, y2 := v2.Dims()
				if x1 != x2 && y1 != y2 {
					return nil, fmt.Errorf("For the matrix math operations, matrixes must be the same arity: %v <-> %v %v <-> %v", x1, x2, y1, y2)
				}
				vm.Debug("MAT dimensions: %v=%v, %v=%v", x1, x2, y1, y2)
				mres := mat.NewDense(x1, y1, nil)
				switch op {
				case Add:
					mres.Add(v1, v2)
				case Sub:
					mres.Sub(v1, v2)
				case Mul:
					mres.MulElem(v1, v2)
				case Div:
					mres.DivElem(v1, v2)
				default:
					return nil, fmt.Errorf("Matrix operation was requested, and I do know for sure that's you got OP code wrong.")
				}
				return &Elem{Type: "MAT", Value: mres}, nil
			}
		}
	}
	return nil, fmt.Errorf("Matrix operation was requested, but I do not know how to do it, due error in data, or OP code")
}

func MatMathOperator(vm *VM, e1 *Elem, e2 *Elem, op int) (*Elem, error) {
	var v float64
	var r float64
	switch e2.Type {
	case "int":
		v = float64(e2.Value.(*big.Int).Int64())
	case "flt":
		v, _ = e2.Value.(*big.Float).Float64()
	default:
		return nil, fmt.Errorf("Matrix operation was requested, but I do not know how to do it, due error in data, or OP code")
	}
	if e1.Type != "MAT" {
		return nil, fmt.Errorf("Matrix operation was requested, but I do not know how to do it, due error in data, or OP code")
	}
	v1 := e1.Value.(*mat.Dense)
	x1, y1 := v1.Dims()
	mres := mat.NewDense(x1, y1, nil)
	for x := 0; x < x1; x++ {
		for y := 0; y < y1; y++ {
			switch op {
			case Add:
				r = v1.At(x, y) + v
			case Sub:
				r = v1.At(x, y) - v
			case Mul:
				r = v1.At(x, y) * v
			case Div:
				r = v1.At(x, y) / v
			default:
				return nil, fmt.Errorf("Matrix operation was requested, but I do not know how to do it, due error in data, or OP code")
			}
			mres.Set(x, y, r)
		}
	}
	return &Elem{Type: "MAT", Value: mres}, nil
}
