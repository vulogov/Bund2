#!/bin/zsh
# Run a benchmark while sampling interference throughout the window.
#
# F135's protocol sampled load once, before the run. That missed the
# interference in criterion 7's run 2, which had to be caught by noticing that
# rows sharing no mechanism had regressed together. This samples every 3 s for
# the whole run and reports the worst process that is not part of this
# measurement, so a contaminated window is visible in the output rather than
# inferred from the result.
#
# **"Not part of this measurement" is decided by ancestry, not by name — F145.**
# The first version excluded `cargo`, `rustc` and the benchmark binary by
# matching the command string, so that the measurement's own build did not read
# as contamination. That also made every *foreign* cargo and rustc invisible:
# on 2026-10-01 a second agent session building an unrelated project held one
# core at 100% for twenty-two minutes and the guard would have called the
# window CLEAN. A concurrent compile is the single worst contaminant available
# and it was the one thing the guard could not see.
#
# So a process is excluded exactly when it is a descendant of this script. That
# is what the name list was a proxy for, and it is both narrower and complete:
# the benchmark's own toolchain is in the subtree, anything else is not.
#
# **The verdict line names no process — RFC-0005's twenty-fourth review, S12.**
# It used to end `peak=23.2% (<command>)`, and that line is what gets quoted
# into RFCs and registers, so the name of whatever happened to be running on
# the machine went into design documents with it. The line now says where the
# name is. The name itself is appended to a log under `target/`, which is not
# tracked: one row a run, with the time, the filter, the peak and the command.
# Set `GUARD_LOG` to put it elsewhere.
set -u
FILTER="$1"; shift
GUARD_LOG="${GUARD_LOG:-target/guard-peaks.log}"
mkdir -p "$(dirname "$GUARD_LOG")" 2>/dev/null
SAMPLES=$(mktemp)
GUARD_PID=$$
(
  while :; do
    # One `ps`, then attribute each row by walking its ppid chain up to this
    # script. A row whose chain reaches GUARD_PID is ours; everything else is
    # eligible to be the peak.
    ps -Ao pid,ppid,pcpu,comm | awk -v guard="$GUARD_PID" '
      NR > 1 {
        pid[NR]=$1; par[$1]=$2; cpu[NR]=$3;
        name[NR]=$4; self[NR]=$1; n=NR
      }
      END {
        for (i = 2; i <= n; i++) {
          p = self[i]; mine = 0; hops = 0
          while (p != 1 && p != 0 && p != "" && hops < 64) {
            if (p == guard) { mine = 1; break }
            p = par[p]; hops++
          }
          if (!mine && cpu[i] + 0 > best + 0) { best = cpu[i]; who = name[i] }
        }
        if (who != "") {
          k = split(who, parts, "/")
          print best, substr(parts[k], 1, 40)
        }
      }'
    sleep 3
  done
) > "$SAMPLES" 2>/dev/null &
SAMPLER=$!
trap "kill $SAMPLER 2>/dev/null" EXIT
"$@" 2>&1
kill $SAMPLER 2>/dev/null
python3 - "$SAMPLES" "$GUARD_LOG" "$FILTER" <<'PY'
import sys, time
rows=[l.split(None,1) for l in open(sys.argv[1]) if l.strip()]
vals=[(float(a),b.strip()) for a,b in rows if a.replace('.','',1).isdigit()]
if not vals:
    print("GUARD: no samples"); raise SystemExit
mx=max(vals); mean=sum(v for v,_ in vals)/len(vals)
verdict="CLEAN" if mx[0]<15 else "CONTAMINATED"
log=sys.argv[2]
try:
    with open(log,"a") as f:
        f.write(f"{time.strftime('%Y-%m-%dT%H:%M:%S')}\t{sys.argv[3]}\t{verdict}\t{mx[0]:.1f}%\t{mx[1]}\n")
    where=f"named in {log}"
except OSError:
    where="not recorded: the log could not be written"
print(f"GUARD [{verdict}] samples={len(vals)} peak={mx[0]:.1f}% ({where}) mean={mean:.1f}%")
PY
rm -f "$SAMPLES"
