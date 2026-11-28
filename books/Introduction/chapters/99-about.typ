#import "../design.typ": *

#plain_title("Afterword", "About This Book")

#dropcap("T")his book kept one harbour's ledger because that is how a language
is learned: not by reading what every word does, but by needing a word and
finding it there, or finding it missing and writing it. Wendhaven was never the
point. The point was the _order_: put a value down before you say what to do
with it; know what is on the stack before every word; name a thing as soon as
you have done it twice; and when the stack gets crowded, go and stand somewhere
else.

If there is a single idea to carry out of these pages, it is the one the first
chapter opened with and the appendix closes on: *the top speaks first.* It
looked backwards in Chapter 3. By Chapter 9 it was writing your `set` for you,
and by Chapter 12 you had stopped noticing it.

#section("Where to go next")

This book stopped at the edge of the language's foundations on purpose. Beyond
it Bund has objects and classes; logic programming, with rules, queries and
constraints; graphs and their algorithms; statistics, models and data frames;
and virtual machines that run side by side and send each other messages. Each
is made of words that take values from a stack and leave values on it, top
first, and each has worked examples in the `bund_examples` folder of the
project.

Three commands are worth keeping at your fingertips as you go:

```
$ bund2 terminal.bund             # a place to try a word
$ bund2 words                     # every name Bund answers to
$ bund2 check --file mine.bund    # will it run short?
```

The first keeps you experimenting. The second tells you whether the word you
are about to write already exists. The third finds the commonest fault in a
stack program before the program is run.

#section("Colophon")

Typeset with Typst in Kyiv Machine, a typewriter face, with terminal screens
and program text in DejaVu Sans Mono. Every screen was recorded from #book_version by a small tool that types
the lines of a session into the book's terminal and writes down what comes
back; none was typed by hand. Two alterations were made to them: the folder the
book was built in is shown as `/home/you/Bund2`, and a line too long for the
page is folded. The harbour of
Wendhaven, its ships and its dues are invented: a scaffold for the language,
not a place you can sail to.

#v(1.2cm)
#brand_mark(width: 26mm)
#v(6mm)
#align(center)[
  #text(font: body_family, size: 11pt, style: "italic", fill: ink_gray)[
    Now teach it a word of your own.
  ]
  #v(3mm)
  #text(font: body_family, size: 10pt, fill: ink_smoke, book_author)
]
