# 1989 — A CFF face takes the window's fitting

Session 1989, a single round, PDF/A lane's worktree. R09's largest remainder; ADR 2789.

**The premise.** Partly true. Round 1984's census refused 18 documents at each part under `as-shown`
on `substitute-stretch`, and each one's stretched font is drawn from a URW face, OpenType with a
`CFF ` table. Run under every profile at both parts with the loss authorised, 14 had nothing else to
refuse under `as-shown`; 020747 and 492484 stop on content-stream rows, 064034 on a hexadecimal
string and 504076 on an operator. The census line names only the first refusal, which is why.

**Built.** ADR 2789 (`pdf_font::cff::with_outlines_scaled`, `pdf_font::stretch::cff_outlines`): each
charstring is run as the interpreter runs it and stated again without calls, every x scaled about
the origin and rounded to a whole unit, every y kept, `vstem` edges and `endchar`'s `adx` scaled,
masks, `hstem` and the width copied, flex kept with its depth (Technical Note #5177 sections 2, 3.1,
3.2, 4.1 to 4.3 and 4.7). This tree's outline reader reads a CFF coordinate as its whole part, so
16.16 fractions were tried first and dropped; a line rounding to no length is not stated, and both
proofs admit that line and nothing else. `hmtx` bearings, `head` and `hhea` follow; advances copied.
The fitting is written where `substitute-stretch` is withheld (ADR 2770); all 35 URW faces take it.

**Freed**: under `as-shown` at both parts 008677, 247224, 316267, 421197, It's all about connecting,
UMTRI-2008-80, drafts_David-Cyberstructure, issue12295, issue14438, issue15262, issue4934,
non-embedded-NuptialScript, sample_fonts_solidconvertor and webCapture; under `no-data-loss` 12 of
them at each part. R09 goes from 44 to 26, the other 4 leaving for the rows they already stood in.
No `as-if-printed` output moves.

**Pixels** (CPU, 72 dpi, pages 1–4, `/OutputIntents` renamed): every freed `as-shown` output at 4 is
within 4 levels of its source; the unfitted face differs by up to 255. Two pages looked at.
**Tests.** Eight charstring tests and four face tests (nine plants run, each failing its test), and
`machine_face.rs`'s `a_machines_cff_face_fitted_horizontally_is_embedded_with_its_fitting`, which
fails with the CFF writer refused.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-font and pdf-transform; nextest of
pdf-font, pdf-transform and pdf-archive 0 (1848); `cargo test -p conformance` 0 (471); censuses
against 1984: shipped 28 NEW, 0 CHANGED, 0 GONE; stating 24 NEW (`no-data-loss`, 12 at each part),
0 CHANGED, 0 GONE; veraPDF 1.30.3 passes the 52 new census outputs and all 167 outputs of the 18
documents under every profile; tier 2 (`corpus`, `raster_golden`) fails at HEAD and after alike,
the same pages; fuzz `cargo check --bins` 0; `state.sh docs` exit 0, after the hand-off.
