# 2548 — A person's text is set only in a face this program may embed

Status: accepted. Session 1857. Q723 is not asked.
Context: ISO 32000-2 §12.5.2, §12.5.6.6, §12.7.4.3, §9.9.1; the project owner's decision of 2026-10-28 on
§12.5.2 ("I agree with your recommendation": the editor sets an annotation's text only in faces this program
can embed, so the archive never meets a font it cannot carry); ADRs 0304, 1414, 1425, 1622, 2170.
Code: `crates/pdf-font/src/embed.rs` (`may_be_embedded`), `crates/pdf-font/src/substitute.rs`
(`installed_covering_embeddable`, `EMBEDDABLE_CANDIDATES`, `covering_paths`, `machine_face`),
`crates/pdf-model/src/variable_text.rs` (`machine_set`, `machine_faces_for`), `crates/pdf-model/src/view/writing.rs`
(`write_added_appearance`), `crates/viewer-core/src/viewer/editing.rs`.

## 1. Decided

**The layout's machine faces are chosen only among faces this program may write into a file.** A value no
compiled-in face draws is set in a face of this machine's (ADR 1414); ADR 1425 writes that face subset and
embedded where its `OS/2` `fsType` permits and leaves the annotation or field without an appearance where it
does not. §12.5.2 has a writer include an appearance dictionary, so that second case was one of the two the
§12.5.2 row departed on. It is now unreachable by construction: `installed_covering_embeddable` walks
`installed_covering`'s own ranking and passes over every face `embed::may_be_embedded` refuses — restricted-
licence usage, bitmap-only embedding, no `glyf` or `CFF ` outlines, the licence and outline halves of
`for_embedding`'s refusals read from the table directory alone, so no candidate is subset to be judged.

**One chooser for the screen and the file.** The filter is in the layout and not at the save, because the
layout is what both draw from: a face chosen for the screen and refused at the save would show one thing and
write another. The cost is written down: on a machine whose only face covering a script forbids embedding,
a field value or note in that script that the screen used to draw from it is now refused on the screen as
in the file, under the existing `Owed` reports. On this machine none is: of the faces `fc-list` names, none
states restricted-licence or bitmap-only usage (a census of each face's `OS/2` `fsType`, run for this
round: 2 575 installable, 45 preview-and-print, 32 editable, 796 files with no `OS/2` table), so no pixel here
can move. A page's own composite font keeps
`installed_covering`, since drawing it is all that is asked of it.

**Bounded before it reads.** Each candidate is a whole font file kept in the face cache, so the walk stops
after `EMBEDDABLE_CANDIDATES` (8) faces; `the_embeddable_walk_reads_no_more_than_its_bound` counts the reads.
A confined process asks its broker for the `skip`th covering face, which `machine_face` now answers from the
same ranked list (`covering_paths`) rather than refusing a `skip` past zero; the wire format is unchanged.

**A save that writes an added annotation with no appearance says so.** `write_added_appearance` answered
`true` where the construction produced no content, so an added free text whose text no face covers was written
without `/AP` in silence; it now answers whether a report came with the empty construction, and
`Written::unappeared` names the annotation (`an_added_note_no_face_can_set_is_named_rather_than_written_bare_in_silence`,
which fails with the old answer planted back).

## 2. What is left on the row

§12.5.2 stays `departed`, on ADR 0304's case alone: text that no compiled-in face and no face this program may
write covers has no appearance this program can construct, retyped or added. Where a retyped note's `/DA`
names a font that cannot set any text (no `Tf`, a font that will not load, a vertical writing mode), the
compiled-in face the editor uses for its own notes could set it; that construction is owed, and the row names it.
