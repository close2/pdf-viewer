# 1045 — A mask group without `/Resources` ran against an empty dictionary

Date: 2026-09-14. Corpus slot of `batch-1044-1049`. ADR 1060 (item 2). Touched `content/transparency.rs`, `tests/soft_masks.rs`, four ledger rows.

## Item 1 — `GHOSTSCRIPT-691218-1.pdf` page 1, the standing miss round 1039 named
Rendered here and by `mupdf`, `poppler`, `ghostscript` and `hayro` at 72 dpi and looked at (trap
1): the references draw the dealership photograph, the "SALES EVENT" banner, the CAMRY and SONATA
headlines and every price figure; ours drew none of them, and a black bar where the banner is.
`open_one` said why: every `gs`, `Do`, `Tf` and `scn` inside the page's luminosity-mask groups
reported a name nobody defined. The file is Ghostscript 8.71's and every form XObject on it omits
`/Resources`; the page's dictionary defines every name. **The clause is
§7.8.3's fourth way**: "PDF files written obeying earlier versions of PDF may have omitted the
Resources entry in all form XObjects and Type 3 fonts used on a page. All resources that are
referenced from those forms and fonts shall be inherited from the resource dictionary of the page
on which they are used." Table 142 makes `/G` a transparency group XObject. **The case is ours —
we misread it, by omission**: a form, an appearance stream and a Type 3 glyph description without
the entry are each read against the page's dictionary, and `build_soft_mask` read the mask's group
against `Dictionary::default()`. The file breaks a writer's `shall` (the third bullet), which is
what the reader's rule exists for.
**Fix**: the group falls back to `self.page_resources` — the page's, not the stream that invoked
it, which is round 1044's reading of the same bullet (ADR 1059) and is why this site takes no new
parameter. The page reports nothing now and matches the references. Calibrated (trap 13):
`a_mask_group_stating_no_resources_is_read_against_the_pages_dictionary` puts the `gs` inside a
form whose own `/Resources` names the `/ExtGState` and no `/XObject`, so the two readings differ
there; with the empty dictionary planted back it fails on `/Fm is not in /XObject`, and the page
loses its photograph and every price figure again.

## Item 2 — the one-pixel subpath of `issue6413.pdf` (round 1042)
Decided as a documented choice, and one decision rather than two: §10.7.4 defines the clip as "the
set of pixels that would be included by a fill operation", the fill of a degenerate subpath is the
departure §8.5.3.3.1's row already records (`collapsed.rs`, ADR 0154), so the clip follows it.
Re-measured at one pixel per unit: point clip, point `re f` and `m h f` all mark 0 pixels.
**Found beside it (trap 40)**: `5 20.5 30 0 re W n` over a full-page fill admits 0 where
`5 20.5 30 0 re f` paints 30 — no hedge in the clause, owed in the backends, on §10.7.4's row.

## Gates
`raster_golden` held 974, moved 0 (exit 0); `pdf-model --test corpus` 974 documents, 61
incomplete, 0 slow (exit 0, after one failure at load 47 that reproduced at 11 s alone against a
30 s bound — trap 36); oracle 3 passed (exit 0); workspace nextest 4688 passed, 3 failed, all
siblings' (`submission.rs`, `pdf-archive editions`). Conformance's ledger test passes.
