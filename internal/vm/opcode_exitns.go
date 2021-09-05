package vm

func ExitNSParser(vm *VM, args ...interface{}) {
	vm.Info("exit NAMESPACE %v", args)
}

func ExitNSEval(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit NAMESPACE %v", args)
	vm.EndNS()
	return nil, nil
}

func ExitNSLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("exit NAMESPACE(in lambda) %v", args)
	return &Elem{Type: "exitNS", Value: args[0].(string)}, nil
}

func ExitNSGen(vm *VM, args ...interface{}) error {
	vm.Fatal("ExitNS Gen not implemented")
	return nil
}

func ExitNSExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("ExitNS Exec not implemented")
	return nil, nil
}

func InitOpcodeExitNs(vm *VM) {
	vm.RegisterOpcode("exitNS", ExitNSParser, ExitNSLambda, ExitNSEval, ExitNSGen, ExitNSExec)
}
