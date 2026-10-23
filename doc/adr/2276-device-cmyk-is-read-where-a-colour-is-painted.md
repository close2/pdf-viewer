# 2276 — `DeviceCMYK` is read where a colour is painted

Status: accepted for integration. Session 1721, 2026-10-23.
Context: ADRs 2012 (the redefinition at selection), 2198, 2248, 2264 (the solids follow the paint),
2277 (identity and cost), 1242, 1622, 2170 (the departure that remains); ISO 32000-2 §§8.6.5.6,
11.7.2, 11.7.5.3, 14.11.5.

## Decision

§11.7.2 says "DeviceCMYK shall be redefined within the transparency group to be the same as the
blending colour space", so what the family means is a property of the group a colour is painted
in, not of the place it was selected. `ColourSpace::device_family` answers the family when a space
is read — after §8.6.5.6's default, before §14.11.5's intent — and the interpreter read it at the
`k`, the `cs` and the `sc`. A `k` resolved to the output intent's profile on the page was then
converted from that profile into an `ICCBased` CMYK group it was inherited into, where the same
`k` selected inside is the group's own components.

The family is now read again at the paint. `SolidSource::Cmyk` is the family itself, however the
selection read it — the device, the output intent, or a group — and paints as `device_cmyk_at`
reads it under the paint's conversion: the group's space inside an `ICCBased` CMYK group, the
intent outside one, the device without either. A whole space holding the family (an `Indexed`
base, a tint transform's alternate, §10.8.3's process space) keeps the reading it held
(`SpaceSource::device_cmyk`), and so does an uncoloured pattern's underlying space
(`TintSource::device_cmyk`); `read_where_painted` replaces that component where the paint reads
the family otherwise. An `Indexed` entry over the family is a selection in it (ADR 2264). A
§8.6.5.6 default is decided at the selection — the clause checks "the ColorSpace subdictionary of
the current resource dictionary" when the space is selected — and is kept.

Which selections read the family is told by allocation (ADR 2277): the profile `device_family`
hands out for it is the group's or the intent's own `Arc`, and no other route reaches a selected
space with either, except a space `icc_spaces` remembers, whose allocation a group's own space
may share, and which is therefore asked about the output intent alone.

## Proof

Four controls in `tests/icc_paint_context/cmyk_redefinition.rs`, written first, on presses whose
black ink the output intent, the group and a nested group absorb differently, so that `0 1 0 1`
is a grid point with three distinct answers. Expected pixels are each press's encoded corner
through the published D50-adapted sRGB matrix, half over white. On HEAD's production code two
fail on ten pixels — every one of the five selections (`k`, `cs`/`sc`, `Indexed`, `Separation`,
uncoloured pattern) inherited into the group reads the intent's press, and selected in one group
and painted in a nested one reads the outer's — and two pass as they must: the presses are told
apart, and a `/DefaultCMYK` naming the intent's own profile stream stays that default inside the
group while a cold `k` there is the group's. The candidate passes all four. ADR 2277 lists the
mutations each control catches.

## What remains

The row is `departed` on one sentence, decided before this: a `DeviceCMYK` group whose press is its
output intent's bi-directional profile converts a `DeviceRGB` paint in through that profile's
`B2A` table, not through §10.4.2.4's device default a stated pair substitutes for (ADRs 1242
section 2, 1622, 2170). What the redefinition at the paint leaves unread is narrower than a
clause: a damaged `ICCBased` resource whose `/Alternate` is the family, selected a second time
inside one group — where `icc_spaces` answers it with an allocation a group may share — and
painted in a nested group of another profile (ADR 2277).
