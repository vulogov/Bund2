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
| `probes/filesystem-words.golden` | — | — | F153: every path in the probe was made absolute, because the effect audit runs probes from a different working directory than the capture and the relative version aborted there, leaving files in the crate directory; the probe gained the cp/mv aliases in the same pass. The program changed, not its answer |
| `probes/sysinfo-words.golden` | — | — | F155: `type.of` does not consume its operand, so every `type.of` line left a raw value on the stack and the first capture pinned ten machine-specific figures, two of them volatile; each line now ends in `drop` and the stack ends empty. The program changed, not its answer |
| `probes/system-shell-words.golden` | — | — | F162: the probe gained a clause each for `sh` and `sh.`, the aliases `system.shell` had landed without; an alias no golden runs is invisible to conformance. The program changed, not its answer |
