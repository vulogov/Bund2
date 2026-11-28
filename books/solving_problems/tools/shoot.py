#!/usr/bin/env python3
"""Turn the book's session files into the screens its chapters print.

    python3 books/solving_problems/tools/shoot.py          # every session
    python3 books/solving_problems/tools/shoot.py c03p2    # the ones named

No screen in the book is typed by hand. A file in `sessions/` is what a reader
types; this runs it through the real `bund2` and writes what came back into
`screens/`, which is what a chapter's `#session(...)` prints.

Two kinds of session, told apart by the extension:

  NAME.txt   lines typed at the terminal of "Bund, Word by Word"
             (`books/Introduction/code/terminal.bund`), started in this
             book's `code/` with `"harbour.bund" study` already typed, as
             Chapter 0 asks of the reader. The transcript shows each line
             after the prompt and, under it, what that line printed.

  NAME.sh    one shell command per line, run from `code/`, shown after `$`.
             `#!rm FILE` names a file the session makes, removed afterwards.

A piped terminal does not echo what it reads, so a line's output is found by
running the session once per line: the first k lines, then the first k-1, and
what the longer run added is what line k printed. A session is a dozen lines,
so that is a dozen short runs.

In a `.txt` session a line starting `#!` is a directive and is not typed:

  #!hide     run the next line but leave it, and its output, off the screen
  #!setup    the same for every line up to `#!show`: the words an earlier
             problem already defined

The binary is `target/release/bund2` under the repository root, or `$BUND2`.
The one change made to what it prints: the directory the repository happens to
be in is shown as `/home/you/Bund2`, so that the book does not carry the path
of the machine it was built on.
"""

import os
import re
import subprocess
import sys
from pathlib import Path

BOOK = Path(__file__).resolve().parent.parent
ROOT = BOOK.parent.parent
CODE = BOOK / "code"
SESSIONS = BOOK / "sessions"
SCREENS = BOOK / "screens"
BUND2 = os.environ.get("BUND2", str(ROOT / "target" / "release" / "bund2"))
TERMINAL = str(BOOK.parent / "Introduction" / "code" / "terminal.bund")
STUDY = '"harbour.bund" study'
PROMPT = "[BUND > "
WIDTH = 73
HOME = "/home/you/Bund2"
ANSI = re.compile(r"\x1b\[[0-9;]*m")
ENV = dict(os.environ, NO_COLOR="1", BUND2_NOCOLOR="1")


def clean(text):
    """Colour codes out, and the place this was built shown as the reader's."""
    return ANSI.sub("", text).replace(str(ROOT), HOME)


def run_terminal(lines):
    """Everything the terminal prints for these lines, banner and farewell cut."""
    out = subprocess.run(
        [BUND2, TERMINAL],
        input="\n".join(lines) + "\n",
        capture_output=True,
        text=True,
        cwd=CODE,
        env=ENV,
        timeout=60,
    )
    text = clean(out.stdout + out.stderr).split("\n")
    # The banner ends at the greeting line; the farewell is the last two lines.
    for i, row in enumerate(text):
        if row.startswith("Bund, Word by Word:"):  # the greeting
            text = text[i + 1 :]
            break
    while text and text[-1] == "":
        text.pop()
    if text and text[-1] == "bye":
        text.pop()
    if text and text[-1] == "":
        text.pop()
    return text


def shoot_terminal(path):
    typed, hidden, hide_next, setup = [STUDY], {0}, False, False
    for raw in path.read_text().split("\n"):
        if raw.strip() == "#!hide":
            hide_next = True
            continue
        if raw.strip() in ("#!setup", "#!show"):
            setup = raw.strip() == "#!setup"
            continue
        if raw.strip() == "":
            continue
        if hide_next or setup:
            hidden.add(len(typed))
            hide_next = False
        typed.append(raw)

    screen, before = [], []
    for k, line in enumerate(typed):
        after = run_terminal(typed[: k + 1])
        if after[: len(before)] != before:
            raise SystemExit(f"{path.name}: line {k} changed earlier output; "
                             "a session must print the same thing every run")
        added = after[len(before) :]
        before = after
        if k in hidden:
            continue
        screen.append(PROMPT + line)
        screen.extend(added)
    return screen


def shoot_shell(path):
    screen, litter = [], []
    for line in path.read_text().split("\n"):
        if line.strip() == "":
            continue
        if line.startswith("#!rm "):      # a file the session makes; removed after
            litter.append(CODE / line[5:].strip())
            continue
        out = subprocess.run(
            line.replace("bund2", BUND2, 1) if line.startswith("bund2") else line,
            shell=True,
            capture_output=True,
            text=True,
            cwd=CODE,
            env=ENV,
            timeout=120,
        )
        screen.append("$ " + line)
        body = clean(out.stdout + out.stderr).rstrip("\n")
        if body:
            screen.extend(body.split("\n"))
    for path in litter:
        path.unlink(missing_ok=True)
    return screen


def main(names):
    SCREENS.mkdir(exist_ok=True)
    wanted = sorted(SESSIONS.glob("*.txt")) + sorted(SESSIONS.glob("*.sh"))
    if names:
        wanted = [p for p in wanted if p.stem in names]
    for path in wanted:
        screen = shoot_terminal(path) if path.suffix == ".txt" else shoot_shell(path)
        rows = []
        for row in screen:
            row = row.rstrip()
            while len(row) > WIDTH:      # a screen is WIDTH columns wide
                rows.append(row[:WIDTH])
                row = "    " + row[WIDTH:]
            rows.append(row)
        (SCREENS / (path.stem + ".txt")).write_text("\n".join(rows) + "\n")
        print(f"{path.stem}: {len(rows)} lines")


if __name__ == "__main__":
    main(set(sys.argv[1:]))
