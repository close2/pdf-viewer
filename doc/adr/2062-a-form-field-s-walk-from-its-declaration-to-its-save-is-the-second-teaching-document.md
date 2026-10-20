# 2062 — A form field's walk from its declaration to its save is the second teaching document

Session 1613, slot 7 of batch ninety-six, a docs round. Status: **accepted** and **built** in
`doc/teaching/how-a-form-field-is-filled.md` and `tools/conformance/tests/teaching.rs`. Context: the
project owner, 2026-10-19, asking for documents "which explain (probably with diagrams) how pdf
instructions land on the screen"; ADR 2002 (the first walk and the diagram convention) and ADR 2003
(what the gate holds and what a note holds); `CLAUDE.md` principle 4. Clauses: ISO 32000-2 §7.5.6,
§12.5.1, §12.6.3, §12.7.2, §12.7.3, §12.7.4.1, §12.7.4.2, §12.7.4.3, §12.7.5.3. ADR 2063 is unused.

## 1. The gap

The first walk follows a content stream *out* of a file to a pixel. A reader's other direction —
a keystroke going *into* a document — crosses as many crates (`pdf-model`'s field tree, view state,
scripts and appearance construction, `viewer-core`'s commands and log, three hosts' keyboards,
`pdf-script` and its worker, `pdf-syntax`'s writer), and the one place it is written down in order
was nowhere: `doc/state-of-play.md` lists what is built, `doc/ui-boundary.md` the messages, and each
module's comment its own step.

## 2. Decisions

1. **One field, the standard's own.** The running example is §12.7.5.3's EXAMPLE, the text field
   `Silly prose`, with the three objects a file needs around it (the catalog's `/AcroForm`, the page's
   `/Annots`, the resource dictionary). A field the standard prints is one a student can find in the
   text, and its Multiline flag makes the walk show the commit that comes from Tab rather than Enter.
2. **Six stages, in the order a value takes them**: the declaration (§12.7.3, §12.7.4), the key
   press in a window, the value into the log (`Edit::SetField`, `Command::CommitField`), the
   scripts, the appearance (§12.7.4.3), the save (§7.5.6). **The scripts come before the appearance**,
   although the brief listed them after: Table 199's `/K` runs inside `ViewState::set_field` before a
   widget takes the value, and the commit's `/V` and `/CO` before the next frame is drawn; only `/F`
   runs at the layout, and stage 5 says so where it runs.
3. **Two shapes of host, drawn as two**: the toolkit hosts place a native control over a page
   delegated without the widget's appearance (ADRs 0244, 0245), and `quorra` draws the appearance
   itself and asks the core for the caret. Both send the whole value per keystroke and the commit as
   its own message, which is the part a new host (the .NET one of RFC 0010) has to keep.
4. **The walk closes the loop** by reopening the saved file: §7.5.6's newest-copy rule read by
   `xref.rs` (ADR 0100) is what makes the appended `/V` the one the next open sees.
5. **The gate holds both walks from one table.** `teaching.rs`'s single document constant becomes
   `WALKS`, each walk with the diagrams it owes (eight, and now seven), and the pointer test asks
   `doc/HANDOVER.md` and `doc/crate-map.md` for every walk. The calibration is unchanged; a second
   one was run by planting three faults in the new document (a missing file, a missing name, a
   missing record), reported three times and then mended.

## 3. Cost

A second document that describes code and goes stale the same way, held the way ADR 2003 holds the
first: names by the gate, sentences by the maintenance note under each diagram. The diagrams were
parsed with Mermaid's own parser (11.4.1, from a scratch install, calibrated by two planted errors);
that check is a round's, not a gate's, because no Mermaid toolchain is in the build (ADR 2002 §2).
