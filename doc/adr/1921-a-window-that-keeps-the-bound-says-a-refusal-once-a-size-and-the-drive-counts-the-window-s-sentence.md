# 1921 — A window that keeps the bound says a refusal once a size, and the drive counts the window's sentence

Session 1542, slot 3 of batch eighty-six. Status: **accepted**. Context: ADR 1892 §2.1 (the
confined worker keeps `viewer_core::MAX_PIXELS`, and `Command::Oversize` is `Uncarried` to it) and
§2.4 (a refusal is said once for the view it is about). Code: `tools/drive-windows.sh`'s
`zoom_past`; nothing in the program changed. It cites no clause and moves no ledger row.

## 1. What was found

At HEAD `70eda087`, drive step 82 read `wrong` for `quorra-confined`: "the refusal was said 10
times over eighteen notches and twenty pointer moves". The brief's hypothesis was that the
confined worker refuses at every frame because nothing remembers the refused view. **That did not
hold.** The window's log for the step (`scratchpad/r1542/z82-head/`) holds:

- **five refusals, one at each of the five notches past the bound**: targets of 365 217 756,
  570 628 494, 891 610 688, 1 393 110 330 and 2 176 736 544 pixels against 268 435 456;
- **each one counted twice**: once as the window's own `page 1: this page cannot be drawn …` and
  once as the events trace's `<- Reported { … }` line, which quotes the same note;
- **nothing at the twenty pointer moves**: the confined window sends the worker no command for a
  pointer that only moves, and no refusal follows one.

So the confined window already follows ADR 1892 §2.4: once for each magnification and
interpretation. The drive's own sentence ("every window … may say a refusal at most once") was
written by the round that wrote §2.4, and its confined run never finished (round 1528 ran out of
its tree ceiling). It asked for something the ADR rules out for a window that keeps the bound, and
it read its count off a log that says each refusal twice.

## 2. Decision

1. **The rule stays ADR 1892 §2.4's.** A window that keeps the bound says the refusal once at each
   new size, and not again while the size is the same. Each notch past the bound is a new question
   with a new answer, which the sentence's figure shows. The owner's report was about a refusal
   said at every frame with nothing changed, and neither of those happens here.
2. **The drive counts the window's sentence and not the trace's echo.** Step 82 is `wrong` where
   one size is refused twice or where any refusal follows the pointer moves. A toolkit window must
   still say none. The count was calibrated by planting both defects in HEAD's log: a refusal
   repeated before the pointer moves reads `wrong 6/5/0`, and one after them reads `wrong`.
   HEAD's log itself reads `works 5/5/0`.

## 3. What this does not do

The confined window still refuses a page past the bound. Drawing such a page as the window shows
it is ADR 1892's build for the toolkit windows, and it is owed here too. The worker would answer
the request with the page's marks (ADR 0640's `Rendered::Listed`). The device would draw them at
window size. And the processor's fallback (`--cpu`, a device refusal) would need
`viewer_host::oversize`'s windows, so that no whole raster past the bound is allocated in the
unconfined process. That build changes what the confined window draws, and it needs its own round.
