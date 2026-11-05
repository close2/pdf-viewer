# Q784 — May an operator name their CMYK press once, rather than on every conversion?

Source: the owner, in conversation on 2026-11-05, after Q780: "isn't gracol a us standard (wouldn't
fogra be better for Europeans?) (maybe make it configurable?)".

## What the tree does

- **The shipped press.** It is `data/icc/GRACoL2006_Coated1v2.icc` (ADR 1153), a US
  commercial-print condition. `data/icc/PROVENANCE.md` records why no European condition is
  shipped:
  - `PSOcoated_v3` (FOGRA51, ECI) forbids distribution without ECI's written permission.
  - `Coated_Fogra39L_VIGC` carries a copyright notice with no grant.
- **A press the operator supplies.** `quorra-transform --output-intent-profile <file.icc>`
  outranks the shipped one (ADR 2130), and the report names it as the operator's. The operator
  must give it on every run.

## The options put

1. **A persistent setting** (recommended): the operator names their CMYK press once, and every
   conversion uses it. A European company downloads FOGRA51 from ECI itself and sets it. The shipped
   default stays GRACoL, the only licence-clean condition.
2. Option 1, plus a FOGRA profile this program generates from Fogra's published characterisation
   data, if that data's licence allows distributing a derived profile.
3. Option 1, plus a warning when no press is configured on a European locale.
4. Leave as is.
