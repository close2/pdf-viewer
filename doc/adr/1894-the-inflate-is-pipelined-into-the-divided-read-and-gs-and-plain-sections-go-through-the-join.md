# 1894 — The inflate is pipelined into the divided read, and `gs` and plain sections go through the join

Session 1529, on the project owner's target of 2026-10-14 ("[c]ontinue improving performance (and
fixing bugs) until you reach diminishing returns"). Status: **accepted** and **built** in
`pdf-model`. Builds ADR 1888 §4.1 inside ADR 1880, and carries two of ADR 1880 §2.4's declines.
Context: ADRs 1855, 1873, 1880, 1888, 1896; traps 13, 154, 166. Code: `content/divide/pipeline.rs`
(new: the producer, the composer, `read`, `hear`), `content/divide.rs` (`Division::pipelined`,
`AFTER_THE_INFLATE`, `last_pipeline_decline`, `Stack`, `Effect::{shift, closes, opens}`,
`step_over_dictionary`, `Chunk::closes_on_a_q`), `content/reader.rs`
(`ContentReader::extend_within`), `content/run.rs` (`Composed::marked`,
`Leftover::only_plain_sections_open`), `content/marked.rs` (`Marked::records_nothing`),
`progress.rs` (`Parts::spoil_if_handed`). Tests: `tests/drawn_while_read.rs` (two new fixtures,
one rewritten; the sweep counts the pipeline's fallbacks). Instrument: `examples/read_split_price.rs`'s
`built` line, three arms. Witness: `sample/demo_heavy_engineering_A1.pdf` of
github.com/cyfung1031/BergPDF. Clauses: §7.2.3, §7.4.4.1, §8.4.2, §8.4.5, §8.11.3.2, §14.6. No
ledger row moves.

## 1. The premise and the hypothesis

**The premise held.** `content/divide.rs` was HEAD's, and nothing in `pdf-model` or `pdf-syntax`
moved between `77fa741b` and `3fc3e7f1`. **The hypothesis held in one half.** The read lands at
386–443 ms on 24 threads, not under 350 ms (§3). The corpus count rose from 50 to 84, past 50.

## 2. Decision

1. **The pieces start as the inflate passes them.** A producer inflates the page's stream on one of
   the pool's threads. It writes into the buffer a piece is lexed from, a step of 256 KiB at a time,
   so no window copy is made. It hands the stream on in segments, at most four for each pool thread
   and at least a mebibyte each. Each segment ends after a `Q` that a byte scan found past an even
   share of the length the inflate predicts so far: white space before it, white space or a
   delimiter after it (§7.2.3). Each segment is lexed on the pool as soon as it is out. The lex
   confirms the cut only where its last token is a `Q` operator ending exactly at the segment's
   end, with no path or clip open. The effects compose in order under one lock, which sends the
   calling thread each fold in the stream's order. A piece starts once its segment is out and every
   fold before it is composed.
2. **A guess that fails moves the cut, never the list.** The calling thread reads the first piece
   into the page's interpreter, as ADR 1880 does (trap 154). It joins a later piece only after the
   lex of that piece's own segment has confirmed the piece's end. Some outcomes the pipeline does
   not settle: a cut that was not confirmed, any refusal, a first segment holding text, a stream of
   one segment. Each restores the interpreter and hands the stream, whole in memory, to ADR 1880's
   plan after the inflate (`Division::AFTER_THE_INFLATE`). That plan divides the page wherever it
   did before. The reason is kept for `last_pipeline_decline`. A lex refusal that a window reads
   differently goes back to the window, as before. A text object ends the cuts at the segment
   holding the first `BT`, and that segment and the rest of the stream are read as the last piece.
   A hand-over is spoiled only where a part was handed (`Parts::spoil_if_handed`).
3. **`gs` is a statement of the spine.** §8.4.2's `Q` restores what §8.4.5's `gs` set, so its
   statement is kept in order like a `cm`, and the next piece's spine states it again. The
   parameters that reach a page-level record are already declined by `outside_the_division` in the
   piece that states them: a soft mask, a transfer function, §11.6.4.3's alpha source, and the
   compositing record.
4. **A cut may fall inside a marked-content section that records nothing but whether it hides.**
   §8.11.3.2 decides visibility from the view's optional-content configuration, not from the
   graphics state, so the sections open at a cut are opened again after the spine's levels. The
   piece begins with that stack and with `hidden` set to the count of hiding sections. A section
   with an `/MCID`, `/ActualText`, a description, an artifact, an associated file or
   `ReversedChars` may not be open across a cut; the piece that would leave one open declines. A
   section closed inside a piece joins in order. Its spans are offsets into a readback that is
   empty before the last piece in the page's own reading too. A property list written inline
   (§14.6.2) is stepped over as `inline_dictionary` reads it. A shape that reader would read some
   other way declines.
5. **A caller on a thread of the pool waits by running the pool's work** (`hear`), because the
   producer and the pieces may be queued on its own thread.
6. **No hand-over from the join.** Round 1530 measured a `parts.offer` after each join on this
   tree's predecessor. The read slowed from 0.57–0.60 s to 1.005–1.009 s on 24 threads, because
   the offer copies each piece's command headers on the joining thread, which is the critical path.
   So the join hands nothing over. An offer that costs the critical path nothing — a range handed over, or the offer made from the
   piece's own thread — is the next build's to measure.

## 3. Measured

One `--clock` hold, headless and on the owner's display. The witness's md5 is `166bb8be…`. Its
first `BT` is at 99.96 % of the stream, and it states no `gs` and no marked content. The read
figures come from `read_split_price`'s `built` line: three arms interleaved in one process, three
rounds an arm, two passes, load 1.9–3.5. The binaries are this round's export built from
`3fc3e7f1` and HEAD's export, each in its own target directory.

| figure | before | after |
|---|---|---|
| the read, 24 threads (whole 1 769–1 846 ms) | divided after the inflate 533–575 ms | **pipelined 386–443 ms** |
| the read, 12 threads (whole 1 802–1 893 ms) | 547–587 ms | **433–480 ms** |
| `quorra` under Xvfb: read ends, first frame | 0.743–0.772 s, 1.846–1.860 s | **0.463–0.471 s, 1.262–1.287 s** |
| `quorra` on the owner's display: read ends, first frame | 0.742–0.772 s, 1.918–1.971 s (processor) | **0.473–0.497 s, 0.980–1.043 s (device chosen)** |

Every list equals the page's. `callgrind_interpret --digest` holds
`07eb7445…68e3`, with 4 408 405 commands and 776 097 064 bytes charged.
The four-pieces sweep covers 974 documents. Before (ADR 1880, HEAD): 50 first pages divided.
**After: 84 divided, all 84 with the inflate pipelined in, and every one states what the whole
reading states.** 83 of them are drawn while read, byte for byte the page drawn whole. The
reasons the rest read whole are unchanged in kind; inline images and compatibility sections are
still the largest named ones. `launch_path` moved one figure out of its band: PDF20's
`peak_anon_mib` at 19.43 against 19.5. That is the machine state ADR 1902 guards, and nothing in
this change.

The sweep found two defects in the pipeline's first form, and both are fixed here. **A font a
spine loaded first** reported its defect in the scratch interpreter, which was dropped. The spine's
notes now go into the piece's. **A reading given up** was restored with its font memo kept, so on
the page read again a failure was remembered and said nothing. `forget_memos` empties the memos
when a division is given up. The hold's binaries predate both fixes. Neither is on the witness's
path except one merge of notes per piece.

## 4. What it costs, and what is left

- Every segment is lexed twice, once by the plan and once by its piece. That costs about a fifth more
  of the pool's work than a piece alone, which the pool absorbs while the inflate is the critical path.
- A page the pipeline does not settle pays for the pieces it ran before giving up, and then pays ADR
  1880's plan. On the corpus that happens only where the reason in §3's count says.
- Still declining: an inline image, a compatibility section, text before the last piece, a
  section that records an extent open across every cut, and a `gs` that reaches a page-level record.
