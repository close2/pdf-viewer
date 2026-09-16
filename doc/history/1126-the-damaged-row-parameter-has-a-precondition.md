# 1126 — /DamagedRowsBeforeError has a precondition, and the refusal narrows to it

2026-09-16. §7.4.6, Table 11. Files: `crates/pdf-model/src/image.rs` (the refusal + its doc),
`crates/pdf-model/tests/ccitt_bound.rs` (a calibration pair), `crates/pdf-sandbox/src/protocol.rs`
(one doc comment), `doc/conformance/ledger.toml` (the 7.4.6 note). No ADR: no substitution was
built, so ADR 1129's trigger did not fire. Every other path in `git status` is a sibling's.

## The reading

Table 11's `/DamagedRowsBeforeError` row: "[t]his entry shall apply only if EndOfLine is true and
K is non-negative." The tree refused **any** positive value, dropping that first sentence. The
concealment resynchronises by "searching for an EndOfLine pattern", so the entry is inert where the
encoding carries none (`/EndOfLine false`) or is Group 4 (`/K` negative); a positive value there
must decode as though the entry were absent, and refusing it threw away a picture over a parameter
the standard says has no effect on it. The refusal now fires only when the entry applies
(`damaged_rows > 0 && end_of_line && k >= 0`).

**The applicable case stays refused, keeping the row `partial`.** `hayro-ccitt` (the pinned,
JBIG2-shared decoder) exposes neither a failure's bit position nor a resume — context and reader
private — so §7.4.6's EndOfLine-search-and-previous-row-substitution is a change to the shared
decoder, not to this filter. A loud refusal beats the ordinary truncated draw, which would drop a
request for concealment silently. Not built (contract's "if the decoder reaches" — it does not).
**The contract's Group 4 fixture recipe is spec-wrong** and unused: K<0 is where the entry does not
apply. Principle 5 — the spec governs the brief.

## Census (trap 8), calibration (trap 13), gates

Parser walk over 1450 opened documents of five on-disk corpora: 47 state a CCITT image, 1048 CCITT
images, **0** with `/DamagedRowsBeforeError > 0` — not even the inapplicable case. SafeDocs
(malformed faxes) is not checked out here. Unwitnessed by the world we have.

A `ccitt_bound.rs` pair differs only in `/EndOfLine`, same `/DamagedRowsBeforeError 2`:
`damaged_rows_is_inert_where_end_of_line_is_false` draws all four lines silently;
`damaged_rows_applies_and_is_refused_where_end_of_line_is_true` is refused, naming the entry. Both
were refused before — the control that moves. Rebuilt the worker (trap 10) after the protocol.rs
comment; CCITT decodes came back blank until then.

Gates, all exit 0: `pdf-sandbox --bins`; `ccitt` 2/2; `ccitt_bound` 9/9; `pdf-model --test corpus`
(draw-incompletely 61/61, slack 0, unchanged); `raster_golden` (held 974, moved 0);
`viewer-confined --test awkward_classes` (killed 0). fmt + clippy pedantic clean on the four files.
