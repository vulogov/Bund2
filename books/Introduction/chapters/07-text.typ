#import "../design.typ": *
#chapter(number: 7, title: "Text")

#dropcap("A") ledger that says `140` and nothing else is a column of riddles.
Whoever opens the book next year needs to read that the _Marta_ brought forty
crates of salt and owed a hundred and forty pennies for them. The reckoning is
done; now it has to be written down so that somebody can read it.

This chapter is about text: how to write it, join it, change it and lay values
into it. But first, a matter of housekeeping that has been put off for three
chapters.

#section("Keeping your words")

Every word you have taught Bund lasts until you leave the terminal. Start it
again and `dues` is gone. So far the book has been retyping the definitions for
you behind each screen. It is time to stop.

Words that are worth keeping go in a file. Here are the six from Part II, in
the book's `code` folder:

#listing("words.bund")

Two things are new. In a file a definition may run over several lines, and
`size` does: the question, the two courses and the word that chooses between
them each have a line. And every word has a comment above it giving its stack
effect and what it is for. Six months from now that comment is the only
documentation there will be.

The terminal has a word for reading such a file: `study`.

#session("ch07-study", caption: "The clerk studies his words")

`study` takes the name of a file in the folder the terminal was started in, and
does everything in it, exactly as if you had typed it. Afterwards `dues` and
`note` are words again.

#session("ch07-nofile", caption: "A file that is not there")

#callout(label: "For the curious")[
  `study` is not part of Bund. It is one of the terminal's own words, three
  lines long, built on a Bund word called `use` that wants a file's whole
  address. Chapter 13 shows how it is made.
]

From here on, when a screen uses `dues` or `size`, assume `words.bund` has been
studied first.

#words((
  ("study", "( filename -- )", [Do everything in a file from the terminal's folder.
  A word of this book's terminal.]),
))

#section("Three ways to write a string")

You have been writing strings between double quotes since the first page. Bund
has two other ways to write one, and all three make the same kind of value.

#session("ch07-three", caption: "Double quotes, single quotes, a colon")

Double quotes are the usual way. Single quotes do the same, and are for text
that has a double quote inside it. A colon in front makes an _atom_: a string
of one word, with no spaces, that needs no closing mark.

The last line settles what an atom is. `"salt"` and `:salt` are equal. An atom
is not a different kind of value. It is a string, written the short way.

Use quotes for text a person will read, and atoms for names the _program_
reads: the name of a word in `:dues { ... } register`, a label, a key. The
difference is to the eye and not to Bund, and that is reason enough.

#section("Printing without ending the line")

`println` prints a value and ends the line. `print` prints a value and stays on
the line, so that the next thing printed follows it.

#session("ch07-print", caption: "Building a line from pieces")

`space` prints one space and `nl` ends the line, so `print nl` is `println`
spelled out.

#words((
  ("print", "( x -- )", [Print the top value. Do not end the line.]),
  ("println", "( x -- )", [Print the top value and end the line.]),
  ("space", "( -- )", [Print one space.]),
  ("nl", "( -- )", [End the line.]),
))

Printing in pieces works, and it has the fault of every word that prints: the
line is gone as soon as it is made. You cannot keep it, test it or hand it to
another word. It is better to _build_ the line as one string and print that.

#section("Joining")

Chapter 3 showed that `+` joins strings, and that the top speaks first.

#session("ch07-join", caption: "Text and numbers")

`140 "Marta owes " +` reads from the word backwards: "Marta owes", then 140.
When one of the two is a number and the other a string, `+` writes the number
out and puts it _after_ the text, whichever of them is on top. If you want the
number first, make it a string yourself with `convert.to_string`, as the
second line does, and then it is two strings and the usual rule.

#warn(label: "To build a phrase, write its end first")[
  `"haven" "Wend" +` is `Wendhaven`. The string on top becomes the beginning.
  When a joined phrase comes out in the wrong order, `swap` before the `+`.
]

#section("Changing text")

The words that work on strings have names beginning `string.`, and the dot is
part of the name. Bund uses dotted names to keep related words together:
`string.upper`, `string.lower`, `string.title`.

#session("ch07-case", caption: "Case and length")

`len` counts the characters in a string. Note what `.s` shows afterwards: the
string is still there, with its length on top. `len` _looks_ at the value and
does not pick it up. That is unusual, and worth remembering, because the string
underneath will be in your way if you forget it. `swap drop` gets rid of it.

#words((
  ("string.upper", "( s -- s )", [All capitals.]),
  ("string.lower", "( s -- s )", [All small letters.]),
  ("string.title", "( s -- s )", [A capital at the start of each word.]),
  ("len", "( x -- x n )", [The length of a string or a list. Leaves the value where
  it was.]),
))

#section("A heading for the ledger")

Chapter 3 repeated a string with `*`. That makes a rule to draw under a
heading, and the heading can be a word.

#session("ch07-rule", caption: "rule and heading")

`heading` takes nothing and leaves nothing. It is all effect and no result,
which is the right shape for a word whose whole job is to print. Notice that
`rule` is a word of its own. The ledger will want a rule at its foot as well.

#section("Laying values into a line")

Joining with `+` becomes tiresome after the second piece. For a line with
several values in it there is `format`. It takes a _template_: a string with
names in curly braces where the values are to go.

#session("ch07-format", caption: "A template and three values")

The template is on top. Under it are the values, and `format` takes one for
each name in the template, in the order the names appear. Top first: `name` gets
`"Marta"`, which is directly under the template, `units` gets 40, and `cargo`
gets `"salt"`, which was put down first of all.

#piles(width: 32mm,
  ("the stack", ("salt", "40", "Marta", "template")),
  ("format", ("Marta: 40 of salt",)),
  note: "Values go down in the reverse of the order the template names them.",
)

#term("Template")[
  A string with `{names}` in it. `format` replaces each name with a value from
  the stack, the first name with the value nearest the top. The names are for
  you; `format` only counts them. A name that appears twice is filled twice
  from one value.
]

The template for a ledger line is long, and it will be used for every ship. It
deserves a name.

#session("ch07-wording", caption: "The wording of a ledger line")

`wording` is a word that does nothing but put a string down. That is a
perfectly good thing for a word to be: a _constant_, a value with a name.
When the council wants the ledger to say "pence", there is one place to change
it.

The values go down in reverse: the due, the cargo, the units, the name. Because
`dues` only needs a number under it, `40 dues` can be the first thing on the
line. And `size` from Chapter 5, the word that left its answer on the stack
instead of printing it, drops straight into a template:

#session("ch07-size", caption: "A word that leaves text can be laid in")

#words((
  ("format", "( values template -- s )", [Fill the template's `{names}` from the
  stack, first name from the top.]),
  ("rule", "( -- )", [Print a rule of 36 dashes.]),
  ("heading", "( -- )", [Print the ledger's heading and a rule.]),
  ("wording", "( -- template )", [The template of one ledger line.]),
))

#section("Taking text apart")

The opposite of joining is splitting. `string.tokenize` breaks a string at its
spaces and leaves the pieces in a list.

#session("ch07-tokens", caption: "A string into words, and each word shouted")

The list was left on the stack by the first line and `loop`, on the second,
picked it up. That list is a value in its own right, and so far the book has
treated lists as nothing more than something for `loop` to walk along. The
next chapter looks at them properly.

#recap((
  [Keep your words in a `.bund` file, one definition after another, each with a
  comment giving its stack effect. `"file.bund" study` loads it at the terminal.],
  [`"double"`, `'single'` and `:atom` all make strings. Atoms are for names the
  program reads.],
  [`print` stays on the line; `println` ends it; `space` and `nl` do what they
  say.],
  [`+` joins strings, top first. A number joined to a string is written after
  it.],
  [`string.upper`, `string.lower` and `string.title` change case. `len` leaves the
  value and puts its length on top.],
  [`format` fills a template's `{names}` from the stack, first name from the top:
  put the values down in reverse order.],
  [A word that only puts a value down is a constant. `wording` is one.],
))

#problems((
  [Build the string `Gull (wax)` from `"Gull"` and `"wax"` using only `+` and
  whatever short strings you need. Mind the order.],
  [Define `shout ( s -- )`, which prints a string in capitals with an exclamation
  mark after it.],
  [Define `underline ( s -- )`, which prints a string and, on the next line, a row
  of dashes exactly as long as the string. You will need `len`, and you will
  need to deal with what `len` leaves behind.],
  [Write a template and one line that produces
  `The Osprey brought 25 barrels.` from the values `"Osprey"` and `25`.],
  [Print each cargo in `"salt herring wax"` on its own line, with its length
  after it.],
))
