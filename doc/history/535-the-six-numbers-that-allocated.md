# 535 — The six numbers that allocated, and the byte nobody had tabulated

2026-08-15. One round on the launch side of `doc/todo/44`: retake the attribution of the owner's
document, which ADR 0332 took before ADR 0341 halved the lexer and ADR 0365 changed the window
the stream arrives through, and take what the new one names. ADR 0370 is the decision, the
tables and the argument; this is the record.

## What the round did

- **Retook the attribution** with callgrind under `RAYON_NUM_THREADS=1`, and separated the two
  reasons the lexer's share could have moved: `Lexer::next_token` is 5 520 M against ADR 0341's
  5 516 M, so it did not move at all and the *share* fell only because ADR 0365's window added
  cost elsewhere. That cost is `run_reader`'s own self instructions, and it is named rather than
  attributed to the lexer.
- **Took three levers**, in the order the attribution named them: §7.3.3's fixed-format parse
  asked before the digit scan; §7.2.3's byte classification as a `const`-built table; and an
  operator's operands read into a fixed-size array instead of two heap `Vec`s.
- **Declined five**, each measured rather than argued: the slice-scan rewrite of
  `read_regular_run` (built and reverted), §7.8.2's operand slicing (measured by removal), the
  `q` clone, the resource lookup and the per-command display-list allocation.
- **Proved the display list byte-identical** — the digest of every corpus document's page one,
  the readback of ISO 32000-2's 1023 pages and the readback of the witness, all three unmoved and
  all three matching the figures earlier rounds recorded. The digest was taken twice and the two
  pairs disagree with each other by 106 documents; the reason is `pdf-sandbox-worker` being on
  disk for the second pair and not the first, and ADR 0370 records it because it is a property of
  the instrument that nothing else in this tree writes down.

## What moved

The witness's interpretation, and by less the ordinary page. The three levers together and the
control's own figure are in ADR 0370's tables; `doc/todo/44` §2 carries the new attribution.

## What the round did not touch

`pdf-render`'s IR and `render-quorra`, which session 533 is assessing from the frame side. The
attribution found nothing on the IR to hand over: the display list's own construction is
0.002% of interpreting this page.
