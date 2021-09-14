package vm

import (
	"context"
	"strings"

	"github.com/rocketlaunchr/dataframe-go"
	"github.com/rocketlaunchr/dataframe-go/imports"
)

func DfFactory(vm *VM) *Elem {
	return &Elem{Type: "df", Value: dataframe.NewDataFrame}
}

func DfToString(vm *VM, e *Elem) string {
	if e.Type == "df" {
		return e.Value.(*dataframe.DataFrame).Table()
	}
	vm.Error("trying to convert a unix and it is not a unix: %v %T", e.Type, e.Value)
	return "<?>"
}

func DfFromString(vm *VM, d string) *Elem {
	res, err := imports.LoadFromCSV(context.Background(), strings.NewReader(d))
	vm.OnError(err, "Error creating DATAFRAME")
	return &Elem{Type: "df", Value: res}
}

func DfCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	return IDK
}

func DfDup(vm *VM, e *Elem) *Elem {
	if e.Type != "df" {
		return nil
	}
	return DfFactory(vm)
}

func RegisterDf(vm *VM) {
	vm.RegisterType("df", DfFactory, DfToString, DfFromString, DfCompare, DfDup)
}
