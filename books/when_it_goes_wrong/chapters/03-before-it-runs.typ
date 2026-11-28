#import "../design.typ": *
#chapter(number: 3, title: "Before It Runs")

#dropcap("T")he harbourmaster reads a manifest before the ship is unloaded.
If it says forty casks and names thirty-nine, he has found the trouble
without lifting anything. `bund2` can read a program in the same way.

#section("check")

`check` reads a program and follows, word by word, how many values each one
needs and how many will be there. It runs nothing.

#session("c3-check", caption: "Three programs, checked")

For `slip.bund` it has a _finding_: at line 11, `dues` needs one value and
none can be proven to be there. That is the fault, found without the first
three lines of output being printed.

For `tare.bund` it has nothing to say. That program is wrong, as Chapter 0
said, but its stack is never short. `check` counts values. It does not know
what they mean.

For `ledger.bund` it is honest about its limits. It followed 28 places and
gave up at one, `manifest`, because that word switches stacks and its effect
cannot be written as "so many in, so many out". A clean report means no
fault was found in what could be followed.

#term("Stack effect")[
  How many values a word takes and how many it leaves, as a pair. `dup` is
  1 and 2. A word whose effect is not a fixed pair, because it depends on
  what it is given, is called _opaque_, and `check` cannot see past it.
]

#words((
  ("bund2 check", "--file FILE", [Report where the stack would run short,
  without running the program.]),
))

#section("The effects check knows")

`check` starts from what Bund's own words declare.

#session("c3-effects", caption: "What the built-in words declare")

More than eleven hundred words, each with what it takes and what it leaves.
`set` takes three and leaves one. `loop` and `format` are opaque: what they
do to the stack depends on the lambda or the template.

The same table can be asked from inside a program.

#session("c3-effect", caption: "Asking for one word's effect")

`?effect` takes a name and leaves a record. It knows the built-in words. For
a word you defined yourself it answers `NODATA`: your word's effect is
worked out by `check` from its body, and is not kept.

#words((
  ("bund2 effects", "", [Every built-in word, with what it takes and
  leaves.]),
  ("?effect", "( name -- record )", [The declared effect of a built-in
  word.]),
))

#section("infer")

`check` works an effect out. `infer` goes a step further and tests the
working-out: it runs the program, then runs each word you defined on
stand-in numbers and compares what it saw with what it had worked out.

#session("c3-infer", caption: "Inferred effects, compared with observed ones")

Five of the six words agree. `entry` could not be observed on the stand-in
numbers, and `infer` says so and does not guess. The odd line `0: 1 pennies` is `entry`
running on stand-in numbers. `infer` really does run your words, so do not
point it at a program that sends letters.

A word marked `WRONG` would mean `check` had been reasoning from a false
effect, and its findings about that word could not be trusted.

#words((
  ("bund2 infer", "--file FILE", [Run the program, then compare each word's
  inferred effect with the one observed.]),
))

#section("At the gate")

The first book ended by building a program into a file of its own. `build`
can be made to check first.

#session("c3-build", caption: "A build that looks before it packs")

`build --check` reads and analyses and writes nothing. With `--strict` a
finding refuses the build. It is off by default because a finding can be
mistaken where a program uses `!` freely, and you should not be locked out
of your own harbour by a cautious clerk.

#callout(label: "A habit worth having")[
  `bund2 check --file` takes a moment and finds the commonest fault there
  is. Run it before you run the program, every time, and you will meet the
  rest of this book less often.
]

#recap((
  [`bund2 check --file` finds a short stack without running anything, and
  says how much of the program it could follow.],
  [It counts values and knows nothing of their meaning: a program that is
  wrong but never short passes.],
  [`bund2 effects` and `?effect` show what the built-in words declare.],
  [`bund2 infer --file` runs the program and tests the effects `check`
  worked out.],
  [`bund2 build --check --strict` refuses to pack a program with a
  finding.],
))
