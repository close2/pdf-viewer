# Round 2167 — an MP4 file's sound track is read in-tree, in the confined media worker

RFC 0015 section 5's step 8, audited as RFC 0012 audited the decoders (ADR 3369, Q988).

## What it did

- **The audit.** Seven candidates in scratch packages under the tree's `deny.toml`: `mp4`,
  `mp4parse`, `re_mp4`, `symphonia-format-isomp4`, `symphonia` (the planted control, fails on
  eleven MPL-2.0 crates), `mp4-atom`, `shiguredo_mp4`. `mp4parse` and the `symphonia` MP4 crate
  fail on MPL-2.0 (RFC 0009's finding holds for the crate alone); `mp4` and `re_mp4` do not return
  the `AudioSpecificConfig`'s bytes; `mp4-atom` and `shiguredo_mp4` pass and are fuzzed. ADR 3369
  has the table and the argument against the reviewed-crate precedent.
- **`pdf_media_worker::mp4::sound_track`**: the first enabled sound track's object type, its
  `DecoderSpecificInfo`, channels, rate, time scale and every access unit's offset, size and
  duration, from `moov`/`trak`/`mdia`/`minf`/`stbl`/`stsd`/`mp4a`/`esds`/`stts`/`stsc`/`stsz`/
  `stz2`/`stco`/`co64` and `dref`. Every count is checked against its box before use; 2^20 access
  units and 2^16 boxes at most. Not yet called by `answer`: step 9 adds the wire.
- **Tests**: 20 in `tests/mp4.rs` from 14496-12's layout, every declined case by its sentence;
  an ignored test reads the 22 held ISO/IEC 14496-26 `compressedMp4` streams from
  `PDF_MEDIA_MP4_STREAMS`. All 22 read, and agree with `ffprobe` on count, byte total, first
  offset and the `AudioSpecificConfig` bytes (evidence, not definition).
- **Fuzz**: `media_mp4`, seeded by `fuzz/seed_media_mp4.py` (13 built files); `doc/verify.md`'s
  line, and `doc/stack.md`'s MP4 demuxing row.

**Premises.** The brief's "fetch 14496-12 if it is on the ITTF list": the ITTF site is closed and
the no-charge copy is behind the ISO/IEC webstore's checkout, so it is not held (Q988). The
14496-26 bitstreams RFC 0012 named are still served from their old path.

**Left, in order.** Step 9: the wire for a file past `MAX_RUN_BYTES`, `rusty_aac` on raw access
units, the edit list's priming trim (`elst`); Q988's text check of the paraphrase.

**Ledger.** No row moves: §13.2.1 stays `out-of-scope` until video is opened; §13.3 is unchanged.
**Gates.** One `-p` set, `pdf-media-worker viewer-qt`: fmt exit 0; clippy `-D warnings
--all-targets` exit 0; `pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` under `xvfb-run
-a`: 104 run, 104 passed, 2 skipped, the audit manifest's difference empty; rustdoc `-D warnings
--document-private-items` exit 0; `cargo deny check` exit 0; `fuzz` `cargo check --bins` exit 0,
clippy of `media_mp4` exit 0 (the whole fuzz crate's clippy fails in `cms`, `fragment`, `saml`,
`vfs_write`, none touched here); `tools/fuzz.sh media_mp4` 600 s, 18.3M runs, no artifact, ft
272 → 923; `cargo test -p conformance` exit 101 on this record's draft, `evidence` and
`state_sections` (both fixed), then the whole suite exit 0.
