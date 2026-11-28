#import "../design.typ": *

#let title = "Harbour Days"

#let intro = [
#dropcap("T")he last six problems are whole programs. Each is a file, run
from the shell and not from the terminal, and each should be laid out the way
`ledger.bund` was in the first book: a comment at the head, the words from
the smallest to the largest, and the few lines that set them going at the
foot.

Write them in `code/`, and run them from there:

```
$ bund2 mine.bund
$ bund2 mine.bund --units 8
```

A program that wants the harbour's words begins by fetching them. Outside the
terminal there is no `study`, so the file's address is spelled out:

```
"/harbour.bund" cwd "file://" + + use
```

#words((
  ("args.parse", "( -- dict )", [The command line, as a dictionary of
  lists.]),
  ("use", "( address -- )", [Fetch a file of Bund and run it.]),
  ("cwd", "( -- path )", [The folder the program was started in.]),
  ("nl  space", "", [End the line; print one space.]),
))

The answers are in `code/answers/`. Yours will differ, and should. Judge them
by what they print.
]

#let problems = (
  (id: "c12p1", file: "duestable.bund", title: "The table of dues", grade: 2,
   ask: [Write a program that prints the dues for cargoes of 10, 20 … 100
   units in two straight columns, under the headings `units` and `dues`, with
   a rule above and below the figures.],
   hint: [`pad` from Problem 35, a little wider. The headings can be padded
   by the same word as the figures.],
   answer: [`pad` begins with `convert.to_string`, which leaves text as it
   is, so the headings and the numbers go through the same word and line up.
   `row` prints one cargo and its dues.]),

  (id: "c12p2", file: "change.bund", title: "Pounds, shillings and pence",
   grade: 2,
   ask: [Twelve pence make a shilling and twenty shillings a pound. Write a
   program that takes `--pennies` on the command line and prints the amount
   in pounds, shillings and pence. For 291 it prints
   `pounds 1, shillings 4, pence 3`. With no argument it reckons 291.],
   hint: [A pound is 240 pence. The shillings are what is left after the
   pounds, counted in twelves.],
   answer: [Three small words, each taking the whole amount. The amount is
   copied for each, and the three answers are left in the order the template
   reads them from the top: pounds, shillings, pence. An argument arrives as
   text in a list, so it is `car convert.to_int`.]),

  (id: "c12p3", file: "primes.bund", title: "Primes", grade: 3,
   ask: [A prime is a number above 1 that no smaller number but 1 divides.
   Write a program that prints the primes below the number given as
   `--below`, on one line. Below 50 there are fifteen.],
   hint: [Try the divisors 2, 3, 4 … in turn, and stop at the first that
   divides the number or whose square is more than the number. Then ask
   which of the two it was. Keep the number on the table.],
   answer: [The number under test lies on the counting table for the whole
   of `prime?`, and only the divisor is on the stack. `try?` answers "go
   on?", and the `while` adds one to the divisor for as long as it says yes.
   When the trying stops, the number is prime if the divisor's square has
   passed it.]),

  (id: "c12p4", file: "heron.bund", title: "A ship from the command line",
   grade: 2,
   ask: [Write a program that reckons one ship named on the command line:
   `--name Heron --cargo wool --units 8` prints
   `Heron: 8 of wool, 44 pennies`.],
   hint: [The arguments are a dictionary. Put it on the table and `has` will
   read from it, as it read from a ship's record in the first book.],
   answer: [`has` does not care what dictionary is on the table. Here it is
   the arguments, and `arg` adds the `car` that takes a value out of its
   list. The four values are put down in the order the wording wants them,
   last name first.]),

  (id: "c12p5", file: "tally.bund", title: "A tally of cargoes", grade: 3,
   ask: [In a week the harbour saw `salt wax salt herring salt wax wool`.
   Write a program that counts how many ships brought each cargo and prints
   the tally in alphabetical order, as `salt: 3`.],
   hint: [A dictionary from a cargo to a count. For each word of the text:
   if the cargo is not yet a key, set it to 1; if it is, set it to one
   more.],
   answer: [The tally rides under each word of the text. `count` sets the
   cargo on the table, because `?key` would use it up, and then either
   writes a 1 or reads the old count from a copy of the tally and writes one
   more. The keys are printed from the sorted list of different words.]),

  (id: "c12p6", file: "day.bund", title: "The ledger, from the manifest",
   grade: 3,
   ask: [The manifest reaches the customs house as three lines of writing:
   `Marta salt 40`, `Gull wax 12`, `Osprey herring 25`. Write the program
   that turns each line into a ship's record, prints the ledger, and totals
   it. The total must be 291.],
   hint: [`string.tokenize` and `unfold` put a line's three words on the
   stack in the order `ship` wants them. The last is a number written as
   text.],
   answer: [`read` is the whole of the new work: the words of a line, onto
   the stack, the last made a number, and `ship`. The rest is the first
   book's ledger. The total starts on the table at 0, each `line` adds its
   due there with `+.`, and the last line takes it up and prints it: 291, as
   it has been since Chapter 3 of the first book.]),
)
