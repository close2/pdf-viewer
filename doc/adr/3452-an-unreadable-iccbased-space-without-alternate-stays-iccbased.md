# 3452 — An unreadable `ICCBased` space without `/Alternate` stays `ICCBased`

Session 2187. Status: **accepted and built.** In `pdf-colour`: `parsing::parse_icc_based`'s last branch. Finding
2184-B02. Clauses: ISO 32000-2 §8.6.5.5, Table 65, §8.6.8. Extends ADR 2973.

## Context

Table 65 gives two places to go when the profile is not supported: the `/Alternate`, and, where it is omitted, "the
colour space that shall be used is DeviceGray, DeviceRGB, or DeviceCMYK, depending on whether the value of N is 1, 3,
or 4". ADR 2973 wrapped the first in `ColourSpace::IccAlternate`, so the selected space stayed `ICCBased`: its
initial colour the all-zero one held to `/Range` (§8.6.8), each value held to `/Range` before the alternate sees it.
The second answered with the bare device space, so `/N 4` started at `DeviceCMYK`'s black and `/Range` was dropped.

## Decision

Both branches end in the same `IccAlternate`: the `/N`-implied device space is the alternate the row names "in case
the one specified in the stream data is not supported", whichever way it was arrived at, and the space `cs` selected
is still the `ICCBased` one. A component `/Range` states no pair for takes the device space's own range, as ADR 2973
has it.

## Consequences

Identity, route and §8.6.6.5 "CMYK colour space" questions look through `IccAlternate` to the device space, as they
did; §8.6.7's nonzero overprint mode, which asks whether the current space *is* `DeviceCMYK`, no longer takes such a
space for one, as it already did not where `/Alternate /DeviceCMYK` was stated.
