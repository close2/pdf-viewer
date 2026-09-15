# 1091 — A place the output will not have

Date: 2026-09-15. Branch: `batch-1086-1091`, worktree `/home/AI/pdf-viewer-rounds`, five siblings. ADR: [1105](../adr/1105-a-place-the-output-will-not-have.md).

## The census, re-run, and why the colour row was not it
`tests/archive_corpus.rs` re-ranked: at PDF/A-2b `metadata/extension-schema-container-fields` 15
(ADR 1087's argument), `graphics/device-rgb-needs-a-default-or-an-rgb-output-intent` 12, then
`annotations/appearance-dictionary-present` 10. **The colour row is `WRONG_FAMILY` and the sweep's
`profile: None` is not why**, which had to be run rather than assumed: all twelve witnesses already
state a PDF/A output intent over a CMYK or grey profile and ISO 19005-2 section 6.2.3 requires every
entry of one array to name the same object, so `--output-intent-profile data/icc/sRGB2014.icc`
converts none of them.

## The row taken, and the two faults wearing its name
Reading the ten witnesses shaped the answer. Three are `Movie`, `Screen` and `3D` — subtypes
ISO 19005-2 section 6.3.1 strikes and this converter *removes* (ADR 1099) — and the preparation
refused the document because it could not construct artwork for an annotation it was about to
delete. Section 6.3.3 binds the annotations a **conforming file** holds, so that refuses over a place
the output does not have. `prepare_appearances` now takes the removal's population and skips it;
where every failing place was skipped the row's decision is the removal's own
`Authorised { ForbiddenAnnotation, ForbiddenAnnotationRemoved }`. No new remedy, nothing constructed.

Six are subtypes the part admits whose clause states no artwork and keep ADR 0816's refusal — **but
nine of the ten were told the wrong reason**: `for_annotation` owes something both for a construction
that drew and fell short and for one that drew nothing, and every witness is the second, so
`Written::drawn` splits them. `Screen` also had no arm in `appearance::construct` — the viewer
answers it earlier, the converter does not — so it met a catch-all about an annotation stating no
`/Subtype`, which §12.5.6.18 answers outright and the arm now quotes.

## Calibration, witnesses, and what moved
Trap 13 both levels. Planted back (`if false &&`),
`an_annotation_the_removal_takes_away_is_not_owed_an_appearance` fails and the other five pass; its
control `..._leaves_behind_still_owes_the_appearance_it_cannot_have` puts an admitted
`Stamp` beside the `Screen` and still refuses with everything authorised, and the sentence split is
two tests asserting mutually exclusive substrings. Through `quorra-transform archive --authorise
forbidden-annotation`, re-validated by stage three, `PDF_A-2b/…6-3-3-t01-fail-r.pdf` and `-fail-t.pdf`
**conform** over 127 requirements and `PDF_A-4/…-fail-q.pdf` over 118; no mark is drawn ADR 1099 did
not already draw. PDF/A-2b 447 → **450** converted, 163 → **160** refused, the row 10 → **7**;
PDF/A-4 176 → **177**, 183 → **182**, the row 7 → **6**; 2u, 2a, 4f, 4e unchanged. **Every default
run is unchanged at every target.**
