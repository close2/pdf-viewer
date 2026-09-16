# Redaction application and clause-driven relocation — two owed builds

Both were opened in batch sixteen (the owner's A64 and A65), set aside unbuilt there, and **built
in batch seventeen as single clean rounds** — redaction application by round 1112 (ADR 1124), then
its image cases by rounds 1119, 1130, 1136 and 1143 (ADRs 1126, 1132, 1133, 1143); relocation by
round 1113 (ADR 1123). The design below is what the two aborted drafts established and the builds
followed; it stays here as *what is* for the redaction cases still owed — a painted path or form
(refused: no per-region unit without geometric subtraction) and a JPXDecode image (refused: an
over-budget decode is a reduced-resolution level, §7.4.9 NOTE 3).

## Redaction application — §12.5.6.23, the owner's A64 ("Owed.")

`doc/questions/A64` is the ratification: applying a redaction is in scope, the overlay text and
fill (`/OverlayText`, `/IC`) are excluded by name (A65's provenance fence), application is a
**write-new-file** operation (RFC 0002's serializer, never a §7.5.6 incremental update), and
where "within the region" is under-specified the choice is documented as a choice.

- **The table is 195** ("Additional entries specific to a redaction annotation"), not 179.
  Region is `/QuadPoints` else `/Rect`; "within" = **bounding-box intersection** (centre-in-region
  leaves legible half-glyphs; containment leaks by construction).
- **The removal needs no font metrics**: §9.4.4's `w0` is recoverable from the placed quad's
  first two corners divided by the text matrix's own x-axis, every term of which the content walk
  already tracks. Hold the walk to the interpreter with a code-count check that refuses the page
  on any disagreement (trap 13's calibration).
- **The interpretation the region test pairs with must be the page WITHOUT `/Annots`** —
  `render::page_to_draw(page, None, false)` — because `Interpretation::text_layer` appends the
  §12.5.3 annotation-appearance pass to every accumulator, and the cut walk reads only
  `/Contents`; otherwise the code-count check fires falsely on every page with a drawn annotation.
- **Refusals the walk owes** (trap 5, refuse the page — never cut it wrong): Type 3 fonts (§9.6.5
  glyph procedures are content streams the walk does not enter); composite fonts encoded by
  anything but `Identity-H` (§9.7.5 codespace ranges decide code-byte width); the `sh` operator
  (§8.7.4.2 paints the whole clip — no byte-range edit removes only the region's share) and a
  soft-mask group over the region; a `W`/`W*` clip becomes `n` rather than a deletion (§8.5.4 sets
  the clip after the painting operator, so deleting the path would drop the clip).
- **Census** (parsed, over the committed corpora): ~5 `/Redact` annotations, all conformance
  fixtures, one end-to-end witness with `/RO`+`/IC`+`/DA` — the Isartor `6.5.2` file, which is a
  `-fail-` fixture (verify it opens and its region resolves before trusting it). A byte-grep for
  `/Redact` cannot see one inside a §7.5.7 object stream — plant a `/Redact`-in-objstm fixture so
  the parsed census is known to catch them. The SafeDocs crawl is not on this machine.
- **Model gap**: `Origin` has no redaction variant and `Report` no per-page departure channel for
  A64's rider (1) — the "removed but overlay/fill not composed" departure needs a home that
  survives into the JSON report, parallel to `Origin::Optimized` carrying `Savings`.
- Whether to apply is a policy asked once where a host can supply it (ADR 1076's shape); the
  `quorra-transform` verb is the batch path, and the viewer surface says applying makes a new file.

## Clause-driven relocation of producer-written appearances — the owner's A65 ("Rule provenance.")

The **amendment** (the scope decision) landed in batch sixteen: `CLAUDE.md`'s fourth amendment
and its ADR. What is owed is the **construction** — moving a producer-written appearance stream
to the position a clause fixes (§12.5.5's matrix), which the amendment puts in scope and ADR 1099's
appended page is the fallback for.

- **The `q`-prepend construction is sound**: prepend one `q` as its own stream, then close
  `depth + 1` states where `depth` is what the producer's content leaves open. The balance rule is
  **§8.4.2** ("Graphics state stack"), NOT §8.4.4. Refuse a producer that "pops further than it
  pushed" (a `Q` with no matching `q`).
- **The load-bearing correction**: annotation `/Annots` array order is **NOT** z-order — ISO
  32000-2 states no painting order among annotations. A refusal must not depend on order. The
  safe, spec-defensible population for the draw-order-overlap refusal is: every annotation that
  **remains** (not removed), is not hidden (clear §12.5.3 Table 167 `Hidden` bit 2 **and** `NoView`
  bit 6), and whose normalised `/Rect` overlaps the moved appearance's device rectangle (§12.5.5
  fits the appearance to the `/Rect`, so the `/Rect` is the rectangle). Any such annotation →
  refuse and fall back to ADR 1099's appended page. This over-refuses relative to true (unknowable)
  z-order, which is the correct direction.
- **Reusable design**: a half-open `overlaps` test (touching edges are not an overlap); `/Rect`
  normalisation per §7.9.5; flattening a single-stream `/Contents` into an array by reference
  without touching a byte (§7.7.3.3 Table 31 admits the array form); skipping §8.9.7 inline-image
  data via `pdf_model::inline_image::scan` when counting `q`/`Q`; bounding the closing stream
  against ISO 32000-1:2008 Annex C Table C.1's "q/Q nesting 28" (ISO 32000-2 prints no such table).
- The class takes in later sites of the same shape, a flattened form field among them; it stops
  where marks become this program's invention (the watermark; the redaction's overlay and fill).

> The clean relocation build is reserved **ADR 1123**; the clean redaction build takes the next free ADR after it.
