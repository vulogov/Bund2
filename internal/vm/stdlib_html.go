package vm

import (
	"fmt"

	"github.com/antchfx/htmlquery"
	"github.com/gammazero/deque"
	"golang.org/x/net/html"
)

func HtmlElement(vm *VM, e *Elem) (*Elem, error) {
	if e.Type != "str" {
		return nil, fmt.Errorf("Source data for HTML construction not found")
	}
	eh, err := vm.GetType("html")
	vm.OnError(err, "Error in HTML contruction")
	res := eh.FromString(vm, e.Value.(string))
	return res, nil
}

func HtmlFindElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e.Type != "html" && e1.Type != "str" {
		return nil, fmt.Errorf("html/Find expecting html object and string ")
	}
	eh, err := vm.GetType("dblock")
	vm.OnError(err, "Error in html/Query")
	res := eh.Factory(vm)
	q := res.Value.(*deque.Deque)
	list, err := htmlquery.QueryAll(e.Value.(*html.Node), e1.Value.(string))
	vm.OnError(err, "Error in html/Query")
	for _, v := range list {
		q.PushBack(&Elem{Type: "html", Value: v})
	}
	return res, nil
}

func HtmlGetElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e.Type != "html" && e1.Type != "str" {
		return nil, fmt.Errorf("html/Get expecting html object and string ")
	}
	d := htmlquery.FindOne(e.Value.(*html.Node), e1.Value.(string))
	return &Elem{Type: "html", Value: d}, nil
}

func HtmlAttrGetElement(vm *VM, e *Elem, e1 *Elem) (*Elem, error) {
	if e.Type != "html" && e1.Type != "str" {
		return nil, fmt.Errorf("html/Attr/Get expecting html object and string ")
	}
	d := htmlquery.SelectAttr(e.Value.(*html.Node), e1.Value.(string))
	return &Elem{Type: "str", Value: d}, nil
}

func InitHtmlFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitHtmlFunctions() reached")
	vm.AddFunction("html", HtmlElement)
	vm.AddOperator("html/Query", HtmlFindElement)
	vm.AddOperator("html/Get", HtmlGetElement)
	vm.AddOperator("html/Attr/Get", HtmlAttrGetElement)
}
