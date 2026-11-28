#import "../design.typ": *
#chapter(number: 3, title: "The Top Speaks First")

#dropcap("F")orty crates of salt are on the quay and the harbour is owed three
pennies for each, and twenty more for the mooring. A clerk who cannot multiply
is no use to anyone. This chapter teaches Bund's arithmetic, reckons the
_Marta_'s due by hand, and on the way meets the one rule in the language that
runs against habit. Learn it here and nothing later in the book will trip you.

#section("Adding")

The word for addition is `+`. It picks up the top two values and puts their sum
down in their place.

#session("ch03-add", caption: "Two crates in, one crate out")

#piles(
  ("40 12", ("40", "12")),
  ("+", ("52",)),
  ("25", ("52", "25")),
  ("+", ("77",)),
)

The numbers come first and the word comes after them. There is no other way
round, because a word can only act on what is already on the stack. You never
need brackets, and there is no order of operations to remember: things happen
in the order you wrote them.

Multiplication is `*`. That is all the arithmetic the _Marta_'s due needs.

#session("ch03-due", caption: "The Marta's due, by hand")

Forty crates at three pennies is 120. Twenty for the mooring makes 140. Keep
that number in mind: it comes back in every chapter.

#section("Taking away")

Now the rule. Take three from ten.

#session("ch03-sub", caption: "Which from which?")

`10 3 -` is not 7. It is −7. Bund took ten from three.

#plate(3, "03-top-crate.png", "The crate in his hands is the one he reads first.")

This is not a mistake and it is not a special case. It is how every word in
Bund works:

#term("The top speaks first")[
  A word takes its _first_ operand from the top of the stack and its second
  from underneath. For `-`, the first operand is what you subtract _from_.
  So `10 3 -` has 3 on top, and reckons 3 − 10.
]

Think of the clerk at the quay. He lifts the top crate, and that is the one in
his hands: the first thing he reads. Then he looks at the one still on the
pile. "Three," he says, "less ten."

#piles(
  ("10 3", ("10", "3")),
  ("-", ("-7",)),
  note: "The top, 3, is the first operand: 3 − 10.",
)

To take three from ten, put the ten on top: `3 10 -`. Read a Bund sum from the
word _backwards_: `-`, then the 10 beside it, then the 3.

With `+` and `*` you never noticed, because 40 + 12 and 12 + 40 are the same.
With `-` and `/` the order is the whole point.

#session("ch03-div", caption: "Division follows the same rule")

`4 20 /` is 20 ÷ 4, which is 5. `20 4 /` is 4 ÷ 20, and in whole numbers that
is 0.

#warn(label: "This is the rule the chapter is named for")[
  `a b -` is *b − a*. `a b /` is *b ÷ a*. The value on top is the one the word
  starts from. If a result has the wrong sign, or a division gives 0, you have
  the operands the other way up. Every time.
]

#callout(label: "For Forth hands")[
  In Forth `10 3 -` is 7: the word reads its operands in the order you typed
  them. Bund reads them in the order it _lifts_ them. Neither is more correct.
  Bund's way has the virtue of being one rule that the comparisons, the
  decisions and the words for text all follow too, as the next chapters show.
]

#section("When the values are already there")

Often you do not get to choose the order. The values are on the stack already,
left by some earlier word, and they are the wrong way up. That is what `swap`
is for.

#session("ch03-swap", caption: "swap turns a sum round")

`10 3 swap -` puts down 10 and 3, exchanges them so that 10 is on top, and
subtracts: 10 − 3. You will write `swap -` and `swap /` often. Read them as
"minus, the other way" and "divided by, the other way".

#words((
  ("+", "( a b -- a+b )", [Add.]),
  ("*", "( a b -- a*b )", [Multiply.]),
  ("-", "( a b -- b-a )", [Subtract the value underneath from the top.]),
  ("/", "( a b -- b/a )", [Divide the top by the value underneath.]),
))

#section("Whole pennies")

Divide seven by two.

#session("ch03-whole", caption: "Integers stay integers")

`2 7 /` is 3, not 3.5. Both numbers were integers, so Bund gave an integer and
threw the half away. Make either of them a float and the answer is a float.
The word `convert.to_float` turns the integer on top into one.

For a ledger this is a feature. A harbour cannot be owed half a penny, and a
sum that is done in whole pennies from start to finish can never be off by a
fraction that somebody rounded differently. Every reckoning in this book is in
integers, and that is deliberate. When you do want fractions, write a decimal
point.

#section("Arithmetic on text")

`+` and `*` are not only for numbers.

#session("ch03-text", caption: "Joining and repeating")

`+` joins two strings, and the rule holds: the top speaks first. `"haven"` went
down first and `"Wend"` on top of it, so the result begins with `Wend`. To build
a phrase, write its _last_ part first.

`*` with a string and a number repeats the string: twenty `=` signs make a rule
to draw under a column. And `+` with a string and a number joins the number onto
the end of the text, which is the quickest way to label a result.

Not every pair makes sense, and Bund says so when it does not.

#session("ch03-wrong", caption: "Kinds that do not add up")

A string and a boolean cannot be added. The second line is the subtler trap:
`"40"` in quotes is text, not a number, so `"40" 3 *` is the text three times
over. `convert.to_int` turns text that looks like a number into the number.

#words((
  ("convert.to_int", "( x -- n )", [Make an integer of the top value.]),
  ("convert.to_float", "( x -- f )", [Make a float of the top value.]),
  ("convert.to_string", "( x -- s )", [Make text of the top value.]),
))

#section("The whole day, by hand")

Now reckon all three ships. Each due is the same sum with a different number at
the front.

#session("ch03-day", caption: "Three ships and a total")

Each of the first three lines leaves its answer on the stack and the next line
works on top of it. Nothing is lost between lines; that is what the stack is
for. After three lines there are three dues, and `+ +` adds them: the first `+`
makes 151 of 56 and 95, and the second adds 140.

The day's total is 291 pennies. That is the other number to remember.

When the whole stack is one column of figures, there is a word that adds all of
it at once.

#session("ch03-sum", caption: "Adding everything")

#words((
  ("*+", "( ... -- sum )", [Add up everything on the stack.]),
))

#section("What is wrong with this")

It works, and it is tiresome. `3 * 20 +` was typed three times, and if the
harbour raises its due to four pennies it must be changed in three places. The
sum has no name, so nobody reading it knows it is a due. A harbourmaster who had
to explain the reckoning afresh for every ship would find another clerk.

The next chapter teaches Bund the word `dues`.

#recap((
  [`+ - * /` each pick up two values and leave one.],
  [*The top speaks first*: a word's first operand is the top of the stack. So
  `a b -` is b − a and `a b /` is b ÷ a. Read a sum backwards from the word.],
  [`swap` before `-` or `/` turns the sum round when the values are already on
  the stack the other way up.],
  [Two integers give an integer: `2 7 /` is 3. Put a decimal point in, or use
  `convert.to_float`, to get a fraction. The ledger stays in whole pennies.],
  [`+` joins strings, top first; `*` repeats a string; `+` with a string and a
  number appends the number.],
  [The _Marta_ owes 140 pennies, the _Gull_ 56, the _Osprey_ 95, and the day's
  total is 291.],
))

#problems((
  [Without typing it, say what each of these leaves: `2 9 -`, `9 2 -`,
  `3 12 /`, `12 3 /`. Then check.],
  [The _Gull_ unloads 12 crates and 5 are found to be spoiled and are sent back
  aboard. Reckon the due on what is left, on one line, without using `swap`.],
  [Now do the same with the 12 and the 5 typed in that order, so that `swap` is
  needed.],
  [The harbour allows one penny in ten off the _Marta_'s due of 140 for prompt
  payment. Reckon the discount in whole pennies, and then what she pays.],
  [Make the string `Marta: 140` from the string `"Marta"`, the string `": "` and
  the number 140.],
))
