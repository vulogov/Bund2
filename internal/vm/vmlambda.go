package vm

import (
	"github.com/gammazero/deque"
)

func (vm *VM) CurrentLambda() *deque.Deque {
	var res *deque.Deque
	if !vm.IsStack() {
		vm.Fatal("Attempt to CurrentLambda() but NS doesn't exists")
		return nil
	}
	if vm.CurrentNS.LSMode.Len() == 0 {
		vm.Fatal("Attempt to detect mode for CurrentLambda() failed")
		return nil
	}
	switch vm.CurrentNS.LSMode.Back().(int) {
	case Llam:
		res = vm.CurrentNS.CurrentLambda()
	case Lgen:
		res = vm.CurrentNS.CurrentGenerator()
	case Lfun:
		res = vm.CurrentNS.CurrentFunction()
	case Lops:
		res = vm.CurrentNS.CurrentOperator()
	}
	return res
}

func (vm *VM) GetCurrentLambda(name string) *deque.Deque {
	if !vm.IsStack() {
		vm.Fatal("Attempt to CurrentLambda() but NS doesn't exists")
		return nil
	}
	if vm.CurrentNS.HasLambda(name) {
		return vm.CurrentNS.GetLambda(name)
	}
	if vm.CurrentNS.HasGenerator(name) {
		return vm.CurrentNS.GetGenerator(name)
	}
	if vm.CurrentNS.HasFunction(name) {
		return vm.CurrentNS.GetFunction(name)
	}
	if vm.CurrentNS.HasOperator(name) {
		return vm.CurrentNS.GetOperator(name)
	}
	return nil
}

func (vm *VM) CurrentFunction() *deque.Deque {
	if !vm.IsStack() {
		vm.Fatal("Attempt to CurrentFunction() but NS doesn't exists")
		return nil
	}
	return vm.CurrentNS.CurrentFunction()
}

func (vm *VM) InLambda() bool {
	if !vm.IsStack() {
		vm.Debug("Attempt to InLambda() but Stack doesn't exists")
		return false
	}
	if vm.CurrentNS.LambdasStack.Len() > 0 {
		vm.Debug("We are in InLambda(%v)", vm.CurrentNS.LambdasStack.Back().(string))
		return true
	}
	return false
}
