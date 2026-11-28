# BUND, FOUR WAYS — book bible (for the book's author, not compiled)

The third book of the harbour, after *Bund, Word by Word* (`books/Introduction`)
and *Bund, Problem by Problem* (`books/solving_problems`). One job done four
times: by commands, by functions, by objects and by rules. Each version prints
the same page and is put to the same two tests.

Read `books/Introduction/BIBLE.md` first. Wendhaven, the three ships and the
voice are the same.

## Title and cover

- **Title** — *Bund, Four Ways*
- **Subtitle** — *One Harbour Day by Commands, by Functions, by Objects and by Rules*
- **Cover eyebrow** — A THIRD BOOK OF THE HARBOUR
- **Version line** — examples run on `bund2 0.57.0`

## The job (fixed — the first two books' numbers do NOT apply here)

The council has changed the tariff: cargo pays by its kind. This is the one
departure from the first book, made because a job in which every cargo is
reckoned alike cannot show the paradigms apart.

| cargo | pennies a unit |
|---|---|
| salt | 3 |
| wax | 5 |
| herring | 2 |
| mooring | 20 a ship |

| ship | cargo | units | owes |
|---|---|---|---|
| *Marta* | salt | 40 | **140** |
| *Gull* | wax | 12 | **80** |
| *Osprey* | herring | 25 | **70** |

The day is **290**. Every program prints exactly:

    Wendhaven ledger
    ------------------------------------
    Marta: 40 of salt, 140 pennies
    Gull: 12 of wax, 80 pennies
    Osprey: 25 of herring, 70 pennies
    ------------------------------------
    Total: 290 pennies

**The two tests.** (1) The fourth cargo: the *Heron*, 8 bales of wool at 4, owes
**52**, and the day is **342**. (2) The new question: which ships owe more than
75? The *Marta* and the *Gull*.

## The programs, in `code/`

`commands.bund`, `functions.bund`, `objects.bund`, `rules.bund`, and
`mixed.bund` for Chapter 6. `code/wool/` holds the first four with the fourth
cargo added; the book shows `diff` between each pair.

The five must print the same bytes, and the four in `wool/` likewise. Check
after any change:

    cd code && for f in functions objects rules mixed; do
      cmp <(bund2 commands.bund) <(bund2 $f.bund); done

The counts the text quotes come from the recorded screen `c5-count`: the
fourth cargo costs 8, 2, 6 and 2 lines. If a program is edited, rebuild and
make the prose of Chapters 1 to 5 agree with that screen again.

## The through-line

*Where does each manner put the knowledge of the harbour?* In the order of the
steps; in a table; in the things themselves; in a list of what is so. The
book's last line: "a step, a value, a thing, or a fact?"

Do not say which is best. Each chapter ends with "what it buys and what it
costs", and Chapter 5 says when each earns its place.

## What the binary showed (0.57.0), and the book relies on

- An unknown cargo is charged as herring by `commands.bund` (the last branch
  of the chain), is `Key not found: wool` in `functions.bund`, is
  `OBJECT class Wool not registered` in `objects.bund`, and is silently absent
  from the ledger in `rules.bund`.
- After `commands.bund` runs, `:name var?` is `Osprey`: variables hold the last
  ship only.
- `set` on an object, and `kb.assert` on a knowledge base, leave a changed copy.
- A parent class's `.init` does not fill in a child's object, so each kind sets
  `.init { landed }`.
- An object carries its own class's slots; `!` finds the rest through `.super`.
- The book's terminal cannot print an object (`.s` fails on one), so every
  session line that calls a method ends `swap drop`.
- `curry` is a conditional: `:name curry :data [ … ] set :lambda { … } set !`.
- `lines` is a built-in word; a program that registers its own breaks.
- `"\n"` in a string is two characters, not a line end, and `format` rejects a
  backslash. `functions.bund` writes the line end as a string across two lines.
- `kb.word` takes the *name* of the variable holding the knowledge base.

## Sources

- The preface quotes `reference/Bund/README.md`: "rather than enforcing
  adherence to a single paradigm, I opted to provide developers with the
  flexibility to choose", and Chapter 2 rests on the same file's statement
  that Bund's functions are not pure.
- `curry` follows `reference/Bund/examples/code_snippets/curry_demo.bund`.
- Classes and objects follow `bund_examples/object_oriented/demo.bund`.
- Rules follow `bund_examples/logic_demo/demo.bund`.

## Screens — the rule

No screen is typed by hand. `tools/shoot.py` runs `sessions/` through the first
book's terminal, started in this book's `code/`, and writes `screens/`. A `.txt`
session usually begins with a hidden `"<program>.bund" study`; a `.sh` session
is shell commands run from `code/`.

## Plates

None yet. `ILLUSTRATIONS.txt` holds eight prompts: a frontispiece and one for
each chapter.
