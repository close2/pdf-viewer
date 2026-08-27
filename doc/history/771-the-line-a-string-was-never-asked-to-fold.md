# 771 — The line a string was never asked to fold

The errata selection rule's eighth use, and the first time its two rankings tied at the head. The
tie-break read a settled clause-13 row to a verdict that confirmed it, and the next settled row down
paid: an erratum makes the literal string one of the two forms every byte string may be written in,
and §7.3.4.2's end-of-line rule — a `shall` over the *bytes* a string holds — turned out to be
unimplemented under an `implemented` row whose note enumerated everything else the reader took.

Date: 2026-08-28.
ADR: [0708](../adr/0708-the-line-a-string-was-never-asked-to-fold.md) — the briefing assigned this
round numbers an owner-merged arc had consumed by the time the round resumed, so the ADR took the
first number free of `main`'s tree.

Touched: `crates/pdf-syntax/src/lexer.rs` (the fix and one new test), `doc/conformance/ledger.toml`
(§7.3.4.2, §7.9.2.4, §7.9.2.2.1, §12.6.4, §12.6.4.16), `doc/errata-read.md`, `doc/todo/01`, the ADR
and this file. **This change can move a pixel**: one lexer serves the file body and the content
streams, so a literal string's byte is a glyph code, and §2 ran whole.

## What the rule gave

302 issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret; **115 were named
nowhere** at this round's base. Over live rows the head is **§7.6.4.1 and §7.6.6 with six
annotations apiece** — unmoved from the seventh use, no round having taken either. Over **every**
row the head is a three-way tie at six between those two and **§12.6.4.17, `out-of-scope`**, which
step 4's "preferring the settled row where they tie" settles. The population is 110 after this
round: five issues gained a verdict.

## What the issues said

`doc/errata-read.md` has all five with the rectangle that places each.

- **#265 and #282**, six annotations on page 541, and they are **§12.6.4.16's, filed under
  §12.6.4.17**: `emit` attributes by the outline section for the page, and page 541 opens
  §12.6.4.16 and reaches §12.6.4.17 before it ends. #265 strikes `transition` from Table 220's `/S`
  row — the published row names §12.6.4.15's action type for the type it defines — and #282 widens
  the action to a RichMedia annotation with `/V` reaching a `/RichMediaContent` dictionary's
  `Views` array. Both are inside `CLAUDE.md`'s multimedia exclusion end to end, and `action.rs`
  refuses the keyword `GoTo3DView` whatever the target is. The row is confirmed, not moved.
- **#276**, three annotations on page 132. Strikes §7.9.2.4's old file-identifier paragraph and
  inserts: unless otherwise stated, **a byte string may be either a literal string (§7.3.4.2) or a
  hexadecimal string (§7.3.4.3)** — with the identifier demoted to an EXAMPLE, a NOTE that a
  signature dictionary's `/Contents` can be required to be hexadecimal, and the next NOTE
  renumbered. This is the annotation that moved the round.
- **#161**, two annotations. §7.9.2.2.1's NOTE 4 named `dieresis` where the UTF-8 marker's EFh is
  `idieresis` — Table D.2 settles it, 357 octal. Informative; nothing decodes from a NOTE.
- **#96**, one strike. Deletes §7.9.2.2.1's NOTE 5 (the UTF-16BE/UCS2 warning) outright — and
  §7.9.2.2.1's row cited that NOTE as what the surrogate-pairing decoder "is there to warn
  against". The citation retired with the NOTE; the reason did not — the normative sentence about
  supplementary characters is why the decoder pairs surrogates, and the row now says so.

## What reading them made this round look at

**§7.9.2.4 is `implemented` with one hexadecimal-string test, and #276 makes its claim cover both
written forms.** So the question went to §7.3.4.2, whose row enumerates what the reading side takes
— "all eight escape sequences, the line continuation, and the rule that an unbalanced parenthesis
is a lexical error" — and the clause's end-of-line rule is in neither the list nor the code:

> An end-of-line marker appearing within a literal string without a preceding REVERSE SOLIDUS shall
> be treated as a byte value of (0Ah), irrespective of whether the end-of-line marker was a CARRIAGE
> RETURN (0Dh), a LINE FEED (0Ah), or both.

`Lexer::read_literal_string` let an unescaped CARRIAGE RETURN through as 0Dh, and a CARRIAGE RETURN
with a LINE FEED behind it through as two bytes where the clause states one. Probed against the
unmodified tree before it was believed: a three-assertion probe failed on its first case. It is not
cosmetic — §7.6's algorithms compare `/O`, `/U`, `/OE`, `/UE` and `/Perms` by length and by byte, a
revision-6 `/U` written with an unescaped marker arrived 49 bytes where the file states 48; §14.4's
`/ID` is compared for equality; and inside `Tj` the byte is a glyph code. The fix is one arm,
consuming the LINE FEED that follows a CARRIAGE RETURN so the pair is one byte.
`an_unescaped_end_of_line_in_a_literal_string_is_one_line_feed` asks all four unescaped forms —
including LINE FEED then CARRIAGE RETURN, which is two markers and two bytes — plus the two escaped
controls that must not move. Calibrated per trap 13 twice: each unescaped case fails against the
old code, and the whole test fails against a plant that consumes the pair and writes 0Dh.

**A fourth mechanism for a settled row's evidence being weaker than its claim**: 755 a round trip
that could not fail, 760 a sentence about a sibling row, 765 a set with no closure check, and now a
row asserting two written forms with a test of one. They share only the status.

**And §12.6.4's note named a refused action type the standard does not have.** It called one of the
nine `/ECMAScript`; Table 201's keyword is `JavaScript` — §12.6.4.17 says the term "is retained in
keywords" for backwards compatibility — and `action.rs` matches the keyword and always did. The
ledger named a key the standard does not state; no sweep prints it, because the key was attributed
to no table number.

## What contradicts the briefing, and what the round adds

- **The briefing's ADR numbers were consumed** by the owner-merged GPU arc (`main` now carries ADRs
  through 0707); this round's ADR is 0708, taken in round order ahead of r772's own collision.
- **The outline mis-filing decided a head for the first time.** The four-hundred-and-twenty-ninth
  recorded the coarseness from `check`'s side; here it put six annotations on the wrong row of a
  settled pair. Costless this time — one exclusion covers both — but the practice the todo now
  states is to read the annotation text before the heading.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`;
its load carried one stale artefact worth naming: a cached mupdf-failure message that quotes a
sibling worktree's path (r707), which is trap 10a's shape in the message text only — the verdict it
caches is path-independent.

`fmt`, `clippy -D warnings` (the only output the documented cold-build gcc lines from `viewer-qt`),
`nextest --workspace`, the doctests, the fuzz `check`, both trap-10 workers before any image gate,
corpus, oracle, the three text gates, both censuses, dates, XMP, JPEG 2000, quorra
(933 agree / 22 differ / 2 refused / 17 not comparable), `fixed_documents` and
`cargo test -p conformance` all green. The lexer fuzz target ran 200 000 executions with no
finding. **The oracle holds and no ratchet moved** — no corpus document was found exercising the
unescaped-marker path on a page a gate rasterises, which is the "count that does not move is not
evidence nothing happened" shape: the defect is real, specification-derived, and was proven by
probe rather than by corpus.

**The wall-clock test failed twice and passed alone five times running.**
`viewer-host::a_launch_waits_for_page_one_instead_of_polling_for_it` failed under a full
`nextest --workspace` at a one-minute load of 35 and once more in a first single run, then passed
five consecutive single runs and a full `--no-fail-fast` workspace run (2687/2687). Same
observation 765 left: the budgeted wait loses its core to parallel test processes. Left for a
`viewer-host` round.

Sixteen sweeps before the edits and after them; `quoted` and `unpriced` not run, this round touching
no page-list note. Every delta is the sweep watching the round work: `pointers` +4 paths all live
(the new ADR and test names), `tables` +3 key citations all agreeing, `quotations` +10 verbatim with
**diverging unchanged at 38 and 2**, `counts` sentences +48 with every defect bucket unchanged,
`owed` +2 terms both named (REVERSE, SOLIDUS — this round's own quotation), `overtaken` +1 decision
record with overtaken unchanged at 45, and `spec-errata check`/`applied` +1 and +11 places, every
new hit inside `doc/errata-read.md`, which is the reading itself and counted apart. `overstated`
moved only in row order.
