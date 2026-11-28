#import "../design.typ": *
#chapter(number: 1, title: "A First Conversation")

#dropcap("B")efore a clerk can keep a ledger he has to turn up for work. This
chapter gets Bund onto your machine, opens the terminal you will use for the
rest of the book, and has Bund say good morning to the harbour. It also makes
the first mistake, on purpose, because the sooner you see what a mistake looks
like the less it will worry you.

#section("Getting Bund")

Bund is built from its source with the Rust toolchain. If you have `cargo`,
three commands fetch the source and build the `bund2` program. The first build
compiles a good deal, so allow it a few minutes.

```
$ git clone https://github.com/vulogov/Bund2
$ cd Bund2
$ cargo build --release -p bund2-cli
```

The result is one file, `target/release/bund2`. Put it somewhere on your `PATH`,
or type the path in full. Then ask it who it is.

#session("ch01-version", caption: "Is it there?")

The first line is the one that matters: the version, and the features it was
built with. The second says which revision of the original Bund this one was
checked against, which you may ignore until the day you report a bug.

#callout(label: "Bund and bund2")[
  The language is _Bund_. The program that runs it is `bund2`, because it is
  the second implementation of the language: a rewrite that keeps Bund's syntax
  and meaning. In this book "Bund" is the language and
  `bund2` is what you type.
]

#section("A program in a file")

A Bund program is text in a file, and by custom the file's name ends in
`.bund`. Here is a whole program. It is in the book's `code` folder, where every
program in this book lives.

#listing("hello.bund")

Hand the file to `bund2` and it runs.

#session("ch01-hello", caption: "Running a program")

Two things happened, in the order they are written. The text between the quotes
was put down. Then `println` picked it up and printed it on a line of its own.
That is the shape of everything in Bund: *put something down, then say what to
do with it*. The lines starting `//` are comments. Bund skips from `//` to the
end of the line, so they are for you and not for the clerk.

#term("Word")[
  Anything in a program that _does_ something is a word. `println` is a word.
  Words are separated by spaces and nothing else: there are no brackets after
  a word, no commas between them and no semicolon at the end of a line.
]

#section("The terminal")

Editing a file and running it is the right way to keep a program. It is a slow
way to learn a language. For that you want a conversation: type a line, see
what Bund makes of it, type the next. The book comes with a terminal that does
this. It is itself a Bund program, and it is in the `code` folder with the
others. Start it from inside that folder, so that it can find the files that
later chapters keep there.

```
$ cd books/Introduction/code
$ bund2 terminal.bund
 █▄▄ █ █ █▄ █ █▀▄
 █▄█ █▄█ █ ▀█ █▄▀

Bund, Word by Word: type help for commands, Ctrl-D to leave.
[BUND >
```

The prompt `[BUND >` means Bund is waiting. Type a line, press Return, and Bund
does what the line says.

#session("ch01-first", caption: "Good morning")

The second line holds two sentences and two `println`s, and Bund took them in
order: put down, print, put down, print. A line may hold as many words as you
like. Bund reads them from left to right and does each as it comes to it.

The last two lines show comments at the terminal. A line that is nothing but a
comment does nothing at all, and a comment after some words leaves the words
alone.

#section("The first mistake")

Now spell a word wrong.

#session("ch01-mistake", caption: "A mistake, and what is left behind")

Read this screen slowly, because all of Bund is in it.

The first line asked for `prinln`, and there is no such word. Bund says so:
`Inline prinln not registered`. _Registered_ is Bund's term for "known as a
word". Then comes a line that was not there before, `=> The Osprey is late.`
The terminal prints `=>` and whatever is now on top of the stack.

The stack is the subject of the next chapter. For now it is enough to see what
this tells you: *the sentence was put down before the mistake was found, and it
is still there.* Bund had already done the first half of the line. Nothing
picked the sentence up, so it stayed.

The second line types the whole thing again, correctly. A new copy of the
sentence is put down and `println` picks _that_ one up. The old copy is still
underneath, so `=>` still shows it.

The third line is one word, `println`, with nothing in front of it. It picks up
what was left over, and now nothing is. No `=>` line follows, because there is
nothing on top to show.

#warn(label: "A mistake does not undo what came before it")[
  When a line fails, everything on that line _before_ the failure has already
  happened. Whatever those words put down is still down. Look at the `=>`
  after an error before you type the line again, or you will be working on top
  of the leftovers. `clear` sweeps everything away.
]

The last line asks the terminal for `help`. Its four commands are not Bund
words. They belong to the terminal, and they are the only things you can type
at the prompt that are not Bund. The word `study` in its last line is for
Chapter 7.

#words((
  ("println", "( x -- )", [Pick up the top thing and print it on a line of its own.]),
  ("//", "", [The rest of the line is a comment.]),
  (".s", "terminal", [Show the whole stack, bottom to top. Changes nothing.]),
  ("clear", "( ... -- )", [Throw away everything that has been put down.]),
  ("quit", "terminal", [Leave. So does Ctrl-D.]),
))

The middle column is new, and from Chapter 4 on it will matter. It says what a
word picks up and what it leaves: `( x -- )` reads "takes one thing, leaves
nothing". For now, glance at it and move on.

#section("What kind of language is this?")

You have now seen the whole of Bund's grammar. There are things you put down,
and there are words that pick them up. A program is a row of the two, read from
left to right. There is no more syntax to learn than that. What there is to
learn is the stack, where the things are put, and that is next.

#recap((
  [Build Bund with `cargo build --release -p bund2-cli`; the program is
  `target/release/bund2`, and `bund2 --version` says which one you have.],
  [A program is a `.bund` file run with `bund2 file.bund`. For learning, use the
  book's terminal: `bund2 terminal.bund`, started in the book's `code` folder.],
  [Bund's grammar is *put something down, then say a word*. Words are separated
  by spaces; `//` starts a comment.],
  [`println` picks up the top thing and prints it. The terminal shows the new top
  of the stack as `=>` after every line.],
  [A line that fails has already done everything before the failure. What was put
  down stays down until a word picks it up or you `clear`.],
))

#problems((
  [Have Bund print the names of the three ships, one to a line, with a single
  line typed at the terminal.],
  [Type `"Marta" "Gull" println`. Before you press Return, say what will be
  printed and what `=>` will show. Then say what one more `println` prints.],
  [Type `println println` on an empty stack. How many errors do you get, and
  why not two?],
))
