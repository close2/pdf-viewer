# 1651 — The DCT encoder the owner chose, confined, and the two features it gates

Slot 5 of batch one hundred and one, 2026-10-20, an archive round. ADRs 2138, 2139; no question.

**Premise that did not hold.** `mozjpeg-rs` 0.9.2 is not a wrapper of C mozjpeg: it is imazen's
pure-Rust port, `forbid(unsafe_code)` in its own source, no build script, C only behind an opt-in
feature nothing enables. It is confined anyway, on ADR 0014's reasons and because five of its seven
new packages carry SIMD `unsafe` (ADR 2138). Hypothesis: "a third of the corpus's photographs
recompress smaller" — over pdf.js's 974, 641 images were eligible and 284 replaced (44 %), 492
declined by rule; 19 documents shrink, 7 232 992 → 5 400 925 bytes; `render --format jpeg` was
a flag and a call, as said.

**Built.** `pdf-sandbox`: `Request::Dct` and `encode_dct`, baseline only (§7.4.8 and its NOTE 4),
eight-bit tables at any quality, the decoders' 2^26-sample budget measured inside the worker's
gigabyte, bytes independent of the CPU's SIMD. `pdf-transform`: `optimize --images quality=<n>`
(`optimize_images.rs`; the walk's numbering kept, Flate-wrapped where the lossless rule would wrap,
so its output is idempotent), `render`/`images --format jpeg` with `--quality`; `downsample=`
refused by name. Nothing reaches the encoder unless a format is asked for (A01).

**Proof that the default did not move.** HEAD's and this round's release `quorra-transform optimize`
over all 974 pdf.js documents: 974 of 974 the same SHA-256 prefix (both refusing counted alike).

**Foreign readback of the lossy output** (not a `foreign_corpus` lane — that walk asserts
bit-identical pages): of 850 written, qpdf `--check` regressed none whose source it passed;
`pdftoppm` and `mutool draw` read page one of all 19 replaced documents, exit 0. Rendered and looked
at: `blendmode.pdf` and `issue14200.pdf`, the same pages, colours and orientation.

**Gates.** In an export of HEAD plus this round's files (siblings mid-edit in `render-cpu` and
`archive/`): `cargo fmt --check -p pdf-sandbox -p pdf-transform` exit 0; `-D warnings` clippy
`--all-targets` on both exit 0; nextest `-p pdf-transform -p pdf-sandbox` 657 passed, 0 failed, 11
skipped. Then the batch's tier 1 found `worker_features` red: the encoder's `bytemuck` took five
features from the windows' graph in the one-run install. Fixed by building the worker alone in
`install` and CI (ADR 2138 section 6): conformance 452/0 failed, its clippy and fmt exit 0, `cargo
deny check` exit 0, `state.sh confined` exit 0; `pdf-model`'s six codec tests 23 of 23. Arms not
run: `cargo tree -i mozjpeg-rs` reaches the arm binaries through `pdf-model`, but no decode route
changed — a fifth match arm only — so the codec tests are the evidence; `t3-foreign_corpus` not
run, the digest sweep above standing for `optimize` and the other writers' code untouched.

**Unfinished.** `downsample=` (`doc/todo/57`); a lossy lane in `foreign_corpus`; the encode is serial
through the one worker, unmeasured against a pool. 4 200 s.
