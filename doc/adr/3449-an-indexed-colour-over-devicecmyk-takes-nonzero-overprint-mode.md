# 3449 — An `Indexed` colour over `DeviceCMYK` takes nonzero overprint mode

Session 2187. Status: **accepted and built.** In `pdf-model`: `content::colour::cmyk_tints` and
`reverts_to_cmyk`, an `Indexed` arm in each. Finding 2184-B03. Clauses: ISO 32000-2 §8.6.7, §11.7.4.3 NOTE 2,
§8.6.6.3. Answers ADR 1241 section 5, which named the work and left it.

## Context

§8.6.7: "Non-zero overprint mode shall apply only to painting operations that use the current colour in the
graphics state when the current colour space is DeviceCMYK". §11.7.4.3 NOTE 2 says what the current colour space is
for three families: "In the case of an Indexed space, it refers to the base colour space (see 8.6.6.3, "Indexed
colour spaces"); likewise for Separation and DeviceN spaces that revert to their alternate colour space". The
`Separation` and `DeviceN` route was built by ADR 1241; the `Indexed` one was not, because `entry_of` was private to
`pdf-colour`. It is public now, so an `Indexed` colour over a `DeviceCMYK` base under `/OPM 1` replaced the
backdrop's black with zero, where `k` of the same four numbers kept it.

## Decision

`cmyk_tints` answers an `Indexed` space whose base is, or reverts to, `DeviceCMYK` with the tints of the table entry
the index selects — `ColourSpace::entry_of`, the one place §8.6.6.3's rounding and clamping happen, so the tints and
the colour of the mark are read from the same entry. The base is asked recursively, so an `Indexed` space over a
`Separation` or `DeviceN` that reverts to `DeviceCMYK` takes the same route the NOTE's "likewise" gives the base on
its own. `reverts_to_cmyk` gains the same arm so the discriminant test still runs before any lookup.

## Consequences

- Every family the NOTE names now has its arm on the `sc`/`scn`, `cs` and `g`/`rg`/`k` routes, which all reach
  `cmyk_tints`. An image's `/ColorSpace` is not current colour and keeps §8.6.7's own answer.
- An `Indexed` selection over `DeviceCMYK` allocates one entry vector per `sc` for the tints; the colour beside it
  already did the same lookup.
- Audit 2184's test compared a table byte `E6` (230/255) with `0.9`, a different colour by one level in the red
  channel; its fixture now uses `CC` and `0.8`, which are the same number, so the test asks only the clause's question.
