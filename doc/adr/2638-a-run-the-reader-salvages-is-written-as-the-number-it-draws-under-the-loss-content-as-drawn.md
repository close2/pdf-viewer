# 2638 — A run the reader salvages is written as the number it draws, under the loss `content-as-drawn`

Status: implemented and measured (Consequences: `509284` and the census).
Session 1902, the second archive slot of batch 131. Builds on ADR 1004 (the reader's salvage),
ADR 2619 (the checker reads the run as the operator it is), ADR 2516 and ADR 2618
(the photograph of a page whose own stream is at fault) and ADR 1176 (the one content-stream repair
written before this one). Context: `crates/pdf-transform/src/archive/as_drawn.rs`.

## Context

ISO 32000-2 §7.2.3: "A sequence of consecutive regular characters comprises a single token." §7.3.3
spells a real with one period, so `33600.272.58` is one token and no number, and §7.8.2 makes it an
operator; ADR 2619 has the checker read it so, and ISO 19005-2 and 19005-4 section 6.2.2 then fail
it. The window draws something else: ADR 1004 keeps the number the lexer reads off the front of such
a run wherever the tail names no operator of §8.2 Table 50, so `33600.272.58` draws as `33600.272`
and `454` followed by the byte 85h as `454`. At HEAD `509284` converted under `as-if-printed` with
pages 2, 4, 9, 14 and 18 photographed for the operator row, and `as-shown` refused it (exit 4).

## Decision

1. **The remedy.** Where every operator the survey found that the base standard does not define is a
   run this reader draws as a number (`as_drawn::is_drawn_as_a_number`, the reader's own rule,
   `draws_the_salvage`), each content stream holding one is written with that number in place of the
   run, in §7.3.3's spelling (`write::object`, which never uses the exponential format §7.3.3
   forbids a writer). Nothing around the run changes: no operator is added, moved or removed. The
   operator row's answer is `Mechanical(RunWrittenAsDrawn)` under the loss `content-as-drawn`
   (`decision/content_stream_fences.rs`); not granted, or a document whose unlisted operators are not
   all runs or hold none, the row keeps the fence's sentence as before.
2. **The loss, and its word.** RFC 0014's table names none, so the word is `content-as-drawn`: the
   characters after the number are the producer's and are gone, and the salvage is ADR 1004's choice
   rather than a value the standard states. That is the difference from ADR 1176's hexadecimal digit,
   which §7.3.4.3 supplies and which is written losing nothing. `as-if-printed`, `as-shown` and
   `state-every-loss` take it at the operator site; the report names each run by its pages, its
   stream, its bytes and its number, and `xmpMM:History` the first sixteen and the count.
3. **The near side of the authoring line** (`doc/scope-ratifications.md`). The question the line asks
   is whether the operation invents marks. It does not: the marks are the ones this window already
   draws under ADR 1004, from the producer's operators. The photograph amendment (A570) and its
   widening to a page whose stream is at fault (A694 item 1, ADR 2516, ADR 2618) already let a
   conversion fix the window's drawing of such a page into the file; this fixes the same drawing in
   fewer bytes, as the producer's own operators, and leaves the text selectable. It is no new
   amendment and needed no question: it takes less than what was ratified.
4. **A cut stream.** `Document::decoded_stream_data` returns the decoded front of a FlateDecode stream
   that stops short, the bytes every reader decoded, so the stream written is that front re-encoded
   with the number in place: the damage after the cut goes with it, and nothing past it was drawn.
5. **The statement page.** `state-every-loss`'s `append-statement` at the operator site lists one
   entry per run (`statement::runs_entry`): the requirement and its clause, the loss and its line,
   the pages, the content stream's object number, the run as the producer wrote it and the number
   written, in the closed vocabulary; a byte outside printable ASCII is written `\xHH`, the report's
   spelling. A page that cannot be composed hands the runs to `discard`, as the halftone and font
   rows' pages do (`config/steps.rs`, `STATED_RUNS`).
6. **The bound.** A run is at least two bytes, so one stream yields at most half its length in runs,
   and one conversion writes at most 65536 (`MOST_RUNS`, checked as each run is found and before its
   record is kept); past it the row keeps its refusal. `getrusage` beside HEAD on one page of 10^6
   runs `1.2.3`: both refuse, 27.7 MiB against HEAD's 28.8 MiB; at the bound, 65536 runs, this tree
   converts in 0.11 s at 41.2 MiB where HEAD refuses at 17.8 MiB. Before the bound the same 10^6 runs
   converted at 366.9 MiB, which is why it is there.
7. **Two rewrites of one stream are not composed**: a stream ADR 1176's repair or `languages.rs`
   also rewrites keeps its runs, refused by name.

## Consequences

- `509284` under `as-if-printed` converts at 2b and at 4 with page 2 alone photographed and 58 runs
  written, pages 4, 9, 14 and 18 kept as text; veraPDF passes both outputs. Page 2's one unlisted
  operator is a `)` where no string began, a delimiter rather than a run, so it stays photographed:
  the slot's finish line of five pages as text is four of five. Under `as-shown` the `)` keeps the
  fence's refusal (exit 4), as at HEAD.
- The plant: the 2b output round-tripped through `qpdf` with `33600.272` put back as `33600.272.58`
  fails our checker on the operator row (exit 4) and the unplanted round-trip passes (exit 0).
  veraPDF does not flag the planted run, lexing it as no operator at all: a disagreement with
  §7.2.3 for the veraPDF census, not a target.
- The census against HEAD's: every total line unchanged. The lines that move are `as-shown`'s
  refusal reasons: `064034` leaves the fence for the sentence of a stream that also holds a
  hexadecimal string with an odd number of digits, where neither rewrite is taken; and the files
  whose one by-name refusal was the operator row leave the TrueType rows' "refused by name" groups,
  still refused by the fence. That second move is a cost this decision leaves open: the operator row
  answering with the fence no longer stops a refused file's TrueType programs being copied.
- Tests: `crates/pdf-transform/tests/archive/as_drawn.rs` (a run written and its page kept as text at
  2b and 4; a stray `)` beside a run keeps the page photographed and the run written; a cut stream
  ending in `454` and 85h; `as-shown` keeps the fence for `Frobnicate`; the statement names each run;
  the bound at and one past).
