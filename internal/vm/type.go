package vm

func (vm *VM) RegisterTypes() {
	vm.Debug("BUND: registering dynamic types")
	RegisterStack(vm)
	RegisterInt(vm)
	RegisterFloat(vm)
	RegisterBoolean(vm)
	RegisterString(vm)
	RegisterCpx(vm)
	RegisterDblock(vm)
	RegisterCALL(vm)
	RegisterSep(vm)
	RegisterUnix(vm)
}
