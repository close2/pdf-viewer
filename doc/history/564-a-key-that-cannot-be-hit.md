# 564 — A key that cannot be hit

2026-08-17. Takes session 560's latency finding: the two `timeout-` artefacts its `page` fuzz run
left, which it recorded for `doc/todo/10` and `doc/todo/16` and moved on from. ADR 0399 has the
argument; this is what the round did and what it ran.

## What the two documents were

`fuzz/artifacts/page/timeout-25f1b7e6…` and `…-b36ba567…`, byte-identical to
`corpus-cache/safedocs/cc-main-2021-31/4851/4851530.pdf` and `…/3375/3375489.pdf` — found by hashing
every file in the cache of that size rather than by reading a name off a list. Both `CubePDF` through
`iTextSharp 5.5.13`; **A** is a Japanese bus timetable whose 3 198 grid rules are `/PaintType 2`
tiling patterns with a `[0 0 1 1]` cell holding an 8 × 8 inline stencil, **B** states 12 092 inline
images in its own `/Contents` of which five are distinct.

**Neither is a bomb**: every operator either draws is one the file states. A is stopped by
`MAX_OPERATIONS` at four million of them, which its ~327 000 pattern cells reach honestly; B is
complete and reports nothing.

## The attribution, and it was not where three rounds of documents said to look

`examples/callgrind_interpret` under `RAYON_NUM_THREADS=1`. **72.69% of A and 88.46% of B was
`Interpreter::draw_image`**, and `image::unpack` — where the samples are actually decoded — was
0.50% and 1.46%. The second line of A's profile is a libc `memmove`, 23.17%, **64 608 calls all from
`draw_image`**, which is `entries.remove(0)` evicting from the front of a 262 144-entry `Vec` once per
insertion after the 64 MiB budget filled.

Both lines are one defect: `image::RasterCache`'s probe is a linear scan, and §8.9.7's inline image
is a fresh allocation at every `BI`, so it added one entry per *draw* that its own key could never
find. ADR 0374's doc comment had said exactly that — "while never hitting in it" — as a harmless
property. *N* draws that each insert an unfindable entry cost *N*²/2 probes.

## What was built

`image::StreamIdentity` — `Allocation` for a stream §7.8.3's dictionary hands out, `Content(u64)`
for §8.9.7's, with the content compared exactly beside the digest the way
`DisplayList::add_clip` compares a clip it found by one. `image::NamedStream` carries a stream and
its name together, because the whole defect was a caller that had the first and not the second.
**No constant moved**: `MAX_TILES` 4096, `MAX_OPERATIONS` four million, `RASTER_BUDGET` 64 MiB.

| | before | after |
|---|---|---|
| A wall clock / peak resident | 122.7 s / 657.6 MB | **0.93–1.26 s / 129.8 MB** |
| A instructions | 330 490 549 519 | **10 663 108 878 (−96.77%)** |
| B wall clock / peak resident | 12.0–14.3 s / 383.6 MB | **0.47–0.63 s / 75.4 MB** |
| B instructions | 71 859 030 215 | **6 295 686 734 (−91.24%)** |
| ISO 32000-2 page 101 × 50, no image on it | 1 216 998 583 | 1 216 999 004 (**+0.00003%**) |

After it, neither profile mentions `draw_image`: A is `Lexer::next_token` 16.60%, the dispatch
10.96%, `get_key` 7.73%, `add_clip` 5.80%.

## The crasher the fuzz run then found, which was nobody's change

`crash-b0fb6133…`: `AddressSanitizer: stack-overflow`, 250 frames of `decode_parts` →
`apply_explicit_mask` → `decode`. An image whose §8.9.6.3 `/Mask` names an image mask stating a
`/Mask` of its own had **no bound at all**, and Table 143's `/Mask` row was unread on §11.6.5.2's
side while its `/SMask` row was guarded. Both are refusals now, not depth bounds: Table 87 and
Table 143 each say the entry "shall not be present", so the standard's depth is one and no constant
is needed. `tests/hostile_budgets.rs` states both shapes; each was confirmed to fail with its guard
removed, and the artefact runs in 192 ms.

## What a person sees

**A slow open is the one place a person waits with no feedback** — `doc/todo/36`'s cadence work is
merged, so a page turn and a zoom have a frame every refresh, and a document that has not opened has
no window to put one in. Before this round, opening A meant **two minutes of nothing** and then a
page reported incomplete; opening B meant twelve seconds. Both are now inside the second that a
launch already costs, so there is nothing to report and nothing to build: the honest answer to "what
does the window do during that time" is that there is no longer a *that time*.

The one document in this tree for which the question stands is still `tmp/Entwurf.pdf`, which is
`doc/todo/16`'s witness and is not in the repository. That file's amendment this round says so
explicitly, because the two witnesses filed there in the five-hundred-and-sixtieth session were a
defect rather than a case for a resumable interpreter — and the lesson written down beside it is
that a latency finding is a defect until it has been attributed, because work nobody asked for looks
from the outside exactly like work the file asked for.

## Output identity, which is the whole of the correctness claim

- `examples/display_list_digest` over **1222 first pages** of the 1231 documents opened from the
  pdf.js corpus and the four submodule corpora: **byte-identical**, both arms with the same
  `pdf-sandbox-worker` on disk. Re-taken after the mask guards landed, still identical.
- `examples/readback` over **all 1023 pages** of ISO 32000-2, concatenated: 2 730 201 bytes,
  `sha256 ed074b1c00292534cc7ccb5fa16e848b99654031e75974aaad62599401ccf21e`, `cmp` silent — the same
  digest session 500 recorded.

So **no pixels move**, proven rather than asserted, and neither a quorra `gpu` lane nor
`doc/todo/00`'s step 7 ink sweep is owed (ADR 0379's precedent).

## The gates

- `cargo fmt --all --check` silent. `cargo clippy --workspace --all-targets` silent of lints; the
  `viewer-qt` `cxx-qt` gcc warnings and the `proc-macro-error2` future-incompat note are
  `doc/todo/02`'s known non-lints.
- `cargo nextest run --workspace`: **2093 tests run: 2093 passed, 15 skipped**, from a base of 2088
  — the five new tests are the whole of the difference.
- `cargo test --workspace --doc`: all `ok`. `cargo test -p conformance`: **5 passed**, `8577
  citations` and `825 quotations, all verbatim within the clause they cite`. The TOML was broken
  twice on the way, by a note whose new prose quoted the standard with unescaped quotation marks —
  which is `doc/todo/02` §6's rule about checking the file rather than the script.
- **corpus**: `974 documents in 2.7s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 65
  incomplete, 0 slow`, with silence lines at 5 codes over 2 documents and 1226 over 41. Every field
  unchanged.
- **oracle**: `1794 pages in 35.5s (1690 we call complete, 104 incomplete)` — `agrees` 906/862,
  `contradicted` 67/66, `ambiguous` 786/753, our geometry 1/0, reference geometry 2/2, `not
  comparable` 13/7, `no render` 19/0, 99.8% reference-cache hit rate. **No page moves.**
- **text_extraction** (two gates): pdf.js `974 documents … overall 99.2% (22836/23015 words)`, 22
  below 90%; PDFBox `40 documents … 99.8% (14257/14281)` both orders, 4 below 90%. Plus the
  positional gate, `10969/11163 matched words in bounds (98.26%)`.
- **dates**, **xmp**, **jpeg2000**: pass. **render-quorra corpus** on the real Radeon 890M:
  `956 pages compared in 30.7s: 931 agree, 23 differ, 2 refused, 18 not comparable`, unchanged.
- **Fuzzing**: `page`, seeded with 2306 units — the tree's corpora plus every twenty-sixth SafeDocs
  member, the sample session 560 used and for the reason `doc/todo/02` §2 gives, plus the two
  witnesses and the crasher — with `-fork=6` for fifteen minutes: `#38187: cov: 34622 ft: 173466
  corp: 5076 oom/timeout/crash: 0/0/0`, five `slow-unit-` artefacts and nothing else. The crasher
  above came out of the *first* such run, before the mask guards existed.
- **The two `timeout-` artefacts are gone, and that was checked rather than inferred.** Run through
  the instrumented target directly they are **25.1 s** and **6.9 s** where they were timeouts, and the
  seeded run reports them as `slow-unit-` instead. They are removed from `fuzz/artifacts/page/` with
  that as the reason; what replaces them in the suite is a *bound* rather than a deletion —
  `image_reuse.rs::every_cell_of_a_hatching_shares_one_decode` asserts that a 42 × 42 lattice of
  cells whose content is one inline stencil holds **one** raster allocation between them, and it fails
  on the tree as it was.
- Neither witness is committed and neither may be: both are SafeDocs members of a Common Crawl
  archive, which `.gitignore`'s licence position and `doc/third-party-data.md` keep out of this
  history. Every fixture this round added is generated, and each says so.

## Where this worktree differed from a fresh clone

`doc/md`, the fourteen `doc/*.pdf`, `doc/pdf.js` and the four `doc/corpora` submodules are symlinks
into the main checkout; `target/tmp/pdfref-cache` is shared with it, which is what keeps the oracle's
hit rate at 99.8% instead of a thousand seconds of reference renders. The build directory is named in
a worktree-local `.cargo/config.toml` rather than exported, for ADR 0344's reason.
