#!rm day.profile.md
#!rm day.profile.dot
bund2 --profile day ledger.bund > /dev/null
sed -n '/^- ⟨program⟩/,/^      - `?key`/p' day.profile.md
