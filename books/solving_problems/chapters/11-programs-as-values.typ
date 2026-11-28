#import "../design.typ": *

#let title = "Programs as Values"

#let intro = [
#dropcap("A") lambda is a value. It can be kept in a dictionary, handed to a
word, taken out of a word and looked at, and run when you choose. A line of
text can be made into a program and run too. These are the first book's last
ideas, and the ones that most repay practice.

#plate("11-shipwright.png", "The ship on the bench, and the ship on the slip.")

#words((
  ("!", "( lambda -- ... )", [Run a lambda, a pointer or a `?try`.]),
  ("lambda=", "( name -- lambda )", [The body of a word you defined.]),
  ("convert.to_list", "( lambda -- list )", [A lambda's contents, as a
  list.]),
  ("`name", "( -- pointer )", [A pointer to a word, put down and not run.]),
  ("bund.eval", "( text -- ... )", [Run a string as Bund.]),
  ("?word", "( name -- flag )", [Is there such a word?]),
  ("?try", "", [A guarded piece of program: set `:try` and `:except`, run
  with `!`.]),
))
]

#let problems = (
  (id: "c11p1", title: "Looking inside", grade: 1,
   ask: [Print the pieces `size` is made of, one to a line. Then leave the
   number of pieces in `dues`.],
   hint: [`lambda=` fetches the body and `convert.to_list` makes it something
   `loop` and `len` can take.],
   answer: [`size` is four pieces: a word, two lambdas, and the word that
   chooses between them. `dues` is two. What you put into a word is what you
   can take out.]),

  (id: "c11p2", title: "A table of rates", grade: 2,
   ask: [Keep a lambda for each cargo in a dictionary: salt is reckoned at 3
   pennies a unit and wax at 5. Define `by-cargo ( units cargo -- pennies )`,
   which finds the cargo's lambda and runs it. `40 "salt" by-cargo` leaves
   120.],
   hint: [`get` leaves the lambda on top of the units. One more word runs
   it.],
   answer: [No `ifthenelse` anywhere. The decision is a lookup, and adding a
   cargo is adding one line to `rates`. The first book's terminal chooses its
   commands the same way.]),

  (id: "c11p3", title: "A word that makes words", grade: 3,
   ask: [Define `rate-word ( name n -- )`, which makes a new word that
   multiplies by `n`. After `"salt-due" 3 rate-word`, typing `40 salt-due`
   leaves 120.],
   hint: [Build the text of a definition, piece by piece, and hand it to
   `bund.eval`. Write the text you want on paper first.],
   answer: [For `"salt-due"` and 3 the text wanted is
   `:salt-due { 3 * } register`. `body` builds the part from the brace on,
   around the number. `rate-word` puts a colon and the name before it and
   runs the result. Joining is top first, so each piece is put down _after_
   what it must come before.]),

  (id: "c11p4", title: "A share of nothing", grade: 2,
   ask: [`0 291 /` stops with an error, because nothing can be divided among
   no one. Define `share ( people pennies -- pennies )`, which divides, and
   leaves 0 when the division fails.],
   hint: [`?try` runs its `:try` lambda, and if that fails it runs `:except`
   with the error on the stack.],
   answer: [When `/` fails, the `:except` lambda finds the error on the
   stack as a value, drops it and puts down 0. The line after it shows that
   nothing else was left behind.]),

  (id: "c11p5", title: "Only if it is new", grade: 2,
   ask: [`register` replaces a word without a murmur. Define
   `new ( name lambda -- )`, which defines the word only if there is none of
   that name. `:dues { 0 } new` must leave `dues` alone; `:fine { 2 * } new`
   must define `fine`.],
   hint: [`?word` uses up the name, and you need it again for `register`.],
   answer: [`swap dup ?word` leaves the lambda, the name, and the answer.
   If the word exists, `shed` drops name and lambda. If not, `keep` puts them
   back in the order `register` wants. `40 dues` still answers 140.]),

  (id: "c11p6", title: "Twice", grade: 2,
   ask: [Define `twice ( x lambda -- x )`, which runs a lambda two times.
   Hand it the lambda `{ 2 * }`, and then hand it a pointer to `fee` in place
   of a lambda.],
   hint: [`!` uses up what it runs, so keep a copy on the table for the
   second time.],
   answer: [`dup .` keeps a copy on the table, `!` runs the first, and
   `take !` runs the copy. A pointer, written with a backtick before the
   word's name, is run by `!` exactly as a lambda is, so `twice` takes either:
   two mooring fees make 80.]),
)
