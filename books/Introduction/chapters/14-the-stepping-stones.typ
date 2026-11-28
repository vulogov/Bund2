#import "../design.typ": *
#chapter(number: 14, title: "The Stepping Stones")

#dropcap("N")o harbour is built on open water. Wendhaven's stone quay stands on
an older timber one, and the timber one on a beach where somebody once
dragged a boat up. A language is built the same way. Almost nothing you have
learned in this book was invented for Bund. It was found in older languages,
carried here, and set down beside something else.

This chapter names the stones. It teaches no new words. It is here so that
you know where the ideas came from, what each one was for when it was new,
and which part of Bund is Bund's own.

#plate(13, "13-stepping-stones.png", "Each stone was set down by somebody else.", width: 50%)

#section("Writing the word last")

The oldest stone was laid before there were computers. In the 1920s the
Polish logician Jan Łukasiewicz noticed that brackets are needed only because
we write the sign _between_ the things it joins. Put the sign in front, every
time, and a formula can be read in exactly one way with no brackets at all.
His notation was called _Polish_ after him.

Turn it round, with the sign _after_ its operands, and you have _reverse
Polish_ or _postfix_ notation. It turned out to suit machines very well. A
machine reading postfix needs to remember nothing about what is coming. It
puts values on a pile, and when it meets a sign it applies it to the top of
the pile.

You have been writing postfix since Chapter 1.

#session("ch14-postfix", caption: "Two sums that need no brackets")

In school notation the first is (3 + 4) × 5, and the brackets are not
optional. Here the order of the words _is_ the order of the work, and there is
nothing for a bracket to say.

#term("Postfix notation")[
  Writing the operation after the values it works on. Also called reverse
  Polish notation, RPN. It needs no brackets and no rules about which sign
  binds tighter, because a word always works on what is already there.
]

#section("My first stone: a calculator")

I did not meet postfix in a book. I met it as a schoolboy, on my desk.

In 1980 a Soviet factory began to sell a programmable calculator, the
Elektronika B3-34, for 85 roubles. The models that followed it, the MK-54,
the MK-61 and the MK-52, all worked the same way, and for a great many
schoolchildren of my generation they were the first computer we ever
touched.

Mine was the MK-56. It was the same machine as the B3-34 inside, built not
for the pocket but for the desk: a broad sloping keyboard, a row of green
digits above it, and a cord to the wall. It did my schoolwork, and when the
schoolwork was done it played.

#plate(14, "MK-56-FANTASY.png", "The MK-56, as Wendhaven's customs house might have built it.", width: 56%)

It had no `=` key. It had a key marked `В↑` that pushed a number up a stack
four places deep, and every operation took its operands from that stack. To
add three and four you keyed `3`, `В↑`, `4`, `+`. Nobody told us this was
called reverse Polish notation, or that it was supposed to be difficult. It
was simply how the machine listened, and soon enough it was how we
thought.

#term("A machine of 98 steps")[
  The MK-56, like the B3-34, held a program of 98 steps, one keystroke to a step, and 14
  numbered registers to keep numbers in. Switch it off and the program was
  gone. Everything it could do had to fit in that.
]

Ninety-eight steps sounds like nothing. It was enough to fly to the Moon.

The magazines of the time printed programs for it. _Tekhnika — Molodyozhi_,
"Technology for the Young", with a million and a half subscribers, gave it a
regular section from January 1985, and _Nauka i Zhizn_ ran columns of its
own. A program was a printed table of keystrokes. You keyed it in by hand,
one step at a time, checked it against the page, and pressed start.

Then, in August 1985, _Tekhnika — Molodyozhi_ began to print a story in
instalments: Mikhail Pukhov's _Kon-Tiki: The Way to Earth_. A pilot and an
engineer set out to fly a small lunar lander from the Moon all the way home.
The instalments ran into 1986, and with them came programs for the B3-34,
each simulating one leg of the journey, and puzzles about the physics of
the flight. The story was not only to be read. You had to fly it.

So you sat down at the calculator and flew. You keyed in how much fuel to burn
and for how long. The machine thought about it, and gave you back your
height and your speed. You wrote them
down, decided, and keyed in the next burn. Burn too little and you arrived
at the surface far too fast. Burn too much and the tank was dry while you
were still falling. The whole of it, gravity and thrust and a ship growing
lighter as its fuel went, had to be packed into those 98 steps.

When you asked the machine for something it could not do, the display showed
`ЕГГОГ`, which was as near as its display could come to the word ERROR.
Readers discovered that the keys did interesting things _after_ that
message, wrote to the magazines about it, and gave the study a name:
errorology.

#callout(label: "What I took from it")[
  Three things, long before I had words for them. That a stack is a natural
  place to keep what you are working on. That a program can be very small and
  still do something that matters to you. And that the way to learn a machine
  is to hold a conversation with it and see what it answers. Everything else
  in this chapter I read about later. This one I lived.
]

#section("Forth: a stack, a dictionary, a conversation")

Around 1970 Charles Moore was writing programs to steer radio telescopes, on
machines with almost no memory. The language he made for himself, Forth, put
three ideas together, and Bund has kept all three.

The first is the _stack_ as the one place where values wait. The second is
the _word_: every operation has a name, the names are kept in a dictionary,
and you make a new word out of words you already have. The name "word" in this
book is Forth's. The third is the _conversation_. You type a line, the
language does it at once, and you look at what is left. You grow a program by
trying it, one word at a time, exactly as you grew the ledger.

Bund is a descendant of Forth and says so. My first description of it
reads: "a concatenative, stack-based programming language inspired by
Forth".

It is also not Forth, and three differences matter to anyone who comes from
there.

#chord_table((
  chord_row("the top speaks first", "In Forth 10 3 - is 7. In Bund it is −7. This book's one rule is Bund's, not Forth's."),
  chord_row("values know their kind", "A Forth stack holds bare machine numbers. A Bund stack holds numbers, strings, lists, dictionaries and lambdas, and each knows what it is."),
  chord_row("words are looked up late", "Redefine due and dues changes with it (Chapter 4). In Forth a definition is fixed to the words as they were."),
))

#section("PostScript: braces that wait")

In 1984 a language appeared that most people have used without knowing it.
PostScript describes a printed page, and for years nearly every laser printer
had an interpreter for it inside. It is postfix, like Forth. What it added
was a way of writing a piece of program that is _put down and not run_: a
procedure, between braces. Its `if` is an ordinary operator that takes a
truth value and a procedure from the stack. It keeps its names in
dictionaries, and it has more than one stack.

If you have read Chapter 5 this will sound familiar. I name
PostScript among the family and does not name it as a source, so call it a
cousin. But a PostScript programmer reading `flag { ... } if` is at home.

#section("Joy and Factor: programs made of programs")

Forth had the practice. The theory came later. Around the turn of this
century Manfred von Thun, a philosopher, designed a small language called Joy
to study one question: what if a program is nothing but functions written one
after another, each taking the whole stack and handing the whole stack on? Put
two programs side by side and you have a program. That is _concatenation_,
and the family got its name, _concatenative_, from that work.

Joy also made the quoted program, the thing Bund calls a lambda, the centre
of the language. Loops, decisions and `map` stopped being syntax and became
ordinary words that take a quoted program. Chapter 13 said the same of Bund.

Factor, begun by Slava Pestov in 2003, took those ideas out of the study and
built a full working language on them, with the tools to match. Above all it
showed how far a language can go when programs write and rewrite programs.
I name it directly in my first booklet on Bund: its _metaprogramming_ was
influenced by Factor.

#term("Metaprogramming")[
  Writing programs that treat programs as their material: building them,
  taking them apart, changing them, running them. In Bund the material is a
  lambda, and the tools are the ones you already have for lists.
]

#section("Lisp: code is data")

The oldest idea in Bund after postfix itself is Lisp's. John McCarthy's
language of 1958 wrote its programs as lists, in the same notation it used
for its data. A program could therefore read a program, build one, and hand
it to the language to run. Nothing about it had to be special.

Lisp is not postfix and has no stack you can see. But this one idea crossed
over whole, and I owe it to Lisp: code as data, and data as code. You
met its two oldest words, `car` and `cdr`, in Chapter 8, with their names
unchanged.

#session("ch14-data", caption: "Three things a Lisp programmer would recognise")

The first line hands a piece of program to a word as a value. The second
takes a word you defined and looks inside it: a list of two words. The third
runs a string as a program. All three are Lisp's inheritance, written
backwards.

#section("More than one stack")

Here is where Bund stops borrowing.

Forth has two stacks, but only one of them is yours. The other belongs to
the machinery, and using it for your own values is a trick that every Forth
book warns you about. So in practice everything a Forth program is doing
lies on one pile, and nothing marks where one piece of work ends and the next
begins. Keeping them apart is the programmer's job. It is the reason Forth
has a reputation for programs only their author can read.

Bund's answer is in its name. A _Bund_ is a bundle. There is not one stack
but a ring of them, each with a name if you want one, and each piece of work
can have a pile to itself.

#session("ch14-stacks", caption: "Two piles that do not know of each other")

And because separate piles need a way to pass things across, there is the
_workbench_: one more stack, shared by all, that holds a value while you
walk from one quay to another. Chapters 10 and 11 were about nothing else.

#callout(label: "What is Bund's own")[
  Named stacks in a ring, and the workbench between them. Every other idea in
  this chapter has a longer history than Bund. This one is what the language
  was made for, and what it is called after.
]

#section("More than one way to think")

One more inheritance, and it is a whole shelf of them.

Most languages hold to one view of what a program is. A list of commands
that change things. A set of functions that turn values into values. A
population of objects that send each other messages. A set of facts and
rules from which answers are deduced. Each view has languages built purely
on it, and each is the best tool for some jobs and an awkward one for others.

Bund is concatenative first and takes from the rest where it helps. This
book has shown you only part of it.

#chord_table((
  chord_row("commands", "Words that print, that write to a dictionary, that switch stacks. Bund also has variables. The book left them out, because the stack and the workbench do the work."),
  chord_row("functions", "Lambdas are values. map takes one. Words can take words and leave words. This comes by way of Lisp and its descendants."),
  chord_row("objects", "Bund has classes and objects, with more than one parent allowed. The tradition is that of Smalltalk. This book does not go there."),
  chord_row("deduction", "Bund has words for facts, rules and questions put to them, in the tradition of Prolog. Nor there."),
))

A language of this kind is called _multiparadigm_. My reason is a plain
one: rather than hold everyone to a single way of thinking, I chose to let
the programmer choose.

#warn(label: "A choice is not an obligation")[
  That Bund has objects does not mean a ledger needs them. Everything in
  this book was done with a stack, a few words and a record. Reach for the
  larger tools when the job is larger.
]

#section("The stones, in order")

#chord_table((
  chord_row("1920s", "Łukasiewicz: notation without brackets. Bund's word order."),
  chord_row("1958", "Lisp: a program is a list. Bund's lambdas, car and cdr."),
  chord_row("about 1970", "Forth: the stack, the word, the dictionary, the conversation. Bund's whole manner of working."),
  chord_row("1970s", "Smalltalk and Prolog: objects, and deduction. The parts of Bund this book leaves for later."),
  chord_row("1980", "The Elektronika B3-34 and its desk-top twin, the MK-56: postfix on a schoolboy's desk, and a flight from the Moon in 98 steps. Where I began."),
  chord_row("1984", "PostScript: procedures in braces, control as ordinary operators. A cousin."),
  chord_row("about 2001", "Joy: programs composed by writing them side by side. The family's name."),
  chord_row("2003", "Factor: a concatenative language that rewrites itself. Bund's metaprogramming."),
  chord_row("Bund", "Many stacks in a ring, a workbench between them, and the top speaks first."),
))

One smaller debt belongs in the list. Chapter 13 said that `bund2` turns your
program into a compact internal form before it runs it. I took that idea from Python and Erlang, two languages that have nothing else in
common with it.

#section("If you want to read further")

Leo Brodie's _Starting FORTH_ is the book that taught a generation to think
on a stack, and it is still the friendliest book about any language in this
family. Almost everything in it carries over, once you remember who speaks
first. His _Thinking Forth_ is about how to cut a problem into words, and it
is about Bund as much as Forth.

For the theory, look for Manfred von Thun's papers on Joy. For what a large
modern concatenative language looks like, look at Factor.

#recap((
  [Postfix notation is a century old. It needs no brackets because a word
  always works on what is already there.],
  [For the author it began with a Soviet programmable calculator, the MK-56,
  and the magazine programs that flew a lander home from the Moon in 98
  steps.],
  [From Forth: the stack, the word, the dictionary, and programming as a
  conversation. Bund differs in three things: the top speaks first, values
  know their kind, and words are looked up late.],
  [From Joy and Factor: a program is words side by side, a quoted program is a
  value, and control is done by ordinary words. Bund's metaprogramming follows
  Factor.],
  [From Lisp: code is data.],
  [Bund's own: a ring of named stacks and a workbench between them. The name
  means a bundle.],
  [Bund is multiparadigm. It has variables, first-class functions, objects and
  deduction as well, and none of them is compulsory.],
))
