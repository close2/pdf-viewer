# Q585 — Does the loss `substitute-stretch` cover a fit of any size, or does it want a bound?

Source: session 1722, ADRs 2262 and 2279, `doc/questions/A566`. Asked because A566 was argued on
the eight documents of ADR 2179, whose largest fit was 0.026 %, and the census that followed found
fits up to 0.72 %.

## What changes

ADR 0358 has the viewer draw a substituted face at the horizontal scale that fits its advances to
the file's `/Widths`. Embedding the face unmodified draws every glyph at the face's own width, at
the same position. A566 lets `as-if-printed` do that under the loss `substitute-stretch` and keeps
`as-shown` stopping on it; the report, the JSON and the file's own `xmpMM:History` name each font
with the scale it lost.

Round 1714's census read 88 fonts in 26 newly refused documents: scales from 0.9999765 (44 of
them) to 0.9928438. The widest is `doc/pdf.js/test/pdfs/bug1671312_reduced.pdf`, a Helvetica-Bold
drawn from Liberation Sans Bold on the restated-program route. Drawn, source against the
`as-if-printed` output at part 2b, page 1:

- at 100 dpi, 346 pixels differ, by at most 18 levels of 255;
- at 288 dpi, 1 055 pixels differ, by at most 51 levels, all on the right-hand edges of the glyphs
  of the one word "Purposes" — the word itself is unchanged to the eye.

The pair is drawn by `cargo run --release -p pdf-model --example render_at -- <file> 1 4 <png>`
on the source and on `quorra-transform archive <file> --to 2b --config
doc/profiles/as-if-printed.toml`'s output; the round that asked worked in `scratchpad/r1722/`.

The standard states no tolerance here: §9.2.4 makes `/Widths` what positions a glyph, and nothing
in ISO 32000-2 or ISO 19005 bounds how far a substitute's shape may be scaled. The restated-program
route can reach far larger fits than the census found — a width a quarter narrower than the face
is a scale of 0.75 — and those are named by the same loss today.

## Recommendation

**No bound.** The loss is a statement that the archive draws the face at its own width, and the
scale is printed beside every font it costs, so an operator reads the size of each fit rather than
trusting a threshold. A bound would be a number the standard does not give, chosen to make a
corpus quiet, and a fit just past it would be refused under the printing profile although nothing
it loses differs in kind.

The alternative is a bound — say 1 % — past which `as-if-printed` stops as well and a font must be
supplied with `--font` or its program restated with the fit drawn into its outlines.

## What the tree does meanwhile

The loss covers any fit and names its scale. `as-if-printed` discards it, `as-shown` stops on it,
and no outline proof is weakened.
