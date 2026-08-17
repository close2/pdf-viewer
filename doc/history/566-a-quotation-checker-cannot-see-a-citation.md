# 566 — A quotation checker cannot see a citation, and two of them came from another edition

**Finding.** quorra's developers reviewed `doc/HAYRO_ISSUES_FOR_QUORRA.md` — the hand-over document
this tree wrote for them — and sent back four clause corrections. All four were read here against
the sponsored EC3 text before being accepted, on principle 5's rule that their reading is evidence
about ours and never the definition of correct, and **all four are right**. Two were misreadings on
this side (§8.9.6.4 for a mask on a grid of its own, where §8.9.6.3 *Explicit masking* is the clause
that permits one; §8.4.3.3 reasoned into a dot under round or square caps, where §8.5.3.2's last
sentence says no output with no cap condition). Two were a wrong clause number over a reading that
was right (§8.7.4.3 for a shading's coordinate space, which is §8.7.2's rule and §8.7.4.1's sentence
about `f`, `S` and `Tj` — and §8.7.4.3's NOTE 2 is a *note*, so it could not have been the normative
source of anything, which is a second count they did not name; and `/Interpolate`, quoted as "shall …
conforming reader" where EC3 says "should … PDF processor" and §8.9.5.3 adds "this is only a hint").

**A fifth was found while checking theirs**, in a part of the document they did not flag: §2's
§10.7.4 blockquote is not in ISO 32000-2 either. Two of five is one shape — **a sentence quoted from
ISO 32000-1 and attributed to ISO 32000-2** — and §0.3 of this standard is the two-second test both
fail: "[s]tarting with ISO 32000-2:2017 (PDF 2.0) the term 'conforming reader' is no longer used".
`pdftotext` over the whole EC3 finds the phrase three times, all three of them saying that.

**Second finding, and the one worth the round: the tree was clean, and the instrument that reads it
has three blind spots.** Every one of the four clause numbers was grepped across `crates/`, the
ledger and every other document, and in all four places the code has the right clause and the right
words — `pdf-model`'s `MaskEntry` splits §8.9.6.3 from §8.9.6.4 at the line Table 87 draws, the
ledger's §8.5.3.2 row has said "no output under any cap" since the twenty-fourth session, every
§8.7.4.3 citation in `crates/` is Table 77's, and `pdf-render`'s `paint.rs` carries the EC3
`/Interpolate` wording verbatim. So the error was the hand-over document's alone — and
`tools/conformance/quotations` reads that document. It had been **reporting the `/Interpolate`
misquotation as "matched 9 of 16 words, then diverged" since the day the document was written, and
nobody had read the output**; removing it takes the sweep's divergence count from 26 to 25, which is
how that was confirmed. The other four are invisible to it, and the reason is the sharp half: the
sweep reports a quotation that matches for at least five words and *then* diverges, so it is most
sensitive where the error is smallest. A wholly foreign sentence lands in the bucket it calls
"sharing too little with any of them to be a quotation of one" and is counted rather than printed;
a **wrong clause number over a correct quotation** it cannot see at all, because it checks what the
words are and never what they are attributed to. The four corrections are one instance of each of
the three populations.

**Third: §8.5.3.2's answer rests on one word, and neither side had said so.** The sentence is "A
single-point open subpath (specified by a trailing m operator) shall produce no output", and the
mark in question — a glyph outline beginning `MoveTo(0,0)` before its real one — is *leading*. Read
the parenthesis as a restriction and a non-trailing single-point open subpath falls under no sentence
in the clause (its *degenerate* is defined as a single-point **closed** path or two or more coincident
points), so §8.4.3.3's general cap rule returns and the dot comes back. Read as a gloss, the clause is
complete. The gloss reading is taken, on the sentence two above it — "This rule shall apply only to
zero-length subpaths of the path being stroked" — which classifies by shape and not by position, and
on §8.5.3.3.1's parallel sentence, where the position *is* carried by normative words outside the
parenthesis. Recorded in the ledger's §8.5.3.2 row, because it is one word away from being the
opposite fact.

**Fourth: their question, measured.** *Does `issue1905.pdf` refuse in the product, or only in the
gate?* **Only in the gate**, and `crates/render-quorra/examples/viewport_refusal.rs` is the
instrument — the gate's whole-page target and the product's window target drawn in the same run, on
the real Radeon 890M, headless, on the lane `viewer-ui` uses at that magnification. At 4× the page
is 4988 × 7936 and the whole-page frame is refused for the 16384 × 16384 coverage sheet; the same
display list at the same magnification into a 1600 × 1000 window draws at every scroll position,
with 38.6 %, 54.5 % and 54.7 % of the frame marked. At 64× — `viewer-core`'s `ZOOM_RANGE` maximum,
so the most a person can ask for — the window frame still draws. `bug1703683_page2_reduced.pdf`
behaves identically, and the *control* is the other two of `REFUSED_AT_FOUR`: `bug1721218_reduced.pdf`
and `issue18032.pdf` refuse the whole-page target and all three window frames alike, because they
refuse before the scene is built. So that list is two kinds of refusal wearing one name, and only
one kind is a property of the gate's target. The marked share is counted rather than assumed, because
trap 12b is a device that returns `Ok(())` over a blank target and "the device took the frame" is not
the same claim as "the page is on it".

**Date.** 2026-08-18.
**ADR.** [0401](../adr/0401-a-quotation-checker-cannot-see-a-citation.md).
**Touched.** `doc/HAYRO_ISSUES_FOR_QUORRA.md` (five corrections in place, each marked with the
standard's own sentence), `doc/QUORRA_HAYRO_MAP_ANSWER.md` (new, the reply),
`doc/conformance/ledger.toml` (§8.5.3.2's note),
`crates/render-quorra/examples/viewport_refusal.rs` (new), `doc/adr/0401`, this file. No code
changed: there was nothing in `crates/` to correct.
