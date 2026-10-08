#!/usr/bin/env bash
#
# A batch of parallel rounds in one worktree, and the merge that follows it.
#
# Six rounds run in ONE worktree on ONE short-lived branch, each briefed with the ledger rows it
# must close, and the merge runs `doc/todo/02` §2's tier 2 and tier 3 once for all of them
# (ADR 1036: the corpus walks caught one self-introduced defect in 98 sessions and raised
# fourteen false alarms at ~25 minutes a round; once per batch they take ~12 minutes and are
# believed). `tools/worktree.sh` is the per-round shape with a build directory each; this one
# has one build directory for every batch, named in the worktree's `.cargo/config.toml`. Not the
# main checkout's: cargo names a path package's artefacts relative to its workspace root, so the
# main checkout and the worktree write the same files, and one built in the main checkout after
# the worktree's sources were written is taken as fresh by the worktree — a `conformance` built
# there reads the main checkout's ledger from every sibling's `cargo test` (trap 50's shape, ADR
# 1440). One directory for every batch rather than one each, because the worktree's path is the
# same every batch, so the second batch finds it warm and a cold build is paid once.
#
#   tools/batch.sh open  batch-1038-1043   # worktree at /home/AI/pdf-viewer-rounds, guard on
#   tools/batch.sh gates                   # tiers 2 and 3, one line per gate, into batch-gates.log
#   tools/batch.sh raster-examples         # CI's raster examples with --check under Xvfb (one gate of those)
#   tools/batch.sh arms [<dir>]            # HEAD's six corpus arms, by page, once a batch (open runs it)
#   tools/batch.sh check                   # what a merge would otherwise look at by hand, one line each
#   tools/batch.sh commit /path/message    # stage the whole population by name, count it, commit
#   tools/batch.sh install                 # after the fast-forward: what a person runs, into main's target/
#   tools/batch.sh close batch-1038-1043   # after `git merge --ff-only` on main: remove both
#
# `commit`, the fast-forward and `close` are three commands, run one at a time with each one's
# output read before the next is typed. Chained with `;` or `&&` they are how batch thirty-five's
# worktree was deleted with 128 files in it (ADR 1313): a refused `git add` let a commit of one
# deletion through, the fast-forward took that, and the close removed the tree. `close` now
# refuses a worktree holding anything uncommitted outside `scratchpad/`, and offers no `--force`.
#
# Checked by `cargo test -p conformance --test batch`, which runs this script against a
# throwaway repository (`BATCH_WORKTREE` moves the worktree there; nothing else reads it).
#
# The loop itself is `doc/todo/02` §8. The gitlink guard is the same one `tools/worktree.sh`
# carries, for the same incident: a symlink where git expects a submodule is staged as a blob
# by any blanket `git add`, and `git status` cannot see it because the index already agrees
# with the working tree (doc/habits/tests-gates-and-reports.md).

set -euo pipefail

root=$(dirname "$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --path-format=absolute --git-common-dir)")
wt=${BATCH_WORKTREE:-/home/AI/pdf-viewer-rounds}
log=${BATCH_GATES_LOG:-/home/AI/batch-gates.log}

# Every subcommand runs under the agent's task budget, RLIMIT_NPROC, read from `tools/bounded.sh`,
# where the figure is written once: `gates`, `open`'s warm-up, the raster examples and `install`
# build and walk, and `check` runs cargo. The limit the calling shell held is read first, so that
# `check` can say whether the merge that ran it was under the bound itself (trap 116, ADR 1612). A
# limit already at or under the budget is kept, since a shell may lower its own but not raise it.
tasks_where_called=$(ulimit -u)
task_budget=$("$(dirname "${BASH_SOURCE[0]}")/bounded.sh" --task-budget)
[ "$tasks_where_called" != unlimited ] && [ "$tasks_where_called" -le "$task_budget" ] ||
    ulimit -u "$task_budget"

# The batch directory's `debug` profile against `doc/environment.md`'s hundred-gigabyte rule.
# Printed and never acted on: the prune is the orchestrator's, made between `close` and the next
# `open` with no round running, and `open`'s warm build is the first thing to write there again,
# so `open` says it before the worktree exists and `close` after the worktree is gone. The rule
# is in KiB (`du -sk`) so a test can plant a tree over a smaller one (ADR 1526).
debug_over_the_rule() {
    local target=${BATCH_TARGET_DIR:-/home/AI/cargo-target/pdf-viewer-batch}
    local rule=${BATCH_DEBUG_RULE_KIB:-104857600} size
    [ -d "$target/debug" ] || return 0
    size=$(du -sk "$target/debug" 2>/dev/null | cut -f1)
    [ -n "$size" ] && [ "$size" -gt "$rule" ] || return 0
    echo "$target/debug is $(du -sh "$target/debug" 2>/dev/null | cut -f1), over the 100 GB rule in doc/environment.md; the prune, between close and open with no round running:"
    echo "    rm -rf $target/debug"
}

open_batch() {
    local branch=$1
    [ -e "$wt" ] && { echo "$wt exists — close the previous batch first"; return 1; }
    debug_over_the_rule
    git -C "$root" worktree add -q -b "$branch" "$wt" HEAD
    # Only where the tree's own `.gitignore` says the file is a checkout's own, so that it can
    # never become part of a batch's population.
    if git -C "$wt" check-ignore -q .cargo/config.toml; then
        mkdir -p "$wt/.cargo"
        printf '[build]\ntarget-dir = "%s"\n' \
            "${BATCH_TARGET_DIR:-/home/AI/cargo-target/pdf-viewer-batch}" > "$wt/.cargo/config.toml"
    fi
    for r in doc/md doc/pdfa corpus-cache tmp fuzz/corpus fuzz/artifacts; do
        [ -e "$root/$r" ] || continue
        rm -rf "${wt:?}/$r"; ln -sfn "$root/$r" "$wt/$r"
    done
    for f in "$root"/doc/*.pdf; do [ -e "$f" ] && ln -sfn "$f" "$wt/doc/$(basename "$f")"; done
    # Submodules: link the content, keep the gitlink, pin it. Population derived from the index.
    while read -r sha path; do
        rm -rf "${wt:?}/$path"; ln -sfn "$root/$path" "$wt/$path"
        git -C "$wt" update-index --cacheinfo "160000,$sha,$path"
        git -C "$wt" update-index --skip-worktree -- "$path"
    done < <(git -C "$wt" ls-files --stage | awk '$1 == "160000" { print $2, $4 }')
    echo "$branch: $wt  (status: $(git -C "$wt" status --short | wc -l) changed; must be 0)"
    warm
    arms_at_open
}

# The batch's first build, started by `open` and left running while the briefs are written, so
# that the rounds find the build directory warm rather than six of them meeting it cold at once:
# cargo's lock on a profile's directory queues every other `dev` build behind the first, and the
# first is the whole workspace (ADR 1451 has the figures). Detached, because its cost is the
# orchestrator's, paid while it writes; a round that arrives first waits on cargo's lock exactly as
# it would wait on a sibling. Skipped where the tree has no workspace manifest (the throwaway
# repository `tests/batch.rs` opens) or `BATCH_WARM=0`. Only `dev`: the directory persists, so the
# `release` and `gates` profiles already hold their dependencies, and what they would rebuild here
# is the merge's workspace crates, which the first sibling to edit a low crate rebuilds again
# (ADR 1463 has the figures).
warm() {
    [ "${BATCH_WARM:-1}" = 0 ] || [ ! -f "$wt/Cargo.toml" ] && return 0
    detach scratchpad/open/build.log "warming the build directory" cargo build --workspace --all-targets
}

# `detach LOG WHAT COMMAND…` starts COMMAND in the worktree in a session of its own, every standard
# stream on LOG (relative to the worktree) or `/dev/null`, prints WHAT with the pid, and returns at
# once. The `;` after `cd` is the construction: `&` binds looser than `&&`, so in `cd … && setsid …
# &` it would put the whole list in a background copy of this shell, and that copy holds the
# caller's standard output open until the command ends — a caller reading the pipe to its end, as a
# command substitution or an agent's shell does, waits out the arms export's half-hour (ADR 1662).
detach() {
    local log=$1 what=$2
    shift 2
    mkdir -p "$wt/$(dirname "$log")"
    (cd "$wt" || exit 1
     setsid nohup "$@" > "$log" 2>&1 < /dev/null &
     echo "$what: pid $!, $log")
}

# HEAD's six corpus arms, exported once per batch so that a pixels round compares its pages against
# them and exports nothing: five pixel rounds in nine batches built and walked HEAD again for that
# comparison (ADR 1638 section 4). An arm is one lane (`cpu`, `gpu`, `compute`) at one scale (1, 4)
# of `render-raster`'s corpus gate, and the export is two files an arm: `<lane>-<scale>x.txt`, the
# gate's own `--nocapture` output, and `<lane>-<scale>x.tsv`, the per-page `PDFVIEWER_RASTER_TIMES`
# file whose fourth column is the frame's digest (ADR 1443) — the gate's printed output names only
# the pages that differ from the oracle, so a comparison by page reads the second. Into
# `/home/AI/arms-<first>/`, the batch's first session read off the branch name, or the directory
# given; `README`'s first line names the commit, which is what keys the export.
#
# **It is HEAD's only if the tree is.** So it refuses a worktree holding anything uncommitted
# outside `scratchpad/`, and `open` starts it before any round is briefed. The test binary and the
# sandbox worker are built once and copied out of the build directory, and all six arms run the
# copies under one hold of the heavy-walk lock: a sibling's edit, or a sibling's `--bins` rebuild,
# landing between two arms would otherwise be built into the second (trap 109), and the export
# would be of no commit. The copies' digests go into `README` and the copies are deleted. An export
# already complete for this commit is kept; one for another commit is refused, never overwritten.
arms_complete() {
    local out=$1 commit=$2 arm
    [ "$(head -1 "$out/README" 2>/dev/null | awk '{ print $4 }' | tr -d ,)" = "$commit" ] || return 1
    grep -q '^done ' "$out/README" && ! grep -q ' exit [0-9]' "$out/README" || return 1
    for arm in $arm_names; do [ -s "$out/$arm.tsv" ] || return 1; done
}
arm_names="cpu-1x gpu-1x compute-1x cpu-4x gpu-4x compute-4x"

export_arms() {
    cd "$wt" || return 1
    local out=${1:-} commit branch dirty
    commit=$(git -C "$wt" rev-parse HEAD)
    if [ -z "$out" ]; then
        branch=$(git -C "$wt" rev-parse --abbrev-ref HEAD)
        [[ "$branch" =~ ^batch-([0-9]+)-[0-9]+$ ]] ||
            { echo "arms: $branch does not name its first session as batch-<first>-<last>; give the directory"; return 1; }
        out=${BATCH_ARMS_ROOT:-/home/AI}/arms-${BASH_REMATCH[1]}
    fi
    dirty=$(population | tr '\0' '\n') || { echo "arms: cannot read $wt's status — nothing exported"; return 1; }
    [ -z "$dirty" ] || {
        echo "arms: $wt holds $(printf '%s\n' "$dirty" | grep -c .) uncommitted path(s) outside scratchpad/, so an export from it would be of no commit — export at open, or from a clean checkout of HEAD:"
        printf '%s\n' "$dirty" | head -10 | sed 's/^/    /'
        return 1
    }
    if [ -e "$out/README" ]; then
        local named; named=$(head -1 "$out/README" | awk '{ print $4 }' | tr -d ,)
        [ "$named" = "$commit" ] ||
            { echo "arms: $out holds the arms of ${named:-no commit}, not $commit — move it aside; an export is never overwritten by another commit's"; return 1; }
        arms_complete "$out" "$commit" && { echo "arms: $out holds every arm of $commit already"; return 0; }
    fi
    [ -f "$wt/Cargo.toml" ] || { echo "arms: no workspace at $wt"; return 1; }
    # The lock is the wrapper's, so that the export's queue and its hold are a line of the lock's
    # log as every other walk's are (ADR 1646), and what runs under it is this script's `arms-held`.
    # The lock first and the directory after it, so that a second export of the same directory
    # queues behind the first and then finds it complete rather than writing beside it.
    # A large walk, on the lock's first lane: its two builds run inside the hold and a cold build
    # is not held to a small walk's 6 GiB, and a killed export costs the batch its pixels baseline.
    # A round's small walks run beside it on the second lane (ADR 1684).
    "$wt/tools/bounded.sh" --lock --round arms --tree 12 -- \
        "$wt/tools/batch.sh" arms-held "$out" "$commit" "$(date +%s)"
}

# Whether this process runs under a hold of the heavy-walk lock. The wrapper keeps the lock's
# descriptor and hands its command a marker instead, `HEAVY_WALK_HELD_BY`, so a daemon the walk
# starts does not keep the lock after it (ADR 1674); whether the marker names an ancestor that holds
# the descriptor is the wrapper's question to answer, asked of it rather than answered twice. A
# descriptor this process has itself — a bare `flock <lock>` above it — is a hold as well.
holds_the_lock() { "$(dirname "${BASH_SOURCE[0]}")/bounded.sh" --held; }

# The export itself, run by `arms` under the lock it took: OUT, the commit it is of, and the second
# the lock was asked for. Not a command to type, and it refuses where it runs under no hold,
# since six arms walked beside another walk are what the lock exists to prevent.
arms_held() {
    cd "$wt" || return 1
    local out=$1 commit=$2 queued=$3 bin=$1/bin exe rc arm lane scale
    holds_the_lock ||
        { echo "arms-held: run by \`tools/batch.sh arms\`, under the heavy-walk lock it takes through tools/bounded.sh --lock"; return 1; }
    arms_complete "$out" "$commit" && { echo "arms: $out holds every arm of $commit already"; return 0; }
    mkdir -p "$out"
    printf 'HEAD arms of %s, exported %s from %s\nfiles: <lane>-<scale>x.txt = the corpus gate'"'"'s --nocapture output; <lane>-<scale>x.tsv = its PDFVIEWER_RASTER_TIMES file, one page a line: name, oracle ms, raster ms, frame digest, mean error\n' \
        "$commit" "$(date '+%Y-%m-%d %H:%M')" "$wt" > "$out/README"
    echo "lock: queued $(($(date +%s) - queued)) s, held from $(date '+%H:%M:%S')" >> "$out/README"
    # One build for every arm, and the test binary's path asked of Cargo rather than guessed (trap 15).
    if RAYON_NUM_THREADS=4 "$wt/tools/bounded.sh" --tree 12 -- cargo build --profile gates -p pdf-sandbox --bins > "$out/build.log" 2>&1 &&
        RAYON_NUM_THREADS=4 "$wt/tools/bounded.sh" --tree 12 -- cargo test --profile gates -p render-raster --test corpus --no-run \
            --message-format=json-render-diagnostics > "$out/build.json" 2>> "$out/build.log"; then
        exe=$(grep -o '"executable":"[^"]*"' "$out/build.json" | tail -1 | sed 's/^"executable":"//; s/"$//' || true)
    fi
    rm -f "$out/build.json"
    local worker; worker=$(dirname "${exe:-/nonexistent/x}")/../pdf-sandbox-worker
    if [ -z "${exe:-}" ] || [ ! -x "$exe" ] || [ ! -x "$worker" ]; then
        echo "build exit 1 — no test binary or no worker; build.log says why" >> "$out/README"
        echo "arms: the build failed — $out/build.log"; return 1
    fi
    mkdir -p "$bin"; cp "$exe" "$bin/corpus"; cp "$worker" "$bin/pdf-sandbox-worker"
    (cd "$bin" && sha256sum corpus pdf-sandbox-worker) | sed 's/^/built: /' >> "$out/README"
    if [ -n "$(population | tr '\0' '\n')" ]; then
        echo "tree exit 1 — a path changed in $wt while the build ran, so the binaries are of no commit" >> "$out/README"
        rm -rf "$bin"
        echo "arms: the tree changed under the build — nothing walked"; return 1
    fi
    for arm in $arm_names; do
        lane=${arm%-*}; scale=${arm#*-}; scale=${scale%x}
        (cd "$wt/crates/render-raster" &&
            RAYON_NUM_THREADS=4 PDFVIEWER_RASTER_SCALE=$scale PDFVIEWER_RASTER_COVERAGE=$lane \
            PDFVIEWER_RASTER_TIMES=$out/$arm.tsv PDF_SANDBOX_WORKER=$bin/pdf-sandbox-worker \
            "$wt/tools/bounded.sh" --tree 12 -- "$bin/corpus" --ignored --nocapture) > "$out/$arm.txt" 2>&1 && rc=0 || rc=$?
        if [ "$rc" -eq 0 ] && ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$out/$arm.txt"; then rc=98; fi
        [ "$rc" -eq 0 ] && [ -s "$out/$arm.tsv" ] || echo "$arm exit $rc" >> "$out/README"
        printf 'arms: %-11s exit=%s  %s page line(s)\n' "$arm" "$rc" "$(wc -l < "$out/$arm.tsv" 2>/dev/null || echo 0)"
    done
    rm -rf "$bin"
    echo "done $(date '+%Y-%m-%d %H:%M'), the lock held $(($(date +%s) - queued)) s from the queue's start" >> "$out/README"
    arms_complete "$out" "$commit" || { echo "arms: $out is incomplete — README names the arm"; return 1; }
    echo "arms: $out holds every arm of $commit"
}

# `open`'s call of the above, detached as `warm` is and for the same reason: its cost is the
# orchestrator's, paid while the briefs are written, and it reads the tree before any round edits it.
# Started second and beside the warm build rather than after it: the warm build is the rounds' and
# runs at the caller's priority, the export's builds and walks at the wrapper's nice 19 behind it,
# and the export's own build — the minute in which a round's first edit would make it refuse — is
# not pushed back by the warm build's two (ADR 1662). Skipped where `warm` is skipped, and by
# `BATCH_ARMS=0`.
arms_at_open() {
    [ "${BATCH_ARMS:-1}" = 0 ] || [ ! -f "$wt/Cargo.toml" ] && return 0
    detach scratchpad/open/arms.log "exporting HEAD's six corpus arms" "$wt/tools/batch.sh" arms
}

# One line per gate: name, exit, the seconds it ran, the seconds it queued for the lock before
# that, and the gate's own summary line. A failure's last thirty lines go beside the log under the
# gate's name, so a merge reads one file and opens one more. The two clocks are apart because they
# answer different questions: `wall` is what the gate costs — its build and its walk, which the
# test's own "finished in" leaves out — and `wait` is what the machine's other walks cost it; a
# dear gate is read off the first and never the second. `tools/state.sh gates-cost` prints them
# (ADR 1476).
#
# Every gate runs behind the heavy-walk lock with four rayon threads: the rounds take the same lock
# for their own corpus walks, so at most one heavy walk is on the machine at a time across the
# batch. On 2026-09-15 six rounds and a merge walked the corpus at once and the whole process was
# killed (raster_golden alone peaks past 7 GiB at twelve threads); the lock costs wall-clock and a
# kill costs the batch. The lock is taken by `tools/bounded.sh --lock`, `--round` the batch's
# branch, so that the merge's holds are lines of the lock's log beside its rounds' and
# `tools/state.sh gates-cost` reads both (ADR 1646, 1662); under the wrapper's ceilings, which are
# the ones every round runs these gates under, and at the caller's priority rather than the
# wrapper's nice 19, because a timing gate's band is a claim about the program a person runs.
#
# The two clocks are the wrapper's: its last line says how long the command ran once the lock was
# granted (`bounded: … after <n>s`, printed after a nested wrapper's own), so `wall` is that and
# `wait` the rest of the time since the lock was asked for. A line with no such sentence is a
# wrapper that never started the command, and its time is all `wall`.
#
# A gate in `clock_gates` is a clock run (`--clock`): its verdict is a time — a band, a floor, or a
# reference program held to a budget, which a loaded machine fails without a defect in either
# (`doc/todo/02` section 2, "Run the sequence on a quiet machine") — so it takes both of the lock's
# lanes and runs alone. Every other gate is a large walk, under the 12 GiB ceiling the gates were
# always run under, on the first lane (ADR 1684).
clock_gates="t2-transform-gate t2-turn_path t3-oracle t3-text_extract t3-render_raster t3-foreign_corpus"
run() {
    local name=$1; shift; local out rc asked ended wall clock=
    [[ " $clock_gates " == *" $name "* ]] && clock=1
    asked=$(date +%s)
    out=$(RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-4}" "$wt/tools/bounded.sh" --lock ${clock:+--clock} --tree 12 \
        --round "$merge_round" --nice 0 -- "$@" 2>&1) && rc=0 || rc=$?
    ended=$(date +%s)
    wall=$(printf '%s\n' "$out" | sed -n 's/^bounded: .* after \([0-9][0-9]*\)s.*/\1/p' | tail -1)
    [ -n "$wall" ] || wall=$((ended - asked))
    # A test line that ran nothing exits 0: `--ignored` over a file with no ignored test is green
    # while checking nothing. It is a failure here, wherever in the line `cargo test` stands —
    # behind `tools/bounded.sh` as much as at its head — and `tests/batch.rs` holds every line's
    # flag to its file's `#[ignore]` attributes before anything runs (ADR 1392).
    if [ "$rc" -eq 0 ] && [[ " $* " == *" cargo test "* ]] &&
        ! printf '%s\n' "$out" | grep -qE 'test result: [a-z]+\. [1-9][0-9]* passed'; then
        rc=98; out+=$'\nran zero tests — a green line that checked nothing (ADR 1392)'
    fi
    printf '%-24s exit=%-4s wall=%-6s wait=%-6s %s\n' "$name" "$rc" \
        "${wall}s" "$((ended - asked - wall))s" \
        "$(printf '%s\n' "$out" | grep -iE 'test result|documents|pages|passed|FAILED|panicked|ran zero tests' | tail -1 | cut -c1-150)" >> "$log"
    [ "$rc" -ne 0 ] && printf '%s\n' "$out" | tail -30 > "$log.fail.$name"
    return 0
}

gates() {
    cd "$wt"; : > "$log"; rm -f "$log".fail.* 2>/dev/null || true
    merge_round=$(git -C "$wt" rev-parse --abbrev-ref HEAD)
    # `tools/batch.sh gates > <the log>` makes this script's standard output the log itself at
    # offset nought, so the summary printed last lands over the first gate's line and that gate is
    # missing from every reading of the log. The summary is in the log already; it is printed only
    # where standard output is somewhere else. The comparison is `test -ef` in this shell, because
    # `/dev/stdout` inside a command substitution is the substitution's own pipe and never the log.
    local onto_the_log=
    [ /dev/stdout -ef "$log" ] && onto_the_log=1
    run build-sandbox  cargo build --profile gates -p pdf-sandbox --bins
    run build-hayro    cargo build --profile gates -p hayro-compare --bin pdfref-hayro
    run build-vfs      cargo build --profile gates -p pdf-vfs --bins
    run build-confined cargo build --profile gates -p viewer-confined --bins
    for t in corpus raster_golden script_corpus dates xmp; do
        run "t2-$t" cargo test --profile gates -p pdf-model --test "$t" -- --ignored --nocapture; done
    # The Tier 1 column of RFC 0008 section 6.7: every script Tier 0 does not run, run in its
    # document's realm with the `engine` feature built for this test binary alone, held to its
    # column's own constants. Bounded as a walk is, because a script's heap is the engine's to bound
    # and the walk is ninety thousand documents of them (ADR 1625).
    run t2-script_corpus_engine tools/bounded.sh --data 8 --tree 12 -- \
        cargo test --profile gates -p pdf-script --features engine --test script_corpus -- --ignored --nocapture
    run t2-jpeg2000       cargo test --profile gates -p pdf-model --test jpeg2000 -- --nocapture
    run t2-transform-gate cargo test --profile gates -p pdf-transform --test gate -- --ignored --nocapture
    run t2-on_disk        cargo test --profile gates -p pdf-syntax --test on_disk -- --ignored --nocapture
    # `release`, not `gates`: the bands in `doc/checks/turn-path.toml` are a claim about the
    # profile a person runs, and the gate prints without judging under any other (ADR 1513).
    run t2-turn_path      cargo test --release -p render-raster --test turn_path -- --ignored --nocapture
    # `release`, as CI's job builds them. The tree ceiling is a walk's 12 GiB, not a build's 8: the
    # release build of `raster-gpu`'s examples peaked at 7.85 GiB over the tree when it was measured,
    # inside a build's ceiling by too little to survive a heavier dependency (ADR 1575).
    run t2-raster_examples tools/bounded.sh --tree 12 -- "$wt/tools/batch.sh" raster-examples
    run t3-oracle         cargo test --profile gates -p pdf-model --test oracle -- --ignored --nocapture
    run t3-text_extract   cargo test --profile gates -p pdf-model --test text_extraction -- --ignored --nocapture
    run t3-selection      cargo test --profile gates -p viewer-core --test selection_census -- --ignored --nocapture
    run t3-accessibility  cargo test --profile gates -p viewer-core --test accessibility_census -- --ignored --nocapture
    run t3-save_round     cargo test --profile gates -p pdf-model --test save_round_trip -- --ignored --nocapture
    run t3-actions        cargo test --profile gates -p pdf-model --test actions -- --ignored --nocapture
    run t3-render_raster  cargo test --profile gates -p render-raster --test corpus -- --ignored --nocapture
    run t3-fixed_docs     cargo test --profile gates -p pdf-model --test fixed_documents -- --ignored --nocapture
    for t in writer_corpus split_corpus merge_corpus pages_corpus optimize_corpus foreign_corpus archive_corpus; do
        run "t3-$t" cargo test --profile gates -p pdf-transform --test "$t" -- --ignored --nocapture; done
    run t3-archive-val    cargo test --profile gates -p pdf-archive --test corpus -- --ignored --nocapture
    run t3-cross-check    cargo test --profile gates -p pdf-archive --test cross_check -- --ignored --nocapture
    run t3-vfs_write      cargo test --profile gates -p pdf-vfs --test write_corpus -- --ignored --nocapture
    run t3-vfs_read       cargo test --profile gates -p pdf-vfs --test read_corpus -- --ignored --nocapture
    run t3-awkward        cargo test --profile gates -p viewer-confined --test awkward_classes -- --ignored --nocapture
    echo "ALL GATES DONE — $(grep -c 'exit=0' "$log") of $(grep -cE 'exit=' "$log") green," \
        "$(awk '{ for (i = 1; i <= NF; i++) if ($i ~ /^wall=/) { sub(/^wall=/, "", $i); sub(/s$/, "", $i); t += $i } }
                END { print t + 0 }' "$log") s of gate wall time" >> "$log"
    [ -n "$onto_the_log" ] || tail -1 "$log"
}

# The examples `.github/workflows/ci.yml` runs with `--check`, read out of that step's `for` loop
# so that this gate and the owner's CI run one list; `raster-gpu`'s `tests/example_checks.rs` holds
# the step to `examples/`, and `tests/batch.rs` holds this reading to the same directory.
raster_example_names() {
    awk '/for example in/ { want = 1; next } want && /^ *do *$/ { exit }
         want { gsub(/\\/, ""); print }' "$wt/.github/workflows/ci.yml" | tr -s ' \t' '\n' | sed '/^$/d'
}

# Every raster example's `--check` under Xvfb, as CI's `raster-examples` job runs it: `cargo test`
# builds no example, so an assertion in one is a comment until something executes it, and two had
# gone stale where only the owner's CI ran them (ADR 1563). One line per example — its exit, its
# seconds, its log — and a summary the gate log reads; a failing example's lines are what `run`
# leaves in the `.fail.` file. The build is bounded apart from the runs, so a slow build is not read
# as a hung example, and each run is bounded by `timeout`, which signals the whole process group:
# `xvfb-run`, its server and the example go together. The exit status is the answer (ADR 1575).
raster_examples() {
    cd "$wt" || return 1
    local logs=${RASTER_EXAMPLES_LOGS:-$log.raster-examples} names name rc began passed=0 failed=0
    names=$(raster_example_names)
    if [ "${1:-}" = --list ]; then printf '%s\n' "$names"; return 0; fi
    [ -n "$names" ] || { echo "raster examples: no example read from ci.yml's --check step"; return 1; }
    command -v xvfb-run > /dev/null || { echo "raster examples: no xvfb-run on this machine"; return 1; }
    rm -rf "$logs"; mkdir -p "$logs"
    began=$(date +%s)
    timeout -k 30 "${RASTER_EXAMPLES_BUILD_SECONDS:-2400}" \
        cargo build --release -p raster-gpu --examples > "$logs/build.log" 2>&1 && rc=0 || rc=$?
    printf '%-18s exit=%-4s %5ss  %s\n' "(build)" "$rc" "$(($(date +%s) - began))" "$logs/build.log"
    [ "$rc" -eq 0 ] || { echo "raster examples: the build FAILED, no example ran"; return 1; }
    for name in $names; do
        began=$(date +%s)
        timeout -k 10 "${RASTER_EXAMPLE_SECONDS:-600}" \
            xvfb-run -a cargo run --release -p raster-gpu --example "$name" -- --check \
            > "$logs/$name.log" 2>&1 && rc=0 || rc=$?
        printf '%-18s exit=%-4s %5ss  %s\n' "$name" "$rc" "$(($(date +%s) - began))" "$logs/$name.log"
        if [ "$rc" -eq 0 ]; then passed=$((passed + 1)); else failed=$((failed + 1)); fi
    done
    if [ "$failed" -eq 0 ]; then
        echo "raster examples: $passed passed, 0 failed"
    else
        echo "raster examples: $passed passed, $failed FAILED"
        return 1
    fi
}

# What a merge checks by hand before it commits, one line each, from inside the worktree.
#
# Every one of them has bitten a merge, and every one was a command somebody had to remember: a
# stray file with no place in the tree, a specification written into a worktree that dies with it
# (sessions 1071, 1079), a `\uXXXX` in a ledger note that blocks tier 1 for all six rounds, a record
# over `doc/todo/02` section 8's budget, a record numbered behind one already committed, an escaped
# section sign that hides a clause citation from the gate that reads them, a symlink staged where
# git expects a submodule, and a formatting difference that fails tier 1 after the commit. Each is a
# command remembered and therefore a command forgotten; this is one command, and its exit status is
# the answer.
#
# **It reports a sibling's in-flight files as findings, and that is the instrument working.** Run
# mid-batch it says what is there now; run at the merge, after every round is in, what it says is
# what the commit would carry.
check_batch() {
    cd "$wt" || return 1
    local bad=0 found

    # The owner's answers that landed in the main checkout and no commit holds yet, newest first,
    # and what is open once they are counted: `tools/main-checkout.py`'s first two lines, repeated
    # here because a merge runs this and a list of tracked files calls every such question open
    # (ADR 1588). Read-only against the main checkout, and never a finding: they are the owner's.
    PYTHONDONTWRITEBYTECODE=1 python3 tools/main-checkout.py --answers | sed -n '2,$p' || true

    # The agent's tasks now, every process and thread of this user, and the limit the shell that
    # ran this held: a merge made without `ulimit -u` reads here as the system's figure above the
    # budget. Never a finding — this script holds itself to the budget whatever it was given — but
    # the gates and builds the merge runs beside it were not (trap 116, ADR 1612).
    local held where
    held=$(ps -u "$(id -un)" -o nlwp= | awk '{ s += $1 } END { print s + 0 }')
    where="ulimit -u $tasks_where_called where called"
    if [ "$tasks_where_called" = unlimited ] || [ "$tasks_where_called" -gt "$task_budget" ]; then
        where+=", ABOVE the budget of $task_budget"
    fi
    printf 'tasks of %s now, and the bound   %s; %s\n' "$(id -un)" "$held" "$where"

    # A file this tree has no place for. The extensions are what a round legitimately adds; a
    # binary, an archive, an editor's leavings and a regenerated header are none of them, and the
    # last is the one that looks innocent — a tracked `include/quorra.h` is fine and an untracked
    # `.h` is somebody's copy. An ICC profile is the one binary a round does legitimately add, and
    # it is admitted **by path rather than by extension**: `data/icc/` is the only place one
    # belongs, `NOTICE` and `data/icc/PROVENANCE.md` are what it owes, and a `.icc` anywhere else
    # is still somebody's copy. `scratchpad/` is the rounds' and never committed (`commit`
    # refuses it), so it is not a finding here either — and it is left out by `population`, the
    # function `commit` stages from, so a path git would print quoted (a space, a non-ASCII
    # character) is excluded by its real directory rather than by the spelling of its quotes. A
    # workspace's `Cargo.lock` is admitted by name: every workspace here tracks its lock (ADR 1439).
    found=$(untracked_paths |
        grep -vE '\.(rs|md|toml|tsv|txt|py|pem|der|crt|xfdf|j2k|jp2|sh|jpg|patch)$' |
        grep -vE '^data/icc/[^/]+\.icc$' | grep -vE '(^|/)Cargo\.lock$' || true)
    printf 'untracked, unexpected extension  %s\n' "$([ -z "$found" ] && echo none || echo "$(printf '%s\n' "$found" | wc -l) file(s)")"
    [ -z "$found" ] || { printf '%s\n' "$found" | sed 's/^/    /'; bad=1; }

    # A regular PDF under this checkout's doc/ dies with the worktree, and the next merge sees the
    # corpus shrink for a cause nobody made. `close` refuses on the same population.
    found=$(find "$wt/doc" -maxdepth 1 -name '*.pdf' -type f 2>/dev/null || true)
    printf 'regular PDF under doc/           %s\n' "$([ -z "$found" ] && echo none || echo present)"
    [ -z "$found" ] || { printf '%s\n' "$found" | sed 's/^/    /'; printf '    move each to %s/doc/ and symlink it here\n' "$root"; bad=1; }

    # A ledger note is a TOML basic string: `\uXXXX` is not in the subset this tree writes, and one
    # of them blocks tier 1 for every round in the worktree at once.
    found=$(grep -nE '\\u[0-9a-fA-F]{4}' doc/conformance/ledger.toml 2>/dev/null || true)
    printf 'ledger \\uXXXX escapes            %s\n' "$([ -z "$found" ] && echo none || echo "$(printf '%s\n' "$found" | wc -l) line(s)")"
    [ -z "$found" ] || { printf '%s\n' "$found" | cut -c1-120 | sed 's/^/    /'; bad=1; }

    # A record over budget. The count and the bound are the conformance crate's, so there is one
    # copy of the figure and it is the one that fails.
    local records
    # The test fails for a record without its `**Gates.**` paragraph too, so the verdict says
    # which, and the listing is every record line the test names — a grep that matched none would
    # end the whole check under `set -e` with the lines after it never printed.
    records=$(cargo test -q -p conformance --test records -- --nocapture 2>&1) && found= || found=$records
    local verdict=none
    if [ -n "$found" ]; then
        verdict="failing, not on the budget"
        grep -q 'over the budget' <<<"$found" && verdict=over
    fi
    printf 'records over budget              %s\n' "$verdict"
    [ -z "$found" ] || {
        printf '%s\n' "$found" | grep -E '^doc/history/' | sed 's/^/    /' || true
        bad=1
    }

    # A record numbered behind one that is already committed. `doc/history/` is read by `ls`
    # order, so a record whose number a committed one already carries sorts into somebody else's
    # round and is invisible where it belongs — and the number is the only thing in the file that
    # says which round wrote it.
    #
    # **Untracked records only**, which is what a round adds: a tracked one's number is committed
    # by definition, so including it would fire on every run and stop being a signal (trap 39).
    # The leading `<n>-` and nothing else is the number — a file name carrying `19005` reads as a
    # record from the twenty-thousandth session to a grep that takes any digits in the path.
    local newest
    newest=$(git ls-files doc/history |
        sed -n 's|^doc/history/\([0-9]\{1,\}\)-.*\.md$|\1|p' | sort -n | tail -1)
    found=$(untracked_paths | grep '^doc/history/' |
        sed -n 's|^\(doc/history/\([0-9]\{1,\}\)-.*\.md\)$|\2 \1|p' |
        awk -v newest="${newest:-0}" '$1 <= newest { printf "%s is numbered %s, behind the committed %s\n", $2, $1, newest }' || true)
    printf 'record numbered behind a merged one %s\n' "$([ -z "$found" ] && echo none || echo "$(printf '%s\n' "$found" | wc -l) record(s)")"
    [ -z "$found" ] || { printf '%s\n' "$found" | sed 's/^/    /'; bad=1; }

    # An escaped section sign in a doc comment of a file this batch changed. `tools/conformance`'s
    # `citation.rs` finds a clause citation by looking for `§` **as a character** on every line, so
    # `\u{a7}7.11.5` is a citation no gate can see, no ratchet counts, and rustdoc prints literally.
    #
    # **The section sign and no other escape**, and that is calibrated rather than chosen: of the
    # fifteen `\u{...}` escapes in `crates/`'s doc comments, five are escaped section signs and
    # every one of them names a real ISO 32000-2 clause that is invisible to the scan; the other
    # ten name a character that is invisible on the page — a soft hyphen, an ESCAPE, a replacement
    # character — or an ellipsis, and no gate reads any of those. A rule wide enough to take in the
    # ten would fire on every run.
    #
    # **Scoped to the lines this batch added**, for the same reason, and to added lines rather than
    # to changed files: scoped by file it reported one of the five against the round that happened
    # to be editing that file for something else, which is a finding addressed to the wrong person.
    # What this catches is a round writing one now, which is what batch 28 lost an hour to.
    local untracked
    untracked=$(untracked_paths | grep -E '^crates/.*\.rs$' || true)
    found=$( { git diff -U0 -- 'crates/*.rs'; git diff -U0 main...HEAD -- 'crates/*.rs'
               printf '%s\n' "$untracked" | while IFS= read -r path; do
                   [ -z "$path" ] || sed 's/^/+/' "$path"
               done; } 2>/dev/null |
        grep -E '^\+\s*(///|//!).*\\u\{[aA]7\}' || true)
    printf 'escaped section sign in a doc comment %s\n' "$([ -z "$found" ] && echo none || echo "$(printf '%s\n' "$found" | wc -l) line(s)")"
    [ -z "$found" ] || { printf '%s\n' "$found" | cut -c1-140 | sed 's/^/    /'; bad=1; }

    # A symlink staged where git expects a submodule. `git status` cannot see it, because the index
    # already agrees with the working tree.
    found=$(git ls-files --stage -- $(git config -f .gitmodules --get-regexp '\.path$' | awk '{print $2}') |
        awk '$1 != "160000" { print $4 }' || true)
    printf 'submodules staged as gitlinks    %s\n' "$([ -z "$found" ] && echo "all $(git config -f .gitmodules --get-regexp '\.path$' | wc -l)" || echo "$(printf '%s\n' "$found" | wc -l) staged as a blob")"
    [ -z "$found" ] || { printf '%s\n' "$found" | sed 's/^/    /'; bad=1; }

    # A symlink at the worktree's root that is not one of the two the worktree is opened with. A
    # round that links a scratch tree in to run something leaves a path every later round and the
    # merge can follow into a directory that is about to be deleted; two were left last batch and
    # removed by hand. `corpus-cache` and `tmp` are the standing set.
    found=$(find "$wt" -mindepth 1 -maxdepth 1 -type l -printf '%f -> %l\n' 2>/dev/null |
        grep -vE '^(corpus-cache|tmp) -> ' || true)
    printf 'symlink at the root, not standing %s\n' "$([ -z "$found" ] && echo none || echo "$(printf '%s\n' "$found" | wc -l) link(s)")"
    [ -z "$found" ] || { printf '%s\n' "$found" | sed 's/^/    /'; bad=1; }

    # A round's scratch instrumentation left in a source a sibling builds: a `TMPFRAME`-shaped
    # timer, or a name carrying a round's own number (`r1234`). One stayed in a sibling's path for
    # a whole round last batch and broke three clippy runs. `TMPDIR` is the environment's own name
    # and is not scratch; this function's source is left out, because it has to spell both shapes.
    found=$(grep -rnIE '\bTMP[A-Z]+\b|\br1[0-9]{3}\b' crates raster tools --exclude-dir=target 2>/dev/null |
        grep -v '^tools/batch\.sh:' | grep -vE '\bTMPDIR\b' || true)
    printf 'scratch identifier in a source    %s\n' "$([ -z "$found" ] && echo none || echo "$(printf '%s\n' "$found" | wc -l) line(s)")"
    [ -z "$found" ] || { printf '%s\n' "$found" | cut -c1-140 | sed 's/^/    /'; bad=1; }

    # A workspace member that is not a crate this tree tracks (trap 114). `cargo new` under
    # `scratchpad/r<n>/` writes its directory into the root `Cargo.toml`'s `members`, and a
    # directory a glob matches is read as a crate whether or not it holds a manifest — so a crate a
    # round made, or a `__pycache__` beside `tools/`'s crates, stops every sibling's cargo at once.
    # Each entry of `members`, a glob expanded the way cargo expands it, must be a directory whose
    # `Cargo.toml` git tracks, and none may be under `scratchpad/`. A worktree with no root manifest
    # has no workspace to break.
    found=
    if [ -f Cargo.toml ]; then
        found=$(git ls-files -z -- '*Cargo.toml' | tr '\0' '\n' |
            PYTHONDONTWRITEBYTECODE=1 python3 -c '
import glob, os, sys, tomllib
tracked = set(line.rstrip("\n") for line in sys.stdin)
with open("Cargo.toml", "rb") as manifest:
    members = tomllib.load(manifest).get("workspace", {}).get("members", [])
for entry in members:
    paths = sorted(glob.glob(entry)) if glob.has_magic(entry) else [entry]
    for path in paths:
        path = os.path.normpath(path)
        if glob.has_magic(entry) and not os.path.isdir(path):
            continue
        if path == "scratchpad" or path.startswith("scratchpad/"):
            print(f"{path}: under scratchpad/ (members entry {entry!r})")
        elif f"{path}/Cargo.toml" not in tracked:
            print(f"{path}: no tracked Cargo.toml (members entry {entry!r})")
' 2>&1 || true)
    fi
    printf 'workspace member not a tracked crate %s\n' "$([ -z "$found" ] && echo none || echo "$(printf '%s\n' "$found" | wc -l) member(s)")"
    [ -z "$found" ] || { printf '%s\n' "$found" | sed 's/^/    /'; bad=1; }

    # A `__pycache__` where a member glob or a crate's own tree would read it, which is what a
    # Python run without `PYTHONDONTWRITEBYTECODE=1` leaves behind (trap 114). Deeper than the
    # globs on purpose: one inside a crate is not a member, but it is the same run's leavings.
    found=$(find tools crates raster -name target -prune -o -name __pycache__ -type d -print 2>/dev/null || true)
    printf '__pycache__ under tools/ or crates/ %s\n' "$([ -z "$found" ] && echo none || echo "$(printf '%s\n' "$found" | wc -l) director(ies)")"
    [ -z "$found" ] || { printf '%s\n' "$found" | sed 's/^/    /'; bad=1; }

    # And the one tier-1 line that fails after a commit rather than before it.
    local fmt
    fmt=$(cargo fmt --all --check 2>&1) && found= || found=$fmt
    printf 'cargo fmt --all --check          %s\n' "$([ -z "$found" ] && echo clean || echo differs)"
    [ -z "$found" ] || { printf '%s\n' "$found" | grep -E '^Diff in' | sort -u | sed 's/^/    /'; bad=1; }

    return "$bad"
}

# The population a merge commits and a close would destroy: every path `git status` reports in
# the worktree — modified, deleted, untracked, both sides of a rename — except `scratchpad/`, which
# is the rounds' and never committed. One record per path, `XY<TAB>path`, NUL-terminated, read from
# `-z`'s listing, where a path is its bytes and never git's quoted, escaped spelling of them.
# `check` reads this function too (through `untracked_paths`), so what it reports and what `commit`
# stages are one population.
population() {
    local entry
    git -C "$wt" status --porcelain=v1 -z --untracked-files=all --no-renames |
        while IFS= read -r -d '' entry; do
            case "${entry:3}" in scratchpad/*) continue ;; esac
            printf '%s\t%s\0' "${entry:0:2}" "${entry:3}"
        done
}

# The untracked part of `population`, one real path per line: what `check`'s findings about added
# files read. A path holding a newline is the one spelling this cannot carry, and no round writes one.
untracked_paths() {
    local entry
    population | while IFS= read -r -d '' entry; do
        [ "${entry%%$'\t'*}" = "??" ] && printf '%s\n' "${entry#*$'\t'}"
    done
    return 0
}

# Stage the population by name, prove the index holds exactly it, and commit. It never
# fast-forwards and never closes: those are the next two commands, each run on its own after this
# one's output has been read (`doc/todo/02` section 8 step 5, ADR 1313).
commit_batch() {
    local message=$1
    [ -s "$message" ] || { echo "$message: no commit message there"; return 1; }
    message=$(realpath "$message")
    case "$message" in
        "$wt"/scratchpad/*) ;;
        "$wt"/*) echo "$message is in the worktree outside scratchpad/, so it would be committed with the batch — put it under scratchpad/ or outside the tree"; return 1 ;;
    esac
    cd "$wt" || return 1
    local entry code path listing
    local -a to_add=()
    listing=$(population | tr '\0' '\n') || { echo "cannot read $wt's status — nothing staged"; return 1; }
    [ -n "$listing" ] || { echo "nothing to commit outside scratchpad/"; return 1; }
    while IFS= read -r -d '' entry; do
        code=${entry%%$'\t'*}; path=${entry#*$'\t'}
        # A deletion already staged is in the population and on no disk, and `git add` refuses a
        # pathspec that matches nothing — for the whole list, which is how batch thirty-five's
        # commit came to carry one deletion and nothing else. It is staged already: count it,
        # never name it to `git add`.
        [ "$code" = "D " ] || to_add+=("$path")
    done < <(population)
    if [ "${#to_add[@]}" -gt 0 ]; then
        printf '%s\0' "${to_add[@]}" |
            git --literal-pathspecs add -A --pathspec-from-file=- --pathspec-file-nul ||
            { echo "git add refused (above) — nothing committed; the index may be partly staged, read git status"; return 1; }
    fi
    local want got
    want=$(printf '%s\n' "$listing" | cut -f2- | sort -u)
    got=$(git diff --cached --no-renames --name-only -z | tr '\0' '\n' | sort -u)
    printf 'population %s path(s), staged %s path(s)\n' "$(printf '%s\n' "$want" | grep -c .)" "$(printf '%s\n' "$got" | grep -c .)"
    if [ "$want" != "$got" ]; then
        echo "the index is not the population — nothing committed:"
        comm -23 <(printf '%s\n' "$want") <(printf '%s\n' "$got") | sed 's/^/    not staged: /'
        comm -13 <(printf '%s\n' "$want") <(printf '%s\n' "$got") | sed 's/^/    staged, not in the population: /'
        return 1
    fi
    if printf '%s\n' "$got" | grep -q '^scratchpad/'; then
        echo "a path under scratchpad/ is staged — nothing committed:"
        printf '%s\n' "$got" | grep '^scratchpad/' | sed 's/^/    /'
        return 1
    fi
    if [ -f .gitmodules ]; then
        local blobs
        blobs=$(git ls-files --stage -- $(git config -f .gitmodules --get-regexp '\.path$' | awk '{print $2}') |
            awk '$1 != "160000" { print $4 }' || true)
        [ -z "$blobs" ] || { echo "a submodule is staged as a blob — nothing committed:"; printf '%s\n' "$blobs" | sed 's/^/    /'; return 1; }
    fi
    git commit -q -F "$message" || { echo "git commit failed — the index is staged, nothing is committed"; return 1; }
    local left; left=$(population | tr '\0' '\n')
    printf 'committed %s; uncommitted outside scratchpad/ now %s (must be 0)\n' \
        "$(git log --oneline -1 | cut -c1-80)" "$(printf '%s' "$left" | grep -c . || true)"
    [ -z "$left" ] || return 1
    printf 'next, as its own command, from %s: git merge --ff-only %s — read it; then tools/batch.sh close %s\n' \
        "$root" "$(git rev-parse --abbrev-ref HEAD)" "$(git rev-parse --abbrev-ref HEAD)"
}

# What a person runs, built once from the commit `main` now names and installed into the main
# checkout's own `target/` — the one place this script writes outside the worktree, because it is
# where `doc/running-the-viewer.md` tells a person to look and it is gitignored. Run at the batch
# boundary, after the fast-forward and before `close` (`doc/todo/02` section 8 step 5, ADR 1511).
#
# **It refuses rather than install a binary of a commit `main` does not name**: a worktree holding
# anything uncommitted, or a branch whose HEAD is not `main`'s, would put a program under `target/`
# that no commit describes. The binaries carry no hash of their own (`quorra --version` opens a file
# called `--version`), so the commit they were built from is written beside them, in
# `target/installed-from`, with each file's SHA-256 — `tools/state.sh binaries` reads it back and
# says how far `main` has moved since.
#
# The names are `doc/todo/02` section 5's, and `tests/batch.rs` holds the two lists equal: the
# programs a person runs, each worker beside the program that looks for it there (a viewer that
# cannot find `pdf-sandbox-worker` refuses JBIG2 and JPEG 2000 rather than decoding them in
# process), and the two C libraries a person links against. One invocation for the programs,
# because each is a whole-graph fat link and Cargo runs them beside each other where separate
# commands run them one after another (ADR 0222); a second for the libraries, which `--bin` cannot
# name. The build directory is asked of Cargo in the worktree, never written down (trap 15).
install_binaries="quorra quorra-confined quorra-gtk quorra-qt pdf-sandbox-worker pdf-view-worker quorra-retrieve quorra-transform quorrafs pdf-vfs-worker"
install_libraries="viewer-ffi pdf-vfs-ffi"
# A program behind a feature no window is built with, as `name:features`: each is built in a Cargo
# run of its own, so that feature unification puts nothing of it into a window's build. The script
# worker links the engine and a window links only the client that spawns it, and it is looked for
# beside the window that spawns it, so it is installed with them (ADRs 1616, 1625).
install_featured="pdf-script-worker:pdf-script-worker/engine"

install_batch() {
    [ -d "$wt" ] || { echo "$wt does not exist — install runs while the batch is open, before close"; return 1; }
    local dirty head on_main
    dirty=$(population | tr '\0' '\n') || { echo "cannot read $wt's status — nothing installed"; return 1; }
    [ -z "$dirty" ] || {
        echo "$wt holds $(printf '%s\n' "$dirty" | grep -c .) uncommitted path(s) outside scratchpad/ — a binary built from them is of no commit; commit first (tools/batch.sh commit), then fast-forward main:"
        printf '%s\n' "$dirty" | head -20 | sed 's/^/    /'
        return 1
    }
    head=$(git -C "$wt" rev-parse HEAD)
    on_main=$(git -C "$root" rev-parse main)
    [ "$head" = "$on_main" ] || {
        echo "main is at $(git -C "$root" rev-parse --short main) and the batch at $(git -C "$wt" rev-parse --short HEAD) — fast-forward main first (from $root: git merge --ff-only $(git -C "$wt" rev-parse --abbrev-ref HEAD)), so that what is installed is what main names"
        return 1
    }
    cd "$wt" || return 1
    local built name source target="$root/target" programs=() packages=() sums
    built=$(cargo metadata --no-deps --format-version 1 |
            grep -oE '"target_directory":"[^"]+"' | head -1 | cut -d'"' -f4)
    [ -n "$built" ] || { echo "cargo metadata named no target directory — nothing installed"; return 1; }
    built=$built/release
    for name in $install_binaries; do programs+=(--bin "$name"); done
    for name in $install_libraries; do packages+=(-p "$name"); done
    echo "building ${install_binaries// /, } and the libraries of ${install_libraries// /, } (--release) in $built"
    cargo build --release "${programs[@]}" || { echo "the release build of the programs failed (above) — nothing installed"; return 1; }
    cargo build --release "${packages[@]}" --lib || { echo "the release build of the libraries failed (above) — nothing installed"; return 1; }
    local entry
    for entry in $install_featured; do
        cargo build --release --bin "${entry%%:*}" --features "${entry#*:}" || { echo "the release build of ${entry%%:*} with ${entry#*:} failed (above) — nothing installed"; return 1; }
    done
    local -a files=()
    for name in $install_binaries; do files+=("$name"); done
    for entry in $install_featured; do files+=("${entry%%:*}"); done
    for name in $install_libraries; do files+=("lib${name//-/_}.so"); done
    for name in "${files[@]}"; do
        [ -f "$built/$name" ] || { echo "$built/$name was not produced — nothing installed"; return 1; }
    done
    # Mode 775: the main checkout is shared through the `coders` group, and a file only its
    # installer could replace would stop the owner rebuilding over it.
    for name in "${files[@]}"; do
        install -Dm775 "$built/$name" "$target/$name" || { echo "install of $name into $target failed — target/ is part-installed; run install again"; return 1; }
    done
    sums=$(cd "$target" && sha256sum "${files[@]}")
    {
        printf '# commit %s\n' "$head"
        printf '# subject %s\n' "$(git -C "$wt" log -1 --format=%s | cut -c1-120)"
        printf '# installed %s from %s by tools/batch.sh install\n' "$(date -Iseconds)" "$built"
        printf '%s\n' "$sums"
    } > "$target/installed-from"
    source=$(git -C "$wt" log -1 --format='%h %cs')
    for name in "${files[@]}"; do
        printf '%-44s %10s bytes  built from %s\n' "$target/$name" "$(stat -c %s "$target/$name")" "$source"
    done
    # What `target/` holds that this did not put there: a program a person may still run, and
    # older than what was just installed.
    local left; left=$(find "$target" -maxdepth 1 -type f -perm -u+x ! -newer "$target/installed-from" -printf '%f\n' |
        grep -vxF -f <(printf '%s\n' "${files[@]}") | sort || true)
    [ -z "$left" ] || printf 'also in %s and not installed by this (older): %s\n' "$target" "$(printf '%s\n' "$left" | paste -sd' ')"
    printf 'installed %s file(s) from %s into %s; the record is %s/installed-from\n' "${#files[@]}" "$source" "$target" "$target"
}

close_batch() {
    # There is no `--force`, and a spelling of one is refused rather than read as a branch name.
    # The fix for a refusal is to commit the work or to move it; discarding it blind is the one
    # answer this command does not offer (ADR 1313).
    case "$1" in -*) echo "close takes a branch name and no options — commit the work (tools/batch.sh commit) or move it; nothing here discards it (ADR 1313)"; return 1 ;; esac
    # A shell whose working directory is the worktree loses it when the worktree goes: every
    # command after the close then fails with "getcwd: cannot access parent directories", which is
    # what happened to the merge of sessions 1038-1043 half a line after the fast-forward. Refuse.
    case "$PWD/" in "$wt"/*) echo "close from outside $wt — your shell is inside it"; return 1 ;; esac
    if [ -d "$wt" ]; then
        # Work nobody committed. `worktree remove --force` below deletes it without a word, and
        # batch thirty-five lost 128 files that way after a chained commit carried one deletion
        # (ADR 1313). Refuse, and say what is there.
        local dirty
        dirty=$(population | tr '\0' '\n') || { echo "cannot read $wt's status — not closing"; return 1; }
        [ -z "$dirty" ] || {
            echo "$wt holds $(printf '%s\n' "$dirty" | grep -c .) uncommitted path(s) outside scratchpad/ — commit them (tools/batch.sh commit) or move them; close would delete them:"
            printf '%s\n' "$dirty" | head -40 | sed 's/^/    /'
            return 1
        }
        # The branch named is the branch the worktree is on: the checks below read the name, and a
        # typed name that matches nothing read as "no commits main lacks".
        local on; on=$(git -C "$wt" rev-parse --abbrev-ref HEAD)
        [ "$on" = "$1" ] || { echo "$wt is on $on, not $1 — not closing"; return 1; }
    fi
    # And a branch with commits main lacks is not finished: the merge of sessions 1038-1043 ran
    # `git merge --ff-only` from inside the worktree, which merged the branch into itself and
    # exited 0, then closed it — deleting the only ref to the batch. The commit was recovered
    # from the object store, but only because nothing had run `gc` yet. Refuse instead.
    # A specification fetched for reading is a corpus document (the oracle and the accessibility
    # census walk page one of every doc/*.pdf). One written into THIS checkout's doc/ rather than
    # the main checkout's dies with the worktree, and the next merge sees the corpus shrink and its
    # floors fail for a cause nobody made — sessions 1071 and 1079. Refuse, and say where it goes.
    local stray; stray=$(find "$wt/doc" -maxdepth 1 -name "*.pdf" -type f 2>/dev/null || true)
    [ -z "$stray" ] || { echo "regular PDF(s) under the worktree's doc/ — move each to $root/doc/ and symlink it here, then close:"; echo "$stray"; return 1; }
    local ahead
    ahead=$(git -C "$root" rev-list --count "main..$1") || { echo "$1 is not a branch main can be compared with — not closing"; return 1; }
    [ "$ahead" = 0 ] || { echo "$1 has $ahead commit(s) main lacks — fast-forward main first (from the main checkout, not from inside the worktree)"; return 1; }
    # `--force` here is for `scratchpad/` and the symlinked data, which the checks above have
    # already shown to be the only things left in the tree.
    if [ -d "$wt" ]; then
        git -C "$root" worktree remove --force "$wt" || { echo "git worktree remove failed (above) — branch kept"; return 1; }
    fi
    git -C "$root" branch -D "$1" 2>/dev/null || true
    git -C "$root" worktree prune
    echo "$1: worktree and branch gone"
    debug_over_the_rule
}

case "${1:-}" in
    open)  open_batch "${2:?branch name}" ;;
    gates) gates ;;
    raster-examples) raster_examples "${2:-}" ;;
    arms)  export_arms "${2:-}" ;;
    arms-held) arms_held "${2:?}" "${3:?}" "${4:?}" ;;
    check) check_batch ;;
    commit) commit_batch "${2:?a commit message file}" ;;
    install) install_batch ;;
    close) close_batch "${2:?branch name}" ;;
    *) awk 'NR < 3 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"; exit 1 ;;
esac
