# 460 — The entry nobody wired, and the file behind the paperclip

**Finding.** The four-hundred-and-fifty-ninth named a sixth refusal shape — a ledger row that
retires its refusal by naming a capability that arrived — and left it with **no sweep**. This
round built the fifteenth sweep for it, ran it, and its strongest hit was §12.5.6.15: an
`implemented` row explaining Table 187's **required** `/FS` as "the embedded file, which is not a
rendering question", while the clause says what activating a file attachment annotation does —
"activating the annotation extracts the embedded file and gives the user an opportunity to view it
or store it in the file system". §7.11.4.1 gives an embedded file two homes and this program
walked one of them, so a document that hung its file on a *page* rather than on the
`/EmbeddedFiles` name tree showed a paperclip with nothing behind it. That is the corpus's one
file attachment annotation — and **all six in ISO 32000-2's own PDF**.

**Date.** 2026-08-13.
**ADR.** [0295](../adr/0295-the-entry-nobody-wired-and-the-file-behind-the-paperclip.md).
**Touched.** `crates/pdf-model/src/attachment.rs` (`of_annotation`, the module header, three
tests), `crates/pdf-model/src/action.rs` (§12.6.4.4's duplicate `/FS` read folded into it),
`crates/pdf-model/examples/file_attachment_census.rs` (new),
`crates/viewer-core/src/viewer.rs` (`exhibit`, `attached_file`, `hand_over`),
`crates/viewer-core/src/query.rs` and `command.rs` (why the two routes are not one list),
`crates/viewer-core/tests/headless.rs` (one test),
`doc/conformance/ledger.toml` (§7.11.4.1, §12.5.6.15),
`doc/todo/01-ledger-partial-rows.md` (the fifteenth sweep and its first run),
`doc/habits.md` (the sixth shape's instrument), `doc/adr/0295-*`, this file.

## The sweep, and what it refuses to read

Fourteen sweeps read a row's stated *reason*. The sixth shape has no wrong reason to find — it
names something that exists — so the fifteenth reads none. It takes the entries the clause's own
tables state, out of the standard itself, and asks two questions of each: does any `.rs` file in
`crates/`, `tools/` or `fuzz/` name it, and does any file the row itself lists in `code = [...]`?

**The second question is the sweep.** `"Open"` — the entry the previous round found unread — *is*
named in `crates/`, by the popup reader under a different table, so question one alone passes the
row the sweep exists for.

First run: 168 rows in the population, 30 stating an entry their own code does not name, 57
entries. Most are refusals `CLAUDE.md` already closes; one was work. A second finding came free:
**§7.11.4.1's row had named the missing caller in so many words** — "§12.5.6.15's file attachment
annotations being the caller the clause names, and not yet built" — so the ledger held the
question and the answer in two families, and no grep in `doc/todo/01` looks for a sentence that
names a clause rather than a capability.

## What the round verified rather than assumed

- **Counted before believed** (trap 11). `file_attachment_census` over the 964 openable documents:
  one `/FileAttachment` annotation, its file embedded, **not** named by the `/EmbeddedFiles` tree,
  and stating `/Contents` beside no `/Desc`. Over `doc/ISO_32000-2_sponsored_EC3.pdf`: six, six,
  none, six. So the corpus witnesses the *unreachable file* and **cannot rank the `shall`** about
  which text describes it — the fixture for that is a pair of documents differing only in
  `/Contents` (trap 8), and each half was watched fail with the rule removed.
- **The design was decided by measurement, and the measurement refused the obvious design.** A
  document-wide list of these files costs a walk of every page's `/Annots`: 78 to 123 ms cold over
  three runs on ISO 32000-2's 1023 pages, 13 to 15 ms warm — and `viewer-ui`, `viewer-gtk` and
  `viewer-qt` all ask `Query::Attachments` on `Event::Opened`, which would put a full page-tree
  walk on the launch path `CLAUDE.md` forbids one on. The file is reached by activating its
  annotation instead, which is the clause's own sentence, and the reason is written beside
  `Query::Attachments` for whoever builds the lazily-filled panel.
- **The first walk written was ten times slower than the one shipped**, and the census still says
  so: asking `Pages::get` for every index descends the page tree once per page, 870 ms warm where
  `Pages::indices` plus `Document::get` is 14. The same multiplication session 141 met with
  §12.3.3's outline.
- **Run in the real window under `Xvfb`**, because no gate here sees a host: a click on the
  paperclip in `annotation-fileattachment.pdf` wrote `Test attachment` beside the document, with
  `the embedded file "Test.txt" does not match the MD5 checksum the document states for it` in
  front of it. That note is correct and is this corpus's first witness to it — the file writes the
  MD5 digest as a UTF-16BE text string with a byte order mark, and Table 45 says "a 16-byte
  string", so a checksum stated wrongly is reported and the bytes still come.
- **No message was added to the boundary.** The file crosses as `Event::Extracted`, which all six
  consumers already handle, so the C ABI, the confined pipe and the three native hosts got it
  without a line.
- **Gates.** `fmt`, `clippy --workspace --all-targets` (silent), `nextest` 1643 passed / 11
  skipped, the doctests, the corpus gate, the oracle, both text gates, dates, XMP, JPEG 2000, the
  quorra corpus gate and `cargo test -p conformance` — all pass, and none of them moved, which is
  what a round that adds a verb and no pixel should produce.
- **Sweeps.** The fourth over this round's nouns (`/FS`, "not a rendering question", "not yet
  built") is clean but for the two rows this round corrected; the first over the ledger prints
  seven, all of them known (printing, geospatial, and three rows quoting their own retired
  wording); the third prints 25 over the ledger and 42 over the source roots, every one a
  boundary a crate keeps.

## What the next round should know

- **The sweep is a reading list with 29 rows still on it**, and the population is worth widening
  the other way: it only looks at rows whose note names an *arrival*. A row that never had a
  refusal to retire is not in it at all.
- **The panel is the half not built.** Listing §12.5.6.15's files document-wide needs the hosts to
  ask `Query::Attachments` when the attachments tab is first shown rather than when the document
  opens — three hosts, plus the count `viewer-ui` prints in its open-time summary line.
- **`doc/md/`'s Table 200 comes out with its columns shifted**, so four of its five keys read as
  prose. The sweep saw one `/DS` where the table has five ECMAScript actions; anything that parses
  that table out of the conversion should check the PDF (`pdftotext -layout`) first.
