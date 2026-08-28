# Session 797 — A strip is a replay, and a reduction's cost is not in the rows it covers

2026-08-28. ADR 0731. Branch `round-797`, from `main` at `babb3f40`. Not merged.

**The finding.** `render-cpu` cuts a target into strips and replays the whole display list into
each, and `pdf_render::replay_ratio` bounds that replay by the **rows** a command covers — exact
for a fill and blind to `Image::area_averaged`, whose cost is per *source* sample. So a scanned
page reads as a replay of 1.00, is granted every strip the machine offers, and reduces the same
image once per strip. `issue12963.pdf` page 1 — a 2480×3506 `JBIG2Decode` scan on 596×842 — spent
**75.74% of the whole rasterisation** inside that one closure, and got **slower the more strips it
was granted**: 30.9 ms at sixteen against 14.0 at two.

**What it took.** A memo on the rasteriser keyed as ADR 0297 keyed `render-quorra`'s upload — the
samples' address, pinned, and the reduction's two factors — plus a warm pass that reduces each
image on the thread that plans the strips, before any strip is queued.

**The half of it that had to be learned rather than written.** The first construction blocked: an
`Arc<OnceLock<_>>` per key, so the first strip to ask ran the reduction and the others waited. It
measured better than what shipped and it **deadlocks**. `area_averaged` divides its rows across
rayon, and a worker waiting inside `par_chunks` can have another strip's job stolen onto its own
stack — a job that comes straight back to the same key, so the thread re-enters the lock it holds.
The corpus gate hung with every one of twenty-six threads in `futex_do_wait` at one per cent of a
core; `raster_digest` and `callgrind_rasterise` had each hung on the same page earlier in the
round and both were written off as a loaded machine. **A lock may be taken to read or write a map
and may never be held across work that itself uses the pool** — that is the rule, and it is why
the sharing is arranged by the warm pass rather than synchronised by a wait.

**Files.** `crates/render-cpu/src/images.rs` (new), `crates/render-cpu/src/lib.rs`,
`crates/pdf-render/src/paint.rs` and `crates/pdf-model/src/content/ext_gstate.rs` (comments),
`doc/conformance/ledger.toml` (§10.7.4, §8.9.5.3), `doc/adr/0731`, `doc/performance.md`,
`doc/todo/45-where-a-frame-goes.md`.

**The spec side.** §10.7.4's row said `Stroke::device_width` resolves a zero width "so that both
backends draw one mark", and this workspace states three rasterisers, all three of which call it —
`render_cpu::convert`, `render_gpu::scene`, `render_quorra::stroke`. ADR 0697's shape, ranked by
`doc/todo/01`'s twenty-second sweep, which puts `pdf-render` at the closest rung because it is the
crate the whole population depends on. The same sentence stood in `pdf-render`'s own `Stroke`
documentation twice and in `pdf-model`'s `/SA` comment; and the sweep's next two hits, once those
were out of the way, were `Image`'s own — "neither backend needs to know about PDF colour spaces"
and "[b]oth backends ask this" of `is_smoothed`, which `render_cpu`, `render_gpu::scene` and
`render_quorra::scene` all call. All six now count what is there. The
clause is met by *more* backends than the sentence claimed, which is what makes the decay silent.

**What is left**, and it is in ADR 0731 and `doc/todo/45`: `render-gpu` still recomputes per draw;
§11.6.5.2's deferred soft-mask image has no address that outlives a draw, so each strip still
produces its own; and `replay_ratio` still has no term for a command whose cost is not in the rows
it covers.
