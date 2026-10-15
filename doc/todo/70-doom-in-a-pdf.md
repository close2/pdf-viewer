# 70 — Doom in a PDF: what a document that runs a program asks of this viewer

Status: **open** — the evaluation is ADR 1950, and its first item is built. A 30-band item: items
2–5 are three fixes and a report that serve every scripted or form document, and item 6 is a
decision.
Priority: the owner's, 2026-10-15: "find out, what still needs to be implemented to get it working"
— and fix what is wrong on its own terms on the way.
Witness: `/home/cl/projects/pdf-viewer/tmp/doom.pdf`, the owner's, **not in the repository**
(6 496 651 bytes, one page, 234 direct widget dictionaries, no `/AcroForm`, a 6 415 756-byte page
`/O` script). The repaired, chained and staged copies ADR 1950 section 4 describes are written from
it with `qpdf --json-input`; none is committed.
Instrument: `crates/pdf-script-worker/examples/script_cost.rs` — the open sequence, each run, each
timer tick and the page interpreted after it, at the field-event budget or `--lifted`.
Clauses: §12.6.4.17, Table 221 (`/JS`), Table 31 (`/Annots`), Table 224 (`/Fields`), §12.7.4.3.

Each item below is argued and priced in ADR 1950 section 5; this file is the order.

## 1. Built — a script this program will not read is said, with why (ADR 1950 section 3)

Left: at Table 197's events and an outline item a host still says Tier 0's sentence beside it,
because `ScriptsRan` does not carry what the view state said (`viewer_core::interact::handed_over`).

## 2. Variable text's `/DA` font loaded once per interpretation, not once per field

42 % of the instructions of interpreting a page of 200 changed text fields are
`pdf_font::LoadedFont::load` called from `variable_text::set_in`. S–M; every form pays it. No pixel
may move: `raster_golden` and the six arms prove it, and the figure is `script_cost`'s "page
interpreted after a tick" before and after, in one clock hold.

## 3. A timer that cannot starve the window

At a period shorter than the tick's own cost (`framebuffer.pdf`, 0 ms: 64–120 ms a tick) `quorra`
presents no frame at all. A due timer waits for the frame its last run asked for. S, `viewer-ui` and
`viewer-core`; driven at 0 ms and 250 ms under Xvfb, every window.

## 4. A widget no `/AcroForm` reaches, and a direct annotation dictionary, said once

Both are silent today. The condition comes from Table 224's Required `/Fields` and Table 31's "shall
contain indirect references" (trap 11), and is costed in gated pages. S.

## 5. A refused method refused at its call, not at its name

`typeof print` throws today (ADR 1591's accessor). M: `crate::surface` gains a member's kind; both
script-corpus gates are held again.

## 6. Running the document — a decision, not recommended

Direct annotations as interactive objects (L), fields from widgets no form names and an appearance
for widgets with no `/DA` (repairs no clause defines), the bounds of ADR 1590 and the worker's
96 MiB, a `Date` that moves inside a run (Doom's start-up waits on it and never ends otherwise),
and an engine whose tick is about 200 ms against the 28.6 ms Doom's 35 tics a second allow (ADR
1950 section 5 item 6). A `Q` file for the owner is owed before any of it is started.
