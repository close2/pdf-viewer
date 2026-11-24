# 3610 — Table 291's controls are the bar, and a press on it is the bar's

Session 2227. Status: accepted. Context: ADRs 3350, 3553, 3595; ISO 32000-2 §13.2.5 Table 291
`/C`, §13.4 Table 307 `/ShowControls`.

## Decision

1. **`/C` true is honoured over a clip with pictures.** Table 291: "A flag specifying whether to
   display a player-specific controller user interface when playing. EXAMPLE 1 play/pause/stop
   controls." The bar ADR 3595 draws for `/ShowControls` is such an interface, so `/C` sets
   `Playing::controls` and is honoured in `MH` and `BE` alike for a clip this program shows
   pictures of. A clip without pictures has nowhere to show it: `uncontrolled` makes an `MH` `/C`
   true non-viable and sets a `BE` one aside, said, as before. (The brief quoted the entry as "the
   player should display a user interface"; the table's words are the ones above.)
2. **A press on the bar is the bar's.** `controller::at` places a press against the bar over a
   rectangle — its bottom `BAR_HEIGHT` pixels, from its left edge — and `VideoPlayer::press_over`
   acts on the button and answers that the press was the bar's, so the page never sees it and the
   release after it activates nothing (the core activates only on a press it saw). Routed in
   `quorra`'s document window and its floating window, GTK's document window and its floating
   window, Qt's window, and the C ABI window through `quorra_window_pointer`. The floating windows
   draw the bar too (`window_picture`), over their bottom.

## Consequences

A press on the track does nothing yet: seeking is not built.
