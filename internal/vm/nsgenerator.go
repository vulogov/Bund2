package vm

import (
	"github.com/gammazero/deque"
)

func (ns *NS) HasGenerator(name string) bool {
	if _, ok := ns.Gen.Load(name); ok {
		return true
	}
	return false
}

func (ns *NS) GetGenerator(name string) *deque.Deque {
	var res *deque.Deque
	if _res, ok := ns.Gen.Load(name); ok {
		ns.VM.Debug("Returning Generator from %v: %v", ns.Name, name)
		res = _res.(*deque.Deque)
	} else {
		ns.VM.Debug("Creating Generator in %v: %v", ns.Name, name)
		res = new(deque.Deque)
		ns.Gen.Store(name, res)
	}
	return res
}

func (ns *NS) HaveGenerator(name string) *deque.Deque {
	var res *deque.Deque
	if _res, ok := ns.Gen.Load(name); ok {
		ns.VM.Debug("Returning Generator from %v: %v", ns.Name, name)
		res = _res.(*deque.Deque)
		return res
	}
	return nil
}

func (ns *NS) InGenerator(name string) bool {
	if _, ok := ns.Gen.Load(name); ok {
		ns.LambdasStack.PushBack(name)
		ns.LSMode.PushBack(Lgen)
		ns.VM.Debug("We are going in Generator(%v)", name)
		return true
	}
	ns.VM.Error("Attempt to go in Generator(%v) failed", name)
	return false
}

func (ns *NS) NameOfCurrentGenerator() string {
	if ns.LambdasStack.Len() < 1 {
		ns.VM.Error("Attempt to get Generator function name on empty Generators stack")
		return ""
	}
	return ns.LambdasStack.Back().(string)
}

func (ns *NS) CurrentGenerator() *deque.Deque {
	var res *deque.Deque
	if ns.LambdasStack.Len() < 1 {
		ns.VM.Error("Attempt to select Generator function on empty Generators stack")
		return nil
	}
	name := ns.LambdasStack.Back().(string)
	if _res, ok := ns.Gen.Load(name); ok {
		ns.VM.Debug("Returning Generator from %v: %v", ns.Name, name)
		res = _res.(*deque.Deque)
		return res
	}
	ns.VM.Error("Something is seriously wrong, current Generator %v not found", name)
	return nil
}

func (ns *NS) CloseGenerator() bool {
	if ns.LambdasStack.Len() < 1 {
		ns.VM.Error("Attempt to close Generator fuction on empty Generators stack")
		return false
	}
	ln := ns.LambdasStack.PopBack().(string)
	ns.LSMode.PopBack()
	ns.VM.Debug("Closing Generator %v. Stack size: %v", ln, ns.LambdasStack.Len())
	return true
}
