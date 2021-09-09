package vm

import (
	"fmt"

	glob "github.com/ganbarodigital/go_glob"
)

func GlobPatternMatchOperator(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return EqOperator(v, e1, e2)
}

func matchshortprefix(v *VM, e1 *Elem, e2 *Elem) bool {
	g := glob.NewGlob(e1.Value.(string))
	_, res, err := g.MatchShortestPrefix(e2.Value.(string))
	if err != nil {
		v.OnError(err, "Error matching short prefix")
	}
	return res
}

func matchshortestsuffix(v *VM, e1 *Elem, e2 *Elem) bool {
	g := glob.NewGlob(e1.Value.(string))
	_, res, err := g.MatchShortestSuffix(e2.Value.(string))
	if err != nil {
		v.OnError(err, "Error matching shortest suffix")
	}
	return res
}

func matchlongestprefix(v *VM, e1 *Elem, e2 *Elem) bool {
	g := glob.NewGlob(e1.Value.(string))
	_, res, err := g.MatchLongestPrefix(e2.Value.(string))
	if err != nil {
		v.OnError(err, "Error matching shortest suffix")
	}
	return res
}

func matchlongestsuffix(v *VM, e1 *Elem, e2 *Elem) bool {
	g := glob.NewGlob(e1.Value.(string))
	_, res, err := g.MatchLongestSuffix(e2.Value.(string))
	if err != nil {
		v.OnError(err, "Error matching shortest suffix")
	}
	return res
}

func rungpo(v *VM, e1 *Elem, e2 *Elem, gpofun func(*VM, *Elem, *Elem) bool) (*Elem, error) {
	if e1.Type != "glob" {
		return nil, fmt.Errorf("Pattern for matching not found")
	}
	if e2.Type != "str" {
		return nil, fmt.Errorf("Pattern matching is done only with strings")
	}
	res := gpofun(v, e1, e2)
	if res == true {
		return &Elem{Type: "bool", Value: true}, nil
	} else {
		return &Elem{Type: "bool", Value: false}, nil
	}
}

func GlobPatternMatchPrefixOperator(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return rungpo(v, e1, e2, matchshortprefix)
}

func GlobPatternMatchSuffixOperator(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return rungpo(v, e1, e2, matchshortestsuffix)
}

func GlobPatternMatchLongestPrefixOperator(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return rungpo(v, e1, e2, matchlongestprefix)
}

func GlobPatternMatchLongestSuffixOperator(v *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	return rungpo(v, e1, e2, matchlongestsuffix)
}

func InitGPMOperators(vm *VM) {
	vm.Debug("[ BUND ] bund.InitGPMOperators() reached")
	vm.AddOperator("===", GlobPatternMatchOperator)
	vm.AddOperator("<===", GlobPatternMatchPrefixOperator)
	vm.AddOperator("===>", GlobPatternMatchSuffixOperator)
	vm.AddOperator("=<==", GlobPatternMatchLongestPrefixOperator)
	vm.AddOperator("==>=", GlobPatternMatchLongestSuffixOperator)
}
