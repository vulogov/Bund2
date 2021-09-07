package vm

import (
	"fmt"
	"math/big"
	"os"
	"plugin"
	"time"

	"github.com/gammazero/deque"
)

func PassthrougElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("PASSTHROUGH: %v", e.Type)
	return e, nil
}

func RotateFrontElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Put(e)
	q := vm.Current
	q.Rotate(1)
	return nil, nil
}

func RotateBackElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Put(e)
	q := vm.Current
	q.Rotate(-1)
	return nil, nil
}

func PrintStack(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("PRINTSTACK: %v", e.Type)
	vm.Put(e)
	res := fmt.Sprintf("(%v)[ ", vm.CurrentNS.Name)
	for i := vm.Current.Len() - 1; i >= 0; i-- {
		_e := vm.Current.At(i)
		eh, err := vm.GetType(_e.(*Elem).Type)
		if err != nil {
			vm.Error("Can not get type: %v", _e.(*Elem).Type)
			continue
		}
		res = res + eh.ToString(vm, _e.(*Elem))
		res = res + ", "
	}
	res = res + " ]"
	fmt.Println(res)
	return nil, nil
}

func DropElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("DROP: %v", e.Type)
	return nil, nil
}

func DropOppositeElement(vm *VM, e *Elem) (*Elem, error) {
	if vm.Current.Len() > 0 {
		vm.Debug("DROP OPPOSITE: %v", e.Type)
		vm.TakeOpposite()
	}
	return e, nil
}

func DupElement(vm *VM, e *Elem) (*Elem, error) {
	eh, err := vm.GetType(e.Type)
	if err != nil {
		return nil, err
	}
	vm.Put(e)
	res := eh.Duplicate(vm, e)
	vm.Debug("DUP: %v %v", e.Type, eh.ToString(vm, e))
	return res, nil
}

func ExecuteElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("EXECUTE: %v", e.Type)
	switch e.Type {
	case "CALL":
		res, err := vm.Exec(e)
		return res, err
	case "unix":
		res, err := vm.Opcode("unix").InEval(vm, e.Value.(string), e.Prefun, e.Functor)
		vm.OnError(err, "Error in executing on UNIX element")
		return res, nil
	}
	return nil, fmt.Errorf("Request to EXECUTE on wrong context: %v", e.Type)
}

func SetAlias(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if vm.Current.Len() < 1 {
		return nil, fmt.Errorf("Namespace stack is too shallow for ALIAS operation: %v:%v", vm.CurrentNS.Name, vm.Current.Len())
	}
	if e1.Type != "str" {
		return nil, fmt.Errorf("String value is required for an ALIAS operation: %v", e1.Type)
	}
	vm.CurrentNS.SetAlias(e1.Value.(string), e2)
	return nil, nil
}

func GetAlias(vm *VM, e *Elem) (*Elem, error) {
	if e.Type != "str" {
		return nil, fmt.Errorf("String value is required for an ALIAS operation: %v", e.Type)
	}
	val := vm.CurrentNS.GetAlias(e.Value.(string))
	if val == nil {
		vm.Error("There is no alias for: %v", e.Value.(string))
		return nil, nil
	}
	return val.(*Elem), nil
}

func SleepFun(vm *VM, e *Elem) (*Elem, error) {
	var d int
	switch e.Type {
	case "int":
		res := big.NewInt(0)
		d = int(res.Mul(e.Value.(*big.Int), big.NewInt(1000000000)).Int64())
	case "flt":
		res := big.NewFloat(0.0)
		_d, _ := res.Mul(e.Value.(*big.Float), big.NewFloat(1000000000.0)).Int64()
		d = int(_d)
	default:
		return nil, fmt.Errorf("Parameter for 'sleep' is not float or int: %v", e.Type)
	}
	vm.Debug("Sleeping for a %v 𝝶-seconds", d)
	time.Sleep(time.Duration(d) * time.Nanosecond)
	return e, nil
}

func ExitFun(vm *VM, e *Elem) (*Elem, error) {
	if e.Type != "int" {
		return nil, fmt.Errorf("Exit function require a number in stack: %v", e.Type)
	}
	res := int(e.Value.(*big.Int).Int64())
	vm.Debug("EXIT is called with %v", res)
	os.Exit(res)
	return nil, nil
}

func NameFun(vm *VM) (*Elem, error) {
	return &Elem{Type: "str", Value: vm.CurrentNS.Name}, nil
}

func TypeFun(vm *VM, e *Elem) (*Elem, error) {
	vm.Put(e)
	return &Elem{Type: "str", Value: e.Type}, nil
}

func SeqFun(vm *VM, e *Elem) (*Elem, error) {
	if e.Type == "int" {
		eh, err := vm.GetType("dblock")
		vm.OnError(err, "Error in 'seq'")
		res := eh.Factory(vm)
		q := res.Value.(*deque.Deque)
		c := int(e.Value.(*big.Int).Int64())
		for i := 0; i < c; i++ {
			q.PushBack(&Elem{Type: "int", Value: big.NewInt(int64(i))})
		}
		return res, nil
	}
	return nil, fmt.Errorf("Seq function require a number in stack: %v", e.Type)
}

func NFun(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type == "int" && e2.Type == "int" {
		eh, err := vm.GetType("dblock")
		vm.OnError(err, "Error in 'N'")
		res := eh.Factory(vm)
		q := res.Value.(*deque.Deque)
		c := int(e1.Value.(*big.Int).Int64())
		n := int(e2.Value.(*big.Int).Int64())
		for i := c; i < n; i++ {
			q.PushBack(&Elem{Type: "int", Value: big.NewInt(int64(i))})
		}
		return res, nil
	}
	return nil, fmt.Errorf("N function require a two numbers in stack: %v %v", e1.Type, e2.Type)
}

func UseFun(v *VM, e1 *Elem) (*Elem, error) {
	if e1.Type == "str" {
		p, err := plugin.Open(e1.Value.(string))
		v.OnError(err, "Error in 'use' function")
		symbol, err := p.Lookup("InitModule")
		v.OnError(err, "Error in 'use' function")
		plug, ok := symbol.(func(*VM))
		if ok {
			plug(v)
		} else {
			return &Elem{Type: "bool", Value: false}, nil
		}
		return &Elem{Type: "bool", Value: true}, nil
	}
	return nil, fmt.Errorf("use function expects string, not this: %v", e1.Type)
}

func InitSystemFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitSystemFunctions() reached")
	vm.AddFunction("passthrough", PassthrougElement)
	vm.AddFunction("dumpstack", PrintStack)
	vm.AddFunction(",", DropElement)
	vm.AddFunction("_,", DropElement)
	vm.AddFunction(",_", DropOppositeElement)
	vm.AddFunction("^", DupElement)
	vm.AddFunction("!", ExecuteElement)
	vm.AddFunction("exec", ExecuteElement)
	vm.AddOperator("setAlias", SetAlias)
	vm.AddOperator("≡", SetAlias)
	vm.AddFunction("alias", GetAlias)
	vm.AddFunction("sleep", SleepFun)
	vm.AddGen("name", NameFun)
	vm.AddFunction("type", TypeFun)
	vm.AddFunction("seq", SeqFun)
	vm.AddOperator("N", NFun)
	vm.AddFunction("exit", ExitFun)
	vm.AddFunction("rotateFront", RotateFrontElement)
	vm.AddFunction("rotate", RotateFrontElement)
	vm.AddFunction("rotateBack", RotateBackElement)
	vm.AddFunction("use", UseFun)
}
