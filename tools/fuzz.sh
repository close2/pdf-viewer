#!/usr/bin/env bash
#
# Run one fuzz target, and say whether it fuzzed anything.
#
# A fuzz target's exit status answers *did it crash*. It does not answer *did it run*, and the
# difference is this project's own worst failure shape: an instrument that reports success without
# having done its job. The eight-hundredth round watched `page` execute 86 912 iterations against
# an empty corpus and exit 0; the eight-hundred-and-tenth measured what that is worth, and the
# answer is that from nothing at all `page` reaches the same 182 features the `document` target
# reaches — its whole reason for existing, `pdf_model::interpret`, is not entered once. A fuzzer
# will not invent a header, a cross-reference section, a page tree and a resource dictionary that
# agree with each other, and no amount of wall clock changes that. ADR 0742.
#
# So this wrapper asks two questions the bare command does not:
#
#   before  Is there a corpus?  An empty corpus directory stops the run, because in this tree every
#           target has one and an empty one means it was lost — a fresh worktree, a clean checkout,
#           a deletion — rather than chosen. `--from-nothing` is how the deliberate case says so.
#   after   Did the run cover anything?  libFuzzer's own final line carries `cov:` and `ft:`, and a
#           run that ends with no features found nothing to fuzz whatever its exit status was.
#
#   tools/fuzz.sh <target> [--from-nothing] [-- <extra libFuzzer arguments>]
#   tools/fuzz.sh --list                    the targets, and how many seeds each has here
#
# **The invocation comes out of `doc/verify.md`, not out of this script.** That file states one
# `cargo +nightly fuzz run <target>` line per target with the flags that target needs — `page` is
# forked over six processes and `x509` runs a million times — and two places stating one command is
# how they drift (ADR 0232 §4). A target with no line there is refused, which is what makes a new
# target arrive documented.
#
# `cargo-fuzz` is installed and is in `~/.cargo/bin`, which is not on `PATH`; two rounds read that
# as its absence. This script puts it there.

set -u -o pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
verify="$root/doc/verify.md"
export PATH="$HOME/.cargo/bin:$PATH"

# Every `[[bin]]` `fuzz/Cargo.toml` declares, which is the population by cargo's own reckoning
# rather than by a list kept here.
targets() {
    awk '/^\[\[bin\]\]/ { want = 1; next }
         want && /^name *= *"/ { gsub(/^name *= *"|"$/, ""); print; want = 0 }' \
        "$root/fuzz/Cargo.toml"
}

# The arguments `doc/verify.md` gives this target, with the trailing comment removed.
#
# Printed on its own line so that the caller sees the command that ran, and matched on the whole
# `fuzz run <target>` phrase so that a target whose name is a prefix of another's cannot answer for
# it.
documented_arguments() {
    local target=$1
    grep -E "cargo \+nightly fuzz run +${target}( |\$)" "$verify" \
        | head -1 \
        | sed -e 's/#.*$//' -e "s/.*cargo +nightly fuzz run  *${target} *//"
}

seeds_in() {
    local target=$1 dir="$root/fuzz/corpus/$1"
    [ -d "$dir" ] || { echo 0; return; }
    find "$dir" -maxdepth 1 -type f | wc -l
}

if [ "${1:-}" = "--list" ]; then
    printf '%-16s %8s   %s\n' target seeds 'doc/verify.md'
    for t in $(targets); do
        args=$(documented_arguments "$t")
        printf '%-16s %8s   %s\n' "$t" "$(seeds_in "$t")" \
            "${args:-NO INVOCATION — this target is in fuzz/Cargo.toml and not in doc/verify.md}"
    done
    exit 0
fi

target=${1:-}
[ -n "$target" ] || { sed -n '3,32p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 2; }
shift

from_nothing=0
if [ "${1:-}" = "--from-nothing" ]; then from_nothing=1; shift; fi
[ "${1:-}" = "--" ] && shift

targets | grep -qx "$target" || {
    echo "fuzz.sh: fuzz/Cargo.toml declares no target named '$target'. Its targets are:"
    targets | sed 's/^/    /'
    exit 2
}

arguments=$(documented_arguments "$target")
[ -n "$arguments" ] || {
    echo "fuzz.sh: doc/verify.md states no 'cargo +nightly fuzz run $target' line."
    echo
    echo "That file owns the invocation — which flags this target needs, how long it runs, and"
    echo "where its seeds come from — and a target it does not name is a target nobody can run"
    echo "the way it is meant to be run. Add the line, then this command works."
    exit 2
}

before=$(seeds_in "$target")
if [ "$before" -eq 0 ] && [ "$from_nothing" -eq 0 ]; then
    echo "fuzz.sh: $target has an empty corpus (fuzz/corpus/$target), so this run would fuzz from"
    echo "nothing. That is not a slower run, it is a different one: see doc/verify.md's block for"
    echo "this target, which says where its seeds come from."
    echo
    echo "  fuzz/corpus is gitignored, so a fresh checkout has none. tools/worktree.sh links a"
    echo "  worktree's to the primary checkout's; a clone re-seeds from the scripts in fuzz/."
    echo
    echo "  Pass --from-nothing if an empty corpus is what this run is actually for."
    exit 1
fi

log=$(mktemp)
trap 'rm -f "$log"' EXIT

echo "fuzz.sh: $target, $before seeds, doc/verify.md's own invocation:"
echo "    cd fuzz && cargo +nightly fuzz run $target $arguments $*"
echo

# `cd` into `fuzz/` because that is where `doc/verify.md` states the command from, and cargo-fuzz
# resolves `fuzz/corpus/<target>` relative to the crate it finds there.
(cd "$root/fuzz" && eval cargo +nightly fuzz run "$target" "$arguments" "$@") 2>&1 | tee "$log"
status=${PIPESTATUS[0]}

# libFuzzer's own last word. `DONE` carries it for an ordinary run and a fork-mode parent prints
# the same three counters without it, so the match is on the counters rather than on the verb.
#
# **The zero this catches is a fork-mode zero**, and knowing that is what stops the check being
# decoration (trap 11). An ordinary run always executes the empty input at `INITED`, so `ft` is
# never zero however barren the corpus — `display_list` from nothing reports `cov: 31 ft: 32`. A
# *parent* under `-fork` reports the shared corpus instead, and one that starts empty and is fed by
# children finding nothing stays at `cov: 0 ft: 0 corp: 0` for as many iterations as it is given.
# That is round 800's observation exactly, and `-fork` is what `page` needs to run at all.
final=$(grep -oE 'cov: [0-9]+ ft: [0-9]+ corp: [0-9]+' "$log" | tail -1)
after=$(seeds_in "$target")

echo
if [ -z "$final" ]; then
    echo "fuzz.sh: $target — libFuzzer printed no coverage line at all, so nothing here can say"
    echo "whether it fuzzed. Read the output above: a build failure, a refused flag and a crash on"
    echo "the first input all look like this, and only the first two are the harness's fault."
    exit 1
fi

features=$(echo "$final" | sed -E 's/.*ft: ([0-9]+).*/\1/')
echo "fuzz.sh: $target — $final, seeds $before → $after, cargo-fuzz exit $status"
if [ "$features" -eq 0 ]; then
    echo
    echo "fuzz.sh: this run found no features, which means it exercised no code path the"
    echo "instrumentation can see. Whatever its exit status, it did not fuzz $target."
    exit 1
fi
exit "$status"
