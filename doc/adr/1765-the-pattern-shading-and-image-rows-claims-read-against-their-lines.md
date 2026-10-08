# 1765 — The pattern, shading and image rows' claims, read against their lines

Session 1465. Status: **accepted** — an audit, with one defect fixed and four stale sentences
corrected. Context: ADR 1754, which did the same for the colour rows. Rows: the 33 `implemented` rows
under `8.7.` and `8.9.`. Code: `crates/pdf-colour/src/shading.rs`, `crates/pdf-colour/src/mesh.rs`,
`crates/pdf-model/src/content/{pattern,image,xobject}.rs`, `crates/pdf-model/src/image.rs`.

## 1. Why this is coverage work on `implemented` rows

Every `partial` leaf waits on an owner's answer (Q308, Q271, Q348, A66's trigger, a policy syntax no
signature names), so no round can move one this batch; a false claim in an `implemented` row is a
false `implemented` (ADR 1754 §1).

## 2. The method

ADR 1754's: every "is read / are read / is applied / is executed / are executed" in the 33 notes —
24 of them (`scratchpad/r1465/claims.py`) — checked against the code it names, then every backticked
name in the same notes checked to exist, and each type-qualified name to be a member of its type
(`scratchpad/r1465/names.py`). Line numbers are this session's tree.

## 3. The claims

| row | claim | where it holds | verdict |
|---|---|---|---|
| 8.7.3 | Table 74's `/TilingType` read, codes 1 and 3 snapped, 2 free | `pattern.rs` 2146; `Interpreter::lattice` 2283; `pdf_render::snap_lattice` | holds |
| 8.7.3.1 | a pattern stating no `/Resources` read against nothing, in both machines | `pattern.rs` 2205; `pdf-archive` `survey.rs` 2240 | holds |
| 8.7.4.1 | Table 75's five entries read (three claims) | `/PatternType` 1838, `/Matrix` 1856, `/Shading` 1859, `/ExtGState` through `PatternInitial::augmented` 148 | holds |
| 8.7.4.3 | `/BBox` read | `shading::bbox_of`, at `sh` 1655 and at a pattern 1893 | holds |
| 8.7.4.3 | `/AntiAlias` read nowhere | no read in `crates/` | holds |
| 8.7.4.4 | a device-space vertex converted as read | `mesh.rs` 189 (`interpolates_in`), 454 | holds |
| 8.7.4.5.2 | `/Matrix` composed ahead, `/BBox` applied by the caller | `shading.rs` 594 | holds |
| 8.7.4.5.5 | a vertex read in a space the output intent reaches | `mesh::read` takes `kind_of`'s space; `Cache::space_of` 424 | holds; **the space was cached without the resources** (§4) |
| 8.9.5 | `RasterCache`'s key: space, fill, compositing; 64 MiB | `Cached` `image.rs` 7584; `RASTER_BUDGET` 7403 | holds |
| 8.9.5.1 | Table 87's ten entries read | each key read in `image.rs` / `content/image.rs` | holds |
| 8.9.5.1 | `/Alternates` read | `Interpreter::alternate_image` `content/image.rs` 181 | holds |
| 8.9.5.1 | `/OC`'s two `shall`s executed (two claims) | `draw_xobject` `xobject.rs` 70, 102 | holds |
| 8.9.5.1 | `/Intent` read where drawn | `image_conversion` 43, `image_intent` 622 | holds |
| 8.9.5.1 | the four required entries read as Table 87 conditions them | `positive_integer` 1433; the mask's one bit 1016 | holds |
| 8.9.5.3 | `/Interpolate` read for every image | `image.rs` 660; `inline_image.rs` 333 | holds |
| 8.9.5.4 | `/DefaultForPrinting` read when printing | `default_for_printing` 236 | holds |
| 8.9.5.4 | every step for a screen executed | `xobject.rs` 70–102, `alternate_image` 181–222 | holds |
| 8.9.6.1 | a state's soft mask applied by `build_soft_mask` | `transparency.rs` 3306, called from `ext_gstate.rs` | holds |
| 8.9.6.2 | every requirement executed on every backend | the row's cited tests | holds |
| 8.9.7 | a real `/W` read; one helper for both readers (two claims) | `integer_entry::dimension` 86; `inline_image.rs` 559 | holds |

Beside the claims, four sentences named code that no longer does what they say, corrected in the notes:
§8.7.4.4's sentence putting §10.5's transfer function inside the mesh conversion and naming
`transferred_corners` (no such function; no colour a shading or mesh makes carries the transfer, which the display list's transfer
channel applies to the finished pixel — `Colouring`, ADRs 1266 and 1279); §8.7.4.3's claim that
`/Background` takes the ramp's §10.5 transfer, for the same reason; §8.9.5.1's
`ColourSpace::parse_with_output_intent` for `image::colour_space`, which calls `parse_under`; and
§8.7.4.3's sentence on `Cache::space_of`'s key, which §4 changed.

## 4. The defect: the shading cache was keyed without the resources

§8.6.5.6 makes the remapping hold for "a colour space given as an entry in an image XObject, inline
image, or shading dictionary", through "the ColorSpace subdictionary of the current resource
dictionary", and carries it into an `Indexed` base and a `Separation` or `DeviceN` alternate.
`shading::Cache` excluded only a top-level *name* from its `built` table, and keyed `spaces` (a
`/ColorSpace` stated by reference) on the object alone. So one shading object painted from the page
and from a form whose resources state a `/DefaultRGB` was drawn in whichever colours the first painting
built — for `[/DeviceRGB]`, for a reference to `/DeviceRGB`, and for a `Separation` whose alternate is
`/DeviceRGB`. `image::RasterCache` had the right key all along.

The key is now the resources' `/ColorSpace` entry, unresolved, interned once per distinct entry in
`Cache::colour_spaces` and joined to both tables as its index; the build is handed a dictionary holding
that entry alone, so what the build reads of the resources is what the key holds by construction. A
named space is therefore cached too: its lookup reads the same entry. A hit costs one comparison per
distinct entry; an entry is cloned once, when first seen, so a page of many shadings under one large
dictionary holds it once (the shape ADR 0798 measured in the image cache).
`shadings.rs`'s `a_device_space_inside_a_shadings_colour_space_is_remapped_by_the_resources_that_paint_it`
draws the three spellings; with the old key planted all three come out red where the form's default
makes them green. HEAD's six corpus arms (`/home/AI/arms-1462/`) and this change's agree on every one
of 5 795 page digests, so no corpus page paints one shading under two `/ColorSpace` entries that differ
in what they make of it; the fixture is the population (trap 8).

## 5. Left beside it

§10.5's note says a shading built under a transfer is not cached, and that a named `/ColorSpace` is
answered the same way — both halves now false, and not this audit's row; the record names it.
