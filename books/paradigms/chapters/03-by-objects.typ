#import "../design.typ": *
#chapter(number: 3, title: "By Objects")

#dropcap("T")hings that know how to answer. In the last two chapters the
knowledge of what a cargo pays sat somewhere apart from the cargo: in a chain
of questions, then in a table. In this manner it sits _in the thing itself_.
A landing of wax knows it is wax and knows what wax pays. You do not reckon
it. You ask it what it owes.

#plate("03-guild.png", "He does not weigh the wax. He asks the chandler.")

#section("A class and an object")

A _class_ describes a kind of thing: what every one of that kind has, and
what it can be asked. An _object_ is one thing of that kind.

In Bund a class is written like a dictionary, between `class` and
`register`. Each `set` adds a _slot_. A slot that holds a lambda is a
_method_, something the object can be asked to do. Any other slot is a
_field_, something it has.

```
:Wax class
  :.super [ :Landing ] set
  :kind "wax" set  :rate 5 set  :.init { landed } set
register
```

This says: `Wax` is a class. It is a kind of `Landing`, which is what
`.super` means. Its kind is `"wax"` and its rate is 5. `.init` is its
_constructor_, the lambda that runs when a new object is made.

Learn the program and look.

```
"objects.bund" study
```

#session("c3-class", caption: "Classes, and what one of them holds")

#section("Asking an object")

`object` makes one. The values the constructor wants go down first, then the
class's name.

#session("c3-ask", caption: "One landing, asked five things")

The first line makes a landing of wax from the _Gull_, 12 units, and keeps it
under a name so that it can be asked several things. Then:

A field the object itself holds is read with `get`, as from any record.

A method is _called_ by putting its name beneath the object and saying `!`:

```
:due :gull var? !
```

`!` looks in the object for a slot called `due`. The object carries the
slots of its own class, `Wax`, and `due` is not among them, so `!` looks in
the parent, `Landing`, where it finds one and runs it. The method finds the object on
top of the stack. By custom it leaves the object there, beneath its answer,
so that the object can be asked something else. That is why each line ends
`swap drop`: the screen wants the answer alone.

#words((
  ("class", "( name -- class )", [Begin a class. Add slots with `set`; finish
  with `register`.]),
  ("object", "( args class-name -- object )", [Make an object, running the
  class's `.init` on it.]),
  ("!", "( name object -- object answer )", [Ask the object: find the slot of
  that name in it or its classes, and run it.]),
  ("?class  ?object", "", [Is there such a class? Is this an object?]),
))

#section("The program")

#listing("objects.bund")

`Landing` holds everything the kinds have in common, and it is where the two
methods are written, once. Look at the first line of `due`:

```
:rate swap !
```

The method _asks the object for its own rate_. It does not know what the
answer will be, and it does not know what kind of object it is running on.
Then the three kinds, three lines each, say only what is different about
them: a name and a number.

#session("c3-same", caption: "One question, put to three objects")

The same two words went to each object. Each answered for itself. Nothing
anywhere in the program asks "is it salt?". That question has disappeared:
it is answered by which object you happen to be holding. This is the idea
the manner is named for, and it has a long name, _polymorphism_. One
question, many forms of answer.

#callout(label: "The constructor is not inherited")[
  Each kind sets its own `.init`, though all three say the same thing,
  `{ landed }`. In Bund a parent's constructor runs on the parent, not on the
  new object, so a class that takes arguments says how. Naming the shared
  part `landed` keeps the three from drifting apart.
]

#section("What it knows afterwards")

The objects are values, like the records of the last chapter, and they do
not change either.

#session("c3-copy", caption: "A changed copy, and the original")

`set` on an object leaves a changed copy, exactly as on a record. The copy
with two crates spoiled owes 134. The _Marta_ kept under her name still owes
140. Objects in Bund are not little boxes of changing state, as they are in
many languages. They are records that carry their own behaviour.

#section("The fourth cargo")

First, the unchanged program and a cargo it has never heard of.

#session("c3-missing", caption: "Wool, before there is a class for it")

There is no way to make the thing at all. The mistake is caught before any
reckoning is tried. (The 8 is the units, left on the stack.) Now the change:

#session("c3-diff", caption: "What changed, and the page it prints")

Six lines, and every one of them _added_. Four make the class `Wool`, a
blank line follows it, and one puts the _Heron_ into the day. Nothing that
existed was opened or altered: not `Landing`, not `due`, not `line`, not
the other kinds. A new kind of thing is a new class, set down beside the
old ones. This is the test the manner was made to pass.

#section("The new question")

Which ships owe more than 75? It can be asked from outside, as in the last
chapter, by putting `:due` to each object and testing the answer. But the
manner's own way is to give every landing a new thing it can be asked, a
method `dear?`, and that method belongs in `Landing`. One class is opened
and one method added, and all the kinds have it.

So a new question means opening the class that everything inherits from.
That is easy while there is one such class and you own it. It is the price
of the manner all the same: _new kinds are cheap and new questions are
not_, which is the last chapter's bargain the other way round.

#section("What it buys and what it costs")

#words((
  ("It buys", "", [A place for everything. What is true of wax is written in
  `Wax` and nowhere else, and a new kind disturbs nothing.]),
  ("It costs", "", [Ceremony: classes, constructors, a calling convention.
  And every new question has to find its way into the parent class.]),
))

#recap((
  [A class says what a kind of thing has and can be asked. An object is one
  such thing.],
  [`:Name class … register` defines a class; `args :Name object` makes an
  object; `:method object !` asks it.],
  [A method written once in the parent asks the object for what differs, and
  each kind answers for itself.],
  [Objects are values: `set` leaves a changed copy.],
  [The fourth cargo added 6 lines and changed none. The new question means
  adding a method to the parent class.],
))
