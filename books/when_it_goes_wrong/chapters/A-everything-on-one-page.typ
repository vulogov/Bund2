#import "../design.typ": *
#appendix(letter: "A", title: "Everything on a Few Pages")

Every flag, console command and word this book has used, with the chapter
that shows it.

#section("Commands of bund2")

#words((
  ("bund2 FILE", "", [Run a program. A failure is reported and the exit
  code is 0. _1_]),
  ("bund2 check --file F", "", [Find a short stack without running. _3_]),
  ("bund2 infer --file F", "", [Run, then test each word's inferred
  effect. _3_]),
  ("bund2 effects", "", [Every built-in word's declared effect. _3_]),
  ("bund2 words", "", [Every name a program can call.]),
  ("bund2 build --check", "", [Analyse and write nothing; `--strict`
  refuses on a finding. _3_]),
))

#section("Flags of a run")

#words((
  ("--report MODE", "", [`auto`, `box` or `trace`: how a failure is
  drawn. _1_]),
  ("--no-dump-stack", "", [Leave the stack out of the report. _1_]),
  ("--raw-values", "", [Values in the report as Bund holds them. _1_]),
  ("-q", "", [Errors only. _2_]),
  ("-v  -vv  -vvv", "", [Let `log.info`, `log.debug`, `log.trace`
  through. _2_]),
  ("--trace FILE", "", [One line per word entered. _5_]),
  ("--debugger", "", [Stop before the first value. _6_]),
  ("--debug-shell", "", [Open a shell on the program after a failure. _9_]),
  ("--profile BASE", "", [Write a profile when the program ends. _10_]),
  ("--stats", "", [What was compiled, in a `jit` build. _10_]),
))

#section("The environment")

#words((
  ("BUND2_REPORT", "", [As `--report`. _1_]),
  ("BUND_LOG_LEVEL", "", [The `log` threshold. _2_]),
  ("BUND2_VERBOSE", "", [As `-v`, for a built program.]),
  ("BUND2_NO_DUMP_STACK", "", [As the flag, for a built program.]),
  ("BUND2_RAW_VALUES", "", [As the flag, for a built program.]),
  ("NO_COLOR", "", [No colour in what is printed.]),
))

#section("At the debugger's prompt")

#words((
  ("s  step", "", [One value, into any word. _6_]),
  ("n  next", "", [One value at this depth. _6_]),
  ("f  finish", "", [Until the current word ends. _6_]),
  ("c  continue", "", [Until something armed stops it. _6_]),
  ("bt", "", [The frames. _6_]),
  ("st", "", [The current stack. _6_]),
  ("info", "", [What is armed, with hit counts. _7_]),
  ("break WORD", "", [Before each call of a word. _7_]),
  ("break WORD N", "", [At the Nth call. _7_]),
  ("break WORD if { }", "", [When the lambda says yes. _7_]),
  ("break N  break F:N", "", [At a line. _7_]),
  ("until WORD", "", [Run to a word, once. _7_]),
  ("words", "", [Words and their calls; `words all`, `words TEXT`. _7_]),
  ("classes", "", [The classes. _7_]),
  ("watch depth > N", "", [When the stack grows past N. _8_]),
  ("watch redefine", "", [When a word is defined again. _8_]),
  ("watch @STACK", "", [At a push to a named stack; may take `if`. _8_]),
  ("watch workbench", "", [At a push to the counting table. _8_]),
  ("catch", "", [Where a failure is raised. _8_]),
  ("delete NAME", "", [A word, a line, `@stack`, `workbench`, `depth`,
  `redefine` or `catch`. _7, 8_]),
  ("?  help", "", [The list of commands.]),
  ("any other line", "", [Run as Bund in the stopped program. _6_]),
  ("Ctrl-D", "", [Detach; the program runs on. _6_]),
))

#section("Words that fail and catch")

#words((
  ("?try", "( -- conditional )", [Set `:try` and `:except`; run with `!`.
  _2_]),
  ("raise", "( text -- )", [Fail with a reason. _2_]),
  ("?error", "( -- error )", [An error value; `!` prints it as a notice.
  _2_]),
  ("log.error  log.warning", "( text -- )", [_2_]),
  ("log.info  log.debug  log.trace", "( text -- )", [_2_]),
))

#section("Words that look")

#words((
  ("debug.stacks", "( -- )", [Every stack. _4_]),
  ("debug.backtrace", "( -- )", [The frames. _4_]),
  ("debug.info", "( -- )", [What is armed. _4_]),
  ("debug.words  debug.classes", "( -- )", [The tables. _4_]),
  ("debug.display_stack", "( -- )", [The stack, raw. _4_]),
  ("debug.display_workbench", "( -- )", [The counting table, raw. _4_]),
  ("debug.dump", "( x -- x )", [The bytes of a value. _4_]),
  ("debug.display_memstat", "( -- )", [Memory in use. _4_]),
  ("debug.display_hostinfo", "( -- )", [Versions and host. _4_]),
  ("debug.display_distributed_info", "( -- )", [For distributed runs. _4_]),
  ("type  ?type", "", [The kind of a value. _4_]),
  ("?effect", "( name -- record )", [A built-in word's effect. _3_]),
))

#section("Words that stop")

#words((
  ("debug.step  debug.next", "( -- )", [_9_]),
  ("debug.finish  debug.continue", "( -- )", [_9_]),
  ("debug.break", "( word -- )", [_9_]),
  ("debug.break.at", "( word n -- )", [_9_]),
  ("debug.break.if", "( word text -- )", [_9_]),
  ("debug.until", "( word -- )", [_9_]),
  ("debug.catch", "( -- )", [_9_]),
  ("debug.watch", "( stack -- )", [_9_]),
  ("debug.watch.if", "( stack text -- )", [_9_]),
  ("debug.watch.workbench", "( -- )", [_9_]),
  ("debug.watch.depth", "( n -- )", [_9_]),
  ("debug.watch.redefine", "( -- )", [_9_]),
  ("debug.delete", "( name -- )", [_9_]),
  ("debug.run", "( text -- )", [Step through a string. _9_]),
  ("debug.shell", "( -- )", [A prompt of plain Bund. _9_]),
  ("debug.feed", "( text -- )", [Answer the next `input`. _9_]),
  ("debug", "( text -- )", [The first Bund's term-by-term debugger. _9_]),
  ("debug.profile", "( base -- )", [A profile of the run so far. _10_]),
))
