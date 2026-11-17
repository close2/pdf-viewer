# 2060 — The Latin-Greek-Cyrillic pack is built, and asked after the shipped face

Session 2060, a PDF/A round in the audit worktree. ADR 2630's second font pack; row 9.9's note; `doc/todo/76`
row 28. ADRs 2934 and 2935; Q888.

**The slice.** ADR 2630 decided Noto's `latin-greek-cyrillic` pack and nobody built it (round 2058's sweep).

**The premise.** It held: no ADR or record after 2630 built it; ADR 2378's order has a place after the CJK pack;
ADR 2642 closed requirement 4. Noto Sans Mono (unread in 2630) maps the same Latin, Greek and Cyrillic blocks as
Noto Serif and has no italic. One part had moved: on this VM ADR 2773 already answers both documents' `as-shown`
lines from the machine's URW faces, so the pack is what makes the answer independent of the converting machine.

**Built.** `tools/lgc-pack.py`: google/fonts `097bc1b8`'s three variable fonts and `OFL.txt` by SHA-256, one
`instancer` static per style (weight 400/700, width 100), written as bare CFF with each width the first operand,
every glyph under each Adobe Glyph List 2.0 name of its character; two builds wrote the same bytes, zip too.
`archive/fonts/lgc_pack.rs` holds the six digests; `face_for` asks the pack where the shipped face lacks a shown
code or is no face for the font, holds the read-back font to the pack face's glyphs (`pack_face_proved`), names
`machine-face`, writes the OFL into the program's `/Metadata`; `rekeyed` asks it after the CJK pack.
`--lgc-pack` and the directory beside the executable, the broker's read list, CI's `lgc-pack` job and the
snapshot's paragraph; `doc/lgc-pack.md`, `NOTICE` section 8. `CjkPack` and `LgcPack` became one pointer each,
which keeps `Plan` under `clippy::large_enum_variant`.

**Conversions.** `issue5244` under `as-if-printed` at 2b and 4 is set as text in Noto Serif Bold where HEAD
photographed it; its page was drawn and looked at. The Greek document is photographed as at HEAD for one code: its
`/Delta` is AGL 2.0's U+2206, which Noto Serif lacks (ADR 2935, Q888). `as-shown`: both as HEAD (the machine's
face). A fixture of beta, a-ogonek, Zhe in `Times-Roman` and Gamma, Zhe, s-caron in `Courier-Bold` converts
under `as-if-printed` at 2b and 4 in the two Noto faces, drawn and looked at. veraPDF 1.30.3 passes all twelve.
**Plant**: the proof given the wrong glyph fails the pack test.

**Gates.** Over `pdf-transform` and `viewer-host`: fmt exit 0; clippy `-D warnings --all-targets` exit 0 (after
`large_enum_variant`, `single_match_else` and `too_many_lines` were answered); nextest 1570 of 1570 passed, and
the two ignored pack tests passed against the built pack; rustdoc `-D warnings --document-private-items` exit 0.
`cd fuzz && cargo check --bins` not owed: no fuzz target depends on `pdf-transform`. Conformance after `git add
-N`: exit 101 on this record's gates paragraph and on `batch.rs`'s wrapper-line test, then exit 0.

**Left.** `/Delta` (Q888); a `TrueType` dictionary and an italic fixed-pitch request; the descriptor-less
fourteen's route; subsetting the pack face (the whole 0.7 MB program goes in); under `as-shown`, a font whose
machine face the proof refuses stops there before the pack is asked.
