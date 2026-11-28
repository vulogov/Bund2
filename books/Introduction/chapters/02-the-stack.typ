#import "../design.typ": *
#chapter(number: 2, title: "The Stack")

#dropcap("T")he _Marta_ has tied up, and her cargo is coming ashore. The men carry
the crates down the gangway and set them on the quay one on top of another,
because the quay is narrow and there is nowhere else to put them. When the
carter comes he takes the top crate, since it is the only one he can get at.
The crate that came off the ship first is at the bottom and will leave last.

That pile is a _stack_, and it is where Bund keeps everything.

#plate(2, "02-quay-stack.png", "One pile, and only the top crate can be lifted.")

#section("Putting things down")

In the last chapter a sentence between quotes was "put down". Now we can say
where: on the stack. Anything you type that is not a word goes onto the top of
it. Type three numbers, one to a line.

#session("ch02-push", caption: "Three numbers go down")

After each line `=>` shows the top of the stack, which is the number you just
typed. `.s` shows all of it. Read it the way it is printed: the bottom is on
the left, the top is on the right, and the arrow points at the top.

#piles(
  ("40", ("40",)),
  ("12", ("40", "12")),
  ("25", ("40", "12", "25")),
  note: "The stack after each line. The inked crate is the top.",
)

#term("Stack")[
  A pile of values with one rule: you can only put a value on the top, and you
  can only take a value from the top. The last one in is the first one out.
  Bund's stack has no fixed size. It grows when you put something down and
  shrinks when a word picks something up.
]

To _push_ a value is to put it on top. Nothing in Bund is called "push": you
push a value by writing it.

#section("Picking things up")

`println` picks up the top value and prints it. With 40, 12 and 25 on the
stack, say it three times.

#session("ch02-lift", caption: "Last in, first out")

They come off in the opposite order to the one they went on in. The 25 went on
last and comes off first. After the third `println` the stack is empty and
`.s` says so.

Nothing here depends on the lines. Bund reads a line from left to right and
does each thing as it meets it, so the whole of that can be said at once:

#session("ch02-oneline", caption: "The same, on one line")

Three values go down, then three words each take the top one. It is worth
reading that line until it is obvious, because every Bund program is that line
with different words in it.

#section("What can go on a stack")

Not only numbers. A stack takes any value Bund has, in any mixture.

#session("ch02-kinds", caption: "Values of several kinds")

That first line put down five values, written five ways:

#words((
  ("40", "integer", [A whole number.]),
  ("3.5", "float", [A number with a fraction.]),
  ("\"salt\"", "string", [Text, between double quotes.]),
  ("true", "boolean", [One of the two answers to a question: `true` or `false`.]),
  (":cargo", "atom", [A string of one word, written with a colon in front and no
  quotes. Used for names and labels.]),
))

When a string or an atom is printed, the quotes and the colon are not: they are
how you _write_ the value, and they are not part of it. That is why `.s` shows
`salt` and `cargo`.

Bund keeps track of what kind each value is, and the word `type` will tell you.
It leaves the value where it was and puts the name of its kind on top.

#callout(label: "For the curious")[
  A language that keeps a value's kind with the value, instead of making you
  declare it beforehand, is called _dynamically typed_. Bund is one. You never
  say "this is a number"; Bund knows, and a word that is handed the wrong kind
  of thing says so when it runs.
]

#section("Three words for rearranging")

A carter can only take the top crate, and sometimes the crate he wants is the
one underneath. Bund has words that rearrange the top of the stack without
doing anything else. Three of them will carry you through most of this book.

#session("ch02-dup", caption: "dup, drop and swap")

`dup` makes a second copy of the top value. `drop` throws the top value away.
`swap` exchanges the top two.

#piles(
  ("40 12", ("40", "12")),
  ("dup", ("40", "12", "12")),
  ("drop", ("40", "12")),
  ("swap", ("12", "40")),
  note: "The session above, drawn.",
)

#words((
  ("dup", "( a -- a a )", [Copy the top value.]),
  ("drop", "( a -- )", [Throw the top value away.]),
  ("swap", "( a b -- b a )", [Exchange the top two values.]),
  ("clear", "( ... -- )", [Throw everything away.]),
  ("type", "( a -- a name )", [Leave the value and put the name of its kind on top.]),
))

Here the middle column earns its place. `( a b -- b a )` is a picture of the
stack before and after, with the top on the _right_ in both halves, exactly as
`.s` prints it. Before `swap` the top is `b`. After it the top is `a`. This
notation is called a _stack effect_, and Chapter 4 makes proper use of it.

You may wonder why anyone would want two copies of a number. The answer is in
the first section: a word _picks up_ what it uses. If you need a value twice,
you need two of it.

#section("Running out of crates")

What if a word wants a value and the stack is empty?

#session("ch02-under", caption: "Nothing to pick up")

`println` on an empty stack is an error: `Stack is too shallow`. So is `swap`
when there is only one value, because it needs two. The message names the word
that went short, in capitals or with a pair of brackets after it, as Bund's
innards happen to spell it.

Asking a stack for more than it holds is called _underflow_, and it is the most
common mistake in any stack language. It is never mysterious. It means you
thought there was one more value on the stack than there was. `.s` will show
you what is really there.

The last two lines of that screen go the other way: `dup dup dup` turns one 40
into four of them. Bund will not complain however high the pile gets.

#warn(label: "Count your crates")[
  Before every word, know what is on the stack. In the beginning, type `.s`
  before and after anything you are unsure of. It costs nothing and it changes
  nothing. Programmers who have used stack languages for thirty years still do
  it.
]

#section("Why a stack?")

Because it lets words talk to each other without being introduced. `println`
does not need to be told _what_ to print. It prints what is on top. The word
before it does not need to know that `println` comes next. It leaves its result
on top and is done. Any word can follow any other, provided the stack holds what
the second one needs. That is what _concatenative_ means: you make a program by
putting words one after another.

So far the only word that does anything with a value is `println`. The next
chapter gives the clerk some arithmetic, and with it the one rule of Bund that
you will have to get used to.

#recap((
  [Bund keeps every value on a *stack*: a pile where you can only add to the top
  and take from the top. Last in, first out.],
  [Writing a value pushes it. A word picks up what it needs from the top.],
  [Values come in kinds — integer, float, string, boolean — and any mixture can
  share a stack. An atom, `:cargo`, is a string written the short way. `type`
  names the kind of the top value.],
  [`dup` copies the top, `drop` discards it, `swap` exchanges the top two.],
  [`.s` shows the whole stack, bottom on the left and top on the right, and
  changes nothing.],
  [A word that finds too few values reports `Stack is too shallow`. That is
  *underflow*, and `.s` is the cure.],
))

#problems((
  [Starting from an empty stack, type `1 2 3` and then use only `swap` and `drop`
  to leave just `3 1`, with 1 on top. Check with `.s`.],
  [What does `.s` show after `7 dup dup drop swap`? Work it out on paper first.],
  [Put `"Marta"` and `40` on the stack, in that order. With one more line, print
  the name first and the number second.],
  [`5 dup println println println` — which word fails, and what was printed
  before it did?],
))
