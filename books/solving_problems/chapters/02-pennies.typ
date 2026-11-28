#import "../design.typ": *

#let title = "Pennies"

#let intro = [
#dropcap("T")he harbour reckons in whole pennies, and its clerk reads the
crate in his hands first. Every problem here turns on that: in `a b -` the
top, `b`, is what is subtracted _from_, and in `a b /` it is what is divided.

#words((
  ("+  *", "( a b -- n )", [Add; multiply. The order does not matter.]),
  ("-", "( a b -- b-a )", [Subtract a from b.]),
  ("/", "( a b -- b/a )", [Divide b by a. Two whole numbers give a whole
  number.]),
  ("*+", "( ... -- sum )", [Add up everything on the stack.]),
))
]

#let problems = (
  (id: "c02p1", title: "A rebate", grade: 1,
   ask: [The _Marta_ brought 40 crates at 3 pennies each, and the harbour
   allows her a rebate of 15. Reckon what she owes for the cargo in one line.
   The answer is 105.],
   hint: [When the 15 goes down it is on top, and the top is what `-`
   subtracts from.],
   answer: [`40 3 *` leaves 120. Then `15` is on top of it, and `15 120 -`
   would be the wrong sum, so `swap` puts the 120 on top first.]),

  (id: "c02p2", title: "Four sums", grade: 2,
   ask: [Write down what each of these leaves, then try them:
   `2 3 4 + *`, `2 3 + 4 *`, `10 2 8 - /` and `2 8 - 60 /`.],
   hint: [Work from the first word, not from the first number. Each word
   takes the two values nearest to it.],
   answer: [14, 20, 0 and 10. The third is the trap: `2 8 -` is 6, and
   `10 6 /` divides 6 by 10, which in whole numbers is nothing. The fourth
   divides 60 by that same 6.]),

  (id: "c02p3", title: "Four officers", grade: 2,
   ask: [The day's 291 pennies are shared equally among four officers of the
   harbour. How much does each receive, and how many pennies are left on the
   table?],
   hint: [What is left is the whole, less four shares.],
   answer: [72 each, and 3 over. The second line multiplies the share back up
   to 288 and then puts 291 on top of it, so that `-` takes 288 from 291.]),

  (id: "c02p4", title: "A third of a penny", grade: 1,
   ask: [A late fine brings the day to 292 pennies. Divide it by three ships,
   first in whole pennies and then exactly.],
   hint: [A whole number divided by a whole number is a whole number. Make
   one of them a float.],
   answer: [Writing `292.0`, or converting with `convert.to_float`, is enough:
   when either number is a float the answer is one. The harbour's ledger stays
   in whole pennies on purpose, so that it always adds up.]),

  (id: "c02p5", title: "Everything on the quay", grade: 1,
   ask: [`140 56 95` is on the stack. Add all three with one word. Then, on a
   fresh stack, reckon the three ships' dues from their cargoes, 40, 12 and
   25, in a single line that ends with that same word.],
   hint: [Put down three quay dues and three mooring fees. Six numbers.],
   answer: [`*+` adds whatever is there, however much of it there is. That is
   its use and its danger: anything else left on the stack is added too.]),

  (id: "c02p6", title: "Squares", grade: 2,
   ask: [By how much does the square of 3 + 4 exceed the sum of the squares
   of 3 and 4? Leave both on the stack, look at them with `.s`, and then
   subtract. The answer is 24.],
   hint: [A square is `dup *`. Before you subtract, ask which of the two is on
   top.],
   answer: [49 went down first and 25 is on top of it. `-` alone would take 49
   from 25 and answer −24, so `swap` comes first.]),
)
