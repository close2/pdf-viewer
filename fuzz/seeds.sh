#!/usr/bin/env bash
#
# Seed every fuzz target's corpus from what this disk holds, into `<root>/<target>` (default
# `fuzz/corpus`, which is gitignored, so a clone starts with none).
#
#   fuzz/seeds.sh [<root> [<target>...]]      every target, or the ones named
#   fuzz/seeds.sh check [<target>...]          is the corpus on disk stale? (below)
#
# A target fuzzed from nothing does not reach what it exists for — libFuzzer will not invent a
# JPEG 2000 box, a JBIG2 segment header or a cross-reference table that agrees with its objects
# (ADR 0742) — so every target has a recipe, and **the recipe is this file's `case` arm for it**.
# `tools/conformance/tests/fuzz_workspace.rs` fails on a file under `fuzz/fuzz_targets/` without
# one, beside its `doc/verify.md` line, so a new target arrives seeded (ADR 1439). The prose about
# a recipe — why a population is the one it is, what a seeder cannot see — stays under the target's
# line in `doc/verify.md`; the command is here.
#
# It walks `corpus-cache`, `doc/corpora` and `doc/pdf.js/test/pdfs`, which is a census over the
# corpus: run it behind the lock, `flock /home/AI/heavy-walk.lock fuzz/seeds.sh`. `-L` because a
# worktree's corpora are symbolic links into the main checkout.
#
# **Every recipe over those documents keeps one seed per shape** (ADRs 1559, 1571): the population
# is some ninety thousand files and 126 GB, and kept whole each recipe wrote tens of thousands of
# seeds and gigabytes. A seeder searches a document's memory map for the name its target needs
# before reading it, and keeps the smallest seed of each shape it states beside the reason — the
# shape being what the target's code branches on. `--every`, given to a seeder, writes the
# population whole, which is what a shape is proved against.
#
# **`check` says whether the corpus on disk is stale**, and writes nothing there. A seeded corpus
# goes stale with nothing failing — a seeder learns a new route, a target grows a branch — and a
# campaign started on it spends its clock rediscovering what fresh seeds hand over at once
# (trap 107). So `check` seeds the target afresh into a scratch directory beside the build output,
# asks libFuzzer for its `INITED cov` — the coverage the corpus gives before one mutation — once
# over the disk corpus and once over the fresh seeds, each a `-runs=0` pass with the limits
# `doc/verify.md` gives the target, and prints both. It says **stale** when the fresh seeds reach
# more edges than the disk corpus by more than `STALE_MARGIN_PERCENT` of the fresh figure
# (`margin` below says why that much), and then what a re-seed would add and where. Each pass
# names the disk corpus *second*, behind an empty scratch directory, because libFuzzer writes what
# it keeps into the first directory it is given: the owner's corpus is read and never written.
# A census like the rest, so behind the lock. ADR 1559.

set -eu -o pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
tree=$(cd -- "$here/.." && pwd)
if [ "${1:-}" = check ]; then
    mode=check
    shift
else
    mode=seed
    root=${1:-$here/corpus}
    [ $# -gt 0 ] && shift
    mkdir -p "$root"
fi
export PYTHONDONTWRITEBYTECODE=1

targets() {
    awk '/^\[\[bin\]\]/ { want = 1; next }
         want && /^name *= *"/ { gsub(/^name *= *"|"$/, ""); print; want = 0 }' "$here/Cargo.toml"
}

# Every document on this disk, NUL-separated, for the seeders that read the list on standard input.
documents() {
    find -L "$tree/corpus-cache" "$tree/doc/corpora" "$tree/doc/pdf.js/test/pdfs" \
        -name '*.pdf' -print0 2>/dev/null || true
}

# Where cargo puts this workspace's build output, which a worktree names in `.cargo/config.toml`.
built() {
    (cd "$tree" && cargo metadata --format-version 1 --no-deps \
        | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
}

counted() { echo "seeds.sh: $(find "$root/$1" -maxdepth 1 -type f | wc -l) $1 seeds in $root/$1"; }

seed() {
    local t=$1
    mkdir -p "$root/$t"
    case $t in
    # `seed_page.py`'s recipe: each of these reads a file, tokenises one or parses its objects,
    # and each keeps the smallest document (or object) of each shape the script states.
    page | document | serialize | lexer | object | linearize)
        documents | python3 "$here/seed_page.py" "$t" "$root/$t" - > /dev/null
        # `page` alone has the second seeder, for the four nested content streams no document
        # states past the memo's allowance (`doc/verify.md` under `page`).
        if [ "$t" = page ]; then python3 "$here/seed_nested_content.py" "$root/$t" > /dev/null; fi
        ;;
    # One object out of a document each, in the framing each target reads.
    xmp | sfnt | cmap | ccitt | crypt | variable_text)
        documents | python3 "$here/seed_streams.py" "$t" "$root/$t" -
        ;;
    # §7.4.7 and §7.4.9: the codestreams of the documents naming either filter, in the framing
    # each target reads, written whole into a scratch directory beside the build output and then
    # the smallest of each shape `seed_codecs.py` states kept.
    jbig2 | jpx)
        (cd "$tree" && cargo build -q --release -p pdf-model --example image_codec_seeds)
        local every
        every=$(mktemp -d "$(built)/seeds-codecs.XXXXXX")
        documents | python3 "$here/seed_shape.py" naming JBIG2Decode JPXDecode \
            | xargs -0 -r "$(built)/release/examples/image_codec_seeds" "$every/jbig2" "$every/jpx" \
            > /dev/null
        python3 "$here/seed_codecs.py" jbig2 "$every/jbig2" "$root/jbig2"
        python3 "$here/seed_codecs.py" jpx "$every/jpx" "$root/jpx"
        rm -rf "$every"
        ;;
    # §7.4.8's frames: every `DCTDecode` stream of the documents, and the band modules' own
    # fixtures under four band heights.
    jpeg_bands)
        documents | python3 "$here/seed_streams.py" "$t" "$root/$t" -
        for f in "$tree"/crates/pdf-model/tests/cut/*.jpg "$tree"/crates/pdf-model/tests/restart/*.jpg; do
            for b in 0 1 3 7; do
                { printf "\\x0$b"; cat "$f"; } > "$root/$t/$(basename "$f" .jpg)-$b"
            done
        done
        ;;
    # The faces this machine offers, which is what `pdf_font::embed` writes into a document.
    embed)
        find /usr/share/fonts -size -128k \( -name '*.ttf' -o -name '*.otf' \) -print0 2>/dev/null \
            | xargs -0 python3 "$here/seed_embed.py" "$root/$t" > /dev/null
        ;;
    # Readbacks that fold, decompose and carry marks, with needles typed against them.
    find) python3 "$here/seed_find.py" "$root/$t" > /dev/null ;;
    # The meet's own test shapes and polygons on a sixteenth-pixel grid.
    meet) python3 "$here/seed_meet.py" "$root/$t" > /dev/null ;;
    # Small documents under each of RFC 0003's five write verbs.
    vfs_write)
        find -L "$tree/doc/pdf.js/test/pdfs" -name '*.pdf' -size -32k -print0 2>/dev/null \
            | xargs -0 python3 "$here/seed_vfs_write.py" "$root/$t" > /dev/null
        ;;
    # The UCD's bidirectional cases and cursive words.
    shaping) python3 "$here/seed_shaping.py" "$root/$t" ;;
    # §12.7.8's FDF files, one per entry the reader handles, and §7.9.4's dates.
    forms_data) python3 "$here/seed_forms_data.py" "$root/$t" ;;
    # A server's answer to the form in `fetched_import.rs`: FDF naming its fields, and every XFDF
    # file the tests hold, each under the eight routes the first byte chooses (ADR 1527).
    fetched_import)
        python3 "$here/seed_fetched_import.py" "$root/$t" "$tree"/crates/pdf-model/tests/xfdf/*.xfdf \
            > /dev/null
        ;;
    # ISO 19444-1's files: every XFDF file the tests hold, one annotation subtype each.
    xfdf) cp --update=none "$tree"/crates/pdf-model/tests/xfdf/*.xfdf "$root/$t/" ;;
    # §12.8's ASN.1, by the routes each seeder's own header names.
    cms) documents | python3 "$here/seed_cms.py" "$root/$t" - ;;
    revocation) documents | python3 "$here/seed_revocation.py" "$root/$t" - ;;
    x509) documents | python3 "$here/seed_x509.py" "$root/$t" "$tree"/crates/pdf-model/src/*.rs - ;;
    # Annex O's fragments arrive with a request, never in a file, so the seeds are the fragments
    # this tree's own reader and its tests state.
    fragment)
        grep -ohE '"(page|nameddest|zoom|view|viewrect|highlight|comment|search|fdf|ef|structure)=[^"]*"' \
            "$tree/crates/pdf-model/src/fragment.rs" "$tree/crates/viewer-core/tests/fragments.rs" \
            | tr -d '"' | while IFS= read -r fragment; do
                printf '%s' "$fragment" > "$root/$t/$(printf '%s' "$fragment" | sha256sum | cut -c1-64)"
            done
        ;;
    # The two targets whose input is a process: each seeder runs the program that writes it.
    confined_wire)
        (cd "$tree" && cargo build -q --release -p viewer-confined --bins)
        python3 "$here/seed_confined_wire.py" "$(built)/release/pdf-view-worker" "$root/$t" \
            "$tree"/doc/PDF20_AN002-AF.pdf "$tree"/doc/PDF-Declarations.pdf \
            "$tree"/doc/ISO_32000-2_sponsored_EC3.pdf "$tree"/doc/PDF20_AN001-BPC.pdf \
            "$tree"/doc/pdf.js/test/pdfs/issue15716.pdf
        ;;
    display_list)
        (cd "$tree" && cargo build -q --release -p viewer-confined --example list_over_the_wire)
        "$(built)/release/examples/list_over_the_wire" --seeds "$root/$t" \
            "$tree"/doc/pdf.js/test/pdfs/*.pdf > /dev/null
        ;;
    *)
        echo "seeds.sh: $t has no recipe here, which tests/fuzz_workspace.rs exists to refuse" >&2
        return 1
        ;;
    esac
    counted "$t"
}

# How far the fresh seeds may lead the disk corpus before it is called stale, as a share of the
# fresh figure. A disk corpus that has been fuzzed holds what its campaigns found and is expected to
# lead; it trails only when it is missing what the seeders now produce. Three `-runs=0` passes over
# one corpus print one figure for a single-threaded target, but a target whose code iterates a
# randomly seeded `HashMap` or splits work over threads can move a handful of edges between passes,
# so two per cent of the figure is noise and is not called stale. The gaps trap 107 is about were
# 4.6 and 9 times, so the margin cannot hide one. ADR 1559.
STALE_MARGIN_PERCENT=2

# The limits `doc/verify.md`'s line gives this target, without its run length or fork count: a
# `-runs=0` pass under a different memory limit or input length would report a corpus the campaign
# never loads, and a fork-mode parent prints no `INITED` at all.
limits() {
    grep -E "cargo \+nightly fuzz run +$1( |\$)" "$tree/doc/verify.md" | head -1 | sed -e 's/#.*$//' \
        | grep -oE -- '-(rss_limit_mb|timeout|max_len|malloc_limit_mb)=[0-9]+' | tr '\n' ' ' || true
}

# libFuzzer's `INITED cov` over the corpus `$2` of target `$1`, or a word saying why there is none.
# The pass is told to keep what it finds in `$3`, an empty scratch directory, so `$2` is only read;
# a seed that crashes the target ends the pass before `INITED`, its input goes to `$3`'s artefacts
# rather than to `fuzz/artifacts`, and the word is `crashed`.
inited() {
    local t=$1 corpus=$2 out=$3 log
    if [ ! -d "$corpus" ] || [ -z "$(find -L "$corpus" -maxdepth 1 -type f -print -quit)" ]; then
        echo absent
        return
    fi
    mkdir -p "$out/kept" "$out/artefacts"
    log="$out/libfuzzer.log"
    # Built by `cargo fuzz build` and run by path rather than by `cargo fuzz run`, which creates
    # `fuzz/artifacts/<target>` — a directory in the main checkout — whatever prefix it is given.
    # Without the sanitiser, as a campaign is (`doc/verify.md`): AddressSanitizer's shadow map is
    # what `tools/bounded.sh`'s `RLIMIT_DATA` refuses, and the figure compared is the one the
    # campaign that loads the corpus will see (ADR 1571).
    (cd "$here" && PATH="$HOME/.cargo/bin:$PATH" cargo +nightly fuzz build -O -s none "$t") > "$log" 2>&1 || true
    # shellcheck disable=SC2046 # the limits are separate words by design
    "$(built)/$(rustc -vV | sed -n 's/^host: //p')/release/$t" -runs=0 $(limits "$t") \
        -artifact_prefix="$out/artefacts/" "$out/kept" "$corpus" >> "$log" 2>&1 || true
    if grep -qE 'INITED cov: [0-9]+' "$log"; then
        grep -oE 'INITED cov: [0-9]+' "$log" | head -1 | grep -oE '[0-9]+$'
    elif grep -qE 'out-of-memory|rss limit' "$log"; then
        echo out-of-memory
    elif grep -qE 'ERROR: (libFuzzer|AddressSanitizer)|panicked at|deadly signal' "$log"; then
        echo crashed
    else
        echo failed
    fi
}

# Seeds `$1` afresh into a scratch directory and compares the two corpora's `INITED cov`. The
# scratch directory goes when the comparison is made; only a pass that gave no figure keeps its
# libFuzzer log, which the line naming it points at.
check() {
    local t=$1 scratch disk fresh_cov disk_cov margin missing seeds_disk seeds_fresh
    scratch=$(mktemp -d "${SEEDS_CHECK_DIR:-$(built)}/seeds-check-$t.XXXXXX")
    disk=$here/corpus/$t
    root=$scratch/seeds
    seed "$t" > /dev/null
    seeds_fresh=$(find "$root/$t" -maxdepth 1 -type f | wc -l)
    seeds_disk=$( (find -L "$disk" -maxdepth 1 -type f 2>/dev/null || true) | wc -l)
    disk_cov=$(inited "$t" "$disk" "$scratch/disk")
    fresh_cov=$(inited "$t" "$root/$t" "$scratch/fresh")
    echo "seeds.sh check $t: disk $seeds_disk seeds, INITED cov $disk_cov; fresh $seeds_fresh seeds," \
         "INITED cov $fresh_cov"
    case $fresh_cov in
    *[!0-9]*)
        echo "seeds.sh check $t: not judged — the fresh seeds give no figure ($fresh_cov) under" \
             "doc/verify.md's limits; $scratch/fresh/libfuzzer.log says why"
        rm -rf "$root"
        return
        ;;
    esac
    case $disk_cov in
    *[!0-9]*)
        echo "seeds.sh check $t: STALE — the disk corpus gives no figure ($disk_cov), so a campaign" \
             "on it starts from less than a re-seed would give it"
        ;;
    *)
        margin=$((fresh_cov * STALE_MARGIN_PERCENT / 100))
        if [ $((fresh_cov - disk_cov)) -le "$margin" ]; then
            echo "seeds.sh check $t: current — the disk corpus reaches $disk_cov edges to the fresh" \
                 "seeds' $fresh_cov, inside a margin of $margin"
            rm -rf "$scratch"
            return
        fi
        echo "seeds.sh check $t: STALE — the fresh seeds reach $((fresh_cov - disk_cov)) edges more" \
             "than the disk corpus, past a margin of $margin"
        ;;
    esac
    # What a re-seed would do. The seeders only add, so it is the fresh seeds whose bytes the disk
    # corpus does not already hold, written where the corpus link resolves.
    missing=$(comm -13 \
        <( (find -L "$disk" -maxdepth 1 -type f -exec sha1sum {} + 2>/dev/null || true) | cut -c1-40 | sort -u) \
        <(find "$root/$t" -maxdepth 1 -type f -exec sha1sum {} + | cut -c1-40 | sort -u) | wc -l)
    echo "seeds.sh check $t: a re-seed, \`fuzz/seeds.sh fuzz/corpus $t\` behind the lock, would add" \
         "$missing of the $seeds_fresh fresh seeds to $(readlink -f "$here/corpus")/$t and remove nothing"
    rm -rf "$scratch"
}

if [ $# -gt 0 ]; then chosen=$*; else chosen=$(targets); fi
if [ "$mode" = check ]; then
    for t in $chosen; do check "$t"; done
    exit 0
fi

for t in $chosen; do seed "$t"; done
