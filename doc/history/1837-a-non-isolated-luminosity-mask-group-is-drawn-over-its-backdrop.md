# 1837 — A non-isolated luminosity mask group is drawn over its backdrop

Slot 6 of batch one hundred and twenty-four, 2026-10-27, a pixels round (Sonnet, high), written by the round's
second agent after a handover; ADR 2508 used, ADR 2509 and Q703 left unused. I worked in `scratchpad/r1837/`.
Ledger: §11.6.5.1's row gains a sentence and three tests; it stays `implemented`.
**Survey** (the whole cache, 65 944 documents, HEAD 08527578, exit 0, 2549 s under the lock): 173 unopenable, 47
locked, 4 encrypted, 41 pageless, 431 incomplete, 20 slow. Ranked by documents with `tools/survey-residue.py`: 245 a font with
no outline for any code the page shows, 53 text skipped (the same fonts), 24 undecodable `FlateDecode`, **22 a
non-isolated group whose element blends with the backdrop it excludes**, 18 damaged check values, 14 font Adler
mismatch. The first three are not this tree's (ADR 2508 section 1 names the ADRs); the fourth was taken.
**Premise** (the head is one defect at one site): held for the fourth row. All 22 are luminosity soft masks
holding a `Multiply`; `build_soft_mask` passed `isolated: true` for every mask, so a non-isolated group was
reported although §11.6.5.1 gives it a fully opaque `/BC` backdrop. **Built:** `state_mask_backdrop` states that
backdrop as a fill in the mask's list, the report no longer fires for it or for an Alpha mask, and
`note_mask_group_structure` holds the call (clippy's line limit). Three tests in `tests/soft_masks.rs`.
**Finish line:** `safedocs survey` of the 22: 22 incomplete before, 0 after. Page one of each, HEAD's export
against the fixed: ten identical, twelve differ by at most three levels (887 pixels at most), ADR 2508 section 5.
**Found, not fixed:** the first agent read a 37 638-pixel difference on `3867162.pdf` as the fix's; it was the
fixed export's missing `pdf-sandbox-worker` (habit 185: the photos are decoded in the worker, so without it the
images are reported and not drawn); with the worker built both exports draw it within two pixels of one level.
**Proposed trap:** never `{:.N?}` a display list in a debug print: the precision applies to every float in it
and one such print wrote a 10 GB log. **Observation:** a 4 s bare survey of 22 documents queued 600 to 1200 s behind the lock (the brief asks for the lock;
the figures here are bare at four rayon threads), and the arms lane waited 2400 s behind another round's holds.
**Gates.** `cargo test --profile gates -p pdf-model --test soft_masks` 19 passed, exit 0 (two of the three new
tests fail with the fix reverted); `cargo fmt --check -p pdf-model` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p
pdf-model --all-targets` exit 0 (two findings of mine, a 108-line function and a missing backtick, cleared first);
rustdoc `-D warnings --document-private-items -p pdf-model` exit 0; `raster_golden` `held 974, moved 0`, exit 0; the
six arms by page against HEAD's six arms on the fixed export, each exit 0, 0 pages moved of 968 and of 965; `corpus`,
`text_extraction` and `oracle` (not clocked) under `--ignored`, each exit 0; `safedocs survey` of the 22, 22 incomplete
before and 0 after, exit 0; `cargo run -p conformance --bin cited` exit 0; `--bin quotations` exit 0, no divergence in
a file of mine; `cargo test -p conformance --no-fail-fast` exit 0, 0 failed, every test of every file ok, run once after the ADR and the record were written. The Opus validator's seven wording findings were fixed by me. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on seven wording and paperwork findings and PASS on the clause reading, the plants, the pixel and arms evidence and the gates; the round fixed them.
