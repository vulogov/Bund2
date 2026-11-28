#import "../design.typ": *

#let title = "Records"

#let intro = [
#dropcap("A") ship is a dictionary of three fields, and `harbour.bund`
builds one with `ship`. The trap in this chapter is always the same: `get`
uses the record up. If you want the record afterwards, or a second field,
`dup` it first.

#plate("08-pigeonholes.png", "Each thing known about a ship has its own place.")

#words((
  ("set", "( dict key value -- dict )", [Write a value under a key.]),
  ("get", "( dict key -- value )", [The value under a key. Uses up the
  dictionary.]),
  ("?key", "( dict key -- dict flag )", [Is the key there? Leaves the
  dictionary.]),
  ("ship", "( name cargo units -- record )", [From `harbour.bund`.]),
  ("arrivals gather", "( -- list )", [The day's three ships, as a list. Start
  from an empty stack.]),
))
]

#let problems = (
  (id: "c08p1", title: "A fourth field", grade: 1,
   ask: [The _Heron_ brings 8 bales of wool and is given berth 3. Make her
   record with `ship`, add a field `:berth`, and then read the berth back
   while keeping the record on the stack.],
   hint: none,
   answer: [A record takes any field you set. `dup` before `get` keeps the
   record: `.s` shows the 3 with the record beneath it.]),

  (id: "c08p2", title: "Spoiled cargo", grade: 3,
   ask: [Two of the _Marta_'s crates are spoiled and are not charged. Define
   `spoil ( record n -- record )`, which leaves the record with its units
   reduced by `n`. After `2 spoil` her dues are 134.],
   hint: [Set `n` aside. Read the units from a copy of the record, subtract,
   and write the result back with `units=`, which wants the value under the
   record.],
   answer: [#words((
     (".", "", [n waits on the table.]),
     ("dup :units get", "", [The record, with its units on top.]),
     ("take swap -", "", [n comes back; the units go on top and n is taken
     from them.]),
     ("swap units=", "", [`units=` wants the record on top of the new
     value.]),
   ))]),

  (id: "c08p3", title: "All that came ashore", grade: 1,
   ask: [How many units of cargo, of every kind, came ashore from the three
   ships? The answer is 77.],
   hint: [A running total, and one field from each record.],
   answer: [`gather` sweeps up the whole stack, so the total's starting 0 is
   put down after it and swapped underneath the list.]),

  (id: "c08p4", title: "Hailing the large ships", grade: 2,
   ask: [Print the name of every ship in the day's arrivals whose cargo is
   large. Only the _Marta_ should be named.],
   hint: [The question needs the units and the printing needs the name: two
   fields of one record.],
   answer: [`big?` copies the record before taking its units, so the record
   is still there under the flag. Then one course takes the name and prints
   it and the other drops the record.]),

  (id: "c08p5", title: "A record with a field missing", grade: 2,
   ask: [Asking for a field that is not there is an error. Define
   `load ( record -- units )`, which leaves the units of a record, or 0 if the
   record has no `:units` field.],
   hint: [`?key` asks without using the record up.],
   answer: [`?key` leaves the record under its flag. When the field is there
   it is fetched; when it is not, `none` drops the record and puts down 0.]),

  (id: "c08p6", title: "Cargo by its worth", grade: 3,
   ask: [The town taxes cargo by kind: salt at 3 pennies a unit, wax at 5,
   herring at 2. Keep those rates in a dictionary, and define
   `value ( record -- pennies )`. The _Gull_'s wax is worth 60, and the three
   cargoes together 230.],
   hint: [A dictionary can be the answer of a word. Look the record's cargo
   up in it, and multiply by the record's units.],
   answer: [`price` puts down a fresh dictionary of rates each time it is
   said. `rate` takes the cargo from a record and looks it up: `swap` puts
   the cargo on top, as the key. `value` takes the units from a copy and the
   rate from the record itself.]),
)
