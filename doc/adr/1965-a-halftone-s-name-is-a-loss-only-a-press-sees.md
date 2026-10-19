# 1965 — A halftone's name is a loss only a press sees

Session 1564, the archive slot of batch ninety, on `doc/todo/72` section 2 item 9.
Status: **accepted** and **built** in `pdf-transform`. Context: ADRs 0816, 1942, 1964;
`doc/pdf-a-mitigations.md` section 4.3; `CLAUDE.md`'s clause 10 entry.

## 1. The site and its witnesses

`graphics/no-halftone-name` (ISO 19005-2 section 6.2.5, ISO 19005-4 section 6.2.5: a halftone
may not state `HalftoneName`) was the most-reached `not-built-yet` site at 2b after ADR 1964 —
three documents (`examples/archive_unbuilt`, now with `--who <site>` naming them), each stating
Acrobat Distiller's type 1 halftone `/HalftoneName (Default)` with its own frequency, angle and
spot function. For `endchar.pdf` it was the only refusal.

## 2. What the name does, from §10.6.5.1

"If this entry is present, all other entries, including HalftoneType , are optional. At rendering
time, if the output device has a halftone with the specified name, that halftone may be used,
overriding any other halftone parameters specified in the dictionary"; without one, "the
halftone's parameters may be defined by the other entries in the dictionary, if any". The name is
a pointer into a marking device's own table. §10.6.1: "Halftoning is not required for such
devices" — those that reproduce continuous tone, as this reader does; `pdf-model`'s
`ext_gstate.rs` reads a halftone for its `TransferFunction` alone, and gives the name no case.

## 3. Decision

Removing the key from a halftone that states a `HalftoneType` moves nothing a screen shows and
changes what a press holding a halftone of that name does, which then screens with the
dictionary's own parameters. That is a loss, `Loss::HalftoneName` (`--authorise halftone-name`),
answered by `Rewrite::HalftoneNameRemoved` at the halftones `super::halftones::removal` reads off
the validator's findings — not a statement, because the pointer is the producer's instruction to
a device and a `Stated` row would remove it under every profile. A halftone that is only a name
is refused (without it the dictionary defines no halftone, and §10.6.5.1's default halftone is a
graphics state's entry), as are a shortened finding list and a halftone inside another object.
`as-shown` answers the site `discard` — the window does not halftone; `as-if-printed` leaves it at
`stop`, since a press would have followed the name; `keep-everything`'s attachment stays unbuilt.

## 4. What it changes

`endchar.pdf` converts to 2b under `as-shown`, and veraPDF 1.30.3 calls the output compliant (0
failed rules). Its one page rendered at 72 and 720 dpi before and after differs by at most 7 of
255 (mean 1.16 at 720 dpi), every difference a neutral or CMYK-black colour moved by the GRACoL
output intent the existing `OutputIntent` row writes (ADR 0927), which the halftone removal does
not reach (§2); veraPDF's `6-2-5-t04-fail-a.pdf`, whose only failure is the name, converts with the
loss authorised and draws identically, pixel for pixel. `--remedy-sites --to 2b`: 35 of 75 sites not
built (36 of 74 before; ADR 1964 lists the transfer-function site again).
