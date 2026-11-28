#import "../design.typ": *
#chapter(number: 13, title: "Under the Hood")

#dropcap("T")here is a clock on the customs house at Wendhaven. For twelve
chapters you have read the time off its face. This one takes you up the stair
behind the dial, with a lantern, to look at the wheels.

Nothing here is needed to keep a ledger. All of it explains why Bund is the way
it is: why a piece of program can sit on a stack, why a word can be redefined
while a program is running, and what, exactly, you have been typing into since
the first page.

#plate(12, "12-clockwork.png", "What was turning all along.")

#section("A program is a list")

Put a lambda on the stack and ask Bund to make a list of it.

#session("ch13-lambda", caption: "What a lambda is made of")

It is a list of two things: the number 3 and the word `*`. That is all a lambda
is. Braces and brackets collect the same things in the same order. The only
difference is a label on the result that says "this one may be run".

The same is true of the words you defined.

#session("ch13-body", caption: "The bodies of two words")

`dues` is a list of two words. `size` is a list of four things: a word, two
lambdas, each a list of one string, and a word. There is nothing else in there:
what you can take out of a word is what you put into it.

#term("Code is data")[
  In Bund a piece of program is a value of the same kind as any other. It can
  be put on a stack, kept in a dictionary, taken apart, built from pieces and
  handed to a word. Words such as `if`, `loop` and `map` are not special
  syntax. They are ordinary words that happen to take a lambda as an operand.
]

That is why Chapter 5 could introduce `if` without any new grammar, and why
there has been none since Chapter 1. A language whose programs are values needs
no syntax for control. It needs words that take programs.

#section("The stack as a program")

Chapter 9 used `lambda*` inside `gather` and promised an explanation. It turns
the whole stack into one lambda.

#session("ch13-star", caption: "A stack, frozen and thawed")

Three values became a lambda holding three values. `!` ran it, and running a
value means putting it down, so the three came back in their old order.

This is how the terminal's `.s` shows you the stack without disturbing it. It
freezes the stack into a lambda with `lambda*`, prints a copy, and thaws the
original with `!`.

#section("Text as a program")

A program can also be made from text. `bund.eval` takes a string and runs it as
Bund.

#session("ch13-eval", caption: "A string, run")

The second line builds its program out of two strings before running it. And
if a program can build a program, it can build a _definition_.

#session("ch13-write", caption: "A word that rewrites a word")

The first line assembles the text `:due { 4 * } register` from three pieces,
and `bund.eval` runs it. `due` is redefined, and because words are looked up
when they run, `dues` follows: 180.

`rate` makes a word of that. `3 rate` puts the due back to three pennies. A
program could read the rate from its command line and set it before the first
ship is reckoned, and no word that uses `due` would need to know.

#warn(label: "With great power")[
  A string handed to `bund.eval` can do anything a program can do. Never
  evaluate text that came from somewhere you do not trust. `bund2 build` has a
  `--noeval` option that switches the word off in a program you mean to give
  away.
]

#words((
  ("lambda*", "( ... -- lambda )", [The whole stack as one lambda, in order.]),
  ("bund.eval", "( text -- ... )", [Run a string as Bund.]),
  ("convert.to_list", "( lambda -- list )", [The same things, as a list.]),
))

#section("Words as values")

Chapter 4 showed the backtick, which puts down a pointer to a word.

#session("ch13-pointer", caption: "A pointer is a value")

And a lambda can be kept anywhere a value can. Here is one in a dictionary,
fetched by its key and run.

#session("ch13-table", caption: "A table of things to do")

Look at what that is. A dictionary whose keys are names and whose values are
pieces of program is a table of commands. Give it a string, and it gives you
back something to run. Hold on to that; you are about to see it used.

#section("How many words?")

`bund2` will list every name it answers to.

#session("ch13-count", caption: "Counting the vocabulary")

Twelve hundred and twelve names. Seventy of them begin `string.`, and four
hundred and forty-four end in a dot, most of them the workbench twins of
Chapter 10. This
book has used about seventy. The rest are the same kind of thing as the ones
you know: they take values from a stack and leave values on it, top first.

#section("The terminal, read aloud")

Now the promise of Chapter 1. `terminal.bund` is the program you have been
typing into. It is printed whole in Appendix D, and read here a piece at a
time, starting at the bottom, where the program is.

#piece("terminal.bund", "\"Bund\" io.banner", caption: "terminal.bund: the program")

`io.banner` turns a string into the large letters you saw on the first day.
The third line is the whole terminal. `input*` takes a prompt and a lambda. It
prints the prompt, reads a line from the keyboard, puts the line on the stack
as a string and runs the lambda. Then it does it again, until there are no
more lines. `bund.prompt` is the coloured `[BUND >`. Everything else is in
`term.line`.

#piece("terminal.bund", ":term.line", caption: "terminal.bund: one line")

`term.line` is two decisions, and you can read both. Is the line empty? Then
drop it. If not: is it a key in the table of commands?

#piece("terminal.bund", ":term.commands", caption: "terminal.bund: the commands")

That is the table of things to do from a moment ago: a dictionary from a
line's text to a lambda. If the line is there, `swap get !` fetches its lambda
and runs it. That is all `help`, `.s`, `clear` and `quit` are: four entries in
a dictionary. If the line is not there, it goes to `term.eval`.

#piece("terminal.bund", ":term.eval", caption: "terminal.bund: running a line")

`term.eval` is `bund.eval` inside a `?try`. Your line is run as Bund. If it
fails, `except` takes the description of the failure, pulls out the text under
`:context`, shortens it and prints it after `error:`. Then `term.show` runs.

#piece("terminal.bund", ":term.show", caption: "terminal.bund: the answer")

This is the `=>` line: try to `dup` the top of the stack and print the copy,
and if there is nothing to `dup`, say nothing.

#piece("terminal.bund", ":term.short", caption: "terminal.bund: a short error")

`term.short` is why the errors in this book fit on a line. Bund reports a
failure as a chain — this failed because that failed because the other did —
and the last link is the one that says what went wrong. The word splits the
text at each "returned error" and keeps the last piece, with a loop that
replaces the piece it holds with each one that comes.

#piece("terminal.bund", ":term.stack", caption: "terminal.bund: .s")

`term.stack` is `.s`. You have just seen its trick: freeze the stack, count
what is in it, print a copy, thaw.

#piece("terminal.bund", ":study", caption: "terminal.bund: study")

And `study` is the three-line word Chapter 7 promised. `use` wants a whole
address, so the file's name is joined to the folder and to `file://`. The top
speaks first, so the pieces are put down last part first.

One word is left, `term.help`, and it is eight lines of `println`.

That is the terminal: about sixty lines of program. You could have written it
after Chapter 12. Every screen in this book was made by it.

#section("What runs your words")

A last look, one level further down, and only a glance.

When `bund2` reads your program it does not run the text as it stands. It
turns it first into a compact internal form, and an _interpreter_ runs that.
Every Bund program runs this way, on every machine. A `bund2` built with the
`jit` feature can go further for the words a program runs most, and translate
them into the machine's own instructions while the program is running. It
changes how fast a program goes and never what it means.

You do not need to know more than that to use the language. It is enough to
know that the stack, the table and the quays are not a picture laid over
something else. They are what is there.

#recap((
  [A lambda is a list with a label that says it may be run. So is the body of
  every word you define. *Code is data.*],
  [`if`, `loop`, `map` and the rest are ordinary words that take a lambda. Bund
  needs no syntax for control.],
  [`lambda*` freezes the whole stack into a lambda; `!` thaws it. That is how
  `.s` and `gather` work.],
  [`bund.eval` runs a string as Bund. A program can build a definition as text
  and evaluate it. Never evaluate text you do not trust.],
  [A dictionary of lambdas is a table of commands.],
  [The book's terminal is about sixty lines of Bund: `input*` reads a line, a
  dictionary handles the four commands, and `bund.eval` inside `?try` does
  the rest.],
))

#problems((
  [Take the body of `dues` with `lambda=`, turn it into a list, and print each
  thing in it on its own line.],
  [Without `register`, run the body of `size` on the number 40. What is left?],
  [Add a command `depth` to a copy of the terminal that prints how many values
  are on the stack and leaves them alone. Which word must you change, and what
  one line do you add?],
  [Write `twice ( x lambda -- y )`, which runs a lambda two times on a value.
  `5 { 3 * } twice` should leave 45. The lambda is needed twice and so is
  nothing else.],
  [Using `rate` as a model, write `moor ( n -- )`, which sets the mooring fee by
  rewriting `fee`.],
))
