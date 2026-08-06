# ADR 0203 — Four components stay four, whichever transform spells them

Status: accepted, 2026-08-06 (session 354).

## Context

The project owner opened a 92-page commercial catalogue and **every page was blank**. The viewer
said one thing about it:

> an image (Im0: colour space a 4-component space on a JPEG of 3 components is not supported) was
> not drawn

`doc/todo/28` measured what that sentence undersold: `open_one` reported **0 commands** on pages 1,
2, 3, 5 and 10, and `render_at` on page one produced a raster whose ink was **0.000**. The image
*is* the page.

The file itself:

```text
pdfimages -list   1757 × 2489, cmyk, 4 components, 8 bits, jpeg, 300 ppi, with an /SMask
the codestream    SOF0: 4 components, APP14 Adobe, transform = 2
```

Transform 2 is Adobe's **YCCK**: the first three channels carry a JFIF luminance-chrominance
transform of what would otherwise be the first three inverted CMYK channels, and the fourth carries
K alongside. `decode_jpeg` asked `zune-jpeg` for CMYK out only where the *input* colour space was
CMYK, so a YCCK codestream fell through to the default — RGB, three components — and
`convert_channels` refused four-against-three by name.

**This is the strongest single piece of demand this project has had from outside its corpus**, and
none of the 974 pdf.js documents carries a YCCK JPEG, so no gate could have found it.

## Decision

### Ask for four, in whichever spelling the codestream uses

`decode_jpeg` now asks `zune-jpeg` for its own input colour space out whenever that space has four
components — `CMYK` (transform 0) or `YCCK` (transform 2). The reason is §8.9.5.1's: the
dictionary's `/ColorSpace` is what interprets an image's components, so a decoder that turned four
into three has answered the clause's question on its behalf.

**`zune-jpeg` has no YCCK → CMYK conversion**, and that is why the arithmetic is here. Its two YCCK
arms both go to RGB and composite the black channel in on the way (`blinn_8x8`), which throws away
exactly the component `/ColorSpace` needs. Asking for `YCCK` out takes the four raw channels
instead — the decoder's own fast path for a four-component input whose output space equals it.

### The inversion is *not* undone, and that is the load-bearing half

`ycck_to_cmyk` is JFIF's inverse followed by a subtraction:

```text
C = 255 − R    M = 255 − G    Y = 255 − B    K = K
```

An Adobe four-component JPEG stores CMYK **inverted** whichever transform it uses, and a PDF that
means the ordinary reading says so with `/Decode [1 0 1 0 1 0 1 0]` — §8.9.5.2's entry, which
`apply_decode_to_channels` applies one step later. A decoder that un-inverted here would undo the
file's `/Decode` twice.

So what this function owes is not "correct CMYK" but **the same convention transform 0 delivers**,
which is also what libjpeg's `ycck_cmyk_convert` produces. One convention, two spellings, one
`/Decode`.

### Checked by arithmetic here and by a picture there

No corpus document carries a YCCK JPEG, so the test is three exact values — white, black, and a
chrominance that separates the channels so that dropping Cb or Cr fails rather than passing by
symmetry.

The picture is checked where the witness is. On the catalogue's first page, at 72 dpi:

| | ink | MAE against `poppler` |
|---|---|---|
| ours | 222.689 | **0.0113** |
| `poppler` | 223.742 | — |
| `mupdf` | 224.255 | 0.0384 |

**We are three times closer to `poppler` than `mupdf` is**, and the residual is the ordinary
CMYK → RGB difference §10.4.2.5 and §10.3 rank. Pages 1, 2, 5, 10 and 40 all report `unsupported
[]` where every one of them reported a refusal.

## Consequences

- **A 92-page catalogue that drew nothing draws.** `doc/todo/28`'s first item closes; its second
  and third — §11.6.6's blending space inside a soft mask and §11.4.7's `/DeviceCMYK` group, twelve
  and four times on one page — are `doc/todo/23`'s and stay open, and this document is now the
  witness a round taking them should measure against.
- **Nothing on the corpus moves**, because nothing on the corpus is YCCK: 974 documents, 73
  incomplete, and the oracle unchanged at 856 / 68 / 750.
- **Table 13's `/ColorTransform` is still closed by decision** and this does not reopen it. That
  entry is a PDF-level override of the codestream's own APP14 marker, and this change reads the
  marker rather than the entry — a four-component codestream is four components whether or not a
  dictionary offers an opinion about the transform.
- The reason no gate found it is worth keeping: **the corpus is the world's documents only as far
  as pdf.js collected them**, and a commercial catalogue in CMYK is a population it has none of.
  Trap 8, from the side that costs a reader a whole document rather than a clause.
