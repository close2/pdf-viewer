# 1937 — An output intent this conversion adds says whether this renderer's drawing moves

Session 1937, a single archive round. ADR 2691; `doc/questions/Q780`; ADR 2692 not used.

**The diagnosis.** All 46 outputs of round 1936's six documents were drawn at 72 dpi beside their
sources, then drawn again with the catalog's `/OutputIntents` renamed. With it renamed, every
output draws pixel-identically to its source, so the intent is the whole cause. `cos` and `sin`
(sRGB2014): 266 pixels, one level. The profile's colorants are sRGB rounded to s15Fixed16, and
ISO 19005 section 6.2.4.3 admits `DeviceRGB` only through a profile, so this difference is
required. `issue9940` (5 172 pixels, at most 7) and `postscript_type4_many_outputs` (every pixel,
at most 76): ADR 1153 states the shipped GRACoL press, where this renderer drew `DeviceCMYK`
through its assumed inks, which no profile states. The banding is GRACoL's cyan clipping out of
sRGB at about 70 % of the ramp. `atan` and `issue17065` are identical.

**The defect and fix (ADR 2691).** Every stated intent said "this renderer already showed those
colours that way". That is false for a CMYK press the conversion adds, whether shipped or
supplied. `output_intent_sentence` now gives those two their own sentence. A profile the file
held, and any RGB or grey one, keeps the old sentence. No output byte changes. Which press the
archive states is the owner's choice: Q780 recommends a profile of the assumed inks for three
profiles, and GRACoL for `as-if-printed`.

**Tests.** `archive/drawn_through_the_intent.rs` (four): DeviceRGB ramps stay within a level;
shipped CMYK moves the drawing and says so, at both parts; a supplied press is named as the
operator's; a held CMYK profile keeps the sentence and stays within a level. Planting `true ||`
fails two of them.

**Census.** Against round 1936's runs, byte for byte once dates and identifiers are masked: nothing
is new, gone or changed. Shipped: 5 070 of 5 070 identical. Stating: 4 551 of 4 551. The rank does
not move. veraPDF 1.30.3: no census output changed, and the 22 direct outputs of `cos`, `issue9940`
and `postscript_type4_many_outputs` pass.

**Gates.** fmt (`pdf-transform`, `pdf-archive`): exit 0. clippy `-D warnings --all-targets`
(`pdf-transform`): exit 0. nextest of `pdf-transform` and `pdf-archive`: 1385 passed, exit 0.
Archive tier (`corpus`, `archive_corpus`, `witnesses`): exit 0 each. Both censuses: exit 0.
`cargo test -p conformance` after `git add -N`: exit 0 (the first run failed on an out-of-tree
path in the ADR's evidence, so the ADR now names the command). `state.sh docs`: exit 0.
