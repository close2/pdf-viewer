# Q566 — May the printing profile remove the viewer's substitute-width fitting?

Source: session 1671, ADR 2179. The eight refused documents are named there.

## What changes

The viewer fits a substitute font's outlines horizontally to the document's widths.
Embedding the same face unmodified removes that fitting. The character positions
remain the document's, but the glyph shapes change slightly. The first refused
fonts in these eight documents use scales 0.9999765, 0.99999064 or 0.9997446;
the largest of those changes is about 0.02554 percent. These are real transforms,
not hinting differences or a tolerance justified by the PDF/A width agreement rule.

The existing `machine-face` loss covers replacing a machine's font with a shipped
face. These documents already use the shipped face, so that permission does not
cover removing the fitting. Silently broadening it would change what existing
configurations authorise.

## Recommendation

Allow a separately named `substitute-stretch` loss: the printing profile may embed
the unmodified face without fitting, while `as-shown` keeps stopping. Before that
is enabled, a focused round must make one font-embedding decision carry every
loss it incurs: a document can need both a machine-face replacement and removal
of fitting. Its report and provenance must name both, and both must be authorised.
That round must also draw the source and output pairs and state their measured
pixel differences; these scale figures are not measurements of rendered error.

The alternative is an exact font-program transformation that preserves the fitted
outlines, including the modified font's licensing and naming obligations.

## What the tree does meanwhile

It refuses these fonts with the specific fitting reason. Neither profile gains
permission, no outline proof is weakened, and the viewer's rendering is unchanged.
