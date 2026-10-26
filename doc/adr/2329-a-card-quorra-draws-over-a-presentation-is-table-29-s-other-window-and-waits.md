# 2329 — A card `quorra` draws over a presentation is Table 29's other window, and waits

Status: implemented. Session 1747, 2026-10-25, the host slot of batch one hundred and fourteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clause: ISO
32000-2 Table 29's `FullScreen` under §12.2's row; no row moves. Context: ADRs 0470, 1628, 1643,
2300, 2309 (its section 3 is the choice this takes), 2317.

## 1. The question

Table 29: `FullScreen` is "[f]ull-screen mode, with no menu bar, window controls, or any other
window visible". ADR 2309 held the three dialogues a document raises while a toolkit window
presents and left `quorra`'s cards out, on the reading that a card drawn inside the presentation's
own surface is not a window.

## 2. The decision

That reading is not the one `quorra` already makes. Its `apply_chrome` takes the panel, the find bar
and the About card down when the presentation hides "any other window", and
`report_unobeyable_chrome` names its panel, find bar and cards as that other window. A question card
drawn over the slide is the same kind of thing as the About card the presentation already hides, so
the three cards a document raises are held by the toolkits' rule: a question at `CLAUDE.md`'s *ask*
level (`App::put_a_question`, every call site), a script's `app.alert` or `app.response`, and
§7.6.4.1's password prompt. Each is said to wait in the pump's own words — now
`presentation_pump::{QUESTION_WAITS, SCRIPT_WAITS, PASSWORD_WAITS}`, read by both — and put up in the
order raised once the window is out of full screen, one at a time because the window draws one card.
The release runs at the top of every `about_to_wait` rather than at each way out of full screen,
so Escape, `p` and a tab change all end the hold, as `synchronize_if_owed` does for the other
windows. A script's question the core withdraws while held leaves the hold and is said withdrawn as
one on the card is. The cards a reader's own key opens (the restriction levels, ISO 19005's
chooser) are not held, for ADR 2309 section 3's reason.

## 3. `quorra-confined`

The confined window starts no presentation: it sends no `Command::Tick`, ignores
`Event::Transition`, and never sets full screen. No card of its own can be over a presentation, so
nothing is held there, and nothing is owed until it presents.

## 4. Witness and cost

Drive step `106-presentation-held` now runs in `quorra`: the deck's link is clicked while it
presents, the window says the question waits, Escape ends the presentation and the card comes up,
answered "Do not". No pixel moves; one `VecDeque` per window and one empty check per loop turn.
