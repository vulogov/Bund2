#import "../design.typ": *
#chapter(number: 2, title: "By Functions")

#dropcap("V")alues in, values out. In this manner nothing is ever changed.
A word takes what it needs from the stack and leaves an answer, and the
answer is a new value: the old ones are as they were. There are no variables
to write over, because there is nothing to remember between one word and the
next except what is on the stack.

#plate("05-mill.png", "What goes in is not changed. Something new comes out.")

You have been doing most of this since Chapter 4 of the first book. `dues`
was a function. What is new here is doing a _whole program_ this way, and
holding to it.

#section("Nothing changes")

Learn the program, and make a ship.

```
"functions.bund" study
```

#session("c2-values", caption: "A value in, a value out")

`ship` makes a record and `due` turns it into a number. Now the last two
lines. `reckoned` is the word that "writes the due into" a ship's record,
and it is given a copy of the _Marta_. The third line shows that what came
back has a `:due` field holding 140. The fourth asks the original, still on
the stack, whether _it_ has a `:due` field. It does not.

`reckoned` did not change the record. It made another. `set` has always
worked this way: it leaves a changed copy and the value it was given is
untouched. A program built only from such words can be read a word at a time
without asking what else it might have disturbed, because it cannot disturb
anything.

#section("A word with one operand already supplied")

`+` takes two numbers. The mooring fee is `+` with one of them fixed at 20.
Bund will make that word for you. It is called _partial application_, and
the word that does it is `curry`.

```
:moored curry
  :data   [ 20 ] set
  :lambda { + } set
!
```

`curry` is built like `?try`: it takes a name, you `set` its parts, and `!`
finishes it. `:data` is what to put on the stack first, and `:lambda` is what
to run then.

#session("c2-curry", caption: "A word made from a word")

The second line shows what was made: a lambda that puts down 20, puts down
the lambda `{ + }`, and runs it. `moored` is an ordinary word from now on.

#words((
  ("curry", "( name -- conditional )", [Begin a word that supplies some values
  and then runs a lambda. Set `:data` and `:lambda`; finish with `!`.]),
))

#section("The program")

#listing("functions.bund")

It is the longest of the four, and worth reading from the bottom. The last
line is the whole program:

```
arrivals { reckoned } map ledger print
```

Take the arrivals; make from them a list of reckoned ships; make from that
list a page of text; print it. Three values, each made from the one before,
and one act at the very end. Everything above that line is a definition, and
no definition prints.

Three things are _values_ here that were _steps_ in the last chapter.

*The tariff* is a dictionary. `rate` does not ask a chain of questions. It
looks the cargo up.

*The ships* are a list of records, made once.

*The page* is a text. `ledger` builds it from four parts, each a text that
ends its own line, and hands it back. Whether it is then printed, kept or
compared with yesterday's is not `ledger`'s business.

#callout(label: "Not quite pure")[
  `line` puts the record on the counting table while `has` reads four fields
  of it, and clears the table before it ends. Seen from outside it is a
  function: text out, nothing disturbed. Inside it uses a little state to
  save a great deal of stack juggling. Bund does not police this, and its
  author says so; the discipline is yours.
]

#section("What it knows afterwards")

Everything. Nothing was written over, because nothing was written. The
words that make the values are still there, and any value can be made again
and looked at.

#session("c2-map", caption: "Any part of the reckoning, on request")

#section("The fourth cargo")

#session("c2-diff", caption: "What changed, and the page it prints")

Two lines: one in the list of ships, one in the tariff. No word was opened.
`rate`, `due`, `line` and `ledger` are exactly as they were, because none of
them ever knew how many kinds of cargo there are. They ask the table.

And the fault of the last chapter cannot happen quietly. Ask the unchanged
program for the due of a cargo it has no rate for:

#session("c2-missing", caption: "Wool, before the tariff knows of it")

It stops, and names the missing key. (The 8 is the ship's units, left on the
stack by the word that failed.) A table that lacks an entry says so. A chain
of questions guesses.

#section("The new question")

Which ships owe more than 75?

#session("c2-question", caption: "A question nobody planned for")

Three short words and a line. The program was not changed and the day was
not run again in any special way: `arrivals { reckoned } map` made the same
list of values the ledger is made from, and a different word was applied to
it. A new question is a new function over the same values.

#section("What it buys and what it costs")

#words((
  ("It buys", "", [Confidence. A word that changes nothing can be tested on
  its own, used anywhere and trusted. New questions are cheap.]),
  ("It costs", "", [Length, here, and some effort to see the whole. The
  program does not read in the order things happen, because nothing
  happens until the last line.]),
))

#recap((
  [A program of functions is values made from values. Nothing is changed,
  and printing is left to the last line.],
  [`set` leaves a changed copy; the original is untouched.],
  [`curry` makes a word from a lambda and the values to hand it first.],
  [The tariff became a table, so a missing cargo is an error and not a
  guess.],
  [The fourth cargo changed 2 lines. The new question took one line and
  three small words, and changed nothing.],
))
