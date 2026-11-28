#import "../design.typ": *
#chapter(number: 1, title: "By Commands")

#dropcap("D")o this, then that. It is how a recipe is written, and how most
people first learn to program. A program in this manner is a list of steps,
carried out in order, and between the steps it remembers things by writing
them down under a name and rubbing them out when they change.

That name is a _variable_. The first book kept everything on the stack and
never used one. Bund has them all the same.

#section("A name that holds a value")

`var` takes a name and a value, and keeps the value under the name. `var?`
takes a name and puts the value back on the stack.

#session("c1-var", caption: "Writing a value down, and changing it")

The second and third `var` lines are the heart of this chapter. Each reads
the total, adds to it, and writes the sum back under the same name. Nothing
is left on the stack: the running total lives in the variable, and each step
_changes_ it. That is what the word "state" means.

#words((
  ("var", "( name value -- )", [Keep the value under the name, replacing what
  was there.]),
  ("var?", "( name -- value )", [The value kept under the name.]),
))

#callout(label: "For readers of the first book")[
  The first book kept its running total on the counting table and added to
  it with `+.`. That was state too. A variable differs in having a name, and
  in being reachable from anywhere in the program.
]

#section("The program")

#listing("commands.bund")

Read it as the clerk would carry it out. `rate` looks at a cargo and asks:
is it salt? If not, is it wax? If not, it must be herring. `reckon` takes a
ship's three values off the stack and files each under a name. From then on
it does not juggle the stack at all: it reads what it wants by name, in any
order, as often as it likes. It works out the due, adds the due to the
total, and prints the line.

Then the day itself: set the total to nought, print the heading, reckon
three ships, print the total.

It is the easiest of the four programs to read, and that is a real virtue.
Each line says what happens next.

#section("What it knows afterwards")

Learn the program at the terminal. It prints the ledger as it goes, and
then leaves behind whatever it last wrote down.

```
"commands.bund" study
```

#session("c1-after", caption: "After the day is done")

The total is there: 290. But ask for the ship's name and the answer is
`Osprey`. The _Marta_ and the _Gull_ are gone. Each ship was written over
the one before, because that is what the steps said to do. A program of
commands knows the present and has rubbed out the past.

#section("The fourth cargo")

Try the _Heron_ before changing anything.

#session("c1-heron", caption: "Wool, reckoned by a program that never heard of it")

No error. The program charged the wool 36 pennies, which is the herring
rate, and added it to the total. Look at `rate` again: after "is it salt?"
and "is it wax?" it does not ask a third question. It _assumes_. Everything
that is not salt or wax is herring, including things nobody thought of.

This is the commonest fault of the manner. A chain of questions has a last
branch, and the last branch catches whatever falls through.

To do it properly a third question goes into the chain, and the ship is
added to the day:

#session("c1-diff", caption: "What changed, and the page it prints")

Eight lines differ. The new ship is one of them. The other seven are inside
`rate`, which had to be opened up and a new question fitted into the middle
of it. The tariff is not written down anywhere as a tariff. It is spread
through the shape of the questions.

#section("The new question")

Which ships owe more than 75? The program cannot say. It reckoned each due,
printed it and wrote the next one over it. To answer, `reckon` itself would
have to be changed to test each due as it goes by, and the day run again.
In this manner a question is something you build into the steps beforehand.

#section("What it buys and what it costs")

#words((
  ("It buys", "", [Plainness. The program reads in the order things happen,
  and a variable has a name that says what it is.]),
  ("It costs", "", [The past. And a tariff, or any other rule, that exists
  only as the shape of some steps, so that changing the rule means changing
  the steps.]),
))

#recap((
  [A program of commands is a list of steps that change what is written
  under names.],
  [`:name value var` writes; `:name var?` reads.],
  [After the run, the variables hold the last ship only.],
  [An unknown cargo fell through the last branch and was charged as herring,
  without complaint.],
  [The fourth cargo changed 8 lines. The new question could not be asked
  without rewriting the steps.],
))
