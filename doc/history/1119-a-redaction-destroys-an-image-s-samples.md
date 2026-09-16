# 1119 — a redaction destroys an image's samples in the region

2026-09-16. ADR 1126, continuing round 1112 (ADR 1124). Contract: build what 1112's redaction
refused — destroying image data within the region (§12.5.6.23) — proven unrecoverable.

**Census (trap 8).** Parsed (stream inflation, not byte-grep): five corpus PDFs carry a `/Redact`
— the Isartor `6.5.2` witness and four veraPDF PDF/A annotation fixtures. **None meets an image,
path or form.** So the image-destruction `shall` has no corpus witness and is proven by a calibrated
fixture (trap 13); the shared-image guard costs nothing real.

**The build.** `crates/pdf-transform/src/redact.rs`: an image XObject whose placement meets the
region has every sample whose centre maps into a region box overwritten to **zero** in its own grid
(§8.9.5.2: 0 maps through any /Decode to Dmin — one constant, no original sample, any colour space),
re-encoded FlateDecode and written as a new stream whose slot is reserved before the closure walk,
so the original samples are copied by nothing (no orphan). Samples come from `Document::image_stream`
(codec-free bytes are the packed samples). The region is inverse-mapped to bound the block, then each
sample centre tested exactly — rotation clears only what is truly inside; outside is byte-identical.

**Refused, each narrower (trap 5, principle 1).** A **shared** image (referenced more than once, or
reached by a shared resource path) is refused not cleared — overwriting it would alter another
placement (single-referrer guard, `reference_counts`). A **codec** image (DCT/JPX/CCITT/JBIG2) or one
whose grid this build cannot count; an **inline** image (§8.9.7, an owed content-stream splice); a
**painted path or form** (§8.5) — the round's reading: a vector mark is not removable "the way text
is" (no per-region unit without geometric subtraction; dropping the whole operator over-removes).

**Proven unrecoverable.** `image_samples_inside_a_quadpoints_region_are_destroyed_and_outside_intact`:
an 8×8 image over the page's [50,150]² square, `/QuadPoints` on its left half — after redaction cols
0–3 are zero in the decoded output, cols 4–7 byte-identical, the original stream not orphaned, the
file re-opens and decodes. Two refusal witnesses (codec, shared). **End to end through
`quorra-transform redact`**: output image inflated — region zero, rest intact, original bytes gone.
`Origin::Redacted` gains an `images` count.

**Row moved:** §12.5.6.23 stays `partial` (image XObject destruction built; overlay departs; inline,
path/form, codec/shared images owed), note rewritten, cites redact.rs + three new tests + ADR 1126.

**Gates** (siblings share the tree; fmt/clippy scoped to mine, `batch.sh` runs `fmt --all --check`
clean). Tier 1: clippy -p pdf-transform -D warnings 0; nextest -p pdf-transform 261; --doc 0;
conformance 259 passed 0 failed (no cited clause unreviewed). Tier 2/3 under the lock, one at a time:
pdf-transform `gate` ok (86.9 pp/s, floor 40); `save_round_trip` ok (ratchets held); pdf-retrieve
`retrieval` 10. tests/redact.rs 10 passed. batch.sh check exit 0.
