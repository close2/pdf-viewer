#!/usr/bin/env bash
#
# What the numbers are today.
#
# This script exists so that no instruction document has to carry a count. A fact that can
# be counted is not written down; what is written down is the command that counts it, and
# this is that command. Every figure it prints is a *gate's own output*, filtered — nothing
# here performs arithmetic on a gate's numbers, because arithmetic beside a gate's figure is
# exactly the thing that goes stale while the figure beside it is current.
#
# It is a shell script rather than a Rust binary on purpose: its whole job is to run other
# programs and show what they said, so a build step in front of it would put a compile
# between a question and its answer, and its source would stop being a readable list of the
# commands the documents used to state in prose. ADR 0281.
#
#   tools/state.sh                 # every section, in cost order (minutes)
#   tools/state.sh quick           # only the sections that need no corpus run (seconds)
#   tools/state.sh ledger oracle   # named sections, in the order given
#   tools/state.sh --list          # the section names
#
# Exit status is the worst of the commands it ran, so a round may trust a zero.

set -u -o pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root" || exit 1

status=0
built_gate_binaries=

# The two programs a gate spawns and Cargo will not build for it (HANDOVER trap 10). Built
# once, on the first section that needs one, so that `quick` never pays for them.
gate_binaries() {
    [ -n "$built_gate_binaries" ] && return 0
    built_gate_binaries=yes
    cargo build --profile gates -p pdf-sandbox --bins >/dev/null 2>&1 || status=1
    cargo build --profile gates -p hayro-compare --bin pdfref-hayro >/dev/null 2>&1 || status=1
}

heading() { printf '\n== %s ==\n%s\n\n' "$1" "$2"; }

# Runs a command, remembers a failure, and prints the lines matching an extended regexp.
# The regexp is the only editorialising this script does: it chooses which of the gate's own
# lines are the summary, and it never rewrites one.
run() {
    local title=$1 filter=$2
    shift 2
    heading "$title" "$*"
    local output
    output=$("$@" 2>&1)
    local code=$?
    [ $code -ne 0 ] && status=$code
    printf '%s\n' "$output" | grep -E "$filter" || {
        printf 'no line matched %s — the gate said:\n' "$filter"
        printf '%s\n' "$output" | tail -20
        status=1
    }
    return 0
}

section_ledger() {
    run "ledger" '.' cargo run -q -p conformance --bin ledger
}

section_conformance() {
    run "conformance (citations, quotations, tables, ledger rows)" \
        '^[0-9]+ (citations|quotations)|owe a review|^conformance ledger|^  (implemented|partial|reported|silent|inapplicable|writer-side|out-of-scope) |name .* distinct tables|name a test file' \
        cargo test -p conformance -- --nocapture
}

section_tests() {
    gate_binaries
    run "tests" 'Summary|tests run|test result' cargo nextest run --workspace
    # Only the crate that has one; two dozen "0 passed" lines are not a summary.
    run "doctests" 'test result: ok\. [1-9]' cargo test --workspace --doc
}

section_corpus() {
    gate_binaries
    run "corpus (974 pdf.js documents, page one)" \
        '^[0-9]+ documents in|^  codes ' \
        cargo test --profile gates -p pdf-model --test corpus -- --ignored --nocapture
}

section_oracle() {
    gate_binaries
    run "oracle (poppler, mupdf, ghostscript)" \
        '^[0-9]+ pages in|^  (agrees|contradicted|ambiguous|our geometry|reference geometry|not comparable|no render) |undiagnosed' \
        cargo test --profile gates -p pdf-model --test oracle -- --ignored --nocapture
}

section_text() {
    gate_binaries
    run "text (pdftotext, and PDFBox's frozen extraction)" \
        '^[0-9]+ documents in' \
        cargo test --profile gates -p pdf-model --test text_extraction -- --ignored --nocapture
}

section_quorra() {
    gate_binaries
    run "quorra against the CPU oracle" \
        '^[0-9]+ pages compared|^  (rasterisation|median page)' \
        cargo test --profile gates -p render-quorra --test corpus -- --ignored --nocapture
}

section_dates() {
    run "dates (§7.9.4)" '^[0-9]+ date strings' \
        cargo test --profile gates -p pdf-model --test dates -- --ignored --nocapture
}

section_xmp() {
    run "XMP (§14.3.2)" "^[0-9]+ documents carry" \
        cargo test --profile gates -p pdf-model --test xmp -- --ignored --nocapture
}

section_jpeg2000() {
    run "JPEG 2000 against ISO/IEC 15444-5's reference software" \
        '^[0-9]+ (codestreams|differing|not comparable)' \
        cargo test --profile gates -p pdf-model --test jpeg2000 -- --nocapture
}

# Annex O's parameters, which `CLAUDE.md` used to state as "N of M carried out" and which
# went stale. `Parameter::unhonoured` is the program's own answer: the variants that reach
# `return None` are carried out, and each arm after it names a parameter and its reason.
section_annex_o() {
    local source=crates/pdf-model/src/fragment.rs
    heading "Annex O's fragment parameters" \
        "sed -n '/pub fn unhonoured/,/^    }\$/p' $source"
    local body
    body=$(sed -n '/pub fn unhonoured/,/^    }$/p' "$source")
    printf 'carried out: %s\n' \
        "$(printf '%s\n' "$body" | sed -n '1,/return None/p' | grep -oE 'Self::[A-Za-z]+' | sed 's/Self:://' | paste -sd' ')"
    printf 'reported:    %s\n' \
        "$(printf '%s\n' "$body" | sed -n '/return None/,$p' | tail -n +2 | grep -oE 'Self::[A-Za-z]+' | sed 's/Self:://' | paste -sd' ')"
    printf '\nwhy each of the reported ones is reported — the arms, verbatim:\n'
    printf '%s\n' "$body" | sed -n '/return None/,$p' | tail -n +2 | head -n -2 | sed 's/^        //'
}

# The other populations a document used to state. Each is a `find` or a `ls`, which is the
# whole point: the answer is on the disk rather than in a sentence about the disk.
section_counts() {
    heading "populations on disk" "find / ls"
    printf 'fuzz targets:        %s\n' "$(ls fuzz/fuzz_targets/*.rs 2>/dev/null | wc -l)"
    printf 'ADRs:                %s\n' "$(ls doc/adr/*.md 2>/dev/null | wc -l)"
    printf 'open todo items:     %s\n' "$(ls doc/todo/[0-9]*.md 2>/dev/null | wc -l)"
    printf 'specification docs:  %s pdf, %s markdown\n' \
        "$(ls doc/*.pdf 2>/dev/null | wc -l)" "$(ls doc/md/*.md 2>/dev/null | wc -l)"
    printf 'pdf.js corpus:       %s\n' "$(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf' 2>/dev/null | wc -l)"
    printf 'doc/corpora:         %s\n' "$(find doc/corpora -name '*.pdf' 2>/dev/null | wc -l)"
    printf 'SafeDocs:            never in this tree — `target/safedocs list --dir <path>` counts\n'
    printf '                     whatever has been fetched, and `survey --dir <path>` re-baselines it\n'
}

# What a person can actually run, and how old it is. `doc/todo/02` §5 is what refreshes it.
section_binaries() {
    heading "binaries a person can run" "ls -l target/"
    ls -l target/ 2>/dev/null | grep -vE '^total|^d' || printf 'nothing installed — doc/todo/02 §5\n'
}

section_disk() {
    heading "the build directory" "du -sh"
    du -sh /home/AI/cargo-target/pdf-viewer 2>/dev/null
    du -sh /home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache 2>/dev/null
}

all="ledger conformance annex-o counts binaries disk tests corpus oracle text quorra dates xmp jpeg2000"
quick="ledger conformance annex-o counts binaries disk"

case ${1-} in
--list) printf '%s\n' $all; exit 0 ;;
esac

case ${1-all} in
all) sections=$all ;;
quick) sections=$quick ;;
*) sections=$* ;;
esac

for section in $sections; do
    case $section in
    ledger) section_ledger ;;
    conformance) section_conformance ;;
    tests) section_tests ;;
    corpus) section_corpus ;;
    oracle) section_oracle ;;
    text) section_text ;;
    quorra) section_quorra ;;
    dates) section_dates ;;
    xmp) section_xmp ;;
    jpeg2000) section_jpeg2000 ;;
    annex-o) section_annex_o ;;
    counts) section_counts ;;
    binaries) section_binaries ;;
    disk) section_disk ;;
    *)
        printf 'no such section: %s (tools/state.sh --list)\n' "$section" >&2
        status=1
        ;;
    esac
done

exit $status
