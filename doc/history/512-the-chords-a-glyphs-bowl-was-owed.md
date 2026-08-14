# 512 — The chords a glyph's bowl was owed, and the reuse that came back as a question

**Finding.** A quorra-release round: `a7babab` → `87898c69`, twenty-five commits, not one line
of source changed here — and seventeen pages left the differing list at scale 1 with none
arriving at any scale on either lane. Fifteen are upstream's chord floor (their ADR 0044: a
cubic's flattening bound is the tighter of a quarter pixel and 1/32 of the cubic's own device
extent, on §10.7.2 NOTE 2's "not to draw inscribed polygons" — the population such a floor
reaches is glyph bowls, so what moved is prose, the whole `tracemonkey` family included); two
are the round cap (`d594566`: the near cap was the *inward* half-disc wound against its own
body, a hole summed ink cannot see). Both §21 defect reports are answered, re-measured here
(round cap −8.9% → −0.1% of Table 53's area; the one-pixel dot −36.1% → −2.1%), and
`sub_pixel_coverage.rs`'s round-cap and dot rows now gate **both** backends — the rows held
against the processor only since session 455, flipped on their own stated condition.
`REFUSED_AT_FOUR` gains `issue18032.pdf` (4 → 5): not the release but a stale baseline —
session 492's §11.4.6 refusal holds at every scale and no round since had run the 4× lane.
`doc/todo/44` §3's two upstream asks came back **priced and unbuilt** (their ADR 0045: an
identical frame replayed is 0.154 ms against 1.538 re-encoded; zoom reuse impossible at any
price since the transform is inside every atlas key), with one question asked back — can the
page and the overlays be two `render` calls into one target? — which this round answered *no*
with the reason `present.rs` names (the selection overlay's `Multiply` fills must see the page
beneath them), recorded in todo/44 §3.1 as the specification fragment composition is to be
designed from. Entwurf frame structure unchanged between pins (encode ~90% of `device` median
on both, A/B/A/B alternated, no wall-clock claims).

**Date.** 2026-08-14.
**ADR.** [0347](../adr/0347-the-release-that-moved-seventeen-pages-without-a-line.md).

**Gates, all run on this tree.** fmt clean; clippy `--workspace --all-targets` silent (the
documented cold-build gcc warnings from `viewer-qt`'s cxx bridge only); nextest workspace
**1848 passed, 15 skipped**; doctests 1 passed; corpus 974 documents, **62 incomplete**
(unchanged); oracle 1794 pages — agrees 905, contradicted 68, ambiguous 786, gates ok; text
extraction 98.26% (10967/11161 words, 485/507 documents) and the PDFBox half green; dates, xmp,
jpeg2000, conformance (5/5) all ok. Quorra corpus, all four lanes at `87898c69`:

| | agree | differ | refused | not comparable |
|---|---:|---:|---:|---:|
| scale 1, `cpu` | 934 | 20 | 2 | 18 |
| scale 1, `gpu` | 933 | 21 | 2 | 18 |
| scale 4, `cpu` | 936 | 10 | 5 | 23 |
| scale 4, `gpu` | 937 | 9 | 5 | 23 |

Both lanes refuse the same five pages at 4×, continuing §22.2's finding. The GPU lane's scale-1
survey swaps four envelope pages in (`bug1863910`, `bug1883609`, `issue16500`, `vertical`) for
three that agree on it (`bug1743245`, `bug1844583`, `issue21068`), all within the lane's stated
32-of-255 envelope.

**Touched.** `Cargo.lock` (the two quorra hashes), `crates/render-quorra/tests/corpus.rs`
(`DIFFERS_AT_THE_EDGES` 24 → 7, `REFUSED_AT_FOUR` 4 → 5, both with the argument in the doc
comment), `crates/render-quorra/tests/sub_pixel_coverage.rs` (the two rows flipped to both
backends; `the_processor_agrees_with_the_area` deleted with its stale bullets),
`doc/QUORRA_UPGRADE.md` (preamble corrected, `87898c69` section appended),
`doc/QUORRA_FEEDBACK.md` (§21 answered with §21.4; §9.2's dated retraction note),
`doc/todo/44` (§3.1: the priced answer, the withdrawn zoom sentence, the question and this
tree's answer), `doc/todo/45` (§3's row points at todo/44 §3.1), ADR 0347, this file.

**For the next round.** The encode-reuse next step is a *conversation*, not code: carry todo/44
§3.1's reason to upstream so fragment composition (or a root pass over stated content) is
designed from it — a scene cache built here first would save the 50 ms `scene` phase and leave
the 234 ms `encode` untouched. Upstream also left two named seams worth watching in their next
release: a blended stroke inside a knockout group wraps under a weaker condition than fills and
images (their closing commit calls it their one open correctness question), and their solid-fill
lane bypasses the `rect_hint` recogniser they already have (most of §19's saving, four lines,
theirs).
