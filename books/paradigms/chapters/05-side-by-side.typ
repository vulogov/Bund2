#import "../design.typ": *
#chapter(number: 5, title: "Side by Side")

#dropcap("F")our programs, one page. Now put them on the table together.

#session("c5-count", caption: "Lines in each program, and lines changed for the fourth cargo")

The first four numbers are the length of each file, comments and all. The
last four are the lines that differ between each program and its version
with the _Heron_ and her wool.

Do not read too much into the lengths. They depend on how freely each file
is commented and on a job this small. The changes are the more honest
measure, and what _kind_ of change each was matters more than its size.

#section("The fourth cargo")

#words((
  ("By commands", "8 lines", [A word was opened and a new question fitted into
  the middle of it. Before the change, wool was charged as herring and
  nothing complained.]),
  ("By functions", "2 lines", [A ship in the list and a rate in the table. No
  word was touched. Before the change, wool was an error.]),
  ("By objects", "6 lines", [All of them new: a class and a ship. Nothing that
  existed was touched.]),
  ("By rules", "2 lines", [Two facts. Before the change, wool was simply
  absent from the ledger.]),
))

Three of the four take the new kind without disturbing anything already
written. The program of commands does not, and the reason is worth saying
exactly: it never wrote the tariff down as _data_. The other three did, each
in its own form. A table, a set of classes and a set of facts are all ways
of saying "here are the kinds, and here is what each pays" in one place,
where a new one can be set beside the old.

#section("The new question")

#words((
  ("By commands", "", [Could not be asked. The steps had to be rewritten and
  the day run again.]),
  ("By functions", "", [A new function over the same values. One line, three
  small words, nothing changed.]),
  ("By objects", "", [A new method in the parent class, which had to be
  opened.]),
  ("By rules", "", [One rule, added while the program ran. Or no rule at
  all: the question could have been put directly.]),
))

Here the order is different. Functions and rules take a new question
easily; objects take it at a price.

#callout(label: "Kinds and questions")[
  Add a new _kind of thing_, and objects are at their best: one new class,
  nothing else touched. Add a new _thing to ask_, and functions are at their
  best: one new word, nothing else touched. Each is awkward at what the other
  finds easy. This trade is old and well known, and no amount of cleverness
  makes it go away. Know which your harbour does more often: grow new
  cargoes, or grow new questions.
]

#section("What each one remembers")

#words((
  ("By commands", "", [The last ship, and the total. The rest was written
  over.]),
  ("By functions", "", [Nothing, and so everything: any value can be made
  again from the words.]),
  ("By objects", "", [Whatever objects you kept. They are values and do not
  change.]),
  ("By rules", "", [Every fact, for as long as the knowledge base is kept,
  and everything that follows from them.]),
))

#section("What goes wrong, and how you find out")

The unknown cargo is the best lesson in the book, because the four programs
failed in three different ways.

By commands, the wrong answer was printed with perfect confidence. By
functions, the program stopped and named the missing key. By rules, the ship
was left out and nothing was said. By objects it could not arise in the same
form at all: there is no way to make a landing of wool until a class `Wool`
exists, and asking for one that does not is an error at once.

A loud failure is better than a quiet one, and a quiet omission is better
than a confident mistake. If you take one practical thing from this book,
take that: _when you write a chain of questions, make the last branch an
error, not a guess._

#section("Which should you use?")

No one of them. Here is a fair summary of when each earns its place.

*Commands*, for the outermost layer of a program: the part that really does
happen in order. Read the arguments, print the heading, do the work, print
the total. Every one of the four programs ends with a few lines of this
kind, whatever manner the rest is in.

*Functions*, for reckoning: anything that turns values into other values
and has no business changing the world. Most of most programs.

*Objects*, when the things you deal with come in kinds that share a shape
and differ in detail, and new kinds keep arriving.

*Rules*, when what you have is knowledge and what you need is answers:
relations between things, questions you cannot list in advance, searches.

#recap((
  [The fourth cargo cost 8, 2, 6 and 2 lines. Only the program of commands
  had to alter what was already written.],
  [The new question was easy for functions and rules, dearer for objects,
  and impossible for commands without a rewrite.],
  [Objects make new kinds cheap; functions make new questions cheap.],
  [The same mistake was a wrong answer, an error, or a silent omission,
  depending on the manner.],
))
