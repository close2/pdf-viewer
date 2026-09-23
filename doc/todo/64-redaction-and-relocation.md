# Redaction application and clause-driven relocation — two owed builds

Both were opened in batch sixteen (the owner's A64 and A65), set aside unbuilt there, and built
afterwards as single clean rounds — redaction application (ADR 1124), then its image cases (ADRs
1126, 1132, 1133, 1143), its painted paths and forms (ADRs 1195, 1196); relocation (ADR 1123). The
design below is what the two aborted drafts established and the builds followed; it stays here as
*what is* for the cases still owed.

**What a redaction still refuses**, each an owed capability with its own sentence: a stroke with a
zero line width, one this walk has seen no stroking colour
operator for, and one whose ExtGState has made §11.6.4.4's `/CA` differ from `/ca`; a codec image
whose decode is not on the grid its dictionary states (a JPEG 2000 codestream over the decoder's
budget comes back at a reduced resolution level, §7.4.9 NOTE 3 — the region maps onto that grid,
but writing it back would resample every sample outside the region too, and the budget is the
decoder's resource bound, ADR 1324), a `JPXDecode` image stating more
than eight bits per component (ADR 1248), a codec picture whose `/Mask` is a §8.9.6.4 colour key
or whose soft mask states §11.6.5.2's `/Matte` (both are in the picture's own sample domain or
colour space, which the re-encode leaves), a codec image whose decode has a shape no fresh raster
holds; an inline image behind a filter §8.9.7 forbids there; a Type 3 font, a composite font not
`Identity-H`, `sh`, and a soft-mask group.

**One came off that list in ADR 1324.** A round cap, a round join and a stroked curve are cut:
no path can state a circle (§8.5.2.2), so the outline is fitted as cubics within `ARC_TOLERANCE`,
a hundredth of the widening the cut already takes, and cut at its roots like any Bézier.

**Three came off that list in ADR 1277.** A mask is image data: a picture's §11.6.5.2 `/SMask` and
§8.9.6.3 `/Mask` are cleared on their own grids under the picture's placement, copied wherever the
picture is, and a codec picture is decoded with them set aside so its fresh dictionary names the
cleared ones. A `JPXDecode` opacity channel (`/SMaskInData` 1 or 2) is cleared beside the samples
and written as the soft-mask image Table 87 says the processor creates. An inline image behind
`DCTDecode` or `CCITTFaxDecode` is decoded and re-expressed, and one naming a colour-space resource
is written back under that name.

**Two came off that list in ADR 1248.** §8.5.4's clipping boundary is not a mark and the clause
separates painting from clipping in time, so the cut marks are written first and the boundary is
re-stated after them from the producer's own bytes, closed with `n`. §8.9.5.4's `/Alternates` is
**dropped** from the redacted page's copy of the image rather than destroyed: the entry is the only
route to a variant, and the closure the writer copies reaches only what is referenced.

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
  soft-mask group over the region; a path carrying a `W`/`W*` clip, because §8.5.4 sets the clip
  from the same path after the painting operator, so cutting its geometry would move the boundary
  every later mark is held to. **The last of those five is lifted in ADR 1248**, on the clause's own
  separation of the two acts in time.
- **A painted path is cut, not refused** (ADR 1195): the region's four edge lines divide the plane
  into nine cells, the middle one the region and the other eight a disjoint tiling of its
  complement, so the difference is the union of eight Sutherland–Hodgman clips against convex
  windows — taken subpath by subpath, so both fill rules survive it. Surviving vertices are the
  producer's own, copied; created ones are checked against a margin that proves §7.3.3's single
  precision cannot round the cut edge back inside the region.
- **The cut is stated over segments, so a §8.5.2.2 Bézier is split rather than refused**
  (ADR 1236): a half-plane's depth is affine in the point and the mapping into the display list's
  space is affine, so the depth along a segment is a polynomial whose Bernstein coefficients are
  the depths of its own control points; its roots are solved in closed form and the segment is
  split there by de Casteljau. Nothing is flattened.
- **A §8.5.3.2 stroke is cut as the outline it marks** (ADR 1236): expanded with the graphics
  state's line parameters — §8.4.3.6's dash pattern applied *before* the outline is taken — cut as
  a fill, and painted with `f` in the **stroking** colour, replayed under Table 74's non-stroking
  operator from the producer's own operand bytes inside a §8.4.2-balanced `q`/`Q`. An arc in the
  outline is fitted within `ARC_TOLERANCE` and cut at its roots (ADR 1324).
- **A form is entered, always** (ADR 1196): the interpreter runs a form's content inline, so its
  codes are in the placed-code count the walk is calibrated against, and a walk that skipped the
  form refused every page whose text is inside one. An object the page does not own — a form or an
  image another page draws — is **copied** for the page rather than replaced, and the copying
  stops at the first dictionary that reaches nothing replaced.
- **Census** (parsed, over the committed corpora): nine documents carry a `/Redact`, all veraPDF
  and Isartor fixtures, five of them reaching a region through the page tree; **not one has a
  painted path, a form or an image under its region**, so the verb applies all nine both before and
  after ADRs 1195 and 1196 and the corpus cannot rank any of this (trap 8). One end-to-end witness
  with `/RO`+`/IC`+`/DA` — the Isartor `6.5.2` file, which is a
  `-fail-` fixture (verify it opens and its region resolves before trusting it). A byte-grep for
  `/Redact` cannot see one inside a §7.5.7 object stream — plant a `/Redact`-in-objstm fixture so
  the parsed census is known to catch them. The SafeDocs crawl is not on this machine.
- **Reported**: `Origin::Redacted` carries the annotations applied and the glyphs, images and
  painted paths the removal reached; the "removed but overlay/fill not composed" departure is a
  per-page `Report::departures` entry, and both survive into RFC 0002 section 4.5's JSON.
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
