# 1178 — Two departures argued, and a masking domain measured

Two of the three rows were decisions nobody had written down; the third was narrower than its note.

## §8.6.6.5 — `partial` → `departed` (ADR 1193)

Read whole. Everything it states is executed except one sentence: the `NChannel` per-component
evaluation of a space with a spot colourant, and with it Table 70's `/Colorants`. Executing it on a
display means combining the *n* colours the components evaluate to, and every candidate was disposed
of — NOTE 3 hands blending to the processor; Table 70 says a `Separation`'s function describes a
colourant *alone* while the space's own describes them *in combination*, which is what a fill needs;
§11.7.3's rules are about an object against a backdrop; and §10.8.3's multiply-in-XYZ, which NOTE 3
names, is conditioned on a separation-simulation request this viewer has no control for.

Priced by `a_spot_components_own_separation_is_stated_and_not_taken`: a space whose `/Colorants`
state blue and green draws its tint transform's red, 255 levels out in two channels, the same
`Separation` alone being the control. `an_nchannel_process_space_is_the_four_components_a_cmyk_group_composites`
is the other half — the process tints reach `Compositing::Subtractive`'s four components
unconverted, an unnamed CMYK one arriving as §8.6.4.4's absence.

## §10.4.2.3 — `partial` → `departed` (ADR 1194)

Its CMYK-to-grey direction is executed exactly; its grey-to-CMYK direction is not answered with,
because §10.4.2.1 ranks the classic family below §10.3's route. Only `departed` survives that
sentence, which is about a *route* rather than an answer — §10.4.2.5's disposition since ADR 0263.
Priced over twenty-one greys: up to **0.60** apart in a component, which §11.3.4's per-component
compositing reads inside a `DeviceCMYK` group, and **under one eight-bit level** apart in the pixel,
the separation and the press's cube being inverses. It does not reach §8.6.7's zero test, which
ADR 1157 makes read the tints the file stated.

The aggregates §10.4.2, §10.4 and §8.6.6 moved to `implemented` with them, as §12.4 already stood above a `departed` §12.4.4.

## §8.9.6.4 — kept `partial`, residue narrowed

Measured, not assumed: a sixteen-bit ramp whose middle samples are `0x8000` and `0x8001` — one unit
apart in sixteen bits, one byte in eight — is masked at the first and painted at the second, and
comes out one colour unmasked. The comparison runs in the file's domain wherever `unpack` sees the
samples, so the residue is the `JPXDecode` decoder's eight-bit hand-off alone. Calibrated by forcing
the comparison to eight bits, which refuses the range and reports it. ADR 1121's disposition stands:
owed, not departed.
