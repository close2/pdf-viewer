#!/usr/bin/env bash
# Review 1513: trap and habit citations in the records of one session range, by the same pattern
# `tools/state.sh traps` uses over the whole of doc/history/ (its section prints no range).
# Usage: bash doc/reviews/1513/cites.sh <first-session> <last-session>   (from the worktree root)
set -euo pipefail
lo=$1 hi=$2
files=$(ls doc/history/ | awk -F- -v lo="$lo" -v hi="$hi" '$1 + 0 >= lo && $1 + 0 <= hi { print "doc/history/" $0 }')
printf 'records %s-%s: %s\n' "$lo" "$hi" "$(printf '%s\n' "$files" | grep -c .)"
for kind in trap habit; do
    # shellcheck disable=SC2086
    hits=$(grep -hoE "\b${kind}s? [0-9]+[a-z]?(( ?[,–-] ?| and | or )[0-9]+[a-z]?)*" $files |
        sed -E "s/^${kind}s? //; s/ and | or |,|–|-/ /g" | tr -s ' ' '\n' | grep -E '^[0-9]+[a-z]?$' || true)
    # shellcheck disable=SC2086
    recs=$( (grep -lE "\b${kind}s? [0-9]+" $files || true) | wc -l)
    printf '%s citations: %s in %s records; by number: %s\n' "$kind" "$(printf '%s\n' "$hits" | grep -c . || true)" "$recs" \
        "$(printf '%s\n' "$hits" | grep . | sort | uniq -c | sort -k1,1nr -k2,2n | awk '{ printf "%s %s×, ", $2, $1 }')"
done
