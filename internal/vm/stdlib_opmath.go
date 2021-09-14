package vm

func AddOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	eh, err := vm.GetType(e1.Type)
	vm.OnError(err, "Error during +")
	res, err := eh.Add(vm, e1, e2)
	vm.OnError(err, "Error during +")
	return res, nil
}

func MulOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	eh, err := vm.GetType(e1.Type)
	vm.OnError(err, "Error during *")
	res, err := eh.Mul(vm, e1, e2)
	vm.OnError(err, "Error during *")
	return res, nil
}

func DivOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	eh, err := vm.GetType(e1.Type)
	vm.OnError(err, "Error during \\")
	res, err := eh.Div(vm, e1, e2)
	vm.OnError(err, "Error during \\")
	return res, nil
}

func SubOperator(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	eh, err := vm.GetType(e1.Type)
	vm.OnError(err, "Error during -")
	res, err := eh.Sub(vm, e1, e2)
	vm.OnError(err, "Error during -")
	return res, nil
}

func InitOpMath(vm *VM) {
	vm.Debug("[ BUND ] bund.InitOpMath() reached")
	vm.AddOperator("+", AddOperator)
	vm.AddOperator("*", MulOperator)
	vm.AddOperator("×", MulOperator)
	vm.AddOperator("\\", DivOperator)
	vm.AddOperator("÷", DivOperator)
	vm.AddOperator("-", SubOperator)
	// vm.AddOperator("**", PowOperator)
}
