# 1889 — Text under a null `/Font` entry draws in a substitute where the reader asks, and is reported at both answers

Slot 9 of batch 129, 2026-10-29, Opus 5.5, one session. ADR 2612 used; ADR 2613 and Q765 unused. Ledger: §9.3.1
stays `implemented`; its note and test list now carry A738's setting.

**Built.** A738's owed option to A35: `ViewState::set_null_font_substitute(bool)`, off by default and never set in
`pdf-model`; `viewer_core::Command::NullFontSubstitute(bool)` pushes it into every open document and every one opened
afterwards (the `Separations` shape, ADR 1228) and crosses the confined wire as tag 46. Where `Tf` or Table 57's
`/Font` meets an entry that resolves to §7.3.9's null and the reader asked, the text draws in §9.6.2.2's Helvetica
at that face's own advances (§9.6.2.1's widths a processor "shall provide"), cached once a page under
`FontKey::NullSubstitute`; an entry that is present and not a font dictionary, and a name nothing states, are not
substituted. The show says ADR 2550's sentence whole at both answers and names the substitute under the second.

**Premises.** A35 and A738 held: the refusal and its report were in `content/font.rs`, no option existed, and the
compiled-in fourteen were the substitute already present. The hypothesis (one setting, one site) held with two sites
(`Tf` and Table 57) and the show. The brief's "Table 32" did not: it is the name dictionary; the clause's tables are
Table 34 (`/Font` maps names to font dictionaries) and Table 103. Q35's "/MissingWidth-style default advances" is
read as the face's defaults, not one constant (ADR 2612 section 2.3) — a reading the owner may revisit.

**Plant.** Dropping the substitute's sentence failed 2 of the 6 new tests; dropping the report at a substituted show
failed 3. **Bound.** No new loop; 200 000 `Tf` under the null name with 800 000 glyphs reached 184 MiB of
`getrusage` high-water under the setting, 186 MiB for the same stream through a stated Helvetica, 37 MiB off.

**Gates.** In an export of HEAD plus this round's files (a sibling's mid-edit broke `pdf-font` in the shared build):
`cargo fmt --check` on pdf-model, viewer-core, viewer-confined, viewer-ui exit 0; clippy `-D warnings --all-targets`
on the four exit 0; `cargo nextest run -p pdf-model -p viewer-core -p viewer-host -p viewer-confined` 3230 passed;
`raster_golden` (gates) 2 passed, exit 0; the six arms by page against HEAD's six arms: cpu-1x, gpu-1x and
compute-1x 0 of 968 moved, cpu-4x, gpu-4x and compute-4x 0 of 965 (the comparer sees 965 of 968 moved between HEAD's cpu-1x and cpu-4x). In the worktree:
`--bin cited` exit 0, `--bin quotations` exit 0 with none of this round's quotations diverging; `cargo test -p
conformance --no-fail-fast` one red, `records` on session 1885's record, not this round's.

**Looked at** (trap 1): the fixture rendered to PNG at 4× draws the null entry's line in the setting's Helvetica
glyph for glyph as the stated Helvetica line beneath it, and draws only the stated line with the setting off.

**Unfinished.** No window or command-line word offers the setting (not this slot); a C caller has no entry for it.
