# 2003 — A teaching document is held by a test for its names, and by a note for its sentences

Session 1583, slot 7 of batch ninety-two. Status: **accepted** and **built** in
`tools/conformance/tests/teaching.rs`. Context: ADR 2002; the owner's "In order to avoid these files
becoming incorrect over time, add notes for future rounds, to adapt, if necessary"; the rule that a
comment carries the current reason (ADR 1023).

## 1. What a test can hold and what it cannot

A test can hold that a name the document makes **exists**: a path, a `path::name` pair, an ADR number.
It cannot hold that the sentence beside the name is **true**: that `Interpreter::run` still dispatches
every operator, or that the window still draws with the lane the diagram shows. The first fails the
build the day a file is renamed; the second fails nobody and is how a teaching document becomes the
thing a student is misled by.

## 2. Both, and each for what it can do

`teaching.rs` holds, over `doc/teaching/how-a-page-lands-on-the-screen.md`: every word starting
`crates/`, `raster/`, `doc/` or `tools/`, and every `…/….rs`, `.md`, `.wgsl` or `.toml` word, names a
file or directory (a crate-relative label such as `pdf-syntax/src/file.rs` is tried under `crates/`
and `raster/crates/`); every `path::name` names a file containing `name` as a whole word; every `ADR
NNNN` (and `raster ADR NNNN`) is a record; eight diagrams at least, each followed by a **Maintenance
note.** before the next heading; `doc/HANDOVER.md` and `doc/crate-map.md` point at the document. It is
calibrated by a text with four planted faults, which must be reported four times, and the same text
mended, which must be reported none.

The **Maintenance note** after each diagram is the other half: it lists the files and records the
diagram depends on and says what change redraws it. The document's last section addresses the round
that changes the code: read the notes for the files you touched, redraw, and rewrite the sentence in
the present tense.

## 3. What the gate deliberately does not do

It does not read diagrams as graphs, check an arrow, or compare a sentence to a function body. A check
that did would pass a stale picture with a correct label, which is worse than no check; the judgement
stays a person's, and the note is how a person is told to make it. Names inside prose without a
slash or a `::` (a bare `xref.rs`) are not checked.
