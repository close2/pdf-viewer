#!/usr/bin/env bash
#
# Run one corpus walk, census, sweep, fuzz run or survey shard under a memory bound, at idle
# priority, and say afterwards what it cost — or what stopped it.
#
# On 2026-09-01 a corpus campaign — eight survey shards over one directory, a census over five
# batches, gates and builds beside them — took the machine from 43 GB of anonymous memory to a
# 90 GB working set against 63 GB of RAM, into 47 GB of swap, and into a soft lockup that ended in
# a hard power-off. No process was under any limit and nothing on the owner's side was acting.
# The eight-hundred-and-sixty-sixth round measured whether that was a leak and it was not: one
# shard's peak is the same whether it walks 340 documents or 680, and a single-threaded walk is
# flat from the first document to the last. What it is instead is **concurrency**: a shard runs
# its documents through a rayon pool of one thread per core, so eight shards on a 24-core machine
# are 192 documents in flight, each with its bytes, its display list and its raster, and the
# figure grows with the documents in flight rather than with the documents walked. Eight shards
# of a slice that peaks at 12.9 GB each is the campaign that was observed. ADR 0798 has the
# measurements.
#
# So the bound here is the *walk's*, and a shard takes a share of it:
#
#   tools/bounded.sh [--shards N] [--data GiB] [--tree GiB] [--nice n] -- <command> [args…]
#
#   --shards N   this process is one of N run side by side (default 1). It gets nproc/N rayon
#                threads and (walk budget)/N of data, so the walk as a whole never exceeds the
#                budget or the machine's cores — eight shards of 24 threads each was the mistake.
#   --data GiB   RLIMIT_DATA for the command and everything it spawns, overriding the share.
#                On Linux ≥ 4.7 this counts every private anonymous mapping, which is what the
#                allocator hands out; RLIMIT_AS would count the file mappings and thread stacks a
#                rasteriser has and refuse programs that are not using memory at all.
#   --tree GiB   a ceiling on the *sum* of resident memory over the command's whole process tree,
#                sampled once a second; the tree is killed if it is crossed. For a `cargo build`,
#                whose memory is spread over many `rustc` processes no single RLIMIT sees.
#   --nice n     the niceness (default 19: everything here runs behind the owner's desktop and
#                behind any round's gates).
#
# The walk budget is 32 GiB, and the figure is argued rather than measured: the machine has
# 61 GiB, the owner's desktop keeps 16, a parallel round's gates and a build keep about 12, and
# what is left is what a corpus walk may use — **one walk on the machine at a time**, which is the
# same agreement as `doc/todo/02` §2's "run nothing beside the sequence".
#
# What a limit does to the channel that reports it is `doc/traps/instruments-and-reports.md`'s
# trap 18, and this script is written against it. RLIMIT_DATA touches no descriptor, so a program
# that runs out of it says so on its own standard error — Rust's allocation failure is one line and
# an abort — and this script pipes that channel through `tee` rather than pointing it at a file,
# keeps a copy, and reads it back afterwards so that the *last* line printed names the bound and
# not the document. The tree ceiling is the wrapper's own kill, and the wrapper says so itself. The
# command's exit status is passed through unchanged; nothing here turns a refusal into a success.

set -u -o pipefail

usage() {
    sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 64
}

walk_budget_gib=32
shards=1
data_gib=
tree_gib=
niceness=19

while [ $# -gt 0 ]; do
    case "$1" in
        --shards) shards=$2; shift 2 ;;
        --data) data_gib=$2; shift 2 ;;
        --tree) tree_gib=$2; shift 2 ;;
        --nice) niceness=$2; shift 2 ;;
        --) shift; break ;;
        -h|--help) usage ;;
        *) echo "bounded: unknown option $1" >&2; usage ;;
    esac
done
[ $# -gt 0 ] || usage
case "$shards" in ''|*[!0-9]*|0) echo "bounded: --shards wants a positive integer" >&2; exit 64 ;; esac

cores=$(nproc)
threads=$(( cores / shards ))
[ "$threads" -ge 1 ] || threads=1
if [ -z "$data_gib" ]; then
    data_gib=$(( walk_budget_gib / shards ))
    [ "$data_gib" -ge 1 ] || data_gib=1
fi
data_bytes=$(( data_gib * 1024 * 1024 * 1024 ))
tree_kib=
[ -n "$tree_gib" ] && tree_kib=$(( tree_gib * 1024 * 1024 ))

# rayon reads this once, at pool creation, and every walk in this tree uses the global pool. A
# caller that has set it already knows better than the share.
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-$threads}"

scratch=$(mktemp -d "${TMPDIR:-/tmp}/bounded.XXXXXX") || exit 1
errlog="$scratch/stderr"
trap 'rm -rf "$scratch"' EXIT

# The command's standard error goes through a pipe and `tee`, never straight to a file: a file
# is what a limit can reach (trap 18) and a pipe is not. Its standard output stays its own —
# fd 3 carries it around the pipeline — because a survey's report is that stream. The subshell
# exits with the *command's* status rather than `tee`'s.
started=$(date +%s)
(
    exec 3>&1
    prlimit --data="$data_bytes" nice -n "$niceness" "$@" 2>&1 1>&3 | tee "$errlog" >&2
    exit "${PIPESTATUS[0]}"
) &
leader=$!

# The resident memory of every process under `leader`, in KiB, and the leader's own high-water
# mark. One `ps` a second; a tree of a few dozen `rustc`s is nothing to it.
tree_rss() {
    ps -eo pid=,ppid=,rss= | awk -v root="$1" '
        { parent[$1] = $2; rss[$1] = $3 }
        END {
            n = 0; queue[n++] = root; total = 0
            for (i = 0; i < n; i++) {
                p = queue[i]; total += rss[p]
                for (c in parent) if (parent[c] == p) queue[n++] = c
            }
            print total
        }'
}
tree_pids() {
    ps -eo pid=,ppid= | awk -v root="$1" '
        { parent[$1] = $2 }
        END {
            n = 0; queue[n++] = root
            for (i = 0; i < n; i++) for (c in parent) if (parent[c] == queue[i]) queue[n++] = c
            for (i = 1; i < n; i++) print queue[i]
        }'
}

peak_kib=0
ceiling_hit=
while kill -0 "$leader" 2>/dev/null; do
    now=$(tree_rss "$leader")
    [ "$now" -gt "$peak_kib" ] && peak_kib=$now
    if [ -n "$tree_kib" ] && [ "$now" -gt "$tree_kib" ]; then
        ceiling_hit=$now
        # Children by pid, never by name: the process table is shared with parallel rounds
        # (doc/environment.md). TERM first so a build can leave its directory consistent.
        victims=$(tree_pids "$leader")
        # shellcheck disable=SC2086
        kill -TERM $victims 2>/dev/null
        sleep 2
        # shellcheck disable=SC2086
        kill -KILL $victims 2>/dev/null
        break
    fi
    sleep 1
done
wait "$leader"
status=$?
elapsed=$(( $(date +%s) - started ))

gib() { awk -v k="$1" 'BEGIN { printf "%.2f", k / 1048576 }'; }

# The last line names the bound where the bound is what ended the run, and the cost otherwise.
# Rust's allocator prints `memory allocation of N bytes failed` and aborts (status 134); a C
# program under the same limit says something else or nothing, and the status carries it.
if [ -n "$ceiling_hit" ]; then
    echo "bounded: KILLED BY THE TREE CEILING — the process tree reached $(gib "$ceiling_hit") GiB" \
         "resident against --tree $tree_gib GiB after ${elapsed}s. That is this wrapper's bound," \
         "not a fault in the command; a build wants a smaller -j, a walk more shards." >&2
    exit 137
fi
if grep -q "memory allocation of .* failed\|MemoryError\|Cannot allocate memory" "$errlog"; then
    echo "bounded: STOPPED BY THE DATA LIMIT — exit $status after ${elapsed}s, the process" \
         "tree peaked at $(gib "$peak_kib") GiB resident under an RLIMIT_DATA of $data_gib GiB" \
         "with $RAYON_NUM_THREADS rayon thread(s). The command's own last words are above; the" \
         "bound is the reason, and the document it was on is what to look at only if the same" \
         "run passes with fewer threads and fails again with more (--shards divides both)." >&2
    exit "$status"
fi
if [ "$status" -eq 134 ]; then
    # A Rust program built with `panic = "abort"` ends a panic this way too, and a panic is not
    # the bound: saying "the data limit" here would be a report firing on a condition it does not
    # state (trap 11). The program's own words are above; this line only refuses to name a cause.
    echo "bounded: ABORTED (status 134) after ${elapsed}s with no allocation failure on its" \
         "standard error — a panic under panic = \"abort\", not the bound. Peak $(gib "$peak_kib") GiB" \
         "resident, RLIMIT_DATA $data_gib GiB, $RAYON_NUM_THREADS rayon thread(s)." >&2
    exit "$status"
fi
echo "bounded: exit $status after ${elapsed}s; peak $(gib "$peak_kib") GiB resident over the" \
     "process tree, under RLIMIT_DATA $data_gib GiB, $RAYON_NUM_THREADS rayon thread(s), nice $niceness" >&2
exit "$status"
