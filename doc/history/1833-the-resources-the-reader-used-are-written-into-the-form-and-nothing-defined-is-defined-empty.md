# 1833 — The resources the reader used are written into the form, and a name nothing defines is defined empty

Slot 2 of batch one hundred and twenty-four, 2026-10-27, an archive round (Sonnet, high), ADR 2490's
R04, run by two agents: the first built the remedy in an export and stopped at 252k context, and I
took its handover. ADR 2500 written; ADR 2501 unused (R03a, which `doc/questions/A694` item 3 now
allows, is owed and not started); no Q699. I worked in `scratchpad/r1833/`.
**Ledger.** No row moves: ISO 19005 has no ledger rows, and §7.8.3, §8.4.5, §8.10.2, §9.4.4 and
§9.6.4 were re-read in `doc/md/` at the clause each comment cites.
**Built.** `Rewrite::PageResources` widened (a form, tiling pattern or annotation appearance with no
`/Resources` is given its page's) and `Rewrite::EmptyResourceDefined` (an empty `ExtGState`, one
shared empty form, one shared empty Type 3 font where every show under the name moved nothing but
its widths); the survey gained `missing_in_streams` and `moving_shows`, and tracks `Tc` and `Tw`.
**Found.** (1) The puzzle of the handover was trap 230: `quorra-transform-worker` was stale and ran
the old archive code; `--bins` rebuilt it. No new trap. (2) 11 of ADR 2490's 22 convert, not 22: 7
need no font, 4 the empty font, the 11 refused each carry a second reason (the fence 6, the
`6.2.11` glyph and route rules 4, a Type 3 glyph procedure 1). (3) The survey's `peek` copied a
directly written category table per lookup, quadratic in the table: 50 000 names took 347 s and
1.3 s after reading the table in place.
**Evidence.** CLI over the 22 documents at 2b and 4, both profiles: 11 written, 44 files, 44 veraPDF
PASS; the empty font planted on veraPDF (without `/Widths` it fails 6.2.11.2); page 1 equal to the
source pixel for pixel except at most one level in one channel from the output intent on two
documents. Census with the CJK pack against HEAD's: the same 11 documents leave the refused list on
each of the eight lines, none enters it; the no-pack pair was not taken.
**Unfinished.** A copy of a form per page for pages whose resources differ, a Type 3 glyph
procedure's resources, a font whose text moved, a colour space, pattern or shading (no empty value):
refused by name, ADR 2500 says what each needs. R03a.
**Gates.** `cargo fmt --check -p pdf-transform -p pdf-archive` exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform
-p pdf-archive --all-targets` exit 0 in the export (HEAD and my changes) and, after the validator's fixes, in the worktree. Rustdoc `-D warnings --document-private-items` exit 0 (export). `cargo test -p
pdf-transform -p pdf-archive --no-fail-fast` exit 0 (worktree; archive 401 passed, 6 ignored). Four
plants (the define rewrite, the assign rewrite, the moving-show record and the survey bound each
disabled) each failed the test meant to catch it, 1 test in 7 each, 2 for the define rewrite. Gate
walks through `tools/state.sh --round 1833 archive` exit 0: the converter over the veraPDF corpus
125 s, the validator against it 0 missed and 0 over in all six flavours, the cross-check 0
disagreements in 13 560 selections. Census with the pack exit 0, 193 s; 44 of 44 veraPDF PASS.
`cargo run -p conformance --bin cited` and `--bin quotations` exit 0. `cargo test -p conformance
--no-fail-fast` exit 101: `every_quotation_is_the_standards_own_words` at `fonts/described.rs:581`
is round 1835's; after the fixes `--test records --test adr_status --test evidence` exit 0, and `cargo test -p pdf-transform --test archive` passes 407 (6 ignored). The validator's three behaviour findings (a standard-fourteen name erased, `"` unrecorded, a copy of the page dictionary per form) and its four paperwork findings were fixed by me. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on three behaviour findings — a standard-fourteen name defined as the empty font erased text the reader drew, the `"` operator's spacing escaped the moving-show bound, and a direct page dictionary was copied once per form — and on citations; the round fixed them, the validator's probes then held, and the orchestrator corrected the two sentences left.
