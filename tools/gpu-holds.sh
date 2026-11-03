#!/usr/bin/env bash
# What a run held the shared graphics adapter's lock for, from the lines the lock itself wrote
# (ADR 2664): run the command with PDF_VIEWER_GPU_LOCK_LOG=<file>, then `tools/gpu-holds.sh <file>`.
#
# Prints the number of holds, the longest single hold, the total locked time, the longest wait for
# the lock, and the longest hold of each section kind, all in milliseconds. A run that wrote no line
# took no lock, which on a machine with a shared adapter means it did no hardware device work.
set -euo pipefail
log=${1:?usage: tools/gpu-holds.sh <the PDF_VIEWER_GPU_LOCK_LOG file>}
[ -f "$log" ] || { echo "gpu-holds: $log does not exist: the run took no lock"; exit 0; }
awk '
    # A gpu-adapter line names the adapter of one bring-up, and is no hold.
    $1 == "gpu-adapter" { adapters[$3]++; next }
    {
        for (i = 1; i <= NF; i++) {
            split($i, kv, "=")
            field[kv[1]] = kv[2]
        }
        held = field["held_ms"] + 0
        waited = field["waited_ms"] + 0
        holds++
        total += held
        if (held > longest) { longest = held; longest_section = field["section"] }
        if (waited > longest_wait) longest_wait = waited
        if (held > by_section[field["section"]]) by_section[field["section"]] = held
        count[field["section"]]++
    }
    END {
        printf "holds %d, longest %.1f ms (%s), total %.1f ms, longest wait %.1f ms\n",
            holds, longest, longest_section, total, longest_wait
        for (a in adapters) printf "  bring-ups on %s: %d\n", a, adapters[a]
        for (s in count) printf "  %-20s %6d holds, longest %.1f ms\n", s, count[s], by_section[s]
    }
' "$log"
