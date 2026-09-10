# Golden exceptions

Goldens deliberately regenerated because the reference implementation has a
defect Bund2 does not reproduce.

| Golden | Defect | Date | Reason |
|--------|--------|------|--------|
| `probes/eq-asymmetry.golden` | — | — | F33 |
| `probes/named-stacks.golden` | — | — | Q21 — the probe was rewritten once move/move_from/to_current were settled against the reference; the narrow version pinned only the part that agreed |
| `probes/stack-navigation.golden` | — | — | the probe gained an ensure_stack_with_capacity section once the operand order and the capacity bound were settled against the oracle |
| `probes/remaining-vocabulary.golden` | — | — | the probe gained var-, ?move and fold_stack once their shapes were settled against the oracle; fold_stack's target was corrected in the same pass |
| `probes/workbench-variants.golden` | — | — | probe extended to the remaining 15 suffix variants; the program changed, not its answer (F78) |
