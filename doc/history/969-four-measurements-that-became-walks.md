# 969 — Four measurements that became walks, and a datum with one reader fewer than its census said

Date: 2026-09-11. ADR: 0980.
Files: `crates/pdf-model/src/icc.rs`, `crates/pdf-font/src/predefined.rs`,
`crates/pdf-font/src/standard.rs`, `crates/pdf-font/src/standard_metrics.rs`,
`doc/conformance/ledger.toml`.

ADR 0971's census of `data/` had seven rows and three assertions. The other four were numbers in a
document, which is the form `CLAUDE.md` spends a whole section arguing against: a fact that can be
counted is not written down, the command that counts it is. This round wrote the four commands.

They are unglamorous and that is the point — every one of them passed on the first run, because
`data/` is clear of every bound it reaches by two to three orders of magnitude. What changed is
that the clearance is now a thing that can *fail*, on the day somebody drops a newer edition of
Adobe's `CMap` files or the ICC's profile into the tree. The shape is ADR 0963's throughout: walk
the whole carried population, then follow one datum through to the value the bound decides, so a
bound raised just far enough to stop a flag firing still fails. Each was calibrated against the
defect it looks for, both halves separately, per trap 13.

The one that had the widest silence under it is `predefined.rs`'s `MAX_DEPTH`. It stops a `usecmap`
chain by answering `None` for the *base*, and the map is then built without every code the base
stated — §9.7.6.2 answering nothing for them, with no report anywhere, because the predefined path
has no document to blame the way `composite.rs`'s embedded reader does. Eighty of the 239 carried
files name a base; the deepest chain is two against a bound of four; and **no file names a base
this binary does not carry**, which nothing had ever asked and which would have produced exactly
the same base-less map.

The fourth walk is the one whose measurement is not "nothing is missing". Eighty-six of
`Times-Roman`'s 315 metric names reach no glyph in the Foxit face, and ADR 0971 had already
established that this is not a defect. A list of 86 names is not assertable and the count is a
ratchet over nothing, so what the walk asserts is the **line**: a name whose Adobe Glyph List
character is U+00FF or below has a glyph in all fourteen faces, and everything absent is above it
or is not in that list at all. That fails when a face loses `eacute` and stays quiet when one gains
`Abreve`, which is the asymmetry worth having.

Then ADR 0971's other outstanding item, the two-readers sweep over `data/standard-fonts/`. Its
habit said the datum had three readers — `sfnt`, `cff`, `type1` — plus a fourth place for the
tables. Both halves of that were wrong. **`type1.rs` is not a reader**: the ten files are named
`.pfb` and are bare CFF programs, so the third reader in the list was a *file extension*. And two
readers live outside `pdf-font` altogether — `viewer-ui`'s chrome, which sets the program's own
panel text in these faces and is loud by construction, and `viewer-ui/tests/notices.rs`, which
walks their `SHA256SUMS`. Neither is a gap. What the round takes from it is the habit one level up,
in the ADR for the orchestrator to place: a reader list assembled from one crate's imports is a
list of that crate's readers.

Nothing was raised. Of `cff.rs`'s two bounds, one is never reached by this datum at all — no
compiled-in face is CID-keyed, so `calls_local_subr`'s depth is never asked — and the other was
A/B'd: raising `MAX_INLINE_DEPTH` from 8 to 64 changes not one of the handful of glyphs whose
advance cannot be restated, which a loud `CffError` names anyway. Both measurements are in the ADR
so that the next round asking does not have to.
