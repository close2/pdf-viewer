Status: complete
Given: 2026-11-05, in conversation — transcribed by the round
Owes: none

> Q784: Persistent setting (Recommended)

Reading: adopts option 1. The operator names their CMYK press once, for example a FOGRA51 profile
downloaded from ECI, and every conversion uses it, ranked as `--output-intent-profile` is today (ADR
2130). The shipped default stays `GRACoL2006_Coated1v2.icc`, because it is the only licence-clean
CMYK condition (`data/icc/PROVENANCE.md`). No profile is generated and no locale is consulted.

Built as `[profile] press = "<file.icc>"` in the operator's configuration, standing in for the
shipped GRACoL wherever `press-assumed` is taken, `--output-intent-profile` outranking it (ADRs
2704 and 2705).
