# 3425 — A form stating no resources draws from the page it is drawn on

Session 2181. Status: **accepted** and built; ADR 3412 item 5. Context: ADR 1021
(`pdf_archive::reach`), ADR 3394, ADR 3410; ISO 32000-2 §7.8.3; ISO 19005-2 and ISO 19005-4
section 6.2.2's last sentence.

## Reading

§7.8.3 associates a resources dictionary with the content stream that states it, and names one
stream that draws with a dictionary it does not state: "PDF files written obeying earlier versions
of PDF may have omitted the Resources entry in all form XObjects and Type 3 fonts used on a page.
All resources that are referenced from those forms and fonts shall be inherited from the resource
dictionary of the page on which they are used." So the names such a stream states are references
into the page's dictionary. `owners` read only a page's `/Contents`, so pdf.js `issue6541`'s page,
whose `/Contents` draws a form stating no `/Resources` that names `/R42` to `/R45`, had those entries
exempted as unreferenced; ADR 3410's removal read every content stream and kept them, but the
exemption itself still withdrew findings at them.

## Decision

`reach::Lender` adds to each page owner the names of every content stream drawn on that page that
states no `/Resources`: from the page's `/Contents` and its annotations' appearance streams it
follows every name through the `/XObject`, `/Pattern`, `/Font` and `/ExtGState` entries of the
dictionary each stream draws with — a form, a tiling pattern, a Type 3 font's glyph procedures, a
soft mask's group — visiting each indirect object once per page. **The page lends at any depth**:
a stream stating no `/Resources` inside a form that states its own still draws from the page, the
reading `pdf_model`'s interpreter takes (ADR 1059), and an appearance stream stating none does the
same (ADR 0255). The names only ever grow, so the exemption can only shrink: the direction
`doc/todo/62` section 3 places every unit of risk against.

Tests: `tests/reach.rs`'s `a_name_a_form_stating_no_resources_invokes_is_the_pages` (with its
undrawn-form control) and `a_form_stating_no_resources_inside_one_that_does_still_draws_from_the_page`;
both fail with the lending switched off.
