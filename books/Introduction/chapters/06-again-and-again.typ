#import "../design.typ": *
#chapter(number: 6, title: "Again and Again")

#dropcap("T")he treadwheel crane on the quay at Wendhaven does one thing. A man
walks inside the wheel, the rope winds, a barrel comes up out of the hold and
is set down. Then it does the same thing again. Nobody tells it "now the second
barrel, now the third". It goes on until the hold is empty.

A clerk who must be told about each ship separately is no better than one who
cannot add. This chapter gives Bund four ways of doing a thing more than once:
a fixed number of times, once for each thing in a row, for as long as something
is true, and until something is.

#plate(6, "06-crane.png", "The same step, until the hold is empty.")

#section("A fixed number of times")

`times` takes a number and a lambda, and runs the lambda that many times.

#session("ch06-times", caption: "Three times")

The lambda here is only `println`, and yet something was printed: 0, then 1,
then 2. Before each run, `times` puts the number of the run on the stack. It
counts from nought, as programmers do, so three runs are numbered 0, 1 and 2.

That counter is there whether you want it or not.

#session("ch06-counter", caption: "The counter is always put down")

In the first half the lambda ignored the counter, and three of them were left
on the stack. In the second half the lambda begins with `drop`, and the stack
is clean afterwards.

#warn(label: "times leaves you a counter each round")[
  Every time round, `times` pushes the run's number, starting at 0. Your lambda
  must use it or `drop` it. If the stack grows mysteriously in a loop, look for
  a counter nobody picked up.
]

#words((
  ("times", "( n lambda -- )", [Run the lambda n times. Before each run, push the
  run's number: 0, 1, … n−1.]),
))

#section("Once for each thing in a row")

Counting is seldom what you want. You want to do something _to each ship_. For
that Bund needs the ships in a row, and a row of values is written between
square brackets. It is called a _list_, and Chapter 8 is all about them. For
now, this is enough: `[ 40 12 25 ]` is one value that holds three.

The word `loop` takes a list and a lambda. For each thing in the list, in
order, it puts that thing on the stack and runs the lambda.

#session("ch06-loop", caption: "Each in turn")

The lambda is `println`, and it was run three times, each time with the next
number on top of the stack. Put a word of your own in its place:

#session("ch06-dues", caption: "The dues of every ship")

That is the day's reckoning in one line, and it does not mention the number of
ships. Add a fourth to the list and it is reckoned with the rest.

The same works for any word that takes one value:

#session("ch06-notes", caption: "The size of every cargo")

#words((
  ("[ ... ]", "( -- list )", [Put down a list of the values between the brackets.]),
  ("loop", "( list lambda -- )", [For each element in order: push it, run the
  lambda.]),
))

#section("Keeping a running total")

Printing each due is not the same as adding them up. For a total, something has
to stay on the stack _between_ one round and the next.

#session("ch06-total", caption: "A total that rides underneath")

Follow it through. `0` goes down first: the total so far. Then the list and the
lambda, which `loop` picks up, leaving only the 0. Now each round:

#piles(
  ("start", ("0",)),
  ("40", ("0", "40")),
  ("dues", ("0", "140")),
  ("+", ("140",)),
  note: "The first round. The total waits underneath while the ship is reckoned.",
)

The 40 is pushed on top of the total. `dues` turns it into 140 without touching
what is underneath. `+` adds the two. One value is left, the new total, exactly
where the old one was, and the next round begins. After three rounds it is 291.

This is the most useful pattern in the language. *Put the thing you are building
at the bottom, and let each round work above it.* The lambda's stack effect is
`( total units -- total )`: it takes the total and one more ship, and leaves
the total.

#section("For as long as something is true")

Sometimes you do not know how many rounds there will be. The carter takes
fifteen crates on each trip. How many trips to clear the _Marta_'s forty?

#session("ch06-while", caption: "Carting until the quay is clear")

Three trips, and on the last the cart is five short of full.

`while` needs explaining with some care. It takes a lambda, and under the
lambda a boolean. If the boolean is `true` it runs the lambda and then looks
for a _new_ boolean on top of the stack, which the lambda must have left. It
goes on until it finds `false`.

So the lambda has two duties: do the work, and finish by answering "again?".
Here is this one, piece by piece, starting with 40 on the stack:

#words((
  ("15 swap -", "", [Take 15 from the count. The `swap` puts the count on top, so
  that it is the count that is subtracted _from_.]),
  ("dup println", "", [Print what is left, keeping a copy.]),
  ("dup 0 <", "", [Leave the answer to "is 0 less than what is left?", keeping
  the count under it.]),
))

The `true` typed before the lambda is the first answer: it lets the loop start.
If it is `false`, the lambda never runs at all.

#session("ch06-never", caption: "false at the start: nothing happens")

#warn(label: "A while loop that forgets to answer")[
  If the lambda does not leave a boolean on top, `while` will pick up whatever
  _is_ on top and complain that it is not one — or, worse, find a stray `true`
  and go round for ever. Write the lambda's last three words first: they are
  the question.
]

#section("Until something is")

`for` is `while` turned round. It runs the lambda _first_ and then looks for
the boolean, so the lambda always runs at least once and no starting answer is
needed.

The harbour doubles the fine for an unpaid due each week. Starting at one
penny, how does it grow before it passes a hundred?

#session("ch06-for", caption: "Doubling until it passes a hundred")

The lambda prints the fine, doubles it, and ends with `dup 100 >`: "is 100
greater than the fine?". While that is true, round it goes again. When the fine
reaches 128 the answer is `false`, the loop stops, and 128 is left on the stack.

#words((
  ("while", "( flag lambda -- )", [While the flag on top is `true`: run the lambda,
  which must leave the next flag.]),
  ("for", "( lambda -- )", [Run the lambda, which must leave a flag; repeat while
  the flag is `true`.]),
))

#section("Which one?")

#words((
  ("loop", "", [You have the things in a list. This is the one you will use most.]),
  ("times", "", [You know the number of rounds and want the round's number.]),
  ("while", "", [You stop on a condition, and may not need to start.]),
  ("for", "", [You stop on a condition, and must do it once to find out.]),
))

If you reach for `while` to walk through a list, stop and use `loop`. A loop
that counts for itself is a loop that can miscount.

The clerk can now go down the quay unaided. What he cannot yet do is write a
decent line in the ledger: `140` is a number, and the ledger wants "Marta, 40
crates of salt, 140 pennies". Text is next.

#recap((
  [`n { ... } times` runs a lambda n times and pushes the run's number, from 0,
  before each run. Use the counter or `drop` it.],
  [`[ ... ] { ... } loop` runs a lambda once for each element of a list, with the
  element on top of the stack.],
  [To build a total, put the starting value at the bottom and let each round
  work above it: `0 [ 40 12 25 ] { dues + } loop` leaves 291.],
  [`flag { ... } while` repeats while the flag is true. The lambda must end by
  leaving the next flag.],
  [`{ ... } for` runs first and asks afterwards, so it always runs once.],
))

#problems((
  [Print the numbers 1 to 5, one to a line, using `times`.],
  [Using `loop` and a running total, count how many of the three cargoes are
  large. The answer is 1.],
  [Using `loop`, print only the large cargoes in `[ 40 12 25 ]`. Remember that a
  test uses up what it tests, and that both courses must leave the stack alike.],
  [The _Osprey_'s 25 barrels are rolled away four at a time. Use `while` to print
  what is left after each trip, and say how many trips it takes.],
  [Use `for` to find the first power of 3 that is greater than 500.],
))
