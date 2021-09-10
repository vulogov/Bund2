package vm

import (
	"fmt"

	"github.com/cstockton/go-conv"
	"github.com/gammazero/deque"
)

func IndexParser(vm *VM, args ...interface{}) {
	vm.Info("INDEX %v", args)
}

func IndexEval(vm *VM, args ...interface{}) (*Elem, error) {
	if !vm.CanGet() {
		return nil, fmt.Errorf("Stack is too shallow for an index operation")
	}
	v := vm.Get()
	if v.Type != "dblock" {
		return nil, fmt.Errorf("You must have (* ) on the stack for an index operation")
	}
	if args[1].(string) != "" {
		si, err := conv.Int(args[0].(string))
		vm.OnError(err, "Error handling #<index>")
		ei, err := conv.Int(args[1].(string))
		vm.OnError(err, "Error handling #<index>")
		vm.Debug("Taking range %v::%v", si, ei)
		if si > ei {
			return nil, fmt.Errorf("Start index must be less for end index for #<index> operation")
		}
		if si > int(BlockLen(v)) || ei > int(BlockLen(v)) {
			return nil, fmt.Errorf("Out of bound for #<index>")
		}
		eh, err := vm.GetType("dblock")
		vm.OnError(err, "Error handling #<index>")
		res := eh.Factory(vm)
		q := res.Value.(*deque.Deque)
		for i := si; i <= ei; i++ {
			d, err := AtBlock(v, int64(i))
			vm.OnError(err, "Error handling #<index>")
			q.PushBack(d)
		}
		return res, nil
	} else {
		si, err := conv.Int(args[0].(string))
		vm.OnError(err, "Error handling #<index>")
		vm.Debug("Taking value %v", si)
		if si > int(BlockLen(v)) {
			return nil, fmt.Errorf("Out of bound for #<index>")
		}
		res, err := AtBlock(v, int64(si))
		vm.OnError(err, "Error handling #<index>")
		return res, nil
	}
	return nil, nil
}

func IndexLambda(vm *VM, args ...interface{}) (*Elem, error) {
	return &Elem{Type: "index", Value: fmt.Sprintf("%v:%v", args[0], args[1])}, nil
}

func IndexGen(vm *VM, args ...interface{}) error {
	vm.Fatal("Index Gen not implemented")
	return nil
}

func IndexExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("Index Exec not implemented")
	return nil, nil
}

func InitOpcodeIndex(vm *VM) {
	vm.RegisterOpcode("index", IndexParser, IndexLambda, IndexEval, IndexGen, IndexExec)
}
