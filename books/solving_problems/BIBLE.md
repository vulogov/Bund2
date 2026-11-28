# BUND, PROBLEM BY PROBLEM — book bible (for the book's author, not compiled)

The companion to *Bund, Word by Word* (`books/Introduction`). Same harbour, same
ships, same numbers, no new story: seventy-three problems, a hint for most, and
an answer for every one, recorded from the real binary.

Read `books/Introduction/BIBLE.md` first. Everything it fixes is fixed here too:
Wendhaven, the *Marta* (salt, 40), the *Gull* (wax, 12), the *Osprey* (herring,
25), 3 pennies a unit, 20 to moor, large above 30, and the check digits
140, 56, 95 and 291.

## Title and cover

- **Title** — *Bund, Problem by Problem*
- **Subtitle** — *Seventy-Three Problems for the Harbourmaster, Every Answer Run*
- **Cover eyebrow** — A SECOND BOOK OF THE HARBOUR
- **Version line** — examples run on `bund2 0.57.0`
- The subtitle and the opening of Chapter 0 and the afterword state the count.
  Change all three when a problem is added or removed.

## The shape

Three parts over one set of data.

1. **The problems**, in twelve chapters.
2. **The hints**: one nudge each, never the answer.
3. **The answers**: a recorded screen and a few lines on what to notice.

A chapter file under `chapters/` defines `title`, `intro` and `problems`, an
array of dictionaries (`id`, `title`, `grade`, `ask`, `hint`, `answer`, and
`file` for a whole program). `solving_problems.typ` lays the array out three
times. A problem's number is its place in the book, so inserting one renumbers
everything after it: search the chapters for "Problem" and "Problems" and mend
the cross-references (to `rem`, `over`, `pair`, `pad`), and Appendix A.

| ch. | title | problems | leans on |
|---|---|---|---|
| 1 | Crates | 1–6 | `dup drop swap`, the table |
| 2 | Pennies | 7–12 | arithmetic, `*+` |
| 3 | Words of Your Own | 13–18 | `register`, `alias`; defines `rem` (14) |
| 4 | Questions | 19–24 | comparisons, `if`, `ifthenelse`; `even?` (20) |
| 5 | Rounds | 25–30 | `times loop while for` |
| 6 | Text | 31–36 | `+ * len format`, `string.` words; `pad` (35) |
| 7 | Lists | 37–42 | `map loop sort unique`; new: `head`, `tail` |
| 8 | Records | 43–48 | `dict set get ?key`, `ship` |
| 9 | The Table and the Quays | 49–54 | `. take +. @name fold`; `over`, `pair` (49), `own`, `back` (52) |
| 10 | A Word That Calls Itself | 55–61 | recursion; `code/fib.bund` |
| 11 | Programs as Values | 62–67 | `! lambda= bund.eval ?word ?try`, pointers |
| 12 | Harbour Days | 68–73 | whole programs in `code/answers/` |

Chapter 10 is the one subject the first book did not teach. Its example is the
owner's naive recursive Fibonacci, `bund_examples/fibonacci/fibonacci.bund`,
copied as `code/fib.bund` without its command-line part.

## What a problem must be

- **Solvable with the first book's words.** The only additions are `head` and
  `tail` (Chapter 7), introduced where they are used.
- **Checkable without the answer.** The statement gives the result to expect.
- **Graded.** One crate: a line. Two: a word or two of your own. Three: a small
  design.
- **Short in the answer.** The terminal takes one line at a time and a screen is
  73 columns wide, so a definition is at most 65 characters. An answer that
  will not fit is split into smaller words. This is a rule of the book and it
  says so: "a word that will not fit is two words".
- **In the harbour.** Ships, cargo, pennies, the customs house. No other people.

## Things learned from the binary while writing (0.57.0)

- There is no remainder word. Problem 14 writes `rem`, and it is used after.
- `map` must get one value from every round, so it cannot filter. Selecting is
  done with `loop` on a stack of one's own, then `gather` (Problem 54).
- A name such as `2dup` is read as `2` then `dup`. The answers say `pair`.
- `push` nests its argument in a list and `at` is not zero-based; neither is
  used. One value becomes a list by `fold` on an empty stack (Problem 52).
- `?key` uses up the key; `get` uses up the dictionary.
- An empty lambda `{ }` and an empty list `[ ]` are parse errors; `list` is the
  empty list.
- Strings cannot be compared with `<` or `>`; `sort` orders them.

## Screens — the rule

No screen is typed by hand. `tools/shoot.py` runs each file in `sessions/`
through the first book's terminal (`books/Introduction/code/terminal.bund`),
started in this book's `code/` with `"harbour.bund" study` already typed, and
writes `screens/`. A session is named for its problem: `c05p4` is the fourth
problem of Chapter 5. `.sh` sessions are shell commands run from `code/`.

`code/harbour.bund` holds the first book's words that this one assumes. It
leaves out `total`, `line` and `manifest`, which problems ask the reader to
write.

## Not yet done

- No plates. The first book's engravings are not reused, and no prompts have
  been written for this one.
