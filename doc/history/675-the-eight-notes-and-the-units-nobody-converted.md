# 675 — The eight notes, and the units nobody converted

672's sixth criterion pointed at the population 672 named. Parallel round, worktree `r675`, branch
`round-675`. **No pixel moves and no list changes**: what changed is six group notes, four ledger
rows and two paragraphs of trap 9. ADR 0499 has the argument.

## The briefing was right, and the population is not one population

The premises checked out against `doc/history/672-*.md` and the arrays in `oracle.rs`: the middle
bucket is **eight groups and twenty-eight pages**, the sixth criterion says what the briefing said
it says, and 672 did tell the next round not to invent a seventh. Nothing to correct there.

What measurement changed is 672's assumption that the eight are one bucket. **Six were taken, and
they came out in three different places:**

| | groups |
|---|---|
| the note's mechanism accounts for the failing bound once its own figures are put in the gate's units | `SUBPIXEL_IMAGE`, `SHARED_JBIG2_DECODER` (4 of 7 pages), `REFERENCES_DREW_NOTHING` |
| it accounts for it, but only an ablation could show that | `LINK_BORDER`, `REFERENCE_GLYPH_WIDTHS` |
| it accounts for **none** of it, and another clause owns it | `NEGATIVE_LINE_WIDTH` |

`SUBSTITUTED_FONT` (8) and `DEVICE_CMYK_CONVERSION` (5) were not taken and stay owed.

## The finding that travels

**Four of the six were answerable with arithmetic nobody had written down.** `raster_compare`
divides by width × height × **four** channels and sums the absolute difference over all four, so a
mark ours paints and a reference does not costs `Δink × 255 × 3 ÷ (w × h × 4)`; a coloured stroke
costs `perimeter × 510 ÷ (w × h × 4)`; and a *differing fraction* counts channels, not pixels. Two
notes had that factor wrong, and two more described `rank_the_contradicted`'s output as levels of
255 when `Distance::of` prints a ratio against the page's own bounds.

## Group by group

- **`SUBPIXEL_IMAGE`** fails on the differing fraction alone, by 1.16 points. Its row of coverage is
  180 columns × 3 channels ÷ 40 000 = **1.35 points**; ablating the inline image measures **1.3575**
  and the page then agrees. Belongs in the top bucket.
- **`SHARED_JBIG2_DECODER`**: all seven of our rasters are byte-identical, and ours against a
  synthetic white sheet is mean 13.12, worst tile 144.56, differing 5.15%, ssim 0.8990 — the whole
  verdict line the gate prints for the four pages whose voting pair decoded nothing. On the other
  three the ink table stops being an identity, by 1.4×, 1.7× and 4.9×, because the ink is displaced.
- **`REFERENCES_DREW_NOTHING`**: the references are constant white, so the failing mean *is* our
  ink — 12.718 against a printed 12.72, 13.672 against 13.67.
- **`LINK_BORDER`**: restating `/Border [0 0 1]` as `/Border [0 0 0]` takes all three pages inside
  every bound; the three references that construct no link appearance do not move by a digit. The
  note's closed form was 6.97 and is 5.23, which `poppler` hits exactly.
- **`REFERENCE_GLYPH_WIDTHS`**: restating the `/W` as `/DW 719` sends `poppler` and `mupdf` onto our
  render while `ghostscript`, which already read the array, does not move at all.
- **`NEGATIVE_LINE_WIDTH`** is the finding. Its ink ladder converts into the gate's mean **exactly**
  (0.6366 predicted, 0.6366 printed) and the mean is a bound the page *meets*. Restating `-0.1 w` as
  the `0` §8.4.1's clip produces leaves `mupdf` — the reference the verdict is taken from — byte for
  byte where it was. The clip owns none of the failing worst tile and none of the failing structural
  similarity; §8.4.3.2's one-device-pixel `shall` owns both.

## Measured

Every §2 gate, whole (fifth round), all green and no count moved: oracle 908 / 65 / 786, corpus,
quorra, both censuses, text, dates, xmp, jpeg2000, fixed documents, conformance, fuzz check, fmt and
`RUSTFLAGS="-D warnings"` clippy. §5's binaries rebuilt and installed. Sweeps: `--bin overtaken`
clean on all six rewritten notes, `--bin quoted` clean on five of six and unchanged on the sixth
(`LINK_BORDER`'s superseded `mean 8.10`, which narrates its own correction).

## Changed

- `oracle.rs` — six notes.
- `doc/conformance/ledger.toml` — §8.4.3.2, §9.7.4.3, §10.7.4 and §12.5.4.
- Trap 9 — two entries: the constant-raster identity, and the unit rule.
- ADR 0499.

## Owed

- **The two groups left**, 13 pages. `SUBSTITUTED_FONT` carries a related defect this round found
  and did not act on: its `bug847420.pdf` paragraph opens "the head of the contradicted list ranked
  in *levels* — 8.65 of 255", and that ranking is in bounds — but whether 8.65 was a hand
  measurement or a misread ranking cannot be settled without measuring the page.
- **A voting reference whose raster is constant contributes nothing to a verdict**, and the gate
  lets it vote. A condition that refused it would be the honest instrument and would move pages
  between four lists at once; trap 11 makes that its own decision, not a corollary of this one.
- Unchanged from 668, 489 and 672: nothing links a group's note to the gate figures it quotes except
  `--bin quoted`, nothing links one to another project's source, and nothing links one to the
  *units* the gate prints in.
