#import "../design.typ": *

#let title = "Lists"

#let intro = [
#dropcap("A") list is one value that holds many, and most of the work on it
is done by handing it to a word together with a lambda. This chapter adds two
words the first book did not use, `head` and `tail`, which cut a list short.
Both take the count first and the list on top of it.

#plate("07-drying-rack.png", "In a row, counted from one end.")

#words((
  ("map", "( list lambda -- list )", [A new list of what the lambda leaves for
  each element.]),
  ("loop", "( list lambda -- )", [Run the lambda for each element.]),
  ("car  cdr", "", [The first element; all but the first.]),
  ("sort  unique", "( list -- list )", [In order; without repeats.]),
  ("head", "( n list -- list )", [The first n elements.]),
  ("tail", "( n list -- list )", [The last n elements.]),
))
]

#let problems = (
  (id: "c07p1", title: "A sum of squares", grade: 1,
   ask: [Make the list of the squares of 1 to 5, and then add it up. The
   answer is 55.],
   hint: [`map` for the squares; a running total and `loop` for the sum.],
   answer: [Two lines, two ideas: `map` makes a list from a list, and
   `0 swap { + } loop` boils a list down to one number. The `swap` puts the
   starting total under the list.]),

  (id: "c07p2", title: "First and last", grade: 1,
   ask: [Three ships came in, in the order `"Marta" "Gull" "Osprey"`. Leave a
   list of the first two. Then leave the last one, as a name and not as a
   list. Then define `last ( list -- x )`.],
   hint: [`tail` gives a list even when it holds one thing.],
   answer: [`1 … tail` leaves a list of one element, and `car` takes the
   element out. Inside `last` the list arrives first, so `swap` puts the count
   under it.]),

  (id: "c07p3", title: "Is it among them?", grade: 3,
   ask: [Define `among? ( list x -- flag )`: is `x` one of the elements? It is
   true for `"wax"` in `[ "salt" "wax" "herring" ]` and false for `"wool"`.],
   hint: [Start with `false` under the list, and let each round `or` its own
   answer into it. The thing sought has to be at hand in every round: keep it
   on the table.],
   answer: [`x` goes to the counting table and stays there for the whole
   loop. Each round compares its element with a copy from `peek` and joins the
   answer to the flag underneath. `take drop` clears the table at the end.]),

  (id: "c07p4", title: "Kinds of cargo", grade: 1,
   ask: [Six ships brought `"salt" "wax" "salt" "herring" "wax" "salt"`.
   Define `kinds ( list -- n )`, the number of different cargoes.],
   hint: none,
   answer: [`unique` removes the repeats, and what is left is counted.]),

  (id: "c07p5", title: "In order", grade: 1,
   ask: [Put the three ships' names in alphabetical order. Then, with `sort`,
   leave the smallest of `[ 40 12 25 ]`, and after that the largest.],
   hint: [After sorting, the smallest is first and the largest is last.],
   answer: [`sort` orders text as well as numbers. The smallest is the `car`
   of the sorted list and the largest is its `tail` of one.]),

  (id: "c07p6", title: "More than a hundred", grade: 2,
   ask: [From `[ 40 12 25 ]` make the list of dues. Then print only those
   dues that are more than 100 pennies.],
   hint: [Both courses of the decision must use up the copy: one prints it,
   the other drops it.],
   answer: [`dear` keeps a copy for the question. `100 <` asks "is 100 less
   than it?"; the lambda on top prints when it is, and the other drops.]),
)
