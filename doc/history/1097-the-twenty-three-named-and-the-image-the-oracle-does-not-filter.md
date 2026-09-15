# 1097 — The twenty-three named, and the image the clause says not to filter

2026-09-15. Files: `crates/render-raster/{examples/image_phase.rs,tests/corpus.rs}`, `doc/QUORRA_FEEDBACK.md` §43, this file. No ADR: nothing here is a decision beyond what a clause states. Five siblings live in the tree, and `crates/render-raster/src/scene.rs`'s diff is 1093's (ADR 1107) rather than this round's. **The oracle's undiagnosed head is empty for an eleventh round** — `1960 pages in 38.6s`, `agrees 1010, contradicted 46, ambiguous 835`, and "ambiguous, undiagnosed" printed with nothing under it — so the contract was `render-raster --test corpus`'s other three numbers.

## The 23, by cause, read rather than counted

Every line below is the run's own sentence checked against the clause it cites, and nothing in the twenty-three is this adapter's to fix. The counts did not move, so no ratchet did.

**Ten want a password §7.6.4.3's empty one is not** (§7.6.4.1): `bug1782186`, `encrypted-attachment`, `issue15893_reduced`, `issue21579`, `issue3371`, `issue6010_1`, `issue6010_2`, `pr6531_1`, `print_protection`, `saslprep-r6`. `poppler` refuses all ten with the empty password supplied, eight as *Incorrect password* and two by a sentence of their own — evidence in principle 5's one direction. `print_protection.pdf`, whose name suggests a §12.8.2 permissions file a reader could open, is not one: its user password is not empty either.

**One states an `/Encrypt` that resolves to nothing**: `PDFBOX-4352-0`, whose cross-reference entry is nineteen digits where §7.5.4 states twenty, so §7.6.1's dictionary is unreachable.

**Five yield no page one**, each for its own fault: `bug1020226` (184 bytes, no `startxref`), `REDHAT-1531897-0` (truncated at 871 of the 7945 its `/L` states), `poppler-937-0-fuzzed` (§7.7.3.2's `/Kids` fuzzed to a bare reference, and object 3 does not parse, so §7.3.10's null), `poppler-85140-0` (a generation number past anything §7.3.3 permits a reader to hold, §7.3.10 again), `Brotli-Prototype-FileA` (`/BrotliDecode`, a filter ISO 32000-2 does not define).

**Four are refused before the scene**, each naming its construction and its clause: `bug1721218_reduced` (§11.6.6, §11.7.2 — a four-component group's pair), `issue16742` and `issue5044` (the same at three CIE-based components through a cube), `issue21346` (§11.5.3, §11.3.4, §11.4.7 — a luminosity whose `Y` is the space's own). All four are raster's scene vocabulary and all four are `doc/QUORRA_FEEDBACK.md` §43's ask; the first had never been named there and now is.

**Two are budgets, reported out loud**: `ContentStreamCycleType3insideType3` spends `MAX_OPERATIONS` on a Type 3 cycle and is pathological under a budget doing its job (`doc/todo/49`); `issue1905` needs 272 158 852 scene-derived bytes against raster's own `DEFAULT_MAX_FRAME_BYTES` of 268 435 456 — 1.4% over, §40, and a budget raised to admit one page is a budget chosen by that page. **One is a capability**: `issue19517` is 12608x16806 where this adapter states 16384 a side (§44). The CPU backend draws all three device refusals and says so.

## pr12564, and the filter that does not reach the pixels

The 8 differing are the six ADR 1102 left plus `DIFFERS_AT_THE_EDGES`'s two. `issue2177` is unchanged in kind and its figures are refreshed in place: `examples/ink_ladder` reads 13 014.53 against 12 935.99 at 1× and 12 937.14 against 12 920.56 at 8×, 0.61% → 0.13%, halving at every rung — a per-boundary cost.

`pr12564` is not that. Its ink is flat along the ladder (60 404.57 / 60 240.12 at 1×, 60 415.91 / 60 266.60 at 8×) and lies on the text; its **worst tile is a 90 × 90 seal placed by `90 0 0 90 32.08 872.42 cm`** — one device pixel per sample at a fractional offset, where `pdf_render::Image::is_smoothed` answers `true` and both backends were therefore asked to filter. Resampled from the document's own samples at that placement, the device rectangle is **0.49** of 255 from raster under a bilinear filter and **0.35** from the oracle under point sampling, 10.8 each from the other rule. The oracle does not filter it and nothing in this tree chose that: `tiny-skia`'s `Pattern::push_stages` substitutes `FilterQuality::Nearest` for **every** pure translation — which an image at its own resolution always composes to — leaving its own integer-translation test unreachable underneath. `examples/image_phase` is the reproduction away from any document, with the same image at 1.5:1 as the control, where the two agree.

Round 1093, which owned `render-cpu`, read the clause the other way and the clause decided it: §10.7.4 maps each device pixel's centre back into source space and forbids averaging over the pixel area, so at one device pixel per sample the point sample *is* the answer and `is_smoothed` now says so (ADR 1107 section 3). The ramp is raster's — a mirrored copy of the old rule in `render_raster::scene` (ADR 0702) — filed as `doc/QUORRA_FEEDBACK.md` §47; `image_phase` is its measurement.

Gates are in the report.
