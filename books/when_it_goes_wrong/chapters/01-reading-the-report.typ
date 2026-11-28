#import "../design.typ": *
#chapter(number: 1, title: "Reading the Report")

#dropcap("W")hen a ship goes onto the bar, the pilot's report says three
things: what happened, where, and what she was carrying. Bund's report says
the same three, and most of the work of mending a program that stops is done
by reading it slowly.

Here is the faulty script of the first book. The last line says `dues` and
whoever wrote it forgot the 12.

#piece("slip.bund", ":due", rest: true, caption: "slip.bund")

#section("The report at a terminal")

#session("c1-run", caption: "A program that stops")

Three lines of the program's own output, and then the report. Read it from
the top.

*Error* is the reason. `MUL` is multiplication, and "too shallow" means it
wanted two values and did not find them.

*The drawing* shows two lines of the file. Line 4 is where the failure
happened: the `*` inside `due`, marked "the value that failed". Line 11 is
where the program was when it set off in that direction: "reached from this
step". The fault in the program is on line 11. The failure was on line 4.
They are rarely the same line, and the report shows you both.

*The traceback* is the path between them, innermost first. `#0` is `due`,
where it stopped. `#1` is `dues`, which called it. Each line of it is called
a _frame_: one word that had been entered and not yet left.

*The note* says which stack was current.

*Content of the stack* is what was there at the end: a single 3. That is
the 3 that `due` put down before `*` came up short. It is the cargo the ship
was carrying when she struck.

#term("Frame")[
  A word that has been entered and has not yet finished. While `due` is
  running inside `dues`, there are two frames. A traceback is the list of
  frames, the newest first.
]

#section("The report in a pipe")

When the output of `bund2` is not going to a terminal, the report is drawn
the older way, as a table. This is what the first book showed, and what you
will find in a log file.

#session("c1-box", caption: "The same failure, as a table")

It has less in it: the place the program was (line 11), that line's text,
and the stack. There is no traceback. `--report` chooses between the two
drawings whatever the output is going to.

#words((
  ("--report trace", "", [The drawing with the traceback.]),
  ("--report box", "", [The table.]),
  ("--report auto", "", [The default: the drawing at a terminal, the table
  in a pipe.]),
  ("BUND2_REPORT", "environment", [The same choice, for every run.]),
))

#section("More and less")

Two flags change the last part of the report.

#session("c1-nodump", caption: "The reason and the place alone")

`--no-dump-stack` leaves the stack out. Use it when the stack is long and
you already know what is on it.

#session("c1-raw", caption: "Each value as Bund holds it")

`--raw-values` shows each value as Bund keeps it inside, and there is more
to a value than the 3. It has an identity, the moment it was made, a number
for its kind, and a tag saying which stack it is on. You will seldom need
this. It is there for the day two values look the same and are not.

#section("A failing program exits with nought")

One thing surprises people who run Bund from other programs.

#session("c1-exit", caption: "The exit code of a program that failed")

A failing script is reported and exits 0. The first Bund did so, and Bund2
keeps to it. Only `bund.exit`, and a failure to start at all, set an exit
code. If another program needs to know that yours failed, catch the failure
yourself, as the next chapter shows, and call `bund.exit` with a code of
your choosing.

#recap((
  [The report gives the reason, the place the failure happened, the place
  the program was, the path between them and the stack.],
  [The fault is usually at "reached from this step", not at "the value that
  failed".],
  [At a terminal the report is a drawing with a traceback; in a pipe it is a
  table without one. `--report trace` and `--report box` choose.],
  [`--no-dump-stack` leaves the stack out; `--raw-values` shows values as
  Bund holds them.],
  [A failing script exits 0.],
))
