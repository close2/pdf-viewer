# Round 2206 — probes that see the filter, and H.264 in the media worker

Two items in the `audit` worktree: `pdf-sandbox`'s confinement probes, then video's third round
(RFC 0015 section 6).

## What changed

- ADR 3525: every probe asserts the filter's `SIGSYS` (`assert_the_filter_kills`), runs under
  `MALLOC_ARENA_MAX=1`, forgets what a served call returns (no profile admits `close`); the ring's
  second-descriptor probe passes only on `MSG_CTRUNC`. Unplanted, every probe ended `SIGSYS` except
  `media-thread`/`media-exec` (exit 17, `ENOMEM`). Planted (`openat`, `socket`, `bind`, `fcntl`,
  `mprotect`, the thread calls admitted; Media's descriptor ceiling +1), the old assertion missed
  eight probes (Landlock, `EMFILE`, `ENOMEM`, a kill from `close` after `socket` and `bind` were
  served); the new one fails all six probing tests. **No hole**: no probed call is on any list.
- ADR 3526: `video::Session` over `rusty_h264-decoder` 0.17.3's scalar build; `mp4::movie_video_track`
  reads `avc1`/`avc3` and `avcC`; samples rewritten to Annex B; output order held to sixteen
  pictures; interlaced streams `VideoError::Interlaced`, handed off by name. RFC 0012's checks
  re-run; `wide` and `safe_arch` (312 and 859 `unsafe` lines) are still in the scalar build: Q1027.
- ADR 3527: the movie (`FRAME_VIDEO_MOVIE` 13 → `FRAME_VIDEO_TRACK` 14 or `FRAME_INTERLACED` 16),
  then runs as `FRAME_PICTURES` (15) streams, each picture into its ring slot, a closing with a
  status; `MediaWorker::decode_mp4_video`; the host checks every picture. Fixtures built by hand
  from H.264's syntax (`I_PCM`, all-skipped P), so expected values are the standard's.
- Conformance: `tools/fetch-h264-conformance.sh` (135 zips pinned by SHA-256, not committed);
  `tests/h264_conformance.rs`, ignored: 67 EXACT, 52 handed off, 4 differ (same as the crate's own
  whole-stream decode), 12 refused (FMO, SP/SI, B without references, explicit B weighting, 4
  truncated multi-reference).
- ADR 3528: `MEDIA_ADDRESS_SPACE_LIMIT` 64 → 256 MiB; measured `VmPeak` 129.6 MiB at 1920×1088 with
  sixteen references, sixteen held and the ring. At 64 MiB that clip died. Fuzz target `media_h264`.

**Ledger.** No row moved: clause 13 video rows stay `out-of-scope` until a window plays video.

**Gates.** fmt 0; clippy `-D warnings --all-targets` (five crates) 0; `cargo build -p pdf-sandbox
--bins` 0; nextest `--lib --bins --tests` (five crates, `xvfb-run -a`) 657 passed, 0 failed —
`expected-failures-current.json` empty, difference none; `cargo deny check` 0; rustdoc `-D warnings --document-private-items` (`pdf-sandbox`,
`pdf-media-worker`) 0; `fuzz` `cargo check --bins` 0; clippy on `x86_64-pc-windows-gnu` and `-msvc`
(`pdf-sandbox`, `pdf-media-worker`, `confined-transport`) 0 and 0; `cargo test -p conformance
--no-fail-fast` 0 (478 passed, 0 failed).

**Left.** RFC 0015 section 6 round 4 (YUV to RGB on the device and in `render-cpu`); the 16 streams
the decoder does not match, to report upstream; Q1027.
