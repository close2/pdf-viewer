# 1123 — A fifteenth empty head, and the twelve differ in outline where the advance is the document's

2026-09-16. Files: this record only. No code edit owed; no ADR — the reading is §9.5 NOTE 5,
already at `CONTRADICTED_SUBSTITUTED_FONT`'s head. Every other path in `git status` (`fragments.rs`,
`xmp.rs`, `ledger.toml`, `object_metadata_census.rs`, the 1120/1122 records) is a sibling's.

**Oracle head empty a fifteenth round** — `1962 pages in 57.1s`, `agrees 1012, contradicted 46,
ambiguous 835`, the undiagnosed line printed with nothing under it. Sizes unchanged from 1117;
contract fell to `SUBSTITUTED_FONT`'s twelve.

## The twelve split: all outline (robustness stays), none position — and the reason is structural

Every one names a **non-embedded** font (`pdffonts`): `bug847420` `Arial,Italic`; `bug850854` and
`PDFBOX-2984-rotations` p1–4 `Helvetica`; `issue9243` `Helvetica-Bold`; `issue6069`/`issue6108`
`Arial`; `issue7580` `ArialMT`; `issue15716` `ZapfDingbats`; `ICC.1-2022-05`'s wordmark three Arial
faces. Absent program → §9.6.2.2 substitution; our Liberation/Arimo clones are §9.8.1-reasonable
(`bug847420` `/Flags 96` Italic|Nonsymbolic → sans italic; `issue7580` `/Flags 32` → sans).

**The advance is never ours to get wrong.** `bug847420` and `issue7580` carry the document's own
`/Widths` — honoured, so positions are the producer's. The other ten carry no `/Widths` and no
`FontDescriptor`: standard-14 fonts whose metrics are the built-in AFM (§9.6.2.3, compiled at
`data/standard-fonts`). Either way position is fixed by construction; a substitute changes only the
*outline* in the fixed advance box, which §9.5 NOTE 5 leaves to the processor's font environment.

**Measured, not inherited** (principle 5), 8× ours v `pdftoppm -cropbox -r576`: `bug847420` ink
spans columns 90–1509 v 98–1515 — 1419 v 1417, **0.14%**; `issue15716` (dingbats) 1490 v 1524 —
same AFM cells, different pictures inside (mean 13.96, 9.53% of pixels differ, `hayro` beside us).
Both re-derive ADR 0962. Every per-page failure is one localised measure over a small mean —
worst-tile-, differing-fraction- or ssim-only (`issue9243` 0.8912/3.10) — an edge, not a drift.
**No page leaves the group; nothing owed to 1118.**

## The 8 + 23 re-confirmed, and the gates

`render-raster --test corpus` (gates, `--ignored`): pass, exit 0 — `958 compared, 943 agree,
8 differ, 7 refused, 16 not comparable`, identical to 1117; the 8 are `issue21068`, `issue2177`,
`issue20232`, `issue269_2`, `issue19083`, `22060_A1_01_Plans`, `pr12564`, `issue15150`
(§45/§46/§47/§48/§24c/§10.7.4). `oracle`: pass, exit 0 — head empty, 1012/46/835. `raster_golden`:
pass, exit 0 — held 974, moved 0. This round changes no code and adds no unexpected file; its
record is within the 40-line budget. Any fmt or over-budget-record red in `batch.sh check` is a
sibling's mid-flight file, not this round's.
