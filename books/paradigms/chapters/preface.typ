#import "../design.typ": *

#plain_title("Preface", "Why Four Ways")

#dropcap("M")ost languages come with an opinion about how a program should
be thought of. One says a program is a list of orders. Another says it is a
set of functions, another a society of objects, another a body of facts and
the questions asked of it. A language usually holds one of these opinions,
and holds it firmly.

Bund holds none of them. It is a stack language first: values go down, words
pick them up, the top speaks first. On that floor it lets you build in any of
the four manners, and in the same file if you like. Its author put it plainly
in the first Bund's own description:

#callout(label: "From Bund's README")[
  "Rather than enforcing adherence to a single paradigm, I opted to provide
  developers with the flexibility to choose."
]

A choice is only worth having if you can tell the options apart. This book
is for that. It takes one job, the day's ledger at Wendhaven, and does it
four times: by commands, by functions, by objects and by rules. Each version
prints the same seven lines. Then it changes the job, twice, and watches
what each version has to do about it.

You will not be told which way is best. You will see what each one costs
and what it buys, on a job small enough to hold in your head.

#section("What you need")

You need _Bund, Word by Word_, the first book of this harbour, or the
knowledge it gives: the stack, words of your own, lists, records, the
counting table. This book begins where that one's Chapter 14 left off: at
the four ways of thinking it named and did not teach.

You need its terminal, started in this book's `code` folder:

```
$ cd books/paradigms/code
$ bund2 ../../Introduction/code/terminal.bund
```

The four programs, and a fifth that mixes them, are in that folder. Every
screen in the book was recorded from them.
