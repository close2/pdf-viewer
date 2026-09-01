#!/usr/bin/env bash
#
# What this round is, before it starts.
#
# `tools/state.sh` answers "what are the numbers"; this one answers the questions a round asks
# *before* it has done anything: which session is next, what to read for the kind of work it is
# about, which of `doc/todo/02` §2's gates that kind of change actually needs, and whether this
# round owes the full sequence and §5's binaries. Then it checks the four things a round has
# actually got wrong here — an uninitialised submodule, a build script baked against a checkout
# that no longer exists, installed binaries older than `HEAD`, an exported `CARGO_TARGET_DIR`, and
# a pipeline on `main` that has been failing since a push no round watched (ADR 0450).
#
# **It changes nothing.** Every command below reads: `ls`, `git`, `grep`, `test`, `gh`. A round that
# wants something fixed fixes it itself, because a script that silently repaired the tree would
# be the instrument altering what it measures.
#
# It performs no arithmetic on a gate's numbers and prints none — same rule as `tools/state.sh`,
# same reason (ADR 0281). The one number it computes is the session, from `ls doc/history/`,
# which is a fact on the disk rather than a sentence about it.
#
#   tools/round.sh                 # the session, the every-round reading list, the checks
#   tools/round.sh pixels          # ... and what a round that changes what gets drawn opens
#   tools/round.sh --list          # the kinds of round it knows
#
# Exit status is 0 when every check passed and 1 when one did not, so a round may trust a zero.

set -u -o pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root" || exit 1

status=0
heading() { printf '\n== %s ==\n\n' "$1"; }
fail() { printf '  ✗ %s\n' "$1"; status=1; }
pass() { printf '  ✓ %s\n' "$1"; }

# The kinds of round, each one row: name, what it is, what it opens beyond the every-round list,
# and which of doc/todo/02 §2 its change needs. The gate column is that file's change→gate map in
# one line; the map itself is the authority and this is the pointer to it.
kinds="pixels oracle parsers loop instruments clause measure host dependency docs"

kind_reading() {
    case $1 in
    pixels)
        printf 'doc/traps/pixels-and-rasterisers.md      traps 1, 2, 6, 12b\n'
        printf 'doc/traps/oracle-and-references.md       because the oracle judges what you drew\n'
        printf 'doc/state-of-play.md                     what already draws\n' ;;
    oracle)
        printf 'doc/traps/oracle-and-references.md       traps 3, 9, 12\n'
        printf 'doc/oracle-and-corpus.md                 the instrument itself\n'
        printf 'doc/habits.md                            "Judging against other implementations"\n'
        printf 'doc/todo/00-ambiguous-bucket.md          the bucket and step 7\n' ;;
    parsers)
        printf 'doc/traps/parsers-and-streams.md         traps 4, 5, 8, 28\n'
        printf 'doc/traps/instruments-and-reports.md     trap 11, before adding a report\n'
        printf 'doc/verify.md                            which fuzz target covers what you touched\n' ;;
    loop)
        printf 'doc/traps/the-interactive-loop.md        trap 12a\n'
        printf 'doc/ui-boundary.md                       the boundary, and the test a message must pass\n'
        printf 'doc/environment.md                       the Xvfb recipe — the only way to drive the loop\n' ;;
    instruments)
        printf 'doc/traps/instruments-and-reports.md     traps 7, 10, 10a, 11\n'
        printf 'doc/habits.md                            "Tests, gates and reports"\n' ;;
    clause)
        printf 'doc/habits.md                            "Reading the specification" and "The ledger"\n'
        printf 'doc/ledger-and-claims.md                 where a false row hides\n'
        printf 'doc/errata-read.md                       what an erratum has moved\n'
        printf 'doc/todo/01-ledger-partial-rows.md       the sweeps, as commands\n' ;;
    measure)
        printf 'doc/habits.md                            "Measuring"\n'
        printf 'doc/performance.md                       the timeline and what is already known\n'
        printf 'doc/traps/instruments-and-reports.md     what a gate is about to lie to you about\n'
        printf 'doc/todo/02-every-round.md               §5 — the binaries, which a measurement owes first\n' ;;
    host)
        printf 'doc/ui-boundary.md                       Command/Event/Query/Answer, and the freeze\n'
        printf 'doc/traps/the-interactive-loop.md        trap 12a\n'
        printf 'doc/todo/30-a-native-host.md             what the hosts still owe\n' ;;
    dependency)
        printf 'doc/stack.md                             the stack, and why rustybuzz is not in it\n'
        printf 'doc/third-party-data.md                  what a datum has to be before it is trusted\n'
        printf 'doc/PLAN.md                              §1\n' ;;
    docs)
        printf 'doc/HANDOVER.md                          the index this script is the companion to\n'
        printf 'CLAUDE.md                                the rule about what may be written down\n' ;;
    *) return 1 ;;
    esac
}

kind_gates() {
    case $1 in
    pixels)      printf 'everything — a change that can move a pixel runs the whole of §2\n' ;;
    oracle)      printf 'the core, the oracle gate and the corpus gate; everything if the change is in pdf-model\n' ;;
    parsers)     printf 'everything — pdf-syntax, pdf-font and pdf-model are under every gate\n' ;;
    loop)        printf 'the core, selection_census and accessibility_census\n' ;;
    instruments) printf 'the core, plus whichever gate the instrument is\n' ;;
    clause)      printf 'the core and cargo test -p conformance; everything if code changed\n' ;;
    measure)     printf 'the core — and §5 first, always, because a stale binary measures the past\n' ;;
    host)        printf 'the core, which builds and tests every host; §5 for what a person runs\n' ;;
    dependency)  printf 'everything, plus cargo deny (doc/verify.md)\n' ;;
    docs)        printf 'the core and cargo test -p conformance, plus --bin quotations and --bin pointers\n' ;;
    esac
}

case ${1-} in
--list) printf '%s\n' $kinds; exit 0 ;;
esac
kind=${1-}

# ---------------------------------------------------------------- the round

last=$(ls doc/history/ 2>/dev/null | grep -oE '^[0-9]+' | sort -n | tail -1)
if [ -z "$last" ]; then
    printf 'doc/history/ holds no numbered file — is this the project root?\n' >&2
    exit 1
fi
session=$((last + 1))
from=doc/history/

# **A parallel round's branch outranks `doc/history/`, and that is not a preference.** A worktree is
# branched before its neighbours have written their files, so `ls doc/history/` there is a count of
# the rounds that finished *before this batch started* — which told the six-hundred-and-eighty-seventh
# and the six-hundred-and-ninety-first that they were session 685, and told both they owed a fifth
# round's obligations they did not owe. The branch name is the assignment itself and cannot go stale.
branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)
case "$branch" in
    round-[0-9]*)
        session=${branch#round-}
        last=$((session - 1))
        from="the branch name"
        ;;
esac

heading "the round"
printf '  session %s, after %s (from %s)\n' "$session" "$last" "$from"
if [ $((session % 5)) -eq 0 ]; then
    printf '  **a fifth round**: doc/todo/02 §2 runs whole, and §5 rebuilds and installs the binaries\n'
else
    printf '  not a fifth round: §2 by the change→gate map, §5 only before a measurement\n'
    printf '  (%s more rounds until the next full sequence)\n' "$((5 - session % 5))"
fi
printf '  and whatever the change: a round that can move a pixel runs §2 whole, and a merge always does\n'

# ---------------------------------------------------------------- the reading

heading "read, whatever this round is"
printf '  doc/todo/README.md                       what is owed, one line per item\n'
printf '  doc/todo/02-every-round.md               the gates, the sweeps, the binaries, the commit\n'
printf '  doc/environment.md                       the machine, the account, the display, the build directory\n'
printf '  doc/HANDOVER.md                          the index: which trap group this round is in\n'

if [ -n "$kind" ]; then
    if kind_reading "$kind" >/dev/null 2>&1; then
        heading "read, because this is a \"$kind\" round"
        kind_reading "$kind" | sed 's/^/  /'
        heading "gates this change needs (doc/todo/02 §2's map is the authority)"
        kind_gates "$kind" | sed 's/^/  /'
    else
        printf '\nno such kind: %s (tools/round.sh --list)\n' "$kind" >&2
        status=1
    fi
else
    heading "and then by what the round is"
    printf '  tools/round.sh <kind>, one of: %s\n' "$kinds"
fi

# ---------------------------------------------------------------- the checks

heading "what a round has got wrong here before"

# 1. The one submodule a build needs. `pdf-spec` will not build without it, and a worktree gets
#    an empty directory rather than an error that says so.
if [ -n "$(ls -A doc/arlington-pdf-model 2>/dev/null)" ]; then
    pass "doc/arlington-pdf-model is checked out (pdf-spec needs it)"
else
    fail "doc/arlington-pdf-model is empty — git submodule update --init, or pdf-spec will not build"
fi

# 2. A build script's env!("CARGO_MANIFEST_DIR") is baked at *its* compile time, and the shared
#    build directory outlives a checkout — so a build-script binary can name a manifest directory
#    that no longer exists and fail with a message about `data/cmaps`. Ask the binary, not the tree.
target=$(cargo metadata --no-deps --format-version 1 2>/dev/null |
         grep -oE '"target_directory":"[^"]+"' | head -1 | cut -d'"' -f4)
[ -n "$target" ] || target=target
#    Only the *newest* build script per crate is asked, because the shared build directory keeps
#    every superseded one and those are noise: cargo will not reach for them again. The count of
#    the superseded stale ones is printed rather than judged, so that "the directory is full of
#    other rounds' worktrees" reads as itself.
#
#    **The population is derived, and the discriminator is which macro the build script uses.**
#    `env!("CARGO_MANIFEST_DIR")` is expanded when the build script itself is *compiled*, so the
#    path is baked into the binary and outlives the checkout it names; `std::env::var_os(...)` is
#    read when cargo *runs* the script, so cargo supplies the live value and it cannot go stale.
#    `crates/pdf-spec/build.rs` takes the second road, which is why it is not asked even though
#    its binary carries the path in its debug info — grepping a binary for a path finds strings
#    the program will never read, so the source has to say which kind it is.
#
#    It was a hand-written list of two names for four hundred and thirty-five commits, and both
#    halves of it were wrong: `conformance` has never had a build script in any commit of this
#    repository, so half of every run looked for a thing that does not exist and found nothing —
#    which prints as a `✓` — while `crates/pdf-sandbox/build.rs`, which bakes the path and then
#    reads a directory under it, was never asked at all. A derived population could not have
#    contained the first or missed the second (ADR 0752, trap 25).
stale= superseded=0 asked=0
while read -r script_source; do
    grep -q 'env!("CARGO_MANIFEST_DIR")' "$script_source" || continue
    crate_path=${script_source%/build.rs}
    package=$(sed -n 's/^name *= *"\([^"]*\)".*/\1/p' "$crate_path/Cargo.toml" | head -1)
    [ -n "$package" ] || continue
    asked=$((asked + 1))
    #    The baked path ends in the crate's own path from the root, so the pattern is the crate's
    #    rather than a second list to keep in step with the first.
    scripts=$(ls -t "$target"/*/build/"$package"-*/build-script-build 2>/dev/null)
    newest=$(printf '%s\n' "$scripts" | head -1)
    for script in $scripts; do
        for baked in $(grep -aoE "/[A-Za-z0-9_./+-]*/$crate_path" "$script" 2>/dev/null | sort -u); do
            [ -d "$baked" ] && continue
            if [ "$script" = "$newest" ]; then stale="$stale $baked"; else superseded=$((superseded + 1)); fi
        done
    done
done < <(git ls-files 'crates/*/build.rs' 'tools/*/build.rs' 2>/dev/null)
if [ "$asked" -eq 0 ]; then
    fail "no build script bakes env!(\"CARGO_MANIFEST_DIR\") — this check has nothing to ask"
elif [ -z "$stale" ]; then
    pass "the newest build script of each crate that bakes its manifest path names a directory that exists"
else
    fail "the newest compiled build script names a directory that is gone:$stale"
    printf '    touch the build script source and rebuild — it is not the tree (doc/environment.md)\n'
fi
[ "$superseded" -gt 0 ] &&
    printf '    (%s superseded build scripts also name gone checkouts — other rounds, not this one)\n' "$superseded"

# 3. What a person can run, against what HEAD is. `doc/todo/02` §5 owns the fix; a stale binary
#    is a measurement of the past, which is the whole reason that section exists.
head_time=$(git log -1 --format=%ct 2>/dev/null)
oldest=$(ls -t target/pdf-viewer target/pdf-viewer-gtk target/pdf-viewer-qt target/pdf-retrieve \
             target/pdf-transform target/pdf-sandbox-worker target/pdf-view-worker \
             target/libviewer_ffi.so 2>/dev/null | tail -1)
if [ -z "$oldest" ]; then
    fail "target/ holds none of §5's binaries — nothing a person can run"
elif [ -n "$head_time" ] && [ "$(stat -c %Y "$oldest" 2>/dev/null || echo 0)" -lt "$head_time" ]; then
    fail "$oldest is older than HEAD — doc/todo/02 §5, and always before a measurement"
else
    pass "target/'s binaries are at least as new as HEAD"
fi

# 4. sccache folds every CARGO_* variable into its Rust cache key, so an *exported*
#    CARGO_TARGET_DIR gives this round a cache nothing will ever read again. --target-dir on the
#    command line is the same isolation and is invisible to the key. ADR 0344.
if [ -n "${CARGO_TARGET_DIR-}" ]; then
    fail "CARGO_TARGET_DIR is exported — use --target-dir instead, or sccache hits nothing (ADR 0344)"
else
    pass "CARGO_TARGET_DIR is not exported"
fi

# 5. Whether the pipeline on `main` is green, which is the one gate this project runs and cannot
#    see. Every round's gates are run in a worktree that branched before its neighbours' files
#    existed, and the merge round pushes and moves on — so a push that fails CI is nobody's news.
#    One did, for five runs and a week, on a Qt enumerator no machine here is old enough to lack
#    (ADR 0450). This is where the next round finds out, because a round reads this file first.
#
#    A *report*, not a gate, and the distinction is trap 10's: this asks GitHub, so it depends on
#    a network and a token and could not be a `✗` a round is obliged to clear. It says which of
#    the four it is — green, red, still running, or not asked — and none of the last three is
#    ever read as green.
#
#    **The fourth and the third used to be one**, which the six-hundred-and-thirtieth session
#    found on this check's first run against a live pipeline: a run that has not finished has an
#    empty `conclusion`, and an empty `conclusion` was the same case as an empty *answer*. So the
#    push the owner was waiting on printed "CI was not asked", which is the one sentence this
#    check exists not to print about a run it can see. `status` is what tells them apart.
if command -v gh > /dev/null 2>&1; then
    #    The token is a file beside the *main* worktree rather than inside the repository, which
    #    is what keeps it out of every commit — so it is found through git's common directory and
    #    never through a relative climb, a worktree being an arbitrary distance away.
    token=${GH_TOKEN-}
    if [ -z "$token" ]; then
        common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)
        beside=$(dirname "${common:-.}")/github-token.txt
        [ -r "$beside" ] && token=$(cat "$beside")
    fi
    run=$(GH_TOKEN="$token" gh run list --branch main --limit 1 \
              --json status,conclusion,displayTitle,databaseId \
              --jq '.[] | "\(.status)\t\(.conclusion)\t\(.databaseId)\t\(.displayTitle)"' 2>/dev/null)
    run_status=${run%%$'\t'*}
    run_conclusion=$(printf '%s' "$run" | cut -f2)
    run_where=$(printf '%s' "$run" | cut -f3-4 | tr '\t' ' ')
    if [ -z "$run" ]; then
        printf '  ! CI was not asked (no token, or no network) — its state here is unknown, not green\n'
    elif [ "$run_status" != completed ]; then
        printf '  ! CI is still %s on main — %s — so it is not green yet either\n' \
               "$run_status" "$run_where"
    elif [ "$run_conclusion" = success ]; then
        pass "CI's last run on main passed — $run_where"
    else
        fail "CI's last run on main is $run_conclusion: gh run view $(printf '%s' "$run" | cut -f3) --log-failed"
    fi
else
    printf '  ! CI was not asked (no gh on PATH) — its state here is unknown, not green\n'
fi

# 6. refs/stash lives in the common git directory, so every worktree shares one stack and a
#    parallel round will take yours. Not a failure — a thing to know before reaching for it.
if [ -n "$(git stash list 2>/dev/null)" ]; then
    printf '  ! the shared stash is not empty, and it is shared between worktrees — do not pop it\n'
    printf '    blind; doc/environment.md says how one round took a neighbour half-finished edit\n'
fi

printf '\n'
exit $status
