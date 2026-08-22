# 668 — The profile that exists on no disk

One contradicted group, taken apart. Parallel round, worktree `r668`, branch `round-668`.
**No pixel moves**: what changed is a group note whose verdict rested on an agreement it called a
coincidence, two ledger rows, and two traps. ADR 0494 has the argument and the tables.

## Which group, and why — the fifth criterion

662's is spent. A contradicted verdict makes two claims, and 662 audited only one of them; this
round asked the other:

> **The verdict says the agreement which outvotes us is evidence (ADR 0005). For each group, does
> the note name a mechanism for the two voting references agreeing — and is it verified rather than
> asserted?**

Over all fourteen non-empty lists: ten name one and check it against a binary, a source, a data
file, a log or a ladder; three name one and infer it from the picture; and
`CONTRADICTED_CALRGB_TO_SCREEN` names none, writing that the pair "happen to agree to 4.41%".

## What the five pages are

Four of `calrgb.pdf` and one of `issue9940.pdf`. The four are the only pages of that document's
seventeen whose `/Gamma`, `/Matrix` and `/WhitePoint` are all the identity or `[1 1 1]` — so the
space *is* XYZ and §8.6.5.3 has nothing left to decide. Against a closed form owing nothing to any
renderer (Bradford onto sRGB's white, then IEC 61966-2-1), `poppler` is 0.013 of 255 and ours 0.025
over eighty swatches, with `hayro` 2.15, `ghostscript` 4.30 and `mupdf` 4.84.

`libgs` carries `gsicc_create_from_cal` among its internal names; `libmupdf` exports
`fz_new_icc_data_from_cal` and defines 437 `lcms2mt_*` symbols; `poppler` has its own
`GfxCalRGBColorSpace`. The pair that outvotes us
**synthesises an ICC profile from Table 63** and hands it to Little CMS.

`gs -sDEVICE=pdfwrite` writes that profile out — 585 bytes, a `scnr` profile whose colorants are the
*diagonal* of an adaptation that is not diagonal and sum 4.4% away from its own `wtpt`. Rendering
the rewritten page, this tree reproduces `ghostscript`'s rendering of the dictionary to **0.07 of
255** where our own path is 4.15 from it; handed the file, `ghostscript` moves 0.03 and `mupdf` 0.83
while ours moves 4.17 and `poppler`'s 4.24. One file is the whole verdict, and it is in neither
binary and on no disk until a renderer makes it — trap 9's eighth mechanism, and the first for which
`objdump`, the embedded-profile scan and the `desc` tag all return empty.

And the agreement is thinnest where the difference is largest: on the 41 swatches where the camps
part the voting pair is a mean 3.78 and a maximum 16 levels apart, and at the swatch carrying the
page's biggest difference we are nearer `ghostscript` (15) than `mupdf` is (16).

## The clause was one sentence away

The note argued from §10.3.1's sentence putting the destination beyond the document's scope. The
*next* sentence of the same subclause is a `shall` requiring the conversion itself to follow the
appropriate ICC specification. And §8.6.5.3's sentence names `WhitePoint` **and** `BlackPoint`; this
group quoted it under `BlackPoint`, while `/WhitePoint` is the whole of what separates the camps.
Fourth round running in which the deciding clause sat in a different row than the group cited.

## Changed

- `oracle.rs` — `CONTRADICTED_CALRGB_TO_SCREEN`'s note rewritten; the title names the mechanism.
- `doc/conformance/ledger.toml` — §10.3.1 gains the corpus witness for its `shall`, §8.6.5.3 the
  record that its quoted sentence names two entries and the row read one.
- Trap 9 — an eighth mechanism; trap 12 — the population a bound is computed over.
- Two stale figures in the note corrected by re-running the gate's own arithmetic.

## Owed

- A criterion for the next round; this one's is spent.
- `mupdf`'s synthesised profile is inferred (0.83 of 255 and a symbol), not obtained.
- Nobody's *shadow* behaviour is explained: handed one profile the four still spread 19 to 54 at the
  darkest swatch.
