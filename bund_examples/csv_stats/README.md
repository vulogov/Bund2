# Statistics over a CSV column

`stats.bund` reads one column of a CSV file and prints its count, minimum,
maximum and arithmetic mean. It is the smallest useful program for the four
statistics words, and it shows how a CSV file reaches a Bund program.

    $ bund2 stats.bund
    file    readings.csv
    column  temperature
    count   10
    min     16.4
    max     23.3
    mean    19.68

    $ bund2 stats.bund --column humidity
    file    readings.csv
    column  humidity
    count   10
    min     47.0
    max     72.0
    mean    58.9

`--file` names another CSV and `--column` another column; the defaults are
`readings.csv` and `temperature`. The file is opened relative to the current
directory when the program runs.

## How it works

**`csv` is a conditional.** `"readings.csv" csv` checks that the file exists
and pushes a value naming it. The program `set`s a `column` and a `lambda` on
that value and runs it with `!`. With a column named, the lambda runs once,
with the column's cells as one LIST on the stack, typed by inference: a column
of numbers arrives as numbers, so no conversion is written in the program.
Without a column the lambda runs once per row with the row as a list.

**Four words, one list.** Each of the four takes a series and answers a
number: `stat.count` an INTEGER, `math.min`, `math.max` and
`stat.mean.arithmetic` a FLOAT, which is why the `day` column of integers
reports `min 1.0`. The `,` forms keep the list they read and leave the answer
above it, so one list serves the first three questions and the plain form of
the last word consumes it:

```bund
:stats.report {
  "count   " print stat.count, println
  "min     " print math.min, println
  "max     " print math.max, println
  "mean    " print stat.mean.arithmetic println
} register
```

**The series can also come from the stack.** With no LIST on top, these words
pull every value down to the bottom of the stack, so `16.4 23.3 19.9 math.max`
answers `23.3` and empties the stack. That form has no `,`: there is nothing
left to keep the values on. The LIST form is the one to reach for when other
values are around.

**An empty column** answers `0` for the count, `NaN` for the extremes, and an
error for the mean, which is the behaviour of the original implementation and
is preserved.

## Running it without building

    cd bund_examples/csv_stats
    bund2 stats.bund
    bund2 stats.bund --column humidity

## Building a standalone binary

    cd bund_examples/csv_stats
    make                  # ./stats
    make run              # ./stats --file readings.csv --column temperature
    make run COLUMN=day
    make clean

The CSV file is not inside the binary. `csv` opens it when the program runs,
so `./stats --file /some/other.csv` works anywhere the file is readable. If
`cargo` is managed by `mise` on your machine, the Makefile's build of `bund2`
itself may need the toolchain override dropped:

    make CARGO="env -u RUSTUP_TOOLCHAIN cargo"
