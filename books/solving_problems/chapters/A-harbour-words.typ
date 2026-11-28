#import "../design.typ": *
#appendix(letter: "A", title: "The Harbour's Words")

These are the words the first book built and this one assumes. They are in
`code/harbour.bund`, and `"harbour.bund" study` teaches them to the terminal.

#listing("harbour.bund")

Three words of the first book are left out, because problems in this book ask
you to write words like them: `total`, `line` and `manifest`.

#section("The words that travel")

A few words defined in the answers are used again in later ones. If you
skipped the problem, here they are.

#words((
  ("rem", "( n d -- r )", [Problem 14. The remainder of n divided by d.]),
  ("even?", "( n -- flag )", [Problem 20.]),
  ("over", "( a b -- a b a )", [Problem 49. A copy of the second value.]),
  ("pair", "( a b -- a b a b )", [Problem 49. Copies of the top two.]),
  ("own", "( x -- x )", [Problem 52. Carry the top value to the stack
  `work`.]),
  ("back", "( x -- x )", [Problem 52. Carry the top value home to `main`.]),
))

```
:rem   { . dup peek swap / take * swap - } register
:even? { 2 rem 0 == } register
:over  { . dup take swap } register
:pair  { over over } register
:own   { . @work take } register
:back  { . @main take } register
```
