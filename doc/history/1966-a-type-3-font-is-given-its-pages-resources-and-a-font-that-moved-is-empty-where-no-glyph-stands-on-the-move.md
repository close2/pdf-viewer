# 1966 — A Type 3 font is given its pages' resources, and a font that moved is empty where no glyph stands on the move

Session 1966, a single round, PDF/A lane's worktree. R04's last two shapes, ADR 2746.

**The premise.** Held: each of the six fails only 6.2.2 under R04, five at a font that moved
(`020747`, `027613`, `189478`, `219789`, `492484`, govdocs files that set `Tc` under a damaged
font name and then `TD`) and one at a Type 3 glyph procedure (`pr4922`, the `3` glyph naming
`/R98`, which only page 2 defines and shows). R04 is the last refusal only for `pr4922` (all four),
`189478` (`as-if-printed`, both parts) and `020747` (`as-if-printed`, 4); the rest stop at the
operator fence, 6.1.13 and the font rows.

**Built (ADR 2746).** A Type 3 font stating no `/Resources` is given the object its pages state,
or the union of the pages' bindings where no page binds a name differently and no page showed a
glyph whose name it missed and another page defines (§9.6.4, §7.8.3); the survey now records which
procedures each page's shown codes select. A name no page defines is ADR 2500's empty resource in
it. A font that moved: §9.4.4's `TJ` and line moves do not depend on the font, so only a spacing
moves the empty font where the reader stood still, and the survey records the show only when a
defined font's glyph is placed from that position before `BT`, `ET`, `Td`, `TD`, `Tm`, `T*`, `'`
or `"`. Rewrites: ADR 2500's two, losses none.

**Freed**: `pr4922` under both profiles at 2b and 4, `189478` at 2b and 4 and `020747` at 4 under
`as-if-printed`. At 72 dpi `pr4922` equals its source (also with `/OutputIntents` renamed);
`189478` (154) and `020747` (255) differ through the photograph (every page of `189478`, six of
`020747`'s) and the embedded faces, and their empty fonts are on photographed pages. **Left**: a
Type 3 font read differently per page (a copy per page, owed, no census document), a spacing a
glyph stands on, a clipping mode.

**Tests** (`archive/reader_resources.rs`): ten moved shows written with the empty font, each
conforming and drawing the source's dark-pixel mask; five refused; the union given and drawn as
the source; three union shapes refused; an empty graphics state in the font. Plants: the pending
show never recorded fails four tests, the mask among them; the union check skipped fails one.

**Census** (both profiles, against round 1965's outputs): shipped 7 NEW, stating 7 NEW, 0 GONE,
0 CHANGED, each NEW one of the outputs above; R04 leaves `rank.py`'s table. veraPDF 1.30.3 passes
the seven CLI outputs and the seven stating ones; with the font's `/Resources` removed `pr4922`
fails 6.2.2-2.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-archive and pdf-transform; nextest
of both 0 (1436 passed); archive tier 0; both censuses 0; `cargo test -p conformance` 0 after
`git add -N`; `tools/state.sh --round 1966 docs` 0.
