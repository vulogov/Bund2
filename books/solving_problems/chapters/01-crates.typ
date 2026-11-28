#import "../design.typ": *

#let title = "Crates"

#let intro = [
#dropcap("B")efore a clerk can reckon he must be able to shift crates: to
copy the one on top, to throw one away, to bring the one beneath to hand.
Nothing in this chapter is arithmetic. It is all in knowing, before you press
the key, what the stack will look like afterwards.

Type `.s` after every answer. It shows the whole stack, bottom first, and
leaves it as it was.

#words((
  ("dup", "( a -- a a )", [Copy the top value.]),
  ("drop", "( a -- )", [Discard the top value.]),
  ("swap", "( a b -- b a )", [Exchange the top two values.]),
  (".  take", "", [Set the top aside on the counting table; take it back.]),
))
]

#let problems = (
  (id: "c01p1", title: "Three alike", grade: 1,
   ask: [Put down `7`, once. With two more words, leave `7 7 7`.],
   hint: none,
   answer: [`dup` copies whatever is on top, and the copy is as good as the
   original: the second `dup` copies the copy.]),

  (id: "c01p2", title: "The one underneath", grade: 1,
   ask: [`1 2` is on the stack. Leave only the `2`.],
   hint: [`drop` can only reach the top. Bring the unwanted crate there
   first.],
   answer: [`swap drop`: bring it up, throw it away. Other stack languages call
   this pair `nip`. If you use it often, give it that name.]),

  (id: "c01p3", title: "Turned and doubled", grade: 1,
   ask: [`1 2` is on the stack. Leave `2 1 1`.],
   hint: none,
   answer: [`swap` first, then `dup`. The other order, `dup swap`, exchanges
   two crates that are alike and so changes nothing.]),

  (id: "c01p4", title: "Read without lifting", grade: 2,
   ask: [`"Marta" 40` is on the stack. Print `Marta`, then `40`, each on its
   own line, and leave the stack exactly as you found it.],
   hint: [`println` uses up what it prints, so print a copy. The name is
   underneath: bring it up, and afterwards put it back where it was.],
   answer: [The same three words twice: `swap dup println`. The first time
   they bring the name up, copy it and print the copy. The second time they do
   the same for the number, and the second `swap` is also what puts the name
   back underneath.]),

  (id: "c01p5", title: "Say it before you run it", grade: 1,
   ask: [Without typing it, write down what `1 2 3 swap drop dup` leaves. Then
   type it and see.],
   hint: [Do it on paper, a word at a time, with the top at the right.],
   answer: [`swap` makes `1 3 2`; `drop` makes `1 3`; `dup` makes `1 3 3`.
   Reading a line this way, one word and one picture at a time, is the whole
   skill of the chapter.]),

  (id: "c01p6", title: "The crate at the bottom", grade: 3,
   ask: [`1 2 3` is on the stack. Leave `3 2 1`. First satisfy yourself that
   `dup`, `drop` and `swap` cannot do it alone, and say why. Then do it with
   the counting table.],
   hint: [`swap` reaches the top two crates and no further. To get at the
   third, something must be set aside.],
   answer: [#piles(
     ("1 2 3", ("1", "2", "3")),
     ("swap", ("1", "3", "2")),
     (".", ("1", "3")),
     ("swap", ("3", "1")),
     ("take", ("3", "1", "2")),
     ("swap", ("3", "2", "1")),
     note: "After the dot, the 2 waits on the table.",
   )
   The three stack words never touch the third crate down, so the `1` could
   never leave the bottom. Once the `2` is on the table the `1` is second from
   the top, and `swap` can reach it.]),
)
