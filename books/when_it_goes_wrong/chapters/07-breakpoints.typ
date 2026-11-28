#import "../design.typ": *
#chapter(number: 7, title: "Breakpoints")

#dropcap("T")he customs officer does not open every crate. He chalks a mark
on the ones he wants and lets the rest go by. A breakpoint is the chalk
mark: a place where the program is to stop, so that it can run freely
everywhere else.

#section("On a word")

`break` and a word's name: stop whenever that word is about to run.

#session("c7-word", caption: "A breakpoint on a word")

The stop is _before_ `net` runs. The frame line names `due`, the word that
is calling it, and the stack shows what `net` is about to be given: the
ship's name and 40. `bt` shows how the program came here: `due`, from
`dues`, from `entry`.

`c` runs to the next time `net` is called, which is for the _Gull_. On the
way the _Marta_'s wrong line was printed. `info` lists what is armed and
how often each has been hit, and `delete` takes one away.

#words((
  ("break WORD", "", [Stop each time the word is about to run.]),
  ("info", "", [What is armed, and how many times each was hit.]),
  ("delete WORD", "", [Take a breakpoint or a watch away.]),
))

#section("On the nth call")

Often the first call is fine and a later one is not. A number after the
word's name says which call to stop at.

#session("c7-count", caption: "The second call, and only the second")

`break dues 2` let the _Marta_ through and stopped for the _Gull_. It will
not stop again. In a loop over a thousand ships, where the six hundredth
goes wrong, this is how you arrive at the six hundredth without pressing
`c` five hundred and ninety-nine times.

#words((
  ("break WORD N", "", [Stop at the Nth call of the word, once.]),
))

#section("On a condition")

A breakpoint may carry a question, and stops only when the answer is yes.
The question is a lambda. It is shown the stack as the word is about to
find it, and must leave true or false.

#session("c7-if", caption: "Stop for a small cargo only")

This is the first book's ledger. `break due if { 20 > }` stops at `due` only
when 20 is greater than the units it is about to be given: the top speaks
first, as everywhere. The _Marta_'s 40 went by. The _Gull_'s 12 stopped the
program, twice, because the ledger reckons each due twice, a thing Chapter
10 will have more to say about. The _Osprey_'s 25 went by.

The traceback here is worth a look for its own sake. Seven frames, and two
of them are "a lambda": the bodies handed to `ifthenelse` and to `loop`.
They have no names, so the debugger says where they are written.

The condition is run in a copy of the program, not in the program itself.
It may look at the stack as much as it likes. It cannot change it.

#words((
  ("break WORD if { … }", "", [Stop when the lambda, shown the stack,
  leaves true.]),
))

#section("On a line")

A number alone is a line of the file.

#session("c7-line", caption: "A breakpoint on line 11")

Line 11 of `tare.bund` is the definition of `net`. The breakpoint does not
stop when the definition is read. It stops when a value _on that line_ is
about to be done, which is when `net` runs. The frame says `net at 0`: this
stop is inside the word, where `break net` stopped just outside it.

With more than one file, `break tare.bund:11` says which.

#words((
  ("break N", "", [Stop before the first value of line N, each time it is
  reached.]),
  ("break FILE:N", "", [The same, in a named file.]),
))

#section("Until")

`until` is a breakpoint and a `c` in one, and it is forgotten once it has
stopped.

#session("c7-until", caption: "Run until a word is called")

It ran to the first call of `line` and stopped with the name and the due on
the stack. Use it to get somewhere once, without leaving a mark behind.

#words((
  ("until WORD", "", [Run to the next call of the word, and stop there
  once.]),
))

#section("On a built-in word")

A breakpoint may be put on one of Bund's own words.

#session("c7-native", caption: "Stopping at println")

The stop is before `println` runs, with the text it is about to print still
on the stack. Every line a program prints passes through here, so this is
the place to stand when you want to know who printed a line you did not
expect: `bt` at this stop will say.

A breakpoint on `dup` or `swap` works too. Those are other names for
`dup_one` and `swap_one`, as the trace showed, and the breakpoint is known
by the name you gave it.

#section("How often each word ran")

While a debugger is attached, Bund counts the calls of every word. `words`
prints the count.

#session("c7-words", caption: "The words, with their calls so far")

The program is stopped at the third call of `dues`. The table says `dues`
and `entry` have been called three times and everything inside `dues` twice,
which is exactly where a program stopped at that moment should be. `swap`
is at generation 2 because Bund itself defines it twice as it starts.

`words all` prints every word Bund has, over a thousand lines. `words du`
prints the ones with `du` in the name. `classes` prints the classes.

#words((
  ("words", "", [The program's words and those called so far, with their
  calls.]),
  ("words all  words TEXT", "", [Every word; the words whose names contain
  the text.]),
  ("classes", "", [Every class, with parents, methods and fields.]),
))

#recap((
  [`break WORD` stops before the word runs; the stack shows what it is about
  to be given.],
  [`break WORD N` stops at the Nth call only.],
  [`break WORD if { … }` stops when the lambda says yes. The lambda sees the
  stack and cannot change it.],
  [`break N` and `break FILE:N` stop at a line.],
  [`until WORD` runs to a word once and leaves no mark.],
  [`info` lists what is armed, `delete` removes one, `words` counts calls.],
))
