package vm

import (
	"fmt"
	"math/big"
)

func PercentOperator(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {

	if e.Type == e1.Type {
		switch e.Type {
		case "int":
			res := big.NewInt(0)
			res.Mul(e.Value.(*big.Int), e1.Value.(*big.Int))
			res.Div(res, big.NewInt(100))
			return &Elem{Type: "int", Value: res}, nil
		case "flt":
			res := big.NewFloat(0.0)
			res.Mul(e.Value.(*big.Float), e1.Value.(*big.Float))
			res.Quo(res, big.NewFloat(100.0))
			return &Elem{Type: "flt", Value: res}, nil
		default:
			return nil, fmt.Errorf("Unknown data types for percent operator: %v", e.Type)
		}
	}

	return nil, fmt.Errorf("Two elements required in stack for percent operator")
}

func PercentOfOperator(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {

	if e.Type == e1.Type {
		switch e.Type {
		case "int":
			res := big.NewInt(0)
			res.Mul(e.Value.(*big.Int), big.NewInt(100))
			res.Div(res, e1.Value.(*big.Int))
			return &Elem{Type: "int", Value: res}, nil
		case "flt":
			res := big.NewFloat(0.0)
			res.Mul(e.Value.(*big.Float), big.NewFloat(100.0))
			res.Quo(res, e1.Value.(*big.Float))
			return &Elem{Type: "flt", Value: res}, nil
		default:
			return nil, fmt.Errorf("Unknown data types for percent operator: %v", e.Type)
		}
	}

	return nil, fmt.Errorf("Two elements required in stack for percent operator")
}

func ChangeOperator(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {

	if e.Type == e1.Type {
		switch e.Type {
		case "int":
			res := big.NewInt(0)
			res.Sub(e.Value.(*big.Int), e1.Value.(*big.Int))
			res.Div(res, e1.Value.(*big.Int))
			res.Mul(res, big.NewInt(100))
			return &Elem{Type: "int", Value: res}, nil
		case "flt":
			res := big.NewFloat(0.0)
			res.Sub(e.Value.(*big.Float), e1.Value.(*big.Float))
			res.Quo(res, e1.Value.(*big.Float))
			res.Mul(res, big.NewFloat(100.0))
			return &Elem{Type: "flt", Value: res}, nil
		default:
			return nil, fmt.Errorf("Unknown data types for percent operator: %v", e.Type)
		}
	}

	return nil, fmt.Errorf("Two elements required in stack for percent operator")
}

func InitOpPercent(vm *VM) {
	vm.Debug("[ BUND ] bund.InitOpPercent() reached")
	vm.AddOperator("percent", PercentOperator)
	vm.AddOperator("percentOf", PercentOfOperator)
	vm.AddOperator("changeOf", ChangeOperator)
}
