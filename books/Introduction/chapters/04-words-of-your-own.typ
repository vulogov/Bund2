#import "../design.typ": *
#chapter(number: 4, title: "Words of Your Own")

#dropcap("O")n his first morning a new clerk has to be told everything. "Count
the crates. Take three pennies for each. Add twenty for the mooring. Write it
down." By the end of the week you say "reckon the _Gull_" and he does all of
it. Nothing about him has changed except his vocabulary: "reckon" has come to
mean those four orders in that sequence.

Bund learns the same way. This chapter teaches it the word `dues`, and with
that you have met the most important idea in the language: *a Bund program is a
set of words you taught it, each built from the ones before.*

#plate(4, "04-teaching.png", "You say it once. After that it has a name.")

#section("A piece of program on the stack")

Until now everything you typed was either put down or done at once. Curly
braces give you a third thing: a piece of program that is _put down_ instead of
being done.

#session("ch04-lambda", caption: "Words that wait")

Nothing was multiplied. The braces wrapped `3 *` into a single value and put
that value on the stack, where `.s` shows it: a _lambda_ holding the number 3
and the word `*`. It is a value like any other. You can `dup` it, `swap` it or
`drop` it.

#term("Lambda")[
  A piece of program written between `{` and `}`. Writing one does not run it.
  It goes onto the stack as a value, and runs only when a word asks it to.
  The name is borrowed from mathematics and means nothing more than "a function
  without a name".
]

The word that runs a lambda is `!`.

#session("ch04-run", caption: "Running what was put down")

`!` picks up the lambda from the top and does what is inside it, exactly as if
you had typed `3 *` at that moment. The 40 was underneath, so `*` found it.

#piles(width: 21mm,
  ("40 { 3 * }", ("40", "{ 3 * }")),
  ("!", ("120",)),
  note: "The lambda is a crate like any other, until ! opens it.",
)

#section("Giving it a name")

A lambda you must type afresh each time has saved you nothing. The word
`register` takes a name and a lambda and makes the name a word.

#session("ch04-register", caption: "Bund learns a word")

Read the first line from the left. `:due` puts down an atom, the name. The
braces put down a lambda. `register` picks up both and teaches Bund that `due`
means `3 *`. It leaves nothing on the stack, so there is no `=>` line.

From then on `due` is a word like `println`. `40 due` leaves 120 and `12 due`
leaves 36.

#words((
  ("{ ... }", "( -- lambda )", [Put down a piece of program without running it.]),
  ("!", "( lambda -- ... )", [Run the lambda on top.]),
  ("register", "( name lambda -- )", [Make the name a word that runs the lambda.]),
))

#callout(label: "A matter of form")[
  This book writes a definition as `:name { body } register`. When a definition
  is long, in a file, the body goes on lines of its own between the braces. At
  the terminal a definition has to fit on one line, which is a good discipline:
  a word that will not fit on a line is usually two words.
]

#section("Words made of words")

The mooring fee is a word too. And once there are two words, a third can be
made of them.

#session("ch04-dues", caption: "The day's reckoning, in words")

`dues` is defined as `due fee`: reckon the due, then add the fee. It contains
no numbers at all. Three ships are reckoned by three lines of two words each,
and `+ +` gives the day's total: 291, as it was by hand in the last chapter.

Look at what the definitions have done. `3 * 20 +` said _how_. `dues` says
_what_. Anyone reading `40 dues` knows what it is for, and the three pennies and
the twenty are each written down in exactly one place.

#section("What a word takes and what it leaves")

When you have more than a handful of words you need to know, for each, what it
expects to find on the stack and what it leaves there. Bund programmers write
that down in a small picture called a _stack effect_, which you have been
seeing in the word tables since Chapter 1.

#term("Stack effect")[
  `( before -- after )`: the values a word picks up, a dash, and the values it
  leaves. In both halves the top of the stack is on the _right_, as `.s` prints
  it. The names are for the reader and mean nothing to Bund.
]

The words of this chapter, each with its effect:

#words((
  ("due", "( units -- pennies )", [Three pennies for each unit of cargo.]),
  ("fee", "( pennies -- pennies )", [Add the 20-penny mooring fee.]),
  ("dues", "( units -- pennies )", [What a ship owes: `due fee`.]),
))

A stack effect is not checked by Bund. It is a promise you make to whoever
reads the word next, and that is usually you. Put it in a comment above every
word you keep:

```
// dues ( units -- pennies )   what a ship owes for its cargo and mooring
:dues { due fee } register
```

When a word's effect is hard to write down, the word is doing too much.

#section("When a word comes up short")

A word made of words fails the way its parts do.

#session("ch04-short", caption: "dues with nothing to reckon")

The error names `MUL`, the multiplication, which is two words down: `dues` ran
`due`, `due` ran `3 *`, and `*` found only one value. The `3` that `due` had
already put down is still on the stack.

This is the lesson of Chapter 1 again, one level deeper. A word is not a sealed
box. It is the words inside it, done in order, and when one of them fails, the
ones before it have already happened.

#section("Changing your mind")

Suppose the town council raises the due to four pennies. Teach Bund `due`
again.

#session("ch04-redefine", caption: "A new due, and the old dues follows")

Only `due` was redefined, and `dues` changed with it. That is because `dues`
does not hold a copy of what `due` meant on the day it was defined. It holds
the _name_, and looks the name up every time it runs.

#callout(label: "For Forth hands")[
  This is the opposite of Forth, where a definition is bound to the words as
  they stood when it was compiled, and redefining `DUE` leaves an old `DUES`
  untouched. In Bund a word is looked up when it is used. You can change one
  word in a running program and everything built on it follows.
]

It is a great convenience and it needs a little care. A name is shared by
everything that uses it. If you redefine a word, make sure the new one takes
and leaves the same things as the old.

#section("Asking about words")

You can ask Bund whether it knows a word, and what a word of yours is made of.

#session("ch04-ask", caption: "Is it a word? What is in it?")

`?word` answers `true` or `false`. `lambda=` puts the body of a word you
defined back on the stack, which is how you find out what `due` currently
means after an afternoon of changing it.

A word can have a second name. `alias` takes the existing name and the new one.

#session("ch04-alias", caption: "Another name for the same word")

#section("A word you do not run yet")

One more thing can be done with a word besides running it: you can put it down.
A backtick in front of a name makes a _pointer_ to that word.

#session("ch04-pointer", caption: "A pointer to a word")

#raw("`dues") did not reckon anything. It put a pointer on the stack, and `!` ran
it. A pointer is to a named word what braces are to a few loose ones: a way to
carry program around as a value. That will matter in the next two chapters,
where words take other pieces of program as their operands.

#words((
  ("?word", "( name -- flag )", [Does Bund know this word?]),
  ("lambda=", "( name -- lambda )", [The body of a word you defined.]),
  ("alias", "( name new -- )", [Give a word a second name.]),
  ("unregister", "( name -- )", [Forget a word you defined.]),
  ("`name", "( -- pointer )", [Put down a pointer to a word, without running it.]),
))

#section("How to write good words")

There is very little syntax in Bund, so nearly all the skill is in choosing
words. Three habits will serve you.

*Keep them short.* A word of three or four words is normal. If a definition
runs past a line, look for a part of it that deserves a name of its own.

*Name what, not how.* `dues` is a good name. `times3plus20` is a description of
its insides, and will be a lie the day the council meets.

*One word, one job.* `due` reckons. `fee` adds the mooring. Neither prints
anything. A word that reckons _and_ prints cannot be used by another word that
only wants the number.

So far Bund does the same thing for every ship. In the next chapter it learns
to tell a large cargo from a small one, and to act on the difference.

#recap((
  [`{ ... }` puts a piece of program on the stack as a *lambda*, without running
  it. `!` runs the lambda on top.],
  [`:name { ... } register` makes a new word. A word can be used in the
  definition of another: `dues` is `due fee`.],
  [A *stack effect*, `( before -- after )`, says what a word takes and leaves,
  top on the right. Bund does not check it. Write it anyway.],
  [A word is looked up by name each time it runs, so redefining `due` changes
  every word that uses it.],
  [`?word` asks whether a word exists, `lambda=` fetches its body, `alias` gives
  it a second name, and a backtick puts down a pointer to it.],
  [Short words, named for what they do, one job each.],
))

#problems((
  [Define `double ( n -- 2n )` and use it to define `quadruple`. Check that
  `5 quadruple` leaves 20.],
  [Define `spoiled ( units bad -- units )`, which takes the number of spoiled
  units away from the number unloaded. `12 5 spoiled` must leave 7. Mind who
  speaks first.],
  [Using `spoiled` and `dues`, reckon on one line what the _Gull_ owes for 12
  crates of which 5 were spoiled.],
  [Define `square ( n -- n*n )` without typing a number in the definition.],
  [Redefine `fee` so that mooring costs 25 pennies, reckon the _Marta_ again,
  and then put `fee` back. What did the _Marta_ owe in between?],
))
