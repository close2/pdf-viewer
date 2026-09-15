# 1080 — A refused program keeps the document's metrics

Date: 2026-09-15. Branch: `batch-1080-1085`, worktree `/home/AI/pdf-viewer-rounds`, five siblings.
ADR: [1094](../adr/1094-a-refused-program-keeps-the-documents-metrics.md). Files: `pdf-font/src/loading.rs`,
`pdf-model/src/content/{font,text}.rs`, `tests/{refused_font_metrics.rs,text_extraction.rs}`, golden, ledger.

Round 1077's last undiagnosed geometry entry: `show_text` returned before §9.4.4's displacement was computed
for a font `pdf-font` refused, so the *next* font's glyphs began at the last `Td`.

## The refusal was about the program, and the width is not only in the program

§9.2.4: "[t]he width information for each glyph shall be stored both in the font dictionary and in the font
program itself", and its NOTE 2 names this case — the redundancy "enables a PDF processor to determine glyph
positioning without having to look inside the font program". §9.4.4 asks for the update unconditionally. So
`LoadedFont::metrics_only` builds a font from the dictionary alone:
`CodeMapping::MetricsOnly` over the composite `CMap` or a simple font's single-byte codes, Table 109's
`/Widths` and Table 120's `/MissingWidth` or §9.7.4.3's `/W`, `/DW`, `/W2` and `/DW2`.
`Interpreter::advance_through_a_refused_font` moves the pen and nothing else — no glyph, no readback, no quad;
the font marked no part of the page — and still counts the string as text the page could not draw, so both
reports stand and the page stays incomplete. It is page-scoped and **not** in the cache that outlives the
page, whose hits report nothing (trap 5).

## The numbers

Witness `issue6127.pdf` p1: `(réf.`'s opening parenthesis moved 162.1009 → **165.1167**, against `pdftotext`
165.117 and `mutool` 165.11672 — the file's own `/W` 250 for CID 3, so `t x` = (0.25 + `Tc` 0.0013) ×
12.0008 = 3.0158 pt. `raster_golden`: held 973, **moved 1** — that page, raster and list, its *report*
digest unchanged (trap 37), and looked at (trap 1): `qualité d'ayant droit" (réf.S3182).`, the run
translated and nothing else altered; no other of the 974 has a refused font before a loaded one on a
line. `text_extraction`: **11094 → 11096 of 11131**, fully-in-bounds documents 493 → 494,
`JUDGED_FLOOR` 503 and `CROSS_AXIS_FLOOR` 8266 unmoved, `issue6127.pdf` off `SELECTION_BELOW_FLOOR` (now
9) as the last entry whose mechanism was this tree's own. `pdf-model --test corpus`: every ratchet at its
ceiling. Oracle: exit 0, every by-name list held, that page *ambiguous (incomplete)* either side. Ledger:
§9.2.4, §9.4.4, §9.7.4.3 and §9.7.5.2 keep `implemented` and gain the construction, witness and test.

`refused_font_metrics.rs` holds both halves — the corpus witness, and a fixture for the simple case, which has
none to be had because a simple font with no program is substituted rather than refused; its control is trap 13's,
an arm stating no width putting the second font back at the `Td`. Tier 1 and `conformance` fail only on siblings'
in-flight `pdf-signature`, `pdf-archive` and `render-cpu`; scoped to this round's two crates all four lines are
clean, 1653 tests, no finding naming its files.
