#import "../design.typ": *

#let title = "Rounds"

#let intro = [
#dropcap("F")our words repeat a thing, and they differ in what stops them.
In all four, whatever you are building rides underneath, and each round works
above it and leaves it where it was.

#words((
  ("times", "( n lambda -- )", [n rounds. Before each, the round's number is
  pushed, counting from 0.]),
  ("loop", "( list lambda -- )", [One round for each element, with the element
  on top.]),
  ("while", "( flag lambda -- )", [While the flag is true: run the lambda,
  which must leave the next flag.]),
  ("for", "( lambda -- )", [Run first, ask afterwards. The lambda leaves the
  flag.]),
))
]

#let problems = (
  (id: "c05p1", title: "One to a hundred", grade: 1,
   ask: [Add up the whole numbers from 1 to 100 with `times`. The answer is
   5050.],
   hint: [The counter starts at nought. The total starts underneath it.],
   answer: [Each round finds the total with the counter on top of it. `1 +`
   turns the counter 0 … 99 into 1 … 100, and the second `+` adds it in.]),

  (id: "c05p2", title: "A table of dues", grade: 2,
   ask: [Print the dues for cargoes of 10, 20, 30, 40 and 50 units, one to a
   line, as `10 units: 50`.],
   hint: [Make the cargo from the counter. It is needed twice: once to print
   and once to reckon.],
   answer: [`1 + 10 *` makes 10, 20 … from the counter. `dup print` shows a
   copy, and `dues println` uses up the original, so the stack is empty after
   every round.]),

  (id: "c05p3", title: "Doubling", grade: 2,
   ask: [An unpaid penny doubles every week. With `for`, print the debt each
   week until it first passes 291. Then, with `while`, leave only the
   _number_ of weeks that took.],
   hint: [For the count, keep it under the debt. Each round must reach
   beneath the debt to add one.],
   answer: [Nine weeks. In the second line the stack is _count debt_.
   `swap 1 + swap` brings the count up, adds one and puts it back, and the
   closing `drop` throws away the debt, which is no longer wanted.]),

  (id: "c05p4", title: "Salt and wax", grade: 3,
   ask: [The crier calls the crates numbered 1 to 15 off a ship. Every third
   crate is salt and every fifth is wax, so for those he calls `salt` or `wax`
   in place of the number, and for the fifteenth, `salt wax`. Print his
   fifteen calls.],
   hint: [`rem` again. Ask about three first. Each answer leads to a second
   question about five, and each of those is a small word.],
   answer: [`cry` asks whether the number is a multiple of three and hands it
   to one of two words. `both` knows it is, and only asks about five. `plain`
   knows it is not, and replaces the number with `"wax"` if five divides it.
   Whichever ran, one value is left, and `cry` prints it.]),

  (id: "c05p5", title: "How many are even", grade: 1,
   ask: [Count the even numbers in `[ 40 12 25 7 30 ]` with `loop` and a
   running tally. There are three.],
   hint: [Each round turns its element into 1 or 0 and adds it to the tally
   underneath.],
   answer: [`even?` uses up the element and leaves a flag; the decision turns
   the flag into 1 or 0. Remember which lambda is which: the one on top,
   `{ 1 }`, is for true.]),

  (id: "c05p6", title: "The hailstone", grade: 3,
   ask: [Take a number. If it is even, halve it; if odd, treble it and add
   one. Go on until you reach 1. Define `steps ( n -- count )`, the number of
   rounds that takes. From 6 it takes 8. How many from 27?],
   hint: [One word for a single step, one for "again?", and the count kept
   under the number.],
   answer: [111. Nobody has proved that every starting number comes down to 1
   at last; it is called the Collatz problem. The word is four small ones:
   `step` does the arithmetic, `tally` adds one to the count underneath,
   `again?` asks whether to go on, and `steps` sets the count and the first
   flag and drops the 1 at the end.]),
)
