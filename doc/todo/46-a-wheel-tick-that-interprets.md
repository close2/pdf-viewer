# A wheel tick that interprets: §12.5.3's re-interpretation runs on the event thread

Status: **open — named by ADR 0707 and deliberately not smuggled into that fix.**
Priority: 46 — performance a person feels directly, on a bounded population of documents.
Corpus: `doc/ISO_32000-2_sponsored_EC3.pdf` — the standard itself carries `NoZoom`
annotations; ADR 0177's-era census put such annotations on 51 of 974 corpus documents
(re-count before acting: `viewer.rs`'s own comment carries the numbers and their decay).
Code: `crates/viewer-core/src/viewer.rs` (the `set_magnification` → `reinterpret` path),
`crates/viewer-core/src/open.rs` (`interpret` runs inside `Viewer::handle`).
Instrument: the trace's event lines — `zoom In … -> 3 event(s) in 19.3ms` — on any
view-dependent document; the owner's `/tmp/trace.txt` of 2026-08-28 is the standing witness.

## The mechanism, and what ADR 0707 already took

§12.5.3 makes a `NoZoom` annotation's placement a function of the magnification, so a zoom
of such a page must re-interpret it. ADR 0707 fixed what that re-interpretation *broke* —
it no longer supersedes the ink, so stand-ins cover the render — but the re-interpretation
itself still runs synchronously inside the zoom event: **10–19 ms per wheel tick** for two
ISO-spec pages, paid on the thread that dispatches every other event. With stand-ins
restored it no longer decides what the person sees; it still decides how fast the event
loop turns during a gesture.

## The shapes, none of them chosen yet

- **Interpretation off the event thread.** The honest fix and the expensive one: `interpret`
  is called inside `Viewer::handle`, and every host consumes `NeedsRender` events
  synchronously produced from it. Moving it means an async seam in `viewer-core`'s
  contract — requests that arrive later than the event that caused them — which touches
  all three hosts and the headless tests. A real round with an ADR, not a patch.
- **Re-place rather than re-interpret.** The display list differs only in the adjusted
  annotations' placement; a re-interpretation that reused everything else would be
  near-free. Today interpretation is monolithic — this is a partial-reinterpretation seam
  nothing else needs, and it should be priced against the first shape rather than assumed
  cheaper.
- **What is *not* acceptable**: debouncing the re-interpretation to gesture-settle. The
  frames rendered mid-gesture are real frames, presented as correct; drawing them from the
  old magnification's interpretation would show the annotation at a size §12.5.3 says it
  never has. A stand-in may approximate; a rendered frame may not.

## What decides the priority

The population is small (51 of 974 then; re-count) but the standard's own PDF is in it,
and that is the document this project opens most. If the kernel-floor round lands and
zoom steps drop toward the 30s, this term becomes a visible fraction of a gesture on
exactly that document.
