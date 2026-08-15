# 539 — The raster a page decodes thirty-six times, and the key that lets it decode once

Date: 2026-08-15. ADR: [0374](../adr/0374-the-raster-a-page-decodes-thirty-six-times.md).

**What was taken.** `doc/todo/47`, written by session 538 with its measurement: `image::decode_parts`
runs at every `Do`, so `22060_A1_01_Plans.pdf` decoded four 2480×2630 photographs nine times each on
one page. `image::RasterCache` is the cache the item asked for, and the item's own header decided the
shape — the three key components it named plus the stream, with the bound left as the question.

**What the key claims**, which is most of the round: a `Do` that agrees with an entry on four things
would have produced those samples. The stream, by the identity of its allocation *with the `Arc`
held* — ADR 0317's pin, which is what makes an address a name and what lets an inline image share
the table safely while never hitting in it; the resource dictionary in force (§8.6.5.1's named
space, §8.6.5.6's `/Default*`); the fill colour (§8.9.6.2's stencil); and what the samples composite
into (§11.4.7, §11.6.5.1 — the one that changes *within* an interpretation). The `Document` is the
fifth input and is out of the key because the cache dies with the page.

**One test per component, and each was confirmed to fail with its component dropped.**
`crates/pdf-model/tests/image_reuse.rs`, run four times against a mutated key: `resources` out →
`a_raster_is_not_shared_across_resource_dictionaries`; `fill` out → `..._across_fill_colours`;
`into` out → `..._across_compositing`; the pin replaced by a bare `*const Stream` →
`a_stream_cannot_inherit_the_raster_of_one_whose_allocation_it_reuses`, on round 2 of 64. Each
second arm is compared against a *fresh uncached decode*, so a wrong answer is the wrong picture
rather than a suspicious pointer. A sixth test is trap 5's: a page drawing one short-of-its-grid
image twice reports exactly what a page drawing it once reports.

**The bound, measured before it was chosen.** Every `Do` on an image over the corpus's page ones was
recorded — 4789 calls in 228 documents — and replayed through a least-recently-used simulation, ADR
0317's method. Base raster decoded: **1796.8 MB with no cache, 1635.7 at 8 and 16 MiB, 781.5 at 32
MiB and at everything above**. The knee is 32 MiB because the repeats that cost anything are 26.1 MB
photographs; `RASTER_BUDGET` is **64 MiB**, the knee doubled, which holds two of them and so serves
an interleave the corpus does not contain and a page can state as easily as the witness's. The
item's other candidate — the last raster only — is 804.2 MB over 4534 decodes, 2.9% short.

**A correction inside the measurement, worth more than the number.** The first replay charged each
entry's *mask* bytes to the saving and read a 2763.8 MB baseline. `issue16263.pdf` repeats a `Do`
forty times over an entry charged 18.9 MB and decodes **no base raster at all**: that 18.9 MB is the
packed soft mask `MaskCache` has shared since ADR 0210. Its peak resident is unmoved by this cache,
which is what caught it.

**What it buys**, callgrind under `RAYON_NUM_THREADS=1` because the machine carried a load average
of 130 to 140 all session: witness page one **58 665 139 034 → 6 544 740 674 (−88.8%)**; the control
— ISO 32000-2 page 101 ×50, no image on it — 1 235 040 931 → 1 234 947 809 (−0.008%). Peak resident
(`ru_maxrss`, three runs an arm) **1031.8 → 235.6 MB** on the witness, 39.4 → 39.2 on the control:
the cache spends no memory the display list was not already spending, because the same `Arc` goes
into both and nine `Do`s used to put nine 26.1 MB allocations in the list.

**Byte identity.** `examples/display_list_digest` over the corpus, both arms, same
`pdf-sandbox-worker` on disk: 964 documents opened, 958 first pages interpreted,
`md5 f9eb6ec03bdee3e9d4edc60e82c508e4` on both.

**One ratchet moved, and it is a hole closed.** `render-quorra::corpus`'s `REFUSED_AT_FOUR` held
`22060_A1_01_Plans.pdf` on resource bytes — 72 uploads of 8 distinct rasters, 548 104 348 against
the 536 870 912 default. The page draws at 4× now and the list is four. `max_resource_bytes` was
not raised.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of lints
(the `viewer-qt@` lines are gcc's on a cold build); `cargo nextest run --workspace` **1981 tests run:
1981 passed, 15 skipped**; `cargo test --workspace --doc` green (24 targets, 0 failures); corpus
**974 documents, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 64 incomplete, 0 slow**;
oracle **1794 pages, 1691 complete, 906 agree, 67 contradicted, 786 ambiguous**; text extraction
**10969/11163 words in bounds (98.26%), 486 of 508 documents fully in bounds**; JPEG 2000 **14
identical**; dates and XMP green; quorra default lane **956 pages: 931 agree, 23 differ, 2 refused,
18 not comparable**, gpu coverage lane **956 pages: 929 agree, 25 differ, 2 refused, 18 not
comparable**, 4× default lane **951 pages: 936 agree, 11 differ, 4 refused, 23 not comparable** with
the amended ratchet; `cargo test -p conformance` 112 unit and 5 ledger tests green.

**Touched.** `crates/pdf-model/src/image.rs` (`RasterCache`, `RASTER_BUDGET`, `Parts: Clone`),
`crates/pdf-model/src/content.rs` and `content/image.rs` (the field and the call site),
`crates/pdf-model/tests/image_reuse.rs` (new), `crates/render-quorra/tests/corpus.rs`
(`REFUSED_AT_FOUR`), `doc/conformance/ledger.toml` (§8.9.5), `doc/performance.md` and
`doc/todo/43` (two claims that stopped being true), `doc/todo/47` (**deleted**),
`doc/todo/README.md`, `doc/adr/0374-*` (new), this file.

**Not run, and why.** `doc/todo/00`'s step 7 ink sweep: no pixel moves — the display list is
byte-identical over the whole corpus, which is a stronger statement than the sweep makes. The fuzz
targets: no parser changed.
