package vm

import (
	"fmt"
	"math/big"

	"gonum.org/v1/gonum/stat"
)

func getMax(a, b float64) float64 {
	if a > b {
		return a
	}
	return b
}

func getMin(a, b float64) float64 {
	if a < b {
		return a
	}
	return b
}

func interval_insert(intervals []Interval, newInterval Interval) []Interval {
	if len(intervals) == 0 {
		return []Interval{newInterval}
	}
	ans := make([]Interval, 0)
	for i, interval := range intervals {
		if newInterval.Start <= interval.End && newInterval.Start >= interval.Start {
			intervals[i].Start = getMin(newInterval.Start, interval.Start)
			intervals[i].End = getMax(newInterval.End, interval.End)
			break
		} else if newInterval.End >= interval.Start && newInterval.End <= interval.End {
			intervals[i].Start = getMin(newInterval.Start, interval.Start)
			intervals[i].End = getMax(newInterval.End, interval.End)
			break
		} else if newInterval.Start <= interval.Start && newInterval.End >= interval.End {
			intervals[i].Start = getMin(newInterval.Start, interval.Start)
			intervals[i].End = getMax(newInterval.End, interval.End)
			break
		} else if newInterval.Start < interval.Start && newInterval.End < interval.Start {
			intervals = append(intervals[:i], append([]Interval{newInterval}, intervals[i:]...)...)
			break
		} else if i == len(intervals)-1 && newInterval.Start > interval.End {
			intervals = append(intervals, newInterval)
			break
		}
	}

	s := intervals[0].Start
	e := intervals[0].End
	for _, interval := range intervals[1:] {
		if interval.Start > e {
			ans = append(ans, Interval{s, e})
			s = interval.Start
			e = interval.End
		} else if interval.Start <= e {
			e = getMax(interval.End, e)
		}
	}
	ans = append(ans, Interval{s, e})
	return ans
}

func IntervalElement(vm *VM) (*Elem, error) {
	eh, err := vm.GetType("interval")
	if err != nil {
		vm.OnError(err, "Error in I functor")
		return nil, err
	}
	res := eh.Factory(vm)
	return res, nil
}

func intervalElementAdd(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	intrvl := e1.Value.([]Interval)
	if BlockLen(e2) != 2 {
		return nil, fmt.Errorf("I/Add have incorrect data arity")
	}
	_start, err := BlockAt(e2, "flt", 0)
	vm.OnError(err, "Error in I/Add")
	_end, err := BlockAt(e2, "flt", 1)
	vm.OnError(err, "Error in I/Add")
	start, _ := _start.(*big.Float).Float64()
	end, _ := _end.(*big.Float).Float64()
	if start > end {
		return nil, fmt.Errorf("I/Add can not use data where start interval larger than end %v %v", start, end)
	}
	vm.Debug("Adding new interval start: %v end: %v", start, end)
	res := interval_insert(intrvl, Interval{Start: start, End: end})
	return &Elem{Type: "interval", Value: res}, nil
}

func IntervalElementAdd(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type == "interval" && e2.Type == "dblock" {
		return intervalElementAdd(vm, e1, e2)
	} else if e2.Type == "interval" && e1.Type == "dblock" {
		return intervalElementAdd(vm, e2, e1)
	} else {
		return nil, fmt.Errorf("I/Add expects interval and (* ): %v %v", e1.Type, e2.Type)
	}
}

func intervalElementStdev(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	var i int64
	data := make([]float64, 0)
	intrvl := e1.Value.([]Interval)
	for i = 0; i < BlockLen(e2); i++ {
		v, err := BlockAt(e2, "flt", i)
		if err == nil {
			val, _ := v.(*big.Float).Float64()
			data = append(data, val)
		}
	}
	ddev := stat.StdDev(data, nil)
	res := intrvl
	for j := 0; j < len(data); j++ {
		start := data[j] - ddev
		end := data[j] + ddev
		res = interval_insert(res, Interval{Start: start, End: end})
	}
	return &Elem{Type: "interval", Value: res}, nil
}

func IntervalElementStdDev(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type == "interval" && e2.Type == "dblock" {
		return intervalElementStdev(vm, e1, e2)
	} else if e2.Type == "interval" && e1.Type == "dblock" {
		return intervalElementStdev(vm, e2, e1)
	} else {
		return nil, fmt.Errorf("I/StdDev expects interval and (* ): %v %v", e1.Type, e2.Type)
	}
}

func intervalElementRange(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	var i int64
	if BlockLen(e2) < 3 {
		return nil, fmt.Errorf("I/Approx data is too shallow")
	}
	data := make([]float64, 0)
	intrvl := e1.Value.([]Interval)
	for i = 1; i < BlockLen(e2); i++ {
		v, err := BlockAt(e2, "flt", i)
		if err == nil {
			val, _ := v.(*big.Float).Float64()
			data = append(data, val)
		}
	}
	c, err := BlockAt(e2, "flt", 0)
	vm.OnError(err, "Error in I/Coeff")
	coeff, _ := c.(*big.Float).Float64()
	res := intrvl
	for j := 0; j < len(data); j++ {
		start := data[j] - (data[j] * coeff)
		end := data[j] + (data[j] * coeff)
		res = interval_insert(res, Interval{Start: start, End: end})
	}
	return &Elem{Type: "interval", Value: res}, nil
}

func IntervalElementRange(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	if e1.Type == "interval" && e2.Type == "dblock" {
		return intervalElementRange(vm, e1, e2)
	} else if e2.Type == "interval" && e1.Type == "dblock" {
		return intervalElementRange(vm, e2, e1)
	} else {
		return nil, fmt.Errorf("I/Approx expects interval and (* ): %v %v", e1.Type, e2.Type)
	}
}

func IntervalElementTest(vm *VM, e1 *Elem, e2 *Elem) (*Elem, error) {
	var v float64
	if e1.Type == "interval" && (e2.Type == "flt" || e2.Type == "int") {
		vm.Put(e1)
		switch e2.Type {
		case "flt":
			v, _ = e2.Value.(*big.Float).Float64()
		case "int":
			v = float64(e2.Value.(*big.Int).Int64())
		}
		for i := 0; i < len(e1.Value.([]Interval)); i++ {
			intrvl := e1.Value.([]Interval)[i]
			if v >= intrvl.Start && v <= intrvl.End {
				return &Elem{Type: "bool", Value: true}, nil
			}
		}
		return &Elem{Type: "bool", Value: false}, nil
	}
	return nil, fmt.Errorf("I/Test expects interval and number: %v %v", e1.Type, e2.Type)
}

func InitIntervalFunctions(vm *VM) {
	vm.Debug("[ BUND ] bund.InitIntervalFunctions() reached")
	vm.AddGen("I", IntervalElement)
	vm.AddOperator("I/Add", IntervalElementAdd)
	vm.AddOperator("I/StdDev", IntervalElementStdDev)
	vm.AddOperator("I/Coeff", IntervalElementRange)
	vm.AddOperator("I/Test", IntervalElementTest)
}
