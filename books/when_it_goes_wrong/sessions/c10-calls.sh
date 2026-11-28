#!rm day.profile.md
#!rm day.profile.dot
bund2 --profile day ledger.bund > /dev/null
grep -E '^. .(dues|due|fee|has). .  . word' day.profile.md | cut -d'|' -f2,4-7
