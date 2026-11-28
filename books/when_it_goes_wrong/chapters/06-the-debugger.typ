#import "../design.typ": *
#chapter(number: 6, title: "The Debugger")

#dropcap("T")here is a way of loading a ship that the stevedores dislike and
the surveyor insists on: one sling at a time, with a halt after each for him
to look in the hold. It is slow. It is also the only way to be certain which
sling shifted the cargo. `--debugger` runs a program that way.

#section("The first stop")

#session("c6-first", caption: "A program run one value at a time")

The program has not run. It stopped before its first value, and the long
line lists what you may type. This book cuts that line short from here on.

Under it is the line that matters, and every stop prints one like it:

```
#0  the top-level stream at 0  next: "net"  (tare.bund:11:1)
```

`#0` is the innermost frame, and here there is only one: the _top-level
stream_, which is the file itself, outside any word. `at 0` counts the
values done so far. `next:` is the value about to be done, and in brackets
is where it is written: line 11, column 1.

`(bund2)` is the prompt. `s` is _step_: do one value and stop again.

Three steps go through the first definition. The name `"net"` is put down.
A lambda of two things is put down. `register` takes both. After each step
there is a second line, `changed:`, which says what that step did to the
stack. You do not have to ask. Every stop reports what is different since
the last one.

`st` shows the current stack, empty here. `c` is _continue_: run on, and
stop only if something says to. Nothing does, so the program runs to its
end.

#words((
  ("--debugger", "flag", [Stop before the first value and take commands.]),
  ("s  step", "", [Do one value, going into any word, and stop.]),
  ("c  continue", "", [Run until something armed says stop.]),
  ("st", "", [Show the current stack.]),
))

#section("Leaving")

#session("c6-eof", caption: "Ctrl-D at the prompt")

Ctrl-D at the prompt _detaches_ the debugger. The program is not killed. It
runs on to its end with nobody watching, and Bund says so. Ctrl-C does the
same. A detached debugger cannot be attached again in that run.

#section("Into, over and out")

Stepping through six definitions to reach the day would be tedious. The
next chapter is about stopping where you choose; here one breakpoint is
borrowed from it to get to `entry`, so that the three ways of moving can be
seen.

#session("c6-moves", caption: "Step, next, finish")

`break entry` and `c` run to the first call of `entry`, with the _Marta_ and
her 40 on the stack.

`s` steps _into_ `entry`. The frame line now says `entry at 0`, and the next
value is `dues`.

`n` is _next_: do one value at this depth, without stopping inside it. The
whole of `dues` runs, with `due`, `net`, `fee` and all, and the stop is back
in `entry` with `dup` next. `changed:` says what `dues` did: took one value
and left −94.

There is the wrong number, seen for the first time at the moment it was
made. `dues` was given 40 and answered −94.

Two more steps, and `bt` prints the frames. `f` is _finish_: run until the
current word has ended. The _Marta_'s line is printed on the way, and the
stop is back in the top-level stream, ready for the _Gull_.

#words((
  ("n  next", "", [Do one value at this depth; do not stop inside it.]),
  ("f  finish", "", [Run until the current word has ended.]),
  ("bt", "", [The frames, innermost first.]),
))

#callout(label: "Halving")[
  `n` is the working tool. Step over a word and look at `changed:`. If the
  result is right, the fault is further on. If it is wrong, run again and
  step _into_ that word, and do the same one level down. Each round cuts the
  suspects in half, and a program of any size gives way in a few rounds.
]

#section("Typing Bund at a stop")

Anything you type that is not one of the debugger's own commands is run as
Bund, in the stopped program, on its stack.

#session("c6-bund", caption: "Mending a value by hand")

The program is stopped where `net` is about to run, with 40 on the stack.
`n` steps over it and the stack shows −38. So `net` is the faulty word: 40
went in and −38 came out, where 38 was wanted.

Now the line `drop 38`. It is not a debugger command, so it is Bund: drop
the wrong value and put down the right one. `st` shows the stack mended.
The breakpoint is deleted, and `c` lets the program run on.

The _Marta_ owes 134, which is right. The other two ships are still wrong,
because nothing mended them.

That is an experiment, not a cure. It proved that if `net` had answered 38,
everything after it would have been correct, and so that `net` is the only
fault on this path. The cure is in the file.

#session("c6-mended", caption: "The cure")

`2 -` is two take away the units, because the top speaks first. The tare
had to be put under the units before subtracting. One `swap`.

#warn(label: "A typed line changes the program")[
  What you type at a stop runs in the real program. It can push, drop,
  switch stacks, redefine a word, or end the run. The debugger does not
  protect the program from you. Words in a typed line are not stepped and
  breakpoints in them do not fire.
]

#section("What the debugger costs")

A program under the debugger is interpreted one value at a time, whatever
faster means of running it Bund may have, and its speed is of no interest
while you are at the prompt. The debugger writes to standard error, so the
program's own page on standard output is not disturbed, and can still be
sent to a file while you step.

#recap((
  [`bund2 --debugger program` stops before the first value.],
  [Each stop prints the innermost frame, the next value and where it is
  written, and what changed since the last stop.],
  [`s` goes into a word, `n` over it, `f` out of it, `c` runs on.],
  [`st` shows the stack and `bt` the frames.],
  [Any other line is run as Bund in the stopped program.],
  [Ctrl-D detaches, and the program runs on.],
))
