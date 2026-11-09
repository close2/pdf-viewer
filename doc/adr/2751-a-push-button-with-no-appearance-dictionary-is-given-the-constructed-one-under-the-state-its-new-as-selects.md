# 2751 — A push button with no appearance dictionary is given the constructed one, under the state its new `/AS` selects

Session 1970, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/prepare/annotations_and_attachments.rs`:
`prepare_appearances`, `Appearances::push_buttons`; `archive/sites.rs`: `push_button`;
`archive/rewrite.rs`: the `AppearanceDictionary` arm). It takes the first site of R16 in
`doc/pdfa-blocking-issues.md`.
Context: ADRs 1099, 1105, 1626, 2319; `doc/questions/A21`, `A48`.
Clauses: ISO 32000-2 §12.5.2 (Table 166), §12.5.5, §12.7.5.2.1–§12.7.5.2.3 (Table 229);
ISO 19005-2 and ISO 19005-4 section 6.3.3; `TechNote 0010` A012, A023.

## 1. The construction

`prepare_appearances` refused every button widget stating no `/AP`, because §12.7.5.2.3 makes a
check box's state names its values and an absent appearance dictionary names none. That reason is
the check box's and the radio button's. **A push button retains no value** (§12.7.5.2.2), so the
reason does not reach it, and ADR 2319 already writes a push button's one stream as the one entry
of a subdictionary under `Normal`, with `/AS /Normal` selecting it.

So a push button (Table 229 bit 17, read through §12.7.4.1's inheritable `/Ff`) is constructed as
any other widget is — `pdf_model::appearance::for_annotation`, Table 192's `/MK`, reported as A21
asks — and the rewrite writes `/AP << /N << /Normal n 0 R >> >>` and `/AS /Normal`. The state name
decides nothing a reader draws: `/AS` selects the one stream and nothing else refers to it.

- **Every profile, both parts.** It is the `Stated` decision of the appearance rows, as every
  constructed appearance is; no loss is named because nothing the file states is dropped, and the
  construction is §12.7.4.3's and Table 192's, which the viewer already draws from.
- **Kept refused**: a check box's or a radio button's widget with no `/AP`, in a sentence that now
  names those two kinds. Writing a state there would be choosing a value of the field
  (A48's forbidden half), and §12.7.5.2.3 puts the on appearance in the dictionary the file omits.

## 2. What it freed

`160F-2019` (pdf.js): seven push buttons named `link`, `/MK << /IF << >> >>` only, so the
construction draws nothing and the reader drew nothing. Under `as-if-printed` it was the document's
only refusal at both parts. `checkbox_no_appearance` is a check box pair and stays refused.

## What it costs

Nothing a reader sees: each written form is the reader's own construction, and on the freed file it
is empty (`doc/history/1970`).
