#import "../design.typ": *

#let title = "Text"

#let intro = [
#dropcap("A") ledger is read by people, and people want words. Text in Bund
joins top first: `"haven" "Wend" +` is `Wendhaven`. A template is filled the
same way, its first `{name}` from the top of the stack. And `len` leaves the
string under its length, so there is usually a `swap` or a `drop` nearby.

#words((
  ("+", "( a b -- text )", [Join, the top string first. A number may be
  joined to a string.]),
  ("*", "( text n -- text )", [Repeat.]),
  ("len", "( x -- x n )", [The length. Leaves the value.]),
  ("format", "( values template -- text )", [Fill the template's names, the
  first from the top.]),
  ("string.upper  string.title", "", [Capitals; a capital on each word.]),
  ("string.tokenize", "( text -- list )", [The words of a string.]),
  ("convert.to_string", "( x -- text )", [Text from any value.]),
))
]

#let problems = (
  (id: "c06p1", title: "A boxed title", grade: 2,
   ask: [Define `boxed ( text -- )`, which prints its text between two bars,
   with a line of dashes above and below exactly as long as the middle line.
   Try it on `"Wendhaven"` and on `"Marta"`.],
   hint: [The middle line is four characters longer than the text. `len`
   leaves the text, so a word that prints a line of dashes can hand it back.],
   answer: [`edge ( text -- text )` prints the dashes and keeps the text.
   `boxed` uses it twice: before the row, with a `dup` so that `row` may use
   up a copy, and after it, followed by a `drop` now that the text is
   finished with.]),

  (id: "c06p2", title: "The wrong way up", grade: 2,
   ask: [`"Marta" "salt" 40` is on the stack, in that order. Without typing
   any of the three again, fill the wording
   `"{name}: {units} crates of {cargo}"` to read
   `Marta: 40 crates of salt`.],
   hint: [The first name in a template is filled from the top, so the name
   must be on top, the units under it and the cargo at the bottom.],
   answer: [The three must go from _name cargo units_ to _cargo units name_:
   the bottom one comes to the top. `. swap take swap` does it, with the
   units set aside on the table while the other two change places.]),

  (id: "c06p3", title: "Counting words", grade: 1,
   ask: [How many words are there in `"the gull brought wax and herring"`?
   Leave the number, and nothing else.],
   hint: [`len` leaves the list under the count.],
   answer: [`string.tokenize` makes a list of the words, `len` counts them,
   and `swap drop` removes the list.]),

  (id: "c06p4", title: "A list with commas", grade: 3,
   ask: [Define `listed ( list -- text )`, which joins a list of names with a
   comma and a space: `[ "Marta" "Gull" "Osprey" ] listed` leaves
   `Marta, Gull, Osprey`. There must be no comma before the first name or
   after the last.],
   hint: [Start the text with the first name, not with nothing: `car` and
   `cdr`. Then each round adds a comma and one more name to what is
   underneath.],
   answer: [`dup car swap cdr` leaves the first name with the rest of the list
   on top of it. Each round then finds _text name_ on the stack. `", " +`
   puts the comma before the name, since the top string comes first; `swap +`
   brings the text up so that it comes first in its turn.]),

  (id: "c06p5", title: "A straight column", grade: 2,
   ask: [Numbers in a ledger are written so that their last figures stand one
   under another. Define `pad ( n -- text )`, which gives a number as text
   six characters wide, with spaces before it. Print 140, 56, 95 and 291 in a
   straight column.],
   hint: [The number of spaces is six less the length. A string times a number
   repeats it.],
   answer: [`len 6 -` takes the length from 6. The spaces are made on top of
   the text, and the top string comes first, so `+` puts them before it.]),

  (id: "c06p6", title: "Ahoy", grade: 1,
   ask: [Define `loud ( text -- text )`, which puts text in capitals with an
   exclamation mark after it, and `hail ( name -- )`, which prints
   `AHOY, MARTA!` for `"marta"`.],
   hint: [To put something _after_ a string, it must be underneath: `swap`.],
   answer: [`"Ahoy, " +` goes before the name because it is on top. `"!"`
   has to go after, so `swap` brings the text up first.]),
)
