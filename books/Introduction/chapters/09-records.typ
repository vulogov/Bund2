#import "../design.typ": *
#chapter(number: 9, title: "Records")

#dropcap("W")hen a ship ties up, her master hands over a manifest: one sheet
that says what she is called, what she carries and how much of it. Nobody at
the customs house says "the second thing on the _Marta_'s sheet". They say "her
cargo". Each fact has a heading, and you ask for it by the heading.

The last chapter ended with a ship squeezed into a list, where the only way to
find her cargo was to count along. This chapter gives each fact its name. The
value that does it is called a _dictionary_, and a dictionary used to describe
one thing is a _record_.

#plate(8, "08-manifest.png", "One sheet, several facts, each under its own heading.")

#section("A value with headings")

The word `dict` puts down an empty dictionary. The word `set` writes one fact
into it.

#session("ch09-dict", caption: "A record, one fact at a time")

Each line adds a _key_ and a _value_: `:name` and `"Marta"`, `:units` and 40.
The dictionary stays on the stack throughout. `set` picks up the value, the key
and the dictionary, and puts the dictionary back with the new fact in it.

Notice the order of the three. The dictionary is at the bottom, the key above
it, the value on top: *the top speaks first*, and what `set` wants to hear first
is the value. Notice too that Bund shows the keys in alphabetical order, not in
the order you wrote them. A dictionary is not a row. It has no first and no
last, only headings.

#term("Dictionary")[
  A value that holds other values under _keys_. You put a value in under a key
  with `set` and ask for it by the key with `get`. A dictionary describing one
  thing — a ship, a cargo, a day — is called a *record*, and its keys are its
  *fields*.
]

Keys are strings, and this is what atoms are for. `:name` is quicker to type
than `"name"` and looks like what it is: a label, not a piece of text.

#section("Asking by the heading")

`get` takes a dictionary and a key, and leaves the value.

#session("ch09-get", caption: "get")

`.s` shows only the 40. `get` picked the dictionary up and did not put it back.
If you want to ask a second question, keep a copy.

#session("ch09-dup", caption: "dup before get")

This is the same lesson as `dup` before a test in Chapter 5. A word picks up
what it uses. If you will need the record again, `dup` it first.

Asking for a key that is not there is an error. You can find out beforehand
with `?key`, which leaves the dictionary where it was and puts the answer on
top.

#session("ch09-key", caption: "Is there such a heading?")

#section("Changing and adding")

`set` with a key that is already there replaces the value. With a new key it
adds a field. `len` counts the fields.

#session("ch09-change", caption: "Two crates fewer, and one more fact")

#words((
  ("dict", "( -- dict )", [Put down an empty dictionary.]),
  ("set", "( dict key value -- dict )", [Write a value under a key.]),
  ("get", "( dict key -- value )", [The value under a key. Uses up the dictionary.]),
  ("?key", "( dict key -- dict flag )", [Is the key there? Leaves the dictionary.]),
  ("len", "( dict -- dict n )", [The number of fields. Leaves the dictionary.]),
))

#section("A word that makes a ship")

Typing `dict :name ... set :units ... set` for every ship is the retyping of
Chapter 3 all over again. There should be a word, `ship`, that takes a name, a
cargo and a number and leaves a finished record.

There is a difficulty, and it is the first real one the stack has given you.
When `ship` starts, the three values are on the stack and the dictionary is
not. `set` wants the dictionary _underneath_ the key and the value. Everything
is the wrong way up.

Take it one field at a time. Suppose the units are on the stack, and an empty
dictionary has just been put on top of them.

#session("ch09-setter", caption: "Writing one field from underneath")

`units=` is `swap :units swap set`. The first `swap` brings the 40 above the
dictionary. `:units` goes on top. The second `swap` puts the key under the
value. Now it is dictionary, key, value, as `set` wants.

#piles(width: 22mm,
  ("40 dict", ("40", "{}")),
  ("swap", ("{}", "40")),
  (":units", ("{}", "40", ":units")),
  ("swap", ("{}", ":units", "40")),
  ("set", ("{units=40}",)),
  note: "units= — two swaps bring the three into the order set wants.",
)

That is a word with a clean effect: `( units record -- record )`. It takes the
value from under the record and writes it in. With one such word for each field,
`ship` is four words long. All of them are in `records.bund`:

#listing("records.bund")

Read `ship` with `"Marta" "salt" 40` on the stack. `dict` puts an empty record
on top. `units=` takes the 40 from under it. Now `"salt"` is under the record,
and `cargo=` takes that. Then `name=` takes `"Marta"`. Each word uses up the
value that is nearest, which is the one put down last. The fields are written
in the reverse of the order you gave them, and you never have to think about
it.

#session("ch09-ship", caption: "ship")

#callout(label: "On naming")[
  `units=` ends in an equals sign for the same reason `large?` ends in a
  question mark. A word that _asks_ ends in `?`. A word that _writes a field_
  ends in `=`. Bund does not care, since any run of characters without a space
  is a name. Your reader cares a great deal.
]

#section("A word that reads a ship")

`brief`, the last word in the file, prints a ship's name and what she owes. It
needs the record twice, once for the name and once for the units, so it begins
with `dup`.

#session("ch09-brief", caption: "brief")

`:units get dues` is worth a second look: ask the record for its units, and
hand them to `dues`. The word from Chapter 4 knows nothing about records and
does not need to. It takes a number.

#section("The day's arrivals")

`arrivals` puts all three records on the stack.

#session("ch09-arrivals", caption: "Three records, last in first out")

They come off in reverse, the _Osprey_ first. The stack is a pile, and a pile
is the wrong thing for a day's manifest, which ought to be a row in the order
the ships came in. You know from the last chapter that brackets will not help:
`[ "Marta" "salt" 40 ship ]` would collect the word `ship` without running it.

The records have to be made first and collected afterwards. `gather` does the
collecting.

#session("ch09-gather", caption: "The whole stack, as a list in order")

`gather` sweeps the stack into a list and keeps the order: bottom first. It is
`fold` from the last chapter with the order the right way round, and it carries
the same warning. It takes _everything_ on the stack, so use it on a stack that
holds only what you mean to gather. Chapter 11 makes that safe.

#callout(label: "For the curious")[
  `gather` is two words, `lambda*` and `convert.to_list`. The first turns the
  whole stack into a lambda, in order, and the second turns a lambda into a
  list. Why Bund should have a word that makes program out of whatever is lying
  on the stack is the subject of Chapter 13.
]

#section("A list of records")

A list of records is the shape most real data has, and everything from the
last three chapters works on it unchanged.

#session("ch09-loop", caption: "The ledger, nearly")

`{ brief } loop` prints a line for each ship. `{ :units get dues } map` turns
the list of records into a list of dues, and `total` adds them. It is 291.

The lambda given to `map` is how you pull one field out of every record:

#session("ch09-names", caption: "One column of the manifest")

#words((
  ("ship", "( name cargo units -- record )", [One ship's record.]),
  ("arrivals", "( -- r r r )", [The three ships of the day, as three records.]),
  ("gather", "( ... -- list )", [The whole stack as a list, bottom first.]),
  ("brief", "( record -- )", [Print a ship's name and what she owes.]),
))

#section("Where this strains")

`brief` used its record twice and got by with one `dup`. The ledger line from
Chapter 7 wants four things from a record: the name, the units, the cargo and
the due. Try to write it and you will find yourself juggling: a `dup` here, a
`swap` there, the record always in the way of the value you have just fetched,
and a definition nobody could read.

When that happens in Bund it is not a sign that you need to be cleverer with
`swap`. It is a sign that something should be _put down somewhere else_ for a
moment. Bund has a place for that, and it is the next chapter.

#recap((
  [A *dictionary* holds values under keys. `dict` makes an empty one; `set` writes
  a value under a key; `get` asks for it. A dictionary describing one thing is a
  *record*.],
  [`set` wants the dictionary, then the key, then the value on top.],
  [`get` uses up the dictionary: `dup` first if you need it again. `?key` and `len`
  leave it where it was.],
  [`units=` is `swap :units swap set`, a word that writes a field from under the
  record. `ship` is `dict units= cargo= name=`.],
  [`gather` collects the whole stack into a list in order, so records can be made
  first and collected afterwards.],
  [`loop`, `map` and `total` work on a list of records as they did on a list of
  numbers: `{ :units get dues } map total` is 291.],
))

#problems((
  [Build a record for a fourth ship, the _Heron_, carrying 8 bales of wool, and
  print her brief.],
  [Define `cargo-of ( record -- cargo )`. It is very short.],
  [Define `heavy? ( record -- flag )`, true when the ship's cargo is large. Use
  `large?`.],
  [From `arrivals gather`, make the list of the three cargoes' sizes: `"large"`
  or `"small"` for each ship, in order.],
  [Define `owes ( record -- record pennies )`, which leaves the record _and_ what
  the ship owes. Then use it to add a field `:due` to a record.],
))
