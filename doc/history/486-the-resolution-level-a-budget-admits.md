# 486 — The resolution level a budget admits

**Finding.** `doc/todo/24`'s JPEG 2000 item, taken now that the fork's push made it takeable:
the confined worker steps down §7.4.9 NOTE 3's resolution progression until its own sample
budget is met, the raster's grid travels out of the sandbox beside the grid the codestream
*states*, the clause's "Width and Height shall match" check reads the statement rather than
being relaxed, and the soft-mask routing is asked about the raster that exists. `issue19517.pdf`
— 847 million samples, refused since the seventh session — decodes at 3152×4202 and page one
draws the receipt `poppler` draws, its 12608×16806 `/SMask` combined at device scale on the
route ADR 0210 built. The sandbox protocol's raster response grew the stated grid, so the magic
moved to `PDFSBX03`. The routing question's two other askers — the interpreter's report and the
eager route — deliberately keep the dictionary's grid so report and behaviour cannot drift; the
argument is in the ADR and beside `soft_mask_entry`.

**Date.** 2026-08-14.
**ADR.** [0321](../adr/0321-the-resolution-level-a-budget-admits.md).
**Touched.** `crates/pdf-sandbox/src/decode.rs` (`jpx_within_budget`, the loop),
`crates/pdf-sandbox/src/protocol.rs` (`Raster::stated_width`/`stated_height`, `PDFSBX03`, two
tests amended, one added), `crates/pdf-model/src/image.rs` (`SamplesOnGrid`, the §7.4.9 check
against the stated grid, `soft_mask_entry`'s base-grid parameter, `MaskCache::read` re-routing
per call), `crates/pdf-model/tests/jpeg2000.rs` (the `NOT_COMPARABLE` reason for this file),
`crates/pdf-model/tests/thumbnails.rs` (the file's 1×1 `/Thumb` becomes comparable and is
pinned — the miniature is the page's own orange, and the disagreement is the one-pixel
instrument's),
`doc/conformance/ledger.toml` (§7.4.9 and §11.6.5.2 notes), `doc/todo/24-image-sampling-intent.md`
(the JPEG 2000 section moved to closed; corpus witness count now none),
`doc/todo/README.md` (item 24's line), `doc/adr/0321-*` (new), this file.
