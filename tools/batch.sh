#!/usr/bin/env bash
#
# A batch of parallel rounds in one worktree, and the merge that follows it.
#
# Six rounds run in ONE worktree on ONE short-lived branch, each briefed with the ledger rows it
# must close, and the merge runs `doc/todo/02` §2's tier 2 and tier 3 once for all of them
# (ADR 1036: the corpus walks caught one self-introduced defect in 98 sessions and raised
# fourteen false alarms at ~25 minutes a round; once per batch they take ~12 minutes and are
# believed). `tools/worktree.sh` is the per-round shape with a build directory each; this one
# shares the main checkout's build directory on purpose, because six agents in one tree share
# one build lock whatever the directory is, and a cold build per batch is twenty minutes nobody
# needs.
#
#   tools/batch.sh open  batch-1038-1043   # worktree at /home/AI/pdf-viewer-rounds, guard on
#   tools/batch.sh gates                   # tiers 2 and 3, one line per gate, into batch-gates.log
#   tools/batch.sh close batch-1038-1043   # after `git merge --ff-only` on main: remove both
#
# The loop itself is `doc/todo/02` §8. The gitlink guard is the same one `tools/worktree.sh`
# carries, for the same incident: a symlink where git expects a submodule is staged as a blob
# by any blanket `git add`, and `git status` cannot see it because the index already agrees
# with the working tree (doc/habits/tests-gates-and-reports.md).

set -euo pipefail

root=$(dirname "$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --path-format=absolute --git-common-dir)")
wt=/home/AI/pdf-viewer-rounds
log=${BATCH_GATES_LOG:-/home/AI/batch-gates.log}

open_batch() {
    local branch=$1
    [ -e "$wt" ] && { echo "$wt exists — close the previous batch first"; return 1; }
    git -C "$root" worktree add -q -b "$branch" "$wt" HEAD
    for r in doc/md doc/pdfa corpus-cache tmp fuzz/corpus fuzz/artifacts; do
        [ -e "$root/$r" ] || continue
        rm -rf "${wt:?}/$r"; ln -sfn "$root/$r" "$wt/$r"
    done
    [ -f "$root/fuzz/Cargo.lock" ] && cp "$root/fuzz/Cargo.lock" "$wt/fuzz/Cargo.lock"
    for f in "$root"/doc/*.pdf; do [ -e "$f" ] && ln -sfn "$f" "$wt/doc/$(basename "$f")"; done
    # Submodules: link the content, keep the gitlink, pin it. Population derived from the index.
    while read -r sha path; do
        rm -rf "${wt:?}/$path"; ln -sfn "$root/$path" "$wt/$path"
        git -C "$wt" update-index --cacheinfo "160000,$sha,$path"
        git -C "$wt" update-index --skip-worktree -- "$path"
    done < <(git -C "$wt" ls-files --stage | awk '$1 == "160000" { print $2, $4 }')
    echo "$branch: $wt  (status: $(git -C "$wt" status --short | wc -l) changed; must be 0)"
}

# One line per gate: name, exit, the gate's own summary line. A failure's last thirty lines go
# beside the log under the gate's name, so a merge reads one file and opens one more.
run() {
    local name=$1; shift; local out rc
    out=$("$@" 2>&1) && rc=0 || rc=$?
    printf '%-24s exit=%-4s %s\n' "$name" "$rc" \
        "$(printf '%s\n' "$out" | grep -iE 'test result|documents|pages|passed|FAILED|panicked' | tail -1 | cut -c1-150)" >> "$log"
    [ "$rc" -ne 0 ] && printf '%s\n' "$out" | tail -30 > "$log.fail.$name"
    return 0
}

gates() {
    cd "$wt"; : > "$log"; rm -f "$log".fail.* 2>/dev/null || true
    run build-sandbox  cargo build --profile gates -p pdf-sandbox --bins
    run build-hayro    cargo build --profile gates -p hayro-compare --bin pdfref-hayro
    run build-vfs      cargo build --profile gates -p pdf-vfs --bins
    run build-confined cargo build --profile gates -p viewer-confined --bins
    for t in corpus raster_golden dates xmp; do
        run "t2-$t" cargo test --profile gates -p pdf-model --test "$t" -- --ignored --nocapture; done
    run t2-jpeg2000       cargo test --profile gates -p pdf-model --test jpeg2000 -- --nocapture
    run t2-transform-gate cargo test --profile gates -p pdf-transform --test gate -- --ignored --nocapture
    run t2-on_disk        cargo test --profile gates -p pdf-syntax --test on_disk -- --ignored --nocapture
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
    echo "ALL GATES DONE — $(grep -c 'exit=0' "$log") of $(grep -cE 'exit=' "$log") green" >> "$log"
    tail -1 "$log"
}

close_batch() {
    # A shell whose working directory is the worktree loses it when the worktree goes: every
    # command after the close then fails with "getcwd: cannot access parent directories", which is
    # what happened to the merge of sessions 1038-1043 half a line after the fast-forward. Refuse.
    case "$PWD/" in "$wt"/*) echo "close from outside $wt — your shell is inside it"; return 1 ;; esac
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
    local ahead; ahead=$(git -C "$root" rev-list --count "main..$1" 2>/dev/null || echo 0)
    [ "$ahead" = 0 ] || { echo "$1 has $ahead commit(s) main lacks — fast-forward main first (from the main checkout, not from inside the worktree)"; return 1; }
    git -C "$root" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$root" branch -D "$1" 2>/dev/null || true
    git -C "$root" worktree prune
    echo "$1: worktree and branch gone"
}

case "${1:-}" in
    open)  open_batch "${2:?branch name}" ;;
    gates) gates ;;
    close) close_batch "${2:?branch name}" ;;
    *) awk 'NR < 3 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"; exit 1 ;;
esac
