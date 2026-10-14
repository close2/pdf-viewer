# 1911 — A page turn's last present is the sharp pass, and it is owed

Session 1537, slot 4 of batch eighty-five. Status: **accepted**; nothing is built, and that is the
decision. Context: ADR 1900 §3 item 2 (the figure), ADR 0443 (the retained pages), ADR 0699 (the
sharp pass, built at the owner's request), ADR 1289 (when it may begin), ADR 1853 (a transition's
last face), `doc/todo/37` rule 5. It cites no clause and moves no ledger row.

## 1. The question

Round 1532 measured a page turn on the owner's display with one instrument for every program.
okular settled in 27–28 ms. `quorra` settled in 49–78 ms. The brief guessed that the extra present
was the stand-in shown at the key, for a frame less than one refresh away. Its fallback was the
transition's last face, which ADR 1853 already says is owed.

## 2. What the turn presents

These are round 1532's own files (`tmp/r1532/owner/r1-quorra-ISO32000.{log,txt}`). The key comes
at 3.088 s in the trace. ISO 32000-2 has no `/Trans`, so ADR 1853 is not involved.

| after the key | present | the trace's word | the instrument's share unlike the settled picture |
|---|---|---|---|
| 1 ms, 6 ms | the retained page, twice | `approximated, over a retained page` | 21.7 % at 0.03 s |
| 16 ms | the page | `presented` (drew 7.4 ms) | 0.3 % at 0.05 s |
| 44 ms | the page at 2× | `sharpened: the settled view at 2x, 4.5 ms` | settled at 0.08 s |

**The hypothesis does not hold, on two counts.** The present that settles the figure is the sharp
pass, not the stand-in. And the stand-in is not a present nobody can see. It is the first change
the instrument photographed, 28 ms after the key, which is okular's figure. Without it the first
change would wait for the frame. The frame was not under a refresh either. It landed 16 ms after
the key, two refreshes at 120 Hz, so rule 5 was right that it missed one.

## 3. Why each present is owed

- **The stand-in**, by rule 5: the frame misses a refresh. The frame was predicted at 13.5 ms,
  the cost of the first frame, which built two pages from cold. It drew in 7.4 ms. The repeat at
  6 ms is the next tick of the cadence that rule 1's `MustFollow` arms. It shows the same picture,
  so it changes nothing a person or the instrument sees. Where the frame is predicted inside a
  refresh, rule 5 already shows no stand-in (§4 has one). The brief's remedy is the rule as it is.
- **The sharp pass**, by ADR 0699. It is the picture the owner asked for: the page drawn at twice
  the resolution and shown at half size, with half the seam leak. ADR 1289 starts it only after
  the view has been still for as long as the pass is predicted to take. So it never stands in
  front of a frame. Drawing the turned page at 2× in the first place would remove one present, but
  it would put the pass in front of the real frame. That costs 4.5 ms on this page and up to four
  times a build on a heavy one, and it is the scheduling that ADR 0699's whole argument rules out.

So okular's 27–28 ms is matched by `quorra`'s first change on ISO 32000-2 and AN001, and their
73–78 ms is the owner's sharper picture arriving 28–30 ms after the page. No present is taken out.
WTPDF's 49 ms is not an extra present: its next page had no retained picture, so nothing stood in.
The frame and the sharp pass went up 14 and 23 ms after the key, inside one photograph.

## 4. What was measured this round

**The owner's display could not be photographed.** The clock hold placed round 1532's instrument
through the owner's loop for HEAD, HEAD with the split, and the split with `--supersample 1`. Every
window opened and presented, but none was found. A probe then found the session locked
(`LockedHint=yes`) with no visible window. So the brief's before-and-after photographs are not
taken (§2 holds them from round 1532).

**Under Xvfb**, on lavapipe at 60 Hz with the same instrument, three runs an arm, the claim was
tested by breaking it. AN001's turn settled in 63–77 ms at HEAD and 63–84 ms with the split.
WTPDF's settled in 54–61 ms and 57–102 ms. With `--supersample 1` both settled at their first
change: 29–35 ms and 24–34 ms. The trace says why. At 60 Hz the AN001 frame is predicted inside a
refresh, so there is no stand-in. The frame is presented 11 ms after the key. The sharp pass lands
at 40 ms and goes up at 52 ms. Take the pass away and the turn presents once.

## 5. What would shorten a turn, left for a round that can measure it

The frame waits for a tick. In WTPDF's trace the frame was asked at the key (3.048 s) and drew in
4.0 ms, but it was collected at the next armed tick, 3.060 s. The render thread wakes the event loop
for the sharp pass, a look and a scene, and not for a frame (`renderer::thread`). So a frame is
held back by up to one period, on a turn and on page one. That is the delay
`crate::cadence` says it never adds ("the first present after a key is not held back by a tick").
A wake after `Finished::Frame` would bring a turn's page forward by up to 8.3 ms at 120 Hz. It
changes when frames go up, so it owes a clock hold on the owner's display and the drive, which
this round's locked session could not give.

