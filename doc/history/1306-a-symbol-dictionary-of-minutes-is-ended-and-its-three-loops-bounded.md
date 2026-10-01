# 1306 — A symbol dictionary of minutes is ended, and its three loops are bounded

Robustness slot of batch forty-seven. ADR [1447](../adr/1447-three-loops-of-a-symbol-dictionary-are-bounded-in-a-patch-and-an-in-process-decode-has-a-deadline.md). No question.

**The text.** ITU-T T.88 (08/2018) is offered only inside a zip with its conformance data; the PDF
link answered `500`. Held at `/home/AI/specs/T.88.pdf`, prepared to `doc/md/T.88.md`, entered in
`doc/third-party-data.md` with both hashes; cited and paraphrased only.

**The cause** (ten minutes of the `jbig2` target behind the lock: 28 timeouts, one slow unit; round
1293's seven were on no disk). Three loops T.88 does not bound: section 6.5.5's height classes may
be empty and are not counted (24 inputs), section 6.5.10's export runs may be zero-length and do
not advance (2), and a symbol is bounded only by its 32-bit width and height (3). Section E.3.4
feeds 1-bits past the data, so a few hundred bytes run any of them. The brief's guess — a symbol
larger than its page — is refuted by the census: a real symbol is 12.8 times its strip's area.

**Census** of 54 498 corpus JBIG2 streams through the pinned codec with counters: 1 537 956 height
classes, none empty in a stream that decodes; at most one zero-length export run a dictionary; at
most 41 843 627 symbol pixels in one decode; slowest real stream 0.92 s.

**Where the fix lives.** `hayro-jbig2` offers no limit and no cancellation, so the bounds are
`doc/patches/hayro-jbig2-symbol-dictionary-bounds.patch` (empty classes ≤ SDNUMNEWSYMS, zero runs
≤ symbols + 1, 2^28 symbol pixels a decode), written against `64efcaca` and **not applied** — the
fork is the owner's. In tree: `in_process.rs` gives the in-process path the worker's deadline on a
kept thread (abandoned, not killed; two at most; a new thread per decode cost +8 ms median on a
137 KB stream, a kept one +0.2 ms), `Sandbox::with_timeout` a shorter one for the confined path.

**The seven fixtures** (`tests/symbol_dictionaries/mod.rs`, minimised against both codecs) on the
pinned codec: 14 s, 17 s, 32 s, 34 s, 68 s, and two past 600 s. Now: each refused inside 1 s at a
250 ms deadline under both isolations; with the patch, 0.4 ms to 0.47 s by the codec's own sentence.

**Gates.** Tier 1 on `pdf-sandbox`, `pdf-model`, `viewer-confined` (rustfmt, clippy `-D warnings`,
tests), `cargo test -p conformance`, both fuzz targets built. Behind the lock: census ×3, fixture timings,
`pdf-model --test corpus` (59 of ceiling 59) and `raster_golden` (974 held, 0 moved), exit 0.
The merge's tier 3 found the kept thread killed by `SIGSYS` (`prctl`) in the confined viewer; a
confined process now decodes on its own thread, and `awkward_classes` and `read_corpus` pass again.
The patch on a scratch copy of the codec: the census's 54 498 outcomes unchanged, and ten minutes
of `jbig2` behind the lock at 96 302 executions and 3 981 edges with no timeout (pinned: 51 493, 3 829, 25).

Left: the patch, for the owner; T.88's Annex K streams — the pinned codec refuses eight of ten,
patched or not, evidence for the fork. `tools/batch.sh`'s extension list gains `patch` (1308's file).
