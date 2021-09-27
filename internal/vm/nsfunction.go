package vm

import (
	"github.com/gammazero/deque"
)

func (ns *NS) HasFunction(name string) bool {
	if _, ok := ns.Fun.Load(name); ok {
		return true
	}
	return false
}

func (ns *NS) GetFunction(name string) *deque.Deque {
	var res *deque.Deque
	if _res, ok := ns.Fun.Load(name); ok {
		ns.VM.Debug("Returning FUNCTION from %v: %v", ns.Name, name)
		res = _res.(*deque.Deque)
	} else {
		ns.VM.Debug("Creating FUNCTION in %v: %v", ns.Name, name)
		res = new(deque.Deque)
		ns.Fun.Store(name, res)
	}
	return res
}

func (ns *NS) HaveFunction(name string) *deque.Deque {
	var res *deque.Deque
	if _res, ok := ns.Fun.Load(name); ok {
		ns.VM.Debug("Returning FUNCTION from %v: %v", ns.Name, name)
		res = _res.(*deque.Deque)
		return res
	}
	return nil
}

func (ns *NS) InFunction(name string) bool {
	if _, ok := ns.Fun.Load(name); ok {
		ns.LambdasStack.PushBack(name)
		ns.LSMode.PushBack(Lfun)
		ns.VM.Debug("We are going in Function(%v)", name)
		return true
	}
	ns.VM.Error("Attempt to go in Function(%v) failed", name)
	return false
}

func (ns *NS) NameOfCurrentFunction() string {
	if ns.LambdasStack.Len() < 1 {
		ns.VM.Error("Attempt to get Function name on empty Lambdas stack")
		return ""
	}
	return ns.LambdasStack.Back().(string)
}

func (ns *NS) CurrentFunction() *deque.Deque {
	var res *deque.Deque
	if ns.LambdasStack.Len() < 1 {
		ns.VM.Error("Attempt to select Function on empty Lambdas stack")
		return nil
	}
	name := ns.LambdasStack.Back().(string)
	if _res, ok := ns.Fun.Load(name); ok {
		ns.VM.Debug("Returning LAMBDA from %v: %v", ns.Name, name)
		res = _res.(*deque.Deque)
		return res
	}
	ns.VM.Error("Something is seriously wrong, current function %v not found", name)
	return nil
}

func (ns *NS) CloseFunction() bool {
	if ns.LambdasStack.Len() < 1 {
		ns.VM.Error("Attempt to close Lambda fuction on empty Lambdas stack")
		return false
	}
	ln := ns.LambdasStack.PopBack().(string)
	ns.LSMode.PopBack()
	ns.VM.Debug("Closing lambda %v. Stack size: %v", ln, ns.LambdasStack.Len())
	return true
}
