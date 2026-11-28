#import "../design.typ": *
#appendix(letter: "C", title: "Answers to the Problems")

Every answer here was run. Where a problem has more than one good answer, this
is one of them; if yours leaves the same thing on the stack, yours is right.
A `clear` between answers has been left out of the screens.

#section("Chapter 1")

#session("ans01", caption: "Chapter 1")

*1.* One line, three strings, three `println`s. *2.* `println` picks up the top,
which is `"Gull"`, and `=>` shows `Marta` still there; the next `println` prints
her. *3.* One error. A line stops at its first failure, so the second `println`
never runs.

#section("Chapter 2")

#session("ans02", caption: "Chapter 2")

*1.* `swap drop swap`. *2.* `7 7`. *3.* `swap` brings the name to the top.
*4.* The third `println` fails, after two 5s were printed: `dup` made two
values, not three.

#section("Chapter 3")

#session("ans03", caption: "Chapter 3")

*1.* 7, −7, 4 and 0. *2.* `5 12 -` is 12 − 5. *3.* With the numbers the other
way up, `swap` first. *4.* A tenth of 140 is `10 140 /`, 14; then `140 -` is
140 − 14, because the 140 just typed is on top. *5.* Write the end first:
`": "` and `"Marta"` join to `Marta: `, and a number joined to a string goes
after it.

#section("Chapter 4")

#session("ans04", caption: "Chapter 4")

*2.* `spoiled` is `swap -`. The units are underneath and must be what is
subtracted _from_, so they have to come to the top. *4.* `dup *`: a number
times its own copy. *5.* 145. Only `fee` was changed, and `dues` followed.

#section("Chapter 5")

#session("ans05", caption: "Chapter 5")

*1.* `5 9 <` asks "is 9 less than 5?". *4.* `fair?` asks two questions of one
number, so it `dup`s. `10 <` is "is 10 less than it?" and `30 >=` is "is 30 at
least it?". *5.* Both lambdas begin with `dues`; only the large course adds 10.
The `dup` is there because `large?` uses up a copy.

#section("Chapter 6")

#session("ans06", caption: "Chapter 6")

*1.* The counter starts at 0, so add 1 before printing. *2.* Each round leaves
1 or 0 and adds it to the tally underneath. *3.* `dup`, ask, then `println` in
one course and `drop` in the other, so that both use up the copy. *4.* Seven
trips, the last three barrels short of a load. *5.* 729.

#section("Chapter 7")

#session("ans07", caption: "Chapter 7")

*1.* Four pieces, written last first. *2.* `"!" swap +` puts the mark after the
text. *3.* `len` leaves the string under its length, so `swap drop` removes it;
`dashes` is a word of its own because a line of dashes is worth having.
*5.* `measure` prints the string and then its length: `len` left them in the
order length-on-top, so `swap` comes first.

#section("Chapter 8")

#session("ans08", caption: "Chapter 8")

*1.* `len` leaves the list, so `drop` it after printing. *4.* `sort` puts the
largest last, `unfold` puts the last on top, `fold` makes the top the first
element, and `car` takes it. It must start from an empty stack because `fold`
sweeps up everything. Chapter 11 mends that. *5.* `unfold fold` reverses a
list, for the same reason. *6.* `total` leaves the sum on top of the count, and
`/` divides the top by what is underneath.

#section("Chapter 9")

#session("ans09", caption: "Chapter 9")

*1.* Eight bales are 24 pennies and the mooring 20. *5.* `owes` `dup`s the
record before asking it, so the record survives. Then `:due swap set`: the
`swap` puts the key under the value.

#section("Chapter 10")

#session("ans10", caption: "Chapter 10, problems 1 to 4")

*1.* `1 2`. The 3 is still on the table, as the next `take` shows. *2.* and
*3.* The table holds one value while `swap` or `dup` works on the other.
*4.* 77: with no `dues` in the lambda, `+.` adds the units themselves.

*5.* The new name is last in the wording, so its value must go down _first_:

```
:wording { "{name}: {units} of {cargo}, {due} pennies, {size}" } register

:fields {
  :units has size
  :units has dues
  :cargo has
  :units has
  :name  has
} register
```

#session("ans10b", caption: "Chapter 10, problem 5")

#section("Chapter 11")

#session("ans11", caption: "Chapter 11, problems 1 and 2")

*1.* Add up `spare`, carry the 30 across on the table, add up `main`: 36.
*2.* `swap .` puts the value on the table and leaves the name, which
`to_current` uses.

#session("ans11b", caption: "Chapter 11, problems 3 to 5")

*3.* The list is carried to a stack called `work`, the answer carried back, and
`"mine"` is undisturbed. *4.* Two. `lambda*` freezes the quay so that `len` can
count it, and `!` puts the ships back. *5.* `unfold fold` again, somewhere it
can do no harm.

#section("Chapter 12")

#session("ans12", caption: "Chapter 12, problems 1 and 2")

*1.* Three ships at five pennies more each: 306. *2.* The _Heron_ owes 44, so
the day comes to 335.

#session("ans12b", caption: "Chapter 12, problem 3")

*3.* On the first line `check` finds it. After `three` it does not, and says
why: it could not work out how many values `three` leaves, so it stopped
counting there and claims nothing about what follows. Running the program
would find it at once.

#section("Chapter 13")

#session("ans13", caption: "Chapter 13")

*1.* The body of `dues` is two words, shown as `F(due)` and `F(fee)`. *2.* A
body is a lambda, and `!` runs it. *3.* The line to add to `term.commands` is
`"depth" { lambda* len . ! take println } set`: freeze the stack, count it, put
the count on the table, thaw, fetch the count. *4.* `dup .` keeps a spare copy
of the lambda on the table for the second run. *5.* `moor` is `rate` with
`fee` and `+` in place of `due` and `*`.
