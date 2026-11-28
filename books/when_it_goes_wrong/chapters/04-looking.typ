#import "../design.typ": *
#chapter(number: 4, title: "Looking")

#dropcap("B")efore anyone sends for the diver, somebody leans over the quay
and looks. Most faults in a small program are found the same way: by putting
a word into the program that shows what is there, running it, and taking the
word out again. Bund has a dozen such words. None of them stops the
program, and none changes the stack.

#plate("04b-over-the-quay.png", "Before anyone sends for the diver, somebody looks.")

#section("Where things are")

`.s` in the book's terminal shows one stack. `debug.stacks` shows them all.

#session("c4-stacks", caption: "Every stack, and the counting table")

Three lines. `@main` has two values and 40 on top; the star says it is the
current stack. `@quay` has one. The workbench, which the first book called
the counting table, has one. When a value has gone missing, this is the
first word to type: it is very often sitting on another stack.

#words((
  ("debug.stacks", "( -- )", [Every stack: its name, its depth and its top.
  The current one is starred.]),
))

#section("What a thing is")

A number and a string holding the digits of a number print alike. `type`
tells them apart.

#session("c4-type", caption: "The kind of a value")

`type` replaces a value with the name of its kind. `?type` is the question:
it takes a value and a kind's name, and leaves the value with a flag on top
of it, so it can go straight into an `if`.

#words((
  ("type", "( x -- name )", [The kind of a value: `Integer`, `String`,
  `Lambda` and so on.]),
  ("?type", "( x name -- x flag )", [Is the value of that kind?]),
))

#section("How a word was reached")

`debug.backtrace` prints the frames, as an error report does, but without
anything having failed. Put it in a word, and each time the word runs it
says who called it.

#piece("where.bund", ":due", rest: true, caption: "where.bund")

#session("c4-where", caption: "A word that says how it was reached")

Three frames, innermost first: `due`, called from `dues`, called from
`entry`. Then the program goes on and prints its 140. When a word is called
from several places and misbehaves in only one, this tells you which.

#words((
  ("debug.backtrace", "( -- )", [Print the frames now open, innermost
  first.]),
))

#section("The tables behind the program")

Bund keeps a table of every word. `debug.words` prints the part of it that
is yours.

#session("c4-words", caption: "The words a program defined")

Each word, where in the file it was registered, and its _generation_: how
many times that name has been defined. A `gen 2` here means a word was
defined twice, which is the fault of `clash.bund` and the subject of a
section in Chapter 8. The count of calls is kept only while a debugger is
attached, and Chapter 7 shows it filled in.

`debug.classes` does the same for classes: each with its parents, its
methods and its fields. A program with no classes of its own still has
Bund's built-in ones, and the list is long.

#words((
  ("debug.words", "( -- )", [The program's words: where each was
  registered, its generation, its calls.]),
  ("debug.classes", "( -- )", [Every class: parents, methods, fields.]),
))

#section("A value as Bund holds it")

Three words show more than you usually want.

#session("c4-display", caption: "The stack, value by value")

`debug.display_stack` prints the current stack as a table of raw values,
the form `--raw-values` gave in Chapter 1. `debug.display_workbench` does
the same for the counting table.

#session("c4-dump", caption: "The bytes of a value")

`debug.dump` goes one level lower and prints the bytes a value is made of.
`28` in the first row is forty, written in sixteens. The second is the four
letters of `salt`. You will want this the day a string that looks right
compares unequal, and has a space in it that no screen shows.

#words((
  ("debug.display_stack", "( -- )", [The current stack, as raw values.]),
  ("debug.display_workbench", "( -- )", [The counting table, likewise.]),
  ("debug.dump", "( x -- x )", [The bytes of the value on top.]),
))

#section("The machine")

Three more words look outward, at what the program is running in.

#session("c4-mem", caption: "Memory, and whether a debugger is attached")

`debug.display_memstat` prints how much memory the program holds. Call it
twice, some time apart, in a program you suspect of growing.
`debug.display_hostinfo` prints the versions of Bund's parts, the machine's
name and its system; it is the table to paste into a report of a fault in
Bund itself. `debug.display_distributed_info` is for programs run across
several machines, which this book does not reach.

`debug.info` says what the debugger has been told to watch for. With no
debugger attached, it says that.

#words((
  ("debug.display_memstat", "( -- )", [Memory in use.]),
  ("debug.display_hostinfo", "( -- )", [Versions, host and system.]),
  ("debug.info", "( -- )", [What is armed: breakpoints and watches.]),
))

#callout(label: "Looking has a cost")[
  Every word in this chapter has to be typed into the program and taken out
  afterwards, and you have to guess beforehand where to put it. When the
  guess is wrong you put it somewhere else and run again. That is fine for
  a program of thirty lines. The next chapters are for when it is not.
]

#recap((
  [`debug.stacks` shows every stack and the counting table: the first thing
  to type when a value has gone missing.],
  [`type` and `?type` say what kind of value you have.],
  [`debug.backtrace` in a word says how the word was reached.],
  [`debug.words` and `debug.classes` print the tables behind the program.],
  [`debug.display_stack`, `debug.display_workbench` and `debug.dump` show
  values as Bund holds them.],
))
