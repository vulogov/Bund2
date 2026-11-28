#import "../design.typ": *
#chapter(number: 11, title: "The Other Quays")

#dropcap("W")endhaven has more than one quay. Salt is landed at one, fish at
another, wax and fine goods at a third, and for good reason: nobody wants a
barrel of herring set down on a crate of beeswax. Each quay has its own pile.
The clerk walks from one to the next, and at any moment he stands at exactly
one of them.

Every stack language has a stack. Bund has as many as you care to name. This is
the feature the language is called after, and it is the answer to a warning
that has now been given twice: that `fold` and `gather`, like `*+`, take the
_whole_ stack, whatever happens to be on it.

#plate(10, "10-three-quays.png", "Three quays, three piles, and one clerk who stands at one of them.")

#section("Another pile")

An at-sign and a name takes you to the stack of that name. If there is no such
stack, Bund makes one.

#session("ch11-quay", caption: "Two stacks")

Two values went onto the first stack. `@quay` moved you to a stack called
`quay`, and `.s` shows it empty: the 40 and the 12 are not here. A 25 was put
down on this one. Then `@main` took you back, and the 40 and the 12 were
waiting exactly as you left them.

#term("Named stack")[
  A stack with a name. `@name` makes it the _current_ stack, creating it if
  need be. Every word works on the current stack and no other. The stack you
  start on is called `main`.
]

Nothing else has changed. `dup`, `+`, `println`, `loop` and every word you have
written do what they always did, on whichever stack you are standing at. A
stack you are not standing at cannot be disturbed, because no word can reach
it.

The word `current` tells you where you are.

#session("ch11-current", caption: "Where am I?")

And `stack_exists` tells you whether a stack of a given name has been made,
without going there.

#session("ch11-exists", caption: "Is there such a quay?")

#words((
  ("@name", "", [Go to the stack called `name`, making it if it does not exist.]),
  ("current", "( -- name )", [The name of the current stack.]),
  ("stack_exists", "( name -- flag )", [Is there a stack of this name?]),
))

#section("Carrying between quays")

If stacks cannot see each other, how does a value get from one to another? On
the workbench. There are many stacks and one table, and the table goes where
you go.

#session("ch11-carry", caption: "Across, on the table")

The 40 was put on the table at `main`, carried to `quay`, and taken off there.
It is on `quay` now and gone from `main`.

#piles(
  ("main", ("40",)),
  (". @quay", ()),
  ("take", ("40",)),
  note: "From one stack to another by way of the workbench.",
)

That is the whole of it. `. @there take` carries one value across. You already
knew both halves.

#section("A stack of one's own")

Now the warning can be lifted. `gather` takes everything on the stack. So give
it a stack on which there is nothing but what it should take.

The file for this chapter begins with a word that does exactly that.

#listing("quays.bund")

Read `manifest`. Go to a stack called `manifest`. Put the day's arrivals on it
and gather them: nothing else is on this stack, so nothing else is gathered.
Put the list on the table. Go back to `main` and take it.

#session("ch11-manifest", caption: "manifest disturbs nothing")

There were two values on the stack before `manifest` ran, and they are still
there, underneath the answer. In Chapter 9 `arrivals gather` would have
swallowed them. `manifest` has the effect `( -- list )` and means it.

This is the idiom, and it is worth learning as a shape:

```
@workplace        go somewhere empty
  ...             do the untidy work
  .               put the answer on the table
@main take        go back and pick it up
```

A word written this way can use `fold`, `gather`, `*+`, `unfold` or `*loop` as
freely as it likes, and its caller never knows.

#warn(label: "Come back to where you started")[
  A word that changes the current stack and does not change it back leaves its
  caller standing on the wrong quay, and every word after it works on the wrong
  pile. The words in this book return to `main`. That is right for a program
  that lives on `main`, and it is the one thing to check first when a word that
  uses stacks misbehaves: `current`.
]

#section("Sorting the arrivals")

Stacks are not only for hiding work. They are places to keep things, and
sometimes a program has several kinds of thing to keep.

A large cargo is unloaded at the wide quay and a small one at the narrow.
`to-wide` and `to-narrow` each carry one record across and come back.
`berth` looks at a record and chooses.

#session("ch11-berth", caption: "Each ship to her quay")

After the loop, `main` is empty. The _Marta_ is at the wide quay, and the
_Gull_ and the _Osprey_ are at the narrow one.

The word that printed them is new. `*loop` is `loop` for a stack instead of a
list: it runs its lambda for each value on the current stack, from the top,
until the stack is empty. It is one more word that takes everything, and on a
stack that holds only ships, everything is what you want.

Look at `berth` once more, because it is the decision of Chapter 5 put to real
use. It `dup`s the record, since the test will use one copy up. It asks the
question. Then two lambdas, the unusual case last: to the narrow quay, unless
she is large.

#words((
  ("manifest", "( -- list )", [The day's arrivals, gathered on a stack of their own.]),
  ("to-wide", "( record -- )", [Carry a record to the stack `wide`.]),
  ("to-narrow", "( record -- )", [Carry a record to the stack `narrow`.]),
  ("berth", "( record -- )", [A large cargo to `wide`, a small one to `narrow`.]),
  ("*loop", "( ... lambda -- )", [Run the lambda for each value on the current stack,
  top first, until it is empty.]),
))

#section("Walking round the quays")

Bund keeps its stacks in a ring, in the order they were made, and you can walk
round the ring without naming anything.

#session("ch11-rotate", caption: "One step at a time")

Three stacks were made, and you were left standing at the last, `wax`. Each
`->` steps to the one made before it: `fish`, `salt`, and then `main`, where
you began.

#callout(label: "For the curious")[
  Walking the ring is for programs that treat their stacks as a set of
  alike things — a queue for each customer, a pile for each day. For the kind
  of program in this book, go by name. A name says where you are going, and a
  step only says that you have moved.
]

#words((
  ("->", "", [Step to the stack made before the current one.]),
))

#section("How many stacks?")

As few as make the program clear. One for the main line of work. One, briefly,
for any job that makes a mess. One each for kinds of thing that must not be
mixed. A program with twenty stacks is as hard to follow as one with twenty
global variables, and for the same reason.

But do use them. A newcomer from Forth will try to do everything on one stack
with ever more ingenious shuffling, and will write words nobody can read,
their author included. In Bund, when a stack gets crowded, you do not shuffle. You
go and stand somewhere else.

You now have every piece of the ledger. Chapter 12 puts them into one file and
runs it as a program.

#recap((
  [`@name` makes the stack of that name the *current* one, creating it if needed.
  You start on `main`. Every word works on the current stack only.],
  [`current` names the stack you are on.],
  [Values cross between stacks on the workbench: `. @there take`.],
  [For work that takes the whole stack, go to a stack of its own, do the work,
  put the answer on the workbench, and come back: `manifest` is the pattern.],
  [`*loop` runs a lambda over every value on the current stack until it is empty.],
  [`->` steps round the ring of stacks. Prefer names.],
  [A word that changes the current stack must change it back.],
))

#problems((
  [Put `1 2 3` on `main` and `10 20` on a stack called `spare`. Without retyping
  any number, finish with the sum of all five on `main`.],
  [Define `to-quay ( x name -- )`, which carries a value to the stack whose name
  is on top and returns to `main`. The word `to_current` takes a name from the
  stack and goes to that stack, which must already exist.],
  [Rewrite `largest` from Chapter 8 so that it is safe to call with other values
  on the stack.],
  [After `manifest { berth } loop`, how many ships are at the narrow quay? Write
  a line that leaves the number on `main`.],
  [Define `reversed ( list -- list )` safely, using a stack of its own.],
))
