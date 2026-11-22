# 3369 — An MP4 file's sound track is read in-tree, in the confined media worker

Session 2167. Status: accepted. Context: RFC 0015 section 5 (step 8, "the MP4 demuxer's audit"),
RFC 0012 sections 2–4 (the audit method), RFC 0009's dependency table, ADRs 2428, 2452, 3297,
3349, `doc/questions/A946`; ISO/IEC 14496-12 (the ISO base media file format), 14496-14 (MP4),
14496-1 (descriptors) and 14496-3 (AAC), none held in `doc/`.

## Decision

1. **The demuxer is in-tree**: `pdf_media_worker::mp4::sound_track`, about 900 lines with its comments, reads the
   boxes an audio track needs — `moov`, `trak`/`tkhd`, `mdia`/`mdhd`/`hdlr`, `minf`/`dinf`/`dref`,
   `stbl` with `stsd`/`mp4a`/`esds`, `stts`, `stsc`, `stsz` or `stz2`, `stco` or `co64` — and
   returns the first enabled sound track's object type, its `DecoderSpecificInfo` bytes (for AAC
   the `AudioSpecificConfig`), its channels, rate and time scale, and every access unit's offset,
   size and duration. It decodes nothing; step 9's decoder takes that table.
2. **It runs in the worker, behind `Profile::Media`**, like ADR 3349's WAVE header: the host reads
   none of the file's structure.
3. **Declined, each with a sentence** (the module comment lists them): fragmented files, encrypted
   (`enca`) and non-`mp4a` entries, audio sample entries of version 1 or 2, a second sample
   description in use, media data in another file, tables that disagree with each other or with the
   file, and more than `MAX_ACCESS_UNITS` (2^20, about six hours of 48 kHz AAC) access units.
4. **Bounds**: each table's entry count is checked against the bytes its box holds before it is
   used; one size for every sample is checked against the file's length; the access-unit table is
   the only allocation that grows with the input; the walk visits at most `MAX_BOXES` (2^16) boxes.
5. **Fuzzed from the first commit**: `fuzz/fuzz_targets/media_mp4.rs`, seeded by
   `fuzz/seed_media_mp4.py` (thirteen files built from the layout, one per table form).

## The candidates (measured 2026-11-22; `cargo deny check licenses bans advisories sources`,
cargo-deny 0.20.2, the tree's `deny.toml`, one scratch package per crate)

| candidate | licence, `deny` | packages | Rust lines | `unsafe` (crate / graph) | C | last release | fuzzed | hostile counts | raw ASC + offsets |
|---|---|---|---|---|---|---|---|---|---|
| `mp4` 0.14.0 | MIT, passes | 23 | 10.4k | 1 / ~840 | none | 2023-08 | no | checked in `stco`, `stsz`; mixed elsewhere | no: three parsed ASC fields |
| `mp4parse` 0.17.0 (Mozilla) | **MPL-2.0, fails** | 20 | 8.4k | 0 / ~2 100 | none | 2023-05 (Firefox vendors the repo) | OSS-Fuzz | allocates from the count, fallibly | yes |
| `re_mp4` 0.5.1 | MIT, passes | 22 | 7.9k | 1 / ~760 | none | 2026-07 | no | mixed (`av01`'s `vec![0; size]`) | no: three parsed fields |
| `symphonia-format-isomp4` 0.6.1 | **MPL-2.0, fails** (four crates) | 14 | 6.9k | 0 / ~470 | none | 2026-08 | no | initial capacity capped at 1 024 | yes, packets not offsets |
| `mp4-atom` 0.17.1 | MIT OR Apache-2.0, passes | 26 | 31.5k | 0 / ~250 | none | 2026-11 | yes | initial capacity capped | yes |
| `shiguredo_mp4` 2026.5.0 | Apache-2.0, passes | 2 (none) | 22.3k | 0 / 0 | none | 2026-10 (date-versioned, frequent canaries) | yes | grows from the slice | yes |
| in-tree `mp4` | the tree's | 0 | ~900 (675 without comments) | 0 (`forbid`) | none | — | from this commit | every count against its box | yes |

RFC 0009's finding holds and extends: `symphonia` fails on MPL-2.0, and so does its MP4 crate
alone, through `symphonia-core`, `-common` and `-metadata`. The planted control (whole `symphonia`)
fails as it should, eleven MPL-2.0 crates. The `unsafe` graph figures are `grep -w` over every
lockfile package, proc-macros included, and are rough. Each row is reproduced by a scratch package
holding one dependency at its exact version and this tree's `deny.toml`; the session worked in
`~/scratch/agent/r2167/survey/`.

## Why in-tree, against the reviewed-crate precedent

The owner's bigint answer prefers a reviewed crate where in-tree code would be the risk. Two
crates pass every check here and are fuzzed — `shiguredo_mp4` (no dependency, no `unsafe`) and
`mp4-atom` — so the question was real. The in-tree reader is taken because:

- **The subset is small and already in the tree.** `pdf_model::media::container` walks the same
  sample tables (`stsz`/`stz2`, `stsc`, `stco`/`co64`, `stts`) for §13.2.4.3's chapter titles
  (ADR 2452), under the `media_container` fuzz target. A crate of 22 000 or 31 000 lines would be
  the third reader of boxes in the tree, for 900 lines of work.
- **The bounds are the tree's own and visible in one file.** Every count is checked against the
  bytes that hold it before anything is allocated (principle 3); of the candidates only
  `shiguredo_mp4` does the equivalent everywhere, and the others cap initial capacity, which bounds
  the first allocation and not the growth.
- **A fault is contained either way.** The reader runs in `Profile::Media`'s process; what a wrong
  table costs is a sound that does not play, said in the window, not a compromise. That is the
  difference from the bigint case, where a subtle error is silent and in the host.
- **The `mp4` and `re_mp4` crates are out on substance**: they parse the `AudioSpecificConfig` into
  three fields and do not return its bytes, which loses SBR and PS signalling that step 9's
  `rusty_aac` reads from them.

**The revisit condition**: if the in-tree reader is found wrong on a file a conforming producer
wrote, and the fix is more than a table rule, `shiguredo_mp4` is the crate to weigh first.

## Evidence

The 22 streams of ISO/IEC 14496-26's `compressedMp4` set (the held list below, fetched from the
ITTF bitstream directory RFC 0012 used; never committed) all read; the ignored test
`the_held_conformance_streams_read` in `crates/pdf-media-worker/tests/mp4.rs` reads them from
`PDF_MEDIA_MP4_STREAMS`. Each stream's access-unit count, byte total and first offset were compared
with `ffprobe -show_packets`, and its `AudioSpecificConfig` bytes with `ffprobe -show_data`: all 22 agree on all four — evidence about the reading, not its definition (principle 5).

Held list: al00_44, al00_48, al00sf_48, al01_48, al02_48, al03_48, al04_44, al04_48, al05_44,
al05_48, al06_48, al07_48, al08_48, al14_48, al16_48, al17_48, al18_48, al19_48, al_sbr_cm_48_2,
al_sbr_ps_01_new, am00_48, am01_48 — at
`standards.iso.org/ittf/PubliclyAvailableStandards/ISO_IEC_14496-26_2010_Bitstreams/DVD1/mpeg4audio-conformance/compressedMp4/<name>.mp4`.

**ISO/IEC 14496-12 is not held.** The ITTF site that listed it as freely available is closed; its
page says the free deliverables are now on the ISO and IEC webstores at no charge, behind a
checkout a round cannot make. The module paraphrases the layout by section number and quotes
nothing; the owner may fetch the free copy into `doc/` (`doc/questions/Q988`).

## What step 9 owes

- **The wire.** The demuxer is a pure function over the whole file; a request today is at most
  `MAX_RUN_BYTES` (256 KiB). Step 9 chooses how the file's `moov` crosses — the whole file under a
  larger request bound for a media clip, or the host showing the worker the top-level box headers'
  positions — and the reply carries the table, which the host checks against the file's length as
  ADR 3349's host checks a WAVE placement.
- **The decoder**: `rusty_aac` with `default-features = false`, fed the `AudioSpecificConfig` and
  each access unit (raw, not ADTS — RFC 0012 converted to ADTS with `ffmpeg -c copy` for its run).
- **The edit list** (`edts`/`elst`, 14496-12 8.6.6), which states the encoder's priming samples to
  trim; this reader does not read it yet, and a decoder that plays every sample plays the priming.

## Cost

About 900 lines in `pdf-media-worker`, 690 of tests, a fuzz target and its seed script. No new
dependency, nothing on the launch path, no wire change.
