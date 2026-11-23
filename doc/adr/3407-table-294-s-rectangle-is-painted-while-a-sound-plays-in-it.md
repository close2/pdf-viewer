# 3407 — Table 294's rectangle is painted while a sound plays in it

Session 2176. Status: accepted. Supersedes ADR 3399 decision 3 for `/W` 3. Context: ADRs 3350,
3399; ISO 32000-2 §13.2.6.1, Tables 218, 293–295, §12.5.6.18.

## Decision

1. **`/W` 3 is this player's too.** Table 294 plays the media in "\[t]he rectangle occupied by the
   screen annotation", and `/B` "shall be used if the media object does not entirely cover the
   rectangle"; a sound covers none of it, so the player honours the type by painting `/B` over the
   whole rectangle at `/O`'s opacity while the sound plays. `/M` and `/F` are ignored for the type
   as the table says. A `/B` that is not three numbers in `0..=1`, or an `/O` that is not one, is
   non-viable in `MH` and set aside, said and defaulted in `BE`. `MH`'s entry comes before `BE`'s.
   `/W` 0 and 1 stay non-viable in `MH`, set aside in `BE`.
2. **The default `/B` is a choice: opaque black.** The table makes it "implementation-defined" and
   asks for "a reasonable value based on the value of W"; for a rectangle on a page this program
   paints the colour a player's surface shows where its media draws nothing, so that the rectangle
   reads as playing. `/W`'s own default is 3, so a rendition stating no screen parameters paints it.
3. **Where.** The clip carries `pdf_model::media::Background`; the core pairs it with the rendition
   action's Table 218 `/AN` as `viewer_core::Screen` on the `SoundData`; the window's player follows
   which sound sounds (`sound::painting`, from the order its answers come in) and sends
   `Command::PlayingIn` when one starts and when it ends, is stopped or refused; the core keeps it
   in `ViewState::playing_in` and `content::annotations` fills the `/Rect` after the annotation's
   appearance — on the screen only, and only where §12.5.3 shows the annotation. Both windows that
   play sounds (GTK and Qt through `native_window`, and `quorra`) send it; it crosses the confined
   wire both ways.
4. A sound action's or sound annotation's sound has no screen and paints nothing.

## Cost

One command, one view-state field, one fill. The rectangle is drawn again at each start and end,
which is a page re-interpretation, as a pointer's appearance change already is.
