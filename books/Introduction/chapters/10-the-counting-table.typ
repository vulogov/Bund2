#import "../design.typ": *
#chapter(number: 10, title: "The Counting Table")

#dropcap("I")n the customs house at Wendhaven there is an oak table, and the
clerk could not do his work without it. When he has a manifest in one hand and
needs both hands for the scales, he does not juggle. He puts the manifest on
the table, weighs, and picks it up again.

The last chapter ended with a word that could not be written without juggling:
a ledger line that needs four things out of one record. Bund has a table. It is
called the _workbench_, and with it the word is five lines long.

#plate(9, "09-counting-table.png", "A place to set one thing down while both hands are busy.")

#section("Setting a thing aside")

The word `.` — a single full stop — takes the top value off the stack and puts
it on the workbench. The word `take` brings it back.

#session("ch10-dot", caption: "Down on the table, and up again")

After the `.`, the 12 is nowhere on the stack. `.s` shows only the 40. It has
not been thrown away. It is on the table, and `take` fetches it.

#term("Workbench")[
  A second place to put values, beside the stack. There is one workbench, and it
  is always there. `.` moves the top of the stack onto it and `take` moves a
  value back. While a value is on the workbench, no ordinary word can see it or
  disturb it.
]

The workbench is itself a pile. If you put three things on it, they come back
last first.

#session("ch10-lifo", caption: "The table is a pile too")

And taking from an empty table is an error, as you would expect.

#session("ch10-empty", caption: "Nothing on the table")

#words((
  (".", "( x -- )", [Move the top of the stack onto the workbench.]),
  ("take", "( -- x )", [Move the top of the workbench back onto the stack.]),
))

#section("Looking without lifting")

`take` removes the value from the table. Often you want to look at it and leave
it there, because you will want it again. Bund has no single word for that, and
it does not need one. You can make it.

#session("ch10-peek", caption: "peek")

`peek` is `take dup .`: fetch it, copy it, put one copy back. The other copy
stays on the stack. The value on the table can be peeked at as many times as
you like.

This is the first word in the book that you needed and Bund did not have. It
took three words to write. That is the usual size of the gap between the
language you were given and the language you want.

#section("A record on the table")

Now the difficulty of the last chapter dissolves. Put the record on the table
once. Then every question about it is: peek at the record, ask for a field.

#session("ch10-has", caption: "has")

`has` is `peek swap get`. It expects a key on the stack. `peek` puts a copy of
the record on top of the key. `swap` puts the key on top, as `get` wants it.
`get` uses up the copy and leaves the value. The record on the table is
untouched.

#piles(
  (":name", (":name",)),
  ("peek", (":name", "record")),
  ("swap", ("record", ":name")),
  ("get", ("Marta",)),
  note: "has — the record on the table is copied, asked and left alone.",
)

Three questions were asked on that screen, and the stack holds three answers
and nothing else. No `dup`, no record in the way. Each `has` leaves exactly one
value, so they can be written one after another like a list of wants.

#section("The ledger line")

Here is the file for this chapter. The first two words you have just seen, and
the last two, `rule` and `heading`, are the ones from Chapter 7, kept here so
that they are not lost when you leave the terminal.

#listing("table.bund")

`fields` is the heart of it. The wording asks for a name, the units, a cargo
and a due, in that order, and `format` fills the first name from the top of the
stack. So the values must go down in reverse: the due first, the name last.
`fields` is that list of wants, written from the bottom up. The only reckoning
in it is the word `dues` after the first line.

`line` reads like the clerk's own movements. Put the record on the table. Fetch
the fields, lay them into the wording, print. Clear the table.

#session("ch10-line", caption: "One line of the ledger")

The stack is empty afterwards and so is the table. `line` has the effect
`( record -- )`: it takes a record and leaves no trace but the printing. That
is what lets it go into a loop.

#session("ch10-ledger", caption: "The day's ledger")

That is the ledger, in one line. Read it aloud: the heading; the
arrivals, gathered; for each, a line; a rule. Every word in it is one you taught
Bund, and each of them is a few words long.

#warn(label: "Clear the table when you are done")[
  A value left on the workbench stays there. Nothing sweeps it for you, `clear`
  does not touch it, and the next word that says `take` will get _your_
  leftover instead of what it expected.

  #session("ch10-left", caption: "Somebody left a 7 on the table")

  A word that puts something on the table should take it off again before it
  ends. `line` finishes with `take drop` for that reason.
]

#section("Reckoning on the table")

Some words can work with the table directly. Their names end in a dot.

#session("ch10-plusdot", caption: "Adding to what is on the table")

`+.` takes one number from the stack and adds it to the number on the
workbench, and the sum stays _on the workbench_. The stack is left clean. That
makes the table a natural home for a running total.

#session("ch10-total", caption: "A total that never touches the stack")

Compare this with Chapter 6, where the total rode at the bottom of the stack
and every round had to work above it. Here the total is out of the way. The
loop's lambda sees one number, turns it into a due and hands it to the table.
When the loop is done, `take` fetches the answer.

#term("Dotted words")[
  Many Bund words have a twin whose name ends in a dot: `+.`, `-.`, `*.` and
  more. The twin takes one of its operands from the workbench instead of the
  stack, and usually leaves its result there. Which operand, and where the
  result goes, differs from word to word, so try a dotted word at the terminal
  before you rely on it.
]

#words((
  ("peek", "( -- x )", [A copy of the top of the workbench. The workbench keeps it.]),
  ("has", "( key -- value )", [A field of the record on the workbench.]),
  ("fields", "( -- due cargo units name )", [What the wording asks for.]),
  ("line", "( record -- )", [Print one line of the ledger.]),
  ("+.", "( n -- )", [Add n to the number on the workbench. The sum stays there.]),
))

#section("When to use the table")

Not always. A stack word that needs one `dup` or one `swap` is clearer left as
it is, and `brief` in the last chapter was fine. Reach for the table when:

- one value is needed three or more times, as the record was;
- a value has to wait while several others are worked on;
- a total or a tally has to survive a loop without being in the loop's way.

And keep one rule: *whatever a word puts on the table, it takes off.* The table
is shared by every word in the program, and there is only one of it.

Only one table, but not only one stack. Bund's name means a bundle, and so far
you have used a bundle of one. The next chapter opens the other quays.

#recap((
  [The *workbench* is a second place for values. `.` moves the top of the stack
  onto it; `take` moves a value back. It is a pile, like the stack.],
  [`:peek { take dup . } register` looks at the workbench without emptying it.],
  [Put a record on the workbench and `has`, which is `peek swap get`, answers any
  number of questions about it without `dup` or `swap`.],
  [`line` is: record to the table, `fields wording format println`, table cleared.
  `heading arrivals gather { line } loop rule` prints the day's ledger.],
  [Dotted words such as `+.` reckon against the workbench; a running total can
  live there through a loop.],
  [A word that puts something on the workbench must take it off again.],
))

#problems((
  [What does `.s` show after `1 2 3 . . take`? And where is the other value?],
  [Define `tuck ( a b -- b a b )`, which copies the top value underneath the
  second, using the workbench.],
  [Define `over ( a b -- a b a )`, which copies the second value to the top.
  Forth has this word built in. Bund lets you build it.],
  [Using `+.`, count the units of cargo landed by all three ships. The answer is
  77.],
  [Add the cargo's size to the ledger line, so that the _Marta_'s reads
  `Marta: 40 of salt, 140 pennies, large`. You need a new wording and one more
  line in `fields`. Where must the new line go?],
))
