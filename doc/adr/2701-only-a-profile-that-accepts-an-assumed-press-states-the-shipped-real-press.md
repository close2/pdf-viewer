# 2701 — Only a profile that accepts an assumed press states the shipped real press

Session 1943, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/prepare/colour_and_graphics.rs`:
`shipped_for`, `ASSUMED_INKS`, `Intent::assumed_inks`; `archive/decision/colour_and_graphics.rs`:
`OUTPUT_INTENT_STATES_THE_ASSUMED_INKS`).
Context: A780, ADR 2700, ADR 2691, ADR 2518, ADR 1153, ADR 0927.

## Decision

Where a conversion adds a CMYK output intent for `DeviceCMYK` the file never characterised, it
writes ADR 2700's profile of the assumed inks, **unless the plan authorises the loss
`press-assumed`**, and then the shipped GRACoL press as before (A780's option a, with ADR 2691's
sentence). Of the shipped profiles only `as-if-printed` authorises it, so `as-shown`,
`no-data-loss` and `state-every-loss` — and every other profile, and a run with no profile — state
the assumed inks.

`press-assumed` is therefore asked at every target (`ASKED_AT_EVERY_TARGET` in
`archive/config.rs`): its row is part 4's, and `as-if-printed`'s `discard` there now also
authorises it at part 2, where it does nothing but choose this press.

The switch is the authorisation rather than the profile's name because the authorisation is what
the profile says about the question: `press-assumed` is the loss of stating a real press for colour
the file never characterised (ADR 2518), and a profile that accepts it is one whose reading — a
print scanned back in, A694 — is honest about a real press. The intent itself stays a statement,
as A780 option a asks; no loss is recorded for it.

The report's sentence for the assumed inks keeps ADR 0927's claim that this renderer already
showed the colours that way, now true (ADR 2700's bounds), and says that the profile characterises
this renderer and not a physical press.
