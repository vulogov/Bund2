package vm

import (
	"fmt"

	vfs "github.com/c2fo/vfs/v5"
)

var bufSize = 4194304 // Default buffer size is 4Mb

func VfsFileElement(vm *VM, e *Elem) (*Elem, error) {
	if e.Type == "str" {
		eh, err := vm.GetType("file")
		vm.OnError(err, "Error in fs/File")
		res := eh.FromString(vm, e.Value.(string))
		return res, nil
	}
	return nil, fmt.Errorf("Expecting to have a str value with URL, but have this: %v", e.Type)
}

func VfsFileExistsElement(vm *VM, e *Elem) (*Elem, error) {
	if e.Type == "file" && e.Value != nil {
		f := e.Value.(vfs.File)
		vm.Put(e)
		res, err := f.Exists()
		if err != nil {
			return nil, err
		}
		return &Elem{Type: "bool", Value: res}, nil
	}
	return nil, fmt.Errorf("Expecting to have a str value with URL, but have this: %v", e.Type)
}

func VfsFileReadElement(vm *VM, e *Elem) (*Elem, error) {
	var res string
	if e.Type == "file" && e.Value != nil {
		f := e.Value.(vfs.File)
		vm.Put(e)
		buf := make([]byte, bufSize)
		for {
			n, err := f.Read(buf)
			if err != nil || n == 0 {
				break
			}
			if n > 0 {
				res += string(buf)
			}
		}
		return &Elem{Type: "str", Value: res}, nil
	}
	return nil, fmt.Errorf("Expecting to have a file value, but have this: %v", e.Type)
}

func VfsFileWriteElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e.Type == "file" && e.Value != nil {
		eh, err := vm.GetType(e1.Type)
		vm.OnError(err, "Error in fs/File/Write")
		buf := eh.ToString(vm, e1)
		_, err = e.Value.(vfs.File).Write([]byte(buf))
		vm.OnError(err, "Error in fs/File/Write during the write operation")
		return e, nil
	}
	return nil, fmt.Errorf("Expecting to have a file value, but have this: %v", e.Type)
}

func VfsFileCloseElement(vm *VM, e *Elem) (*Elem, error) {
	if e.Type == "file" && e.Value != nil {
		err := e.Value.(vfs.File).Close()
		vm.OnError(err, "Error in fs/File/Close")
		return e, nil
	}
	return nil, fmt.Errorf("Expecting to have a file value, but have this: %v", e.Type)
}

func InitVFSFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitVFSFunctions() reached")
	vm.AddFunction("fs/File", VfsFileElement)
	vm.AddFunction("fs/File/Exists", VfsFileExistsElement)
	vm.AddOperator("fs/File/Write", VfsFileWriteElement)
	vm.AddFunction("fs/File/Read", VfsFileReadElement)
	vm.AddFunction("fs/File/Close", VfsFileCloseElement)
}
