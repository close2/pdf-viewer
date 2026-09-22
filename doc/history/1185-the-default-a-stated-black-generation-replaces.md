# 1185 — The default a stated black generation replaces

## §10.4.2.4, §11.7.5.3, §8.4.5 — the black generation is evaluated (ADR 1207)

ADR 1069 refused to evaluate Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2` because "there is no step of
the conversion it *does* perform for a stated function to replace". False, forty lines from the function
that ADR named: `colour::INK_LADDER` is "[t]welve, from all the black there is down to none", citing this
clause's permission to "return a larger value for extra black".

§10.4.2.4 requires every device to have the pair; §11.7.5.3's first bullet says a file's pair is used
instead of the device's, for a `DeviceRGB` colour painted into a `DeviceCMYK` group — so one parameter of
one conversion changes and §10.4.2.1's ranking is untouched. New `pdf_colour::black_generation` holds the
pair and the formula, a graphics-state parameter carried by `Conversion` beside §8.6.5.8's intent and
§8.6.5.9's black point; `ColourSpace::to_cmyk_under` is the one arm that uses it, and Table 57's two
routes are both read — `gs` and §11.6.7's pattern dictionary.

**§11.7.5.3 moved `implemented` → `partial`, downwards and on purpose**: the argument that made the clause
vacuous is gone, so its second bullet is a `shall` answered with a report — a group painted into a
four-component parent, converted per pixel in a backend where no colour space exists.
`Unsupported::BlackGeneration` names that and a press whose own `B2A` states the conversion in.
**Nothing in the corpus moves**: `raster_golden` held 974, moved 0 — no first page states such a
*function* inside a `DeviceCMYK` group. Both fixture pages were rendered and looked at (trap 1).

## §8.6.5.9 — the blocker is a construction, not a missing document (ADR 1208)

No ICC text held states BPC: the application note defers to ISO 18619, ICC.1's v2 editions define the
black-point *tag* and no use of it, ICC.1:2022 and ISO 15076-1:2010 have no such tag and never use the
phrase, and ICC.2:2023 defers twice. But the ICC publishes the approved committee draft and a white paper
stating the mapping, free. ADR 1208 lists the eight things ISO 18619 adds; the LUT-destination estimation
among them *is* the diagnosis of the eleven levels ADR 0510 measured without one.
`doc/third-party-data.md` gains the provenance and the note's own CC BY 4.0.

## §10.7.4 — the union requirement stands, and is priced (`doc/todo/11` item 9)

The clause states it outright — the clipping region is the pixel set a fill would include — so the residue
is the clause's rather than a claim about this tree's clip representation. One price per backend:
`render-cpu` could OR a second fill into its mask today, `render-raster` would need an alpha mask group and
`render-gpu` a readback. Not taken: one backend alone would make the oracle report the fix as a defect.
