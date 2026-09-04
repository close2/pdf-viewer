# 0904 — A dimension written as a real, and one answer to §7.3.3 rather than two

Session 932. Status: **accepted**.

## Context

The nine-hundred-and-thirty-second session walked
`corpus-cache/tika-issue-tracker/batch5/qpdf` under `doc/todo/03`'s rule and ranked its first
pages by ink, ours against `pdftoppm -cropbox` and `mutool draw -b CropBox` at 72 dpi. The head
is not close to anything else in the directory:

| | ours | poppler | mupdf |
|---|---|---|---|
| `qpdf-278-0.pdf` | **0.000** | 177.973 | 177.313 |

The next row down is 0.895 outside the interval the two references bracket, so this one is the
head by a factor of nearly two hundred. The page is a book cover — a full-bleed photograph on a
1062 × 1425 sheet — and this tree drew a blank white page and said so:

```
interpreted, 0 commands, unsupported [Image { name: "<inline>: malformed image: missing or invalid /Width" }]
```

The whole content stream is one inline image, and its dictionary is

```
BI
/W 1062.00 /H 1425.00 /BPC 8 /CS /RGB /F [/A85 /Fl]
ID
```

`/W` and `/H` are **reals**. `pdf_model::image::positive_integer` read them with
`Object::as_integer`, which answers `None` for anything but an integer object, so the image had no
grid and the page had no marks.

## What the clause says

Table 87 types `/Width` and `/Height` as integers, and §7.3.3 is what makes writing a real there
an error:

> A real number shall not be present when an integer is expected.

That is a `shall` addressed to whoever wrote the file, and the file breaks it. **The standard
states nothing at all about what a reader does next**, so this is a choice in the sense
`CLAUDE.md`'s principle 5 means: a documented decision, not a derivation.

## Decision

**The choice was already made in this tree, and the decision here is to have one answer to one
clause rather than two.** ADR 0371 met the same sentence from the other side — §7.10.5's
calculator, an operand typed `int` reached by a real — and answered it with a rule:

> a real is truncated where an integer is wanted

on the ground that "a file that does it anyway is a file this viewer still has to draw".
`pdf_model::image::dimension_entry` is that rule, applied to Table 87's `/Width` and `/Height`,
and it is now the single place in `image.rs` where either entry is read: `positive_integer` and
the four dimension closures that had each written their own `as_integer().and_then(u32::try_from)`
all go through it. That consolidation is most of the value. Had only `positive_integer` been
changed, a real `/Width` on a `/Mask` would have read as 1062 in one function and as 0 in the next
— two readers of one file inside one crate, which is the failure the "one rule in one place"
discipline exists to prevent.

Three things the rule deliberately does not do:

- **It does not widen to every integer entry.** `/BitsPerComponent`, Table 11's `/Rows` and
  `/Columns`, and the rest keep `as_integer`. The population measured below is `/Width` and
  `/Height`; a tolerance no document exercises is untested code rather than robustness, and the
  helper is one line from any of them the day a document asks.
- **It does not accept a value that is not a grid.** A NaN, an infinity, a negative and a value
  past `u32`'s range are refused exactly as before, and the callers' own `> 0` filter and
  `MAX_SAMPLES` are unchanged. What used to be a refusal for *every* real is now a refusal for
  the reals that name no grid.
- **It does not round.** Truncation is ADR 0371's word and the tests pin it: `/W 2.9` over twelve
  `DeviceRGB` bytes is a 2 × 2 image, and a rounding reader would want eighteen bytes for a 3 × 3
  one. `1062.00` cannot tell those two apart, which is why the fractional case is a test of its
  own.

## What it changes

`qpdf-278-0.pdf` draws its cover: **ink 177.313 against poppler's 177.973 and mupdf's 177.313**,
and `magick compare -metric AE` against `mutool draw`'s raster at 72 dpi is **0**, pixel for
pixel. It is a row in `doc/checks/fixed-documents.toml`.

The population is in [`doc/todo/03` §49](../todo/03-more-corpora.md), measured the way session 926
measured its own: the whole survey re-run over every corpus on this disk and the two passes
diffed, rather than a count of what the fix was written for.

## The tests, and that they fail without it

`crates/pdf-model/tests/inline_images.rs` gains two, both of which are the file's first test with
two periods added, so what they isolate is the periods and nothing else. Both were run against the
defect before they were believed (trap 13): with `dimension_entry`'s real arm returning `None`
they fail, and with it in place the file's 24 tests pass.

## What was considered and declined

**Accepting only a real whose value is an exact integer**, and refusing `1062.5`. It is the
narrower rule and it decides nothing — there is exactly one integer `1062.00` can be — but it is a
*second* answer to §7.3.3 living beside ADR 0371's, and the argument for one rule in one place is
the whole of this ADR. A reader that truncated in the calculator and refused in the image decoder
would owe an explanation nobody could give from the clause.

**Reading the dimensions out of the encoded data instead.** `/F [/A85 /Fl]` is not an image codec,
so there is no frame here to ask — that route is §7.4.8's and `image::frame_as_defined` already
takes it where a `DCTDecode` stream states its own (ADR 0799). It answers a different question.
