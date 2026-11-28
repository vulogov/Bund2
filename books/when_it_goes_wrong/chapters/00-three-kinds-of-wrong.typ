#import "../design.typ": *
#chapter(number: 0, title: "Three Kinds of Wrong")

#dropcap("A") program can let you down in three ways, and they call for
different tools. Telling them apart is the first skill, and it takes one
look.

#section("It stops")

The program runs for a while and then Bund halts it and prints a report. A
word found too little on the stack, or the wrong kind of value, or a key
that was not in the dictionary.

This is the gentle kind of failure. Bund tells you that something is wrong,
what it was, and where. Part I is about reading what it tells you, about
catching such failures on purpose, and about finding them before the program
is run at all.

#section("It is wrong")

The program runs to the end, prints its page, and the page is false. No
report, because as far as Bund can tell nothing failed. Every word was given
the number of values it asked for. They were the wrong values.

This is the hard kind, and it is what most of this book is for. You have to
see inside a run: what was on the stack, which word was called by which, and
where the first wrong value appeared. Part II is about looking, and Part III
about stopping the program where you choose.

#section("It is slow")

The program is right, and you are still waiting for it. Somewhere it does
more work than it needs to, and guessing where is famously unreliable. Part
IV is about measuring.

#section("The programs")

The book uses a handful of small programs, all about the day's dues at
Wendhaven under the first book's tariff: three pennies a unit and twenty for
the mooring. The _Marta_ brings 40 of salt, the _Gull_ 12 of wax and the
_Osprey_ 25 of herring. They owe 140, 56 and 95, which comes to 291.

#words((
  ("slip.bund", "it stops", [The first book's broken script: a number was
  left out of the last line.]),
  ("tare.bund", "it is wrong", [A new allowance, written the wrong way round.
  Every due comes out negative.]),
  ("leak.bund", "it is wrong", [A word that leaves one value too many. The
  names disappear from the ledger.]),
  ("clash.bund", "it is wrong", [Two words given one name. The ledger prints
  rules where its lines should be.]),
  ("ledger.bund", "it is slow", [The first book's ledger, whole and correct.
  It does one piece of work twice.]),
))

All of them are in Appendix B. None is longer than a page except the
ledger, which you have read before.

#callout(label: "Nothing here is new Bund")[
  You will not learn a new way to write a program in this book. Every
  program in it uses words from the first book. What is new is everything
  around the program: the flags of `bund2`, the debugger's console, and
  about forty words whose names begin with `debug` or `log`.
]
