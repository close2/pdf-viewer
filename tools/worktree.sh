#!/usr/bin/env bash
#
# Prepare a worktree for one parallel round, and take it away again afterwards.
#
# A round running beside three others needs four things the checkout does not give it: a branch,
# its own build directory, the gitignored data the gates read, and the submodules. Doing that by
# hand is where two of this project's worst half-hours went — `git add -A` in a worktree whose
# submodules had been replaced by symlinks turned six gitlinks into blobs, and a build directory
# left behind when its worktree was removed is 19-29 GB nobody is looking at. Both are here so
# that neither is a thing a round has to remember.
#
#   tools/worktree.sh open 675 [676 ...]     create, wire up, and print what each round is given
#   tools/worktree.sh close 675 [676 ...]    remove the checkout AND its build directory, together
#   tools/worktree.sh list                   what exists now, with the size of each build directory
#
# doc/environment.md is the prose; this is the command.

set -euo pipefail

root=$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)
builds=/home/AI/cargo-target

# The gitignored data every gate reads. Symlinked rather than copied: `doc/md` alone is large, the
# corpora are submodules, and a round has no business writing to any of them.
linked=(doc/md doc/pdf.js doc/arlington-pdf-model corpus-cache)

open_one() {
    local n=$1 wt="$root/.claude/worktrees/r$n"
    [ -e "$wt" ] && { echo "r$n: exists already — close it first"; return 1; }

    git -C "$root" worktree add -q -b "round-$n" "$wt" HEAD

    # Its own target-dir, in a per-worktree config rather than an exported CARGO_TARGET_DIR, which
    # sccache cannot see. Keeps four rounds off one build lock.
    mkdir -p "$wt/.cargo"
    printf '[build]\ntarget-dir = "%s/pdfv-r%s"\n' "$builds" "$n" > "$wt/.cargo/config.toml"

    for p in "${linked[@]}"; do
        [ -e "$root/$p" ] || continue
        rm -rf "${wt:?}/$p"
        ln -s "$root/$p" "$wt/$p"
    done
    for p in "$root"/doc/*.pdf; do
        [ -e "$p" ] || continue
        ln -sf "$p" "$wt/doc/$(basename "$p")"
    done
    # The corpora are submodules, and a fresh worktree gets them empty. Linking them to the main
    # checkout costs nothing and saves 424 MB a round — but a symlink where git expects a gitlink is
    # a loaded gun: `git add doc` rewrites mode 160000 to 120000 and the gitlink is gone, which has
    # now happened twice here (once under `git add -A`, once under a plain `git add doc`).
    #
    # `--skip-worktree` disarms it. Git then never compares that path against the working tree, so
    # `add` leaves the gitlink alone; measured both ways before this line was written. The rule the
    # two incidents actually teach is the one worth keeping: **a hazard a document warns about is a
    # hazard every future round has to remember, and this is what it costs to not need to.**
    for p in "$root"/doc/corpora/*/; do
        [ -d "$p" ] || continue
        local name; name=$(basename "$p")
        if [ -z "$(ls -A "$wt/doc/corpora/$name" 2>/dev/null)" ]; then
            rmdir "$wt/doc/corpora/$name" 2>/dev/null || true
            ln -sfn "${p%/}" "$wt/doc/corpora/$name"
            git -C "$wt" update-index --skip-worktree "doc/corpora/$name"
        fi
    done

    echo "r$n: $wt"
    echo "     branch round-$n, build $builds/pdfv-r$n"
}

close_one() {
    local n=$1 wt="$root/.claude/worktrees/r$n"
    git -C "$root" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$root" branch -D "round-$n" 2>/dev/null || true
    # The pair, as one act. A checkout removed without its build directory is the 425 GB mistake.
    rm -rf "${builds:?}/pdfv-r$n"
    echo "r$n: checkout and build directory both gone"
}

case "${1:-}" in
    open)  shift; for n in "$@"; do open_one "$n"; done ;;
    close) shift; for n in "$@"; do close_one "$n"; done; git -C "$root" worktree prune ;;
    list)
        git -C "$root" worktree list
        echo
        # The guard is invisible when it works, so print whether it is on. A round that has just
        # been bitten by a staged gitlink should be able to tell "the guard is off here" from
        # "the guard is on and something else happened" without reasoning about it.
        for wt in "$root"/.claude/worktrees/r*/; do
            [ -d "$wt" ] || continue
            n=$(basename "${wt%/}")
            flags=$(git -C "$wt" ls-files -v doc/corpora 2>/dev/null | grep -c '^S ' || true)
            total=$(git -C "$wt" ls-files doc/corpora 2>/dev/null | wc -l)
            if [ "$flags" -gt 0 ] && [ "$flags" -eq "$total" ]; then
                printf '  %-8s gitlink guard on  (%s/%s skip-worktree)\n' "$n" "$flags" "$total"
            else
                printf '  %-8s GITLINK GUARD OFF (%s/%s) — a blanket `git add` here can stage a symlink over a submodule\n' "$n" "$flags" "$total"
            fi
        done
        echo
        for d in "$builds"/pdfv-r*; do
            [ -e "$d" ] || continue
            n=$(basename "$d"); n=${n#pdfv-r}
            wt="$root/.claude/worktrees/r$n"
            [ -d "$wt" ] && state="live" || state="ORPHANED — its worktree is gone"
            printf '  %-28s %6s  %s\n' "$(basename "$d")" "$(du -sh "$d" 2>/dev/null | cut -f1)" "$state"
        done ;;
    *) sed -n '3,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 1 ;;
esac
