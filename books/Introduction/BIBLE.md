# THE HARBOUR LEDGER — book bible (for the book's author, not compiled)

This is the shared reference for *Bund, Word by Word*. Every chapter teaches a
part of the Bund language by using it on ONE running example, so the place, the
cast, the numbers and the vocabulary must stay the same from chapter to chapter.
Keep the example small. It is a scaffold for the language, not a simulation of a
port. Use only what a chapter needs.

## Logline

A small harbour town keeps a ledger. Ships tie up, their cargo is stacked on the
quay, the harbourmaster's clerk counts it and reckons the dues. The reader is the
harbourmaster; Bund is the clerk, who knows more than a thousand words on the first
morning and learns a new one every time the reader teaches it.

## The book, in one line

> *You put things down on a stack, you say a word, and the word picks them up —
> top one first.*

## Title and cover

- **Title** — *Bund, Word by Word*
- **Subtitle** — *A First Book of a Concatenative Language with Many Stacks*
- **Cover eyebrow** — A HARBOUR, A LEDGER, A STACK
- **Version line** — examples run on `bund2 0.57.0`
- **Author** — Vladimir Ulogov
- **Text face** — Kyiv Machine, read from `assets/Kyiv_Machine.ttf`. One weight,
  no italic: emphasis is underlined, strong is struck twice. It draws `{ } [ ]`
  as parentheses and `*` as a bullet, so those characters are withheld from it
  (`body_family` in `design.typ`) and code is always set in DejaVu Sans Mono.

## The through-line (repeat it, the way the plakat book repeats "stay in prose")

**The top speaks first.** Every Bund word takes its *first* operand from the top
of the stack. This one rule explains everything a newcomer finds backwards:

| you type | you get | because |
|---|---|---|
| `10 3 -` | `-7` | the top, 3, is the first operand: 3 − 10 |
| `3 10 -` | `7` | 10 − 3 |
| `4 20 /` | `5` | 20 ÷ 4 |
| `1 2 <` | `false` | it asks 2 < 1 |
| `" world" "Hello" +` | `Hello world` | the top string comes first |
| `c { else } { then } ifthenelse` | | the lambda on top is *then* |
| `dict :name "Marta" set` | | value on top, key under it, receiver under that |

Never apologise for this rule and never call it a quirk. It is the rule, it is
uniform, and the book's job is to make it second nature by Chapter 3. The
harbour image for it: *the last crate set down is the first crate lifted, and
the clerk reads the label of the crate in his hands first.*

Bund is not Forth. Do not teach "move the operator to the end". Where a reader
who knows Forth or an HP calculator will be surprised, say so in a
`For Forth hands` callout and move on.

## The setting

- **Wendhaven** — an invented harbour town of no particular country, late
  sixteenth century in mood: cogs and hulks, a stone quay, a customs house with
  a counting table. Invented on purpose; no real port, league or person.
- **The quay** — where cargo is stacked. *The* picture of the stack: crates go
  down one on another, and only the top one can be lifted.
- **The counting table** — the clerk's bench in the customs house. The picture
  of the **workbench**: a place to set one thing aside while both hands are busy.
- **The other quays** — the salt quay, the fish quay, the wax quay. The picture
  of **named stacks**: several piles, one clerk, and he stands at one at a time.
- **The ledger** — the bound book the day's reckoning is written into. The
  picture of the finished program.

## The cast

- **The harbourmaster** — the reader. Always "you". Gives orders, teaches words.
- **The clerk** — Bund itself. Literal-minded, tireless, never guesses. Used
  sparingly: he is a way to say "the interpreter", not a character with a story.
  No name, no dialogue beyond what the screen shows.
- No other people. Ships have names; their masters do not appear.

## The ships and their cargo (fixed — do not vary between chapters)

| ship | kind | cargo | units |
|---|---|---|---|
| *Marta* | cog | salt | 40 crates |
| *Gull* | hulk | wax | 12 crates |
| *Osprey* | cog | herring | 25 barrels |

Atoms and keys, exactly as spelled in code: `:name`, `:cargo`, `:units`.
Ship names in code are strings: `"Marta"`, `"Gull"`, `"Osprey"`.

## The money (fixed)

- Everything is reckoned in whole **pennies**. No fractions of a penny, ever:
  the ledger is integer arithmetic on purpose, and Chapter 3 says why.
- **Quay due** — 3 pennies per unit of cargo.
- **Mooring fee** — 20 pennies per ship, flat.
- **Large cargo** — more than 30 units. A large cargo is flagged in the ledger.
- So: *Marta* 40 × 3 + 20 = **140**; *Gull* 12 × 3 + 20 = **56**;
  *Osprey* 25 × 3 + 20 = **95**; the day's total is **291**.

These four numbers — 140, 56, 95, 291 — are the book's check digits. Any screen
that reckons dues must arrive at them.

## The words the reader builds (the vocabulary grows; names are fixed)

| word | effect | first defined | meaning |
|---|---|---|---|
| `due` | `( units -- pennies )` | Ch. 4 | units × 3 |
| `fee` | `( pennies -- pennies )` | Ch. 4 | adds the 20-penny mooring fee |
| `dues` | `( units -- pennies )` | Ch. 4 | `due fee` — the word made of words |
| `large?` | `( units -- flag )` | Ch. 5 | more than 30 units |
| `size` | `( units -- text )` | Ch. 5 | `"large"` or `"small"` |
| `note` | `( units -- )` | Ch. 5 | `size println` |
| `rule` | `( -- )` | Ch. 7 | a rule of 36 dashes |
| `heading` | `( -- )`, `( day -- )` from Ch. 12 | Ch. 7 | the ledger's heading |
| `wording` | `( -- template )` | Ch. 7 | how a ledger line reads |
| `total` | `( list -- sum )` | Ch. 8 | adds a list up |
| `units=` `cargo=` `name=` | `( value record -- record )` | Ch. 9 | write one field |
| `ship` | `( name cargo units -- record )` | Ch. 9 | builds one ship's record |
| `arrivals` | `( -- r r r )` | Ch. 9 | the three ships |
| `gather` | `( ... -- list )` | Ch. 9 | the whole stack as a list, in order |
| `brief` | `( record -- )` | Ch. 9 | name and due |
| `peek` | `( -- x )` | Ch. 10 | a copy of the workbench's top |
| `has` | `( key -- value )` | Ch. 10 | a field of the record on the workbench |
| `fields` | `( -- due cargo units name )` | Ch. 10 | what the wording asks for |
| `line` | `( record -- )` | Ch. 10 | prints one ledger line |
| `manifest` | `( -- list )` | Ch. 11 | arrivals gathered on their own stack |
| `berth` | `( record -- )` | Ch. 11 | to the stack `wide` or `narrow` |
| `reckoned` `unreckoned` `entry` | `( record -- )` | Ch. 12 | the guarded ledger entry |

The files, in `code/`: `words.bund` (Ch. 4–6), `records.bund` (Ch. 9),
`table.bund` (Ch. 10), `quays.bund` (Ch. 11), `ledger.bund` (Ch. 12, the whole
program), `terminal.bund` (the book's terminal, read in Ch. 13).

A word once defined keeps its name and its stack effect for the rest of the book.

## The escalation (so chapters build on each other)

1. **Ch. 1** — proof of life: the clerk says the harbour's name. Strings, `println`,
   comments, the first mistake.
2. **Ch. 2** — crates on the quay: numbers go down, `println` lifts them off,
   `dup` `drop` `swap`, `.s`, running out of crates.
3. **Ch. 3** — reckoning one ship by hand: `+ - * /`, the top speaks first,
   whole pennies, 40 × 3 + 20 = 140.
4. **Ch. 4** — the same reckoning as words: `{ }`, `!`, `register`, `due`, `fee`,
   `dues`, stack-effect notation.
5. **Ch. 5** — is the cargo large? `true`/`false`, comparisons, `if`,
   `ifthenelse`, `large?`, `note`.
6. **Ch. 6** — three ships, one after another: `times`, `loop`, `while`, `for`.
7. **Ch. 7** — the ledger line as text: strings, atoms, joining, `format`,
   the `string.` words.
8. **Ch. 8** — the day's cargo as a list: `[ ]`, `len`, `car`, `cdr`, `loop`,
   `map`, `sort`, summing to 291.
9. **Ch. 9** — a ship as a record: `dict`, `set`, `get`, `?key`; a list of
   records; `ship` and `brief`. `line` waits for the workbench in Ch. 10.
10. **Ch. 10** — the counting table: `.` and `take`, the dotted words.
11. **Ch. 11** — the other quays: `@name`, `current`, `move`, rotating.
12. **Ch. 12** — the ledger as a program in a file: a script, its arguments,
    a mistake caught with `?try`, `bund2 check`, a standalone binary.
13. **Ch. 13** — under the hood: code is data, pointers, `bund.eval`, and the
    terminal the reader has been typing into all along, read line by line.
14. **Ch. 14** — the stepping stones: where the ideas came from. No new words,
    no problems. Postfix (Łukasiewicz), Forth, PostScript, Joy and Factor,
    Lisp; then what is Bund's own (the ring of stacks, the workbench) and the
    paradigms the book leaves for later. An influence is stated as one only
    where the author's own booklet states it ("Basics of BUND language in 10
    minutes": inspired by Forth; metaprogramming from Factor; code-as-data from
    Lisp; byte code from Python and Erlang). Everything else is called kin.
    The chapter is in the author's first person. Its personal section, "My
    first stone: a calculator", rests on what the author said (he used a
    Soviet programmable RPN calculator, an Elektronika MK-56, for school and games) and on two
    sources for the history: Wikipedia, "Elektronika B3-34", and K.
    Tatarchenko, IEEE Spectrum, 27 Sep 2018 (the Tekhnika — Molodyozhi
    serial "Kon-Tiki: The Way to Earth", M. Pukhov, from August 1985).

## The terminal

`bund2` has no built-in interactive mode. The book ships its own, written in
Bund: `books/Introduction/code/terminal.bund`, a short descendant of
`bund_examples/bund_repl/repl.bund`. It differs in three ways, each of them for
the beginner:

- an error is one short line, not the evaluator's whole nested trace;
- `.s` prints the entire stack on one line, bottom to top, and leaves it alone;
- after each line it shows the top of the stack as `=> value`;
- `"file.bund" study` loads a file of words from the folder it was started in.

The reader starts it in Chapter 1 and is not asked to understand it until
Chapter 13, where it is the last program in the book. That is the book's quiet
arc: *the thing you were typing into was sixty lines of the language you were
learning.*

## Screens — the rule

**No screen is typed by hand.** Each `#session(...)` in a chapter names a file
in `sessions/`; `tools/shoot.py` feeds it to the real `bund2` and writes the
transcript into `screens/`, which is what the chapter prints. If a screen and
the binary disagree, the binary is right and the prose changes.

Screens use the plakat book's method: a monospace `screen()` mockup with a
caption bar, never a screenshot. What the reader types follows the prompt
`[BUND>`; what Bund answers is shown plainly.

## Chapter furniture (the same in every chapter)

- a drop-cap opening that starts from something the harbourmaster needs;
- `#section` / `#subsection`; `#session` screens with captions;
- `#term` for each new idea, once, where it first matters;
- `#words` — a table of the words the section introduced, with stack effects;
- `#warn` for the traps (nearly always: *who speaks first*);
- `#callout` for asides, labelled by reader: `For Forth hands`, `For the curious`;
- a closing paragraph that hands off to the next chapter;
- `#recap` — "What you learned";
- `#problems` — three to five exercises. Answers are in Appendix C, and every
  answer is a session that was run.

## Voice

First person where the author speaks, "you" for the reader, short declarative
sentences, a concrete need before every feature. Dry humour is welcome; jokes
at the reader's expense are not. Never "simply", never "just". Say what a word
does before saying what it is called.

## What the book does not cover

Objects and classes, logic programming, graphs, statistics and data frames,
SMYSL, threads and the bus, the JIT. Each is named once, in the afterword, as a
place to go next. A first book that mentions everything teaches nothing.

## Illustrations

Fourteen engravings (thirteen plates and the MK-56 of Chapter 14) in the manner of the author's portrait — black line on white,
cross-hatched, sixteenth-century — listed with their prompts in
`ILLUSTRATIONS.txt`. They show the *harbour*, never the computer: the pictures
carry the metaphor and the screens carry the facts. Until the files exist the
book prints a framed placeholder in each one's place; `have_art` in `design.typ`
switches them on.
