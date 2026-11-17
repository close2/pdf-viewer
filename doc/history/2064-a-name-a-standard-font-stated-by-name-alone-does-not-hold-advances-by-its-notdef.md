# 2064 — A name a standard font stated by name alone does not hold advances by its `.notdef`

Session 2064, a fonts round in the audit worktree. What round 2062 left: rows 9.6.5.2, 9.2.4 and 9.6.2.2; ADR 2942.

**The slice.** Record 2062's last "Left" item: a `Times-Roman` stated by name alone showing `/beta` was laid out at
zero advance, and the converted file overprinted its glyphs.

**The premise.** It held on HEAD. In the window the `beta` code reached no glyph (`glyph_index` `None`) and advanced
by 0, against `/.notdef`'s 0.25 and `A`'s 0.722; the archive's pack route drew the pack's real beta at that zero
(2062's `nd` output: `/Widths` 0 for beta and Zhe, the glyphs piled up). No ADR after 2938 settled it.

**The reading.** §9.6.5.2 substitutes `.notdef` for a name the program lacks; §9.2.4 positions glyphs "according to
their widths", the width being the painted glyph's; Table 109's `/MissingWidth` sentence needs a range and a
descriptor a name-alone font does not state. So the width is `.notdef`'s, derived, not chosen. The paragraph under
Table 109 is struck by Errata Collection 3 (ADR 0254) and is not leaned on.

**Built.** `metrics::notdef_advanced` gives such a code the drawn face's glyph-0 advance (250 for Times, 600 for
Courier) and records it; `LoadedFont::advanced_by_notdef` says so. A descriptor-bearing standard font keeps ADR
2687's `/MissingWidth`. `from_pack::own_advances` states the pack glyph's own advance for such a code (the window's
rule applied to the face the file draws); `pack_face_proved` and `state_widths` take the laid-out advances.

**Conversions.** 2062's `nd` fixture (`Times-Roman`: beta, a-ogonek, Zhe; `Courier-Bold`: Gamma, Zhe, s-caron) under
`as-if-printed`: `/Widths` beta 0 → 570, Zhe 0 → 980, Courier 0 → 600; drawn and looked at, no overlap; the window
now sets the s-caron after two `.notdef` advances. `as-shown` refuses as before (`machine-face`). veraPDF 1.30.3
passes both outputs (2b, 4). **Plants**: `notdef_advanced` switched off fails both new pdf-font tests; `own_advances`
switched off fails the new pack test.

**Gates.** Over `pdf-font`, `pdf-transform`, `pdf-model`: fmt exit 0; clippy `-D warnings --all-targets` exit 101
on `load_composite`'s length and a quotation's names, then exit 0; nextest 4287 of 4287 passed, and the five pack
tests passed against the built pack; rustdoc `-D warnings --document-private-items` exit 0. `cd fuzz && cargo check
--bins` not owed: no public type or signature changed (one method added). Conformance after `git add -N`: exit 101 on a quotation
filed under §9.2.4 rather than §9.8.1, then exit 0.

**Left.** A code no encoding names (an empty slot) still takes 0; a font outside the fourteen keeps ADR 2898's
order; a machine face holding the glyph (ADR 0270) answers with its own advance, as before.
