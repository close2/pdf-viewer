# 1169 — The actions family, and the digit the standard already supplied

`doc/todo/66`'s Actions family, and the owner's revisit note on ADR 0947 decision 3.

## What was found, against `doc/adr_revisit/0947-*`

- The refusal's wording, §7.3.4.3's completion sentence and ADR 1120's provenance test all held.
- **The ADR 1026 claim did not hold as stated.** That ADR re-read whether a resource carve-out
  *narrows* the two hexadecimal rows; the refusal itself had not been re-read since ADR 0947.
- **The line was already drawn where the note wanted it.** `doc/pdf-a-conversion-limits.md`
  section 5.2 records the owner's `A50` admitting the `F`-to-`f` respelling because it cannot
  change a mark; §7.3.4.3 states the odd-digit string's value outright, which is stronger.
- **Only two `WRITER_EMITS` rows can fail on a page**, on opposite sides: the standard states the
  value a completed digit writes and states none for a byte that is neither a digit nor white
  space. A second defect fell out — the same failure at a resource-reached stream object took
  `Rewrite::WholeFileRewritten`, which touches no content bytes: a promise that never arrived.

## What was built

- `crates/pdf-transform/src/archive/actions.rs`, new: nine action rows under one
  `Loss::InteractiveBehaviour`, as three counted-apart rewrites. A removed action's `/Next` subtree
  is promoted into its place, in §12.6.2 NOTE 1's order; three shapes refuse by name. ADR 1175.
- `crates/pdf-archive/src/table/interaction.rs`: `action_sites` and four predicates, so the ISO
  19005 reading stays in the validator and the converter walks what it read.
- `crates/pdf-transform/src/archive/hexadecimal.rs`, new: the final digit written into the
  producer's own content stream, one stream object at a time — §7.7.3.3's Table 31 bounds a
  `/Contents` seam to a token boundary — with §8.9.7's inline image data stepped over. ADR 1176.

## Measured

`archive_corpus` under the heavy-walk lock, before and after, as printed:

- **PDF/A-2b**: 450 → **470** failing documents converted, 160 → **140** refused; by default
  139 → **140** converted.
- **PDF/A-4**: 188 → **206** converted, 171 → **153** refused; by default 161 → **162**.
- All four action rows left both top-ten lists; `graphics/separations-of-one-name-agree` (4 at
  each), `file-structure/no-external-stream-data` (3) and
  `fonts/cid-system-info-agrees-with-the-cmap` (3) were standing behind them.
