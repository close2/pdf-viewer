# 3595 — The controller bar is drawn by the processor and crosses the C ABI as three facts

Session 2223. Status: accepted. Context: RFC 0015 section 6 round 12; ADRs 3545, 3562, 3578,
3594; ISO 32000-2 §13.4 Table 307 `/ShowControls`.

## Decision

1. **What is on it is a choice.** Table 307 says "whether to display a movie controller bar while
   playing the movie" and nothing of what the bar holds. `viewer_host::video::controller` draws
   the three things every player's bar has: a button (two bars while the clip runs, a triangle
   while it is paused or held), a track filled to the place in the pass where the pass's length
   is known, and that place as minutes and seconds in seven-segment figures — rectangles, so the
   bar carries no font and loads nothing at the first clip.
2. **One raster, every window.** The bar is drawn on the processor into a raster `BAR_HEIGHT`
   pixels high and as wide as the clip's rectangle, and laid over the rectangle's bottom edge:
   `quorra` as an overlay above the picture on either kind of window, GTK as a second
   `GtkPicture` above the picture's, Qt as one more frame appended after the picture's. It is
   drawn again with each picture, and with each sentence the player says (a pause, a resume).
3. **The C ABI gets the facts, not pixels.** `quorra_window_controller` answers whether the clip
   runs, its place in milliseconds and the pass's length or -1; a host draws its own bar, as it
   draws its own controls. Space pauses and resumes, as in the toolkits.
4. **A press.** `VideoPlayer::press` maps a press on the button to pause or resume; no window
   routes its pointer to it yet, and the keyboard's Space does the same.

## Consequences

Table 291's `/C` true still sets a rendition aside: the bar is offered for a movie's
`/ShowControls`, and taking `/C` with it is a reading of its own for a later round.
