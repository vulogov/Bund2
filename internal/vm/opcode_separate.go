package vm

func SeparateParser(vm *VM, args ...interface{}) {
	vm.Info("SEPARATE %v", args)
}

func SeparateEval(vm *VM, args ...interface{}) (*Elem, error) {
	var val *Elem
	if vm.CurrentNS.GetOption("separateinline", false).(bool) {
		vm.Debug("SEPARATE inline")
		val = &Elem{Type: "SEPARATE", Value: nil}
	} else {
		vm.Debug("SEPARATE command")
		val = vm.Separate()
	}
	return val, nil
}

func SeparateLambda(vm *VM, args ...interface{}) (*Elem, error) {
	return &Elem{Type: "SEPARATE", Value: nil}, nil
}

func SeparateGen(vm *VM, args ...interface{}) error {
	vm.Fatal("SEPARATE Gen not implemented")
	return nil
}

func SeparateExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("SEPARATE Exec not implemented")
	return nil, nil
}

func InitOpcodeSeparate(vm *VM) {
	vm.RegisterOpcode("SEPARATE", SeparateParser, SeparateLambda, SeparateEval, SeparateGen, SeparateExec)
}
