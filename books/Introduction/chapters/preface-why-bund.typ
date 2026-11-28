#import "../design.typ": *

#v(1cm)
#align(center)[
  #text(font: body_family, size: 20pt, weight: "bold", fill: ink_black, "Why Bund?")
]
#v(6mm)

Most programming languages ask you to do the same thing before you may do
anything: give everything a name. A value needs a variable. A step needs a
function, the function needs parameters, and the parameters need names of their
own that mean nothing outside it. Half of what you type is not the work. It is
labels on boxes, so that you can find the box again three lines later.

Bund is built on a different bet: that most of those names are unnecessary. In
Bund you put a value down, and the next word picks it up. Nothing is named
unless the name is worth having. A program is a row of words read left to
right, each one taking what the last one left, and when you want a new word you
make it out of the words you already have. Languages of this family are called
_concatenative_, and the oldest of them, Forth, has been flying spacecraft and
aiming telescopes since before most of us wrote a line of code.

The place where values are put down is a _stack_, and here Bund goes its own
way. Forth has one stack for your data. Bund has as many as you like, each with
a name if you want one, and a workbench between them for the thing you need to carry from one to
another. That is what the name means. _Bund_ is the German word for a bundle, a
league, an association: not one stack but a bundle of them, each holding its
own part of the work so that the parts do not get under each other's feet.

I built Bund because I wanted a language whose rules are few enough to hold in
my head and honest enough to show me its state. There is no hidden machinery. Whatever your
program knows is on a stack where you can look at it. Whatever it can do is a
word you can list. A program is data, and you may build one while another is
running.

This book is the argument made concrete. It does not catalogue the twelve
hundred names Bund answers to. It takes one small job, the daily ledger of a
harbour, and follows it from the first line the language prints to a finished program,
meeting each idea at the moment the job needs it. There is one rule to learn on
the way, and it will feel backwards for about a chapter. After that it will
feel like the only sensible way to do it. I will tell you what it is on the
next page.

#v(4mm)
#line(length: 100%, stroke: 0.5pt + ink_rule)
