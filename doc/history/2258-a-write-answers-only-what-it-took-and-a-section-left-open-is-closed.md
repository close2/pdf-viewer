# 2258 — A write answers only what it took, a section left open is closed, and a move is composed

Lane 3, the PDF/A converter's remaining content writes (rounds 2240, 2244, 2249). ADRs 3733–3736.

- **ADR 3733, the trap first**: ADR 3558's fence answered every listed class at a page or form once
  one operator was written out of its streams (`wrote_out_of(holder)`), so a write that took one
  class answered the ones it left. The proof's second file is now read for what it still needs
  (`still_repaired`, holder and class), and a class is answered at a holder only where it is gone:
  `as-shown` refuses by that class, `as-if-printed` keeps the photograph. A two-stream `/Contents`
  fixture (a `Q` whose operand stands in the first stream) fails without it.
- **ADR 3734**: a section left open where a page's last stream, or a form's own, ends is closed by
  `\nEMC` — not inside a producer's open text object, after operands nothing took, or past
  `MOST_SECTIONS`. `050734` stays photographed: its section is open inside an unterminated string.
- **ADR 3735**: `inline_image::Scan::unread_data` names data past the end §8.9.3's arithmetic or the
  first filter's EOD states, up to the `EI` the search found (with `/L` absent or naming that end);
  the write cuts it. A hexadecimal string's byte that is neither digit nor white space is written
  out; where every one is, section 6.1.6's `hexadecimal-string-holds-only-digits` is answered at
  `content-as-drawn` (`fenced::STRAY_BYTES`, `stray_bytes_written`). Both under the proof.
- **ADR 3736**: `Td`/`TD` from a matrix the walk states is composed onto it in single precision
  (`StatedMatrix`, `Moved`); `T*`, `'`, `"` still leave it unstated. ADR 3661's left case converts.
- **ADR 3698's guard** has a fixture: an `/LZWDecode` form refused by `/EarlyChange 7` logs
  `stream-data-undecodable`, and the page's `ETBT` is left (fails with the guard removed).
- **Census**, closing against opening on this round's base (pack): `as-if-printed` does not move;
  `as-shown` moves no count, and only the refusal sentence of `367594` and `392154` (both parts)
  changes, from `string-unterminated` to `operator-short-of-operands`, the class ADR 3733 now leaves
  unanswered there. Without the pack, the same four lines and nothing else. veraPDF 1.30.3:
  opening and closing alike, 5523 outputs, 5471 compliant, 40 not, 12 unfinished, the same rules.
- Left: the carriers of ADR 3680 (`poppler-90-0-fuzzed`, `064034`) hold other classes and do not
  move. A probe form painted in DeviceGray and then Lab (`q /Fm0 Do Q /CS0 cs /Fm0 Do`) failed the
  proof after the write, where ADR 3697 judges such a stream in neither space; not followed here.

**Gates.** Over `-p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt -p conformance`: fmt 0;
clippy `-D warnings --all-targets` 0 (a first run's 101 — six one-letter bindings, `decide` past
100 lines — fixed before it); sandbox `--bins` 0; nextest `--lib --bins --tests` (`xvfb-run -a`)
100, 5377 of 5380, `audit-diff.py` "failed 3, expected 3" (the `pdf-model` 10.5 audit tests the
manifest lists); rustdoc `pdf-model`, `pdf-transform` 0; fuzz 0; `cargo test -p conformance` 0.
