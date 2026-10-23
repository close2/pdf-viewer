# 2264 — Device solids and every `Indexed` entry follow the paint context

Status: accepted for integration. Session 1715, 2026-10-23.
Context: ADRs 2198, 2234, 2248 (the boundary this moves), 2265 (identity and cost);
ISO 32000-2 §§8.6.6.3, 10.4.2.2, 10.4.2.3, 11.6.6, 11.7.5.3.

## Decision

A plain `DeviceGray` or `DeviceCMYK` colour — `g`, `k`, their stroking forms, `cs`'s initial
colour and `sc`/`scn`'s operand-count recovery — is kept as its family and operands until a mark
consumes it, and converted there into the current target, as `DeviceRGB` already was (ADR 2198).
§11.6.6 converts a source colour into an isolated group's space as it is painted, and §10.4.2.3
gives `0 1 0 0 k` the grey `1 − min(1, 0.59)`, 105 on eight bits, in a `DeviceGray` group; the
colour converted at selection on the page was the device's magenta [236, 0, 140], and that is
what the group drew (ADR 2248's probe).

An `Indexed` entry over **any** base is a selection in that base: §8.6.6.3's lookup is decoded
once (`entry_of`), and the entry follows the base's own route — inline for the three device
families, the profile for `ICCBased` (ADR 2234), the whole space for `Separation`, `DeviceN`,
`CalGray`, `CalRGB` and `Lab` (ADR 2248). One arm of `SolidSource::selected` replaces the guard
that admitted only an immediate `ICCBased` base.

`SolidSource` gains `Gray(f32)` and `Cmyk([f32; 4])`, inline beside `Rgb`: no allocation, no new
cache, no second enum; the two bounded memos and their complete-context key are unchanged, and a
selection seeds the memo with the colour it converted, so an unchanged context paints the byte
the selection painted. Pattern binding keeps precedence; `/None` paints nothing and is not kept.

## Proof

Seven controls in `tests/icc_paint_context/device_solids.rs`, written first and run against HEAD
`e883ed13`'s production code: six failed on the stale colour, one passed as it must (a late `ri`
or `/UseBlackPtComp` over `g` or `k` on the device, whose route reads neither). The `DeviceCMYK`
group paints its colour at half alpha over a yellow it restores from, because an opaque fill
alone composites nothing there and the group gives its own space up for the parent's. Four
omissions in an isolated export each fail on assertions; ADR 2265 lists them.

## What remains outside

§11.7.2's redefinition of `DeviceCMYK` inside an `ICCBased` CMYK group is applied when the space
is read, at selection. A `k` that §14.11.5's output intent resolved to its profile outside such
a group therefore keeps that profile when inherited into it, where a `k` selected inside becomes
the group's own space. A probe on the candidate, built from variants 0 and 30 of `transparency_groups.rs`'s
press fixture as the output intent and the group's profile and not kept in the tree, reads the
inherited half-alpha magenta [255, 143, 190] against the cold selection's [255, 145, 194], and
the two agree without the output intent. That is the question the row faces next: a control
committed red first, then the family read at the paint rather than the resolved profile.
