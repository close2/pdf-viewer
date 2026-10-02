#!/usr/bin/env bash
#
# Seed every fuzz target's corpus from what this disk holds, into `<root>/<target>` (default
# `fuzz/corpus`, which is gitignored, so a clone starts with none).
#
#   fuzz/seeds.sh [<root> [<target>...]]      every target, or the ones named
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

set -eu -o pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
tree=$(cd -- "$here/.." && pwd)
root=${1:-$here/corpus}
[ $# -gt 0 ] && shift
mkdir -p "$root"
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
    # Whole documents, `seed_page.py`'s recipe: each of these reads a file, or tokenises one.
    page | document | serialize | lexer | object | linearize)
        documents | xargs -0 python3 "$here/seed_page.py" "$root/$t" > /dev/null
        # `page` alone has the second seeder, for the four nested content streams no document
        # states past the memo's allowance (`doc/verify.md` under `page`).
        if [ "$t" = page ]; then python3 "$here/seed_nested_content.py" "$root/$t" > /dev/null; fi
        ;;
    # One object out of a document each, in the framing each target reads.
    xmp | sfnt | cmap | ccitt | crypt | variable_text)
        documents | python3 "$here/seed_streams.py" "$t" "$root/$t" -
        ;;
    # §7.4.7 and §7.4.9: every codestream the documents hold, in the framing each target reads.
    jbig2 | jpx)
        (cd "$tree" && cargo build -q --release -p pdf-model --example image_codec_seeds)
        mkdir -p "$root/jbig2" "$root/jpx"
        documents | xargs -0 "$(built)/release/examples/image_codec_seeds" "$root/jbig2" "$root/jpx"
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

if [ $# -gt 0 ]; then chosen=$*; else chosen=$(targets); fi
for t in $chosen; do seed "$t"; done
