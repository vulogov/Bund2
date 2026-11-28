#import "../design.typ": *
#appendix(letter: "A", title: "The Words of This Book")

Every Bund word the book used, grouped by what it is for, with its stack
effect. In an effect the top of the stack is on the right. Where a word takes
two operands, the one on the right is the one it reads first.

This is a glossary of about seventy words out of the twelve hundred that
`bund2 words` will list. It is the part every other part stands on.

#section("Printing")

#words((
  ("println", "( x -- )", [Print a value and end the line.]),
  ("print", "( x -- )", [Print a value and stay on the line.]),
  ("space", "( -- )", [Print one space.]),
  ("nl", "( -- )", [End the line.]),
))

#section("The stack")

#words((
  ("dup", "( a -- a a )", [Copy the top value.]),
  ("drop", "( a -- )", [Discard the top value.]),
  ("swap", "( a b -- b a )", [Exchange the top two values.]),
  ("clear", "( ... -- )", [Empty the current stack.]),
  ("type", "( a -- a name )", [The name of the top value's kind, put on top of it.]),
  ("fold", "( ... -- list )", [The whole stack as a list, top first.]),
  ("*loop", "( ... lambda -- )", [Run the lambda for each value on the stack until it
  is empty.]),
  ("*+", "( ... -- sum )", [Add up everything on the stack.]),
))

#section("Arithmetic")

#words((
  ("+", "( a b -- a+b )", [Add numbers; join strings, b first; append a number to a
  string.]),
  ("*", "( a b -- a*b )", [Multiply; repeat a string.]),
  ("-", "( a b -- b-a )", [Subtract a from b.]),
  ("/", "( a b -- b/a )", [Divide b by a. Two integers give an integer.]),
  ("convert.to_int", "( x -- n )", [An integer from a string or a float.]),
  ("convert.to_float", "( x -- f )", [A float from an integer or a string.]),
  ("convert.to_string", "( x -- s )", [Text from any value.]),
))

#section("Defining words")

#words((
  ("{ ... }", "( -- lambda )", [A piece of program, put down and not run.]),
  ("!", "( lambda -- ... )", [Run a lambda, a pointer or a `?try`.]),
  ("register", "( name lambda -- )", [Make the name a word.]),
  ("unregister", "( name -- )", [Forget a word you defined.]),
  ("alias", "( name new -- )", [Give a word a second name.]),
  ("?word", "( name -- flag )", [Is there such a word?]),
  ("lambda=", "( name -- lambda )", [The body of a word you defined.]),
  ("`name", "( -- pointer )", [A pointer to a word, put down and not run.]),
))

#section("Questions and decisions")

#words((
  ("true  false", "( -- flag )", [The two booleans.]),
  ("==  !=", "( a b -- flag )", [Equal; different.]),
  (">  <", "( a b -- flag )", [Is b greater than a; is b less than a.]),
  (">=  <=", "( a b -- flag )", [Is b at least a; is b at most a.]),
  ("not", "( flag -- flag )", [The opposite.]),
  ("and  or", "( a b -- flag )", [Both; either.]),
  ("if", "( flag lambda -- )", [Run the lambda when the flag is true.]),
  ("ifthenelse", "( flag else then -- )", [Run `then`, the lambda on top, when the
  flag is true; otherwise `else`.]),
  ("?try", "( -- conditional )", [A guarded piece of program: set `:try` and
  `:except`, run with `!`.]),
))

#section("Repetition")

#words((
  ("times", "( n lambda -- )", [Run the lambda n times, pushing 0 … n−1 before each
  run.]),
  ("loop", "( list lambda -- )", [Run the lambda for each element, with the element
  on top.]),
  ("map", "( list lambda -- list )", [Run the lambda for each element and collect
  what it leaves.]),
  ("while", "( flag lambda -- )", [While the flag on top is true, run the lambda,
  which leaves the next flag.]),
  ("for", "( lambda -- )", [Run the lambda, which leaves a flag; repeat while it is
  true.]),
))

#section("Text")

#words((
  ("\"...\"  '...'  :atom", "( -- s )", [Three ways to write a string.]),
  ("format", "( values template -- s )", [Fill the template's `{names}` from the
  stack, first name from the top.]),
  ("string.upper", "( s -- s )", [Capitals.]),
  ("string.lower", "( s -- s )", [Small letters.]),
  ("string.title", "( s -- s )", [A capital on each word.]),
  ("string.tokenize", "( s -- list )", [The words of a string.]),
  ("len", "( x -- x n )", [The length of a string, list or dictionary. Leaves the
  value.]),
))

#section("Lists")

#words((
  ("[ ... ]", "( -- list )", [A list of what is written. Words inside are kept, not
  run.]),
  ("list", "( -- list )", [An empty list.]),
  ("car", "( list -- x )", [The first element.]),
  ("cdr", "( list -- list )", [All but the first element.]),
  ("sort", "( list -- list )", [In order.]),
  ("unique", "( list -- list )", [Without repeats.]),
  ("++", "( a b -- list )", [Joined: b's elements, then a's.]),
  ("unfold", "( list -- ... )", [The elements onto the stack, last on top.]),
))

#section("Dictionaries")

#words((
  ("dict", "( -- dict )", [An empty dictionary.]),
  ("set", "( dict key value -- dict )", [Write a value under a key.]),
  ("get", "( dict key -- value )", [The value under a key. Uses up the dictionary.]),
  ("?key", "( dict key -- dict flag )", [Is the key there? Leaves the dictionary.]),
))

#section("The workbench and the stacks")

#words((
  (".", "( x -- )", [Move the top of the stack to the workbench.]),
  ("take", "( -- x )", [Move the top of the workbench to the stack.]),
  ("+.", "( n -- )", [Add n to the number on the workbench; the sum stays there.]),
  ("@name", "", [Make the stack `name` current, creating it if need be.]),
  ("current", "( -- name )", [The current stack's name.]),
  ("to_current", "( name -- )", [Make an existing stack current, by name.]),
  ("stack_exists", "( name -- flag )", [Is there such a stack?]),
  ("->", "", [Step to the stack made before the current one.]),
))

#section("Programs")

#words((
  ("args.parse", "( -- dict )", [The command line, as a dictionary of lists.]),
  ("lambda*", "( ... -- lambda )", [The whole stack as one lambda, in order.]),
  ("convert.to_list", "( lambda -- list )", [A lambda's contents, as a list.]),
  ("bund.eval", "( text -- ... )", [Run a string as Bund.]),
  ("use", "( address -- )", [Fetch a file of Bund by its `file://` address and run
  it.]),
  ("cwd", "( -- path )", [The folder the program was started in.]),
  ("input*", "( prompt lambda -- )", [Read lines until there are no more; run the
  lambda on each.]),
  ("bund.exit", "( code -- )", [Stop the program.]),
))

#section("The words you wrote")

These are not Bund's. They are yours, and they are in `ledger.bund` and the
files of the chapters named.

#words((
  ("due", "( units -- pennies )", [Ch. 4. Three pennies a unit.]),
  ("fee", "( pennies -- pennies )", [Ch. 4. Add the mooring fee.]),
  ("dues", "( units -- pennies )", [Ch. 4. `due fee`.]),
  ("large?", "( units -- flag )", [Ch. 5. More than 30 units.]),
  ("size", "( units -- text )", [Ch. 5. `"large"` or `"small"`.]),
  ("note", "( units -- )", [Ch. 5. Print the size.]),
  ("rule", "( -- )", [Ch. 7. A rule of dashes.]),
  ("heading", "( -- )", [Ch. 7. The ledger's heading. In Ch. 12 it takes the day.]),
  ("wording", "( -- template )", [Ch. 7. How a ledger line reads.]),
  ("total", "( list -- sum )", [Ch. 8. Add a list up.]),
  ("ship", "( name cargo units -- record )", [Ch. 9. One ship's record.]),
  ("arrivals", "( -- r r r )", [Ch. 9. The day's three ships.]),
  ("gather", "( ... -- list )", [Ch. 9. The whole stack as a list, in order.]),
  ("brief", "( record -- )", [Ch. 9. A ship's name and what she owes.]),
  ("peek", "( -- x )", [Ch. 10. A copy of the top of the workbench.]),
  ("has", "( key -- value )", [Ch. 10. A field of the record on the workbench.]),
  ("fields", "( -- due cargo units name )", [Ch. 10. What the wording asks for.]),
  ("line", "( record -- )", [Ch. 10. One line of the ledger.]),
  ("manifest", "( -- list )", [Ch. 11. The arrivals, gathered on their own stack.]),
  ("berth", "( record -- )", [Ch. 11. To the wide quay or the narrow.]),
  ("entry", "( record -- )", [Ch. 12. Reckon a ship if her record allows.]),
))
