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
    # A gate that fails is named where it failed, not only in the script's exit status. Until
    # the nine-hundred-and-ninety-eighth session a non-zero exit set `status` and printed
    # nothing, so a reader of the printed state could see every gate's summary line and not the
    # one that had failed — the merge of rounds 992–997 got `exit 101` from a forty-seven-section
    # run and no line saying where.
    [ $code -ne 0 ] && { status=$code; printf '  ✗ the gate exited %s\n' "$code"; }
    printf '%s\n' "$output" | grep -E "$filter" || {
        printf 'no line matched %s — the gate said:\n' "$filter"
        printf '%s\n' "$output" | tail -20
        status=1
    }
    return 0
}

section_ledger() {
    # Every status the ledger has, `departed` among them and counted as itself: the word the owner
    # added in answer to doc/questions/Q63 says a requirement was decided against with its cost
    # recorded, and folding it into `implemented` would hide the sentence while folding it into
    # `partial` would go on counting a decision as debt. ADR 1119.
    run "ledger" '.' cargo run -q -p conformance --bin ledger
}

# The reading list behind every `departed` row. The status says a requirement was decided against
# with its cost recorded in an ADR, and the gate can check that the ADR exists but not that its
# premise still holds — so this prints, per row, the argument its first sentence names and how many
# later ADRs cite that argument or the clause. Whether a premise has expired is a person's to read
# (ADR 1166); the whole reading list is one command away, `--bin departures`.
section_departures() {
    run "the argument behind every departed row" \
        '^[0-9]+ departed row|^§|^  (nothing later cites|since, to re-read)' \
        cargo run -q -p conformance --bin departures
}

# Every mention of a command-line flag, against the flags the program named actually accepts.
# Both populations are derived — the binaries from the workspace's manifests, the accepted set
# from each binary's own source — so a program added or a flag renamed is counted without this
# script being edited (ADR 1213). The filter keeps the run's denominators, the per-binary line and
# every finding; `cargo test -p conformance --test flags` is the gate, and its second test is the
# calibration.
section_flags() {
    run "flags a message names against the flags the program accepts" \
        '^[0-9]+ binary\(ies\)|mention\(s\) of a flag|^  [a-z]' \
        cargo run -q -p conformance --bin flags
}

# Every Rust path a doc comment names, against the items this workspace declares. A `§` is checked
# against the standard and a file path against the tree; a path in prose — `Interpreter::run`,
# `crate::edit::apply` — was read by nothing, and rustdoc resolves only the linked form and only
# under `cargo doc`, which no tier runs (ADR 1273). The filter keeps the run's denominators, each
# finding and the count on every rung; `cargo test -p conformance --test names` is the gate, and
# its second test is the calibration.
section_names() {
    run "a Rust path a doc comment names against the items this tree declares" \
        'path\(s\) in |name\(s\) this workspace declares|whose prefix this tree declares|^  [a-z]|^ *[0-9]+  ' \
        cargo run -q --release -p conformance --bin names
}

# Every clause a source file cites, against the `code` list of the row for that clause. A `code`
# list is the ledger's index into the tree and it decays in one direction only: a round adding a
# reader cites the clause beside the code, because principle 5 requires it, and editing a row in
# another file is the step it forgets (ADR 1274). It ranks rather than fails — a citation may point
# at a neighbour rather than implement anything — so the filter keeps the denominators and the top
# rung; `cargo test -p conformance --test cited` gates the calibration alone. A checker citing the
# clause it checks has a rung of its own, printed as a count per named crate (`cited::CHECKERS`).
# The no-row pairs under `raster/` have a line and a listing of their own (`cited::RASTER`): a `§`
# there that meant one of the library's own documents is written "section N", and the listing is
# what is left of that.
section_cited() {
    run "a clause a file cites against that clause's own code list" \
        'pair\(s\) over |a row claiming work|a checker cites|^  crates/|pair\(s\) the row already names|no-row pair\(s\) lie under|^  §' \
        cargo run -q --release -p conformance --bin cited
}

# Whether the four navigational documents — `doc/PLAN.md`, `doc/crate-map.md`,
# `doc/state-of-play.md`, `doc/HANDOVER.md` — are still true of the tree, asked of the three sweeps
# that count their own population and of the history `CLAUDE.md`'s comment rule keeps out of them.
# `pointers` prints its counts and every absent pointer those four documents hold; `overtaken` and
# `unread` print the counts over the page-list notes and the ledger, which the four documents point
# into. `retired` is not run here, because its nouns are the caller's — the string a correction
# retired — and a list of them written into this script would be the stale copy it exists to find.
# A hit is a reading list and not a verdict: each sweep's own last line says what its noise is.
section_navigation() {
    run "the four navigational documents' pointers into the tree" \
        '^[0-9]+ path pointer|— doc/(PLAN|crate-map|state-of-play|HANDOVER)\.md:[0-9]+$' \
        cargo run -q --release -p conformance --bin pointers
    run "page-list notes a later decision overtook" \
        '^[0-9]+ page-list note' \
        cargo run -q --release -p conformance --bin overtaken
    run "ledger entries claimed unread that the tree quotes" \
        '^[0-9]+ rows claim' \
        cargo run -q --release -p conformance --bin unread
    # `grep -c` exits 1 when no file matches, which is the clean answer, so only 2 is a failure.
    run "history the comment rule keeps out of the four documents" \
        ':[0-9]+$' \
        bash -c 'grep -cE "hundred-and-|one round later|round [0-9]{3,4}|session [0-9]{3,4}|used to (reach|read|be|draw|name|hold|refuse)" "$@"; [ $? -le 1 ]' \
        history doc/PLAN.md doc/crate-map.md doc/state-of-play.md doc/HANDOVER.md
}

# Every ledger note's opening and closing sentence, against the row's own `status` field. A note's
# last sentence is the "what keeps this row `partial`" clause and every later round appends above
# it; its first sentence is what a round that moves a status rewrites around. Both are inside the
# row, so no other sweep compares them with the field beside them (ADR 1249). It prints a reading
# list rather than failing: whether a sentence is about this row is a question about English.
section_last_sentences() {
    run "a note's opening and closing sentence against its own status" \
        'row\(s\) carry a note|^doc/conformance/ledger.toml' \
        cargo run -q -p conformance --bin last_sentences
}

# The frontier map's membership against the ledger's own open rows. `doc/todo/65` states its
# population in its opening lines — the `partial` and `reported` rows and only those — and both
# sides are lists, so this is the one comparison in this family that fails rather than prints
# (ADR 1250). `conformance` runs the same test; this is the line that shows its summary alone.
section_frontier() {
    run "the frontier map against the ledger's open rows" \
        'open row\(s\), each placed once|places §|is §' \
        cargo test -q -p conformance --test conformance -- --nocapture the_frontier_map
}

section_conformance() {
    run "conformance (citations, quotations, tables, ledger rows)" \
        '^[0-9]+ (citations|quotations)|naming a section of one of this|instruction documents, every one|owe a review|^conformance ledger|^  (implemented|partial|departed|reported|silent|inapplicable|writer-side|out-of-scope) |unsettled rows owe a debt|name .* distinct tables|name a test file' \
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

section_golden() {
    gate_binaries
    run "our own output held by name — the raster golden over the tracked corpus, a change detector and not a verdict (ADR 1016)" \
        '^[0-9]+ tracked documents on disk|^held [0-9]+|^  (moved|unheld|left):' \
        cargo test --profile gates -p pdf-model --test raster_golden -- --ignored --nocapture
}

section_oracle() {
    gate_binaries
    run "oracle (poppler, mupdf, ghostscript)" \
        '^[0-9]+ pages in|^  (agrees|contradicted|ambiguous|our geometry|reference geometry|not comparable|no render) |undiagnosed' \
        cargo test --profile gates -p pdf-model --test oracle -- --ignored --nocapture
}

section_text() {
    gate_binaries
    # Three gates in one binary since ADR 0333: two about which characters a page reads back as,
    # and one about *where* its words are — whose verdict and judged set are ratcheted since ADR
    # 0424 and are therefore two lines worth keeping.
    run "text (pdftotext, PDFBox's frozen extraction, and where the words are)" \
        '^[0-9]+ documents in|^[0-9]+ of [0-9]+ documents judged|^verdict:' \
        cargo test --profile gates -p pdf-model --test text_extraction -- --ignored --nocapture
}

# ADR 0323's instrument 1, composed half: the loop from a press to a selection, which the line
# above cannot see — it judges where the text layer *says* the words are and this drags across
# poppler's boxes in device pixels. The filter keeps the three properties and the refusal classes.
section_selection() {
    run "the selection loop (a drag across poppler's word boxes)" \
        '^[0-9]+ documents in|^the (drag|readback|caret)|^[0-9]+ of [0-9]+ documents refused|^ +[0-9]+ +[a-z/]|^  [a-z]' \
        cargo test --profile gates -p viewer-core --test selection_census -- --ignored --nocapture
}

# ADR 0323's third instrument, and the only one of the three with no reference to disagree with
# it: nobody else puts a comparable tree on AT-SPI. So it is a ratchet, and the filter keeps its
# counts and the *classes* of silence rather than the per-page witnesses, which the run prints
# under each class for whoever is reading it rather than watching it.
section_accessibility() {
    run "the accessibility tree (§14.7–§14.9, a ratchet: no reference to disagree with)" \
        '^[0-9]+ documents in|^(structure|pages that answer|elements reached|untagged pages)|^  [a-z§]' \
        cargo test --profile gates -p viewer-core --test accessibility_census -- --ignored --nocapture
}

# The adapter crate is `render-raster`, and this said `render-quorra` until session 945 — a name
# the rename to `raster` left behind (`doc/questions/A05`: the rendering library is named for what
# it is, and `render-raster` is this tree's adapter for it). `cargo` answered "package ID
# specification `render-quorra` did not match any packages", `run` reported that no line matched
# the pattern, and the sequence carried on: the gate was not failing, it was not running.
# `doc/todo/02` §2 has carried the correct name throughout, which is how two statements of one
# command drift when only one of them is executed.
section_quorra() {
    gate_binaries
    run "raster against the CPU oracle" \
        '^[0-9]+ pages compared|^  (rasterisation|median page)' \
        cargo test --profile gates -p render-raster --test corpus -- --ignored --nocapture
}

section_fixed() {
    gate_binaries
    run "documents a round fixed outside the gates (doc/checks/fixed-documents.toml)" \
        '^fixed-documents:|no longer do what' \
        cargo test --profile gates -p pdf-model --test fixed_documents -- --ignored --nocapture
}

section_transform() {
    gate_binaries
    run "the transform suite (RFC 0002 section 12's floor, and inventories held to the document)" \
        '^transform:' \
        cargo test --profile gates -p pdf-transform --test gate -- --ignored --nocapture
}

# RFC 0002 section 9's walk for the writer: every corpus document the suite opens, a file
# attached, read back and removed, the input's bytes under every update. The filter keeps the
# counts and the census lists' headings; the per-document lines under each are for a reader.
section_writer() {
    gate_binaries
    run "the transform writer over the corpus (attach, read back, remove)" \
        '^transform-writer:' \
        cargo test --profile gates -p pdf-transform --test writer_corpus -- --ignored --nocapture
    run "split over the corpus (RFC 0002 section 9's layers 2 and 3, first page)" \
        '^transform-split:' \
        cargo test --profile gates -p pdf-transform --test split_corpus -- --ignored --nocapture
    run "merge over the corpus (RFC 0002 section 9's layers 2 and 3, plus each reconciliation)" \
        '^transform-merge:' \
        cargo test --profile gates -p pdf-transform --test merge_corpus -- --ignored --nocapture
    run "pages over the corpus (a quarter turn and a page out, RFC 0002 section 9's layers 2 and 3)" \
        '^transform-pages:' \
        cargo test --profile gates -p pdf-transform --test pages_corpus -- --ignored --nocapture
    run "optimize over the corpus (RFC 0002 section 9's layers 2 and 3, and its idempotence gate)" \
        '^transform-optimize:' \
        cargo test --profile gates -p pdf-transform --test optimize_corpus -- --ignored --nocapture
    # RFC 0002 section 9's fourth layer, and the only gate here that asks somebody else: the five
    # writers' output read by poppler, mupdf and qpdf, each foreign reading compared with that
    # same reader's reading of the source. It prints a skip line under its own prefix where the
    # readers are not installed, so this line stays green on a machine without them.
    run "the five writers' output read by poppler, mupdf and qpdf (RFC 0002 section 9's foreign readback)" \
        '^transform-foreign:' \
        cargo test --profile gates -p pdf-transform --test foreign_corpus -- --ignored --nocapture
}

# `CLAUDE.md` principle 2's four numbers, plus the fifth it makes a gate of its own — and the
# three figures on the launch path that have no clock in them at all.
#
# **`--release` rather than `--profile gates`, and this is the only section that says so.** The
# two profiles differ by 4.06% to 12.30% on `Document::open` (`Cargo.toml`'s own table, ADR
# 0666), which is wider than the bands this gate holds; a launch figure is a claim about the
# program a person runs, so it is taken under the profile that produces one. The worker beside it
# has to be the release build for the same reason and for trap 10's.
#
# The filter keeps every `launch-path:` line, which is the whole report: it is four documents deep
# and a reader wants the table rather than a total. `NOT JUDGED` is one of those lines — see ADR
# 0884 for why a wall-clock gate on this machine says that rather than failing.
section_launch() {
    cargo build --release -p pdf-sandbox --bins >/dev/null 2>&1 || status=1
    run "the launch path (principle 2's four numbers, doc/checks/launch-path.toml)" \
        '^launch-path:' \
        cargo test --release -p viewer-ui --test launch_path -- --ignored --nocapture
}

# Principle 2's fifth number, which is the one a person feels after the launch: what a *frame*
# costs, stage by stage, against the 8.333 ms `doc/todo/36` asks for. `section_launch` above times
# a page turn end to end; this says where the time inside one goes — interpretation, this crate's
# scene walk, raster's encode, the transfer, the device's own passes — for a text page, a page of
# vector artwork and a page that is one photograph, at 1x and at 2x.
#
# **It needs the graphics device and it takes about a minute**, which is why it is not in `quick`:
# every row is the minimum of three rounds on a device of its own. On a machine with no adapter it
# fails loudly rather than skipping, exactly as the corpus comparison beside it does.
section_frame() {
    run "what a frame costs, stage by stage (doc/todo/36's budget)" \
        '^frame budget|minima of|^the budget is|page [0-9]+ —|^  (turn|warm|step) |of one refresh' \
        cargo run --release -q -p render-raster --example frame_budget
}

# RFC 0003 section 5.2's five write verbs and section 4's whole layout, over every corpus document
# the core opens. The `--bins` build is trap 10: a `--profile gates --test` line builds one test
# target and nothing else, so `pdf-vfs-worker` beside it would otherwise be whatever an earlier
# round left. There is no third line here: session 917's `awkward_classes` became the read walk's
# population in session 919 (ADR 0878), and the walk of the *other* confined program that inherited
# the name is `section_confined` below, `doc/todo/02` §2's since session 995.
section_vfs() {
    gate_binaries
    cargo build --profile gates -p pdf-vfs --bins >/dev/null 2>&1 || status=1
    run "the five write verbs over the corpus, through the core (RFC 0003 section 5.2)" \
        '^vfs-write:' \
        cargo test --profile gates -p pdf-vfs --test write_corpus -- --ignored --nocapture
    run "the whole layout listed, stat'd and read over the corpus (RFC 0003 section 4)" \
        '^vfs-read:' \
        cargo test --profile gates -p pdf-vfs --test read_corpus -- --ignored --nocapture
}

# The other confined program — `pdf-view-worker`, the process a person reads pages in — over a
# document of each awkward class from every corpus on the disk (ADR 0879). `doc/verify.md`'s run
# until session 995; `doc/todo/02` §2's since (ADR 1015). What fails it is a death, and the filter
# keeps the per-root and per-class counts and the `killed:` line; the reasons listed under them
# are for a reader. The `--bins` build is trap 10 for this crate's own worker, and the walk runs
# under `tools/bounded.sh` because it is the heaviest of the session-995 lines by memory.
section_confined() {
    cargo build --profile gates -p viewer-confined --bins >/dev/null 2>&1 || status=1
    run "the confined viewer over every awkward class on the disk (what fails it is a death)" \
        '^view-awkward:   [a-z]|^view-awkward: killed|^bounded:' \
        tools/bounded.sh -- cargo test --profile gates -p viewer-confined --test awkward_classes -- --ignored --nocapture
}

# The mitigation catalogue's own gap, which `doc/todo/66` names as an instrument gap rather than
# leaving as a number in prose. The item's done condition has two halves — no site left saying its
# catalogued remedy is not built, and no shipped profile left producing a `does not carry out yet`
# note — and `--remedy-sites` now prints a total for each of them, so this section filters two
# sentences the program wrote rather than counting its lines. The header's rule is kept exactly:
# nothing here adds anything up.
#
# **The profile half needs no corpus, and that is the finding rather than a shortcut.** The note it
# counts comes from `Configuration::unbuilt`, which reads the answers a profile gives and asks
# which of them have code behind them; both are properties of the profile and the target, so a
# conversion prints the same notes whatever document it is handed. Walking a corpus for them would
# have counted the corpus. The corpus half of `doc/todo/66` is `section_archive` below, which keeps
# the walk's per-target conversion counts — a different question, and that one does need the walk.
#
# Cheap — a table lookup per target and per profile, no corpus and no document — so it is in
# `quick`, and `section_archive` calls it so that a run of that section alone prints the whole item.
section_remedies() {
    heading "the mitigation catalogue's gap, per PDF/A target" \
        "quorra-transform archive --remedy-sites --to <target> [--config <profile>]"
    # The targets come from the program rather than from a list here, so that a target added to
    # `pdf_archive::Target::ALL` is counted without this script being edited: `--to ""` is refused
    # with a sentence naming every one of them.
    local target listing code trailer targets profile answered name absent
    targets=$(cargo run -q -p pdf-transform --bin quorra-transform -- \
        archive --remedy-sites --to '' 2>&1 |
        sed -n 's/^error: --to .*: the targets are //p' | tr -d ',')
    if [ -z "$targets" ]; then
        printf '  \xe2\x9c\x97 --remedy-sites names no targets; the sentence it is read from has moved\n'
        status=1
        return 0
    fi
    for target in $targets; do
        listing=$(cargo run -q -p pdf-transform --bin quorra-transform -- \
            archive --remedy-sites --to "$target" 2>&1)
        code=$?
        if [ $code -ne 0 ]; then
            printf '  \xe2\x9c\x97 --remedy-sites --to %s exited %s\n' "$target" "$code"
            printf '%s\n' "$listing" | tail -5
            status=$code
            continue
        fi
        trailer=$(printf '%s\n' "$listing" |
            sed -n 's/^\([0-9]* of [0-9]*\) sites not built yet\..*/\1/p')
        if [ -z "$trailer" ]; then
            printf '  \xe2\x9c\x97 --remedy-sites --to %s printed no total; its trailer has moved\n' "$target"
            status=1
            continue
        fi
        printf '  %-3s %s sites with the catalogued remedy not built yet\n' "$target" "$trailer"
    done
    # The profiles come from the directory rather than from a list here, for the same reason the
    # targets come from the program: a profile added to `doc/profiles/` is asked about without this
    # script being edited. A profile a target refuses to read is data, not a broken instrument —
    # the remedy words a target admits are its own — so it is printed and does not fail the run.
    heading "what a shipped profile answers that this version does not carry out" \
        "quorra-transform archive --remedy-sites --to <target> --config doc/profiles/<profile>.toml"
    for profile in doc/profiles/*.toml; do
        name=$(basename "$profile" .toml)
        for target in $targets; do
            listing=$(cargo run -q -p pdf-transform --bin quorra-transform -- \
                archive --remedy-sites --to "$target" --config "$profile" 2>&1)
            answered=$(printf '%s\n' "$listing" |
                sed -n 's/^\([0-9]* of [0-9]*\) sites answered with a remedy not carried out yet.*/\1/p')
            if [ -z "$answered" ]; then
                printf '  %-22s %-3s refused: %s\n' "$name" "$target" \
                    "$(printf '%s\n' "$listing" | sed -n 's/^error: //p' | head -1)"
                continue
            fi
            # **ADR 1199's wrinkle, made visible.** A site whose answer is conditional on data the
            # caller supplies can drop off the target's own listing while a profile still answers
            # it with a remedy this version does not carry out — so the two halves of this section
            # can disagree, and the disagreement is exactly what the catalogue's gap is made of.
            # Counted in one pass over the same output, because the `--config` run prints the
            # listing above the answers; a non-zero count is a finding to read, not a broken
            # instrument, and `--remedy-sites --to <target> --config <profile>` names each.
            absent=$(printf '%s\n' "$listing" | awk '
                /^  [a-z][a-z0-9\/-]* \(/ { listed[$1] = 1; next }
                /^  "/ {
                    site = $0
                    sub(/^  "/, "", site)
                    sub(/".*/, "", site)
                    if (!(site in listed)) { missing++ }
                }
                END { print missing + 0 }')
            printf '  %-22s %-3s %s answers not carried out yet, %s at a site the listing does not name\n' \
                "$name" "$target" "$answered" "$absent"
        done
    done
}

# ISO 19005's two readings, clause by clause: the validator against every witness the veraPDF
# corpus holds, per target (`crates/pdf-archive/tests/corpus.rs`), and the converter over the same
# corpus held to that validator run twice (`crates/pdf-transform/tests/archive_corpus.rs`). The
# first filter keeps each target's heading, the column names and its `all` row — `over` is the
# column that matters, and every one of it is a question for doc/pdfa/ before it is a bug; the
# per-clause rows under each heading are for a reader. The second keeps the converter's own
# summary line per target and what a user who answers nothing gets. Both say so, loudly, without
# `doc/veraPDF-corpus`, and the filters keep that line too. `doc/state-of-play.md` said this
# script printed the comparison for some sessions before it did (ADR 1015).
section_archive() {
    section_remedies
    run "the validator against the veraPDF corpus (ISO 19005, clause by clause per target)" \
        '^== PDF_A|^  clause |^  all |not here|^bounded:' \
        tools/bounded.sh -- cargo test --profile gates -p pdf-archive --test corpus -- --ignored --nocapture
    run "the converter over the veraPDF corpus (conforms in, conforms out, no glyph moves)" \
        '^archive |^    answering nothing|not here|^bounded:' \
        tools/bounded.sh -- cargo test --profile gates -p pdf-transform --test archive_corpus -- --ignored --nocapture
    run "the survey's resource selections held to the interpreter's (A61, ADR 1055)" \
        '^cross-check: |not here|^bounded:' \
        tools/bounded.sh -- cargo test --profile gates -p pdf-archive --test cross_check -- --ignored --nocapture
}

section_dates() {
    run "dates (§7.9.4)" '^[0-9]+ date strings' \
        cargo test --profile gates -p pdf-model --test dates -- --ignored --nocapture
}

section_xmp() {
    run "XMP (§14.3.2)" "^[0-9]+ documents carry" \
        cargo test --profile gates -p pdf-model --test xmp -- --ignored --nocapture
}

# §7.5.6's incremental update over the corpus, read back by this tree and by poppler and mupdf
# (ADR 0334); its counts ratchet since ADR 1011. Twelve seconds, and a corpus walk all the same.
section_save() {
    run "save round-trip (§7.5.6)" \
        '^[0-9]+ documents in|^Restrict\((On|Off)\)|^  (prefix failed|readback failed|reference disagreed|reference would not answer|panicked)|ratchet' \
        tools/bounded.sh -- cargo test --profile gates -p pdf-model --test save_round_trip -- --ignored --nocapture
}

# §12.6.3's page-scoped triggers counted over the corpus and held in both directions, and
# §12.6.4's embedded go-to opened inside the document that carries it. An object-graph walk
# that costs a second, `#[ignore]`d only for the submodule it needs (ADR 1015).
section_actions() {
    run "actions (§12.6.3's triggers over the corpus, held both ways; §12.6.4's embedded go-to)" \
        '^/[A-Z]+: [0-9]+ in|skipping|^bounded:' \
        tools/bounded.sh -- cargo test --profile gates -p pdf-model --test actions -- --ignored --nocapture
}

# ADR 0809's argument, run: every corpus document opened from disk and from memory and every
# object the cross-reference table names compared. Two seconds, and it fails on one object.
section_on_disk() {
    run "on disk against in memory (ADR 0809: every corpus object read both ways)" \
        'documents agree on|nothing walked|^bounded:' \
        tools/bounded.sh -- cargo test --profile gates -p pdf-syntax --test on_disk -- --ignored --nocapture
}

section_jpeg2000() {
    run "JPEG 2000 against ISO/IEC 15444-5's reference software" \
        '^[0-9]+ (codestreams|differing|not comparable)' \
        cargo test --profile gates -p pdf-model --test jpeg2000 -- --nocapture
}

# Annex O's parameters. `Parameter::unhonoured` is the program's own answer rather than a count in
# a document: the variants that reach `return None` are carried out, and each arm after it names a
# parameter and the reason it is only reported (ADR 0281).
section_annex_o() {
    local source=crates/pdf-model/src/fragment.rs
    heading "Annex O's fragment parameters" \
        "sed -n '/pub fn unhonoured/,/^    }\$/p' $source"
    local body
    body=$(sed -n '/pub fn unhonoured/,/^    }$/p' "$source")
    printf 'carried out: %s\n' \
        "$(printf '%s\n' "$body" | sed -n '1,/return None/p' | grep -oE 'Self::[A-Za-z]+' | sed 's/Self:://' | paste -sd' ')"
    local reported
    reported=$(printf '%s\n' "$body" | sed -n '/return None/,$p' | tail -n +2 | grep -oE 'Self::[A-Za-z]+' | sed 's/Self:://' | paste -sd' ')
    printf 'reported:    %s\n' "${reported:-none}"
    # A heading with nothing under it reads as an instrument that found nothing to say rather
    # than as a program with nothing left to report, so the empty case says which it is.
    if [ -z "$reported" ]; then
        printf '\nevery parameter Annex O names is carried out; nothing is only reported\n'
    else
        printf '\nwhy each of the reported ones is reported — the arms, verbatim:\n'
        printf '%s\n' "$body" | sed -n '/return None/,$p' | tail -n +2 | head -n -2 | sed 's/^        //'
    fi
}

# The other populations a document used to state. Each is a `find` or a `ls`, which is the
# whole point: the answer is on the disk rather than in a sentence about the disk.
section_governing() {
    heading "quotations of CLAUDE.md, against CLAUDE.md" \
        "tools/governing-quotations.py"
    # The other half of `--bin quotations`. That one reads a quotation against `doc/md/` and
    # reports the ones that match a specification and then diverge, so a quotation of this
    # project's *own* governing document matches nothing and is invisible to it. Twenty-three
    # ledger rows quoted a retired sentence of CLAUDE.md for ninety-two sessions on that
    # account (ADR 0989). It reports rather than fails: attribution is a proximity rule, so
    # part of what it prints is correct prose saying what CLAUDE.md *used* to state.
    python3 tools/governing-quotations.py || status=1
}

# The last twelve rounds' records, beside the budget `doc/todo/02` section 8 states.
#
# The budget is forty lines and nothing counted it: the six records of sessions 1086-1091 ran 44,
# 47, 19, 40, 40 and 40, and the two over were found by a later round running `wc -l`. The figure
# lives in the check rather than here, so there is one copy of it and it is the one that fails.
section_records() {
    run "records (the last twelve, against doc/todo/02 section 8's budget)" \
        'against a budget of|^  1[0-9]{3} |records, [0-9]+ of them counted' \
        cargo test -q -p conformance --test records -- --nocapture
}

section_counts() {
    heading "populations on disk" "find / ls"
    printf 'fuzz targets:        %s\n' "$(ls fuzz/fuzz_targets/*.rs 2>/dev/null | wc -l)"
    # And how many of them have nothing to fuzz *here*, which is a fact about the disk and not
    # about the tree: `fuzz/corpus` is gitignored. An unseeded target still exits 0 — on `page` it
    # reaches 182 features where its corpus reaches 169 360 — so a count of zero here is the
    # difference between fuzzing and appearing to. `tools/fuzz.sh --list` names them (ADR 0742).
    printf 'fuzz targets unseeded here: %s\n' \
        "$(for t in fuzz/fuzz_targets/*.rs; do
               [ -n "$(ls -A "fuzz/corpus/$(basename "$t" .rs)" 2>/dev/null)" ] || echo x
           done 2>/dev/null | wc -l)"
    printf 'ADRs:                %s\n' "$(ls doc/adr/*.md 2>/dev/null | wc -l)"
    printf 'open todo items:     %s\n' "$(ls doc/todo/[0-9]*.md 2>/dev/null | wc -l)"
    printf 'specification docs:  %s pdf, %s markdown\n' \
        "$(ls doc/*.pdf 2>/dev/null | wc -l)" "$(ls doc/md/*.md 2>/dev/null | wc -l)"
    printf 'pdf.js corpus:       %s\n' "$(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf' 2>/dev/null | wc -l)"
    # `-L`: in a parallel worktree each corpus under doc/corpora is a *symlink* into the main
    # checkout (tools/worktree.sh), and find does not follow symlinks it discovers — so without
    # it every worktree round read "0" here while the main tree held hundreds of documents.
    printf 'doc/corpora:         %s\n' "$(find -L doc/corpora -name '*.pdf' 2>/dev/null | wc -l)"
    printf 'SafeDocs:            never in this tree — `target/safedocs list --dir <path>` counts\n'
    printf '                     whatever has been fetched, and `survey --dir <path>` re-baselines it\n'
}

# How much of `viewer-core`'s vocabulary a C caller can reach.
#
# `doc/ui-boundary.md` and `doc/todo/30` both said the ABI's entry points were "the whole
# vocabulary", which was true when ADR 0346 wrote it and decayed as the vocabulary grew —
# eight messages have been added or reshaped since. The claim is countable, so this counts it
# rather than any document restating it (ADR 0509).
#
# **Only `viewer-ffi` is asked, deliberately.** Every `Command::` and `Query::` in that crate
# is a call: it has no trace module and no wire protocol, so naming a variant there means
# offering it. `viewer-ui` names all of them in `trace.rs` and `viewer-confined` in its
# protocol, so the same grep over those two would answer 100% and mean nothing — trap 11's
# shape, a count whose condition is not the question.
# The `Kind::Variant` names a set of crates uses **in code**, with comments removed first and a
# **word boundary** in front of the name.
#
# **The strip is not fussiness; it is trap 11 caught in the act.** The first run of
# `section_windows` reported both native hosts reaching §12.3.5's collection, on the evidence of
# one line in `viewer-host/src/panel.rs` that read *"a different answer ([`Query::Collection`])
# that this host does not yet ask"*. A rustdoc link is a sentence about a question, not a call —
# so a count whose condition is "the name appears" reported the exact opposite of what the
# sentence said. Both sections below strip `//` to end of line before matching. (That sentence is
# gone since ADR 0711, which made it true the other way; the trap is not, and it is why the strip
# stays.)
#
# **And the `\b` is the same trap a second time, one round on** (ADR 0603). Without it
# `Command::[A-Za-z]+` matches the *tail* of `PathCommand::Close` — `pdf_render`'s path-closing
# display-list command, which `viewer-ui` writes on every rounded rectangle it draws — so the
# question "does this window ever close a document?" was answered by a piece of chrome geometry.
# A grep for an enumeration's variant is a claim about a *path* through the source, and a suffix
# is not one.
#
# **`trace.rs` is excluded for the reason `section_hosts` gives one paragraph up**, and it is the
# third face of the same mistake: `viewer-ui`'s trace formatter matches `Command` exhaustively in
# order to *print* a command's name, so it named every variant of an enumeration that host sends
# twenty-two of. `section_hosts` wrote that down as its reason for asking `viewer-ffi` alone, and
# `section_windows` was then built over `viewer-ui` anyway — the condition was documented and not
# applied, sixty lines apart in one file. A match arm that formats a name is a name printed, not a
# question asked.
#
# **And `quorra-confined` is excluded because it is a different window in the same crate**
# (ADR 0713): it sits on `viewer-confined`'s boundary, where `Query::Frame` is the payload and the
# render events never cross, so counting its sources under `viewer-ui` made this section report
# the tier-2 window asking a question that host's own reading row correctly says it never asks —
# the `SPENT` check fired on a reason that had not been spent. What the confined window reaches is
# its own scope statement (its module documentation and ADR 0713), not yet a column here; it
# becomes one when the established windows move onto that boundary and there is a population to
# rank.
names_in_code() {
    local kind=$1
    shift
    find "$@" -name '*.rs' ! -name trace.rs ! -path '*quorra-confined*' -exec cat {} + \
        | sed 's|//.*||' \
        | grep -oE "\b$kind::[A-Za-z]+" \
        | sed "s/$kind:://" \
        | sort -u
}

section_hosts() {
    heading "viewer-core's vocabulary, and how much of it the C ABI offers" \
        "Command:: and Query:: named in crates/viewer-ffi/src code (comments stripped), against the two enums"
    local kind file all named missing
    for kind in Command Query; do
        file=crates/viewer-core/src/$(printf '%s' "$kind" | tr '[:upper:]' '[:lower:]').rs
        all=$(sed -n "/^pub enum $kind/,/^}/p" "$file" | grep -oE '^    [A-Z][A-Za-z]*' | tr -d ' ' | sort -u)
        named=$(names_in_code "$kind" crates/viewer-ffi/src)
        missing=$(comm -23 <(printf '%s\n' "$all") <(printf '%s\n' "$named" | grep -Fx -f <(printf '%s\n' "$all")))
        printf '%-8s %s of %s reach the ABI\n' "$kind:" \
            "$(($(printf '%s\n' "$all" | grep -c .) - $(printf '%s\n' "$missing" | grep -c .)))" \
            "$(printf '%s\n' "$all" | grep -c .)"
        if [ -n "$missing" ]; then
            printf '         a C caller cannot ask for: %s\n' "$(printf '%s\n' "$missing" | paste -sd' ')"
        fi
    done
    printf '%-8s %s\n' "symbols:" "$(grep -c 'unsafe(no_mangle)' crates/viewer-ffi/src/abi.rs) entry points in abi.rs"
}


# What a *window* reaches, which is the other half of the question `hosts` asks.
#
# **This section exists because a round found the gap by reading rather than by counting.** The
# seven-hundred-and-fourth session took the last three panels into the two native hosts and wrote
# down that §12.3.5's collection and §12.5.6.14's popup windows were still `viewer-ui`'s alone —
# and then that *nothing counted it*, the way this script counts what a C caller cannot ask. A
# parity claim with no instrument decays exactly the way a ledger row does, which is the whole
# argument of ADR 0509's third criterion.
#
# The population is the three hosts that put something on a screen, and **`viewer-host` and
# `viewer-accessibility` are added to each of them** rather than counted on their own: they are the
# crates all three depend on precisely because a host's non-toolkit half lives there, so a window
# that calls `viewer_host::page_entry` reaches §12.3.4 and §12.4.2 without naming either. Counting
# the host crates alone would report three windows blind to a panel all three draw — trap 11's
# shape.
#
# **`viewer-accessibility` joined that list in the seven-hundred-and-thirty-first session, and the
# section said so before this comment did.** That round took the six queries §14.7's tree is built
# from out of `viewer-ui`'s own `access.rs` and into `viewer_accessibility::Reading`, so that the
# two native hosts could publish the same tree rather than derive a second one — and the next run
# reported `viewer-ui` reaching *fewer* queries than before, with `AccessibilityTree` and
# `Readback` credited to no window at all on the day all three started asking them. The population
# is "the crates a window's non-toolkit half lives in", and one had been left out of it. ADR 0623.
#
# `viewer-confined` is deliberately **not** here, for the reason `hosts` gives about `trace.rs`: it
# puts every variant on a wire, so the same grep would answer 100% and mean nothing.
#
# # The reading, which is the half this section did not have
#
# **A count of what a window does not reach is not a list of debts, and printing it without saying
# which is which is how a parity claim decays quietly** (ADR 0603). ADR 0577 wrote that down as a
# note and left the sorting to a later round; two rounds then read the number, wrote "eleven
# queries", and moved on — which is exactly what an uninterpreted figure invites.
#
# So the reasons are below, one per variant, each saying *debt* or *not a debt* and why. They are
# a **reading** rather than a count, which is what `CLAUDE.md` permits to be written down: no
# command can decide whether a `GtkEntry` owning its own caret is a gap or a delegation.
#
# What keeps it from going stale is that the section checks it in **both** directions — a variant
# with no reason is named as owing one, and a reason for a variant every window now reaches is
# named as spent. A round that closes one of these deletes its line and the check says so if it
# did not.
reading() {
    cat <<'READING'
Command:Delegate|not a debt|viewer-ui alone, and by construction. §6.3.2.2's instruction takes the widget appearances *out* of the page so that a host can put real controls there; a tier-2 host that draws its own chrome places none, so delegating would leave a form with holes. viewer_ui::chrome::ChoiceList is the drawn counterpart (ADR 0596).
Query:Dirty|not a debt|all three windows learn that a document has been edited from Event::Dirty and mark their titles from it. The question is for a host that did not keep the event.
Query:Frame|not a debt|viewer-ui alone, and it is the tier rather than a gap: a tier-2 host draws its own pixels onto its own surface and hands the viewer none, so there is no frame of the viewer's to ask about. The answer would be Answer::None, which its own documentation says.
Query:Caret|not a debt|a delegation. Both native hosts place a real GtkEntry or QLineEdit over §12.7's field, and a toolkit's own entry owns its caret; §12.7.4.3's layout question arises only for a host that draws the field itself.
Query:Offset|not a debt|the same delegation: a click placing the cursor inside a toolkit's own entry is the toolkit's arithmetic.
Query:FieldSelection|not a debt|the same delegation: a drag selecting inside a toolkit's own entry is the toolkit's, and Ctrl+C in it is the toolkit's binding (ADR 0519).
Query:FreeTextAt|a debt, named and refused out loud|§12.5.6.6's free-text drag is `t` in viewer_host::keys and both native hosts refuse it by name (ADR 0526), because authoring that annotation is a drag mode plus an editor. doc/todo/33's, not this file's.
Command:View|not a debt, and the reason is this section's own exclusion|a window that keeps the viewer in its own process never loses the view, so it has nothing to put back. The pair exists for a host whose worker can die under it: quorra-confined asks Query::View per frame and echoes the answer back as this, so that a restarted worker resumes where the reader was rather than at page one (ADRs 0734, 0737). That window is deliberately not in this section's population — it is a second window in viewer-ui's crate — which is why a variant one real window does reach reads here as reached by nobody. Closing this line means a *counted* window gaining a worker it can lose.
Query:View|not a debt, and the same exclusion|the question half of the pair above, and not answerable from Query::PageGeometry: recovering a magnification from that answer's scale needs a division this crate refuses to round-trip in `f32`, and inverting its origin would be a host holding a second opinion about the layout arithmetic. Asked per frame by quorra-confined, which this section does not count.
READING
}

section_windows() {
    heading "viewer-core's vocabulary, and how much of it each window reaches" \
        "Command:: and Query:: named in each host's code and viewer-host's (comments stripped)"
    local kind file all host named missing everywhere total sorted
    sorted=$(mktemp)
    for kind in Command Query; do
        file=crates/viewer-core/src/$(printf '%s' "$kind" | tr '[:upper:]' '[:lower:]').rs
        all=$(sed -n "/^pub enum $kind/,/^}/p" "$file" | grep -oE '^    [A-Z][A-Za-z]*' | tr -d ' ' | sort -u)
        total=$(printf '%s\n' "$all" | grep -c .)
        everywhere=""
        for host in viewer-ui viewer-gtk viewer-qt; do
            named=$(names_in_code "$kind" "crates/$host/src" crates/viewer-host/src \
                                  crates/viewer-accessibility/src)
            missing=$(comm -23 <(printf '%s\n' "$all") <(printf '%s\n' "$named" | grep -Fx -f <(printf '%s\n' "$all")))
            printf '%-12s %s reaches %s of %s\n' "$kind:" "$host" \
                "$((total - $(printf '%s\n' "$missing" | grep -c .)))" "$total"
            if [ -n "$missing" ]; then
                printf '             it does not ask for: %s\n' "$(printf '%s\n' "$missing" | paste -sd' ')"
            fi
            everywhere="$everywhere$missing
"
        done
        # A variant missing from all three is the one this section was built to name: a question
        # the boundary answers and no window on any toolkit puts in front of a reader.
        missing=$(printf '%s' "$everywhere" | grep -v '^$' | sort | uniq -c | awk '$1 == 3 {print $2}')
        if [ -n "$missing" ]; then
            printf '             NO WINDOW asks for: %s\n' "$(printf '%s\n' "$missing" | paste -sd' ')"
        else
            printf '             every %s reaches at least one window\n' "$kind"
        fi
        printf '%s' "$everywhere" | grep -v '^$' | sed "s/^/$kind:/" >> "$sorted"
    done
    say_the_reading "$sorted"
    rm -f "$sorted"
}

# The reading beside the count, checked in both directions.
#
# `$1` holds one `Kind:Variant` line per host that does not reach it, so a variant three windows
# miss appears three times and the count of them is printed: *which* windows is what turns a
# reason into a claim somebody can check.
say_the_reading() {
    local unreached line variant hosts verdict why
    unreached=$(sort -u "$1")
    printf '\nthe reading — which of those are debts, and why (ADR 0603, doc/todo/30)\n'
    while IFS= read -r variant; do
        [ -n "$variant" ] || continue
        hosts=$(grep -c "^$variant\$" "$1")
        line=$(reading | grep -F "$variant|" || true)
        if [ -z "$line" ]; then
            printf '  %-28s UNREAD — %s window(s) do not reach it and nothing here says whether\n' \
                "${variant#*:}" "$hosts"
            printf '  %-28s that is a debt. This round owes a reading, in this table.\n' ""
            continue
        fi
        verdict=$(printf '%s' "$line" | cut -d'|' -f2)
        why=$(printf '%s' "$line" | cut -d'|' -f3)
        printf '  %-18s %-2s %s\n' "${variant#*:}" "$hosts" "$verdict"
        printf '%s\n' "$why" | fold -s -w 84 | sed -e 's/[[:space:]]*$//' -e 's/^/                        /'
    done <<EOF
$unreached
EOF
    # And the other direction: a reason kept for something every window now reaches is a sentence
    # about a debt somebody closed, which is exactly how a document goes stale while a count stays
    # right. The round that closes one deletes its line, and this is what says it did not.
    reading | cut -d'|' -f1 | while IFS= read -r variant; do
        [ -n "$variant" ] || continue
        grep -qx -- "$variant" "$1" && continue
        printf '  %-18s SPENT — every window reaches it now, so this reason has outlived it\n' \
            "${variant#*:}"
    done
}

# What a person can actually run, and how old it is. `doc/todo/02` §5 is what refreshes it.
section_binaries() {
    heading "binaries a person can run" "ls -l target/"
    ls -l target/ 2>/dev/null | grep -vE '^total|^d' || printf 'nothing installed — doc/todo/02 §5\n'
}

section_disk() {
    heading "the build directory" "du -sh"
    # Asked for rather than written down: a worktree round has a `target-dir` of its own, and the
    # literal path this used to carry reported the *main* tree's directory from inside every one of
    # them (trap 15). `tools/round.sh` has derived it all along.
    local built root
    built=$(cargo metadata --no-deps --format-version 1 2>/dev/null |
            grep -oE '"target_directory":"[^"]+"' | head -1 | cut -d'"' -f4)
    [ -n "$built" ] || built=target
    du -sh "$built" 2>/dev/null
    du -sh "$built/tmp/pdfref-cache" 2>/dev/null
    # And the root all of them sit in, because that is what `doc/todo/02` §5a's hundred gigabytes
    # is about and this section could not see it. The line above is deliberately the *round's own*
    # directory and stays — from a worktree it is a few hundred megabytes, which answers "what did
    # I build" and reads, wrongly, as an answer to "is the disk full". The two are one line apart
    # now rather than two orders of magnitude apart in silence (ADR 0752).
    #
    # The root is the parent of the round's own directory, which is a convention rather than a
    # derivation — so three things have to hold before it is worth printing, and in an ordinary
    # clone none of them does: the build directory has to sit *outside* the checkout (otherwise
    # the parent is the repository and its size is a fact about the source), and the parent has
    # to hold more than the one directory. `tools/worktree.sh list` breaks the figure down by
    # whose each directory is.
    root=$(dirname "$built")
    if [ "$root" != "." ] && [ "$root" != "$(git rev-parse --show-toplevel 2>/dev/null)" ] &&
       [ "$(find "$root" -maxdepth 1 -mindepth 1 2>/dev/null | wc -l)" -gt 1 ]; then
        du -sh "$root" 2>/dev/null
    fi
}

section_questions() {
    run "questions (the owner's word, and what each answer left open)" \
        '^doc/questions/ holds|^  A[0-9]|^owed by ' \
        cargo test -q -p conformance --test questions -- --nocapture
}

# Every bound in the tree, printed beside the population or the figure it bounds, off one run
# (ADR 1075, ADR 1081). It answers the question no single gate can: **is any gate about to stop
# being able to fire** — a ceiling drifted above its population, a floor below it, a named
# population that moved, a band a figure is creeping toward the edge of.
#
# **A composed section**: every line it runs is a line another section already runs, with the
# ratchet table kept instead of that gate's own summary, so it is not in `all` — a full run pays
# for these walks once. It is the most expensive thing here that is not `oracle`, because the
# bounds are spread over eight gates and two of them read a reference renderer.
#
# **The population is derived, because a hand-written list of gate files is trap 25's shape** —
# ADR 1075's own tier-1 check found three files carrying bounds nobody had listed. Every tracked
# test file under `crates/` or `tools/` that calls into `gate-ratchet` is a line here, and *how*
# to run it comes from `doc/todo/02` §2, which owns the sequence: a gate the sequence runs under
# `--release` is run under `--release` and this script states no second opinion about it. The
# tier-1 check goes first, because its own line says how many bounds are routed through the
# crate, which is the count the table below has to fill.
section_ratchets() {
    gate_binaries
    cargo build --release -p pdf-sandbox --bins >/dev/null 2>&1 || status=1
    # The command is assembled through a variable on purpose: `state_sections.rs` reads every
    # `cargo test` line in this script as a gate line, and a loop written literally would tell it
    # this script runs a gate called `"$package"`.
    local cargo=cargo file line package target profile ignored
    run "bounds routed through gate-ratchet (the count the table below has to fill)" \
        '^ratchets: ' \
        "$cargo" test -p conformance --test ratchets -- --nocapture
    for file in $(git ls-files 'crates/*/tests/*.rs' 'tools/*/tests/*.rs' |
        awk -F/ 'NF == 4' | xargs grep -l 'gate_ratchet::' | sort); do
        package=$(printf '%s\n' "$file" | cut -d/ -f2)
        target=$(basename "$file" .rs)
        [ "$package/$target" = conformance/ratchets ] && continue
        line=$(grep -E "^cargo test .*-p +$package .*--test +$target " doc/todo/02-every-round.md | head -1)
        profile=$(printf '%s\n' "$line" | grep -oE -- '--profile [a-z]+|--release')
        # `--ignored` and not `--include-ignored`, from the sequence's own line. The difference is
        # not cosmetic: `save_round_trip.rs` holds an ignored corpus walk *and* an unignored
        # single-document check over the same document in the same temporary directory, and
        # libtest runs the two in parallel threads — one deletes the file the other's `mupdf` is
        # reading, and the gate fails naming a document that is fine.
        ignored=$(printf '%s\n' "$line" | grep -oE -- '--ignored')
        [ -z "$profile" ] &&
            printf 'doc/todo/02 §2 names no line for %s --test %s, so it runs under the default profile\n' \
                "$package" "$target"
        run "$package --test $target" '^ratchet: ' \
            tools/bounded.sh -- "$cargo" test $profile -p "$package" --test "$target" -- $ignored --nocapture
    done
}

all="ledger departures flags names cited last-sentences navigation conformance annex-o governing questions records counts hosts windows binaries disk tests corpus golden oracle text selection accessibility quorra fixed transform writer archive vfs confined launch frame dates xmp save actions on-disk jpeg2000"
quick="ledger departures flags names cited last-sentences navigation conformance annex-o governing questions records counts hosts windows binaries disk remedies"

# Sections another section already runs. Not in `all`, because a full run pays for every line
# they run — `ratchets` through the gates it composes, `remedies` inside `archive` — and named by
# `--list`, because a section a reader cannot discover is a section nobody runs. `remedies` is in
# `quick` as well, since it is the one line of `archive` that needs no corpus.
composed="ratchets remedies frontier"

case ${1-} in
--list) printf '%s\n' $all $composed; exit 0 ;;
esac

case ${1-all} in
all) sections=$all ;;
quick) sections=$quick ;;
*) sections=$* ;;
esac

for section in $sections; do
    case $section in
    ledger) section_ledger ;;
    departures) section_departures ;;
    flags) section_flags ;;
    names) section_names ;;
    cited) section_cited ;;
    last-sentences) section_last_sentences ;;
    navigation) section_navigation ;;
    frontier) section_frontier ;;
    conformance) section_conformance ;;
    tests) section_tests ;;
    corpus) section_corpus ;;
    golden) section_golden ;;
    oracle) section_oracle ;;
    text) section_text ;;
    selection) section_selection ;;
    accessibility) section_accessibility ;;
    quorra) section_quorra ;;
    fixed) section_fixed ;;
    transform) section_transform ;;
    writer) section_writer ;;
    vfs) section_vfs ;;
    launch) section_launch ;;
    frame) section_frame ;;
    dates) section_dates ;;
    xmp) section_xmp ;;
    save) section_save ;;
    actions) section_actions ;;
    on-disk) section_on_disk ;;
    archive) section_archive ;;
    confined) section_confined ;;
    jpeg2000) section_jpeg2000 ;;
    annex-o) section_annex_o ;;
    governing) section_governing ;;
    questions) section_questions ;;
    records) section_records ;;
    counts) section_counts ;;
    hosts) section_hosts ;;
    windows) section_windows ;;
    binaries) section_binaries ;;
    disk) section_disk ;;
    ratchets) section_ratchets ;;
    remedies) section_remedies ;;
    *)
        printf 'no such section: %s (tools/state.sh --list)\n' "$section" >&2
        status=1
        ;;
    esac
done

exit $status
