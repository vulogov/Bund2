#import "../design.typ": *

#let title = "Words of Your Own"

#let intro = [
#dropcap("A") word is a name for a piece of program: an atom, a lambda,
`register`. A good one is short, does one thing, and has a stack effect you
can say aloud. Write the effect down before you write the word.

#plate("03-tool-rack.png", "A small tool, on its own peg, with its own name.")

The terminal takes a definition on one line. When a word will not fit on a
line, that is the language telling you it is two words.

#words((
  (":name { ... } register", "", [Make `name` a word.]),
  ("alias", "( name new -- )", [Give a word a second name.]),
  ("peek", "( -- x )", [From `harbour.bund`: a copy of what lies on the
  counting table.]),
))
]

#let problems = (
  (id: "c03p1", title: "Half and quarter", grade: 1,
   ask: [Define `half ( n -- n )`, and then `quarter` without writing a
   number in it. `140 half` leaves 70 and `140 quarter` leaves 35.],
   hint: [In `half` the 2 arrives on top, and the top is what `/` divides.],
   answer: [`2 swap /` puts the number on top of the 2 before dividing. A
   quarter is half of a half.]),

  (id: "c03p2", title: "What is left over", grade: 3,
   ask: [Bund has no word for the remainder of a division. Write one:
   `rem ( n d -- r )`. `291 4 rem` leaves 3, `40 15 rem` leaves 10 and
   `12 3 rem` leaves 0, and nothing else is left behind. Keep this word.
   Later problems use it.],
   hint: [The remainder is n − (n ÷ d) × d. That uses n twice and d twice.
   Put d on the counting table, where `peek` can copy it.],
   answer: [#words((
     (".", "", [d goes to the table. The stack holds n.]),
     ("dup peek swap /", "", [A copy of n, a copy of d, and n on top: n ÷ d.]),
     ("take *", "", [d comes back for good and multiplies the quotient.]),
     ("swap -", "", [n on top, so that the product is taken from it.]),
   ))
   The table held one value while both hands were busy, and was left
   clear.]),

  (id: "c03p3", title: "The tithe", grade: 2,
   ask: [A tenth of every due goes to the town. Define `tithe ( pennies --
   pennies )`, and `after-tithe`, which leaves what remains for the harbour.
   `140 tithe` leaves 14; `140 after-tithe` leaves 126.],
   hint: [`after-tithe` needs the amount twice: once to tithe, once to
   subtract from.],
   answer: [`dup tithe` leaves the amount with its tithe on top of it. The
   amount must be on top for `-`, so `swap`.]),

  (id: "c03p4", title: "A second name", grade: 1,
   ask: [Give `dues` the second name `reckon`. Then raise the mooring fee to
   25 and try `40 reckon`. Did the new name follow the change? Put the fee
   back to 20 afterwards.],
   hint: [`alias` takes the word's name and then the new one.],
   answer: [`reckon` is `dues`, and `dues` looks for `fee` each time it runs.
   So the new name answers 145 as soon as `fee` is changed, without being
   told.]),

  (id: "c03p5", title: "Powers", grade: 1,
   ask: [Define `square`, then `cube` using `square`, then `sixth` using both
   and nothing else. `3 cube` leaves 27 and `2 sixth` leaves 64.],
   hint: [A sixth power is the cube of a square.],
   answer: [`cube` keeps a copy of the number under its square, and
   multiplies. `sixth` has no stack words in it at all: it is two words said
   one after the other, which is what _concatenative_ means.]),

  (id: "c03p6", title: "Shillings and pence", grade: 2,
   ask: [Twelve pennies make a shilling. Define `shillings` and `odd-pence`,
   and then `coins ( pennies -- )`, which prints `24s 3d` for 291 and
   `11s 8d` for 140.],
   hint: [`odd-pence` is one use of `rem`. For the printing, `format` fills
   the first name in its template from the top of the stack.],
   answer: [`both` leaves the odd pence with the shillings on top of them,
   which is the order the template `"{s}s {d}d"` asks for: its first name is
   filled from the top.]),
)
