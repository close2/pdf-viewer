# 1888 — The inflate is pipelined into the divided read, not divided, and the next build is the drawer

Session 1526, the step back of batch eighty-three on the project owner's target of 2026-10-10,
"improve performance": the first frame, zooming, the processor and the device, with a step back
every batch by first principles and by finding out how others do it. Status: **accepted**. It picks
the next build and builds nothing. Context: ADRs 1855 and 1873, and `doc/todo/69` §6–§7.
Instrument: `crates/pdf-model/examples/serial_terms_price.rs` (new); the draw on the pool is
`crates/render-raster/examples/pipeline_price.rs`. Witness: `/tmp/demo_heavy_engineering_A1.pdf`.
Clauses: §7.4.4.1 (Flate is RFC 1950/1951), §7.7.3.3 Table 31 (`/Contents`), §8.4.2 (the cut). No
ledger row moves.

## 1. What is left serial once the operators run on many threads

ADR 1873 priced B, one page read on many threads cut where `Q` restores the state, at a 608 ms read
against 1 828 ms. Its four terms were the inflate, a planning pass, the pieces at once and the join.
Session 1522 builds B. This ADR prices what B leaves on one thread. All figures come from HEAD
`56e42ea5`, built from an export into its own target directory, in one `--clock` hold (two passes,
interleaved, load 3.2–7.1, three rounds an arm). They are unpinned unless the row says so.

| term | figure | rate (output) |
|---|---|---|
| the stream as written | one part, `/FlateDecode`, 39 493 955 → 278 579 415 bytes (7.05×), zlib header `78 9c`, **zero** byte-aligned `00 00 FF FF` | |
| the inflate whole into one buffer (the decoder's floor) | 163–175 ms; pinned to the fast cores 155–164 ms | 1.69–1.80 GB/s |
| through the page's 64 KiB window (today's path) | 185–202 ms; pinned 178–188 ms | 1.47–1.56 GB/s |
| into place in 1 MiB chunks, each published | 165–181 ms; pinned 158–166 ms | 1.60–1.76 GB/s |
| the lexer over the bytes in memory | 380–424 ms; pinned 366–385 ms | **0.71–0.76 GB/s** |
| the inflate and the lexer through the window, one thread | 523–567 ms | 0.50–0.53 GB/s |
| the exact planning pass (a lex that counts depth), serial | 359–397 ms | 0.71–0.78 GB/s |
| a byte scan for a `Q` between delimiters | 60–65 ms (4 400 170 candidates; the page has 4 408 405 commands) | 4.6 GB/s |
| the drawer, one thread (`render_cpu::Underway`, 85 parts, 1 416 × 1 000) | 1 108–1 194 ms painting (one run 1 664 ms), 64–87 ms finishing | |
| the draw whole on the pool, 12 threads / 24 threads | **490–508 ms / 505–525 ms** | |
| the serial read and the headless frame, for scale | read 2 093–2 101 ms; read then draw 2 615–2 624 ms | |

The pieces were timed two ways. In the first, every piece waits for the whole inflate. In the
second, a piece starts once the inflate has passed its last byte. Either way the threads take the
pieces in order, and an inflating thread runs beside them. In every count the joined list equals
the page's list. Neither the join nor the hand-over is in these figures.

| pieces / threads | after the inflate | behind the inflate | behind: inflate ends · first piece · half in order · all (best run) |
|---|---|---|---|
| 12 / 12 | 400–641 ms | 364–431 ms | 178 · 215 · 268 · 364 ms |
| **48 / 12** | 394–442 ms | **268–306 ms** | 196 · 52 · 166 · 268 ms |
| 48 / 20 | 383–581 ms | 285–331 ms | 221 · 46 · 188 · 285 ms |
| 8 / 8, pinned to the fast cores | 575–694 ms | 518–590 ms | 177 · 361 · 484 · 518 ms |

## 2. How others do it — evidence about approaches (principle 5)

- **mupdf** reads a content stream through a chain of filters (`pdf_open_contents_stream` →
  `pdf_open_filter` → `build_filter` → `fz_open_flated`), and the lexer pulls bytes from it as it
  needs them. The inflate is interleaved with the lexing on one thread, which is how this tree's
  window already reads.
- **pdf.js** inflates a page's streams whole before it interprets them. `Page.getContentStream`
  (`src/core/document.js`) sends an `isAsync` stream to the platform's `DecompressionStream`
  (`asyncGetBytesFromDecompressionStream`, `src/core/decode_stream.js`) and inflates a `/Contents`
  array's parts together under `Promise.all`; then one evaluator loop reads the result.
- **pdfium**'s `CPDF_ContentParser` decodes each stream whole (`LoadAllDataFiltered`), joins the
  streams, and then parses `kParseStepLimit` (100) operators between pauses. Its progress is spread
  over time, but the work stays serial.
- **Acrobat** publishes nothing on this, so it is no evidence either way.
- **pigz**'s manual: "Decompression can't be parallelized, at least not without specially prepared
  deflate streams for that purpose." It decompresses on one thread and uses three more for reading,
  writing and the check value. Its `--independent` option is that preparation: each block starts
  after `00 00 FF FF 00 00 00 FF FF` with no dictionary. The witness has no such marker.
- **rapidgzip** (Knespel and Brunst, HPDC 2023) does divide a stream with no preparation. It finds
  block boundaries by speculation, decodes each chunk with markers standing for the back-references
  it cannot resolve yet, and resolves them once the previous chunk's 32 KiB window is known. Its
  README says that window propagation is serial, and reports up to 24 GB/s with an index and
  12 GB/s without on 128 cores (Silesia), with one core about as fast as igzip.

None of the viewers spreads one stream's inflate over threads. pdf.js's parts are the only
division any of them makes, and Table 31 provides that division itself. An array is "as if all of
the streams in the array were concatenated with at least one white-space character added between
the streams' data, in order, to form a single stream", and "[t]he division between streams may
occur only at the boundaries between lexical tokens". So each part is a filter run of its own that
starts at a token boundary. The witness has one part.

## 3. The options, priced

- **Divide the inflate.** The producer gave the stream no restart point, so dividing it means
  writing rapidgzip's decoder in-tree, in safe Rust: a speculative block finder, a marker decoder
  and a serial window pass. zlib-rs offers none of the three. The most it can win is bounded by
  the table: behind the inflate, the read ends 63–79 ms after the inflate does, and the pieces'
  own floor (the after-the-inflate read less its inflate) is 226–261 ms. A free inflate takes the
  read from 268–306 ms to that floor, **at most 42–45 ms**. **Not built.** It gets priced again
  when the pieces' floor falls below the inflate.
- **Pipeline the inflate into the divided read.** On 48 pieces this takes **394–442 ms to 268–306
  ms**, 6.8–7.8× the serial read. The inflate stays on one thread and the pieces hide behind it.
  The hypothesis said the opposite: the inflate would hide behind the pieces. By quantity it still
  holds: 126–136 ms of a 163–175 ms inflate leaves the critical path. Two conditions decide it:
  - **Many pieces, at least four a thread.** A piece cannot start before its last byte is out, so
    the last piece starts when the inflate ends. With twelve pieces that last piece is a twelfth of
    the work, and the gain falls to 364–431 ms.
  - **The planning is not a serial pass behind the inflate.** The lexer reads at 0.71–0.78 GB/s
    and the inflate writes at 1.69–1.80 GB/s, 2.3× faster, so an exact planner on one thread
    becomes the floor at 359–397 ms. A cut is guessed by bytes instead, from a `Q` between
    delimiters near each target (4.6 GB/s over the whole stream, and only a window near each
    target is read). The piece before it confirms the guess when its own lex arrives there
    between tokens, at the depth it summarised, with none of the things ADR 1873 §5.2 refuses
    left open. A guess that fails moves the cut, never the list. A `Q` inside a string, a comment
    or an inline image's data is such a failure.

  The producer inflates into place, so the window's 22–27 ms copy is not paid twice. The inflating
  thread is the critical path, so it belongs on a fast core (trap 154). Running beside the pieces,
  it slows from 163–175 ms to 177–254 ms.
- **Start the drawer from the first piece.** On one thread, not as it is. `Underway` takes
  1.17–1.26 s (one run 1.75 s, at load 5.1), which is longer than the pipelined read plus the whole
  draw on the pool, 0.76–0.83 s.
  A read this short is also under `GLIMPSE_AFTER` (500 ms), so ADR 1855's drawer would not start at
  all. The frame becomes the read, then the draw on the pool.
- **The draw on the pool is now the first frame's largest term**: 490–525 ms against a read of
  268–306 ms. It does not move between 12 and 24 threads, so some part of it does not divide.

## 4. Decision

1. **ADR 1873's build pipelines the inflate**: at least four pieces a thread, each started when the
   inflate has passed its end, cuts guessed by bytes and confirmed by the piece before, and the
   inflate into place on a fast core. `doc/todo/69` §7 holds it as a requirement of §6's build. It
   is not a separate build.
2. **The inflate is not divided.** Doing so is worth at most 42–45 ms and costs a new RFC 1951
   decoder.
3. **The next build for the first frame is the drawer.** First, find what in the draw on the pool
   does not divide (490–508 ms on 12 threads, 505–525 on 24). The strip planner, which session 1523
   moves onto the pool, is the first suspect. Then the pooled drawer takes the pieces in order as
   they finish: on 48 pieces behind the inflate the first is done at 44–55 ms and half at
   166–206 ms. Its gain is bounded by the read, 0.27–0.31 s. The projection is a headless frame of
   0.76–0.83 s against 2.61–2.62 s once pipelined, and 0.5–0.6 s with that drawer. These are
   projections, and the builds measure them.

## 5. Not measured here

- **The hand-over.** Session 1522's hypothesis measures it: each piece's list handed over as
  parts, against ADR 1873's 117–149 ms join, which copies 529 MB serially.
- **The window.** No figure here needs `quorra`. The owner's display is the builds' to measure.
