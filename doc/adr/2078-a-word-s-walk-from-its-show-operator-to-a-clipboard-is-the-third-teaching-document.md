# 2078 — A word's walk from its show operator to a clipboard is the third teaching document

Session 1621, slot 7 of batch ninety-seven, a docs round. Status: **accepted** and **built** in
`doc/teaching/how-text-is-selected-and-copied.md` and `tools/conformance/tests/teaching.rs`.
Context: the project owner, 2026-10-19, asking for documents "which explain (probably with diagrams)
how pdf instructions land on the screen"; ADR 2002 (the first walk and the diagram convention), ADR
2003 (what the gate holds and what a note holds), ADR 2062 (the second walk); `CLAUDE.md` principle
4. Clauses: ISO 32000-2 §7.6.4.2 Table 22, §9.3.3, §9.4.3, §9.4.4, §9.10.2, §14.6.2, §14.7.5.2,
§14.8.2.2, §14.8.2.3, §14.8.2.5.1, §14.8.2.5.2, §14.8.2.6.2, §14.8.3.3, §14.9.4. ADR 2079 is unused.

## 1. The gap

The first walk follows a show operator to its pixels and says nothing of the text the same pass
writes; the second follows a keystroke into a field. What a person does most with a page after
looking at it — selecting a sentence and pasting it elsewhere — crosses `pdf-font`'s §9.10.2
methods, `pdf-model`'s readback, marked content and structure tree, `viewer-core`'s pointer,
selection and copy, `viewer-host`'s choice of order, and five hosts' clipboards, and the one place
it was written down in order was nowhere.

## 2. Decisions

1. **One word, the standard's own.** The running example is §14.9.4's EXAMPLE, *Drucker* divided as
   `Druk-` and `ker` with `/ActualText (c)` over the `k-`, inside the least tagged page that gives
   the word a place in a structure: a soft hyphen after a second word, a heading the stream draws
   after the paragraph and the tree puts first, and a folio tagged `/Artifact`. A word the standard
   prints is one a student can find in the text, and this one exercises the replacement, the
   line-end rule of ADR 1948, the soft hyphen of ADR 1971 and the two orders at once.
2. **Every string in the walk was read off the tree.** The readback came from
   `crates/pdf-model/examples/readback.rs`, the drag and both copies from the C ABI
   (`quorra_pointer`, `quorra_select`, `quorra_copy`) over the example written out as a file. The
   walk's last section asks a round that changes stages 1, 2 or 4 to do the same, because a string
   reasoned out is a sentence the gate cannot catch.
3. **Five stages, in the order the text takes them**: the readback (§9.10.2, §9.4.4, §14.8.2.6.2),
   what the producer stated over the glyphs (§14.9.4, §14.8.2.3, §14.7.5.2), the drag, the
   structure's order (§14.8.2.5), the copy (Table 22 bit 5). **`/ActualText` and the soft hyphen sit
   before the drag**, although the brief listed them after the structure: a replacement is applied
   at the `EMC`, inside the interpretation, so by the time a pointer moves the readback already
   says `Drucker`; the soft hyphen arrives in stage 2 as a character and is undone in stage 5 by
   `rejoined`, and the walk says so in both places.
4. **Selection is drawn as a choice.** Stage 3 names no clause for what a drag means, because the
   standard states none; it names the two that the choices are built on (§9.4.4's placement,
   Table 120's line height) and the module that writes each choice down.
5. **The gate holds three walks from one table.** `WALKS` gains the third entry with six diagrams.
   The calibration is unchanged; a second was run by planting three faults in the new walk (a file,
   a name, an ADR), reported three times and then taken out.

## 3. Cost

A third document that describes code and goes stale as the other two do, held the same way: names
by the gate, sentences by the maintenance notes. It crosses more crates than either, and the files of slots running beside it in the same batch (the confined window's sending half, the
toolkit windows' device) are named only where their selection and clipboard paths are, which those
slots do not touch. The diagrams were parsed with Mermaid 11.4.1 from a scratch install (21 of 21
across the three walks, two planted errors caught); that check is a round's, not a gate's, for ADR
2002 §2's reason.
