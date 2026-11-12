# 2830 — A font the window drew as nothing is embedded drawing nothing

Session 2006, a single archive round.
Status: accepted and built. In `pdf-font`: `embed::with_glyphs_emptied`, `cff::with_glyphs_emptied`,
`embed::postscript_name` made public. In `pdf-transform`: `archive/fonts.rs` (`embed_pass`: a cut
program's glyphs emptied, no machine's face for it; `CutProgram::describe`),
`archive/fonts/described.rs` (`drawn_as_nothing`, `loaded_as_written`),
`archive/fonts/machine_cid.rs` (`drawn_as_nothing`, `written_in`), `archive/decision.rs` and
`decision/fonts.rs` (the loss `unreadable-program`, in the font site's bundle),
`archive/config.rs` (its per-loss key), `archive/statement/fonts.rs` (its line),
`archive/rewrite/values.rs` (the substitution's `/CIDToGIDMap` kept);
`doc/profiles/as-shown.toml` (`unreadable-program = "discard"`).
Tests: `crates/pdf-transform/tests/archive/cut_program.rs` (all six), `pdf-font`'s
`embed::tests::emptied_glyphs_draw_nothing_at_the_advance_they_had`.
Context: ADR 1094 (a refused program keeps the document's metrics), ADR 2636 (a cut program takes
the unembedded route), ADR 2824 (a CID drawn as nothing goes to an empty glyph), ADR 2829 (a
second per-loss key at the font site), the owner's note on R08/R03 of 2026-10-28 ("as if printed
and scanned"), ADR 2693 (a loss is named in the history).
Clauses: ISO 32000-2 §9.1, §9.2.4, §9.7.4.2, §9.8.1, §9.9; ISO 19005-2 sections 6.2.11.4.1,
6.2.11.5, 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1, 6.2.10.5, 6.2.10.9.

## Context

ADR 2636 answered a font program whose data does not decode whole, or is no program in its
format, as a font with none: the front dropped, the shipped face embedded. It argued that no loss
word was owed because "the front draws nothing in this reader". The premise held and the
conclusion did not: the window draws such a font from its widths alone and marks nothing (ADR
1094), and the face embedded in its place draws every glyph. Rendered on the processor at 72 dpi,
`issue11651`'s `as-if-printed` output shows "Hello world Test" in Liberation Sans where the window
shows two red boxes, and `178360`'s shows a whole newsletter of text where the window shows its
frames. `as-if-printed` promises the page as if printed and scanned, and a print of this reader
carries no text there.

No clause forbids the window's nothing. §9.1 says a font dictionary carries "information that can
be used to provide a substitute when the font program is not available", and §9.8.1 that the
descriptor's metrics "enable a PDF processor to synthesise a substitute font or select a similar
font when the font program is unavailable": both permit a substitute and neither requires one.
Whether the window should draw one for an unreadable program is ADR 1094's question, not this one.

## Decision

1. **The route follows the window.** Where `cut_program` finds a rendered font's program cut or
   unparsed and `LoadedFont::load` refuses it — the condition under which the window draws it from
   metrics alone — the program written draws nothing for every code the page shows. Whether the
   window should draw a substitute instead is Q837; should it come to, `cut_program`, which reads a
   font `load` accepts as a program drawn whole, changes with it.
2. **A simple font keeps the route ADR 2636 gave it, and its shown glyphs are emptied.** The
   shipped face, the encoding or `cmap` restatement and the advances are the route's, proved as
   before; the font as written is then loaded, the glyph each shown code reaches is found, and
   `pdf_font::embed::with_glyphs_emptied` empties it: a `glyf` glyph given no bytes in `loca`
   (ISO/IEC 14496-22), a `CFF ` glyph rewritten as its width and `endchar` (Technical Note #5177).
   Every other byte is the face's, so the encoding work the route did stands. The machine's face is
   not offered: nothing of its shapes would be drawn.
3. **A composite font goes to ADR 2824's empty glyphs directly**: `.notdef` of the shipped `glyf`
   face for the descendant and one empty glyph per `/W` advance, a `/CIDToGIDMap` stream sending
   each shown CID to its glyph (§9.7.4.2), under a `CIDFontType2`. No `/ToUnicode` is needed, and
   none is read; the producer's `CMap`, `/CIDSystemInfo` and `/ToUnicode` stay.
4. **The proof**: the font as written, loaded by the window's reader, draws no outline for any
   shown code, is no substitute, and advances each code by the bits the window's metrics-only
   reading advances it; otherwise nothing is written and the font is refused by name.
5. **The loss `unreadable-program`.** The page draws as the window drew it, and the program front
   is gone; what the output also loses is the substitute another reader may have drawn from the
   descriptor, which §9.1 and §9.8.1 describe as what the dictionary's information is for: the
   file now states that the text draws nothing. It is a loss of its own, and the face losses
   (`machine-face`, `substitute-stretch`, `glyphs-from-to-unicode`) no longer answer a cut program.
   **Per profile**: `as-shown` admits it by a per-loss key, as ADR 2829 admits the glyph choice,
   since the window showed nothing; `as-if-printed` and `state-every-loss` carry it in the font
   site's bundle, a print having carried nothing; `refuse-any-loss`, `no-data-loss`,
   `keep-everything`, `only-metadata-loss` and `derive-attachments` stop, as they stopped before on
   the face losses. The report, the file's `xmpMM:History` and the statement page name it per font.

6. **The substitution's `/CIDToGIDMap` is not written over.** A cut program is embedded, so the
   source fails `fonts/cid-to-gid-map-present` where its descendant states no map, and that
   remedy's `Identity` was written whenever the entry read as absent through the source — which a
   reference to the stream this route writes, an object only the output holds, always does. The
   output then reached glyphs past the subset's last; this tree's reader drew nothing for them and
   its checker passed them, while veraPDF failed 6.2.11.4.1, 6.2.11.5 and 6.2.11.8 (round 1996's
   habit: two readers of one byte). The remedy now leaves a `CIDFont` whose gains carry a map.

## Consequences

`issue11651` converts under `as-shown` at both parts; its `as-if-printed` and `state-every-loss`
outputs draw what the window drew. `435321` and `268346` (part 4) convert as before under the
two bundle profiles, now drawing what the window drew. A program refused by name where the route
cannot prove the nothing is a refusal where there was an output showing text no reader of this
program saw.
