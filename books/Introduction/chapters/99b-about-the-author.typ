#import "../design.typ": *

#pagebreak(weak: true)

#hide(heading(
  level: 1, numbering: none, outlined: true, bookmarked: true,
  "About the Author",
))

#v(1cm)
#{
  set par(first-line-indent: 0pt)
  text(font: body_family, size: 9pt, tracking: 2pt, fill: ink_gray, upper("Afterword"))
  v(4mm)
  text(font: body_family, size: 36pt, weight: "regular", fill: ink_black, "About the author")
}
#v(6mm)
#line(length: 100%, stroke: 0.5pt + ink_rule)
#v(8mm)

#grid(
  columns: (56mm, 1fr),
  gutter: 7mm,
  [
    #portrait()
    #v(2mm)
    #align(center, text(font: body_family, style: "italic", size: 9pt, fill: ink_gray, "Vladimir Ulogov."))
  ],
  [
    Vladimir Ulogov has spent decades building infrastructure for distributed
    systems — the kind of software that watches other software. Early in his
    career he worked on monitoring and telemetry platforms; later years took him
    into federated observability, telemetry buses, and the architecture of
    systems that have to make sense of millions of data points without losing
    the thread.

    Observability is a discipline of _legible state_: of insisting that a
    system be able to show what it holds and how it came to hold it. It is not
    an accident that the language he built keeps everything it knows on a stack
    where you can look at it, and that its programs are values you can take
    apart.
  ],
)

#v(4mm)

He wrote his first programs for a PDP-11, and what makes him slightly unusual in
his corner of the industry is a tendency, ever since, to write his own tools:
not small utilities, but programming languages and formats. Bund is one of
them. Its first implementation lives in a series of Rust crates —
`rust_dynamic`, `rust_multistack`, `rust_multistackvm`, `bundcore` — each a
building block that exists because the off-the-shelf options did not fit the
shape of the work. The Bund of this book is the second implementation: the same
language, rebuilt from the ground up and checked, program by program, against
the first.

#section("A work of love")

Bund is open source. You can read it, build it, study it and change it; the
terms are in the project's repository. It was not designed as a product. It is
a language made by someone who wanted it to exist, offered to anyone who finds
that a stack, a table and a few quays are a good way to think.

#section("A note on cooperation")

Vladimir believes firmly in the human capacity for mutual help — that we make
better work, and live better lives, when we share what we know and what we
build. Open source is one of the most concrete expressions of cooperation our
era has produced: code read, improved, and passed forward by people who will
never meet. If this book leaves you able to teach a machine one word it did not
know, that is enough.

#section("Where to find more")

/ *GitHub*: `@vulogov` — the source for Bund and the Rust crates beneath it. Issues and pull requests welcome.
/ *LinkedIn*: `/in/vladimirulogov` — posts on observability, the occasional long-form essay.
/ *YouTube*: `@vulogov` — talks and walkthroughs.

#v(4mm)

#text(font: body_family, style: "italic", size: 11pt, fill: ink_gray,
  "If a word in this book does not do what the page says, open an issue on GitHub. The author reads them."
)

#v(8mm)
#align(center, text(font: body_family, size: 8pt, fill: ink_faint, tracking: 4pt,
  upper("end of the book")))
