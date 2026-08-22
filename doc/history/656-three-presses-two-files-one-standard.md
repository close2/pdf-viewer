# 656 — Three presses, two files, one standard

One contradicted group, taken apart. Parallel round, worktree `r656`, branch `round-656`.
**No pixel moves**; what changed is a group note that described three of its five members from their
dictionaries, a comment citing a NOTE one subclause off, and two quotations of a sentence Errata
Collection 3 struck. ADR 0484 has the argument and the tables.

## Which group, and why

651 chose by `git blame` and then by the note that described a picture with no number. That group is
done, so this round asked the same question one level down: **how many of its own members does a
group's note measure?** Thirteen of the fourteen non-empty groups answer *all of them* — a row per
page in `CONTRADICTED_SHARED_JBIG2_DECODER`'s ink table, five accounted cohorts in
`CONTRADICTED_GLYPH_EDGES`' twenty-six, a `/BaseFont` and a cap-row count for each of
`CONTRADICTED_SUBSTITUTED_FONT`'s eight.

`CONTRADICTED_DEVICE_CMYK_CONVERSION` is the exception. Two of its five pages carried the whole
argument; the other three appear once, in a sentence about what their *dictionaries* contain — trap
9's fourth shape, on the clause family ADR 0456 says it already cost six rounds.

## What the three pages are

All three admit a closed form. `function_based_shading_cmyk.pdf` page 1 is three §8.7.4.5.2 shadings
over a §7.10.2 sampled function of `/Size [2 2]` — `C = u(1−v)`, `M = (1−u)v`, `Y = uv`,
`K = 64uv/255` — plus the same construction through a `/Separation`. Page 2 is that square six times
at six integer offsets. `type4psfunc.pdf` is one axial shading whose 292-byte §7.10.5 tint transform
hand-evaluates to `(0, m, y, 0)`, so the ink is `(0, 0.2(1−t), 0.8(1−t), 0)`.

## Measured, not looked at

The first statement has no renderer in it: ADR 0009's sixteen ink corners, interpolated over the
closed-form CMYK, are **within one level of 255 of our own raster at all 125 sample points**. Ours is
that arithmetic, and it is what validates the forms.

Sampled against those forms, with both candidate press profiles run through **our own** A2B
evaluator, worst channel difference in levels of 255:

| | ours | `poppler` | `mupdf` | `ghostscript` | `hayro` |
|---|---|---|---|---|---|
| Artifex SWOP profile | 48 | 51 | 8 | 8 | 8 |
| CGATS001Compat micro profile | 48 | 51 | 5 | 4 | 4 |
| ours | — | 4 | 48 | 48 | 48 |
| `mupdf` | 48 | 51 | — | 6 | 4 |

Two camps at every point of every page, and both profiles inside the second. So nothing here is
§8.7.4.5.2's interpolation, §7.10.2's `/Order`, §7.10.5's operators or either tint transform: **the
group's name is right about all five members**, and had been right about three of them by assumption.

## The agreement is two files, and one of them is nobody's neighbour

`/usr/share/ghostscript/iccprofiles/default_cmyk.icc` is 187 484 bytes, `desc` *Artifex CMYK SWOP
Profile*, `md5 fd199526f0a7e0bceb294a777cd84252`. `libgs.so` embeds no profile and reads it off the
disk; `libmupdf.so` embeds **the same bytes at the same digest** at offset 3 360 896. That turns "one
profile seen twice" from an inference about outputs into byte identity.

**`hayro` is the one that does not fit.** It sits with that pair and shares nothing with it:
`objdump -p` on `pdfref-hayro` lists `libgcc_s`, `libm`, `libc` and no colour library, and it carries
its own `CGATS001Compat-v2-micro.icc` — 8 464 bytes, `desc` `uCMY`, `cprt` `CC0`, one `A2B0` tag.
Two independently authored files, and our evaluator on either predicts all three renderers. What they
share is the **press**: Artifex's `desc` says SWOP and CGATS TR 001 is the data SWOP publishes. That
is trap 9's sixth mechanism — not shared code, data, default, wider code or coincidence, but a shared
published *standard*, which no dependency graph shows and no digest comparison finds.

Page 2 also asks each renderer about itself, six identical squares being the document's own
invariant: four return six squares differing in zero channels, `poppler` paints the top row on three
of them and leaves it white on the other three — 600 pixels at 255 levels, one row.

## The clause

§10.3.2's NOTE licenses all four assumptions — "[e]stablishing a CIE-based **source** colour space
can happen … by assumptions made by the PDF processor software". `CMYK_CORNERS` cited §10.3.1's NOTE,
which says the same of the **destination**; the two are one subclause apart, differ by one word, and
both carry that phrase.

`spec-errata emit` over the family, run before writing, printed Issue #181 under §10.4.1's heading.
It is §10.3.1's: the StrikeOut's `/Rect [83.128 333.629 238.273 346.049]` is over the words
`pdftotext -bbox` puts at (86.42, 494.66)–(237.24, 507.60), the clause's last line, which this tree
quoted in two places. `check` could not have found it — `MIN_WORDS` is 4 and the struck run is two
tokens, so a quotation can sit on retired text and pass the gate built to find that.

## Changed

- `oracle.rs` — the group note rewritten around the measurements; "all three" corrected to four.
- `doc/traps/oracle-and-references.md` — trap 9's shared-data bullet gains the digest and the
  ICC-header scan; a new bullet for the shared standard.
- `colour.rs` — `CMYK_CORNERS` quotes §10.3.2's NOTE; `BRADFORD`'s quotation of the retired ICC
  citation is prose.
- Ledger §10.3.1 and §10.3.2; `doc/errata-read.md`'s row for #181 and what it says about `check`.

## Owed

- `poppler`'s dropped row, and the three press assumptions, are unreported upstream.
- Nothing asks a group whether its note has a number for every name it holds; the count that chose
  this group was done by hand.
