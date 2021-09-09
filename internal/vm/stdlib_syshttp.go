package vm

import (
	"fmt"

	"net/url"

	"github.com/levigross/grequests"
)

func HttpSetProxyElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e.Type != "http" {
		if e1.Type != "str" {
			fmt.Errorf("http/Set/Proxy expects to have a string object: %v", e.Type)
		}
		u, err := url.Parse(e1.Value.(string))
		vm.OnError(err, "Error parsing URL")
		h := e.Value.(*HTTP)
		h.RO.Proxies["http"] = u
		h.RO.Proxies["https"] = u
		h.RO.Proxies["ftp"] = u
		return e, nil
	}
	return nil, fmt.Errorf("http/Set/Proxy expects to have a http object: %v", e.Type)
}

func HttpSetAgentElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e.Type != "http" {
		if e1.Type != "str" {
			fmt.Errorf("http/Set/Agent expects to have a string object: %v", e.Type)
		}
		h := e.Value.(*HTTP)
		h.RO.UserAgent = e1.Value.(string)
		return e, nil
	}
	return nil, fmt.Errorf("http/Set/Proxy expects to have a http object: %v", e.Type)
}

func HttpSetHeadersElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e.Type != "http" {
		if e1.Type != "dblock" {
			fmt.Errorf("http/Set/Headers expects to have a dblock object: %v", e.Type)
		}
		hdr, err := Block2Dict(vm, e1)
		vm.OnError(err, "Error in getting data from DBLOCK")
		h := e.Value.(*HTTP)
		h.RO.Headers = *hdr
		return e, nil
	}
	return nil, fmt.Errorf("http/Set/Headers expects to have a http object: %v", e.Type)
}

func HttpSetParamsElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e.Type != "http" {
		if e1.Type != "dblock" {
			fmt.Errorf("http/Set/Params expects to have a dblock object: %v", e.Type)
		}
		hdr, err := Block2Dict(vm, e1)
		vm.OnError(err, "Error in getting data from DBLOCK")
		h := e.Value.(*HTTP)
		h.RO.Params = *hdr
		return e, nil
	}
	return nil, fmt.Errorf("http/Set/Params expects to have a http object: %v", e.Type)
}

func HttpSetDataElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e.Type != "http" {
		if e1.Type != "dblock" {
			fmt.Errorf("http/Set/Data expects to have a dblock object: %v", e.Type)
		}
		hdr, err := Block2Dict(vm, e1)
		vm.OnError(err, "Error in getting data from DBLOCK")
		h := e.Value.(*HTTP)
		h.RO.Data = *hdr
		return e, nil
	}
	return nil, fmt.Errorf("http/Set/Data expects to have a http object: %v", e.Type)
}

func HttpGetElement(vm *VM, e *Elem) (*Elem, error) {
	switch e.Type {
	case "str":
		ro := &grequests.RequestOptions{
			InsecureSkipVerify: true,
			IsAjax:             false,
		}
		resp, err := grequests.Get(e.Value.(string), ro)
		vm.OnError(err, "Error in GET(%v)", e.Value.(string))
		return &Elem{Type: "str", Value: resp.String()}, nil
	case "http":
		h := e.Value.(*HTTP)
		resp, err := grequests.Get(h.URL, h.RO)
		vm.OnError(err, "Error in GET(%v)", h.URL)
		return &Elem{Type: "str", Value: resp.String()}, nil
	}
	return nil, fmt.Errorf("http/Get expects to have a string with proper URL: %v", e.Type)
}

func HttpPostElement(vm *VM, e *Elem) (*Elem, error) {
	switch e.Type {
	case "str":
		ro := &grequests.RequestOptions{
			InsecureSkipVerify: true,
			IsAjax:             false,
		}
		resp, err := grequests.Post(e.Value.(string), ro)
		vm.OnError(err, "Error in GET(%v)", e.Value.(string))
		return &Elem{Type: "str", Value: resp.String()}, nil
	case "http":
		h := e.Value.(*HTTP)
		resp, err := grequests.Post(h.URL, h.RO)
		vm.OnError(err, "Error in GET(%v)", h.URL)
		return &Elem{Type: "str", Value: resp.String()}, nil
	}
	return nil, fmt.Errorf("http/Get expects to have a string with proper URL: %v", e.Type)
}

func InitHttpFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitHttpFunctions() reached")
	vm.AddFunction("http/Get", HttpGetElement)
	vm.AddFunction("http", HttpGetElement)
	vm.AddFunction("http/Post", HttpPostElement)
	vm.AddOperator("http/Set/Proxy", HttpSetProxyElement)
	vm.AddOperator("http/Set/Agent", HttpSetAgentElement)
	vm.AddOperator("http/Set/Headers", HttpSetHeadersElement)
	vm.AddOperator("http/Set/Params", HttpSetParamsElement)
	vm.AddOperator("http/Set/Data", HttpSetDataElement)
}
