package vm

import (
	"fmt"
)

type Interval struct {
	Start float64
	End   float64
}

func IntervalFactory(vm *VM) *Elem {
	return &Elem{Type: "interval", Value: make([]Interval, 0)}
}

func IntervalToString(vm *VM, e *Elem) string {
	if e.Type == "interval" {
		out := "[ "
		for i := 0; i < len(e.Value.([]Interval)); i++ {
			iv := e.Value.([]Interval)[i]
			out += fmt.Sprintf("(%v,%v) ", iv.Start, iv.End)
		}
		out += " ]"
		return out
	}
	vm.Error("trying to convert a interval and it is not a interval: %v %T", e.Type, e.Value)
	return "<?>"
}

func IntervalFromString(vm *VM, d string) *Elem {
	return IntervalFactory(vm)
}

func IntervalCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	return IDK
}

func IntervalDup(vm *VM, e *Elem) *Elem {
	if e.Type == "interval" {
		i := IntervalFactory(vm)
		i.Value = e.Value
		return i
	}
	return nil
}

func RegisterInterval(vm *VM) {
	vm.RegisterType("interval", IntervalFactory, IntervalToString, IntervalFromString, IntervalCompare, IntervalDup)
}
