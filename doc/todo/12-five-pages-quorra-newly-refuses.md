# Five pages quorra newly refuses — diagnosed, reproduced, and waiting on one line upstream

Status: **diagnosed and handed over.** The gate fails here until quorra is fixed; the project
owner took the upstream half in the three-hundred-and-eleventh session.
Priority: 12 — a defect: five documents that drew do not draw.
Corpus: 5 of 974.
Clauses: none. A resource budget in a dependency.
Code: `crates/render-quorra/tests/corpus.rs`'s `REFUSED` ratchet; `doc/QUORRA_FEEDBACK.md` §10.

## What fails

```sh
cargo test --release -p render-quorra --test corpus -- --ignored --nocapture
```

```
assertion `left == right` failed: the pages quorra refuses have changed
  left: ["bug1703683_page2_reduced.pdf", "bug1721218_reduced.pdf", "issue12810.pdf",
         "issue14497.pdf", "issue1905.pdf", "issue9418.pdf"]
 right: ["bug1721218_reduced.pdf"]
```

`bug1721218_reduced.pdf` is the expected one — its coverage outgrows a 16384 × 16384 scratch
image, argued and recorded. **Five are new**, all "over the stated budget" of 256 MiB of
scene-derived bytes: 616 862 585 needed for `bug1703683_page2_reduced.pdf`, 312 400 361 for
`issue14497.pdf`, 280 762 806 for `issue12810.pdf`, and `issue1905.pdf` and `issue9418.pdf` beside
them.

## The cause, read and then reproduced

**`DEFAULT_MAX_FRAME_BYTES` did not move** — identical at `7d5dafb` and `7599081`. What a frame is
*charged* did, with the GPU coverage lane session 310 bumped quorra for.

`encode.rs` charges `winding.device_bytes()` **unconditionally** at the end of encoding.
`device.rs`, where the winding texture is actually created, adds those same bytes only
`if !winding.is_empty()`. `device_bytes()` is `width × height × 8` for an `rgba16float` target, and
`width`/`height` are stamped from **the whole scratch sheet** — which the CPU lane's tiles size
just as much as the GPU lane's, because both share one sheet.

So on `Coverage::Cpu` — the default, and what the offscreen rasteriser this gate drives gets —
every frame is charged eight bytes per texel of its entire coverage sheet for a texture that is
never allocated, never counted where allocations are counted, and never rendered into. **The
pre-flight check is stricter than the thing it checks**, which is the one direction a budget must
not be wrong in.

**Reproduced.** A local checkout of `7599081` patched with the guard the allocation site already
has:

```rust
if !winding.is_empty() {
    encoder.charge(winding.device_bytes())?;
}
```

and `[patch."https://github.com/close2/quorra"]` pointing at it, re-running the gate:

```
957 pages compared in 23.8s: 913 agree, 43 differ, 1 refused, 17 not comparable
```

913 / 43 / 1 / 17 is this gate's recorded state from before the coverage lane, to the number. The
fix costs nothing else and restores exactly what was taken. The patch was **not** committed — it
points into a scratch directory, and the fix belongs upstream.

## What is left here

1. **Nothing, until quorra ships it.** Written up as `doc/QUORRA_FEEDBACK.md` §10 with both call
   sites quoted and the reproduction; the project owner is taking the upstream half.
2. **When it lands**: bump `Cargo.lock`, re-run the gate, expect 913 / 43 / 1 / 17, and delete this
   file.
3. **Do not raise the budget** to make the gate pass. The constant exists because a GPU buffer
   sized from document-derived arithmetic is a decompression bomb under another name (principle 3),
   and a ratchet moved to accommodate a regression is trap 5 exactly.
4. **One question the fix does not answer**, left in §10 for whoever is there: the constant's
   comment says 256 MiB is "beyond any real page by orders of magnitude", and five real first pages
   were within reach of it. If that is entirely the phantom texture, the comment is safe; if it is
   not, a budget whose comment says it cannot be reached is a budget nobody re-reads.

## What this cost, and the habit it belongs to

The regression shipped in session 310 and was found in 311, by running the gates rather than by
any gate running itself. **CI runs none of this**: `every_corpus_page_agrees_with_the_cpu_oracle`
is `#[ignore]`d, as are the corpus, oracle, text, dates, XMP and actions gates, and
`cargo test --workspace` skips them all. Ten ignored tests over seven files, every ratchet in the
handover among them. That is its own item, and it is the reason this one had a session to hide in.
