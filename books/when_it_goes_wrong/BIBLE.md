# Bund, When It Goes Wrong — the bible

The fourth book of the harbour. Read this before changing a program, a
session or a chapter.

## What the book is

Three kinds of trouble — a program that stops, one that is wrong, one that
is slow — and every means `bund2` 0.57.0 has of finding out why. It teaches
no new way of writing Bund. Every program uses the first book's words.

## The programs (`code/`)

All use the first book's tariff: 3 pennies a unit, 20 mooring; Marta 40,
Gull 12, Osprey 25; 140, 56, 95; total 291.

| file | fault | symptom | cure (`code/mended/`) |
|---|---|---|---|
| `slip.bund` | the first book's `broken.bund`: `dues` with no operand on line 11 | stops: `Stack is too shallow for inline MUL()` | none; it is the specimen |
| `tare.bund` | `:net { 2 - }` is 2 − units | -94, -10, -49, total -153; should be 134, 50, 89, 273 | `2 swap -` |
| `leak.bund` | `dup dup 30 <` in `warned` | `40: 140 pennies`; total right | one `dup` |
| `clash.bund` | `line` defined twice | four rules, no ships; total right | second word renamed `rule` |
| `ledger.bund` | none; the first book's file, byte for byte | `dues` called 6 times for 3 ships | not mended; the chapter says how |

`ledger.bund` must stay identical to `books/Introduction/code/ledger.bund`:
the screens quote its line numbers.

The rest are specimens of one feature each: `log`, `look`, `views`,
`where`, `mem` (Chapters 2 and 4); `stopped`, `armed`, `stepped`, `shell`,
`asks`, `old` (Chapter 9); `snap` (Chapter 10).

## Sessions

Three kinds, told apart by extension; `tools/shoot.py` documents them.

- `.txt` — typed at the first book's terminal.
- `.sh` — shell commands, standard output and error in the order written.
- `.tty` — a program that stops and takes lines, run on a pseudo-terminal
  73 columns wide. Every debugger screen is one of these, so the prompts and
  the echo are the binary's.

Two changes are made to what the binary prints: the build folder is shown
as `/home/you/Bund2`, and the debugger's 330-character first line is cut
short except where a session says `#!banner` (Chapter 6, once).

`debug.display_hostinfo` is described and never recorded: it prints the
name of the machine the book was built on.

## What the binary showed (0.57.0, default features)

- At a terminal a failure is drawn with a traceback; in a pipe it is the
  table. `--report` chooses. A failing script exits 0.
- `log.error` is shown by default, `log.warning` and `log.info` from `-v`,
  `log.debug` from `-vv`, `log.trace` from `-vvv`; `-q` silences all.
- `check` passes `tare`, `leak` and `clash`: it counts values only.
- `?effect` answers `NODATA` for a word the program defined.
- A breakpoint on a word stops before the word runs; the frame line names
  the caller. `break N` stops inside the word.
- `until WORD` runs at once; it needs no `c`.
- A watch stops while the value is being pushed, so `st` and `changed:`
  show the stack a moment before.
- A condition that fails does not stop; it says so once per reason.
- `catch` stops with the frames intact and cannot rescue the run.
- A console opened by `debug.step` detaches on end of input.
- `debug.break.if` and `debug.watch.if` take the condition as a string.
- `debug.feed` typed at the first book's terminal is read by the terminal's
  own `input*`; it is shown in a script.
- Profile times vary run to run; calls and steps are exact. This build has
  no tier, so `--stats` has no screen.

## Sources

`docs/rfc/RFC-0008-debugger-and-observability.md` (the debugger, its words,
the console, D113, D115, D191), `docs/rfc/RFC-0011-profiler.md`,
`bund_examples/profiler_demo/README.md`, and `bund2 help script`. Claims
about built programs (the stopping words do nothing in a bundle) and about
Ctrl-C detaching are from RFC-0008 and were not recorded.

## Rules carried from the other books

- No long program as the main matter of a chapter: pieces with `#piece`,
  wholes in Appendix B.
- No source line wider than 73 characters.
- No screen typed by hand.
- The owner draws the plates; `ILLUSTRATIONS.txt` has the prompts.
