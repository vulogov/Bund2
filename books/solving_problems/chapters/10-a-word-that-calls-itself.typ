#import "../design.typ": *

#let title = "A Word That Calls Itself"

#let intro = [
#dropcap("T")he first book never did this, and it is the oldest trick in
stack languages: a word may say its own name. Bund looks a word up when it
runs, not when it is written, so nothing special is needed. What is needed is
a way out. Every such word asks a question first, and one of the answers must
not lead back in.

#plate("10-spiral.png", "Each turn is the one before it, made smaller.")

The classic is the Fibonacci numbers: 0, 1, 1, 2, 3, 5, 8 … each the sum of
the two before it. Here it is in Bund, as `code/fib.bund`.

#listing("fib.bund")

Read it with the first book's rule. `dup 1 <` asks "is 1 less than n?". If it
is not, n is 0 or 1 and is its own answer: the `if` does nothing and n stays.
If it is, the lambda runs. `dup 1 swap - fib` leaves n with fib(n−1) on top of
it; `swap 2 swap - fib` brings n up and turns it into fib(n−2); and `+` adds
the two. The word says its own name twice.

#words((
  ("\"fib.bund\" study", "", [Learn `fib` from the file.]),
  ("rem  over", "", [From Problems 14 and 49. This chapter uses both.]),
))
]

#let problems = (
  (id: "c10p1", title: "Casting off", grade: 1,
   ask: [Define `countdown ( n -- )`, which prints n, then n−1, and so on down
   to 1, and then `cast off`. It must call itself and use no loop.],
   hint: [Two courses: stop, or print one number and go again with one less.
   Each can be a word, and one of them will name `countdown` before
   `countdown` exists. That is allowed.],
   answer: [`go` was defined first and mentions `countdown`, which did not yet
   exist. Bund did not mind, because it looks for the word only when `go`
   runs. `dup 0 <` asks "is 0 less than n?"; when it is not, `end` is the way
   out.]),

  (id: "c10p2", title: "The factorial", grade: 1,
   ask: [The factorial of n is n times the factorial of n−1, and the
   factorial of 1 is 1. Define `fact`. `5 fact` leaves 120. What is
   `20 fact`?],
   hint: [It has the shape of `fib`, with one call in place of two.],
   answer: [2432902008176640000. When n is more than 1, a copy of it is
   lowered by one and handed to `fact`, and the answer is multiplied by the n
   that waited underneath. Twenty calls are waiting, each with its n, before
   the first multiplication is done.]),

  (id: "c10p3", title: "The first eleven", grade: 2,
   ask: [Learn `fib` from `fib.bund`. Before running anything, work out
   `4 fib` on paper, writing down every call. Then leave `10 fib`, and then
   the list of the Fibonacci numbers from the 0th to the 10th.],
   hint: [On paper, `4 fib` needs `3 fib` and `2 fib`; `3 fib` needs `2 fib`
   and `1 fib`. Notice how often the same small question is asked.],
   answer: [`4 fib` is 3, and it asks for `2 fib` twice and `1 fib` three
   times. A word you defined can be handed to `map` like any other, which
   gives the whole row in one line.]),

  (id: "c10p4", title: "Counting the calls", grade: 2,
   ask: [How many times does `fib` run to reckon `5 fib`? And `10 fib`, and
   `20 fib`? Change the word so that it counts its own calls.],
   hint: [Put a 0 on the counting table before you start, and let the word's
   first act be `1 +.`.],
   answer: [15 calls for the fifth number, 177 for the tenth, 21891 for the
   twentieth. The work grows as fast as the numbers themselves, because each
   call asks two more questions and remembers none of the answers. The count
   lives on the table, out of the way of a word that is busy with the
   stack.]),

  (id: "c10p5", title: "The quick way", grade: 3,
   ask: [Reckon the same numbers without asking any question twice. Keep two
   neighbouring Fibonacci numbers on the stack and step them forward n times.
   Define `fib-fast`. `10 fib-fast` leaves 55. What is `90 fib-fast`, which
   the slow word would not finish in your lifetime?],
   hint: [From _a b_ the next pair is _b_ and _a + b_. `over` makes that
   easy. Start from 0 and 1.],
   answer: [2880067194370816120, at once. `next` turns _a b_ into _b a+b_.
   `fib-fast` sets n aside, puts down 0 and 1, takes n back for `times`, and
   steps the pair n times, dropping each round's counter. The answer is the
   lower of the two, so the last word is `drop`.]),

  (id: "c10p6", title: "The greatest common measure", grade: 3,
   ask: [Euclid's rule: the greatest number that divides both a and b is b
   itself when the remainder of a by b is nothing, and otherwise it is the
   greatest that divides b and that remainder. Define `gcd ( a b -- n )`.
   `140 56 gcd` leaves 28 and `95 140 gcd` leaves 5. What do 291 and 56
   share?],
   hint: [Let the way out be "b is 0": then the answer is a. Otherwise go
   again with b, and the remainder of a by b.],
   answer: [Only 1. `turn` makes the next pair: `swap over` leaves _b a b_,
   and `rem` turns the top two into the remainder, so _b_ and the remainder
   are left. When the second number has come down to 0, `drop` removes it and
   the first is the answer.]),

  (id: "c10p7", title: "A list, taken apart", grade: 2,
   ask: [Define `sum ( list -- n )`, which adds up a list and calls itself to
   do it: the sum of a list is its first element plus the sum of the rest,
   and the sum of an empty list is 0.],
   hint: [`len` tells you whether the list is empty, and leaves it. `car` and
   `cdr` take it apart.],
   answer: [`split` leaves the first element with the rest of the list on top.
   `sum` adds the first to the sum of the rest. The way out is the empty
   list, which `list` puts down on the screen's last line. `loop` does this
   job better; the point is that a list has the same shape as the word that
   walks it.]),
)
