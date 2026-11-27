# 3771 — An overprinting mark on the simulated press still takes the page's default

Session 2267. Status: **superseded by ADR 3806**, which builds the per-component choice. Context: ISO 32000-2 §11.7.5.2, §10.8.3; ADRs 1279,
3769; review 2248's D01.

§11.7.5.2: "If a graphics object is painted with overprinting enabled … the halftone and transfer
function to use at a given point shall be determined independently for each colour component. An
object is opaque for a given component only if overprinting yields the source colour (not the
backdrop colour) for that component." ADR 1279 read this for the screen: each of its three
components is converted from all four inks, so a component kept from the backdrop reaches all three
and the mark is opaque for none.

On §10.8.3's simulated press the four inks are the device's components, and a kept component is
that one alone: the mark is opaque for the components it paints and the clause's answer differs per
component. `pdf_model` still hands such a mark no function (`draw_mark`), so on the press the page's
default applies to the components it paints too. Doing it per component needs a channel that
chooses per plane; it is not built here, and `draw_mark`'s comment says which device each
sentence is true of.
