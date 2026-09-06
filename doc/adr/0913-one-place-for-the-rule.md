# 0913 — One place for §7.3.3's reader-side answer, and the sixth call site

Session 936. Status: **accepted**. The code that follows from ADR 0912's reading. Completes the
consolidation ADR 0904 began and calls "most of its value".

## Context

ADR 0904's own account of what it was worth is worth quoting, because this ADR is the same
sentence one module further out:

> Had only `positive_integer` been changed, a real `/Width` on a `/Mask` would have read as 1062
> in one function and as 0 in the next — two readers of one file inside one crate, which is the
> failure the "one rule in one place" discipline exists to prevent.

It unified five call sites, all of them in `crates/pdf-model/src/image.rs`. There is a sixth, in
`crates/pdf-model/src/inline_image.rs`:

```rust
fn unfiltered_length(document: &Document, dict: &Dictionary) -> Option<usize> {
    let width = usize::try_from(document.get_key(dict, "Width").as_integer()?).ok()?;
    let height = usize::try_from(document.get_key(dict, "Height").as_integer()?).ok()?;
```

and it is the one that matters most in the world, because **both** of the two documents in 90 128
that write a Table 87 dimension as a real write it inside a `BI` (ADR 0912's census).

## What the sixth site decides

Not a pixel directly — where the image *ends*. §8.9.3 fixes the layout of unfiltered samples, so
`/W`, `/H`, `/BPC` and the colour space say exactly how many bytes stand between `ID` and `EI`, and
`data_extent` checks that arithmetic against the `EI` it predicts. Where the arithmetic answers
nothing, the extent falls back to `search_for_terminator` — the forward search that
`data_that_contains_ei_is_not_cut_short_by_it` exists to show is wrong, because sample data that
happens to spell a delimited ` EI ` stops it early, and interpretation then resumes *inside* the
remaining samples and executes whatever they spell. Usually nothing, and the rest of the content
stream is lost with no report at all.

So with a real `/W`, `crate::image` had a grid and `crate::inline_image` had none: the decoder drew
the image and the scanner guessed its end. Exactly ADR 0904's shape, one module over, and invisible
to that round because its witness is *filtered* — `/F [/A85 /Fl]` — and a filtered image's extent
comes from the filter's own end-of-data marker rather than from this arithmetic.

## Decision

**`crates/pdf-model/src/integer_entry.rs` is the one place §7.3.3's reader-side answer lives**, and
both `crate::image` and `crate::inline_image` read Table 87's dimensions through it. The module
comment carries ADR 0912's four families and the argument for each, so a future round that wants to
widen the tolerance finds the reasoning at the site rather than in an ADR it has to know exists.

Three things about the shape:

- **It is not `pdf_syntax::Object`.** An accessor beside `as_integer` and `as_number` is Q31's
  position 2 — one rule applied everywhere at once — and that is the owner's call, not a round's.
  A `pdf-model` module can be promoted the day the answer comes; the reverse would be a behaviour
  change nobody asked for.
- **It answers for `/Width` and `/Height` and nothing else**, which is a measurement rather than
  caution (ADR 0912's table). What is settled meanwhile is Q31's own recommendation, which this
  round takes as binding on itself: wherever the tolerance goes, it goes into *one* function for
  that entry rather than into each call site.
- **The rule is the nearest integer, ties away from zero**, which supersedes ADR 0904's truncating
  paragraph on the evidence in ADR 0912.

## The tests, and that each fails without its fix

Both were run against the defect before they were believed (trap 13).

`a_real_grid_still_predicts_where_the_data_ends` is
`data_that_contains_ei_is_not_cut_short_by_it` with `6` written `6.00`, so what it isolates is the
two periods. With `unfiltered_length` reading `as_integer` again it fails, and it fails *saying the
right thing*:

```
the fixture should draw completely: [Image { name: "<inline>: its samples stop at 1 bytes where
6x1 at 8 bits and 1 component(s) needs 6 …" }, Operator { operator: "EI" }]
```

— one byte of six, because the search stopped at the ` EI ` inside the samples, and an `EI`
operator reported afterwards because interpretation resumed in the middle of the data.

`a_fractional_dimension_names_the_nearest_integer_rather_than_the_truncated_one` replaces ADR
0904's `a_fractional_dimension_is_truncated_rather_than_rounded`, and with `round()` changed back
to `trunc()` it fails on the report it demands — a 3 × 3 grid over twelve bytes is short and says
so, where truncation drew a 2 × 2 in silence. `integer_entry`'s own
`a_real_just_under_an_integer_is_that_integer` fails there too, on
`GHOSTSCRIPT-695872-0.pdf`'s two literal values.

## What it changes

One document of 1 353 digested, and only what it *said*: the false §7.4.8 report is gone and the
display list is byte-identical. ADR 0912 has the measurement and the instrument's new column.
