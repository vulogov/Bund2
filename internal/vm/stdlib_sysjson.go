package vm

import (
	"fmt"
	"math/big"

	"github.com/Jeffail/gabs/v2"
	"github.com/gammazero/deque"
)

func JsonArrayGetElement(vm *VM, e *Elem, d *Elem) (*Elem, error) {
	vm.Debug("json/Array/Get: %v", e.Type)
	if e.Type == "json" {
		if !vm.CanGet() {
			return nil, fmt.Errorf("Stack is too shallow for json/Array/Get")
		}
		if d.Type != "str" {
			return nil, fmt.Errorf("string is expected for json/Array/Set")
		}
		j := e.Value.(*gabs.Container)
		if !j.ExistsP(d.Value.(string)) {
			return nil, fmt.Errorf("Key %v not found in JSON", d.Value.(string))
		}
		vm.Put(e)
		eh, err := vm.GetType("dblock")
		vm.OnError(err, "Error in json/Array/Get")
		res := eh.Factory(vm)
		q := res.Value.(*deque.Deque)
		for _, child := range j.Path(d.Value.(string)).Children() {
			v := child.Data()
			switch v.(type) {
			case int:
				q.PushBack(&Elem{Type: "int", Value: big.NewInt(int64(v.(int)))})
			case int64:
				q.PushBack(&Elem{Type: "int", Value: big.NewInt(v.(int64))})
			case float32:
				q.PushBack(&Elem{Type: "flt", Value: big.NewFloat(float64(v.(float32)))})
			case float64:
				q.PushBack(&Elem{Type: "flt", Value: big.NewFloat(v.(float64))})
			case string:
				q.PushBack(&Elem{Type: "str", Value: string(v.(string))})
			default:
				return nil, fmt.Errorf("Unsupported data in JSON while in json/Value/Get")
			}
		}
		return res, nil
	}
	return e, nil
}

func JsonArrayAddElement(vm *VM, e *Elem, d *Elem) (*Elem, error) {
	vm.Debug("json/Array/Add: %v", e.Type)
	if e.Type == "json" {
		j := e.Value.(*gabs.Container)
		if d.Type != "dblock" {
			return nil, fmt.Errorf("(* ) is expected for json/Array/Add")
		}
		if BlockLen(d) == 0 {
			return nil, fmt.Errorf("(* ) is incorrect arity for json/Array/Add")
		}
		k, err := Block_At(d, int64(0))
		vm.OnError(err, "Error in json/Array/Add")
		switch key := k.(type) {
		case string:
			if !j.ExistsP(key) {
				j.ArrayP(key)
			}
			for i := 1; i < int(BlockLen(d)); i++ {
				v, err := Block_At(d, int64(i))
				vm.OnError(err, "Error in json/Array/Add")
				j.ArrayAppendP(v, key)
			}
		}
	}
	return e, nil
}

func JsonArrayNewElement(vm *VM, e *Elem, d *Elem) (*Elem, error) {
	vm.Debug("json/Array/New: %v", e.Type)
	if e.Type == "json" {
		j := e.Value.(*gabs.Container)
		if d.Type != "dblock" {
			return nil, fmt.Errorf("(* ) is expected for json/Array/New")
		}
		if BlockLen(d) == 0 {
			return nil, fmt.Errorf("(* ) is incorrect arity for json/Array/New")
		}
		for i := 0; i < int(BlockLen(d)); i++ {
			k, err := Block_At(d, int64(i))
			vm.OnError(err, "Error in json/Array/New")
			switch key := k.(type) {
			case string:
				j.ArrayP(key)
			}
		}
	}
	return e, nil
}

func JsonGetElement(vm *VM, e *Elem, d *Elem) (*Elem, error) {
	vm.Debug("json/Value/Get: %v", e.Type)
	if e.Type == "json" {
		j := e.Value.(*gabs.Container)
		if d.Type != "str" {
			return nil, fmt.Errorf("string is expected for json/Value/Set")
		}
		if !j.ExistsP(d.Value.(string)) {
			return nil, fmt.Errorf("Key %v not found in JSON", d.Value.(string))
		}
		v := j.Path(d.Value.(string)).Data()
		vm.Put(e)
		switch v.(type) {
		case int:
			return &Elem{Type: "int", Value: big.NewInt(int64(v.(int)))}, nil
		case int64:
			return &Elem{Type: "int", Value: big.NewInt(v.(int64))}, nil
		case float32:
			return &Elem{Type: "flt", Value: big.NewFloat(float64(v.(float32)))}, nil
		case float64:
			return &Elem{Type: "flt", Value: big.NewFloat(v.(float64))}, nil
		case string:
			return &Elem{Type: "str", Value: string(v.(string))}, nil
		}
		return nil, fmt.Errorf("Unsupported data in JSON while in json/Value/Get")
	}
	return e, nil
}

func JsonSetElement(vm *VM, e *Elem, d *Elem) (*Elem, error) {
	vm.Debug("json/Value/Set: %v", e.Type)
	if e.Type == "json" {
		j := e.Value.(*gabs.Container)
		if d.Type != "dblock" {
			return nil, fmt.Errorf("(* ) is expected for json/Value/Set")
		}
		if BlockLen(d) == 0 || (BlockLen(d)%2) != 0 {
			return nil, fmt.Errorf("(* ) is incorrect arity for json/Value/Set")
		}
		i := 0
		for {
			if i >= int(BlockLen(d)) {
				break
			}
			k, err := Block_At(d, int64(i))
			vm.OnError(err, "Error in json/Value/Set")
			v, err := Block_At(d, int64(i+1))
			vm.OnError(err, "Error in json/Value/Set")
			switch key := k.(type) {
			case string:
				j.SetP(v, key)
			}
			i = i + 2
		}
	}
	return e, nil
}

func JsonNewElement(vm *VM, e *Elem) (*Elem, error) {
	vm.Debug("json/New: %v", e.Type)
	if e.Type == "str" {
		eh, err := vm.GetType("json")
		vm.OnError(err, "error on json/New")
		res := eh.FromString(vm, e.Value.(string))
		return res, nil
	}
	return e, nil
}

func InitJsonFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitJsonFunctions() reached")
	vm.AddFunction("json/New", JsonNewElement)
	vm.AddOperator("json/Value/Set", JsonSetElement)
	vm.AddOperator("json/Value/Get", JsonGetElement)
	vm.AddOperator("json/Array/New", JsonArrayNewElement)
	vm.AddOperator("json/Array/Add", JsonArrayAddElement)
	vm.AddOperator("json/Array/Get", JsonArrayGetElement)
}
