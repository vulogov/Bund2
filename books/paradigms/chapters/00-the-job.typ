#import "../design.typ": *
#chapter(number: 0, title: "The Job")

#dropcap("T")he town council of Wendhaven has changed the tariff. Until now
every crate and barrel paid three pennies to cross the quay, whatever was in
it. From this morning cargo pays by its kind.

#words((
  ("salt", "3 pennies", [a unit]),
  ("wax", "5 pennies", [a unit]),
  ("herring", "2 pennies", [a unit]),
  ("mooring", "20 pennies", [a ship, as before]),
))

The same three ships come in.

#words((
  ("Marta", "40 of salt", [3 × 40 + 20 = 140 pennies]),
  ("Gull", "12 of wax", [5 × 12 + 20 = 80 pennies]),
  ("Osprey", "25 of herring", [2 × 25 + 20 = 70 pennies]),
))

The day comes to 290 pennies, one fewer than under the old tariff. The
_Marta_ still owes her 140.

The job is to print the ledger. This is the page, and every program in this
book must print exactly this:

#session("c0-ledger", caption: "The page every version must print")

#section("Why the tariff had to change")

Under the old tariff every cargo was reckoned alike, and one word, `dues`,
served them all. A job in which everything is alike does not show the
paradigms apart. The new tariff puts a _kind_ into the reckoning: what a
ship owes now depends on what she carries. How a program says "it depends on
the kind" is where the four ways differ most.

#section("Two tests")

A program is not finished when it prints the right page. It is finished
when the harbour stops changing, which it does not. So each version will be
put to two tests.

#term("The fourth cargo")[
  Next week the _Heron_ arrives with 8 bales of wool, and wool pays 4 pennies
  a unit. She owes 52, and the day comes to 342. What must be changed in the
  program, and in how many places?
]

#term("The new question")[
  The harbourmaster asks something nobody planned for: _which ships owe more
  than 75 pennies?_ The answer is the _Marta_ and the _Gull_. Can the program
  answer, and what does it take?
]

The first test adds a new kind of thing. The second adds a new thing to
ask. They pull in different directions, and no way of writing a program is
equally good at both.

#section("How to read the chapters")

Each of the next four chapters has the same shape: the idea, the few words
Bund gives you for it, the whole program, a look at what the program knows
after it has run, and the two tests. Chapter 5 puts the four side by side.
Chapter 6 stops choosing, and uses each where it fits.

#recap((
  [Cargo now pays by kind: salt 3, wax 5, herring 2, and 20 to moor.],
  [The three ships owe 140, 80 and 70, and the day comes to 290.],
  [Every version must print the same page, and face the same two tests: a
  fourth cargo, and a new question.],
))
