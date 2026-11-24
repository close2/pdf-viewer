# 3586 — Table 304's monitor is found among the toolkit's own, and never guessed

Session 2221. Status: accepted. Context: RFC 0015 section 6 round 11; ADR 3585; ISO 32000-2
§13.2.7.5 Table 304, §13.2.6.1 Table 294's `/M`.

## Decision

1. **One arithmetic, every toolkit.** `viewer_host::video::screens::monitor` takes the toolkit's
   monitors as `Monitor` (area in device pixels, colour depth where reported, primary) and the
   document window as `DocumentWindow` (its area, or only the monitor holding most of it, or
   nothing), and answers Table 304's value as the table words it. `quorra` maps winit's
   `available_monitors`, GTK maps GDK's `monitors` and `monitor_at_surface`.
2. **Value 2's rule is followed literally**: "If no monitor is considered primary, shall treat as
   case 0". GDK 4 has no primary monitor, and winit on Wayland none either.
3. **Ties** ("Ties may be broken in an implementation-dependent manner") go to the monitor holding
   the largest section of the document window, then to the earlier in the toolkit's list. A
   choice, not a reading.
4. **What a toolkit does not report is said, never guessed.** GDK 4 states no colour depth and no
   window position: GTK offers 0, 2, 4, 5, 6. winit on Wayland states 32 bits for every output
   whatever it is, and no position: Wayland offers the same five. X11's depth is the screen's root
   depth, the same for every monitor, so 3 is offered and ties there. "Smallest section" counts only
   monitors holding some of the window.
5. **Where `/M` applies.** A full-screen window goes on the chosen monitor. A floating window is
   positioned by `/RT` and `/P`, and `/M` positions it where `/RT` is 3, "The monitor specified by
   M"; under the other `/RT` values the position decides the monitor. A host that places no window
   cannot honour a floating window's `/M` (ADR 3585).

## Costs

No machine here has a second physical monitor; the choice between two is tested over two described
monitors, and the toolkits' lists are exercised with the one a virtual display has.
