package vm

import (
	"github.com/gammazero/deque"
)

func (ns *NS) HasOperator(name string) bool {
	if _, ok := ns.Fun.Load(name); ok {
		return true
	}
	return false
}

func (ns *NS) GetOperator(name string) *deque.Deque {
	var res *deque.Deque
	if _res, ok := ns.Fun.Load(name); ok {
		ns.VM.Debug("Returning Operator from %v: %v", ns.Name, name)
		res = _res.(*deque.Deque)
	} else {
		ns.VM.Debug("Creating Operator in %v: %v", ns.Name, name)
		res = new(deque.Deque)
		ns.Fun.Store(name, res)
	}
	return res
}

func (ns *NS) HaveOperator(name string) *deque.Deque {
	var res *deque.Deque
	if _res, ok := ns.Fun.Load(name); ok {
		ns.VM.Debug("Returning Operator from %v: %v", ns.Name, name)
		res = _res.(*deque.Deque)
		return res
	}
	return nil
}

func (ns *NS) InOperator(name string) bool {
	if _, ok := ns.Fun.Load(name); ok {
		ns.LambdasStack.PushBack(name)
		ns.VM.Debug("We are going in Operator(%v)", name)
		return true
	}
	ns.VM.Error("Attempt to go in Operator(%v) failed", name)
	return false
}

func (ns *NS) NameOfCurrentOperator() string {
	if ns.LambdasStack.Len() < 1 {
		ns.VM.Error("Attempt to get Operator function name on empty Operators stack")
		return ""
	}
	return ns.LambdasStack.Back().(string)
}

func (ns *NS) CurrentOperator() *deque.Deque {
	var res *deque.Deque
	if ns.LambdasStack.Len() < 1 {
		ns.VM.Error("Attempt to select Operator function on empty Operators stack")
		return nil
	}
	name := ns.LambdasStack.Back().(string)
	if _res, ok := ns.Fun.Load(name); ok {
		ns.VM.Debug("Returning Operator from %v: %v", ns.Name, name)
		res = _res.(*deque.Deque)
		return res
	}
	ns.VM.Error("Something is seriously wrong, current Operator %v not found", name)
	return nil
}

func (ns *NS) CloseOperator() bool {
	if ns.LambdasStack.Len() < 1 {
		ns.VM.Error("Attempt to close Lambda fuction on empty Lambdas stack")
		return false
	}
	ln := ns.LambdasStack.PopBack().(string)
	ns.VM.Debug("Closing Operator %v. Stack size: %v", ln, ns.LambdasStack.Len())
	return true
}
