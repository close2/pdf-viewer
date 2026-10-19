# 70 — Doom in a PDF: what a document that runs a program asks of this viewer

Status: **open** — the evaluation is ADR 1950; items 1–5 are built (ADRs 1950, 1956, 1957, 1974,
1975), and item 6 is a decision.
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

At Table 197's events and an outline item `ScriptsRan::said` carries what the view state said, and
a host drops Tier 0's sentence on `ScriptsRan::answered`, so the script is said once (ADR 1975).

## 2. Built — variable text's `/DA` font loaded once per interpretation (ADR 1956)

`variable_text::LayoutFonts`, held by the interpreter and passed to every layout: 603 loads become 6
on the framebuffer witness, and interpreting it 2 199 M → 954 M instructions. No digest of the six
arms moved. The appearance's `Tf` reads the layout's font out of the interpreter's `FontCache`, so
a face is loaded once an interpretation for both, and not again where the cache outlives it, as an
open document's does (ADR 1974): 6 → 3 loads on the witness at three ticks.

## 3. Built — a timer that cannot starve the window (ADR 1957)

In `quorra` a due timer waits while a render of a page on the screen has not reached the window: at
0 ms, 0 → 54 frames presented in 20 s. The toolkit windows draw the fields in controls and the page
on their own thread, and answered every tick with its page without the rule (ADR 1957 section 3).

## 4. Built — a widget no `/AcroForm` reaches, and a direct annotation dictionary, said once

At a page's open with a runner (`script_sites::report_unreachable`): Table 31's sentence for an
annotation written into `/Annots` as a dictionary, and §12.7.3's or Table 224's for widgets where the
catalog names no form or the form no `/Fields` — `field_table`'s two early returns, nothing wider.
Page one of the corpus: 0, 1 (`issue12963.pdf`) and 1 (`issue15597.pdf`) documents of 944. Left:
with scripts off, a person's press on such a widget is still unsaid.

## 5. Built — a refused method refused at its call, not at its name (ADR 1957 section 2)

`crate::surface::Member` on every row; `typeof print` answers `function`, and the call throws. None
of the eight property rows is a method in Adobe's reference (ADR 1975 section 2). Left: the
reference's XFA page asks `typeof xfa == "object"` to tell an Acrobat form, "The xfa object is
undefined in an Acrobat form", and here that read throws.

## 6. Running the document — a decision, not recommended

Direct annotations as interactive objects (L), fields from widgets no form names and an appearance
for widgets with no `/DA` (repairs no clause defines), the bounds of ADR 1590 and the worker's
96 MiB, a `Date` that moves inside a run (Doom's start-up waits on it and never ends otherwise),
and an engine whose tick is about 200 ms against the 28.6 ms Doom's 35 tics a second allow (ADR
1950 section 5 item 6). A `Q` file for the owner is owed before any of it is started.
