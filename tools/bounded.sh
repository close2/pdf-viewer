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
#   --self-test  run the sampler against synthetic process tables and against live trees — one
#                that fans out, one that crosses the ceiling, one whose sampler stalls — and exit
#                0 only if every case holds. `tools/conformance/tests/bounded.rs` runs it under
#                `cargo test -p conformance`, so the sequence's last line exercises the bound.
#
# The walk budget is 12 GiB a round, and the figure was 32 until 2026-09-02, when 32 turned out to
# be sized for a machine running one walk and this machine was running three rounds. The timeline,
# from the user slice's own accounting: at 09:03:14 the eight-hundred-and-seventy-fourth round
# launched `bounded.sh --data 32 -- safedocs survey --dir …/MOZILLA` in the background — the whole
# walk budget for one 24-thread process, no `--tree` — beside the owner's desktop, the Claude
# process, sccache and two other rounds' gates and builds; the slice's memory.peak reached
# 61.09 GB of 61.9; from 09:05 every shell call of that round and its neighbour's stalled; the
# survey was killed at 09:07:23; and at 09:08:04 the Claude process aborted (its own abort(), not
# oomd and not the kernel's OOM killer, whose oom_kill count is 0 in every cgroup). RLIMIT_DATA is
# **per process**, so `--data 32` bounded nothing the machine cared about: what mattered was the
# sum over every round, and no single limit sees that. So four rules, the owner's and binding on
# every round (`doc/environment.md`'s parallel-round agreements carry them too):
#
#   1. one corpus walk at a time across ALL rounds, not one per round;
#   2. `--data` never above 12 GiB for a round;
#   3. every bounded run also carries `--tree` — 12 GiB for a walk, 8 for a build — and this
#      script defaults it to 12 where the caller gave none, so that a run without a tree ceiling
#      cannot be started by omission;
#   4. the sum of what a round has in flight stays under 16 GiB.
#
# This script enforces the half of that it can see: `--data` above 12 GiB is refused unless `--tree`
# is given as well, because a data limit above the round's share is exactly the invocation that
# needs the ceiling most. **One walk on the machine at a time** is the same agreement as
# `doc/todo/02` §2's "run nothing beside the sequence".
#
# What a limit does to the channel that reports it is `doc/traps/instruments-and-reports.md`'s
# trap 18, and this script is written against it. RLIMIT_DATA touches no descriptor, so a program
# that runs out of it says so on its own standard error — Rust's allocation failure is one line and
# an abort — and this script pipes that channel through `tee` rather than pointing it at a file,
# keeps a copy, and reads it back afterwards so that the *last* line printed names the bound and
# not the document. The tree ceiling is the wrapper's own kill, and the wrapper says so itself. The
# command's exit status is passed through unchanged; nothing here turns a refusal into a success.
#
# The tree ceiling is only as good as the sampler that measures it, and the sampler used to be
# able to stall. Until the eight-hundred-and-eightieth round it walked the process table with an
# inner loop over *every* process for *every* node of the tree — quadratic, 6 s a sample over a
# tree of 8 000 processes and 16 s over 16 000, measured on a synthetic table — with no guard
# against visiting a pid twice, and no bound at all on how long `ps` might take under exactly the
# memory pressure the ceiling exists to prevent; round 874 watched one such sample hang for
# minutes and killed it by pid. A bound that is not being measured is a bound that is not there.
# So now: one `ps` a second, read into per-parent child lists and walked once (linear in the
# table: the self-test's hundred-thousand-row case is about a hundred milliseconds), each pid
# counted once; the sample runs in the background against a deadline, and a sample that misses
# it is abandoned rather than waited for; and a run of missed samples — the wrapper *blind* for
# that long — kills the tree and says so, because that is the machine going down and the walk is
# the one thing on it this wrapper can stop. ADR 0807.

set -u -o pipefail

usage() {
    sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 64
}

walk_budget_gib=12
round_share_gib=12
shards=1
data_gib=
tree_gib=
niceness=19
self_test=

# The sampler's cadence: one sample a second, each abandoned after `sample_deadline` seconds,
# and `blind_limit` abandoned samples in a row kill the tree. Half a minute of blindness is the
# figure: on 2026-09-02 every shell call on the machine stalled for three minutes before the
# session was lost, and a wrapper that had noticed within thirty seconds would have ended the
# walk that caused it. The self-test lowers both to keep its stall case short.
sample_interval=1
sample_deadline=5
blind_limit=6

while [ $# -gt 0 ]; do
    case "$1" in
        --shards) shards=$2; shift 2 ;;
        --data) data_gib=$2; shift 2 ;;
        --tree) tree_gib=$2; shift 2 ;;
        --nice) niceness=$2; shift 2 ;;
        --self-test) self_test=1; shift ;;
        --) shift; break ;;
        -h|--help) usage ;;
        *) echo "bounded: unknown option $1" >&2; usage ;;
    esac
done

# ---------------------------------------------------------------------------------------------
# The sampler.
#
# `walk_table ROOT` reads `pid ppid rss` rows on standard input and prints the tree under ROOT:
# the resident total in KiB on the first line, then one descendant pid per line. Children are
# gathered per parent in one pass — indexed, not concatenated: a string of a hundred thousand
# pids grown one at a time was a second and a half by itself — and the tree is walked once from
# the root, so the cost is the table's size and not its size times the tree's; `seen` makes a pid that appears twice — a
# cycle assembled from pids reused while `ps` was reading `/proc`, or a duplicated row — cost
# one visit rather than a loop that never ends.
walk_table() {
    awk -v root="$1" '
        { rss[$1] = $3; kid[$2, ++kids[$2]] = $1 }
        END {
            n = 0; queue[n++] = root; seen[root] = 1; total = 0
            for (i = 0; i < n; i++) {
                p = queue[i]; total += rss[p]
                for (j = 1; j <= kids[p]; j++) {
                    c = kid[p, j]
                    if (!(c in seen)) { seen[c] = 1; queue[n++] = c }
                }
            }
            print total
            for (i = 1; i < n; i++) print queue[i]
        }'
}

# The process table, as `walk_table` reads it. A function rather than a string so that the
# self-test can stand a stalling one in its place.
process_table() { ps -eo pid=,ppid=,rss=; }

# `sample_tree ROOT FILE` writes `walk_table`'s output for the live table into FILE, in the
# background, and waits for it no longer than `sample_deadline` seconds. Returns 0 when the
# sample landed and 1 when it was abandoned — a sampler stuck inside the kernel does not die on
# SIGKILL either, so the abandoned one is left to finish or not on its own and never waited for.
sample_tree() {
    local root=$1 file=$2 sampler waited=0
    ( process_table | walk_table "$root" > "$file.partial" && mv "$file.partial" "$file" ) &
    sampler=$!
    while kill -0 "$sampler" 2>/dev/null; do
        if [ "$waited" -ge $(( sample_deadline * 10 )) ]; then
            # Disowned first, so that the shell reports nothing when the kill lands — or does
            # not: a `ps` blocked inside the kernel ignores SIGKILL until it returns.
            disown "$sampler" 2>/dev/null
            kill -KILL "$sampler" 2>/dev/null
            return 1
        fi
        sleep 0.1
        waited=$(( waited + 1 ))
    done
    wait "$sampler" 2>/dev/null
    [ -s "$file" ]
}

# `watch_tree LEADER KIB` samples the tree under LEADER once an interval until it exits or the
# resident sum crosses KIB, and sets `peak_kib`, `ceiling_hit` and `blind` for the caller. The
# kill list is the pids of the sample that crossed the ceiling — by pid and never by name,
# because the process table is shared with parallel rounds (doc/environment.md) — TERM first so
# a build can leave its directory consistent, then KILL.
watch_tree() {
    local leader=$1 ceiling=$2 sample="$scratch/sample" now misses=0 victims
    peak_kib=0; ceiling_hit=; blind=
    while kill -0 "$leader" 2>/dev/null; do
        if sample_tree "$leader" "$sample"; then
            misses=0
            now=$(head -n 1 "$sample")
            [ "$now" -gt "$peak_kib" ] && peak_kib=$now
            if [ "$now" -gt "$ceiling" ]; then
                ceiling_hit=$now
                victims=$(tail -n +2 "$sample")
                # shellcheck disable=SC2086
                kill -TERM $victims 2>/dev/null
                sleep 2
                # shellcheck disable=SC2086
                kill -KILL $victims 2>/dev/null
                return
            fi
        else
            misses=$(( misses + 1 ))
            echo "bounded: a sample of the process tree did not return within ${sample_deadline}s ($misses of $blind_limit before the tree is stopped)" >&2
            if [ "$misses" -ge "$blind_limit" ]; then
                blind=$misses
                # No fresh list can be had — that is the condition — so the last good sample's.
                victims=$( [ -s "$sample" ] && tail -n +2 "$sample" )
                # shellcheck disable=SC2086
                kill -TERM $victims "$leader" 2>/dev/null
                sleep 2
                # shellcheck disable=SC2086
                kill -KILL $victims "$leader" 2>/dev/null
                return
            fi
        fi
        sleep "$sample_interval"
    done
}

# ---------------------------------------------------------------------------------------------
# The self-test: each case prints one line, and the script exits 1 on the first that fails.
if [ -n "$self_test" ]; then
    scratch=$(mktemp -d "${TMPDIR:-/tmp}/bounded-self-test.XXXXXX") || exit 1
    trap 'rm -rf "$scratch"' EXIT
    fail() { echo "bounded --self-test: FAILED — $*" >&2; exit 1; }
    self=${BASH_SOURCE[0]}

    # 1. A flat tree of 100 000 children under the root, beside 500 strangers: the total is the
    #    root's 100 plus 100 000 tens, every child is listed once, and the whole walk costs well
    #    under the sampler's interval. The quadratic walk this replaced needed minutes here.
    awk 'BEGIN { print 1000, 1, 100; for (i = 1; i <= 100000; i++) print 1000 + i, 1000, 10
                 for (i = 1; i <= 500; i++) print 200000 + i, 1, 5 }' > "$scratch/flat"
    started_ns=$(date +%s%N)
    walk_table 1000 < "$scratch/flat" > "$scratch/flat.out"
    cost_ms=$(( ($(date +%s%N) - started_ns) / 1000000 ))
    [ "$(head -n 1 "$scratch/flat.out")" = 1000100 ] || fail "flat tree: total $(head -n 1 "$scratch/flat.out"), wanted 1000100"
    [ "$(tail -n +2 "$scratch/flat.out" | wc -l)" = 100000 ] || fail "flat tree: $(tail -n +2 "$scratch/flat.out" | wc -l) descendants listed, wanted 100000"
    [ "$cost_ms" -lt 1000 ] || fail "flat tree: one sample cost ${cost_ms} ms, which is not a fraction of the interval it has to fit"
    echo "bounded --self-test: a flat tree of 100000 sampled correctly in ${cost_ms} ms"

    # 2. A chain 50 000 deep, and a table holding a cycle and a duplicated row: the walk ends,
    #    and every pid counts once.
    awk 'BEGIN { print 1000, 1, 1; for (i = 1; i <= 50000; i++) print 1000 + i, 1000 + i - 1, 1 }' > "$scratch/chain"
    [ "$(walk_table 1000 < "$scratch/chain" | head -n 1)" = 50001 ] || fail "chain: total $(walk_table 1000 < "$scratch/chain" | head -n 1), wanted 50001"
    printf '1000 1 1\n1001 1000 2\n1002 1001 4\n1001 1002 2\n1002 1001 4\n' > "$scratch/cycle"
    [ "$(walk_table 1000 < "$scratch/cycle" | head -n 1)" = 7 ] || fail "cycle: total $(walk_table 1000 < "$scratch/cycle" | head -n 1), wanted 7"
    [ "$(walk_table 1000 < "$scratch/cycle" | tail -n +2 | sort | tr '\n' ' ')" = "1001 1002 " ] || fail "cycle: descendants $(walk_table 1000 < "$scratch/cycle" | tail -n +2 | tr '\n' ' ')"
    echo "bounded --self-test: a chain of 50000, a cycle and a duplicate walked once each"

    # 3. A live tree that fans out into two hundred short-lived children under the wrapper
    #    itself: exit 0, and the peak is a positive figure.
    "$self" --tree 1 --data 1 --nice 0 -- bash -c 'for i in $(seq 200); do sleep 0.3 & done; wait' \
        > "$scratch/fan.out" 2> "$scratch/fan.err"
    status=$?
    [ "$status" -eq 0 ] || fail "fan-out: exit $status: $(tail -n 1 "$scratch/fan.err")"
    grep -q 'bounded: exit 0 after [0-9]*s; peak [0-9]*\.[0-9]* GiB resident' "$scratch/fan.err" || fail "fan-out: $(tail -n 1 "$scratch/fan.err")"
    grep -q 'peak 0\.00 GiB' "$scratch/fan.err" && fail "fan-out: the peak is 0.00 GiB, so the sampler saw nothing: $(tail -n 1 "$scratch/fan.err")"
    echo "bounded --self-test: a live tree of 200 children: $(tail -n 1 "$scratch/fan.err" | sed 's/^bounded: //')"

    # 4. A child that holds 1.5 GiB resident under a ceiling of 1 GiB is killed by the ceiling —
    #    exit 137 and the line that names it — and not by the data limit, which is above it.
    if command -v python3 > /dev/null; then
        "$self" --tree 1 --data 3 --nice 0 -- python3 -c 'import time; b = b"x" * (1536 << 20); time.sleep(20)' \
            > "$scratch/ceiling.out" 2> "$scratch/ceiling.err"
        status=$?
        [ "$status" -eq 137 ] || fail "ceiling: exit $status, wanted 137: $(tail -n 1 "$scratch/ceiling.err")"
        grep -q 'KILLED BY THE TREE CEILING' "$scratch/ceiling.err" || fail "ceiling: $(tail -n 1 "$scratch/ceiling.err")"
        echo "bounded --self-test: a child over the ceiling was stopped: $(tail -n 1 "$scratch/ceiling.err" | cut -c1-110)…"
    else
        echo "bounded --self-test: NOT RUN — the ceiling case wants python3 to hold 1.5 GiB resident, and there is none" >&2
    fi

    # 5. A sampler that never returns: with the deadline at a second and the limit at three,
    #    the wrapper goes blind, stops the tree within a few seconds and names the reason.
    process_table() { sleep 60; }
    sample_deadline=1; blind_limit=3
    (
        exec 3>&1
        sleep 30 2>&1 1>&3 &
        wait $!
    ) &
    leader=$!
    started=$(date +%s)
    watch_tree "$leader" $(( 1024 * 1024 )) 2> "$scratch/blind.err"
    elapsed=$(( $(date +%s) - started ))
    [ -n "$blind" ] || fail "blind: the watch returned without going blind"
    kill -0 "$leader" 2>/dev/null && fail "blind: the leader is still running after the watch stopped it"
    [ "$elapsed" -lt 15 ] || fail "blind: took ${elapsed}s to stop a tree whose sampler stalled"
    [ "$(grep -c 'did not return within' "$scratch/blind.err")" = 3 ] || fail "blind: $(cat "$scratch/blind.err")"
    echo "bounded --self-test: a stalled sampler stopped the tree in ${elapsed}s after 3 missed samples"

    echo "bounded --self-test: every case holds"
    exit 0
fi

[ $# -gt 0 ] || usage
case "$shards" in ''|*[!0-9]*|0) echo "bounded: --shards wants a positive integer" >&2; exit 64 ;; esac

cores=$(nproc)
threads=$(( cores / shards ))
[ "$threads" -ge 1 ] || threads=1
if [ -z "$data_gib" ]; then
    data_gib=$(( walk_budget_gib / shards ))
    [ "$data_gib" -ge 1 ] || data_gib=1
fi
case "$data_gib" in ''|*[!0-9]*|0) echo "bounded: --data wants a positive integer of GiB" >&2; exit 64 ;; esac
if [ "$data_gib" -gt "$round_share_gib" ] && [ -z "$tree_gib" ]; then
    echo "bounded: --data $data_gib GiB is above a round's share of $round_share_gib, and no --tree ceiling was given — see the header for 2026-09-02, when exactly this invocation took the machine down; pass --tree as well, or a smaller --data" >&2
    exit 64
fi
# Rule 3 of the header: a run without a tree ceiling is not started by omission.
[ -n "$tree_gib" ] || tree_gib=$round_share_gib
case "$tree_gib" in ''|*[!0-9]*|0) echo "bounded: --tree wants a positive integer of GiB" >&2; exit 64 ;; esac
data_bytes=$(( data_gib * 1024 * 1024 * 1024 ))
tree_kib=$(( tree_gib * 1024 * 1024 ))

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

watch_tree "$leader" "$tree_kib"
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
if [ -n "$blind" ]; then
    echo "bounded: KILLED BLIND — $blind samples of the process tree in a row did not return within" \
         "${sample_deadline}s each, so for the last $(( blind * (sample_deadline + sample_interval) ))s the" \
         "--tree $tree_gib GiB ceiling was not being measured; the tree was stopped after ${elapsed}s" \
         "rather than run unbounded. Peak seen before that: $(gib "$peak_kib") GiB. A stalled \`ps\` is" \
         "the machine under memory pressure: look at what else is running before running this again." >&2
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
