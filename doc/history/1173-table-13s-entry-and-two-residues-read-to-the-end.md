# 1173 — Table 13's entry, and two residues read to the end

Date: 2026-09-22. Branch: `batch-1171-1176`, worktree `/home/AI/pdf-viewer-rounds`, shared with five
siblings (1171–1176). ADRs:
[1183](../adr/1183-table-13s-entry-is-read-and-one-reading-stays-a-departure.md),
[1184](../adr/1184-who-the-jpx-baseline-restriction-binds.md).

**§7.4.8's `/ColorTransform` is read, and the row is `departed`.** `decode_jpeg` takes the entry out
of the filter's own `/DecodeParms` and `image::colour_transform` ranks Table 13's three cases in the
clause's order. It was not the one `if` ADR 1177 priced it at: `zune-jpeg` reports the marker's value
and its own defaults through one `input_colorspace()`, so `image::adobe_transform` reads the Adobe
`APP14` segment off the header segments; the decoder has no switch for declining a transform, so
"No transformation" is spelt by asking it for the space it read in; and a three-component frame's
input space is *provisional*, so the untouched space is named rather than echoed or the decode asks
four channels from three (ADR 0266's defect, one line away). `ycbcr_to_rgb` is new and shares
`jfif_inverse` with `ycck_to_cmyk`.

The identifier reading moved with it: `R`, `G`, `B` component identifiers now decide only in the
clause's third case, where the ranking ends without an answer. That case is the one departure left
and it makes the row `departed` rather than `implemented` — the default of 1 is addressed to a
processor and is not conditioned on the file conforming (ADR 1183) — and `tests/colour_transform.rs`
holds it as a fixture, sixteen scenes, five failing if the ranking is removed.

**§7.4.9 loses one of its two residues, by reading rather than by building.** The clause states its
colour-space restriction three times and only the third is addressed to a reader: the data and image
sentences bind whoever writes them, the processor sentence asks for *support* rather than a refusal.
So "the restriction is not checked" is struck; what is left is the thirteen codestreams that decode
one level off `opj_decompress` (upstream) and that support obligation, unmeasurable against a set
ISO/IEC 15444-2 defines and the owner declined to buy. The three entries this round was asked to
close were already executed and tested; they are re-checked, not rebuilt. ADR 1184.

**§7.4.6's `/DamagedRowsBeforeError` is blocked still, and the check is written down.**
`hayro-ccitt`'s `decode` answers `Result<usize>` whose count exists only on success, `DecodeError`
carries no position, `BitReader::byte_pos` is crate-private and `DecoderContext` has no accessor.
0.3.0 is the latest release and the pinned fork revision is the head of that crate's history.

**Measured.** `raster_golden` held 974, moved 0; `pdf-model --test corpus` every ratchet at its
ceiling, slack 0; `colour_transform_census` over 65 944 documents reproduces 158 in the deciding
case, 0 of which would draw differently. Files: `crates/pdf-model/src/image.rs`, `tests/colour_transform.rs` (new), `tests/oracle.rs`,
`doc/conformance/ledger.toml` (§7.4.6, §7.4.8, §7.4.9), `doc/todo/65`, the two ADRs and this.
