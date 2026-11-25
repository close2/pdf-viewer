# 3655 — A grey is black ink alone on the simulated press of the assumed inks

Session 2238. Status: accepted. Context: review 2237 (B03), ISO 32000-2 §10.3.2, §10.4.2.3 and
§10.8.3; ADRs 0263, 2712, 2758 and 2765.

## Decision

§10.8.3's simulated device of a page with no four-component output intent is the assumed inks
(`simulated_press`), a device whose native colour space is CMYK. §10.3.2: "If the native device
colour space is CMYK, then converting colours in the DeviceGray colour space to that CMYK should
follow the method described in 10.4.2.3" — black `1 − gray` and no other ink. ADR 2758 read that
sentence for an intent's press only, keying `grey_reading` on the intent, so a grey was separated
into four inks on the assumed press.

`GreyReading::AssumedInksBlack` is the new answer, given by
`content::colour::output_intent_reading` when the run is drawn for the simulation and the intent
(if any) gave `Device`. `ColourSpace::device_family` reads it as
`ColourSpace::grey_as_assumed_inks_black`: the same `Separation` as ADR 2712's construction, over
`DeviceCMYK` rather than an intent's profile — on the assumed press `DeviceCMYK` *is* that press's
four inks, so `0.5 g` and `0 0 0 0.5 k` arrive as one colour on every route. A `/DefaultGray`
outranks it, a 'GRAY' intent still stands in for the family by its component count, and the
screen view (simulation off) keeps this reader's grey, which no clause moves.

Everything drawn under the simulation reads the grey this way, including a page whose own
blending space is one or three components and whose spot planes are drawn beside it: the device
being simulated is the same CMYK press, as it is under a CMYK intent (ADR 2765).
