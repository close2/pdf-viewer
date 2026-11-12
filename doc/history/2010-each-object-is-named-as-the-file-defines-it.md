# 2010 — Each object is named as the file defines it

Session 2010, a single round, PDF/A lane's worktree. The brief's three items: `freetexts.pdf`'s regression, §9.9.2
across the archive conversion's writers (ADR 2832's "What is left"), and ADR 2830 section 6's checker gap. ADRs
2838, 2839.

**The premise.** Half true. HEAD refused `freetexts` under all eight profiles at both parts on
`fonts/font-dictionary-states-type-font` alone. The file does hold an inline font: its update appends `1 2 obj`
(a FreeText the page's `/Annots` names) and `2 2 obj`, whose resources state `/Invalid_font` with no `/Type`.
Round 2005's binary typed it; round 2000's §7.3.10 reading of `Document::get` made `get(2 0)` null, and
`pdf-archive`'s examination still keyed every object at generation zero, so it named the holder `2 0` and the
converter, reading it, found nothing and refused.

**Built.** ADR 2839: `Examination::objects` keys each object at `Document::defined_id`, `reach` keys an arrival at
the reference's own identifier and finds an object only where number and generation agree, `metadata_streams`
reads the defined generation. `fonts/embedded-programs-define-every-glyph-shown` judges a shown code's glyph
against the program's own count (`maxp`, the `CharStrings` INDEX): an index past the last glyph defines nothing
(ISO 19005-2 6.2.11.4.1); 6.2.11.5 asks nothing of a glyph the program does not hold. ADR 2838:
`SubsetNames::name_program` names a subset held as a program, its bytes its identity; `Substitutes` carries one
record, the `machine_cid`, `rekeyed` and `pack` writers name through it, and both appearance preparations call
`for_annotation_in` with a copy of it. Ledger 9.9.2 and 9.9 stay `partial` (FDF imports), notes rewritten.

**Freed**: `freetexts` under all eight profiles at 2b and 4 (`/Type /Font` written into 2 2, lossless under
every profile). **Pixels** (CPU, 72 dpi, `/OutputIntents` renamed): each output against its source differs by at
most 2 in a channel, 232 to 238 px, no geometry.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 (`pdf-archive`, `pdf-transform`, `pdf-model`; 101 first:
`rekeyed::embed` at 111 lines); nextest of the three 0 (4145 passed, 47 skipped); four new tests each fail under a
plant; `fuzz` `cargo check --bins` 0; veraPDF 1.30.3: the 16 `freetexts` outputs PASS.
`cargo test -p conformance` 0 after `git add -N` (101 first: this record's placeholder Gates paragraph); archive tier
0 and 0; censuses against round 2006 (`cmp.py`): shipped 4 NEW (`freetexts` under `as-if-printed` and `as-shown`
at 2b and 4), stating 4 NEW (`no-data-loss`, `state-every-loss`), no GONE, every other output byte-identical
masked (5393 and 4799), so the generation cause took no other document and no subset was renamed; veraPDF the 8
NEW outputs PASS; `rank.py --table`: R10 7 to 6 at each part, sections 2 and 9 replaced;
`tools/state.sh --round 2010 docs` 0. Tier 2 not run: the round touches no renderer and no `pdf-font`
(`pdf-model` gains a method and `Clone` on a record).
