# 2309 — A dialogue a document raises waits until the presentation ends

Status: implemented, for integration. Session 1737, 2026-10-24, the clause slot of batch one
hundred and thirteen. Clause: ISO 32000-2 Table 29's `FullScreen` under §12.2's row. Context: ADRs
1628, 1643, 2105, 2300, 2301.

## 1. What the text asks

Table 29: `FullScreen` is "[f]ull-screen mode, with no menu bar, window controls, or any other
window visible". A question put in a dialogue while the window presents is another window visible,
and the presentation it interrupts is the reader's.

## 2. The decision

`viewer_host::presentation_pump` holds the three dialogues a *document* raises — a question at the
*ask* level (`NativeWindow::ask`, which every former `put_the_question` call site now reaches), a
script's `app.alert` or `app.response`, and §7.6.4.1's password prompt — while the window presents
full screen, says on the terminal that each waits, and puts them up in the order raised once the
window has left full screen. The release is in `synchronize_if_owed`, which every window's drain
runs on a settled queue, so Escape, `p` and a tab whose document is not presenting all end the hold
without a line in any toolkit. A toolkit with one question dialogue gets the next only after the
answer (`answer_the_question` releases again). A script's question the core withdraws while held
leaves the hold and is said withdrawn.

## 3. What is chosen rather than derived

- **Held, not answered.** A question answered or refused for a reader watching a slide would be an
  answer nobody gave; leaving full screen is always the reader's act.
- **The reader's own windows are not held.** A print dialogue, a file chooser and ISO 19005's
  chooser open on a key the reader pressed; that is the reader asking for a window in front.
- **`quorra` and `quorra-confined` are not covered.** Their questions are cards drawn inside the
  presentation's own surface and their presentation is not the shared pump's; Table 29 names other
  *windows*, and a card is not one. Holding them is a later choice, not owed by this reading.
