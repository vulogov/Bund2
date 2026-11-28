#import "../design.typ": *
#chapter(number: 5, title: "The Trace")

#dropcap("T")he tide-watcher on the mole writes down every ship that passes,
in order, and nothing else. He does not stop them and he does not ask their
business. At the end of the day his book is the only complete account of
what moved. A trace is that book.

#plate("05-tide-watcher.png", "Every ship that passes, in order, and nothing else.")

#section("One line for every call")

`--trace` takes the name of a file and writes one line into it for every
word the program enters.

#session("c5-trace", caption: "The start of a trace")

The program ran as usual and printed its page, which is wrong: these are
the negative dues of `tare.bund`. The trace is in the file.

Its first line is a comment that explains the rest. Then six `register`
lines, one for each word the program defined, and a `return`, which is the
`.` that put the nought on the counting table. A trace names a word by what
it really is, and `.` is another name for `return`.

Then the day begins, and the indentation is the story. `entry` called
`dues`. `dues` called `due`, which called `net`, which called `-`. Back out
one step, `due` called `*`. Back out again, `dues` called `fee`. Everything
indented under a line happened inside that word.

#term("Trace")[
  A record of every word entered during a run, in order, indented by how
  many words were open around it. It records which words ran. It does not
  record what was on the stack.
]

Look at what it tells you about the fault. The dues are wrong, and here is
everything that happens when a due is reckoned: `net`, `-`, `*`, `fee`, `+`.
Five words. One of them is at fault. The trace has turned "the program is
wrong" into a list of five suspects without your having read the program.

#words((
  ("--trace FILE", "", [Write one line per word entered, natives included,
  indented by depth.]),
))

#section("Counting in a trace")

A trace is a plain file, and the ordinary tools of a shell work on it.

#session("c5-count", caption: "Questions asked of a trace")

Fifty-three lines for the whole day. `dues` was entered three times, once a
ship, which is right. `println` four times: three lines and the total.

This is how you answer "was that word ever called?" and "how many times?",
which are the two questions most often worth asking. A word you expected and
cannot find in the trace was never reached, and the fault is before it.

#section("A trace of a failure")

When a program stops, the trace stops with it, and marks the place.

#session("c5-fail", caption: "Where the trace ends")

The last lines of the trace of `slip.bund`. `dues` called `due`, `due`
called `*`, and under `*` is a line beginning `!` with the reason. It is the
traceback of Chapter 1 drawn the other way up, with everything that went
before it still on the page above.

#callout(label: "Tracing is not logging")[
  The `log` words of Chapter 2 say what you thought worth saying, where you
  thought to say it. A trace says everything, and you did not have to think
  of anything beforehand. For a fault you do not yet understand, begin with
  the trace.
]

#warn(label: "A trace of a long run is long")[
  Every call is a line. A loop over ten thousand ships writes several
  hundred thousand. Trace a small day that shows the fault, not the whole
  year.
]

#recap((
  [`bund2 --trace FILE program` writes one line for every word entered.],
  [Indentation shows which word called which.],
  [Built-in words appear under their real names: `.` is `return`, `swap` is
  `swap_one`, `dup` is `dup_one`.],
  [`grep -c` on a trace answers "was it called?" and "how often?".],
  [A failure is marked with `!` at the point it was raised.],
))
