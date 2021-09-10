package vm

import (
	"fmt"
	"strings"

	"github.com/antchfx/htmlquery"
	"golang.org/x/net/html"
)

func HtmlFactory(vm *VM) *Elem {
	res, err := htmlquery.Parse(strings.NewReader(""))
	vm.OnError(err, "Error parsing HTML document")
	return &Elem{Type: "html", Value: res}
}

func HtmlToString(vm *VM, e *Elem) string {
	if e.Type == "html" {
		return fmt.Sprintf("%v", htmlquery.OutputHTML(e.Value.(*html.Node), false))
	}
	vm.Error("trying to convert a html and it is not a html: %v %T", e.Type, e.Value)
	return "<?>"
}

func HtmlFromString(vm *VM, d string) *Elem {
	res, err := htmlquery.Parse(strings.NewReader(d))
	vm.OnError(err, "Error parsing HTML document")
	return &Elem{Type: "html", Value: res}
}

func HtmlCompare(vm *VM, e1 *Elem, e2 *Elem) int {
	return IDK
}

func HtmlDup(vm *VM, e *Elem) *Elem {
	if e.Type == "html" {
		res := HtmlFactory(vm)
		return res
	}
	return nil
}

func RegisterHtml(vm *VM) {
	vm.RegisterType("html", HtmlFactory, HtmlToString, HtmlFromString, HtmlCompare, HtmlDup)
}
