# 1171 — A print operation has two ends

The HOST-UI round of batch twenty-six: RFC 0004 sections 4 to 6, as far as one round carried them.

## What was found

**Table 167 has two device columns and this tree read one.** `annotation::displayed` consulted bits
2 and 6 and never bit 3, so every page this program produced was decided by the screen's column —
over the most-stated annotation flag in either corpus (316 383 of the crawl's). Bit 3's *third*
sentence is what decides the population rather than an edge case: an annotation stating no `/F` has
the bit clear, so reading the first two alone takes every constructed appearance off every printed
page.

**`Operation::Print` already existed.** The brief expected printing to be gated by an operation this
program does not have; `pdf-transform`'s `render` verb has consumed it since session 872. What was
missing was a window that performs one, and `viewer_host::restriction::inert` said so by name.

**The key table has no Control, and that is load-bearing.** `meaning` takes `shift` and nothing
else, deliberately (ADRs 0470, 0526). `Key::P` was §12.4.4's presentation, so printing took Shift
and P — and the residue is that every conventional Ctrl+X binding in these windows is whatever bare
X means, which is handed over rather than fixed.

**`print_protection.pdf` cannot serve as a print-refusal fixture.** `/P −3392` clears bit 3, but
`1234` is its *owner* password and §7.6.4.1 exempts the owner. A walk for a document that opens with
no password and withholds bit 3 was killed after forty minutes, so the four levels over
`Operation::Print` have no end-to-end fixture; the path is `Command::Copy`'s own.

## What was built

Print intent in the model (`AnnotationView::purpose`, `ViewState::set_paper`,
`annotation::target_media`, `contains_appearance_streams`); `Command::Print` with `Start`, `Paper`
and `Finish`, `Event::Printing`, `Query::PrintPage` in the core and across the confined wire;
`viewer_host::printing` (RFC 0004's DPI clamp, §12.2's Table 147 defaults); Shift+P level in all
three windows; `GtkPrintOperation` in `quorra-gtk`, preview in the other two; six C entry points.

## What is left owed

The Qt `QPrinter` bridge, the winit panel and IPP, the spool container, page ranges, scale modes and
n-up — each named in RFC 0004's new status paragraph and in ADR 1180 §6. The GTK path was compiled
and reviewed and **not driven**: no print job was sent on a shared machine.
