# 2275 — A move by the leading is stated, and a damaged encoding CMap is written as read

Lane `gwg`, PDF/A: the archive converter's leftovers (records 2244, 2249, 2258, 2270, 2273). ADRs 3801
to 3804.

- **The leading** (ADR 3801): `as_drawn`'s walk follows Tl — 0 at a page's start (Table 103), unstated
  in a form — through `TL`, `TD` (`-ty`), `q` and `Q`, so `T*`, `'` and `"` are each Table 106's
  `0 -Tl TD` composed onto the line matrix, and a show after a cut that followed one is enclosed with
  the matrix stated. `"`'s `aw`/`ac` set `Tw`/`Tc`, which a `BT` does not reset: nothing restates them.
- **A form drawn in `DeviceGray` then `Lab`** (ADR 3802): no defect. The survey opens it twice, the walk
  starts it `UNCOUNTED` (ADR 3697) and no write draws both occurrences the same, so it stays refused;
  the class's sentence now says so (it said the operator "is written out"). `as-if-printed` photographs.
- **`Td` on a `Tm` matrix across a cut**: tested (`2 0 0 2 10 100 Tm 5 5 Td` restates `20 110`).
  **A section closed where a photograph stays**: 2258's two-stream test holds it (the `EMC` written,
  the page still photographed). **Inline image data**: a cut after `/AHx`'s EOD is tested.
- **Bit 26** (ADR 3803): `rich_text_value` logs `model/rich-text-malformed` where markup parsed and a
  token did not, or a reference was kept; plain characters log nothing.
- **`cmap-damaged`** (ADR 3804, superseding 3782's decision): `CMap::reads_alike` compares what
  extraction and selection read (codespace in order, mappings and notdef by length, WMode);
  `CMap::sections_as_read` writes the reading; `font_maps::encoding_cmap` writes a damaged map again
  and proves it by reading it back alike — every string cut and selected alike. The class joins
  `fonts/font-maps-read-as-written`. The reader is corrected: a name inside a section cleared the
  operands before it in silence; it is now an entry read past, the rest kept.
- **The report** carries `restated_font_maps`, the lines `font_maps::Restated::entries` always held.
- Fixture outputs (30) through veraPDF 1.30.3: all compliant.
- **Census** against 2273's (`r2273/`), with the pack: every profile line and per-reason document
  list as 2273's; no document moved. Without the pack: the same, every line as 2273's.
  veraPDF 1.30.3 over the 5523 outputs: 5471 compliant, 40 not, 12 unfinished, as 2273's.
- `pdf-archive --test corpus`: 0. `tools/state.sh --round 2275 launch`: queued behind
  the clock lock past the wall budget; its log is `r2275/launch.log`.
- Ledger: §9.7.5.3, §9.3.5, §9.4.2, §12.7.5.3, §8.6.8 and §8.9.7 stay `implemented`, gaining code
  and tests.
- Left, in order: a damaged map using `beginbfchar` (ADR 3428's route answers first); the leading
  into a form through the survey; a per-glyph TrueType instruction failure (no document).

**Gates.** Over `-p pdf-syntax -p pdf-font -p pdf-model -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 6370 of 6370, rustdoc `-D warnings
--document-private-items` (four touched crates, private directory) 0, fuzz `cargo check --bins` 0,
`cargo test -p conformance --no-fail-fast` 0 (485 passed); after the rebase onto `39521304e`.
