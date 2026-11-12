# 2829 — `as-shown` admits the glyph choice where the machine's face went in proved

Session 2005, a single archive round.
Status: accepted and built. In `pdf-transform`: `archive/config.rs` (the site key
`glyphs-from-to-unicode`, read beside `text-as-image`, and the authorisation it stands for);
`doc/profiles/as-shown.toml` (the key, `discard`).
Tests: `crates/pdf-transform/tests/archive/machine_cid.rs`
`as_shown_admits_the_glyph_choice_only_where_the_machines_face_went_in` and
`which_shipped_profiles_admit_the_glyph_choice`.
Context: owner answer A832 (`doc/questions/A832`), A570 (the two losses), ADR 2394 (the first
loss of this site answered apart from the others), ADR 2807 and ADR 2828 (the route and its proof),
ADR 2774 (which profile takes the machine's face), ADR 2693 (a loss is named in the history).
Clauses: ISO 32000-2 §9.7.4.2, §9.7.5.2, §9.10.2; ISO 19005-2 sections 6.2.11.4.1 and 6.7.2; ISO
19005-4 section 6.2.10.4.1.

## Context

An `Adobe-Identity` font the file does not embed has CIDs that index a program nobody has
(§9.7.4.2), and §9.7.5.2 forbids `Identity-H` with such a font. The window draws it from the
character its `/ToUnicode` states for each code (ADR 0270). A570 named two losses for embedding
it: `machine-face` and `glyphs-from-to-unicode`, and `as-shown` stopped on both. ADRs 2807 and 2828
embed the face the window drew, proved code by code to draw the same outline at the same metrics,
which leaves `glyphs-from-to-unicode` alone. A832 (2026-11-11, "agreed"): `as-shown` admits it
when and only when that proof holds; the loss stays named; every profile that withholds it keeps
its stop; `as-if-printed` is unchanged.

## Decision

1. **A second per-loss key at `fonts/font-programs-embedded`**, `glyphs-from-to-unicode`, read
   as `text-as-image` is (ADR 2394): `discard` authorises that loss whatever the row's remedy says
   of the others. `as-shown` sets it; no other profile gains it.
2. **"Only where the proof held" is the routes' own invariant, not a second test.** The machine's
   route is the only one that names `glyphs-from-to-unicode` without `machine-face`
   (`machine_cid.rs`); it writes nothing it has not proved, and every font it does not take falls
   back to the re-keying route, which names both. `as-shown` withholds `machine-face`, so it
   still stops there. A flag carried from the proof to the decision would restate that invariant
   in a second place, where the two could disagree.
3. **The loss stays named**: the report's font row and the file's `xmpMM:History` say
   `glyphs-from-to-unicode` as they did under `as-if-printed`; the sentence describing it now
   names the face as the shipped one or the machine's.

## Consequences

Under `as-shown` at both parts, an `Adobe-Identity` font whose window face this route can write
converts. `refuse-any-loss`, `no-data-loss`, `keep-everything`, `only-metadata-loss` and
`derive-attachments` keep their stop; `as-if-printed` and `state-every-loss` already authorised
the loss and keep the shipped face (ADR 2774). Should a later route name the loss alone without a
proof, this decision would admit it too, so such a route must name `machine-face` or be argued.
