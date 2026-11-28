#import "../design.typ": *

#plain_title("Before you begin", "How to Use This Book")

#dropcap("N")obody learns a stack language by reading one. You learn it the
way the clerk at Wendhaven learned his trade: by lifting crates until your
hands know where each one is. _Bund, Word by Word_ showed you the words. This
book makes you use them, seventy-three times.

It is a book of problems and nothing else. There is no new story and almost
no new vocabulary. The harbour is the same harbour, the three ships are the
same three, and the day still comes to 291 pennies.

#section("What you need")

You need to have read the first book, or to be willing to keep it open
beside this one. Every problem may use any word that book taught, whichever
chapter of this one it stands in.

You need the first book's terminal, started in this book's `code` folder:

```
$ cd books/solving_problems/code
$ bund2 ../../Introduction/code/terminal.bund
```

And you need the harbour's words. The first book had you build them a
chapter at a time: `dues`, `large?`, `ship`, `arrivals` and the rest. They are
gathered in one file, `harbour.bund`, which is printed in Appendix A. Learn
them as soon as the terminal greets you:

```
"harbour.bund" study
```

After that the terminal knows what the first book left it knowing.

#session("c00-setup", caption: "The harbour's words, ready")

Every screen in this book was recorded from a terminal started exactly so.

#section("How the book is laid out")

It is in three parts, and the order matters.

*The problems* come first, in twelve chapters. The chapters follow the first
book: stack, arithmetic, words, decisions, loops, text, lists, records, the
table and the quays. Two more go a little past it, and the last is six whole
programs. Each chapter opens with a reminder of the words it leans on.

*The hints* come next: one nudge for a problem, never the answer. A hint
tells you where to look.

*The answers* come last. Every one was run, and what you see is what Bund
printed. Under each screen are a few lines on what to notice in it.

#section("How hard")

Each problem is marked with crates.

#{
  set par(first-line-indent: 0pt)
  grid(columns: (22mm, 1fr), row-gutter: 3.2mm, align: horizon,
    grade(1), [A line. If you know the chapter's words, you can do it.],
    grade(2), [A word or two of your own, and some thought about what is on
    top.],
    grade(3), [A small design. Expect to get it wrong first.],
  )
}

#section("How to work")

Try each problem at the terminal before you look at anything. Type `.s` when
you are lost; it costs nothing. When you are stuck, read the hint and go back
to the terminal. Read the answer last, and read it even when you were right.

#callout(label: "Is my answer right?")[
  If yours leaves the same thing on the stack, or prints the same lines, and
  leaves nothing else behind, it is right. The answer in the book is one
  good answer. Where yours is shorter, yours is better.
]

Two rules will save you most of your mistakes, and both are the first
book's.

*The top speaks first.* Every word takes its first operand from the top of
the stack. `10 3 -` is −7. When an answer comes out backwards, this is why,
and `swap` is the cure.

*Keep words short.* The terminal takes one line at a time, and a definition
must fit on it. When a word will not fit, do not look for a longer line.
Break the word in two and give each half a name. Nearly every three-crate
problem in this book is solved that way.
