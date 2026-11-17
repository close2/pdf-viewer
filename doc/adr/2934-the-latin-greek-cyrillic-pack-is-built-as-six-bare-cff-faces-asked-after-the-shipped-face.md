# 2934 — The Latin-Greek-Cyrillic pack is built as six bare CFF faces, asked after the shipped face

Session 2060. Status: **implemented.** Context: ADR 2630 (the decision, its four requirements on a pack and its
licence reading), ADR 2642 (both width statements restated), ADR 2378 (the `Adobe-Identity` face order), ADRs
2302 and 2303 (the CJK pack, its route and how it is found), ADR 2162 (`machine-face`), ADR 2773 (the machine's
face the window drew), A569 and A737 item 1. `doc/lgc-pack.md` is the layout.

## 1. The premise, checked

No ADR or record after ADR 2630 built the pack (`grep` of `doc/adr` and `doc/history`; `doc/todo/76` row 28).
ADR 2378's order has its place: shipped face, then the CJK pack, then this pack. ADR 2642 closed requirement 4.
Noto Sans Mono, which ADR 2630 left unread, maps 3 490 code points at the pinned commit: the same Latin, Greek and
Cyrillic blocks as Noto Serif (128 of Latin Extended-A, 121 Greek, 256 Cyrillic), with mathematical operators
and box drawing besides; it is published in no italic. Every source states `fsType` 0.

One premise moved since ADR 2630: on this VM both documents' `as-shown` lines are already answered by ADR 2773,
which embeds the machine's URW face the window drew. The pack is what makes the answer independent of the
converting machine, and the `as-if-printed` lines are the pack's.

## 2. The decision

1. **Inputs**: google/fonts at commit `097bc1b8`, the two Noto Serif and the one Noto Sans Mono variable fonts
   and `OFL.txt`, each held to its SHA-256 (`tools/lgc-pack.py`'s `SOURCES`).
2. **Faces**: one static instance per style by fontTools 4.66.1's `instancer` at weight 400 or 700 and width 100
   — Noto Serif in the Foxit serif face's four styles, Noto Sans Mono in its two upright ones.
3. **Format**: bare CFF, each charstring drawn from the instance's outlines with its width as the first operand
   and no subroutine, so it goes under a `Type1` dictionary as `/FontFile3 /Type1C` (Table 124) exactly as the
   Foxit faces do, and ADR 2642's restatement reads one statement.
4. **Names**: one glyph under every name the Adobe Glyph List gives a character the source maps (`afii10017` and
   `Acyrillic` both), `uniXXXX` where it gives none (ADR 2935).
5. **The order**: on the descriptor route, the operator's face, then the machine's face the window drew where the
   profile withholds `machine-face`, then the shipped face, then — where the shipped face lacks a shown code or is
   no face for the font at all — the pack's face for the request, taken only if it draws every shown code. On the
   `Adobe-Identity` route, after the CJK pack. A pack file whose digest is not the builder's is refused by name.
6. **The proof**: the converted font is read back over the file's own descriptor; every shown code must select
   the pack face's own glyph, and every one of the 256 advances must be the source's, with no stretch.
7. **The loss** is `machine-face`; the notice is the OFL, verbatim, in each program's `/Metadata`.

## 3. Not answered, each by its sentence

A `TrueType` dictionary (no row admits a bare CFF under it, and the pack face is not restated as another kind);
an italic fixed-pitch request; the standard fourteen's descriptor-less route, which holds the shipped face to the
window; a code whose glyph name the pack lacks, of which the Greek document's `/Delta` is the one met (ADR 2935).

## 4. Evidence

Two builds wrote the same bytes (the zip too). `issue5244` under `as-if-printed` at 2b and 4 is set as text in
Noto Serif Bold, where HEAD photographed it; the Greek document is photographed as at HEAD, for `/Delta` alone.
A fixture of `Times-Roman` showing beta, a-ogonek and Zhe and `Courier-Bold` showing Gamma, Zhe and s-caron
converts under `as-if-printed` at 2b and 4 and draws all six. veraPDF 1.30.3 passes every output. The plant: the
proof given the wrong glyph fails the pack test.
