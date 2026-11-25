# 3654 — A device grey read as the press's black never names a spot plane

Session 2238. Status: accepted. Context: review 2237 (B02), ISO 32000-2 §10.2, §8.6.6.4; ADRs
2758 and 2765.

## Decision

`DeviceGray` on a CMYK press is carried as a one-component `Separation` whose tints are
§10.4.2.3's formula (`Tints::GreyAsBlack`) and whose colourant name is `DeviceGray`, a label for
reports. A page may name a spot colourant `/DeviceGray` — any name is a colourant name — and
`SpotSet::available` compared names, so the grey painted that spot's plate.

§10.2 settles the grey by its family: "Colours specified in any device or CIE-based colour space
shall be rendered as process colours." So `SpotSet::named` answers a `GreyAsBlack` space by its
tints, as `Named::Process`, before any name is compared; the paint (`reach`) and the overprint
question both read `named` and cannot disagree. `DeviceSpots::without_a_plane` stops at the same
space. The construction keeps its name: it is what a report prints, and the guard is on the
variant that cannot be written in a file.

Rejected: renaming the carrier to a name no file can state (a name holding a NUL byte). It would
work by an accident of §7.3.5's syntax and leave the next name-keyed question unguarded.
