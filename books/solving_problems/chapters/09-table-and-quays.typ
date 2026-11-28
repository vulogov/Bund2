#import "../design.typ": *

#let title = "The Table and the Quays"

#let intro = [
#dropcap("W")hen two hands are not enough, Bund gives you somewhere to put
things down. The counting table holds a value while you work on what was
beneath it. Another quay, a stack with a name, gives a word a clear floor to
work on when it needs to sweep up everything in sight.

One table serves every quay. That is how a value is carried from one stack to
another: set it on the table, walk, take it.

#words((
  (".  take", "", [To the table; back from it.]),
  ("peek", "( -- x )", [From `harbour.bund`: a copy of the table's top.]),
  ("+.", "( n -- )", [Add n to the number on the table.]),
  ("@name", "", [Stand at the stack `name`, making it if need be.]),
  ("fold", "( ... -- list )", [The whole current stack as a list, top
  first.]),
  ("gather", "( ... -- list )", [From `harbour.bund`: the whole current stack
  as a list, in the order it was put down.]),
))
]

#let problems = (
  (id: "c09p1", title: "Three old friends", grade: 2,
   ask: [Other stack languages are born with these three. Build them:
   `over ( a b -- a b a )`, `pair ( a b -- a b a b )`, and
   `rot ( a b c -- b c a )`.],
   hint: [For `over`, set the top aside, copy what is under it, and bring the
   top back. `pair` needs no table of its own.],
   answer: [`pair` is `over` twice. Forth calls it `2dup`, and you may be
   tempted to. Do not: Bund reads `2dup` as the number 2 followed by `dup`,
   and `1 2 2dup` leaves `1 2 2 2`.]),

  (id: "c09p2", title: "The larger of two", grade: 2,
   ask: [With `pair`, define `larger ( a b -- n )`. Then use it in a `loop` to
   find the largest of `[ 40 12 25 ]` without sorting.],
   hint: [Ask the question of copies, and then drop the loser. For the list,
   the largest so far rides underneath.],
   answer: [`pair <` asks, of copies, whether the top is the smaller. If it
   is, `drop` removes it; if not, `swap drop` removes the other. In the loop
   the 0 is the first "largest so far", which is safe only because no cargo is
   below nought.]),

  (id: "c09p3", title: "Sum and count in one pass", grade: 2,
   ask: [Go through `[ 140 56 95 ]` once, and finish with the sum and the
   number of elements both on the stack. Then leave their average.],
   hint: [Two running values and one stack to keep them under the element.
   Put one of them on the table.],
   answer: [The sum rides on the stack and the count on the table, where `+.`
   adds to it without lifting it. `take` brings the count up at the end, and
   `swap /` divides the sum by it.]),

  (id: "c09p4", title: "One crate, wrapped", grade: 3,
   ask: [Define `wrap ( x -- list )`, which leaves a list holding just `x`,
   whatever else is on the stack. Then `append ( list x -- list )`, which adds
   `x` at the end. Two helping words will serve you for the rest of the
   chapter: `own`, which carries the top value to a stack called `work`, and
   `back`, which carries the top value home to `main`.],
   hint: [`fold` makes a list of everything on the stack. On a stack that
   holds one thing, that is a list of one.],
   answer: [`own` is `. @work take`: to the table, across, and down again.
   `back` is the same journey home. Between them `fold` sees a stack with one
   value on it. `"mine"`, which was on `main` all along, is untouched.
   `append` wraps and joins, with a `swap` because `++` puts the top list
   first.]),

  (id: "c09p5", title: "One to n", grade: 2,
   ask: [Define `range ( n -- list )`, the list of whole numbers from 1 to n.
   `5 range` leaves `[ 1 2 3 4 5 ]`. Check it by adding the list up.],
   hint: [`times` will put the numbers on a stack for you, if the stack is
   your own. `gather` then makes them a list in order.],
   answer: [On the `work` stack, `{ 1 + } times` leaves 1, 2 … n lying there,
   since each round's counter is turned into a number and not picked up.
   `gather` makes the list and `back` carries it home.]),

  (id: "c09p6", title: "A sieve for cargoes", grade: 3,
   ask: [`map` must have one value from every round, so it cannot leave
   things out. Define `keep-large ( list -- list )`, which keeps only the
   large cargoes: from `[ 40 12 25 31 30 ]` it leaves `[ 40 31 ]`.],
   hint: [On a stack of your own, a `loop` may leave a value or not, as it
   likes. Then gather what is lying there.],
   answer: [`sift` leaves a large cargo where it is and drops any other. Run
   over the list on the `work` stack, that leaves only the large ones, and
   `gather` makes a list of them. This is the pattern for every word that
   picks some things out of many.]),
)
