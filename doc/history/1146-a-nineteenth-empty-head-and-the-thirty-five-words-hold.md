# 1146 — A nineteenth empty head, and the thirty-five out-of-bounds words are all box geometry

2026-09-16. Files: this record only. No code edit owed; no ADR — no reading was decided, a
population was re-confirmed. Every `git status` path outside this file is a sibling's: 1142's
extraction of a `pdf-colour` crate (`colour`/`function`/`icc`/`mesh`/`shading`/`transfer` moved out
of `pdf-model`) was mid-flight and broke the workspace build for one window (`Stated` needing
cross-crate visibility); it cleared on its own, and the stale `gates/pdf-sandbox-worker` its rehash
left was rebuilt (`-p pdf-sandbox --bins`) — shared gate infra.

**Oracle head empty a nineteenth round** — `1962 pages in 59.0s`, `agrees 1012, contradicted 46,
ambiguous 835` (807 complete), the "ambiguous, undiagnosed, and furthest from the nearest reference"
line printed with nothing under it. So the contract shifted to the other robustness instrument:
`text_extraction`'s 35 words out of bounds, re-examined for the first time since round 1103.

## The 35 words, named by cause (`PDFVIEWER_SELECTION_DETAIL`), all nine documents

The verdict judges only *matched* words, so every one of the 35 is a word this tree extracted
correctly; the disagreement is where its box sits, never whether the word was read. **Zero real
extraction misses.** Five classes, each a reference convention or substitution, not a bound this tree
owes (principle 5):
- **Substitute/absent-program advances growing along the word** — `TrueType_without_cmap` (4.51),
  `issue18099_reduced` (1.02), `issue2391-2` (12.58): each repairs a missing program (§9.5 NOTE 5).
- **Box convention constant in word length, far edge 3× the near** — `issue20232`'s 15 (`инв.`
  3.60/1.20, `Копировал` 1.20/3.60, `1:1` 5.18/1.72): no `/BaseFont`, §9.5 NOTE 5.
- **Vertical writing box convention** (§9.7.4.3 vs an axis-aligned quad) — `vertical.pdf`
  (9.21/9.21), `issue11555` (20.27/44.24, centre 0.67, unembedded `KozMinPro`).
- **Angle the frame cannot follow** — `issue1905` (5 oblique labels), `bug1771477` (sheared `des`
  1.91); and **`/Rotate 90`, nothing embedded** — `issue14497` (`165`, `320`).

The one drift from 1103 is `issue20232`'s worst horizontal delta, 3.60 → 5.18 pt on `1:1`; the
mechanism is unchanged (far edge 3× the near on all fifteen), so it stays evidence, no ratchet moved.

## State of robustness, the 8+23, gates (under the lock, one at a time)

Oracle: 1012 agree / 46 contradicted / 835 ambiguous of 1962 (also 3 our-geometry, 2 reference-
geometry, 46 not-comparable, 18 no-render), head undiagnosed empty. `render-raster`: 943 agree / 8
differ / 7 refused / 16 not comparable of 958 — **8+23 unchanged**. `text_extraction`: overall 99.3%
(24608/24787), geometry verdict 99.69% (11096/11131), 494/503 docs, ratchets 503/503 and 8266/8266.
`oracle`, `text_extraction`, `render-raster --test corpus`, `raster_golden` (held 974, moved 0),
`selection_census` all exit 0; `tools/batch.sh check` clean. No code edit; no ratchet moved.
