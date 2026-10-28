# 1859 — A mask group's luminosity takes the device branch, and §10.7.5's fit is drawn in every backend

Slot 9 of batch 126, 2026-10-28, Opus 5.5, four sessions; ADRs 2552, 2553 used, Q725 unused. Part (B) first (no `A714`
at the start), then part (A) on A714. Ledger: §11.5.3 `implemented` to `departed` (A23's colorimetric sentence); §10.7.5
`departed` to `implemented`.

**Part (B).** `tools/state.sh oracle`'s head was `issue21346.pdf` (11.17), held as the references' departure from
§11.5.3's colorimetric branch, a true reading; but `doc/questions/A23` had answered "switch to the way all other viewers
do it", never built. **Plant**: only that group's `/CS` set to `/DeviceRGB` drew the references' (206, 223, 235).
**Built** (ADR 2552): every mask group composited in its `/CS`, weighed by the device branch; `luminance_derivation`
deleted.

**Part (A), A714 option 1** (ADR 2553): `raster::grid_fit` at the encoder's stroke and line sites and in `line.wgsl`'s
vertex stage, `encode/expansion.rs` declining by design; `CpuRasterizer`'s fit on by default. The hypothesis held
(stroke and placement alone decide the fit). The corpus gate found two lane defects, both fixed: the line lane asked
roundness of the promoted pen (`bug1743245.pdf` 10.35 to 0.108 levels), and `render-raster` cut dashes before the fit
(`issue14297.pdf` 0.229 to 0.0124). **Plant**: each defect put back turned its check red (the new unequal-stretch shape
128 levels off; `issue14297.pdf` 0.229). Test: three shapes, eight placements, every lane within 2 levels.

**Files.** A23: ADR 2552's code list, pdf-model `tests/{transparency_groups,oracle}.rs`. The fit: render-cpu
`{lib,stroke}.rs`; render-raster `src/stroke.rs`, `tests/stroke_adjustment.rs` (new), `tests/corpus.rs`; raster-gpu
`raster.rs`, `raster/grid_fit.rs` (new), `encode/{stroke,line,expansion}.rs`, `shaders/line.wgsl`; pdf-render
`grid_fit.rs`. Both: `raster_golden.tsv`, `doc/checks/fixed-documents.toml`, the ledger,
`doc/todo/{65,23,README,_scan-conversion}.md`, `doc/state-of-play.md`, `doc/questions/Q97`, ADRs 2552, 2553, 2530, 2531.

**Gates.** A23's export: oracle (`--clock`) exit 0, 1971 pages, agrees 1029, contradicted 46, ambiguous 837 (HEAD 1028,
47, 837); clippy `-D warnings`, `cargo fmt --check` exit 0; nextest exit 0, 2914 passed; `raster_golden` 4 moved;
pdf-model corpus exit 0; HEAD's six arms: 3 digests moved per arm, errors within 0.0006; `fixed_documents` 1 of 77 moved
(`4605565`'s ink band). The fit's export: clippy, fmt exit 0; `raster` tests exit 0, 846 passed; render-cpu tests exit
0; `raster_golden` 7 moved; render-raster corpus gate exit 0, 968 agree, 0 differ, 62.9 s; HEAD's six arms: each 1× arm
moved exactly those 7, no 4× arm any (`issue14297.pdf` 0.1233 to 0.1160 cpu/gpu, 0.0201 to 0.0124 compute); oracle
(`--clock`, 72.1 s) exit 0, 1028, 47, 837; `fixed_documents` exit 0, 0 moved. Shared tree: `cargo check --tests` exit 0;
`--bin cited` 0, `--bin quotations` 0, `cargo test -p conformance --no-fail-fast` 0, 470 passed. Docs walk exit 101,
1695 s: this round's unescaped `[f]` in `pdf-model/src/soft_mask.rs` fixed (pdf-model rustdoc exit 0); siblings'
`viewer-ui/src/chrome/archive.rs:231` E0308 and private links in pdf-transform, pdf-archive and pdf-render's
`shading.rs`. Durations: session 1 5507 s, sessions 2–3 about 10900 s, session 4 about 4900 s.

**Found, not fixed.** A23 was answered and never built: a census of answered A files against the tree may find more. A
dotted rule (`[0 2] d`, round caps, 0.6 px, no `/SA`) differs 18 levels between `render-raster`'s dash dots and
`render-cpu` at the rule's end (`crates/render-raster/src/stroke.rs:568`, `dash`).
