package vm

func (vm *VM) runop(t string, args ...interface{}) {
	var res *Elem
	var err error

	if !vm.InLambda() {
		var functor string
		var prefunction string
		if len(args) >= 3 {
			data := args[0].(string)
			if len(args[1].(string)) > 0 {
				prefunction = args[1].(string)
				eh, err := vm.GetType(prefunction)
				if err == nil {
					vm.Debug("PREFUNCTION: for datatype %v", prefunction)
					res := eh.FromString(vm, data)
					vm.Put(res)
				} else {
					vm.Debug("PREFUNCTION: for call %v", prefunction)
					vm.Put(&Elem{Type: "str", Value: data})
					vm.runop("CALL", prefunction, "", "")
				}
			} else {
				res_eval, err := vm.Opcode(t).InEval(vm, args...)
				if err != nil {
					vm.OnError(err, "Error in function application before functor: %v", args[0])
				}
				if res_eval != nil {
					vm.Put(res_eval)
				}
			}
			if len(args[2].(string)) > 0 {
				functor = args[2].(string)
				vm.runop("CALL", functor, "", "")
			}
		} else {
			vm.Fatal("RUNOP not enough arguments %v %v", t, args)
		}
	} else {
		res, err = vm.Opcode(t).InLambda(vm, args...)
		if err != nil {
			vm.OnError(err, "Error placing in lambda: %v", t)
		}
		if len(args) >= 3 {
			if len(args[1].(string)) > 0 {
				res.Prefun = args[1].(string)
			}
			if len(args[2].(string)) > 0 {
				res.Functor = args[2].(string)
			}
		}
		ls := vm.CurrentLambda()
		if ls != nil {
			vm.Debug("TO f(%v) %v", vm.CurrentNS.NameOfCurrentLambda(), res)
			ls.PushBack(res)
		} else {
			vm.Debug("Following OPCODE not been pushed to Lambda stack: %v", res.Type)
		}
	}
}

func (vm *VM) RunOp(t string, args ...interface{}) {
	if vm.CheckIgnore() {
		return
	}
	vm.runop(t, args...)
}

func (vm *VM) AlwaysRunOp(t string, args ...interface{}) {
	vm.runop(t, args...)
}

func (vm *VM) LambdaRunOp(t string, args ...interface{}) {
	res, err := vm.Opcode(t).InEval(vm, args...)
	if err != nil {
		vm.OnError(err, "Error evaluating: %v", t)
	}
	if res != nil {
		vm.Put(res)
	}
}
