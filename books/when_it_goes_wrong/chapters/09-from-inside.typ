#import "../design.typ": *
#chapter(number: 9, title: "From Inside the Program")

#dropcap("S")o far the debugger has been something done to a program from
outside, with a flag. Every one of its commands is also a Bund word, and a
program may call them on itself. The ship carries her own surveyor.

#section("A program that stops itself")

#piece("stopped.bund", "\"Marta owes\"", rest: true, caption: "stopped.bund")

`debug.step` in a program means: from here on, stop before each value.

#session("c9-step", caption: "A debugger that was not asked for on the command line")

The program was started plainly. It printed its first line, reached
`debug.step`, and a console opened on the terminal it was already using.
From there it is the debugger of Chapter 6: `s`, `st`, `c`, and any line of
Bund.

This is what other languages call a breakpoint written into the source. It
is the fastest way into the debugger when you know the place: put
`debug.step` on the line before, run the program as you always do, and you
are there.

#session("c9-nobody", caption: "The same program with nobody at the keyboard")

When there is nobody to type, the console finds its input ended, detaches,
and the program runs on. A `debug.step` left in by mistake does not hang a
program run overnight.

#warn(label: "Not in a built program")[
  In a program packed with `bund2 build`, the words that stop a program do
  nothing at all. A shipped program does not halt at a prompt because a
  breakpoint was left in it.
]

#section("Setting traps from the program")

Everything Chapters 7 and 8 typed at the console has a word.

#piece("armed.bund", "\"due\" 2", until: "40 dues", caption: "armed.bund: the traps")

#session("c9-armed", caption: "Breakpoints armed by the program")

`debug.info` printed the three traps. The first ship went through. The
second call of `due` stopped the program, and then `fee`, because it was
about to be handed 36, which is less than 100.

Note the condition is written as a string here, `"{ 100 > }"`, where the
console took a lambda. It is the one difference between the two.

#words((
  ("debug.break", "( word -- )", [`break WORD`. A line number as a string
  breaks on a line.]),
  ("debug.break.at", "( word n -- )", [`break WORD N`.]),
  ("debug.break.if", "( word text -- )", [`break WORD if`; the condition is
  Bund source in a string.]),
  ("debug.until", "( word -- )", [`until WORD`.]),
  ("debug.catch", "( -- )", [`catch`.]),
  ("debug.watch", "( stack -- )", [`watch @STACK`; the name is written
  `"@main"`.]),
  ("debug.watch.if", "( stack text -- )", [`watch @STACK if`.]),
  ("debug.watch.workbench", "( -- )", [`watch workbench`.]),
  ("debug.watch.depth", "( n -- )", [`watch depth > N`.]),
  ("debug.watch.redefine", "( -- )", [`watch redefine`.]),
  ("debug.delete", "( name -- )", [`delete`.]),
  ("debug.step  debug.next", "( -- )", [`s` and `n`.]),
  ("debug.finish  debug.continue", "( -- )", [`f` and `c`.]),
))

Why arm from a program, when the console can do it? Because the trap is
then part of the file. It is there the next time, it can be sent to a
colleague with the fault, and it can be set at a moment the console could
only reach by hand: after the ledger is loaded, say, and before the second
day begins.

#section("One line, a term at a time")

`debug.step` before a `bund.eval` does not step through the string being
evaluated. `debug.run` is the word for that.

#piece("stepped.bund", "\"40 dues", rest: true, caption: "stepped.bund")

#session("c9-run", caption: "A string of Bund, stepped")

Each stop is at one term of the string: `40`, then `dues`, then `println`.
`n` steps over each, and `s` would go into `dues`. When the string is done
the program carries on.

#words((
  ("debug.run", "( text -- )", [Run a string of Bund with a stop offered
  before each term.]),
))

#section("A shell in the middle")

`debug.shell` is simpler than the debugger and older. It stops the program
and gives you a prompt at which every line is Bund. There is no stepping
and there are no breakpoints. Ctrl-D ends the shell and the program goes
on.

#piece("shell.bund", "40 dues", rest: true, caption: "shell.bund")

#session("c9-shell", caption: "A prompt inside a running program")

The program reckoned 140 and opened the shell. `debug.stacks` shows the
140 there. The second line is an experiment on a copy of it. Then Ctrl-D,
and the program's own `println` prints the 140 it was about to print all
along.

The same shell can be asked for from outside, to open only if the program
fails.

#session("c9-after", caption: "A shell on the wreck")

`--debug-shell` ran `slip.bund` to its failure, printed the report, and
then opened a shell on the program as the failure left it. The 3 is still
on the stack. The words are still defined, so the last line can try what
the program should have said: drop the 3, put down the 12 that was
forgotten, and `dues println` answers 56.

The frames are gone by then, as `debug.backtrace` shows. For the frames,
use `catch`. For everything else about a failed program, this is the
gentlest way in.

#words((
  ("debug.shell", "( -- )", [Open a prompt at which every line is run as
  Bund. Ctrl-D resumes the program.]),
  ("--debug-shell", "flag", [After a failure, open that shell on the program
  as it was left.]),
))

#section("Answering for the keyboard")

A program that asks a question is awkward to debug, because the debugger's
console is using the keyboard. `debug.feed` puts a line where the next
`input` will find it.

#piece("asks.bund", "\"Heron\"", rest: true, caption: "asks.bund")

#session("c9-feed", caption: "A question answered in advance")

`input` did not wait. It read `Heron`, the line that had been fed. At a
stop in the debugger, type `"Heron" debug.feed` and then `c`, and the
program's next question is answered.

#words((
  ("debug.feed", "( text -- )", [Queue one line for the program's next
  read.]),
))

#section("The first Bund's debugger")

One word is kept as the first Bund had it. `debug` takes a string and
shows each of its terms in a table before doing it, with a prompt after
each.

#session("c9-old", caption: "The word debug")

Each table is one term: its kind, its value, and the raw form. An empty
line at the prompt moves to the next term, and any other line is run as
Bund. It does not go into words and has no breakpoints. `debug.run` is its
successor, and the one to reach for.

#words((
  ("debug", "( text -- )", [Show each term of a string in a table, then do
  it, with a prompt between.]),
))

#recap((
  [`debug.step` in a program opens the debugger there, without a flag.],
  [Every console command has a word: `debug.break`, `debug.watch`,
  `debug.catch` and the rest. A condition is written as a string.],
  [`debug.run` steps through a string of Bund a term at a time.],
  [`debug.shell` is a plain prompt inside the program; `--debug-shell`
  opens it after a failure.],
  [`debug.feed` answers the program's next `input`.],
  [With nobody at the keyboard the console detaches; in a built program the
  stopping words do nothing.],
))
