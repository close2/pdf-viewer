#!/usr/bin/env bash
#
# Seed the fuzz targets whose corpus a script here can build from what this disk holds, into
# `<root>/<target>` (default `fuzz/corpus`, which is gitignored, so a clone starts with none).
#
#   fuzz/seeds.sh [<root>]
#
# A target fuzzed from nothing does not reach what it exists for — libFuzzer will not invent a
# JPEG 2000 box, a JBIG2 segment header or a cross-reference table that agrees with its objects
# (ADR 0742) — so every target has a recipe, and this runs the ones for the targets that arrived
# with ADR 1423 — `shaping`, `xfdf`, `linearize`, `jbig2` and `jpx` — and for `forms_data`, whose
# corpus held no FDF file at all. The older targets keep the
# recipe `doc/verify.md` states under each one's line; the scripts they name are beside this one.
#
# It walks `doc/pdf.js/test/pdfs` and `doc/corpora`, which is a census over the corpus: run it
# behind the lock, `flock /home/AI/heavy-walk.lock fuzz/seeds.sh`.

set -eu -o pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
tree=$(cd -- "$here/.." && pwd)
root=${1:-$here/corpus}
mkdir -p "$root"

# §7.9.2.2.1's Unicode text: the UCD's bidirectional cases and cursive words.
python3 "$here/seed_shaping.py" "$root/shaping"

# §12.7.8's FDF files, one per entry the reader handles, and §7.9.4's dates.
python3 "$here/seed_forms_data.py" "$root/forms_data"

# ISO 19444-1's files: every XFDF file the tests hold, one annotation subtype each.
mkdir -p "$root/xfdf"
cp -n "$tree"/crates/pdf-model/tests/xfdf/*.xfdf "$root/xfdf/"
echo "seeds.sh: $(ls "$root/xfdf" | wc -l) xfdf seeds in $root/xfdf"

# Annex F: whole documents, the recipe `serialize` has, since both write a file out of one.
find -L "$tree/doc/corpora" "$tree/doc/pdf.js/test/pdfs" -name '*.pdf' -print0 \
    | xargs -0 python3 "$here/seed_page.py" "$root/linearize" > /dev/null
echo "seeds.sh: $(ls "$root/linearize" | wc -l) linearize seeds in $root/linearize"

# §7.4.7 and §7.4.9: every codestream the documents hold, in the framing each target reads.
(cd "$tree" && cargo build -q --release -p pdf-model --example image_codec_seeds)
seeder=$(cd "$tree" && cargo metadata --format-version 1 --no-deps \
    | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')/release/examples/image_codec_seeds
find -L "$tree/doc/corpora" "$tree/doc/pdf.js/test/pdfs" -name '*.pdf' -print0 \
    | xargs -0 "$seeder" "$root/jbig2" "$root/jpx"
