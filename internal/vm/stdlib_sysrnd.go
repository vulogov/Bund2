package vm

import (
	"crypto/rand"
	"fmt"
	"math/big"
	mrand "math/rand"
	"time"
)

func IntRandomElement(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type != "int" && e2.Type != "int" {
		return nil, fmt.Errorf("Integers required for rnd/Integer operation: %v:%v", e2.Type, e1.Type)
	}
	max := e2.Value.(*big.Int)
	min := e1.Value.(*big.Int)
	max.Sub(max, min)
	if max.Cmp(min) != 1 {
		return nil, fmt.Errorf("max(%v) > min(%v) for rnd/Integer operation/Integer", max.String(), min.String())
	}
	res, err := rand.Int(rand.Reader, max)
	vm.OnError(err, "Error in generating random number rand/Integer")
	res.Add(res, min)
	return &Elem{Type: "int", Value: res}, nil
}

func FltRandomElement(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type != "flt" && e2.Type != "flt" {
		return nil, fmt.Errorf("Floats required for rnd/Float operation: %v:%v", e2.Type, e1.Type)
	}
	max, _ := e2.Value.(*big.Float).Float64()
	min, _ := e1.Value.(*big.Float).Float64()
	res := (min + mrand.Float64()*(max-min))
	return &Elem{Type: "flt", Value: res}, nil
}

func PrimeRandomElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("rnd/Prime: %v", e.Type)
	if e.Type != "int" {
		return nil, fmt.Errorf("Integer required for rnd/Prime operation: %v", e.Type)
	}
	bits := int(e.Value.(*big.Int).Int64())
	if bits <= 2 {
		return nil, fmt.Errorf("You must request more than two bits for rnd/Prime operation: %v", bits)
	}
	res, err := rand.Prime(rand.Reader, bits)
	vm.OnError(err, "Error in generating prime rand/Prime")
	return &Elem{Type: "int", Value: res}, nil
}

func InitRndFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitRndFunctions() reached")
	mrand.Seed(time.Now().UnixNano())
	vm.AddOperator("rnd/Integer", IntRandomElement)
	vm.AddOperator("rnd/Float", FltRandomElement)
	vm.AddFunction("rnd/Prime", PrimeRandomElement)
}
