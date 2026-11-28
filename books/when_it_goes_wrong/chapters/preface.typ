#import "../design.typ": *

#plain_title("Preface", "Why a Book About Failure")

#dropcap("E")very harbour keeps two books. One is the ledger, which says
what came in and what was paid. The other is thinner and less often shown to
visitors. It records the day the crane dropped a cask, the night a ship went
onto the bar, the morning the totals would not agree. A harbour that keeps
the second book well has fewer entries to make in it.

#plate("00-mishap-book.png", "The other book, less often shown to visitors.")

The first three books of this series were about the ledger. This one is
about the other book. It assumes you can write a Bund program, and it starts
at the moment the program does something you did not mean.

That moment comes to everybody, and more often in a stack language than in
most. There are no names on the stack to tell you what a value is. A word
that takes one value too few, or leaves one too many, says nothing at the
place where it happens. The trouble shows up later, somewhere else, in a
word that was written correctly.

Bund2 has a great deal of machinery for this, and most of it has never been
written about for a beginner. There is a report that draws the failing line.
There is a checker that finds a short stack without running anything. There
is a trace of every call. There is a debugger that stops a running program
and lets you type Bund into it, with breakpoints, watches and a way of
catching a failure at the instant it is raised. There is a profiler that
counts every step. This book goes through all of it, on programs small
enough to hold in your head.

#section("What you need")

The same as before: `bund2`, version 0.57.0 or later, and the book's `code`
folder, which is `books/when_it_goes_wrong/code`. The terminal of the first
book is used again for the short experiments, and is started from that
folder:

```
$ cd books/when_it_goes_wrong/code
$ bund2 ../../Introduction/code/terminal.bund
```

Every screen in the book was recorded from the binary. The debugger's
screens were recorded on a real terminal, with the prompts and the echo as
you will see them.

#section("How to read it")

Chapter 0 sorts trouble into three kinds, and the rest of the book follows
that order. If you are here because something is broken now, Chapter 11 is a
table of symptoms and the tool for each. Appendix A has every flag, every
console command and every `debug` word on a few pages.

Break things as you read. Each faulty program in the `code` folder has a
mended twin in `code/mended`, so you can always see what the cure was.
