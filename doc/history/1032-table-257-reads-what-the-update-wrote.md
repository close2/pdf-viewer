# 1032 — Table 257 reads what the update wrote, and one file answers differently at each level

Ledger slot of batch 1032–1037, five siblings in one worktree. Theme: finishing what 1026
refused — §12.8.2.2's Table 257 applied to what §12.8.2.2.2 compares.

**1. The reading.** Level 1 needs no analysis ("any change to the document shall invalidate the
signature"); 2 and 3 permit *operations* — "filling in forms, instantiating page templates, and
signing", and those "as well as annotation creation, deletion, and modification". An object's
**type** names no operation: a widget is a field and an annotation at once, and level 2 permits
filling it in while forbidding annotating it. So the classifier reads which *entries* moved —
Table 226's `/V` with Table 166's `/AP`, `/AS` and `/M`. ADR 1049.

**2. What landed.** `Kind` says what a changed object was taken to be and `Verdict` what the level
says about it; `Ranking` puts every one in an exact bucket, and `Judgement` has three answers and
no fourth where something changed — permitted, not permitted, or not ranked. None says valid.

**3. Two things the standard settled.** §7.5.8's cross-reference stream is the update's own table:
§7.5.6 requires the section, §7.5.8 lets it be an object, and Table 257 disregards a DSS-only
update that has one. And the carve-out is a question about a whole update, "only" read as written —
one holding validation material **beside** other objects is unranked, not forbidden.

**4. The corpus answered three ways.** `prefilled_f1040.pdf`'s fifteen changed objects are **all**
classified, none refused: two widgets filled in, six whose `/Rect` also moved by a thousandth of a
unit, four appearance streams, three cross-reference streams. Level 1 says not permitted (12
objects), level 2 not permitted (9), level 3 within what is permitted (15). §7.3.3 decided the rest
— "Wherever a real number is expected, an integer may be used instead." — so one rectangle
differing only in written form is `RestatedUnchanged`.

**5. Still refused, by name.** Page-template instantiation has no classifier, so a document that
did it is refused rather than permitted; so is a replaced catalog at 2 and 3, and a `/P` outside
1–3. No transform parameter selects an object (§12.8.2.1's `shall`), nothing in the *program*
reports any of this (1026's debt, unchanged), and `xfa_filled_imm1344e.pdf` still refuses
comparison. **Rows:** §12.8.2, §12.8.2.1, §12.8.2.2, §12.8.2.2.2, §12.8.2.3, §12.8.2.4, `partial`.

**6. Gates.** Tier 1 and the whole of tier 2 (rule 2 — `pdf-signature` is under everything), all
green: `raster_golden` 974 held 0 moved, `launch_path` 26 banded 0 outside, `conformance` 7 passed,
`nextest` 4595 passed. The workspace clippy line is red only on **siblings'** in-flight files this
round did not touch (`viewer-confined/.../panels.rs`, `pdf-model/tests/silent_fonts.rs`);
`-p pdf-signature --all-targets` is clean with every lint on. Six new unit tests, each calibrated
by planting the defect it finds (trap 13). The corpus walks are the merge's.
