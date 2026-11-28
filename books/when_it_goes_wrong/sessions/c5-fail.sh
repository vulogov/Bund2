#!rm slip.trace
bund2 --trace slip.trace slip.bund > /dev/null 2>&1
tail -8 slip.trace
