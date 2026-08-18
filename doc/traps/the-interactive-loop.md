# Traps: the interactive loop, and the spaces a point lives in

Status: **standing** — each is a mistake somebody actually made in this tree.
Read by: a round that turns a press into a command, a command into a request, or a request into a
frame — and any round that converts between the page's space, the display list's and the raster's.
`doc/ui-boundary.md` is the interface; `doc/environment.md`'s `Xvfb` recipe is the only way to
exercise the whole loop.

`doc/HANDOVER.md` is the index and names which group holds which trap. **Every trap keeps its
number**, because `crates/`, `tools/`, `doc/conformance/ledger.toml` and dozens of ADRs cite them
by number and an ADR is not edited to follow a file that moved underneath it (ADR 0232 §2).

## Traps

### 12a. The display list's space is not the raster's, and a doc comment said it was

PDF's y axis points up from the bottom of the page; a raster's points down from its top row. The
flip lives in `TargetSpec::for_page` — "the page's top edge is raster row zero", ADR 0064 — and
**not** in `base_transform`, so a caller holding a pixel position must subtract it from the page's
height before asking `user_space_at` anything.

`user_space_at`'s own doc comment said it took a point in "the page's space — the display list's,
and the raster's" for seventy-five sessions, and **every click followed that sentence into the
mirror of the point it meant**. No gate clicks, so nothing saw it; the tests written for it took
their point from a grid scan of the broken mapping and asked whether *a* link was there, and on
the test document the mirror of a link is another link. ADR 0118.

Two rules out of it: **flip about the *page's* height, not the raster's** — the raster is rounded
up to contain the page and the spare fraction of a row is at the bottom — and **when a test needs
a point, take it from the document rather than from the code under test**.
