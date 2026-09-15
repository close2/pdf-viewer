# 1117 — A fourteenth empty head, and both largest groups hold through two oracle moves

2026-09-16. Files: this record only. No code edit is owed; no ADR. Every path in `git status`
(`navigation.rs`, `ledger.toml`, `state-of-play.md`) is a sibling's.

**The oracle's undiagnosed head is empty a fourteenth round** — `1962 pages in 54.6s`,
`agrees 1012, contradicted 46, ambiguous 835`, "ambiguous, undiagnosed, and furthest from the
nearest reference:" printed with nothing under it — so the contract was the 46 contradicted pages,
re-read for a lapsed cause since ADR 1102/1107 moved the oracle (round 1103 last read them).

## The 46 by group, and the two largest re-read against every member on this build

Fourteen groups: `SUBSTITUTED_FONT` 12, `GLYPH_EDGES` 10, `CALRGB_TO_SCREEN` 5,
`DEVICE_CMYK_CONVERSION` 5, `SHARED_JBIG2_DECODER` 3, `LINK_BORDER` 3, and eight singletons. The
test asserts this partition exact, so every page is still `contradicted`, sizes unchanged from 1103. **No page's cause lapsed.**

- **`GLYPH_EDGES`, all ten.** Each is convicted by `poppler and mupdf` — the voting pair that
  shares `libfreetype.so.6` (trap 9), `ghostscript` outside — with mean ≤ 2.96, worst tile ≤ 12.09,
  ssim ≥ 0.8965, only the differing fraction over bound (5.15%–10.26%). Ink is conserved and the
  named departure is closed (ADR 1082); what convicts is the references' shared hinting, not ours.
  ADR 1102 (a stroke-width substitution) and 1107 (§11.4.4 group colour under a mode) touch neither
  a fill's edge nor these pages, and the metrics confirm no move. `unencrypted.pdf p2` keeps its
  second, image-box-rounding half (ssim 0.8965, §10.7.4). Claim holds, not a misreading.
- **`SUBSTITUTED_FONT`, all twelve.** Differences confined to glyph tiles of a different substituted
  face: `PDFBOX-2984-rotations` p1–4 and `ICC.1-2022-05` contradicted on worst tile alone (mean
  ≤ 2.63, ssim ≥ 0.9658); `issue15716` stays heavy (mean 13.96) with `hayro` beside us at 188.87 v
  188.73. §9.5 NOTE 5: "the results depend on the availability of fonts in the PDF processor's
  environment" (verbatim in `doc/md/`). Not our departure; claim holds.

## The 8 + 23 re-confirmed, and §43–§48 unchanged on our side

`render-raster --test corpus`: `958 compared, 943 agree, 8 differ, 7 refused, 16 not comparable` —
every page and reason identical to round 1110 (8 differ under §45/§46/§47/§48/§24c/§10.7.4; 23 =
4 §43 refusals, 2 budgets, 1 §44 capability, 10 password, 1 null `/Encrypt`, 5 no first page).

**No §43–§48 ask is closable in the adapter.** §43's four need `raster` group-conversion vocabulary
(no group-level readback here); §44's `issue19517` refusal is correct and tiling is `raster`'s call;
§45/§46 live in `raster`'s converter and reduce. §47/§48's ordinary-image path hands
`ImageFilter::Auto` (`scene.rs`) on purpose — reading `is_smoothed` there would bake the view into
the command and regress the zoom-invariant scene (ADR 0702). No ratchet moved; no fix owed.
