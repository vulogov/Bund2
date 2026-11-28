#import "../design.typ": *

#plain_title("Afterword", "About This Book")

#dropcap("O")ne page of seven lines, printed five times, is a small thing to
build a book on. It was kept small so that the differences could be seen.
On a job of this size no manner is hard. What differs is where each puts the
knowledge of the harbour: in the order of the steps, in a table, in the
things themselves, or in a list of what is so. Everything else in the book
followed from that.

#section("Where to go next")

Each manner goes much further than this book took it.

Objects in Bund have multiple inheritance, built-in base classes that give
every object an identity and a way of printing itself, and classes that wrap
a single value. The `object_oriented` folder of `bund_examples` goes through
all of it.

Deduction has tabling for rules that would otherwise go round for ever,
questions put to the person at the keyboard, and constraints over numbers
that solve puzzles by narrowing down. The `logic_demo` and
`logic_programming` folders show them, the second by solving the Zebra
Puzzle twice.

Functions have the whole of the first book's lists and lambdas behind them,
and the problem book's chapters on words that call themselves and on
programs as values are practice in exactly this manner.

#section("Colophon")

Typeset with Typst in Kyiv Machine, a typewriter face, with terminal screens
and program text in DejaVu Sans Mono. Every screen was recorded from
#book_version by a small tool that types the lines of a session into the
terminal, or runs a command, and writes down what comes back; none was typed
by hand. Two alterations were made to them: the folder the book was built in
is shown as `/home/you/Bund2`, and a line too long for the page is folded.
The harbour of Wendhaven, its ships and its tariff are invented.

#v(1.2cm)
#brand_mark(width: 26mm)
#v(6mm)
#align(center)[
  #text(font: body_family, size: 11pt, style: "italic", fill: ink_gray)[
    A step, a value, a thing, or a fact?
  ]
  #v(3mm)
  #text(font: body_family, size: 10pt, fill: ink_smoke, book_author)
]
