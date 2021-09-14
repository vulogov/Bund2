package vm

import (
	"fmt"
	"math"
	"math/big"
)

func leibniz(iteration int64) *big.Float {
	var n int64
	n = 0
	sum := big.NewFloat(0.0)
	numerator := big.NewFloat(1.0)
	denominator := big.NewFloat(1.0)
	for {
		toSum := big.NewFloat(0.0)
		toSum.Quo(numerator, denominator)
		sum.Add(sum, toSum)
		numerator.Mul(numerator, big.NewFloat(-1.0))
		denominator.Add(denominator, big.NewFloat(2.0))
		n++
		if n == iteration {
			break
		}
	}
	res := big.NewFloat(0.0)
	return res.Mul(sum, big.NewFloat(4.0))
}

func MathPiElement(vm *VM, e *Elem) (*Elem, error) {
	if e.Type == "int" {
		return &Elem{Type: "flt", Value: leibniz(e.Value.(*big.Int).Int64())}, nil
	}
	return nil, fmt.Errorf("math/Pi expects number : %v", e.Type)
}

func MathSinElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("SIN(): %v", e.Type)
	switch e.Type {
	case "flt":
		r, _ := e.Value.(*big.Float).Float64()
		return &Elem{Type: "flt", Value: big.NewFloat(math.Sin(r))}, nil
	case "int":
		return &Elem{Type: "flt", Value: big.NewFloat(math.Sin(float64(e.Value.(*big.Int).Int64())))}, nil
	case "dblock":
		return DblockWalk(vm, e, math.Sin)
	case "MAT":
		return MatrixWalk(vm, e, math.Sin)
	}
	return e, nil
}

func MathCosElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("COS(): %v", e.Type)
	switch e.Type {
	case "flt":
		r, _ := e.Value.(*big.Float).Float64()
		return &Elem{Type: "flt", Value: big.NewFloat(math.Cos(r))}, nil
	case "int":
		return &Elem{Type: "flt", Value: big.NewFloat(math.Cos(float64(e.Value.(*big.Int).Int64())))}, nil
	case "dblock":
		return DblockWalk(vm, e, math.Cos)
	case "MAT":
		return MatrixWalk(vm, e, math.Cos)
	}
	return e, nil
}

func MathTanElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("TAN(): %v", e.Type)
	switch e.Type {
	case "flt":
		r, _ := e.Value.(*big.Float).Float64()
		return &Elem{Type: "flt", Value: big.NewFloat(math.Tan(r))}, nil
	case "int":
		return &Elem{Type: "flt", Value: big.NewFloat(math.Tan(float64(e.Value.(*big.Int).Int64())))}, nil
	case "dblock":
		return DblockWalk(vm, e, math.Tan)
	case "MAT":
		return MatrixWalk(vm, e, math.Tan)
	}
	return e, nil
}

func MathSqrtElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("SQRT(): %v", e.Type)
	switch e.Type {
	case "flt":
		r, _ := e.Value.(*big.Float).Float64()
		return &Elem{Type: "flt", Value: big.NewFloat(math.Sqrt(r))}, nil
	case "int":
		return &Elem{Type: "flt", Value: big.NewFloat(math.Sqrt(float64(e.Value.(*big.Int).Int64())))}, nil
	case "dblock":
		return DblockWalk(vm, e, math.Sqrt)
	case "MAT":
		return MatrixWalk(vm, e, math.Sqrt)
	}
	return e, nil
}

func MathExpElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("EXP(): %v", e.Type)
	switch e.Type {
	case "flt":
		r, _ := e.Value.(*big.Float).Float64()
		return &Elem{Type: "flt", Value: big.NewFloat(math.Exp(r))}, nil
	case "int":
		return &Elem{Type: "flt", Value: big.NewFloat(math.Exp(float64(e.Value.(*big.Int).Int64())))}, nil
	case "dblock":
		return DblockWalk(vm, e, math.Exp)
	case "MAT":
		return MatrixWalk(vm, e, math.Exp)
	}
	return e, nil
}

func MathLogElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("LOG(): %v", e.Type)
	switch e.Type {
	case "flt":
		r, _ := e.Value.(*big.Float).Float64()
		return &Elem{Type: "flt", Value: big.NewFloat(math.Log(r))}, nil
	case "int":
		return &Elem{Type: "flt", Value: big.NewFloat(math.Log(float64(e.Value.(*big.Int).Int64())))}, nil
	case "dblock":
		return DblockWalk(vm, e, math.Log)
	case "MAT":
		return MatrixWalk(vm, e, math.Log)
	}
	return e, nil
}

func MathLog10Element(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("LOG10(): %v", e.Type)
	switch e.Type {
	case "flt":
		r, _ := e.Value.(*big.Float).Float64()
		return &Elem{Type: "flt", Value: big.NewFloat(math.Log10(r))}, nil
	case "int":
		return &Elem{Type: "flt", Value: big.NewFloat(math.Log10(float64(e.Value.(*big.Int).Int64())))}, nil
	case "dblock":
		return DblockWalk(vm, e, math.Log10)
	case "MAT":
		return MatrixWalk(vm, e, math.Log10)
	}
	return e, nil
}

func InitMathFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitMathFunctions() reached")
	vm.AddFunction("math/Sin", MathSinElement)
	vm.AddFunction("math/Cos", MathCosElement)
	vm.AddFunction("math/Tan", MathTanElement)
	vm.AddFunction("math/Sqrt", MathSqrtElement)
	vm.AddFunction("math/Exp", MathExpElement)
	vm.AddFunction("math/Log", MathLogElement)
	vm.AddFunction("math/Log10", MathLog10Element)
	vm.AddFunction("math/Pi", MathPiElement)
}
