#import "../design.typ": *
#chapter(number: 8, title: "Watches")

#dropcap("A") breakpoint is for when you know the place. Often you do not.
You know only what went wrong: something was left on the stack, a word
stopped being what it was, a number went negative. The night watchman does
not stand at one door. He is told what to look out for, and walks the whole
quay. A watch is an instruction of that kind.

#section("A stack that grows")

`leak.bund` reckons the day correctly and prints numbers where the ships'
names should be.

#piece("leak.bund", "// warned", until: "0 .", caption: "leak.bund: the new words")

`warned` was added to announce a large cargo. Since then the ledger says
`40: 140 pennies` for the _Marta_. `check` finds nothing. A value is being
left where it should not be, and the question is who leaves it.

In a healthy run of this program the stack is never more than four deep.
So: stop if it ever gets deeper.

#session("c8-depth", caption: "Stop when the stack is deeper than it should be")

It stopped at once, inside `warned`, called from `entry`. The stack has the
_Marta_ and then 40 three times. One 40 is the ship's units, one is the copy
`warned` makes to compare with 30. The third has no business there. Line 15
of the file, the place the frame names, begins `dup dup`.

`watch depth > N` stops at the push that takes the current stack from N
values to N + 1. It does not stop at every push above N, or a stack that
stays deep would stop at each one. It is ready again once the stack has
come back down.

#words((
  ("watch depth > N", "", [Stop when the current stack grows past N
  values.]),
  ("delete depth", "", [Stop watching the depth.]),
))

#section("A word that changes")

`clash.bund` prints four ruled lines and a correct total, and no ships.

#session("c8-redefine", caption: "Stop when a word is defined a second time")

`watch redefine` stops when a name that already has a word is given
another. It stopped for `line`, and `bt` says where: line 17 of the file.

#piece("clash.bund", "// line ( name due", until: "0 .", caption: "clash.bund: two words, one name")

Somebody wanted a word to rule a line across the page and called it `line`.
There was already a `line`, which prints a ship. Bund allows a word to be
defined again, and from then on the name means the newer one. `entry` asks
for `line` by name when it runs, so it gets the ruler.

This fault is easy to make and hard to see, because both definitions are
correct. Bund's own words can be shadowed in the same way: define `lines` or
`total` in a large enough program and you may have replaced a built-in word
that something else depends on.

#words((
  ("watch redefine", "", [Stop when a word is defined that already
  exists.]),
  ("delete redefine", "", [Stop watching for it.]),
))

#session("c8-mended", caption: "Both cures")

One `dup` fewer, and a different name.

#section("A stack by name")

A watch can be put on one named stack, and stops whenever a value is
pushed to it.

#session("c8-stack", caption: "Who puts things on the manifest?")

The first book's ledger keeps the arrivals on a stack called `manifest`.
The first stop is the first push to it: `arrivals` is putting down the name
of the _Marta_, and `changed:` notes that the program has moved to that
stack. A watch stops at the instant a value is being put down, so the stack
is shown as it was a moment before, and the name appears at the second
stop, when `"salt"` is on its way.

Use this when a stack that should be private has something on it that you
did not put there. The watch names the word that did.

#words((
  ("watch @STACK", "", [Stop at each push to the named stack.]),
  ("delete @STACK", "", [Stop watching it.]),
))

#section("The counting table")

The same, for the workbench.

#session("c8-bench", caption: "Who puts things on the table?")

Two stops. The first is the program's own `0 .`, the day's total being
started. The second is inside `entry`, where `+.` is adding the first due
to it. The stack shows the ship and the due in question: −94.

A total that comes out wrong was put on the table by somebody. This watch
shows every hand that touches it.

#words((
  ("watch workbench", "", [Stop at each push to the counting table.]),
  ("delete workbench", "", [Stop watching it.]),
))

#section("A watch with a question")

A watch on a stack may carry a condition, as a breakpoint may. The lambda
is shown the value being pushed.

#session("c8-if", caption: "Stop when a negative number is put down")

`watch @main if { 0 > }` asks, of each value pushed to `main`: is nought
greater than it? It is the question the whole of `tare.bund` comes down to.
No due should ever be negative, so stop at the first number that is.

It stopped inside `net`, at the end of its body, and `bt` gives the whole
path. That is the faulty word, found without knowing where to look and
without reading the program.

Two things on that screen need a word. The complaint at the top is the
condition failing on the ship's name: a string cannot be compared with
nought. A condition that fails does not stop the program. It says so once
for each kind of failure and carries on. And `changed:` shows only the name
on the stack, though −38 is the value that tripped the watch. A watch stops
at the instant the value is being put down, so the stack is shown as it was
a moment before.

#words((
  ("watch @STACK if { … }", "", [Stop at a push when the lambda, shown the
  value, leaves true.]),
))

#section("Catching a failure as it is raised")

When a program stops with a report, it is over. The frames have been thrown
away, and all that is left is what the report printed. `catch` stops the
program at the instant a failure is raised, while everything is still
standing.

#session("c8-catch", caption: "Stopped at the moment of failure")

`slip.bund` again. `caught:` gives the reason, and the program is stopped
inside `due` with both frames still there and the 3 on the stack. You may
look at anything: other stacks, the counting table, the words. Nothing you
do will rescue the run. The debugger says as much, in brackets: the failure
goes on its way as soon as the program moves, and `c` produces the report
you would have had anyway.

`catch` is the thing to arm when a program fails rarely, deep inside, and
the report alone does not tell you enough.

#words((
  ("catch", "", [Stop where a failure is raised, before it unwinds.]),
  ("delete catch", "", [Stop catching.]),
))

#recap((
  [`watch depth > N` finds the word that leaves one value too many.],
  [`watch redefine` finds a name used twice.],
  [`watch @STACK` and `watch workbench` name every word that pushes to a
  stack or to the counting table.],
  [A watch may carry a condition: `watch @main if { 0 > }` stops at the
  first negative number.],
  [`catch` stops at a failure while its frames are still there.],
))
