#!/usr/bin/env bash
# Review 1456 section 1.3, re-taken by review 1513 (brief range moved to 1456 onwards): what a round reads before it writes.
# Usage: bash doc/reviews/1513/reading.sh   (from the worktree root)
# Sums `tools/round.sh --lines [kind]` per kind (the script prints no total), then counts each
# brief under /home/AI written from the template: its common part (up to the first "## Slot")
# and its longest slot part, against the template's 60 + 25.
set -euo pipefail
every=$(tools/round.sh --lines | awk '$1 ~ /^[0-9]+$/ { t += $1 } END { print t }')
printf 'every round: %s lines\n' "$every"
for k in $(tools/round.sh --list); do
    printf '  %-12s %5s\n' "$k" "$(tools/round.sh --lines "$k" | awk '$1 ~ /^[0-9]+$/ { t += $1 } END { print t }')"
done
printf 'briefs (whole / common part / longest slot part / identical lines with the previous brief):\n'
prev=
for b in $(ls /home/AI/batch-1[345][0-9][0-9]-brief.md | sort -t- -k2 -n); do
    n=${b##*/batch-}; n=${n%%-*}
    [ "$n" -ge 1456 ] || continue
    whole=$(wc -l < "$b")
    common=$(awk '/^## Slot/ { exit } { n++ } END { print n }' "$b")
    slot=$(awk '/^## Slot/ { if (n > m) m = n; n = 0; on = 1; next } on { n++ } END { if (n > m) m = n; print m + 0 }' "$b")
    same=-
    [ -n "$prev" ] && same=$(comm -12 <(sort -u "$prev") <(sort -u "$b") | grep -c . || true)
    printf '  %s  %4s %4s %4s %5s\n' "$n" "$whole" "$common" "$slot" "$same"
    prev=$b
done
