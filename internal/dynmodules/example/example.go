package main

import "C"
import (
  vmmod "github.com/vulogov/Bund2/internal/vm"
)

func FourtyTwo(vm *vmmod.VM) (*vmmod.Elem, error) {
  return &vmmod.Elem{Type: "str", Value:"42"}, nil
}


func InitModule(vm *vmmod.VM)  {
	vm.Debug("Loading dynamic module example.so")
  vm.AddGen("FourtyTwo", FourtyTwo)
}
