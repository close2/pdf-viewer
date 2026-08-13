# Carry an image *and its sampling intent* to the backends

Status: **the vocabulary is built and one of the three consumers is on it** (session 370, ADR
0210). Two remain. Neither is blocked by the display list, and — since the rev moved to `1dc833f7` —
the JPEG 2000 one is not blocked at all: the fork this workspace pins carries the
reduced-resolution fix, and the edits that follow it are listed below, ready to take.
Priority: 24
Corpus: 1 document on the corpus gate (`issue19517.pdf`); the rest is one backend's
Clauses: §7.4.9, §8.7.4.5.3, §8.9.6.3, §10.7.4
Code: `crates/pdf-render/src/paint.rs` (`ImageSource`, `Grid`, `ImageAtDeviceScale`),
`crates/pdf-model/src/image.rs`, `crates/pdf-sandbox/src/decode.rs`, all three backends

## What the vocabulary is

`Command::Image` carries a `pdf_render::ImageSource`, which is either `Decoded(Image)` — the
raster on the grid the file states, which is every image but one on this corpus — or
`AtDeviceScale(DeferredImage)`, a raster the display list *names* and a backend produces once it
knows the scale. `ImageSource::at(placement)` is what a backend calls; `Grid::for_placement` is
where the device grid is decided, once, so that the three backends cannot ask for different ones.
An implementation owes one thing: `samples(Grid) -> Image`, no finer than the grid it was asked
for.

The interpreter still does not know the device scale, which is what
`zooming_rasterises_again_without_interpreting_again` asserts and what makes a display list
re-rasterisable at any zoom. A deferred raster holds no document and no lifetime — `Document`
caches behind `RefCell` and is not `Sync` — so it carries whatever it needs to answer, which for a
soft mask is the file's own packed bytes.

## Closed: a mask on a grid the bound refuses

`issue16263.pdf`, a 2×2 image with a 34862×4332 `/SMask` — 604 MB of RGBA on the finer of the two
grids, drawn as black bars until the display list could carry the two rasters apart. It now
combines at device resolution (§10.7.4's centre rule), draws in 49 MB, and **agrees with the
reference consensus**. The corpus's incomplete list went 73 → 72 and no corpus document reports an
`/SMask` at all.

## Still owed

### JPEG 2000 at a reduced resolution level — one revision away, and what follows it

`issue19517.pdf`, 12608×16806 in four channels, 847 million samples, refused for wanting
gigabytes. §7.4.9 NOTE 3 addresses the answer to this program by name: "[v]iewing and printing
applications can gain performance benefits by using the resolution progression."

**This entry said the blocker was "an API on `hayro-jpeg2000` — a decode that can be told where to
stop", and that was false since 10 December 2025.** `DecodeSettings::target_resolution` is in the
revision this workspace already pins. Session 396 found what the real cost was, and it is
measured in ADR 0233: asking for a reduced level skipped the bit-planes and the wavelet but still
reserved a coefficient for every sample of the **full**-resolution image — one allocation of
3.4 GB for this file however small a raster was asked for. Resident size never showed it; address
space did, and address space is what `pdf_sandbox::lockdown`'s gigabyte bounds.

**Step 0 is done.** The owner pushed `feat/reduced-resolution-allocates-less` to `close2/hayro`
and the workspace manifest's `rev` for `hayro-jpeg2000` is `1dc833f7` — all 183 of the crate's
own asset tests byte-identical to snapshots from unpatched `main`, per its commit message. The
pull request offering both fixes upstream is the owner's; nothing here waits on it. What was true
until this move is worth keeping in one sentence: against `2a1abd14` every edit below turned an
accurate refusal into a worker killed by `RLIMIT_AS`, which is why nothing below was committed
first.

Now, in order, and each of them verified against a `[patch]` build in session 396:

1. **`pdf-sandbox`'s `jpx` steps down resolution levels until `MAX_SAMPLES` is met.** Parse with
   `target_resolution: None`, and while the sample count is over the bound re-parse asking for
   half of what came back, refusing if the size stops shrinking. About twenty lines. The bound
   itself was corrected to 2^26 in session 396 and is now measured rather than estimated — see
   its own documentation.
2. **The raster's grid travels with it.** `samples_of` and `decode_jpx` return the grid the codec
   produced rather than letting `decode_parts` assume the dictionary's, and the `Image` is built
   at it. Every other codec returns the dictionary's grid unchanged, so this is one extra pair on
   five return values.
3. **§7.4.9's "Width and Height shall match the corresponding width and height values in the JPEG
   2000 data" needs the *full* size to check against**, which a reduced decode no longer returns.
   Either `pdf_sandbox::Raster` carries the codestream's own stated grid beside the raster's — the
   honest fix, and it keeps the check — or the check moves into the worker, which is where the
   codestream's header is read. Session 396's experiment relaxed it to `<=` to get a number and
   **that is not the committable form**.
4. **`soft_mask_entry` is told the base raster's real grid**, not the dictionary's, so that
   `combined_grid` decides on the two rasters that exist. With the base at 3152×4202 and this
   file's 12608×16806 `/SMask`, that is what routes the mask to §10.7.4's device-scale path
   instead of an eager 848 MB combination — no change to the mask machinery, which already
   handles it.

What that buys, measured against the patched build (ADR 0233): page one goes from **0 commands
and one reported image** to **1 command and nothing reported**, drawing a raster of 3152×4202 that
agrees with `poppler`. It costs 5.6 s and 2.5 GB peak across the process tree, which is a number
to reduce and not a reason to leave the page blank.

**A cheaper alternative exists and is written down rather than taken** (ADR 0233's "Alternatives
considered"): decide the reduction at interpret time from the memory budget alone, with no device
in it. Simpler, keeps the display list a pure function of the file, and on *this* document would
produce a raster finer than any screen. If the fork's push is slow, that is the half to build.

### A sampled shading on `render-gpu`

**This entry said "on the GPU backends" and was wrong about the one that ships**. `render-quorra`
draws a sampled grid — `sampled_fill` uploads it as a raster and clips it to the path — and
`render-cpu` evaluates it; only the Vello backend refuses, through `brush_for`'s
`UnsupportedPaint`, because a grid is not a brush any gradient can express. So this is one
backend's gap rather than a clause's, and no page on the quorra corpus gate is refused for it.

What is left of the item is the *shape*: §8.7.4.5.3's type 1 shading reduces to a grid standing
in for a function of two variables, and `pdf-render` fixes that grid at a
resolution — `FUNCTION_GRID`, 128 — before any device has been asked. That is the same
interpret-time resolution decision `ImageSource` removed for a mask, one command over, on
`Paint::Shading` rather than on `Command::Image`. Nothing in ADR 0210 prejudges what it looks
like; what it settles is that "a raster plus the intent to resolve it at the device" is
expressible without the interpreter learning the scale.

And a fourth claimant this file did not have: **§8.9.6.3's explicit mask**, whose own ledger row
carries the same "the true answer is a composite at device resolution" sentence the soft mask's
did. It can be moved onto `ImageSource` whenever a document asks for it, and none on this corpus
does — every stencil here refines to a grid that fits.
