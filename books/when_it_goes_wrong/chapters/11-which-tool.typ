#import "../design.typ": *
#chapter(number: 11, title: "Which Tool, When")

#dropcap("A") shipwright called to a leak does not begin by taking the ship
apart. He asks what was seen, and where the water is, and reaches for the
least thing that will answer. This chapter is that habit, written down. It
has no new words in it.

#plate("11-shipwright.png", "He reaches for the least thing that will answer.")

#section("By what you see")

#words((
  ("A report", "it stops", [Read it: the reason, "reached from this step",
  the traceback, the stack. Chapter 1. If that is not enough, `catch` under
  the debugger, or `--debug-shell`. Chapters 8 and 9.]),
  ("Too shallow", "it stops", [`bund2 check --file`. It will usually name
  the line. Chapter 3.]),
  ("A wrong number", "it is wrong", [`--trace` to list the words that made
  it. Then `n` over each in the debugger and read `changed:`. Chapters 5
  and 6.]),
  ("A value missing", "it is wrong", [`debug.stacks`. It is probably on
  another stack, or still on the counting table. Chapter 4.]),
  ("A value too many", "it is wrong", [`watch depth > N`. Chapter 8.]),
  ("A word misbehaves", "it is wrong", [`watch redefine`, and look for
  `gen 2` in `words`. Chapters 7 and 8.]),
  ("Wrong on one ship", "it is wrong", [`break WORD N` or
  `break WORD if { … }`. Chapter 7.]),
  ("A stray line printed", "it is wrong", [`break println`, then `bt`.
  Chapter 7.]),
  ("A total gone wrong", "it is wrong", [`watch workbench`. Chapter 8.]),
  ("Never reached", "it is wrong", [`grep` for the word in a trace.
  Chapter 5.]),
  ("It takes too long", "it is slow", [`--profile`, and read the calls
  before the times. Chapter 10.]),
  ("It grows", "it is slow", [`debug.display_memstat` twice, and
  `watch depth`. Chapters 4 and 8.]),
))

#section("The order to try things in")

When you have no idea at all, this order wastes least.

*First, read.* The report, if there is one. The whole of it.

*Second, check.* `bund2 check --file` costs a second and needs no thought.

*Third, trace.* A trace asks nothing of you beforehand and shows what
ran. It usually turns a mystery into a short list.

*Fourth, stop.* Now you know where to stand. Break there, and step over
one word at a time, reading what changed.

*Last, measure.* Only when the program is right.

Each step is cheaper than the one after it, and each makes the next one
shorter. The temptation is always to start at the fourth, because the
debugger is the most interesting tool. It is also the one that takes
longest when you do not yet know where to put the breakpoint.

#section("Making the next fault easier")

Some habits cost nothing while a program is being written and repay the
day it goes wrong.

*Write the stack effect above every word.* `// dues ( units -- pennies )`.
Half the faults in this book are a word that did not keep the promise in
its comment, and `check` will hold the word to it.

*Keep words short.* A breakpoint on a three-value word tells you a great
deal. A breakpoint on a thirty-value word tells you to start stepping.

*Leave the bell in.* A `log.debug` at the door of each large word is free
until you ask for `-vv`.

*Raise early, with a reason.* A word that checks its operand and says
`"a cargo cannot weigh less than nothing" raise` fails at the cause. A
word that does not check fails three words later, at the consequence.

*Catch only at the edge.* Every `?try` in the middle of a program is a
place where a fault can vanish.

*Keep the faulty input.* A file of one ship that shows the fault is worth
more than the whole year's ledger. Make it, and keep it after the cure, so
that the fault can be shown never to have come back.

#section("What was never needed")

Look back over the book for what did not happen. Nobody added a `println`
to a program to see a value and took it out again. Nobody read a program
from the top looking for the mistake. Each fault was found by asking the
running program a question and reading the answer: who was called, what
changed, who pushed this, when did it go negative.

That is the whole craft. A program that has gone wrong knows exactly what
it did. The tools in this book are ways of asking it.
