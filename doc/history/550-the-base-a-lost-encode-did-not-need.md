# 550 — The base a lost encode did not need, and the word every refusal now says

2026-08-16. A defect round on the project owner's feature, against `tmp/trace3.entwurf.txt` from
their machine — AMD Radeon 890M under RADV, a surface stating 120 Hz. **The third report of the
same sentence in three sessions**, and the third time the cause was a refusal asking for something
it did not need.

## The item

Their trace carries this line twice in twenty-four presents, and each time the frame before it is a
rendering that repacked the glyph atlas (5.595 → 6.085, and 7.572 → 7.575):

```text
no reprojection: the device has no retained encode to replay — the last frame repacked its
glyph atlas, or none has reached it yet
```

That refusal is about **capturing a new base**. `Stale::Settled` already held the last real frame's
pixels as an `Arc<[u8]>` this host owns, so a lost encode should have meant *"reproject from the
base you already have"* — not *"show nothing"*.

## The reading, verified against the code before anything was changed

Three questions, and the answer to the third is one function away from the brief's.

**What does `Settled` hold after a repack? Nothing.** `Stale::settled` — called for every real
frame — constructed a fresh `Settled { page, target, base: None }`. The pixels of the *previous*
rendering were dropped by the frame that replaced them.

**Does the base survive? No, and that is the whole defect.** In the owner's trace it *had* been
captured — `5.354 approximated: … (read back in 5.4 ms …)` — and was destroyed by the 5.595
rendering a quarter of a second later, which is why the 6.085 view change found nothing. The shape
repeats exactly at 7.296 → 7.572 → 7.575.

**Was `plan` refusing for want of a capture?** Not `plan` — the refusal was in `App::approximate`,
which called `capture_base`, got `None`, and returned `false` in silence. But `plan` carried the
same mistake in its run-level form: its first line was `if self.refused`, where `refused` means
"the window will not be read back again on this machine" — so a device that declined **one**
readback switched reprojection off for the whole session, including for pixels this host was
already holding.

So the reading is confirmed, and it generalises: **a base is unusable when there has never been
one, when the page changed, or when the window changed shape. A lost capture is not on that list.**

ADR 0384 section 6 had looked straight at this and written *"it cannot be worked around from
here"*. Every clause of that paragraph is true about **capturing** and a non-sequitur about
**drawing**.

## What was built

All of it inside `crates/viewer-ui/src/bin/pdf-viewer/`, which is a binary. ADR 0385 has the
argument.

1. **The base outlives the frame it was captured from.** `Base` moved from `Settled` to `Stale` and
   gained the page `Arc` it is of and the placement it was drawn at, which makes it a complete
   statement rather than a field whose meaning came from the record it hung off. `Settled` keeps a
   `captured: bool` in its place.
2. **The composition moved onto the base.** `Stale::composed` reads the placement off the base it
   is going to draw, so `Plan::Reproject` carries the *view being asked for* rather than a
   transform composed against a frame record that may no longer be the one the pixels came from.
   ADR 0383's "compose, do not chain" is not weakened by this: it becomes *unrepresentable* to get
   wrong, because there is no method that accepts a transform from outside.
3. **A refusal to capture is not a refusal to draw.** `Stale::refuse` became
   `refuse_captures`, and `plan` asks the honest question — is there a base held, or may one be
   asked for. Four sites in `capture_base` follow: none of them refuses a reprojection any more,
   and the one whose readback *re-encoded* now keeps the pixels it paid for.
4. **Every refusal says which of two kinds it is, and the count reaches the summary.** Every path
   that declines a reprojection was audited against the owner's three words. Five were
   *unnecessary* and all five were the same mistake; the rest print *impossible* or *unwise* by
   name, with the numbers where there are numbers, and `Stale::refusals` counts them by kind for
   the exit summary. There is deliberately no word for *unnecessary*: it is a defect rather than a
   state.
5. **One ordering defect, found by the round's own new tests.** The page check now comes before the
   "did this view move" question. A page turn at an unchanged magnification satisfied the second,
   so it fell out as the one answer that says nothing — harmless while every refusal was silent,
   and not harmless once silence means "not a view change".

## What was measured

`Xvfb :77` at 900×1100, llvmpipe, release binaries of this tree at `b5505453` and after, driven by
`xdotool`: twelve `+` at 1500 ms, a burst of six at 30 ms, then `Escape` — the summary is printed
only by a clean exit, and a `SIGTERM` skips `exiting`, which cost a run to find out. Witness
`tmp/Entwurf.pdf`, the owner's own, not in the repository and named in no test;
`doc/PDF20_AN001-BPC.pdf` renders inside a refresh on this adapter and never reaches the case.

**Two runs of each binary, and only one number moved — which is the honest report.** The
reprojection count is dominated by run-to-run variance here (14, 17 before; 17, 17 after), so
there is no A/B table of the shape ADR 0384 could write, and inventing one would be worse than
saying so. What is deterministic is the frame this round is about:

| | at ≈38.67 s of the same scripted sequence | outcome |
|---|---|---|
| before, run 1 | `no reprojection: the device has no retained encode to replay` | **nothing moved** |
| before, run 2 | the same line, 38.674 | **nothing moved** |
| after, run 1 | `…the base already held stands in, composed against the frame that produced it` | **drawn in 4.1 ms** against a frame expected to take 96.5 |
| after, run 2 | the same, 38.676 | **drawn in 4.7 ms** against a frame expected to take 120.3 |

The same view change, four times, at the same point of the same sequence: refused twice, drawn
twice, at a twentieth of the cost of the frame it stood in for.

The refusal vocabulary reads as intended in a live trace — `no reprojection (unwise): this frame is
expected to take 13.9 ms against a 16.7 ms refresh, so it lands inside one and is itself the frame
every refresh that was asked for` — and the summary closes with `3 view change(s) showed the real
frame instead: 1 had nothing true to move and 2 were a judgement between two measurements`.

**What this harness cannot say**, stated plainly because a harness is what hid this feature's
defect twice already: llvmpipe's costs are unlike a real device's (the frame and the readback are
both processor work here and move together, which is exactly why the counts are noise); `Xvfb`
states no refresh rate, so every run takes the 60 Hz floor where the owner's surface states 120 and
therefore meets this case twice as often; and the only reason the harness reaches the case at all
is that the atlas repack is quorra's rather than the adapter's, which is luck rather than design.

## Gates

Every one run after the last code edit, all green, and nothing on a judged path moved. `fmt` clean;
`clippy --workspace --all-targets` silent of Rust lints (the `viewer-qt` `-Wmaybe-uninitialized`
lines are gcc's about generated code, as `doc/todo/02` says); **2040 tests run, 2040 passed, 15
skipped** — four more than the 2036 this round started from, all four in `stale.rs`; doctests pass;
sandbox and `pdfref-hayro` binaries built; corpus gate **974 documents, 0 unopenable, 8 locked, 2
encrypted beyond us, 6 pageless, 64 incomplete, 0 slow**; oracle **1794 pages, 906 agree, 67
contradicted, 786 ambiguous**; both text gates pass (**98.26%**, 10 969 of 11 163 matched words in
bounds, 486 of 508 documents fully in bounds); dates, XMP, JPEG 2000 and conformance pass;
`render-quorra`'s default lane **956 pages: 931 agree, 23 differ, 2 refused, 18 not comparable**
and its gpu lane at scale 4 **951 pages: 937 agree, 10 differ, 4 refused, 23 not comparable** —
both matching ADR 0384's record figure for figure, which is `doc/todo/37` rule 2's own gate.

Two gates caught this round's own mistakes, and both are worth recording because neither was in the
code's behaviour. `every_citation_names_a_clause_that_exists` rejected `ADR 0384 §6` in a doc
comment: a `§` in this tree means ISO 32000-2, and an ADR's section is written out. And
`a_page_turn_is_never_reprojected` caught the ordering defect above on the first run of the new
tests, which is the one behavioural error this round made.
