# 530 — The window a content stream is read through

**Finding.** Road D is in the shipped code. A page's `/Contents` is pumped into a fixed window
and the interpreter lexes through that, so **Bomb B — 1.85 MB of file inflating to 1.77 GiB —
costs 8.4 MB of peak resident memory where it cost 1032 MB**, and stops at `MAX_OPERATIONS`
rather than at `max_stream_len`, which is exactly the consequence ADR 0362 predicted. Bomb A
falls from 768 MB to 5.6 MB. The owner's 141 MiB witness is interpreted from **193.7 MB** where
it took 381 MB — ADR 0362 predicted "about 193 MB" from `massif`'s four blocks, to the megabyte
— and draws the same page.

**The page is the same page, as bytes.** The readback of all 1023 pages of ISO 32000-2 is
2 730 201 bytes, `sha256 ed074b1c…`, on both arms and equal to what session 500 recorded thirty
rounds ago; the display-list digest of every pdf.js document's page one is identical; and every
gate's own output was captured on both arms and diffed as a sorted set — corpus, oracle, text
extraction, dates, XMP, JPEG 2000, both quorra lanes — with nothing differing but two wall-clock
lines.

**What it costs is +5.74% of the instructions to interpret an ordinary page** (ISO 32000-2 page
101, fifty times: 1 190 383 283 → 1 258 702 834) **and +10.08% on the witness**, none of it in
the lexer, all of it in the reader's own per-token bookkeeping. Three shapes were written and
measured; the ADR has the table and says which was kept.

**Date.** 2026-08-15.
**ADR.** [0365](../adr/0365-the-window-a-content-stream-is-read-through.md).
**Touched.** `crates/pdf-syntax/src/filter.rs` (`Pump`, `Pumped`, `turn` shared with
`inflate_buffer`), `crates/pdf-syntax/src/document.rs` (`stream_source`, `StreamSource`),
`crates/pdf-syntax/src/lib.rs`, `crates/pdf-model/src/content/reader.rs` (new — the window),
`crates/pdf-model/src/content/run.rs` (`run_reader`, `Step`, `Value`, the inline-image
lookahead), `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/page.rs`
(`content_with_report` drains the reader; `ContentIssue::TokenTooLong`),
`crates/pdf-model/src/inline_image.rs` (`InlineImageError::Unbuffered`),
`crates/pdf-model/tests/content_window.rs` (new — five tests),
`doc/conformance/ledger.toml` (§7.4, §7.8.2, §8.9.7), `doc/todo/14`, `doc/todo/10` §5,
`doc/adr/0365-*` (new), this file.

## The four numbers, and none of them invented

`WINDOW` 64 KiB and `CEILING` 1 MiB come from ADR 0362's census — 225 775 555 tokens in 39 976
documents, largest 390.16 KiB, none past 1 MiB. `SLACK` 4 KiB is where the fast path ends rather
than a bound. `LOOKAHEAD` 16 MiB is 1.78 times the largest inline image of the 93 930 measured,
against a clause that recommends 4096 bytes twice over and requires nothing.

## The two things a bounded buffer cannot do, and what it says instead

A token longer than `CEILING` is `ContentIssue::TokenTooLong` and is stepped over to the next
white-space byte; an inline image whose data outruns `LOOKAHEAD` is
`InlineImageError::Unbuffered`, which is a different sentence from `NoTerminator` for ADR
0306's reason — one is about the file and the other about this reader. Both have tests that
fail without them.

## What is left, and it is written in `doc/todo/14` rather than claimed away

The four *other* content streams §7.8.2 names — forms, patterns, Type 3 glyph procedures,
appearance streams — are still decoded whole, on the argument that §11.6.6's paired runs read
the same form two and three times over and the memo is what makes that cheap. So a bomb hidden
in a form XObject still costs its gibibyte, exactly as it did before this round. And the pump is
`FlateDecode`'s alone: LZW, ASCII85, ASCIIHex and RunLength come back whole, because a second
implementation of each beside the existing one is trap 6 and the honest fix is to make the
existing one resumable.

## Gates

`fmt` silent. `clippy --workspace --all-targets` silent of lints (the `viewer-qt` `cargo:warning=`
lines are gcc's on a cold build, `doc/todo/02` §2). `nextest --workspace` **1947 tests run: 1947
passed, 15 skipped** — the five new ones are `tests/content_window.rs`, and the two that pin the
loud answers were confirmed to fail with each report taken out and to pass with it back. Doctests pass. Conformance **98 + 5 passed**, 0
unreviewed. Corpus **974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 64
incomplete, 0 slow**, the three silence counts unmoved. Oracle, text extraction, dates, XMP,
JPEG 2000 and both quorra lanes all pass and all match the base arm line for line.

**The fuzzers, because this changes how the parser is fed under `#![forbid(unsafe_code)]`**:
`lexer` and `object` 50 000 runs each, clean; `page` — the target that reaches
`pdf_model::interpret` — seeded from 898 corpus documents, **50 641 iterations, 0 oom, 0 timeout,
0 crash**, 32 678 edges, and libFuzzer's own log names `content::reader::Window::widen` among the
functions it reached, so the growth path was exercised rather than assumed.

## Two things found on the way

**`doc/todo/14` attributed Table 31's sentence to §7.8.2 for nine sessions**, and the quotation
gate caught it the moment the sentence was quoted in code: "[t]he division between streams may
occur only at the boundaries between lexical tokens" is §7.7.3.3's, in Table 31's `/Contents`
row. Corrected where it was written.

**`ru_maxrss` is still not the instrument** (ADR 0362), and this round is why it matters: the
after arm's figures are 5.6 and 8.4 MB, which is *below* the floor that measurement has.
