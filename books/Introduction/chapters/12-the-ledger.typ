#import "../design.typ": *
#chapter(number: 12, title: "The Ledger")

#dropcap("A")t the end of the day the clerk rules a line under the last entry,
writes the total beneath it and shuts the book. Tomorrow he will do it again,
and he will not expect you to stand beside him.

Everything so far has been a conversation. You typed, Bund answered. A program
is the same words with nobody typing them: a file that reckons the day from top
to bottom and stops. This chapter puts the whole book into one file, runs it,
breaks it on purpose, and turns it into a program you could hand to someone who
has never heard of Bund.

#plate(11, "11-ledger.png", "The day's reckoning, written and ruled off.")

#section("The program, a piece at a time")

`ledger.bund` is a hundred and twenty lines, and most of them are comment. It
is printed whole in Appendix D. Here it is read as it was built: five small
vocabularies, each resting on the one before, and then the dozen lines at the
bottom that are the program. You have met nearly every word in it.

_Reckoning_ knows about pennies. These are the three words of Chapter 4,
exactly as you wrote them.

#piece("ledger.bund", "// due (", until: "// ---", caption: "ledger.bund: reckoning")

_Records_ knows what a ship is. The three setters, `ship` and `arrivals` are
from Chapter 9: `ship` starts with an empty `dict` and each setter puts one
field into it.

#piece("ledger.bund", "// units=", until: "// gather", caption: "ledger.bund: records")

`gather` is from Chapter 9 too, and `manifest` from Chapter 11. `manifest`
goes to a stack of its own, so that `gather` sweeps up the three ships and
nothing else, and comes back to `main` with the list.

#piece("ledger.bund", "// gather", until: "// ---", caption: "ledger.bund: the manifest")

_The table_ knows how to ask a record a question. Both words are from
Chapter 10. With a record on the counting table, `:cargo has` is her cargo,
and the record is still there for the next question.

#piece("ledger.bund", "// peek", until: "// ---", caption: "ledger.bund: the table")

_Writing_ knows how a line should read. `rule` is from Chapter 7, and so was
`heading`, but `heading` has changed. It now takes the day as a string and
joins it to the title. The top speaks first, so `"Wendhaven ledger for " +`
puts the title in front of whatever was underneath.

#piece("ledger.bund", "// rule", until: "// wording", caption: "ledger.bund: the heading")

`wording`, `fields` and `line` are as Chapter 10 left them. `line` puts the
record on the table, asks it four questions, fills the template, and takes
the record off again.

#piece("ledger.bund", "// wording", until: "// ---", caption: "ledger.bund: one line")

That is seventeen words, and sixteen of them you have typed before.

#section("The new word")

The fifth vocabulary, _the entry_, puts the others together for one ship. Its
first word is new.

#piece("ledger.bund", ":reckoned", caption: "ledger.bund: reckoned")

`reckoned` writes a ship's line and adds her due to a total. It needs the
record twice, once for `line` and once to reckon the due, so it starts with
`dup`.

The total lives on the table, and `+.` adds to it. You might think `line`,
which also uses the table, would disturb it. It does not. `line` puts the
record _on top of_ the total and takes it off again. The table is a pile, and
what is underneath is safe.

Two more words, `unreckoned` and `entry`, are for a ship whose record is
missing its units. They are shown where they are needed, two sections on.

#section("The program")

Everything above only teaches Bund words. Nothing is printed and nothing is
reckoned until the bottom of the file, where the words are used.

#piece("ledger.bund", "heading", caption: "ledger.bund: the program")

Read it as a sentence. Write the heading. Put a total of nought on the table.
Take the manifest and make an entry for each ship. Rule a line. Take the
total off the table and print it.

`heading` wants the day, and four lines just above these put it on the stack.
They are the subject of the next section.

#term("Script")[
  A file of Bund that is run from top to bottom by `bund2`. Definitions and
  the lines that use them are the same kind of thing: a `register` is one more
  word being done. That is why a word must be defined _above_ the first line
  that runs it.
]

#session("ch12-run", caption: "The day's ledger")

A hundred and forty, fifty-six, ninety-five, and two hundred and ninety-one.
The four numbers you reckoned by hand in Chapter 3.

#section("Telling the program something")

The program names the day in its heading, and the day comes from outside.
Whatever follows the file's name on the command line is handed to the program.

#session("ch12-day", caption: "An argument")

The word that reads the command line is `args.parse`. It leaves a dictionary:
each `--name value` becomes a key, and under the key is a list of the values
given. With no arguments at all there is only an empty list under `args`.

#session("ch12-args", caption: "What args.parse leaves when nothing was given")

Now the four lines that find the day can be read:

#piece("ledger.bund", "// The day comes", caption: "ledger.bund: the day")

Ask whether there is a `day`. `?key` leaves the dictionary and the answer. If
there is, the lambda on top runs: get the list under `:day` and take its first
element. If not, drop the dictionary and say `"today"`. Either way one string
is left on the stack, which is the rule for a decision that Chapter 5 gave: both
courses, the same stack.

#words((
  ("args.parse", "( -- dict )", [The command line, as a dictionary of lists.]),
))

#section("A hole in the manifest")

Real manifests have holes in them. Suppose a ship comes in whose master has
lost the sheet that says how much she carries. Her record has a name and a
cargo and no units.

Without a guard, `line` would ask for `:units`, `get` would fail, and the day's
ledger would stop at that ship with the rest unreckoned. `entry` looks before
it leaps. It asks `:units ?key`, and only a record that has the field goes to
`reckoned`.

#piece("ledger.bund", "// unreckoned", until: "// ---", caption: "ledger.bund: the entry")

`?key` leaves the record and the answer. The lambda on top runs when the
answer is yes, so `{ reckoned }` is written second. `unreckoned` takes the
ship's name and puts a sentence after it: the top speaks first, so the name
is swapped back to the top before `+`.

You can try it without editing the file. `study` does everything in a file, so
studying `ledger.bund` runs the day's ledger _and_ leaves all its words behind.

#session("ch12-heron", caption: "A ship with no units")

The _Heron_ got a sentence in place of a line, nothing was left on the stack,
and the total on the table was not touched.

#section("Catching what you did not foresee")

`entry` guards against a fault you knew might happen. For the ones you did not
think of, Bund has a net. It is the one piece of the language in this chapter
that you have not seen, and it is built from pieces you have.

Sharing the day's dues among the ships that paid them is a division, and a day
with no ships is a division by nought.

#session("ch12-try", caption: "A word that cannot fail")

`?try` puts down a small dictionary. Into it go two lambdas under the keys
`:try` and `:except`, written with `set` exactly as you would write any two
fields. `!` runs it: the `try` lambda is run, and if anything in it fails, the
failure is caught, a value describing it is put on the stack, and the `except`
lambda runs instead.

Here `except` drops that description and leaves 0. So `per` always leaves a
number. With three ships it is 97. With none, it is 0, and the program goes on.

#callout(label: "For the curious")[
  This is how the terminal you have been using keeps going when you make a
  mistake. Each line you type is run inside a `?try`, and the `except` lambda
  prints the word `error:` and the reason. The next chapter shows it.
]

#warn(label: "A net is not a floor")[
  Use `?try` at the edge of a program, where the world comes in: a file, a
  number someone typed, a manifest with a hole. Do not wrap it round your own
  words to hide their mistakes. A fault you catch and ignore is still a fault,
  and now nobody will be told.
]

#words((
  ("?try", "( -- conditional )", [Put down a guarded piece of program, to be filled
  with a `:try` and an `:except` lambda and run with `!`.]),
  ("reckoned", "( record -- )", [Write the ship's line and add her due to the total
  on the table.]),
  ("unreckoned", "( record -- )", [Say that the ship could not be reckoned.]),
  ("entry", "( record -- )", [Reckon a ship if her record has its units.]),
))

#section("When a program fails")

At the terminal a mistake costs you one line. In a script it stops the
program. Here is a short one with a slip in it.

#listing("broken.bund")

The last line but one prints "Gull owes", and the last line says `dues` — and
whoever wrote it forgot the 12.

#session("ch12-broken", caption: "A script that stops")

The first three lines of output are the program doing its work. Then Bund
stops and reports, and the report is worth reading from the top.

*Error* is the reason, and you know this one: `MUL`, the multiplication inside
`due`, found too little on the stack. *Location* is the file, the line and the
column where the failing word was written: line 11. *Source* is that line.
*Stack* is which stack was current. And underneath, Bund shows what was on the
stack when it stopped: a single 3, the one `due` had put down before `*` came
up short. It is the screen from Chapter 4, with an address on it.

#section("Finding it before you run it")

That mistake could have been found without running anything. `bund2` has a
second command, `check`, which reads a program and works out, word by word, how
many values each one needs and how many will be there.

#session("ch12-check", caption: "check")

It found the same fault, at the same place, and it did not print "Marta owes"
on the way, because it ran nothing. `check` knows the stack effects that Bund's
own words declare, works out the effect of a word you define from its
definition, and follows the count down the file.

It cannot follow everything, and it is honest about that.

#session("ch12-check-ledger", caption: "What check could and could not follow")

No findings in the ledger. But it gave up at `manifest`, a word whose effect
it could not pin down to a fixed number of values in and a fixed number out.
`check` says what it tracked and claims nothing about the rest. A clean report means "no fault found in what could be followed". It does
not mean "no fault".

#callout(label: "A habit worth having")[
  Run `bund2 check --file` on a script before you run the script. It takes a
  moment, and a stack that is one short is the commonest fault there is. This
  is the stack-effect comment of Chapter 4, checked by the machine.
]

#section("A program to give away")

To run `ledger.bund` a person needs `bund2` and the file. `build` makes one
file that needs nothing else.

#session("ch12-build", caption: "A standalone program")

`build` writes a copy of `bund2` with your program packed inside it. The result
is an ordinary program. It takes the same arguments, and whoever runs it need
never know what it was written in.

#words((
  ("bund2 FILE", "", [Run a script. What follows the name goes to the program.]),
  ("bund2 check", "--file FILE", [Report where a script would run short, without
  running it.]),
  ("bund2 build", "--file F --output P", [Write the script into a standalone
  program P.]),
))

#section("What you have built")

Turn to Appendix D, where the file is printed whole, and count. Twenty words were taught to Bund, none
longer than five lines, and the program that uses them is a dozen lines more.
There is not one variable in it. There are two decisions, one loop and one
running total. Every value went from the word that made it to the word that
wanted it by being left on a stack or a table, and where that would have been
awkward, a word was written to make it easy.

That is the craft of this language. You do not write a program in Bund. You
grow a vocabulary until the program is a sentence in it.

One chapter remains, and it is about the thing you have been typing into.

#recap((
  [A *script* is a file run from top to bottom. Definitions come first; the lines
  that use them come after.],
  [`bund2 ledger.bund --day Monday` passes arguments; `args.parse` leaves them as
  a dictionary of lists.],
  [Guard against the fault you can foresee with a question: `:units ?key` and a
  decision.],
  [`?try`, filled with a `:try` and an `:except` lambda and run with `!`, catches
  a failure and lets the program go on. Use it at the edges.],
  [A failing script stops with a report: the reason, the file and line, the
  current stack and what was on it.],
  [`bund2 check --file` finds stack faults without running the program, and says
  what it could not follow.],
  [`bund2 build --file F --output P` makes a standalone program.],
))

#problems((
  [Copy `ledger.bund` and make the mooring fee 25 pennies. What is the new
  total? Work it out before you run it.],
  [Add the _Heron_, with 8 bales of wool, to the arrivals. What is the total
  now, with the fee back at 20?],
  [Put a `+` with nothing before it on the first line of a small script and find
  it with `check`. Then put it on the last line, after a word that uses
  `gather`. Does `check` still find it? Read what it says about where it
  stopped.],
  [_A project, with no answer at the back._ Add a second argument, `--due`, so
  that the last line can read `Total: 291 pennies, due Friday`. When it is not
  given, leave that part out.],
  [_Another project._ Make the program print, after the total, how many ships
  were reckoned. Keep the count on a stack of its own.],
))
