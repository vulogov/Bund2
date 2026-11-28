#import "../design.typ": *

#v(1cm)
#text(font: body_family, size: 22pt, weight: "bold")[One Harbour, One Ledger]
#v(6mm)
#line(length: 100%, stroke: 0.5pt + ink_rule)
#v(8mm)

#plate(1, "01-harbour.png", "Wendhaven — the harbour this book keeps the ledger of.", width: 62%)

Bund is a large language. It has stacks and words, and on top of those it has
objects, logic programming, graphs, statistics, data frames and virtual machines
that talk to each other. Read the list of its words from end to end and you will
know _what each one does_ and still not know _how to begin_.

This book does the opposite of a reference. It takes _one small job_ and stays
with it. By the last page you will have written a complete program, and you
will know the part of Bund that every other part stands on.

#section("The job we are doing")

#term("The harbour ledger")[
  Wendhaven is a small harbour town, and an invented one. Ships tie up at its
  quay and their cargo is stacked there. For each ship the harbour is owed a
  _due_ that depends on how much cargo came ashore, and a flat _mooring fee_.
  At the end of the day the dues are written into a ledger and added up.
]

Three ships come in on the day this book describes, and they are the same three
in every chapter:

#words((
  ("Marta", "40 crates", [a cog carrying salt]),
  ("Gull", "12 crates", [a hulk carrying wax]),
  ("Osprey", "25 barrels", [a cog carrying herring]),
))

The harbour takes three pennies for every crate or barrel and twenty pennies
for the mooring. You are the harbourmaster. Bund is your clerk: he knows more than a
thousand words on the first morning, he does exactly what he is told, and he
learns a new word every time you teach him one.

I chose a ledger on purpose. It needs arithmetic, a decision or two, the same
step repeated for every ship, a little text, a list, a record, and somewhere to
keep a running total. That is the whole of a first programming language, and
none of it needs more than a line or two at a time.

#section("The one idea to take away")

If you remember nothing else from this book, remember this: in Bund you put
things down on a stack, you say a word, and the word picks them up —
*the top one first*.

#term("The top speaks first")[
  Every Bund word takes its _first_ operand from the top of the stack: the
  thing you put down last. Type `10 3 -` and Bund answers `-7`, because the
  top, 3, is the first operand and the sum is 3 − 10. To take three from ten
  you write `3 10 -`.
]

On a quay the last crate set down is the first one lifted, and the clerk reads
the label of the crate in his hands before he looks at the one still on the
pile. Bund holds to that throughout: in arithmetic, in comparisons, in joining
text, in choosing between two courses of action.
Chapter 3 is about nothing else, and from there on the book reminds you
whenever it matters.

#callout(label: "For Forth hands")[
  If you know Forth, or an HP calculator, this is the one place Bund will
  surprise you: `10 3 -` is 7 there and −7 here. Bund is not Forth with more
  stacks. Unlearn "move the operator to the end" now and the rest is familiar.
]

#section("The shape of the journey")

#screen(caption: "From the first word to the finished ledger")[```
  I    First Words ......... a conversation · the stack · who speaks first
  II   New Words ........... words of your own · decisions · repetition
  III  Things to Reckon .... text · lists · records
  IV   More Than One Stack . the counting table · the other quays
  V    The Ledger .......... the program in a file · under the hood
  VI   Where From .......... the languages Bund stands on
  VII  Reference ........... the words · who speaks first · answers
```]

#section("How to read the screens")

Bund is used at a terminal, so the book teaches with terminal screens. What you
type follows the prompt `[BUND >`. What Bund answers is printed under it. After
each line the terminal shows what is now on top of the stack, as `=> value`.

#session("ch03-due", caption: "A screen: two lines typed, two answers")

No screen in this book was typed by hand. Each one is a recording: the lines
were fed to #book_version and what came back is what you see. Type them and you
will get the same.

Every chapter ends with a few problems, and the answers are at the back. Do
them. Bund is learned in the fingers, and reading about a stack is to using one
as reading about swimming is to the water.

#section("What you need")

A computer with a terminal, and the Rust toolchain to build Bund with. The next
chapter builds it and says good morning to the harbour.

#recap((
  [This book follows *one job*, a harbour's ledger, from the first printed line
  to a finished program. Three ships — _Marta_, _Gull_ and _Osprey_ — are the
  same in every chapter.],
  [The central rule: *the top speaks first*. A word takes its first operand from
  the top of the stack, so `10 3 -` is −7.],
  [Screens show what you type after `[BUND >` and what Bund answers; `=>` shows
  the top of the stack. Every screen was recorded from #book_version.],
  [Each chapter ends with problems. The answers are in Appendix C.],
))
