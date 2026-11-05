# 1951 — A `glyf` program under a `CIDFontType0`, and a collection nothing reads

Session 1951, a single archive round on R10. ADRs 2716 and 2717.

**The premise, checked per document against round 1949's census.** Of R10's 13, the `glyf`-under-
`CIDFontType0` three are one: `issue9949` (a TrueType program under `/FontFile3 /CIDFontType0C`, its
descendant written inside the `Type0`); `issue215` is a CFF OpenType under a simple `TrueType`, and
`issue11740_reduced` a Type 1 under a `CIDFontType0`. The `CIDSystemInfo` three held, but
`issue11768_reduced` also stops at R03a. `hesr13613` is the defect and also a font inside another object.

**Built.** ADR 2716: a `CIDFontType0` over a `glyf` program becomes `CIDFontType2`, its map kept or
written as the `Identity` this reader read, the program named `/OpenType` under its own key, in the
font or in its `Type0`'s `/DescendantFonts` (`font_files.rs`, `rewrite/values.rs`); a lossless
restatement under `font-program-refiled`. ADR 2717: `collection-restated`, a stated interpretation,
writes the `CMap`'s `/CIDSystemInfo` into the `CIDFont` where the program is embedded, no `/FD`, and
neither collection is one this reader holds a Unicode table (or, vertical, forms) for
(`collections.rs`); the fence in `decision/fonts.rs` becomes a remedy.

**The defect.** `restate_metrics` reached a program only through a descriptor that is an object, so
`hesr13613`'s fonts, whose descriptors are written inside them, were skipped and the refusal said no
font needed restating. It now says the descriptor is written inside the font.

**Proof.** `issue9949`, `issue18117`, `issue9534_reduced` drawn at 72 dpi on the CPU beside their
sources: 0 to 7 levels apart under the four profile-part pairs, and 0 everywhere with `/OutputIntents`
renamed (ADR 2691); veraPDF 1.30.3 passes all 12 outputs.

**Tests.** `font_files`: the retyping, a map naming `/Other` refused, the no-row refusal moved to a
CFF OpenType under `TrueType`; `collections`, 3; `rewrite::values`, 2. Plants: the retyping off fails
2, the embedding guard off fails 1.

**Census.** Both rerun. Shipped against round 1945: 12 NEW (the three, both profiles, both parts), 5105
of 5105 the same. Stating against round 1943: 12 NEW, 4561 of 4561 the same. veraPDF: 24 of 24 pass.
`rank.py --table`: R10 frees 7 and 7, stops 9 / 9 under `as-shown`.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-transform`): exit 0. nextest `pdf-transform`
and `pdf-archive`: 1422 passed, exit 0. Archive tier: `pdf-archive` corpus 0, `archive_corpus` 0.
`cargo test -p conformance` after `git add -N`: exit 0. `state.sh docs`: exit 0. Round 1949's clocks
on HEAD, `launch_path` and `turn_path`: exit 101, refused before measuring by the hung-section marker
`/run/lock/gpu.lock.pdf-viewer-hung` (08:51 today, ADR 2665), which a person removes.
