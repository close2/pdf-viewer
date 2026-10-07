#!/usr/bin/env bash
#
# What this round is, before it starts.
#
# `tools/state.sh` answers "what are the numbers"; this one answers the questions a round asks
# *before* it has done anything: which session is next, what to read for the kind of work it is
# about, which of `doc/todo/02` §2's gates that kind of change actually needs, and whether this
# round owes the full sequence. Then it checks the things a round has
# actually got wrong here — an uninitialised submodule, a build script baked against a checkout
# that no longer exists, installed binaries of a commit `main` has moved past, an exported `CARGO_TARGET_DIR`, and
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
#   tools/round.sh --lines [kind]  # how many lines each list is, which is what a review measures
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
#
# The trap column names the group **file** and never its trap numbers. It listed them until session
# 967, and every one of the five lists had gone stale as traps were added — doc/traps/README.md is
# where a trap's number, its position and its rule live, and a second copy of them here was a copy
# nothing checked. That index is in the standing list below; a group file is what a round opens
# where a line of it bites (ADR 1036).
#
# The habits are named the same way, one file per kind of work rather than doc/habits.md plus the
# title of a section inside it, which is what this script could say before ADR 0983 split that file.
kinds="pixels oracle parsers loop instruments clause measure host script writer archive fuzz dependency docs"

# Each kind's list is SHORT on purpose: the every-round list below is what every round reads, and
# a kind adds the two or three files that kind of work cannot do without — its trap group, the
# habit file for its method, and the one document that states what it is changing. Everything
# else is one hop away through doc/HANDOVER.md, which is the index and not a reading list
# (ADR 1639). `tools/round.sh --lines [kind]` prints how long each list is, which is the figure
# doc/reviews/1401 keeps an eye on.
kind_reading() {
    case $1 in
    pixels)
        printf 'doc/traps/pixels-and-rasterisers.md      the group for anything that can move a pixel\n'
        printf 'doc/habits/judging-against-other-implementations.md  a differing page is read against the clause, not a reference\n' ;;
    oracle)
        printf 'doc/traps/oracle-and-references.md       the group for a verdict, a reference or a tolerance\n'
        printf 'doc/habits/judging-against-other-implementations.md  what an agreement is evidence of\n'
        printf 'doc/oracle-and-corpus.md                 the instrument itself; doc/todo/00 for the bucket and step 7\n' ;;
    parsers)
        printf 'doc/traps/parsers-and-streams.md         the group for a parser, a filter, a font or a codec\n'
        printf 'doc/verify.md                            the fuzz block: which target covers what you touched\n' ;;
    loop)
        printf 'doc/traps/the-interactive-loop.md        the group for a press, a space or a toolkit loop\n'
        printf 'doc/ui-boundary.md                       the boundary, and the test a message must pass\n' ;;
    instruments)
        printf 'doc/traps/instruments-and-reports.md     the group for a gate, a number or a report\n'
        printf 'doc/habits/tests-gates-and-reports.md     what discriminates, and what a ratchet says\n' ;;
    clause)
        printf 'doc/habits/reading-the-specification.md   modal verbs, silences, and what doc/md/ is\n'
        printf 'doc/habits/the-ledger-and-claims-about-this-tree.md  how a row or a reason goes stale\n'
        printf 'doc/todo/01-ledger-partial-rows.md       the sweeps, as commands; doc/todo/65 for the frontier map a status change edits\n' ;;
    measure)
        printf 'doc/habits/measuring.md                  A/B in one sitting, and which number to quote\n'
        printf 'doc/traps/instruments-and-reports.md     what a gate is about to lie to you about\n'
        printf 'doc/performance.md                       section 3e: the frame table and its gate, turn_path\n' ;;
    host)
        printf 'doc/ui-boundary.md                       Command/Event/Query/Answer, and the freeze\n'
        printf 'doc/traps/the-interactive-loop.md        the group for a press, a space or a toolkit loop\n'
        printf 'doc/todo/30-a-native-host.md             what the hosts still owe; tools/drive-windows.sh is the last thing you run\n' ;;
    script)
        printf 'doc/rfc/0008-a-script-is-a-document-acting-on-its-reader.md  accepted (A193): built in its section 11 order\n'
        printf 'doc/traps/instruments-and-reports.md     traps 118 and 121: a confined Boa context, a column that changes what it walks\n'
        printf 'doc/todo/56-a-script-engine-that-is-memory-safe.md  the engine and its ceilings\n' ;;
    writer)
        printf 'doc/todo/57-the-transform-suite.md       the suite, Annex F among it; doc/rfc/0002 for the design\n'
        printf 'doc/traps/parsers-and-streams.md         traps 47, 89 and 90: page lists, a writer that mirrors a reader\n' ;;
    archive)
        printf 'doc/rfc/0006-pdf-a-validation-and-conversion.md  the design; doc/rfc/0007 for a refusal as a question\n'
        printf 'doc/pdf-a-mitigations.md                 every refusal and its remedy; doc/todo/66 for what this version carries out\n'
        printf 'doc/third-party-data.md                  ISO 19005 is cited by section and paraphrased, never quoted\n' ;;
    fuzz)
        printf 'doc/verify.md                            the fuzz block: tools/fuzz.sh, fuzz/seeds.sh check, -s none behind the lock\n'
        printf 'doc/traps/instruments-and-reports.md     traps 107, 111 and 112: a stale seed corpus, cargo fuzz run, a sanitiser build under a bound\n' ;;
    dependency)
        printf 'doc/stack.md                             the stack, and why rustybuzz is not in it\n'
        printf 'doc/third-party-data.md                  what a datum has to be before it is trusted\n' ;;
    docs)
        printf 'doc/HANDOVER.md                          the index this script is the companion to\n'
        printf 'doc/habits/the-ledger-and-claims-about-this-tree.md  how a claim about this tree goes stale\n' ;;
    *) return 1 ;;
    esac
}

kind_gates() {
    case $1 in
    pixels)      printf 'everything of tier 2 — a change that can move a pixel runs the whole of section 2, and looks at a page\n' ;;
    oracle)      printf 'the core, the oracle gate and the corpus gate; everything if the change is in pdf-model\n' ;;
    parsers)     printf 'everything of tier 2 — pdf-syntax, pdf-font and pdf-model are under every gate\n' ;;
    loop)        printf 'the core, selection_census and accessibility_census\n' ;;
    instruments) printf 'the core, plus whichever gate the instrument is\n' ;;
    clause)      printf 'the core and cargo test -p conformance; everything if code changed\n' ;;
    measure)     printf 'the core — and a --release build of what is measured first, always, because a stale binary measures the past\n' ;;
    host)        printf 'the core, which builds and tests every host, and a drive under Xvfb with more than one document\n' ;;
    script)      printf 'the core; --features engine --test script_corpus behind the lock, and -p pdf-model --test script_corpus where the view state is reached\n' ;;
    writer)      printf 'the core and -p pdf-transform --test gate; the writer walks are the merge'"'"'s unless the change is what they assert\n' ;;
    archive)     printf 'the core; -p pdf-archive --test corpus and -p pdf-transform --test archive_corpus behind the lock\n' ;;
    fuzz)        printf 'the core and cargo test -p conformance --test fuzz_workspace; the campaign itself behind the lock\n' ;;
    dependency)  printf 'everything, plus cargo deny (doc/verify.md)\n' ;;
    docs)        printf 'the core and cargo test -p conformance, plus --bin quotations and --bin pointers\n' ;;
    esac
}

# The every-round list, as `path  what it is`; a `#section` suffix names the one section a round
# reads of a longer file, and `--lines` counts that section alone.
every_round_reading() {
    printf 'CLAUDE.md                                the five principles, and what *done* means\n'
    printf 'doc/todo/02-every-round.md#0             section 0: the round on one page — the contract, tier 1, the record\n'
    printf 'doc/environment.md#rules                 the rule block that opens it: one line per shared-machine rule\n'
    printf 'doc/traps/every-round.md                 the ten traps that carry four-fifths of the citations\n'
    printf 'doc/habits/every-round.md                the ten habits the records show rounds paying for\n'
}

# How long a reading list is, in lines: `wc -l` per file, and for a `#section` entry the lines
# from that section'"'"'s heading to the next heading of the same level — a fact on the disk, which
# is the one kind of number this script prints.
lines_of() {
    local entry path section
    while read -r entry _; do
        path=${entry%%#*}; section=${entry#*#}
        if [ "$section" = "$entry" ]; then
            printf '  %6s  %s\n' "$(wc -l < "$path")" "$path"
        else
            case $section in
            0) printf '  %6s  %s (section 0)\n' "$(sed -n '/^## 0\./,/^## 1\./p' "$path" | wc -l)" "$path" ;;
            rules) printf '  %6s  %s (the rule block)\n' "$(sed -n '/^## The rules/,/^## Working/p' "$path" | wc -l)" "$path" ;;
            esac
        fi
    done
}

case ${1-} in
--list) printf '%s\n' $kinds; exit 0 ;;
--lines)
    heading "lines a round reads before it writes anything"
    printf '  every round:\n'; every_round_reading | lines_of
    if [ -n "${2-}" ]; then
        if kind_reading "$2" >/dev/null 2>&1; then
            printf '  and as a "%s" round:\n' "$2"; kind_reading "$2" | lines_of
        else
            printf '\nno such kind: %s (tools/round.sh --list)\n' "$2" >&2; exit 1
        fi
    fi
    exit 0 ;;
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

# **Neither source outranks the other; the later of the two is the answer.** Each is a floor that
# goes stale in its own direction, and taking either alone has now been wrong here twice.
#
# `ls doc/history/` is too low in a parallel worktree, which is branched before its neighbours have
# written their files — it told the six-hundred-and-eighty-seventh and the six-hundred-and-ninety-first
# that they were session 685, and told both they owed a fifth round's obligations they did not owe.
#
# **The branch name is too low the moment a branch carries more than one round**, which is what this
# project actually does — `round-945/the-fifth-round` carried sessions 946 to 959. The comment here
# used to say the branch "cannot go stale"; it went stale for fourteen rounds, wrote "Session 945"
# into two ADRs beside a `doc/history/945-` file about something else, and — because 945 divides by
# five — printed **a fifth round** every single round, which is the worse half. An always-yes signal
# is not a conservative failure. It over-runs the gates, so nothing breaks, and it destroys the one
# thing the signal exists to say (ADR 0963).
#
# So: the maximum. A worktree branched as `round-960/…` takes 960 over a `doc/history/` that stops
# at 954; the fourteenth round on `round-945/…` takes `doc/history/` over the branch. Neither case
# is stale, and neither needs anybody to remember which source to trust.
branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)
case "$branch" in
    round-[0-9]*)
        # `round-940/pdf-a-validator` is the shape this project actually uses, so the number has
        # to be cut out of the name rather than assumed to be the whole of it: taking everything
        # after `round-` gave `940/pdf-a-validator`, and the first arithmetic on it — the fifth
        # round test below — failed with `pdf: unbound variable` under `set -u`, which is the
        # round opener refusing to open a round.
        assigned=${branch#round-}
        assigned=${assigned%%[!0-9]*}
        if [ -n "$assigned" ] && [ "$assigned" -gt "$session" ]; then
            session=$assigned
            last=$((session - 1))
            from="the branch name"
        else
            from="doc/history/, over a branch name that has carried $((session - assigned)) rounds"
        fi
        ;;
esac

heading "the round"
printf '  session %s, after %s (from %s)\n' "$session" "$last" "$from"
printf '  tier 1 every round, tier 2 by doc/todo/02 section 2'"'"'s map, tier 3 at the merge (ADR 1036);\n'
printf '  the merge runs all three tiers once per batch, so no round of a batch owes the whole sequence (ADR 1638)\n'

# ---------------------------------------------------------------- the reading

heading "read, whatever this round is (tools/round.sh --lines counts it)"
every_round_reading | sed 's/^/  /'
printf '  doc/HANDOVER.md is the index to open for a file this list did not give you, not a reading\n'

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
#    It was a hand-written list of two names, and both halves of it were wrong. Trap 25 in
#    `doc/traps/instruments-and-reports.md` is that incident and the general shape (ADR 0752).
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

# 3. What a person can run, against what `main` is. The main checkout's `target/` is the one a person
#    runs from, `tools/batch.sh install` fills it at a batch boundary, and `target/installed-from`
#    names the commit (ADR 1511). A round never installs there and never measures from there: a
#    measurement builds its own `--release` binary first (`doc/todo/02` section 5).
main_checkout=$(dirname "$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)")
installed=$(sed -n 's/^# commit //p' "$main_checkout/target/installed-from" 2>/dev/null | head -1)
if [ -z "$installed" ]; then
    fail "$main_checkout/target/ names no commit — no tools/batch.sh install has run since ADR 1511"
elif [ "$(git rev-list --count "$installed..main" 2>/dev/null || echo unknown)" != 0 ]; then
    fail "$main_checkout/target/ is $(git rev-list --count "$installed..main" 2>/dev/null || echo an unknown number of) commit(s) behind main — the merge's tools/batch.sh install"
else
    pass "$main_checkout/target/ holds main's own commit"
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
#    **The fourth and the third are told apart by `status`**: a run that has not finished has an
#    empty `conclusion`, and read alone that is the same case as an empty *answer*, which would
#    print "CI was not asked" about a run this check can see — the one sentence it exists not to
#    print (ADR 0463).
#
#    Which job failed, on which step, and whether GitHub ran it at all is `tools/state.sh
#    main-checkout`'s, which reads the public interface and needs no token (ADR 1563): the token
#    below is the owner's, and a round's shell has none of its own.
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
        fail "CI's last run on main is $run_conclusion: run $(printf '%s' "$run" | cut -f3); its jobs: tools/state.sh main-checkout"
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

# 7. Every crate of this tree is one the citation and quotation sweeps read. The check itself is
#    `cargo test -p conformance --test workspaces` — session 1004 built it here because
#    `tools/conformance` was another round's, and session 1010, whose it was, derived the roots
#    and moved it into the crate the defect lives in. What stays here is the *pointer*: a round
#    that has not run the tests should still be told where the question is answered, and a copy
#    of the answer in two places is the drift this project has a rule about.
printf '  · every workspace member is read by the conformance sweeps:'
printf ' `cargo test -p conformance --test workspaces` (ADR 1029)\n'

printf '\n'
exit $status
