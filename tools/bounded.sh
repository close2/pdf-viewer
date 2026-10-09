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
#   tools/bounded.sh [--lock [--clock | --long] [--round N]] [--build '<cargo build arguments>']… [--shards N] [--data GiB] [--tree GiB] [--tasks N] [--nice n] -- <command> [args…]
#   tools/bounded.sh --held
#
#   --shards N   this process is one of N run side by side (default 1). It gets nproc/N rayon
#                threads and (walk budget)/N of data, so the walk as a whole never exceeds the
#                budget or the machine's cores — eight shards of 24 threads each was the mistake.
#                A run under --lock gets four rayon threads instead, whatever N: the figure the
#                merge runs every gate at, and the one each lane's ceiling was measured at.
#   --data GiB   RLIMIT_DATA for the command and everything it spawns, overriding the share.
#                On Linux ≥ 4.7 this counts every private anonymous mapping, which is what the
#                allocator hands out; RLIMIT_AS would count the file mappings and thread stacks a
#                rasteriser has and refuse programs that are not using memory at all.
#   --tree GiB   a ceiling on the *sum* of resident memory over the command's whole process tree,
#                sampled once a second; the tree is killed if it is crossed. For a `cargo build`,
#                whose memory is spread over many `rustc` processes no single RLIMIT sees.
#   --tasks N    RLIMIT_NPROC for the command and everything it spawns (default and ceiling: the
#                agent's task budget below). The kernel counts it against every task of the user,
#                so it bounds the agent as a whole, not this command alone.
#   --task-budget  print the agent's task budget and exit: `tools/*.sh` ask for the figure here,
#                so it is written down once (`ulimit -u "$(tools/bounded.sh --task-budget)"`).
#   --nice n     the niceness (default 19: everything here runs behind the owner's desktop and
#                behind any round's gates).
#   --lock       take the heavy-walk lock (/home/AI/heavy-walk.lock) before the command starts, hold
#                it until the wrapper ends, and append one line to /home/AI/heavy-walk.log saying how
#                long the run queued for it, behind what, how long it held it, the tree's peak, the
#                run's kind and its lane (below) — a run stopped while it queued writes its line too,
#                with `lane=-`. Walks are granted in the order they asked. The lock has two lanes: a run whose `--tree` is 6
#                GiB or less takes either, any larger one the first, a `--clock` run both and a `--long` run
#                the second only. An
#                ancestor that holds the lock already — `flock <lock> tools/bounded.sh --lock …`, or
#                a `--lock` wrapper this command runs under — is found and used, never queued
#                behind. The command runs without the lanes' descriptors and with
#                `HEAVY_WALK_HELD_BY` naming the process that holds them for the command.
#   --clock      with --lock: this run's figure is a time — a gate with a band on a clock, an A/B
#                pair, a launch measured with its clocks — so it takes both lanes, and while it
#                waits for them no other run is granted either. A run that holds a clock run anywhere
#                inside it declares this itself; a `--clock` run inside a hold of one lane is refused.
#   --long       with --lock: this run holds its lane for as long as it chooses — a fuzz campaign, a
#                seed census over the corpora — so it takes the second lane and only the second, and
#                waits for it rather than take the first while the first is free; the first stays
#                a walk's. Its `--tree` is the second lane's, 6 GiB, and is refused above that; a
#                `--long` run inside a hold of the first lane is refused (ADR 1756).
#   --held       exit 0 if the caller runs under a hold of the lock — it has a lane's descriptor
#                open, or `HEAVY_WALK_HELD_BY` names an ancestor that has — and 1 otherwise. What a
#                script that must run under the lock asks before it walks (`tools/batch.sh arms-held`).
#   --round N    the round's session number, written on that line so a round's lock time can be
#                read off the log (default `-`).
#   --build ARGS `cargo build ARGS` before the command, inside the same hold and the same bound, each
#                in the order given; a build that fails ends the run with its status and the
#                command is not started. ARGS are separate words (`--profile gates -p pdf-sandbox
#                --bins`). What a walk spawns and Cargo will not build for it — the sandbox worker of
#                the walk's profile above all — is rebuilt here rather than before the lock, because a
#                walk that queued behind the lock spawns whatever the tree was when the build ran,
#                and five siblings edit it meanwhile (trap 109). The line's `cmd=` is the command's,
#                so a gate keeps its name on the log (ADR 1710).
#   --self-test  run the sampler against synthetic process tables and against live trees — one
#                that fans out, one that crosses the ceiling, one whose sampler stalls, one that
#                forks past a task limit of its own, the lock's lanes, a build inside a hold, a long
#                run pinned to the second lane, walks granted in the order they asked and a wait
#                stopped before its grant — and
#                exit 0 only if every case holds. `tools/conformance/tests/bounded.rs` runs it under
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
#
# **And no memory bound sees a process count.** On 2026-10-06 one tool forked a task per package
# and never waited: the agent's scope climbed about 3 400 tasks a minute to 52 259, and their
# stacks — 50 GB resident and 91 GB of swap — are what the system's OOM daemon killed the whole
# agent for, every round with it (trap 116). Each of those tasks was small, so neither RLIMIT_DATA
# nor the tree ceiling, which a stalled `ps` cannot sample anyway, was the bound that could act.
# RLIMIT_NPROC is: `fork` and `clone` fail with EAGAIN once the user holds that many tasks, at the
# call, with nothing to sample. ADR 1612.
#
# **And the lock's cost was nobody's number.** One heavy walk on the machine at a time is rule 1
# above, kept by `flock` on one file; three records put a round's queue for it at 350 to 3 900 s,
# each by hand, and the question whether the machine wants a second lock was declined for want of
# a count (doc/reviews/1401, section 3). So `--lock` takes the lock here and writes one line per run
# when the wrapper ends — `<asked> batch=<branch> round=<N> wait=<s> hold=<s> exit=<status>
# cmd=<command>`, the branch being the batch's (that of the tree this script is in) —
# and `tools/state.sh gates-cost` prints the last batch's lines and each round's sum. ADR 1646.
#
# **The descriptor stays with the wrapper and is not handed to the command.** Handed down, as
# `flock <lock> <command>` leaves it, the lock was held by anything the walk started for as long as
# that thing lived, and a build inside the lock starts `sccache`'s server, which outlives the build
# by design: on 2026-10-07 one held the machine's lock for about eighteen minutes after the walk
# that started it had ended, while two rounds queued (ADR 1659 section 4, trap 131).
# So the command and the `tee` beside it run with the descriptor closed, and the subshell that
# waits for them keeps it: the lock is held exactly while the command runs, and still while it
# runs after the wrapper above it was killed. What a script under the lock reads instead of the
# descriptor is `HEAVY_WALK_HELD_BY`, that subshell's pid, which `--held` accepts only while that
# process is an ancestor of the caller and has the descriptor open — so a daemon the walk left
# behind, reparented away from it, is under no hold. ADR 1674.
#
# **And one lock was one lane for a machine with room for two.** Every walk queued behind every
# other, whatever either would peak at, and the rounds of one batch queued 10 759 s behind holds
# that a second lane of 6 GiB could have run beside them (ADR 1671 section 3). So the lock has a
# second lane for walks declared at `--tree 6` or less; a clock run takes both, because a time
# measured beside another walk is the busy end of trap 110, and a gate in front of the lanes lets a
# waiting clock run stop every new grant rather than wait for two holds to end together. ADR 1684.
#
# **And a small walk was the same kind whether it held its lane for a minute or an hour.** A small
# walk takes the second lane, or the first when the second is busy, so two campaigns of an hour each
# held both lanes between them while a 25 s corpus gate queued 4 289 s behind them. So a run whose
# length is its own choice declares `--long` and is pinned to the second lane: a second long run
# queues behind the first, every walk keeps the first lane, and a clock run waiting for a long run
# stops no walk while it waits (below). ADR 1756.
#
# **And a walk asked first was not granted first, and a walk that stopped waiting left no line.** Walks
# polled for a freed lane with no order among them, so round 1470's small walk queued 4 339.7 s while
# one asked eleven minutes after it queued 490.8 s behind the same holds; and the line was written at
# a hold's end, so a run stopped while it queued paid its queue on no line. So a walk leaves a ticket
# when it asks and is granted a lane only where no earlier ask takes that lane, and a wait that ends
# before a grant writes its line with `lane=-` (below). ADR 1790.

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

# The agent's task budget: RLIMIT_NPROC is checked against the count of **every** task — process
# and thread — the user holds, not against this command's, so whatever figure is set is the budget
# of all six rounds, the orchestrator and their builds together. 8192 is eight times what they hold
# at work: six rounds' builds, test pools and Xvfb servers sit under 1 000 tasks, and the incident
# this answers was a spawner at 52 259. A command that needs more is a fork loop, not a workload.
# Every heavy line in `tools/*.sh` reads this figure through `--task-budget`; no other copy exists.
task_budget=8192
tasks=$task_budget

# The lock every round's walks queue on, and the log of what they paid for it. The two variables
# exist for the self-test, which takes a lock of its own and must never queue behind a real walk.
lock_path=${HEAVY_WALK_LOCK:-/home/AI/heavy-walk.lock}
lock_log=${HEAVY_WALK_LOG:-/home/AI/heavy-walk.log}
take_lock=
clock=
long=
held_query=
round=-
lane_fds=
lanes=
builds=()

while [ $# -gt 0 ]; do
    case "$1" in
        --shards) shards=$2; shift 2 ;;
        --data) data_gib=$2; shift 2 ;;
        --tree) tree_gib=$2; shift 2 ;;
        --tasks) tasks=$2; shift 2 ;;
        --task-budget) echo "$task_budget"; exit 0 ;;
        --nice) niceness=$2; shift 2 ;;
        --lock) take_lock=1; shift ;;
        --clock) clock=1; shift ;;
        --long) long=1; shift ;;
        --held) held_query=1; shift ;;
        --round) round=$2; shift 2 ;;
        --build) builds+=("$2"); shift 2 ;;
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
# The lock, in two lanes (ADR 1684).
#
# Three files: the first lane is the lock file itself, the second is `<lock>.lane2`, and `<lock>.gate`
# is the order. A run's kind is what it declared: `--clock` is a clock run and takes both lanes; a
# `--tree` of `small_lane_gib` or less is a small walk and takes either, the second first so that the
# first stays free for a large one; any other walk is large and takes the first; and a `--long` run
# takes the second only, so that however long it holds, the first is never held by a run whose
# length it chose (ADR 1756). Two lanes are at
# most 12 + 6 GiB of walks, which ADR 1659 section 2 found room for beside everything else the machine
# runs, and only because `--tree` is a kill and not a promise.
#
# `flock` gives a waiting exclusive holder no priority, so a clock run waiting for two lanes would
# wait until two holds happened to end together, while small walks took each lane as it freed. The
# gate is that priority: a clock run takes the gate first and keeps it while it waits for the first
# lane and then the second — always in that order, so no two takers each hold the lane the other
# waits for — and a walk is granted a lane only at a moment it finds the gate free. So while a clock
# run waits, no walk is granted either lane, the ones queued before it included, and the clock run
# waits only for the holds it found running. A walk polls rather than blocks, twice a second, because
# it waits for the gate and a lane at once and `flock` waits for one file.
#
# A long run is the one hold a clock run may not wait for behind the gate: it ends when it chose to,
# an hour on, and the gate would stop the first lane for all of it (ADR 1756). So a clock run takes a
# fourth file first, `<lock>.clock`, the clock turn, which orders the clock runs and stops every long
# run's grant; while the second lane's holder is a long run — `<lock>.lane2.long` names its live
# wrapper — the clock run waits holding the turn and not the gate, and the first lane grants walks;
# then it takes the gate and the lanes as before.
#
# A walk's grant follows its ask (ADR 1790). Pollers raced for a freed lane, so a walk asked later
# could take it first: round 1470's small walk asked at 23:01 queued 4 339.7 s while round 1468's,
# asked eleven minutes after it, queued 490.8 s behind the same holds. So every walk — small, large
# and long — leaves a ticket in a fifth file, `<lock>.queue`, when it asks, one line in ask order:
# its wrapper's pid, that process's start time, the lanes it takes and its holder word. A walk is
# granted lane N only where no live ticket asked before its own takes N; a small walk behind a large
# one keeps the second lane, which the large one does not take. The file is read and written under
# its own `flock`, held for one poll's look and grant and never across a wait, and a ticket whose
# process is gone — or whose pid now names a process started at another time — is dropped by the
# next look. Clock runs take no ticket: the clock turn and the gate order them as before.
#
# `lock_take` sets `lane_fds` (the descriptors this wrapper holds, one per lane), `lanes` (as the log
# line names them), `asked_ms`, `held_ms` and `behind`. A descriptor this process already has open on
# the first lane is a bare `flock` ancestor's — `flock <lock> tools/bounded.sh --lock …` — and is
# locked again rather than a second one opened: `flock` locks an open file description, so a second
# description of the same file would queue behind the caller's own lock for ever, while the
# inherited one is granted at once. The wait is printed when there is one, so a round watching its
# shell knows what it is waiting for, and so is the run it found holding the lock (ADR 1659).
lane2_path=$lock_path.lane2
gate_path=$lock_path.gate
clock_turn_path=$lock_path.clock
queue_path=$lock_path.queue
small_lane_gib=6
locked_threads=4
# How often a walk looks at its lanes. The variable exists for the self-test, whose order case gives
# its earlier asks a slow look so that a lane they are owed is free while a later ask looks first.
poll_interval=${HEAVY_WALK_POLL:-0.5}
lock_waiting=
lock_waiter=
queue_fd=
# How long a look waits for the queue's lock, which every taker holds for one look of a few
# milliseconds. Past it the look is given up and tried at the next poll, so that a wrapper stopped
# inside its look stops the grants and not the waiting process: it stays interruptible, and says so.
queue_patience=2
declare -A ahead_of_me=()

now_ms() { echo $(( $(date +%s%N) / 1000000 )); }

# `lock_holder FILE` prints the run holding FILE's lock as one word: its command line, blanks made
# `_`, or `unknown`. Two kinds of holder need two sources. A bare `flock <lock> <command>` keeps the
# `flock` process alive as the command's parent, and `/proc/locks` names its pid. A `--lock`
# wrapper's lock was taken by a `flock -n <fd>` that has already exited — `/proc/locks` keeps the
# taker's pid, not the holder's — so the wrapper leaves its own line in `<FILE>.holder` while it
# holds, and that file is read when the pid is gone.
lock_holder() {
    local file=$1 inode pid said=
    inode=$(stat -L -c %i -- "$file" 2>/dev/null) || { echo unknown; return; }
    pid=$(awk -v ino="$inode" '$2 == "FLOCK" { n = split($6, at, ":"); if (at[n] == ino) { print $5; exit } }' /proc/locks 2>/dev/null)
    if [ -n "$pid" ] && [ -r "/proc/$pid/cmdline" ]; then
        said=$(tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null)
    fi
    if [ -z "$said" ] && [ -s "$file.holder" ]; then
        read -r pid said < "$file.holder"
        kill -0 "$pid" 2>/dev/null || said=
    fi
    [ -n "$said" ] || said=unknown
    printf '%s' "$said" | tr ' \t\n' '___' | cut -c1-120
}

# `lane_path N` is lane N's file.
lane_path() { if [ "$1" = 1 ]; then echo "$lock_path"; else echo "$lane2_path"; fi; }

# `lock_open_in PID FILE` prints the number of a descriptor PID has open on FILE, and fails where it
# has none.
lock_open_in() {
    local target link
    [ -e "$2" ] || return 1
    target=$(readlink -f -- "$2") || return 1
    for link in /proc/"$1"/fd/*; do
        if [ "$(readlink -- "$link" 2>/dev/null)" = "$target" ]; then
            echo "${link##*/}"
            return 0
        fi
    done
    return 1
}

# `lanes_open_in PID` prints the lanes PID has a descriptor open on, `1`, `2` or `1 2`, and fails
# where it has none.
lanes_open_in() {
    local held=
    lock_open_in "$1" "$lock_path" > /dev/null && held=1
    lock_open_in "$1" "$lane2_path" > /dev/null && held="${held:+$held }2"
    [ -n "$held" ] && echo "$held"
}

# `marked_ancestor_lanes` prints the lanes held by the process `HEAVY_WALK_HELD_BY` names, where that
# process is an ancestor of this one: the subshell of a `--lock` wrapper this process runs under. The
# marker alone is only an inherited word — a daemon the walk started keeps it after the walk — so the
# parent chain is what makes it a hold.
marked_ancestor_lanes() {
    local holder=${HEAVY_WALK_HELD_BY:-} pid=$$
    case "$holder" in ''|*[!0-9]*) return 1 ;; esac
    while [ "$pid" -gt 1 ]; do
        pid=$(awk '$1 == "PPid:" { print $2; exit }' "/proc/$pid/status" 2>/dev/null)
        [ -n "$pid" ] || return 1
        [ "$pid" != "$holder" ] || { lanes_open_in "$pid"; return; }
    done
    return 1
}

# What `--held` answers: this process has a lane's descriptor open — handed down by a bare
# `flock <lock>` — or a marked ancestor holds a lane.
lock_held_here() { lanes_open_in $$ > /dev/null || marked_ancestor_lanes > /dev/null; }

# `lock_queued WHAT HOLDERS` says the wait once, and keeps the holders it found as `behind`.
lock_queued() {
    [ "$behind" = - ] || return 0
    behind=$2
    echo "bounded: queued for the heavy-walk lock ($1) at $(date '+%H:%M:%S'), behind $behind" >&2
}

# `lock_wait FD` takes FD's lock, waiting in the kernel, and stays interruptible while it waits. A
# `flock` in the foreground holds back every trap of this shell until it returns, so a clock run
# stopped while it queued ended only once it was granted, and then on no line. In the background,
# `wait` returns at the signal and the trap runs at once; the lock the background `flock` takes is
# this wrapper's all the same, because a `flock` lock belongs to the open file description, which
# both processes share. The exit trap ends the waiter, which holds a copy of every lane this wrapper
# was granted already.
lock_wait() {
    local status
    flock "$1" & lock_waiter=$!
    wait "$lock_waiter"; status=$?
    lock_waiter=
    return "$status"
}

# `lane_hold N FD` takes lane N on FD, waiting in the kernel: a clock run's way, behind the gate.
lane_hold() {
    flock -n "$2" || { lock_queued "lane $1" "$(lock_holder "$(lane_path "$1")")"; lock_wait "$2"; }
}

# `proc_start PID NAME [STATE]` sets NAME to PID's start time in clock ticks since boot, the 22nd
# field of `/proc/PID/stat`, and STATE to its state letter, the third, and fails where PID is no
# process or a zombie. The fields are counted after the command's closing parenthesis, because the
# command may hold blanks and parentheses of its own. A pid alone names a ticket's process only until
# the kernel hands the number to another one.
proc_start() {
    local stat name=$2 state_name=${3:-}
    read -r stat < "/proc/$1/stat" 2>/dev/null || return 1
    stat=${stat##*) }
    # shellcheck disable=SC2086
    set -- $stat
    [ $# -ge 20 ] && [ "$1" != Z ] || return 1
    printf -v "$name" '%s' "${20}"
    [ -z "$state_name" ] || printf -v "$state_name" '%s' "$1"
}

# `queue_ask` appends this walk's ticket, `<pid> <start> <lanes, comma-joined> <holder word>`, under
# the queue's lock. The word is the one `lock_holder` prints for a holder, so a run queued behind an
# earlier ask names it on its line as it would name the ask once it held the lane.
queue_ask() {
    local start
    proc_start $$ start || return 1
    queue_word=$(printf 'round=%s %s' "$round" "$command_words" | sed 's/[[:space:]]*$//' | tr ' \t\n' '___' | cut -c1-120)
    queue_line="$$ $start ${wanted// /,} $queue_word"
    # A ticket that could not be written now is written by the first look that has the lock.
    flock -w "$queue_patience" "$queue_fd" || return 0
    printf '%s\n' "$queue_line" >> "$queue_path"
    flock -u "$queue_fd"
}

# `queue_look` reads the tickets under the queue's lock, which the caller holds, and sets
# `ahead_of_me[N]` to the word of the first live ticket asked before this walk's that takes lane N.
# A dead ticket is dropped and the file written again in place — its inode is the one every taker
# locks — and this walk's own ticket is written back if it was lost, after every other. A stopped
# process's ticket is kept and holds back nobody while it is stopped: it could not take a lane it
# was owed, and a job suspended in a terminal would otherwise stop every walk behind it.
queue_look() {
    local pid start want word now state kept= mine= dropped= lane
    ahead_of_me=()
    while read -r pid start want word; do
        [ -n "$pid" ] || continue
        if ! proc_start "$pid" now state || [ "$now" != "$start" ]; then dropped=1; continue; fi
        kept+="$pid $start $want $word"$'\n'
        if [ "$pid" = $$ ]; then mine=1; continue; fi
        [ -z "$mine" ] || continue
        case $state in T|t) continue ;; esac
        for lane in ${want//,/ }; do
            [ -n "${ahead_of_me[$lane]:-}" ] || ahead_of_me[$lane]=$word
        done
    done < "$queue_path"
    [ -n "$mine" ] || { kept+="$queue_line"$'\n'; dropped=1; }
    [ -z "$dropped" ] || printf '%s' "$kept" > "$queue_path"
}

# `queue_leave` takes this walk's ticket out, under the queue's lock: at its grant, which holds the
# lock already, and from the exit trap of a walk that stopped waiting.
queue_leave() {
    local pid rest kept=
    [ -n "$queue_fd" ] || return 0
    # Past the patience, the ticket stays: the walk that left it is ending, and the next look drops it.
    flock -w "$queue_patience" "$queue_fd" || return 0
    while read -r pid rest; do
        [ -z "$pid" ] || [ "$pid" = $$ ] || kept+="$pid $rest"$'\n'
    done < "$queue_path"
    printf '%s' "$kept" > "$queue_path"
    flock -u "$queue_fd"
}

# `lock_take` returns at once, taking nothing and writing no line, where a `--lock` wrapper above
# this one holds the lock already: that wrapper's line is the hold, and a second one would count it
# twice. A clock run under an ancestor that holds one lane only would measure beside whatever walk
# took the other, so it is refused, with the cure: the outermost run declares `--clock`. A long run
# under an ancestor that holds the first lane, or under a bare `flock` of it, would keep the first
# lane for the length the long run chose, so it is refused too: the outermost run declares `--long`.
lock_take() {
    local inherited ancestor gate_fd turn_fd fd lane earlier wanted=
    asked_ms=$(now_ms)
    behind=-
    inherited=$(lock_open_in $$ "$lock_path") || inherited=
    if [ -z "$inherited" ] && ancestor=$(marked_ancestor_lanes); then
        if [ "$kind" = clock ] && [ "$ancestor" != "1 2" ]; then
            echo "bounded: a --clock run inside a hold of lane $ancestor only would be timed beside the walk on the other lane; declare the outermost --lock run --clock" >&2
            return 64
        fi
        if [ "$kind" = long ] && [ "$ancestor" != 2 ]; then
            echo "bounded: a --long run inside a hold of lane ${ancestor// / and } would keep the first lane for as long as it runs; declare the outermost --lock run --long" >&2
            return 64
        fi
        return 0
    fi
    if [ -n "$inherited" ] && [ "$kind" = long ]; then
        echo "bounded: a --long run under its caller's flock of the first lane would keep that lane for as long as it runs; run it under tools/bounded.sh --lock --long alone" >&2
        return 64
    fi
    # From here the run waits for a grant, and a wait that ends before one is still a line.
    lock_waiting=1
    if [ -n "$inherited" ]; then
        # The caller's own `flock` holds the first lane; a clock run takes the second after it, the
        # order the gate's holder keeps, and not the gate, which a clock run waiting for the first
        # lane may hold.
        flock "$inherited" || return 1
        lane_fds=$inherited lanes=1
        if [ "$kind" = clock ]; then
            exec {fd}>>"$lane2_path" || return 1
            lane_hold 2 "$fd" || return 1
            lane_fds="$lane_fds $fd" lanes=1+2
        fi
    elif [ "$kind" = clock ]; then
        # The clock turn first: it orders the clock runs among themselves and stops every long run's
        # grant, so that no long run takes the second lane while this run waits for it.
        exec {turn_fd}>>"$clock_turn_path" || return 1
        flock -n "$turn_fd" || { lock_queued "the clock turn, behind an earlier clock run" "$(lock_holder "$clock_turn_path")"; lock_wait "$turn_fd" || return 1; }
        printf '%s round=%s %s\n' "$$" "$round" "$command_words" > "$clock_turn_path.holder" 2>/dev/null
        # A long run on the second lane holds it for as long as it chose, and waiting for it behind the
        # gate would stop the first lane for as long too. So the gate is taken once the second lane is
        # no long run's, and until then the first keeps granting walks.
        while long_holds_lane2; do
            lock_queued "lane 2, held by a long run; the first lane grants walks meanwhile" "$(lock_holder "$lane2_path")"
            sleep "$poll_interval"
        done
        exec {gate_fd}>>"$gate_path" || return 1
        flock -n "$gate_fd" || { lock_queued "the gate, behind an earlier clock run" "$(lock_holder "$gate_path")"; lock_wait "$gate_fd" || return 1; }
        printf '%s round=%s %s\n' "$$" "$round" "$command_words" > "$gate_path.holder" 2>/dev/null
        for lane in 1 2; do
            exec {fd}>>"$(lane_path "$lane")" || return 1
            lane_hold "$lane" "$fd" || return 1
            lane_fds="${lane_fds:+$lane_fds }$fd"
        done
        lanes=1+2
        for fd in "$gate_path" "$clock_turn_path"; do
            [ "$(cut -d' ' -f1 "$fd.holder" 2>/dev/null)" != "$$" ] || rm -f -- "$fd.holder"
        done
        exec {gate_fd}>&- {turn_fd}>&-
    else
        exec {gate_fd}>>"$gate_path" || return 1
        case $kind in small) wanted="2 1" ;; long) wanted=2 ;; *) wanted=1 ;; esac
        [ "$kind" != long ] || exec {turn_fd}>>"$clock_turn_path" || return 1
        declare -A lane_fd=()
        for lane in $wanted; do exec {fd}>>"$(lane_path "$lane")" || return 1; lane_fd[$lane]=$fd; done
        exec {queue_fd}>>"$queue_path" || return 1
        queue_ask || return 1
        while [ -z "$lanes" ]; do
            if [ "$kind" = long ] && ! { flock -n "$turn_fd" && flock -u "$turn_fd"; }; then
                lock_queued "a clock run waits for the second lane, and no long run is granted it meanwhile" "$(lock_holder "$clock_turn_path")"
            elif flock -n "$gate_fd"; then
                flock -u "$gate_fd"
                # One look and at most one grant under the queue's lock, so that no two walks each
                # read themselves first for the same lane; no grant without it.
                fd= earlier=
                if ! flock -w "$queue_patience" "$queue_fd"; then
                    lock_queued "the queue's lock, held past ${queue_patience}s by a look that has not ended" "$(lock_holder "$queue_path")"
                    continue
                fi
                queue_look
                for lane in $wanted; do
                    if [ -n "${ahead_of_me[$lane]:-}" ]; then
                        fd="${fd:+$fd+}${ahead_of_me[$lane]}" earlier=1
                    elif flock -n "${lane_fd[$lane]}"; then
                        lanes=$lane
                        queue_leave
                        break
                    else
                        fd="${fd:+$fd+}$(lock_holder "$(lane_path "$lane")")"
                    fi
                done
                flock -u "$queue_fd"
                [ -n "$lanes" ] && break
                lock_queued "lane ${wanted// / or }${long:+, the only lane a long run takes}${earlier:+, behind an earlier ask}" "$fd"
            else
                lock_queued "a clock run holds the gate while it waits for both lanes" "$(lock_holder "$gate_path")"
            fi
            sleep "$poll_interval"
        done
        for lane in $wanted; do
            if [ "$lane" = "$lanes" ]; then lane_fds=${lane_fd[$lane]}; else eval "exec ${lane_fd[$lane]}>&-"; fi
        done
        exec {gate_fd}>&- {queue_fd}>&-
        queue_fd=
        [ "$kind" != long ] || exec {turn_fd}>&-
    fi
    held_ms=$(now_ms)
    for lane in ${lanes//+/ }; do
        printf '%s round=%s %s\n' "$$" "$round" "$command_words" > "$(lane_path "$lane").holder" 2>/dev/null
    done
    [ "$kind" != long ] || echo "$$" > "$lane2_path.long" 2>/dev/null
}

# Whether the second lane is held by a long run: the pid the long run's wrapper leaves in
# `<lock>.lane2.long` while it holds the lane is alive. A wrapper killed before it could remove the
# file leaves a pid that is not, and that is no hold.
long_holds_lane2() {
    local pid
    [ -s "$lane2_path.long" ] && read -r pid < "$lane2_path.long" 2>/dev/null && kill -0 "$pid" 2>/dev/null
}

# `lock_record STATUS` appends the run's one line, if it held the lock or waited for it. Shorter than
# `PIPE_BUF` and written with `O_APPEND`, so two wrappers cannot interleave one. `kind=` is what the run
# declared and `lane=` what it was granted, so that ADR 1671's re-ask can replay a batch's lines under
# either rule. **A wait that ended before a grant is a line too** (ADR 1790): a run stopped while it
# queued — a `timeout`, a round that gave up — paid its queue and held nothing, and on no line its
# queue was nobody's figure; round 1469 stopped a small walk after about ten minutes and round 1472's
# check died at its `timeout` behind the first lane, and neither is on the log. Such a line says
# `hold=0.0s`, the wrapper's own status and `lane=-`. A `SIGKILL` leaves no line, and its ticket is
# dropped by the next walk's look.
lock_record() {
    local ended_ms lane holder
    if [ -n "$lock_waiter" ]; then kill "$lock_waiter" 2>/dev/null; wait "$lock_waiter" 2>/dev/null; lock_waiter=; fi
    if [ -z "${held_ms:-}" ]; then
        [ -n "$lock_waiting" ] || return 0
        queue_leave
        for holder in "$gate_path.holder" "$clock_turn_path.holder"; do
            [ "$(cut -d' ' -f1 "$holder" 2>/dev/null)" != "$$" ] || rm -f -- "$holder"
        done
        held_ms=$(now_ms) lanes=-
    fi
    ended_ms=$(now_ms)
    printf '%s batch=%s round=%s wait=%ss hold=%ss exit=%s peak=%sGiB behind=%s kind=%s lane=%s cmd=%s\n' \
        "$(date -d "@$(( asked_ms / 1000 ))" '+%Y-%m-%dT%H:%M:%S')" "$batch" "$round" \
        "$(seconds $(( held_ms - asked_ms )))" "$(seconds $(( ended_ms - held_ms )))" "$1" \
        "$(awk -v k="${peak_kib:-0}" 'BEGIN { printf "%.2f", k / 1048576 }')" "${behind:--}" \
        "$kind" "$lanes" "$command_words" >> "$lock_log" ||
        echo "bounded: the lock was held or waited for, and its line could not be appended to $lock_log" >&2
    [ "$lanes" != - ] || return 0
    for lane in ${lanes//+/ }; do
        holder=$(lane_path "$lane").holder
        [ "$(cut -d' ' -f1 "$holder" 2>/dev/null)" != "$$" ] || rm -f -- "$holder"
    done
    [ "$kind" != long ] || [ "$(cat "$lane2_path.long" 2>/dev/null)" != "$$" ] || rm -f -- "$lane2_path.long"
}
seconds() { awk -v ms="$1" 'BEGIN { printf "%.1f", ms / 1000 }'; }

[ -z "$held_query" ] || { lock_held_here; exit; }

# ---------------------------------------------------------------------------------------------
# The self-test: each case prints one line, and the script exits 1 on the first that fails.
if [ -n "$self_test" ]; then
    scratch=$(mktemp -d "${TMPDIR:-/tmp}/bounded-self-test.XXXXXX") || exit 1
    trap 'rm -rf "$scratch"' EXIT
    fail() { echo "bounded --self-test: FAILED — $*" >&2; exit 1; }
    self=${BASH_SOURCE[0]}
    # **Every hand-off between a case's runs waits for an event, never for a time.** A holder holds
    # until the case releases it, and the case releases it once the run it is about has said what it
    # was asked to say; the bound on each wait is a minute, and it is only the bound. Holds of two and
    # five seconds that an idle machine always outlasted failed under the merge's whole-workspace test
    # run at a load of 16, and failed every time with six busy loops on the two processors the test
    # ran on (ADR 1710). `appears FILE PATTERN WHY` waits for FILE to exist, or with a PATTERN for a
    # line of it to match; `hold_until` is the command of a holder, which ends once `<its file>.go`
    # exists.
    appears() {
        local _
        for _ in $(seq 600); do
            if [ -z "$2" ]; then [ -e "$1" ] && return 0; else grep -q -- "$2" "$1" 2>/dev/null && return 0; fi
            sleep 0.1
        done
        fail "$3"
    }
    hold_until='for _ in $(seq 600); do [ -e "$0.go" ] && exit 0; sleep 0.1; done; exit 9'
    # A run that found its lane free never polls, and one that queued sleeps at least one poll, so
    # under a poll's length is the discriminating figure for "it waited for nothing" — not 0.0 s,
    # which is the time three `flock` calls take on an idle machine only.
    waited_nothing() { awk -v w="$1" -v p="$poll_interval" 'BEGIN { exit !(w < p) }'; }

    # 1. A flat tree of 100 000 children under the root, beside 500 strangers: the total is the
    #    root's 100 plus 100 000 tens, every child is listed once, and the whole walk costs well
    #    under the sampler's interval. The quadratic walk this replaced needed minutes here. The cost
    #    is the walk's processor time, which is the algorithm's; its wall time on a loaded machine is
    #    the load's, and is printed beside it.
    awk 'BEGIN { print 1000, 1, 100; for (i = 1; i <= 100000; i++) print 1000 + i, 1000, 10
                 for (i = 1; i <= 500; i++) print 200000 + i, 1, 5 }' > "$scratch/flat"
    started_ns=$(date +%s%N)
    cpu=$( { TIMEFORMAT='%3U %3S'; time walk_table 1000 < "$scratch/flat" > "$scratch/flat.out"; } 2>&1 )
    wall_ms=$(( ($(date +%s%N) - started_ns) / 1000000 ))
    cost_ms=$(awk -v t="$cpu" 'BEGIN { split(t, f, " "); printf "%d", (f[1] + f[2]) * 1000 }')
    [ "$(head -n 1 "$scratch/flat.out")" = 1000100 ] || fail "flat tree: total $(head -n 1 "$scratch/flat.out"), wanted 1000100"
    [ "$(tail -n +2 "$scratch/flat.out" | wc -l)" = 100000 ] || fail "flat tree: $(tail -n +2 "$scratch/flat.out" | wc -l) descendants listed, wanted 100000"
    [ "$cost_ms" -lt 1000 ] || fail "flat tree: one sample cost ${cost_ms} ms, which is not a fraction of the interval it has to fit"
    echo "bounded --self-test: a flat tree of 100000 sampled correctly in ${cost_ms} ms of processor time (${wall_ms} ms of wall)"

    # 2. A chain 50 000 deep, and a table holding a cycle and a duplicated row: the walk ends,
    #    and every pid counts once.
    awk 'BEGIN { print 1000, 1, 1; for (i = 1; i <= 50000; i++) print 1000 + i, 1000 + i - 1, 1 }' > "$scratch/chain"
    [ "$(walk_table 1000 < "$scratch/chain" | head -n 1)" = 50001 ] || fail "chain: total $(walk_table 1000 < "$scratch/chain" | head -n 1), wanted 50001"
    printf '1000 1 1\n1001 1000 2\n1002 1001 4\n1001 1002 2\n1002 1001 4\n' > "$scratch/cycle"
    [ "$(walk_table 1000 < "$scratch/cycle" | head -n 1)" = 7 ] || fail "cycle: total $(walk_table 1000 < "$scratch/cycle" | head -n 1), wanted 7"
    [ "$(walk_table 1000 < "$scratch/cycle" | tail -n +2 | sort | tr '\n' ' ')" = "1001 1002 " ] || fail "cycle: descendants $(walk_table 1000 < "$scratch/cycle" | tail -n +2 | tr '\n' ' ')"
    echo "bounded --self-test: a chain of 50000, a cycle and a duplicate walked once each"

    # 3. A live tree that fans out into two hundred children under the wrapper itself:
    #    exit 0, and the peak is a positive figure.
    #
    #    **The children live for three seconds and not for a third of one**, and the
    #    difference is the whole case. The sampler ticks once a second, so a tree that is
    #    gone before the first tick leaves a peak of 0.00 and this case fails for a reason
    #    that is the machine's speed rather than the wrapper's behaviour — which is what it
    #    did on every CI runner from the session that wrote it until the ninth of September,
    #    while passing on the machine it was written on. A self-test whose verdict depends
    #    on losing a race is not a self-test.
    #    The ceiling is eight gibibytes and not one: two hundred shells that are alive
    #    when the sampler walks them cost 1.22 GiB of resident memory here, so the case as
    #    first written would now be killed by its own bound — which is the sampler working
    #    and the case's ceiling being a figure nobody had measured.
    "$self" --tree 8 --data 1 --nice 0 -- bash -c 'for i in $(seq 200); do sleep 3 & done; wait' \
        > "$scratch/fan.out" 2> "$scratch/fan.err"
    status=$?
    [ "$status" -eq 0 ] || fail "fan-out: exit $status: $(tail -n 1 "$scratch/fan.err")"
    grep -q 'bounded: exit 0 after [0-9]*s; peak [0-9]*\.[0-9]* GiB resident' "$scratch/fan.err" || fail "fan-out: $(tail -n 1 "$scratch/fan.err")"
    grep -q 'peak 0\.00 GiB' "$scratch/fan.err" && fail "fan-out: the peak is 0.00 GiB, so the sampler saw nothing: $(tail -n 1 "$scratch/fan.err")"
    echo "bounded --self-test: a live tree of 200 children: $(tail -n 1 "$scratch/fan.err" | sed 's/^bounded: //')"

    # 4. A child that holds 1.5 GiB resident under a ceiling of 1 GiB is killed by the ceiling —
    #    exit 137 and the line that names it — and not by the data limit, which is above it.
    if command -v python3 > /dev/null; then
        "$self" --tree 1 --data 3 --nice 0 -- env PYTHONDONTWRITEBYTECODE=1 python3 -c 'import time; b = b"x" * (1536 << 20); time.sleep(20)' \
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
    #    The stalled samplers and the leader's child outlive the case on purpose — a sampler stuck
    #    in the kernel is abandoned, never waited for — so the case runs in a subshell whose
    #    standard output and error are set with `exec` and its verdict comes back in a file. A
    #    redirection on the call would leave the outer descriptors saved in every forked sampler,
    #    and a caller reading this script through a pipe, as `cargo test` does, would wait out
    #    their sleeps, a minute after the last case had printed.
    (
        exec > /dev/null 2> "$scratch/blind.err" < /dev/null
        process_table() { sleep 60; }
        sample_deadline=1; blind_limit=3
        (
            exec 3>&1
            sleep 30 2>&1 1>&3 &
            wait $!
        ) &
        leader=$!
        started=$(date +%s)
        watch_tree "$leader" $(( 1024 * 1024 ))
        alive=$(kill -0 "$leader" 2>/dev/null && echo alive)
        echo "${blind:-sighted} $(( $(date +%s) - started )) ${alive:-stopped}" > "$scratch/blind.verdict"
    )
    read -r blind elapsed alive < "$scratch/blind.verdict" || fail "blind: the case left no verdict"
    [ "$blind" != sighted ] || fail "blind: the watch returned without going blind"
    [ "$alive" = stopped ] || fail "blind: the leader is still running after the watch stopped it"
    # Under the leader's own thirty seconds, which is when a watch that stopped nothing would return:
    # the three misses take about six seconds idle and twice that on a loaded machine.
    [ "$elapsed" -lt 25 ] || fail "blind: took ${elapsed}s to stop a tree whose sampler stalled"
    [ "$(grep -c 'did not return within' "$scratch/blind.err")" = 3 ] || fail "blind: $(cat "$scratch/blind.err")"
    echo "bounded --self-test: a stalled sampler stopped the tree in ${elapsed}s after 3 missed samples"

    # 6. A fork loop under a task limit of 64: the limit counts the user's tasks, which already
    #    number more than 64 on a working machine and fewer on a fresh runner, so the loop is
    #    refused at its first fork or within its first 64 — and it is bounded by its own count of
    #    128 as well, so a wrapper that failed to apply the limit costs 128 sleeping children for
    #    two seconds rather than a fork bomb. Exit 75 is the loop's own word that a fork was
    #    refused; the wrapper's last line must name the task limit. Root is exempt from
    #    RLIMIT_NPROC, so there the case cannot be run and says so.
    if [ "$(id -u)" = 0 ]; then
        echo "bounded --self-test: NOT RUN — root is exempt from RLIMIT_NPROC, so the task-limit case cannot be seen to hold" >&2
    elif command -v python3 > /dev/null; then
        "$self" --tasks 64 --data 1 --nice 0 -- env PYTHONDONTWRITEBYTECODE=1 python3 -c '
import os, sys, time
born = 0
try:
    for _ in range(128):
        if os.fork() == 0:
            time.sleep(2)
            os._exit(0)
        born += 1
except BlockingIOError as refusal:
    print(f"fork refused after {born} children: {refusal}", file=sys.stderr)
    sys.exit(75)
print(f"all {born} forks succeeded", file=sys.stderr)
' > "$scratch/tasks.out" 2> "$scratch/tasks.err"
        status=$?
        [ "$status" -eq 75 ] || fail "task limit: exit $status, wanted 75 (a refused fork): $(tail -n 2 "$scratch/tasks.err" | tr '\n' ' ')"
        grep -q 'STOPPED BY THE TASK LIMIT' "$scratch/tasks.err" || fail "task limit: $(tail -n 1 "$scratch/tasks.err")"
        echo "bounded --self-test: a fork loop under --tasks 64 was refused: $(grep -o 'fork refused after [0-9]* children' "$scratch/tasks.err")"
    else
        echo "bounded --self-test: NOT RUN — the task-limit case wants python3 for its fork loop, and there is none" >&2
    fi

    # 7. The lock, on a file of the case's own. A holder keeps it for two seconds: a `--lock` run
    #    queues behind it, passes its command's status through, and writes one line whose wait is
    #    at least a second — the calibration, since a wrapper that did not take the lock reads 0.0.
    #    A second run finds it free and waits nothing; a third runs under `flock` on the same file
    #    and must finish rather than queue behind its own caller; and the lock is free afterwards.
    #    The queued line names its holder: the bare `flock`'s command from `/proc/locks`, and then a
    #    `--lock` holder's round from the file it leaves, the source `/proc/locks` cannot be.
    #    The holder releases a second after the run has said it queued, so the run's wait is at least
    #    that second however long it took to start.
    lock_case() { HEAVY_WALK_LOCK="$scratch/lock" HEAVY_WALK_LOG="$scratch/lock.log" "$self" "$@"; }
    ( flock "$scratch/lock" sh -c ': > "$0"; '"$hold_until" "$scratch/held" ) &
    holder=$!
    appears "$scratch/held" "" "lock: the case's own holder never took its lock"
    lock_case --lock --round 7 --tree 12 --data 1 --nice 0 -- sh -c 'exit 3' > /dev/null 2> "$scratch/lock.err" &
    queued=$!
    appears "$scratch/lock.err" 'queued for the heavy-walk lock' "lock: the run behind the holder never said it queued"
    sleep 1
    : > "$scratch/held.go"
    wait "$queued"
    status=$?
    wait "$holder"
    [ "$status" -eq 3 ] || fail "lock: exit $status, wanted the command's 3: $(tail -n 1 "$scratch/lock.err")"
    grep -q 'queued for the heavy-walk lock' "$scratch/lock.err" || fail "lock: the wait was not said: $(cat "$scratch/lock.err")"
    lock_case --lock --round 7 --tree 12 --data 1 --nice 0 -- true > /dev/null 2>&1 || fail "lock: a free lock's run failed"
    timeout 20 flock "$scratch/lock" env HEAVY_WALK_LOCK="$scratch/lock" HEAVY_WALK_LOG="$scratch/lock.log" \
        "$self" --lock --round 7 --tree 12 --data 1 --nice 0 -- true > /dev/null 2>&1 ||
        fail "lock: a run under its caller's flock did not finish (exit $?): it queued behind its own caller"
    flock -n "$scratch/lock" true || fail "lock: the lock is still held after every run ended"
    [ "$(wc -l < "$scratch/lock.log")" = 3 ] || fail "lock: $(wc -l < "$scratch/lock.log") lines logged, wanted 3: $(cat "$scratch/lock.log")"
    first_wait=$(sed -n '1s/.* wait=\([0-9.]*\)s .*/\1/p' "$scratch/lock.log")
    awk -v w="$first_wait" 'BEGIN { exit !(w >= 1.0) }' || fail "lock: the queued run logged wait=${first_wait}s, wanted at least a second: $(head -n 1 "$scratch/lock.log")"
    grep -q '^[0-9T:-]* batch=[^ ]* round=7 wait=[0-9.]*s hold=[0-9.]*s exit=3 peak=[0-9.]*GiB behind=flock_[^ ]* kind=large lane=1 cmd=sh -c exit 3 *$' "$scratch/lock.log" ||
        fail "lock: the line is not the shape the header states: $(head -n 1 "$scratch/lock.log")"
    for line in 2 3; do
        waited_nothing "$(sed -n "${line}s/.* wait=\([0-9.]*\)s .*/\1/p" "$scratch/lock.log")" ||
            fail "lock: run $line found the lock free or its caller's and still waited: $(sed -n "${line}p" "$scratch/lock.log")"
    done
    HEAVY_WALK_LOCK="$scratch/lock" HEAVY_WALK_LOG="$scratch/held.log" \
        "$self" --lock --round 77 --tree 12 --data 1 --nice 0 -- sh -c "$hold_until" "$scratch/held77" > /dev/null 2>&1 &
    holder=$!
    appears "$scratch/lock.holder" . "lock: the --lock holder never wrote its file"
    lock_case --lock --round 7 --tree 12 --data 1 --nice 0 -- true > /dev/null 2> "$scratch/lock77.err" &
    queued=$!
    appears "$scratch/lock77.err" 'queued for the heavy-walk lock' "lock: the run behind a --lock holder never said it queued"
    : > "$scratch/held77.go"
    wait "$queued"
    wait "$holder"
    tail -n 1 "$scratch/lock.log" | grep -q ' behind=[^ ]*round=77_sh_-c' ||
        fail "lock: a run queued behind a --lock holder did not name its round: $(tail -n 1 "$scratch/lock.log")"
    [ -e "$scratch/lock.holder" ] && fail "lock: the holder's file outlived the holder: $(cat "$scratch/lock.holder")"
    echo "bounded --self-test: a run queued ${first_wait}s behind a holder, one found the lock free and one its caller's, each logged, each holder named"

    # 8. The lock stays with the wrapper (ADR 1674). A daemon planted inside a hold — a `sleep` the
    #    command starts and leaves running, as a build leaves `sccache`'s server — must not keep the
    #    lock once the wrapper has ended; the command must not have the lock's descriptor; `--held`
    #    must answer yes inside the hold, through the marker, and no outside it and in the daemon
    #    once the hold is over; and a `--lock` run nested inside a hold must run under it rather
    #    than queue behind its own ancestor, writing no line of its own.
    held_case() { HEAVY_WALK_LOCK="$scratch/lock" HEAVY_WALK_LOG="$scratch/held8.log" "$self" "$@"; }
    held_case --held && fail "held: --held said yes outside any hold"
    held_case --lock --round 8 --tree 1 --data 1 --nice 0 -- sh -c '
        "$0" --held && echo yes > "$1.inside"
        ls -l /proc/$$/fd | grep -q "/lock\(\.lane2\)\{0,1\}\$" && echo yes > "$1.descriptor"
        (for _ in $(seq 200); do [ -e "$1.over" ] && break; sleep 0.1; done
         "$0" --held && echo yes > "$1.daemon"; : > "$1.asked"; exec sleep 30) > /dev/null 2>&1 < /dev/null &
        echo $! > "$1.pid"
        timeout 20 "$0" --lock --round 88 --tree 1 --data 1 --nice 0 -- true > /dev/null 2>&1 && echo yes > "$1.nested"
        exit 0
    ' "$self" "$scratch/held8" > /dev/null 2>&1 || fail "held: the hold's own run failed"
    daemon=$(cat "$scratch/held8.pid" 2>/dev/null)
    flock -n "$scratch/lock" true && flock -n "$scratch/lock.lane2" true; free=$?
    : > "$scratch/held8.over"
    appears "$scratch/held8.asked" "" "held: the daemon never asked --held"
    [ -n "$daemon" ] && kill "$daemon" 2>/dev/null
    [ "$free" -eq 0 ] || fail "held: a sleep the command left running kept the lock after the wrapper ended"
    [ -e "$scratch/held8.inside" ] || fail "held: --held said no inside a hold, so the marker is not read"
    [ -e "$scratch/held8.descriptor" ] && fail "held: the command was handed the lock's descriptor"
    [ -e "$scratch/held8.daemon" ] && fail "held: --held said yes in a daemon after the hold had ended"
    [ -e "$scratch/held8.nested" ] || fail "held: a --lock run inside a hold did not finish: it queued behind its own ancestor"
    [ "$(wc -l < "$scratch/held8.log")" = 1 ] || fail "held: $(wc -l < "$scratch/held8.log") lines logged for one hold and one nested run, wanted 1"
    echo "bounded --self-test: a daemon left by the command did not keep the lock; --held read the marker inside, no outside and none in the daemon; a nested --lock ran under its ancestor"

    # 9. The two lanes and the gate (ADR 1684), on files of the case's own. A large walk holds the
    #    first lane for five seconds, and a small walk asked beside it is granted the second at once
    #    for two. A clock run asked next waits for both — the calibration of the gate is the next
    #    run: a small walk asked while the clock run waits must not be granted the second lane when
    #    the first small walk frees it, but only once the clock run has ended. The clock run's own
    #    command finds both lanes held; each line names its kind, its lane and, for a queued run,
    #    whom it queued behind. A `--clock` with no `--lock`, and one inside a hold of a single lane,
    #    are refused. The two walks hold until the clock run and the walk behind it have each said they
    #    queued; then the small walk is released a second before the large one, which is the window a
    #    gate that did not hold would hand the second lane to the late walk in.
    lane_case() { HEAVY_WALK_LOCK="$scratch/lane" HEAVY_WALK_LOG="$scratch/lane.log" "$self" "$@"; }
    lane_case --lock --round 91 --tree 12 --data 1 --nice 0 -- sh -c "$hold_until" "$scratch/lane91" > /dev/null 2>&1 &
    large=$!
    appears "$scratch/lane.holder" . "lanes: the large walk never took the first lane"
    lane_case --lock --round 92 --tree 1 --data 1 --nice 0 -- sh -c "$hold_until" "$scratch/lane92" > /dev/null 2>&1 &
    small=$!
    appears "$scratch/lane.lane2.holder" . "lanes: the small walk never took the second lane"
    lane_case --lock --clock --round 93 --tree 1 --data 1 --nice 0 -- sh -c '
        flock -n "$1" true || echo held > "$0.1"; flock -n "$1.lane2" true || echo held > "$0.2"
        sleep 1; date +%s%N > "$0.end"' "$scratch/clock" "$scratch/lane" > /dev/null 2> "$scratch/clock.err" &
    timed=$!
    appears "$scratch/clock.err" 'queued for the heavy-walk lock' "lanes: the clock run never queued for the lanes"
    lane_case --lock --round 94 --tree 1 --data 1 --nice 0 -- sh -c 'date +%s%N > "$0"' "$scratch/late" > /dev/null 2> "$scratch/late.err" &
    late=$!
    appears "$scratch/late.err" 'a clock run holds the gate' "lanes: the walk asked behind a waiting clock run never queued at the gate"
    : > "$scratch/lane92.go"
    sleep 1
    : > "$scratch/lane91.go"
    wait "$large" "$small" "$timed" "$late"
    lane_line() { grep " round=$1 " "$scratch/lane.log"; }
    lane_line 91 | grep -q ' kind=large lane=1 ' || fail "lanes: the large walk's line: $(lane_line 91)"
    lane_line 92 | grep -q ' kind=small lane=2 ' && waited_nothing "$(lane_line 92 | sed 's/.* wait=\([0-9.]*\)s .*/\1/')" ||
        fail "lanes: a small walk beside a large one was not granted the second lane at once: $(lane_line 92)"
    lane_line 93 | grep -q ' behind=[^ ]*round=91[^ ]* kind=clock lane=1+2 ' || fail "lanes: the clock run's line: $(lane_line 93)"
    clock_end=$(cat "$scratch/clock.end" 2>/dev/null) late_start=$(cat "$scratch/late" 2>/dev/null)
    [ -n "$clock_end" ] && [ -n "$late_start" ] && [ "$late_start" -ge "$clock_end" ] ||
        fail "lanes: a small walk asked while a clock run waited was granted a lane before the clock run ended (${late_start:-never} against ${clock_end:-never} ns)"
    lane_line 94 | grep -q ' behind=[^ ]*round=93[^ ]* kind=small lane=[12] ' || fail "lanes: the walk asked behind a waiting clock run does not name it: $(lane_line 94)"
    [ -e "$scratch/clock.1" ] && [ -e "$scratch/clock.2" ] || fail "lanes: the clock run's command found a lane free under it"
    lane_case --clock --tree 1 --data 1 --nice 0 -- true > /dev/null 2>&1; status=$?
    [ "$status" -eq 64 ] || fail "lanes: --clock without --lock exited $status, wanted 64"
    lane_case --lock --round 95 --tree 1 --data 1 --nice 0 -- "$self" --lock --clock --round 96 --data 1 --nice 0 -- true > /dev/null 2>&1; status=$?
    [ "$status" -eq 64 ] || fail "lanes: a --clock run inside a hold of one lane exited $status, wanted 64"
    flock -n "$scratch/lane" true && flock -n "$scratch/lane.lane2" true && flock -n "$scratch/lane.gate" true ||
        fail "lanes: a lane or the gate is still held after every run ended"
    echo "bounded --self-test: a small walk ran beside a large one; a clock run waited $(lane_line 93 | sed 's/.* wait=\([0-9.]*s\) .*/\1/') for both lanes and the walk asked behind it ran after it; --clock alone and inside one lane refused"

    # 10. A build inside the hold (ADR 1710), with a `cargo` of the case's own first on the path: it
    #     writes its arguments, whether `--held` says yes and whether it was handed a lane's
    #     descriptor. Two builds run in their order, under the hold and without a descriptor, before
    #     the command, which sees both; the line's `cmd=` is the command's. A build that fails ends
    #     the run with its status, and its command never starts.
    mkdir -p "$scratch/bin"
    printf '%s\n' '#!/usr/bin/env bash' \
        '[ "$1" = build ] || exit 64; shift' 'echo "build $*" >> "$BUILD_CASE_LOG"' \
        '"$BUILD_CASE_SELF" --held && echo "held $*" >> "$BUILD_CASE_LOG"' \
        'ls -l /proc/$$/fd | grep -q "/build\(\.lane2\)\{0,1\}\$" && echo "descriptor $*" >> "$BUILD_CASE_LOG"' \
        'case "$*" in *fails*) exit 101 ;; esac' > "$scratch/bin/cargo"
    chmod +x "$scratch/bin/cargo"
    build_case() {
        PATH="$scratch/bin:$PATH" BUILD_CASE_LOG="$scratch/build.calls" BUILD_CASE_SELF="$self" \
            HEAVY_WALK_LOCK="$scratch/build" HEAVY_WALK_LOG="$scratch/build.log" "$self" "$@"
    }
    build_case --lock --round 101 --tree 1 --data 1 --nice 0 --build '--profile gates -p first --bins' --build '-p second' \
        -- sh -c 'cp "$0" "$0.seen"' "$scratch/build.calls" > /dev/null 2> "$scratch/build.err" ||
        fail "build: a run whose builds succeeded exited $?: $(tail -n 3 "$scratch/build.err")"
    [ "$(cat "$scratch/build.calls.seen" 2>/dev/null)" = "$(printf '%s\n' 'build --profile gates -p first --bins' 'held --profile gates -p first --bins' 'build -p second' 'held -p second')" ] ||
        fail "build: the builds did not run in order, under the hold and without a lane, before the command: $(cat "$scratch/build.calls" 2>/dev/null)"
    grep -q ' round=101 .* exit=0 .* cmd=sh -c cp ' "$scratch/build.log" || fail "build: the line does not name the command: $(cat "$scratch/build.log")"
    build_case --lock --round 102 --tree 1 --data 1 --nice 0 --build 'fails' -- touch "$scratch/build.ran" > /dev/null 2> "$scratch/build.err"
    status=$?
    [ "$status" -eq 101 ] || fail "build: a failed build's run exited $status, wanted the build's 101"
    [ -e "$scratch/build.ran" ] && fail "build: the command ran after its build failed"
    grep -q 'failed (exit 101) inside the hold, so the command was not started' "$scratch/build.err" || fail "build: the refusal was not said: $(tail -n 2 "$scratch/build.err")"
    grep -q ' round=102 .* exit=101 ' "$scratch/build.log" || fail "build: the failed run's line: $(cat "$scratch/build.log")"
    echo "bounded --self-test: two builds ran in order inside the hold, without a lane, before their command; a failed build ended its run with exit 101 and the command unstarted"

    # 11. A long run is pinned to the second lane (ADR 1756), on files of the case's own. A long run
    #     takes the second lane; a second long run asked beside it finds the first lane free and must
    #     not take it, but queue behind the first long run; a small walk asked while it queues is
    #     granted the first lane at once and ends before either long run is released. Calibrated by
    #     the first lane itself: while the second long run queues, the case takes the first lane with
    #     `flock -n`, which fails if that run took it — the defect this kind exists to prevent, and
    #     what the run did when it was a small walk. Each line names `kind=long lane=2`. `--long`
    #     with no `--lock`, beside `--clock`, above the second lane's 6 GiB, inside a hold of the
    #     first lane and under a bare `flock` of it are refused.
    long_case() { HEAVY_WALK_LOCK="$scratch/long" HEAVY_WALK_LOG="$scratch/long.log" "$self" "$@"; }
    long_case --lock --long --round 111 --data 1 --nice 0 -- sh -c "$hold_until" "$scratch/long111" > /dev/null 2>&1 &
    first_long=$!
    appears "$scratch/long.lane2.holder" . "long: the first long run never took the second lane"
    long_case --lock --long --round 112 --data 1 --nice 0 -- sh -c 'date +%s%N > "$0"' "$scratch/long112" > /dev/null 2> "$scratch/long112.err" &
    second_long=$!
    appears "$scratch/long112.err" 'queued for the heavy-walk lock (lane 2' "long: the second long run never queued for the second lane, so it took the first"
    flock -n "$scratch/long" true || fail "long: the second long run took the first lane while the second was held"
    long_case --lock --round 113 --tree 1 --data 1 --nice 0 -- true > /dev/null 2>&1 || fail "long: the small walk beside two long runs failed"
    [ -e "$scratch/long112" ] && fail "long: the second long run ran before the first was released"
    : > "$scratch/long111.go"
    wait "$first_long" "$second_long"
    long_line() { grep " round=$1 " "$scratch/long.log"; }
    long_line 111 | grep -q ' kind=long lane=2 ' || fail "long: the first long run's line: $(long_line 111)"
    long_line 112 | grep -q ' behind=[^ ]*round=111[^ ]* kind=long lane=2 ' || fail "long: the second long run's line does not name the first: $(long_line 112)"
    long_line 113 | grep -q ' kind=small lane=1 ' && waited_nothing "$(long_line 113 | sed 's/.* wait=\([0-9.]*\)s .*/\1/')" ||
        fail "long: a small walk beside a long run was not granted the first lane at once: $(long_line 113)"
    long_case --long --data 1 --nice 0 -- true > /dev/null 2>&1; status=$?
    [ "$status" -eq 64 ] || fail "long: --long without --lock exited $status, wanted 64"
    long_case --lock --long --clock --data 1 --nice 0 -- true > /dev/null 2>&1; status=$?
    [ "$status" -eq 64 ] || fail "long: --long beside --clock exited $status, wanted 64"
    long_case --lock --long --tree 12 --data 1 --nice 0 -- true > /dev/null 2>&1; status=$?
    [ "$status" -eq 64 ] || fail "long: --long --tree 12 exited $status, wanted 64"
    long_case --lock --round 114 --tree 12 --data 1 --nice 0 -- "$self" --lock --long --round 115 --data 1 --nice 0 -- true > /dev/null 2>&1; status=$?
    [ "$status" -eq 64 ] || fail "long: a --long run inside a hold of the first lane exited $status, wanted 64"
    long_case --lock --round 116 --tree 1 --data 1 --nice 0 -- "$self" --lock --long --round 117 --data 1 --nice 0 -- true > /dev/null 2>&1; status=$?
    [ "$status" -eq 0 ] || fail "long: a --long run inside a hold of the second lane exited $status, wanted 0"
    timeout 20 flock "$scratch/long" env HEAVY_WALK_LOCK="$scratch/long" HEAVY_WALK_LOG="$scratch/long.log" \
        "$self" --lock --long --round 118 --data 1 --nice 0 -- true > /dev/null 2>&1; status=$?
    [ "$status" -eq 64 ] || fail "long: a --long run under a bare flock of the first lane exited $status, wanted 64"
    flock -n "$scratch/long" true && flock -n "$scratch/long.lane2" true && flock -n "$scratch/long.gate" true ||
        fail "long: a lane or the gate is still held after every run ended"
    echo "bounded --self-test: a second long run queued $(long_line 112 | sed 's/.* wait=\([0-9.]*s\) .*/\1/') for the second lane with the first free, and a small walk took the first beside them at once; --long alone, with --clock, above 6 GiB, inside the first lane and under its flock refused"

    # 12. A clock run behind a long run (ADR 1756), on files of the case's own. A long run holds the
    #     second lane. A clock run asked next waits for it holding the clock turn and not the gate, so
    #     a small walk asked while it waits is granted the first lane at once — the calibration, since
    #     a clock run that took the gate first stops that walk until the long run ends. A second long
    #     run asked while the clock run waits is not granted the second lane when the first frees it,
    #     but only once the clock run has ended.
    turn_case() { HEAVY_WALK_LOCK="$scratch/turn" HEAVY_WALK_LOG="$scratch/turn.log" "$self" "$@"; }
    turn_case --lock --long --round 121 --data 1 --nice 0 -- sh -c "$hold_until" "$scratch/turn121" > /dev/null 2>&1 &
    first_long=$!
    appears "$scratch/turn.lane2.long" . "turn: the long run never marked the second lane"
    turn_case --lock --clock --round 122 --tree 1 --data 1 --nice 0 -- sh -c 'sleep 1; date +%s%N > "$0"' "$scratch/turn122" > /dev/null 2> "$scratch/turn122.err" &
    timed=$!
    appears "$scratch/turn122.err" 'held by a long run' "turn: the clock run never said it waits for the long run"
    turn_case --lock --round 123 --tree 1 --data 1 --nice 0 -- true > /dev/null 2>&1 || fail "turn: the small walk beside a waiting clock run failed"
    waited_nothing "$(grep ' round=123 ' "$scratch/turn.log" | sed 's/.* wait=\([0-9.]*\)s .*/\1/')" ||
        fail "turn: a small walk asked while a clock run waited for a long run was not granted the first lane at once: $(grep ' round=123 ' "$scratch/turn.log")"
    turn_case --lock --long --round 124 --data 1 --nice 0 -- sh -c 'date +%s%N > "$0"' "$scratch/turn124" > /dev/null 2> "$scratch/turn124.err" &
    second_long=$!
    appears "$scratch/turn124.err" 'a clock run waits for the second lane' "turn: the long run asked behind a waiting clock run never queued at the turn"
    : > "$scratch/turn121.go"
    wait "$first_long" "$timed" "$second_long"
    clock_end=$(cat "$scratch/turn122" 2>/dev/null) late_start=$(cat "$scratch/turn124" 2>/dev/null)
    [ -n "$clock_end" ] && [ -n "$late_start" ] && [ "$late_start" -ge "$clock_end" ] ||
        fail "turn: a long run asked while a clock run waited took the second lane before the clock run ended (${late_start:-never} against ${clock_end:-never} ns)"
    grep ' round=122 ' "$scratch/turn.log" | grep -q ' behind=[^ ]*round=121[^ ]* kind=clock lane=1+2 ' ||
        fail "turn: the clock run's line does not name the long run: $(grep ' round=122 ' "$scratch/turn.log")"
    [ -e "$scratch/turn.lane2.long" ] && fail "turn: the long run's mark outlived it"
    flock -n "$scratch/turn" true && flock -n "$scratch/turn.lane2" true && flock -n "$scratch/turn.gate" true && flock -n "$scratch/turn.clock" true ||
        fail "turn: a lane, the gate or the clock turn is still held after every run ended"
    echo "bounded --self-test: a clock run waited $(grep ' round=122 ' "$scratch/turn.log" | sed 's/.* wait=\([0-9.]*s\) .*/\1/') for a long run without stopping the first lane, where a small walk ran at once, and the long run asked behind it ran after it"

    # 13. A walk's grant follows its ask (ADR 1790), on files of the case's own. A large walk holds the
    #     first lane and a small one the second. Two small walks, 133 and 134, ask in that order and
    #     look at their lanes every two seconds; the second lane is freed and a third, 135, asks at once
    #     and looks every half second, so the freed lane is under the last ask's look while the earlier
    #     two sleep — the race a poll without order loses, and the calibration: with the queue's look
    #     planted out, 135 took the lane first. 133 must be granted the second lane, 134 once 133 ends
    #     and 135 once 134 ends, and 135's line names an earlier ask. Then a large walk asks for the
    #     first lane, still held, and a small walk asked after it is granted the second at once: an
    #     earlier ask holds back only the lanes it takes. No ticket outlives its walk.
    order_case() { HEAVY_WALK_LOCK="$scratch/order" HEAVY_WALK_LOG="$scratch/order.log" "$self" "$@"; }
    order_case --lock --round 131 --tree 12 --data 1 --nice 0 -- sh -c "$hold_until" "$scratch/order131" > /dev/null 2>&1 &
    first_holder=$!
    appears "$scratch/order.holder" . "order: the large walk never took the first lane"
    order_case --lock --round 132 --tree 1 --data 1 --nice 0 -- sh -c "$hold_until" "$scratch/order132" > /dev/null 2>&1 &
    second_holder=$!
    appears "$scratch/order.lane2.holder" . "order: the small walk never took the second lane"
    starts_then_holds='date +%s%N > "$0.start"; '"$hold_until"
    for walk in 133 134; do
        HEAVY_WALK_POLL=2 order_case --lock --round "$walk" --tree 1 --data 1 --nice 0 -- sh -c "$starts_then_holds" "$scratch/order$walk" \
            > /dev/null 2> "$scratch/order$walk.err" &
        eval "order_pid_$walk=\$!"
        appears "$scratch/order$walk.err" 'queued for the heavy-walk lock' "order: walk $walk never said it queued"
    done
    : > "$scratch/order132.go"
    wait "$second_holder"
    order_case --lock --round 135 --tree 1 --data 1 --nice 0 -- sh -c "$starts_then_holds" "$scratch/order135" \
        > /dev/null 2> "$scratch/order135.err" &
    order_pid_135=$!
    for _ in $(seq 600); do
        { [ -e "$scratch/order135.start" ] || grep -q 'queued for the heavy-walk lock' "$scratch/order135.err" 2>/dev/null; } && break
        sleep 0.1
    done
    # Only the second lane is free to them, so one walk at a time holds it: the one started and not
    # yet released is the one granted.
    for walk in 133 134 135; do
        granted=
        for _ in $(seq 600); do
            for any in 133 134 135; do
                [ -e "$scratch/order$any.start" ] && [ ! -e "$scratch/order$any.go" ] && granted=$any
            done
            [ -n "$granted" ] && break
            sleep 0.1
        done
        [ -n "$granted" ] || fail "order: no walk was granted the second lane, which walk $walk was owed"
        [ "$granted" = "$walk" ] || fail "order: walk $granted was granted the second lane while walk $walk, which asked before it, still waited"
        : > "$scratch/order$walk.go"
    done
    wait "$order_pid_133" "$order_pid_134" "$order_pid_135"
    order_case --lock --round 136 --tree 12 --data 1 --nice 0 -- true > /dev/null 2> "$scratch/order136.err" &
    large=$!
    appears "$scratch/order136.err" 'queued for the heavy-walk lock' "order: the large walk behind the first lane's holder never queued"
    order_case --lock --round 137 --tree 1 --data 1 --nice 0 -- true > /dev/null 2>&1 || fail "order: the small walk behind a waiting large one failed"
    : > "$scratch/order131.go"
    appears "$scratch/order.log" ' round=136 ' "order: the large walk was never granted the first lane its holder freed"
    wait "$first_holder" "$large"
    order_line() { grep " round=$1 " "$scratch/order.log"; }
    for walk in 133 134 135; do
        order_line "$walk" | grep -q ' kind=small lane=2 ' || fail "order: walk $walk's line: $(order_line "$walk")"
    done
    order_line 135 | grep -q ' behind=[^ ]*round=13[34]' || fail "order: the last ask's line does not name the asks ahead of it: $(order_line 135)"
    waited_nothing "$(order_line 137 | sed 's/.* wait=\([0-9.]*\)s .*/\1/')" && order_line 137 | grep -q ' kind=small lane=2 ' ||
        fail "order: a small walk behind a large walk's ask was not granted the free second lane at once: $(order_line 137)"
    order_line 136 | grep -q ' kind=large lane=1 ' || fail "order: the large walk's line: $(order_line 136)"
    [ -s "$scratch/order.queue" ] && fail "order: a ticket outlived its walk: $(cat "$scratch/order.queue")"
    echo "bounded --self-test: three small walks were granted the second lane in the order they asked, the last asked when the lane was free; a large walk's ask held back the first lane only"

    # 14. A wait that ends before a grant is a line (ADR 1790), on files of the case's own. A large walk
    #     holds the first lane. A large walk queued behind it and stopped with TERM leaves a line with
    #     its wrapper's 143, `hold=0.0s` and `lane=-`, and takes its ticket with it. A clock run queued
    #     behind the same holder, in the kernel, and stopped with TERM leaves its line while the holder
    #     still holds — the calibration, since a `flock` in the foreground held back the trap until the
    #     grant and the run then ended on no line. A walk killed with KILL writes nothing and leaves its
    #     ticket, a walk suspended with STOP keeps its own, and the walk asked after both is granted the
    #     lane when the holder ends all the same; the suspended one, continued, is granted it next.
    stop_lock="$scratch/stop" stop_log="$scratch/stop.log"
    HEAVY_WALK_LOCK=$stop_lock HEAVY_WALK_LOG=$stop_log "$self" --lock --round 141 --tree 12 --data 1 --nice 0 -- sh -c "$hold_until" "$scratch/stop141" > /dev/null 2>&1 &
    stop_holder=$!
    appears "$scratch/stop.holder" . "stop: the holder never took the first lane"
    HEAVY_WALK_LOCK=$stop_lock HEAVY_WALK_LOG=$stop_log "$self" --lock --round 142 --tree 12 --data 1 --nice 0 -- true > /dev/null 2> "$scratch/stop142.err" &
    stopped=$!
    appears "$scratch/stop142.err" 'queued for the heavy-walk lock' "stop: the walk behind the holder never queued"
    kill -TERM "$stopped"
    wait "$stopped"; status=$?
    [ "$status" -eq 143 ] || fail "stop: a walk stopped while it queued exited $status, wanted 143"
    grep ' round=142 ' "$stop_log" | grep -q ' hold=0\.0s exit=143 .* behind=[^ ]*round=141[^ ]* kind=large lane=- cmd=true' ||
        fail "stop: a walk stopped while it queued left no line, or not this one: $(cat "$stop_log" 2>/dev/null)"
    grep -q "^$stopped " "$scratch/stop.queue" && fail "stop: the stopped walk's ticket outlived it: $(cat "$scratch/stop.queue")"
    HEAVY_WALK_LOCK=$stop_lock HEAVY_WALK_LOG=$stop_log "$self" --lock --clock --round 143 --tree 1 --data 1 --nice 0 -- true > /dev/null 2> "$scratch/stop143.err" &
    stopped=$!
    appears "$scratch/stop143.err" 'queued for the heavy-walk lock (lane 1' "stop: the clock run behind the holder never queued for the first lane"
    kill -TERM "$stopped"
    appears "$stop_log" ' round=143 ' "stop: a clock run stopped while it waited in the kernel left no line"
    grep -q ' round=141 ' "$stop_log" && fail "stop: a clock run stopped while it waited in the kernel wrote its line only once the holder had ended, so the trap waited for the grant"
    wait "$stopped"; status=$?
    [ "$status" -eq 143 ] || fail "stop: a clock run stopped while it queued exited $status, wanted 143"
    grep ' round=143 ' "$stop_log" | grep -q ' hold=0\.0s exit=143 .* kind=clock lane=- cmd=true' || fail "stop: the clock run's line: $(grep ' round=143 ' "$stop_log")"
    flock -n "$scratch/stop.lane2" true && flock -n "$scratch/stop.gate" true && flock -n "$scratch/stop.clock" true ||
        fail "stop: the stopped clock run left the second lane, the gate or the clock turn held"
    HEAVY_WALK_LOCK=$stop_lock HEAVY_WALK_LOG=$stop_log "$self" --lock --round 144 --tree 12 --data 1 --nice 0 -- true > /dev/null 2> "$scratch/stop144.err" &
    stopped=$!
    appears "$scratch/stop144.err" 'queued for the heavy-walk lock' "stop: the walk to be killed never queued"
    kill -KILL "$stopped"
    wait "$stopped" 2>/dev/null
    grep -q "^$stopped " "$scratch/stop.queue" || fail "stop: a walk killed with KILL left no ticket, so the case does not test its drop"
    HEAVY_WALK_LOCK=$stop_lock HEAVY_WALK_LOG=$stop_log "$self" --lock --round 145 --tree 12 --data 1 --nice 0 -- true > /dev/null 2> "$scratch/stop145.err" &
    suspended=$!
    appears "$scratch/stop145.err" 'queued for the heavy-walk lock' "stop: the walk to be suspended never queued"
    kill -STOP "$suspended"
    HEAVY_WALK_LOCK=$stop_lock HEAVY_WALK_LOG=$stop_log "$self" --lock --round 146 --tree 12 --data 1 --nice 0 -- true > /dev/null 2> "$scratch/stop146.err" &
    after=$!
    appears "$scratch/stop146.err" 'queued for the heavy-walk lock' "stop: the walk after the killed and the suspended one never queued"
    : > "$scratch/stop141.go"
    appears "$stop_log" ' round=146 ' "stop: the walk asked after a killed and a suspended one was never granted the lane, so a ticket of theirs held it back"
    kill -CONT "$suspended"
    appears "$stop_log" ' round=145 ' "stop: the suspended walk, continued, was never granted the lane"
    wait "$stop_holder" "$after" "$suspended"; status=$?
    [ "$status" -eq 0 ] || fail "stop: the suspended walk exited $status"
    grep ' round=146 ' "$stop_log" | grep -q ' kind=large lane=1 ' || fail "stop: the walk asked after a killed one: $(grep ' round=146 ' "$stop_log")"
    grep ' round=145 ' "$stop_log" | grep -q ' kind=large lane=1 ' || fail "stop: the suspended walk: $(grep ' round=145 ' "$stop_log")"
    grep -q ' round=144 ' "$stop_log" && fail "stop: a walk killed with KILL wrote a line, which it cannot"
    [ -s "$scratch/stop.queue" ] && fail "stop: a ticket outlived its walk: $(cat "$scratch/stop.queue")"
    echo "bounded --self-test: a walk and a clock run stopped while they queued each left a line with lane=- at once; a killed and a suspended walk's tickets held back nobody"

    echo "bounded --self-test: every case holds"
    exit 0
fi

[ $# -gt 0 ] || usage
case "$shards" in ''|*[!0-9]*|0) echo "bounded: --shards wants a positive integer" >&2; exit 64 ;; esac

cores=$(nproc)
threads=$(( cores / shards ))
[ "$threads" -ge 1 ] || threads=1
# A walk's peak is the documents in flight, one a thread (ADR 0798), so the lanes' ceilings and the
# kind a walk declares were measured at the merge's four threads (ADR 1698). A locked run takes that
# figure rather than the machine's cores: a small walk at 24 threads is not the walk its kind priced,
# and the rule line no longer has to spell the four for the run to be the one it declared (ADR 1766).
[ -n "$take_lock" ] && threads=$locked_threads
if [ -z "$data_gib" ]; then
    data_gib=$(( walk_budget_gib / shards ))
    [ "$data_gib" -ge 1 ] || data_gib=1
fi
case "$data_gib" in ''|*[!0-9]*|0) echo "bounded: --data wants a positive integer of GiB" >&2; exit 64 ;; esac
if [ "$data_gib" -gt "$round_share_gib" ] && [ -z "$tree_gib" ]; then
    echo "bounded: --data $data_gib GiB is above a round's share of $round_share_gib, and no --tree ceiling was given — see the header for 2026-09-02, when exactly this invocation took the machine down; pass --tree as well, or a smaller --data" >&2
    exit 64
fi
# Rule 3 of the header: a run without a tree ceiling is not started by omission. A long run's
# default is the second lane's ceiling, the only lane it takes.
if [ -z "$tree_gib" ]; then
    if [ -n "$long" ]; then tree_gib=$small_lane_gib; else tree_gib=$round_share_gib; fi
fi
case "$tree_gib" in ''|*[!0-9]*|0) echo "bounded: --tree wants a positive integer of GiB" >&2; exit 64 ;; esac
case "$tasks" in ''|*[!0-9]*|0) echo "bounded: --tasks wants a positive integer" >&2; exit 64 ;; esac
if [ -n "$clock" ] && [ -z "$take_lock" ]; then
    echo "bounded: --clock declares how a run takes the heavy-walk lock, and this run takes none; pass --lock as well" >&2
    exit 64
fi
if [ -n "$long" ] && [ -z "$take_lock" ]; then
    echo "bounded: --long declares how a run takes the heavy-walk lock, and this run takes none; pass --lock as well" >&2
    exit 64
fi
if [ -n "$long" ] && [ -n "$clock" ]; then
    echo "bounded: --long and --clock together: a clock run takes both lanes for a time it measures, a long run one lane for a time it chooses; a campaign is not timed, so declare --long" >&2
    exit 64
fi
if [ -n "$long" ] && [ "$tree_gib" -gt "$small_lane_gib" ]; then
    echo "bounded: --long holds the second lane, whose ceiling is $small_lane_gib GiB, and this run asks --tree $tree_gib; a run that needs more is a large walk on the first lane and does not declare --long (ADR 1756)" >&2
    exit 64
fi
# The run's kind, which decides its lanes (the lock's section above): what it declared, never what it
# turns out to peak at, because the lane is granted before the walk starts.
if [ -n "$long" ]; then kind=long
elif [ -n "$clock" ]; then kind=clock
elif [ "$tree_gib" -le "$small_lane_gib" ]; then kind=small
else kind=large
fi
if [ "$tasks" -gt "$task_budget" ]; then
    echo "bounded: --tasks $tasks is above the agent's task budget of $task_budget, and the limit counts every task of the user, so it would lend this command the other rounds' share — see trap 116" >&2
    exit 64
fi
data_bytes=$(( data_gib * 1024 * 1024 * 1024 ))
tree_kib=$(( tree_gib * 1024 * 1024 ))

# rayon reads this once, at pool creation, and every walk in this tree uses the global pool. A
# caller that has set it already knows better than the share.
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-$threads}"

# A signal ends the wrapper through `exit`, so the trap below still writes the lock's line; the
# command is a child and keeps its own dispositions.
scratch=
trap 'code=$?; lock_record "$code"; [ -z "$scratch" ] || rm -rf "$scratch"' EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

if [ -n "$take_lock" ]; then
    command_words=$(printf '%s ' "$@" | tr '\n\t' '  ' | cut -c1-200)
    # The branch of the tree this script is in, not of the directory it was called from: a round
    # measuring in a private export of HEAD calls the batch worktree's wrapper from there, and its
    # line is still the batch's.
    batch=$(git -C "$(dirname -- "${BASH_SOURCE[0]}")" rev-parse --abbrev-ref HEAD 2>/dev/null) || batch=-
    lock_take; taken=$?
    if [ "$taken" -eq 64 ]; then exit 64; fi
    [ "$taken" -eq 0 ] || { echo "bounded: could not take the heavy-walk lock $lock_path" >&2; exit 75; }
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/bounded.XXXXXX") || exit 1
errlog="$scratch/stderr"

# The command's standard error goes through a pipe and `tee`, never straight to a file: a file
# is what a limit can reach (trap 18) and a pipe is not. Its standard output stays its own —
# fd 3 carries it around the pipeline — because a survey's report is that stream. The subshell
# exits with the *command's* status rather than `tee`'s.
# Where this wrapper took the lock, the command and its `tee` run with every lane's descriptor
# closed and the subshell keeps them, so the lock ends with the command and not with what the
# command left running (the header, ADR 1674); the marker names the subshell, the process that holds
# them for it. `without_lanes` closes them in its own process, one side of the pipeline, and becomes
# the program. The output's copy is a descriptor of its own number for the same reason: an inherited
# lock is on whatever number its `flock` opened, often 3.
started=$(date +%s)
without_lanes() {
    local fd
    for fd in $lane_fds; do eval "exec $fd>&-"; done
    exec "$@"
}
(
    exec {out_fd}>&1
    [ -z "$lane_fds" ] || export HEAVY_WALK_HELD_BY=$BASHPID
    # Each `--build` in the leader's own tree, so the ceiling sees it, and with the lanes' descriptors
    # closed as the command's are: a build starts `sccache`'s server, which would keep a descriptor it
    # was handed for as long as it idles (trap 131). Its output goes to a file and is shown only when
    # it fails, so a survey's standard output is still only the survey's.
    for build in "${builds[@]}"; do
        build_started=$(date +%s)
        # shellcheck disable=SC2086 # the arguments are separate words by design (the header)
        ( without_lanes prlimit --data="$data_bytes" --nproc="$tasks" nice -n "$niceness" cargo build $build ) \
            > "$scratch/build.log" 2>&1 {out_fd}>&-
        built=$?
        if [ "$built" -ne 0 ]; then
            tail -n 20 "$scratch/build.log" >&2
            echo "bounded: \`cargo build $build\` failed (exit $built) inside the hold, so the command was not started" >&2
            exit "$built"
        fi
        echo "bounded: built \`cargo build $build\` inside the hold in $(( $(date +%s) - build_started ))s" >&2
    done
    without_lanes prlimit --data="$data_bytes" --nproc="$tasks" nice -n "$niceness" "$@" \
        2>&1 1>&"$out_fd" {out_fd}>&- | without_lanes tee "$errlog" {out_fd}>&- >&2
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
# A refused `fork` or thread spawn reads EAGAIN on every path: bash prints `fork: retry: Resource
# temporarily unavailable`, Rust's `failed to spawn thread` carries `(os error 11)`, Python raises
# `BlockingIOError`. The line names the limit and the user's count beside it, since the count is
# the agent's and not this command's alone. Only for a run that failed: a program that met EAGAIN
# on a non-blocking descriptor and carried on prints the same words.
if [ "$status" -ne 0 ] && grep -q "Resource temporarily unavailable\|BlockingIOError" "$errlog"; then
    echo "bounded: STOPPED BY THE TASK LIMIT — exit $status after ${elapsed}s: a fork or a thread" \
         "spawn was refused under an RLIMIT_NPROC of $tasks, which counts every task of $(id -un)" \
         "($(ps -u "$(id -un)" -o nlwp= | awk '{ s += $1 } END { print s + 0 }') now). The command's own" \
         "last words are above; a command that meets this bound is spawning without waiting (trap 116)." >&2
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
     "process tree, under RLIMIT_DATA $data_gib GiB, RLIMIT_NPROC $tasks, $RAYON_NUM_THREADS rayon thread(s), nice $niceness" >&2
exit "$status"
