# 2095 — The golden's sixty-three moved pages are restated, each traced to its commit

Session 2095, an oracle round in `pdf-viewer-audit`: the train at `594ae80f` held 911 and moved 63. ADR 3083 is
the table; ADRs 3084 and 3085 and Q917 unused.

- **Method (ADR 3051 §1).** Every commit from `5adc3d83` to `594ae80f` that changes a crate `pdf-model` depends
  on was built on this machine in a scratch copy, its golden written by the walk's update mode: `5adc3d83`,
  `66c33750`, `fb3699c3`, `5506296e`, `8a995579`, `fa29fe4c`, `5fac2b49`, `3b78a396`, then `594ae80f` in the
  worktree (`122d9952` touches viewer crates only, none in `pdf-model`'s tree). A scratch example planted only in
  those copies dumped each page's list and reports as text and its raster as bytes at `5adc3d83`, `66c33750` and
  `fb3699c3`; its digests equal each commit's golden for all 63, so the dumps are the golden's measurements.
- **At `5adc3d83` all 63 are the file's own lines**; each then moves at one commit only.
- **55 list only, `66c33750` (ADR 3024):** the only added lines are `anti_alias: false,`, none removed; raster
  bytes and reports identical.
- **`issue17065`, `issue18529`, `66c33750`:** the same field, `true` from the file's own axial `/AntiAlias true`;
  rasters differ by at most 2 and 1 levels inside the shaded area, in columns across the ramp: Table 77's filter.
- **Five reports only, `fb3699c3` (ADR 3046):** one report's wording, "every other code by Table 120's
  /MissingWidth" to "107 by the drawn face's own glyph or its .notdef"; lists and rasters identical.
- **`poppler-90-0-fuzzed`, `fb3699c3`:** 764 glyph translations move by 0.3586 = 250/1000 em at its 1.43462
  scale: code 0 of a descriptor-less Times font (`/FirstChar 1`) now advances by the drawn face's glyph (§9.2.4);
  553 pixels of the last row change.
- **`594ae80f` (ADR 3074) moved nothing:** its golden equals `3b78a396`'s on all 974 pages.
- ADR 3024's "No golden moves" held of pixels only: the field is in the `Debug` text the golden digests.
- `issue19389` (ADR 3051 §5's re-check after round 2087) is held through every commit; `issue19517` moved at
  `fa29fe4c` and back at `5fac2b49`, and is held.
- **Restated:** one update run in the worktree, whose diff is exactly the 63; no line written by hand. The next
  check holds 974 and moves 0.
- **Trap:** a `pkill -f <script>` from a shell whose own command line names the script kills that shell.

**Gates.** `raster_golden` in update mode 0 (held 911, moved the 63), then in check mode 0 (held 974, moved 0).
Over `-p pdf-model`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0, nextest `--lib --bins
--tests` 0 (2874 passed; the manifest expects none), rustdoc `-D warnings --document-private-items` 0.
`cargo test -p conformance` 101 once (`records`: this record had no gates paragraph yet), then 0 for `--test records`.
