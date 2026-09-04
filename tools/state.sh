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

section_quorra() {
    gate_binaries
    run "quorra against the CPU oracle" \
        '^[0-9]+ pages compared|^  (rasterisation|median page)' \
        cargo test --profile gates -p render-quorra --test corpus -- --ignored --nocapture
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
# two figures on the launch path that have no clock in them at all.
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

# RFC 0003 section 5.2's five write verbs and section 4's whole layout, over every corpus document
# the core opens. The `--bins` build is trap 10: a `--profile gates --test` line builds one test
# target and nothing else, so `pdf-vfs-worker` beside it would otherwise be whatever an earlier
# round left. There is no third line here: session 917's `awkward_classes` became the read walk's
# population in session 919 (ADR 0878), and the walk of the *other* confined program that inherited
# the name is `doc/verify.md`'s rather than `doc/todo/02` §2's.
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
# **And `pdf-viewer-confined` is excluded because it is a different window in the same crate**
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
    find "$@" -name '*.rs' ! -name trace.rs ! -path '*pdf-viewer-confined*' -exec cat {} + \
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
Command:Close|not a debt|every window opens exactly one document from its command line and lives as long as it, so there is no second document to close and closing the only one is quitting. It is Query::Collection's companion: §12.3.5 presents several documents, and a host that presented one would hold two DocumentIds and need both this and Focus.
Command:Focus|not a debt|the same pair, and the same condition: choosing between two open documents is a question no window can be asked while every window has one.
Command:Delegate|not a debt|viewer-ui alone, and by construction. §6.3.2.2's instruction takes the widget appearances *out* of the page so that a host can put real controls there; a tier-2 host that draws its own chrome places none, so delegating would leave a form with holes. viewer_ui::chrome::ChoiceList is the drawn counterpart (ADR 0596).
Query:Dirty|not a debt|all three windows learn that a document has been edited from Event::Dirty and mark their titles from it. The question is for a host that did not keep the event.
Query:Frame|not a debt|viewer-ui alone, and it is the tier rather than a gap: a tier-2 host draws its own pixels onto its own surface and hands the viewer none, so there is no frame of the viewer's to ask about. The answer would be Answer::None, which its own documentation says.
Query:Caret|not a debt|a delegation. Both native hosts place a real GtkEntry or QLineEdit over §12.7's field, and a toolkit's own entry owns its caret; §12.7.4.3's layout question arises only for a host that draws the field itself.
Query:Offset|not a debt|the same delegation: a click placing the cursor inside a toolkit's own entry is the toolkit's arithmetic.
Query:FieldSelection|not a debt|the same delegation: a drag selecting inside a toolkit's own entry is the toolkit's, and Ctrl+C in it is the toolkit's binding (ADR 0519).
Query:FreeTextAt|a debt, named and refused out loud|§12.5.6.6's free-text drag is `t` in viewer_host::keys and both native hosts refuse it by name (ADR 0526), because authoring that annotation is a drag mode plus an editor. doc/todo/33's, not this file's.
Command:View|not a debt, and the reason is this section's own exclusion|a window that keeps the viewer in its own process never loses the view, so it has nothing to put back. The pair exists for a host whose worker can die under it: pdf-viewer-confined asks Query::View per frame and echoes the answer back as this, so that a restarted worker resumes where the reader was rather than at page one (ADRs 0734, 0737). That window is deliberately not in this section's population — it is a second window in viewer-ui's crate — which is why a variant one real window does reach reads here as reached by nobody. Closing this line means a *counted* window gaining a worker it can lose.
Query:View|not a debt, and the same exclusion|the question half of the pair above, and not answerable from Query::PageGeometry: recovering a magnification from that answer's scale needs a division this crate refuses to round-trip in `f32`, and inverting its origin would be a host holding a second opinion about the layout arithmetic. Asked per frame by pdf-viewer-confined, which this section does not count.
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

all="ledger conformance annex-o counts hosts windows binaries disk tests corpus oracle text selection accessibility quorra fixed transform writer vfs launch dates xmp jpeg2000"
quick="ledger conformance annex-o counts hosts windows binaries disk"

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
    selection) section_selection ;;
    accessibility) section_accessibility ;;
    quorra) section_quorra ;;
    fixed) section_fixed ;;
    transform) section_transform ;;
    writer) section_writer ;;
    vfs) section_vfs ;;
    launch) section_launch ;;
    dates) section_dates ;;
    xmp) section_xmp ;;
    jpeg2000) section_jpeg2000 ;;
    annex-o) section_annex_o ;;
    counts) section_counts ;;
    hosts) section_hosts ;;
    windows) section_windows ;;
    binaries) section_binaries ;;
    disk) section_disk ;;
    *)
        printf 'no such section: %s (tools/state.sh --list)\n' "$section" >&2
        status=1
        ;;
    esac
done

exit $status
