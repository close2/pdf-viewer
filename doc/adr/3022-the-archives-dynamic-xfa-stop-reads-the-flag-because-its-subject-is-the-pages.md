# 3022 — The archive's dynamic-XFA stop reads the flag, because its subject is the pages

Session 2082. Status: **implemented** (comments only; the predicate is unchanged). Replaces the premise
`archive::sites::xfa_form_is_dynamic` gave for reading `/NeedsRendering` alone. Its decision, ADR 1257's, stands on a
different reason. Context: review 2038 (finding D57). Clauses: ISO 32000-2 §7.7.2 Table 29 (`/NeedsRendering`),
§12.7.3, Annex K.1–K.2.

## Context

The archive converter's `dynamic = "stop"` stops on a form whose document states `/NeedsRendering true`.
Two premises supported that. The template "is XML this program does not parse, and `CLAUDE.md` excludes XFA on the
annex's own permission". And for a form with no flag, "nothing in the file distinguishes them from a static one".
Neither is true now. `CLAUDE.md` puts XFA's static half and, after the ledger, the layout of its dynamic half on the
near side (ADR 2876). `pdf_model::xfa::Template::dynamic` parses the template and finds what XFA 3.3 excludes from
XFAF, a `subformSet` or an `occur` whose bounds vary (ADR 2380). So a wider class can be identified, and the audit
asks whether the stop should take it.

## Decision

**No.** The stop keeps reading the flag, because what it decides is whether the pages the file holds are the
pages to keep. That is Table 29's subject: the entry "specifies whether the document shall be regenerated when the
document is first opened". A document that states it true says its held pages are not what a reader is to show.
Converting them would archive pages the producer disowned, and `stop` is the reader's way to refuse that. A
document whose template grammar is dynamic and which states no flag has stated the default. Its held pages are the
ones a reader shows, and this viewer shows them. Removing `/XFA`, which ISO 19005 requires, then loses the form's
dynamic behaviour and no page content. That loss is the one every XFA form suffers in an archive, and the conversion
already names it for a static form. The grammar describes how the form behaves. It does not say that the held pages
are wrong.

Laying out a dynamic template (ADR 2876, `CLAUDE.md`'s fourth amendment) is a separate question. If that layout is
built and a reader asks for generated pages, the archive could keep them. That would be an option on the archive's
own terms, decided when the layout exists, and it is not a reason to widen this stop.

## Consequences

`xfa_form_is_dynamic`'s doc comment states this reason. The audit's grammar-only refusal test was kept as unexecuted
review material and asserts nothing in the tree, so nothing fails. D57 is `argued`.
