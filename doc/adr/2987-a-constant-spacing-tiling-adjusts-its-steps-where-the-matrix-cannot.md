# 2987 — A constant-spacing tiling adjusts `/XStep` and `/YStep` where adjusting the matrix would distort the cell

Session 2076. Status: **implemented.** Amends ADR 1080. Context: review 2038 (findings B72, T133). Clause: ISO 32000-2
§8.7.3.1 Table 74 (`/TilingType`).

Table 74's code 1: "Pattern cells shall be spaced consistently - that is, by a multiple of a device pixel. To
achieve this, the PDF processor may need to distort the pattern cell slightly by making small adjustments to XStep ,
YStep , and the transformation matrix. The amount of distortion shall not exceed 1 device pixel." ADR 1080 took the
adjustment on the matrix alone and, where that would move a wide `/BBox`'s far corner more than a pixel, placed the
tiling at the file's geometry — so the `shall` about spacing went unmet exactly where the cell is wide.

The table names three levers. Where the matrix snap is refused, `pdf_render::snap_steps` rounds each step instead:
the cell is drawn by the file's matrix, undistorted, and every site moves from its neighbour by less than half a
pixel per axis. A scalar step can put its vector on the grid only where the vector lies along a device axis — every
matrix that neither shears nor rotates by other than a quarter turn — and any other keeps the file's geometry, as
before. Both the cell placement and the lattice read the one adjusted `(XStep, YStep)`, so the sites a region reaches
and the sites drawn agree.

Tests: `pdf-model` `tiling` `type_one_keeps_constant_pixel_spacing_when_matrix_distortion_would_exceed_one_pixel`,
`tiling_signed_steps_spec_audit`; `pdf-render` `lattice::tests::a_step_on_a_device_axis_snaps_to_whole_pixels`.
