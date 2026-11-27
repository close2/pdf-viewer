# 3808 — A Type 5 halftone's entry for a spot colourant is read by its name

Session 2276. Status: accepted. Context: ISO 32000-2 §10.5, §10.6.5.6 (Table 132), Tables 128 to
131, §10.8.3; erratum #311 (approved); ADR 3769.

## The reading

Table 132's "any colourant name" key holds "\[t]he halftone corresponding to the colourant or colour
component named by the key", and its `TransferFunction` "overrides the current transfer function in
the graphics state for the same component" (Table 128; Table 130's amended text makes it required
for "a nonprimary or nonstandard primary colour component"). §10.5: a halftone dictionary "is the
only way to set transfer functions for nonprimary colour components". So on the simulated press a
spot colourant a Type 5 halftone names takes that entry's function, and one it does not name takes
`/Default`'s ("used for any colourant or colour component that does not have an entry of its own").

§10.6.5.6's sentence "If nonprimary colourants are requested when the current halftone is defined
by any means other than a Type 5 halftone dictionary, the gray halftone screen and transfer
function shall be used for all such colourants" would give a spot colourant Table 57's fourth
function under `/TR`'s array of four. Erratum #311, approved, deletes the paragraph it stands in, so
it is not the standard (ADR 3581) and ADR 3769's answer stands: that array sets no function for a
spot colourant.

## The construction

`HalftoneTransfer` keeps every Type 5 key that is not the dictionary's own or one of the press's
four, each read once; the screen's red, green and blue are read out of the same list.
`pdf_colour::Transfer` carries them by name, `pdf_render::TransferMap` samples each into a table
(`with_named_spots`, `spot_table`), and `resolve_press_transfers` gives colourant `3 × n + c` of the
page's `SpotSeparation` — channel `c` of spot plane `n` — its own table by its name. `Red`, `Green`
and `Blue` are among the names, because on a CMYK device a colourant of those names is a spot one
and §10.6.5.6 calls them primaries only "for DeviceRGB". Tested on every lane by `press_transfers`.
