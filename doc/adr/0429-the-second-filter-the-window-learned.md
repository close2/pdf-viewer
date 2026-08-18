# ADR 0429 — The second filter the window learned

Status: accepted, 2026-08-19. Session 594. Takes the second of the two residues ADR 0427 left at
the end of `doc/todo/14`: a pump for the §7.4 filters that are not `FlateDecode`. ADR 0365 windowed
a page's `/Contents`, ADR 0427 the four other content streams §7.8.2 names; both pumped a single
`FlateDecode` and handed everything else back whole. Amends §7.4.4.2 and §7.8.2's ledger rows.

## What changed, in one line

**A decompression bomb encoded with `LZWDecode` stops being an allocation.** A 2.37 MB file whose
`/Contents` names a 1.5 GiB LZW decode costs **10 MB of peak resident memory and reports
`MAX_OPERATIONS`** where whole-decoding it cost **1035 MB and refused `TooLarge`** — and it is
*faster*, 0.11 s against 2.12 s, because the window never allocates the gibibyte the whole route
builds and then throws away.

## Why LZW, and why only these two

`doc/todo/14`'s second residue named four filters and ranked them: LZW reaches about **1365:1** on a
long run of one byte (NOTE 2 of §7.4.4.1 says so outright), which makes it the sharper bomb, so "if
you take only one, take that one". This round takes LZW and, in the same routing change, admits it
beside `FlateDecode`. The other three are left out on their own expansion ratio, which is exactly
what a bomb is a small input naming a large one:

- `ASCIIHexDecode` produces **one byte per two** hexadecimal digits — its output is *smaller* than
  its input, so no file inflates through it at all.
- `ASCII85Decode` produces four bytes per five characters, or four per `z`: **4:1 at the very
  most**, and only from a stream that is nothing but `z`. §7.4.3 also makes an out-of-grammar
  character one that "shall cause an error", discarding the whole decode — which a window, having
  already handed its bytes to a lexer, cannot do.
- `RunLengthDecode` produces at most 128 bytes per two: **64:1**.

So the two filters that can name a bomb are the two that are pumped. `doc/todo/14` carries the three
ratios and what pumping the remainder would still need, against the day one is wanted for a reason
other than a bomb.

## The construction: one decoder, two loops

Trap 6 is the hazard — "a second decoder beside the first is how two implementations of one clause
drift" — and `doc/todo/14` names the shape that avoids it: make the existing decoder *resumable*, a
state struct with a `pump(&mut self, out: &mut [u8])`, and express the whole-buffer entry point as a
loop over it. That is exactly what `FlateDecode` already did through `filter::turn`, shared between
`inflate_buffer` (grows a `Vec`) and `Pump` (fills a window).

So §7.4.4.2's algorithm moved into a `filter::Lzw` struct — its table, its bit accumulator, its
input cursor — with a `step(&data)` that reads one code and leaves the sequence it names in
`pending()`. `filter::lzw` (the whole decode) is now a loop over `step`; `Lzw::pump` (the window)
hands the same sequence over in as many turns as the window has room for. The three details that
decide whether an LZW decoder is right — the width growing before the entry that needs it, a code
naming the entry about to be created, high-order-bit-first packing — are stated once, on `step`,
each beside the sentence of the clause it comes from.

`filter::Pump` gained a `Pumping` enum (`Inflate` or `Lzw { early_change }`) and became an `Engine`
of two variants. The route is chosen once, by the document, and *carried* — `Pump::pumping()`
returns it — because one of §7.8.2's content streams is read more than once and a fresh pump is made
per read, so the second read must build the same decoder without asking the question again and
possibly answering it differently.

`Lzw` is boxed inside the engine: §7.4.4.2's table is twelve kilobytes, and an inflating pump would
otherwise carry room for a table it will never fill.

## The route is one function, and it moved

`Document::is_pumpable`, which returned a `bool`, became `Document::pumping`, which returns
`Option<filter::Pumping>` — the one place a chain's route is decided, so a page's `/Contents`
(`stream_source`) and §7.8.2's other four (`nested_content_source`) cannot answer it differently,
and so that a filter gaining a pump is one edit. It reads the stage's `/DecodeParms` exactly as
`decoded_stream_data_reported` does — the predictor guard is unchanged — and now returns
`Inflate` for `FlateDecode`/`Fl` and `Lzw { early_change }` for `LZWDecode`/`LZW`, reading Table 8's
`/EarlyChange` (default 1) so the two routes cannot disagree about which bit stream the codes are.

## What binds, from principle 3

- **The bound removes the allocation rather than surviving it.** The whole-buffer `lzw` keeps its
  `Limits::max_stream_len` guard — a bound is a statement about an allocation and that route has one
  — but the windowed route has no allocation to bound, so it makes none: the aggregate bound is the
  reader's, §7.7.3.3's array bound `content::reader` already applies over the whole content. An LZW
  bomb reaches the window and is read, in a buffer that never grows, until `MAX_OPERATIONS` stops
  the *program* it decodes to.
- **A refusal is refused by name.** A page's LZW `/Contents` that outgrows the reader's aggregate
  bound reports `ContentIssue::TooLarge`; a corrupt or truncated LZW stream reports its
  `Damage`, met where the pump reaches it, exactly as ADR 0343 requires and never dressed up as the
  damaged-prefix sentence that belongs to the producer's own bytes.
- `#![forbid(unsafe_code)]` holds; every error is `FilterRefusal`/`ContentIssue`/`StreamRefusal`,
  typed and propagated.

## The measurement

The bomb is rebuildable from `tmp/`-less description: `lzw_bomb.py` encodes ISO 32000-2 §7.4.4.2's
codes with Table 8's `/EarlyChange` default, exploiting that the code sequence is periodic for
periodic content so one clear-to-clear epoch is computed and repeated. The unit is `n\n` — §8.7.2's
no-op path operator and an EOL — so the decode is *operators* rather than one long token. A/B in one
sitting with this round's own patch (`Document::pumping` reverted to Flate-only for the "before"
arm), release binary, `RAYON_NUM_THREADS=1`, peak from `VmHWM` in `/proc`:

| the 1.5 GiB LZW bomb in `/Contents` | before (whole decode) | after (pumped) |
|---|---|---|
| peak resident | **1035 MB** | **10.2 MB** |
| wall clock to interpret | 2.12 s | **0.11 s** |
| report | `TooLarge { part: Some(0), limit: 1 GiB }` | `MAX_OPERATIONS` |

Both directions improve, which was the prediction: the whole route allocates a gibibyte it then
refuses, and the window allocates none.

## Correctness

`examples/display_list_digest` over every pdf.js corpus document's page one — 975 lines, `sha256
04a07587…` — is **byte-identical** across the change, with the same `pdf-sandbox-worker` on disk for
both arms. The digest is the display list's own `Debug` rendering reduced to a per-document hash, so
a change to what any page draws would move a line; none moved. No committed corpus document states a
single-`LZWDecode` content stream large enough to route through the window (trap 8), so the
routing change is invisible to the corpus by construction — which is why the unit tests carry it:
`an_lzw_pump_and_the_whole_decode_agree` reads the clause's own example and a table-filling bomb
through windows of 1, 3, 7, 64 and 4096 bytes and asserts each equals the whole decode;
`the_lzw_pump_reports_the_damage_the_whole_decode_does` pins the two `Damage` cases;
`an_lzw_bomb_costs_the_window_rather_than_its_decode` reads a bomb the whole route refuses through a
4 KiB window that never grows; and `nested_content_window.rs` gains an LZW arm asserting a form
routed through the window draws what the whole decode draws.

## Fuzzing

`page`, seeded with `seed_nested_content.py`'s 26 nested-content documents plus a small LZW
`/Contents` page and an LZW form whose decode outgrows the memo, under `-fork=6 -rss_limit_mb=4096
-timeout=60` for about ten minutes. **No crash, OOM or new hang.** One `timeout-` artifact minimised
during the run is a `FlateDecode` form driving inline-image `EI`-scan lookahead — 59 s in a release
binary, single-thread, and it *terminates* — which is a pre-existing inline-image pathology
(§8.9.7's unbounded lookahead) rather than anything this round touched: it names no LZW, and the
Flate path's behaviour is the byte-identical one the digest proved.

## What road D still owes

One residue, unchanged from ADR 0427: **§8.7.3.1's tiling cell still costs its gibibyte.** A cell is
run once per cell painted and `Tiling` holds its decode for the whole tiling, so windowing it would
trade an allocation for unbounded work — the measurement is 0.24 s held against 9.0 s windowed. The
fix is `pdf_render::Repeats`, the cell drawn once and its commands repeated, one step past
`fold_repeated_marks`; it is a `pdf-render` change rather than a filter one, and `doc/todo/14`
carries it as the last thing road D owes.

## Files

- `crates/pdf-syntax/src/filter.rs` — `Lzw`, `Step`, `Lzw::step`, `Lzw::pump`; `lzw` a loop over
  `step`; `Pump` gains `Pumping`/`Engine`/`Inflate`; `Pump::new`, `Pump::pumping`; the four new
  tests and the `pack_lzw`/`lzw_bomb`/`drain` helpers.
- `crates/pdf-syntax/src/document.rs` — `is_pumpable` → `pumping`; `stream_source`,
  `nested_content_source` build the pump from it.
- `crates/pdf-syntax/src/lib.rs` — exports `Pumping`.
- `crates/pdf-model/src/content/reader.rs` — `Nested::Windowed` carries the `Pumping`;
  `Window::single` takes it.
- `crates/pdf-model/tests/nested_content_window.rs` — `Coding` enum, `lzw_literals`, the LZW form
  arm.
- `doc/conformance/ledger.toml` — §7.4.4.2, §7.8.2.
- `doc/todo/14`, and this round's history file.
