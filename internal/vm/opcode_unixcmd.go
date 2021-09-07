package vm

import (
	"fmt"

	"github.com/commander-cli/cmd"
)

func UnixcmdParser(vm *VM, args ...interface{}) {
	vm.Info("UNIXCMD %v", args)
}

func UnixcmdEval(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("UNIXCMD: %v", args)
	if len(args) == 0 {
		return nil, fmt.Errorf("We do not know what to execute for UNIX cmd")
	}
	c := cmd.NewCommand(args[0].(string))
	err := c.Execute()
	vm.OnError(err, "Error evaluating UNIXCMD")
	return &Elem{Type: "str", Value: c.Stdout()}, nil
}

func UnixcmdLambda(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Debug("UNIXCMD(in lambda) %v", args)
	return &Elem{Type: "UNIXCMD", Value: args[0].(string)}, nil
}

func UnixcmdGen(vm *VM, args ...interface{}) error {
	vm.Fatal("UNIXCMD Import not implemented")
	return nil
}

func UnixcmdExec(vm *VM, args ...interface{}) (*Elem, error) {
	vm.Fatal("UNIXCMD Exec not implemented")
	return nil, nil
}

func InitOpcodeUnixcmd(vm *VM) {
	vm.RegisterOpcode("unix", UnixcmdParser, UnixcmdLambda, UnixcmdEval, UnixcmdGen, UnixcmdExec)
}
