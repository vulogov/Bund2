#import "../design.typ": *

#let title = "Questions"

#let intro = [
#dropcap("A") question leaves `true` or `false`, and a decision picks one of
them up. Two habits save most of the trouble. Read a comparison from the
word backwards: `30 <` asks "is 30 less than it?". And remember that a
question uses up what it asks about, so `dup` first if you still need it.

#plate("04-scales.png", "It tips one way or the other.")

#words((
  ("==  !=", "( a b -- flag )", [Equal; different.]),
  (">  <", "( a b -- flag )", [Is b greater than a; is b less than a.]),
  (">=  <=", "( a b -- flag )", [Is b at least a; is b at most a.]),
  ("not  and  or", "", [The opposite; both; either.]),
  ("if", "( flag lambda -- )", [Run the lambda when the flag is true.]),
  ("ifthenelse", "( flag else then -- )", [The lambda on top runs when the
  flag is true.]),
))
]

#let problems = (
  (id: "c04p1", title: "Five questions", grade: 1,
   ask: [Write down the answer to each, then ask Bund: `30 40 >`, `40 30 >`,
   `30 30 <=`, `"wax" "salt" ==` and `12 large? not`.],
   hint: [`30 40 >` asks whether 40 is greater than 30.],
   answer: [True, false, true, false, true. The top is the subject of every
   question: `40 30 >` asks about the 30.]),

  (id: "c04p2", title: "Odd and even", grade: 1,
   ask: [Using `rem` from Problem 14, define `even?` and then `odd?`.
   `40 even?` is true, `25 even?` is false and `25 odd?` is true.],
   hint: [An even number leaves nothing over when divided by 2.],
   answer: [`odd?` is not written out again. It is `even?` and `not`.]),

  (id: "c04p3", title: "Three states of an account", grade: 2,
   ask: [Define `account ( pennies -- )`. It prints `owing` for an amount
   below nought, `paid` for nought and `in credit` for anything above.],
   hint: [Two questions, one inside the other. Settle nought first; then only
   two cases remain, and they can be a word of their own.],
   answer: [`state` asks whether the amount is nought and keeps a copy. If it
   is, the copy is dropped and `"paid"` put down. If not, `debt` asks the
   second question of the copy. Both courses leave one string, and `account`
   prints it.]),

  (id: "c04p4", title: "A tariff in three steps", grade: 3,
   ask: [The harbour tries a new tariff. A cargo of ten units or fewer pays
   no quay due. Above ten and up to thirty it pays 3 pennies a unit. Above
   thirty it pays only 2 a unit. Every ship still pays the mooring fee.
   Define `tariff ( units -- pennies )`: 8 and 10 units pay 20, 25 units pay
   95 and 40 units pay 100.],
   hint: [`dup 10 <` asks whether there are more than ten. The lambda on top
   is the one that runs when there are.],
   answer: [`rate` is the tariff above ten units, and `tariff` decides whether
   to use it. Both leave the stack alike, one number in place of the units,
   and `fee` is said once, after the decision, because it does not depend on
   it.]),

  (id: "c04p5", title: "Days of rest", grade: 2,
   ask: [The harbour numbers the days of the week from 1. On days 6 and 7 the
   customs house is shut. Define `rest-day? ( day -- flag )` and `open?`.],
   hint: [Two questions about one number: `dup`, ask, `swap`, ask.],
   answer: [After `dup 6 ==` the flag is on top of the day, so `swap` brings
   the day up for the second question, and `or` joins the two flags.]),

  (id: "c04p6", title: "Within bounds", grade: 2,
   ask: [A clerk's slip gives a cargo as −5 crates, another as 250. No ship
   in Wendhaven carries fewer than none or more than a hundred. Define
   `clamp ( n -- n )`, which leaves a number alone if it lies between 0 and
   100 and otherwise leaves the nearer bound.],
   hint: [Two small words, one for each bound, each an `if`.],
   answer: [`floor0` replaces a number below nought with 0, `cap` replaces one
   above a hundred with 100, and `clamp` is the two said together. `0 >` asks
   "is 0 greater than it?".]),
)
