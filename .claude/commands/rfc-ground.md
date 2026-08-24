Read every file under $ARGUMENTS in `reference/`. Do not draft an RFC.

Produce a grounding note at `docs/research/grounding/<area>.md` containing:

- what each file does, one line each, with a `path:line` anchor
- every word or method registered, with its observed stack arity where the
  source makes it determinable
- every place a runtime-mutable table is read or written
- every place a value's identity, timestamp, tags, or attributes are read
- anything contradicting a claim in `docs/research/` — cite both sides
- anything you could not determine from the source

Then append new decisions to `docs/registers/decisions.md` and new defects to
`docs/registers/defects.md`.
