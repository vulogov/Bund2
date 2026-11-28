#!rm tare.trace
bund2 --trace tare.trace tare.bund > /dev/null
wc -l tare.trace
grep -c "^  dues" tare.trace
grep -c "println" tare.trace
