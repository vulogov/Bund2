#import "../design.typ": *

#plain_title("Afterword", "About This Book")

#dropcap("T")he three earlier books each ended with a program that worked.
This one was written from the other end. Every program in it was made to
fail first, in one particular way, and the chapter was then written from
what the binary did about it.

That method found things the author of the book did not expect. The report
at a terminal is not the report in a pipe, and the first book had shown only
the second. A watch stops a moment before the value it is watching for is
on the stack. A condition that cannot be evaluated says so once and steps
aside. The first book's own ledger reckons every due twice. None of these
was in the three earlier books, and each is on a screen in this one,
recorded and not composed.

#section("How the screens were made")

The debugger's screens were recorded on a pseudo-terminal, a real terminal
as far as `bund2` can tell, 73 columns wide. The prompts and the echo of
each line typed are the program's own. Two changes were made to what it
printed. The folder the book was built in is shown as `/home/you/Bund2`.
And the long line that lists the debugger's commands is cut short after
Chapter 6 has shown it whole.

Times in Chapter 10 are from one run on one machine and will not be yours.
Calls and steps will be.

#section("What is not here")

Programs spread over several machines have a view of their own,
`debug.display_distributed_info`, and their own ways of going wrong. A
`bund2` built with the `jit` feature has a second way of running a word,
with `--stats` to report on it. Neither is in this book, the first because
it needs a book of its own and the second because the `bund2` these screens
came from was built without it.

#section("Where to go next")

`bund_examples/profiler_demo` goes through every feature of
`debug.profile` on a program that takes snapshots of itself from six calls
deep. `bund2 help script` lists the flags, and is short.

The best exercise is the one the book was written by. Take a program you
trust. Break it in one small way, close the file, and come back tomorrow to
find the fault with these tools alone, without reading the source. It is a
humbling quarter of an hour, and after a few of them a real fault looks
much less like a mystery.
