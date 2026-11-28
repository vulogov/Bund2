#import "../design.typ": *
#chapter(number: 5, title: "Decisions")

#dropcap("N")ot every ship is treated alike. A cargo of more than thirty units
is a _large_ cargo in Wendhaven: it goes through the wide door of the customs
house, it is inspected, and the ledger carries a mark against it. The clerk has
to look at a number and do one thing or another.

This chapter is about asking a question, getting `true` or `false` for an
answer, and acting on it. It is also where the rule of Chapter 3 pays its way,
because it governs all of it.

#plate(5, "05-two-doors.png", "One road, two doors.")

#section("Asking a question")

A comparison is a word like `+`: it picks up two values and leaves one. What it
leaves is a _boolean_, `true` or `false`.

#session("ch05-compare", caption: "Greater than, less than")

Now read them by the rule. *The top speaks first.* `30 40 >` has 40 on top, so
it asks "is 40 greater than 30?", and the answer is `true`. `40 30 >` asks "is
30 greater than 40?": `false`. And `40 30 <` asks "is 30 less than 40?".

#warn(label: "Read a comparison from the top")[
  `a b >` asks whether *b is greater than a*. Say the top value first, then the
  word, then the value underneath: "40 … greater than … 30". If a test seems to
  give the opposite of what you meant, this is why.
]

There are six comparisons, and all six read the same way.

#words((
  ("==", "( a b -- flag )", [Are they equal?]),
  ("!=", "( a b -- flag )", [Are they different?]),
  (">", "( a b -- flag )", [Is b greater than a?]),
  ("<", "( a b -- flag )", [Is b less than a?]),
  (">=", "( a b -- flag )", [Is b greater than or equal to a?]),
  ("<=", "( a b -- flag )", [Is b less than or equal to a?]),
))

Equality does not care which way up the values are, and it works on any kind of
value.

#session("ch05-equal", caption: "Equal and not equal")

#section("A question with a name")

"Is this a large cargo?" will be asked of every ship, so it should be a word.
By a custom Bund shares with its relatives, a word that answers a question has
a name ending in a question mark.

#session("ch05-large", caption: "large?")

The definition is `30 <`, and it rewards a slow reading. When `large?` runs,
the number of units is already on the stack. `30` goes on top of it. `<` then
asks, top first: "is 30 less than the units?" That is the same as asking
whether the units are more than 30.

#piles(
  ("40", ("40",)),
  ("30", ("40", "30")),
  ("<", ("true",)),
  note: "Inside large? — \"is 30 less than 40?\"",
)

Forty is large and twelve is not. Thirty is not either: the rule says _more
than_ thirty. Test a word like this at its edge. It is where the mistakes are.

#section("Acting on the answer")

The word `if` takes a boolean and a lambda. If the boolean is `true` it runs the
lambda. If not, it does nothing.

#session("ch05-if", caption: "Only when it is true")

The first line printed and the second did not. The lambda is on top and the
boolean under it, and `if` picks up both, so nothing is left behind either way.

This is why lambdas were introduced before decisions. The thing to be done
_perhaps_ cannot be done at once. It has to wait on the stack until `if` has
looked at the answer, and waiting on the stack is what a lambda is.

#callout(label: "For the curious")[
  `if` insists on a real boolean. A number will not do, not even 1 or 0.

  #session("ch05-notbool", caption: "A number is not an answer")

  If you have a number and want a decision, compare it with something.
]

#section("One thing or the other")

More often there are two courses and you want exactly one of them. The word is
`ifthenelse`, and it takes a boolean and _two_ lambdas.

#session("ch05-else", caption: "Then and else")

Forty is large, and the answer came back `large`, although `{ "small" }` is
written first. Once more: *the top speaks first.* The lambda on top is the one
for `true`.

#term("ifthenelse")[
  `flag { else } { then } ifthenelse`. The lambda on top, the one written
  _last_, runs when the flag is `true`. The one underneath runs when it is
  `false`. Read it from the word backwards: "if … then this … else that".
]

#piles(width: 21mm,
  ("large?", ("true",)),
  ("{ \"small\" }", ("true", "{ small }")),
  ("{ \"large\" }", ("true", "{ small }", "{ large }")),
  ("ifthenelse", ("large",)),
  note: "The lambda on top is the one for true.",
)

Write the unusual case first and the usual case last, and it reads naturally:
"small, unless it is large".

#words((
  ("if", "( flag lambda -- )", [Run the lambda when the flag is `true`.]),
  ("ifthenelse", "( flag else then -- )", [Run `then`, the top lambda, when the flag
  is `true`; otherwise run `else`.]),
))

#section("Two more words for the ledger")

Now the clerk can be taught to say which kind of cargo a ship carries.

#session("ch05-note", caption: "size and note")

`size` turns a number of units into the text `large` or `small` and leaves it
on the stack. `note` prints it. They are two words, not one, for the reason
given at the end of the last chapter: a word that leaves its answer on the
stack can be used by other words, and one that prints it cannot. Chapter 7 will
want the text without the printing.

#words((
  ("large?", "( units -- flag )", [Is this more than 30 units?]),
  ("size", "( units -- text )", [`"large"` or `"small"`.]),
  ("note", "( units -- )", [Print the size.]),
))

#section("Keeping what you tested")

`large?` picks up the number it tests. If you want the number afterwards, make
a copy before you ask.

#session("ch05-keep", caption: "dup before the question")

`dup` leaves two 40s. `large?` uses one of them, and the other is still there
for `println` inside the lambda. On the second line the cargo is small, the
lambda never runs, and the spare 12 is left on the stack, as `.s` shows.

That leftover is the trap in every decision. The two courses must leave the
stack the same, or what follows will find something it did not expect. Here the
`true` course uses up the number and the `false` course does not. An
`ifthenelse` with `{ drop }` for the other case would put it right.

#warn(label: "Both courses, the same stack")[
  Whichever way a decision goes, it should take the same things and leave the
  same things. Write the stack effect of each lambda. If they differ, the word
  has two effects, and the next word cannot know which one it got.
]

#section("Combining answers")

Booleans can be combined. `not` turns an answer round. `and` is true when both
are. `or` is true when either is.

#session("ch05-logic", caption: "not, and, or")

#words((
  ("not", "( flag -- flag )", [The opposite answer.]),
  ("and", "( a b -- flag )", [True when both are true.]),
  ("or", "( a b -- flag )", [True when at least one is true.]),
))

The clerk can now reckon a ship and judge its cargo. But there are three ships,
and you are still naming each one yourself. The next chapter has Bund go down
the quay on its own.

#recap((
  [A comparison picks up two values and leaves `true` or `false`. *The top speaks
  first*: `a b >` asks whether b is greater than a.],
  [A word that asks a question is named with a `?`: `large?` is `30 <`, "is 30
  less than the units?".],
  [`flag { ... } if` runs the lambda when the flag is true. The flag must be a
  boolean, not a number.],
  [`flag { else } { then } ifthenelse` runs one of two lambdas. The one on top,
  written last, is the one for `true`.],
  [A test uses up what it tests: `dup` first if you need the value again. Both
  courses of a decision should leave the stack alike.],
  [`not`, `and` and `or` combine answers.],
))

#problems((
  [Say what each leaves, then check: `5 9 <`, `9 5 <`, `5 5 >=`, `"a" "a" !=`.],
  [Define `empty? ( units -- flag )`, true when a ship brought nothing at all.],
  [Define `small? ( units -- flag )` in terms of `large?`.],
  [Define `fair? ( units -- flag )`, true for a cargo of more than 10 and at most
  30 units. You will need `dup` and `and`.],
  [Define `charge ( units -- pennies )`: a large cargo pays `dues` and a further
  10 pennies for inspection; a small one pays `dues` alone. The _Marta_ must
  come to 150 and the _Gull_ to 56.],
))
