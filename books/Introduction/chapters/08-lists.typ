#import "../design.typ": *
#chapter(number: 8, title: "Lists")

#dropcap("T")he _Osprey_'s herring stands on the quay in a row: twenty-five
barrels, end to end, in the order they came out of the hold. The clerk walks
along the row with a tally stick and touches each one. He does not need to pile
them up to count them, and he could tell you which is the seventh.

A stack is a pile. A _list_ is a row. Chapter 6 used one to give `loop`
something to walk along. This chapter is about the list itself: a single value
that holds many, in an order, and the words that work on the whole of it at
once.

#plate(7, "07-barrel-row.png", "Things in a row, in an order, counted from one end.")

#section("One value that holds many")

Square brackets make a list of whatever is written between them.

#session("ch08-list", caption: "Two lists")

`.s` shows one value on the stack, not three. That is the point of a list. The
three cargoes travel together, can be handed to a word together, and take up
one place on the stack however many there are. Bund prints a list with `::`
after each element, which is its own way of writing them and nothing you need
to type.

#term("List")[
  A row of values held as one value. The elements keep the order they were
  written in. They may be of any kind, mixed, and may be lists themselves.
]

#section("How long, the first, and the rest")

`len`, from the last chapter, counts the elements of a list as it counted the
characters of a string. And as it did then, it leaves the list where it was.

#session("ch08-len", caption: "len looks and does not take")

Two small words take a list apart from the front. `car` gives the first
element. `cdr` gives the list without its first element.

#session("ch08-carcdr", caption: "The first, and the rest")

`cdr car` is "the first of the rest", which is the second. The rest of a list
of one is the empty list, `[]`.

#callout(label: "For the curious")[
  `car` and `cdr` are the oldest words in this book. They come from Lisp, in
  the 1950s, where they were the names of two parts of a machine word on
  the computer Lisp was first built for. The machine is long gone and the names
  have outlived it. Say "car" and "could-er".
]

#words((
  ("len", "( list -- list n )", [The number of elements. Leaves the list.]),
  ("car", "( list -- x )", [The first element.]),
  ("cdr", "( list -- list )", [The list without its first element.]),
  ("list", "( -- list )", [Put down an empty list.]),
))

Asking for the first element of an empty list is an error, as taking from an
empty stack is.

#session("ch08-empty", caption: "Nothing in the row")

#section("A new list from an old one")

`loop` runs a lambda for each element and keeps nothing: whatever the lambda
does with the element is its own business. `map` runs a lambda for each element
and _collects what each run leaves_ into a new list.

#session("ch08-map", caption: "Every cargo reckoned, every cargo judged")

One list of cargoes went in. A list of dues came out, in the same order, and on
the second line a list of sizes. Nothing was printed and no running total was
kept: `map` turned one row into another.

#piles(width: 30mm,
  ("the list", ("[ 40 12 25 ]",)),
  ("{ dues } map", ("[ 140 56 95 ]",)),
  note: "map: the same lambda applied to each element, the answers kept in order.",
)

The lambda given to `map` should have the effect `( x -- y )`: one value in,
one value out. `dues` and `size` both do, which is why they fit without any
wrapping. This is the reward for writing words that leave their answer on the
stack.

#words((
  ("loop", "( list lambda -- )", [Run the lambda for each element. Keep nothing.]),
  ("map", "( list lambda -- list )", [Run the lambda for each element and collect
  what it leaves.]),
))

#section("Adding a list up")

Chapter 6 kept a running total under a loop by putting 0 down first. With the
list already on the stack, the 0 has to go _underneath_ it, and `swap` does
that. The whole pattern makes a word.

#session("ch08-total", caption: "total")

Read `total` with a list on the stack. `0` goes on top of the list. `swap` puts
it underneath. `{ + }` goes on top, and `loop` adds each element to what is
underneath.

The last line is the whole of the day's reckoning, and it is worth reading
aloud: _the cargoes, each one's dues, the total._ Three steps, three words,
left to right, and no variable anywhere. Each word hands its result to the
next. That is what people mean when they say a concatenative program reads like
a pipeline.

#section("Putting in order")

`sort` puts a list in order: numbers from the smallest, strings by the
alphabet. `unique` removes repeats.

#session("ch08-sort", caption: "sort and unique")

Each picks up a list and leaves a new one in its place.

#section("Joining two lists")

`++` joins two lists into one. And the top speaks first.

#session("ch08-join", caption: "Which list comes first?")

`[ 40 12 ] [ 25 ] ++` puts the 25 in front, because `[ 25 ]` was on top. To add
to the _end_ of a list, the list must be on top and the new piece underneath.
It is the rule you know from joining strings, and the cure is the same: write
the end first, or `swap`.

#words((
  ("sort", "( list -- list )", [The elements in order.]),
  ("unique", "( list -- list )", [The elements without repeats.]),
  ("++", "( a b -- list )", [Join two lists: b's elements first, then a's.]),
))

#section("Between the row and the pile")

Sometimes it is easier to work on the elements where every other word can
reach them, on the stack. `unfold` empties a list onto the stack. `fold` goes
the other way, and sweeps the stack into a list.

#session("ch08-unfold", caption: "A list onto the stack")

After `unfold` the three numbers are three values, the last element on top, and
`*+` from Chapter 3 adds them.

#session("ch08-fold", caption: "The stack into a list")

Look at the order. `fold` picks the values up from the top, as every word does,
and the first one it picks up becomes the first element. The stack held 40, 12,
25 with 25 on top, and the list begins with 25.

#warn(label: "fold takes the whole stack")[
  `fold` does not stop at the values you were thinking of. It takes
  _everything_ on the stack. If there was something underneath that you wanted
  to keep, it is in the list now. Chapter 11 shows the proper remedy: do this
  kind of work on a stack of its own.
]

#words((
  ("unfold", "( list -- ... )", [Put every element on the stack, the last on top.]),
  ("fold", "( ... -- list )", [Sweep the whole stack into a list, top first.]),
))

#section("What the brackets will not do")

One thing about `[ ]` surprises everyone once.

#session("ch08-literal", caption: "Words inside brackets are kept, not run")

`[ 1 2 + ]` is not a list holding 3. It is a list of three things: 1, 2, and
the word `+`, which Bund shows as `F(+)`. Brackets _collect_ what is written
between them. They do not run it. Even `true` is collected as a word and not
as a boolean.

So a list written with brackets can hold numbers and strings, and other lists,
written out plainly. If the elements have to be _reckoned_, reckon them first
and collect them afterwards, with `map`, or on the stack with `fold`.

#section("Lists inside lists")

An element of a list may be a list.

#session("ch08-mixed", caption: "A ship, its cargo, and what the cargo is")

That list describes the _Marta_ on a day she carried two kinds of goods. It
works. It is also a bad idea. What is the second element? You have to remember
that the name comes first, the number second and the goods third, and the next
person to read `cdr cdr car` has to work it out.

When the parts of a thing have _names_, a row is the wrong shape for it. The
next chapter gives each part its name.

#recap((
  [`[ ... ]` makes a *list*: many values held as one, in order.],
  [`len` counts the elements and leaves the list. `car` is the first element and
  `cdr` is the rest.],
  [`map` applies a lambda to each element and collects the answers into a new
  list. The lambda should take one value and leave one.],
  [`:total { 0 swap { + } loop } register` adds a list up, and
  `[ 40 12 25 ] { dues } map total` is the day's reckoning: 291.],
  [`sort` orders, `unique` removes repeats, `++` joins, top first.],
  [`unfold` puts a list's elements on the stack; `fold` sweeps the _whole_ stack
  into a list, top first.],
  [Words written inside `[ ]` are collected, not run.],
))

#problems((
  [Make the list of the three ships' names and print how many there are, leaving
  nothing on the stack.],
  [Using `map`, turn `[ 40 12 25 ]` into a list of `true` and `false`: which
  cargoes are large?],
  [Define `count ( list -- n )`, which leaves only the number of elements.],
  [Define `largest ( list -- x )` using `sort` and what you know about `unfold`.
  Start from an empty stack, and say why that matters.],
  [The list `[ 25 12 40 ]` is in the order the ships left. Make it the order they
  arrived, `[ 40 12 25 ]`, using `unfold` and `fold`.],
  [Define `average ( list -- n )` in whole pennies, using `total` and `len`.
  The average of the three dues is 97.],
))
