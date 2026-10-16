# A REPL written in Bund

`repl.bund` is a read–eval–print loop for Bund, written in Bund. It reads a
line, evaluates it in the same virtual machine it is running in, shows what is
on top of the stack, and reads the next line. Words you register stay
registered and the stack carries over from line to line, so a session builds
up a program the way a Forth session does.

    $ ./bund_repl
     █▄▄ █ █ █▄ █ █▀▄
     █▄█ █▄█ █ ▀█ █▄▀

    Bund REPL — type help for the commands, Ctrl-D to leave.
    [BUND> 1 2 +
    => 3
    [BUND> :sq { dup * } register
    => 3
    [BUND> 7 sq
    => 49
    [BUND> foo
    error: … foo not registered
    => 49
    [BUND> quit
    bye

## How it works

Three words from the standard library do the heavy lifting, and the rest is a
few lines of Bund around them.

- **`input*`** reads lines until Ctrl-D, pushing each one, trimmed, and running
  a lambda on it. `bund.prompt { repl.line } input*` is the whole loop: the
  coloured `[BUND>` prompt, the handler, and the reading.
- **`bund.eval`** parses a string and runs it in this VM. That is what makes
  the session cumulative: `:sq { dup * } register` typed at the prompt
  registers `sq` for every later line.
- **`?try`** catches a failing line instead of ending the program. It pushes
  the error as a value with the failure's text in its `context` slot; the
  loop prints that and carries on.

The loop's own commands — `help`, `stack`, `clear`, `quit` and their aliases —
live in a dictionary from the line's text to the lambda that handles it:

```bund
:repl.commands {
  dict
  "help"  { repl.help } set
  "stack" { debug.display_stack } set
  "quit"  { "bye" println 0 bund.exit } set
  …
} register
```

A line is looked up with `?key`; when it is there, `get !` fetches the lambda
and runs it, and when it is not, the line goes to `bund.eval`. The command
table is ordinary data, which is Bund's metaprogramming in its smallest form:
adding a command is adding an entry.

`repl.show` prints `=> value` after each line when the stack is not empty. It
does that by trying `dup` under `?try`: on an empty stack `dup` fails, and the
`except` branch drops the error value and prints nothing.

## Running it without building

    bund2 bund_examples/bund_repl/repl.bund

The program reads its lines from standard input, so a session can also be
scripted:

    printf '1 2 +\n7 sq\nquit\n' | bund2 bund_examples/bund_repl/repl.bund

## Building a standalone binary

    cd bund_examples/bund_repl
    make          # ./bund_repl
    make run      # start a session
    make try      # drive a scripted session through a pipe
    make clean

`bund2 build` compiles nothing. It copies the `bund2` executable that runs it
and writes the program's text, deflated, into a region reserved inside every
`bund2` binary; the copy finds the program in its own memory at start and runs
it. No Rust toolchain, C compiler or linker is needed. On macOS the copy is
re-signed with the system's `codesign`, because an edited binary will not run
there until it is. `bund2 build --inspect bund_repl` prints what the binary
carries.

If `cargo` is managed by `mise` on your machine, the Makefile's build of
`bund2` itself may need the toolchain override dropped:

    make CARGO="env -u RUSTUP_TOOLCHAIN cargo"

## Things to try

    [BUND> 1 2 3 4 5
    [BUND> stack
    [BUND> [ 1 2 3 ] { 10 * } map
    [BUND> :greet { "hello, " + println } register
    [BUND> "world" greet
    [BUND> dict :a 1 set :b 2 set
    [BUND> :a get

Bund's binary words take their right operand first, so `"world" "hello, " +`
is `"hello, " + "world"`; that is why `greet` needs no `swap`.
