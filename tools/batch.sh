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
#   tools/batch.sh check                   # the six things a merge looks at by hand, one line each
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
    mkdir -p "$wt/scratchpad/open"
    (cd "$wt" && setsid nohup cargo build --workspace --all-targets \
        > "$wt/scratchpad/open/build.log" 2>&1 < /dev/null &
     echo "warming the build directory: pid $!, scratchpad/open/build.log")
}

# One line per gate: name, exit, the seconds it ran, the seconds it queued for the lock before
# that, and the gate's own summary line. A failure's last thirty lines go beside the log under the
# gate's name, so a merge reads one file and opens one more. The two clocks are apart because they
# answer different questions: `wall` is what the gate costs — its build and its walk, which the
# test's own "finished in" leaves out — and `wait` is what the machine's other walks cost it; a
# dear gate is read off the first and never the second. `tools/state.sh gates-cost` prints them
# (ADR 1476).
#
# Every gate runs behind /home/AI/heavy-walk.lock with four rayon threads: the rounds take the
# same lock for their own corpus walks, so at most one heavy walk is on the machine at a time
# across the batch. On 2026-09-15 six rounds and a merge walked the corpus at once and the whole
# process was killed (raster_golden alone peaks past 7 GiB at twelve threads); the lock costs
# wall-clock and a kill costs the batch.
run() {
    local name=$1; shift; local out rc asked began ended stamp
    stamp=$(mktemp)
    asked=$(date +%s)
    out=$(RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-4}" flock /home/AI/heavy-walk.lock \
        sh -c 'date +%s > "$0"; exec "$@"' "$stamp" "$@" 2>&1) && rc=0 || rc=$?
    ended=$(date +%s)
    began=$(cat "$stamp" 2>/dev/null); rm -f "$stamp"
    [ -n "$began" ] || began=$asked
    # A test line that ran nothing exits 0: `--ignored` over a file with no ignored test is green
    # while checking nothing. It is a failure here, and `tests/batch.rs` holds every line's flag
    # to its file's `#[ignore]` attributes before anything runs (ADR 1392).
    if [ "$rc" -eq 0 ] && [ "$1 $2" = "cargo test" ] &&
        ! printf '%s\n' "$out" | grep -qE 'test result: [a-z]+\. [1-9][0-9]* passed'; then
        rc=98; out+=$'\nran zero tests — a green line that checked nothing (ADR 1392)'
    fi
    printf '%-24s exit=%-4s wall=%-6s wait=%-6s %s\n' "$name" "$rc" \
        "$((ended - began))s" "$((began - asked))s" \
        "$(printf '%s\n' "$out" | grep -iE 'test result|documents|pages|passed|FAILED|panicked|ran zero tests' | tail -1 | cut -c1-150)" >> "$log"
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
    # `release`, not `gates`: the bands in `doc/checks/turn-path.toml` are a claim about the
    # profile a person runs, and the gate prints without judging under any other (ADR 1513).
    run t2-turn_path      cargo test --release -p render-raster --test turn_path -- --ignored --nocapture
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
    tail -1 "$log"
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
    records=$(cargo test -q -p conformance --test records -- --nocapture 2>&1) && found= || found=$records
    printf 'records over budget              %s\n' "$([ -z "$found" ] && echo none || echo over)"
    [ -z "$found" ] || { printf '%s\n' "$found" | grep -E 'over the budget' | sed 's/^/    /'; bad=1; }

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
        awk -v newest="${newest:-0}" '$1 <= newest { printf "%s is numbered %s, behind the committed %s\n", $2, $1, newest }')
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
    local -a files=()
    for name in $install_binaries; do files+=("$name"); done
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
    check) check_batch ;;
    commit) commit_batch "${2:?a commit message file}" ;;
    install) install_batch ;;
    close) close_batch "${2:?branch name}" ;;
    *) awk 'NR < 3 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"; exit 1 ;;
esac
