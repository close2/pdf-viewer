Status: complete
Given: 2026-11-05, in conversation — transcribed by the round
Owes: a persistent setting naming the operator's CMYK press once, used by every conversion where today `--output-intent-profile` must be given per run; the shipped default stays GRACoL (an archive round)

> Q784: Persistent setting (Recommended)

Reading: adopts option 1. The operator names their CMYK press once, for example a FOGRA51 profile
downloaded from ECI, and every conversion uses it, ranked as `--output-intent-profile` is today (ADR
2130). The shipped default stays `GRACoL2006_Coated1v2.icc`, because it is the only licence-clean
CMYK condition (`data/icc/PROVENANCE.md`). No profile is generated and no locale is consulted.
