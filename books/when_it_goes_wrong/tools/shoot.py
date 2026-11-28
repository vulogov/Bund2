#!/usr/bin/env python3
"""Turn the book's session files into the screens its chapters print.

    python3 books/when_it_goes_wrong/tools/shoot.py          # every session
    python3 books/when_it_goes_wrong/tools/shoot.py c3-ask   # the ones named

No screen in the book is typed by hand. A file in `sessions/` is what a reader
types; this runs it through the real `bund2` and writes what came back into
`screens/`, which is what a chapter's `#session(...)` prints.

Two kinds of session, told apart by the extension:

  NAME.txt   lines typed at the terminal of "Bund, Word by Word"
             (`books/Introduction/code/terminal.bund`), started in this
             book's `code/`. The transcript shows each line after the
             prompt and, under it, what that line printed.

  NAME.sh    one shell command per line, run from `code/`, shown after `$`,
             with what it wrote to standard output and standard error in
             the order it wrote them.
             `#!rm FILE` names a file the session makes, removed afterwards.

  NAME.tty   a program that stops and takes lines: the debugger's console,
             `debug.shell`, a script that reads. The first line is the
             command, shown after `$`; each line after it is typed when the
             program has gone quiet. It is run on a pseudo-terminal 73
             columns wide, so the prompts and the echo are the program's
             own, as a reader at a terminal sees them. A last Ctrl-D is
             sent if the program is still waiting. `#!rm FILE` as above.

A piped terminal does not echo what it reads, so a line's output is found by
running the session once per line: the first k lines, then the first k-1, and
what the longer run added is what line k printed. A session is a dozen lines,
so that is a dozen short runs.

In a `.txt` session a line starting `#!` is a directive and is not typed:

  #!hide     run the next line but leave it, and its output, off the screen
  #!setup    the same for every line up to `#!show`: usually the `study`
             of the program the screen then asks questions of

The binary is `target/release/bund2` under the repository root, or `$BUND2`.
Two changes are made to what it prints. The directory the repository happens
to be in is shown as `/home/you/Bund2`, so that the book does not carry the
path of the machine it was built on. And the line the debugger prints when it
first stops, which lists every command and is 330 characters long, is cut to
its first words and an ellipsis, except in a session that says `#!banner`:
Chapter 6 prints it whole, once.
"""

import os
import pty
import re
import select
import struct
import fcntl
import termios
import tempfile
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
PROMPT = "[BUND > "
WIDTH = 73
HOME = "/home/you/Bund2"
ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")
BANNER = "bund2: stopped. s(tep), n(ext), f(inish), c(ontinue), bt, st, info, "
ENV = dict(os.environ, NO_COLOR="1", BUND2_NOCOLOR="1",
           PATH=str(Path(BUND2).parent) + os.pathsep + os.environ.get("PATH", ""))


def clean(text):
    """Colour codes out, and the place this was built shown as the reader's."""
    return ANSI.sub("", text).replace(str(ROOT), HOME)


def run_terminal(lines):
    """Everything the terminal prints for these lines, banner and farewell cut."""
    out = subprocess.run(
        [BUND2, TERMINAL],
        input="\n".join(lines) + "\n",
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        cwd=CODE,
        env=ENV,
        timeout=60,
    )
    text = clean(out.stdout).split("\n")
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
    typed, hidden, hide_next, setup = [], set(), False, False
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
            raise SystemExit(f"{path.name}: line {k + 1} changed earlier output; "
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
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            cwd=CODE,
            env=ENV,
            timeout=120,
        )
        screen.append("$ " + line)
        body = clean(out.stdout).rstrip("\n")
        if body:
            screen.extend(body.split("\n"))
    for path in litter:
        path.unlink(missing_ok=True)
    return screen


def shoot_tty(path):
    lines = [l for l in path.read_text().split("\n") if l.strip() != ""]
    litter = [CODE / l[5:].strip() for l in lines if l.startswith("#!rm ")]
    banner = any(l.strip() == "#!banner" for l in lines)
    lines = [l for l in lines if not l.startswith("#!")]
    command, typed = lines[0], lines[1:]
    home = tempfile.mkdtemp(prefix="bund-home-")
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(CODE)
        os.environ.update(ENV, TERM="dumb", HOME=home)
        argv = command.split()
        if argv[0] == "bund2":
            argv[0] = BUND2
        os.execvp(argv[0], argv)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 50, WIDTH, 0, 0))

    def until_quiet(wait=0.5):
        got, ended = b"", False
        while True:
            ready, _, _ = select.select([fd], [], [], wait)
            if not ready:
                break
            try:
                data = os.read(fd, 65536)
            except OSError:
                ended = True
                break
            if not data:
                ended = True
                break
            got += data
        return got, ended

    out, ended = until_quiet(1.0)
    for line in typed:
        if ended:
            raise SystemExit(f"{path.name}: the program ended before `{line}` was typed")
        os.write(fd, (line + "\n").encode())
        more, ended = until_quiet()
        out += more
    if not ended:
        os.write(fd, b"\x04")
        more, ended = until_quiet()
        out += more.replace(b"^D\x08\x08", b"")
    try:
        os.close(fd)
        os.waitpid(pid, 0)
    except OSError:
        pass
    for extra in litter:
        extra.unlink(missing_ok=True)
    text = clean(out.decode("utf-8", "replace")).replace("\r\n", "\n").replace("\r", "")
    rows = text.rstrip("\n").split("\n")
    if not banner:
        rows = [BANNER + "…" if r.startswith(BANNER) else r for r in rows]
    return ["$ " + command] + rows


def main(names):
    SCREENS.mkdir(exist_ok=True)
    wanted = (sorted(SESSIONS.glob("*.txt")) + sorted(SESSIONS.glob("*.sh"))
              + sorted(SESSIONS.glob("*.tty")))
    if names:
        wanted = [p for p in wanted if p.stem in names]
    for path in wanted:
        screen = {".txt": shoot_terminal, ".sh": shoot_shell, ".tty": shoot_tty}[path.suffix](path)
        rows = []
        for row in screen:
            row = row.rstrip()
            while len(row) > WIDTH:      # a screen is WIDTH columns wide
                cut = row.rfind(" ", 40, WIDTH + 1)
                cut = WIDTH if cut < 0 else cut
                rows.append(row[:cut])
                row = "    " + row[cut:].lstrip()
            rows.append(row)
        (SCREENS / (path.stem + ".txt")).write_text("\n".join(rows) + "\n")
        print(f"{path.stem}: {len(rows)} lines")


if __name__ == "__main__":
    main(set(sys.argv[1:]))
