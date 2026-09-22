# 1190 — A patch travels to the device, and a stencil keeps its shape

## What moved

**§8.7.4.5.7 and §8.7.4.5.8, `partial` → `departed`** (ADR 1217). §10.7.3's NOTE 2 separates the
two tolerances a patch answers to and says which is which: flatness is "measured in
device-dependent units of pixel width", smoothness "as a fraction of colour component range". The
interpreter has the second and cannot have the first, so `pdf_render::SurfacePatch` now carries a
patch's control net, its four corner values and the tolerance to the backends, and
`MeshRaster::build` — the one function all three reach — derives the fineness from the net's second
differences against half a device pixel and from the corners' bilinear cross term against §10.7.3.
`mesh::MAX_PATCHES` is a bound of its own and `Shaded::truncated` names which bound fired.
`departed` is the one branch left: a patch whose colours §8.7.4.4 has converted between its
corners, which no corpus file takes.

**§11.3.7.2, §11.3.7.3, §11.6.4.3 corrected, §11.6.5.2 re-read** (ADR 1218). A stencil under an
`/SMask` of its own held §11.6.4.2's shape times §11.6.4.3's opacity in one alpha, so §11.4.6 could
be told neither. `soft_mask_entry` routes such an image to the device-scale producer on the
clause's reason rather than the grids', `ImageAtDeviceScale::shape` hands the stencil back, and the
knockout element reaches the display list as a `Shaped` command.

## What it cost, measured

- `raster_golden`: 10 of 974 moved, **6 of them type 6/7 pages** and 4 whose display-list *digest*
  alone moved because the `Mesh` variant gained a field (checked per document, not assumed).
  Re-run after ADR 1218: 974 held.
- Oracle over the six, both ways in one sitting: 1 agrees, 5 ambiguous, 0 contradicted, unchanged.
  Full oracle green. `doc/todo/00` step 7 re-run over 848 artefact pages: the six sit between
  −0.061 and +0.831, none near the −1 alarm.
- callgrind, same binary with the fineness forced back to ten: `personwithdog.pdf` page 1 three
  times, **2 463 460 526 → 804 415 600** instructions.
- Populations: 1099 image `/SMask`s over the 974 documents, **none on a stencil** — the fixture is
  the witness — and **16 behind an image codec**, 27 189 055 samples, which is §11.6.5.2's price.

## What is left, and where it is written

§11.6.5.2's codec residue is **not** `doc/todo/41`'s cache: that cache holds the §7.4 chain's
output and `Document::image_stream` memoises only the prefix in front of the codec. `MaskCache`'s
`ObjectId` key would take it with no new key; the decision owed is a bound on the decoded grey
plane, and `doc/todo/41` and §11.6.5.2's row now carry it with the numbers.
