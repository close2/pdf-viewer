# 2693 — Two press statements go under `as-if-printed`, each named in the file's history

Session 1939, a single archive round.
Status: accepted and built: `doc/profiles/as-if-printed.toml` (two entries); `pdf-transform`'s
`archive/halftones.rs` (`name_history`), `archive/overprint.rs` (`zeroed_history`),
`archive/prepare.rs` (`NO_PLACE_TO_RECORD_A_PRESS`, the two `*_recorded` conditions) and
`archive/prepare/metadata.rs` (two rows of `recorded_events`).
Context: ADR 2490 (R17, "press statements kept"), ADR 1965 (the halftone's name), ADR 1978 (the
overprint mode), ADR 2498 (the image-smoothing history entry this copies), `doc/questions/A694`.
Clauses: ISO 32000-2 §8.6.7, §10.6.1, §10.6.5.1, §11.7.4.3; ISO 19005-2 and ISO 19005-4,
sections 6.2.4.2 and 6.2.5.

## The premise, checked

ADR 2490 gave R17 four documents. Round 1938's census refuses each under `as-if-printed` at 2b and
at 4 by one site and no other: `endchar.pdf`, `reserve3.pdf` and `ESUjulyaugust2002.pdf` by
`graphics/no-halftone-name` (Acrobat Distiller's type 1 halftone, `/HalftoneName (Default)` beside
its own frequency, angle and spot function), `102952.pdf` by
`graphics/no-overprint-mode-one-under-icc-cmyk` (three graphics states in object streams stating
`/OP true /OPM 1`, governing DeviceCMYK marks). `as-shown` converts all four at both parts.

One half of the brief did not hold: `no-data-loss` does not refuse the halftone name. It answers
the site `try = ["append-statement", "discard"]` and writes the three documents with the name stated
on its appended page; what it refuses by name is the overprint mode, and that stays.

## What this reader draws with and without them

The halftone: §10.6.1 needs no halftoning on a continuous-tone device, and `pdf-model` reads a
halftone for its transfer function alone, so the name decides nothing drawn here. The overprint
mode: this reader composites by it only where §11.7.4.3's special blend mode applies, a page or group
blending in four components, and the converter refuses that case by name under every profile
(ADR 1978). `102952.pdf` states no `/Group` anywhere, so no mark of it is composited that way.
Measured at 72 dpi on the CPU: with the key renamed in place, `reserve3.pdf`'s 18 pages and
`ESUjulyaugust2002.pdf`'s 140 draw identically to their sources; `102952.pdf`'s output, the mode at 0,
draws its 77 pages identically to the source once `/OutputIntents` is renamed (ADR 2691). Under
§10.8.3's simulation, which a reader asks for, the mode does reach a press's planes; that is the
press this profile does not imitate.

## Decision

1. `as-if-printed` answers both sites `discard`, the entries `as-shown` has. The loss words are the
   ones that exist, `halftone-name` and `overprint-mode`, and the rewrites are the ones ADR 1965 and
   ADR 1978 built. A print scanned back in (A694) was screened and mixed by the printer that made it,
   and §8.6.7 ignores the mode on a device whose colourants are not CMYK; neither statement is one
   the paper would have carried. The two halftone sites the profile keeps at `stop`, a threshold
   halftone and a halftone origin, are not R17's and are not changed here.
2. Each removal is written into the output's own `xmpMM:History`, in every profile that takes it,
   as ADR 2498 writes the images whose smoothing went: the halftone's object and the name it stated,
   and the graphics states whose mode went with the pages where a DeviceCMYK mark was painted under
   them. The entry is a condition of the rewrite: a packet that will not take it withdraws the
   removal (`NO_PLACE_TO_RECORD_A_PRESS`). A mode restated where no DeviceCMYK mark was governed
   costs nothing and records nothing.
3. `no-data-loss` and `refuse-any-loss` are unchanged.

## What it costs

Every output that takes either loss gains one history entry, so `as-shown`'s and the stating
profiles' outputs of the documents involved change by that entry and by nothing drawn.
