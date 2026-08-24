Review RFC-$ARGUMENTS as an adversarial reader. Do not edit the RFC.

- Verify every `path:line` citation by opening the file. Report any that do not
  say what the RFC claims. This is the main job — re-read the cited lines, not
  the RFC.
- Find claims that need a citation and lack one.
- Check each acceptance criterion is actually checkable, and name what checks
  it. "Faster" is not checkable.
- Check the Preservation analysis section enumerates every behaviour the design
  could change, including ones the author may not have noticed.
- Check consistency with accepted RFCs and with ERRATA.
- List what the RFC assumes but does not state.

Write findings to `docs/rfc/reviews/RFC-<n>-review-<date>.md`.
