# 1170 — Three censuses, and a departure that was never one

Date: 2026-09-22. Branch: `batch-1165-1170`, worktree `/home/AI/pdf-viewer-rounds`, shared with
five siblings (1165–1169). ADRs:
[1177](../adr/1177-a-departure-whose-witness-writes-the-name-somewhere-else.md),
[1178](../adr/1178-what-the-special-overprinting-mode-actually-reaches.md).

**§7.4.8's `/ColorTransform`, over the crawl.**
`crates/pdf-model/examples/colour_transform_census.rs`, 65 944 documents in 159 s: 792 516
`DCTDecode` images, of which **158 over 17 documents are in the case Table 13's entry decides and
none would draw differently if it were read**. Over the 974 that case is empty. The finding is
where the name is written — ADR 0036's deciding witness states `/ColorTransform 0` as a direct key
of the image dictionary, and both clauses that mention the parameter put it in `/DecodeParms`
(§7.4.1, §7.4.8), so the departure's premise was about a grep and not about the clause. The row
moves `departed` → `partial` (ADR 1177); ten of the seventeen documents read by hand
are the rule, and the zero is calibrated with two one-byte plants.

**§11.7.4.3's special overprinting mode, over the crawl.**
`crates/pdf-model/examples/overprint_ink_group_census.rs`, which ADR 1158 section 4 handed to
`doc/todo/23`. Every column is the interpreter's own verdict — `overprints()`, the commands whose
`blend()` is `BlendMode::Overprint`, the non-Normal group §11.7.4.3 builds around one, and
`Unsupported::Overprint` — so a change to the rule moves the number rather than the predicate, and
a page with the verdict and no mark is its own printed column. **1826 documents and 9863
pages paint under the mode — 2.8% of the crawl that opens — 145 documents and 493 pages of them
under a non-Normal blend mode, and not one page carries an `Unsupported::Overprint`.** So the
by-name refusal in `render-raster` and `render-gpu` is the largest coverage loss either carries.
The self-check found two defects in this example and one in the tree: the verdict is set where the
mode is computed rather than where a mark is emitted, so 177 of 10 040 pages are refused for a mark
that is not on them. ADR 1178; ten of ten witnesses read by hand are the rule.

**`tools/state.sh` grows the catalogue's gap**, which `doc/todo/66` asks for: the item is done when
a shipped profile produces no `does not carry out yet` note, and the sites with no code behind them
are where those notes come from. `section_remedies` prints, per PDF/A target, how many refusal
sites it binds and how many say their remedy is not built yet, taking the target list from the
program. Cheap, so it is in `quick`; `section_archive` calls it and already kept the converter's
per-target conversion counts.

Files: the two examples, `tools/state.sh`, `doc/verify.md`, `doc/conformance/ledger.toml` (§7.4.8
only), `doc/todo/23`, `doc/todo/65`, `doc/todo/README.md`, the two ADRs and this file.
