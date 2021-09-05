package vm

import (
	"fmt"
)

type Elem struct {
	Name    string
	Type    string
	Value   interface{}
	Prefun  string
	Functor string
}

const (
	Eq  = 0
	Gt  = 1
	Ls  = -1
	IDK = -2
	Ne  = 2
)

type FactoryFun func(v *VM) *Elem
type ToStringFun func(v *VM, e *Elem) string
type FromStringFun func(v *VM, d string) *Elem
type CompareFun func(v *VM, e1 *Elem, e2 *Elem) int
type DupFun func(v *VM, e *Elem) *Elem
type OperatorFun func(v *VM, e1 *Elem, e2 *Elem) (*Elem, error)
type FunctionFun func(v *VM, e1 *Elem) (*Elem, error)
type GenFun func(v *VM) (*Elem, error)
type ApplyFun func(v *VM, e1 *Elem, f func(interface{}) interface{}) (*Elem, error)

type ElemHandler struct {
	Type       string
	Factory    FactoryFun
	ToString   ToStringFun
	FromString FromStringFun
	Compare    CompareFun
	Duplicate  DupFun
	Add        OperatorFun
	Sub        OperatorFun
	Mul        OperatorFun
	Div        OperatorFun
	Apply      ApplyFun
}

func NoopOperator(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return e1, nil
}
func NoopFunction(v *VM, e1 *Elem) (*Elem, error) {
	return e1, nil
}
func NoopGenerator(v *VM) (*Elem, error) {
	return nil, nil
}
func NoopApply(v *VM, e1 *Elem, f func(interface{}) interface{}) (*Elem, error) {
	return e1, nil
}

func (vm *VM) GenType(t string, ff FactoryFun, ts ToStringFun, fs FromStringFun, cf CompareFun, df DupFun) *ElemHandler {
	res := ElemHandler{Type: t, Factory: ff, ToString: ts, FromString: fs, Compare: cf, Duplicate: df}
	res.Add = NoopOperator
	res.Sub = NoopOperator
	res.Mul = NoopOperator
	res.Div = NoopOperator
	res.Apply = NoopApply
	return &res
}

func (vm *VM) RegisterType(t string, ff FactoryFun, ts ToStringFun, fs FromStringFun, cf CompareFun, df DupFun) bool {
	if _, ok := vm.Types.Load(t); ok {
		return true
	}
	res := vm.GenType(t, ff, ts, fs, cf, df)
	vm.Types.Store(t, res)
	vm.Debug("Register BUND datatype: %v", t)
	return true
}

func (vm *VM) SetType(t string, eh *ElemHandler) bool {
	if _, ok := vm.Types.Load(t); ok {
		return true
	}
	vm.Types.Store(t, eh)
	vm.Debug("Register BUND datatype: %v", t)
	return true
}

func (vm *VM) GetType(t string) (*ElemHandler, error) {
	if res, ok := vm.Types.Load(t); ok {
		return res.(*ElemHandler), nil
	}
	return nil, fmt.Errorf("BUND do not have datatype: %v", t)
}
