#!/usr/bin/env bash
#
# Start a long command detached, and wait for it in calls that each return within a wall.
#
# An agent's requests are billed against a prompt cache that lives five minutes, measured from the
# start of the request that last read it, and a foreground command sends nothing while it runs. A
# build, a walk or a clock hold that takes longer than that makes the agent's next request write
# its whole context again at 1.25 times an input token, where a read costs 0.05 times on the model
# the rounds run. The rounds' own wait-loops were written to the harness's ten-minute ceiling, so
# almost every one of them lost the cache; ADR 1976 has the census and the price. So a command that
# may run longer than the wall is started here, and waited for here, and every call returns inside
# the wall whether the command has ended or not — the agent's next request is then the heartbeat
# that keeps the cache warm.
#
#   tools/wait.sh --start <log> [--wall S] [--tail N] -- <command> [args…]
#   tools/wait.sh [--wall S] [--tail N] [--until ERE] <log>
#   tools/wait.sh --stop <log>
#   tools/wait.sh --self-test
#
#   --start <log>  run the command in a session of its own, its standard output and error into
#                  <log> (which must not exist yet: an old end line would end the wait at once),
#                  its pid, process group and start time into <log>.pid, and a last line
#                  `wait: exit <status> after <n>s` appended when it ends; then wait as below. A
#                  `tools/bounded.sh --lock --tree <GiB> …` run is started this way like any other: the lock is
#                  the wrapper's, held in the detached session, and its own lines go into <log>.
#   <log>          wait until <log> carries its end line, the process group of <log>.pid is gone,
#                  or the wall is reached, whichever is first.
#   --wall S       the most one call waits, in seconds (default 240, at most 270): the cache's five
#                  minutes hold the agent's own turn too, so the wall leaves it a minute.
#   --tail N       the lines of <log> printed before the verdict line (default 3).
#   --until ERE    the end line for a log this script did not start (a harness background task's
#                  output file, a `> log &` of older habit); without it, and without <log>.pid,
#                  the end is tools/bounded.sh's own closing line.
#   --stop <log>   send TERM to the process group <log>.pid names, and say whether it went.
#
# A wait's options may stand before its log or after it — `tools/wait.sh <log> --wall 30` waits as
# `tools/wait.sh --wall 30 <log>` does — because a wait that printed this text and exited was read
# as a wait that had been made (ADR 2011). Anything but one log is refused with a sentence.
#
# Exit status: the command's own once it has ended; 75 (EX_TEMPFAIL) when the wall came first and
# the command is still running — re-issue `tools/wait.sh <log>`; 70 when the process group is gone
# without its end line (it was killed); 64 for a usage error. The verdict line is the last line
# printed and always starts `wait:`, so a command whose own status is 70 or 75 is still told apart.
#
# A wait is only a watcher: killing it — the harness's own timeout, an interrupt — leaves the
# command running in its session, and the next `tools/wait.sh <log>` picks the wait up again.
# ADR 1976 is the decision, ADR 1977 the rule it puts into the round's contract.

set -u -o pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
wall=240
wall_ceiling=270
tail_lines=3
until_ere=
start_log=
stop_log=
self_test=

usage() {
    sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 64
}

refuse() {
    printf 'wait: %s\n' "$1" >&2
    exit 64
}

needs_value() {
    [ "$1" -ge 2 ] || refuse "$2 needs a value after it"
}

# A word that is not an option is the log to wait for, wherever it stands; after --start's log it
# is the command's first word instead, as it always was.
logs=()
while [ $# -gt 0 ]; do
    case $1 in
        --start) needs_value $# "$1"; start_log=$2; shift 2 ;;
        --stop) needs_value $# "$1"; stop_log=$2; shift 2 ;;
        --wall) needs_value $# "$1"; wall=$2; shift 2 ;;
        --tail) needs_value $# "$1"; tail_lines=$2; shift 2 ;;
        --until) needs_value $# "$1"; until_ere=$2; shift 2 ;;
        --self-test) self_test=1; shift ;;
        --) shift; break ;;
        -h|--help) usage ;;
        -*) refuse "no such option: $1" ;;
        *)
            [ -n "$start_log" ] && break
            logs+=("$1")
            shift
            ;;
    esac
done

case $wall in ''|*[!0-9]*) refuse "--wall takes whole seconds, not '$wall'" ;; esac
case $tail_lines in ''|*[!0-9]*) refuse "--tail takes a line count, not '$tail_lines'" ;; esac
if [ "$wall" -lt 1 ] || [ "$wall" -gt "$wall_ceiling" ]; then
    refuse "--wall $wall is outside 1..$wall_ceiling s: past it the next request finds the five-minute cache gone (ADR 1976)"
fi

# The line the runner appends, and tools/bounded.sh's closing lines for a log it did not start.
end_ere='^wait: exit [0-9]+ after [0-9]+s$'
bounded_ere='^bounded: (exit [0-9]+ after|STOPPED BY|ABORTED)'

# This script's own process group, which is its caller's: never one to wait on or to stop.
own_group() {
    ps -o pgid= -p "$$" | tr -d ' '
}

# Whether any process of the group is still alive. `kill -0` on a group asks the kernel, which is
# the one answer that cannot name a sibling's process: the group is the one this script made.
group_alive() {
    kill -0 -- "-$1" 2>/dev/null
}

read_pid_file() {
    # <pid> <pgid> <start epoch>, written once by --start.
    read -r run_pid run_pgid run_started < "$1.pid" || return 1
    case "$run_pid$run_pgid$run_started" in ''|*[!0-9]*) return 1 ;; esac
}

start() {
    local log=$1
    shift
    [ $# -gt 0 ] || refuse "--start $log needs a command after --"
    [ -e "$log" ] && refuse "$log exists already: an earlier run's end line would end this wait at once — name a new log"
    [ -e "$log.pid" ] && refuse "$log.pid exists already: name a new log"
    mkdir -p "$(dirname "$log")" || refuse "cannot make the directory of $log"
    : > "$log" || refuse "cannot write $log"
    # The agent's task budget, as every script under tools/ that runs a build holds itself to it
    # (trap 116, ADR 1612); the command inherits it.
    local task_budget
    task_budget=$("$root/tools/bounded.sh" --task-budget)
    ulimit -u "$task_budget"
    # `setsid` makes the run a session and a process group of its own, so the harness ending this
    # call — or its timeout killing it — does not reach the command, and `--stop` has one group to
    # name. The runner writes `<log>.pid` itself, after `setsid` has made it the group's leader: read
    # from here, `ps` can answer before `setsid` has run and name this script's own group, which is
    # the caller's, and `--stop` would then end the caller.
    # shellcheck disable=SC2016 # the runner's variables are its own, expanded when it runs
    setsid bash -c 'pidfile=$1; shift
printf "%s %s %s\n" "$$" "$(ps -o pgid= -p "$$" | tr -d " ")" "$(date +%s)" > "$pidfile.part"
mv "$pidfile.part" "$pidfile"
started=$SECONDS; "$@"; status=$?
printf "wait: exit %s after %ss\n" "$status" "$((SECONDS - started))"; exit "$status"' \
        wait-run "$log.pid" "$@" < /dev/null >> "$log" 2>&1 &
    local n=0
    until [ -e "$log.pid" ] || [ "$n" -ge 100 ]; do
        sleep 0.05
        n=$((n + 1))
    done
    read_pid_file "$log" || refuse "the run did not write $log.pid within 5 s"
    local pid=$run_pid pgid=$run_pgid
    if [ "$pgid" != "$pid" ] || [ "$pgid" = "$(own_group)" ]; then
        refuse "the run's process group $pgid is not its own (pid $pid): --stop would reach the caller"
    fi
    printf 'wait: started pid %s (process group %s) into %s\n' "$pid" "$pgid" "$log"
}

stop() {
    local log=$1
    read_pid_file "$log" || refuse "$log.pid is missing or not one this script wrote"
    [ "$run_pgid" = "$(own_group)" ] && refuse "$log.pid names this caller's own process group $run_pgid"
    if ! group_alive "$run_pgid"; then
        printf 'wait: process group %s of %s had already ended\n' "$run_pgid" "$log"
        exit 0
    fi
    kill -TERM -- "-$run_pgid" 2>/dev/null
    local n=0
    while group_alive "$run_pgid" && [ "$n" -lt 20 ]; do
        sleep 0.5
        n=$((n + 1))
    done
    if group_alive "$run_pgid"; then
        printf 'wait: process group %s of %s is still alive 10 s after TERM\n' "$run_pgid" "$log"
        exit 70
    fi
    printf 'wait: process group %s of %s stopped with TERM\n' "$run_pgid" "$log"
    exit 0
}

# Wait for <log>; print its last lines and one verdict line; exit as the header says.
wait_for() {
    local log=$1
    [ -e "$log" ] || refuse "$log does not exist (a run is started with --start)"
    local ere=$until_ere have_pid=
    if [ -z "$ere" ]; then
        if [ -e "$log.pid" ]; then
            read_pid_file "$log" || refuse "$log.pid is not one this script wrote"
            have_pid=1
            ere=$end_ere
        else
            ere=$bounded_ere
        fi
    elif [ -e "$log.pid" ] && read_pid_file "$log"; then
        have_pid=1
    fi
    local waited_from=$SECONDS line= vanished=
    while :; do
        line=$(grep -E -- "$ere" "$log" | tail -n 1)
        [ -n "$line" ] && break
        if [ -n "$have_pid" ] && ! group_alive "$run_pgid"; then
            # The group can end between the grep above and this test with its line just written.
            line=$(grep -E -- "$ere" "$log" | tail -n 1)
            [ -n "$line" ] && break
            vanished=1
            break
        fi
        [ $((SECONDS - waited_from)) -ge "$wall" ] && break
        sleep 1
    done
    [ "$tail_lines" -gt 0 ] && tail -n "$tail_lines" "$log" | grep -v -E -- "$end_ere" | sed 's/^/  /'
    local since=
    [ -n "$have_pid" ] && since=", $(($(date +%s) - run_started)) s since it started"
    if [ -n "$line" ]; then
        local status=0
        case $line in
            wait:\ exit\ *|bounded:\ exit\ *)
                status=${line#*exit }; status=${status%% *} ;;
            bounded:\ STOPPED*) status=${line#*— exit }; status=${status%% *} ;;
            bounded:\ ABORTED*) status=134 ;;
        esac
        case $status in ''|*[!0-9]*) status=1 ;; esac
        printf 'wait: %s ended%s — %s\n' "$log" "$since" "$line"
        exit "$status"
    fi
    if [ -n "$vanished" ]; then
        printf 'wait: %s: process group %s is gone without its end line%s — it was killed\n' \
            "$log" "$run_pgid" "$since"
        exit 70
    fi
    printf 'wait: %s still running after this call'"'"'s %s s%s; re-issue tools/wait.sh %s\n' \
        "$log" "$wall" "$since" "$log"
    exit 75
}

self_test() {
    local scratch fails=0 out status
    scratch=$(mktemp -d "${TMPDIR:-/tmp}/wait-self-test.XXXXXX") || exit 1
    trap 'rm -rf "$scratch"' EXIT
    local me=${BASH_SOURCE[0]}
    check() { # <name> <expected status> <status> <output> <ERE the output must match>
        if [ "$3" -eq "$2" ] && printf '%s\n' "$4" | grep -q -E -- "$5"; then
            printf 'ok   %s\n' "$1"
        else
            printf 'FAIL %s: status %s (wanted %s), output:\n%s\n' "$1" "$3" "$2" "$4"
            fails=$((fails + 1))
        fi
    }

    out=$(bash "$me" --start "$scratch/a.log" --wall 20 -- bash -c 'echo first; sleep 1; echo second'); status=$?
    check "a run that ends inside the wall returns its status and its last lines" 0 "$status" "$out" \
        '^  second$'
    check "and its verdict is the last line" 0 "$status" "$(printf '%s\n' "$out" | tail -n 1)" \
        '^wait: .*a\.log ended, [0-9]+ s since it started — wait: exit 0 after [0-9]+s$'

    out=$(bash "$me" --start "$scratch/b.log" --wall 20 -- bash -c 'exit 3'); status=$?
    check "a failing run returns the command's own status" 3 "$status" "$out" 'wait: exit 3 after'

    out=$(bash "$me" --start "$scratch/a.log" -- true 2>&1); status=$?
    check "a log that exists already is refused" 64 "$status" "$out" 'exists already'

    out=$(bash "$me" "$scratch/a.log" --wall 5 --tail 1); status=$?
    check "a log before its options is waited for as one after them" 0 "$status" "$out" \
        'a\.log ended'

    out=$(bash "$me" "$scratch/a.log" "$scratch/b.log" 2>&1); status=$?
    check "two logs are refused with a sentence, not the usage text" 64 "$status" "$out" \
        '^wait: one wait takes one log'

    out=$(bash "$me" "$scratch/a.log" --wall 2>&1); status=$?
    check "an option missing its value is refused by name" 64 "$status" "$out" \
        '^wait: --wall needs a value'

    out=$(bash "$me" --wall 600 "$scratch/a.log" 2>&1); status=$?
    check "a wall past the cache's lifetime is refused" 64 "$status" "$out" 'outside 1\.\.270'

    local t0=$SECONDS
    out=$(bash "$me" --start "$scratch/c.log" --wall 1 -- sleep 3); status=$?
    check "the wall comes first: 75, and the run named to re-issue" 75 "$status" "$out" \
        'still running .*re-issue tools/wait\.sh .*c\.log'
    [ $((SECONDS - t0)) -le 3 ] || check "the call returned at its wall" 0 1 "took $((SECONDS - t0)) s" x
    local pid pgid
    read -r pid pgid _ < "$scratch/c.log.pid"
    check "the run leads a process group of its own, not the caller's" 0 \
        "$([ "$pgid" = "$pid" ] && [ "$pgid" != "$(own_group)" ] && echo 0 || echo 1)" \
        "pid $pid, pgid $pgid" '.'
    out=$(bash "$me" --wall 20 "$scratch/c.log"); status=$?
    check "the run outlived the watcher, and a second wait picks it up" 0 "$status" "$out" 'c\.log ended'

    bash "$me" --start "$scratch/d.log" --wall 1 -- sleep 60 > /dev/null
    out=$(bash "$me" --stop "$scratch/d.log"); status=$?
    check "--stop ends the process group" 0 "$status" "$out" 'stopped with TERM'
    out=$(bash "$me" --wall 5 "$scratch/d.log"); status=$?
    check "a group gone without its end line is 70, said at once" 70 "$status" "$out" 'it was killed'

    printf 'building\nbounded: exit 4 after 9s; peak 1.0 GiB resident\n' > "$scratch/e.log"
    out=$(bash "$me" --wall 5 "$scratch/e.log"); status=$?
    check "a log it did not start ends on tools/bounded.sh's closing line" 4 "$status" "$out" \
        'ended — bounded: exit 4'
    printf 'still going\n' > "$scratch/f.log"
    out=$(bash "$me" --wall 1 --until '^done$' "$scratch/f.log"); status=$?
    check "--until without its line waits out the wall" 75 "$status" "$out" 'still running'

    if [ "$fails" -eq 0 ]; then
        printf 'wait.sh self-test: every case holds\n'
        exit 0
    fi
    printf 'wait.sh self-test: %s case(s) failed\n' "$fails"
    exit 1
}

if [ -n "$self_test" ]; then
    self_test
elif [ -n "$stop_log" ]; then
    [ ${#logs[@]} -eq 0 ] || refuse "--stop $stop_log takes no second log (${logs[0]})"
    stop "$stop_log"
elif [ -n "$start_log" ]; then
    [ ${#logs[@]} -eq 0 ] || refuse "--start $start_log names its own log, so ${logs[0]} is one too many"
    start "$start_log" "$@"
    wait_for "$start_log"
else
    [ $# -eq 0 ] || refuse "a command after -- is --start's: tools/wait.sh --start <log> -- <command>"
    [ ${#logs[@]} -eq 1 ] || refuse "one wait takes one log, not ${#logs[@]}: tools/wait.sh [--wall S] [--tail N] [--until ERE] <log>, the options before the log or after it"
    wait_for "${logs[0]}"
fi
