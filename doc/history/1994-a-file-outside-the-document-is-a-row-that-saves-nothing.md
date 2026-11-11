# 1994 — A file outside the document is a row that saves nothing

Session 1994, a single ledger round (§14.13.2, §14.13.4), the ledger lane. ADRs 2799 and 2800; Q828.

**The slice, and why.** `doc/questions/A800` item 2: audit every trigger gate (`doc/todo/76`, ADR
2800), then build the first effort-only one outside the other lanes' crates. That is A45's panel
row: the owner had given its shape, and its only gate, "when the attachments panel is designed",
had expired — `Query::Attachments` lists in four windows.

**The premise.** Held. 28 gates: 1 built here, 8 effort only (XFA's for its measurement alone), 3
other lanes', 5 owner-blocked, 3 outside releases, 2 measured and declined, 2 scope exclusions, 4
already built. The §7.6.5 handler
is round 1995's. A66's and A168's Readings and `doc/todo/01` and `63` now say what A800 lifted.

**Built.** `Query::ExternalFiles` answers with §14.13.2's external specifications from the catalog's
`/AF` and the page in view's, one row per name and relationship. It crosses the confined pipe as
kind 37. `viewer_host::panel::{file_rows, external_rows}` give each one a row: the name, "Outside this
document — <relationship>", and `Inert`. The rows go into `viewer-gtk`, `viewer-qt`, `viewer-ffi`
and `quorra`'s sidebar, and follow a collection's rows where the document states one.

**Tests.** `external_associated_files.rs` (two), `panel.rs`'s two row tests, `viewer-ui`'s files-tab
test and the confined round trip. **Plant**: page homes dropped, an `Extract` action, a clickable
sidebar row. Each fails its test; reversed, all four pass.

**Gates.** fmt `--check` on the seven crates: exit 0. Clippy `-D warnings --all-targets`: exit 0.
Nextest of the seven: 1191 of 1191, 7 skipped. The first pass failed `every_query_is_carried` on its
hand-written count of 36; now 37. Conformance after `git add -N`: exit 0. The first pass failed
`every_quotation_is_the_standards_own_words`: §14.13.2's blockquote sat under §14.13.4. `fuzz cargo
check --bins`: exit 0. `state.sh docs` and tier 2 by the map (`selection_census`,
`accessibility_census`, `awkward_classes`) exit 0, each
finished after the hand-back; `cargo doc -D warnings` over the seven crates: exit 0. `corpus` and `raster_golden` were not run:
neither `pdf-syntax` nor `pdf-model` changed. `launch_path` was not run: load 91–156. Not driven
under Xvfb.

**Left of the rows.** Both stay `implemented`, and their notes now name the panel. §14.13.5 to
§14.13.9's external carriers reach no caller; they are off the panel's path. `doc/todo/76` section 3
is the order of the remaining gates. Q828 asks for the owner-blocked ones and the exclusions.
