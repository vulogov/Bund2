Implement work item $ARGUMENTS.

Read the work item, the RFC section it cites, and the pinned reference source
it names. Then implement, with tests.

Rules:
- Behaviour listed under Preserve must match the reference exactly.
- Behaviour listed under Deviate is approved; anything else that differs is not.
  If you find yourself needing an unlisted deviation, stop and say so.
- Run `cargo xtask conform` before you finish. State the before and after
  numbers. No previously-green golden may regress.
- Never edit `tests/golden/`.
