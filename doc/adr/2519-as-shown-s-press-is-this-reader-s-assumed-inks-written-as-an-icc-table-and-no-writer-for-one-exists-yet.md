# 2519 — `as-shown`'s press is this reader's assumed inks written as an ICC table, and no writer for one exists yet

Status: deferred, owed to a writer in `pdf-colour`. Session 1842, the archive slot of batch one
hundred and twenty-five. Context: `doc/questions/A694` item 2 (b), ADRs 2114 section 4, 2130 section
3, 2490 (row R05 (b)), 2518, 1978 and 1990; `doc/pdfa-remedies.md`'s row for
`press-assumed`.

## What was asked

A694 answered "yes to a and b" for R05. (a) is ADR 2518. (b) is the route `as-shown` would take at
part 4's `DeviceCMYK` row: write the colours this reader's window drew `DeviceCMYK` in — the sixteen
corners of `pdf_colour`'s `CMYK_CORNERS`, interpolated multilinearly between corners in
gamma-encoded sRGB — as an ICC CMYK profile, and name that profile as the `/DefaultCMYK`. The
statement would then be true of the page the profile's reader was looking at, so it would cost
nothing and be a `Decision::Stated`, not a loss: ADR 2130 section 3 refused the *shipped* press
because "a shipped `DefaultCMYK` would be a statement this reader's own page contradicts", and a
table of the inks the page was drawn from contradicts nothing. The contract's condition was that the
route be taken only if an ICC table writer exists or fits the slot.

## What exists

- **A reader.** `pdf_colour::icc` parses `A2B0`, `A2B1` and `A2B2` in `mft1`, `mft2` and `mAB `
  encodings.
- **Fixtures, and no writer.** `mft2_tag` and `lut16_profile` in `pdf-colour`'s own tests, and the
  copy in `pdf-model`'s `soft_mask.rs` tests, build a two-point-grid profile for a test and nothing
  else: one tag, no `desc`, no `cprt`, no white point.
- **A tag editor.** `pdf-transform`'s `archive/icc.rs` adds a `desc` and a `cprt` to a profile that
  lacks them (ADR 1990); it moves a table and writes none.

So there is no writer, and the slot does not build one.

## What the writer needs

1. **The profile.** A 128-byte header (class `scnr` or `prtr`, space `CMYK`, a PCS, a version the
   part admits), the tag table, `desc`, `cprt`, `wtpt` and an `A2B0` lookup table. An input-class
   profile needs no `B2A0`, which is why the class matters: a `prtr` profile asks for the inverse
   tables too, and the assumed press has an inverse only as a search (`Press::table`).
2. **The grid.** The model interpolates in gamma-encoded sRGB and an ICC table interpolates in the
   PCS, so a two-point grid, which holds the sixteen corners exactly, does not reproduce the
   interior. A finer grid does, to the table's quantisation: nine points an axis is 6 561 nodes and
   39 kB. The bound to hold it to is a measurement the writer's round makes — the largest 8-bit
   difference between the profile route and `cmyk()` over the whole input cube — and not a number
   to assume here.
3. **Where it lives.** In `pdf-colour`, which owns the corners and the sRGB-to-PCS transform; the
   archive consumes the bytes the way it consumes `prepare::CMYK` today.
4. **ISO 19005's profile rules**, cited by section: the edition and tags a profile of `ICCBased`
   carries (section 6.2.4.2, and ADR 1990's reading of its `desc` and `cprt`), and a `N` of four
   (ISO 32000-2 §8.6.5.5). veraPDF is planted with the writer's output before the route is declared
   freed, and the output is read back through every reader of the file, `pdf_colour::icc`'s
   included, so the profile route is checked to reproduce the window's pixels (trap 284).
5. **The overprint clause.** A `DefaultCMYK` over any ICC profile makes `DeviceCMYK` marks
   `ICCBased` CMYK ones, and ISO 19005 section 6.2.4.2 forbids overprint mode 1 for them. ADR 2518
   section 5 records that the assumed-press route refuses such a file; this route meets the same
   clause, and ADR 1978's `overprint-mode` is the loss that answers it.

## What stays as it is

`as-shown` stops at the row, with the sentence that names the flag (ADR 2130) and now the loss
`press-assumed`; an operator's `--output-intent-profile` still answers it. Nothing is lost by the
deferral that a refusal did not already lose.
