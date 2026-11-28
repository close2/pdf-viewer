# 3897 — Krovak Modified's polynomial is read from IOGP's registry

Session 2299. Status: **accepted** and built in `tools/epsg-table.py`. Context: A1095 (the registry
carried under IOGP's terms, values unchanged), ADRs 3821, 3823 and 3875, Q1114 (no form asking for
personal details); IOGP Guidance Note 7-2 sections 3.1.2.3 and 3.1.2.4.

## Context

The carried table is written from PROJ 9.9.0's SQL export of the EPSG Dataset. Its `conversion`
table has seven parameter slots, and Krovak Modified (1042) and its North Orientated form (1043)
state nineteen: the seven of Krovak, then the evaluation point (8617, 8618) and C1 to C10 (1026 to
1035). ADR 3875 built the polynomial and tested it on the section's printed parameters, and refused
the four systems that use it (5224, 5225, 5515, 5516) by the parameter they lacked.

What was searched before another source was taken: every file of PROJ 9.9.0's `data/sql/`. No row
holds 8617 or 1026 as a parameter of 5219, 5220, 5511 or 5512, and no table besides `conversion`
carries a conversion's parameters (`other_transformation` is for transformations, and
`transformations_czechia.sql` holds PROJ's own grid operations between the two S-JTSK systems). The
polynomial is not in PROJ's export.

## Decision

The twelve parameters come from IOGP's own registry, the EPSG Dataset's public API
(`https://apps.epsg.org/api/v1/Conversion/<code>/`). It asks for no account and no personal details,
and what it serves is the Dataset itself, which IOGP's terms of use (`LICENSE-EPSG`) govern
whichever copy it is read from.
`python3 -I -B tools/epsg-table.py --fetch` saves the four conversions' records beside PROJ's
archive, and the generator reads them under two checks:

- the SHA-256 of the record's parameter list (one `code value unit` line each, in the record's
  order, every number as the API's text spells it) must be the one `REGISTRY_SHA256` pins. The
  digest covers the parameters only, so a change to a record's usage or remarks does not stop the
  generator, and a change to a value does;
- the record's first seven parameters must be PROJ's seven, by code, value and unit, so the two
  sources are seen to describe one conversion.

The eighth parameter on is appended as the API's text spells it (values unchanged, A1095). A used
conversion of method 1042 or 1043 that `REGISTRY_SHA256` does not name stops the generator by its
code. Each refusal was planted and seen: one digest character changed, and one value changed in a
copied record with its digest re-pinned.

The registry serves the Dataset's current version, not v13.102. The four records' revision dates
are 2011-04-16, 2011-05-09 and 2023-01-17, all before v13.102. Their seven Krovak parameters equal
PROJ's, and the polynomial is the one section 3.1.2.3 prints to its ten significant figures.

## Consequences

EPSG 5224, 5225, 5515 and 5516 resolve. `krovak_modified_reproduces_the_examples_through_its_codes`
holds each to the section's grid point (westing 5568990.91 m, southing 6050538.71 m, negated for
the North Orientated forms) to the centimetre, and holds the reverse to the millisecond of arc. The
Greenwich forms take the same point 17°40′ west of Ferro. No parameter is now refused because the
table cannot hold it. ADR 3823's command leaves 13 of 5780 projected systems over 3 methods (1125,
1102, 1130).

The generator now reads two sources. A later PROJ tag that carries more slots would make the
registry records redundant, and the agreement check is what would show it.
