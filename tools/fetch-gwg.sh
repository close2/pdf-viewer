#!/usr/bin/env bash
# Fetches the Ghent PDF Output Suite 5.0 into doc/corpora/gwg-output-suite/, pinned by hash.
#
# The other test corpora are git submodules under doc/corpora/. This one has no repository, and
# its licence permits use "for its intended purpose of testing workflow setup" while forbidding
# sale and commercial use without the Ghent Workgroup's written permission, so no byte of it is
# tracked here: the directory is git-ignored, and this script is the submodule's equivalent — two
# URLs, two SHA-256 hashes, and a refusal when either differs (`doc/gwg-output-suite-provenance.md`,
# ADR 2728).
#
#   tools/fetch-gwg.sh            # fetch (or reuse the download cache), verify, unpack
#   PDF_VIEWER_GWG=<dir> tools/fetch-gwg.sh   # unpack somewhere else
#
# The zips are kept in `$PDF_VIEWER_GWG_CACHE` (default `$PDF_VIEWER_SCRATCH/gwg`) and downloaded
# only where a verified copy is not already there. The suite's own legal notice travels with the
# copy: it is printed in `Ghent_PDF-Output-Suite-V50_ReadMeFirst.pdf`, which the unpacked tree
# holds, and `NOTICE.txt` beside it says where.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=tools/places.sh
. "$here/places.sh"
: "${PDF_VIEWER_GWG_CACHE:=$PDF_VIEWER_SCRATCH/gwg}"

base=https://gwg.org/wp-content/uploads/2026/05
# name in the cache | name on the server | SHA-256
pins=(
    "testpages.zip Ghent_PDF_Output_Suite_V50_Testpages.zip 52d271ddd97dcb5778d1da2bdb3b86a66b96083b7f57ee3fd8ed2b5e5a5d7d29"
    "patches.zip Ghent_PDF_Output_Suite_V50_Patches-2.zip 109625cc2f77c3fec703edd19e28b3676323526517c9f65dc9e45b752fa2fb47"
)

mkdir -p "$PDF_VIEWER_GWG_CACHE"
# A worktree's doc/corpora/ entries are links into the main checkout's; the suite is unpacked
# where the link points, so every tree reads one copy.
if [ -L "$PDF_VIEWER_GWG" ]; then PDF_VIEWER_GWG=$(readlink -f "$PDF_VIEWER_GWG"); fi
mkdir -p "$(dirname "$PDF_VIEWER_GWG")"
staging=$(mktemp -d "${PDF_VIEWER_GWG}.partial.XXXXXX")
trap 'rm -rf "$staging"' EXIT

for pin in "${pins[@]}"; do
    read -r cached served want <<<"$pin"
    zip=$PDF_VIEWER_GWG_CACHE/$cached
    if [ ! -f "$zip" ] || [ "$(sha256sum "$zip" | cut -d' ' -f1)" != "$want" ]; then
        printf 'fetch-gwg: downloading %s\n' "$served" >&2
        curl --fail --location --silent --show-error -o "$zip.download" "$base/$served"
        mv "$zip.download" "$zip"
    fi
    got=$(sha256sum "$zip" | cut -d' ' -f1)
    if [ "$got" != "$want" ]; then
        printf 'fetch-gwg: %s has SHA-256 %s, and the pin is %s; refusing to unpack it\n' \
            "$served" "$got" "$want" >&2
        exit 1
    fi
    unzip -q -o "$zip" -x '__MACOSX/*' '*/.DS_Store' -d "$staging"
done

cat >"$staging/NOTICE.txt" <<'EOF'
Ghent PDF Output Suite 5.0, (c) 2016 Ghent Workgroup (https://gwg.org/).

The suite's legal notice is printed in
Ghent_PDF_Output_Suite_V50_Patches/Ghent_PDF-Output-Suite-V50_ReadMeFirst.pdf, and it travels
with this copy. In short: the suite may be used for its intended purpose of testing workflow
setup; it may not be sold or used commercially without the Ghent Workgroup's written permission.

This directory was written by tools/fetch-gwg.sh from the two zips it pins by SHA-256. It is
git-ignored, and nothing in it may be committed to the pdf-viewer repository
(doc/gwg-output-suite-provenance.md).
EOF

rm -rf "$PDF_VIEWER_GWG"
mv "$staging" "$PDF_VIEWER_GWG"
trap - EXIT
printf 'fetch-gwg: %s holds %s PDF files\n' "$PDF_VIEWER_GWG" \
    "$(find "$PDF_VIEWER_GWG" -name '*.pdf' | wc -l)"
