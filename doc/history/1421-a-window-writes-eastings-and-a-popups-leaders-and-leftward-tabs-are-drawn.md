# 1421 — A window writes a projected `/DCS`'s easting and northing, and a popup's leaders and leftward tabs are drawn

Host slot of batch sixty-six, 2026-10-08. ADRs 1678, 1679. No ledger row moved (Table 269's and the
popup's rows are not this slot's). No question written.

**Premise.** Held: `located.rs` called `display_position`, `Geospatial::display` answers
`Displayed::Projected`, and round 1415 left a right-to-left tab and a leader said. Not as stated: a
leader never reached a host — `popup/rich.rs` read it and turned it into a sentence — so it needed a
hunk there (`RichTabStop::leader`, `RichLeader`, `RichLeaderPattern`, `RichRuleStyle`, the sentence
removed), the re-export in `popup.rs`, and one line in slot 5's `rich_text.rs` (`parts` names
`Leader`, `LeaderPattern`, `Linear`, `RuleStyle`). `RichTabStop` and `viewer_host::popup::TabStop`
are no longer `Copy`.

**Built.** (ADR 1678) `Located::At::display` carries `pdf_model::geospatial::Displayed`; every
window writes `easting … Meter, northing … Meter` to two places (`viewer_host::measuring::grid`);
the confined wire's display byte gained value 3, greeting `PDFVCF10`. Drive step 58 in all four
windows. (ADR 1679) `quorra` lays a right-to-left paragraph's tabs leftward — its default stops lie
left of the leftmost stated one, the cursor starts at the line's right edge — and draws dots,
content and solid, dashed and dotted rules as a field's leaders are drawn (ADR 1660). Both toolkits
say both (`toolkit_unapplied`); Pango was measured to place a right-to-left stop from the start
edge (a stop at 100 of 400 put the text at 300), Qt's was not measured. The C ABI hands no leader,
so `viewer-ffi`'s `abi_note` says one, which a C caller had read from `pdf-model` before.

**Gates.** `rustfmt --check --edition 2024` on the 12 touched `.rs` files: exit 0 (`rich_text.rs`
hand-formatted, its children being slot 5's). `cargo clippy --all-targets` on `viewer-core`,
`-host`, `-ui`, `-confined`, `-ffi`, `-gtk`, `-qt`: no lint in their files, exit 0; with
`RUSTFLAGS="-D warnings"` it stops at slot 5's mid-edit `pdf-font/src/pairs.rs` (exit 101), and
`pdf-model`'s two remaining lints are in slot 5's `layout.rs` and `style.rs`. `cargo nextest run` on
the five core host crates: 849 passed; `viewer-ffi` and `viewer-host` after the ABI hunk: 250
passed; `viewer-gtk`, `viewer-qt`: 47 passed; `pdf-model --lib popup`: 23 passed. Planted (trap 13):
the leftward branch and the leader draw each disabled, both new panel tests failed; reversed by
patch. `cargo test -p conformance`: 405 passed, exit 0 (an earlier run failed two of slot 6's
mid-edit `bounded.rs` tests). Behind the lock, the release build and `tools/drive-windows.sh`: exit
0 after 964 s, 4.95 GiB peak; 203 works, 0 wrong, 0 to look at, 6 not offered (the floor's 4 and
steps 54 and 55 in `quorra-gtk`). Looked at (trap 1): step 58's photograph, and the two leaders and
the leftward tab rasterised from the panel tests. `accessibility_census` not run: no node changed.

**Unfinished.** A right-to-left tab in the two toolkits: Pango's stops would have to be handed per
allocation as distances from the right edge, a change to the note being a label; Qt's placement is
unmeasured. A leader in the two toolkits: neither API has a fill. The C ABI carries no leader.
