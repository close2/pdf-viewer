Status: complete
Given: 2026-11-25, in conversation — transcribed by the round
Owes: log lone UTF-16 surrogate destinations in /ToUnicode as font/to-unicode-damaged, and in the same round have the converter's /ToUnicode rewrite drop ranges whose destinations fall in the surrogate block (D800–DFFF) as a loss of no character so all nine affected corpus documents remain written at PDF/A-2b

> Q1051: agreed

Reading:
Adopt the recommendation. Log `/ToUnicode` ranges whose destinations fall in the UTF-16 surrogate block (`D800–DFFF`) as `font/to-unicode-damaged` (closing the silent-repair gap), and in the same round have `pdf-archive`'s `/ToUnicode` rewrite drop those surrogate-destination ranges, stated as a loss of no character (since lone surrogates name no Unicode scalar value), so the nine affected corpus documents continue converting cleanly to PDF/A-2b.
