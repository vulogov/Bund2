#import "../design.typ": *
#chapter(number: 10, title: "The Profile")

#dropcap("T")he harbourmaster thinks the crane is slow. The crane-man says
it is the carters. The carters blame the gate. Nobody is lying and nobody
knows, until one morning a boy with a slate stands on the quay and writes
down how long each thing takes. Guessing where a program spends its time is
the same argument, and a profile is the boy with the slate.

#plate("10-slate.png", "A boy with a slate, writing down how long each thing takes.")

#section("Asking for one")

`--profile` takes a name and, when the program ends, writes two files that
begin with it.

#session("c10-run", caption: "A profiled run")

The program printed its page as usual. `day.profile.md` is a report to
read, and `day.profile.dot` is the same thing as a graph, for a drawing
program.

The report is long. Three parts of it answer nearly every question, and
this chapter reads those.

#words((
  ("--profile BASE", "", [Write `BASE.profile.md` and `BASE.profile.dot`
  when the program ends.]),
))

#section("The call tree")

#session("c10-tree", caption: "Who called whom, and what it cost")

This is the trace of Chapter 5 with the repeats folded together and a time
on every line. The whole program took the time on the first line. Under it,
`loop` ran once, and inside `loop` everything about the three ships
happened: `entry` three times, `reckoned` three times, and so on down.

A time on a line is everything spent inside that word, including the words
it called. So the lines under a word add up to less than the word itself,
and what is left over is the word's own.

Read it for proportion, not for the figures. The figures on your machine
will differ from these, and will differ from one run to the next. What
holds is the shape: nearly all of a ship's time goes into `line`, and
inside `line` into `format` and `fields`.

#warn(label: "Small times are mostly the clock")[
  This program runs for a quarter of a thousandth of a second. At that
  scale the act of timing a word costs about as much as the word, and the
  report says so at length in its header. Time is worth reading in a
  program that runs for seconds. In a small one, read the counts.
]

#section("Counts are exact")

Beside each time the report gives two numbers that do not depend on the
clock or the machine: how many times a word was _called_, and how many
_steps_ it took. A step is one value done. Two profiles of the same program
agree on these to the last digit.

#session("c10-calls", caption: "Calls and steps for four words")

The columns are the word, its kind, its stack effect, its calls and its
steps. `has` was called twelve times: four fields for each of three ships.

Now look at `dues`. Six calls. There are three ships.

Go back to the tree. `dues` appears in it twice: under `fields`, three
times, and under `reckoned`, three times. The ledger works out what each
ship owes once to print her line, throws the answer away, and works it out
again to add it to the total.

Nobody would have guessed that. The first book's ledger was read line by
line in two books and the doubled work was never noticed, because both
uses are correct and each is sensible where it stands. It took a count to
show it. The cure would be for `reckoned` to work the due out once and hand
it to `line`; in a program of three ships it does not matter, and in one of
three hundred thousand it is half the arithmetic.

#callout(label: "What a profile is for")[
  A profile seldom tells you a word is slow. It tells you a word is called
  more often than you thought. Look for the count that surprises you.
]

#section("A profile in the middle")

`debug.profile` writes the profile of the run _so far_, from inside the
program, and the run goes on.

#piece("snap.bund", "40 dues", rest: true, caption: "snap.bund")

#session("c10-snap", caption: "A snapshot, with and without --profile")

Run plainly, the program keeps no profile, and `debug.profile` says so once
and is otherwise harmless. A program may be left with the word in it.

Run with `--profile end`, it writes two reports: `half`, at the moment the
word was reached, and `end`, when the program finished. The last line of
the screen is from the header of `half`: the run had not ended, and the
call that was open when the snapshot was taken is counted up to that
moment.

Call it with the same name inside a long loop and one report is kept up to
date: read the file at any time and it is as fresh as the last write.

#words((
  ("debug.profile", "( base -- )", [Write the profile of the run so far to
  `base.profile.md` and `base.profile.dot`.]),
))

#section("The graph, and what this book cannot show")

The `.dot` file is the call tree for Graphviz. If you have it,
`dot -Tsvg day.profile.dot -o day.svg` draws every word as a box, shaded
by how much time was its own, with an arrow for every call.

A `bund2` built with the `jit` feature can translate the words a program
runs most into the machine's own instructions. For such a build, `--stats`
prints what was translated, and `--jit-threshold` says how many runs of a
word earn it. The `bund2` this book was recorded with has no such tier,
which `words` has been saying at the top of its table since Chapter 7, and
so there is no screen of it here. A profiled run is always interpreted, so
that every step can be counted.

#recap((
  [`bund2 --profile BASE program` writes a report and a graph when the
  program ends.],
  [The call tree shows who called whom and the time inside each.],
  [Times vary from run to run and are mostly clock in a small program.
  Calls and steps are exact.],
  [The profile of the ledger shows `dues` called six times for three ships:
  work done twice.],
  [`debug.profile` writes a snapshot mid-run, and warns once if the run
  keeps no profile.],
))
