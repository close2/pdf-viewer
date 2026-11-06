#!/usr/bin/env bash
# Renders every page of the Ghent PDF Output Suite on the CPU, in the screen view and in §10.8.3's
# press view, so that a person can judge each patch against its README (`doc/gwg-output-suite.md`,
# ADR 2728). The patches a pixel comparison can decide are checked by
# `crates/pdf-model/tests/gwg_output_suite.rs`; this script is for the eye.
#
#   tools/gwg-render.sh <out-dir> [scale]     # scale 2.5 is 180 dpi; PNGs land in <out-dir>/{screen,press}/
#
# The suite is wherever `tools/places.sh` says `PDF_VIEWER_GWG` is (`tools/fetch-gwg.sh` puts it
# there). The renders are of a licensed suite and are never written into the tree: point
# <out-dir> at scratch.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=tools/places.sh
. "$here/places.sh"

out=${1:?usage: tools/gwg-render.sh <out-dir> [scale]}
scale=${2:-2.5}
if [ ! -d "$PDF_VIEWER_GWG" ]; then
    printf 'gwg-render: the suite is not at %s; tools/fetch-gwg.sh fetches it\n' "$PDF_VIEWER_GWG" >&2
    exit 1
fi
root=$(cd "$here/.." && pwd -P)
case $(realpath -m "$out") in "$root" | "$root"/*) printf 'gwg-render: %s is inside the tree; renders of the suite stay out of it\n' "$out" >&2; exit 1 ;; esac

task_budget=$("$here/bounded.sh" --task-budget)
ulimit -u "$task_budget"
# A small walk under the heavy-walk lock, which builds what it runs inside its own hold: the sandbox
# worker decodes JBIG2 and JPX and Cargo does not rebuild it for an example (traps 10 and 109) —
# without it those patches draw nothing, which reads as a deficit of the renderer.
if [ -z "${HEAVY_WALK_HELD_BY:-}" ]; then
    exec "$here/bounded.sh" --lock --tree 6 --build '--release -p pdf-sandbox --bins' --build '--release -p pdf-model --example render_at' -- "$0" "$@"
fi
# The binary carries the tree it was built from, so its path is cargo's answer and not a guess (trap 15).
target=$(cargo metadata --format-version 1 --no-deps | grep -o '"target_directory":"[^"]*"' | cut -d'"' -f4)
binary=$target/release/examples/render_at

mkdir -p "$out/screen" "$out/press"
failed=0
drawn=0
while IFS= read -r -d '' pdf; do
    name=$(basename "$pdf" .pdf | tr ' ' '_')
    pages=$(pdfinfo "$pdf" 2>/dev/null | awk '/^Pages:/ {print $2}')
    for ((page = 1; page <= ${pages:-1}; page++)); do
        for mode in screen press; do
            flag=; [ "$mode" = press ] && flag=--simulate
            if ! "$binary" "$pdf" "$page" "$scale" "$out/$mode/$name-p$page.png" $flag >/dev/null 2>"$out/$mode/$name-p$page.err"; then
                printf 'gwg-render: %s page %d (%s) failed: %s\n' "$name" "$page" "$mode" \
                    "$(tail -1 "$out/$mode/$name-p$page.err")" >&2
                failed=$((failed + 1))
            fi
            drawn=$((drawn + 1))
        done
    done
# The trailing slash follows a worktree's link into the main checkout's copy.
done < <(find "$PDF_VIEWER_GWG/" -name 'GWG*.pdf' -path '*/Patches/*' -print0 -o \
    -name 'Ghent_PDF-Output-Test-V50_*.pdf' -print0)
printf 'gwg-render: %s of %s page renders failed to draw\n' "$failed" "$drawn"
# A suite in which nothing was found is a broken place, not a clean run (trap 25).
[ "$drawn" -gt 0 ] && [ "$failed" -eq 0 ]
