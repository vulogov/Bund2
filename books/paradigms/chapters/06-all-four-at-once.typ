#import "../design.typ": *
#chapter(number: 6, title: "All Four at Once")

#dropcap("T")he four programs were kept pure on purpose, each in one manner,
so that the manners could be told apart. Nobody writes real programs like
that, and Bund does not ask you to. The four are not rival languages. They
are four sets of words in one language, standing on one stack, and a line of
one can call a word of another.

#plate("07-crossroads.png", "A step, a value, a thing, or a fact?")

You have seen it happen already. The program of rules used `is` to do its
arithmetic and `format` to print. The program of functions put a record on
the counting table. The program of objects ran a `loop` over a list. None of
them was pure, and none was the worse for it.

#section("Each part in the manner that suits it")

Here is the ledger once more, written without loyalty to any one way.

#listing("mixed.bund")

Read it for who does what.

*What is so* is stated as facts, because the ships and the tariff are
knowledge, and knowledge is easiest to add to in that form. A fourth cargo
would be two more facts.

*The arithmetic* is a word, `charge`, a function of two numbers. The rule
for a due does not do the sum itself. It hands its unknowns to `charge`
through `is`. The rule says what a due depends on, and the function says how
it is worked out. Change the mooring fee and you change one word.

*The answers* come back from `kb.query` as a list of records: plain values,
which `line` turns into text exactly as the function of Chapter 2 did, and a
`loop` adds up.

*The page* is printed by commands, a step at a time, because printing a
page really is a thing that happens in order.

#session("c6-run", caption: "The same page, a fifth time")

It is 56 lines, about as long as the shortest of the four, and it would
pass both tests as well as the best of them: two facts for the wool, and
any question you care to ask.

#section("A question made into a word")

One more bridge, and the neatest. `kb.word` takes a knowledge base, the name
of a relation and the number of its parts, and makes the relation an
ordinary Bund word.

#session("c6-word", caption: "A relation, used as a word")

After the first line, `due` is a word that takes two values, one for each
part of the relation, and leaves a list of every solution. For a part you do
not know, put down `∅`, which is Bund's value for "nothing here".

So `:gull ∅ due` asks what the _Gull_ owes, and `∅ 70 due` asks who owes 70:
the same word, run in either direction. The last line leaves both parts
unknown, gets every ship with her due, and then treats the answer as what
it is, a list, taking the second element of each with `cdr car` inside a
`map`. A question asked by deduction, answered as a value, and finished off
with a function. One line.

#words((
  ("kb.word", "( kb-name relation n -- )", [Make the relation a word of n
  operands that leaves a list of its solutions.]),
  ("∅", "( -- nothing )", [No value: an operand left unknown.]),
))

#section("The floor they all stand on")

Look back over the five programs for what never changed.

Every one of them puts values on a stack and says words that pick them up,
the top one first. `40 :Salt object` and `:harbour var? [ due S D ] kb.query`
and `arrivals { reckoned } map` are the same sentence in three dialects:
operands, then the word. A class is defined with `set`, the word you learned
for records. A rule is a list. A method is a lambda in a slot. A question is
a list handed to a word.

That is why the four manners mix so easily here. In most languages each
paradigm brings its own grammar, and joining two of them is a negotiation. In
Bund there is one grammar, the stack's, and the paradigms are vocabularies.

So the choice the author left you is a small one each time, and you make it
over and over: for _this_ part, is it a step, a value, a thing, or a fact?

#recap((
  [The four manners are sets of words in one language, and a line of one
  can call a word of another.],
  [`mixed.bund` states the day as facts, does its arithmetic in a function,
  takes its answers as values and prints by commands.],
  [`kb.word` turns a relation into an ordinary word that can be run in
  either direction.],
  [Under all four there is one stack and one rule: the top speaks first.],
))
