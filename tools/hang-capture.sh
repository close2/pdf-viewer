#!/usr/bin/env bash
# One supervised run on the shared card that may hang it — one device test, or a command such as a
# parallel suite — under the shared lock from before the run starts until the card has come back
# (ADR 2668 and ADR 2675, the owner's A776 and A777).
#
#   tools/hang-capture.sh [--no-radv-debug] [--wall <s>] <out-dir> -- <test binary> <test name> [args…]
#   tools/hang-capture.sh --command [--no-radv-debug] [--wall <s>] <out-dir> -- <command> [args…]
#
# The first form runs one test `--exact`; the second runs a command as given — a `cargo nextest run
# --archive-file` over binaries archived before, so that several test processes hold devices on the
# card together, as a parallel suite does (ADR 2675). `--no-radv-debug` leaves `RADV_DEBUG` unset,
# because its hang mode serialises every draw and submission and so is a second change to the run
# (trap 318); `--wall` replaces the 60 s bound on the run.
#
# Whatever is run is built before this is called, outside any lock: the lock is held for device work
# only. Then, in order, each step a line with its time in <out-dir>/run.log:
#
#   1. the shared lock (`/run/lock/gpu.lock`), waited for up to 10 min and held across every later
#      step: the script runs itself again under the machine's `with-gpu-lock`, so the whole
#      procedure below, and not the test alone, is the command the wrapper's `flock` holds the lock
#      for;
#   2. the run, on RADV (`WGPU_BACKEND=vulkan` and the radeon ICD — ADR 2663), with `RADV_DEBUG=hang`
#      unless `--no-radv-debug`, under its wall. Every process's section hook is pointed at one
#      private lock file in <out-dir>, because it would otherwise wait forever on the lock this script
#      holds, and that file's hang marker (ADR 2665 §3) is cleared first; the processes of a
#      `--command` run serialise their device sections on it among themselves, as on the shared one;
#   3. the kernel's log since the run began, kept in <out-dir>/kernel.txt. If it shows a ring
#      timeout or a reset, the script waits — still inside the lock — for `GPU reset(n) succeeded`
#      and then for `vulkaninfo --summary` on RADV to exit 0, for at most 10 min in all;
#   4. only then is the lock released, by closing the descriptor as the script ends.
#
# A RADV hang dump (`~/radv_dumps_<pid>_<time>`) the run leaves is moved into <out-dir>.
#
# `with-gpu-lock <test>` alone would release the lock when the test ends, which is before the
# kernel has finished resetting a card the test hung, and the reset is the moment the other user's
# work must be kept off the card; so the wrapper is given this whole procedure. Exit status: the
# test's, 75 where the lock was not had, or 70 where the card did not come back inside the bound
# (the lock is released then too, and the log says so).
set -uo pipefail

all_args=("$@")
mode=test
radv_debug=hang
test_wall_s=60
while :; do
    case "${1:-}" in
        --command) mode=command; shift ;;
        --no-radv-debug) radv_debug=; shift ;;
        --wall) test_wall_s=${2:?hang-capture: --wall needs seconds}; shift 2 ;;
        *) break ;;
    esac
done
out=${1:?usage: tools/hang-capture.sh [--command] [--no-radv-debug] [--wall <s>] <out-dir> -- <run…>}
shift
[ "${1:-}" = "--" ] || { echo "hang-capture: expected -- after the output directory" >&2; exit 64; }
shift
binary=${1:?hang-capture: nothing to run}
if [ "$mode" = test ]; then
    test_name=${2:?hang-capture: no test name}
    shift 2
    run=("$binary" "$test_name" --exact --nocapture "$@")
else
    test_name="$*"
    run=("$@")
fi

shared_lock=/run/lock/gpu.lock
radv_icd=/usr/share/vulkan/icd.d/radeon_icd.json
recovery_bound_s=600

mkdir -p "$out"
log="$out/run.log"
say() { printf '%s %s\n' "$(date '+%H:%M:%S.%3N')" "$*" | tee -a "$log"; }

[ "$mode" = command ] || [ -x "$binary" ] || { say "the test binary $binary is not executable"; exit 66; }

# The first invocation only waits for the lock and hands this same script to the wrapper; the
# second, under it, does the work. The output directory is fresh, so `lock held` in its log can only
# be this run's.
if [ -z "${HANG_CAPTURE_LOCKED:-}" ]; then
    [ ! -e "$log" ] || { echo "hang-capture: $log exists; give a fresh output directory" >&2; exit 64; }
    say "waiting for the shared lock $shared_lock"
    GPU_LOCK_TIMEOUT=600 with-gpu-lock env HANG_CAPTURE_LOCKED=1 "$0" "${all_args[@]}"
    status=$?
    grep -q "lock held" "$log" || { say "the shared lock was not had inside 600 s; nothing ran"; exit 75; }
    say "shared lock released"
    exit "$status"
fi
locked_at=$(date +%s)
say "lock held"

private_lock="$out/inner.lock"
: >"$private_lock"
rm -f "$private_lock.pdf-viewer-hung"
since=$(date '+%Y-%m-%d %H:%M:%S')
before_dumps=$(ls -d "$HOME"/radv_dumps_* 2>/dev/null | sort)

say "test start (RADV_DEBUG=${radv_debug:-unset}, wall ${test_wall_s} s): $test_name"
debug_env=(-u RADV_DEBUG)
[ -z "$radv_debug" ] || debug_env=(RADV_DEBUG="$radv_debug")
env "${debug_env[@]}" WGPU_BACKEND=vulkan VK_ICD_FILENAMES="$radv_icd" \
    PDF_VIEWER_GPU_LOCK="$private_lock" PDF_VIEWER_GPU_LOCK_LOG="$out/holds.log" \
    timeout --kill-after=10 "$test_wall_s" "${run[@]}" \
    >"$out/test.log" 2>&1
status=$?
say "test end: exit $status"
[ ! -e "$private_lock.pdf-viewer-hung" ] || say "hang marker left: $(tr '\n' ' ' <"$private_lock.pdf-viewer-hung")"

# A job the kernel times out is reported ten seconds after it stuck, so a test that did not pass is
# given that long before the kernel's log is read for the first time.
if [ "$status" -ne 0 ]; then
    say "the test did not pass; waiting 12 s for the kernel's ring timeout before reading its log"
    sleep 12
fi

kernel() { journalctl -k --since "$since" --no-pager >"$out/kernel.txt" 2>&1; }
kernel
card_back=yes
if grep -qE 'ring [a-z0-9_.]+ timeout|GPU reset begin|priv_reg_irq|VM fault' "$out/kernel.txt"; then
    say "the kernel logged a fault or timeout:"
    grep -E 'timeout|Process information|GPU reset|VRAM|priv_reg|VM fault' "$out/kernel.txt" | tee -a "$log"
    card_back=no
    deadline=$((locked_at + recovery_bound_s))
    if grep -q 'GPU reset begin' "$out/kernel.txt" || grep -qE 'timeout, signaled seq' "$out/kernel.txt"; then
        until grep -qE 'GPU reset\([0-9]+\) succeeded' "$out/kernel.txt"; do
            [ "$(date +%s)" -lt "$deadline" ] || break
            sleep 2
            kernel
        done
        if grep -qE 'GPU reset\([0-9]+\) succeeded' "$out/kernel.txt"; then
            say "reset finished: $(grep -E 'GPU reset\([0-9]+\) succeeded' "$out/kernel.txt" | tail -1)"
        else
            say "no finished reset inside the bound"
        fi
    fi
fi

# The card is ready again when a device can be brought up on it; probed after every run, hang or
# none, because the release below hands it to somebody else.
deadline=$((locked_at + recovery_bound_s))
while :; do
    if VK_ICD_FILENAMES="$radv_icd" timeout 30 vulkaninfo --summary >"$out/probe.txt" 2>&1; then
        say "device probe: vulkaninfo --summary on RADV exit 0"
        card_back=yes
        break
    fi
    say "device probe failed (exit $?)"
    [ "$(date +%s)" -lt "$deadline" ] || { card_back=no; break; }
    sleep 5
done
kernel

for dump in $(ls -d "$HOME"/radv_dumps_* 2>/dev/null | sort); do
    if ! grep -qxF "$dump" <<<"$before_dumps"; then
        mv "$dump" "$out/" && say "RADV dump kept: $out/$(basename "$dump")"
    fi
done

held=$(($(date +%s) - locked_at))
if [ "$card_back" = yes ]; then
    say "releasing the shared lock after ${held} s"
    exit "$status"
fi
say "the card did not come back inside ${recovery_bound_s} s; releasing the lock after ${held} s — look before any further device run"
exit 70
