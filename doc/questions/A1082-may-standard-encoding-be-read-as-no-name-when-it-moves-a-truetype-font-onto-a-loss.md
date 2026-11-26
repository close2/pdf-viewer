Status: complete
Given: 2026-11-26, in conversation — transcribed by the round
Owes: read StandardEncoding as absent for every simple font (meeting ISO 32000-2 Annex D Table D.1 everywhere and passing standard_encoding_is_no_predefined_encoding_spec_audit), and give StandardEncoding its own repair identifier mapped on TrueType fonts to standard-truetype-encoding's rewrite and proof rather than generic font-maps-read-as-written

> Q1082: yes, please (Option 1)

Reading:
Option 1. Treat `StandardEncoding` as absent for every simple font so ISO 32000-2 Annex D Table D.1 is met everywhere (consistent with A865's rule that compliance follows the specification rather than corpus hit counts), and introduce a dedicated repair identifier for `StandardEncoding` that routes nonsymbolic TrueType fonts in the archive converter to `standard-truetype-encoding`'s mechanical rewrite and proof rather than `font-maps-read-as-written`, preserving lossless TrueType conversion and existing archive tests.
