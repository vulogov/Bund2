package vm

import (
	"fmt"
	"math/big"

	"github.com/gammazero/deque"
	"gonum.org/v1/gonum/mat"
)

func MatrixElement(vm *VM, e *Elem) (*Elem, error) {
	var x, y int
	var fill float64
	if e.Type != "dblock" {
		return nil, fmt.Errorf("You have to pass DATABLOCK as (* ) for M (Matrix) creation. I have: %v", e.Type)
	}
	if BlockLen(e) == 2 {
		_x, err := BlockAt(e, "int", 1)
		vm.OnError(err, "Error during Matrix creation in M")
		_y, err := BlockAt(e, "int", 0)
		vm.OnError(err, "Error during Matrix creation in M")
		x = int(_x.(int64))
		y = int(_y.(int64))
		fill = 0.0
	} else if BlockLen(e) == 3 {
		_fill, err := BlockAt(e, "flt", 0)
		vm.OnError(err, "Error during Matrix creation in M")
		_x, err := BlockAt(e, "int", 2)
		vm.OnError(err, "Error during Matrix creation in M")
		_y, err := BlockAt(e, "int", 1)
		vm.OnError(err, "Error during Matrix creation in M")
		x = int(_x.(int64))
		y = int(_y.(int64))
		fill, _ = _fill.(*big.Float).Float64()
	} else {
		return nil, fmt.Errorf("You have to pass (* x y <init value> y x ) to M")
	}
	vm.Debug("Creating matrix X=%v Y=%v FILL=%v", x, y, fill)
	farray := make([]float64, (x * y))
	for i := 0; i < len(farray); i++ {
		farray[i] = fill
	}
	return &Elem{Type: "MAT", Value: mat.NewDense(x, y, farray)}, nil
}

func matrixSet(vm *VM, mtx *Elem, e *Elem) {
	var x, y int
	var v float64

	if BlockLen(e) == 3 {
		_v, err := BlockAt(e, "flt", 0)
		vm.OnError(err, "Error during M/Set")
		_x, err := BlockAt(e, "int", 2)
		vm.OnError(err, "Error during M/Set")
		_y, err := BlockAt(e, "int", 1)
		vm.OnError(err, "Error during M/Set")
		x = int(_x.(int64))
		y = int(_y.(int64))
		v, _ = _v.(*big.Float).Float64()
		switch m := mtx.Value.(type) {
		case *mat.Dense:
			vm.Debug("M/Set X=%v Y=%v V=%v", x, y, v)
			m.Set(x, y, v)
		default:
			vm.Fatal("You do not have a proper MAT value")
		}
		return
	}
	vm.Fatal("You have to pass (* value y x ) to M/Set")
}

func MatrixElementSet(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	switch e.Type {
	case "MAT":
		switch e1.Type {
		case "dblock":
			matrixSet(vm, e, e1)
			return e, nil
		}
	case "dblock":
		switch e1.Type {
		case "MAT":
			matrixSet(vm, e1, e)
			return e1, nil
		}
	}
	return nil, fmt.Errorf("You must have MAT and dblock to perform M/Set operation")
}

func matrixGet(vm *VM, mtx *Elem, e *Elem) *Elem {
	var x, y int

	if BlockLen(e) == 2 {
		_x, err := BlockAt(e, "int", 1)
		vm.OnError(err, "Error during M/Get")
		_y, err := BlockAt(e, "int", 0)
		vm.OnError(err, "Error during M/Get")
		x = int(_x.(int64))
		y = int(_y.(int64))
		switch m := mtx.Value.(type) {
		case *mat.Dense:
			vm.Debug("M/Get X=%v Y=%v", x, y)
			val := m.At(x, y)
			return &Elem{Type: "flt", Value: big.NewFloat(val)}
		default:
			vm.Fatal("You do not have a proper MAT value")
		}
		vm.Fatal("I do not know what to do in M/Get")
	}
	vm.Fatal("You have to pass (* value y x ) to M/Set")
	return nil
}

func MatrixElementGet(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	switch e.Type {
	case "MAT":
		switch e1.Type {
		case "dblock":
			res := matrixGet(vm, e, e1)
			vm.Put(e)
			return res, nil
		}
	case "dblock":
		switch e1.Type {
		case "MAT":
			res := matrixGet(vm, e1, e)
			vm.Put(e1)
			return res, nil
		}
	}
	return nil, fmt.Errorf("You must have MAT and dblock to perform M/Set operation")
}

func MatrixElementMake(vm *VM, e *Elem) (*Elem, error) {
	var data []float64
	if e.Type != "dblock" {
		return nil, fmt.Errorf("Expecting to get DBLOCk with data for matrix creation: %v", e.Type)
	}
	q := e.Value.(*deque.Deque)
	rows := 0
	cols := 0
	colsize := 0
	for i := q.Len() - 1; i >= 0; i-- {
		e1 := q.At(i).(*Elem)
		if e1.Type == "SEPARATE" {
			rows += 1
			if colsize == 0 {
				colsize = cols
			}
			if cols != colsize {
				return nil, fmt.Errorf("M/Make matrix is not uniform: %v <> %v", colsize, cols)
			}
			cols = 0
			continue
		}
		if e1.Type == "flt" {
			cols += 1
			v, _ := e1.Value.(*big.Float).Float64()
			data = append(data, v)
		}
		if e1.Type == "int" {
			cols += 1
			data = append(data, float64(e1.Value.(*big.Int).Int64()))
		}
	}
	res := mat.NewDense(rows, colsize, data)
	return &Elem{Type: "MAT", Value: res}, nil
}

func MatrixElementBlock(vm *VM, e *Elem) (*Elem, error) {
	if e.Type != "MAT" {
		return nil, fmt.Errorf("Expecting to get MAT with data for BLOCK creation: %v", e.Type)
	}
	eh, err := vm.GetType("dblock")
	vm.OnError(err, "Error in M/ToBlock")
	res := eh.Factory(vm)
	q := res.Value.(*deque.Deque)
	x, y := e.Value.(*mat.Dense).Dims()
	for i := 0; i < x; i++ {
		q.PushFront(&Elem{Type: "SEPARATE", Value: nil})
		for j := 0; j < y; j++ {
			e1 := e.Value.(*mat.Dense).At(i, j)
			q.PushFront(&Elem{Type: "flt", Value: big.NewFloat(e1)})
		}
	}
	return res, nil
}

func InitMatFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitMatFunctions() reached")
	vm.AddFunction("M", MatrixElement)
	vm.AddFunction("M/Make", MatrixElementMake)
	vm.AddFunction("M/ToBlock", MatrixElementBlock)
	vm.AddOperator("M/Set", MatrixElementSet)
	vm.AddOperator("M/Get", MatrixElementGet)
}
