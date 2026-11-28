#import "../design.typ": *

#plain_title("Afterword", "About This Book")

#dropcap("S")eventy-three problems is not many. It is enough to find out
that the difficulty in a stack language is never the word you do not know. It
is the crate you forgot was there. Every answer in this book is short, and
most of the hard ones became easy at the moment a long word was broken into
two small ones with honest names.

That is the habit to keep. Write the stack effect first. Keep each word to a
line. When a result comes out backwards, remember who speaks first.

#section("Where to go next")

The last two chapters of problems went a step past the first book, into
words that call themselves and programs handled as values. Beyond them lie
the parts of Bund this book did not touch: objects and classes, rules and
queries, graphs, statistics and data frames, and virtual machines that send
each other messages. Each has worked examples in the `bund_examples` folder
of the project, and the Fibonacci word of Chapter 10 is one of them: it is
the program the interpreter is timed with.

#section("Colophon")

Typeset with Typst in Kyiv Machine, a typewriter face, with terminal screens
and program text in DejaVu Sans Mono. Every screen was recorded from
#book_version by a small tool that types the lines of a session into the
terminal and writes down what comes back; none was typed by hand. One
alteration was made to them: a line too long for the page is folded.
The harbour of Wendhaven, its ships and its dues are invented.

#v(1.2cm)
#brand_mark(width: 26mm)
#v(6mm)
#align(center)[
  #text(font: body_family, size: 11pt, style: "italic", fill: ink_gray)[
    Now set yourself a seventy-fourth.
  ]
  #v(3mm)
  #text(font: body_family, size: 10pt, fill: ink_smoke, book_author)
]
