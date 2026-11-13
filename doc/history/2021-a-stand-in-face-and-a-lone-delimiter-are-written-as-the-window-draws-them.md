# 2021 — A stand-in face and a lone delimiter are written as the window draws them

Session 2021, a single round, PDF/A lane's worktree: the leftovers rounds 2014 and 2016 named, largest
group first. ADRs 2858, 2859.

**The premise.** Half true. HEAD reproduced every census line of the eight documents, under all eight
profiles at both parts. `poppler-395-0-fuzzed`'s appearance row is its only refusal under the shipped
profiles; four profiles decline it as a source, and its signature refuses it under the stating ones.
`hesr13613`'s operator row is its only refusal at 2b; at 4, `fonts/cid-to-gid-map-present` stood behind it.

**Built.**
- ADR 2858 (`pdf-model` `appearance.rs`, `variable_text.rs`; `archive/prepare/annotations_and_attachments.rs`,
  `decision.rs`). ADR 0112's stand-in for a `/DA` font `/DR` does not define (§12.7.4.3) is written
  where it is the whole of what is owed, which a second layout with the stand-in unreported proves.
  It goes under the new loss `stand-in-face`, granted at both appearance rows by `as-if-printed` and
  `state-every-loss`. `as-shown` asks, since it stops at the face the stand-in needs.
- ADR 2859 (`archive/as_drawn.rs`, `fenced.rs`). A lone `>` or `)` (§7.2.3) is §7.8.2's error, at
  which the reader clears the operands. Where none stands before it, and the write then answers every
  operator, it is written as white space under `content-as-drawn`.
- Ledger 12.7.4.3 and 7.8.2 stay `implemented`, their notes, code and tests extended.

**Freed.** `poppler-395-0-fuzzed` under `as-if-printed` at 2b and 4; `hesr13613` at 2b under
`as-if-printed`, `as-shown` and `state-every-loss`. **Pixels** (CPU, 72 dpi, intent renamed): `hesr13613`
under `as-shown` at most 2 pixels a page by 1 level, under `as-if-printed` up to 20 348 (the fonts row's
`machine-face`, `substitute-stretch`); `poppler`'s `/Helv` note identical, its stand-in 468 (`machine-face`).

**Left.** `hesr13613` at 4 (one CIDFont's `/CIDToGIDMap`). The MediaBox `/UserUnit` bracket (ADR 0816's
fence). The Private Use `ActualText`, an entry the window never draws. `issue6931`'s transfer function
(ADR 1964). `issue5010`'s program name. ADR 2847 §3's widths in `d0`.

**Gates.**
- fmt 0; clippy `-D warnings --all-targets` (`pdf-transform`, `pdf-model`) 101 three times, then 0;
  nextest 0 (4169 with `pdf-model`, then 1532); six tests fail under six plants; `conformance` 101 (this
  record's length), then 0; `fuzz` `check --bins` 0; archive tier 0 and 0.
- Censuses against 2016 (`cmp.py`): shipped 4 NEW, stating 1 NEW (the freed files), 0 GONE. CHANGED
  32 and 30: `content-as-drawn`'s history sentence, and the offsets after it.
- veraPDF 1.30.3: 5 of 5 PASS. `rank.py --table`: under `as-if-printed` R03 15/6 to 14/5 and R16 2/2
  to 1/1, sections 2 and 9 replaced. `tools/state.sh --round 2021 docs` 0.
- Tier 2 at the merge: `corpus` 101, `raster_golden` 101 as HEAD's (the same pair, the same 23 by name).
