# 0801 — A CPU-second is not a unit of work, a font cache per split, and the transform gate

Session 868. Status: **accepted**. The second decision record of RFC 0002's implementation, on
the long-lived branch `round-867`.

## Context

Session 867's baseline for `pdf-transform render` (its history file §3) read as a defect: 200
pages of ISO 32000-2 at 150 dpi cost 7.9 s of CPU on one thread and 18.9 s on twenty-four — 2.4×
the processor time for the same pages. Two suspects were named without being measured: one
`FontCache` per rayon worker, and `interpret`'s own inner banding contending with the outer
loop. `doc/todo/57` made attributing the gap the first thing this round does, and RFC 0002
section 12 owed a gate with a perf floor from the suite's first landing, which round 867 had
not created.

## What was measured, in one sitting

The `gates`-profile binary, a quiet machine, pages 1–200 to PNG, `RAYON_NUM_THREADS` set by hand.
The baseline reproduced first (1.06 s wall, 20.4 s CPU at 24 threads; 8.17 s and 7.96 s at one).

**The thread curve, before any change**, is what said the two suspects were the wrong shape:

| threads | wall | CPU |
|---|---|---|
| 1 | 8.17 s | 7.96 s |
| 2 | 7.92 s | 14.35 s |
| 4 | 2.93 s | 11.26 s |
| 8 | 1.96 s | 14.22 s |
| 12 | 1.61 s | 17.51 s |
| 24 | 1.06 s | 20.38 s |

Two threads do the work of one and burn the CPU of two — and four threads cost *less* CPU than
two. Neither contention nor inner banding produces a curve with a bump at two; something whose
*count* varies with how the work is split does.

**The arm that removed the first suspect** — one `FontCache` shared by every page, which
`pdf_model::content::FontCache` already permits (it is behind a mutex, `Sync` since ADR 0710) —
took the two-thread row to 4.12 s wall at 7.91 s CPU, the four-thread row to 2.43 s at 9.31 s,
and moved the twenty-four-thread row **not at all**: 1.08 s at 19.86 s.

**The arm that removes every in-process suspect at once**: twenty-four separate single-threaded
processes over disjoint ranges of the same 200 pages, sharing no cache, no lock, no allocator and
no document. Their CPU seconds sum to **20.6 s** (1.34 s wall); twelve such processes sum to 16.2 s
against 17.3 s for twelve threads in one process.

## Decision

1. **The "2.4× CPU gap" is the hardware's, and it is not a cost to fix.** This machine has twelve
   cores with two hardware threads each and a single-core boost well above its all-core clock
   (`lscpu`: 5.16 GHz maximum, `powersave` governor). A CPU-second at twenty-four threads is worth
   less than half a CPU-second at one, and twenty-four processes that share nothing show it as
   plainly as one process with twenty-four threads. The suite is a throughput tool (RFC 0002
   section 12) and its wall clock is 7.6× better at twenty-four threads than at one, which is
   the number that describes it. `doc/performance.md` carries the rule this leaves behind: **on
   this machine, compare against N separate processes before attributing CPU time to sharing.**
   `doc/habits.md` already says wall-clock benchmarks lie under load; this is the same lesson for
   CPU time under simultaneous multithreading.

2. **The defect that *was* this crate's is fixed: one font cache per run, not one per rayon
   split.** `map_init`'s constructor runs once per split of the parallel iterator, and a split is
   what a steal makes — so the number of caches grew with the stealing, each one parsing the
   document's fonts again, and the cost showed exactly where a CPU-second was still worth what it
   was: at two and four threads. The shared cache halves the two-thread wall clock and takes a
   fifth off the four-thread one. It is one field moved (`Job::fonts`), and the module comment of
   `src/render.rs` carries the table, which is what `CLAUDE.md`'s "an optimisation carries its
   benchmark number and a comment" asks. The rasteriser stays per job: `CpuRasterizer::new()` is
   two words and an empty memo.

3. **The transform gate exists, with the floor RFC 0002 section 12 asked for.**
   `crates/pdf-transform/tests/gate.rs`, on `doc/todo/02` §2's sequence and in `tools/state.sh`.
   It times `render` of the same 200 pages **through the program the build produced**
   (`CARGO_BIN_EXE_pdf-transform`, trap 16), prints the pages per second it measured, and holds
   them above **40**: a fifth of this round's measurement, and above the single-thread figure, so
   that losing the cross-page parallelism trips it while a neighbouring round's gate sequence on
   the same machine does not. It then holds one of the two hundred pages byte for byte to the
   oracle backend's raster produced independently in the test, and holds the `images` and
   `attachments` inventories to **walks of the document written in the test** — the image
   `XObject`s the page tree's resources reach, forms descended, by object id; and the file names
   §12.5.6.15's annotations carry, page by page — rather than to what the tool printed once.
   It calls `require_the_sandbox()` like every other gate that decodes an image.

4. **The three halves `doc/todo/57` §4 named landed**, each with a test on a document that has
   one:
   - **Inline images** (§8.9.7) in `images`: the page's content and each reached form's content
     are lexed with `pdf_syntax::Lexer` for `BI`, and what follows is read by
     `pdf_model::inline_image::scan` — the interpreter's own reader, so there is one in the tree.
     Listed at every placement, with `inline: true` and no object number. The fixture is the
     corpus's `issue11124.pdf`, whose 48 unfiltered bytes contain `EI` twenty-four bytes in:
     §8.9.3's arithmetic is the only way to find its end, and the test holds the first two
     decoded pixels to the bytes the file states.
   - **`--native`**: `DCTDecode` written as `.jpg` and `JPXDecode` as `.jp2`, the bytes being
     `Document::image_stream`'s — every filter in front of the codec run, the codec not — and
     everything else decoded to PNG; `JBIG2Decode` and `CCITTFaxDecode` decoded *with a warning
     naming the codec*, per image, because neither is a file on its own and RFC 0002 section 6.3
     declined sidecar formats. The extension is appended to the expanded name under this flag,
     because the caller cannot know which of three it will be. `.jp2` over `.jpx` is a choice and
     is documented as one: §7.4.9 says the data is "a full JPX file structure", JP2 is its subset,
     and `.jp2` is what readers open. What a native JPEG loses — `/SMask`, `/Decode` — is stated
     in the module comment and the usage text rather than hidden.
   - **File attachment annotations** (§12.5.6.15) in `attachments`: every page's `/Annots` walked
     for `/Subtype /FileAttachment`, each read by `pdf_model::attachment::of_annotation` — the
     viewer's reader, which makes the annotation's `/Contents` the description as the clause's one
     `shall` requires — deduplicated by stream against the tree's and `/AF`'s files, listed with
     the page. ISO 32000-2's own PDF files nothing in its name tree and carries every one of its
     six files this way; before this round `attachments` listed nothing for it.

## Consequences

- No first-row crate changed: `pdf-transform` and its tests, two documents and `tools/state.sh`.
  The core gates and the conformance gate are the sequence owed, and the new gate ran green.
- `tools/conformance/tests/sandbox_gates.rs` now demands `require_the_sandbox` of the transform
  gate, because the gate line is in §2's block; it has it.
- The floor is a wall-clock number and inherits the habit's warning about load. It is deliberately
  loose for that reason; a regression of the kind item 2 fixed shows in it only when the machine
  has few threads, and the instrument for that class remains the thread curve above, taken by
  hand.
- What the suite still owes is `doc/todo/57`'s list, shortened: `--no-mask`, `render`'s
  `--page-box` and `--no-annotations` (first-row changes), the worker split, the two misplaced
  items, and everything blocked on RFC 0002 §13.
