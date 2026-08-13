# 461 — Four contradicted pages that differ in one entry nobody reads

**Finding.** `oracle.rs` held `calrgb.pdf` pages 1, 5, 11 and 12 in `CONTRADICTED_SUBSTITUTED_FONT`
since the sixth session, on that group's own weakest rule — *the page names a font nobody embedded*
— under two sentences with no number behind them. They are not a font. **The four pages state one
`CalRGB` in three of Table 63's four entries and differ only in the fourth**, `/BlackPoint` at
`[0 0 0]`, `[1 1 1]`, `[8 8 8]` and `[50 50 50]` — and below the header our raster, `poppler`'s,
`mupdf`'s and `ghostscript`'s are **byte-identical across all four pages**, so the gate is printing
one measurement four times. What contradicts us is the swatches: against `poppler` not one channel
of the flat interiors moves by more than four levels, while two thirds of the difference against
the pair that decides the verdict lies inside swatches that hold no glyph. §8.6.5.3's half is the
identity on page 1 and all five renderers apply it; §10.3.1's half is the one the standard puts
"beyond the scope of this document". They move to `CONTRADICTED_CALRGB_TO_SCREEN`, beside the eight
pages of the same file that already carry that reading. **Ninth for nine on a group's name naming a
hypothesis.**

**Date.** 2026-08-13.
**ADR.** [0296](../adr/0296-four-contradicted-pages-that-differ-in-one-entry-nobody-reads.md).
**Touched.** `crates/pdf-model/tests/oracle.rs` (the new group, `CONTRADICTED_SUBSTITUTED_FONT`
17 → 12 → 8, and the two neighbouring entries whose cross-references were wrong),
`crates/pdf-model/src/colour.rs` (`cie_to_srgb`'s `/BlackPoint` argument, and the `CalRGB` half of
`a_cal_spaces_black_point_does_not_move_its_colours`),
`crates/pdf-model/examples/black_point_census.rs` (new),
`doc/conformance/ledger.toml` (§8.6.5.2, §8.6.5.3), `doc/adr/0296-*`, this file.

## Which page was taken, and which was rejected

The round's brief was the demand-driven track and the contradicted list. Two candidates were
opened.

**Rejected: the six sans pages of `CONTRADICTED_SUBSTITUTED_FONT`.** ADR 0267 declined to change
the compiled-in sans and named exactly what would reopen it — "a file that states a
`/FontDescriptor` with a usable `/CapHeight` for a non-embedded face is asking, in the standard's
own vocabulary, for capitals of a stated height", with `issue7580.pdf` ruled out because its
descriptor states `/CapHeight 0`. The other five were checked this round and there is no such file:
`bug850854.pdf`, `issue6069.pdf`, `issue6108.pdf`, `issue9243.pdf` and `issue11403_reduced.pdf`
state **no `/FontDescriptor` at all**, and `bug847420.pdf` states `/CapHeight 500` for `/Arial`,
which is neither Arial's cap height nor a number that would move it toward anybody — honouring it
would shorten our capitals by a further 27% and take the page *away* from all four references. So
ADR 0267's condition is unmet on every one of the six and the decision stands with one more
measurement behind it. That is a negative result and it is why the page was not taken.

**Taken: `calrgb.pdf` pages 1, 5, 11 and 12.** The tell was in the gate's own output before
anything was opened: four contradicted lines identical in all four printed metrics to two decimals.
A group's note pointing at another group's mechanism ("a residue of colour management rather than
of fonts") did the rest.

## What the artefacts and the measurement said

The four-panel strip says the difference is swatches and labels together, which is not decisive,
because the labels are `/Times-Roman` with no `/FontFile` and every renderer substitutes. Four
measurements made it decisive, and the ADR has them in full:

1. **`md5` of the raw RGB below the header**, device rows 150–1090, per renderer per page: four
   renderers, one raster each across the four pages. `hayro` is the only one `/BlackPoint` moves —
   0.87 of 255 from us on page 1, 16.54 on page 12 — and it does not vote.
2. **A flat-region mask**, a pixel whose 7 × 7 neighbourhood is one colour in every one of the five
   renderers: 76.6% of the page, and it contains no glyph. Mean over it: `poppler` **0.004** of
   255, `mupdf` 1.677, `ghostscript` 1.362. Against `poppler` the differing-fraction count over
   that region is **zero channels**.
3. **The swatch values**, read off the five rasters at the swatch centres the content stream
   states. Page 1's space is the identity in all three transformation entries, so `A B C
   = 0.75 0 0` *is* `X Y Z = (0.75, 0, 0)`, and `DeviceRGB` would be `(191, 0, 0)`. Everybody
   paints `(255, 0, 60–66)`. **Nobody here assumes `DeviceRGB`**, which is the hypothesis
   `CONTRADICTED_CALIBRATED_COLOUR` carries for a different page and which this document refutes
   for itself.
4. **Every pair's differing fraction in the gate's own units** — four channels per pixel, alpha
   included. `ours ↔ poppler` 1.62%, `mupdf ↔ ghostscript` 4.41% (the consensus, so the bound is
   twice it, 8.82%), `ours ↔ ghostscript` 11.23%, `poppler ↔ ghostscript` **11.65%**. Two camps of
   two, and **the reference that agrees with us is further from the consensus pair than we are**.
   The gate's printed figure and its printed bound both reproduce exactly from that table, which is
   what makes it a check on the gate rather than a number beside it.

The worst tile is worth one line of its own, because it is the trap in miniature: it sits at
(192, 0) on all four pages — the header line printing the `/BlackPoint` values — so on this page
the worst tile measures the *label font* and the differing fraction measures the *swatches*, and
only the second decides the verdict. Page 12's 13.86 against the others' 14.16 is that header
printing `[50.00000 …]` instead of `[0.00000 …]`, and nothing else.

## The claim that was false, and the command that replaces it

`colour.rs::cie_to_srgb` argues that `/BlackPoint` is read and deliberately not applied — ADR 0012
and §8.6.5.9's "left to the PDF processor to determine" — and closed with "`calgray.pdf` page 3 and
`calrgb.pdf` page 14 are the corpus's only examples". There are **eleven**, in those same two
files. `crates/pdf-model/examples/black_point_census.rs` is the command that counts them, over
every object the cross-reference table lists so that a `/BlackPoint` inside an `/Indexed` base or a
`/DeviceN` alternate is counted like one in a page's own `/ColorSpace`.

And `a_cal_spaces_black_point_does_not_move_its_colours` pinned the decision on `CalGray` only
while its name and its comment both said "a Cal space". It now carries the four black points
`calrgb.pdf` states, on the swatch whose value the rasters give — and reintroducing a stretch on
the `CalRGB` path was tried, and fails it at the first of the four with `(0, 0, 0)` against
`(255, 0, 62)`. That `(255, 0, 62)` is the corpus raster's own byte, which is the closed form
meeting the picture.

## The gates

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets`, `cargo nextest run
--workspace` (1643 run, 1643 passed, 11 skipped), `cargo test --workspace --doc`, and under
`--profile gates`: `corpus`, `oracle` (2 passed; **contradicted 68 total, 66 on pages we call
complete** — unchanged, which is the point: four names moved between two `const` arrays and no
verdict moved), `text_extraction`, `dates`, `xmp`, `jpeg2000`, `render-quorra`'s `corpus`, and
`cargo test -p conformance`. All green. §5's binaries rebuilt and installed.

**`doc/todo/00` step 7's ink sweep is not owed and was not run.** The whole diff under `crates/` is
`tests/oracle.rs`, a doc comment and a test in `colour.rs`, and a new example, so this tree's
rasters are byte-identical by construction and a before/after pair would compare a file with
itself. That is the four-hundred-and-sixth session's precedent, stated in `doc/todo/00` itself, and
the reason it is written down here is that a round *changing* drawing and skipping the sweep leaves
the number unwatched rather than unchanged.

## What the next round should know

- **`CONTRADICTED_CALIBRATED_COLOUR` is the only unmeasured sentence left in this neighbourhood.**
  Its one page, `issue9940.pdf`, says `mupdf` and `ghostscript` "take its components for
  `DeviceRGB`". That is not what they do on `calrgb.pdf`. It may still be true there — the space is
  reached through an `/Indexed` `/DeviceN` alternate, which is a different path — but nobody has
  looked, and the entry now says so.
- **The contradicted list is 68 and every one of its members has a written diagnosis**; what it no
  longer has is a member whose diagnosis names a mechanism belonging to a different group. The next
  round wanting a page from this list should read the *notes* for that shape rather than the
  ranking, which has nothing left to prefer.
- **ADR 0267's reopening condition is now measured to be unmet on all six of its pages**, so a
  round tempted by the sans cap height should read that paragraph and this record's first section
  before spending an afternoon on it.
