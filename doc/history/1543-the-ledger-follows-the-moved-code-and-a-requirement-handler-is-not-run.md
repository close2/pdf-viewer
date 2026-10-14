# 1543 — The ledger follows the moved code, a requirement handler is not run, and the bookmark root refuses by name

Slot 4 of batch eighty-six, 2026-10-14, a docs round. ADRs 1922, 1923; no question.
**Ledger: §12.11.5 `out-of-scope` → `implemented`** (ADR 1923).

**Premise.** Held: 49 rows named `render-cpu/src/lib.rs`, the §12.11.5 row stood at line 4798, and
ADR 1904 §3 named it; 38 rows named `pdf-model/src/image.rs`, 27 `viewer-qt/src/host.rs`, 25
`viewer-gtk/src/host.rs`, 0 `quorra/renderer.rs`. **Hypothesis.** Held in part. The rows name
files, not items, so each row's items were read from its note, its clause's citations in the new
modules and ADR 1914's and 1912's maps. Every item a note names still has a definition somewhere,
so no row lost its site to the move.

**(a) Paths.** 108 rows re-pointed; each old path is replaced in place, only by the modules holding
the row's items. A root is kept where an item the row means stayed in it: `rasterize`, `paint`,
`HIGH_PRECISION_PIPELINE` (14 rows), `decode`, `decode_parts`, `MAX_SAMPLES`, `sample_alpha` (12),
`Showing`, `standing`, `opening_commands`, root fields (Qt 3, GTK 15). Ten prose paths follow
(`render_cpu::images` → `reduced_images`, `image.rs::ccitt_rows` → `image/ccitt.rs::ccitt_rows`, …).
**Findings, older than the moves**: `transparency::knockout_group_elements` (§11.4.6, §11.6.2)
and `Decode::raw_of` (§11.6.5.2) name nothing at HEAD; `render-cpu/src/image.rs:85` still says
`images`; `input.rs` in both hosts calls selection "§12.4.2's", which is page labels.

**(b) §12.11.5, re-read.** At `on` the name tree runs, and a `JS` handler's `/Script` names an
entry of it; the `shall` to disable it was owed and unread. `requirements::disabled_handlers` reads
`/RH` under every requirement whose type is one of Table 275's (all checked); the open sequence
skips and reports each one. §12.11.1's and §12.11's notes no longer rest on the exclusion.

**(c) `bookmarkRoot`.** The root carries `parent` (null) and `doc`. The four mutators are Tier 2,
and the six outline reads are refused with their reason: the realm is not handed the outline (ADR 1922).

**Files.** `doc/conformance/ledger.toml`; `crates/pdf-script/src/{surface.rs,engine/bridge.rs}`,
`crates/pdf-script/tests/{fixtures.rs,hook.rs}`; `crates/pdf-model/src/requirements.rs`,
`crates/pdf-model/src/view/script_sites.rs`; `doc/state-of-play.md`; ADRs 1922, 1923; this record.

**Gates.** `rustfmt --check` on the six sources: 0. On an export of HEAD plus this round's seven files: clippy `-D warnings` `pdf-script --features engine --all-targets` 0 and `pdf-model --all-targets` 0; `cargo nextest run -p pdf-model` 0, 2 049 passed; `-p pdf-script --features engine` 0, 173 passed. Trap 13: with the root's installation and the skip removed, three tests fail (the two new ones and `every_refused_member_throws_not_allowed_error_by_name`). `cargo test -p conformance` in the worktree: 101, on a sibling's file alone (`viewer-qt/src/host.rs`'s `Host::arrivals`; §12.10.2's test name failed earlier and no longer); the ledger, quotation and names checks pass in the export. `--bin quotations` 0, `--bin pointers` 0. `tools/batch.sh check`: 1, on slot 1's AFM files alone. 4 600 s.
