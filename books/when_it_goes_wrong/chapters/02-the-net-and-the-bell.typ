#import "../design.typ": *
#chapter(number: 2, title: "The Net and the Bell")

#dropcap("A") harbour does not wait for a ship to strike before it thinks
about ships striking. There is a net under the gangway and a bell on the
mole. This chapter is about the two things a program can do about failure
before any tool is brought to it: catch one, and say what it is doing.

#section("The net")

The first book introduced `?try`. A piece of program goes in under `:try`,
another under `:except`, and `!` runs it. If the first fails, the second
runs in its place.

#session("c2-try", caption: "Catching a failure, and looking at what was caught")

The first line adds with nothing on the stack. It fails, and `except` prints
a sentence.

Look at what `except` was given. It drops something before it prints. When
the `try` part fails, Bund puts a value on the stack that describes the
failure, and runs `except` with it there. The other lines look at that
value.

It is a small record. Under `:context` is the text of the failure, the same
text a report would have shown. `0 1 /` is one divided by nought, and the
text says so.

Its type is `Conditional`, the same kind of thing as `?try` itself. That
means it can be run, and the last line runs it with `!`. Running an error
value prints it as a notice and lets the program go on. It is the quickest
way to say "this went wrong, and I am carrying on".

#words((
  ("?try", "( -- conditional )", [A guarded piece of program. Set `:try` and
  `:except`; run with `!`.]),
  (":context get", "( error -- text )", [The reason, from a caught error.]),
  ("!", "( error -- )", [Run on a caught error: print it as a notice.]),
))

#section("Raising your own")

A failure need not come from Bund. `raise` takes a string and fails with it.

#session("c2-raise", caption: "A failure of your own making")

The first line fails, and the terminal reports it as it would any other.
The second catches it: the text you raised is in the `:context`, after the
words Bund puts in front of every failure inside a lambda.

`?error` puts down an empty error value. Give it a `:context` and run it,
and you have printed a notice without anything having failed.

#words((
  ("raise", "( text -- )", [Fail, with this reason.]),
  ("?error", "( -- error )", [An error value, to be filled and run.]),
))

#warn(label: "A net is not a floor")[
  The first book said it and it bears repeating. Catch at the edge of the
  program, where the world comes in. A failure caught and dropped in the
  middle of your own words is a fault nobody will ever be told about, and
  every tool in the rest of this book will be blind to it.
]

#section("The bell")

A program can say what it is doing as it goes, at five levels of urgency.

#piece("log.bund", "\"reckoning begins\"", rest: true, caption: "log.bund")

Each `log` word takes a string. Whether it is shown depends on how the
program was run.

#session("c2-log", caption: "The same program, run three ways")

Run plainly, only `log.error` speaks. `-q` silences even that. `-v` lets
`log.info` and `log.warning` through, and `bund2` itself says what it is
running.

#session("c2-loud", caption: "Everything, and a threshold from the environment")

`-vv` adds `log.debug` and `-vvv` adds `log.trace`. `BUND_LOG_LEVEL` sets
the threshold without a flag, which is useful for a program somebody else
starts.

The log lines go to standard error, so they do not get mixed into a page
the program is printing to a file.

#words((
  ("log.error", "( text -- )", [Always shown, unless `-q`.]),
  ("log.warning  log.info", "( text -- )", [Shown from `-v`.]),
  ("log.debug", "( text -- )", [Shown from `-vv`.]),
  ("log.trace", "( text -- )", [Shown from `-vvv`.]),
  ("-q  -v  -vv  -vvv", "flags", [Errors only; then info, debug, trace.]),
  ("BUND_LOG_LEVEL", "environment", [The threshold: `error`, `warning`,
  `info`, `debug` or `trace`.]),
))

#callout(label: "Leave the bell in")[
  A `println` put in to find a fault has to be taken out again. A
  `log.debug` can stay. It costs almost nothing when nobody is listening,
  and the next person to hunt a fault in that word will find the bell
  already hung.
]

#recap((
  [`?try` catches a failure. `except` is run with an error value on the
  stack; its `:context` is the reason.],
  [An error value run with `!` prints as a notice and the program goes on.],
  [`raise` fails with a reason of your own; `?error` makes an error value
  without a failure.],
  [The `log` words speak at five levels, and `-q`, `-v`, `-vv`, `-vvv` or
  `BUND_LOG_LEVEL` choose how many are heard.],
))
