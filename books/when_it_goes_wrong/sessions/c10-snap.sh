#!rm half.profile.md
#!rm half.profile.dot
#!rm end.profile.md
#!rm end.profile.dot
bund2 snap.bund
bund2 --profile end snap.bund
ls *.profile.md
sed -n 5p half.profile.md | cut -d'|' -f3
