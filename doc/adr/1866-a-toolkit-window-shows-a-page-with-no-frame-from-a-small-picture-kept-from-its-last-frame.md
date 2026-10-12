# 1866 — A toolkit window shows a page with no frame from a small picture kept from its last frame

Session 1514. Status: **accepted** and **built** in `quorra-gtk` and `quorra-qt`. Owed by ADR 1843
§4 and round 1503's record: a page that arrives on the screen with no frame showed the ground.
Context: ADRs 0443, 0457, 1843, 1865; `doc/todo/37`'s five rules. Code:
`crates/viewer-host/src/stand_in.rs` (`Retained`, `reduced`, `taps`, `RETAINED_PAGES_WORD`), both
hosts' `take_the_drawn`, their `NeedsRender` arm, `Host::placements` (GTK) and `Host::frames` (Qt),
both binaries' `--proxy-pages`. Tests: `stand_in.rs`'s five new ones, each binary's
`the_retained_pages_are_a_count_the_window_is_given`. Drive: step 82's `retained_page`.

## 1. What was found, and the premise that did not hold

**A rotation cannot reach either window.** The brief's third item, a turn that shows the old frame
unscaled, rests on a view rotation, and there is none: `viewer_core::Command` has no rotation and
`Viewing` carries a page, a zoom and a scroll; no key, menu or binary word in the three windows turns
a page; §7.7.3.3's `/Rotate` is the file's and fixed while it is open. Every change of a page's
extent the core makes is a magnification, which ADR 1843 already scales. So nothing was built for
it, and `stand_in::placed`'s refusal to stretch a change of shape stays as the guard it was.

**A page with no frame is a revisit.** `Viewer::arrange` drops the pixels of a page that leaves the
arrangement, so the page that arrives with no frame is a page turned back to under Table 29's
`SinglePage`, a page scrolled back into a column, or one a notch out brings back. Read in the code
at HEAD: `quorra-gtk`'s refresh changes nothing when the viewer answers no frame, so it leaves the
page it had up — page two, after a turn back to page one — for the length of the draw; `quorra-qt`
paints nothing and shows the ground.

**The hypothesis was not needed**: a retained page drawn on the drawing thread is `quorra`'s
construction, because its frame is a picture of the window (ADR 0443). A toolkit window's frame is
the whole page already (`viewer_core::MAX_PIXELS`), so the picture is a reduction of a frame it was
handed — no draw, and nothing on the drawing thread at all.

## 2. Decision

1. **`viewer_host::stand_in::Retained`** keeps, from the first finished frame of a page at the
   document's ink, that page reduced to 512 pixels along its longer side (`quorra`'s `PROXY_EDGE`,
   for ADR 0443's reasons), newest first, eight by default (`PROXY_PAGES`). A request under another
   ink drops every picture of that document (`RenderRequest::ink`, ADR 0457), and a frame drawn under
   the old ink keeps nothing.
2. **`Retained::fill`** adds the picture for every page of the document in front that the
   arrangement places and no pixels are shown for, scaled by the toolkit into the extent the page has
   now — the path ADR 1843's stand-in already takes — so the frame that lands replaces it (rule 1).
3. **Each pixel is the alpha-weighted average of up to sixteen samples** spread over its footprint,
   as many as whole frame pixels fit it, so the cost is bounded by the picture and not by the frame,
   however far the page is zoomed in.
4. **Rule 2 is kept by where the crate sits**: `viewer-host` is linked by the three windows and
   `viewer-ffi` and by no gate, oracle or harness; `Query::Frame` still answers the true raster.
   Rule 3: the window says `stands in at W×H, its retained w×h page scaled until its frame lands`,
   apart from a frame standing in. **`--proxy-pages N`** is the extent, `quorra`'s word, read by one
   parser in both windows; `0` turns it off.

## 3. Measured

Xvfb, step 82's `retained_page`: HEAD says no retained picture in either window (the control);
this round **works** in both, twice — the red block 196–197 × 196 against the landed frame's 198 ×
198 in `quorra-gtk`, 200 × 200 against 202 × 202 in `quorra-qt`, no blue. The reduction, by each
window's trace, 1.1–4.0 ms a page kept on Xvfb and 1.2–6.8 ms on the owner's display, on the
toolkit's thread, once per page per ink. **Not judged**: a photograph 0.15 s after the turn on the
owner's display — GTK's `Left` was not delivered and Qt's `xwd` of its window held page two after
the window had said `its retained` — and the Xvfb before-photograph, whose turns were not
delivered to a window with no manager; the step's own verdicts are the figure.
