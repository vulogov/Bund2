#import "../design.typ": *
#chapter(number: 4, title: "By Rules")

#dropcap("S")ay what is so, and ask. The three programs so far all say _how_
the ledger is reckoned, in three accents. This one does not say how at all.
It states what is true of the day, and what a due _is_, and then puts
questions. Finding the answers is Bund's work.

It is the strangest of the four manners and, for some jobs, far the
shortest.

#section("Facts, rules and variables")

A _fact_ is a list that states something:

```
[ ship marta salt 40 ]
[ rate salt 3 ]
```

There is a ship, the _Marta_, with salt, 40 units. The rate for salt is 3.
The words have no meaning to Bund. `ship` and `rate` are names you chose,
and the facts mean what you decide they mean, so long as you are
consistent.

A _rule_ says that one thing holds whenever some others do. It has a head,
an arrow, and the goals that must hold:

```
[ [ due S D ] ←
    [ ship S C U ] [ rate C R ] [ mooring M ]
    [ is D { U R * M + } ] ]
```

Read it aloud: _S owes D if S is a ship with cargo C and U units, and the
rate for C is R, and the mooring fee is M, and D is U times R plus M._

A name that begins with a capital letter is a _variable_. It is not the
variable of Chapter 1, a place where a value is kept. It is an unknown, as in
school algebra: something Bund is to find a value for, such that everything
said about it is true at once.

`is` is a bridge back to ordinary Bund. It runs a lambda, in which the
variables already found stand for their values, and gives the result to `D`.

A list of facts and rules is made into a _knowledge base_ by `kb!`.

#section("Asking")

```
"rules.bund" study
```

#session("c4-ask", caption: "Five questions")

Each question is a pattern with unknowns in it. `kb.query` finds every way
of filling the unknowns so that the pattern holds, and answers with a list of
records, one for each way. `kb.once` gives the first only. `kb.holds` says
whether there is any.

The first question, `[ ship S C U ]`, has three unknowns and matches every
ship. The second fixes the cargo: `[ ship S salt U ]` asks for the ships
carrying salt, and only the _Marta_ fits. The third asks what the _Gull_
owes. There is no fact about that. Bund found the rule for `due`, worked
through its goals, and came back with 80.

#words((
  ("kb!", "( list -- kb )", [Make a knowledge base from a list of facts and
  rules.]),
  ("kb.query", "( kb goal -- list )", [Every solution, each a record of what
  the variables stood for.]),
  ("kb.once", "( kb goal -- record )", [The first solution.]),
  ("kb.holds", "( kb goal -- flag )", [Is there a solution?]),
))

#section("Asking backwards")

Here is what no other manner in this book can do.

#session("c4-back", caption: "The same rule, asked the other way")

`[ due S 70 ]` gives the due and asks for the ship. _Who owes 70?_ The
_Osprey_. Nobody wrote a word for that. The rule for `due` says what
relation holds between a ship and a due, and a relation can be asked from
either end. The second line asks which cargo pays 5, and the third who
brought wax.

A function runs one way: from what it is given to what it leaves. A rule
has no direction. It states something that is so, and you may leave blank
whichever part you do not know.

The last line asks about a ship that never came. The answer is an empty
list. That is not an error: there is nothing that fits, and Bund says so.

#section("The program")

#listing("rules.bund")

The knowledge base is the top half, and it is the whole of the reckoning.
The tariff is three facts. The day is three facts. The mooring fee is a
fact. What a due is, is one rule. What an entry in the ledger is, is another:
a ship together with her due.

The bottom half only prints. `line` takes one answer, which is a record of
what `S`, `U`, `C` and `D` stood for, and formats it. The names in a
knowledge base are plain lower-case atoms, so `string.title` gives the
_Marta_ her capital back.

#session("c4-run", caption: "The same page")

#section("The fourth cargo")

#session("c4-diff", caption: "What changed, and the page it prints")

Two lines: a fact that the _Heron_ came, and a fact that wool pays 4. No
rule was touched, because the rule for a due never mentioned any cargo by
name.

And a ship whose cargo has no rate? Try it in your head: `[ rate C R ]`
finds nothing, the rule for `due` fails for that ship, and she has no entry.
She is not charged the wrong amount, as in Chapter 1, and the program does
not stop, as in Chapter 2. She is silently absent from the ledger. Whether
that is better depends on who is reading the ledger.

#section("The new question")

Which ships owe more than 75? In this manner a new question is the thing
you do all day. It can be asked as it stands, or given a name by adding a
rule.

#session("c4-rule", caption: "A rule added while the program runs")

The first line writes the rule: _S is dear if S owes D and D is more than
75._ `test` is the other bridge to Bund. It runs a lambda that must leave
true or false, and the goal holds if it leaves true. `D 75 <` asks "is 75
less than D?", top first as ever.

The second line adds the rule to the knowledge base with `kb.assert`. Like
`set`, `kb.assert` leaves a changed copy, so the line keeps the copy under
the old name. After that `dear` can be asked like anything else: who is
dear, and is the _Gull_?

#words((
  ("is", "[ is V { ... } ]", [In a rule: V is what the lambda leaves.]),
  ("test", "[ test { ... } ]", [In a rule: holds if the lambda leaves true.]),
  ("kb.assert", "( kb clause -- kb )", [The knowledge base with one more fact
  or rule.]),
))

#section("What it buys and what it costs")

#words((
  ("It buys", "", [Questions. Any of them, in any direction, including the
  ones nobody planned for. And a program whose every line is a statement
  about the harbour, not about the computer.]),
  ("It costs", "", [Control. You do not say in what order things are tried,
  and when an answer is missing there is no step to point at. Arithmetic and
  printing are done through bridges to the other manners.]),
))

#recap((
  [A program of rules states facts and what follows from them, and is then
  asked questions.],
  [A capitalised name is an unknown. `kb.query` finds every way of filling
  the unknowns.],
  [A rule has no direction: the same rule answers "what does she owe?" and
  "who owes 70?".],
  [`is` and `test` run Bund inside a rule.],
  [The fourth cargo changed 2 lines, both of them facts. The new question
  was one rule, added while the program ran.],
))
