package vm

type OpcodeParserFun func(v *VM, args ...interface{})
type OpcodeInLambdaFun func(v *VM, args ...interface{}) (*Elem, error)
type OpcodeEvalFun func(v *VM, args ...interface{}) (*Elem, error)
type OpcodeGenerationFun func(v *VM, args ...interface{}) error
type OpcodeExecFun func(v *VM, args ...interface{}) (*Elem, error)

type OPCODE struct {
	Name     string
	InParser OpcodeParserFun
	InLambda OpcodeInLambdaFun
	InEval   OpcodeEvalFun
	InGen    OpcodeGenerationFun
	InExec   OpcodeExecFun
}

func (vm *VM) RegisterOpcodes() {
	vm.Debug("Registering VM opcodes")
	InitOpcodeBlock(vm)
	InitOpcodeNs(vm)
	InitOpcodeExitNs(vm)
	InitOpcodeInteger(vm)
	InitOpcodeFloat(vm)
	InitOpcodeBoolean(vm)
	InitOpcodeString(vm)
	InitOpcodeComplex(vm)
	InitOpcodeDBlock(vm)
	InitOpcodeCall(vm)
	InitOpcodeRCall(vm)
	InitOpcodeSeparate(vm)

}

func (vm *VM) RegisterOpcode(t string, ip OpcodeParserFun, il OpcodeInLambdaFun, ef OpcodeEvalFun, ex OpcodeGenerationFun, im OpcodeExecFun) bool {
	if _, ok := vm.Opcodes.Load(t); ok {
		return true
	}
	vm.Opcodes.Store(t, &OPCODE{Name: t, InParser: ip, InLambda: il, InEval: ef, InGen: ex, InExec: im})
	vm.Debug("Register BUND opcode: %v", t)
	return true
}

func (vm *VM) Opcode(t string) *OPCODE {
	if res, ok := vm.Opcodes.Load(t); ok {
		return res.(*OPCODE)
	}
	vm.Fatal("BUND do not have opcode: %v", t)
	return nil
}
