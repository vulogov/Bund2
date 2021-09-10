package vm

func InitFUNCTIONS(vm *VM) {
	vm.Debug("[ BUND ] Initializing standard library")
	InitPrintFunctions(vm)
	InitSystemFunctions(vm)
	InitRndFunctions(vm)
	InitStringFunctions(vm)
	InitMathFunctions(vm)
	InitJsonFunctions(vm)
	InitMatFunctions(vm)
	InitIntervalFunctions(vm)
	InitGPMOperators(vm)
	InitOpCmp(vm)
	InitVFSFunctions(vm)
	InitHttpFunctions(vm)
	InitHtmlFunctions(vm)
}
