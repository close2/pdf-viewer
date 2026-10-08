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
#   tools/bounded.sh [--lock [--clock] [--round N]] [--shards N] [--data GiB] [--tree GiB] [--tasks N] [--nice n] -- <command> [args…]
#   tools/bounded.sh --held
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
#                run's kind and its lane (below). The lock has two lanes: a run whose `--tree` is 6
#                GiB or less takes either, any larger one the first, and a `--clock` run both. An
#                ancestor that holds the lock already — `flock <lock> tools/bounded.sh --lock …`, or
#                a `--lock` wrapper this command runs under — is found and used, never queued
#                behind. The command runs without the lanes' descriptors and with
#                `HEAVY_WALK_HELD_BY` naming the process that holds them for the command.
#   --clock      with --lock: this run's figure is a time — a gate with a band on a clock, an A/B
#                pair, a launch measured with its clocks — so it takes both lanes, and while it
#                waits for them no other run is granted either. A run that holds a clock run anywhere
#                inside it declares this itself; a `--clock` run inside a hold of one lane is refused.
#   --held       exit 0 if the caller runs under a hold of the lock — it has a lane's descriptor
#                open, or `HEAVY_WALK_HELD_BY` names an ancestor that has — and 1 otherwise. What a
#                script that must run under the lock asks before it walks (`tools/batch.sh arms-held`).
#   --round N    the round's session number, written on that line so a round's lock time can be
#                read off the log (default `-`).
#   --self-test  run the sampler against synthetic process tables and against live trees — one
#                that fans out, one that crosses the ceiling, one whose sampler stalls, one that
#                forks past a task limit of its own — and exit 0 only if every case holds. `tools/conformance/tests/bounded.rs` runs it under
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
held_query=
round=-
lane_fds=
lanes=

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
        --held) held_query=1; shift ;;
        --round) round=$2; shift 2 ;;
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
# first stays free for a large one; any other walk is large and takes the first. Two lanes are at
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
# `lock_take` sets `lane_fds` (the descriptors this wrapper holds, one per lane), `lanes` (as the log
# line names them), `asked_ms`, `held_ms` and `behind`. A descriptor this process already has open on
# the first lane is a bare `flock` ancestor's — `flock <lock> tools/bounded.sh --lock …` — and is
# locked again rather than a second one opened: `flock` locks an open file description, so a second
# description of the same file would queue behind the caller's own lock for ever, while the
# inherited one is granted at once. The wait is printed when there is one, so a round watching its
# shell knows what it is waiting for, and so is the run it found holding the lock (ADR 1659).
lane2_path=$lock_path.lane2
gate_path=$lock_path.gate
small_lane_gib=6
poll_interval=0.5

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

# `lane_hold N FD` takes lane N on FD, waiting in the kernel: a clock run's way, behind the gate.
lane_hold() {
    flock -n "$2" || { lock_queued "lane $1" "$(lock_holder "$(lane_path "$1")")"; flock "$2"; }
}

# `lock_take` returns at once, taking nothing and writing no line, where a `--lock` wrapper above
# this one holds the lock already: that wrapper's line is the hold, and a second one would count it
# twice. A clock run under an ancestor that holds one lane only would measure beside whatever walk
# took the other, so it is refused, with the cure: the outermost run declares `--clock`.
lock_take() {
    local inherited ancestor gate_fd fd lane wanted=
    asked_ms=$(now_ms)
    behind=-
    inherited=$(lock_open_in $$ "$lock_path") || inherited=
    if [ -z "$inherited" ] && ancestor=$(marked_ancestor_lanes); then
        if [ "$kind" = clock ] && [ "$ancestor" != "1 2" ]; then
            echo "bounded: a --clock run inside a hold of lane $ancestor only would be timed beside the walk on the other lane; declare the outermost --lock run --clock" >&2
            return 64
        fi
        return 0
    fi
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
        exec {gate_fd}>>"$gate_path" || return 1
        flock -n "$gate_fd" || { lock_queued "the gate, behind an earlier clock run" "$(lock_holder "$gate_path")"; flock "$gate_fd" || return 1; }
        printf '%s round=%s %s\n' "$$" "$round" "$command_words" > "$gate_path.holder" 2>/dev/null
        for lane in 1 2; do
            exec {fd}>>"$(lane_path "$lane")" || return 1
            lane_hold "$lane" "$fd" || return 1
            lane_fds="${lane_fds:+$lane_fds }$fd"
        done
        lanes=1+2
        [ "$(cut -d' ' -f1 "$gate_path.holder" 2>/dev/null)" != "$$" ] || rm -f -- "$gate_path.holder"
        exec {gate_fd}>&-
    else
        exec {gate_fd}>>"$gate_path" || return 1
        if [ "$kind" = small ]; then wanted="2 1"; else wanted=1; fi
        declare -A lane_fd=()
        for lane in $wanted; do exec {fd}>>"$(lane_path "$lane")" || return 1; lane_fd[$lane]=$fd; done
        while [ -z "$lanes" ]; do
            if flock -n "$gate_fd"; then
                flock -u "$gate_fd"
                for lane in $wanted; do
                    flock -n "${lane_fd[$lane]}" && { lanes=$lane; break; }
                done
                [ -n "$lanes" ] && break
                fd=
                for lane in $wanted; do fd="${fd:+$fd+}$(lock_holder "$(lane_path "$lane")")"; done
                lock_queued "lane ${wanted// / or }" "$fd"
            else
                lock_queued "a clock run holds the gate while it waits for both lanes" "$(lock_holder "$gate_path")"
            fi
            sleep "$poll_interval"
        done
        for lane in $wanted; do
            if [ "$lane" = "$lanes" ]; then lane_fds=${lane_fd[$lane]}; else eval "exec ${lane_fd[$lane]}>&-"; fi
        done
        exec {gate_fd}>&-
    fi
    held_ms=$(now_ms)
    for lane in ${lanes//+/ }; do
        printf '%s round=%s %s\n' "$$" "$round" "$command_words" > "$(lane_path "$lane").holder" 2>/dev/null
    done
}

# `lock_record STATUS` appends the run's one line, if it held the lock. Shorter than `PIPE_BUF`
# and written with `O_APPEND` while the lock is still held, so two wrappers cannot interleave one.
# `kind=` is what the run declared and `lane=` what it was granted, so that ADR 1671's re-ask can
# replay a batch's lines under either rule.
lock_record() {
    [ -n "${held_ms:-}" ] || return 0
    local ended_ms lane holder
    ended_ms=$(now_ms)
    printf '%s batch=%s round=%s wait=%ss hold=%ss exit=%s peak=%sGiB behind=%s kind=%s lane=%s cmd=%s\n' \
        "$(date -d "@$(( asked_ms / 1000 ))" '+%Y-%m-%dT%H:%M:%S')" "$batch" "$round" \
        "$(seconds $(( held_ms - asked_ms )))" "$(seconds $(( ended_ms - held_ms )))" "$1" \
        "$(awk -v k="${peak_kib:-0}" 'BEGIN { printf "%.2f", k / 1048576 }')" "${behind:--}" \
        "$kind" "$lanes" "$command_words" >> "$lock_log" ||
        echo "bounded: the lock was held, and its line could not be appended to $lock_log" >&2
    for lane in ${lanes//+/ }; do
        holder=$(lane_path "$lane").holder
        [ "$(cut -d' ' -f1 "$holder" 2>/dev/null)" != "$$" ] || rm -f -- "$holder"
    done
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
    [ "$elapsed" -lt 15 ] || fail "blind: took ${elapsed}s to stop a tree whose sampler stalled"
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
    lock_case() { HEAVY_WALK_LOCK="$scratch/lock" HEAVY_WALK_LOG="$scratch/lock.log" "$self" "$@"; }
    ( flock "$scratch/lock" sh -c ': > "$0"; sleep 2' "$scratch/held" ) &
    holder=$!
    for _ in $(seq 50); do [ -e "$scratch/held" ] && break; sleep 0.1; done
    [ -e "$scratch/held" ] || fail "lock: the case's own holder never took its lock"
    lock_case --lock --round 7 --tree 12 --data 1 --nice 0 -- sh -c 'exit 3' > /dev/null 2> "$scratch/lock.err"
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
    grep -q '^[0-9T:-]* batch=[^ ]* round=7 wait=[0-9.]*s hold=[0-9.]*s exit=3 peak=[0-9.]*GiB behind=[^ ]*sleep_2[^ ]* kind=large lane=1 cmd=sh -c exit 3 *$' "$scratch/lock.log" ||
        fail "lock: the line is not the shape the header states: $(head -n 1 "$scratch/lock.log")"
    for line in 2 3; do
        [ "$(sed -n "${line}s/.* wait=\([0-9.]*\)s .*/\1/p" "$scratch/lock.log")" = 0.0 ] ||
            fail "lock: run $line found the lock free or its caller's and still waited: $(sed -n "${line}p" "$scratch/lock.log")"
    done
    HEAVY_WALK_LOCK="$scratch/lock" HEAVY_WALK_LOG="$scratch/held.log" \
        "$self" --lock --round 77 --tree 12 --data 1 --nice 0 -- sleep 2 > /dev/null 2>&1 &
    holder=$!
    for _ in $(seq 50); do [ -s "$scratch/lock.holder" ] && break; sleep 0.1; done
    lock_case --lock --round 7 --tree 12 --data 1 --nice 0 -- true > /dev/null 2>&1
    wait "$holder"
    tail -n 1 "$scratch/lock.log" | grep -q ' behind=[^ ]*round=77_sleep_2' ||
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
    for _ in $(seq 100); do [ -e "$scratch/held8.asked" ] && break; sleep 0.1; done
    [ -n "$daemon" ] && kill "$daemon" 2>/dev/null
    [ -e "$scratch/held8.asked" ] || fail "held: the daemon never asked --held"
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
    #    are refused.
    lane_case() { HEAVY_WALK_LOCK="$scratch/lane" HEAVY_WALK_LOG="$scratch/lane.log" "$self" "$@"; }
    lane_seen() { for _ in $(seq 50); do [ -s "$1" ] && return 0; sleep 0.1; done; fail "lanes: $1 never appeared"; }
    lane_case --lock --round 91 --tree 12 --data 1 --nice 0 -- sleep 5 > /dev/null 2>&1 &
    large=$!
    lane_seen "$scratch/lane.holder"
    lane_case --lock --round 92 --tree 1 --data 1 --nice 0 -- sleep 2 > /dev/null 2>&1 &
    small=$!
    lane_seen "$scratch/lane.lane2.holder"
    lane_case --lock --clock --round 93 --tree 1 --data 1 --nice 0 -- sh -c '
        flock -n "$1" true || echo held > "$0.1"; flock -n "$1.lane2" true || echo held > "$0.2"
        sleep 1; date +%s%N > "$0.end"' "$scratch/clock" "$scratch/lane" > /dev/null 2>&1 &
    timed=$!
    lane_seen "$scratch/lane.gate.holder"
    lane_case --lock --round 94 --tree 1 --data 1 --nice 0 -- sh -c 'date +%s%N > "$0"' "$scratch/late" > /dev/null 2>&1 &
    late=$!
    wait "$large" "$small" "$timed" "$late"
    lane_line() { grep " round=$1 " "$scratch/lane.log"; }
    lane_line 91 | grep -q ' kind=large lane=1 ' || fail "lanes: the large walk's line: $(lane_line 91)"
    lane_line 92 | grep -q ' wait=0\.0s .* kind=small lane=2 ' || fail "lanes: a small walk beside a large one was not granted the second lane at once: $(lane_line 92)"
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
case "$tasks" in ''|*[!0-9]*|0) echo "bounded: --tasks wants a positive integer" >&2; exit 64 ;; esac
if [ -n "$clock" ] && [ -z "$take_lock" ]; then
    echo "bounded: --clock declares how a run takes the heavy-walk lock, and this run takes none; pass --lock as well" >&2
    exit 64
fi
# The run's kind, which decides its lanes (the lock's section above): what it declared, never what it
# turns out to peak at, because the lane is granted before the walk starts.
if [ -n "$clock" ]; then kind=clock
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
