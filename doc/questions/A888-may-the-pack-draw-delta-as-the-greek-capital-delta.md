Status: complete
Given: 2026-11-17, in conversation — transcribed by the round
Owes: give the Latin-Greek-Cyrillic font pack builder a fallback table for the glyph names AGL 1.x double-mapped (Delta -> U+0394, Omega -> U+03A9, mu -> U+03BC, etc.) so that where the source Noto face has no glyph for the AGL 2.0 code point it copies the Greek letter's glyph, updating the pack's digests and tests (a font / PDF/A round)

> Q888: agreed with your recommendation (option 2)

Reading: overrides Q888's conservative "No, for now" recommendation and authorizes building the AGL 1.x Greek-letter fallback table in the pack builder: where the source Noto face lacks a glyph for the AGL 2.0 code point of a historically double-mapped PostScript glyph name (`Delta` / `U+2206`, `Omega` / `U+2126`, `mu` / `U+00B5`, etc. — which were the sole glyph names in PostScript Standard 14 fonts such as `Times-Roman` and remain the canonical names in Adobe's AGLFN), copy the corresponding Greek letter glyph (`U+0394`, `U+03A9`, `U+03BC`, etc.) into the pack and update the pack's digests so documents like ADR 2630's Greek file draw real text instead of being photographed.
