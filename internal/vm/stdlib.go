package vm

func InitFUNCTIONS(vm *VM) {
	vm.Debug("[ BUND ] Initializing standard library")
	InitPrintFunctions(vm)
	InitSystemFunctions(vm)
}
