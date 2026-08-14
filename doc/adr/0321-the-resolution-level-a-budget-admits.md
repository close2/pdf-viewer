# 0321 — The resolution level a budget admits, and the grid that travels with it

Date: 2026-08-14 (session 486)
Status: accepted

## Context

`doc/todo/24`'s JPEG 2000 item had been fully unblocked one move at a time: session 370 built
the display list's half (`ImageSource::AtDeviceScale`, ADR 0210), session 396 found that a
reduced decode still cost the full image's address space, wrote the fix on `close2/hayro` and
measured everything (ADR 0233), and the owner's push moved the workspace's `rev` to `1dc833f7`,
which carries both `DecodeSettings::target_resolution` and the allocation fix. What remained
was the four edits ADR 0233 verified against a `[patch]` build and deliberately did not commit,
because against the then-pinned revision each of them turned an accurate refusal into a worker
killed by `RLIMIT_AS`.

The witness is `issue19517.pdf`: a 12608×16806 scan in four channels — 847 million samples,
ten gigabytes to decode whole — over the confined worker's 2^26-sample budget, refused since
the seventh session, for a page a screen shows at about four megapixels. §7.4.9 NOTE 3
addresses the answer to this program by name: "[v]iewing and printing applications can gain
performance benefits by using the resolution progression."

## Decision

### 1. The worker steps down the resolution progression until its own budget is met

`pdf_sandbox::decode::jpx_within_budget` parses the codestream at full resolution and, while
the declared sample count exceeds `MAX_SAMPLES`, re-parses asking for half of what the last
reading offered — `target_resolution` selects the finest level *at least* that large, so each
accepted step is one more decomposition level skipped. Every step is a header parse; no sample
is decoded until the loop settles. A request the codestream cannot better — the levels have run
out, or it is a palettised JP2 whose decoder declines reduction because a reduced index is a
different palette entry — ends in a refusal naming the stated grid, so nothing became quieter.

**The decision lives in the worker, against the worker's budget, and not against a device
grid.** That is ADR 0233's "alternative considered" adopted deliberately rather than by
drift, and the reasons hold better on this side of the pipe: the budget being defended is the
worker's address-space ceiling, which the worker must enforce without trusting its caller
(the pipe carries no invariants — the rule `MAX_SAMPLES`'s own documentation already states);
the display list stays a pure function of the file, which the oracle's comparison rests on;
and on the witness the level chosen (3152×4202, 53 million samples) is finer than any screen
the page meets. What the device-grid variant would add — a coarser decode when the image is
drawn small — is a performance refinement `ImageSource::AtDeviceScale` can carry later
without touching this round's shape.

### 2. The raster's grid travels, and the codestream's statement travels beside it

`samples_of` now answers with `SamplesOnGrid` — the RGBA, the grid it is on, and the
`/SMaskInData` flag — and `decode_parts` builds the `Image` at the grid the codec produced.
Every codec but a reduced JPEG 2000 decode returns the dictionary's grid unchanged; an image
occupies the unit square whatever its resolution (§8.9.5.1), so the raster's own grid is the
only honest one to declare.

`pdf_sandbox::Raster` gains `stated_width`/`stated_height`, the grid the codestream itself
states, read from the first unreduced parse. §7.4.9's conformance rule — "Width and Height
shall match the corresponding width and height values in the JPEG 2000 data" — is about what
the *data* says, so `decode_jpx` checks the dictionary against the statement, not against
whatever level the budget chose. Session 396's experiment relaxed the check to `<=` to get a
number; that form is exactly what `doc/todo/24` forbade committing, because it would stop
rejecting a dictionary that genuinely contradicts its data. The wire format grew eight bytes
and the parent recomputes the invariant a reduced decode guarantees — the statement is never
smaller than the raster — so the protocol magic moved to `PDFSBX03`: a parent and worker from
different builds must find that out at the handshake, not by reading grid bytes as samples.

### 3. The mask routing is asked about the raster that exists

`soft_mask_entry` takes the base grid as a parameter instead of reading the dictionary.
`decode_parts` passes the decoded raster's grid, which is what sends this file's 3152×4202
base under its 12608×16806 `/SMask` — `RunLengthDecode`, `DeviceGray`, no image codec — to
§10.7.4's device-scale combination instead of an eager 848 MB one on a grid neither raster is
on. No change to the mask machinery: the route ADR 0210 built already handles it.

**Two callers keep the dictionary's grid, and that is the drift-proofing rather than an
oversight.** `unapplied_soft_mask` (the interpreter's report) and `apply_soft_mask` (the eager
route) ask with the same pair the dictionary states — the only pair knowable without a decode —
so the report and the eager behaviour cannot disagree by construction. The route that consults
the decoded grid only ever *applies* a mask, so it cannot open a gap for the report to miss;
and in the one cell where the decoded grid declines the deferred route while the dictionary's
admits the eager one, the eager combination is bounded by the `MAX_SAMPLES` the dictionary
already passed, and the mask's own decode reduces the same way the base's did.

`MaskCache::read` now runs the routing decision on every call and caches only the read of the
mask's packed bytes — the part whose inputs are all the mask object's own — so a cache hit
cannot route a differently-sized base onto a path its own grids would not choose.

## What it bought, on the page

Page one of `issue19517.pdf` goes from **0 commands and one reported image** to **1 command
and nothing reported**. Rendered at scale 1.0 through `render-cpu` and put beside `pdftoppm`'s
render: the same flat orange receipt with the same faint text (RMSE 0.0032 of full scale at
800 px wide; a contrast stretch shows the same TVA table, dotted rule and date line in both,
ours the crisper). The corpus incomplete list loses the file; the jpeg2000 gate's
`NOT_COMPARABLE` entry for it changes reason — ours is now a reduced-level decode of the same
codestream the reference decodes whole, two of the format's own versions of one image.

## Consequences

- §7.4.9's ledger row now records the resolution progression as used, and stays `partial` for
  the baseline-restriction check and the thirteen codestreams one level off the reference
  software (`hayro-jpeg2000`'s, `doc/JPEG2000_FEEDBACK.md`).
- The soft-mask residue changes shape: what is not built is a codec-carrying *mask* decoded at
  a device-chosen grid — a per-draw decode where the packed-bytes route reads by indexing —
  and no corpus document states one. `eligible_for_the_device_scale` says so in place.
- A `pdf-sandbox` parent and worker must be rebuilt together across this change; the magic
  makes a mismatch a refused handshake rather than a misread response.
- The reduction is decided once per decode from a memory bound. Zooming does not re-decode a
  JPEG 2000 base at a finer level; the raster chosen is finer than any screen for every file
  the corpus has, and the device-grid refinement remains expressible on the vocabulary ADR
  0210 built if a witness ever asks for it.
