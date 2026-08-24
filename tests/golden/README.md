# Golden outputs

Expected final state for the hermetic example corpus, captured from the
reference implementation in `reference/Bund`.

Of the 120 example programs and 12 test programs in the reference repository,
those that touch no network, filesystem, image, database, or bus form the
hermetic conformance suite. The remainder are manual smoke tests.

Each golden records the full final state, not stdout: every named stack's
contents, the workbench, the exit status, the error text, and stdout. A
concatenative program's meaning is what it leaves on the stacks.

## These files are sacred

When Bund2 disagrees with a golden, **the golden is not the thing that
changes**. A failure has exactly three dispositions:

1. **Bund2 bug** — fix Bund2. This is the common case.
2. **Reference bug** — record it in `docs/registers/defects.md` with a
   disposition, then regenerate that one golden:
   `cargo xtask golden --accept <name> --reason F<n>`
   and note it in `EXCEPTIONS.md`.
3. **Approved deviation** — already listed in the work item's Deviate section.
   If it is not listed, stop: an unplanned deviation is a decision, and it goes
   to `docs/registers/decisions.md` before any code changes.

Quietly regenerating a golden because it is easier is not a fourth option. CI
treats this directory as read-only; regeneration goes through `--accept` with a
reason.

Regenerate everything only when a reference submodule SHA changes, and review
the diff as carefully as you would review code.
