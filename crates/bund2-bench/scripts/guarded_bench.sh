#!/bin/zsh
# Run a benchmark while sampling interference throughout the window.
#
# F135's protocol sampled load once, before the run. That missed the
# interference in criterion 7's run 2, which had to be caught by noticing that
# rows sharing no mechanism had regressed together. This samples every 3 s for
# the whole run and reports the worst non-benchmark process, so a contaminated
# window is visible in the output rather than inferred from the result.
set -u
FILTER="$1"; shift
SAMPLES=$(mktemp)
(
  while :; do
    ps aux | awk 'NR>1 && $11 !~ /interpret-|cargo|rustc|guarded_bench|^ps$|awk/ {print $3, substr($11,1,40)}' \
      | sort -rn | head -1
    sleep 3
  done
) > "$SAMPLES" 2>/dev/null &
SAMPLER=$!
trap "kill $SAMPLER 2>/dev/null" EXIT
"$@" 2>&1
kill $SAMPLER 2>/dev/null
python3 - "$SAMPLES" <<'PY'
import sys
rows=[l.split(None,1) for l in open(sys.argv[1]) if l.strip()]
vals=[(float(a),b.strip()) for a,b in rows if a.replace('.','',1).isdigit()]
if not vals:
    print("GUARD: no samples"); raise SystemExit
mx=max(vals); mean=sum(v for v,_ in vals)/len(vals)
verdict="CLEAN" if mx[0]<15 else "CONTAMINATED"
print(f"GUARD [{verdict}] samples={len(vals)} peak={mx[0]:.1f}% ({mx[1]}) mean={mean:.1f}%")
PY
rm -f "$SAMPLES"
