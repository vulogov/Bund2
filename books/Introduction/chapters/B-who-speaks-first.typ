#import "../design.typ": *
#appendix(letter: "B", title: "Who Speaks First, on One Page")

When you want the rule without the story, this is it.

#callout(label: "The rule")[
  A word takes its *first* operand from the *top* of the stack. Read from the
  word backwards: the word, then the value beside it, then the one before that.
]

#section("In arithmetic")

#chord_table((
  chord_row("10 3 -", "−7. The top, 3, is what you subtract from: 3 − 10."),
  chord_row("3 10 -", "7. To take three from ten, put the ten on top."),
  chord_row("4 20 /", "5. Twenty divided by four."),
  chord_row("20 4 /", "0. Four divided by twenty, in whole numbers."),
  chord_row("a b swap -", "a − b. swap turns the sum round when the values are already there."),
))

#section("In questions")

#chord_table((
  chord_row("30 40 >", "true. \"Is 40 greater than 30?\""),
  chord_row("40 30 >", "false. \"Is 30 greater than 40?\""),
  chord_row("units 30 <", "\"Is 30 less than the units?\" — that is, are there more than 30."),
  chord_row("x 100 >", "\"Is 100 greater than x?\" — that is, is x still under 100."),
))

#section("In decisions")

#chord_table((
  chord_row("flag { A } if", "A runs when the flag is true."),
  chord_row("flag { B } { A } ifthenelse", "A, on top, runs when true. B runs when false."),
))

#section("In text and lists")

#chord_table((
  chord_row("\"haven\" \"Wend\" +", "Wendhaven. The string on top comes first."),
  chord_row("[ 40 12 ] [ 25 ] ++", "[ 25 40 12 ]. The list on top comes first."),
  chord_row("c b a template format", "The first {name} in the template is filled from a, the nearest."),
))

#section("In records")

#chord_table((
  chord_row("dict :key value set", "The value is on top, the key under it, the dictionary under that."),
  chord_row("dict :key get", "The key is on top, the dictionary under it."),
  chord_row("value dict swap :key swap set", "Writing a field when the value is underneath the record."),
))

#section("In loops")

#chord_table((
  chord_row("n { A } times", "The lambda is on top, the count under it. A finds the round's number on the stack."),
  chord_row("list { A } loop", "The lambda is on top, the list under it. A finds the element on the stack."),
  chord_row("0 list { + } loop", "What you are building goes down first and waits underneath."),
  chord_row("flag { A } while", "The lambda is on top, the first flag under it. A must leave the next flag."),
))

#section("When a result is the wrong way round")

#piles(
  ("10 3", ("10", "3")),
  ("-", ("-7",)),
  ("10 3 swap", ("3", "10")),
  ("-", ("7",)),
  note: "Wrong sign, a zero from a division, a phrase back to front: swap, then the word.",
)

There are three cures, in the order you should reach for them.

*Write the operands the other way.* If you are typing them, type the one the
word should start from last.

*`swap`.* If they are already on the stack, exchange them.

*Name it.* If you find yourself writing `swap -` often, there is a word waiting
to be defined. `spoiled ( units bad -- units )` in the answers to Chapter 4 is
`swap -` with a name that says which is which. After that nobody, including
you, has to think about it again.

#callout(label: "The one rule")[
  The last crate set down is the first crate lifted, and the clerk reads the
  label of the crate in his hands first. It is the same in arithmetic, in
  questions, in decisions, in text and in records: in every word this book has
  used. One rule in place of a dozen is the whole merit of it.
]
