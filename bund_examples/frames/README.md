# frames

RFC-0017's frames, factors and missing values, over a small dose trial:
nine patients at three sites, two with no response recorded.

    make            # run demo.bund
    make check      # run it and compare the output with expected.txt

or, without `make`, from this directory:

    bund2 demo.bund

Nothing is drawn at random, so a run prints the same lines each time. The
model's numbers are rounded to six decimals before they are printed.

## The output

`expected.txt` is what a run prints. A test in `crates/bund2-cli/tests`,
`frames_demo.rs`, runs the demo and compares the two. It is not a golden:
the reference has no frame words, so there is no oracle to capture one
from.

| | What it shows | Words | It prints |
|---|---|---|---|
| 1 | CSV text read into a frame, `""` and `NA` read as NONE, `site` made a factor | `frame.csv.parse`, `frame.text`, `frame.names`, `frame.nrow`, `frame.column`, `na.count`, `na.omit` | the table, 9 rows, 2 missing, mean 5.528571 |
| 2 | Complete rows, a filter by a lambda over each row, a stable sort with NONE last | `frame.complete`, `frame.filter`, `frame.sort` | 7 rows, 6 rows, the sorted table |
| 3 | One row per site under `"omit"`, with `.dropped`; the same under `"refuse"` | `frame.aggregate` | east, north, south in level order; the refusal naming row 4 |
| 4 | An inner and a left join on `site` | `frame.join` | 6 rows; 9 rows, east's town NONE |
| 5 | A MATRIX for a linear model, and how many rows `"omit"` dropped | `frame.matrix`, `stat.lm` | 2 dropped; intercept 2.917647, slope 0.140588, r² 0.92006 |
| 6 | The first three rows written as CSV | `frame.head`, `frame.csv.text` | NONE empty, every STRING quoted |
| 7 | Where NONE comes from, and where it cannot | `na`, `na.is`, `na.fill`, `frame.new`, `frame.with` | the literal refused; `[ 1 :: 3 :: NONE :: ]` |

## Three idioms

- **A frame is a plain MAP.** `columns`, `data` and `levels`; every
  `frame.` word checks that shape on entry and refuses a malformed one
  naming what is wrong. It travels through `encode.base64`, but not
  through `json.from_value`, which refuses its STRINGs as the reference
  does.
- **NONE in a LIST literal is not NONE.** Inside `[ ]`, `na` stays a word,
  as `true` and `false` do, so `[ 1 na ]` holds a CALL. Build the LIST
  with `+`, or read it from CSV or JSON.
- **`file` takes an absolute path.** It makes a `file://` URI of its
  operand, so a relative name fetches nothing. The demo holds its CSV as
  a STRING so it runs from anywhere.
